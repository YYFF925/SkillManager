//! 部署准入：管理域边界与支持声明过滤（FR-007、FR-017）。
//!
//! 两条规则互相不可绕过：域边界为硬约束；支持声明对“新增部署”为硬约束，
//! 对“已有部署”按 FR-007 保留并警告。共享目录是部署目标，要求所有消费者受支持。

use crate::domain::{Domain, Target};
use crate::error::{CoreError, Result};
use crate::vault::index::SkillEntry;

/// Hermes default 档案对应的 Agent key（用于覆盖层维度与支持声明）。
pub const HERMES_AGENT_KEY: &str = "hermes";

/// 该目标在覆盖层与支持声明中使用的 Agent 维度；`None` 表示通用（共享目录）。
pub fn agent_dimension(target: &Target) -> Option<&str> {
    match target {
        Target::Agent { key } => Some(key.as_str()),
        Target::Shared => None,
        Target::HermesDefault => Some(HERMES_AGENT_KEY),
    }
}

/// 准入结论。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Admission {
    /// 允许部署。
    Allowed,
    /// 该目标不在支持范围内：禁止**新增**部署；已有部署保留并警告（FR-007）。
    Unsupported(String),
}

/// 管理域边界：编码库技能不得直接部署给龙虾 Agent，反之亦然（不可由支持声明绕过）。
pub fn check_domain(skill_domain: Domain, target_domain: Domain, target: &Target) -> Result<()> {
    if skill_domain == target_domain {
        return Ok(());
    }
    Err(CoreError::other(format!(
        "管理域边界：{} 技能不能直接部署到 {}（{}）",
        skill_domain.label(),
        target_domain.label(),
        target
    )))
}

/// 支持声明过滤：未声明=所属域内全支持；显式空集合=不支持任何目标。
pub fn support_admission(entry: &SkillEntry, target: &Target, os: &str) -> Admission {
    if let Some(oses) = &entry.oses {
        let supported = oses.iter().any(|declared| declared == os);
        if !supported {
            return Admission::Unsupported(format!(
                "当前系统 {os} 不在该技能声明支持的系统内（{}）",
                list(oses)
            ));
        }
    }

    match target {
        // 共享目录：要求所有消费者（编码 Agent）都受支持。
        Target::Shared => match &entry.agents {
            None => Admission::Allowed,
            Some(agents) if agents.is_empty() => Admission::Unsupported(
                "共享目录部署要求所有消费者受支持，该技能声明为不支持任何 Agent（显式空支持集合）"
                    .to_string(),
            ),
            Some(agents) => Admission::Unsupported(format!(
                "共享目录部署要求所有消费者（编码 Agent）都受支持，该技能已收窄 Agent 支持范围为 {}",
                list(agents)
            )),
        },
        _ => {
            let key = agent_dimension(target).unwrap_or_default();
            match &entry.agents {
                None => Admission::Allowed,
                Some(agents) if agents.is_empty() => Admission::Unsupported(
                    "该技能声明为不支持任何 Agent（显式空支持集合）".to_string(),
                ),
                Some(agents) => {
                    if agents.iter().any(|declared| declared == key) {
                        Admission::Allowed
                    } else {
                        Admission::Unsupported(format!(
                            "该技能未声明支持目标 {target}（支持范围：{}）",
                            list(agents)
                        ))
                    }
                }
            }
        }
    }
}

fn list(values: &[String]) -> String {
    if values.is_empty() {
        "无".to_string()
    } else {
        values.join("、")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(agents: Option<Vec<String>>, oses: Option<Vec<String>>) -> SkillEntry {
        SkillEntry {
            domain: Domain::Code,
            name: "alpha".to_string(),
            category: None,
            dir: std::path::PathBuf::from("/vault/code/alpha"),
            vault_rel_dir: "code/alpha".to_string(),
            entry_file: "SKILL.md".to_string(),
            description: None,
            comment: None,
            tags: Vec::new(),
            agents,
            oses,
            source: None,
            has_translation: false,
            translation_stale: false,
            manager_error: None,
            presets: Vec::new(),
        }
    }

    fn agent(key: &str) -> Target {
        Target::Agent {
            key: key.to_string(),
        }
    }

    #[test]
    fn undeclared_support_is_all_within_domain() {
        let e = entry(None, None);
        assert_eq!(support_admission(&e, &agent("codex"), "macos"), Admission::Allowed);
        assert_eq!(support_admission(&e, &Target::Shared, "macos"), Admission::Allowed);
    }

    #[test]
    fn explicit_empty_support_has_no_target() {
        let e = entry(Some(Vec::new()), None);
        assert!(matches!(
            support_admission(&e, &agent("codex"), "macos"),
            Admission::Unsupported(_)
        ));
    }

    #[test]
    fn narrowed_agent_list_filters_targets() {
        let e = entry(Some(vec!["zcode".to_string()]), None);
        assert_eq!(support_admission(&e, &agent("zcode"), "macos"), Admission::Allowed);
        assert!(matches!(
            support_admission(&e, &agent("codex"), "macos"),
            Admission::Unsupported(_)
        ));
        assert!(matches!(
            support_admission(&e, &Target::Shared, "macos"),
            Admission::Unsupported(_)
        ));
    }

    #[test]
    fn os_declaration_filters_current_system() {
        let e = entry(None, Some(vec!["macos".to_string()]));
        assert_eq!(support_admission(&e, &agent("codex"), "macos"), Admission::Allowed);
        assert!(matches!(
            support_admission(&e, &agent("codex"), "windows"),
            Admission::Unsupported(_)
        ));
    }

    #[test]
    fn domain_boundary_is_reported() {
        let err = check_domain(Domain::Code, Domain::Lobster, &Target::HermesDefault).unwrap_err();
        assert!(err.to_string().contains("管理域"), "{err}");
        assert!(check_domain(Domain::Code, Domain::Code, &agent("codex")).is_ok());
    }
}
