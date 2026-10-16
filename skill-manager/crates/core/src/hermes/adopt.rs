//! 档案独有技能收编（T059）。
//!
//! 把档案目录中未受管的独有技能收编到龙虾库并部署到 default 档案，**原档案启用状态保持不变**。
//! 复用 US1 的扫描/资料读写、`vault::manage::ensure_unique_name`（同域同名阻止）与
//! US3 的 `DeploymentService::deploy_to`（副本接管/部署）。不依赖 US2 的上游获取，
//! 不创建第二套库内技能内容。收编完成后档案目录成为指向 default 共享副本的链接，
//! 使“所有档案共用库内一份技能内容”成立；内容已先复制进库与 default，不会丢失。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::deploy::DeploymentService;
use crate::domain::{names_equal, validate_skill_name, Domain, SkillRef, Target};
use crate::error::{CoreError, Result};
use crate::vault::manage::ensure_unique_name;

use super::disabled::read_disabled;
use super::enables::{create_dir_symlink, link_state, scan_skill_dirs, HermesService, LinkState};
use super::profiles::resolve_profile;

/// 收编的逐项结果。
#[derive(Clone, Debug, Serialize)]
pub struct AdoptItem {
    pub name: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_skill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl AdoptItem {
    fn failed(name: &str, error: &CoreError) -> Self {
        Self {
            name: name.to_string(),
            ok: false,
            library_skill: None,
            default_path: None,
            message: Some(error.to_string()),
        }
    }
}

impl<'a> HermesService<'a> {
    /// 批量收编某具名档案的独有技能到龙虾库并部署 default。
    pub fn adopt(&self, profile: &str, names: &[String]) -> Result<Vec<AdoptItem>> {
        let info = resolve_profile(self.root(), profile)?;
        if info.is_default() {
            return Err(CoreError::other(
                "default 档案的技能已直接落在库内；请用技能库维护，无需收编",
            ));
        }
        let vault_root = self
            .vault_root()
            .ok_or_else(|| CoreError::other("尚未接入 SkillVault：收编需要写入龙虾库"))?
            .to_path_buf();

        let mut items = Vec::new();
        for name in names {
            items.push(
                match self.adopt_one(profile, &vault_root, &info.skills_dir, name) {
                    Ok(item) => item,
                    Err(err) => AdoptItem::failed(name, &err),
                },
            );
        }
        Ok(items)
    }

    fn adopt_one(
        &self,
        profile: &str,
        vault_root: &Path,
        profile_skills: &Path,
        name: &str,
    ) -> Result<AdoptItem> {
        validate_skill_name(name)?;
        let source = locate_profile_skill(profile_skills, name).ok_or_else(|| {
            CoreError::other(format!(
                "档案技能目录中没有该技能：{name}（{}）",
                profile_skills.display()
            ))
        })?;
        if !matches!(link_state(&source), LinkState::Real) {
            return Err(CoreError::other(format!(
                "该技能已由本管理器接管（档案内不是实目录）：{}",
                source.display()
            )));
        }

        // FR-004：同域同名阻止（不覆盖库内原件）。
        ensure_unique_name(vault_root, Domain::Lobster, name, None)?;

        let reference = SkillRef {
            domain: Domain::Lobster,
            name: name.to_string(),
        };
        let target_dir = reference.dir_in(vault_root, None);
        if target_dir.exists() {
            return Err(CoreError::DuplicateSkill {
                domain: Domain::Lobster.to_string(),
                name: name.to_string(),
            });
        }
        copy_dir_all(&source, &target_dir)?;

        // 部署到 default 档案（副本接管）。
        let service =
            DeploymentService::new(vault_root.to_path_buf(), self.resolver(), self.state());
        let report = service.deploy_to(
            std::slice::from_ref(&reference),
            std::slice::from_ref(&Target::HermesDefault),
            false,
        )?;
        if !report.failures.is_empty() {
            let reason = report
                .failures
                .iter()
                .map(|item| item.message.clone())
                .collect::<Vec<_>>()
                .join("；");
            return Err(CoreError::other(format!(
                "已复制进龙虾库，但部署到 default 档案未完成：{reason}"
            )));
        }

        // 档案目录改为指向 default 共享副本的链接；内容已进库与 default，不会丢失。
        let copy = self.default_skills_dir().join(name);
        std::fs::remove_dir_all(&source).map_err(|err| CoreError::io(source.clone(), err))?;
        create_dir_symlink(&copy, &source).map_err(|err| CoreError::io(source.clone(), err))?;

        // 启用状态保持不变：该档案未把此技能列入 Hermes 禁用名单时登记本机启用名单。
        let disabled = read_disabled(&resolve_profile(self.root(), profile)?.home);
        if !disabled.contains(name) {
            self.state()
                .set_hermes_enable(profile, Domain::Lobster, name)?;
        }

        Ok(AdoptItem {
            name: name.to_string(),
            ok: true,
            library_skill: Some(reference.to_string()),
            default_path: Some(copy.display().to_string()),
            message: None,
        })
    }
}

/// 在档案技能目录（递归）中定位名为 `name` 的技能目录。
fn locate_profile_skill(skills_dir: &Path, name: &str) -> Option<PathBuf> {
    scan_skill_dirs(skills_dir)
        .into_iter()
        .find(|(found, _)| names_equal(found, name))
        .map(|(_, path)| path)
}

/// 递归复制目录（跳过 `.git` 与符号链接）。
fn copy_dir_all(source: &Path, target: &Path) -> Result<()> {
    std::fs::create_dir_all(target).map_err(|err| CoreError::io(target.to_path_buf(), err))?;
    let entries =
        std::fs::read_dir(source).map_err(|err| CoreError::io(source.to_path_buf(), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| CoreError::io(source.to_path_buf(), err))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".git" {
            continue;
        }
        let from = entry.path();
        let to = target.join(&name);
        let file_type = entry
            .file_type()
            .map_err(|err| CoreError::io(from.clone(), err))?;
        if file_type.is_dir() {
            copy_dir_all(&from, &to)?;
        } else if file_type.is_file() {
            std::fs::copy(&from, &to).map_err(|err| CoreError::io(to.clone(), err))?;
        }
    }
    Ok(())
}
