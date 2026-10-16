//! 上游获取与更新（US2/US4，T032～T046）。
//!
//! 设计结论见 `specs/002-skill-management/research.md` D19～D21 与 `data-model.md` §1.3/1.6。
//! 本模块只承载“对技能上游仓库 B”的获取、发现、检查与显式采用；SkillVault 跨机同步是另一条链路。

pub mod apply;
pub mod changes;
pub mod check;
pub mod decisions;
pub mod discover;
pub mod fetch;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use serde::Serialize;

use crate::context::AppContext;
use crate::deploy::targets::TargetResolver;
use crate::domain::{Domain, SkillRef};
use crate::error::{CoreError, Result};
use crate::manager::SourceRecord;
use crate::state::StateStore;
use crate::sync::GitProgress;
use crate::vault::acquire::{self, CategoryChoice, ImportOutcome};

pub use apply::{UpdateItem, UpdateRepoOutcome};
pub use check::{CandidateView, ChangeView, CheckReport, CheckScope, SkillCheckView};
pub use decisions::DecisionEntry;
pub use discover::Candidate;
pub use fetch::FetchedRepo;

/// 上游服务：绑定库根、本机状态、目标解析器与缓存所在的状态目录。
pub struct UpstreamService<'a> {
    pub vault_root: PathBuf,
    pub state: &'a StateStore,
    pub resolver: &'a TargetResolver<'static>,
}

impl<'a> UpstreamService<'a> {
    pub fn new(
        vault_root: PathBuf,
        state: &'a StateStore,
        resolver: &'a TargetResolver<'static>,
    ) -> Self {
        Self {
            vault_root,
            state,
            resolver,
        }
    }

    /// 由本机上下文构造。
    pub fn from_context(
        ctx: &'a AppContext,
        resolver: &'a TargetResolver<'static>,
    ) -> Result<Self> {
        let vault_root = ctx.vault()?.root().to_path_buf();
        Ok(Self::new(vault_root, ctx.state(), resolver))
    }

    /// 缓存与检查事实所在的隔离状态目录（随 `--state-dir`）。
    pub fn state_dir(&self) -> PathBuf {
        self.state.dir().to_path_buf()
    }

    // -----------------------------------------------------------------------
    // preview / import
    // -----------------------------------------------------------------------

    /// `skill preview`：获取并列出候选，不入库；进度与取消由调用方传入。
    pub fn preview(
        &self,
        repo: &str,
        reference: Option<&str>,
        on_progress: &mut dyn FnMut(&GitProgress),
        cancel: &AtomicBool,
    ) -> Result<PreviewReport> {
        let fetched = fetch::fetch_repo(&self.state_dir(), repo, reference, on_progress, cancel)?;
        let candidates = discover::scan_repo(&fetched.path)?;
        let default_category = CategoryChoice::Default.resolve(repo, candidates.len())?;
        Ok(PreviewReport {
            repo: repo.to_string(),
            reference: reference.map(str::to_string),
            remote_version: fetched.oid,
            skill_count: candidates.len(),
            default_category,
            candidates,
        })
    }

    /// `skill import`：把选定候选入库，并建立/确认发现基线。
    pub fn import(
        &self,
        repo: &str,
        reference: Option<&str>,
        domain: Domain,
        subpaths: &[String],
        category: CategoryChoice,
        on_progress: &mut dyn FnMut(&GitProgress),
        cancel: &AtomicBool,
    ) -> Result<ImportOutcome> {
        let fetched = fetch::fetch_repo(&self.state_dir(), repo, reference, on_progress, cancel)?;
        let candidates = discover::scan_repo(&fetched.path)?;
        let outcome = acquire::import_skills(
            &self.vault_root,
            domain,
            repo,
            reference,
            &fetched.oid,
            &fetched.path,
            subpaths,
            category,
        )?;
        // 首次观察建立基线：首次已存在的候选（含未被选中的 C）不随后被当作“后来新增”。
        let observed: Vec<String> = candidates.iter().map(|c| c.subpath.clone()).collect();
        decisions::seed_baseline(&self.vault_root, repo, domain, &observed)?;
        Ok(outcome)
    }
}

/// 预览结果。
#[derive(Clone, Debug, Serialize)]
pub struct PreviewReport {
    pub repo: String,
    #[serde(rename = "ref")]
    pub reference: Option<String>,
    pub remote_version: String,
    /// 来源仓库技能总数（默认分类依据）。
    pub skill_count: usize,
    /// 默认分类（多技能仓库为仓库名；单技能仓库为未分类）。
    pub default_category: Option<String>,
    pub candidates: Vec<Candidate>,
}

/// 仓库地址去重键。
pub fn repo_key(repo: &str) -> String {
    SourceRecord::repo_key(repo)
}

/// 一个技能的上游定位（repo/ref/subpath）。
#[derive(Clone, Debug)]
pub struct SkillUpstream {
    pub reference: SkillRef,
    pub dir: PathBuf,
    pub source: SourceRecord,
}

/// 收集库内带来源的技能（可按仓库/单个技能收窄）。
pub fn upstream_targets(
    vault_root: &Path,
    scope: &check::CheckScope,
) -> Result<Vec<SkillUpstream>> {
    let index = crate::vault::SkillIndex::build(vault_root)?;
    let mut out = Vec::new();
    for entry in index.all() {
        let Some(source) = &entry.source else {
            continue;
        };
        let reference = entry.reference();
        let included = match scope {
            check::CheckScope::Single(target) => {
                target.domain == entry.domain && crate::domain::names_equal(&target.name, &entry.name)
            }
            check::CheckScope::Repo(repo) => repo_key(repo) == repo_key(&source.repo),
            check::CheckScope::All => true,
        };
        if included {
            out.push(SkillUpstream {
                reference,
                dir: entry.dir.clone(),
                source: source.clone(),
            });
        }
    }
    Ok(out)
}

/// 按仓库去重、按 `ref` 分组的获取缓存。
pub struct FetchCache {
    entries: HashMap<String, FetchedRepo>,
}

impl FetchCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn key(repo: &str, reference: Option<&str>) -> String {
        format!("{}\u{0}{}", repo_key(repo), reference.unwrap_or("\u{0}HEAD"))
    }

    /// 获取（复用缓存）并返回检出目录与远端 OID。
    pub fn fetch(
        &mut self,
        state_dir: &Path,
        repo: &str,
        reference: Option<&str>,
        on_progress: &mut dyn FnMut(&GitProgress),
        cancel: &AtomicBool,
    ) -> Result<&FetchedRepo> {
        let key = Self::key(repo, reference);
        if !self.entries.contains_key(&key) {
            let fetched =
                fetch::fetch_repo(state_dir, repo, reference, on_progress, cancel)?;
            self.entries.insert(key.clone(), fetched);
        }
        Ok(self.entries.get(&key).expect("刚插入"))
    }

    /// 取到指定提交（若缓存 HEAD 不是该提交则单独获取）。
    pub fn fetch_at(
        &mut self,
        state_dir: &Path,
        repo: &str,
        reference: Option<&str>,
        oid: &str,
        on_progress: &mut dyn FnMut(&GitProgress),
        cancel: &AtomicBool,
    ) -> Result<FetchedRepo> {
        let fetched = self.fetch(state_dir, repo, reference, on_progress, cancel)?.clone();
        if fetched.oid == oid {
            return Ok(fetched);
        }
        fetch::fetch_oid(state_dir, repo, reference, oid, on_progress, cancel)
    }
}

impl Default for FetchCache {
    fn default() -> Self {
        Self::new()
    }
}

/// 上游技能目录（`subpath` 为 `.` 时为仓库根）。
pub fn upstream_skill_dir(repo_root: &Path, subpath: &str) -> PathBuf {
    if subpath == "." || subpath.is_empty() {
        repo_root.to_path_buf()
    } else {
        repo_root.join(subpath.split('/').collect::<PathBuf>())
    }
}

/// 计算相对上游子路径的变化（只读）。
pub fn upstream_changes(
    skill_dir: &Path,
    upstream_dir: &Path,
) -> Result<(changes::BaseDiff, Vec<String>)> {
    Ok((
        changes::base_diff(skill_dir, upstream_dir)?,
        changes::manager_changes(skill_dir, upstream_dir)?,
    ))
}

/// 供 `--to`/`--version` 缺省值：最近检查所得的具体远端提交。
pub fn latest_checked_version(
    state: &StateStore,
    repo: &str,
    domain: Domain,
    name: &str,
) -> Result<Option<String>> {
    state.latest_remote_version(repo, domain, name)
}

/// 上游来源缺失时的统一错误。
pub fn no_source(reference: &SkillRef) -> CoreError {
    CoreError::other(format!(
        "技能没有上游来源记录（自建技能）：{reference}；无法检查或更新上游"
    ))
}
