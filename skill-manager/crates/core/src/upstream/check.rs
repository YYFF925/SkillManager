//! 上游检查（T042，US4）：单技能 / 仓库 / 全库检查。
//!
//! - 同一来源仓库跨两管理域**只获取一次**（按仓库地址去重、按所选 `ref` 分别解析）；
//! - 每个技能**分别对照其采用版本**，同一版本不重复应用；
//! - 本机记录 `remote_version, checked_at, result`；**只更新检查事实**，
//!   不更新技能内容、采用版本或部署（检查类命令的允许写边界）。
//! - 变化文件汇总与新增候选来自 T043；上游改名按删除+新增呈现，不做重命名识别。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;

use serde::Serialize;

use crate::domain::{Domain, SkillRef};
use crate::error::Result;
use crate::state::upstream_checks::UpstreamCheckRecord;
use crate::sync::GitProgress;
use crate::vault::SkillIndex;

use super::{
    decisions, discover, upstream_skill_dir, upstream_targets, FetchCache, UpstreamService,
};

/// 检查范围。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckScope {
    /// 单个技能 `<domain>/<name>`。
    Single(SkillRef),
    /// 某个来源仓库地址。
    Repo(String),
    /// 全库登记来源。
    All,
}

/// 仓库级获取结果。
#[derive(Clone, Debug, Serialize)]
pub struct RepoStamp {
    pub repo: String,
    #[serde(rename = "ref")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_version: Option<String>,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 一个技能的变化文件汇总（相对库内当前内容）。
#[derive(Clone, Debug, Default, Serialize)]
pub struct ChangeView {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub deleted: Vec<String>,
    /// 管理资料同项变化（`meta:<键>` / `translation` / `overlay:<agent>/<os>:<路径>`）。
    pub manager: Vec<String>,
}

/// 单个技能的检查结果。
#[derive(Clone, Debug, Serialize)]
pub struct SkillCheckView {
    pub skill: String,
    pub repo: String,
    pub subpath: String,
    pub adopted_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_version: Option<String>,
    /// `up_to_date` / `update_available` / `skipped` / `upstream_missing` / `error`。
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<ChangeView>,
}

/// 新增候选。
#[derive(Clone, Debug, Serialize)]
pub struct CandidateView {
    pub repo: String,
    pub domain: Domain,
    pub subpath: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 是否属于“后来新增”（基线之后新出现且当前仍存在）。
    pub is_new: bool,
}

/// 一次检查的完整结果。
#[derive(Clone, Debug, Default, Serialize)]
pub struct CheckReport {
    pub repos: Vec<RepoStamp>,
    pub skills: Vec<SkillCheckView>,
    pub candidates: Vec<CandidateView>,
}

impl UpstreamService<'_> {
    /// 执行一次检查；进度与取消由调用方传入（CLI 到 stderr，GUI 到事件）。
    pub fn check(
        &self,
        scope: &CheckScope,
        on_progress: &mut dyn FnMut(&GitProgress),
        cancel: &AtomicBool,
    ) -> Result<CheckReport> {
        let targets = upstream_targets(&self.vault_root, scope)?;
        let mut report = CheckReport::default();
        if targets.is_empty() {
            return Ok(report);
        }

        // 按 (repo_key, ref) 分组：跨域同仓库只获取一次。
        let mut groups: BTreeMap<(String, String), Vec<crate::upstream::SkillUpstream>> =
            BTreeMap::new();
        for target in targets {
            let key = (
                super::repo_key(&target.source.repo),
                target.source.reference.clone().unwrap_or_default(),
            );
            groups.entry(key).or_default().push(target);
        }

        let mut cache = FetchCache::new();
        for targets in groups.values() {
            let repo = targets[0].source.repo.clone();
            let reference = targets[0].source.reference.clone();
            match cache.fetch(&self.state_dir(), &repo, reference.as_deref(), on_progress, cancel) {
                Ok(fetched) => {
                    let oid = fetched.oid.clone();
                    let tree = fetched.path.clone();
                    report.repos.push(RepoStamp {
                        repo: repo.clone(),
                        reference: reference.clone(),
                        remote_version: Some(oid.clone()),
                        ok: true,
                        error: None,
                    });
                    let candidates = discover::scan_repo(&tree).unwrap_or_default();
                    for target in targets {
                        report.skills.push(self.check_skill(target, &oid, &tree)?);
                    }
                    self.collect_candidates(&repo, &candidates, targets, &mut report)?;
                }
                Err(err) => {
                    report.repos.push(RepoStamp {
                        repo: repo.clone(),
                        reference: reference.clone(),
                        remote_version: None,
                        ok: false,
                        error: Some(err.to_string()),
                    });
                    for target in targets {
                        let result = "error".to_string();
                        self.record(&repo, target.reference.domain, &target.reference.name, None, &result)?;
                        report.skills.push(SkillCheckView {
                            skill: target.reference.to_string(),
                            repo: repo.clone(),
                            subpath: target.source.subpath.clone(),
                            adopted_version: target.source.version.clone(),
                            remote_version: None,
                            result,
                            changes: None,
                        });
                    }
                }
            }
        }
        Ok(report)
    }

    fn check_skill(
        &self,
        target: &crate::upstream::SkillUpstream,
        remote_oid: &str,
        tree: &std::path::Path,
    ) -> Result<SkillCheckView> {
        let reference = &target.reference;
        let source = &target.source;
        let (result, changes) = if remote_oid == source.version {
            ("up_to_date".to_string(), None)
        } else if source.skipped_version.as_deref() == Some(remote_oid) {
            ("skipped".to_string(), None)
        } else {
            let upstream_dir = upstream_skill_dir(tree, &source.subpath);
            if !crate::vault::scan::is_skill_dir(&upstream_dir) {
                ("upstream_missing".to_string(), None)
            } else {
                let (base, manager) = super::upstream_changes(&target.dir, &upstream_dir)?;
                (
                    "update_available".to_string(),
                    Some(ChangeView {
                        added: base.added,
                        modified: base.modified,
                        deleted: base.deleted,
                        manager,
                    }),
                )
            }
        };
        self.record(
            &source.repo,
            reference.domain,
            &reference.name,
            Some(remote_oid.to_string()),
            &result,
        )?;
        Ok(SkillCheckView {
            skill: reference.to_string(),
            repo: source.repo.clone(),
            subpath: source.subpath.clone(),
            adopted_version: source.version.clone(),
            remote_version: Some(remote_oid.to_string()),
            result,
            changes,
        })
    }

    /// 汇总某仓库在相关管理域内的“当前仍存在、未入库、未拒绝”的候选。
    fn collect_candidates(
        &self,
        repo: &str,
        candidates: &[discover::Candidate],
        targets: &[crate::upstream::SkillUpstream],
        report: &mut CheckReport,
    ) -> Result<()> {
        let domains: BTreeSet<Domain> = targets.iter().map(|t| t.reference.domain).collect();
        let index = SkillIndex::build(&self.vault_root)?;
        for domain in domains {
            let imported: BTreeSet<String> = index
                .all()
                .iter()
                .filter(|entry| entry.domain == domain)
                .filter_map(|entry| entry.source.as_ref())
                .filter(|source| super::repo_key(&source.repo) == super::repo_key(repo))
                .map(|source| source.subpath.clone())
                .collect();
            let rejected: BTreeSet<String> =
                decisions::rejected(&self.vault_root, repo, domain)?.into_iter().collect();
            let baseline = decisions::baseline(&self.vault_root, repo, domain)?;
            for candidate in candidates {
                if imported.contains(&candidate.subpath) || rejected.contains(&candidate.subpath) {
                    continue;
                }
                let is_new = baseline
                    .as_ref()
                    .map(|seen| !seen.iter().any(|s| s == &candidate.subpath))
                    .unwrap_or(false);
                report.candidates.push(CandidateView {
                    repo: repo.to_string(),
                    domain,
                    subpath: candidate.subpath.clone(),
                    name: candidate.name.clone(),
                    description: candidate.description.clone(),
                    is_new,
                });
            }
        }
        Ok(())
    }

    fn record(
        &self,
        repo: &str,
        domain: Domain,
        name: &str,
        remote_version: Option<String>,
        result: &str,
    ) -> Result<()> {
        self.state.upsert_upstream_check(&UpstreamCheckRecord {
            repo: repo.to_string(),
            domain,
            name: name.to_string(),
            remote_version,
            checked_at: Some(chrono::Utc::now().to_rfc3339()),
            result: Some(result.to_string()),
        })
    }
}

