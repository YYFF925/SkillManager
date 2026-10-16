//! 预设：库内组合定义（`presets/<domain>.yaml`）与管理、部署。
//!
//! 组合定义随 SkillVault 同步，部署状态由各机维护；定义编辑不触发部署。

pub mod deploy;
pub mod files;
pub mod manage;

pub use files::{presets_dir, presets_path, Preset, PresetFile, PRESETS_DIR};
pub use manage::{
    add_members, create, delete, get, list, member_names, remove_members, rename, reorder,
    reorder_members, set_icon, PresetMemberView, PresetView,
};
