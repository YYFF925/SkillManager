//! 本机状态存储：SQLite（WAL + busy_timeout），只存本机状态。

pub mod deployments;
pub mod hermes;
pub mod migrate;
pub mod settings;
pub mod tool_checks;
pub mod tool_installs;
pub mod upstream_checks;

pub use deployments::{DeploymentRecord, DeploymentStatus};
pub use tool_checks::{CheckStatus, ToolCheckRecord};
pub use tool_installs::ToolInstallRecord;
pub use upstream_checks::UpstreamCheckRecord;

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{CoreError, Result};

/// 数据库文件名。
pub const DB_FILE: &str = "skill-manager.db";

const SCHEMA: &str = include_str!("schema.sql");

/// 设置键（data-model §2.6）。
pub mod setting_key {
    pub const THEME: &str = "theme";
    pub const FONT_SCALE: &str = "font_scale";
    pub const AUTO_CHECK_MINUTES: &str = "auto_check_minutes";
    pub const LAST_SYNC_AT: &str = "last_sync_at";
    pub const LAST_CHECK_AT: &str = "last_check_at";
    pub const VAULT_PATH: &str = "vault_path";
    pub const GIT_AUTO_PUSH: &str = "git_auto_push";
}

/// 主题默认值与合法取值（默认跟随系统）。
pub const THEME_DEFAULT: &str = "system";
/// 主题可选值：浅色 / 深色 / 跟随系统。
pub const THEME_ALLOWED: [&str; 3] = ["light", "dark", "system"];

/// 字号默认值与合法取值。
pub const FONT_SCALE_DEFAULT: &str = "default";
/// 字号可选值。
pub const FONT_SCALE_ALLOWED: [&str; 4] = ["small", "default", "large", "xlarge"];

/// 自动检查频率：默认 60 分钟（FR-029，三项共用）。
pub const AUTO_CHECK_MINUTES_DEFAULT: i64 = 60;
/// 可选频率：0=关闭 / 30 / 60 / 360 / 1440。
pub const AUTO_CHECK_MINUTES_ALLOWED: [i64; 5] = [0, 30, 60, 360, 1440];

/// 本机状态存储。
pub struct StateStore {
    conn: Connection,
    dir: PathBuf,
}

impl StateStore {
    /// 打开（必要时创建）状态目录下的数据库，并保证表结构存在。
    pub fn open(state_dir: impl AsRef<Path>) -> Result<Self> {
        let dir = state_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir).map_err(|err| CoreError::io(dir.clone(), err))?;
        let db_path = dir.join(DB_FILE);

        let conn = Connection::open(&db_path).map_err(|err| CoreError::State(err.to_string()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", 5_000)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.execute_batch(SCHEMA)?;
        migrate::apply(&conn)?;

        Ok(Self { conn, dir })
    }

    /// 打开内存数据库，供单元测试使用。
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| CoreError::State(e.to_string()))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.execute_batch(SCHEMA)?;
        migrate::apply(&conn)?;
        Ok(Self {
            conn,
            dir: PathBuf::new(),
        })
    }

    /// 状态目录。
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    // ---- settings ----

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        let value = self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get::<_, String>(0)
            })
            .optional()?;
        Ok(value)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn remove_setting(&self, key: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM settings WHERE key = ?1", [key])?;
        Ok(())
    }

    /// 全部设置项。
    pub fn settings(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM settings ORDER BY key")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 主题：默认跟随系统。
    pub fn theme(&self) -> Result<String> {
        Ok(self
            .setting(setting_key::THEME)?
            .unwrap_or_else(|| THEME_DEFAULT.to_string()))
    }

    /// 字号：默认 `default`。
    pub fn font_scale(&self) -> Result<String> {
        Ok(self
            .setting(setting_key::FONT_SCALE)?
            .unwrap_or_else(|| FONT_SCALE_DEFAULT.to_string()))
    }

    /// 已接入的 SkillManagerVault 路径（本机记住，便于下次直接进入技能库）。
    pub fn vault_path(&self) -> Result<Option<PathBuf>> {
        Ok(self
            .setting(setting_key::VAULT_PATH)?
            .filter(|value| !value.is_empty())
            .map(PathBuf::from))
    }

    pub fn set_vault_path(&self, path: impl AsRef<Path>) -> Result<()> {
        self.set_setting(
            setting_key::VAULT_PATH,
            &path.as_ref().to_string_lossy(),
        )
    }

    // ---- agents ----

    /// 写入或更新一个 Agent 的本机配置。
    pub fn upsert_agent(&self, agent: &AgentConfig) -> Result<()> {
        self.conn.execute(
            "INSERT INTO agents (key, enabled, sort_order, skills_dir, project_rel_dir, instructions_target)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(key) DO UPDATE SET
               enabled = excluded.enabled,
               sort_order = excluded.sort_order,
               skills_dir = excluded.skills_dir,
               project_rel_dir = excluded.project_rel_dir,
               instructions_target = excluded.instructions_target",
            params![
                agent.key,
                agent.enabled as i64,
                agent.order,
                agent.skills_dir,
                agent.project_rel_dir,
                agent.instructions_target,
            ],
        )?;
        Ok(())
    }

    /// 某个 Agent 的本机配置。
    pub fn agent(&self, key: &str) -> Result<Option<AgentConfig>> {
        let result = self
            .conn
            .query_row(
                "SELECT key, enabled, sort_order, skills_dir, project_rel_dir, instructions_target
                 FROM agents WHERE key = ?1",
                [key],
                AgentConfig::from_row,
            )
            .optional()?;
        Ok(result)
    }

    /// 全部 Agent 本机配置，按顺序返回。
    pub fn agents(&self) -> Result<Vec<AgentConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT key, enabled, sort_order, skills_dir, project_rel_dir, instructions_target
             FROM agents ORDER BY sort_order, key",
        )?;
        let rows = stmt.query_map([], AgentConfig::from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 仅启停。
    pub fn set_agent_enabled(&self, key: &str, enabled: bool) -> Result<()> {
        self.conn.execute(
            "INSERT INTO agents (key, enabled) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET enabled = excluded.enabled",
            params![key, enabled as i64],
        )?;
        Ok(())
    }

    /// 仅排序。
    pub fn set_agent_order(&self, key: &str, order: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO agents (key, sort_order) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET sort_order = excluded.sort_order",
            params![key, order],
        )?;
        Ok(())
    }

    /// 设置或清除技能目录；传 None 表示恢复默认。
    pub fn set_agent_skills_dir(&self, key: &str, dir: Option<&str>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO agents (key, skills_dir) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET skills_dir = excluded.skills_dir",
            params![key, dir],
        )?;
        Ok(())
    }

    /// 设置或清除项目技能相对路径（仅配置保留）。
    pub fn set_agent_project_rel_dir(&self, key: &str, dir: Option<&str>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO agents (key, project_rel_dir) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET project_rel_dir = excluded.project_rel_dir",
            params![key, dir],
        )?;
        Ok(())
    }

    /// 设置或清除指令分发目标。
    pub fn set_agent_instructions_target(&self, key: &str, path: Option<&str>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO agents (key, instructions_target) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET instructions_target = excluded.instructions_target",
            params![key, path],
        )?;
        Ok(())
    }
}

/// 本机 Agent 配置行。
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AgentConfig {
    pub key: String,
    pub enabled: bool,
    pub order: i64,
    /// 实际技能目录（绝对路径）；None 表示使用内置默认目录。
    pub skills_dir: Option<String>,
    /// 项目技能相对路径（仅配置保留，不实现项目技能管理）。
    pub project_rel_dir: Option<String>,
    /// 指令分发目标（`AGENTS.md` 位置）；None 表示使用内置默认。
    pub instructions_target: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            key: String::new(),
            enabled: false,
            order: 0,
            skills_dir: None,
            project_rel_dir: None,
            instructions_target: None,
        }
    }
}

impl AgentConfig {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            ..Self::default()
        }
    }

    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            key: row.get(0)?,
            enabled: row.get::<_, i64>(1)? != 0,
            order: row.get(2)?,
            skills_dir: row.get(3)?,
            project_rel_dir: row.get(4)?,
            instructions_target: row.get(5)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_created_and_settings_round_trip() {
        let store = StateStore::open_in_memory().unwrap();
        assert_eq!(store.theme().unwrap(), "system");
        assert_eq!(store.auto_check_minutes().unwrap(), 60);

        store.set_setting(setting_key::THEME, "dark").unwrap();
        assert_eq!(store.theme().unwrap(), "dark");
        store
            .set_setting(setting_key::AUTO_CHECK_MINUTES, "1440")
            .unwrap();
        assert_eq!(store.auto_check_minutes().unwrap(), 1440);
    }

    #[test]
    fn agent_config_defaults_are_null_not_empty_strings() {
        let store = StateStore::open_in_memory().unwrap();
        let mut agent = AgentConfig::new("codex");
        agent.enabled = true;
        agent.order = 3;
        store.upsert_agent(&agent).unwrap();

        let stored = store.agent("codex").unwrap().unwrap();
        assert!(stored.enabled);
        assert_eq!(stored.order, 3);
        // 目录类配置缺省为 NULL（恢复默认）；不是空字符串。
        assert_eq!(stored.skills_dir, None);
        assert_eq!(stored.project_rel_dir, None);
        assert_eq!(stored.instructions_target, None);

        store.set_agent_skills_dir("codex", Some("/tmp/x")).unwrap();
        assert_eq!(
            store.agent("codex").unwrap().unwrap().skills_dir.as_deref(),
            Some("/tmp/x")
        );
        store.set_agent_skills_dir("codex", None).unwrap();
        assert_eq!(store.agent("codex").unwrap().unwrap().skills_dir, None);
    }

    #[test]
    fn shared_assets_have_no_mirror_tables() {
        let store = StateStore::open_in_memory().unwrap();
        let mut stmt = store
            .connection()
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap();
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(std::result::Result::unwrap)
            .collect();
        // 本机状态表固定为这七张；技能清单、预设、工具资料等共享资产不入库。
        assert_eq!(
            names,
            vec![
                "agents",
                "deployments",
                "hermes_enables",
                "settings",
                "tool_checks",
                "tool_installs",
                "upstream_checks",
            ]
        );
    }

    #[test]
    fn vault_path_is_remembered_per_state_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = StateStore::open(dir.path()).unwrap();
        assert_eq!(store.vault_path().unwrap(), None);
        store.set_vault_path("/tmp/vault").unwrap();
        assert_eq!(
            store.vault_path().unwrap().unwrap().to_string_lossy(),
            "/tmp/vault"
        );
    }
}
