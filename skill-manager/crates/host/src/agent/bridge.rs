//! 探测、必要时安装并拉起已锁定的 Agent CLI（OpenCode → Codex → Gemini）。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::Ordering;

use serde_json::{Value, json};
use uuid::Uuid;

use super::acp::{self, DetectedCli};
use super::context::{self, AgentMessage, AgentSession};
use super::tools;
use crate::HostRuntime;

const CANDIDATES: &[(&str, &[&str], &str)] = &[
    ("opencode", &["opencode", "opencode-ai"], "opencode-ai"),
    ("codex", &["codex", "codex-cli"], "@openai/codex"),
    ("gemini", &["gemini", "gemini-cli"], "@google/gemini-cli"),
];

pub fn detect_and_lock() -> Result<DetectedCli, String> {
    detect_and_lock_in(None)
}

pub fn detect_and_lock_in(runtime: Option<&HostRuntime>) -> Result<DetectedCli, String> {
    if let Some(runtime) = runtime {
        if let Some(stored) = load_runtime(runtime) {
            if stored.binary_path.is_file() {
                return Ok(stored);
            }
        }
    }
    for (id, bins, _pkg) in CANDIDATES {
        for bin in *bins {
            if let Some(path) = which(bin) {
                let cli = DetectedCli {
                    cli_id: (*id).to_string(),
                    binary_path: path,
                    installed_by_app: false,
                };
                if let Some(runtime) = runtime {
                    let _ = persist_runtime(runtime, &cli);
                }
                return Ok(cli);
            }
        }
    }
    if let Some(runtime) = runtime {
        if let Some(cli) = install_first_available(runtime) {
            return Ok(cli);
        }
    }
    Err("未检测到可用的管理 CLI（已按 OpenCode → Codex CLI → Gemini CLI 探测，并尝试在本机目录安装）。对话不可用，不会假装已经改库。".to_string())
}

fn load_runtime(runtime: &HostRuntime) -> Option<DetectedCli> {
    let ctx = runtime.get().ok()?;
    ctx.state()
        .connection()
        .query_row(
            "SELECT cli_id, binary_path, installed_by_app FROM agent_runtime WHERE id = 1",
            [],
            |row| {
                Ok(DetectedCli {
                    cli_id: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    binary_path: PathBuf::from(
                        row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    ),
                    installed_by_app: row.get::<_, i64>(2).unwrap_or(0) != 0,
                })
            },
        )
        .ok()
        .filter(|cli| !cli.cli_id.is_empty())
}

fn persist_runtime(runtime: &HostRuntime, cli: &DetectedCli) -> Result<(), String> {
    let ctx = runtime.get()?;
    ctx.state()
        .connection()
        .execute(
            "INSERT INTO agent_runtime (id, cli_id, binary_path, detected_at, installed_by_app)
             VALUES (1, ?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
               cli_id = excluded.cli_id,
               binary_path = excluded.binary_path,
               detected_at = excluded.detected_at,
               installed_by_app = excluded.installed_by_app",
            rusqlite::params![
                cli.cli_id,
                cli.binary_path.display().to_string(),
                chrono::Utc::now().to_rfc3339(),
                if cli.installed_by_app { 1 } else { 0 },
            ],
        )
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn install_dir(runtime: &HostRuntime) -> PathBuf {
    runtime.state_dir().join("agent-runtime")
}

fn install_first_available(runtime: &HostRuntime) -> Option<DetectedCli> {
    let root = install_dir(runtime);
    for (id, bins, package) in CANDIDATES {
        match install_package(&root.join(id), package, bins[0]) {
            Ok(path) => {
                let cli = DetectedCli {
                    cli_id: (*id).to_string(),
                    binary_path: path,
                    installed_by_app: true,
                };
                let _ = persist_runtime(runtime, &cli);
                return Some(cli);
            }
            Err(_) => continue,
        }
    }
    None
}

fn install_package(dest: &Path, package: &str, bin_name: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dest).map_err(|err| err.to_string())?;
    let status = Command::new("npm")
        .args(["install", "--omit=dev", "--prefix"])
        .arg(dest)
        .arg(package)
        .status()
        .map_err(|err| format!("无法在软件内安装 {package}：{err}"))?;
    if !status.success() {
        return Err(format!("npm install {package} 失败"));
    }
    let unix = dest.join("node_modules").join(".bin").join(bin_name);
    let windows = dest
        .join("node_modules")
        .join(".bin")
        .join(format!("{bin_name}.cmd"));
    if unix.is_file() {
        Ok(unix)
    } else if windows.is_file() {
        Ok(windows)
    } else {
        Err(format!("安装后未找到 {bin_name}"))
    }
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

fn builtin_skill_text() -> String {
    let dir = tools::builtin_skill_dir();
    let path = dir.join("SKILL.md");
    std::fs::read_to_string(path).unwrap_or_else(|_| {
        "你是 Skill Manager 当前 Host 上的内置管理 Agent。两层上下文已注入；细节用工具；删除/覆盖走确认计划。本说明不入库。".to_string()
    })
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

    let detected = match detect_and_lock_in(Some(runtime)) {
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

    let skill_dir = tools::builtin_skill_dir();
    let skill_text = builtin_skill_text();
    let history = context::prior_transcript(runtime, &session_id, &task_id);
    let cancel = acp::register_task(&task_id);
    if let Err(err) = acp::ensure_session(
        &detected,
        &skill_dir,
        &skill_text,
        runtime,
        &session_id,
        &cancel,
    ) {
        acp::forget_task(&task_id);
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
            "cli": detected.cli_id,
        }));
    }
    let runtime_bg = runtime.clone();
    let session_bg = session_id.clone();
    let task_bg = task_id.clone();
    let cancel_bg = cancel.clone();
    let snapshot_bg = snapshot.clone();
    let detected_bg = detected.clone();
    let skill_dir_bg = skill_dir.clone();
    let user_text_bg = text.clone();
    std::thread::spawn(move || {
        let result = acp::run_turn(
            &detected_bg,
            &skill_dir_bg,
            &skill_text,
            &runtime_bg,
            &session_bg,
            &task_bg,
            &snapshot_bg,
            &history,
            &user_text_bg,
            cancel_bg.clone(),
        );
        let (role, text) = match result {
            Ok(text) => ("assistant", text),
            Err(err) if err == "cancelled" || cancel_bg.load(Ordering::SeqCst) => (
                "system",
                "任务已取消，不会标成成功，也不会继续写库。".to_string(),
            ),
            Err(err) => ("assistant", err),
        };
        let _ = context::push_message(
            &runtime_bg,
            &session_bg,
            AgentMessage {
                role: role.to_string(),
                text,
                task_id: Some(task_bg),
                created_at: now_rfc3339(),
            },
        );
    });

    Ok(json!({
        "taskId": task_id,
        "status": "running",
        "cli": detected.cli_id,
        "injectedSkillDir": skill_dir.is_dir(),
        "message": format!("已锁定管理 CLI {}，正在执行。", detected.cli_id),
        "snapshot": snapshot,
        "userText": text,
    }))
}

pub fn cancel_task(runtime: &HostRuntime, task_id: &str, _body: Value) -> Result<Value, String> {
    acp::cancel_live(task_id);
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
        let ids: Vec<_> = CANDIDATES.iter().map(|(id, _, _)| *id).collect();
        assert_eq!(ids, ["opencode", "codex", "gemini"]);
    }

    #[test]
    fn missing_cli_is_failed_not_running() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = skill_manager_core::AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        let result = post_message(
            &runtime,
            json!({ "sessionId": "s1", "text": "完善中文注释" }),
        )
        .unwrap();
        // 本测试环境通常没有锁定 CLI；不得把任务标成 running。
        if which("opencode").is_none() && which("codex").is_none() && which("gemini").is_none() {
            assert_eq!(result["status"], "failed");
        }
    }

    #[test]
    fn builtin_skill_dir_exists_in_repo() {
        let dir = tools::builtin_skill_dir();
        assert!(
            dir.join("SKILL.md").is_file(),
            "builtin skill must exist for injection: {}",
            dir.display()
        );
    }
}
