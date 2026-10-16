//! SkillManagerVault 同步状态（T051）：只读事实，不提交、不推送、不改库。
//!
//! 连接状态由 `git ls-remote` 判断；未提交改动、相对上游的未拉取提交数与进行中的冲突
//! 都按 Git 现状如实报告。远端不可用时本地管理与部署继续可用（FR-040）。

use serde::Serialize;

use crate::error::Result;
use crate::state::StateStore;
use crate::vault::VaultConnection;

use super::SyncService;

/// `sync status` 结果。
#[derive(Clone, Debug, Serialize)]
pub struct SyncStatus {
    pub vault: String,
    /// 默认远端地址；未配置为 None。
    pub remote: Option<String>,
    /// 远端可访问。
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_error: Option<String>,
    pub branch: Option<String>,
    /// 上游跟踪引用（如 `origin/main`）。
    pub upstream: Option<String>,
    /// 工作区未提交改动（`git status --porcelain` 各行）。
    pub uncommitted: Vec<String>,
    pub dirty: bool,
    /// 本地领先上游的提交数；无上游无从判断为 None。
    pub ahead: Option<u64>,
    /// 未拉取（落后上游）的提交数；无上游无从判断为 None。
    pub behind: Option<u64>,
    /// 存在未完成的合并冲突。
    pub conflict: bool,
    /// 未合并路径（供 Agent 检查）。
    pub conflicts: Vec<String>,
    /// 最近一次成功拉取时间。
    pub last_sync_at: Option<String>,
    /// GUI 保存后自动上传是否已确认。
    pub auto_push: bool,
}

impl SyncService<'_> {
    /// 读取同步状态（只读）。
    pub fn status(&self) -> Result<SyncStatus> {
        status(self.vault, self.state)
    }

    /// `sync conflicts`：输出供 Agent 检查的 Git 冲突细节（只读）。
    pub fn conflicts(&self) -> Result<ConflictView> {
        let git = self.vault.git();
        let conflicts = git.unmerged_paths()?;
        let porcelain: Vec<String> = git
            .status_porcelain()?
            .lines()
            .map(str::trim_end)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();
        let has_conflict = !conflicts.is_empty();
        Ok(ConflictView {
            has_conflict,
            conflicts,
            status: porcelain,
            message: if has_conflict {
                "存在未合并路径；请处理冲突文件后再次 sync pull 继续（不提供应用内合并界面）"
                    .to_string()
            } else {
                "当前没有进行中的 Git 冲突；若上次拉取冲突已处理，可再用 sync pull 继续"
                    .to_string()
            },
        })
    }
}

/// `sync conflicts` 结果。
#[derive(Clone, Debug, Serialize)]
pub struct ConflictView {
    pub has_conflict: bool,
    /// 未合并路径。
    pub conflicts: Vec<String>,
    /// `git status --porcelain` 各行（含冲突状态码）。
    pub status: Vec<String>,
    pub message: String,
}

/// 读取同步状态（只读）。
pub fn status(vault: &VaultConnection, state: &StateStore) -> Result<SyncStatus> {
    let git = vault.git();
    let branch = git.current_branch()?;
    let remote_name = vault.default_remote_name()?;
    let remote = vault.remote_url()?;

    let (connected, connection_error) = match &remote_name {
        None => (false, Some("未配置远端".to_string())),
        Some(name) => match git.remote_reachable(name) {
            Ok(()) => (true, None),
            Err(detail) => (false, Some(detail)),
        },
    };

    // 上游：优先本地上游配置，其次 `remote/branch`（克隆后常见形态）。
    let mut upstream = git.upstream_ref()?;
    if upstream.is_none() {
        if let (Some(name), Some(branch)) = (&remote_name, &branch) {
            let candidate = format!("{name}/{branch}");
            if git.has_rev(&candidate) {
                upstream = Some(candidate);
            }
        }
    }

    let (ahead, behind) = match &upstream {
        Some(upstream) => {
            let (ahead, behind) = git.ahead_behind("HEAD", upstream)?;
            (Some(ahead), Some(behind))
        }
        None => (None, None),
    };

    let porcelain = git.status_porcelain()?;
    let uncommitted: Vec<String> = porcelain
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    let conflicts = git.unmerged_paths()?;

    Ok(SyncStatus {
        vault: vault.root().display().to_string(),
        remote,
        connected,
        connection_error,
        branch,
        upstream,
        dirty: !uncommitted.is_empty(),
        uncommitted,
        ahead,
        behind,
        conflict: !conflicts.is_empty(),
        conflicts,
        last_sync_at: state.last_sync_at()?,
        auto_push: state.git_auto_push()?,
    })
}
