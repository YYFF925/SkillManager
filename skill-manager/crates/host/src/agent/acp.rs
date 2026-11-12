//! ACP stdio JSON-RPC：拉起当前 ACP 兼容 CLI，按协议声明 Host 工具，支持取消与会话复用。

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::tools;
use crate::HostRuntime;
use crate::invoke::InvokeSource;

const INIT_TIMEOUT: Duration = Duration::from_secs(12);
const PROMPT_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const FS_READ_MAX: usize = 200_000;

#[derive(Clone, Debug)]
pub struct DetectedCli {
    pub cli_id: String,
    pub binary_path: PathBuf,
    pub installed_by_app: bool,
    pub acp_args: Vec<String>,
    pub user_selected: bool,
}

struct LiveTask {
    cancel: Arc<AtomicBool>,
    pid: Mutex<Option<u32>>,
}

struct LiveAcp {
    proc: AcpProc,
    acp_session_id: String,
    cli_path: PathBuf,
}

impl Drop for LiveAcp {
    fn drop(&mut self) {
        kill_child(&mut self.proc.child);
    }
}

fn tasks() -> &'static Mutex<HashMap<String, LiveTask>> {
    static TASKS: std::sync::OnceLock<Mutex<HashMap<String, LiveTask>>> =
        std::sync::OnceLock::new();
    TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn sessions() -> &'static Mutex<HashMap<String, Arc<Mutex<Option<LiveAcp>>>>> {
    static SESSIONS: std::sync::OnceLock<Mutex<HashMap<String, Arc<Mutex<Option<LiveAcp>>>>>> =
        std::sync::OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn session_slot(host_session: &str) -> Arc<Mutex<Option<LiveAcp>>> {
    let mut map = sessions().lock().expect("ACP session map");
    map.entry(host_session.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(None)))
        .clone()
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
            kill_pid(pid);
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
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: BufReader<std::process::ChildStdout>,
}

/// Windows 上 CreateProcess 不能跑无扩展 Unix `#!/bin/sh` 垫片（os error 193）。
/// ACP 优先原生 `opencode.exe`（管道直接接到 CLI）；没有 exe 才退回 `.cmd`。
pub(crate) fn spawnable_cli_path(path: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        windows_spawnable_cli_path(path)
    }
    #[cfg(not(windows))]
    {
        path.is_file().then(|| path.to_path_buf())
    }
}

fn windows_ext(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn is_usable_exe(path: &Path) -> bool {
    path.is_file()
}

/// npm 可选依赖里的原生二进制，避开 几百字节的 launcher stub。
fn find_bundled_opencode_exe(from: &Path) -> Option<PathBuf> {
    let start = if from.is_dir() {
        from
    } else {
        from.parent()?
    };
    for dir in start.ancestors() {
        if dir.file_name().and_then(|name| name.to_str()) != Some("node_modules") {
            continue;
        }
        let mut stub = None;
        for pkg in ["opencode-windows-x64", "opencode-windows-arm64", "opencode-ai"] {
            let exe = dir.join(pkg).join("bin").join("opencode.exe");
            if !is_usable_exe(&exe) {
                continue;
            }
            let len = std::fs::metadata(&exe).map(|meta| meta.len()).unwrap_or(0);
            if len > 1024 {
                return Some(exe);
            }
            if stub.is_none() {
                stub = Some(exe);
            }
        }
        if let Some(exe) = stub {
            return Some(exe);
        }
    }
    None
}

#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn windows_spawnable_cli_path(path: &Path) -> Option<PathBuf> {
    if let Some(exe) = find_bundled_opencode_exe(path) {
        return Some(exe);
    }
    let ext = windows_ext(path);
    if matches!(ext.as_str(), "exe" | "com") {
        return path.is_file().then(|| path.to_path_buf());
    }
    let exe = path.with_extension("exe");
    if is_usable_exe(&exe) {
        return Some(exe);
    }
    if matches!(ext.as_str(), "cmd" | "bat") {
        return path.is_file().then(|| path.to_path_buf());
    }
    let cmd = path.with_extension("cmd");
    if cmd.is_file() {
        return Some(cmd);
    }
    None
}

#[cfg_attr(not(windows), allow(dead_code))]
fn windows_acp_cmd_args(bin: &Path) -> Vec<std::ffi::OsString> {
    vec!["/C".into(), bin.as_os_str().to_os_string()]
}

/// ACP 启动方式：exe 直接拉起（stdio 接到 CLI）；.cmd 才经 cmd /C。不要 start。
#[cfg_attr(not(windows), allow(dead_code))]
fn windows_acp_invocation(bin: &Path) -> (std::ffi::OsString, Vec<std::ffi::OsString>) {
    let ext = windows_ext(bin);
    if matches!(ext.as_str(), "cmd" | "bat") {
        ("cmd".into(), windows_acp_cmd_args(bin))
    } else {
        (bin.as_os_str().to_os_string(), Vec::new())
    }
}

fn cli_process_command(bin: &Path) -> Command {
    #[cfg(windows)]
    {
        let (program, args) = windows_acp_invocation(bin);
        let mut cmd = skill_manager_core::process::command(&program);
        cmd.args(args);
        cmd
    }
    #[cfg(not(windows))]
    {
        Command::new(bin)
    }
}

fn spawn_with_args(bin: &Path, args: &[&str], skill_dir: &Path) -> Result<AcpProc, String> {
    let bin = spawnable_cli_path(bin).ok_or_else(|| {
        format!(
            "无法启动管理 CLI：{} 不是可在本机拉起的程序（Windows 上应使用 .cmd，不能是无扩展 Unix 垫片）",
            bin.display()
        )
    })?;
    let mut cmd = cli_process_command(&bin);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if skill_dir.is_dir() {
        cmd.env("SKILL_MANAGER_BUILTIN_SKILLS", skill_dir);
        // OpenCode 1.x 实测认 OPENCODE_CONFIG_DIR/skills/，不认 OPENCODE_SKILLS_PATH。
        cmd.env("OPENCODE_CONFIG_DIR", skill_dir);
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
        stdin: Arc::new(Mutex::new(stdin)),
        stdout: BufReader::new(stdout),
    })
}

fn candidate_arg_sets(cli: &DetectedCli) -> Vec<Vec<String>> {
    let mut sets = Vec::new();
    if !cli.acp_args.is_empty() {
        sets.push(cli.acp_args.clone());
    }
    for set in default_acp_arg_sets(&cli.cli_id) {
        let owned: Vec<String> = set.iter().map(|s| (*s).to_string()).collect();
        if !sets.contains(&owned) {
            sets.push(owned);
        }
    }
    sets
}

/// 未登记 `--args` 时的尝试顺序。按文档试 `acp` / `--acp` / 无参；握手失败再试下一组。
pub(crate) fn default_acp_arg_sets(_cli_id: &str) -> Vec<&'static [&'static str]> {
    vec![&["acp"], &["--acp"], &[]]
}

fn initialize_payload(skill_dir: &Path, skill_text: &str) -> Value {
    json!({
        "protocolVersion": 1,
        "clientInfo": { "name": "skill-manager", "version": env!("CARGO_PKG_VERSION") },
        "clientCapabilities": {
            "fs": { "readTextFile": true, "writeTextFile": false },
            "session": { "mcp": { "stdio": true } }
        },
        "initializationOptions": {
            "skillDir": skill_dir,
            "skillText": skill_text,
        },
    })
}

#[cfg_attr(not(windows), allow(dead_code))]
fn windows_taskkill_args(pid: u32) -> Vec<String> {
    vec![
        "/PID".into(),
        pid.to_string(),
        "/T".into(),
        "/F".into(),
    ]
}

fn kill_pid(pid: u32) {
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status();
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(windows_taskkill_args(pid))
            .status();
    }
}

fn kill_child(child: &mut Child) {
    kill_pid(child.id());
    let _ = child.kill();
    let _ = child.try_wait();
}

/// `read_line` 在子进程活着但不说话时会一直堵；握手必须能超时杀掉再试下一组参数。
fn arm_handshake_watchdog(pid: u32) -> (Arc<AtomicBool>, std::thread::JoinHandle<()>) {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    let handle = std::thread::spawn(move || {
        let deadline = Instant::now() + INIT_TIMEOUT;
        while Instant::now() < deadline {
            if flag.load(Ordering::SeqCst) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        if !flag.load(Ordering::SeqCst) {
            kill_pid(pid);
        }
    });
    (stop, handle)
}

fn request_bounded(
    proc: &mut AcpProc,
    method: &str,
    params: Value,
    cancel: &AtomicBool,
    runtime: Option<&HostRuntime>,
    host_session: Option<&str>,
    acp_session: Option<&str>,
) -> Result<Value, String> {
    let (stop, watchdog) = arm_handshake_watchdog(proc.child.id());
    let result = request(
        proc,
        method,
        params,
        Instant::now() + INIT_TIMEOUT,
        cancel,
        runtime,
        host_session,
        acp_session,
    );
    stop.store(true, Ordering::SeqCst);
    let _ = watchdog.join();
    if result.is_err() {
        kill_child(&mut proc.child);
    }
    result
}

fn handshake_initialize(
    proc: &mut AcpProc,
    skill_dir: &Path,
    skill_text: &str,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let init = request_bounded(
        proc,
        "initialize",
        initialize_payload(skill_dir, skill_text),
        cancel,
        None,
        None,
        None,
    )?;
    if looks_like_auth_failure(&init) {
        return Err(
            "管理 CLI 尚未登录模型。请先在该 CLI 中完成登录，再回来对话。任务不会标成进行中。"
                .to_string(),
        );
    }
    Ok(())
}

/// 进程能 spawn 不算成功：每组参数都要 ACP initialize 握手，失败再试下一组。
pub(crate) fn initialize_with_args(
    cli: &DetectedCli,
    skill_dir: &Path,
    skill_text: &str,
    cancel: &AtomicBool,
) -> Result<(AcpProc, Vec<String>), String> {
    let mut last = "没有可用的 ACP 启动参数".to_string();
    for args in candidate_arg_sets(cli) {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut proc = match spawn_with_args(&cli.binary_path, &refs, skill_dir) {
            Ok(proc) => proc,
            Err(err) => {
                last = err;
                continue;
            }
        };
        match handshake_initialize(&mut proc, skill_dir, skill_text, cancel) {
            Ok(()) => return Ok((proc, args)),
            Err(err) => {
                kill_child(&mut proc.child);
                last = err;
            }
        }
    }
    if looks_like_auth_failure(&json!(last)) || last.to_lowercase().contains("auth") {
        Err(
            "管理 CLI 尚未登录模型。请先在该 CLI 中完成登录，再回来对话。任务不会标成进行中。"
                .to_string(),
        )
    } else {
        Err(format!(
            "无法与 {bin} 完成 ACP 握手（已试 acp / --acp / 无参）。{last}",
            bin = cli.binary_path.display()
        ))
    }
}

/// 登记前探测：握手成功才返回实际可用的 ACP 参数；会关掉探测进程。
pub(crate) fn probe_acp(cli: &DetectedCli, skill_dir: &Path) -> Result<Vec<String>, String> {
    let cancel = AtomicBool::new(false);
    let (mut proc, args) = initialize_with_args(cli, skill_dir, "", &cancel)?;
    kill_child(&mut proc.child);
    Ok(args)
}

fn write_msg(stdin: &mut ChildStdin, value: &Value) -> Result<(), String> {
    let line = format!("{value}\n");
    stdin
        .write_all(line.as_bytes())
        .and_then(|_| stdin.flush())
        .map_err(|err| format!("写入管理 CLI 失败：{err}"))
}

fn write_locked(stdin: &Arc<Mutex<ChildStdin>>, value: &Value) -> Result<(), String> {
    let mut guard = stdin
        .lock()
        .map_err(|_| "管理 CLI stdin 锁已损坏".to_string())?;
    write_msg(&mut guard, value)
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
    host_session: Option<&str>,
    acp_session: Option<&str>,
) -> Result<Value, String> {
    let id = next_id();
    write_locked(
        &proc.stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }),
    )?;
    loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = write_locked(
                &proc.stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "method": "session/cancel",
                    "params": { "sessionId": acp_session.unwrap_or("") }
                }),
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
            handle_incoming(proc, &msg, method, runtime, host_session)?;
        }
    }
}

fn handle_incoming(
    proc: &mut AcpProc,
    msg: &Value,
    method: &str,
    runtime: Option<&HostRuntime>,
    host_session: Option<&str>,
) -> Result<(), String> {
    let Some(id) = msg.get("id").cloned() else {
        return Ok(());
    };
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    match method {
        "session/request_permission" => {
            write_locked(
                &proc.stdin,
                &json!({ "jsonrpc": "2.0", "id": id, "result": permission_decision(&params) }),
            )?;
        }
        "fs/read_text_file" => match fs_read(&params) {
            Ok(result) => {
                write_locked(
                    &proc.stdin,
                    &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                )?;
            }
            Err(err) => {
                write_locked(
                    &proc.stdin,
                    &json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": err } }),
                )?;
            }
        },
        "fs/write_text_file" => {
            write_locked(
                &proc.stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": "writeTextFile 未开放。库写入必须经 skill_set / skill_delete 等 Host 工具，不能直写文件。"
                    }
                }),
            )?;
        }
        "tools/call" | "tool/call" => {
            let name = params
                .get("name")
                .or_else(|| params.get("toolName"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let args = params
                .get("arguments")
                .or_else(|| params.get("input"))
                .cloned()
                .unwrap_or(params);
            let stdin = Arc::clone(&proc.stdin);
            let runtime = runtime.cloned();
            let host_session = host_session.map(str::to_string);
            std::thread::spawn(move || {
                let result = dispatch_tool(runtime.as_ref(), &name, args, host_session.as_deref());
                let _ = write_locked(
                    &stdin,
                    &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                );
            });
        }
        name if name.starts_with("fs/") => {
            write_locked(
                &proc.stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": format!("未实现的文件系统方法：{name}") }
                }),
            )?;
        }
        other => {
            write_locked(
                &proc.stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": format!("未知方法：{other}") }
                }),
            )?;
        }
    }
    Ok(())
}

fn dispatch_tool(
    runtime: Option<&HostRuntime>,
    name: &str,
    args: Value,
    session_id: Option<&str>,
) -> Value {
    let Some(runtime) = runtime else {
        return json!({ "error": "no runtime" });
    };
    let Some(command) = tools::tool_command_name(name) else {
        return json!({ "error": format!("未知或不允许的管理工具：{name}") });
    };
    let args = tools::normalize_args(&command, args);
    if let Some(listen) = wait_for_listen(runtime) {
        match crate::http::forward_invoke(&listen, &command, &args, InvokeSource::Agent, session_id)
        {
            Ok(value) => value,
            Err(err) => json!({ "error": err }),
        }
    } else {
        match tools::run_tool(runtime, &command, args, session_id) {
            Ok(value) => value,
            Err(err) => json!({ "error": err }),
        }
    }
}

fn wait_for_listen(runtime: &HostRuntime) -> Option<String> {
    for _ in 0..20 {
        if let Some(addr) = runtime.listen_addr() {
            return Some(addr);
        }
        if let Some(listen) = crate::lock::peek_meta(runtime.state_dir()).and_then(|m| m.listen) {
            return Some(listen);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    None
}

pub(crate) fn permission_decision(params: &Value) -> Value {
    let allow = !looks_like_bypass(params) && tools::is_allowed(&permission_tool_name(params));
    select_permission_option(params, allow)
}

fn permission_tool_name(params: &Value) -> String {
    for key in [
        params.pointer("/toolCall/title"),
        params.pointer("/toolCall/kind"),
        params.pointer("/toolCall/rawInput/toolName"),
        params.pointer("/toolCall/rawInput/name"),
        params.pointer("/subject/toolCall/title"),
        params.get("title"),
        params.pointer("/toolCall/id"),
    ] {
        if let Some(Value::String(s)) = key {
            if !s.is_empty() {
                return s.clone();
            }
        }
    }
    String::new()
}

fn looks_like_bypass(params: &Value) -> bool {
    if tools::is_allowed(&permission_tool_name(params)) {
        return false;
    }
    if params.pointer("/subject/type").and_then(Value::as_str) == Some("command") {
        return true;
    }
    let kind = params
        .pointer("/toolCall/kind")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();
    if matches!(
        kind.as_str(),
        "execute" | "edit" | "delete" | "terminal" | "shell" | "bash"
    ) {
        return true;
    }
    let blob = params.to_string().to_lowercase();
    const KEYS: &[&str] = &[
        "bash",
        "shell",
        "terminal",
        "powershell",
        "cmd.exe",
        "apply_patch",
        "str_replace",
        "write_text_file",
        "fs/write",
        "run_command",
        "execute",
    ];
    KEYS.iter().any(|key| blob.contains(key))
        || blob.contains("\"edit\"")
        || blob.contains("\"write\"")
}

fn select_permission_option(params: &Value, allow: bool) -> Value {
    let options = params.get("options").and_then(Value::as_array);
    let pick = options.and_then(|opts| {
        opts.iter().find(|opt| {
            let kind = opt
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_lowercase();
            let id = opt
                .get("optionId")
                .or_else(|| opt.get("option_id"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_lowercase();
            if allow {
                kind.contains("allow") || id.contains("allow")
            } else {
                kind.contains("reject") || kind.contains("deny") || id.contains("reject")
            }
        })
    });
    if let Some(opt) = pick {
        let option_id = opt
            .get("optionId")
            .or_else(|| opt.get("option_id"))
            .cloned()
            .unwrap_or(json!("allow-once"));
        json!({ "outcome": { "outcome": "selected", "optionId": option_id } })
    } else if allow {
        json!({ "outcome": { "outcome": "selected", "optionId": "allow-once" } })
    } else {
        json!({ "outcome": { "outcome": "cancelled" } })
    }
}

fn fs_read(params: &Value) -> Result<Value, String> {
    let path = params
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "缺少 path".to_string())?;
    let data = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    let content = if data.len() > FS_READ_MAX {
        data.chars().take(FS_READ_MAX).collect::<String>()
    } else {
        data
    };
    Ok(json!({ "content": content }))
}

fn mcp_cli_path() -> Result<PathBuf, String> {
    if let Ok(raw) = std::env::var("SKILL_MANAGER_MCP_BIN") {
        let path = PathBuf::from(raw);
        if path.is_file() {
            return Ok(path);
        }
    }
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let dir = exe.parent().unwrap_or_else(|| Path::new("."));
    for name in ["skill-manager", "skill-manager.exe"] {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    let stem = exe.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    if stem == "skill-manager" {
        return Ok(exe);
    }
    Err("找不到 skill-manager CLI，无法经 ACP 声明 Host 工具。请确认桌面包含 sidecar 或 PATH 中有 skill-manager。".into())
}

fn mcp_server_spec(runtime: &HostRuntime, host_session: &str) -> Result<Value, String> {
    let listen = wait_for_listen(runtime)
        .ok_or_else(|| "Host HTTP 尚未监听，无法向 CLI 声明管理工具".to_string())?;
    let command = mcp_cli_path()?;
    Ok(json!({
        "type": "stdio",
        "name": "skill-manager",
        "command": command,
        "args": ["host", "mcp"],
        "env": [
            { "name": "SKILL_MANAGER_HOST_LISTEN", "value": listen },
            { "name": "SKILL_MANAGER_AGENT_SESSION", "value": host_session },
        ]
    }))
}

pub(crate) fn initialize_or_fail(
    cli: &DetectedCli,
    skill_dir: &Path,
    skill_text: &str,
    cancel: &AtomicBool,
) -> Result<AcpProc, String> {
    // 仅 advertise authMethods 不等于未登录：OpenCode 始终列出 opencode-login，但仍可用其免费模型。
    Ok(initialize_with_args(cli, skill_dir, skill_text, cancel)?.0)
}

fn child_alive(child: &mut Child) -> bool {
    matches!(child.try_wait(), Ok(None))
}

/// 握手在标 running 之前完成：未登录或无法声明工具则失败。已有活 session 则跳过。
/// 返回是否新建了 ACP session（新建才回灌 Host 历史）。
pub(crate) fn ensure_session(
    cli: &DetectedCli,
    skill_dir: &Path,
    skill_text: &str,
    runtime: &HostRuntime,
    host_session: &str,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    let slot = session_slot(host_session);
    let mut guard = slot.lock().map_err(|_| "ACP 会话锁已损坏".to_string())?;
    if let Some(live) = guard.as_mut() {
        if live.cli_path == cli.binary_path && child_alive(&mut live.proc.child) {
            return Ok(false);
        }
    }
    *guard = None;
    let mut proc = initialize_or_fail(cli, skill_dir, skill_text, cancel)?;
    let mcp = match mcp_server_spec(runtime, host_session) {
        Ok(value) => value,
        Err(err) => {
            kill_child(&mut proc.child);
            return Err(err);
        }
    };
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let new_session = request_bounded(
        &mut proc,
        "session/new",
        json!({
            "cwd": cwd,
            "mcpServers": [mcp],
        }),
        cancel,
        Some(runtime),
        Some(host_session),
        None,
    );
    let acp_session = match new_session {
        Ok(value) => value
            .get("sessionId")
            .and_then(Value::as_str)
            .unwrap_or(host_session)
            .to_string(),
        Err(err) => {
            kill_child(&mut proc.child);
            return Err(err);
        }
    };
    *guard = Some(LiveAcp {
        proc,
        acp_session_id: acp_session,
        cli_path: cli.binary_path.clone(),
    });
    Ok(true)
}

/// 同一 Host 对话复用 ACP session；进程死了才 initialize + session/new。
pub(crate) fn run_turn(
    cli: &DetectedCli,
    skill_dir: &Path,
    skill_text: &str,
    runtime: &HostRuntime,
    host_session: &str,
    task_id: &str,
    snapshot: &Value,
    history: &str,
    user_text: &str,
    cancel: Arc<AtomicBool>,
) -> Result<String, String> {
    let fresh = ensure_session(cli, skill_dir, skill_text, runtime, host_session, &cancel)?;
    let slot = session_slot(host_session);
    let mut guard = slot.lock().map_err(|_| "ACP 会话锁已损坏".to_string())?;
    let live = guard
        .as_mut()
        .ok_or_else(|| "ACP session 未建立".to_string())?;
    bind_pid(task_id, live.proc.child.id());
    let prompt = if fresh {
        format!(
            "{skill_text}\n\n# 此前对话（Host 存档，ACP 新 session 回灌）\n{}\n# 当前 Host 快照（摘要+计数+引用，不含正文）\n{}\n\n# 用户\n{user_text}\n\n请用 Host 经 session/new.mcpServers 声明的工具（skill_list / skill_get / skill_read / skill_set 等）读取细节并执行。不要用 bash/edit 直写库。删除/覆盖必须走确认计划；未开启 git_auto_push 时不要上传。",
            if history.is_empty() {
                "（无）\n\n"
            } else {
                history
            },
            snapshot
        )
    } else {
        format!(
            "# 当前 Host 快照（摘要+计数+引用，不含正文）\n{snapshot}\n\n# 用户\n{user_text}\n\n请用已声明的 Host 工具继续。不要用 bash/edit 直写库。"
        )
    };
    let until = Instant::now() + PROMPT_TIMEOUT;
    let acp_session = live.acp_session_id.clone();
    let result = request(
        &mut live.proc,
        "session/prompt",
        json!({
            "sessionId": acp_session,
            "prompt": [{ "type": "text", "text": prompt }]
        }),
        until,
        &cancel,
        Some(runtime),
        Some(host_session),
        Some(&acp_session),
    );
    forget_task(task_id);
    if cancel.load(Ordering::SeqCst) {
        *guard = None;
    }
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
    fn windows_prefers_cmd_shim_over_unix_bin() {
        let dir = tempfile::tempdir().unwrap();
        let unix = dir.path().join("opencode");
        let cmd = dir.path().join("opencode.cmd");
        std::fs::write(&unix, b"#!/bin/sh\nexit 0\n").unwrap();
        std::fs::write(&cmd, b"@echo off\n").unwrap();
        assert_eq!(windows_spawnable_cli_path(&unix).as_deref(), Some(cmd.as_path()));
        assert_eq!(windows_spawnable_cli_path(&cmd).as_deref(), Some(cmd.as_path()));
        assert_eq!(windows_spawnable_cli_path(&unix).unwrap().extension().unwrap(), "cmd");
    }

    #[test]
    fn windows_prefers_native_exe_over_cmd_wrapper() {
        let dir = tempfile::tempdir().unwrap();
        let nm = dir.path().join("node_modules");
        let bin_dir = nm.join(".bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::create_dir_all(nm.join("opencode-windows-x64").join("bin")).unwrap();
        std::fs::write(bin_dir.join("opencode"), b"#!/bin/sh\n").unwrap();
        std::fs::write(bin_dir.join("opencode.cmd"), b"@echo off\n").unwrap();
        let exe = nm.join("opencode-windows-x64").join("bin").join("opencode.exe");
        std::fs::write(&exe, vec![0u8; 4096]).unwrap();
        assert_eq!(
            windows_spawnable_cli_path(&bin_dir.join("opencode")).as_deref(),
            Some(exe.as_path())
        );
        let (program, args) = windows_acp_invocation(&exe);
        assert_eq!(program, exe.as_os_str());
        assert!(args.is_empty(), "exe must be spawned directly so ACP stdio is not on cmd.exe");
        assert!(
            !args.iter().any(|arg| arg.to_string_lossy().eq_ignore_ascii_case("start")),
            "ACP must not use start"
        );
    }

    #[test]
    fn windows_unix_shim_alone_is_not_spawnable() {
        let dir = tempfile::tempdir().unwrap();
        let unix = dir.path().join("opencode");
        std::fs::write(&unix, b"#!/bin/sh\nexit 0\n").unwrap();
        assert_eq!(windows_spawnable_cli_path(&unix), None);
    }

    #[test]
    fn windows_acp_launch_uses_cmd_not_start() {
        let bin = Path::new(
            r"C:\qa-host-a\agent-runtime\opencode\node_modules\.bin\opencode.cmd",
        );
        let (program, args) = windows_acp_invocation(bin);
        assert_eq!(program, "cmd");
        let argv: Vec<String> = args
            .into_iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(argv, vec!["/C", bin.to_str().unwrap()]);
        assert!(
            !argv.iter().any(|arg| arg.eq_ignore_ascii_case("start")),
            "ACP handshake must keep piped stdio; start is only for login, got {argv:?}"
        );
        assert!(argv.iter().all(|arg| !arg.contains('"')));
    }

    #[test]
    fn windows_timeout_kills_process_tree() {
        let args = windows_taskkill_args(34160);
        assert_eq!(args, vec!["/PID", "34160", "/T", "/F"]);
    }

    #[test]
    fn auth_error_text_is_detected() {
        assert!(looks_like_auth_failure(
            &json!({"error": "unauthenticated"})
        ));
        assert!(!looks_like_auth_failure(&json!({"ok": true})));
    }

    #[test]
    fn permission_rejects_bash_and_edit() {
        let bash = permission_decision(&json!({
            "title": "Run bash",
            "toolCall": { "kind": "execute", "title": "bash", "rawInput": { "command": "rm -rf vault" } },
            "options": [
                { "optionId": "allow-once", "kind": "allow_once" },
                { "optionId": "reject-once", "kind": "reject_once" }
            ]
        }));
        assert_eq!(bash["outcome"]["optionId"], "reject-once");

        let edit = permission_decision(&json!({
            "title": "Edit SKILL.md",
            "toolCall": { "kind": "edit", "title": "edit" },
            "options": [{ "optionId": "allow-always", "kind": "allow_always" }]
        }));
        assert_eq!(edit["outcome"]["outcome"], "cancelled");
    }

    #[test]
    fn permission_allows_host_skill_set() {
        let allow = permission_decision(&json!({
            "title": "skill_set",
            "toolCall": { "title": "skill_set", "kind": "other" },
            "options": [
                { "optionId": "allow-once", "kind": "allow_once" },
                { "optionId": "reject-once", "kind": "reject_once" }
            ]
        }));
        assert_eq!(allow["outcome"]["optionId"], "allow-once");
    }

    #[test]
    fn fs_write_is_not_empty_success() {
        let params = json!({ "path": "/tmp/x", "content": "nope" });
        assert!(looks_like_bypass(&json!({
            "toolCall": { "kind": "edit", "title": "fs/write_text_file" }
        })));
        let _ = params;
    }

    #[test]
    fn permission_uses_cli_option_id() {
        let params = json!({
            "title": "skill_set",
            "toolCall": { "title": "skill_set", "kind": "other" },
            "options": [
                {"optionId": "once", "kind": "allow_once"},
                {"optionId": "reject", "kind": "reject_once"}
            ]
        });
        assert_eq!(permission_decision(&params)["outcome"]["optionId"], "once");
    }
}
