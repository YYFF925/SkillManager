//! 应用上游更新（T045，US4）：显式单技能更新与仓库批量更新。
//!
//! - 单项：**仅所选技能**更新，并同步本机已有部署；采用版本随成功采用推进；
//! - 批量：处理该仓库尚未采用、未跳过的已有条目，以及显式选定的新增入库条目；
//!   新增入库**不首次部署**；
//! - 仅删除技能内文件时移除相应文件、其他变化照常应用，保留技能与已有部署并刷新；
//! - 只有**整个技能被上游删除**时才走 US3 清理（保留暂停副本处置记录）；
//!   批量更新不自动删除整个技能，而是报告待显式处理；
//! - 原文变化保留译文并更新待译提示；管理资料按 D3 同项上游优先合并。

use std::sync::atomic::AtomicBool;

use serde::Serialize;

use crate::deploy::DeploymentService;
use crate::domain::{Domain, SkillRef};
use crate::error::{CoreError, Result};
use crate::manager;
use crate::output::ItemError;
use crate::sync::GitProgress;
use crate::vault::acquire::{self, CategoryChoice, ImportItem};
use crate::vault::SkillIndex;

use super::check::{ChangeView, CheckScope};
use super::{latest_checked_version, no_source, upstream_skill_dir, upstream_targets, FetchCache, UpstreamService};

/// 一次单项/批量中某一技能的应用结果。
#[derive(Clone, Debug, Serialize)]
pub struct UpdateItem {
    pub skill: String,
    pub repo: String,
    /// `updated` / `removed` / `unchanged`。
    pub action: String,
    pub from_version: String,
    pub to_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<ChangeView>,
}

/// 仓库批量更新结果。
#[derive(Clone, Debug, Default)]
pub struct UpdateRepoOutcome {
    pub repo: String,
    pub items: Vec<UpdateItem>,
    /// 上游删除整个技能、需显式单项处理（批量不自动删除）。
    pub pending_deletions: Vec<String>,
    /// 本次选定入库的新增候选。
    pub imported: Vec<ImportItem>,
    pub errors: Vec<ItemError>,
}

impl UpstreamService<'_> {
    /// 显式单技能更新；`to` 缺省采用最近检查所得的具体远端提交。
    pub fn update(
        &self,
        reference: &SkillRef,
        to: Option<String>,
        on_progress: &mut dyn FnMut(&GitProgress),
        cancel: &AtomicBool,
    ) -> Result<UpdateItem> {
        let index = SkillIndex::build(&self.vault_root)?;
        let entry = index
            .get(reference)
            .ok_or_else(|| CoreError::SkillNotFound {
                domain: reference.domain.to_string(),
                name: reference.name.clone(),
            })?;
        let source = entry.source.clone().ok_or_else(|| no_source(reference))?;
        let skill_dir = entry.dir.clone();

        let to = match to {
            Some(to) => to,
            None => latest_checked_version(self.state, &source.repo, reference.domain, &reference.name)?
                .ok_or_else(|| {
                    CoreError::other(format!(
                        "尚无检查结果：{reference}；请先执行 skill check（不在应用时静默选择另一个版本）"
                    ))
                })?,
        };

        let mut cache = FetchCache::new();
        let fetched = cache.fetch_at(
            &self.state_dir(),
            &source.repo,
            source.reference.as_deref(),
            &to,
            on_progress,
            cancel,
        )?;
        let upstream_dir = upstream_skill_dir(&fetched.path, &source.subpath);

        if !crate::vault::scan::is_skill_dir(&upstream_dir) {
            // 上游删除整个技能：经显式 update 确认后清库并清理受管部署。
            let service =
                DeploymentService::new(self.vault_root.clone(), self.resolver, self.state);
            let outcome = service.delete_skills(std::slice::from_ref(reference))?;
            if !outcome.failures.is_empty() {
                let reason = outcome
                    .failures
                    .iter()
                    .map(|item| item.message.clone())
                    .collect::<Vec<_>>()
                    .join("；");
                return Err(CoreError::other(format!(
                    "上游已删除该技能，但库内条目删除失败：{reason}"
                )));
            }
            return Ok(UpdateItem {
                skill: reference.to_string(),
                repo: source.repo.clone(),
                action: "removed".to_string(),
                from_version: source.version.clone(),
                to_version: to,
                changes: None,
            });
        }

        let report = manager::merge_from_upstream(&skill_dir, &upstream_dir)?;
        advance_source(&skill_dir, &to)?;
        let deployments = DeploymentService::new(self.vault_root.clone(), self.resolver, self.state)
            .sync(Some(reference), None, false)?;

        let manager_items = collect_manager_items(&report);
        let changes = Some(ChangeView {
            added: report.base.added,
            modified: report.base.modified,
            deleted: report.base.deleted,
            manager: manager_items,
        });
        if !deployments.failures.is_empty() {
            let reason = deployments
                .failures
                .iter()
                .map(|item| item.message.clone())
                .collect::<Vec<_>>()
                .join("；");
            return Err(CoreError::other(format!(
                "技能内容已更新到 {to}，但本机已有部署同步未完成：{reason}"
            )));
        }

        Ok(UpdateItem {
            skill: reference.to_string(),
            repo: source.repo.clone(),
            action: "updated".to_string(),
            from_version: source.version.clone(),
            to_version: to,
            changes,
        })
    }

    /// 仓库批量更新：处理未采用、未跳过的已有条目与显式选定的新增。
    pub fn update_repo(
        &self,
        repo: &str,
        include_new: &[String],
        domain: Option<Domain>,
        category: CategoryChoice,
        on_progress: &mut dyn FnMut(&GitProgress),
        cancel: &AtomicBool,
    ) -> Result<UpdateRepoOutcome> {
        let mut outcome = UpdateRepoOutcome {
            repo: repo.to_string(),
            ..UpdateRepoOutcome::default()
        };
        let targets = upstream_targets(&self.vault_root, &CheckScope::Repo(repo.to_string()))?;
        let mut cache = FetchCache::new();

        for target in targets {
            if let Some(domain) = domain {
                if target.reference.domain != domain {
                    continue;
                }
            }
            let reference = target.reference.clone();
            let source = target.source.clone();
            let remote = match latest_checked_version(
                self.state,
                &source.repo,
                reference.domain,
                &reference.name,
            )? {
                Some(remote) => remote,
                None => {
                    outcome.errors.push(
                        ItemError::new("尚无检查结果，请先执行 skill check")
                            .skill(reference.to_string()),
                    );
                    continue;
                }
            };
            if remote == source.version
                || source.skipped_version.as_deref() == Some(remote.as_str())
            {
                continue;
            }

            let fetched = match cache.fetch_at(
                &self.state_dir(),
                &source.repo,
                source.reference.as_deref(),
                &remote,
                on_progress,
                cancel,
            ) {
                Ok(fetched) => fetched.clone(),
                Err(err) => {
                    outcome
                        .errors
                        .push(ItemError::from_error(&err).skill(reference.to_string()));
                    continue;
                }
            };
            let upstream_dir = upstream_skill_dir(&fetched.path, &source.subpath);
            if !crate::vault::scan::is_skill_dir(&upstream_dir) {
                // 批量不自动删除整个技能：报告待显式单项 update 处理。
                outcome.pending_deletions.push(reference.to_string());
                continue;
            }

            match manager::merge_from_upstream(&target.dir, &upstream_dir) {
                Ok(report) => {
                    if let Err(err) = advance_source(&target.dir, &remote) {
                        outcome
                            .errors
                            .push(ItemError::from_error(&err).skill(reference.to_string()));
                        continue;
                    }
                    outcome.items.push(UpdateItem {
                        skill: reference.to_string(),
                        repo: source.repo.clone(),
                        action: "updated".to_string(),
                        from_version: source.version.clone(),
                        to_version: remote.clone(),
                        changes: Some({
                            let manager = collect_manager_items(&report);
                            ChangeView {
                                added: report.base.added,
                                modified: report.base.modified,
                                deleted: report.base.deleted,
                                manager,
                            }
                        }),
                    });
                }
                Err(err) => outcome
                    .errors
                    .push(ItemError::from_error(&err).skill(reference.to_string())),
            }
        }

        if !include_new.is_empty() {
            let Some(domain) = domain else {
                outcome.errors.push(ItemError::new(
                    "选定新增入库必须同时给 --domain（新增进入哪个管理域）",
                ));
                return Ok(outcome);
            };
            match cache.fetch(&self.state_dir(), repo, None, on_progress, cancel) {
                Ok(fetched) => {
                    let version = fetched.oid.clone();
                    let tree = fetched.path.clone();
                    match acquire::import_skills(
                        &self.vault_root,
                        domain,
                        repo,
                        None,
                        &version,
                        &tree,
                        include_new,
                        category,
                    ) {
                        Ok(imported) => {
                            outcome.imported = imported.items;
                            outcome.errors.extend(imported.errors);
                        }
                        Err(err) => outcome.errors.push(ItemError::from_error(&err)),
                    }
                }
                Err(err) => outcome.errors.push(ItemError::from_error(&err)),
            }
        }

        Ok(outcome)
    }
}

fn collect_manager_items(report: &manager::MergeReport) -> Vec<String> {
    let mut items: Vec<String> = report
        .meta_changed
        .iter()
        .map(|key| format!("meta:{key}"))
        .collect();
    if report.translation_updated {
        items.push("translation".to_string());
    }
    for overlay in &report.overlays_updated {
        items.push(format!("overlay:{overlay}"));
    }
    items
}

fn advance_source(skill_dir: &std::path::Path, to: &str) -> Result<()> {
    let Some(mut record) = manager::read_source(skill_dir)? else {
        return Ok(());
    };
    record.version = to.to_string();
    record.skipped_version = None;
    manager::write_source(skill_dir, &record)
}
