//! Hermes 档案人格源与分发（SOUL.md）。
//!
//! 源内容保存在 SkillManagerVault 的 `hermes-identity/`（`global.md` + `<档案id>.md`），随库同步。
//! 保存或库拉取源变化后，按「全局在前、专属在后」无条件覆盖本机全部已发现档案的 `SOUL.md`。
//! 磁盘上的 `SOUL.md` 不是真值：不检测冲突、不导入现网文件、空合成也写成空文件。

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::{CoreError, Result};
use crate::vault::compose_instructions;

use super::profiles::discover_profiles;

/// 人格源目录（相对 SkillManagerVault 根）。
pub const IDENTITY_DIR: &str = "hermes-identity";
/// 全局人格源文件名。
pub const GLOBAL_FILE: &str = "global.md";
/// 合成字符数接近 Hermes 截断地板时提示（官方默认下限约 2 万字符）。
pub const LONG_SOUL_CHARS: usize = 20_000;

/// 一次编辑/读取的作用域。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentityScope {
    Global,
    Profile(String),
}

impl IdentityScope {
    /// 解析 `global` 或档案 id。
    pub fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(CoreError::other("人格作用域不能为空（global 或档案 id）"));
        }
        if trimmed == "global" {
            Ok(IdentityScope::Global)
        } else {
            Ok(IdentityScope::Profile(trimmed.to_string()))
        }
    }

    /// 对外的作用域字符串。
    pub fn as_str(&self) -> &str {
        match self {
            IdentityScope::Global => "global",
            IdentityScope::Profile(id) => id,
        }
    }

    /// 源文件名。
    pub fn file_name(&self) -> String {
        match self {
            IdentityScope::Global => GLOBAL_FILE.to_string(),
            IdentityScope::Profile(id) => format!("{id}.md"),
        }
    }
}

/// 与指令相同的四态拼接：去空白；双空→空；单侧有→只用该侧；双有→全局\\n\\n专属。
pub fn compose_identity(global: &str, profile: &str) -> String {
    compose_instructions(global, profile)
}

/// 一个档案的分发结果。
#[derive(Clone, Debug, Serialize)]
pub struct IdentityTargetResult {
    pub profile: String,
    pub path: String,
    pub ok: bool,
    pub chars: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 一次源读取结果。
#[derive(Clone, Debug, Serialize)]
pub struct IdentitySourceView {
    pub scope: String,
    /// 该作用域的源内容；文件不存在按空内容。
    pub source: String,
    /// 按拼接规则得到的合成预览。
    pub composed: String,
}

/// 人格源与分发服务。
pub struct IdentityService {
    vault_root: PathBuf,
    hermes_root: PathBuf,
}

impl IdentityService {
    pub fn new(vault_root: PathBuf, home: PathBuf) -> Self {
        Self {
            vault_root,
            hermes_root: super::hermes_home(&home),
        }
    }

    /// 人格源目录。
    pub fn sources_dir(&self) -> PathBuf {
        self.vault_root.join(IDENTITY_DIR)
    }

    fn source_path(&self, scope: &IdentityScope) -> PathBuf {
        self.sources_dir().join(scope.file_name())
    }

    /// 读取源内容：文件不存在返回空串；其它读取失败如实报错。
    pub fn read_source(&self, scope: &IdentityScope) -> Result<String> {
        read_source_file(&self.source_path(scope))
    }

    /// 读取作用域源内容与合成预览。
    pub fn get(&self, scope: &IdentityScope) -> Result<IdentitySourceView> {
        let source = self.read_source(scope)?;
        let composed = match scope {
            IdentityScope::Global => source.trim().to_string(),
            IdentityScope::Profile(_) => {
                let global = self.read_source(&IdentityScope::Global)?;
                compose_identity(&global, &source)
            }
        };
        Ok(IdentitySourceView {
            scope: scope.as_str().to_string(),
            source,
            composed,
        })
    }

    /// 写入源内容（空内容也写入空文件），再覆盖本机全部已发现档案的 `SOUL.md`。
    pub fn save(&self, scope: &IdentityScope, content: &str) -> Result<Vec<IdentityTargetResult>> {
        let dir = self.sources_dir();
        std::fs::create_dir_all(&dir).map_err(|err| CoreError::io(dir.clone(), err))?;
        let path = self.source_path(scope);
        std::fs::write(&path, content).map_err(|err| CoreError::io(path, err))?;
        self.distribute()
    }

    /// 按当前源内容刷新本机全部已发现档案（库拉取到人格源变化后调用）。
    ///
    /// 不创建档案、不改技能名单。单档案失败不中断其余。
    pub fn distribute(&self) -> Result<Vec<IdentityTargetResult>> {
        let mut results = Vec::new();
        for profile in discover_profiles(&self.hermes_root) {
            // 档案 home 还不存在时不创建 Hermes 根；写 SOUL.md 会变成身份标记。
            if !profile.home.is_dir() {
                continue;
            }
            let id = profile.name.clone();
            let path = profile.home.join("SOUL.md");
            let result = match self.compose_for(&id) {
                Ok(text) => {
                    let chars = text.chars().count();
                    match atomic_write(&path, &text) {
                        Ok(()) => IdentityTargetResult {
                            profile: id,
                            path: path.display().to_string(),
                            ok: true,
                            chars,
                            error: None,
                        },
                        Err(err) => IdentityTargetResult {
                            profile: id,
                            path: path.display().to_string(),
                            ok: false,
                            chars,
                            error: Some(err.to_string()),
                        },
                    }
                }
                Err(err) => IdentityTargetResult {
                    profile: id,
                    path: path.display().to_string(),
                    ok: false,
                    chars: 0,
                    error: Some(err.to_string()),
                },
            };
            results.push(result);
        }
        Ok(results)
    }

    /// 某档案的合成内容（全局 + 专属）。
    pub fn compose_for(&self, profile_id: &str) -> Result<String> {
        let global = self.read_source(&IdentityScope::Global)?;
        let exclusive = self.read_source(&IdentityScope::Profile(profile_id.to_string()))?;
        Ok(compose_identity(&global, &exclusive))
    }

    /// 人格源整体签名（用于判断库拉取后源是否变化）。
    pub fn signature(&self) -> Result<String> {
        source_signature(&self.sources_dir())
    }
}

fn read_source_file(path: &Path) -> Result<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(CoreError::io(path.to_path_buf(), err)),
    }
}

/// 同目录临时文件写入后替换，避免截断的 `SOUL.md`。
fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|err| CoreError::io(parent.to_path_buf(), err))?;
    let tmp_name = format!(
        ".{}.tmp",
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "SOUL.md".to_string())
    );
    let tmp = parent.join(tmp_name);
    let mut file = std::fs::File::create(&tmp).map_err(|err| CoreError::io(tmp.clone(), err))?;
    file.write_all(content.as_bytes())
        .map_err(|err| CoreError::io(tmp.clone(), err))?;
    file.sync_all()
        .map_err(|err| CoreError::io(tmp.clone(), err))?;
    drop(file);
    #[cfg(windows)]
    {
        let _ = std::fs::remove_file(path);
    }
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = std::fs::remove_file(&tmp);
            Err(CoreError::io(path.to_path_buf(), err))
        }
    }
}

fn source_signature(dir: &Path) -> Result<String> {
    let mut entries: Vec<(String, String)> = Vec::new();
    match std::fs::read_dir(dir) {
        Ok(iter) => {
            for entry in iter {
                let entry = entry.map_err(|err| CoreError::io(dir.to_path_buf(), err))?;
                let file_name = entry.file_name().to_string_lossy().into_owned();
                if !file_name.ends_with(".md") {
                    continue;
                }
                let content = read_source_file(&entry.path())?;
                entries.push((file_name, content));
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(CoreError::io(dir.to_path_buf(), err)),
    }
    entries.sort();
    let mut hasher = Sha256::new();
    for (name, content) in entries {
        hasher.update(name.as_bytes());
        hasher.update([0u8]);
        hasher.update(content.as_bytes());
        hasher.update([0u8]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hermes::profiles::{DEFAULT_PROFILE, PROFILE_IDENTITY_MARKERS};

    fn write_profile(root: &Path, name: &str) {
        let home = if name == DEFAULT_PROFILE {
            root.to_path_buf()
        } else {
            root.join("profiles").join(name)
        };
        std::fs::create_dir_all(home.join("skills")).unwrap();
        std::fs::write(
            home.join(PROFILE_IDENTITY_MARKERS[0]),
            "skills:\n  disabled:\n",
        )
        .unwrap();
    }

    #[test]
    fn compose_covers_four_states() {
        assert_eq!(compose_identity("", ""), "");
        assert_eq!(compose_identity("  全局  ", ""), "全局");
        assert_eq!(compose_identity("", "专属"), "专属");
        assert_eq!(compose_identity("全局", "专属"), "全局\n\n专属");
    }

    #[test]
    fn source_scope_file_names() {
        assert_eq!(
            IdentityScope::parse("global").unwrap().file_name(),
            "global.md"
        );
        assert_eq!(
            IdentityScope::parse("drawing").unwrap().file_name(),
            "drawing.md"
        );
        assert_eq!(
            IdentityScope::parse("default").unwrap().file_name(),
            "default.md"
        );
        assert!(IdentityScope::parse("  ").is_err());
    }

    #[test]
    fn missing_source_is_empty_but_read_failure_is_reported() {
        let root = tempfile::TempDir::new().unwrap();
        let dir = root.path().join(IDENTITY_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(read_source_file(&dir.join(GLOBAL_FILE)).unwrap(), "");
        std::fs::create_dir_all(dir.join(GLOBAL_FILE)).unwrap();
        assert!(read_source_file(&dir.join(GLOBAL_FILE)).is_err());
    }

    #[test]
    fn save_overwrites_all_discovered_profiles_including_empty_and_existing() {
        let vault = tempfile::TempDir::new().unwrap();
        let home = tempfile::TempDir::new().unwrap();
        let hermes = home.path().join(".hermes");
        write_profile(&hermes, DEFAULT_PROFILE);
        write_profile(&hermes, "blank");
        std::fs::write(hermes.join("SOUL.md"), "旧 default 人格").unwrap();
        std::fs::write(hermes.join("profiles/blank/SOUL.md"), "旧 blank 人格").unwrap();

        let service = IdentityService::new(vault.path().to_path_buf(), home.path().to_path_buf());
        let results = service.save(&IdentityScope::Global, "全局人格").unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|item| item.ok));
        assert_eq!(
            std::fs::read_to_string(hermes.join("SOUL.md")).unwrap(),
            "全局人格"
        );
        assert_eq!(
            std::fs::read_to_string(hermes.join("profiles/blank/SOUL.md")).unwrap(),
            "全局人格"
        );

        service
            .save(&IdentityScope::Profile("blank".into()), "blank 专属")
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(hermes.join("SOUL.md")).unwrap(),
            "全局人格"
        );
        assert_eq!(
            std::fs::read_to_string(hermes.join("profiles/blank/SOUL.md")).unwrap(),
            "全局人格\n\nblank 专属"
        );

        // 空合成也覆盖成空文件，不保留现网内容。
        service.save(&IdentityScope::Global, "").unwrap();
        service
            .save(&IdentityScope::Profile("blank".into()), "")
            .unwrap();
        assert_eq!(std::fs::read_to_string(hermes.join("SOUL.md")).unwrap(), "");
        assert_eq!(
            std::fs::read_to_string(hermes.join("profiles/blank/SOUL.md")).unwrap(),
            ""
        );
    }

    #[test]
    fn signature_changes_when_source_changes() {
        let vault = tempfile::TempDir::new().unwrap();
        let home = tempfile::TempDir::new().unwrap();
        let service = IdentityService::new(vault.path().to_path_buf(), home.path().to_path_buf());
        let empty = service.signature().unwrap();
        service.save(&IdentityScope::Global, "hello").unwrap();
        let filled = service.signature().unwrap();
        assert_ne!(empty, filled);
    }
}
