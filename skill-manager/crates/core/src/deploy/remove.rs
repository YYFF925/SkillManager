//! 撤销部署与删库清理（T027）：撤销保留库内技能；删库与上游删除共用同一清理入口。
//!
//! 库内条目删除而副本暂停时，保留记录与 `target_path`，报告“库已删除、副本清理暂停”；
//! 不新增待删任务队列，也不自动丢弃本地副本（FR-022）。

use std::path::PathBuf;

use crate::deploy::compose::remove_path;
use crate::domain::SkillRef;
use crate::error::{CoreError, Result};
use crate::output::ItemError;
use crate::state::deployments::DeploymentStatus;

use super::apply::{
    dedup_targets, DeployItemView, DeployReport, DeploymentService, SkillDeleteOutcome,
};

impl DeploymentService<'_> {
    /// 批量撤销：技能 × 目标，逐项独立处理；撤销保留库内技能。
    pub fn undo(
        &self,
        references: &[SkillRef],
        targets: &[crate::domain::Target],
        discard_local_changes: bool,
    ) -> Result<DeployReport> {
        let targets = dedup_targets(targets);
        let mut report = DeployReport::default();
        for reference in references {
            for target in &targets {
                let target_str = target.as_str();
                let Some(record) =
                    self.state
                        .deployment(reference.domain, &reference.name, &target_str)?
                else {
                    report.push_failure(
                        ItemError::new("未部署到该目标（没有本机记录）")
                            .skill(reference.to_string())
                            .target(target_str),
                    );
                    continue;
                };

                let path = PathBuf::from(&record.target_path);
                let result = (|| -> Result<()> {
                    if self.is_locally_changed(&record, &path)? && !discard_local_changes {
                        return Err(CoreError::other("受管副本已被外部修改，撤销暂停；确认丢弃请加 --discard-local-changes"));
                    }
                    remove_path(&path)?;
                    self.state.delete_deployment(reference.domain, &reference.name, &target_str).map(|_| ())
                })();
                match result {
                    Ok(()) => report.push_item(DeployItemView {
                        skill: reference.to_string(), target: target_str,
                        path: record.target_path.clone(), action: "removed".to_string(),
                        status: DeploymentStatus::Ok.as_str().to_string(), applied_overlays: Vec::new(), message: None,
                    }),
                    Err(err) => report.push_failure(ItemError::from_error(&err).skill(reference.to_string()).target(target_str).path(record.target_path.clone())),
                }
            }
        }
        Ok(report)
    }

    /// 删库清理（删库与上游删除共用）：逐项尝试移除副本，外改项保留记录并如实报告暂停。
    pub fn cleanup_deployments(&self, reference: &SkillRef) -> Result<DeployReport> {
        let mut report = DeployReport::default();
        for record in self.state.deployments_for(reference.domain, &reference.name)? {
            let path = PathBuf::from(&record.target_path);
            let result = (|| -> Result<()> {
                if self.is_locally_changed(&record, &path)? {
                    self.state.upsert_deployment(&super::apply::paused_record(&record))?;
                    return Err(CoreError::other("库已删除、该副本清理暂停（副本已被外部修改）；请检查后继续清理"));
                }
                remove_path(&path)?;
                self.state.delete_deployment(reference.domain, &record.name, &record.target).map(|_| ())
            })();
            match result {
                Ok(()) => report.push_item(DeployItemView {
                    skill: record.reference().to_string(), target: record.target.clone(), path: record.target_path.clone(),
                    action: "removed".to_string(), status: DeploymentStatus::Ok.as_str().to_string(), applied_overlays: Vec::new(), message: None,
                }),
                Err(err) => report.push_failure(ItemError::from_error(&err).skill(record.reference().to_string()).target(record.target.clone()).path(record.target_path.clone())),
            }
        }
        Ok(report)
    }

    /// `skill delete <ref>...`：删除库内条目并清理其受管部署。
    pub fn delete_skills(&self, references: &[SkillRef]) -> Result<SkillDeleteOutcome> {
        let index = self.index()?;
        let mut outcome = SkillDeleteOutcome::default();
        for reference in references {
            let Some(entry) = index.get(reference) else {
                outcome.failures.push(
                    ItemError::new("技能不存在").skill(reference.to_string()),
                );
                continue;
            };
            let dir = entry.dir.clone();
            if let Err(err) =
                std::fs::remove_dir_all(&dir).map_err(|err| CoreError::io(dir.clone(), err))
            {
                outcome.failures.push(
                    ItemError::new(format!("删除库内条目失败：{err}"))
                        .skill(reference.to_string())
                        .path(dir.display().to_string()),
                );
                continue;
            }
            outcome.deleted.push(reference.to_string());

            let cleanup = self.cleanup_deployments(reference)?;
            outcome.removed.extend(cleanup.items);
            outcome.paused.extend(cleanup.failures);
        }
        Ok(outcome)
    }
}
