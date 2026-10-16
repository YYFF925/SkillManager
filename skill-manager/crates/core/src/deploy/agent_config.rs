//! 本机 Agent 配置（T067）：检测、启停、排序、技能目录、项目相对路径与指令目标。
//!
//! 检测只读本机目录存在事实（`AgentEntry::detected`），不启动 Agent、不部署技能；
//! 启停、顺序与各类目录只写本机状态（SQLite `agents`），不进入 SkillVault。
//! 全量 53 项内置名单保留，界面据此区分“已检测/其他支持项”。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::deploy::targets::{AgentCatalog, AgentEntry};
use crate::error::{CoreError, Result};
use crate::state::{AgentConfig, StateStore};

/// 本机 Agent 的一体化视图：内置目录资料 + 本机配置 + 检测结果。
#[derive(Clone, Debug, Serialize)]
pub struct AgentView {
    pub key: String,
    pub display_name: String,
    pub domain: &'static str,
    /// 本机检测目录存在（只读事实）。
    pub detected: bool,
    /// 启用状态；无本机记录时默认启用（与旧行为一致：新检测到的 Agent 默认启用）。
    pub enabled: bool,
    pub order: i64,
    /// 当前实际使用的技能目录（覆盖优先，否则内置默认）。
    pub skills_dir: String,
    /// 是否为用户本机覆盖目录（false 表示内置默认）。
    pub skills_dir_custom: bool,
    /// 技能目录的内置默认值。
    pub skills_dir_default: String,
    /// 项目技能相对路径；仅配置保留，不实现项目技能管理。
    pub project_rel_dir: Option<String>,
    /// 项目技能相对路径的内置默认值。
    pub project_rel_dir_default: Option<String>,
    /// 指令分发目标（`AGENTS.md` 位置）。
    pub instructions_target: String,
    /// 品牌图标文件名（相对 `public/`）；无图标为 None。
    pub icon: Option<String>,
    pub invert_in_dark: bool,
}

/// 目录类配置的种类。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirKind {
    Skills,
    ProjectRel,
    Instructions,
}

impl DirKind {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim() {
            "skills" => Ok(DirKind::Skills),
            "project-rel" | "project_rel" => Ok(DirKind::ProjectRel),
            "instructions" => Ok(DirKind::Instructions),
            other => Err(CoreError::other(format!(
                "未知目录种类 {other}（应为 skills、project-rel 或 instructions）"
            ))),
        }
    }
}

/// 本机 Agent 配置服务：内置名单 + 本机状态 + 模拟 home。
pub struct AgentConfigService<'a> {
    home: PathBuf,
    catalog: &'static AgentCatalog,
    state: &'a StateStore,
}

impl<'a> AgentConfigService<'a> {
    pub fn new(home: PathBuf, catalog: &'static AgentCatalog, state: &'a StateStore) -> Self {
        Self {
            home,
            catalog,
            state,
        }
    }

    fn entry(&self, key: &str) -> Result<&'static AgentEntry> {
        self.catalog
            .get(key)
            .ok_or_else(|| CoreError::InvalidTarget(format!("{key} 不在内置 Agent 名单中")))
    }

    fn config(&self, key: &str) -> Result<Option<AgentConfig>> {
        self.state.agent(key)
    }

    /// 全量 53 项名单与本机状态；`detected` 为当前路径存在性判断。
    pub fn list(&self) -> Result<Vec<AgentView>> {
        let configs = self.state.agents()?;
        let mut out = Vec::with_capacity(self.catalog.len());
        for entry in self.catalog.all() {
            let config = configs.iter().find(|config| config.key == entry.key);
            out.push(self.view(entry, config));
        }
        Ok(out)
    }

    /// 重新检测：数据是磁盘事实的纯函数，因此与 `list` 相同（重新读取本机状态与路径）。
    pub fn detect(&self) -> Result<Vec<AgentView>> {
        self.list()
    }

    fn view(&self, entry: &AgentEntry, config: Option<&AgentConfig>) -> AgentView {
        let skills_dir_default = entry.default_skills_dir(&self.home);
        let custom = config.and_then(|config| config.skills_dir.as_deref());
        let skills_dir = custom
            .map(PathBuf::from)
            .unwrap_or_else(|| skills_dir_default.clone());
        AgentView {
            key: entry.key.clone(),
            display_name: entry.display_name.clone(),
            domain: entry.reference_domain.as_str(),
            detected: entry.detected(&self.home),
            enabled: config.map(|config| config.enabled).unwrap_or(true),
            order: config.map(|config| config.order).unwrap_or(0),
            skills_dir: skills_dir.display().to_string(),
            skills_dir_custom: custom.is_some(),
            skills_dir_default: skills_dir_default.display().to_string(),
            project_rel_dir: config.and_then(|config| config.project_rel_dir.clone()),
            project_rel_dir_default: entry.project_skill_path.clone(),
            instructions_target: config
                .and_then(|config| config.instructions_target.clone())
                .unwrap_or_else(|| {
                    entry
                        .detect_dir(&self.home)
                        .join("AGENTS.md")
                        .display()
                        .to_string()
                }),
            icon: entry.icon.clone(),
            invert_in_dark: entry.invert_in_dark_theme,
        }
    }

    /// 启用或停用；只写本机状态。
    pub fn set_enabled(&self, key: &str, enabled: bool) -> Result<AgentView> {
        let entry = self.entry(key)?;
        self.state.set_agent_enabled(key, enabled)?;
        Ok(self.view(entry, self.config(key)?.as_ref()))
    }

    /// 按给定顺序写入显示顺序（列表前一项 order 更小）。
    pub fn reorder(&self, order: &[String]) -> Result<Vec<AgentView>> {
        for key in order {
            self.entry(key)?;
        }
        for (position, key) in order.iter().enumerate() {
            self.state.set_agent_order(key, position as i64)?;
        }
        self.list()
    }

    /// 修改一类目录；只写本机状态。
    ///
    /// `skills` 与 `instructions` 要求绝对路径（相对路径会随进程工作目录漂移）；
    /// `project-rel` 要求相对路径、禁止 `..`，空值等于清除（仅配置保留）。
    pub fn set_dir(&self, key: &str, kind: DirKind, raw: &str) -> Result<AgentView> {
        let entry = self.entry(key)?;
        match kind {
            DirKind::Skills => {
                let path = require_absolute(raw, "技能目录")?;
                self.state
                    .set_agent_skills_dir(key, Some(&path.display().to_string()))?;
            }
            DirKind::ProjectRel => {
                let value = normalize_project_rel(raw)?;
                self.state.set_agent_project_rel_dir(key, value.as_deref())?;
            }
            DirKind::Instructions => {
                let path = require_absolute(raw, "指令目标")?;
                self.state
                    .set_agent_instructions_target(key, Some(&path.display().to_string()))?;
            }
        }
        Ok(self.view(entry, self.config(key)?.as_ref()))
    }

    /// 恢复一类目录为内置默认（清除本机覆盖）。
    pub fn reset_dir(&self, key: &str, kind: DirKind) -> Result<AgentView> {
        let entry = self.entry(key)?;
        match kind {
            DirKind::Skills => self.state.set_agent_skills_dir(key, None)?,
            DirKind::ProjectRel => self.state.set_agent_project_rel_dir(key, None)?,
            DirKind::Instructions => self.state.set_agent_instructions_target(key, None)?,
        }
        Ok(self.view(entry, self.config(key)?.as_ref()))
    }

    /// 解析某 Agent 当前使用的技能目录（覆盖优先）。
    pub fn skills_dir(&self, key: &str) -> Result<PathBuf> {
        let view = self.view(self.entry(key)?, self.config(key)?.as_ref());
        Ok(PathBuf::from(view.skills_dir))
    }
}

/// 要求绝对路径；相对路径会随工作目录变化，导致部署写错位置。
fn require_absolute(raw: &str, label: &str) -> Result<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CoreError::other(format!("{label}不能为空")));
    }
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err(CoreError::other(format!(
            "{label}必须是绝对路径（收到 {trimmed}）"
        )));
    }
    Ok(path)
}

/// 归一化项目相对路径：必须是相对路径、禁止 `..`，空值等于清除。
///
/// 仅配置保留，不实现项目技能管理；这里只保证存下来的值形状可用。
pub fn normalize_project_rel(raw: &str) -> Result<Option<String>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let normalized = trimmed.replace('\\', "/");
    if normalized.starts_with('/') {
        return Err(CoreError::other("项目相对路径必须是相对路径"));
    }
    let mut parts: Vec<&str> = Vec::new();
    for part in normalized.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err(CoreError::other("项目相对路径不能包含 .."));
        }
        parts.push(part);
    }
    if parts.is_empty() {
        return Ok(None);
    }
    Ok(Some(parts.join("/")))
}

/// 目标技能目录下的一个候选：用于本地技能列表与删除。
pub(crate) fn is_within(root: &Path, candidate: &Path) -> bool {
    let root = canonical_or_lexical(root);
    let candidate = canonical_or_lexical(candidate);
    candidate != root && candidate.starts_with(&root)
}

fn canonical_or_lexical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_rel_normalization_rules() {
        assert_eq!(normalize_project_rel("").unwrap(), None);
        assert_eq!(normalize_project_rel("  ").unwrap(), None);
        assert_eq!(
            normalize_project_rel(".cursor//skills").unwrap().as_deref(),
            Some(".cursor/skills")
        );
        assert!(normalize_project_rel("/abs/path").is_err());
        assert!(normalize_project_rel("a/../b").is_err());
    }

    #[test]
    fn absolute_dir_requires_absolute_path() {
        assert!(require_absolute("/tmp/x", "技能目录").is_ok());
        assert!(require_absolute("relative/x", "技能目录").is_err());
        assert!(require_absolute("", "技能目录").is_err());
    }

    #[test]
    fn containment_rejects_siblings_and_self() {
        let root = tempfile::TempDir::new().unwrap();
        let inside = root.path().join("alpha");
        std::fs::create_dir_all(&inside).unwrap();
        let sibling = root.path().join("..").join("outside");
        assert!(is_within(root.path(), &inside));
        assert!(!is_within(root.path(), root.path()));
        assert!(!is_within(root.path(), &sibling));
    }
}
