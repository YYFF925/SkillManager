//! 工具清单：资料读取、搜索、一级分类与状态筛选（T075/T077 视图）。
//!
//! 清单是**内存派生量**，每次从库文件与本机状态重建：共享资料来自
//! `tools/<slug>/tool.yaml`、`guide.md`，本机事实来自 `tool_installs` / `tool_checks`。
//! 未安装的工具**保留在清单中**并如实显示“未安装”（可暗显），不隐藏（FR-045）。
//!
//! 单条资料损坏只标出该条错误，其余工具仍可浏览。

use std::path::Path;

use serde::Serialize;

use crate::domain::SkillRef;
use crate::error::{CoreError, Result};
use crate::state::{CheckStatus, StateStore};
use crate::toolcenter::check::{compare_versions, effective_status};
use crate::toolcenter::files::{self, ToolFile, ToolMeta};
use crate::vault::index::SkillIndex;

/// 本机安装事实视图。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ToolInstallView {
    pub version: String,
    pub method: Option<String>,
    pub path: Option<String>,
    pub config_path: Option<String>,
}

/// 本机检查结果视图。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ToolCheckView {
    pub status: String,
    pub latest_version: Option<String>,
    pub checked_at: Option<String>,
    pub output: Option<String>,
}

/// 配套 Skill 一条关联：库内状态与来源版本分别展示。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ToolSkillView {
    pub domain: String,
    pub name: String,
    pub repo: Option<String>,
    pub subpath: Option<String>,
    /// 库内是否已有该 `(domain, name)`；缺失时界面显示“未入库”。
    pub in_vault: bool,
    pub vault_path: Option<String>,
    pub comment: Option<String>,
    pub tags: Vec<String>,
    pub category: Option<String>,
    /// 该配套 Skill 库内记录的来源版本（与工具版本各自管理）。
    pub source_version: Option<String>,
}

/// 清单/详情共用的工具条目；`intro`/`guide`/`check_command` 只在详情时加载。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ToolEntry {
    /// 工具 slug（目录名），即工具身份。
    pub slug: String,
    pub name: String,
    /// 卡片短摘要。
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub category: Option<String>,
    /// 工具资料目录（绝对路径）。
    pub dir: String,
    pub has_guide: bool,
    pub has_check_command: bool,
    pub skills: Vec<ToolSkillView>,
    /// 无安装记录为 `None`，界面显示“未安装”。
    pub install: Option<ToolInstallView>,
    pub check: ToolCheckView,
    /// 已安装且可比较时的派生结论：`latest` / `updatable` / `unknown`；未安装为 `None`。
    pub comparison: Option<String>,
    pub comparison_reason: Option<String>,
    /// 概览长简介（可读、多段；仅详情加载）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intro: Option<String>,
    /// 版本检查命令；仅详情加载。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_command: Option<String>,
    /// 维护指南正文；仅详情加载。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guide: Option<String>,
    /// 该条资料读取失败原因；不为空时其余字段不作有效结论。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 状态筛选取值（与 `contracts/cli.md` 的 `--status` 一致）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolStatusFilter {
    Installed,
    NotInstalled,
    Updatable,
    Latest,
    Failed,
    Unchecked,
    NoCommand,
    NoGuide,
}

impl ToolStatusFilter {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "installed" => Ok(Self::Installed),
            "not-installed" => Ok(Self::NotInstalled),
            "updatable" => Ok(Self::Updatable),
            "latest" => Ok(Self::Latest),
            "failed" => Ok(Self::Failed),
            "unchecked" => Ok(Self::Unchecked),
            "no-command" => Ok(Self::NoCommand),
            "no-guide" => Ok(Self::NoGuide),
            other => Err(CoreError::other(format!(
                "未知工具状态筛选 {other}（应为 installed|not-installed|updatable|latest|failed|unchecked|no-command|no-guide）"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Installed => "installed",
            Self::NotInstalled => "not-installed",
            Self::Updatable => "updatable",
            Self::Latest => "latest",
            Self::Failed => "failed",
            Self::Unchecked => "unchecked",
            Self::NoCommand => "no-command",
            Self::NoGuide => "no-guide",
        }
    }
}

/// 工具清单查询条件。
#[derive(Clone, Debug, Default)]
pub struct ToolQuery {
    /// 文本搜索（名称、短摘要、标签、分类）。
    pub query: Option<String>,
    /// 一级分类。
    pub category: Option<String>,
    pub status: Option<ToolStatusFilter>,
}

fn not_installed_check() -> ToolCheckView {
    ToolCheckView {
        status: CheckStatus::Unchecked.as_str().to_string(),
        latest_version: None,
        checked_at: None,
        output: None,
    }
}

fn skill_view(index: Option<&SkillIndex>, link: &files::ToolSkillLink) -> ToolSkillView {
    let reference = SkillRef {
        domain: link.domain,
        name: link.name.clone(),
    };
    let entry = index.and_then(|index| index.get(&reference));
    ToolSkillView {
        domain: link.domain.as_str().to_string(),
        name: link.name.clone(),
        repo: link.repo.clone(),
        subpath: link.subpath.clone(),
        in_vault: entry.is_some(),
        vault_path: entry.map(|entry| entry.vault_rel_dir.clone()),
        comment: entry.and_then(|entry| entry.comment.clone()),
        tags: entry.map(|entry| entry.tags.clone()).unwrap_or_default(),
        category: entry.and_then(|entry| entry.category.clone()),
        source_version: entry
            .and_then(|entry| entry.source.as_ref())
            .map(|source| source.version.clone()),
    }
}

fn error_entry(root: &Path, slug: &str, message: String) -> ToolEntry {
    ToolEntry {
        slug: slug.to_string(),
        name: slug.to_string(),
        summary: None,
        tags: Vec::new(),
        category: None,
        dir: files::tool_dir(root, slug).to_string_lossy().into_owned(),
        has_guide: files::guide_path(root, slug).is_file(),
        has_check_command: false,
        skills: Vec::new(),
        install: None,
        check: not_installed_check(),
        comparison: None,
        comparison_reason: None,
        intro: None,
        check_command: None,
        guide: None,
        error: Some(message),
    }
}

/// 组装一条工具条目；`with_detail` 为真时加载简介、检查命令与指南正文。
pub fn build_entry(
    root: &Path,
    state: &StateStore,
    index: Option<&SkillIndex>,
    slug: &str,
    with_detail: bool,
) -> Result<ToolEntry> {
    let meta: ToolMeta = match ToolFile::load(root, slug) {
        Ok(file) => file.meta()?,
        Err(err) => return Ok(error_entry(root, slug, err.to_string())),
    };

    let install = state.tool_install(slug)?.map(|record| ToolInstallView {
        version: record.version,
        method: record.method,
        path: record.path,
        config_path: record.config_path,
    });

    let record = state.tool_check(slug)?;
    let has_command = meta.check_command.is_some();
    let status = effective_status(has_command, record.as_ref().map(|item| item.status));
    let check = ToolCheckView {
        status: status.as_str().to_string(),
        latest_version: record
            .as_ref()
            .filter(|_| status == CheckStatus::Checked)
            .and_then(|item| item.latest_version.clone()),
        checked_at: record.as_ref().and_then(|item| item.checked_at.clone()),
        output: record.as_ref().and_then(|item| item.check_output.clone()),
    };

    // 未安装工具只显示上游版本，不派生“本机最新/可更新”。
    let (comparison, comparison_reason) = match (&install, &check.latest_version) {
        (Some(install), Some(latest)) if status == CheckStatus::Checked => {
            let derived = compare_versions(&install.version, latest);
            (Some(derived.kind().to_string()), derived.reason().map(str::to_string))
        }
        _ => (None, None),
    };

    let guide = if with_detail {
        // 单条指南读取失败不影响资料本身，只如实标注。
        match files::read_guide(root, slug) {
            Ok(text) => text,
            Err(_) => None,
        }
    } else {
        None
    };

    Ok(ToolEntry {
        slug: slug.to_string(),
        name: meta.name.clone(),
        summary: meta.summary.clone(),
        tags: meta.tags.clone(),
        category: meta.category.clone(),
        dir: files::tool_dir(root, slug).to_string_lossy().into_owned(),
        has_guide: files::guide_path(root, slug).is_file(),
        has_check_command: has_command,
        skills: meta
            .skills
            .iter()
            .map(|link| skill_view(index, link))
            .collect(),
        install,
        check,
        comparison,
        comparison_reason,
        intro: if with_detail { meta.intro.clone() } else { None },
        check_command: if with_detail {
            meta.check_command.clone()
        } else {
            None
        },
        guide,
        error: if meta.name.trim().is_empty() {
            Some("工具资料缺少 name（工具名称不能为空）".to_string())
        } else {
            None
        },
    })
}

fn matches_query(entry: &ToolEntry, query: &ToolQuery) -> bool {
    if let Some(category) = &query.category {
        if entry.category.as_deref() != Some(category.as_str()) {
            return false;
        }
    }
    if let Some(text) = &query.query {
        let needle = text.trim().to_lowercase();
        if !needle.is_empty() {
            let haystack = [
                entry.slug.clone(),
                entry.name.clone(),
                entry.summary.clone().unwrap_or_default(),
                entry.category.clone().unwrap_or_default(),
                entry.tags.join(" "),
            ]
            .join(" ")
            .to_lowercase();
            if !haystack.contains(&needle) {
                return false;
            }
        }
    }
    if let Some(filter) = query.status {
        let matched = match filter {
            ToolStatusFilter::Installed => entry.install.is_some(),
            ToolStatusFilter::NotInstalled => entry.install.is_none(),
            ToolStatusFilter::Updatable => entry.comparison.as_deref() == Some("updatable"),
            ToolStatusFilter::Latest => entry.comparison.as_deref() == Some("latest"),
            ToolStatusFilter::Failed => entry.check.status == "failed",
            ToolStatusFilter::Unchecked => entry.check.status == "unchecked",
            ToolStatusFilter::NoCommand => entry.check.status == "no_command",
            ToolStatusFilter::NoGuide => !entry.has_guide,
        };
        if !matched {
            return false;
        }
    }
    true
}

/// 列出工具（含未安装条目）；资料损坏的单条照常返回并带 `error`。
pub fn list(root: &Path, state: &StateStore, query: &ToolQuery) -> Result<Vec<ToolEntry>> {
    let index = SkillIndex::build(root).ok();
    let mut out = Vec::new();
    for slug in files::all_slugs(root)? {
        let entry = build_entry(root, state, index.as_ref(), &slug, false)?;
        if matches_query(&entry, query) {
            out.push(entry);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name).then(a.slug.cmp(&b.slug)));
    Ok(out)
}

/// 读取一个工具的详情；未登记时报错。
pub fn get(root: &Path, state: &StateStore, slug: &str) -> Result<ToolEntry> {
    if !files::tool_path(root, slug).is_file() {
        return Err(CoreError::other(format!(
            "工具未登记：{slug}（缺少 {}）",
            files::tool_path(root, slug).display()
        )));
    }
    let index = SkillIndex::build(root).ok();
    build_entry(root, state, index.as_ref(), slug, true)
}

/// 清单中出现过的一级分类，按名称排序。
pub fn categories(root: &Path, state: &StateStore) -> Result<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for entry in list(root, state, &ToolQuery::default())? {
        if let Some(category) = entry.category {
            if !out.contains(&category) {
                out.push(category);
            }
        }
    }
    out.sort();
    Ok(out)
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
    fn uninstalled_tool_stays_in_the_list() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        write_tool(root.path(), "ripgrep", "name: ripgrep\nsummary: 搜索\ncategory: 命令行工具\n");
        let entries = list(root.path(), &state, &ToolQuery::default()).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].install.is_none());
        // 未登记检查命令的工具如实表达“未登记命令”，不冒充“未检查”。
        assert_eq!(entries[0].check.status, "no_command");
        assert!(entries[0].comparison.is_none());
    }

    #[test]
    fn status_filters_are_distinct() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        write_tool(
            root.path(),
            "rg",
            "name: rg\ncategory: 命令行工具\ncheck_command: echo 1.0\n",
        );
        write_tool(root.path(), "no-command", "name: no-command\n");
        state
            .set_tool_install(&ToolInstallRecord {
                tool_slug: "rg".to_string(),
                version: "0.9".to_string(),
                method: None,
                path: None,
                config_path: None,
            })
            .unwrap();

        let not_installed = list(
            root.path(),
            &state,
            &ToolQuery {
                status: Some(ToolStatusFilter::NotInstalled),
                ..ToolQuery::default()
            },
        )
        .unwrap();
        assert_eq!(not_installed.len(), 1);
        assert_eq!(not_installed[0].slug, "no-command");

        let no_command = list(
            root.path(),
            &state,
            &ToolQuery {
                status: Some(ToolStatusFilter::NoCommand),
                ..ToolQuery::default()
            },
        )
        .unwrap();
        assert_eq!(no_command.len(), 1);
        assert_eq!(no_command[0].slug, "no-command");

        let no_guide = list(
            root.path(),
            &state,
            &ToolQuery {
                status: Some(ToolStatusFilter::NoGuide),
                ..ToolQuery::default()
            },
        )
        .unwrap();
        assert_eq!(no_guide.len(), 2);
    }

    #[test]
    fn broken_tool_file_is_reported_per_entry() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        write_tool(root.path(), "bad", "- not-a-mapping\n");
        write_tool(root.path(), "good", "name: good\n");
        // 缺少 tool.yaml 的目录也如实呈现。
        std::fs::create_dir_all(files::tool_dir(root.path(), "stray")).unwrap();

        let entries = list(root.path(), &state, &ToolQuery::default()).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().find(|item| item.slug == "bad").unwrap().error.is_some());
        assert!(entries.iter().find(|item| item.slug == "good").unwrap().error.is_none());
        assert!(entries.iter().find(|item| item.slug == "stray").unwrap().error.is_some());
    }

    #[test]
    fn skill_links_report_missing_library_entry() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        write_tool(
            root.path(),
            "rg",
            "name: rg\nskills:\n  - domain: code\n    name: rg-usage\n    repo: https://x/y.git\n",
        );
        let entry = get(root.path(), &state, "rg").unwrap();
        assert_eq!(entry.skills.len(), 1);
        assert!(!entry.skills[0].in_vault);
        assert!(entry.skills[0].vault_path.is_none());
        assert_eq!(entry.skills[0].repo.as_deref(), Some("https://x/y.git"));
    }

    #[test]
    fn latest_and_updatable_require_installation() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        write_tool(root.path(), "rg", "name: rg\ncheck_command: echo 1.1\n");
        state
            .set_tool_check(&crate::state::ToolCheckRecord {
                tool_slug: "rg".to_string(),
                status: CheckStatus::Checked,
                latest_version: Some("1.1".to_string()),
                checked_at: Some("2026-10-15T00:00:00Z".to_string()),
                check_output: None,
            })
            .unwrap();
        // 未安装：只显示上游版本。
        let entry = get(root.path(), &state, "rg").unwrap();
        assert_eq!(entry.check.latest_version.as_deref(), Some("1.1"));
        assert!(entry.comparison.is_none());

        // 已安装且相等：派生 latest。
        let mut install = ToolInstallRecord {
            tool_slug: "rg".to_string(),
            version: "1.1".to_string(),
            method: None,
            path: None,
            config_path: None,
        };
        state.set_tool_install(&install).unwrap();
        state
            .set_tool_check(&crate::state::ToolCheckRecord {
                tool_slug: "rg".to_string(),
                status: CheckStatus::Checked,
                latest_version: Some("1.1".to_string()),
                checked_at: Some("2026-10-15T00:00:00Z".to_string()),
                check_output: None,
            })
            .unwrap();
        let entry = get(root.path(), &state, "rg").unwrap();
        assert_eq!(entry.comparison.as_deref(), Some("latest"));

        // 安装回写会重置检查结果，不再派生结论。
        install.version = "1.2".to_string();
        state.set_tool_install(&install).unwrap();
        let entry = get(root.path(), &state, "rg").unwrap();
        assert_eq!(entry.check.status, "unchecked");
        assert!(entry.comparison.is_none());
    }
}
