//! 自建技能（T036）：在 SkillVault 中创建技能目录与管理资料，**不部署**（FR-015、US2-6）。
//!
//! 只创建库内资产；首次部署是另一个独立动作（`deploy to`）。复用 US1 的同名与分类校验。

use std::path::Path;

use crate::domain::{validate_category_name, SkillRef};
use crate::error::{CoreError, Result};
use crate::manager::MetaFile;
use crate::vault::manage::ensure_unique_name;

/// 创建自建技能目录及其入口文件与初始管理资料。
///
/// 入口文件使用最小 frontmatter（承载目录名），使后续扫描/部署一致可识别。
pub fn create_skill(
    vault_root: &Path,
    reference: &SkillRef,
    category: Option<&str>,
    comment: Option<&str>,
    tags: &[String],
) -> Result<()> {
    if let Some(category) = category {
        validate_category_name(category)?;
    }
    ensure_unique_name(vault_root, reference.domain, &reference.name, None)?;

    let target = reference.dir_in(vault_root, category);
    if target.exists() {
        return Err(CoreError::DuplicateSkill {
            domain: reference.domain.to_string(),
            name: reference.name.clone(),
        });
    }
    std::fs::create_dir_all(&target).map_err(|err| CoreError::io(target.clone(), err))?;

    let entry = format!(
        "---\nname: {name}\n---\n\n# {name}\n",
        name = reference.name
    );
    let entry_path = target.join("SKILL.md");
    std::fs::write(&entry_path, entry.as_bytes()).map_err(|err| CoreError::io(entry_path, err))?;

    if comment.is_some() || !tags.is_empty() {
        let mut meta = MetaFile::load(&target)?;
        meta.set_comment(comment);
        meta.set_tags(tags);
        meta.save(&target)?;
    }
    Ok(())
}
