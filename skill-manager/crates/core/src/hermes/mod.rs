//! Hermes 档案接入（US6，T056～T060）。
//!
//! 单 Hermes 根下的档案发现、default 部署候选池、外部启用协议与本机启用名单。
//! 协议事实与核实结论见 `specs/002-skill-management/research.md` §五。

pub mod adopt;
pub mod disabled;
pub mod enables;
pub mod profiles;

use std::path::{Path, PathBuf};

pub use adopt::AdoptItem;
pub use disabled::{config_path, read_disabled, DisabledState};
pub use enables::{HermesItem, HermesService, ProfileSkillView, ProfileSkillsView, ProfileView};
pub use profiles::{
    discover_profiles, is_valid_profile_id, resolve_profile, ProfileInfo, ProfileKind,
    DEFAULT_PROFILE,
};

/// 本机 Hermes 根目录：`HERMES_HOME` 环境变量优先，否则平台默认。
///
/// 与 Hermes 自身的 `get_hermes_home()` 一致（research.md §五）：不设可写设置键，
/// 避免“配置说 A、Hermes 实际用 B”的静默分叉。default 档案就是该根自身，
/// `hermes:default` 部署目录为 `<根>/skills`，具名档案为 `<根>/profiles/<名>`。
pub fn hermes_home(home: &Path) -> PathBuf {
    if let Some(value) = std::env::var_os("HERMES_HOME") {
        let trimmed = value.to_string_lossy().trim().to_string();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    platform_default_hermes_home(home)
}

#[cfg(not(windows))]
fn platform_default_hermes_home(home: &Path) -> PathBuf {
    home.join(".hermes")
}

#[cfg(windows)]
fn platform_default_hermes_home(_home: &Path) -> PathBuf {
    // Windows 平台默认 `%LOCALAPPDATA%/hermes`（`dirs::data_local_dir()` 即该目录）。
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("hermes"))
        .join("hermes")
}
