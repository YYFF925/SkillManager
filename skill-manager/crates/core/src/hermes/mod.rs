//! Hermes 档案接入（US6，T056～T060）。
//!
//! 单 Hermes 根下的档案发现、default 部署候选池、外部启用协议与本机启用名单。
//! 协议事实与核实结论见 `specs/002-skill-management/research.md` §五。

pub mod adopt;
pub mod disabled;
pub mod enables;
pub mod profiles;

pub use adopt::AdoptItem;
pub use disabled::{config_path, read_disabled, DisabledState};
pub use enables::{HermesItem, HermesService, ProfileSkillView, ProfileSkillsView, ProfileView};
pub use profiles::{
    discover_profiles, is_valid_profile_id, resolve_profile, ProfileInfo, ProfileKind,
    DEFAULT_PROFILE,
};
