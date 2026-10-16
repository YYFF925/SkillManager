//! 收编本机 Agent 中未受管的技能（T035，US2-5）。
//!
//! 规则：
//! - 管理域由目标所属域决定，共享目录进入编码库；**同名库技能阻止而不覆盖**；
//! - 支持逐技能资料输入（注释/标签/分类/支持声明/译文）；公共参数只表示用户明确要求的共用值，
//!   不把同一段注释擅自套给全部技能（逐技能值用 `--metadata-file` 传入）；
//! - 调用 US3 的 `DeploymentService::adopt_existing` 接管原副本，**不新增其他目标部署**；
//! - 不强制补充上游来源（收编通常针对自建技能）。

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::deploy::targets::TargetResolver;
use crate::deploy::DeploymentService;
use crate::domain::{SkillRef, Target};
use crate::error::{CoreError, Result};
use crate::manager::{MetaFile, SupportDecl};
use crate::output::ItemError;
use crate::state::StateStore;
use crate::vault::acquire::copy_skill_dir;
use crate::vault::manage::ensure_unique_name;

/// 单个技能的收编资料输入；`None` 表示不修改该字段。
#[derive(Clone, Debug, Default)]
pub struct AdoptMeta {
    pub comment: Option<String>,
    /// `Some` 表示替换标签集合。
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
    pub agents: Option<SupportDecl>,
    pub oses: Option<SupportDecl>,
    pub translation: Option<String>,
}

impl AdoptMeta {
    /// 用逐技能值覆盖公共值（逐技能提供的字段优先）。
    fn merged(&self, common: &AdoptMeta) -> AdoptMeta {
        AdoptMeta {
            comment: self.comment.clone().or_else(|| common.comment.clone()),
            tags: self.tags.clone().or_else(|| common.tags.clone()),
            category: self.category.clone().or_else(|| common.category.clone()),
            agents: self.agents.clone().or_else(|| common.agents.clone()),
            oses: self.oses.clone().or_else(|| common.oses.clone()),
            translation: self
                .translation
                .clone()
                .or_else(|| common.translation.clone()),
        }
    }
}

/// 收编逐项结果。
#[derive(Clone, Debug, Serialize)]
pub struct AdoptItem {
    pub name: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// 一次收编的结论。
#[derive(Clone, Debug, Default, Serialize)]
pub struct AdoptOutcome {
    pub agent: String,
    pub domain: String,
    pub items: Vec<AdoptItem>,
    pub adopted: usize,
    pub errors: Vec<ItemError>,
}

/// 收编本机 Agent 或共享目录的未受管技能至对应域，并接管原部署。
pub fn adopt_skills(
    vault_root: &Path,
    resolver: &TargetResolver<'static>,
    state: &StateStore,
    agent_key: &str,
    names: &[String],
    common: &AdoptMeta,
    per_skill: &BTreeMap<String, AdoptMeta>,
) -> Result<AdoptOutcome> {
    let target = if agent_key == "shared" {
        Target::Shared
    } else {
        Target::Agent {
            key: agent_key.to_string(),
        }
    };
    let domain = resolver.domain_of(&target)?;
    let skills_dir = std::path::PathBuf::from(resolver.resolve(&target)?.path);

    let mut outcome = AdoptOutcome {
        agent: agent_key.to_string(),
        domain: domain.to_string(),
        ..AdoptOutcome::default()
    };

    for name in names {
        match adopt_one(
            vault_root,
            resolver,
            state,
            &skills_dir,
            &target,
            domain,
            name,
            common,
            per_skill.get(name),
        ) {
            Ok(item) => {
                outcome.adopted += 1;
                outcome.items.push(item);
            }
            Err(err) => {
                outcome.items.push(AdoptItem {
                    name: name.clone(),
                    ok: false,
                    skill: None,
                    target: None,
                    path: None,
                    message: Some(err.to_string()),
                });
                outcome.errors.push(
                    ItemError::from_error(&err).skill(format!("{agent_key}/{name}")),
                );
            }
        }
    }
    Ok(outcome)
}

#[allow(clippy::too_many_arguments)]
fn adopt_one(
    vault_root: &Path,
    resolver: &TargetResolver<'static>,
    state: &StateStore,
    skills_dir: &Path,
    target: &Target,
    domain: crate::domain::Domain,
    name: &str,
    common: &AdoptMeta,
    per: Option<&AdoptMeta>,
) -> Result<AdoptItem> {
    crate::domain::validate_skill_name(name)?;
    let source = skills_dir.join(name);
    if !crate::vault::scan::is_skill_dir(&source) {
        return Err(CoreError::other(format!(
            "该 Agent 技能目录下没有这项技能：{}",
            source.display()
        )));
    }
    if is_managed(state, skills_dir, &source)? {
        return Err(CoreError::other(format!(
            "该技能已是受管部署，无需收编：{name}"
        )));
    }

    ensure_unique_name(vault_root, domain, name, None)?;
    let reference = SkillRef {
        domain,
        name: name.to_string(),
    };
    let meta = per.cloned().unwrap_or_default().merged(common);
    let category = meta.category.clone();
    if let Some(category) = &category {
        crate::domain::validate_category_name(category)?;
    }
    let target_dir = reference.dir_in(vault_root, category.as_deref());
    if target_dir.exists() {
        return Err(CoreError::DuplicateSkill {
            domain: domain.to_string(),
            name: name.to_string(),
        });
    }

    copy_skill_dir(&source, &target_dir)?;
    apply_meta(&target_dir, &meta)?;

    let service = DeploymentService::new(vault_root.to_path_buf(), resolver, state);
    let report = service.adopt_existing(&reference, target)?;
    if let Some(error) = report.failures.first() {
        return Err(CoreError::other(format!(
            "已复制进库，但接管原部署未完成：{}",
            error.message
        )));
    }

    Ok(AdoptItem {
        name: name.to_string(),
        ok: true,
        skill: Some(reference.to_string()),
        target: Some(target.as_str()),
        path: Some(target_dir.display().to_string()),
        message: None,
    })
}

fn apply_meta(skill_dir: &Path, meta: &AdoptMeta) -> Result<()> {
    let has_meta = meta.comment.is_some()
        || meta.tags.is_some()
        || meta.agents.is_some()
        || meta.oses.is_some();
    if has_meta {
        let mut file = MetaFile::load(skill_dir)?;
        if let Some(comment) = &meta.comment {
            file.set_comment(Some(comment));
        }
        if let Some(tags) = &meta.tags {
            file.set_tags(tags);
        }
        if let Some(agents) = &meta.agents {
            file.set_agents(agents);
        }
        if let Some(oses) = &meta.oses {
            file.set_oses(oses);
        }
        file.save(skill_dir)?;
    }
    if let Some(translation) = &meta.translation {
        crate::manager::write_translation(skill_dir, translation)?;
    }
    Ok(())
}

/// 该候选路径是否已由本管理器接管（存在指向它的部署记录）。
fn is_managed(state: &StateStore, skills_dir: &Path, candidate: &Path) -> Result<bool> {
    let _ = skills_dir;
    let canonical = candidate.canonicalize().unwrap_or_else(|_| candidate.to_path_buf());
    for record in state.deployments()? {
        let path = Path::new(&record.target_path)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(&record.target_path).to_path_buf());
        if path == canonical {
            return Ok(true);
        }
    }
    Ok(false)
}
