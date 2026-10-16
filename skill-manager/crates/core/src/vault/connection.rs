//! SkillManagerVault 接入：选择已有本地库、确认 Git 仓库与远端。
//!
//! 不提供克隆或创建向导（FR-002）；首次接入直接选择已有仓库。

use std::path::{Path, PathBuf};

use crate::domain::Domain;
use crate::error::{CoreError, Result};
use crate::sync::Git;

/// 已接入的 SkillManagerVault。
#[derive(Clone, Debug)]
pub struct VaultConnection {
    root: PathBuf,
    git: Git,
}

/// 接入时的核对结果，供界面/CLI 如实表达。
#[derive(Clone, Debug, serde::Serialize)]
pub struct VaultInfo {
    pub root: String,
    pub remote: Option<String>,
    pub has_remote: bool,
}

impl VaultConnection {
    /// 打开已有本地 SkillManagerVault。要求目录存在且是 Git 仓库；远端单独核对。
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let requested = path.as_ref();
        if !requested.exists() {
            return Err(CoreError::PathMissing(requested.to_path_buf()));
        }
        if !requested.is_dir() {
            return Err(CoreError::NotADirectory(requested.to_path_buf()));
        }
        let git = Git::new(requested);
        if !git.is_repo() {
            return Err(CoreError::NotAGitRepo(requested.to_path_buf()));
        }
        // 用仓库根，避免用户选中库内子目录后路径不一致。
        let root = git.top_level().unwrap_or_else(|_| requested.to_path_buf());
        Ok(Self {
            git: Git::new(&root),
            root,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn git(&self) -> &Git {
        &self.git
    }

    /// 管理域目录（`code/`、`lobster/`）。
    pub fn domain_dir(&self, domain: Domain) -> PathBuf {
        self.root.join(domain.dir_name())
    }

    /// 按来源约定解析的库内路径。
    pub fn resolve(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    /// 默认远端名（优先 origin，否则第一个）。
    pub fn default_remote_name(&self) -> Result<Option<String>> {
        let remotes = self.git.remotes()?;
        if remotes.iter().any(|name| name == "origin") {
            return Ok(Some("origin".to_string()));
        }
        Ok(remotes.into_iter().next())
    }

    /// 默认远端地址。
    pub fn remote_url(&self) -> Result<Option<String>> {
        match self.default_remote_name()? {
            Some(name) => self.git.remote_url(&name),
            None => Ok(None),
        }
    }

    pub fn has_remote(&self) -> bool {
        self.remote_url().ok().flatten().is_some()
    }

    /// 接入核对信息。
    pub fn info(&self) -> VaultInfo {
        let remote = self.remote_url().unwrap_or(None);
        VaultInfo {
            root: self.root.display().to_string(),
            has_remote: remote.is_some(),
            remote,
        }
    }

    /// 首次接入核对：FR-002 要求所选库已配置远端。
    pub fn validate_intake(&self) -> Result<VaultInfo> {
        let info = self.info();
        if !info.has_remote {
            return Err(CoreError::NoRemote(self.root.clone()));
        }
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_repo() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        let git = Git::new(dir.path());
        git.ok(&["init", "--quiet", "--initial-branch=main"]).unwrap();
        dir
    }

    #[test]
    fn open_rejects_missing_and_non_repo_paths() {
        let missing = tempfile::TempDir::new().unwrap().path().join("nope");
        assert!(matches!(
            VaultConnection::open(&missing),
            Err(CoreError::PathMissing(_))
        ));

        let plain = tempfile::TempDir::new().unwrap();
        assert!(matches!(
            VaultConnection::open(plain.path()),
            Err(CoreError::NotAGitRepo(_))
        ));
    }

    #[test]
    fn open_resolves_to_repository_root() {
        let repo = make_repo();
        std::fs::create_dir_all(repo.path().join("code/inner")).unwrap();
        let conn = VaultConnection::open(repo.path().join("code/inner")).unwrap();
        assert_eq!(
            conn.root().canonicalize().unwrap(),
            repo.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn intake_requires_remote_and_reports_reason() {
        let repo = make_repo();
        let conn = VaultConnection::open(repo.path()).unwrap();
        assert!(!conn.has_remote());
        let err = conn.validate_intake().unwrap_err();
        assert!(matches!(err, CoreError::NoRemote(_)));
        assert!(err.to_string().contains("未配置远端"), "{err}");
    }

    #[test]
    fn domain_dirs_are_inside_vault() {
        let repo = make_repo();
        let conn = VaultConnection::open(repo.path()).unwrap();
        assert!(conn.domain_dir(Domain::Code).ends_with("code"));
        assert!(conn.domain_dir(Domain::Lobster).ends_with("lobster"));
    }
}
