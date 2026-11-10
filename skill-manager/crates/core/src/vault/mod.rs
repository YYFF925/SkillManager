//! SkillManagerVault 接入、目录模型、内存索引与文件监听。

pub mod acquire;
pub mod adopt;
pub mod archive;
pub mod capacity;
pub mod connection;
pub mod copy;
pub mod create;
pub mod index;
pub mod instructions;
pub mod manage;
pub mod scan;
pub mod tasks;
pub mod watch;

pub use acquire::{CategoryChoice, ImportItem, ImportOutcome};
pub use adopt::{AdoptItem, AdoptMeta, AdoptOutcome};
pub use archive::{
    delete_archived, find_archived, list_archived, move_to_archive, restore_from_archive,
    ArchivedSkill, ARCHIVE_DIR,
};
pub use capacity::{scan as scan_capacity, CapacityReport, CapacitySkill};
pub use connection::{VaultConnection, VaultInfo};
pub use copy::copy_skill;
pub use create::create_skill;
pub use index::{
    AgentSupport, CategoryFilter, DeploymentView, FileNode, PresetRef, ReadLang, SearchQuery,
    SkillBody, SkillContent, SkillDetail, SkillEntry, SkillIndex, SkillSummary, SourceView,
    MAINTENANCE_SKILL_NAME,
};
pub use instructions::{
    compose_instructions, InstructionScope, InstructionSourceView, InstructionTargetResult,
    InstructionsService,
};
pub use manage::{ensure_unique_name, locate, update_meta, ManageItem, MetaUpdate};
pub use watch::VaultWatcher;
