//! 部署合成、目标管理、准入、执行、同步与撤销。

pub mod agent_config;
pub mod apply;
pub mod compose;
pub mod local_skills;
pub mod policy;
pub mod remove;
pub mod sync;
pub mod targets;

pub use agent_config::{AgentConfigService, AgentView, DirKind};
pub use apply::{
    DeployItemView, DeployReport, DeploymentService, DeploymentStatusView,
    SkillArchiveDeleteOutcome, SkillArchiveOutcome, SkillDeleteOutcome, SkillRestoreOutcome,
};
pub use compose::{compose_skill, ComposedArtifact, ComposedFile};
pub use local_skills::{LocalDeleteOutcome, LocalSkillRead, LocalSkillService, LocalSkillView};
pub use policy::{agent_dimension, Admission, HERMES_AGENT_KEY};
pub use targets::{
    parse_target_arg, AgentCatalog, AgentEntry, TargetResolution, TargetResolver, SHARED_TARGET,
};

/// 当前操作系统标识，取值与 `.manager/meta.yaml` 的 `oses` 一致。
pub fn current_os() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    }
}
