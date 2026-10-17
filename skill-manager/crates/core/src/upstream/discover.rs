//! 上游仓库技能候选发现与预览（T033）。
//!
//! 识别谓词与库内扫描一致（`SKILL.md` 优先、其次 `skill.md`，命中即不向下递归），
//! 见 `docs/technical-reference/git-and-discovery.md`「仓库内容识别」与 D20：
//! - 技能名按**目录身份**保留（不采用 frontmatter 的 `name` 替代）；
//! - 候选携带仓库内相对子路径，跨平台统一用 `/`；
//! - 排除 `.git`、`node_modules` 等无关范围，限制递归深度，避免链接环扫描不结束；
//! - 相对仓库根 `.agents/skills/` 及其子路径不是可入库候选，也不再向下扫描（FR-058）；
//! - 单个候选也列出；默认分类依据**来源仓库技能总数**，由调用者据此决定。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{CoreError, Result};
use crate::vault::scan;

/// 递归扫描时跳过的目录名。
pub const SKIP_DIRS: [&str; 3] = [".git", "node_modules", "__pycache__"];
/// 来源发现主动排除的仓库内路径（FR-058）。相对仓库根，`/` 分隔。
pub const EXCLUDED_PREFIX: &str = ".agents/skills";
/// 递归深度上限（防止意外深层/链接环导致的长时间扫描）。
pub const MAX_DEPTH: usize = 8;

/// 一个仓库内技能候选。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Candidate {
    /// 仓库内相对子路径，`/` 分隔；仓库根自身即技能时为 `.`。
    pub subpath: String,
    /// 技能名 = 目录名（目录身份）。
    pub name: String,
    /// 入口文件 frontmatter 的 description（仅展示）。
    pub description: Option<String>,
    pub comment: Option<String>,
    pub tags: Vec<String>,
}

impl Candidate {
    /// 用于复制时定位的来源目录。
    pub fn dir_in(&self, repo_root: &Path) -> PathBuf {
        if self.subpath == "." {
            repo_root.to_path_buf()
        } else {
            repo_root.join(self.subpath.split('/').collect::<PathBuf>())
        }
    }
}

/// 扫描一个已检出的来源仓库，返回全部技能候选。
pub fn scan_repo(repo_root: &Path) -> Result<Vec<Candidate>> {
    if !repo_root.is_dir() {
        return Err(CoreError::NotADirectory(repo_root.to_path_buf()));
    }
    let mut out = Vec::new();
    if scan::is_skill_dir(repo_root) {
        out.push(make(repo_root, ".")?);
        return Ok(out);
    }
    collect(repo_root, repo_root, 0, &mut out)?;
    out.sort_by(|a, b| a.subpath.cmp(&b.subpath));
    Ok(out)
}

fn collect(root: &Path, dir: &Path, depth: usize, out: &mut Vec<Candidate>) -> Result<()> {
    if depth >= MAX_DEPTH {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir).map_err(|err| CoreError::io(dir.to_path_buf(), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| CoreError::io(dir.to_path_buf(), err))?;
        let file_type = entry
            .file_type()
            .map_err(|err| CoreError::io(entry.path(), err))?;
        // 符号链接不跟随，避免链接环与外部目录。
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if SKIP_DIRS.contains(&name.as_str()) {
            continue;
        }
        let path = entry.path();
        if excluded_from_discovery(root, &path) {
            continue;
        }
        if scan::is_skill_dir(&path) {
            let subpath = path
                .strip_prefix(root)
                .map(|rel| rel.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| name.clone());
            out.push(make(&path, &subpath)?);
            continue;
        }
        collect(root, &path, depth + 1, out)?;
    }
    Ok(())
}

fn excluded_from_discovery(root: &Path, path: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else {
        return false;
    };
    let rel = rel.to_string_lossy().replace('\\', "/");
    rel == EXCLUDED_PREFIX || rel.starts_with(&format!("{EXCLUDED_PREFIX}/"))
}

fn make(dir: &Path, subpath: &str) -> Result<Candidate> {
    let name = if subpath == "." {
        dir.file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .ok_or_else(|| CoreError::other("仓库根技能无法确定名称"))?
    } else {
        subpath.rsplit('/').next().unwrap_or(subpath).to_string()
    };
    let description = scan::read_frontmatter(dir).description;
    let meta = crate::manager::MetaFile::load(dir)?.meta()?;
    Ok(Candidate {
        subpath: subpath.to_string(),
        name,
        description,
        comment: meta.comment,
        tags: meta.tags,
    })
}

/// 从仓库地址推导仓库名：取最后一个路径分量并去掉 `.git` 后缀。
///
/// 仅用于多技能仓库的默认分类；不推断来源身份。
pub fn repo_name(repo: &str) -> String {
    let trimmed = repo.trim().trim_end_matches('/');
    let last = trimmed
        .rsplit(['/', ':'])
        .find(|segment| !segment.is_empty())
        .unwrap_or(trimmed);
    last.strip_suffix(".git").unwrap_or(last).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn finds_nested_skills_and_stops_descending() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        write(&root.join("tools/alpha/SKILL.md"), "a");
        write(&root.join("tools/alpha/references/inner/SKILL.md"), "不应作为独立技能");
        write(&root.join("beta/skill.md"), "b");
        write(&root.join("README.md"), "x");
        write(&root.join(".git/config"), "x");

        let candidates = scan_repo(root).unwrap();
        let paths: Vec<&str> = candidates.iter().map(|c| c.subpath.as_str()).collect();
        assert_eq!(paths, vec!["beta", "tools/alpha"]);
        assert_eq!(candidates[1].name, "alpha");
    }

    #[test]
    fn single_root_skill_is_reported_with_dot() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().join("solo");
        write(&root.join("SKILL.md"), "x");
        let candidates = scan_repo(&root).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].subpath, ".");
        assert_eq!(candidates[0].name, "solo");
    }

    #[test]
    fn repo_name_strips_git_suffix_and_handles_scp() {
        assert_eq!(repo_name("https://host/u/repo.git"), "repo");
        assert_eq!(repo_name("git@host:u/repo.git"), "repo");
        assert_eq!(repo_name("/tmp/x/skills/"), "skills");
    }

    #[test]
    fn skips_repo_root_agents_skills_and_keeps_regular_skills() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        write(&root.join("tools/alpha/SKILL.md"), "a");
        write(&root.join(".agents/skills/speckit/SKILL.md"), "不应入库");
        write(
            &root.join(".agents/skills/nested/other/SKILL.md"),
            "也不应入库",
        );
        write(&root.join(".agents/other/keep/SKILL.md"), "仍应出现");

        let candidates = scan_repo(root).unwrap();
        let paths: Vec<&str> = candidates.iter().map(|c| c.subpath.as_str()).collect();
        assert_eq!(paths, vec![".agents/other/keep", "tools/alpha"]);
    }

    #[test]
    fn remaining_regular_skill_count_is_one_when_agents_skills_is_skipped() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        write(&root.join("solo/SKILL.md"), "s");
        write(&root.join(".agents/skills/speckit/SKILL.md"), "x");

        let candidates = scan_repo(root).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].subpath, "solo");
    }
}
