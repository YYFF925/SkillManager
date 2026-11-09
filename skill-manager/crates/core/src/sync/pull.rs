//! SkillManagerVault 拉取（T051，FR-037）。
//!
//! 无冲突时静默应用库文件，随后刷新**已有**部署（不新增目标、不首次部署），并在指令源
//! 变化时刷新本机**已有启用**指令目标；冲突暂停本次同步并保留 Git 冲突现场，交 Agent
//! 检查、用户裁决后再次 `sync pull` 继续；远端不可用时显示未连接，本地操作继续。
//!
//! 未提交改动与远端**未重叠**时，直接合并（Git 允许脏工作区改无关文件），本机改动保留。
//! 与远端改了同一技能/正文路径时**不开始合并**，返回 `blocked_uncommitted`，工作区不动，
//! 交 Agent 询问用户先上传或丢弃——不把未提交改动 stash 成冲突标记。仅共享账本
//! `upstream-decisions.yaml` 按 `(repo, domain)` 对拒绝列表与发现基线取并集。
//! 已提交之间的 Git 冲突仍按 FR-039 暂停。
//!
//! 不提供克隆修复、断开远端或历史恢复入口；后台/手动拉取都不提交、不推送。

use serde::Serialize;

use crate::deploy::apply::{DeployItemView, DeploymentService};
use crate::error::Result;
use crate::output::ItemError;
use crate::upstream::decisions::{self, DecisionEntry, DECISIONS_FILE};
use crate::vault::instructions::{InstructionTargetResult, InstructionsService};

use super::git::Git;
use super::SyncService;

/// `sync pull` 结果。
#[derive(Clone, Debug, Serialize)]
pub struct PullReport {
    /// `up_to_date` / `pulled` / `conflict` / `blocked_uncommitted` / `disconnected` / `no_remote` / `no_upstream` / `error`。
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
    /// 未提交改动与远端更新重叠、因此未开始合并的路径。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocked_paths: Vec<String>,
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
    /// 本次是否暂存并恢复了本机未提交改动。
    pub restored_uncommitted: bool,
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
            blocked_paths: Vec::new(),
            deployments: Vec::new(),
            deployment_errors: Vec::new(),
            instructions_changed: false,
            instruction_targets: Vec::new(),
            connection_error: None,
            message: message.into(),
            errors: Vec::new(),
            restored_uncommitted: false,
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
        let mut restored_uncommitted = false;

        if git.merge_in_progress() {
            let conflicts = git.unmerged_paths()?;
            if !conflicts.is_empty() {
                return Ok(self.conflict_report(remote, branch, upstream, behind, conflicts, None));
            }
            // 上一次冲突已由用户/Agent 在 Git 层处理完毕，完成合并后继续刷新。
            git.ok(&["commit", "--no-edit", "-m", "合并远端 SkillManagerVault 变更"])?;
            let (restored, conflicts) = self.restore_pull_autostash(&git, &root, None, None)?;
            if !conflicts.is_empty() {
                return Ok(self.conflict_report(
                    remote,
                    branch,
                    upstream,
                    behind,
                    conflicts,
                    Some("合并已完成；恢复本机未提交改动时发生冲突，请处理后再继续"),
                ));
            }
            restored_uncommitted = restored;
        } else {
            let leftover = git.unmerged_paths()?;
            if !leftover.is_empty() {
                return Ok(self.conflict_report(
                    remote,
                    branch,
                    upstream,
                    behind,
                    leftover,
                    Some("工作区仍有未合并路径（可能来自上次恢复本机改动）；请处理后再同步"),
                ));
            }
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

            let uncommitted = git.uncommitted_paths()?;
            let incoming = git.incoming_paths("HEAD", &upstream_ref)?;
            let overlap: Vec<String> = uncommitted
                .iter()
                .filter(|path| incoming.iter().any(|incoming| incoming == *path))
                .cloned()
                .collect();
            let content_overlap: Vec<String> = overlap
                .iter()
                .filter(|path| path.as_str() != DECISIONS_FILE)
                .cloned()
                .collect();
            if !content_overlap.is_empty() {
                return Ok(self.blocked_uncommitted_report(
                    remote,
                    branch,
                    upstream,
                    behind,
                    content_overlap,
                ));
            }

            let local_decisions = if overlap.iter().any(|path| path == DECISIONS_FILE) {
                Some(decisions::load(&root)?)
            } else {
                None
            };
            if local_decisions.is_some() {
                clear_decisions_for_merge(&git, &root)?;
            }

            let out = git.run(&["merge", "--no-edit", &upstream_ref])?;
            if !out.success() {
                if let Some(local) = &local_decisions {
                    let _ = decisions::save(&root, local);
                }
                let conflicts = git.unmerged_paths()?;
                if !conflicts.is_empty() {
                    return Ok(self.conflict_report(
                        remote,
                        branch,
                        upstream,
                        behind,
                        conflicts,
                        None,
                    ));
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

            if let Some(local) = &local_decisions {
                let merged = decisions::load(&root)?;
                write_union_decisions(&root, &git, local, &merged)?;
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
        if restored_uncommitted {
            message.push_str("；已恢复本机未提交改动");
        }
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
            blocked_paths: Vec::new(),
            deployments: deploy.items,
            deployment_errors: deploy.failures,
            instructions_changed,
            instruction_targets,
            connection_error: None,
            message,
            errors,
            restored_uncommitted,
        })
    }

    /// 恢复 `stash_push_pull_guard` 留下的本机改动。
    ///
    /// `upstream-decisions.yaml` 在有快照时按并集写回，不把账本文件冲突当成技能内容冲突。
    fn restore_pull_autostash(
        &self,
        git: &Git,
        root: &std::path::Path,
        local_decisions: Option<&[DecisionEntry]>,
        merged_decisions: Option<&[DecisionEntry]>,
    ) -> Result<(bool, Vec<String>)> {
        let had_stash = git.pull_autostash_ref()?.is_some();
        let popped = git.stash_pop_pull_guard()?;
        if let (Some(local), Some(merged)) = (local_decisions, merged_decisions) {
            write_union_decisions(root, git, local, merged)?;
        }
        if popped {
            return Ok((had_stash, Vec::new()));
        }
        let mut conflicts = git.unmerged_paths()?;
        if let (Some(local), Some(merged)) = (local_decisions, merged_decisions) {
            if conflicts.iter().any(|path| path == DECISIONS_FILE) {
                write_union_decisions(root, git, local, merged)?;
                conflicts.retain(|path| path != DECISIONS_FILE);
            }
        }
        if conflicts.is_empty() {
            git.drop_pull_autostash()?;
            return Ok((had_stash, Vec::new()));
        }
        Ok((false, conflicts))
    }

    fn blocked_uncommitted_report(
        &self,
        remote: Option<String>,
        branch: Option<String>,
        upstream: Option<String>,
        behind: Option<u64>,
        blocked_paths: Vec<String>,
    ) -> PullReport {
        let listed = blocked_paths.join("、");
        PullReport {
            connected: true,
            remote,
            branch,
            upstream,
            behind,
            blocked_paths,
            ..PullReport::bare(
                "blocked_uncommitted",
                format!(
                    "本机未提交改动与远端更新了同一文件，本次未拉取（工作区未改）。重叠路径：{listed}。请询问用户先 sync push 上传，或丢弃这些文件后再 sync pull；不要自行 stash / checkout 覆盖"
                ),
            )
        }
    }

    fn conflict_report(
        &self,
        remote: Option<String>,
        branch: Option<String>,
        upstream: Option<String>,
        behind: Option<u64>,
        conflicts: Vec<String>,
        extra: Option<&str>,
    ) -> PullReport {
        let mut message =
            "拉取发生冲突，本次同步已暂停；请用 sync conflicts 查看未合并路径，处理后再 sync pull 继续"
                .to_string();
        if let Some(extra) = extra {
            message.push('；');
            message.push_str(extra);
        }
        PullReport {
            connected: true,
            remote,
            branch,
            upstream,
            behind,
            conflicts,
            ..PullReport::bare("conflict", message)
        }
    }
}

fn clear_decisions_for_merge(git: &Git, root: &std::path::Path) -> Result<()> {
    let file = decisions::path(root);
    let checkout = git.run(&["checkout", "--", DECISIONS_FILE])?;
    if checkout.success() {
        return Ok(());
    }
    if file.exists() {
        std::fs::remove_file(&file).map_err(|err| crate::error::CoreError::io(file, err))?;
    }
    Ok(())
}

fn write_union_decisions(
    root: &std::path::Path,
    git: &Git,
    local: &[DecisionEntry],
    remote: &[DecisionEntry],
) -> Result<()> {
    let merged = decisions::union_merge(local, remote);
    decisions::save(root, &merged)?;
    let conflicts = git.unmerged_paths()?;
    if conflicts.iter().any(|path| path == DECISIONS_FILE) {
        git.ok(&["add", "--", DECISIONS_FILE])?;
        let _ = git.run(&["reset", "-q", "HEAD", "--", DECISIONS_FILE]);
    }
    Ok(())
}
