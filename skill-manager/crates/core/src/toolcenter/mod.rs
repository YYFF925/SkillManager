//! 工具中心（US9）：共享工具资料、本机安装事实与检查、维护任务与配套 Skill 候选。
//!
//! 共享资料是 `tools/<tool-slug>/tool.yaml` + `guide.md`（随 SkillVault 同步）；本机安装
//! 事实与检查结果只在 SQLite（`state::tool_installs` / `state::tool_checks`）。软件只做版本
//! 检查；实际安装、更新与资料编写交给外部 Agent（FR-046/047）。

pub mod catalog;
pub mod check;
pub mod files;
pub mod skills;
pub mod tasks;

pub use catalog::{
    ToolCheckView, ToolEntry, ToolInstallView, ToolQuery, ToolSkillView, ToolStatusFilter,
};
pub use check::{InstallComparison, ToolCheckOutcome, check_all, check_tool, compare_versions};
pub use files::{ToolFile, ToolMeta, ToolSkillLink, ToolWriteReport};
pub use skills::{ToolSkillCandidate, ToolSkillCandidates, ToolVersionView};
pub use tasks::{ToolTaskAction, maintenance_skill_line, tool_task};
