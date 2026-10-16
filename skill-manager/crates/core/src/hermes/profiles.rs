//! Hermes 单根下的档案发现（T056 核实结论，research.md §五）。
//!
//! 规则与当前 Hermes 一致：default 是**根目录自身**；具名档案是 `<root>/profiles/<名>`，
//! 名匹配 `^[a-z0-9][a-z0-9_-]{0,63}$`、目录含身份标记、且 `profiles/.deleted/<名>` 无墓碑。
//! 无身份标记的副作用目录不会被认作档案。发现只读本机事实，不启动 Hermes、不改任何文件。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{CoreError, Result};

/// default 档案标识。
pub const DEFAULT_PROFILE: &str = "default";

/// 具名档案根目录名。
pub const PROFILES_DIR: &str = "profiles";

/// 已删除档案的墓碑目录名。
pub const DELETED_DIR: &str = ".deleted";

/// 档案身份标记（含任一即视为档案目录）。
pub const PROFILE_IDENTITY_MARKERS: [&str; 6] = [
    "config.yaml",
    ".env",
    "SOUL.md",
    "profile.yaml",
    "auth.json",
    "state.db",
];

/// 档案种类。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileKind {
    Default,
    Named,
}

impl ProfileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProfileKind::Default => "default",
            ProfileKind::Named => "named",
        }
    }
}

/// 一个已发现的档案。
#[derive(Clone, Debug, Serialize)]
pub struct ProfileInfo {
    pub name: String,
    pub kind: ProfileKind,
    /// 档案 home（default 为根自身）。
    pub home: PathBuf,
    /// 档案技能目录：default 为 `<root>/skills`，具名为 `<home>/skills`。
    pub skills_dir: PathBuf,
}

impl ProfileInfo {
    pub fn is_default(&self) -> bool {
        self.kind == ProfileKind::Default
    }
}

/// 档案 id 是否合法（与 Hermes `PROFILE_ID_RE` 一致：小写字母/数字开头，其余小写字母/数字/`-`/`_`，≤64）。
pub fn is_valid_profile_id(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if name.chars().count() > 64 {
        return false;
    }
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// 具名档案根目录。
pub fn profiles_root(root: &Path) -> PathBuf {
    root.join(PROFILES_DIR)
}

/// default 档案（根自身）的技能目录。
pub fn default_skills_dir(root: &Path) -> PathBuf {
    root.join("skills")
}

/// 具名档案 home。
pub fn named_profile_home(root: &Path, name: &str) -> PathBuf {
    profiles_root(root).join(name)
}

/// 墓碑路径 `profiles/.deleted/<名>`。
pub fn tombstone_path(root: &Path, name: &str) -> PathBuf {
    profiles_root(root).join(DELETED_DIR).join(name)
}

/// 目录是否含档案身份标记。
pub fn has_identity(dir: &Path) -> bool {
    PROFILE_IDENTITY_MARKERS
        .iter()
        .any(|marker| dir.join(marker).is_file() || dir.join(marker).is_symlink())
}

/// 档案是否已删除（存在墓碑）。
pub fn is_deleted(root: &Path, name: &str) -> bool {
    tombstone_path(root, name).exists()
}

/// 发现根下的全部档案：default 在前，其后为存活的具名档案（按名排序）。
pub fn discover_profiles(root: &Path) -> Vec<ProfileInfo> {
    let mut out = vec![ProfileInfo {
        name: DEFAULT_PROFILE.to_string(),
        kind: ProfileKind::Default,
        home: root.to_path_buf(),
        skills_dir: default_skills_dir(root),
    }];

    let base = profiles_root(root);
    let Ok(entries) = std::fs::read_dir(&base) else {
        return out;
    };
    let mut named: Vec<ProfileInfo> = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == DEFAULT_PROFILE
            || !is_valid_profile_id(&name)
            || !has_identity(&entry.path())
            || is_deleted(root, &name)
        {
            continue;
        }
        named.push(ProfileInfo {
            name: name.clone(),
            kind: ProfileKind::Named,
            home: entry.path(),
            skills_dir: entry.path().join("skills"),
        });
    }
    named.sort_by(|a, b| a.name.cmp(&b.name));
    out.extend(named);
    out
}

/// 解析单个档案；不存在时报可定位错误。
pub fn resolve_profile(root: &Path, name: &str) -> Result<ProfileInfo> {
    if name == DEFAULT_PROFILE {
        return Ok(ProfileInfo {
            name: DEFAULT_PROFILE.to_string(),
            kind: ProfileKind::Default,
            home: root.to_path_buf(),
            skills_dir: default_skills_dir(root),
        });
    }
    if !is_valid_profile_id(name) {
        return Err(CoreError::other(format!(
            "档案名不合法：{name}（应为小写字母/数字开头，仅含小写字母、数字、- 或 _）"
        )));
    }
    let home = named_profile_home(root, name);
    if !home.is_dir() {
        return Err(CoreError::other(format!(
            "档案不存在：{name}（{}）",
            home.display()
        )));
    }
    if !has_identity(&home) {
        return Err(CoreError::other(format!(
            "目录不像 Hermes 档案（缺少 config.yaml/.env/SOUL.md 等身份标记）：{}",
            home.display()
        )));
    }
    if is_deleted(root, name) {
        return Err(CoreError::other(format!(
            "档案已删除（存在墓碑）：{name}（{}）",
            tombstone_path(root, name).display()
        )));
    }
    Ok(ProfileInfo {
        name: name.to_string(),
        kind: ProfileKind::Named,
        skills_dir: home.join("skills"),
        home,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("skills")).unwrap();
        std::fs::write(dir.path().join("config.yaml"), "skills:\n  disabled:\n").unwrap();
        std::fs::create_dir_all(dir.path().join("profiles/blank")).unwrap();
        std::fs::write(dir.path().join("profiles/blank/config.yaml"), "x: 1\n").unwrap();
        std::fs::create_dir_all(dir.path().join("profiles/sideeffect")).unwrap();
        std::fs::create_dir_all(dir.path().join("profiles/gone")).unwrap();
        std::fs::write(dir.path().join("profiles/gone/config.yaml"), "x: 1\n").unwrap();
        std::fs::create_dir_all(dir.path().join("profiles/.deleted/gone")).unwrap();
        dir
    }

    #[test]
    fn profile_id_grammar_matches_hermes() {
        assert!(is_valid_profile_id("blank"));
        assert!(is_valid_profile_id("u7ed8-u753b-u52a9-u624b"));
        assert!(!is_valid_profile_id("Blank"));
        assert!(!is_valid_profile_id(".deleted"));
        assert!(!is_valid_profile_id("-lead"));
        assert!(!is_valid_profile_id(""));
    }

    #[test]
    fn discovery_keeps_default_and_identity_profiles() {
        let dir = root();
        let names: Vec<String> = discover_profiles(dir.path())
            .into_iter()
            .map(|profile| profile.name)
            .collect();
        assert_eq!(names, vec!["default", "blank"]);
    }

    #[test]
    fn resolve_reports_missing_identity_and_tombstone() {
        let dir = root();
        assert!(resolve_profile(dir.path(), "blank").is_ok());
        assert!(resolve_profile(dir.path(), "default").is_ok());
        assert!(resolve_profile(dir.path(), "nope").is_err());
        assert!(resolve_profile(dir.path(), "sideeffect").is_err());
        assert!(resolve_profile(dir.path(), "gone").is_err());
        assert!(resolve_profile(dir.path(), "Bad").is_err());
    }
}
