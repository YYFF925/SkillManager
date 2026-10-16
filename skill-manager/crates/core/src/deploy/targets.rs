//! 部署目标：内置 Agent 名单、目标所属域与路径解析。
//!
//! 检测只读本机存在事实，不启动 Agent、不部署技能。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::domain::{Domain, Target};
use crate::error::{CoreError, Result};

/// 内置 Agent 目录资产（源自 docs/technical-reference/agent-catalog.json，53 项）。
const AGENT_CATALOG: &str = include_str!("../../assets/agent-catalog.json");

/// 单个内置 Agent 的目录资料。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEntry {
    pub key: String,
    pub display_name: String,
    /// 该 Agent 面向的管理域。
    pub reference_domain: Domain,
    /// 相对用户 home 的技能目录。
    pub skill_path: String,
    /// 相对 home 的安装检测线索；与技能目录分开。
    pub detect_path: String,
    /// 项目级技能路径资料；仅作配置保留，不启用项目技能管理。
    pub project_skill_path: Option<String>,
    /// 额外发现目录，不自动作为部署地址。
    #[serde(default)]
    pub additional_scan_paths: Vec<String>,
    #[serde(default)]
    pub recursive_scan: bool,
    /// 图标路径，相对本仓库 `public/` 的 `assets/agent-icons/`。
    pub icon: Option<String>,
    #[serde(default)]
    pub invert_in_dark_theme: bool,
}

impl AgentEntry {
    /// 部署目标标识。
    pub fn target(&self) -> Target {
        Target::Agent {
            key: self.key.clone(),
        }
    }

    /// 本机技能目录（未考虑用户覆盖）。
    pub fn default_skills_dir(&self, home: &Path) -> PathBuf {
        home.join(&self.skill_path)
    }

    /// 安装检测目录。
    pub fn detect_dir(&self, home: &Path) -> PathBuf {
        home.join(&self.detect_path)
    }

    /// 检测线索是否存在（纯路径存在性判断，不查进程）。
    pub fn detected(&self, home: &Path) -> bool {
        self.detect_dir(home).exists()
    }

    /// 前端可用的图标地址；无图标文件时为 None（不伪造图标）。
    pub fn icon_url(&self) -> Option<String> {
        self.icon.as_ref().map(|path| format!("/{path}"))
    }

    /// 项目技能相对路径是否只作为配置保留（当前恒为真）。
    pub fn project_rel_config_only(&self) -> bool {
        true
    }
}

/// 内置 Agent 名单。
#[derive(Clone, Debug)]
pub struct AgentCatalog {
    entries: Vec<AgentEntry>,
    index: HashMap<String, usize>,
}

impl AgentCatalog {
    /// 解析资产 JSON。
    pub fn parse(json: &str) -> Result<Self> {
        let entries: Vec<AgentEntry> = serde_json::from_str(json)
            .map_err(|err| CoreError::other(format!("Agent 目录资产无法解析：{err}")))?;
        let mut index = HashMap::with_capacity(entries.len());
        for (position, entry) in entries.iter().enumerate() {
            if index.insert(entry.key.clone(), position).is_some() {
                return Err(CoreError::other(format!(
                    "Agent 目录资产存在重复 key：{}",
                    entry.key
                )));
            }
        }
        Ok(Self { entries, index })
    }

    /// 内置名单（进程内只解析一次）。
    pub fn embedded() -> &'static Self {
        static CATALOG: OnceLock<AgentCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            AgentCatalog::parse(AGENT_CATALOG).expect("内置 Agent 目录资产应当可解析")
        })
    }

    pub fn all(&self) -> &[AgentEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, key: &str) -> Option<&AgentEntry> {
        self.index.get(key).map(|position| &self.entries[*position])
    }

    /// 某个管理域的 Agent。
    pub fn by_domain(&self, domain: Domain) -> Vec<&AgentEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.reference_domain == domain)
            .collect()
    }
}

/// 目标解析结果。
#[derive(Clone, Debug, Serialize)]
pub struct TargetResolution {
    pub target: String,
    pub label: String,
    /// 实际使用路径：Agent 与 Hermes 为目标目录，共享目录为配置的共享路径。
    pub path: String,
    /// 目标所属域；共享目录属于编码域。
    pub domain: Option<Domain>,
    pub agent_key: Option<String>,
}

/// 目标解析上下文。
pub struct TargetResolver<'a> {
    home: PathBuf,
    catalog: &'a AgentCatalog,
    shared_dir: Option<PathBuf>,
    /// 本机配置的 Agent 技能目录覆盖（`agents.skills_dir`）；缺省时用内置默认目录。
    agent_dirs: HashMap<String, PathBuf>,
}

impl<'a> TargetResolver<'a> {
    pub fn new(
        home: impl Into<PathBuf>,
        catalog: &'a AgentCatalog,
        shared_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            home: home.into(),
            catalog,
            shared_dir,
            agent_dirs: HashMap::new(),
        }
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    /// 设置本机 Agent 技能目录覆盖（来自本机配置）。
    pub fn set_agent_dirs(&mut self, dirs: impl IntoIterator<Item = (String, PathBuf)>) {
        self.agent_dirs = dirs.into_iter().collect();
    }

    /// 某个 Agent 当前解析使用的技能目录（覆盖优先，否则内置默认）。
    pub fn agent_skills_dir(&self, key: &str) -> Result<PathBuf> {
        let entry = self
            .catalog
            .get(key)
            .ok_or_else(|| CoreError::InvalidTarget(format!("{key} 不在内置 Agent 名单中")))?;
        Ok(self
            .agent_dirs
            .get(key)
            .cloned()
            .unwrap_or_else(|| entry.default_skills_dir(&self.home)))
    }

    /// 用内置名单与设置中的共享目录解析目标路径。
    pub fn resolve(&self, target: &Target) -> Result<TargetResolution> {
        self.resolve_with_override(target, None)
    }

    /// 解析目标路径；`skills_dir_override` 为本机配置的覆盖目录。
    pub fn resolve_with_override(
        &self,
        target: &Target,
        skills_dir_override: Option<&Path>,
    ) -> Result<TargetResolution> {
        match target {
            Target::Agent { key } => {
                let entry = self.catalog.get(key).ok_or_else(|| {
                    CoreError::InvalidTarget(format!("{key} 不在内置 Agent 名单中"))
                })?;
                let path = match skills_dir_override {
                    Some(dir) => dir.to_path_buf(),
                    None => self.agent_skills_dir(key)?,
                };
                Ok(TargetResolution {
                    target: target.as_str(),
                    label: entry.display_name.clone(),
                    path: path.display().to_string(),
                    domain: Some(entry.reference_domain),
                    agent_key: Some(entry.key.clone()),
                })
            }
            Target::Shared => {
                let dir = self.shared_dir.as_ref().ok_or_else(|| {
                    CoreError::InvalidTarget(
                        "尚未设置共享目录路径（shared 是部署目标，不是第二个管理库）".to_string(),
                    )
                })?;
                Ok(TargetResolution {
                    target: target.as_str(),
                    label: "共享目录".to_string(),
                    path: dir.display().to_string(),
                    // 共享目录只用编码域技能。
                    domain: Some(Domain::Code),
                    agent_key: None,
                })
            }
            Target::HermesDefault => {
                let entry = self.catalog.get("hermes").ok_or_else(|| {
                    CoreError::InvalidTarget("内置名单缺少 hermes".to_string())
                })?;
                // 与 `HermesService` 同源：根按 `HERMES_HOME` 环境变量与平台默认解析，
                // 目标目录为 `<根>/skills`（default 档案就是根自身）。
                let skills = crate::hermes::hermes_home(&self.home).join("skills");
                Ok(TargetResolution {
                    target: target.as_str(),
                    label: "Hermes default 档案".to_string(),
                    path: skills.display().to_string(),
                    domain: Some(Domain::Lobster),
                    agent_key: Some(entry.key.clone()),
                })
            }
        }
    }

    /// 目标所属域。部署准入与档案启用都用它判断管理域边界。
    pub fn domain_of(&self, target: &Target) -> Result<Domain> {
        self.resolve(target)?
            .domain
            .ok_or_else(|| CoreError::InvalidTarget(format!("{target} 没有所属域")))
    }

    /// 本机检测到的 Agent（路径存在性判断）。
    pub fn detected(&self) -> Vec<&'a AgentEntry> {
        self.catalog
            .all()
            .iter()
            .filter(|entry| entry.detected(&self.home))
            .collect()
    }

    /// 指令分发目标：`<检测目录>/AGENTS.md`。
    pub fn instructions_target(&self, key: &str) -> Result<PathBuf> {
        let entry = self
            .catalog
            .get(key)
            .ok_or_else(|| CoreError::InvalidTarget(format!("{key} 不在内置 Agent 名单中")))?;
        Ok(entry.detect_dir(&self.home).join("AGENTS.md"))
    }
}

/// 共享目录目标的固定标识。
pub const SHARED_TARGET: Target = Target::Shared;

/// 解析 CLI/GUI 传入的目标参数：Agent key、`shared` 或 `hermes:default`。
pub fn parse_target_arg(raw: &str) -> Result<Target> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CoreError::InvalidTarget("目标不能为空".to_string()));
    }
    if trimmed == "shared" || trimmed.contains(':') {
        return trimmed.parse();
    }
    Ok(Target::Agent {
        key: trimmed.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_covers_all_53_builtin_agents() {
        let catalog = AgentCatalog::embedded();
        assert_eq!(catalog.len(), 53);
        assert_eq!(catalog.by_domain(Domain::Code).len(), 47);
        assert_eq!(catalog.by_domain(Domain::Lobster).len(), 6);
        assert!(catalog.get("hermes").is_some());
        assert!(catalog.get("codex").is_some());
    }

    #[test]
    fn detection_is_path_existence_only() {
        let home = tempfile::TempDir::new().unwrap();
        let catalog = AgentCatalog::embedded();
        let resolver = TargetResolver::new(home.path(), catalog, None);
        assert!(resolver.detected().is_empty());

        std::fs::create_dir_all(home.path().join(".codex")).unwrap();
        let detected = resolver.detected();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].key, "codex");
    }

    #[test]
    fn agent_skills_dir_uses_catalog_path_and_override() {
        let home = tempfile::TempDir::new().unwrap();
        let catalog = AgentCatalog::embedded();
        let resolver = TargetResolver::new(home.path(), catalog, None);

        let target = Target::Agent {
            key: "codex".to_string(),
        };
        let resolved = resolver.resolve(&target).unwrap();
        assert!(resolved.path.ends_with(".codex/skills"));
        assert_eq!(resolved.domain, Some(Domain::Code));

        let resolved = resolver
            .resolve_with_override(&target, Some(Path::new("/custom/skills")))
            .unwrap();
        assert_eq!(resolved.path, "/custom/skills");
    }

    #[test]
    fn shared_requires_configured_dir_and_belongs_to_code() {
        let home = tempfile::TempDir::new().unwrap();
        let catalog = AgentCatalog::embedded();
        let without = TargetResolver::new(home.path(), catalog, None);
        let err = without.resolve(&SHARED_TARGET).unwrap_err();
        assert!(err.to_string().contains("共享目录"), "{err}");

        let with = TargetResolver::new(
            home.path(),
            catalog,
            Some(PathBuf::from("/shared/skills")),
        );
        let resolved = with.resolve(&SHARED_TARGET).unwrap();
        assert_eq!(resolved.path, "/shared/skills");
        assert_eq!(resolved.domain, Some(Domain::Code));
    }

    #[test]
    fn hermes_default_is_lobster_domain() {
        let home = tempfile::TempDir::new().unwrap();
        let catalog = AgentCatalog::embedded();
        let resolver = TargetResolver::new(home.path(), catalog, None);
        let resolved = resolver.resolve(&Target::HermesDefault).unwrap();
        assert_eq!(resolved.domain, Some(Domain::Lobster));
        assert!(resolved.path.contains("hermes"), "{}", resolved.path);
    }

    #[test]
    fn unknown_agent_key_is_reported() {
        let home = tempfile::TempDir::new().unwrap();
        let catalog = AgentCatalog::embedded();
        let resolver = TargetResolver::new(home.path(), catalog, None);
        let err = resolver
            .resolve(&Target::Agent {
                key: "nope".to_string(),
            })
            .unwrap_err();
        assert!(matches!(err, CoreError::InvalidTarget(_)));
    }

    #[test]
    fn catalog_has_no_duplicate_keys() {
        let catalog = AgentCatalog::embedded();
        let mut keys: Vec<&str> = catalog.all().iter().map(|e| e.key.as_str()).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(before, keys.len());
    }
}
