//! 上游变化汇总（T043）：已有技能的文件增/改/删与管理资料变化的只读计算。
//!
//! 只读取事实、不写入；判断依据是**库内当前内容**与**上游当前内容**的逐文件比较
//! （按技能目录内相对路径），管理资料按 D3 的“同项”粒度（meta 每个字段、整份译文、
//! overlay 同路径文件）比较“上游是否提供该项且与本地不同”。这不提供逐行差异界面。
//!
//! 上游改名按“删除旧技能 + 新增新技能”处理：旧子路径在上游消失即 `upstream_missing`，
//! 新子路径进入候选；不做重命名识别。

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::deploy::compose::collect_base_files;
use crate::error::Result;
use crate::manager::{self, MetaFile};

/// 基础内容文件（不含 `.manager`）的增/改/删。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct BaseDiff {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub deleted: Vec<String>,
}

impl BaseDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }
}

/// 计算基础内容变化：上游相对库内当前内容的新增/修改/删除文件。
pub fn base_diff(skill_dir: &Path, upstream_dir: &Path) -> Result<BaseDiff> {
    let local: BTreeMap<String, Vec<u8>> = collect_base_files(skill_dir)?.into_iter().collect();
    let upstream: BTreeMap<String, Vec<u8>> = collect_base_files(upstream_dir)?.into_iter().collect();

    let mut diff = BaseDiff::default();
    for (rel, bytes) in &upstream {
        match local.get(rel) {
            None => diff.added.push(rel.clone()),
            Some(existing) if existing != bytes => diff.modified.push(rel.clone()),
            Some(_) => {}
        }
    }
    for rel in local.keys() {
        if !upstream.contains_key(rel) {
            diff.deleted.push(rel.clone());
        }
    }
    Ok(diff)
}

/// 管理资料变化项：D3 同项粒度，只列出“上游提供且与本地不同”的项。
///
/// 返回形如 `meta:comment`、`translation`、`overlay:<agent>/<os>:<相对路径>`。
pub fn manager_changes(skill_dir: &Path, upstream_dir: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let local = MetaFile::load(skill_dir)?;
    let upstream = MetaFile::load(upstream_dir)?;
    for key in upstream.raw_keys() {
        if !is_known_meta_key(&key) {
            continue;
        }
        if upstream.raw_value(&key).is_some() && upstream.raw_value(&key) != local.raw_value(&key) {
            out.push(format!("meta:{key}"));
        }
    }

    if manager::read_translation(upstream_dir)?.is_some() {
        let local_translation = manager::read_translation(skill_dir)?;
        let upstream_translation = manager::read_translation(upstream_dir)?;
        if local_translation != upstream_translation {
            out.push("translation".to_string());
        }
    }

    for entry in manager::list_overlays(upstream_dir)? {
        let current = manager::read_overlay(
            skill_dir,
            entry.agent.as_deref(),
            entry.os.as_deref(),
            &entry.rel,
        )?;
        let provided = manager::read_overlay(
            upstream_dir,
            entry.agent.as_deref(),
            entry.os.as_deref(),
            &entry.rel,
        )?;
        if current != provided {
            out.push(format!(
                "overlay:{}/{}:{}",
                entry.agent.as_deref().unwrap_or(manager::GENERIC_DIMENSION),
                entry.os.as_deref().unwrap_or(manager::GENERIC_DIMENSION),
                entry.rel
            ));
        }
    }
    Ok(out)
}

/// 上游是否提供了任意管理资料（用于“仅管理资料变化”判断）。
pub fn upstream_has_manager(upstream_dir: &Path) -> Result<bool> {
    if !MetaFile::load(upstream_dir)?.raw_keys().is_empty() {
        return Ok(true);
    }
    if manager::read_translation(upstream_dir)?.is_some() {
        return Ok(true);
    }
    Ok(!manager::list_overlays(upstream_dir)?.is_empty())
}

/// D3 中软件已知的 meta 键；变化汇总只对未知键之外的同项做结论。
fn is_known_meta_key(key: &str) -> bool {
    matches!(
        key,
        "tags" | "comment" | "agents" | "oses" | manager::TRANSLATION_SOURCE_HASH_KEY
    )
}
