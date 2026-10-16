//! 工具配套 Skill 候选（T081，FR-049）。
//!
//! 候选来自工具资料的 `skills` 关联（`(domain, name)` + 可选来源 repo/subpath）。
//! **候选本身不是一次入库**：入库仍沿用 US2 的候选获取/选择流程（`skill preview` →
//! `skill import`），必须由用户勾选，且不自动部署；工具版本与各配套 Skill 的来源版本分别展示。
//! 技能正文不复制进工具资料。
//!
//! 单候选也列出；未入库的候选只有来源信息，如实显示“未入库”，不编造库内资料。

use std::path::Path;

use serde::Serialize;

use crate::domain::SkillRef;
use crate::error::Result;
use crate::state::StateStore;
use crate::toolcenter::catalog::{self, ToolEntry};
use crate::vault::index::SkillIndex;

/// 一个配套 Skill 候选。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ToolSkillCandidate {
    pub domain: String,
    pub name: String,
    /// 候选来源仓库（可空；未入库且无来源时无法入库）。
    pub repo: Option<String>,
    /// 来源仓库内子路径。
    pub subpath: Option<String>,
    /// 库内是否已有该技能；未入库时界面显示“未入库”。
    pub in_vault: bool,
    pub vault_path: Option<String>,
    /// 技能用途（frontmatter 描述）；未入库时无库内资料，为 `None`。
    pub description: Option<String>,
    /// 中文注释；未入库时为 `None`。
    pub comment: Option<String>,
    pub tags: Vec<String>,
    pub category: Option<String>,
    /// 该配套 Skill 库内记录的来源版本（与工具版本各自管理）。
    pub source_version: Option<String>,
}

/// 工具自身的版本概况（与 Skill 来源版本分别展示）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ToolVersionView {
    pub installed_version: Option<String>,
    pub installed: bool,
    pub status: String,
    pub latest_version: Option<String>,
    pub comparison: Option<String>,
    pub comparison_reason: Option<String>,
}

/// 一个工具的配套候选视图。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ToolSkillCandidates {
    pub slug: String,
    pub name: String,
    pub tool: ToolVersionView,
    pub candidates: Vec<ToolSkillCandidate>,
}

fn candidate(index: Option<&SkillIndex>, link: &crate::toolcenter::files::ToolSkillLink) -> ToolSkillCandidate {
    let reference = SkillRef {
        domain: link.domain,
        name: link.name.clone(),
    };
    let entry = index.and_then(|index| index.get(&reference));
    ToolSkillCandidate {
        domain: link.domain.as_str().to_string(),
        name: link.name.clone(),
        repo: link.repo.clone(),
        subpath: link.subpath.clone(),
        in_vault: entry.is_some(),
        vault_path: entry.map(|entry| entry.vault_rel_dir.clone()),
        description: entry.and_then(|entry| entry.description.clone()),
        comment: entry.and_then(|entry| entry.comment.clone()),
        tags: entry.map(|entry| entry.tags.clone()).unwrap_or_default(),
        category: entry.and_then(|entry| entry.category.clone()),
        source_version: entry
            .and_then(|entry| entry.source.as_ref())
            .map(|source| source.version.clone()),
    }
}

/// 读取一个工具的配套候选（含工具自身版本概况）。
pub fn candidates(root: &Path, state: &StateStore, slug: &str) -> Result<ToolSkillCandidates> {
    let entry: ToolEntry = catalog::get(root, state, slug)?;
    let meta = crate::toolcenter::files::ToolFile::load(root, slug)?.meta()?;
    let index = SkillIndex::build(root).ok();
    let list = meta
        .skills
        .iter()
        .map(|link| candidate(index.as_ref(), link))
        .collect();

    Ok(ToolSkillCandidates {
        slug: entry.slug.clone(),
        name: entry.name.clone(),
        tool: ToolVersionView {
            installed: entry.install.is_some(),
            installed_version: entry.install.as_ref().map(|install| install.version.clone()),
            status: entry.check.status.clone(),
            latest_version: entry.check.latest_version.clone(),
            comparison: entry.comparison.clone(),
            comparison_reason: entry.comparison_reason.clone(),
        },
        candidates: list,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tool(root: &Path, slug: &str, body: &str) {
        let path = crate::toolcenter::files::tool_path(root, slug);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn single_candidate_is_listed_with_source() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        write_tool(
            root.path(),
            "browser-use",
            "name: browser-use\nsummary: 浏览器自动化\nskills:\n  - domain: code\n    name: browser-use\n    repo: https://example.invalid/bu.git\n    subpath: browser-use\n",
        );
        let view = candidates(root.path(), &state, "browser-use").unwrap();
        assert_eq!(view.candidates.len(), 1);
        let item = &view.candidates[0];
        assert_eq!(item.name, "browser-use");
        assert!(!item.in_vault);
        assert!(item.description.is_none());
        assert_eq!(item.repo.as_deref(), Some("https://example.invalid/bu.git"));
        assert!(!view.tool.installed);
        assert_eq!(view.tool.status, "no_command");
    }

    #[test]
    fn in_vault_candidate_carries_library_metadata() {
        let root = tempfile::TempDir::new().unwrap();
        let state = StateStore::open_in_memory().unwrap();
        let dir = root.path().join("code/browser-use");
        std::fs::create_dir_all(dir.join(".manager")).unwrap();
        std::fs::write(dir.join("SKILL.md"), "---\ndescription: 浏览器任务\n---\n").unwrap();
        std::fs::write(
            dir.join(".manager/meta.yaml"),
            "comment: 浏览器任务\ntags: [页面交互]\n",
        )
        .unwrap();
        write_tool(
            root.path(),
            "browser-use",
            "name: browser-use\nskills:\n  - domain: code\n    name: browser-use\n",
        );
        let view = candidates(root.path(), &state, "browser-use").unwrap();
        let item = &view.candidates[0];
        assert!(item.in_vault);
        assert_eq!(item.comment.as_deref(), Some("浏览器任务"));
        assert_eq!(item.tags, vec!["页面交互".to_string()]);
        assert_eq!(item.description.as_deref(), Some("浏览器任务"));
    }
}
