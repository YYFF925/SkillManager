//! 两层上下文：客户端只报引用，Host 从 core 补齐。

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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
    let view: ViewRef =
        serde_json::from_value(body.get("view").cloned().unwrap_or(json!({}))).unwrap_or_default();
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

pub fn push_message(
    runtime: &HostRuntime,
    session_id: &str,
    message: AgentMessage,
) -> Result<(), String> {
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
        .prepare(
            "SELECT session_id, view_ref, created_at FROM agent_sessions ORDER BY created_at DESC",
        )
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

/// 同一 Host 对话里、本轮之前的消息，供新 ACP session 回灌。
pub fn prior_transcript(runtime: &HostRuntime, session_id: &str, current_task_id: &str) -> String {
    let Ok(list) = list_messages(runtime, session_id) else {
        return String::new();
    };
    let Some(messages) = list.get("messages").and_then(Value::as_array) else {
        return String::new();
    };
    let mut out = String::new();
    for msg in messages {
        if msg.get("taskId").and_then(Value::as_str) == Some(current_task_id) {
            continue;
        }
        let role = msg.get("role").and_then(Value::as_str).unwrap_or("user");
        let text = msg.get("text").and_then(Value::as_str).unwrap_or("");
        if text.is_empty() {
            continue;
        }
        out.push_str("### ");
        out.push_str(role);
        out.push('\n');
        out.push_str(text);
        out.push_str("\n\n");
    }
    out
}

fn empty_global(reason: &str) -> Value {
    json!({
        "error": reason,
        "skillCount": 0,
        "skills": [],
        "translationStale": [],
        "translationStaleCount": 0,
        "pendingUpdates": [],
        "pendingUpdateCount": 0,
        "newCandidates": [],
        "newCandidateCount": 0,
        "failedChecks": [],
        "failedCheckCount": 0,
        "deployments": [],
        "pausedDeployments": [],
        "pausedCount": 0,
        "pullConflicts": { "hasConflict": false, "paths": [], "gitStatus": [], "message": reason },
        "lastCommandFailures": [],
        "presets": [],
        "presetCount": 0,
        "hermes": { "count": 0, "refs": [] },
        "tools": { "count": 0, "refs": [], "failed": [] },
        "agents": [],
        "agentCount": 0,
    })
}

fn selected_skill_ref(view: &ViewRef) -> Option<String> {
    view.selection.as_ref().and_then(|sel| {
        sel.get("refs")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| sel.get("ref").and_then(Value::as_str).map(str::to_string))
    })
}

fn omit_skill_bodies(mut detail: Value) -> Value {
    if let Some(content) = detail.get_mut("content").and_then(Value::as_object_mut) {
        content.remove("original");
        content.remove("translation");
        content.insert("bodyOmitted".into(), json!(true));
    }
    detail
}

fn last_command_failures(runtime: &HostRuntime) -> Vec<Value> {
    let Ok(ctx) = runtime.get() else {
        return Vec::new();
    };
    let path = skill_manager_core::diagnostics::log_path(ctx.state());
    drop(ctx);
    skill_manager_core::diagnostics::read_recent(&path, 40)
        .unwrap_or_default()
        .into_iter()
        .rev()
        .filter(|entry| entry.level == "error" || entry.level == "panic")
        .take(8)
        .map(|entry| {
            json!({
                "at": entry.at,
                "action": entry.action,
                "message": entry.message,
            })
        })
        .collect()
}

fn page_objects(runtime: &HostRuntime, page: &str, domain: Option<&str>) -> Value {
    match page {
        "library-code" | "library-lobster" => {
            let domain = if page.ends_with("lobster") {
                "lobster"
            } else {
                "code"
            };
            match crate::commands::skill::skill_list(
                runtime,
                Some(domain.into()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ) {
                Ok(list) => json!({
                    "kind": "library",
                    "domain": domain,
                    "total": list.total,
                    "categories": list.categories,
                    "tagCandidates": list.tag_candidates,
                    "skills": list.skills.iter().take(80).map(|skill| json!({
                        "ref": format!("{}/{}", skill.domain, skill.name),
                        "category": skill.category,
                        "tags": skill.tags,
                        "comment": skill.comment,
                        "translation_stale": skill.translation_stale,
                    })).collect::<Vec<_>>(),
                }),
                Err(err) => json!({ "kind": "library", "domain": domain, "error": err }),
            }
        }
        "presets" => {
            match crate::commands::preset::preset_list(runtime, domain.map(str::to_string)) {
                Ok(list) => json!({
                    "kind": "presets",
                    "presets": list.iter().map(|preset| json!({
                        "ref": format!("{}/{}", preset.domain, preset.id),
                        "name": preset.name,
                        "memberCount": preset.members.len(),
                        "members": preset.members.iter().map(|m| m.name.clone()).collect::<Vec<_>>(),
                        "missingMembers": preset.missing_members,
                    })).collect::<Vec<_>>(),
                }),
                Err(err) => json!({ "kind": "presets", "error": err }),
            }
        }
        "hermes" => match crate::commands::hermes::hermes_profiles(runtime) {
            Ok(profiles) => json!({
                "kind": "hermes",
                "root": profiles.root,
                "profiles": profiles.profiles.iter().map(|profile| json!({
                    "name": profile.name,
                    "displayName": profile.display_name,
                    "kind": profile.kind,
                    "path": profile.path,
                    "skillsDir": profile.skills_dir,
                    "enabledCount": profile.enabled_count,
                })).collect::<Vec<_>>(),
            }),
            Err(err) => json!({ "kind": "hermes", "error": err }),
        },
        "tools" => match crate::commands::tool::tool_list(runtime, None) {
            Ok(tools) => json!({
                "kind": "tools",
                "tools": tools.iter().map(|tool| json!({
                    "slug": tool.slug,
                    "name": tool.name,
                    "category": tool.category,
                    "hasGuide": tool.has_guide,
                    "checkStatus": tool.check.status,
                    "installed": tool.install.is_some(),
                    "error": tool.error,
                })).collect::<Vec<_>>(),
            }),
            Err(err) => json!({ "kind": "tools", "error": err }),
        },
        "agents-code" | "agents-lobster" | "settings" => {
            if page == "settings" {
                json!({
                    "kind": "settings",
                    "settings": crate::commands::settings::settings_get(runtime).ok(),
                    "sync": crate::commands::sync::sync_status(runtime).ok(),
                    "vault": crate::commands::vault::vault_current(runtime).ok(),
                })
            } else {
                let want = if page.ends_with("lobster") {
                    "lobster"
                } else {
                    "code"
                };
                match crate::commands::agent::agent_list(runtime) {
                    Ok(agents) => json!({
                        "kind": "agents",
                        "domain": want,
                        "agents": agents.iter().filter(|agent| agent.domain == want).map(|agent| json!({
                            "key": agent.key,
                            "displayName": agent.display_name,
                            "detected": agent.detected,
                            "enabled": agent.enabled,
                            "skillsDir": agent.skills_dir,
                        })).collect::<Vec<_>>(),
                    }),
                    Err(err) => json!({ "kind": "agents", "error": err }),
                }
            }
        }
        page if page.starts_with("agent:") => {
            json!({ "kind": "agent", "target": page.trim_start_matches("agent:") })
        }
        other => json!({ "kind": "page", "page": other }),
    }
}

fn assemble_view(runtime: &HostRuntime, view: &ViewRef) -> Result<Value, String> {
    let page = view.page.clone().unwrap_or_default();
    let selection = view.selection.clone().unwrap_or(json!({}));
    let mut opened = Value::Null;
    if let Some(reference) = selected_skill_ref(view) {
        opened = match crate::commands::skill::skill_get(runtime, reference.clone()) {
            Ok(detail) => omit_skill_bodies(
                serde_json::to_value(detail).unwrap_or(json!({ "ref": reference })),
            ),
            Err(_) => json!({ "ref": reference, "empty": true, "reason": "引用已失效" }),
        };
    } else if let Some(preset_id) = selection.get("presetId").and_then(Value::as_str) {
        if let Some(domain) = &view.domain {
            opened = match crate::commands::preset::preset_get(
                runtime,
                domain.clone(),
                preset_id.to_string(),
            ) {
                Ok(preset) => {
                    serde_json::to_value(preset).unwrap_or(json!({ "presetId": preset_id }))
                }
                Err(err) => json!({ "presetId": preset_id, "error": err }),
            };
        }
    } else if let Some(agent) = selection
        .get("agent")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            page.strip_prefix("agent:")
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
    {
        opened = match crate::commands::agent::agent_local_skills(runtime, agent.clone()) {
            Ok(list) => json!({
                "target": list.agent,
                "skillsDir": list.skills_dir,
                "managed": list.managed,
                "total": list.total,
                "skills": list.skills.iter().map(|skill| json!({
                    "name": skill.name,
                    "status": skill.status,
                    "managed": skill.managed,
                    "librarySkill": skill.library_skill,
                    "path": skill.path,
                })).collect::<Vec<_>>(),
            }),
            Err(err) => json!({ "target": agent, "error": err }),
        };
    } else if let Some(tool) = selection.get("tool").and_then(Value::as_str) {
        opened = match crate::commands::tool::tool_get(runtime, tool.to_string()) {
            Ok(entry) => {
                let mut value = serde_json::to_value(entry).unwrap_or(json!({ "slug": tool }));
                if let Some(obj) = value.as_object_mut() {
                    obj.remove("guide");
                    obj.remove("intro");
                    obj.insert("bodyOmitted".into(), json!(true));
                }
                value
            }
            Err(err) => json!({ "slug": tool, "error": err }),
        };
    }
    Ok(json!({
        "page": view.page,
        "domain": view.domain,
        "selection": view.selection,
        "opened": opened,
        "pageObjects": page_objects(runtime, &page, view.domain.as_deref()),
    }))
}

/// 发消息时从 core 组装 global（摘要+计数+引用）与 view 详情。不含文件正文。
pub fn assemble_snapshot(runtime: &HostRuntime, view: &ViewRef) -> Result<Value, String> {
    let (
        skills,
        translation_stale,
        deployments,
        paused,
        pending_updates,
        mut failed_checks,
        presets,
        hermes,
        tool_summary,
        agents,
    ) = {
        let ctx = runtime.get()?;
        let Some(vault) = ctx.vault_opt() else {
            drop(ctx);
            return Ok(json!({
                "global": empty_global("尚未接入 SkillManagerVault"),
                "view": assemble_view(runtime, view).unwrap_or_else(|err| json!({
                    "empty": true,
                    "reason": err,
                    "page": view.page,
                    "domain": view.domain,
                    "selection": view.selection,
                })),
            }));
        };
        let vault_root = vault.root().to_path_buf();
        let index = skill_manager_core::vault::index::SkillIndex::build(&vault_root)
            .map_err(|err| err.to_string())?;
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
                    "comment": summary.comment,
                    "translation_stale": summary.translation_stale,
                    "source": summary.source.is_some(),
                })
            })
            .collect();
        let translation_stale: Vec<_> = skills
            .iter()
            .filter(|s| {
                s.get("translation_stale")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        let deployment_rows = ctx.state().deployments().map_err(|err| err.to_string())?;
        let deployments: Vec<Value> = deployment_rows
            .iter()
            .map(|row| {
                json!({
                    "skill": format!("{}/{}", row.domain, row.name),
                    "target": row.target,
                    "targetPath": row.target_path,
                    "status": row.status.as_str(),
                })
            })
            .collect();
        let paused: Vec<Value> = deployment_rows
            .iter()
            .filter(|row| {
                matches!(
                    row.status,
                    skill_manager_core::state::DeploymentStatus::PausedLocalChange
                )
            })
            .map(|row| {
                json!({
                    "ref": format!("{}/{}", row.domain, row.name),
                    "target": row.target,
                    "targetPath": row.target_path,
                    "status": "paused_local_change",
                    "reason": "受管副本被外部修改，覆盖已暂停",
                })
            })
            .collect();
        let checks = ctx.state().upstream_checks().unwrap_or_default();
        let pending_updates: Vec<Value> = checks
            .iter()
            .filter(|row| row.result.as_deref() == Some("update_available"))
            .map(|row| {
                json!({
                    "ref": format!("{}/{}", row.domain, row.name),
                    "repo": row.repo,
                    "remoteVersion": row.remote_version,
                })
            })
            .collect();
        let failed_checks: Vec<Value> = checks
            .iter()
            .filter(|row| row.result.as_deref() == Some("error"))
            .map(|row| {
                json!({
                    "ref": format!("{}/{}", row.domain, row.name),
                    "repo": row.repo,
                    "result": "error",
                })
            })
            .collect();
        let presets = skill_manager_core::preset::list(&vault_root, None)
            .unwrap_or_default()
            .into_iter()
            .map(|preset| {
                json!({
                    "ref": format!("{}/{}", preset.domain, preset.id),
                    "name": preset.name,
                    "memberCount": preset.members.len(),
                })
            })
            .collect::<Vec<Value>>();
        let hermes = match ctx.target_resolver() {
            Ok(resolver) => {
                skill_manager_core::hermes::HermesService::from_context(&ctx, &resolver)
                    .ok()
                    .and_then(|svc| svc.profiles().ok())
                    .map(|profiles| {
                        json!({
                            "count": profiles.len(),
                            "refs": profiles.iter().map(|p| p.name.clone()).collect::<Vec<_>>(),
                        })
                    })
                    .unwrap_or(json!({ "count": 0, "refs": [] }))
            }
            Err(_) => json!({ "count": 0, "refs": [] }),
        };
        let tools = skill_manager_core::toolcenter::catalog::list(
            &vault_root,
            ctx.state(),
            &skill_manager_core::toolcenter::ToolQuery::default(),
        )
        .unwrap_or_default();
        let tool_summary = json!({
            "count": tools.len(),
            "refs": tools.iter().take(50).map(|t| t.slug.clone()).collect::<Vec<_>>(),
            "failed": tools.iter().filter(|t| t.check.status == "failed").map(|t| json!({
                "slug": t.slug,
                "output": t.check.output,
            })).collect::<Vec<_>>(),
        });
        let agents = match ctx.home() {
            Ok(home) => skill_manager_core::deploy::AgentConfigService::new(
                home,
                ctx.catalog(),
                ctx.state(),
            )
            .list()
            .unwrap_or_default()
            .into_iter()
            .map(|agent| {
                json!({
                    "key": agent.key,
                    "displayName": agent.display_name,
                    "detected": agent.detected,
                    "enabled": agent.enabled,
                    "skillsDir": agent.skills_dir,
                })
            })
            .collect::<Vec<Value>>(),
            Err(_) => Vec::new(),
        };
        (
            skills,
            translation_stale,
            deployments,
            paused,
            pending_updates,
            failed_checks,
            presets,
            hermes,
            tool_summary,
            agents,
        )
    };

    let schedule = runtime.current_schedule().ok().flatten();
    let new_candidates: Vec<Value> = schedule
        .as_ref()
        .and_then(|report| report.upstream_report.as_ref())
        .map(|report| {
            report
                .candidates
                .iter()
                .map(|candidate| {
                    json!({
                        "repo": candidate.repo,
                        "domain": candidate.domain.as_str(),
                        "subpath": candidate.subpath,
                        "name": candidate.name,
                        "description": candidate.description,
                        "isNew": candidate.is_new,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    if let Some(report) = schedule
        .as_ref()
        .and_then(|row| row.upstream_report.as_ref())
    {
        for repo in &report.repos {
            if let Some(error) = &repo.error {
                failed_checks.push(json!({
                    "repo": repo.repo,
                    "error": error,
                }));
            }
        }
    }
    let mut last_failures = last_command_failures(runtime);
    if let Some(report) = &schedule {
        for (action, outcome) in [
            ("upstream", &report.upstream),
            ("pull", &report.pull),
            ("tool_check", &report.tool_check),
        ] {
            if outcome.status == "error" {
                if let Some(detail) = &outcome.detail {
                    last_failures.insert(
                        0,
                        json!({
                            "at": null,
                            "action": action,
                            "message": detail,
                        }),
                    );
                }
            }
        }
    }
    let pull_conflicts = match crate::commands::sync::sync_conflicts(runtime) {
        Ok(view) => serde_json::to_value(view).unwrap_or(json!({
            "hasConflict": false,
            "paths": [],
            "gitStatus": [],
        })),
        Err(err) => json!({
            "hasConflict": false,
            "paths": [],
            "gitStatus": [],
            "message": err,
        }),
    };
    let view_detail = assemble_view(runtime, view)?;

    Ok(json!({
        "global": {
            "skills": skills,
            "skillCount": skills.len(),
            "translationStale": translation_stale,
            "translationStaleCount": translation_stale.len(),
            "pendingUpdates": pending_updates,
            "pendingUpdateCount": pending_updates.len(),
            "newCandidates": new_candidates,
            "newCandidateCount": new_candidates.len(),
            "failedChecks": failed_checks,
            "failedCheckCount": failed_checks.len(),
            "deployments": deployments,
            "pausedDeployments": paused,
            "pausedCount": paused.len(),
            "pullConflicts": pull_conflicts,
            "lastCommandFailures": last_failures,
            "presets": presets,
            "presetCount": presets.len(),
            "hermes": hermes,
            "tools": tool_summary,
            "agents": agents,
            "agentCount": agents.len(),
        },
        "view": view_detail,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HostRuntime;

    #[test]
    fn snapshot_has_global_summary_fields_without_bodies() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = skill_manager_core::AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        let snap = assemble_snapshot(&runtime, &ViewRef::default()).unwrap();
        let global = snap.get("global").expect("global layer");
        for key in [
            "skillCount",
            "pendingUpdates",
            "newCandidates",
            "pausedDeployments",
            "pullConflicts",
            "lastCommandFailures",
            "presets",
            "hermes",
            "tools",
            "agents",
            "translationStale",
        ] {
            assert!(global.get(key).is_some(), "missing {key}");
        }
        assert!(global.get("skills").is_some());
        let view = snap.get("view").expect("view layer");
        assert!(
            view.get("pageObjects").is_some(),
            "view must fill current page objects"
        );
    }

    #[test]
    fn snapshot_view_fills_library_page_without_skill_ref() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = skill_manager_core::AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        let snap = assemble_snapshot(
            &runtime,
            &ViewRef {
                page: Some("library-code".into()),
                domain: Some("code".into()),
                selection: None,
            },
        )
        .unwrap();
        let view = snap.get("view").expect("view layer");
        assert_eq!(
            view.get("page").and_then(Value::as_str),
            Some("library-code")
        );
        let objects = view.get("pageObjects").expect("page objects");
        assert!(
            objects.get("kind").is_some() || objects.get("error").is_some(),
            "page objects must not be only the page name: {objects}"
        );
        assert!(snap["global"]["newCandidates"].is_array());
        assert!(snap["global"]["pullConflicts"].is_object());
        assert!(snap["global"]["lastCommandFailures"].is_array());
    }

    #[test]
    fn prior_transcript_skips_current_task() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = skill_manager_core::AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        remember_session(
            &runtime,
            AgentSession {
                session_id: "s1".into(),
                view: ViewRef::default(),
                snapshot: json!({}),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )
        .unwrap();
        push_message(
            &runtime,
            "s1",
            AgentMessage {
                role: "user".into(),
                text: "先改注释".into(),
                task_id: Some("t1".into()),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )
        .unwrap();
        push_message(
            &runtime,
            "s1",
            AgentMessage {
                role: "assistant".into(),
                text: "已改".into(),
                task_id: Some("t1".into()),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )
        .unwrap();
        push_message(
            &runtime,
            "s1",
            AgentMessage {
                role: "user".into(),
                text: "再口语一点".into(),
                task_id: Some("t2".into()),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )
        .unwrap();
        let text = prior_transcript(&runtime, "s1", "t2");
        assert!(text.contains("先改注释"));
        assert!(text.contains("已改"));
        assert!(!text.contains("再口语一点"));
    }
}
