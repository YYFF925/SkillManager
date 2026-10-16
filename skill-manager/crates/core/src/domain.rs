//! 领域身份：管理域、技能引用、部署目标与目录名校验。
//!
//! 技能身份 = 管理域 + 技能名（目录名），不引入数字 ID 或 UUID（research.md D1）。

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

/// 管理域：编码库与龙虾库。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Domain {
    Code,
    Lobster,
}

impl Domain {
    pub const ALL: [Domain; 2] = [Domain::Code, Domain::Lobster];

    /// 库内目录名，同时是 CLI 参数值。
    pub fn as_str(self) -> &'static str {
        match self {
            Domain::Code => "code",
            Domain::Lobster => "lobster",
        }
    }

    /// 界面显示名。
    pub fn label(self) -> &'static str {
        match self {
            Domain::Code => "编码库",
            Domain::Lobster => "龙虾库",
        }
    }

    pub fn dir_name(self) -> &'static str {
        self.as_str()
    }
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Domain {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "code" => Ok(Domain::Code),
            "lobster" => Ok(Domain::Lobster),
            other => Err(CoreError::InvalidRef(format!(
                "未知管理域 {other}（应为 code 或 lobster）"
            ))),
        }
    }
}

/// 技能引用：`(domain, name)`。
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SkillRef {
    pub domain: Domain,
    pub name: String,
}

impl SkillRef {
    /// 构造并校验技能名。
    pub fn new(domain: Domain, name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        validate_skill_name(&name)?;
        Ok(Self { domain, name })
    }

    /// 解析 `domain/name`。
    pub fn parse(value: &str) -> Result<Self> {
        let (domain, name) = value.split_once('/').ok_or_else(|| {
            CoreError::InvalidRef(format!("技能引用应为 <domain>/<name>，收到 {value}"))
        })?;
        let domain = Domain::from_str(domain.trim())?;
        Self::new(domain, name.trim())
    }

    /// 库内相对目录：`<domain>/[<category>/]<name>`。
    pub fn rel_dir(&self, category: Option<&str>) -> PathBuf {
        let mut path = PathBuf::from(self.domain.dir_name());
        if let Some(category) = category {
            path.push(category);
        }
        path.push(&self.name);
        path
    }

    /// 库内相对目录（含域，不含库根）。别名，语义更明确。
    pub fn vault_rel_dir(&self, category: Option<&str>) -> String {
        self.rel_dir(category).to_string_lossy().replace('\\', "/")
    }

    /// 库内绝对路径。
    pub fn dir_in(&self, vault_root: &Path, category: Option<&str>) -> PathBuf {
        vault_root.join(self.rel_dir(category))
    }
}

impl fmt::Display for SkillRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.domain.as_str(), self.name)
    }
}

impl FromStr for SkillRef {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self> {
        SkillRef::parse(value)
    }
}

/// 部署目标：本机 Agent、共享目录、Hermes default 档案。
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target {
    /// 内置 Agent 名单中的目标，值为 Agent key。
    Agent { key: String },
    /// 共享目录（特殊部署目标，不是第二个管理库）。
    Shared,
    /// Hermes default 档案。
    HermesDefault,
}

impl Target {
    /// 持久化与 CLI 使用的字符串形式。
    pub fn as_str(&self) -> String {
        match self {
            Target::Agent { key } => format!("agent:{key}"),
            Target::Shared => "shared".to_string(),
            Target::HermesDefault => "hermes:default".to_string(),
        }
    }

    pub fn agent_key(&self) -> Option<&str> {
        match self {
            Target::Agent { key } => Some(key),
            _ => None,
        }
    }

    pub fn is_shared(&self) -> bool {
        matches!(self, Target::Shared)
    }

    pub fn is_hermes_default(&self) -> bool {
        matches!(self, Target::HermesDefault)
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_str())
    }
}

impl FromStr for Target {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self> {
        let trimmed = value.trim();
        if trimmed == "shared" {
            return Ok(Target::Shared);
        }
        if trimmed == "hermes:default" {
            return Ok(Target::HermesDefault);
        }
        if let Some(key) = trimmed.strip_prefix("agent:") {
            if key.is_empty() {
                return Err(CoreError::InvalidTarget(trimmed.to_string()));
            }
            return Ok(Target::Agent {
                key: key.to_string(),
            });
        }
        Err(CoreError::InvalidTarget(format!(
            "{trimmed}（应为 agent:<key>、shared 或 hermes:default）"
        )))
    }
}

/// 目录身份名是否相等：同域内唯一按大小写不敏感比较（macOS 文件系统不区分大小写）。
pub fn names_equal(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// 校验技能名可以作为一个单层目录名。
pub fn validate_skill_name(name: &str) -> Result<()> {
    validate_segment(name, true)
}

/// 校验分类名可以作为一个单层目录名。
pub fn validate_category_name(name: &str) -> Result<()> {
    validate_segment(name, false)
}

fn validate_segment(name: &str, is_skill: bool) -> Result<()> {
    let fail = |reason: &str| -> CoreError {
        if is_skill {
            CoreError::InvalidSkillName {
                name: name.to_string(),
                reason: reason.to_string(),
            }
        } else {
            CoreError::InvalidCategory {
                name: name.to_string(),
                reason: reason.to_string(),
            }
        }
    };

    if name.is_empty() {
        return Err(fail("不能为空"));
    }
    if name == "." || name == ".." {
        return Err(fail("不能是 . 或 .."));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(fail("不能包含路径分隔符"));
    }
    if name.chars().any(|c| c.is_control()) {
        return Err(fail("不能包含控制字符"));
    }
    Ok(())
}

/// 把磁盘上的相对路径拆成（分类, 技能名）；分类最多一级。
pub fn split_vault_rel(rel: &Path) -> Result<(Option<String>, String)> {
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    match parts.len() {
        1 => Ok((None, parts.into_iter().next().expect("长度已判定"))),
        2 => {
            let mut it = parts.into_iter();
            let category = it.next().expect("长度已判定");
            let name = it.next().expect("长度已判定");
            Ok((Some(category), name))
        }
        _ => Err(CoreError::InvalidRef(format!(
            "库内技能路径超过一级分类：{}",
            rel.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_roundtrip() {
        assert_eq!(Domain::from_str("code").unwrap(), Domain::Code);
        assert_eq!(Domain::from_str("lobster").unwrap(), Domain::Lobster);
        assert!(Domain::from_str("other").is_err());
    }

    #[test]
    fn skill_ref_parses_domain_and_name() {
        let r = SkillRef::parse("code/matt-pr").unwrap();
        assert_eq!(r.domain, Domain::Code);
        assert_eq!(r.name, "matt-pr");
        assert_eq!(r.to_string(), "code/matt-pr");
        assert!(SkillRef::parse("matt-pr").is_err());
    }

    #[test]
    fn skill_name_rejects_path_segments() {
        assert!(SkillRef::new(Domain::Code, "a/b").is_err());
        assert!(SkillRef::new(Domain::Code, "..").is_err());
        assert!(SkillRef::new(Domain::Code, "").is_err());
        assert!(SkillRef::new(Domain::Code, "正常技能名").is_ok());
    }

    #[test]
    fn same_domain_names_are_case_insensitive() {
        // 同域内大小写不敏感比较；跨域同名允许。
        assert!(names_equal("Matt-PR", "matt-pr"));
        assert!(SkillRef::new(Domain::Lobster, "matt-pr").is_ok());
    }

    #[test]
    fn domain_dirs_are_distinct_but_names_can_repeat() {
        let a = SkillRef::new(Domain::Code, "dup").unwrap();
        let b = SkillRef::new(Domain::Lobster, "dup").unwrap();
        assert_ne!(a.rel_dir(None), b.rel_dir(None));
    }

    #[test]
    fn target_roundtrip() {
        for raw in ["agent:codex", "shared", "hermes:default"] {
            let parsed: Target = raw.parse().unwrap();
            assert_eq!(parsed.as_str(), raw);
        }
        assert!(Target::from_str("codex").is_err());
        assert!(Target::from_str("agent:").is_err());
    }

    #[test]
    fn rel_dir_respects_single_level_category() {
        let r = SkillRef::new(Domain::Code, "skill-a").unwrap();
        assert_eq!(r.rel_dir(None), PathBuf::from("code/skill-a"));
        assert_eq!(r.rel_dir(Some("工具")), PathBuf::from("code/工具/skill-a"));
    }

    #[test]
    fn split_vault_rel_handles_zero_or_one_category() {
        let (c, n) = split_vault_rel(Path::new("skill-a")).unwrap();
        assert_eq!(c, None);
        assert_eq!(n, "skill-a");
        let (c, n) = split_vault_rel(Path::new("工具/skill-a")).unwrap();
        assert_eq!(c.as_deref(), Some("工具"));
        assert_eq!(n, "skill-a");
        assert!(split_vault_rel(Path::new("a/b/c")).is_err());
    }
}
