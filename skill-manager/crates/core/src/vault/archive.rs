//! 技能归档：把暂时不用的技能移出正常库，完整保留目录与 `.manager`。
//!
//! 归档目录随 SkillManagerVault 同步：`archive/<domain>/[<category>/]<name>/`。
//! 卸载受管部署由 DeploymentService 在移动前完成，本模块只负责库内文件移动。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::domain::{names_equal, Domain, SkillRef};
use crate::error::{CoreError, Result};
use crate::manager::MetaFile;
use crate::vault::scan::{self, ScannedSkill};

/// 库根下的归档目录名。
pub const ARCHIVE_DIR: &str = "archive";

/// 一条已归档技能（分类、标签等仍在目录与 `.manager` 中）。
#[derive(Clone, Debug, Serialize)]
pub struct ArchivedSkill {
    pub domain: Domain,
    pub name: String,
    pub category: Option<String>,
    pub vault_rel_dir: String,
    pub comment: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip)]
    pub dir: PathBuf,
}

/// `SkillManagerVault/archive`。
pub fn archive_root(vault_root: &Path) -> PathBuf {
    vault_root.join(ARCHIVE_DIR)
}

/// 归档内技能目录：`archive/<domain>/[<category>/]<name>`。
pub fn archived_dir(vault_root: &Path, reference: &SkillRef, category: Option<&str>) -> PathBuf {
    reference.dir_in(&archive_root(vault_root), category)
}

/// 列出已归档技能；`domain` 为 None 时两域都列。
pub fn list_archived(root: &Path, domain: Option<Domain>) -> Result<Vec<ArchivedSkill>> {
    let domains = match domain {
        Some(domain) => vec![domain],
        None => Domain::ALL.to_vec(),
    };
    let mut out = Vec::new();
    for domain in domains {
        let base = archive_root(root).join(domain.dir_name());
        for scanned in scan::scan_domain_from(&base, domain)? {
            out.push(to_archived(scanned));
        }
    }
    out.sort_by(|left, right| {
        left.domain
            .cmp(&right.domain)
            .then(left.category.cmp(&right.category))
            .then(left.name.cmp(&right.name))
    });
    Ok(out)
}

/// 按引用查找一条归档技能（同域同名，大小写不敏感）。
pub fn find_archived(root: &Path, reference: &SkillRef) -> Result<ArchivedSkill> {
    list_archived(root, Some(reference.domain))?
        .into_iter()
        .find(|item| names_equal(&item.name, &reference.name))
        .ok_or_else(|| CoreError::other(format!("归档中没有技能 {reference}")))
}

/// 把库内技能目录移入归档，保留原分类层级。
pub fn move_to_archive(
    root: &Path,
    reference: &SkillRef,
    category: Option<&str>,
) -> Result<ArchivedSkill> {
    if let Some(existing) = list_archived(root, Some(reference.domain))?
        .into_iter()
        .find(|item| names_equal(&item.name, &reference.name))
    {
        return Err(CoreError::other(format!(
            "归档中已有同名技能 {reference}（{}）",
            existing.vault_rel_dir
        )));
    }

    let source = reference.dir_in(root, category);
    if !source.is_dir() {
        return Err(CoreError::SkillNotFound {
            domain: reference.domain.to_string(),
            name: reference.name.clone(),
        });
    }
    let target = archived_dir(root, reference, category);
    if target.exists() {
        return Err(CoreError::other(format!(
            "归档目标已存在，未移动：{}",
            target.display()
        )));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|err| CoreError::io(parent, err))?;
    }
    std::fs::rename(&source, &target).map_err(|err| CoreError::io(target.clone(), err))?;
    Ok(describe_dir(
        reference.domain,
        category.map(str::to_string),
        &target,
    ))
}

/// 从归档还原到正常库，保留原分类；不部署。
pub fn restore_from_archive(root: &Path, reference: &SkillRef) -> Result<ArchivedSkill> {
    let archived = find_archived(root, reference)?;
    let live = crate::vault::index::SkillIndex::build(root)?;
    if live.get(reference).is_some() {
        return Err(CoreError::DuplicateSkill {
            domain: reference.domain.to_string(),
            name: reference.name.clone(),
        });
    }
    let target = reference.dir_in(root, archived.category.as_deref());
    if target.exists() {
        return Err(CoreError::other(format!(
            "还原目标已存在，未移动：{}",
            target.display()
        )));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|err| CoreError::io(parent, err))?;
    }
    std::fs::rename(&archived.dir, &target).map_err(|err| CoreError::io(target.clone(), err))?;
    Ok(describe_dir(reference.domain, archived.category, &target))
}

/// 从归档永久删除（库内技能与受管部署应已不在场）。
pub fn delete_archived(root: &Path, reference: &SkillRef) -> Result<ArchivedSkill> {
    let archived = find_archived(root, reference)?;
    std::fs::remove_dir_all(&archived.dir)
        .map_err(|err| CoreError::io(archived.dir.clone(), err))?;
    Ok(archived)
}

fn to_archived(scanned: ScannedSkill) -> ArchivedSkill {
    let mut item = describe_dir(scanned.domain, scanned.category, &scanned.dir);
    item.vault_rel_dir = format!("{ARCHIVE_DIR}/{}", scanned.vault_rel_dir);
    item
}

fn describe_dir(domain: Domain, category: Option<String>, dir: &Path) -> ArchivedSkill {
    let name = dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (comment, tags) = match MetaFile::load(dir).and_then(|meta| meta.meta()) {
        Ok(meta) => (meta.comment, meta.tags),
        Err(_) => (None, Vec::new()),
    };
    let vault_rel_dir = match &category {
        Some(category) => format!("{ARCHIVE_DIR}/{}/{category}/{name}", domain.dir_name()),
        None => format!("{ARCHIVE_DIR}/{}/{name}", domain.dir_name()),
    };
    ArchivedSkill {
        domain,
        name,
        category,
        vault_rel_dir,
        comment,
        tags,
        dir: dir.to_path_buf(),
    }
}
