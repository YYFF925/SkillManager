//! 两层上下文：客户端只报引用，Host 从 core 补齐。

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::HostRuntime;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ViewRef {
    pub page: Option<String>,
    pub domain: Option<String>,
    pub selection: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentSession {
    pub session_id: String,
    pub view: ViewRef,
    #[serde(default)]
    pub snapshot: Value,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentMessage {
    pub role: String,
    pub text: String,
    pub task_id: Option<String>,
    pub created_at: String,
}

pub fn set_context(runtime: &HostRuntime, body: Value) -> Result<Value, String> {
    let session_id = body
        .get("sessionId")
        .or_else(|| body.get("session_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| "缺少 sessionId".to_string())?
        .to_string();
    let view: ViewRef = serde_json::from_value(body.get("view").cloned().unwrap_or(json!({})))
        .unwrap_or_default();
    let ctx = runtime.get()?;
    let conn = ctx.state().connection();
    conn.execute(
        "INSERT INTO agent_sessions (session_id, view_ref, snapshot, created_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(session_id) DO UPDATE SET view_ref = excluded.view_ref",
        rusqlite::params![
            session_id,
            serde_json::to_string(&view).unwrap_or_else(|_| "{}".into()),
            "",
            chrono::Utc::now().to_rfc3339(),
        ],
    )
    .map_err(|err| err.to_string())?;
    Ok(json!({ "ok": true, "sessionId": session_id }))
}

pub fn view_for(runtime: &HostRuntime, session_id: &str) -> ViewRef {
    let Ok(ctx) = runtime.get() else {
        return ViewRef::default();
    };
    ctx.state()
        .connection()
        .query_row(
            "SELECT view_ref FROM agent_sessions WHERE session_id = ?1",
            [session_id],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn remember_session(runtime: &HostRuntime, session: AgentSession) -> Result<(), String> {
    let ctx = runtime.get()?;
    ctx.state()
        .connection()
        .execute(
            "INSERT INTO agent_sessions (session_id, view_ref, snapshot, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(session_id) DO UPDATE SET
               view_ref = excluded.view_ref,
               snapshot = excluded.snapshot",
            rusqlite::params![
                session.session_id,
                serde_json::to_string(&session.view).unwrap_or_else(|_| "{}".into()),
                session.snapshot.to_string(),
                session.created_at,
            ],
        )
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub fn push_message(runtime: &HostRuntime, session_id: &str, message: AgentMessage) -> Result<(), String> {
    let ctx = runtime.get()?;
    ctx.state()
        .connection()
        .execute(
            "INSERT INTO agent_messages (session_id, role, text, task_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                session_id,
                message.role,
                message.text,
                message.task_id,
                message.created_at,
            ],
        )
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub fn mark_task_cancelled(runtime: &HostRuntime, task_id: &str) -> Result<(), String> {
    let ctx = runtime.get()?;
    ctx.state()
        .connection()
        .execute(
            "INSERT INTO agent_messages (session_id, role, text, task_id, created_at)
             VALUES (
               COALESCE((SELECT session_id FROM agent_messages WHERE task_id = ?1 LIMIT 1), 'unknown'),
               'system',
               'cancelled',
               ?1,
               ?2
             )",
            rusqlite::params![task_id, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub fn list_sessions(runtime: &HostRuntime) -> Result<Value, String> {
    let ctx = runtime.get()?;
    let mut stmt = ctx
        .state()
        .connection()
        .prepare("SELECT session_id, view_ref, created_at FROM agent_sessions ORDER BY created_at DESC")
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(json!({
                "sessionId": row.get::<_, String>(0)?,
                "view": serde_json::from_str::<Value>(&row.get::<_, String>(1).unwrap_or_default()).unwrap_or(json!({})),
                "createdAt": row.get::<_, String>(2)?,
            }))
        })
        .map_err(|err| err.to_string())?;
    let sessions: Vec<Value> = rows.filter_map(|row| row.ok()).collect();
    Ok(json!({ "sessions": sessions }))
}

pub fn list_messages(runtime: &HostRuntime, session_id: &str) -> Result<Value, String> {
    let ctx = runtime.get()?;
    let mut stmt = ctx
        .state()
        .connection()
        .prepare(
            "SELECT role, text, task_id, created_at FROM agent_messages WHERE session_id = ?1 ORDER BY id",
        )
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([session_id], |row| {
            Ok(json!({
                "role": row.get::<_, String>(0)?,
                "text": row.get::<_, String>(1)?,
                "taskId": row.get::<_, Option<String>>(2)?,
                "createdAt": row.get::<_, String>(3)?,
            }))
        })
        .map_err(|err| err.to_string())?;
    let messages: Vec<Value> = rows.filter_map(|row| row.ok()).collect();
    Ok(json!({ "sessionId": session_id, "messages": messages }))
}

/// 发消息时从 core 组装 global（摘要+计数+引用）与 view 详情。
pub fn assemble_snapshot(runtime: &HostRuntime, view: &ViewRef) -> Result<Value, String> {
    let ctx = runtime.get()?;
    let vault = match ctx.vault_opt() {
        Some(v) => v.root().to_path_buf(),
        None => {
            return Ok(json!({
                "global": { "error": "尚未接入 SkillManagerVault" },
                "view": { "empty": true, "reason": "尚未接入 SkillManagerVault" }
            }));
        }
    };
    let index = skill_manager_core::vault::index::SkillIndex::build(&vault).map_err(|err| err.to_string())?;
    let skills: Vec<Value> = index
        .all()
        .iter()
        .map(|entry| {
            let summary = entry.to_summary();
            json!({
                "ref": format!("{}/{}", summary.domain, summary.name),
                "domain": summary.domain,
                "name": summary.name,
                "category": summary.category,
                "tags": summary.tags,
                "translation_stale": summary.translation_stale,
                "source": summary.source.is_some(),
            })
        })
        .collect();
    let translation_stale: Vec<_> = skills
        .iter()
        .filter(|s| s.get("translation_stale").and_then(Value::as_bool).unwrap_or(false))
        .cloned()
        .collect();
    let deployments = ctx.state().deployments().map_err(|err| err.to_string())?;
    let deploy_summary: Vec<Value> = deployments
        .iter()
        .map(|row| {
            json!({
                "skill": format!("{}/{}", row.domain, row.name),
                "target": row.target,
                "status": format!("{:?}", row.status),
            })
        })
        .collect();

    let selected_ref = view
        .selection
        .as_ref()
        .and_then(|sel| {
            sel.get("refs")
                .and_then(Value::as_array)
                .and_then(|a| a.first())
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| {
            view.selection
                .as_ref()
                .and_then(|sel| sel.get("ref").and_then(Value::as_str).map(str::to_string))
        });

    drop(ctx);
    let view_detail = if let Some(reference) = selected_ref {
        match crate::commands::skill::skill_get(runtime, reference.clone()) {
            Ok(detail) => serde_json::to_value(detail).unwrap_or(json!({ "ref": reference })),
            Err(_) => json!({ "ref": reference, "empty": true, "reason": "引用已失效" }),
        }
    } else {
        json!({
            "page": view.page,
            "domain": view.domain,
            "selection": view.selection,
            "empty": view.selection.is_none(),
        })
    };

    Ok(json!({
        "global": {
            "skills": skills,
            "skillCount": skills.len(),
            "translationStale": translation_stale,
            "translationStaleCount": translation_stale.len(),
            "deployments": deploy_summary,
        },
        "view": view_detail,
    }))
}
