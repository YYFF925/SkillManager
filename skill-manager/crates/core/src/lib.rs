//! Skill Manager 业务核心。
//!
//! 业务规则只存在于本 crate；CLI 与 Tauri command 都是薄入口层（宪章 V）。

pub mod context;
pub mod deploy;
pub mod diagnostics;
pub mod domain;
pub mod error;
pub mod hermes;
pub mod manager;
pub mod output;
pub mod preset;
pub mod scheduler;
pub mod state;
pub mod sync;
pub mod toolcenter;
pub mod upstream;
pub mod vault;

pub use context::{AppContext, GlobalOptions};
pub use diagnostics::{DiagnosticsExport, DiagnosticsReport};
pub use domain::{Domain, SkillRef, Target};
pub use error::{CoreError, Result};
pub use preset::{Preset, PresetFile, PresetMemberView, PresetView};
pub use scheduler::{ScheduleReport, TaskOutcome, Trigger};
pub use hermes::{
    AdoptItem, DisabledState, HermesItem, HermesService, ProfileSkillView, ProfileSkillsView,
    ProfileView,
};
pub use manager::{MetaFile, SkillMeta, SourceRecord, SupportDecl};
pub use output::{BatchOutcome, CliResult, ItemError, Outcome};
pub use state::{AgentConfig, StateStore};
pub use toolcenter::{
    InstallComparison, ToolCheckOutcome, ToolEntry, ToolQuery, ToolSkillCandidate,
    ToolSkillCandidates, ToolTaskAction, ToolVersionView, ToolWriteReport,
};
pub use vault::{
    CapacityReport, CapacitySkill, PresetRef, SkillEntry, SkillIndex, VaultConnection, VaultWatcher,
};
