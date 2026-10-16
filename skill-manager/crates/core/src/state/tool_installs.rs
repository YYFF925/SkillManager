//! 工具本机安装事实（本机状态，data-model §2.4；T076）。
//!
//! 本表**只**表达 Agent 已核实回写的安装事实：版本、安装方式、安装位置、配置位置。
//! 无此行显示“未安装”（FR-045）；软件检查不能创建或改写这行，也不能修改版本字段。
//! Agent 安装或更新回写后，把该工具的检查结果重置为 `unchecked`（§2.4a）。

use rusqlite::{OptionalExtension, params};

use crate::error::{CoreError, Result};
use crate::state::StateStore;

/// 一条本机安装记录。
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolInstallRecord {
    /// 对应 `tools/<slug>`。
    pub tool_slug: String,
    /// 本机实际安装版本（Agent 核实后回写）。
    pub version: String,
    /** 安装方式（如 uv / npm / brew / 安装包）。 */
    pub method: Option<String>,
    /** 安装位置。 */
    pub path: Option<String>,
    /** 配置位置。 */
    pub config_path: Option<String>,
}

const COLUMNS: &str = "tool_slug, version, method, path, config_path";

impl ToolInstallRecord {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            tool_slug: row.get(0)?,
            version: row.get(1)?,
            method: row.get(2)?,
            path: row.get(3)?,
            config_path: row.get(4)?,
        })
    }
}

impl StateStore {
    /// 回写（或更新）安装事实；随后把该工具的检查结果重置为 `unchecked`。
    pub fn set_tool_install(&self, record: &ToolInstallRecord) -> Result<()> {
        let version = record.version.trim();
        if version.is_empty() {
            return Err(CoreError::other(
                "工具安装版本不能为空（本机已不存在该工具时用 tool set-install 清除记录）",
            ));
        }
        self.connection().execute(
            "INSERT INTO tool_installs (tool_slug, version, method, path, config_path)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(tool_slug) DO UPDATE SET
               version = excluded.version,
               method = excluded.method,
               path = excluded.path,
               config_path = excluded.config_path",
            params![
                record.tool_slug,
                version,
                record.method,
                record.path,
                record.config_path,
            ],
        )?;
        self.reset_tool_check(&record.tool_slug)
    }

    /// 清除本机安装记录（Agent 核实该工具已不在本机时使用），并重置检查结果。
    ///
    /// 没有它，已卸载的工具会永久显示“已安装”，让检查继续与一个不存在的版本对比。
    pub fn clear_tool_install(&self, tool_slug: &str) -> Result<bool> {
        let removed = self
            .connection()
            .execute("DELETE FROM tool_installs WHERE tool_slug = ?1", [tool_slug])?;
        self.reset_tool_check(tool_slug)?;
        Ok(removed > 0)
    }

    /// 读取一条安装记录；无记录返回 `None`（界面显示“未安装”）。
    pub fn tool_install(&self, tool_slug: &str) -> Result<Option<ToolInstallRecord>> {
        let result = self
            .connection()
            .query_row(
                &format!("SELECT {COLUMNS} FROM tool_installs WHERE tool_slug = ?1"),
                [tool_slug],
                ToolInstallRecord::from_row,
            )
            .optional()?;
        Ok(result)
    }

    /// 全部安装记录。
    pub fn tool_installs(&self) -> Result<Vec<ToolInstallRecord>> {
        let mut stmt = self
            .connection()
            .prepare(&format!("SELECT {COLUMNS} FROM tool_installs ORDER BY tool_slug"))?;
        let rows = stmt.query_map([], ToolInstallRecord::from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_records_are_independent_and_clearable() {
        let store = StateStore::open_in_memory().unwrap();
        assert_eq!(store.tool_install("ripgrep").unwrap(), None);

        let record = ToolInstallRecord {
            tool_slug: "ripgrep".to_string(),
            version: "14.1.0".to_string(),
            method: Some("brew".to_string()),
            path: Some("/opt/homebrew/bin/rg".to_string()),
            config_path: None,
        };
        store.set_tool_install(&record).unwrap();
        assert_eq!(store.tool_install("ripgrep").unwrap(), Some(record.clone()));
        assert_eq!(store.tool_installs().unwrap().len(), 1);

        assert!(store.clear_tool_install("ripgrep").unwrap());
        assert_eq!(store.tool_install("ripgrep").unwrap(), None);
        assert!(!store.clear_tool_install("ripgrep").unwrap());
    }

    #[test]
    fn empty_version_is_rejected() {
        let store = StateStore::open_in_memory().unwrap();
        let record = ToolInstallRecord {
            tool_slug: "x".to_string(),
            version: "  ".to_string(),
            method: None,
            path: None,
            config_path: None,
        };
        assert!(store.set_tool_install(&record).is_err());
    }
}
