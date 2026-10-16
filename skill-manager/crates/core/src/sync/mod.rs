//! SkillManagerVault 跨电脑同步（US5，T051～T053）。
//!
//! 库同步与技能上游更新是两条链路：这里只负责同一个 SkillManagerVault 的拉取、上传与状态表达。
//! 部署记录、Agent 配置与指令启用名单留本机；无冲突拉取后刷新已有部署与已有启用指令目标，
//! 新增技能不首次部署（FR-036～FR-040）。

pub mod git;
pub mod pull;
pub mod push;
pub mod status;

pub use git::{Git, GitOutput, GitProcess, GitProgress};
pub use pull::PullReport;
pub use push::PushReport;
pub use status::{ConflictView, SyncStatus};

use std::path::PathBuf;

use crate::deploy::targets::{AgentCatalog, TargetResolver};
use crate::state::StateStore;
use crate::vault::VaultConnection;

/// 同步服务：绑定已接入的库、本机状态、目标解析器与内置名单。
pub struct SyncService<'a> {
    pub vault: &'a VaultConnection,
    pub home: PathBuf,
    pub catalog: &'static AgentCatalog,
    pub state: &'a StateStore,
    pub resolver: &'a TargetResolver<'static>,
}

impl<'a> SyncService<'a> {
    pub fn new(
        vault: &'a VaultConnection,
        home: PathBuf,
        catalog: &'static AgentCatalog,
        state: &'a StateStore,
        resolver: &'a TargetResolver<'static>,
    ) -> Self {
        Self {
            vault,
            home,
            catalog,
            state,
            resolver,
        }
    }

    pub fn vault_root(&self) -> PathBuf {
        self.vault.root().to_path_buf()
    }
}
