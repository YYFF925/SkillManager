//! 技能目录识别与扫描。
//!
//! 识别谓词（docs/technical-reference/skill-detection.md）：
//! `is_dir() && 目录下存在精确文件名 SKILL.md 或 skill.md 的普通文件`。
//! 文件名大小写敏感；`README.md`/`CLAUDE.md` 不作标记；命中即不向下递归。
//! 技能身份 = 目录名；分类最多一级，超过一级的嵌套不是技能。

use std::path::{Path, PathBuf};

use crate::domain::{Domain, validate_category_name, validate_skill_name};
use crate::error::{CoreError, Result};

/// 候选入口文件名，`SKILL.md` 在前、`skill.md` 在后（精确匹配）。
pub const SKILL_ENTRY_FILES: [&str; 2] = ["SKILL.md", "skill.md"];

/// 扫描时跳过的目录名（按目录名精确匹配）。
pub const SKIP_DIRS: [&str; 1] = [".git"];

/// 目录是否为技能：存在精确文件名的入口文件（普通文件）。
pub fn is_skill_dir(dir: &Path) -> bool {
    entry_file(dir).is_some()
}

/// 入口文件路径与文件名（`SKILL.md` 优先）。
///
/// 按目录项的实际名称做精确比较：大小写不敏感的文件系统（macOS 默认）上
/// 只存在 `skill.md` 时就报 `skill.md`，不会误报为 `SKILL.md`。
pub fn entry_file(dir: &Path) -> Option<(PathBuf, &'static str)> {
    if !dir.is_dir() {
        return None;
    }
    let entries = std::fs::read_dir(dir).ok()?;
    let mut names: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    for candidate in SKILL_ENTRY_FILES {
        if !names.iter().any(|name| name == candidate) {
            continue;
        }
        let path = dir.join(candidate);
        if path.is_file() {
            return Some((path, candidate));
        }
    }
    None
}

/// 入口文件名；不在技能目录时返回 `None`。
pub fn entry_file_name(dir: &Path) -> Option<&'static str> {
    entry_file(dir).map(|(_, name)| name)
}

/// frontmatter 中用于展示的字段。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Frontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// 读取入口文件的 frontmatter；缺失、未闭合或解析失败均按无字段处理。
///
/// 这里只用于展示（名称与描述），读取失败不改变技能身份。
pub fn read_frontmatter(dir: &Path) -> Frontmatter {
    let Some((path, _)) = entry_file(dir) else {
        return Frontmatter::default();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Frontmatter::default();
    };
    parse_frontmatter(&text)
}

/// 解析 frontmatter：以 `---` 开头，取首个 `---` 到其后首个 `---` 之间的 YAML。
pub fn parse_frontmatter(text: &str) -> Frontmatter {
    let trimmed = text.trim_start_matches('\u{feff}');
    let mut lines = trimmed.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return Frontmatter::default();
    }

    let mut block = String::new();
    let mut closed = false;
    for line in lines {
        if line.trim_end() == "---" {
            closed = true;
            break;
        }
        block.push_str(line);
        block.push('\n');
    }
    if !closed {
        return Frontmatter::default();
    }

    let Ok(value) = yaml_serde::from_str::<yaml_serde::Value>(&block) else {
        return Frontmatter::default();
    };
    let field = |key: &str| -> Option<String> {
        value
            .get(key)
            .and_then(yaml_serde::Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string)
    };
    Frontmatter {
        name: field("name"),
        description: field("description"),
    }
}

/// 扫描得到的技能（未加载管理资料）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScannedSkill {
    pub domain: Domain,
    pub category: Option<String>,
    pub name: String,
    pub dir: PathBuf,
    /// 库内相对目录，`/` 分隔，含域。
    pub vault_rel_dir: String,
    pub entry_file: String,
}

/// 扫描一个管理域；域目录不存在时返回空（尚未使用过该库）。
pub fn scan_domain(root: &Path, domain: Domain) -> Result<Vec<ScannedSkill>> {
    let base = root.join(domain.dir_name());
    if !base.exists() {
        return Ok(Vec::new());
    }
    if !base.is_dir() {
        return Err(CoreError::NotADirectory(base));
    }

    let mut out = Vec::new();
    for child in subdirs(&base)? {
        let (child_path, child_name) = child;
        if is_skill_dir(&child_path) {
            out.push(scanned(domain, None, child_name, child_path)?);
            continue;
        }
        // 分类目录：只检查下一层；更深嵌套不是技能。
        for grand in subdirs(&child_path)? {
            let (grand_path, grand_name) = grand;
            if is_skill_dir(&grand_path) {
                out.push(scanned(domain, Some(child_name.clone()), grand_name, grand_path)?);
            }
        }
    }
    out.sort_by(|a, b| {
        a.domain
            .cmp(&b.domain)
            .then(a.category.cmp(&b.category))
            .then(a.name.cmp(&b.name))
    });
    Ok(out)
}

/// 扫描两个管理域。
pub fn scan_vault(root: &Path) -> Result<Vec<ScannedSkill>> {
    if !root.exists() {
        return Err(CoreError::PathMissing(root.to_path_buf()));
    }
    if !root.is_dir() {
        return Err(CoreError::NotADirectory(root.to_path_buf()));
    }
    let mut all = Vec::new();
    for domain in Domain::ALL {
        all.extend(scan_domain(root, domain)?);
    }
    Ok(all)
}

fn scanned(
    domain: Domain,
    category: Option<String>,
    name: String,
    dir: PathBuf,
) -> Result<ScannedSkill> {
    let entry_file = entry_file_name(&dir)
        .ok_or_else(|| CoreError::other(format!("不是技能目录：{}", dir.display())))?
        .to_string();
    // 名称来自目录，仍然做一次形状校验，避免出现无法作为单层目录名的身份。
    validate_skill_name(&name)?;
    if let Some(category) = &category {
        validate_category_name(category)?;
    }
    let rel = match &category {
        Some(category) => PathBuf::from(domain.dir_name()).join(category).join(&name),
        None => PathBuf::from(domain.dir_name()).join(&name),
    };
    Ok(ScannedSkill {
        domain,
        category,
        name,
        dir,
        vault_rel_dir: rel.to_string_lossy().replace('\\', "/"),
        entry_file,
    })
}

/// 目录下的子目录（跳过文件与被排除目录）。
fn subdirs(dir: &Path) -> Result<Vec<(PathBuf, String)>> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|err| CoreError::io(dir.to_path_buf(), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| CoreError::io(dir.to_path_buf(), err))?;
        let file_type = entry
            .file_type()
            .map_err(|err| CoreError::io(entry.path(), err))?;
        // 符号链接不跟随，避免链接环与外部目录被当作库内容。
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if SKIP_DIRS.contains(&name.as_str()) {
            continue;
        }
        out.push((entry.path(), name));
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_requires_closed_block() {
        assert_eq!(parse_frontmatter("---\nname: a\n---\n正文").name.as_deref(), Some("a"));
        assert_eq!(parse_frontmatter("---\nname: a\n正文").name, None);
        assert_eq!(parse_frontmatter("正文").name, None);
        assert_eq!(parse_frontmatter("---\nname: a\n---\n").name.as_deref(), Some("a"));
    }

    #[test]
    fn entry_file_reports_the_actual_on_disk_name() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(entry_file(dir.path()).is_none());

        // 只有小写兼容入口时如实报 skill.md（大小写不敏感文件系统上不能靠路径探测区分）。
        std::fs::write(dir.path().join("skill.md"), "x").unwrap();
        assert_eq!(entry_file_name(dir.path()), Some("skill.md"));

        let upper = tempfile::TempDir::new().unwrap();
        std::fs::write(upper.path().join("SKILL.md"), "x").unwrap();
        assert_eq!(entry_file_name(upper.path()), Some("SKILL.md"));

        // README 不作标记。
        let other = tempfile::TempDir::new().unwrap();
        std::fs::write(other.path().join("README.md"), "x").unwrap();
        assert!(!is_skill_dir(other.path()));
    }
}
