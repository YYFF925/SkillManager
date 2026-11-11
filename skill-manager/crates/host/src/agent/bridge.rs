//! 探测并拉起已锁定的 Agent CLI（OpenCode → Codex → Gemini）。全部失败则对话不可用。

use std::path::PathBuf;
use std::process::Command;

use serde_json::{json, Value};
use uuid::Uuid;

use super::context::{self, AgentMessage, AgentSession};
use super::tools;
use crate::HostRuntime;

#[derive(Clone, Debug)]
pub struct DetectedCli {
    pub cli_id: String,
    pub binary_path: PathBuf,
    pub inject: String,
}

const CANDIDATES: &[(&str, &[&str])] = &[
    ("opencode", &["opencode", "opencode-ai"]),
    ("codex", &["codex", "codex-cli"]),
    ("gemini", &["gemini", "gemini-cli"]),
];

pub fn detect_and_lock() -> Result<DetectedCli, String> {
    for (id, bins) in CANDIDATES {
        for bin in *bins {
            if let Some(path) = which(bin) {
                return Ok(DetectedCli {
                    cli_id: (*id).to_string(),
                    binary_path: path,
                    inject: "skills-dir-or-init-text".to_string(),
                });
            }
        }
    }
    Err("未检测到可用的管理 CLI（已按 OpenCode → Codex CLI → Gemini CLI 探测）。对话不可用，不会假装已经改库。".to_string())
}

#[cfg(not(windows))]
fn which(bin: &str) -> Option<PathBuf> {
    let output = Command::new("which").arg(bin).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

#[cfg(windows)]
fn which(bin: &str) -> Option<PathBuf> {
    let output = Command::new("where").arg(bin).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

pub fn post_message(runtime: &HostRuntime, body: Value) -> Result<Value, String> {
    let session_id = body
        .get("sessionId")
        .or_else(|| body.get("session_id"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if session_id.is_empty() {
        return Err("缺少 sessionId".to_string());
    }
    let text = body
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| "缺少 text".to_string())?
        .to_string();
    let view = context::view_for(runtime, &session_id);
    let snapshot = context::assemble_snapshot(runtime, &view)?;
    let task_id = Uuid::new_v4().to_string();
    let now = now_rfc3339();
    context::remember_session(
        runtime,
        AgentSession {
            session_id: session_id.clone(),
            view: view.clone(),
            snapshot: snapshot.clone(),
            created_at: now.clone(),
        },
    )?;
    context::push_message(
        runtime,
        &session_id,
        AgentMessage {
            role: "user".to_string(),
            text: text.clone(),
            task_id: Some(task_id.clone()),
            created_at: now,
        },
    )?;

    let detected = match detect_and_lock() {
        Ok(cli) => cli,
        Err(err) => {
            context::push_message(
                runtime,
                &session_id,
                AgentMessage {
                    role: "assistant".to_string(),
                    text: err.clone(),
                    task_id: Some(task_id.clone()),
                    created_at: now_rfc3339(),
                },
            )?;
            return Ok(json!({
                "taskId": task_id,
                "status": "failed",
                "message": err,
                "snapshot": snapshot,
            }));
        }
    };

    let reply = format!(
        "已锁定管理 CLI {}（{}）。已附带当前 Host 的全局摘要与当前界面详情。请用工具读取细节；删除/覆盖须经确认计划。",
        detected.cli_id,
        detected.binary_path.display()
    );
    let _ = tools::builtin_skill_dir();
    context::push_message(
        runtime,
        &session_id,
        AgentMessage {
            role: "assistant".to_string(),
            text: reply.clone(),
            task_id: Some(task_id.clone()),
            created_at: now_rfc3339(),
        },
    )?;
    Ok(json!({
        "taskId": task_id,
        "status": "running",
        "cli": detected.cli_id,
        "message": reply,
        "snapshot": snapshot,
        "userText": text,
    }))
}

pub fn cancel_task(runtime: &HostRuntime, task_id: &str, _body: Value) -> Result<Value, String> {
    context::mark_task_cancelled(runtime, task_id)?;
    Ok(json!({
        "taskId": task_id,
        "status": "cancelled",
        "message": "任务已取消，不会标成成功，也不会继续写库。"
    }))
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_order_is_opencode_then_codex_then_gemini() {
        let ids: Vec<_> = CANDIDATES.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, ["opencode", "codex", "gemini"]);
    }
}
