//! SkillManagerVault 上传（T052，FR-038）。
//!
//! 上传工作区**全部**改动（含非软件产生的修改），提交并推送自己的 SkillManagerVault；
//! **不向任何来源仓库提交**。仅两条路径可达：GUI 保存按已确认策略调用、Agent 获明确同意后
//! 显式 `sync push`。失败保留可读原因并支持用户主动重试；后台不提交、不重试推送。

use serde::Serialize;

use crate::error::{CoreError, Result};

use super::SyncService;

/// `sync push` 结果。
#[derive(Clone, Debug, Serialize)]
pub struct PushReport {
    /// 本次是否产生了新的提交。
    pub committed: bool,
    /// 本次是否执行了推送。
    pub pushed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    pub remote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// 推送前本地领先上游的提交数；无上游无从判断为 None。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ahead: Option<u64>,
    pub message: String,
}

impl SyncService<'_> {
    /// 提交并推送本机 SkillManagerVault。`message` 为提交信息；缺省使用固定说明。
    pub fn push(&self, message: Option<&str>) -> Result<PushReport> {
        let git = self.vault.git();
        let root = self.vault_root();

        let Some(remote_name) = self.vault.default_remote_name()? else {
            return Err(CoreError::NoRemote(root));
        };
        let remote = self.vault.remote_url()?;

        let Some(branch) = git.current_branch()? else {
            return Err(CoreError::other(
                "当前处于游离 HEAD，无法上传；请先切换到跟踪远端的分支",
            ));
        };

        // 已同意的上传失败可重试；这里先确认远端可用，失败原因如实返回。
        if let Err(detail) = git.remote_reachable(&remote_name) {
            return Err(CoreError::other(format!(
                "远端不可用（未连接），本次上传失败，可稍后主动重试：{detail}"
            )));
        }

        // 提交当前工作区全部改动（不按路径或来源拆分）。
        git.ok(&["add", "-A"])?;
        let staged = git.run(&["diff", "--cached", "--quiet"])?;
        let has_staged = !staged.success();
        let mut committed = false;
        let mut commit = None;
        if has_staged {
            let text = message
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("更新 SkillManagerVault（Skill Manager 上传）");
            git.ok(&["commit", "-m", text])?;
            committed = true;
            commit = Some(git.rev_parse("HEAD")?);
        }

        // 推送前确认是否还有未推送的提交。
        let mut upstream = git.upstream_ref()?;
        if upstream.is_none() {
            let candidate = format!("{remote_name}/{branch}");
            if git.has_rev(&candidate) {
                upstream = Some(candidate);
            }
        }
        let ahead = match &upstream {
            Some(upstream) => Some(git.ahead_behind("HEAD", upstream)?.0),
            None => None,
        };
        let needs_push = committed || ahead.map(|count| count > 0).unwrap_or(true);

        if !needs_push {
            return Ok(PushReport {
                committed,
                pushed: false,
                commit,
                remote,
                branch: Some(branch),
                ahead,
                message: "没有需要上传的改动，已是最新".to_string(),
            });
        }

        let out = match &upstream {
            Some(_) => git.run(&["push"])?,
            // 尚无上游跟踪：首次推送并建立跟踪关系。
            None => git.run(&["push", "-u", &remote_name, &branch])?,
        };
        if !out.success() {
            let detail = out.stderr.trim();
            return Err(CoreError::other(format!(
                "上传失败，可稍后主动重试（后台不会自动重试）：{}",
                if detail.is_empty() { "git push 失败" } else { detail }
            )));
        }

        Ok(PushReport {
            committed,
            pushed: true,
            commit,
            remote,
            branch: Some(branch),
            ahead,
            message: if committed {
                "已提交并推送工作区全部改动".to_string()
            } else {
                "已推送已有本地提交".to_string()
            },
        })
    }
}
