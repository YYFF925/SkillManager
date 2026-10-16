//! 部署执行：多技能至多目标的独立副本部署、接管与状态查询（T026）。
//!
//! 业务规则：未受管同名直接覆盖并登记（FR-018）；受管副本被外部修改时暂停该项（FR-020）；
//! 准入（域边界与支持声明）逐项应用；批量为逐项独立副本，不做整批回滚。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::deploy::compose::{compose_skill, hash_dir, write_artifact, ComposedArtifact};
use crate::deploy::policy::{self, Admission};
use crate::deploy::targets::{TargetResolution, TargetResolver};
use crate::domain::{names_equal, Domain, SkillRef, Target};
use crate::error::Result;
use crate::output::ItemError;
use crate::state::deployments::{DeploymentRecord, DeploymentStatus};
use crate::state::StateStore;
use crate::vault::index::SkillIndex;

use super::current_os;

/// 逐项结果（成功或有警告但已执行）。
#[derive(Clone, Debug, Serialize)]
pub struct DeployItemView {
    pub skill: String,
    pub target: String,
    pub path: String,
    /// `deployed` / `updated` / `unchanged` / `removed` / `would_*`。
    pub action: String,
    pub status: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub applied_overlays: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// 一次部署/同步/撤销的完整结果：成功项 + 未完成项。
#[derive(Clone, Debug, Default)]
pub struct DeployReport {
    pub items: Vec<DeployItemView>,
    pub failures: Vec<ItemError>,
}

impl DeployReport {
    pub fn push_item(&mut self, item: DeployItemView) {
        self.items.push(item);
    }

    pub fn push_failure(&mut self, error: ItemError) {
        self.failures.push(error);
    }
}

/// `deploy status` 的一项。
#[derive(Clone, Debug, Serialize)]
pub struct DeploymentStatusView {
    pub deployment_blocks: std::collections::BTreeMap<String, String>,
    pub skill: String,
    pub target: String,
    pub target_path: String,
    pub status: String,
    pub source_hash: Option<String>,
    pub deployed_hash: Option<String>,
    pub synced_at: Option<String>,
    /// 库内技能是否仍存在；false 表示条目已删除、记录与副本待处置。
    pub library_exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// `skill delete` / 上游删除的清理结果。
#[derive(Clone, Debug, Default)]
pub struct SkillDeleteOutcome {
    /// 已删除的库内条目。
    pub deleted: Vec<String>,
    /// 已成功移除的副本。
    pub removed: Vec<DeployItemView>,
    /// 库已删除但副本清理暂停（保留记录与路径）。
    pub paused: Vec<ItemError>,
    /// 库内条目删除失败。
    pub failures: Vec<ItemError>,
}

/// 一次技能/目标组合的结论。
pub(crate) enum PairOutcome {
    Done(DeployItemView),
    Failed(ItemError),
}

/// 部署服务：绑定库根、目标解析器与本机状态。
pub struct DeploymentService<'a> {
    pub(crate) vault_root: PathBuf,
    pub(crate) resolver: &'a TargetResolver<'static>,
    pub(crate) state: &'a StateStore,
}

impl<'a> DeploymentService<'a> {
    pub fn new(
        vault_root: PathBuf,
        resolver: &'a TargetResolver<'static>,
        state: &'a StateStore,
    ) -> Self {
        Self {
            vault_root,
            resolver,
            state,
        }
    }

    pub fn vault_root(&self) -> &Path {
        &self.vault_root
    }

    pub(crate) fn index(&self) -> Result<SkillIndex> {
        SkillIndex::build(&self.vault_root).inspect_err(|err| {
            let _ = crate::diagnostics::record(self.state, "error", "deploy.index", &err.to_string());
        })
    }

    pub(crate) fn os(&self) -> &'static str {
        current_os()
    }

    /// 批量部署：技能 × 目标，逐项独立处理。
    pub fn deploy_to(
        &self,
        references: &[SkillRef],
        targets: &[Target],
    ) -> Result<DeployReport> {
        let index = self.index()?;
        let targets = dedup_targets(targets);
        let mut report = DeployReport::default();
        for reference in references {
            for target in &targets {
                match self.deploy_pair(&index, reference, target) {
                    Ok(PairOutcome::Done(item)) => report.push_item(item),
                    Ok(PairOutcome::Failed(error)) => report.push_failure(error),
                    Err(err) => report.push_failure(ItemError::from_error(&err).skill(reference.to_string()).target(target.as_str())),
                }
            }
        }
        Ok(report)
    }

    /// 收编时接管原副本（供 US2/US6 调用）：按库合成覆盖并登记为受管部署。
    pub fn adopt_existing(&self, reference: &SkillRef, target: &Target) -> Result<DeployReport> {
        self.deploy_to(std::slice::from_ref(reference), std::slice::from_ref(target))
    }

    /// 本机部署记录（供 `deploy status` 与 GUI 展示）。
    pub fn status(
        &self,
        target: Option<&Target>,
        paused_only: bool,
    ) -> Result<Vec<DeploymentStatusView>> {
        let index = self.index()?;
        let mut out = Vec::new();
        for record in self.state.deployments()? {
            if let Some(target) = target {
                if record.target != target.as_str() {
                    continue;
                }
            }
            if paused_only && record.status != DeploymentStatus::PausedLocalChange {
                continue;
            }
            let entry = index.get(&record.reference());
            let warning = match (entry, record.target.parse::<Target>().ok()) {
                (Some(entry), Some(parsed)) => {
                    match policy::support_admission(entry, &parsed, self.os()) {
                        Admission::Allowed => None,
                        Admission::Unsupported(reason) => {
                            Some(format!("支持范围已收窄，已有部署保留并警告：{reason}"))
                        }
                    }
                }
                _ => None,
            };
            out.push(DeploymentStatusView {
                deployment_blocks: entry.map(|entry| entry.deployment_blocks()).unwrap_or_default(),
                skill: record.reference().to_string(),
                target: record.target.clone(),
                target_path: record.target_path.clone(),
                status: record.status.as_str().to_string(),
                source_hash: record.source_hash.clone(),
                deployed_hash: record.deployed_hash.clone(),
                synced_at: record.synced_at.clone(),
                library_exists: entry.is_some(),
                warning,
            });
        }
        Ok(out)
    }

    // -----------------------------------------------------------------------
    // 单项部署
    // -----------------------------------------------------------------------

    pub(crate) fn deploy_pair(
        &self,
        index: &SkillIndex,
        reference: &SkillRef,
        target: &Target,
    ) -> Result<PairOutcome> {
        let Some(entry) = index.get(reference) else {
            return Ok(PairOutcome::Failed(
                ItemError::new("技能不存在").skill(reference.to_string()),
            ));
        };
        if let Some(reason) = &entry.manager_error {
            return Ok(PairOutcome::Failed(
                ItemError::new(format!("管理资料读取失败，拒绝部署：{reason}"))
                    .skill(reference.to_string())
                    .target(target.as_str()),
            ));
        }

        let resolution = match self.resolver.resolve(target) {
            Ok(resolution) => resolution,
            Err(err) => {
                return Ok(PairOutcome::Failed(
                    ItemError::new(err.to_string())
                        .skill(reference.to_string())
                        .target(target.as_str()),
                ));
            }
        };
        let target_domain = resolution.domain.unwrap_or(entry.domain);
        if let Err(err) = policy::check_domain(entry.domain, target_domain, target) {
            return Ok(PairOutcome::Failed(
                ItemError::new(err.to_string())
                    .skill(reference.to_string())
                    .target(target.as_str()),
            ));
        }

        let admission = policy::support_admission(entry, target, self.os());
        let existing = self
            .state
            .deployment(entry.domain, &entry.name, &target.as_str())?;
        let copy_path = target_copy_path(&resolution, &entry.name);

        if let Admission::Unsupported(reason) = &admission {
            if existing.is_none() {
                return Ok(PairOutcome::Failed(
                    ItemError::new(format!("禁止新增部署：{reason}"))
                        .skill(reference.to_string())
                        .target(target.as_str())
                        .path(copy_path.display().to_string()),
                ));
            }
        }

        let composed = compose_skill(
            &entry.dir,
            policy::agent_dimension(target),
            self.os(),
            entry.comment.as_deref(),
        )?;

        // 受管副本被外部修改：暂停该项，不覆盖（FR-020）。
        if let Some(record) = &existing {
            if self.is_locally_changed(record, &copy_path)? {
                let paused = paused_record(record);
                self.state.upsert_deployment(&paused)?;
                return Ok(PairOutcome::Failed(
                    ItemError::new("受管副本已被外部修改，部署暂停；确认按库覆盖请用 deploy sync --ref <技能> --target <目标> --overwrite-local")
                        .skill(reference.to_string())
                        .target(target.as_str())
                        .path(copy_path.display().to_string()),
                ));
            }
        }

        let mut message = match &admission {
            Admission::Allowed => None,
            Admission::Unsupported(reason) => Some(format!(
                "支持范围已收窄，已有部署保留并警告：{reason}"
            )),
        };
        if let Some(note) = &composed.comment_note {
            message = Some(match message {
                Some(existing) => format!("{existing}\n{note}"),
                None => note.clone(),
            });
        }

        let action = if existing.is_some() {
            "updated"
        } else {
            "deployed"
        };

        write_artifact(&copy_path, &composed.files)?;
        let record = record_ok(
            entry.domain,
            &entry.name,
            target,
            &copy_path,
            &composed,
            now_iso(),
        );
        self.state.upsert_deployment(&record)?;

        Ok(PairOutcome::Done(DeployItemView {
            skill: reference.to_string(),
            target: target.as_str(),
            path: copy_path.display().to_string(),
            action: action.to_string(),
            status: DeploymentStatus::Ok.as_str().to_string(),
            applied_overlays: composed.applied_overlays,
            message,
        }))
    }

    /// 目标副本是否被外部修改（与最近成功部署的产物基线不一致）。
    pub(crate) fn is_locally_changed(
        &self,
        record: &DeploymentRecord,
        copy_path: &Path,
    ) -> Result<bool> {
        let actual = hash_dir(copy_path)?;
        match (actual, &record.deployed_hash) {
            (Some(actual), Some(deployed)) => Ok(&actual != deployed),
            (Some(_), None) => Ok(true),
            // 副本缺失不算外部修改：按记录重建。
            (None, _) => Ok(false),
        }
    }
}

// ---------------------------------------------------------------------------
// 共用
// ---------------------------------------------------------------------------

pub(crate) fn dedup_targets(targets: &[Target]) -> Vec<Target> {
    let mut out: Vec<Target> = Vec::new();
    for target in targets {
        if !out.iter().any(|existing| existing == target) {
            out.push(target.clone());
        }
    }
    out
}

pub(crate) fn target_copy_path(resolution: &TargetResolution, name: &str) -> PathBuf {
    PathBuf::from(&resolution.path).join(name)
}

pub(crate) fn record_ok(
    domain: Domain,
    name: &str,
    target: &Target,
    path: &Path,
    composed: &ComposedArtifact,
    synced_at: String,
) -> DeploymentRecord {
    DeploymentRecord {
        domain,
        name: name.to_string(),
        target: target.as_str(),
        target_path: path.display().to_string(),
        status: DeploymentStatus::Ok,
        source_hash: Some(composed.source_hash.clone()),
        deployed_hash: Some(composed.artifact_hash.clone()),
        synced_at: Some(synced_at),
    }
}

pub(crate) fn paused_record(record: &DeploymentRecord) -> DeploymentRecord {
    let mut paused = record.clone();
    paused.status = DeploymentStatus::PausedLocalChange;
    paused.synced_at = Some(now_iso());
    paused
}

pub(crate) fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// 技能引用匹配（大小写不敏感，与库内同域唯一规则一致）。
pub(crate) fn refs_match(a: &SkillRef, b: &SkillRef) -> bool {
    a.domain == b.domain && names_equal(&a.name, &b.name)
}

/// 记录是否属于给定技能列表。
pub(crate) fn record_selected(record: &DeploymentRecord, refs: &[SkillRef]) -> bool {
    refs.iter()
        .any(|reference| refs_match(reference, &record.reference()))
}
