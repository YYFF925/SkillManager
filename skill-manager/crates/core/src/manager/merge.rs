//! 上游更新合并（T044，D3「同项上游值优先」）。
//!
//! 规则（FR-032、US4-5）：
//! - 基础内容文件（不含 `.manager`）：以上游为准镜像——上游新增/修改则采用，
//!   上游已删除的文件从库内移除（技能内文件删除属于内容更新，不删除整个技能）。
//! - `meta.yaml`：**逐键**，上游提供则该键采用上游值（本地改过同项也不进冲突流程），
//!   上游未提供的键保留本地值（含未知字段）。
//! - `translation.md`：整份一项，上游提供则采用；本地未维护时保留。原文变化而本次未维护译文时
//!   保留译文，时效依据（`translation_source_hash`）自然失配从而给出待更新提示。
//! - `overlay/`：同路径文件一项，上游提供的路径采用上游文件，本地独有的路径保留。
//! - 分类目录由本地保留（合并只写技能目录内容，不移动目录）。
//!
//! 本模块只做内容合并；采用版本推进、部署同步由 `upstream/apply.rs` 负责。

use std::collections::BTreeMap;
use std::path::Path;

use crate::deploy::compose::{collect_base_files, remove_path};
use crate::error::{CoreError, Result};
use crate::manager::{self, MetaFile};

use crate::upstream::changes;

/// 一次合并的逐项结论（用于更新报告）。
#[derive(Clone, Debug, Default)]
pub struct MergeReport {
    pub base: changes::BaseDiff,
    /// 采用上游值的 meta 键。
    pub meta_changed: Vec<String>,
    pub translation_updated: bool,
    /// 采用上游文件的 overlay 项（`<agent>/<os>:<相对路径>`）。
    pub overlays_updated: Vec<String>,
}

impl MergeReport {
    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
            && self.meta_changed.is_empty()
            && !self.translation_updated
            && self.overlays_updated.is_empty()
    }
}

/// 软件已知的 meta 键（变化报告只对已知同项给结论）。
const KNOWN_META_KEYS: [&str; 5] = [
    "tags",
    "comment",
    "agents",
    "oses",
    manager::TRANSLATION_SOURCE_HASH_KEY,
];

/// 把上游技能目录的内容合并进库内技能目录。调用者随后负责推进采用版本与同步部署。
pub fn merge_from_upstream(skill_dir: &Path, upstream_dir: &Path) -> Result<MergeReport> {
    let base = changes::base_diff(skill_dir, upstream_dir)?;

    let local_before = MetaFile::load(skill_dir)?;
    let upstream_meta = MetaFile::load(upstream_dir)?;
    let upstream_translation = manager::read_translation(upstream_dir)?;

    let mut meta_changed = Vec::new();
    for key in KNOWN_META_KEYS {
        let provided = upstream_meta.raw_value(key);
        if provided.is_some() && provided != local_before.raw_value(key) {
            meta_changed.push(key.to_string());
        }
    }

    apply_base(skill_dir, upstream_dir, &base)?;

    // 译文先写（会按当前原文更新时效依据），随后 meta 合并会用上游显式提供的依据覆盖。
    let translation_updated = if let Some(text) = &upstream_translation {
        manager::write_translation(skill_dir, text)?;
        true
    } else {
        false
    };

    let mut local_meta = MetaFile::load(skill_dir)?;
    if local_meta.merge_from(&upstream_meta) {
        local_meta.save(skill_dir)?;
    }

    let mut overlays_updated = Vec::new();
    for entry in manager::list_overlays(upstream_dir)? {
        let provided = manager::read_overlay(
            upstream_dir,
            entry.agent.as_deref(),
            entry.os.as_deref(),
            &entry.rel,
        )?;
        let Some(bytes) = provided else { continue };
        let current = manager::read_overlay(
            skill_dir,
            entry.agent.as_deref(),
            entry.os.as_deref(),
            &entry.rel,
        )?;
        if current.as_deref() == Some(bytes.as_slice()) {
            continue;
        }
        manager::write_overlay(
            skill_dir,
            entry.agent.as_deref(),
            entry.os.as_deref(),
            &entry.rel,
            &bytes,
        )?;
        overlays_updated.push(format!(
            "{}/{}:{}",
            entry.agent.as_deref().unwrap_or(manager::GENERIC_DIMENSION),
            entry.os.as_deref().unwrap_or(manager::GENERIC_DIMENSION),
            entry.rel
        ));
    }

    Ok(MergeReport {
        base,
        meta_changed,
        translation_updated,
        overlays_updated,
    })
}

/// 镜像上游基础内容：删除本地多出的文件，写入上游文件（含修改与新增）。
fn apply_base(skill_dir: &Path, upstream_dir: &Path, base: &changes::BaseDiff) -> Result<()> {
    for rel in &base.deleted {
        remove_path(&skill_dir.join(rel.split('/').collect::<std::path::PathBuf>()))?;
    }
    let upstream: BTreeMap<String, Vec<u8>> = collect_base_files(upstream_dir)?.into_iter().collect();
    for (rel, bytes) in &upstream {
        let path = skill_dir.join(rel.split('/').collect::<std::path::PathBuf>());
        if path.exists() && path.is_dir() {
            remove_path(&path)?;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| CoreError::io(parent.to_path_buf(), err))?;
        }
        std::fs::write(&path, bytes).map_err(|err| CoreError::io(path, err))?;
    }
    Ok(())
}
