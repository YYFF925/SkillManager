//! 默认管理 CLI 钉死为 OpenCode；用户仍可覆盖为任意 ACP 兼容 CLI。
//! 不扫描一长串厂商命令行。安装与登录是产品流程，不是测试跳过条件。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::Ordering;

use serde_json::{json, Value};
use uuid::Uuid;

use super::acp::{self, DetectedCli};
use super::context::{self, AgentMessage, AgentSession};
use super::tools;
use crate::HostRuntime;

/// D9 已跑通 ACP 的引导宿主。产品默认钉这一家，不再探测厂商清单。
const DEFAULT_CLI_ID: &str = "opencode";
const DEFAULT_CLI_NAME: &str = "OpenCode";
const DEFAULT_BIN: &str = "opencode";
const DEFAULT_PACKAGE: &str = "opencode-ai";
const DEFAULT_ACP_ARGS: &[&str] = &["acp"];
const DEFAULT_LOGIN_ARGS: &[&str] = &["auth", "login"];

const INSTALL_HINT: &str = "尚未安装默认管理 CLI OpenCode。请在对话面板点「安装 OpenCode」，或执行 host agent-cli --install。装到本机 Host 目录，不打进 Skill Manager 安装包。对话不可用，不会假装已经改库。";
const LOGIN_HINT: &str = "在本机完成 OpenCode 登录（浏览器授权）。密钥留在该 CLI，不会写入 Host、Vault 或管理站。";

pub fn detect_and_lock() -> Result<DetectedCli, String> {
    detect_and_lock_in(None)
}

pub fn detect_and_lock_in(runtime: Option<&HostRuntime>) -> Result<DetectedCli, String> {
    let resolved = resolve_runtime(runtime);
    resolved.cli.ok_or(resolved.message)
}

pub fn runtime_view(runtime: &HostRuntime) -> Value {
    runtime_status_json(&resolve_runtime(Some(runtime)))
}

pub fn install_default_cli(runtime: &HostRuntime) -> Result<Value, String> {
    let dest = install_dir(runtime).join(DEFAULT_CLI_ID);
    let path = install_package(&dest, DEFAULT_PACKAGE, DEFAULT_BIN)?;
    let cli = default_detected(path, true, false);
    persist_runtime(runtime, &cli)?;
    Ok(runtime_status_json(&ready_status(cli)))
}

pub fn launch_login(runtime: &HostRuntime) -> Result<Value, String> {
    let resolved = resolve_runtime(Some(runtime));
    let Some(cli) = resolved.cli.clone() else {
        return Err(resolved.message);
    };
    let command = login_command(&cli);
    match spawn_login(&cli) {
        Ok(()) => Ok(json!({
            "ok": true,
            "cliId": cli.cli_id,
            "loginCommand": command,
            "needsLogin": true,
            "message": format!("已打开 {DEFAULT_CLI_NAME} 登录。在本机窗口或浏览器完成授权后回到对话。{LOGIN_HINT}"),
        })),
        Err(err) => Ok(json!({
            "ok": false,
            "cliId": cli.cli_id,
            "loginCommand": command,
            "needsLogin": true,
            "message": format!("无法自动打开登录窗口（{err}）。请在本机执行：{command}。{LOGIN_HINT}"),
        })),
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

struct ResolvedRuntime {
    cli: Option<DetectedCli>,
    needs_install: bool,
    needs_login: bool,
    message: String,
}

fn ready_status(cli: DetectedCli) -> ResolvedRuntime {
    ResolvedRuntime {
        cli: Some(cli),
        needs_install: false,
        needs_login: false,
        message: String::new(),
    }
}

fn resolve_runtime(runtime: Option<&HostRuntime>) -> ResolvedRuntime {
    if let Some(cli) = from_env() {
        if cli.binary_path.is_file() {
            if let Some(runtime) = runtime {
                let _ = persist_runtime(runtime, &cli);
            }
            return ready_status(cli);
        }
        return ResolvedRuntime {
            cli: None,
            needs_install: false,
            needs_login: false,
            message: format!(
                "环境变量 SKILL_MANAGER_AGENT_CLI 指向的命令行不存在：{}",
                cli.binary_path.display()
            ),
        };
    }
    if let Some(runtime) = runtime {
        if let Some(stored) = load_runtime(runtime) {
            if stored.binary_path.is_file() {
                return ready_status(stored);
            }
        }
        if let Some(cli) = look_for_app_install(runtime) {
            let _ = persist_runtime(runtime, &cli);
            return ready_status(cli);
        }
    }
    // 只认钉死的默认名，不扫 claude / codex / gemini / hermes 等清单。
    if let Some(path) = which(DEFAULT_BIN) {
        let cli = default_detected(path, false, false);
        if let Some(runtime) = runtime {
            let _ = persist_runtime(runtime, &cli);
        }
        return ready_status(cli);
    }
    ResolvedRuntime {
        cli: None,
        needs_install: true,
        needs_login: false,
        message: INSTALL_HINT.to_string(),
    }
}

fn runtime_status_json(resolved: &ResolvedRuntime) -> Value {
    let cli = resolved.cli.as_ref();
    json!({
        "cliId": cli.map(|c| Value::String(c.cli_id.clone())).unwrap_or(Value::String(DEFAULT_CLI_ID.into())),
        "displayName": cli.map(|c| display_name(&c.cli_id)).unwrap_or_else(|| DEFAULT_CLI_NAME.to_string()),
        "defaultCliId": DEFAULT_CLI_ID,
        "binaryPath": cli.map(|c| Value::String(c.binary_path.display().to_string())).unwrap_or(Value::Null),
        "acpArgs": cli.map(|c| json!(c.acp_args)).unwrap_or_else(|| json!(DEFAULT_ACP_ARGS)),
        "userSelected": cli.map(|c| c.user_selected).unwrap_or(false),
        "installedByApp": cli.map(|c| c.installed_by_app).unwrap_or(false),
        "pinned": cli.is_some(),
        "ready": cli.is_some() && !resolved.needs_login,
        "installed": cli.is_some(),
        "needsInstall": resolved.needs_install,
        "needsLogin": resolved.needs_login,
        "loginCommand": cli.map(login_command).unwrap_or_else(|| format!("{DEFAULT_BIN} {}", DEFAULT_LOGIN_ARGS.join(" "))),
        "loginHint": LOGIN_HINT,
        "installHint": INSTALL_HINT,
        "exclusiveVendors": false,
        "error": if resolved.message.is_empty() { Value::Null } else { Value::String(resolved.message.clone()) },
    })
}

fn display_name(cli_id: &str) -> String {
    if cli_id == DEFAULT_CLI_ID || cli_id == "opencode-ai" {
        DEFAULT_CLI_NAME.to_string()
    } else {
        cli_id.to_string()
    }
}

fn login_command(cli: &DetectedCli) -> String {
    format!("{} {}", cli.binary_path.display(), DEFAULT_LOGIN_ARGS.join(" "))
}

fn default_detected(path: PathBuf, installed_by_app: bool, user_selected: bool) -> DetectedCli {
    DetectedCli {
        cli_id: DEFAULT_CLI_ID.to_string(),
        binary_path: path,
        installed_by_app,
        acp_args: DEFAULT_ACP_ARGS.iter().map(|s| (*s).to_string()).collect(),
        user_selected,
    }
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

fn look_for_app_install(runtime: &HostRuntime) -> Option<DetectedCli> {
    let dest = install_dir(runtime).join(DEFAULT_CLI_ID).join("node_modules").join(".bin");
    let unix = dest.join(DEFAULT_BIN);
    let windows = dest.join(format!("{DEFAULT_BIN}.cmd"));
    let path = if unix.is_file() {
        unix
    } else if windows.is_file() {
        windows
    } else {
        return None;
    };
    Some(default_detected(path, true, false))
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

fn spawn_login(cli: &DetectedCli) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        Command::new(&cli.binary_path)
            .args(DEFAULT_LOGIN_ARGS)
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()
            .map_err(|err| err.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Command::new(&cli.binary_path)
            .args(DEFAULT_LOGIN_ARGS)
            .spawn()
            .map_err(|err| err.to_string())?;
        Ok(())
    }
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

    let resolved = resolve_runtime(Some(runtime));
    let detected = match resolved.cli.clone() {
        Some(cli) => cli,
        None => {
            context::push_message(
                runtime,
                &session_id,
                AgentMessage {
                    role: "assistant".to_string(),
                    text: resolved.message.clone(),
                    task_id: Some(task_id.clone()),
                    created_at: now_rfc3339(),
                },
            )?;
            let mut payload = runtime_status_json(&resolved);
            payload["taskId"] = json!(task_id);
            payload["status"] = json!("failed");
            payload["message"] = json!(resolved.message);
            payload["snapshot"] = snapshot;
            return Ok(payload);
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
        let needs_login = looks_like_auth_text(&err);
        let message = if needs_login {
            format!(
                "{DEFAULT_CLI_NAME} 尚未完成登录。请在对话面板点「打开登录」，或在本机执行：{}。{LOGIN_HINT}",
                login_command(&detected)
            )
        } else {
            err.clone()
        };
        context::push_message(
            runtime,
            &session_id,
            AgentMessage {
                role: "assistant".to_string(),
                text: message.clone(),
                task_id: Some(task_id.clone()),
                created_at: now_rfc3339(),
            },
        )?;
        return Ok(json!({
            "taskId": task_id,
            "status": "failed",
            "message": message,
            "snapshot": snapshot,
            "cli": detected.cli_id,
            "cliId": detected.cli_id,
            "needsLogin": needs_login,
            "needsInstall": false,
            "loginCommand": login_command(&detected),
            "ready": false,
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

fn looks_like_auth_text(err: &str) -> bool {
    let text = err.to_lowercase();
    text.contains("auth")
        || text.contains("login")
        || text.contains("unauthenticated")
        || text.contains("api key")
        || text.contains("api_key")
        || text.contains("not logged")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|err| err.into_inner())
    }

    struct IsolatedPath {
        old: Option<std::ffi::OsString>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl IsolatedPath {
        fn empty() -> Self {
            let lock = test_path_lock();
            let old = std::env::var_os("PATH");
            unsafe { std::env::set_var("PATH", "") };
            Self { old, _lock: lock }
        }
    }

    impl Drop for IsolatedPath {
        fn drop(&mut self) {
            match &self.old {
                Some(value) => unsafe { std::env::set_var("PATH", value) },
                None => unsafe { std::env::remove_var("PATH") },
            }
        }
    }

    struct IsolatedEnv {
        key: &'static str,
        old: Option<std::ffi::OsString>,
    }

    impl IsolatedEnv {
        fn unset(key: &'static str) -> Self {
            let old = std::env::var_os(key);
            unsafe { std::env::remove_var(key) };
            Self { key, old }
        }
    }

    impl Drop for IsolatedEnv {
        fn drop(&mut self) {
            match &self.old {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    #[test]
    fn default_cli_is_opencode_not_a_vendor_list() {
        assert_eq!(DEFAULT_CLI_ID, "opencode");
        assert_eq!(DEFAULT_PACKAGE, "opencode-ai");
        assert_eq!(DEFAULT_ACP_ARGS, &["acp"]);
        assert_eq!(DEFAULT_LOGIN_ARGS, &["auth", "login"]);
    }

    #[test]
    fn missing_default_cli_is_install_capability_not_skip() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = skill_manager_core::AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        let _path = IsolatedPath::empty();
        let _env = IsolatedEnv::unset("SKILL_MANAGER_AGENT_CLI");
        let view = runtime_view(&runtime);
        assert_eq!(view["cliId"], "opencode");
        assert_eq!(view["defaultCliId"], "opencode");
        assert_eq!(view["needsInstall"], true);
        assert_eq!(view["ready"], false);
        assert_eq!(view["exclusiveVendors"], false);
        assert_eq!(view["cliId"].as_str(), Some("opencode"));
        let err = detect_and_lock_in(Some(&runtime)).unwrap_err();
        assert!(err.contains("安装 OpenCode") || err.contains("host agent-cli --install"));
        assert!(!err.contains("未检测到已登录或已安装"));
    }

    #[test]
    fn app_installed_default_is_pinned_without_path_scan() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = skill_manager_core::AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        let _path = IsolatedPath::empty();
        let _env = IsolatedEnv::unset("SKILL_MANAGER_AGENT_CLI");
        let bin_dir = dir
            .path()
            .join("agent-runtime")
            .join("opencode")
            .join("node_modules")
            .join(".bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let fake = bin_dir.join("opencode");
        std::fs::write(&fake, b"#!/bin/sh\nexit 0\n").unwrap();
        chmod_exec(&fake);
        let view = runtime_view(&runtime);
        assert_eq!(view["cliId"], "opencode");
        assert_eq!(view["needsInstall"], false);
        assert_eq!(view["ready"], true);
        assert_eq!(view["pinned"], true);
        assert_eq!(view["installedByApp"], true);
        let loaded = load_runtime(&runtime).unwrap();
        assert_eq!(loaded.cli_id, "opencode");
        assert!(loaded.binary_path.ends_with("opencode"));
    }

    #[test]
    fn path_claude_codex_or_hermes_is_not_auto_registered() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = skill_manager_core::AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        let _env = IsolatedEnv::unset("SKILL_MANAGER_AGENT_CLI");
        let bin_dir = dir.path().join("path-bins");
        std::fs::create_dir_all(&bin_dir).unwrap();
        for name in ["claude", "codex", "hermes-acp", "gemini", "claude-acp"] {
            let fake = bin_dir.join(name);
            std::fs::write(&fake, b"#!/bin/sh\nexit 0\n").unwrap();
            chmod_exec(&fake);
        }
        let _path = IsolatedPath::empty();
        unsafe { std::env::set_var("PATH", &bin_dir) };
        let view = runtime_view(&runtime);
        assert_eq!(view["cliId"], "opencode");
        assert_eq!(view["needsInstall"], true);
        assert_eq!(view["ready"], false);
        assert!(load_runtime(&runtime).is_none());
    }

    #[test]
    fn post_message_does_not_claim_success_when_cli_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = skill_manager_core::AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        let _path = IsolatedPath::empty();
        let _env = IsolatedEnv::unset("SKILL_MANAGER_AGENT_CLI");
        let result = post_message(
            &runtime,
            json!({ "sessionId": "s1", "text": "完善中文注释" }),
        )
        .unwrap();
        assert_eq!(result["status"], "failed");
        assert_eq!(result["needsInstall"], true);
        assert_eq!(result["cliId"], "opencode");
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
