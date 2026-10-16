//! 同步已有部署（T027）：只处理本机已有记录，不新增目标。
//!
//! 目标副本被外部修改时逐项暂停（`paused_local_change`）其余继续；经用户裁决后以
//! `--overwrite-local` 显式按库覆盖并恢复 `ok`（FR-019/FR-020）。

use std::path::PathBuf;

use crate::deploy::compose::{compose_skill, hash_dir, write_artifact};
use crate::deploy::policy::{self, Admission};
use crate::domain::{SkillRef, Target};
use crate::error::Result;
use crate::output::ItemError;
use crate::state::deployments::DeploymentRecord;
use crate::state::deployments::DeploymentStatus;
use crate::vault::index::SkillIndex;

use super::apply::{
    paused_record, record_ok, record_selected, DeployItemView, DeployReport, DeploymentService,
    PairOutcome,
};

impl DeploymentService<'_> {
    /// 同步已有部署；`reference`/`target` 用于收窄范围。
    pub fn sync(
        &self,
        reference: Option<&SkillRef>,
        target: Option<&Target>,
        overwrite_local: bool,
    ) -> Result<DeployReport> {
        let index = self.index()?;
        let mut report = DeployReport::default();
        let references: Vec<SkillRef> = reference.cloned().into_iter().collect();
        for record in self.state.deployments()? {
            if reference.is_some() && !record_selected(&record, &references) {
                continue;
            }
            if let Some(target) = target {
                if record.target != target.as_str() {
                    continue;
                }
            }
            match self.sync_record(&index, &record, overwrite_local)? {
                PairOutcome::Done(item) => report.push_item(item),
                PairOutcome::Failed(error) => report.push_failure(error),
            }
        }
        Ok(report)
    }

    fn sync_record(
        &self,
        index: &SkillIndex,
        record: &DeploymentRecord,
        overwrite_local: bool,
    ) -> Result<PairOutcome> {
        let reference = record.reference();
        let Some(entry) = index.get(&reference) else {
            return Ok(PairOutcome::Failed(
                ItemError::new("库内技能不存在，无法同步；可用 deploy undo 删除该副本并清理记录")
                    .skill(reference.to_string())
                    .target(record.target.clone())
                    .path(record.target_path.clone()),
            ));
        };
        let Ok(target) = record.target.parse::<Target>() else {
            return Ok(PairOutcome::Failed(
                ItemError::new(format!("部署目标标识无法解析：{}", record.target))
                    .skill(reference.to_string())
                    .target(record.target.clone()),
            ));
        };

        let composed = compose_skill(
            &entry.dir,
            policy::agent_dimension(&target),
            self.os(),
            entry.comment.as_deref(),
        )?;
        // 目标目录设置后来改变仍按记录定位原副本。
        let copy_path = PathBuf::from(&record.target_path);
        let actual = hash_dir(&copy_path)?;

        let warning = match policy::support_admission(entry, &target, self.os()) {
            Admission::Allowed => None,
            Admission::Unsupported(reason) => {
                Some(format!("支持范围已收窄，已有部署保留并警告：{reason}"))
            }
        };

        let locally_changed = match (&actual, &record.deployed_hash) {
            (Some(actual), Some(deployed)) => actual != deployed,
            (Some(_), None) => true,
            (None, _) => false,
        };
        if locally_changed && !overwrite_local {
            self.state.upsert_deployment(&paused_record(record))?;
            return Ok(PairOutcome::Failed(
                ItemError::new("受管副本已被外部修改，已暂停覆盖；确认按库覆盖请用 deploy sync --ref <技能> --target <目标> --overwrite-local")
                    .skill(reference.to_string())
                    .target(record.target.clone())
                    .path(copy_path.display().to_string()),
            ));
        }

        let already_current = actual.as_deref() == Some(composed.artifact_hash.as_str());
        let action = if already_current {
            "unchanged"
        } else if actual.is_some() {
            "updated"
        } else {
            "deployed"
        };
        if !already_current {
            write_artifact(&copy_path, &composed.files)?;
        }

        let mut message = warning;
        if let Some(note) = &composed.comment_note {
            message = Some(match message {
                Some(existing) => format!("{existing}\n{note}"),
                None => note.clone(),
            });
        }
        self.state.upsert_deployment(&record_ok(
            entry.domain,
            &entry.name,
            &target,
            &copy_path,
            &composed,
            super::apply::now_iso(),
        ))?;

        Ok(PairOutcome::Done(DeployItemView {
            skill: reference.to_string(),
            target: record.target.clone(),
            path: copy_path.display().to_string(),
            action: action.to_string(),
            status: DeploymentStatus::Ok.as_str().to_string(),
            applied_overlays: composed.applied_overlays,
            message,
        }))
    }
}
