//! 拉起用户已登录的任意 ACP 兼容 CLI；模型由该 CLI 决定。
//! D9 探测清单只用于找引导宿主，不是许可名单。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::Ordering;

use serde_json::{json, Value};
use uuid::Uuid;

use super::acp::{self, DetectedCli};
use super::context::{self, AgentMessage, AgentSession};
use super::tools;
use crate::HostRuntime;

/// 本机 PATH 上常见的 ACP 宿主示例，**不是**唯一选项。原生 `codex` 无 ACP，故不列入。
const KNOWN_ACP_HOSTS: &[(&str, &[&str])] = &[
    ("opencode", &["opencode", "opencode-ai"]),
    ("gemini", &["gemini", "gemini-cli"]),
    ("codex-acp", &["codex-acp"]),
    ("claude-acp", &["claude-agent-acp", "claude-acp"]),
    ("cursor-agent", &["cursor-agent"]),
];

/// D9 第一家通过 ACP 清单者，仅在本机没有任何兼容 CLI 时作为引导安装。
const BOOTSTRAP_ID: &str = "opencode";
const BOOTSTRAP_BIN: &str = "opencode";
const BOOTSTRAP_PACKAGE: &str = "opencode-ai";

const MISSING_CLI: &str = "未检测到已登录或已安装的 ACP 管理 CLI。可登记任意兼容命令行：skill-manager host agent-cli --bin <路径> [--args acp]，或设置 SKILL_MANAGER_AGENT_CLI。模型由该 CLI 决定，不限于 OpenCode / Codex / Gemini。没有可用 CLI 时才会安装引导宿主 OpenCode。对话不可用，不会假装已经改库。";

pub fn detect_and_lock() -> Result<DetectedCli, String> {
    detect_and_lock_in(None, true)
}

pub fn detect_and_lock_in(
    runtime: Option<&HostRuntime>,
    install: bool,
) -> Result<DetectedCli, String> {
    if let Some(cli) = from_env() {
        if let Some(runtime) = runtime {
            let _ = persist_runtime(runtime, &cli);
        }
        return Ok(cli);
    }
    if let Some(runtime) = runtime {
        if let Some(stored) = load_runtime(runtime) {
            if stored.binary_path.is_file() {
                return Ok(stored);
            }
        }
    }
    if let Some(cli) = detect_known_on_path() {
        if let Some(runtime) = runtime {
            let _ = persist_runtime(runtime, &cli);
        }
        return Ok(cli);
    }
    if install {
        if let Some(runtime) = runtime {
            if let Some(cli) = install_bootstrap(runtime) {
                return Ok(cli);
            }
        }
    }
    Err(MISSING_CLI.to_string())
}

pub fn runtime_view(runtime: &HostRuntime) -> Value {
    match detect_and_lock_in(Some(runtime), false) {
        Ok(cli) => json!({
            "cliId": cli.cli_id,
            "binaryPath": cli.binary_path.display().to_string(),
            "acpArgs": cli.acp_args,
            "userSelected": cli.user_selected,
            "installedByApp": cli.installed_by_app,
            "bootstrap": BOOTSTRAP_ID,
            "exclusiveVendors": false,
        }),
        Err(err) => json!({
            "cliId": Value::Null,
            "error": err,
            "bootstrap": BOOTSTRAP_ID,
            "exclusiveVendors": false,
        }),
    }
}

pub fn set_user_cli(
    runtime: &HostRuntime,
    bin: &str,
    acp_args: Vec<String>,
    cli_id: Option<&str>,
) -> Result<DetectedCli, String> {
    let path = resolve_bin(bin).ok_or_else(|| format!("找不到可执行文件：{bin}"))?;
    ensure_executable(&path)?;
    let id = cli_id
        .map(str::to_string)
        .unwrap_or_else(|| infer_cli_id(&path));
    let mut cli = DetectedCli {
        cli_id: id,
        binary_path: path,
        installed_by_app: false,
        acp_args,
        user_selected: true,
    };
    let skill_dir = tools::prepare_opencode_config(runtime.state_dir())
        .unwrap_or_else(|_| tools::builtin_skill_dir());
    cli.acp_args = acp::probe_acp(&cli, &skill_dir)?;
    persist_runtime(runtime, &cli)?;
    Ok(cli)
}

pub fn clear_user_cli(runtime: &HostRuntime) -> Result<(), String> {
    let ctx = runtime.get()?;
    ctx.state()
        .connection()
        .execute("DELETE FROM agent_runtime WHERE id = 1", [])
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn from_env() -> Option<DetectedCli> {
    let raw = std::env::var("SKILL_MANAGER_AGENT_CLI").ok()?;
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let path = resolve_bin(raw)?;
    let acp_args = std::env::var("SKILL_MANAGER_AGENT_ACP_ARGS")
        .ok()
        .map(|s| {
            s.split_whitespace()
                .filter(|p| !p.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Some(DetectedCli {
        cli_id: infer_cli_id(&path),
        binary_path: path,
        installed_by_app: false,
        acp_args,
        user_selected: true,
    })
}

fn detect_known_on_path() -> Option<DetectedCli> {
    for (id, bins) in KNOWN_ACP_HOSTS {
        for bin in *bins {
            if let Some(path) = which(bin) {
                return Some(DetectedCli {
                    cli_id: (*id).to_string(),
                    binary_path: path,
                    installed_by_app: false,
                    acp_args: Vec::new(),
                    user_selected: false,
                });
            }
        }
    }
    None
}

fn infer_cli_id(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("custom")
        .trim_end_matches(".exe")
        .trim_end_matches(".cmd")
        .to_string()
}

fn resolve_bin(bin: &str) -> Option<PathBuf> {
    let as_path = PathBuf::from(bin);
    if as_path.is_file() {
        return Some(as_path);
    }
    which(bin)
}

fn ensure_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path)
            .map_err(|err| format!("无法读取 {path}：{err}", path = path.display()))?
            .permissions()
            .mode();
        if mode & 0o111 == 0 {
            return Err(format!(
                "{} 存在但不可执行，不能登记为 ACP 宿主。",
                path.display()
            ));
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn load_runtime(runtime: &HostRuntime) -> Option<DetectedCli> {
    let ctx = runtime.get().ok()?;
    ctx.state()
        .connection()
        .query_row(
            "SELECT cli_id, binary_path, installed_by_app, acp_args, user_selected
             FROM agent_runtime WHERE id = 1",
            [],
            |row| {
                let acp_args = row
                    .get::<_, Option<String>>(3)?
                    .and_then(|raw| serde_json::from_str(&raw).ok())
                    .unwrap_or_default();
                Ok(DetectedCli {
                    cli_id: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    binary_path: PathBuf::from(
                        row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    ),
                    installed_by_app: row.get::<_, i64>(2).unwrap_or(0) != 0,
                    acp_args,
                    user_selected: row.get::<_, i64>(4).unwrap_or(0) != 0,
                })
            },
        )
        .ok()
        .filter(|cli| !cli.cli_id.is_empty() && !cli.binary_path.as_os_str().is_empty())
}

fn persist_runtime(runtime: &HostRuntime, cli: &DetectedCli) -> Result<(), String> {
    let ctx = runtime.get()?;
    let args = serde_json::to_string(&cli.acp_args).unwrap_or_else(|_| "[]".into());
    ctx.state()
        .connection()
        .execute(
            "INSERT INTO agent_runtime (id, cli_id, binary_path, detected_at, installed_by_app, acp_args, user_selected)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
               cli_id = excluded.cli_id,
               binary_path = excluded.binary_path,
               detected_at = excluded.detected_at,
               installed_by_app = excluded.installed_by_app,
               acp_args = excluded.acp_args,
               user_selected = excluded.user_selected",
            rusqlite::params![
                cli.cli_id,
                cli.binary_path.display().to_string(),
                chrono::Utc::now().to_rfc3339(),
                if cli.installed_by_app { 1 } else { 0 },
                args,
                if cli.user_selected { 1 } else { 0 },
            ],
        )
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn install_dir(runtime: &HostRuntime) -> PathBuf {
    runtime.state_dir().join("agent-runtime")
}

fn install_bootstrap(runtime: &HostRuntime) -> Option<DetectedCli> {
    let dest = install_dir(runtime).join(BOOTSTRAP_ID);
    let path = install_package(&dest, BOOTSTRAP_PACKAGE, BOOTSTRAP_BIN).ok()?;
    let cli = DetectedCli {
        cli_id: BOOTSTRAP_ID.to_string(),
        binary_path: path,
        installed_by_app: true,
        acp_args: vec!["acp".into()],
        user_selected: false,
    };
    let _ = persist_runtime(runtime, &cli);
    Some(cli)
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

    let detected = match detect_and_lock_in(Some(runtime), true) {
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

    let skill_dir = tools::prepare_opencode_config(runtime.state_dir())
        .unwrap_or_else(|_| tools::builtin_skill_dir());
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
        "message": format!("已使用管理 CLI {}，正在执行。模型由该 CLI 决定。", detected.cli_id),
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
    fn known_hosts_are_examples_not_exclusive_three() {
        let ids: Vec<_> = KNOWN_ACP_HOSTS.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&"opencode"));
        assert!(!ids.contains(&"codex"), "native Codex has no ACP");
        assert_eq!(BOOTSTRAP_ID, "opencode");
        assert!(ids.len() > 3 || ids.iter().any(|id| *id == "cursor-agent" || *id == "claude-acp"));
    }

    #[test]
    fn missing_cli_without_install_explains_registration() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = skill_manager_core::AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        if detect_known_on_path().is_none() && std::env::var("SKILL_MANAGER_AGENT_CLI").is_err() {
            let err = detect_and_lock_in(Some(&runtime), false).unwrap_err();
            assert!(err.contains("SKILL_MANAGER_AGENT_CLI") || err.contains("agent-cli"));
            assert!(err.contains("不限于"));
        }
    }

    #[test]
    fn post_message_does_not_claim_success_when_cli_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = skill_manager_core::AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        let result = post_message(
            &runtime,
            json!({ "sessionId": "s1", "text": "完善中文注释" }),
        )
        .unwrap();
        assert_ne!(
            result["status"], "success",
            "未完成任务不得标成功；实际 status={:?} message={:?}",
            result["status"],
            result["message"]
        );
    }

    fn chmod_exec(path: &std::path::Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perm = std::fs::metadata(path).unwrap().permissions();
            perm.set_mode(0o755);
            std::fs::set_permissions(path, perm).unwrap();
        }
    }

    fn write_fake_acp(dir: &std::path::Path) -> std::path::PathBuf {
        let fake = dir.join("my-acp-agent");
        std::fs::write(
            &fake,
            br#"#!/usr/bin/env python3
import json, sys
args = sys.argv[1:]
if args == ["acp"]:
    sys.stderr.write("not acp\n")
    sys.exit(2)
if args != ["--acp"]:
    sys.exit(2)
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        msg = json.loads(line)
    except Exception:
        continue
    if msg.get("method") == "initialize":
        print(json.dumps({"jsonrpc": "2.0", "id": msg.get("id"), "result": {"protocolVersion": 1}}), flush=True)
"#,
        )
        .unwrap();
        chmod_exec(&fake);
        fake
    }

    #[test]
    fn user_selected_cli_requires_acp_handshake() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = skill_manager_core::AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        let fake = dir.path().join("not-acp");
        std::fs::write(&fake, b"#!/bin/sh\nexit 0\n").unwrap();
        chmod_exec(&fake);
        let err = set_user_cli(
            &runtime,
            fake.to_str().unwrap(),
            Vec::new(),
            Some("custom-host"),
        )
        .unwrap_err();
        assert!(
            err.contains("ACP") || err.contains("握手"),
            "登记不得只看文件存在：{err}"
        );
        assert!(load_runtime(&runtime).is_none());
    }

    #[test]
    fn user_selected_cli_is_persisted_after_handshake() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = skill_manager_core::AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        let fake = write_fake_acp(dir.path());
        let cli = set_user_cli(
            &runtime,
            fake.to_str().unwrap(),
            Vec::new(),
            Some("custom-host"),
        )
        .unwrap();
        assert_eq!(cli.cli_id, "custom-host");
        assert!(cli.user_selected);
        assert_eq!(cli.acp_args, vec!["--acp"]);
        let loaded = load_runtime(&runtime).unwrap();
        assert_eq!(loaded.cli_id, "custom-host");
        assert_eq!(loaded.acp_args, vec!["--acp"]);
        assert!(loaded.user_selected);
        let view = runtime_view(&runtime);
        assert_eq!(view["exclusiveVendors"], false);
        assert_eq!(view["cliId"], "custom-host");
    }

    #[test]
    fn builtin_skill_dir_exists_in_repo() {
        let dir = tools::builtin_skill_dir();
        assert!(
            dir.join("SKILL.md").is_file(),
            "builtin skill must exist for injection: {}",
            dir.display()
        );
        let tmp = tempfile::tempdir().unwrap();
        let config = tools::prepare_opencode_config(tmp.path()).unwrap();
        assert!(config
            .join("skills/skill-manager-builtin-agent/SKILL.md")
            .is_file());
    }

    #[test]
    fn default_args_for_unlisted_cli_try_acp() {
        let sets = acp::default_acp_arg_sets("my-vendor");
        assert_eq!(sets, vec![&["acp"][..], &["--acp"][..], &[][..]]);
        assert_eq!(acp::default_acp_arg_sets("opencode"), sets);
    }
}
