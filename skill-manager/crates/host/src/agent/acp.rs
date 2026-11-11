//! ACP stdio JSON-RPC：拉起已锁定 CLI，转发工具，支持取消。

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::tools;
use crate::HostRuntime;

const INIT_TIMEOUT: Duration = Duration::from_secs(12);
const PROMPT_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Debug)]
pub struct DetectedCli {
    pub cli_id: String,
    pub binary_path: PathBuf,
    pub installed_by_app: bool,
}

struct LiveTask {
    cancel: Arc<AtomicBool>,
    pid: Mutex<Option<u32>>,
}

fn tasks() -> &'static Mutex<HashMap<String, LiveTask>> {
    static TASKS: std::sync::OnceLock<Mutex<HashMap<String, LiveTask>>> =
        std::sync::OnceLock::new();
    TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_task(task_id: &str) -> Arc<AtomicBool> {
    let cancel = Arc::new(AtomicBool::new(false));
    if let Ok(mut g) = tasks().lock() {
        g.insert(
            task_id.to_string(),
            LiveTask {
                cancel: Arc::clone(&cancel),
                pid: Mutex::new(None),
            },
        );
    }
    cancel
}

pub fn bind_pid(task_id: &str, pid: u32) {
    if let Ok(g) = tasks().lock() {
        if let Some(task) = g.get(task_id) {
            if let Ok(mut pid_slot) = task.pid.lock() {
                *pid_slot = Some(pid);
            }
        }
    }
}

pub fn cancel_live(task_id: &str) -> bool {
    let Ok(g) = tasks().lock() else {
        return false;
    };
    let Some(task) = g.get(task_id) else {
        return false;
    };
    task.cancel.store(true, Ordering::SeqCst);
    if let Ok(pid) = task.pid.lock() {
        if let Some(pid) = *pid {
            let _ = Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .status();
        }
    }
    true
}

pub fn forget_task(task_id: &str) {
    if let Ok(mut g) = tasks().lock() {
        g.remove(task_id);
    }
}

fn next_id() -> u64 {
    static ID: AtomicU64 = AtomicU64::new(1);
    ID.fetch_add(1, Ordering::Relaxed)
}

pub(crate) struct AcpProc {
    child: Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

fn spawn_with_args(bin: &Path, args: &[&str], skill_dir: &Path) -> Result<AcpProc, String> {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if skill_dir.is_dir() {
        cmd.env("SKILL_MANAGER_BUILTIN_SKILLS", skill_dir);
        cmd.env("OPENCODE_SKILLS_PATH", skill_dir);
        cmd.env("CODEX_SKILLS_DIR", skill_dir);
        cmd.env("GEMINI_SKILLS_DIR", skill_dir);
    }
    let mut child = cmd
        .spawn()
        .map_err(|err| format!("无法启动管理 CLI：{err}"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "管理 CLI 没有 stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "管理 CLI 没有 stdout".to_string())?;
    Ok(AcpProc {
        child,
        stdin,
        stdout: BufReader::new(stdout),
    })
}

fn spawn_candidates(cli: &DetectedCli, skill_dir: &Path) -> Result<AcpProc, String> {
    let patterns: &[&[&str]] = match cli.cli_id.as_str() {
        "opencode" => &[&["acp"], &["--acp"]],
        "codex" => &[&["app-server"], &["--acp"], &["mcp"]],
        "gemini" => &[&["--acp"], &["acp"]],
        _ => &[&["acp"], &["--acp"]],
    };
    let mut last = "没有可用的 ACP 启动参数".to_string();
    for args in patterns {
        match spawn_with_args(&cli.binary_path, args, skill_dir) {
            Ok(proc) => return Ok(proc),
            Err(err) => last = err,
        }
    }
    Err(last)
}

fn write_msg(stdin: &mut std::process::ChildStdin, value: &Value) -> Result<(), String> {
    let line = format!("{value}\n");
    stdin
        .write_all(line.as_bytes())
        .and_then(|_| stdin.flush())
        .map_err(|err| format!("写入管理 CLI 失败：{err}"))
}

fn read_msg(
    stdout: &mut BufReader<std::process::ChildStdout>,
    until: Instant,
) -> Result<Value, String> {
    loop {
        if Instant::now() > until {
            return Err("等待管理 CLI 响应超时".to_string());
        }
        let mut header = String::new();
        stdout
            .read_line(&mut header)
            .map_err(|err| format!("读取管理 CLI 失败：{err}"))?;
        if header.is_empty() {
            return Err("管理 CLI 已退出".to_string());
        }
        let trimmed = header.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            let len: usize = rest
                .trim()
                .parse()
                .map_err(|_| "无效的 Content-Length".to_string())?;
            let mut blank = String::new();
            stdout
                .read_line(&mut blank)
                .map_err(|err| err.to_string())?;
            let mut buf = vec![0u8; len];
            use std::io::Read;
            stdout.read_exact(&mut buf).map_err(|err| err.to_string())?;
            return serde_json::from_slice(&buf).map_err(|err| err.to_string());
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            return Ok(value);
        }
    }
}

fn looks_like_auth_failure(value: &Value) -> bool {
    let text = value.to_string().to_lowercase();
    text.contains("auth")
        || text.contains("login")
        || text.contains("unauthenticated")
        || text.contains("api key")
        || text.contains("api_key")
        || text.contains("not logged")
}

fn request(
    proc: &mut AcpProc,
    method: &str,
    params: Value,
    until: Instant,
    cancel: &AtomicBool,
    runtime: Option<&HostRuntime>,
    session_id: Option<&str>,
) -> Result<Value, String> {
    let id = next_id();
    write_msg(
        &mut proc.stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }),
    )?;
    loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = write_msg(
                &mut proc.stdin,
                &json!({ "jsonrpc": "2.0", "method": "session/cancel", "params": {} }),
            );
            return Err("cancelled".to_string());
        }
        let msg = read_msg(&mut proc.stdout, until)?;
        if msg.get("id").and_then(Value::as_u64) == Some(id)
            || msg.get("id").and_then(Value::as_i64) == Some(id as i64)
        {
            if let Some(error) = msg.get("error") {
                return Err(error.to_string());
            }
            return Ok(msg.get("result").cloned().unwrap_or(json!({})));
        }
        if let Some(method) = msg.get("method").and_then(Value::as_str) {
            handle_incoming(proc, &msg, method, runtime, session_id, cancel)?;
        }
    }
}

fn handle_incoming(
    proc: &mut AcpProc,
    msg: &Value,
    method: &str,
    runtime: Option<&HostRuntime>,
    session_id: Option<&str>,
    _cancel: &AtomicBool,
) -> Result<(), String> {
    if let Some(id) = msg.get("id") {
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let result = match method {
            "session/request_permission" => {
                json!({ "outcome": { "outcome": "selected", "optionId": "allow-once" } })
            }
            name if name.starts_with("fs/") => json!({ "content": "" }),
            name => {
                if let Some(runtime) = runtime {
                    match tools::run_tool(runtime, name, params, session_id) {
                        Ok(value) => value,
                        Err(err) => json!({ "error": err }),
                    }
                } else {
                    json!({ "error": "no runtime" })
                }
            }
        };
        write_msg(
            &mut proc.stdin,
            &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        )?;
    }
    Ok(())
}

pub(crate) fn initialize_or_fail(
    cli: &DetectedCli,
    skill_dir: &Path,
    skill_text: &str,
    cancel: &AtomicBool,
) -> Result<AcpProc, String> {
    let mut proc = spawn_candidates(cli, skill_dir)?;
    let until = Instant::now() + INIT_TIMEOUT;
    let init = match request(
        &mut proc,
        "initialize",
        json!({
            "protocolVersion": 1,
            "clientInfo": { "name": "skill-manager", "version": env!("CARGO_PKG_VERSION") },
            "clientCapabilities": { "fs": { "readTextFile": true, "writeTextFile": false } },
            "initializationOptions": {
                "skillDir": skill_dir,
                "skillText": if skill_dir.is_dir() { Value::Null } else { json!(skill_text) },
            }
        }),
        until,
        cancel,
        None,
        None,
    ) {
        Ok(value) => value,
        Err(err) => {
            let _ = proc.child.kill();
            if looks_like_auth_failure(&json!(err)) || err.to_lowercase().contains("auth") {
                return Err(
                    "管理 CLI 尚未登录模型。请先在该 CLI 中完成登录，再回来对话。任务不会标成进行中。"
                        .to_string(),
                );
            }
            return Err(err);
        }
    };
    if looks_like_auth_failure(&init) {
        let _ = proc.child.kill();
        return Err(
            "管理 CLI 尚未登录模型。请先在该 CLI 中完成登录，再回来对话。任务不会标成进行中。"
                .to_string(),
        );
    }
    if init
        .get("authMethods")
        .and_then(Value::as_array)
        .is_some_and(|m| !m.is_empty())
        && init.get("authenticated").and_then(Value::as_bool) != Some(true)
    {
        let _ = proc.child.kill();
        return Err(
            "管理 CLI 尚未登录模型。请先在该 CLI 中完成登录，再回来对话。任务不会标成进行中。"
                .to_string(),
        );
    }
    Ok(proc)
}

pub(crate) fn run_prompt(
    mut proc: AcpProc,
    runtime: &HostRuntime,
    session_id: &str,
    task_id: &str,
    prompt: String,
    cancel: Arc<AtomicBool>,
) -> Result<String, String> {
    bind_pid(task_id, proc.child.id());
    let until = Instant::now() + PROMPT_TIMEOUT;
    let new_session = request(
        &mut proc,
        "session/new",
        json!({ "cwd": std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")) }),
        Instant::now() + INIT_TIMEOUT,
        &cancel,
        Some(runtime),
        Some(session_id),
    );
    let acp_session = match new_session {
        Ok(value) => value
            .get("sessionId")
            .and_then(Value::as_str)
            .unwrap_or(session_id)
            .to_string(),
        Err(_) => session_id.to_string(),
    };
    let result = request(
        &mut proc,
        "session/prompt",
        json!({
            "sessionId": acp_session,
            "prompt": [{ "type": "text", "text": prompt }]
        }),
        until,
        &cancel,
        Some(runtime),
        Some(session_id),
    );
    let _ = proc.child.kill();
    forget_task(task_id);
    match result {
        Ok(value) => {
            if let Some(text) = value.get("text").and_then(Value::as_str) {
                Ok(text.to_string())
            } else if let Some(stop) = value.get("stopReason").and_then(Value::as_str) {
                Ok(format!("管理 CLI 已结束（{stop}）。细节已通过工具执行。"))
            } else {
                Ok("管理 CLI 已完成本轮。".to_string())
            }
        }
        Err(err) if err == "cancelled" => Err("cancelled".into()),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_error_text_is_detected() {
        assert!(looks_like_auth_failure(
            &json!({"error": "unauthenticated"})
        ));
        assert!(!looks_like_auth_failure(&json!({"ok": true})));
    }
}
