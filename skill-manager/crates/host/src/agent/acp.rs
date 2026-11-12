//! ACP stdio JSON-RPC：拉起当前 ACP 兼容 CLI，按协议声明 Host 工具，支持取消与会话复用。

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
#[cfg(not(windows))]
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::tools;
use crate::invoke::InvokeSource;
use crate::HostRuntime;

#[cfg(windows)]
#[path = "acp_win.rs"]
mod acp_win;

/// Windows ACP CreateProcess：隐藏 ConPTY（不用 CREATE_NO_WINDOW，也不用会抢走管道的 CREATE_NEW_CONSOLE）。
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const ACP_SW_HIDE: u16 = 0;
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const ACP_STARTF_USESHOWWINDOW: u32 = 0x0000_0001;
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const ACP_STARTF_USESTDHANDLES: u32 = 0x0000_0100;
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const ACP_CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const ACP_CREATE_UNICODE_ENVIRONMENT: u32 = 0x0000_0400;
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const ACP_EXTENDED_STARTUPINFO_PRESENT: u32 = 0x0008_0000;
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const ACP_CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// ConPTY 只给 Bun 一个隐藏控制台（避免 CREATE_NO_WINDOW 把 stdin 当已关闭）。
/// ACP JSON-RPC 必须走另一套管道：三个 std handle 都要有效，空 stdin/stdout 会 0xC0000142。
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn windows_acp_create_flags() -> (u32, u16, u32) {
    (
        ACP_STARTF_USESHOWWINDOW | ACP_STARTF_USESTDHANDLES,
        ACP_SW_HIDE,
        ACP_EXTENDED_STARTUPINFO_PRESENT | ACP_CREATE_UNICODE_ENVIRONMENT,
    )
}

/// 子进程要继承 ACP 的 stdin/stdout/stderr 管道。ConPTY 句柄本身不继承。
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const ACP_CONPTY_INHERIT_HANDLES: i32 = 1;

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

enum CliChild {
    Std(Child),
    #[cfg(windows)]
    Hidden(acp_win::HiddenChild),
}

impl CliChild {
    fn id(&self) -> u32 {
        match self {
            CliChild::Std(child) => child.id(),
            #[cfg(windows)]
            CliChild::Hidden(child) => child.pid,
        }
    }

    fn wait_exit(&mut self, timeout: Duration) -> Option<i32> {
        match self {
            #[cfg(windows)]
            CliChild::Hidden(child) => {
                let ms = timeout.as_millis().min(u32::MAX as u128) as u32;
                child.try_exit_code(ms).map(|code| code as i32)
            }
            CliChild::Std(child) => {
                let deadline = Instant::now() + timeout;
                loop {
                    match child.try_wait() {
                        Ok(Some(status)) => return status.code(),
                        Ok(None) => {}
                        Err(_) => return None,
                    }
                    if Instant::now() >= deadline {
                        return None;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        }
    }
}

pub(crate) struct AcpProc {
    child: CliChild,
    stdin: Arc<Mutex<Box<dyn Write + Send>>>,
    stdout_rx: mpsc::Receiver<std::io::Result<String>>,
    stderr: Arc<Mutex<String>>,
    timed_out: Arc<AtomicBool>,
    started: Instant,
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
        // 测试/探测用 Command：不要 CREATE_NO_WINDOW。真握手走 CreateProcessW +
        // SW_HIDE（见 acp_win），给 Bun 控制台但窗口隐藏；登录仍用 start。
        let mut cmd = Command::new(&program);
        cmd.args(args);
        cmd
    }
    #[cfg(not(windows))]
    {
        Command::new(bin)
    }
}

fn collect_stderr_reader(mut stderr: impl Read + Send + 'static) -> Arc<Mutex<String>> {
    let collected = Arc::new(Mutex::new(String::new()));
    let slot = Arc::clone(&collected);
    std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = stderr.read_to_string(&mut buf);
        if let Ok(mut guard) = slot.lock() {
            *guard = strip_ansi(&buf);
        }
    });
    collected
}

fn take_stderr(proc: &AcpProc) -> Option<String> {
    for _ in 0..50 {
        let text = proc
            .stderr
            .lock()
            .ok()
            .map(|guard| guard.trim().to_string())
            .filter(|text| !text.is_empty());
        if text.is_some() {
            return text;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}

fn format_cli_exit(
    timed_out: bool,
    elapsed: Duration,
    code: Option<i32>,
    pid: u32,
    stderr: Option<&str>,
) -> String {
    let code_text = match code {
        Some(code) => format_exit_code(code),
        None => format!("still-running pid {pid}"),
    };
    let cause = if timed_out {
        format!(
            "握手超时（{}s 看门狗已 taskkill 进程树，exit {code_text}）",
            INIT_TIMEOUT.as_secs()
        )
    } else if elapsed < Duration::from_secs(2) && code.is_some() {
        format!("已立即退出（exit {code_text}）")
    } else if code.is_none() {
        format!("已停止输出但仍在运行（pid {pid}）")
    } else {
        format!("已退出（exit {code_text}）")
    };
    match stderr {
        Some(text) if !text.is_empty() => {
            format!("管理 CLI {cause}：{}", strip_ansi(text))
        }
        _ => format!("管理 CLI {cause}"),
    }
}

fn looks_like_tui(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    text.contains('\u{1b}')
        || text.contains('\u{9b}')
        || lower.contains("npm list")
        || lower.contains("\u{1b}]0;")
        || lower.contains("[?1049")
        || lower.contains("[2j")
}

pub(crate) fn strip_ansi(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == 0x1b {
            if i + 1 >= bytes.len() {
                break;
            }
            match bytes[i + 1] {
                b'[' => {
                    i += 2;
                    while i < bytes.len() && !(0x40..=0x7e).contains(&bytes[i]) {
                        i += 1;
                    }
                    i += 1;
                }
                b']' => {
                    i += 2;
                    while i < bytes.len() && bytes[i] != 0x07 {
                        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                    if i < bytes.len() && bytes[i] == 0x07 {
                        i += 1;
                    }
                }
                b'(' | b')' | b'#' => i += 3,
                _ => i += 2,
            }
            continue;
        }
        if b == 0x9b {
            i += 1;
            while i < bytes.len() && !(0x40..=0x7e).contains(&bytes[i]) {
                i += 1;
            }
            i += 1;
            continue;
        }
        out.push(b);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn format_exit_code(code: i32) -> String {
    const STATUS_DLL_INIT_FAILED: u32 = 0xC000_0142;
    if code as u32 == STATUS_DLL_INIT_FAILED {
        format!("{code} / 0xC0000142 STATUS_DLL_INIT_FAILED")
    } else {
        code.to_string()
    }
}

fn describe_cli_exit(proc: &mut AcpProc) -> String {
    let code = proc.child.wait_exit(Duration::from_millis(800));
    let stderr = take_stderr(proc);
    format_cli_exit(
        proc.timed_out.load(Ordering::SeqCst),
        proc.started.elapsed(),
        code,
        proc.child.id(),
        stderr.as_deref(),
    )
}

fn is_cli_exit_error(message: &str) -> bool {
    message.contains("已退出")
        || message.contains("立即退出")
        || message.contains("握手超时")
        || message.contains("仍在运行")
}

fn skill_env(skill_dir: &Path) -> Vec<(&str, &Path)> {
    if skill_dir.is_dir() {
        vec![
            ("SKILL_MANAGER_BUILTIN_SKILLS", skill_dir),
            // OpenCode 1.x 实测认 OPENCODE_CONFIG_DIR/skills/，不认 OPENCODE_SKILLS_PATH。
            ("OPENCODE_CONFIG_DIR", skill_dir),
        ]
    } else {
        Vec::new()
    }
}

fn spawn_stdout_reader(
    stdout: Box<dyn Read + Send>,
) -> mpsc::Receiver<std::io::Result<String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let _ = tx.send(Ok(String::new()));
                    break;
                }
                Ok(_) => {
                    if tx.send(Ok(line)).is_err() {
                        break;
                    }
                }
                Err(err) => {
                    let _ = tx.send(Err(err));
                    break;
                }
            }
        }
    });
    rx
}

fn acp_proc(
    child: CliChild,
    stdin: Box<dyn Write + Send>,
    stdout: Box<dyn Read + Send>,
    stderr: Arc<Mutex<String>>,
    timed_out: Arc<AtomicBool>,
    started: Instant,
) -> AcpProc {
    AcpProc {
        child,
        stdin: Arc::new(Mutex::new(stdin)),
        stdout_rx: spawn_stdout_reader(stdout),
        stderr,
        timed_out,
        started,
    }
}

#[cfg(windows)]
fn spawn_with_args(bin: &Path, args: &[&str], skill_dir: &Path) -> Result<AcpProc, String> {
    let bin = spawnable_cli_path(bin).ok_or_else(|| {
        format!(
            "无法启动管理 CLI：{} 不是可在本机拉起的程序（Windows 上应使用 .cmd，不能是无扩展 Unix 垫片）",
            bin.display()
        )
    })?;
    let (program, mut prefix) = windows_acp_invocation(&bin);
    prefix.extend(args.iter().map(|arg| std::ffi::OsString::from(*arg)));
    let extra = skill_env(skill_dir);
    let spawned = acp_win::spawn_hidden(&program, &prefix, &extra)
        .map_err(|err| format!("无法启动管理 CLI：{err}"))?;
    let stderr = collect_stderr_reader(spawned.stderr);
    Ok(acp_proc(
        CliChild::Hidden(spawned.child),
        Box::new(spawned.stdin),
        Box::new(spawned.stdout),
        stderr,
        Arc::new(AtomicBool::new(false)),
        Instant::now(),
    ))
}

#[cfg(not(windows))]
fn spawn_with_args(bin: &Path, args: &[&str], skill_dir: &Path) -> Result<AcpProc, String> {
    let bin = spawnable_cli_path(bin).ok_or_else(|| {
        format!(
            "无法启动管理 CLI：{} 不是可在本机拉起的程序（Windows 上应使用 .cmd，不能是无扩展 Unix 垫片）",
            bin.display()
        )
    })?;
    let mut cmd = cli_process_command(&bin);
    cmd.env("NO_COLOR", "1")
        .env("FORCE_COLOR", "0")
        .env("TERM", "dumb");
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in skill_env(skill_dir) {
        cmd.env(key, value);
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
    let stderr = child
        .stderr
        .take()
        .map(collect_stderr_reader)
        .unwrap_or_else(|| Arc::new(Mutex::new(String::new())));
    Ok(acp_proc(
        CliChild::Std(child),
        Box::new(stdin),
        Box::new(stdout),
        stderr,
        Arc::new(AtomicBool::new(false)),
        Instant::now(),
    ))
}

pub(crate) fn normalize_acp_args(args: &[String]) -> Vec<String> {
    if args.is_empty() || args == ["acp"] {
        vec!["--print-logs".into(), "acp".into()]
    } else {
        args.to_vec()
    }
}

fn candidate_arg_sets(cli: &DetectedCli) -> Vec<Vec<String>> {
    let mut sets = Vec::new();
    let stored = normalize_acp_args(&cli.acp_args);
    if !stored.is_empty() {
        sets.push(stored);
    }
    for set in default_acp_arg_sets(&cli.cli_id) {
        let owned: Vec<String> = set.iter().map(|s| (*s).to_string()).collect();
        if !sets.contains(&owned) {
            sets.push(owned);
        }
    }
    sets
}

/// OpenCode 文档是 `opencode --print-logs acp`。无参会进 TUI 并拉起 `npm list -g`，不要试。
/// `--acp` 只给别的 CLI；握手超时或 TUI 输出后不再试下一组。
pub(crate) fn default_acp_arg_sets(_cli_id: &str) -> Vec<&'static [&'static str]> {
    vec![&["--print-logs", "acp"], &["acp"], &["--acp"]]
}

fn should_retry_next_args(err: &str) -> bool {
    if err.contains("握手超时")
        || err.contains("STATUS_DLL_INIT_FAILED")
        || err.contains("0xC0000142")
        || looks_like_tui(err)
    {
        return false;
    }
    err.contains("Unknown command") || err.contains("立即退出")
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
        // taskkill 本身可 CREATE_NO_WINDOW；ACP CLI 不能。
        let _ = skill_manager_core::process::command("taskkill")
            .args(windows_taskkill_args(pid))
            .status();
    }
}

fn interrupt_child(child: &CliChild) {
    #[cfg(windows)]
    if let CliChild::Hidden(child) = child {
        child.hpcon.close();
    }
    kill_pid(child.id());
}

fn kill_child(child: &mut CliChild) {
    interrupt_child(child);
    match child {
        CliChild::Std(child) => {
            let _ = child.kill();
        }
        #[cfg(windows)]
        CliChild::Hidden(_) => {}
    }
    let _ = child.wait_exit(Duration::from_millis(800));
}

/// `read_line` 在 ConPTY/conhost 仍活着时会一直堵；超时必须先关 PTY 再 taskkill，读线程才能收到 EOF。
fn arm_handshake_watchdog(
    child: &CliChild,
    timed_out: Arc<AtomicBool>,
) -> (Arc<AtomicBool>, std::thread::JoinHandle<()>) {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    let pid = child.id();
    #[cfg(windows)]
    let pty = match child {
        CliChild::Hidden(child) => Some(Arc::clone(&child.hpcon)),
        CliChild::Std(_) => None,
    };
    let handle = std::thread::spawn(move || {
        let deadline = Instant::now() + INIT_TIMEOUT;
        while Instant::now() < deadline {
            if flag.load(Ordering::SeqCst) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        if !flag.load(Ordering::SeqCst) {
            timed_out.store(true, Ordering::SeqCst);
            #[cfg(windows)]
            if let Some(pty) = pty {
                pty.close();
            }
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
    let (stop, watchdog) = arm_handshake_watchdog(&proc.child, Arc::clone(&proc.timed_out));
    let result = request(
        proc,
        method,
        params,
        Instant::now() + INIT_TIMEOUT + Duration::from_secs(2),
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
    let mut tried = Vec::new();
    for args in candidate_arg_sets(cli) {
        tried.push(args.clone());
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
                if !should_retry_next_args(&last) {
                    break;
                }
            }
        }
    }
    Err(handshake_failure_message(&cli.binary_path, &last, &tried))
}

fn format_arg_set(args: &[String]) -> String {
    if args.is_empty() {
        "无参".into()
    } else {
        args.join(" ")
    }
}

fn handshake_failure_message(bin: &Path, last: &str, tried: &[Vec<String>]) -> String {
    if looks_like_auth_failure(&json!(last)) && !is_cli_exit_error(last) {
        "管理 CLI 尚未登录模型。请先在该 CLI 中完成登录，再回来对话。任务不会标成进行中。"
            .to_string()
    } else {
        let tried_text = if tried.is_empty() {
            "无可用参数".to_string()
        } else {
            tried
                .iter()
                .map(|args| format_arg_set(args))
                .collect::<Vec<_>>()
                .join(" / ")
        };
        format!(
            "无法与 {bin} 完成 ACP 握手（已试 {tried_text}）。{last}",
            bin = bin.display(),
            last = strip_ansi(last).trim()
        )
    }
}

/// 登记前探测：握手成功才返回实际可用的 ACP 参数；会关掉探测进程。
pub(crate) fn probe_acp(cli: &DetectedCli, skill_dir: &Path) -> Result<Vec<String>, String> {
    let cancel = AtomicBool::new(false);
    let (mut proc, args) = initialize_with_args(cli, skill_dir, "", &cancel)?;
    kill_child(&mut proc.child);
    Ok(args)
}

fn write_msg(stdin: &mut dyn Write, value: &Value) -> Result<(), String> {
    let line = format!("{value}\n");
    stdin
        .write_all(line.as_bytes())
        .and_then(|_| stdin.flush())
        .map_err(|err| format!("写入管理 CLI 失败：{err}"))
}

fn write_locked(stdin: &Arc<Mutex<Box<dyn Write + Send>>>, value: &Value) -> Result<(), String> {
    let mut guard = stdin
        .lock()
        .map_err(|_| "管理 CLI stdin 锁已损坏".to_string())?;
    write_msg(&mut **guard, value)
}

fn recv_line(proc: &mut AcpProc, until: Instant) -> Result<String, String> {
    loop {
        if Instant::now() > until {
            proc.timed_out.store(true, Ordering::SeqCst);
            kill_child(&mut proc.child);
            return Err(describe_cli_exit(proc));
        }
        let wait = until
            .saturating_duration_since(Instant::now())
            .min(Duration::from_millis(200));
        match proc.stdout_rx.recv_timeout(wait) {
            Ok(Ok(line)) => return Ok(line),
            Ok(Err(err)) => return Err(format!("读取管理 CLI 失败：{err}")),
            Err(RecvTimeoutError::Timeout) => {
                if proc.child.wait_exit(Duration::ZERO).is_some() {
                    return Err(describe_cli_exit(proc));
                }
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => return Err(describe_cli_exit(proc)),
        }
    }
}

fn read_msg(proc: &mut AcpProc, until: Instant) -> Result<Value, String> {
    loop {
        let header = recv_line(proc, until)?;
        if header.is_empty() {
            return Err(describe_cli_exit(proc));
        }
        let trimmed = strip_ansi(&header);
        let trimmed = trimmed.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            let len: usize = rest
                .trim()
                .parse()
                .map_err(|_| "无效的 Content-Length".to_string())?;
            let blank = recv_line(proc, until)?;
            let _ = blank;
            let mut body = String::new();
            while body.len() < len {
                body.push_str(&recv_line(proc, until)?);
            }
            return serde_json::from_str(body.trim()).map_err(|err| err.to_string());
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            return Ok(value);
        }
        if let Ok(mut guard) = proc.stderr.lock() {
            if !guard.is_empty() && !guard.ends_with('\n') {
                guard.push('\n');
            }
            guard.push_str(&strip_ansi(trimmed));
            guard.push('\n');
        }
    }
}

fn looks_like_auth_failure(value: &Value) -> bool {
    if value.get("protocolVersion").is_some() && value.get("error").is_none() {
        return false;
    }
    let text = match value.get("error") {
        Some(err) => err.to_string().to_lowercase(),
        None => value.to_string().to_lowercase(),
    };
    text.contains("unauthenticated")
        || text.contains("auth_required")
        || text.contains("not logged")
        || text.contains("api key")
        || text.contains("api_key")
        || (text.contains("login") && text.contains("error"))
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
        let msg = read_msg(proc, until)?;
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

fn child_alive(child: &mut CliChild) -> bool {
    child.wait_exit(Duration::ZERO).is_none()
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
    fn process_exit_is_not_reported_as_missing_login() {
        let last = "管理 CLI 已退出（exit 1）：Error: Unknown command acp";
        let msg = handshake_failure_message(
            Path::new(r"C:\qa-host-a\opencode.exe"),
            last,
            &[vec!["--print-logs".into(), "acp".into()]],
        );
        assert!(msg.contains("exit 1"), "{msg}");
        assert!(msg.contains("Unknown command") || msg.contains("已退出"), "{msg}");
        assert!(
            !msg.contains("尚未登录"),
            "process death must not be blamed on missing login: {msg}"
        );
    }

    #[test]
    fn windows_acp_exe_is_not_spawned_via_cmd_wrapper() {
        let exe = Path::new(r"C:\qa-host-a\node_modules\opencode-windows-x64\bin\opencode.exe");
        let (program, args) = windows_acp_invocation(exe);
        assert_eq!(program, exe.as_os_str());
        assert!(args.is_empty());
        let cmd = cli_process_command(exe);
        let program = cmd.get_program().to_string_lossy();
        assert!(
            program.contains("opencode.exe"),
            "ACP exe must be the program, not cmd.exe, got {program}"
        );
        assert!(
            !program.to_ascii_lowercase().contains("cmd"),
            "CREATE_NO_WINDOW cmd wrapper makes Bun close stdin, got {program}"
        );
    }

    #[test]
    fn windows_acp_hides_console_without_create_no_window() {
        let (dw_flags, show, creation) = windows_acp_create_flags();
        assert_eq!(show, ACP_SW_HIDE);
        assert_eq!(
            dw_flags & ACP_STARTF_USESHOWWINDOW,
            ACP_STARTF_USESHOWWINDOW
        );
        assert_eq!(
            dw_flags & ACP_STARTF_USESTDHANDLES,
            ACP_STARTF_USESTDHANDLES,
            "ACP NDJSON must use pipes; ConPTY as stdio makes OpenCode emit TUI"
        );
        assert_eq!(
            ACP_CONPTY_INHERIT_HANDLES,
            1,
            "child must inherit the ACP std pipes"
        );
        assert_eq!(
            creation & ACP_CREATE_NO_WINDOW,
            0,
            "CREATE_NO_WINDOW makes Bun close ACP stdin"
        );
        assert_eq!(
            creation & ACP_CREATE_NEW_CONSOLE,
            0,
            "CREATE_NEW_CONSOLE steals stdio onto an empty hidden console"
        );
        assert_eq!(
            creation & ACP_EXTENDED_STARTUPINFO_PRESENT,
            ACP_EXTENDED_STARTUPINFO_PRESENT,
            "ConPTY needs STARTUPINFOEX"
        );
    }

    #[test]
    fn stored_acp_args_are_print_logs_acp() {
        assert_eq!(
            normalize_acp_args(&["acp".into()]),
            vec!["--print-logs", "acp"]
        );
        let cli = DetectedCli {
            cli_id: "opencode".into(),
            binary_path: PathBuf::from("opencode"),
            installed_by_app: true,
            acp_args: vec!["acp".into()],
            user_selected: false,
        };
        assert_eq!(
            candidate_arg_sets(&cli)[0],
            vec!["--print-logs", "acp"]
        );
    }

    #[test]
    fn timeout_does_not_try_remaining_arg_sets() {
        assert!(!should_retry_next_args(
            "管理 CLI 握手超时（12s 看门狗已 taskkill 进程树，exit 1）"
        ));
        assert!(!should_retry_next_args(
            "管理 CLI 已立即退出（exit -1073741502 / 0xC0000142 STATUS_DLL_INIT_FAILED）"
        ));
        assert!(should_retry_next_args(
            "管理 CLI 已立即退出（exit 1）：Unknown command acp"
        ));
        assert!(!should_retry_next_args(
            "管理 CLI 已立即退出（exit 1）：\u{1b}[2J\u{1b}]0;OpenCode\u{07}"
        ));
    }

    #[test]
    fn handshake_timeout_reports_watchdog_not_unknown() {
        let last = format_cli_exit(true, INIT_TIMEOUT, Some(1), 34160, None);
        assert!(last.contains("握手超时"), "{last}");
        assert!(last.contains("taskkill"), "{last}");
        assert!(last.contains("exit 1"), "{last}");
        assert!(!last.contains("unknown"), "{last}");
        let msg = handshake_failure_message(
            Path::new(r"C:\qa-host-a\opencode.exe"),
            &last,
            &[vec!["--print-logs".into(), "acp".into()]],
        );
        assert!(msg.contains("握手超时"), "{msg}");
        assert!(msg.contains("--print-logs acp"), "{msg}");
        assert!(
            !msg.contains("--acp"),
            "timeout must not claim --acp was tried: {msg}"
        );
        assert!(
            !msg.contains("无参"),
            "timeout must not claim empty args were tried: {msg}"
        );
        assert!(
            !msg.contains("尚未登录"),
            "timeout must not be blamed on missing login: {msg}"
        );
        assert!(is_cli_exit_error(&last));
    }

    #[test]
    fn strip_ansi_drops_csi_and_osc() {
        let raw = "\u{1b}[2J\u{1b}[?1049h\u{1b}]0;OpenCode\u{07}{\"jsonrpc\":\"2.0\"}\n";
        let cleaned = strip_ansi(raw);
        assert!(cleaned.contains("jsonrpc"), "{cleaned:?}");
        assert!(!cleaned.contains('\u{1b}'), "{cleaned:?}");
        let msg = handshake_failure_message(
            Path::new(r"C:\qa-host-a\opencode.exe"),
            raw,
            &[vec!["--print-logs".into(), "acp".into()]],
        );
        assert!(msg.contains("--print-logs acp"), "{msg}");
        assert!(!msg.contains('\u{1b}'), "{msg}");
        assert!(!msg.contains("--acp"), "{msg}");
        assert!(!msg.contains("无参"), "{msg}");
    }

    #[test]
    fn dll_init_failure_is_not_unknown_or_login() {
        let last = format_cli_exit(
            false,
            Duration::from_millis(80),
            Some(-1073741502),
            9,
            None,
        );
        assert!(last.contains("0xC0000142"), "{last}");
        assert!(last.contains("STATUS_DLL_INIT_FAILED"), "{last}");
        assert!(last.contains("立即"), "{last}");
        let msg = handshake_failure_message(
            Path::new(r"C:\qa-host-a\opencode.exe"),
            &last,
            &[vec!["--print-logs".into(), "acp".into()]],
        );
        assert!(msg.contains("--print-logs acp"), "{msg}");
        assert!(!msg.contains("--acp"), "{msg}");
        assert!(!msg.contains("尚未登录"), "{msg}");
        assert!(is_cli_exit_error(&last));
    }

    #[test]
    fn immediate_exit_reports_code_and_stderr() {
        let last = format_cli_exit(
            false,
            Duration::from_millis(80),
            Some(1),
            9,
            Some("Error: Unknown command acp"),
        );
        assert!(last.contains("立即"), "{last}");
        assert!(last.contains("exit 1"), "{last}");
        assert!(last.contains("Unknown command"), "{last}");
        assert!(!last.contains("unknown"), "{last}");
        assert!(is_cli_exit_error(&last));
        let msg = handshake_failure_message(
            Path::new(r"C:\qa-host-a\opencode.exe"),
            &last,
            &[vec!["--print-logs".into(), "acp".into()]],
        );
        assert!(
            !msg.contains("尚未登录"),
            "immediate exit must not be blamed on missing login: {msg}"
        );
    }

    #[test]
    fn still_running_after_eof_is_not_unknown() {
        let last = format_cli_exit(false, Duration::from_secs(12), None, 34160, None);
        assert!(last.contains("34160"), "{last}");
        assert!(last.contains("仍在运行"), "{last}");
        assert!(!last.contains("unknown"), "{last}");
        assert!(is_cli_exit_error(&last));
    }

    #[test]
    fn auth_error_text_is_detected() {
        assert!(looks_like_auth_failure(
            &json!({"error": "unauthenticated"})
        ));
        assert!(!looks_like_auth_failure(&json!({"ok": true})));
        assert!(
            !looks_like_auth_failure(&json!({
                "protocolVersion": 1,
                "authMethods": [{ "id": "opencode-login", "name": "Login with opencode" }]
            })),
            "OpenCode always lists opencode-login; that is not a failed handshake"
        );
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

    #[cfg(windows)]
    #[test]
    fn conpty_attribute_passes_hpcon_value_not_address() {
        let hpcon = 0x1234usize as *mut core::ffi::c_void;
        assert_eq!(
            acp_win::conpty_attribute_lp_value(hpcon),
            hpcon,
            "MSDN sample passes hPC, not &hPC; &hPC yields 0xC0000142"
        );
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
