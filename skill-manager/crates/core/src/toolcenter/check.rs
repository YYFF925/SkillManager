//! 工具版本检查（T077，FR-046；data-model §1.5 / §2.4a）。
//!
//! 执行**单个已登记工具**的 `check_command`（工作目录为该工具资料目录），把标准输出当作
//! **上游最新版本**字符串，与本机安装记录对比。软件只读取与对比，不为来源类型增加专用
//! 查询器，也不访问第二条查询路径。检查**不安装、不更新、不改本机安装事实**，只写检查结果。
//!
//! 无法比较时（见 `compare_versions`）如实说明原因，不冒充“最新”或“可更新”；未安装工具
//! 只显示上游版本，不派生“本机最新/可更新”。
//!
//! “检查”是 FR-029 共用周期的一部分：`check_all` 供统一调度调用。

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::error::{CoreError, Result};
use crate::state::{CheckStatus, StateStore, ToolCheckRecord};
use crate::toolcenter::files::{self, ToolFile};

/// 本机安装版本与检查所得上游版本的对比结论。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InstallComparison {
    /// 两侧可比较且相等：本机已是检查所得的最新版本。
    Latest,
    /// 上游版本高于本机安装版本。
    Updatable,
    /// 不能得出确定结论（无法比较，或上游低于本机记录）。
    Unknown { reason: String },
}

impl InstallComparison {
    pub fn kind(&self) -> &'static str {
        match self {
            InstallComparison::Latest => "latest",
            InstallComparison::Updatable => "updatable",
            InstallComparison::Unknown { .. } => "unknown",
        }
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            InstallComparison::Unknown { reason } => Some(reason),
            _ => None,
        }
    }
}

/// 版本串切分后的一个段。
#[derive(Clone, Debug, PartialEq, Eq)]
enum Segment {
    Num(u128),
    Text(String),
}

/// 规范化：去空白、去掉可选先导 `v`/`V`，再切成“非数字段 / 数字段”交替序列。
fn segments(raw: &str) -> Vec<Segment> {
    let trimmed = raw.trim();
    let trimmed = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    let mut out = Vec::new();
    let mut buffer = String::new();
    let mut buffer_is_digit = false;
    for ch in trimmed.chars() {
        let is_digit = ch.is_ascii_digit();
        if !buffer.is_empty() && is_digit != buffer_is_digit {
            out.push(finish(&buffer, buffer_is_digit));
            buffer.clear();
        }
        buffer_is_digit = is_digit;
        buffer.push(ch);
    }
    if !buffer.is_empty() {
        out.push(finish(&buffer, buffer_is_digit));
    }
    out
}

fn finish(text: &str, is_digit: bool) -> Segment {
    if is_digit {
        // 过长数字段按 u128 饱和处理：仍能比较大小，不因溢出误判。
        Segment::Num(text.parse::<u128>().unwrap_or(u128::MAX))
    } else {
        Segment::Text(text.to_lowercase())
    }
}

/// 比较两个版本字符串；返回 `None` 表示**无法比较**（形状不同、空值等）。
///
/// 约定（T074）：两侧规范化后段数与每段类型都相同、且**非数字段（分隔符/前缀）逐段一致**
/// 时才比较——只有数字段参与大小比较（按数值），因此 `1.2.3` 与 `1.2.4` 可比，
/// 而 `1.2.3` 与 `2024-10-15`（分隔符不同）不猜测。
pub fn compare_order(installed: &str, upstream: &str) -> Option<std::cmp::Ordering> {
    let left = segments(installed);
    let right = segments(upstream);
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return None;
    }
    let mut result = std::cmp::Ordering::Equal;
    for (a, b) in left.iter().zip(right.iter()) {
        // 先整体校验形状（段类型与分隔符逐段一致），再比较数字段；否则第一个数字段
        // 就会提前返回、跳过形状校验。
        match (a, b) {
            (Segment::Num(a), Segment::Num(b)) => {
                if result == std::cmp::Ordering::Equal {
                    result = a.cmp(b);
                }
            }
            // 非数字段必须一致：不同来源的命名风格不互相猜测大小。
            (Segment::Text(a), Segment::Text(b)) if a == b => {}
            // 对应位置类型不同或分隔符不同：不猜测。
            _ => return None,
        }
    }
    Some(result)
}

/// 依据安装版本与检查所得上游版本派生结论；调用方须已确认两者都存在。
pub fn compare_versions(installed: &str, upstream: &str) -> InstallComparison {
    match compare_order(installed, upstream) {
        Some(std::cmp::Ordering::Equal) => InstallComparison::Latest,
        Some(std::cmp::Ordering::Less) => InstallComparison::Updatable,
        Some(std::cmp::Ordering::Greater) => InstallComparison::Unknown {
            reason: format!(
                "检查所得上游版本（{upstream}）低于本机安装版本（{installed}），不判定为可更新"
            ),
        },
        None => InstallComparison::Unknown {
            reason: format!(
                "版本字符串无法比较：本机 {installed}，上游 {upstream}；软件不按来源类型猜测版本顺序"
            ),
        },
    }
}

/// 有效检查状态：有记录用记录值；无记录时，未登记命令表达为 `no_command`，否则 `unchecked`。
pub fn effective_status(has_command: bool, stored: Option<CheckStatus>) -> CheckStatus {
    match stored {
        Some(status) => status,
        None if has_command => CheckStatus::Unchecked,
        None => CheckStatus::NoCommand,
    }
}

/// 一次工具检查的结果。
#[derive(Clone, Debug, Serialize)]
pub struct ToolCheckOutcome {
    pub slug: String,
    pub name: String,
    /// 有效检查状态（`unchecked`/`checking`/`checked`/`failed`/`no_command`）。
    pub status: String,
    /// 成功获取的上游最新版本。
    pub latest_version: Option<String>,
    pub checked_at: Option<String>,
    /// 命令结果或失败原因。
    pub output: Option<String>,
    /// 本机安装版本；未安装为 `None`。
    pub installed_version: Option<String>,
    /// 已安装且可比较时的派生结论；未安装为 `None`。
    pub comparison: Option<InstallComparison>,
}

/// 当前时间（RFC 3339）。
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// 按平台构造执行 `check_command` 的 shell。
fn shell_command(command: &str) -> Command {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(command);
        cmd
    }
    #[cfg(not(windows))]
    {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd
    }
}

fn outcome_from(
    slug: &str,
    name: &str,
    status: CheckStatus,
    latest_version: Option<String>,
    checked_at: Option<String>,
    output: Option<String>,
    installed_version: Option<String>,
) -> ToolCheckOutcome {
    let comparison = match (&installed_version, &latest_version) {
        (Some(installed), Some(upstream)) if status == CheckStatus::Checked => {
            Some(compare_versions(installed, upstream))
        }
        _ => None,
    };
    ToolCheckOutcome {
        slug: slug.to_string(),
        name: name.to_string(),
        status: status.as_str().to_string(),
        latest_version,
        checked_at,
        output,
        installed_version,
        comparison,
    }
}

/// 执行单个已登记工具的检查。
///
/// 不安装、不更新、不改本机安装事实；只写该工具的检查结果（`tool_checks`）。
pub fn check_tool(root: &Path, state: &StateStore, slug: &str) -> Result<ToolCheckOutcome> {
    let file = ToolFile::load(root, slug)?;
    let meta = file.meta()?;
    meta.validate()?;
    let dir = files::tool_dir(root, slug);
    let installed = state
        .tool_install(slug)?
        .map(|record| record.version)
        .filter(|version| !version.trim().is_empty());

    let Some(command) = meta.check_command.clone() else {
        let checked_at = now_rfc3339();
        state.set_tool_check(&ToolCheckRecord {
            tool_slug: slug.to_string(),
            status: CheckStatus::NoCommand,
            latest_version: None,
            checked_at: Some(checked_at.clone()),
            check_output: Some("未登记检查命令".to_string()),
        })?;
        return Ok(outcome_from(
            slug,
            &meta.name,
            CheckStatus::NoCommand,
            None,
            Some(checked_at),
            Some("未登记检查命令".to_string()),
            installed,
        ));
    };

    // 标记“检查中”，让定时/手动检查进行中的界面状态诚实。
    let started = now_rfc3339();
    state.set_tool_check(&ToolCheckRecord {
        tool_slug: slug.to_string(),
        status: CheckStatus::Checking,
        latest_version: None,
        checked_at: Some(started),
        check_output: None,
    })?;

    let output = shell_command(&command)
        .current_dir(&dir)
        .output()
        .map_err(|err| CoreError::io(dir.clone(), err))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let checked_at = now_rfc3339();

    let failed = !output.status.success() || stdout.is_empty();
    let note = if failed {
        let mut text = format!("退出码：{}", output.status.code().unwrap_or(-1));
        if !stderr.is_empty() {
            text.push_str(&format!("；标准错误：{stderr}"));
        }
        // 说明性输出走标准错误，因此这里不把 stderr 当作版本。
        text
    } else if stderr.is_empty() {
        "检查成功".to_string()
    } else {
        format!("检查成功；标准错误：{stderr}")
    };

    let status = if failed {
        CheckStatus::Failed
    } else {
        CheckStatus::Checked
    };
    let latest = if failed { None } else { Some(stdout) };

    state.set_tool_check(&ToolCheckRecord {
        tool_slug: slug.to_string(),
        status,
        latest_version: latest.clone(),
        checked_at: Some(checked_at.clone()),
        check_output: Some(note.clone()),
    })?;

    Ok(outcome_from(
        slug,
        &meta.name,
        status,
        latest,
        Some(checked_at),
        Some(note),
        installed,
    ))
}

/// 检查全部已登记工具；单项失败如实记录并继续，不中断其余工具。
pub fn check_all(root: &Path, state: &StateStore) -> Result<Vec<ToolCheckOutcome>> {
    let mut out = Vec::new();
    for slug in files::registered_slugs(root)? {
        match check_tool(root, state, &slug) {
            Ok(outcome) => out.push(outcome),
            Err(err) => {
                // 资料损坏时该条单独失败，其余工具继续可查。
                let message = err.to_string();
                state.set_tool_check(&ToolCheckRecord {
                    tool_slug: slug.clone(),
                    status: CheckStatus::Failed,
                    latest_version: None,
                    checked_at: Some(now_rfc3339()),
                    check_output: Some(message.clone()),
                })?;
                out.push(ToolCheckOutcome {
                    slug,
                    name: String::new(),
                    status: CheckStatus::Failed.as_str().to_string(),
                    latest_version: None,
                    checked_at: None,
                    output: Some(message),
                    installed_version: None,
                    comparison: None,
                });
            }
        }
    }
    Ok(out)
}

/// 工具资料目录（供任务文本与界面定位）。
pub fn tool_dir_of(root: &Path, slug: &str) -> PathBuf {
    files::tool_dir(root, slug)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ToolInstallRecord;

    fn write_tool(root: &Path, slug: &str, body: &str) {
        let path = files::tool_path(root, slug);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn numeric_versions_compare() {
        assert_eq!(
            compare_versions("14.1.0", "14.1.1"),
            InstallComparison::Updatable
        );
        assert_eq!(
            compare_versions("0.12.0", "0.12.0"),
            InstallComparison::Latest
        );
        assert_eq!(
            compare_versions("v1.10", "1.9"),
            InstallComparison::Unknown {
                reason: "检查所得上游版本（1.9）低于本机安装版本（v1.10），不判定为可更新"
                    .to_string()
            }
        );
    }

    #[test]
    fn incomparable_versions_do_not_pretend_a_conclusion() {
        let comparison = compare_versions("7.0", "rev 12ab34");
        assert_eq!(comparison.kind(), "unknown");
        assert!(comparison.reason().unwrap().contains("无法比较"));
        // 段数不同不猜测。
        assert!(compare_order("1.2.3", "1.2").is_none());
        // 分隔符不同（semver 与日期样式）不猜测大小。
        assert!(compare_order("1.2.3", "2024-10-15").is_none());
        assert!(compare_order("1.2.3", "1.2.4").is_some());
    }

    #[test]
    fn check_records_failure_without_touching_install() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        write_tool(
            root.path(),
            "rg",
            "name: rg\ncheck_command: sh ./missing-would-fail.sh\n",
        );
        state
            .set_tool_install(&ToolInstallRecord {
                tool_slug: "rg".to_string(),
                version: "14.1.0".to_string(),
                method: Some("brew".to_string()),
                path: None,
                config_path: None,
            })
            .unwrap();

        let outcome = check_tool(root.path(), &state, "rg").unwrap();
        assert_eq!(outcome.status, "failed");
        assert_eq!(outcome.installed_version.as_deref(), Some("14.1.0"));
        assert!(outcome.comparison.is_none());
        // 检查不改安装事实。
        assert_eq!(
            state.tool_install("rg").unwrap().unwrap().version,
            "14.1.0"
        );
    }

    #[test]
    fn check_parses_upstream_version_and_derives_updatable() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        write_tool(
            root.path(),
            "rg",
            "name: rg\ncheck_command: sh ./check-latest.sh\n",
        );
        std::fs::write(
            files::tool_dir(root.path(), "rg").join("check-latest.sh"),
            "echo 14.1.1\n",
        )
        .unwrap();
        state
            .set_tool_install(&ToolInstallRecord {
                tool_slug: "rg".to_string(),
                version: "14.1.0".to_string(),
                method: None,
                path: None,
                config_path: None,
            })
            .unwrap();

        let outcome = check_tool(root.path(), &state, "rg").unwrap();
        assert_eq!(outcome.status, "checked");
        assert_eq!(outcome.latest_version.as_deref(), Some("14.1.1"));
        assert_eq!(outcome.comparison.unwrap().kind(), "updatable");
    }

    #[test]
    fn uninstalled_tool_has_no_comparison() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        write_tool(
            root.path(),
            "rg",
            "name: rg\ncheck_command: sh ./check-latest.sh\n",
        );
        std::fs::write(
            files::tool_dir(root.path(), "rg").join("check-latest.sh"),
            "echo 14.1.1\n",
        )
        .unwrap();

        let outcome = check_tool(root.path(), &state, "rg").unwrap();
        assert_eq!(outcome.status, "checked");
        assert_eq!(outcome.latest_version.as_deref(), Some("14.1.1"));
        assert!(outcome.installed_version.is_none());
        // 未安装不派生“本机最新/可更新”。
        assert!(outcome.comparison.is_none());
    }

    #[test]
    fn empty_stdout_is_a_failure() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        write_tool(root.path(), "rg", "name: rg\ncheck_command: sh ./no.sh\n");
        std::fs::write(
            files::tool_dir(root.path(), "rg").join("no.sh"),
            "echo 说明性输出 1>&2\n",
        )
        .unwrap();
        let outcome = check_tool(root.path(), &state, "rg").unwrap();
        assert_eq!(outcome.status, "failed");
        assert!(outcome.latest_version.is_none());
    }

    #[test]
    fn missing_command_is_expressed_as_no_command() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        write_tool(root.path(), "rg", "name: rg\n");
        let outcome = check_tool(root.path(), &state, "rg").unwrap();
        assert_eq!(outcome.status, "no_command");
        assert_eq!(
            state.tool_check_status("rg").unwrap(),
            CheckStatus::NoCommand
        );
        // 无记录时也表达为“未登记命令”，而不是“未检查”。
        assert_eq!(effective_status(false, None), CheckStatus::NoCommand);
        assert_eq!(effective_status(true, None), CheckStatus::Unchecked);
    }

    #[test]
    fn check_all_continues_after_broken_metadata() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        write_tool(root.path(), "good", "name: good\ncheck_command: echo 1.0\n");
        write_tool(root.path(), "bad", "- not-a-mapping\n");
        let outcomes = check_all(root.path(), &state).unwrap();
        assert_eq!(outcomes.len(), 2);
        let bad = outcomes.iter().find(|item| item.slug == "bad").unwrap();
        assert_eq!(bad.status, "failed");
        let good = outcomes.iter().find(|item| item.slug == "good").unwrap();
        assert_eq!(good.status, "checked");
    }
}
