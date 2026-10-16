//! 运行上下文：同一份 core 逻辑由 CLI 与 Tauri 共用。
//!
//! `--vault` 指定共享内容真值（SkillVault），`--state-dir` 隔离本机状态，
//! 两者一起即可在同一台机器上模拟两台电脑的运行参数。

use std::path::{Path, PathBuf};

use crate::deploy::targets::{AgentCatalog, TargetResolver};
use crate::error::{CoreError, Result};
use crate::state::{setting_key, StateStore};
use crate::vault::VaultConnection;

/// 入口共用的全局参数。
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GlobalOptions {
    /// 输出结构化 JSON 结果。
    #[serde(default)]
    pub json: bool,
    /// SkillVault 根目录。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault: Option<PathBuf>,
    /// 本机状态目录。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_dir: Option<PathBuf>,
}

/// 应用上下文。
pub struct AppContext {
    vault: Option<VaultConnection>,
    state: StateStore,
    catalog: &'static AgentCatalog,
}

impl AppContext {
    /// 默认本机状态目录。
    pub fn default_state_dir() -> PathBuf {
        let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("SkillManager")
    }

    /// 只连接本机状态；SkillVault 稍后按需接入。
    pub fn open(state_dir: Option<PathBuf>) -> Result<Self> {
        let dir = state_dir.unwrap_or_else(Self::default_state_dir);
        let state = StateStore::open(&dir)?;
        Ok(Self {
            vault: None,
            state,
            catalog: AgentCatalog::embedded(),
        })
    }

    /// 连接状态目录并直接接入指定 SkillVault。
    pub fn open_with_vault(
        vault: impl AsRef<Path>,
        state_dir: Option<PathBuf>,
    ) -> Result<Self> {
        let mut ctx = Self::open(state_dir)?;
        ctx.attach_vault(vault)?;
        Ok(ctx)
    }

    /// 接入（或切换）SkillVault；接入结果记在本机状态，便于下次直接进入。
    pub fn attach_vault(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let connection = VaultConnection::open(path.as_ref())?;
        self.state.set_vault_path(connection.root())?;
        self.vault = Some(connection);
        Ok(())
    }

    /// 按本机记住的库路径接入；未记住时返回 None。
    pub fn attach_remembered_vault(&mut self) -> Result<Option<&VaultConnection>> {
        let Some(path) = self.state.vault_path()? else {
            return Ok(None);
        };
        if !path.exists() {
            return Ok(None);
        }
        self.attach_vault(&path)?;
        Ok(self.vault.as_ref())
    }

    pub fn state(&self) -> &StateStore {
        &self.state
    }

    pub fn catalog(&self) -> &'static AgentCatalog {
        self.catalog
    }

    /// 已接入的 SkillVault；尚未接入时报错。
    pub fn vault(&self) -> Result<&VaultConnection> {
        self.vault.as_ref().ok_or_else(|| {
            CoreError::other("尚未接入 SkillVault：请先选择已有本地技能库目录")
        })
    }

    pub fn vault_opt(&self) -> Option<&VaultConnection> {
        self.vault.as_ref()
    }

    /// 当前用户 home。
    pub fn home(&self) -> Result<PathBuf> {
        dirs::home_dir().ok_or_else(|| CoreError::other("无法确定当前用户 home 目录"))
    }

    /// 设置中的共享部署目录。
    pub fn shared_dir(&self) -> Result<Option<PathBuf>> {
        Ok(self
            .state
            .setting(setting_key::SHARED_DIR)?
            .filter(|value| !value.is_empty())
            .map(PathBuf::from))
    }

    /// 目标解析器（内置名单 + 本机共享目录设置 + 本机 Agent 目录覆盖）。
    pub fn target_resolver(&self) -> Result<TargetResolver<'static>> {
        let mut resolver = TargetResolver::new(self.home()?, self.catalog, self.shared_dir()?);
        let dirs: Vec<(String, PathBuf)> = self
            .state
            .agents()?
            .into_iter()
            .filter_map(|config| config.skills_dir.map(|dir| (config.key, PathBuf::from(dir))))
            .collect();
        resolver.set_agent_dirs(dirs);
        Ok(resolver)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_uses_given_state_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let ctx = AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        ctx.state().set_setting(setting_key::THEME, "dark").unwrap();
        assert!(dir.path().join("skill-manager.db").exists());
        assert_eq!(ctx.state().theme().unwrap(), "dark");
    }

    #[test]
    fn vault_is_absent_until_attached() {
        let dir = tempfile::TempDir::new().unwrap();
        let ctx = AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        assert!(ctx.vault_opt().is_none());
        let err = ctx.vault().unwrap_err();
        assert!(err.to_string().contains("尚未接入"), "{err}");
    }

    #[test]
    fn attaching_vault_remembers_path_in_state() {
        let state_dir = tempfile::TempDir::new().unwrap();
        let vault_dir = tempfile::TempDir::new().unwrap();
        crate::sync::Git::new(vault_dir.path())
            .ok(&["init", "--quiet", "--initial-branch=main"])
            .unwrap();

        let mut ctx = AppContext::open(Some(state_dir.path().to_path_buf())).unwrap();
        ctx.attach_vault(vault_dir.path()).unwrap();
        assert!(ctx.vault_opt().is_some());

        let reopened = AppContext::open(Some(state_dir.path().to_path_buf())).unwrap();
        assert!(reopened.state().vault_path().unwrap().is_some());
    }

    #[test]
    fn separate_state_dirs_model_two_computers() {
        let vault_dir = tempfile::TempDir::new().unwrap();
        crate::sync::Git::new(vault_dir.path())
            .ok(&["init", "--quiet", "--initial-branch=main"])
            .unwrap();

        let state_a = tempfile::TempDir::new().unwrap();
        let state_b = tempfile::TempDir::new().unwrap();
        let a = AppContext::open_with_vault(vault_dir.path(), Some(state_a.path().to_path_buf()))
            .unwrap();
        let b = AppContext::open_with_vault(vault_dir.path(), Some(state_b.path().to_path_buf()))
            .unwrap();

        a.state()
            .set_setting(setting_key::SHARED_DIR, "/a/only")
            .unwrap();
        assert_eq!(b.state().setting(setting_key::SHARED_DIR).unwrap(), None);
    }
}
