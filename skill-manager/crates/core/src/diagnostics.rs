//! 诊断与运行日志（T085，FR-053；技术参考 diagnostics-and-documents.md）。
//!
//! 提供三层可获取事实：**运行日志**（操作/错误与时间）、**崩溃记录**（panic 线索）与
//! **诊断导出**（版本、平台、有关路径、最近日志与崩溃，写出真实文件供 Agent 排查）。
//! 边界：由用户主动获取；不自动上传；导出前对 home 路径、URL userinfo、常见凭据参数与
//! 令牌前缀做替换，减少把真实凭据一起交出的后果。日志落在本机状态目录，不随 SkillManagerVault 同步。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::context::AppContext;
use crate::error::{CoreError, Result};
use crate::state::StateStore;

/// 日志目录名（本机状态目录下）。
pub const LOG_DIR_NAME: &str = "logs";
/// 运行日志文件名。
pub const LOG_FILE: &str = "skill-manager.log";
/// 崩溃记录文件名。
pub const CRASH_FILE: &str = "crash.log";

/// 日志目录固定为 `<状态目录>/logs`（不再保留可写设置键）。
///
/// 内存状态库（仅测试）没有目录，返回空路径；`record` 对空路径不落文件。
pub fn log_dir(store: &StateStore) -> PathBuf {
    if store.dir().as_os_str().is_empty() {
        return PathBuf::new();
    }
    store.dir().join(LOG_DIR_NAME)
}

/// 运行日志路径。
pub fn log_path(store: &StateStore) -> PathBuf {
    log_dir(store).join(LOG_FILE)
}

/// 崩溃记录路径。
pub fn crash_path(store: &StateStore) -> PathBuf {
    log_dir(store).join(CRASH_FILE)
}

/// 一条诊断记录（日志或崩溃）。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogEntry {
    pub at: String,
    /// `info` / `warn` / `error` / `panic` / `raw`。
    pub level: String,
    /// 操作名（例如 `deploy`、`acquire`、`scheduler`）。
    pub action: String,
    pub message: String,
}

/// 记录一条运行日志；写入前按 [`sanitize`] 替换敏感内容。
pub fn record(
    store: &StateStore,
    level: &str,
    action: &str,
    message: &str,
) -> Result<()> {
    let dir = log_dir(store);
    if dir.as_os_str().is_empty() {
        return Ok(());
    }
    let entry = LogEntry {
        at: now(),
        level: level.to_string(),
        action: action.to_string(),
        message: sanitize(message, None),
    };
    append(&dir.join(LOG_FILE), &entry)
}

/// 记录一条崩溃线索（供 panic hook 使用）。
pub fn record_crash_at(dir: &Path, message: &str) -> Result<()> {
    if dir.as_os_str().is_empty() {
        return Ok(());
    }
    let entry = LogEntry {
        at: now(),
        level: "panic".to_string(),
        action: "panic".to_string(),
        message: sanitize(message, None),
    };
    append(&dir.join(CRASH_FILE), &entry)
}

fn append(path: &Path, entry: &LogEntry) -> Result<()> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| CoreError::io(parent, err))?;
    }
    let line = serde_json::to_string(entry)
        .map_err(|err| CoreError::other(format!("序列化诊断记录失败：{err}")))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| CoreError::io(path, err))?;
    writeln!(file, "{line}").map_err(|err| CoreError::io(path, err))
}

/// 读取最近 `limit` 条记录（新记录在后）；文件不存在返回空。
pub fn read_recent(path: &Path, limit: usize) -> Result<Vec<LogEntry>> {
    let bytes = match crate::manager::read_optional(path)? {
        Some(bytes) => bytes,
        None => return Ok(Vec::new()),
    };
    let text = String::from_utf8_lossy(&bytes);
    let mut entries: Vec<LogEntry> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<LogEntry>(line).unwrap_or_else(|_| LogEntry {
                at: String::new(),
                level: "raw".to_string(),
                action: "log".to_string(),
                message: line.to_string(),
            })
        })
        .collect();
    if entries.len() > limit {
        entries.drain(..entries.len() - limit);
    }
    Ok(entries)
}

/// 诊断报告（用户主动获取）。
#[derive(Clone, Debug, Serialize)]
pub struct DiagnosticsReport {
    pub generated_at: String,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub vault: Option<String>,
    pub state_dir: String,
    pub log_path: String,
    pub crash_path: String,
    pub last_check_at: Option<String>,
    pub last_sync_at: Option<String>,
    pub auto_check_minutes: i64,
    pub theme: String,
    pub font_scale: String,
    /// 最近的运行日志（脱敏后）。
    pub recent_logs: Vec<LogEntry>,
    /// 崩溃记录（脱敏后）。
    pub crashes: Vec<LogEntry>,
    /// 边界说明；不包含任何凭据。
    pub notes: Vec<String>,
}

/// 诊断导出结果：写出的真实文件路径与同内容报告。
#[derive(Clone, Debug, Serialize)]
pub struct DiagnosticsExport {
    pub path: String,
    pub report: DiagnosticsReport,
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// 组装诊断报告；日志/崩溃各取最近 `recent` 条。
pub fn report(ctx: &AppContext, recent: usize) -> Result<DiagnosticsReport> {
    let store = ctx.state();
    let home = ctx.home().ok();
    let log = log_path(store);
    let crash = crash_path(store);

    let mut recent_logs = read_recent(&log, recent)?;
    for entry in &mut recent_logs {
        entry.message = sanitize(&entry.message, home.as_deref());
    }
    let mut crashes = read_recent(&crash, recent)?;
    for entry in &mut crashes {
        entry.message = sanitize(&entry.message, home.as_deref());
    }

    Ok(DiagnosticsReport {
        generated_at: now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        vault: ctx
            .vault_opt()
            .map(|vault| sanitize(&vault.root().display().to_string(), home.as_deref())),
        state_dir: sanitize(&store.dir().display().to_string(), home.as_deref()),
        log_path: sanitize(&log.display().to_string(), home.as_deref()),
        crash_path: sanitize(&crash.display().to_string(), home.as_deref()),
        last_check_at: store.last_check_at()?,
        last_sync_at: store.last_sync_at()?,
        auto_check_minutes: store.auto_check_minutes()?,
        theme: store.theme()?,
        font_scale: store.font_scale()?,
        recent_logs,
        crashes,
        notes: vec![
            "诊断资料由用户主动获取，不会自动上传。".to_string(),
            "已对 home 路径、URL 中的账号信息、常见凭据参数与令牌前缀做替换。".to_string(),
            "替换覆盖常见形态，不构成完整凭据检测；分享前请自行确认。".to_string(),
        ],
    })
}

/// 导出诊断报告到日志目录下的真实文件，返回文件路径。
pub fn export(ctx: &AppContext, recent: usize) -> Result<DiagnosticsExport> {
    let report = report(ctx, recent)?;
    let dir = log_dir(ctx.state());
    if dir.as_os_str().is_empty() {
        return Err(CoreError::other(
            "本机状态目录不可用，无法导出诊断资料",
        ));
    }
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("diagnostics-{stamp}.txt"));
    let text = render(&report);
    std::fs::create_dir_all(&dir).map_err(|err| CoreError::io(dir.clone(), err))?;
    std::fs::write(&path, text.as_bytes()).map_err(|err| CoreError::io(path.clone(), err))?;
    Ok(DiagnosticsExport {
        path: path.display().to_string(),
        report,
    })
}

/// 报告的可读文本形式（导出文件内容）。
fn render(report: &DiagnosticsReport) -> String {
    let mut out = String::new();
    out.push_str("Skill Manager 诊断资料\n");
    out.push_str(&format!("生成时间：{}\n", report.generated_at));
    out.push_str(&format!("版本：{}\n", report.version));
    out.push_str(&format!("平台：{} / {}\n", report.os, report.arch));
    out.push_str(&format!(
        "SkillManagerVault：{}\n",
        report.vault.as_deref().unwrap_or("（尚未接入）")
    ));
    out.push_str(&format!("状态目录：{}\n", report.state_dir));
    out.push_str(&format!("运行日志：{}\n", report.log_path));
    out.push_str(&format!("崩溃记录：{}\n", report.crash_path));
    out.push_str(&format!(
        "最近检查时间：{}\n",
        report.last_check_at.as_deref().unwrap_or("（无记录）")
    ));
    out.push_str(&format!(
        "最近同步时间：{}\n",
        report.last_sync_at.as_deref().unwrap_or("（无记录）")
    ));
    out.push_str(&format!("自动检查频率：{} 分钟\n", report.auto_check_minutes));
    out.push_str(&format!("主题：{}\n", report.theme));
    out.push_str(&format!("字号：{}\n", report.font_scale));

    out.push_str("\n最近运行日志\n");
    if report.recent_logs.is_empty() {
        out.push_str("（无记录）\n");
    }
    for entry in &report.recent_logs {
        out.push_str(&format!(
            "[{}] {} {}：{}\n",
            entry.at, entry.level, entry.action, entry.message
        ));
    }

    out.push_str("\n崩溃记录\n");
    if report.crashes.is_empty() {
        out.push_str("（无记录）\n");
    }
    for entry in &report.crashes {
        out.push_str(&format!("[{}] {}\n", entry.at, entry.message));
    }

    out.push_str("\n说明\n");
    for note in &report.notes {
        out.push_str(&format!("- {note}\n"));
    }
    out
}

// ---------------------------------------------------------------------------
// 脱敏
// ---------------------------------------------------------------------------

/// 需要替换值的常见凭据参数名（小写）。
const SECRET_KEYS: [&str; 10] = [
    "token",
    "access_token",
    "refresh_token",
    "password",
    "passwd",
    "secret",
    "client_secret",
    "api_key",
    "apikey",
    "auth",
];

/// 常见令牌前缀；需处于词首边界才替换，避免误伤普通词汇。
const TOKEN_PREFIXES: [&str; 8] = [
    "ghp_",
    "gho_",
    "github_pat_",
    "glpat-",
    "xoxb-",
    "xoxp-",
    "sk-",
    "AKIA",
];

/// 替换文本中的敏感片段：home 路径 → `~`；URL userinfo → `***`；
/// `key=value` 形式的凭据参数值与令牌前缀后的串 → `***`。
pub fn sanitize(text: &str, home: Option<&Path>) -> String {
    let mut out = text.to_string();
    if let Some(home) = home {
        let home = home.to_string_lossy();
        if !home.is_empty() && home != "/" {
            out = out.replace(home.as_ref(), "~");
        }
    }
    out = redact_url_userinfo(&out);
    out = redact_secret_params(&out);
    redact_token_prefixes(&out)
}

fn redact_url_userinfo(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(position) = rest.find("://") {
        out.push_str(&rest[..position + 3]);
        let after = &rest[position + 3..];
        let end = after
            .find(|ch: char| matches!(ch, '@' | '/' | ' ' | '\n' | '\t' | '"' | '\''))
            .unwrap_or(after.len());
        let authority = &after[..end];
        if after[end..].starts_with('@') && authority.contains(':') {
            out.push_str("***");
        } else {
            out.push_str(authority);
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

fn redact_secret_params(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<char> = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '=' {
            let mut start = i;
            while start > 0 && is_key_char(chars[start - 1]) {
                start -= 1;
            }
            let key: String = chars[start..i].iter().collect::<String>().to_lowercase();
            if SECRET_KEYS.contains(&key.as_str()) {
                out.truncate(out.len() - (i - start));
                out.extend(chars[start..i].iter().copied());
                out.push('=');
                out.extend("***".chars());
                i += 1;
                while i < chars.len() && !is_value_delimiter(chars[i]) {
                    i += 1;
                }
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out.into_iter().collect()
}

fn redact_token_prefixes(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        // 词首边界：避免把 task-1、desk-notes 这类普通词误判为令牌。
        let at_boundary = i == 0 || !is_word_char(chars[i - 1]);
        if at_boundary {
            if let Some(prefix) = TOKEN_PREFIXES
                .iter()
                .find(|prefix| matches_prefix(&chars, i, prefix))
            {
                out.push_str(prefix);
                i += prefix.chars().count();
                while i < chars.len() && is_word_char(chars[i]) {
                    i += 1;
                }
                out.push_str("***");
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn matches_prefix(chars: &[char], at: usize, prefix: &str) -> bool {
    let mut index = at;
    for expected in prefix.chars() {
        if chars.get(index) != Some(&expected) {
            return false;
        }
        index += 1;
    }
    true
}

fn is_key_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

fn is_value_delimiter(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '&' | ',' | ';' | '"' | '\'' | ')' | ']' | '}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with_logs() -> (tempfile::TempDir, StateStore) {
        let dir = tempfile::TempDir::new().unwrap();
        let store = StateStore::open(dir.path()).unwrap();
        (dir, store)
    }

    #[test]
    fn records_and_reads_recent_logs_in_order() {
        let (_dir, store) = store_with_logs();
        for index in 0..5 {
            record(&store, "error", "deploy", &format!("第 {index} 项失败")).unwrap();
        }
        let recent = read_recent(&log_path(&store), 2).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[1].message, "第 4 项失败");
        assert_eq!(recent[1].level, "error");
    }

    #[test]
    fn crash_records_are_separate_from_run_logs() {
        let (dir, store) = store_with_logs();
        record_crash_at(&log_dir(&store), "部署时 panic：索引为 0").unwrap();
        assert!(crash_path(&store).exists());
        assert!(!log_path(&store).exists());
        let crashes = read_recent(&crash_path(&store), 10).unwrap();
        assert_eq!(crashes.len(), 1);
        assert_eq!(crashes[0].level, "panic");
        assert!(crashes[0].message.contains("索引为 0"));
        assert!(dir.path().join("logs/crash.log").exists());
    }

    #[test]
    fn sanitize_replaces_paths_userinfo_params_and_tokens() {
        let home = Path::new("/Users/demo");
        let text = "读取 /Users/demo/.zcode/skills 失败；远端 https://user:pw@example.invalid/repo.git；\
                   地址示例 https://example.invalid/pub；token=abc123&x=1；Bearer ghp_abcdefghijklmnop";
        let out = sanitize(text, Some(home));
        assert!(out.contains("~/.zcode/skills"), "{out}");
        assert!(out.contains("https://***@example.invalid/repo.git"), "{out}");
        assert!(out.contains("https://example.invalid/pub"), "{out}");
        assert!(out.contains("token=***"), "{out}");
        assert!(out.contains("ghp_***"), "{out}");
        assert!(!out.contains("abc123"), "{out}");
        assert!(!out.contains("ghp_abcdefghijklmnop"), "{out}");
    }

    #[test]
    fn sanitize_does_not_damage_plain_words() {
        let out = sanitize("task-1234 /tmp/desk-notes 已完成", None);
        assert_eq!(out, "task-1234 /tmp/desk-notes 已完成");
    }

    #[test]
    fn in_memory_store_does_not_write_files() {
        let store = StateStore::open_in_memory().unwrap();
        assert!(log_dir(&store).as_os_str().is_empty());
        record(&store, "info", "test", "不会落文件").unwrap();
    }
}
