//! 从上游选定候选项入库（T034）。
//!
//! 规则：
//! - 选定候选复制入域，**同名阻止**（复用 `ensure_unique_name`），不覆盖库内原件；
//! - 分类：显式给定 / 显式未分类 / 默认（依据**来源仓库技能总数**——多技能仓库按仓库名，
//!   单技能仓库不分类）；
//! - 继承上游自带 `.manager`（tags/translation/comment/agents/oses/overlay；缺声明=域内全支持）；
//! - 登记 `source.yaml`：repo/ref/subpath/version（实际采用的完整提交 OID）/registered_at；
//! - 来源与采用版本属同一 SkillManagerVault 共享资产：不把 SkillManagerVault 远端当成技能上游。

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::domain::{validate_category_name, validate_skill_name, Domain, SkillRef};
use crate::error::{CoreError, Result};
use crate::manager::{self, SourceRecord};
use crate::output::ItemError;
use crate::upstream::discover::{self, Candidate};

/// 分类选择。
#[derive(Clone, Debug)]
pub enum CategoryChoice {
    /// 显式一级分类。
    Given(String),
    /// 显式不分类（域根下）。
    Uncategorized,
    /// 默认：按来源仓库技能总数决定。
    Default,
}

impl CategoryChoice {
    /// 解析为一个具体分类；`total_skills` 为来源仓库技能总数。
    pub fn resolve(&self, repo: &str, total_skills: usize) -> Result<Option<String>> {
        let category = match self {
            CategoryChoice::Given(name) => Some(name.clone()),
            CategoryChoice::Uncategorized => None,
            CategoryChoice::Default => {
                if total_skills > 1 {
                    Some(discover::repo_name(repo))
                } else {
                    None
                }
            }
        };
        if let Some(name) = &category {
            validate_category_name(name)?;
        }
        Ok(category)
    }
}

/// 入库成功的一项。
#[derive(Clone, Debug, Serialize)]
pub struct ImportItem {
    pub subpath: String,
    pub skill: String,
    pub name: String,
    pub category: Option<String>,
    pub version: String,
    pub path: String,
}

/// 一次入库的结论。
#[derive(Clone, Debug, Default)]
pub struct ImportOutcome {
    pub items: Vec<ImportItem>,
    pub errors: Vec<ItemError>,
}

/// 从已检出的来源仓库把选定子路径复制入库。
pub fn import_skills(
    vault_root: &Path,
    domain: Domain,
    repo: &str,
    reference: Option<&str>,
    version: &str,
    source_root: &Path,
    subpaths: &[String],
    category: CategoryChoice,
) -> Result<ImportOutcome> {
    let candidates = discover::scan_repo(source_root)?;
    let total = candidates.len();
    let by_subpath: BTreeMap<&str, &Candidate> =
        candidates.iter().map(|c| (c.subpath.as_str(), c)).collect();

    let mut outcome = ImportOutcome::default();
    for raw_subpath in subpaths {
        let subpath = normalize_subpath(raw_subpath);
        let Some(candidate) = by_subpath.get(subpath.as_str()).copied() else {
            outcome.errors.push(
                ItemError::new("来源仓库中没有该技能子路径（不作仓库根回退）")
                    .skill(format!("{domain}/{}", subpath)),
            );
            continue;
        };
        match import_one(
            vault_root,
            domain,
            repo,
            reference,
            version,
            source_root,
            candidate,
            &category,
            total,
        ) {
            Ok(item) => outcome.items.push(item),
            Err(err) => outcome.errors.push(err),
        }
    }
    Ok(outcome)
}

#[allow(clippy::too_many_arguments)]
fn import_one(
    vault_root: &Path,
    domain: Domain,
    repo: &str,
    reference: Option<&str>,
    version: &str,
    source_root: &Path,
    candidate: &Candidate,
    category: &CategoryChoice,
    total: usize,
) -> std::result::Result<ImportItem, ItemError> {
    let fail = |err: CoreError| ItemError::from_error(&err).skill(format!("{domain}/{}", candidate.name));

    validate_skill_name(&candidate.name).map_err(fail)?;
    let category = category.resolve(repo, total).map_err(fail)?;
    let reference_struct = SkillRef {
        domain,
        name: candidate.name.clone(),
    };
    crate::vault::manage::ensure_unique_name(vault_root, domain, &candidate.name, None).map_err(fail)?;

    let target = reference_struct.dir_in(vault_root, category.as_deref());
    if target.exists() {
        return Err(fail(CoreError::DuplicateSkill {
            domain: domain.to_string(),
            name: candidate.name.clone(),
        }));
    }

    let source_dir = candidate.dir_in(source_root);
    copy_skill_dir(&source_dir, &target).map_err(fail)?;
    // 上游的 source.yaml 属于它自己的来源记录，库内重新登记本次采用的版本。
    let upstream_source = manager::source_path(&target);
    if upstream_source.exists() {
        std::fs::remove_file(&upstream_source)
            .map_err(|err| fail(CoreError::io(upstream_source.clone(), err)))?;
    }
    manager::write_source(
        &target,
        &SourceRecord {
            repo: repo.to_string(),
            reference: reference.map(str::to_string),
            subpath: candidate.subpath.clone(),
            version: version.to_string(),
            skipped_version: None,
            registered_at: today(),
        },
    )
    .map_err(fail)?;

    Ok(ImportItem {
        subpath: candidate.subpath.clone(),
        skill: reference_struct.to_string(),
        name: candidate.name.clone(),
        category,
        version: version.to_string(),
        path: target.display().to_string(),
    })
}

/// 归一化子路径：去掉首尾空白与末尾 `/`，保证与候选 subpath 一致。
pub fn normalize_subpath(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        ".".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 记录登记日期（`YYYY-MM-DD`）。
pub fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// 递归复制技能目录：跳过 `.git` 与 `.DS_Store`，不跟随符号链接；保留 `.manager`。
pub(crate) fn copy_skill_dir(source: &Path, target: &Path) -> Result<()> {
    if !source.is_dir() {
        return Err(CoreError::NotADirectory(source.to_path_buf()));
    }
    std::fs::create_dir_all(target).map_err(|err| CoreError::io(target.to_path_buf(), err))?;
    let entries =
        std::fs::read_dir(source).map_err(|err| CoreError::io(source.to_path_buf(), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| CoreError::io(source.to_path_buf(), err))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".git" || name == ".DS_Store" {
            continue;
        }
        let from = entry.path();
        let to = target.join(&name);
        let file_type = entry
            .file_type()
            .map_err(|err| CoreError::io(from.clone(), err))?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            copy_skill_dir(&from, &to)?;
        } else if file_type.is_file() {
            std::fs::copy(&from, &to).map_err(|err| CoreError::io(to.clone(), err))?;
        }
    }
    Ok(())
}
