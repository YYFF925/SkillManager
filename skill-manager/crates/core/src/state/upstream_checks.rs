//! 上游检查状态（本机，data-model §2.5）。
//!
//! `(repo, domain, name)` → `remote_version, checked_at, result`。这里只记“最近查到什么、
//! 何时查的、结果如何”，**不**应用技能变化、**不**改采用版本或部署；采用版本与跳过决定
//! 保存在库内 `source.yaml`（共享）。仓库地址按去重规则（去首尾空白、末尾 `/`）归一化。
//!
//! 检查类命令可以保存本机检查事实，因此这张表属于允许写的边界。

use rusqlite::{OptionalExtension, params};

use crate::domain::Domain;
use crate::error::Result;
use crate::manager::SourceRecord;
use crate::state::StateStore;

/// 一条本机上游检查记录。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpstreamCheckRecord {
    /// 归一化后的仓库地址。
    pub repo: String,
    pub domain: Domain,
    pub name: String,
    pub remote_version: Option<String>,
    pub checked_at: Option<String>,
    /// `up_to_date` / `update_available` / `skipped` / `upstream_missing` / `error`。
    pub result: Option<String>,
}

const COLUMNS: &str = "repo, domain, name, remote_version, checked_at, result";

impl UpstreamCheckRecord {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        let domain_raw: String = row.get(1)?;
        let domain = domain_raw.parse::<Domain>().map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(err))
        })?;
        Ok(Self {
            repo: row.get(0)?,
            domain,
            name: row.get(2)?,
            remote_version: row.get(3)?,
            checked_at: row.get(4)?,
            result: row.get(5)?,
        })
    }
}

impl StateStore {
    /// 写入或更新一条检查记录（联合键 `(repo, domain, name)`）。
    pub fn upsert_upstream_check(&self, record: &UpstreamCheckRecord) -> Result<()> {
        let repo = SourceRecord::repo_key(&record.repo);
        self.connection().execute(
            "INSERT INTO upstream_checks (repo, domain, name, remote_version, checked_at, result)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(repo, domain, name) DO UPDATE SET
               remote_version = excluded.remote_version,
               checked_at = excluded.checked_at,
               result = excluded.result",
            params![
                repo,
                record.domain.as_str(),
                record.name,
                record.remote_version,
                record.checked_at,
                record.result,
            ],
        )?;
        Ok(())
    }

    /// 读取一条检查记录。
    pub fn upstream_check(
        &self,
        repo: &str,
        domain: Domain,
        name: &str,
    ) -> Result<Option<UpstreamCheckRecord>> {
        let repo = SourceRecord::repo_key(repo);
        let result = self
            .connection()
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM upstream_checks WHERE repo = ?1 AND domain = ?2 AND name = ?3"
                ),
                params![repo, domain.as_str(), name],
                UpstreamCheckRecord::from_row,
            )
            .optional()?;
        Ok(result)
    }

    /// 最近一次检查所得的具体远端提交（供 `--to`/`--version` 缺省值）。
    pub fn latest_remote_version(
        &self,
        repo: &str,
        domain: Domain,
        name: &str,
    ) -> Result<Option<String>> {
        Ok(self
            .upstream_check(repo, domain, name)?
            .and_then(|record| record.remote_version))
    }

    /// 全部检查记录。
    pub fn upstream_checks(&self) -> Result<Vec<UpstreamCheckRecord>> {
        let mut stmt = self.connection().prepare(&format!(
            "SELECT {COLUMNS} FROM upstream_checks ORDER BY repo, domain, name"
        ))?;
        let rows = stmt.query_map([], UpstreamCheckRecord::from_row)?;
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
    fn check_records_round_trip_and_normalize_repo() {
        let store = StateStore::open_in_memory().unwrap();
        let record = UpstreamCheckRecord {
            repo: "https://x/y.git".to_string(),
            domain: Domain::Code,
            name: "alpha".to_string(),
            remote_version: Some("a".repeat(40)),
            checked_at: Some("2026-10-15T00:00:00Z".to_string()),
            result: Some("update_available".to_string()),
        };
        store.upsert_upstream_check(&record).unwrap();
        let loaded = store
            .upstream_check("https://x/y.git/", Domain::Code, "alpha")
            .unwrap()
            .unwrap();
        assert_eq!(loaded.remote_version, Some("a".repeat(40)));
        assert_eq!(
            store
                .latest_remote_version(" https://x/y.git ", Domain::Code, "alpha")
                .unwrap(),
            Some("a".repeat(40))
        );
        assert_eq!(store.upstream_checks().unwrap().len(), 1);
    }
}
