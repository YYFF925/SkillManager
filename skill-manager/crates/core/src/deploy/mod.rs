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
    DeployItemView, DeployReport, DeploymentService, DeploymentStatusView, SkillDeleteOutcome,
};
pub use compose::{compose_skill, ComposedArtifact, ComposedFile};
pub use local_skills::{LocalDeleteOutcome, LocalSkillService, LocalSkillView};
pub use policy::{Admission, HERMES_AGENT_KEY, agent_dimension};
pub use targets::{
    AgentCatalog, AgentEntry, TargetResolution, TargetResolver, SHARED_TARGET, parse_target_arg,
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
