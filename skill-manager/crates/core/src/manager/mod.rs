//! 管理层（`.manager`）：技能的六项管理资料与来源记录。
//!
//! 管理资料是 SkillManagerVault 内的共享文件，GUI、CLI 与外部 Agent 手改地位平等；
//! 本模块只负责文件读写与字段形状校验，不承载部署或获取规则。

pub mod files;
pub mod merge;

pub use merge::{merge_from_upstream, MergeReport};
pub use files::{
    entry_content_hash, list_overlays, manager_dir, meta_path, normalize_tags, overlay_dir,
    overlay_path, read_overlay, read_optional, read_source, read_translation, remove_overlay,
    remove_translation, source_path, translation_path, write_overlay, write_source,
    write_translation, MetaFile, OverlayEntry, SkillMeta, SourceRecord, SupportDecl,
    GENERIC_DIMENSION, MANAGER_DIR, META_FILE, OVERLAY_DIR, SOURCE_FILE, TRANSLATION_FILE,
    TRANSLATION_SOURCE_HASH_KEY,
};
