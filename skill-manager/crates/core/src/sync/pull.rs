//! SkillManagerVault 拉取（T051，FR-037）。
//!
//! 无冲突时静默应用库文件，随后刷新**已有**部署（不新增目标、不首次部署），并在指令源
//! 变化时刷新本机**已有启用**指令目标；冲突暂停本次同步并保留 Git 冲突现场，交 Agent
//! 检查、用户裁决后再次 `sync pull` 继续；远端不可用时显示未连接，本地操作继续。
//!
//! 不提供克隆修复、断开远端或历史恢复入口；后台/手动拉取都不提交、不推送。

use serde::Serialize;

use crate::deploy::apply::{DeployItemView, DeploymentService};
use crate::error::Result;
use crate::output::ItemError;
use crate::vault::instructions::{InstructionTargetResult, InstructionsService};

use super::SyncService;

/// `sync pull` 结果。
#[derive(Clone, Debug, Serialize)]
pub struct PullReport {
    /// `up_to_date` / `pulled` / `conflict` / `disconnected` / `no_remote` / `no_upstream` / `error`。
    pub outcome: String,
    pub connected: bool,
    pub remote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behind: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
    /// 刷新已有部署的结果。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub deployments: Vec<DeployItemView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub deployment_errors: Vec<ItemError>,
    /// 本次拉取是否改变了指令源。
    pub instructions_changed: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instruction_targets: Vec<InstructionTargetResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_error: Option<String>,
    pub message: String,
    /// 库文件已应用，但部署或指令目标尚未完成的逐项原因。
    pub errors: Vec<ItemError>,
}

impl PullReport {
    fn bare(outcome: &str, message: impl Into<String>) -> Self {
        Self {
            outcome: outcome.to_string(),
            connected: false,
            remote: None,
            branch: None,
            upstream: None,
            behind: None,
            conflicts: Vec::new(),
            deployments: Vec::new(),
            deployment_errors: Vec::new(),
            instructions_changed: false,
            instruction_targets: Vec::new(),
            connection_error: None,
            message: message.into(),
            errors: Vec::new(),
        }
    }
}

impl SyncService<'_> {
    /// 拉取同一 SkillManagerVault；`sync pull` 的核心入口。
    pub fn pull(&self) -> Result<PullReport> {
        let git = self.vault.git();
        let root = self.vault_root();

        let Some(remote_name) = self.vault.default_remote_name()? else {
            return Ok(PullReport::bare(
                "no_remote",
                "尚未配置远端，无法拉取；本地库管理、部署与 deploy sync 继续可用",
            ));
        };
        let remote = self.vault.remote_url()?;

        if let Err(detail) = git.remote_reachable(&remote_name) {
            return Ok(PullReport {
                connected: false,
                connection_error: Some(detail),
                remote,
                ..PullReport::bare(
                    "disconnected",
                    "远端不可用（未连接）；本地库管理、部署与 deploy sync 继续可用",
                )
            });
        }

        let branch = git.current_branch()?;
        let before_signature = InstructionsService::new(
            root.clone(),
            self.home.clone(),
            self.catalog,
            self.state,
        )
        .signature()?;

        git.ok(&["fetch", &remote_name])?;

        // 上游：优先本地上游配置，其次 `remote/branch`。
        let mut upstream = git.upstream_ref()?;
        if upstream.is_none() {
            if let Some(branch) = &branch {
                let candidate = format!("{remote_name}/{branch}");
                if git.has_rev(&candidate) {
                    upstream = Some(candidate);
                }
            }
        }

        let mut behind = None;

        if git.merge_in_progress() {
            let conflicts = git.unmerged_paths()?;
            if !conflicts.is_empty() {
                return Ok(self.conflict_report(remote, branch, upstream, behind, conflicts));
            }
            // 上一次冲突已由用户/Agent 在 Git 层处理完毕，完成合并后继续刷新。
            git.ok(&["commit", "--no-edit", "-m", "合并远端 SkillManagerVault 变更"])?;
        } else {
            let Some(upstream_ref) = upstream.clone() else {
                return Ok(PullReport {
                    remote,
                    branch,
                    ..PullReport::bare(
                        "no_upstream",
                        "当前分支未配置可用的上游引用，无法判断是否落后；请先配置远端跟踪分支",
                    )
                });
            };
            let (_, count) = git.ahead_behind("HEAD", &upstream_ref)?;
            behind = Some(count);
            if count == 0 {
                return Ok(PullReport {
                    connected: true,
                    remote,
                    branch,
                    upstream: Some(upstream_ref),
                    behind: Some(0),
                    ..PullReport::bare("up_to_date", "远端无新提交，已是最新")
                });
            }
            let out = git.run(&["merge", "--no-edit", &upstream_ref])?;
            if !out.success() {
                let conflicts = git.unmerged_paths()?;
                if !conflicts.is_empty() {
                    return Ok(self.conflict_report(remote, branch, upstream, behind, conflicts));
                }
                let detail = out.stderr.trim();
                return Ok(PullReport {
                    connected: true,
                    remote,
                    branch,
                    upstream: Some(upstream_ref),
                    behind,
                    ..PullReport::bare(
                        "error",
                        format!(
                            "拉取合并失败（未进入冲突状态）：{}",
                            if detail.is_empty() { "git merge 失败" } else { detail }
                        ),
                    )
                });
            }
        }

        // 库文件已应用：刷新已有部署（不新增目标、不首次部署）。
        // 内存索引在每次调用时按需重建，此处重建一次以反映新库内容。
        let _ = crate::vault::SkillIndex::build(&root)?;
        let deploy = DeploymentService::new(root.clone(), self.resolver, self.state)
            .sync(None, None, false)?;

        // 指令源变化时刷新本机已有启用指令目标（不改启用名单/指令路径）。
        let instructions = InstructionsService::new(
            root.clone(),
            self.home.clone(),
            self.catalog,
            self.state,
        );
        let after_signature = instructions.signature()?;
        let instructions_changed = before_signature != after_signature;
        let instruction_targets = if instructions_changed {
            instructions.distribute()?
        } else {
            Vec::new()
        };

        self.state
            .set_last_sync_at(&crate::deploy::apply::now_iso())?;

        let mut errors = deploy.failures.clone();
        errors.extend(instruction_targets.iter().filter(|target| !target.ok).map(|target| {
            ItemError::new(target.error.clone().unwrap_or_else(|| "指令刷新失败".to_string()))
                .target(format!("agent:{}", target.agent))
                .path(target.path.clone())
        }));
        let mut message = format!(
            "已拉取并应用库变更；刷新已有部署 {} 项{}",
            deploy.items.len(),
            if instructions_changed {
                format!("；指令源已变化，刷新 {} 个已启用目标", instruction_targets.iter().filter(|target| target.ok).count())
            } else {
                String::new()
            }
        );
        for error in &errors {
            message.push_str(&format!("\n未完成：{} {} {}", error.target.as_deref().unwrap_or(""), error.path.as_deref().unwrap_or(""), error.message));
        }

        Ok(PullReport {
            outcome: "pulled".to_string(),
            connected: true,
            remote,
            branch,
            upstream,
            behind,
            conflicts: Vec::new(),
            deployments: deploy.items,
            deployment_errors: deploy.failures,
            instructions_changed,
            instruction_targets,
            connection_error: None,
            message,
            errors,
        })
    }

    fn conflict_report(
        &self,
        remote: Option<String>,
        branch: Option<String>,
        upstream: Option<String>,
        behind: Option<u64>,
        conflicts: Vec<String>,
    ) -> PullReport {
        PullReport {
            connected: true,
            remote,
            branch,
            upstream,
            behind,
            conflicts,
            ..PullReport::bare(
                "conflict",
                "拉取发生冲突，本次同步已暂停；请用 sync conflicts 查看未合并路径，处理后再 sync pull 继续",
            )
        }
    }
}
