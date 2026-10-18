//! 库内维护操作：分类移动与重命名、标签增删与全库治理、管理资料保存。
//!
//! 所有操作只改库内文件（SkillManagerVault 是共享内容真值），不写本机状态、不部署。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::domain::{Domain, SkillRef, names_equal, validate_category_name};
use crate::error::{CoreError, Result};
use crate::manager::{MetaFile, SupportDecl};
use crate::vault::index::SkillIndex;
use crate::vault::scan;

/// 批量操作的单项结果。
#[derive(Clone, Debug, Serialize)]
pub struct ManageItem {
    pub skill: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ManageItem {
    fn ok(reference: &SkillRef) -> Self {
        Self {
            skill: reference.to_string(),
            ok: true,
            message: None,
        }
    }

    fn failed(reference: &SkillRef, error: CoreError) -> Self {
        Self {
            skill: reference.to_string(),
            ok: false,
            message: Some(error.to_string()),
        }
    }
}

fn not_found(reference: &SkillRef) -> CoreError {
    CoreError::SkillNotFound {
        domain: reference.domain.to_string(),
        name: reference.name.clone(),
    }
}

/// 定位技能目录。索引从库文件重建，因此外部移动目录后仍能解析。
pub fn locate(root: &Path, reference: &SkillRef) -> Result<PathBuf> {
    let index = SkillIndex::build(root)?;
    index
        .get(reference)
        .map(|entry| entry.dir.clone())
        .ok_or_else(|| not_found(reference))
}

/// 同域同名校验（大小写不敏感）；`ignore` 用于忽略技能自身（原地改写）。
///
/// 供创建、入库、收编、复制与移动复用（FR-004）：同域阻止，跨域允许。
pub fn ensure_unique_name(
    root: &Path,
    domain: Domain,
    name: &str,
    ignore: Option<&Path>,
) -> Result<()> {
    for scanned in scan::scan_domain(root, domain)? {
        if !names_equal(&scanned.name, name) {
            continue;
        }
        if let Some(ignore) = ignore {
            if same_path(&scanned.dir, ignore) {
                continue;
            }
        }
        return Err(CoreError::DuplicateSkill {
            domain: domain.to_string(),
            name: name.to_string(),
        });
    }
    Ok(())
}

/// 分类移动：移动实际目录，技能名与身份不变（FR-003）。
pub fn move_to_category(
    root: &Path,
    reference: &SkillRef,
    category: Option<&str>,
) -> Result<PathBuf> {
    if let Some(category) = category {
        validate_category_name(category)?;
    }

    let source = locate(root, reference)?;
    let target = reference.dir_in(root, category);
    if same_path(&source, &target) {
        return Ok(target);
    }

    // 目标目录已被占用（含大小写不同的同名技能）时阻止，并给出可定位原因。
    if target.exists() {
        return Err(blocked_move(reference, &target));
    }
    let index = SkillIndex::build(root)?;
    if let Some(other) = index.all().iter().find(|entry| {
        entry.domain == reference.domain
            && names_equal(&entry.name, &reference.name)
            && entry.category.as_deref() == category
            && !same_path(&entry.dir, &source)
    }) {
        return Err(blocked_move(reference, &other.dir));
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|err| CoreError::io(parent, err))?;
    }
    std::fs::rename(&source, &target).map_err(|err| CoreError::io(target.clone(), err))?;
    Ok(target)
}

/// 重命名一级分类目录：该分类下全部技能一并移动，技能名与部署名不变（FR-003）。
///
/// 目标分类已存在时拒绝合并；同名不改动。
pub fn rename_category(root: &Path, domain: Domain, from: &str, to: &str) -> Result<usize> {
    let from = from.trim();
    let to = to.trim();
    validate_category_name(from)?;
    validate_category_name(to)?;
    if from == to {
        return Ok(0);
    }

    let source = root.join(domain.dir_name()).join(from);
    let target = root.join(domain.dir_name()).join(to);
    if !source.is_dir() {
        return Err(CoreError::other(format!(
            "分类「{from}」不存在（{}）",
            domain.label()
        )));
    }
    if target.exists() && !same_path(&source, &target) {
        return Err(CoreError::other(format!(
            "目标分类「{to}」已存在，未重命名"
        )));
    }

    std::fs::rename(&source, &target).map_err(|err| CoreError::io(target.clone(), err))?;

    let index = SkillIndex::build(root)?;
    Ok(index
        .all()
        .iter()
        .filter(|entry| {
            entry.domain == domain
                && entry
                    .category
                    .as_deref()
                    .is_some_and(|category| names_equal(category, to))
        })
        .count())
}

fn blocked_move(reference: &SkillRef, target: &Path) -> CoreError {
    CoreError::other(format!(
        "目标分类目录已存在，未移动：{}（技能 {reference}）",
        target.display()
    ))
}

fn same_path(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// 管理资料写入
// ---------------------------------------------------------------------------

/// 管理资料修改请求；`None` 表示保持原值（不是清空）。
#[derive(Clone, Debug, Default)]
pub struct MetaUpdate {
    /// `Some(None)` = 清除注释。
    pub comment: Option<Option<String>>,
    /// `Some(vec)` = 替换标签集合。
    pub tags: Option<Vec<String>>,
    /// `Some(All)` = 删除限制键恢复“全部”。
    pub agents: Option<SupportDecl>,
    pub oses: Option<SupportDecl>,
}

/// 一次加载、就地改键、写回：不传的字段保持原值，未知字段保留。
pub fn update_meta(root: &Path, reference: &SkillRef, update: &MetaUpdate) -> Result<()> {
    let dir = locate(root, reference)?;
    let mut meta = MetaFile::load(&dir)?;
    if let Some(tags) = &update.tags {
        meta.set_tags(tags);
    }
    if let Some(comment) = &update.comment {
        meta.set_comment(comment.as_deref());
    }
    if let Some(agents) = &update.agents {
        meta.set_agents(agents);
    }
    if let Some(oses) = &update.oses {
        meta.set_oses(oses);
    }
    meta.save(&dir)
}

/// 替换标签集合（空集合=无标签）。
pub fn set_tags(root: &Path, reference: &SkillRef, tags: &[String]) -> Result<()> {
    update_meta(
        root,
        reference,
        &MetaUpdate {
            tags: Some(tags.to_vec()),
            ..MetaUpdate::default()
        },
    )
}

/// 设置或清除中文注释。
pub fn set_comment(root: &Path, reference: &SkillRef, comment: Option<&str>) -> Result<()> {
    update_meta(
        root,
        reference,
        &MetaUpdate {
            comment: Some(comment.map(str::to_string)),
            ..MetaUpdate::default()
        },
    )
}

/// 设置支持声明；`All` 删除对应键恢复“全部”。
pub fn set_support(
    root: &Path,
    reference: &SkillRef,
    agents: SupportDecl,
    oses: SupportDecl,
) -> Result<()> {
    update_meta(
        root,
        reference,
        &MetaUpdate {
            agents: Some(agents),
            oses: Some(oses),
            ..MetaUpdate::default()
        },
    )
}

// ---------------------------------------------------------------------------
// 批量标签增删
// ---------------------------------------------------------------------------

/// 批量添加标签；只增减目标标签，其他标签原样保留。
pub fn add_tags_many(root: &Path, references: &[SkillRef], tags: &[String]) -> Vec<ManageItem> {
    apply_tags(root, references, tags, true)
}

/// 批量移除标签；只增减目标标签，其他标签原样保留。
pub fn detach_tags_many(root: &Path, references: &[SkillRef], tags: &[String]) -> Vec<ManageItem> {
    apply_tags(root, references, tags, false)
}

fn apply_tags(
    root: &Path,
    references: &[SkillRef],
    tags: &[String],
    add: bool,
) -> Vec<ManageItem> {
    references
        .iter()
        .map(|reference| match update_tags(root, reference, tags, add) {
            Ok(()) => ManageItem::ok(reference),
            Err(err) => ManageItem::failed(reference, err),
        })
        .collect()
}

fn update_tags(root: &Path, reference: &SkillRef, tags: &[String], add: bool) -> Result<()> {
    let dir = locate(root, reference)?;
    let mut meta = MetaFile::load(&dir)?;
    let mut current = meta.meta()?.tags;

    for tag in tags {
        let tag = tag.trim();
        if tag.is_empty() {
            continue;
        }
        if add {
            current.push(tag.to_string());
        } else {
            current.retain(|existing| existing != tag);
        }
    }

    meta.set_tags(&current);
    meta.save(&dir)
}

// ---------------------------------------------------------------------------
// 全库标签治理
// ---------------------------------------------------------------------------

/// 全库重命名标签（两域都生效）；返回受影响技能数。
pub fn rename_tag(root: &Path, old: &str, new: &str) -> Result<usize> {
    let old = old.trim();
    let new = new.trim();
    if old.is_empty() || new.is_empty() {
        return Err(CoreError::other("标签名不能为空"));
    }
    if old == new {
        return Ok(0);
    }

    let index = SkillIndex::build(root)?;
    let mut affected = 0;
    for entry in index.all() {
        if !entry.tags.iter().any(|tag| tag == old) {
            continue;
        }
        let mut meta = MetaFile::load(&entry.dir)?;
        let tags: Vec<String> = meta
            .meta()?
            .tags
            .into_iter()
            .map(|tag| if tag == old { new.to_string() } else { tag })
            .collect();
        meta.set_tags(&tags);
        meta.save(&entry.dir)?;
        affected += 1;
    }
    Ok(affected)
}

/// 全库删除标签关联（两域都生效，不删除技能）；返回受影响技能数。
pub fn remove_tag(root: &Path, tag: &str) -> Result<usize> {
    let tag = tag.trim();
    if tag.is_empty() {
        return Err(CoreError::other("标签名不能为空"));
    }

    let index = SkillIndex::build(root)?;
    let mut affected = 0;
    for entry in index.all() {
        if !entry.tags.iter().any(|existing| existing == tag) {
            continue;
        }
        let mut meta = MetaFile::load(&entry.dir)?;
        let tags: Vec<String> = meta
            .meta()?
            .tags
            .into_iter()
            .filter(|existing| existing != tag)
            .collect();
        meta.set_tags(&tags);
        meta.save(&entry.dir)?;
        affected += 1;
    }
    Ok(affected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_and_unique_name_share_case_insensitive_comparison() {
        let root = tempfile::TempDir::new().unwrap();
        let dir = root.path().join("code/alpha");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), "a").unwrap();

        assert!(ensure_unique_name(root.path(), Domain::Code, "ALPHA", None).is_err());
        assert!(ensure_unique_name(root.path(), Domain::Code, "alpha", Some(&dir)).is_ok());
        assert!(ensure_unique_name(root.path(), Domain::Lobster, "alpha", None).is_ok());
    }
}
