//! 技能库容量报告（T086，FR-053）。
//!
//! 按**当前库文件**只读扫描：总占用（排除 `.git`）、技能占用合计与较大技能排行。
//! 不建立另一套共享资产存储，也不把容量显示当作自动排除大技能或清理文件的授权。
//! 空库如实返回 0 与空排行。

use std::path::Path;

use serde::Serialize;
use walkdir::WalkDir;

use crate::error::{CoreError, Result};
use crate::vault::scan;

/// 排行榜中的一项技能。
#[derive(Clone, Debug, Serialize)]
pub struct CapacitySkill {
    pub domain: &'static str,
    pub name: String,
    pub category: Option<String>,
    /// 技能目录内文件字节合计（含 `.manager`，排除 `.git`）。
    pub size: u64,
    pub file_count: u64,
    /// 库内绝对路径，供界面定位到具体技能。
    pub path: String,
}

/// 一次容量扫描结果。
#[derive(Clone, Debug, Serialize)]
pub struct CapacityReport {
    pub vault: String,
    /// 整个库文件字节合计（排除 `.git`）。
    pub total_size: u64,
    pub file_count: u64,
    /// 全部技能占用合计。
    pub skill_total_size: u64,
    pub skill_count: usize,
    /// 较大技能排行（按占用降序，最多 `top` 项）。
    pub skills: Vec<CapacitySkill>,
    /// 口径说明。
    pub note: String,
}

/// 计算容量；`top` 为排行榜条数。
pub fn scan(root: &Path, top: usize) -> Result<CapacityReport> {
    if !root.exists() {
        return Err(CoreError::PathMissing(root.to_path_buf()));
    }
    if !root.is_dir() {
        return Err(CoreError::NotADirectory(root.to_path_buf()));
    }

    let (total_size, file_count) = dir_size(root)?;

    let mut skills = Vec::new();
    let mut skill_total_size = 0u64;
    for item in scan::scan_vault(root)? {
        let (size, files) = dir_size(&item.dir)?;
        skill_total_size += size;
        skills.push(CapacitySkill {
            domain: item.domain.as_str(),
            name: item.name,
            category: item.category,
            size,
            file_count: files,
            path: item.dir.display().to_string(),
        });
    }
    skills.sort_by(|a, b| {
        b.size
            .cmp(&a.size)
            .then(a.domain.cmp(b.domain))
            .then(a.name.cmp(&b.name))
    });
    let skill_count = skills.len();
    if top > 0 && skills.len() > top {
        skills.truncate(top);
    }

    Ok(CapacityReport {
        vault: root.display().to_string(),
        total_size,
        file_count,
        skill_total_size,
        skill_count,
        skills,
        note: "按当前库文件统计，排除 .git；技能含其 .manager 管理资料。容量仅供参考，不自动清理或排除任何文件。"
            .to_string(),
    })
}

/// 目录内文件字节与文件数合计；跳过 `.git`，不跟随符号链接。
fn dir_size(dir: &Path) -> Result<(u64, u64)> {
    let mut bytes = 0u64;
    let mut files = 0u64;
    let walker = WalkDir::new(dir).into_iter().filter_entry(|entry| {
        !(entry.file_type().is_dir() && entry.file_name() == ".git")
    });
    for entry in walker {
        let entry = entry
            .map_err(|err| CoreError::other(format!("扫描容量失败（{}）：{err}", dir.display())))?;
        if !entry.file_type().is_file() {
            continue;
        }
        bytes += entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        files += 1;
    }
    Ok((bytes, files))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_total_and_skill_ranking() {
        let root = tempfile::TempDir::new().unwrap();
        let git = root.path().join(".git");
        std::fs::create_dir_all(&git).unwrap();
        std::fs::write(git.join("pack"), "x".repeat(1000)).unwrap();

        let big = root.path().join("code").join("big");
        std::fs::create_dir_all(&big).unwrap();
        std::fs::write(big.join("SKILL.md"), "x".repeat(500)).unwrap();

        let small = root.path().join("code").join("cat").join("small");
        std::fs::create_dir_all(&small).unwrap();
        std::fs::write(small.join("SKILL.md"), "x").unwrap();

        let report = scan(root.path(), 10).unwrap();
        // .git 不计入。
        assert_eq!(report.total_size, 501);
        assert_eq!(report.file_count, 2);
        assert_eq!(report.skill_count, 2);
        assert_eq!(report.skill_total_size, 501);
        assert_eq!(report.skills[0].name, "big");
        assert_eq!(report.skills[0].size, 500);
        assert_eq!(report.skills[1].name, "small");
        assert_eq!(report.skills[1].category.as_deref(), Some("cat"));
        assert!(report.skills[1].path.ends_with("code/cat/small"));
    }

    #[test]
    fn empty_library_is_reported_as_zero() {
        let root = tempfile::TempDir::new().unwrap();
        let report = scan(root.path(), 10).unwrap();
        assert_eq!(report.total_size, 0);
        assert_eq!(report.skill_count, 0);
        assert!(report.skills.is_empty());
    }

    #[test]
    fn top_limits_ranking_but_not_totals() {
        let root = tempfile::TempDir::new().unwrap();
        for (index, size) in [10usize, 30, 20].into_iter().enumerate() {
            let dir = root.path().join("code").join(format!("s{index}"));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("SKILL.md"), "x".repeat(size)).unwrap();
        }
        let report = scan(root.path(), 2).unwrap();
        assert_eq!(report.skills.len(), 2);
        assert_eq!(report.skill_count, 3);
        assert_eq!(report.skill_total_size, 60);
        assert_eq!(report.skills[0].name, "s1");
    }
}
