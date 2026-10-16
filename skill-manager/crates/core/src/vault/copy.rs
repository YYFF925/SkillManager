//! 跨管理域复制技能（T036，US2-7）。
//!
//! 复制保留原件与来源记录（含 `source.yaml`），两份**独立维护、不互相同步**；
//! 复制**不部署**，也不新增任何目标。复用 US1 的同名与分类校验。

use std::path::Path;

use crate::domain::{Domain, SkillRef};
use crate::error::{CoreError, Result};
use crate::vault::acquire::copy_skill_dir;
use crate::vault::manage::{ensure_unique_name, locate};

/// 把技能复制到另一个管理域；`category` 为副本的分类（`None`=域根下）。
pub fn copy_skill(
    vault_root: &Path,
    reference: &SkillRef,
    to: Domain,
    category: Option<&str>,
) -> Result<SkillRef> {
    if let Some(category) = category {
        crate::domain::validate_category_name(category)?;
    }
    let source = locate(vault_root, reference)?;
    let target_ref = SkillRef {
        domain: to,
        name: reference.name.clone(),
    };
    ensure_unique_name(vault_root, to, &reference.name, None)?;

    let target = target_ref.dir_in(vault_root, category);
    if target.exists() {
        return Err(CoreError::DuplicateSkill {
            domain: to.to_string(),
            name: reference.name.clone(),
        });
    }
    copy_skill_dir(&source, &target)?;
    Ok(target_ref)
}
