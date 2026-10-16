//! 上游新增候选决定与发现基线（T039/T046）。
//!
//! 共享文件 `upstream-decisions.yaml`（随 SkillManagerVault 同步），按 `(repo, domain)` 记录：
//! - `rejected_subpaths`：用户明确“不入库并记住”的来源子路径（互不影响的跨域选择）；
//! - `seen_subpaths`：**发现基线**——该 `(repo, domain)` 首次被观察时已存在的候选子路径集合。
//!
//! 基线的意义（spec.md「后来新增与未入库分开」）：首次已存在但未被选择的 C 不是“后来新增”，
//! 也不被记作拒绝；只有基线之后新出现、当前仍存在的 B 才属于新增候选。基线**只在首次观察时
//! 建立、从不自动扩充**，因此更新其他技能不会让未决定、仍存在的 B 丢失提示。仍可手动预览选择 C。
//! 这不是候选历史或队列系统：只保存一份一次性集合。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::Domain;
use crate::error::{CoreError, Result};
use crate::manager::SourceRecord;

/// 决定文件名（位于 SkillManagerVault 根）。
pub const DECISIONS_FILE: &str = "upstream-decisions.yaml";

/// 一条 `(repo, domain)` 的决定记录。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionEntry {
    pub repo: String,
    pub domain: Domain,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rejected_subpaths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seen_subpaths: Vec<String>,
}

impl DecisionEntry {
    /// 该记录是否与给定仓库地址（去重后）同源。
    pub fn matches(&self, repo: &str, domain: Domain) -> bool {
        self.domain == domain && SourceRecord::repo_key(&self.repo) == SourceRecord::repo_key(repo)
    }
}

/// 决定文件路径。
pub fn path(vault_root: &Path) -> PathBuf {
    vault_root.join(DECISIONS_FILE)
}

/// 读取全部决定记录；文件不存在或为空视为空列表。
pub fn load(vault_root: &Path) -> Result<Vec<DecisionEntry>> {
    let file = path(vault_root);
    let Some(bytes) = crate::manager::read_optional(&file)? else {
        return Ok(Vec::new());
    };
    if bytes.iter().all(|byte| byte.is_ascii_whitespace()) {
        return Ok(Vec::new());
    }
    let text = String::from_utf8(bytes)
        .map_err(|err| CoreError::other(format!("{} 不是有效 UTF-8：{err}", file.display())))?;
    let entries: Vec<DecisionEntry> =
        yaml_serde::from_str(&text).map_err(|err| CoreError::Yaml {
            path: file.clone(),
            message: err.to_string(),
        })?;
    Ok(entries)
}

/// 写入全部决定记录（空列表会写出空文件，保留共享资产可读性）。
pub fn save(vault_root: &Path, entries: &[DecisionEntry]) -> Result<()> {
    let file = path(vault_root);
    let text = yaml_serde::to_string(&entries.to_vec()).map_err(|err| CoreError::Yaml {
        path: file.clone(),
        message: err.to_string(),
    })?;
    let parent = file
        .parent()
        .ok_or_else(|| CoreError::other("决定文件没有父目录"))?;
    std::fs::create_dir_all(parent).map_err(|err| CoreError::io(parent.to_path_buf(), err))?;
    let temp = parent.join(format!(".{DECISIONS_FILE}.tmp.{}", std::process::id()));
    std::fs::write(&temp, text.as_bytes()).map_err(|err| CoreError::io(temp.clone(), err))?;
    std::fs::rename(&temp, &file).map_err(|err| CoreError::io(file, err))
}

fn entry_of<'a>(entries: &'a mut Vec<DecisionEntry>, repo: &str, domain: Domain) -> &'a mut DecisionEntry {
    if let Some(position) = entries.iter().position(|entry| entry.matches(repo, domain)) {
        return &mut entries[position];
    }
    entries.push(DecisionEntry {
        repo: repo.to_string(),
        domain,
        rejected_subpaths: Vec::new(),
        seen_subpaths: Vec::new(),
    });
    entries.last_mut().expect("刚插入")
}

/// 已明确拒绝的来源子路径。
pub fn rejected(vault_root: &Path, repo: &str, domain: Domain) -> Result<Vec<String>> {
    Ok(load(vault_root)?
        .into_iter()
        .find(|entry| entry.matches(repo, domain))
        .map(|entry| entry.rejected_subpaths)
        .unwrap_or_default())
}

/// 发现基线；尚未建立时为 `None`。
pub fn baseline(vault_root: &Path, repo: &str, domain: Domain) -> Result<Option<Vec<String>>> {
    Ok(load(vault_root)?
        .into_iter()
        .find(|entry| entry.matches(repo, domain))
        .map(|entry| entry.seen_subpaths)
        .filter(|seen| !seen.is_empty()))
}

/// 首次观察时建立基线；已有基线时不改动（返回 false）。
pub fn seed_baseline(
    vault_root: &Path,
    repo: &str,
    domain: Domain,
    observed: &[String],
) -> Result<bool> {
    let mut entries = load(vault_root)?;
    let entry = entry_of(&mut entries, repo, domain);
    if !entry.seen_subpaths.is_empty() {
        return Ok(false);
    }
    entry.seen_subpaths = dedup(observed);
    save(vault_root, &entries)?;
    Ok(true)
}

/// 记录明确“不入库并记住”的子路径（去重）。
pub fn reject(vault_root: &Path, repo: &str, domain: Domain, subpaths: &[String]) -> Result<()> {
    let mut entries = load(vault_root)?;
    let entry = entry_of(&mut entries, repo, domain);
    for subpath in subpaths {
        if !entry.rejected_subpaths.iter().any(|existing| existing == subpath) {
            entry.rejected_subpaths.push(subpath.clone());
        }
    }
    save(vault_root, &entries)
}

/// 列出决定记录（可按仓库地址/管理域过滤）。
pub fn entries(
    vault_root: &Path,
    repo: Option<&str>,
    domain: Option<Domain>,
) -> Result<Vec<DecisionEntry>> {
    let mut all = load(vault_root)?;
    all.retain(|entry| {
        repo.map(|repo| SourceRecord::repo_key(&entry.repo) == SourceRecord::repo_key(repo))
            .unwrap_or(true)
            && domain.map(|domain| entry.domain == domain).unwrap_or(true)
    });
    Ok(all)
}

fn dedup(values: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if !out.iter().any(|existing| existing == value) {
            out.push(value.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_is_seeded_once_and_never_auto_extended() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        let observed = vec!["a".to_string(), "c".to_string()];
        assert!(seed_baseline(root, "https://x/y.git", Domain::Code, &observed).unwrap());
        // 再次观察（例如检查）不扩充：B 仍能在后续被识别为“后来新增”。
        assert!(!seed_baseline(
            root,
            "https://x/y.git",
            Domain::Code,
            &["a".to_string(), "b".to_string(), "c".to_string()]
        )
        .unwrap());
        let baseline = baseline(root, "https://x/y.git/", Domain::Code).unwrap().unwrap();
        assert_eq!(baseline, vec!["a".to_string(), "c".to_string()]);
    }

    #[test]
    fn rejection_is_shared_and_domain_independent() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        reject(root, "https://x/y.git", Domain::Code, &["tools/c".to_string()]).unwrap();
        assert_eq!(
            rejected(root, "https://x/y.git", Domain::Code).unwrap(),
            vec!["tools/c".to_string()]
        );
        assert!(rejected(root, "https://x/y.git", Domain::Lobster).unwrap().is_empty());
    }
}
