//! 工具本机检查结果（本机状态，data-model §2.4a；T076）。
//!
//! 与安装事实**分开存储**：未安装的工具也可以有检查结果（只表示已知上游版本），
//! 分开存储正是为了避免“未安装却有检查结果”被误表示成已安装。检查不能插入安装行。
//! Agent 安装或更新回写后，本表对应记录被重置为 `unchecked`。

use rusqlite::{OptionalExtension, params};

use crate::error::{CoreError, Result};
use crate::state::StateStore;

/// 检查状态：无记录为 `unchecked`。
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    /// 尚未检查（无记录）。
    Unchecked,
    /// 检查进行中。
    Checking,
    /// 成功取得上游最新版本。
    Checked,
    /// 检查失败（非零退出、空结果或执行失败）。
    Failed,
    /// 该工具未登记检查命令。
    NoCommand,
}

impl CheckStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            CheckStatus::Unchecked => "unchecked",
            CheckStatus::Checking => "checking",
            CheckStatus::Checked => "checked",
            CheckStatus::Failed => "failed",
            CheckStatus::NoCommand => "no_command",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "unchecked" => Ok(CheckStatus::Unchecked),
            "checking" => Ok(CheckStatus::Checking),
            "checked" => Ok(CheckStatus::Checked),
            "failed" => Ok(CheckStatus::Failed),
            "no_command" => Ok(CheckStatus::NoCommand),
            other => Err(CoreError::State(format!("未知工具检查状态：{other}"))),
        }
    }
}

/// 一条工具检查记录。
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolCheckRecord {
    pub tool_slug: String,
    pub status: CheckStatus,
    /// 成功获取的上游最新版本；失败/未登记时不得当作本次有效结论。
    pub latest_version: Option<String>,
    pub checked_at: Option<String>,
    /// 本次检查的命令结果（失败原因或说明），供如实展示。
    pub check_output: Option<String>,
}

const COLUMNS: &str = "tool_slug, status, latest_version, checked_at, check_output";

impl ToolCheckRecord {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        let raw: String = row.get(1)?;
        let status = CheckStatus::parse(&raw).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(err))
        })?;
        Ok(Self {
            tool_slug: row.get(0)?,
            status,
            latest_version: row.get(2)?,
            checked_at: row.get(3)?,
            check_output: row.get(4)?,
        })
    }
}

impl StateStore {
    /// 写入或更新一条检查记录。
    pub fn set_tool_check(&self, record: &ToolCheckRecord) -> Result<()> {
        self.connection().execute(
            "INSERT INTO tool_checks (tool_slug, status, latest_version, checked_at, check_output)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(tool_slug) DO UPDATE SET
               status = excluded.status,
               latest_version = excluded.latest_version,
               checked_at = excluded.checked_at,
               check_output = excluded.check_output",
            params![
                record.tool_slug,
                record.status.as_str(),
                record.latest_version,
                record.checked_at,
                record.check_output,
            ],
        )?;
        Ok(())
    }

    /// 读取一条检查记录；无记录返回 `None`（等价于 `unchecked`）。
    pub fn tool_check(&self, tool_slug: &str) -> Result<Option<ToolCheckRecord>> {
        let result = self
            .connection()
            .query_row(
                &format!("SELECT {COLUMNS} FROM tool_checks WHERE tool_slug = ?1"),
                [tool_slug],
                ToolCheckRecord::from_row,
            )
            .optional()?;
        Ok(result)
    }

    /// 检查状态；无记录为 `unchecked`。
    pub fn tool_check_status(&self, tool_slug: &str) -> Result<CheckStatus> {
        Ok(self
            .tool_check(tool_slug)?
            .map(|record| record.status)
            .unwrap_or(CheckStatus::Unchecked))
    }

    /// 全部检查记录。
    pub fn tool_checks(&self) -> Result<Vec<ToolCheckRecord>> {
        let mut stmt = self
            .connection()
            .prepare(&format!("SELECT {COLUMNS} FROM tool_checks ORDER BY tool_slug"))?;
        let rows = stmt.query_map([], ToolCheckRecord::from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 把检查结果重置为 `unchecked`（删除记录，保留“无记录即未检查”的单一表示）。
    pub fn reset_tool_check(&self, tool_slug: &str) -> Result<()> {
        self.connection()
            .execute("DELETE FROM tool_checks WHERE tool_slug = ?1", [tool_slug])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_record_reads_as_unchecked() {
        let store = StateStore::open_in_memory().unwrap();
        assert_eq!(store.tool_check("uv").unwrap(), None);
        assert_eq!(store.tool_check_status("uv").unwrap(), CheckStatus::Unchecked);
    }

    #[test]
    fn check_records_do_not_create_install_rows() {
        let store = StateStore::open_in_memory().unwrap();
        store
            .set_tool_check(&ToolCheckRecord {
                tool_slug: "uv".to_string(),
                status: CheckStatus::Checked,
                latest_version: Some("0.9.0".to_string()),
                checked_at: Some("2026-10-15T00:00:00Z".to_string()),
                check_output: None,
            })
            .unwrap();
        assert_eq!(store.tool_check_status("uv").unwrap(), CheckStatus::Checked);
        // 检查不产生安装事实。
        assert_eq!(store.tool_install("uv").unwrap(), None);

        store.reset_tool_check("uv").unwrap();
        assert_eq!(store.tool_check_status("uv").unwrap(), CheckStatus::Unchecked);
    }

    #[test]
    fn install_write_back_resets_check() {
        let store = StateStore::open_in_memory().unwrap();
        store
            .set_tool_check(&ToolCheckRecord {
                tool_slug: "uv".to_string(),
                status: CheckStatus::Failed,
                latest_version: None,
                checked_at: Some("2026-10-15T00:00:00Z".to_string()),
                check_output: Some("退出码 1".to_string()),
            })
            .unwrap();
        store
            .set_tool_install(&crate::state::ToolInstallRecord {
                tool_slug: "uv".to_string(),
                version: "0.8.0".to_string(),
                method: None,
                path: None,
                config_path: None,
            })
            .unwrap();
        assert_eq!(store.tool_check_status("uv").unwrap(), CheckStatus::Unchecked);
    }
}
