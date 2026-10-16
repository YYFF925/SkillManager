//! 部署记录（本机状态，data-model §2.1）。
//!
//! 只存本机事实：技能引用 `(domain, name)`、目标标识、实际副本绝对路径、状态与两个指纹。
//! 受管身份以本表行为唯一依据，不向技能产物写受管标记文件。

use rusqlite::{OptionalExtension, params};

use crate::domain::{Domain, SkillRef};
use crate::error::{CoreError, Result};
use crate::state::StateStore;

/// 部署状态。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeploymentStatus {
    /// 已部署且产物与基线一致。
    Ok,
    /// 受管副本被外部修改，暂停覆盖。
    PausedLocalChange,
}

impl DeploymentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            DeploymentStatus::Ok => "ok",
            DeploymentStatus::PausedLocalChange => "paused_local_change",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "ok" => Ok(DeploymentStatus::Ok),
            "paused_local_change" => Ok(DeploymentStatus::PausedLocalChange),
            other => Err(CoreError::State(format!("未知部署状态：{other}"))),
        }
    }
}

/// 一条部署记录。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeploymentRecord {
    pub domain: Domain,
    pub name: String,
    /// `agent:<key>` / `shared` / `hermes:default`。
    pub target: String,
    /// 实际部署副本目录的绝对路径。
    pub target_path: String,
    pub status: DeploymentStatus,
    /// 合成输入指纹（库内容）。
    pub source_hash: Option<String>,
    /// 最近成功部署的合成产物指纹。
    pub deployed_hash: Option<String>,
    pub synced_at: Option<String>,
}

impl DeploymentRecord {
    pub fn reference(&self) -> SkillRef {
        SkillRef {
            domain: self.domain,
            name: self.name.clone(),
        }
    }

    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        let domain_raw: String = row.get(0)?;
        let status_raw: String = row.get(4)?;
        let domain = domain_raw.parse::<Domain>().map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(err),
            )
        })?;
        let status = DeploymentStatus::parse(&status_raw).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                Box::new(err),
            )
        })?;
        Ok(DeploymentRecord {
            domain,
            name: row.get(1)?,
            target: row.get(2)?,
            target_path: row.get(3)?,
            status,
            source_hash: row.get(5)?,
            deployed_hash: row.get(6)?,
            synced_at: row.get(7)?,
        })
    }
}

const COLUMNS: &str = "domain, name, target, target_path, status, source_hash, deployed_hash, synced_at";

impl StateStore {
    /// 写入或更新一条部署记录（联合键 `(domain, name, target)`）。
    pub fn upsert_deployment(&self, record: &DeploymentRecord) -> Result<()> {
        self.connection().execute(
            "INSERT INTO deployments
               (domain, name, target, target_path, status, source_hash, deployed_hash, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(domain, name, target) DO UPDATE SET
               target_path = excluded.target_path,
               status = excluded.status,
               source_hash = excluded.source_hash,
               deployed_hash = excluded.deployed_hash,
               synced_at = excluded.synced_at",
            params![
                record.domain.as_str(),
                record.name,
                record.target,
                record.target_path,
                record.status.as_str(),
                record.source_hash,
                record.deployed_hash,
                record.synced_at,
            ],
        )?;
        Ok(())
    }

    /// 精确读取一条记录。
    pub fn deployment(
        &self,
        domain: Domain,
        name: &str,
        target: &str,
    ) -> Result<Option<DeploymentRecord>> {
        let result = self
            .connection()
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM deployments WHERE domain = ?1 AND name = ?2 AND target = ?3"
                ),
                params![domain.as_str(), name, target],
                DeploymentRecord::from_row,
            )
            .optional()?;
        Ok(result)
    }

    /// 某技能的全部记录。
    pub fn deployments_for(&self, domain: Domain, name: &str) -> Result<Vec<DeploymentRecord>> {
        let mut stmt = self.connection().prepare(&format!(
            "SELECT {COLUMNS} FROM deployments WHERE domain = ?1 AND name = ?2 ORDER BY target"
        ))?;
        let rows = stmt.query_map(params![domain.as_str(), name], DeploymentRecord::from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(CoreError::from)
    }

    /// 本机全部部署记录。
    pub fn deployments(&self) -> Result<Vec<DeploymentRecord>> {
        let mut stmt = self.connection().prepare(&format!(
            "SELECT {COLUMNS} FROM deployments ORDER BY domain, name, target"
        ))?;
        let rows = stmt.query_map([], DeploymentRecord::from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(CoreError::from)
    }

    /// 删除一条记录；返回是否实际删除。
    pub fn delete_deployment(&self, domain: Domain, name: &str, target: &str) -> Result<bool> {
        let affected = self.connection().execute(
            "DELETE FROM deployments WHERE domain = ?1 AND name = ?2 AND target = ?3",
            params![domain.as_str(), name, target],
        )?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_round_trip_and_are_keyed_by_skill_and_target() {
        let store = StateStore::open_in_memory().unwrap();
        let record = DeploymentRecord {
            domain: Domain::Code,
            name: "alpha".to_string(),
            target: "agent:codex".to_string(),
            target_path: "/tmp/home/.codex/skills/alpha".to_string(),
            status: DeploymentStatus::Ok,
            source_hash: Some("s1".to_string()),
            deployed_hash: Some("d1".to_string()),
            synced_at: Some("2026-10-15T00:00:00Z".to_string()),
        };
        store.upsert_deployment(&record).unwrap();
        let loaded = store
            .deployment(Domain::Code, "alpha", "agent:codex")
            .unwrap()
            .unwrap();
        assert_eq!(loaded, record);

        let mut paused = record.clone();
        paused.status = DeploymentStatus::PausedLocalChange;
        paused.deployed_hash = Some("d2".to_string());
        store.upsert_deployment(&paused).unwrap();
        assert_eq!(
            store
                .deployment(Domain::Code, "alpha", "agent:codex")
                .unwrap()
                .unwrap()
                .status,
            DeploymentStatus::PausedLocalChange
        );
        assert_eq!(store.deployments().unwrap().len(), 1);

        assert!(store.delete_deployment(Domain::Code, "alpha", "agent:codex").unwrap());
        assert!(store.deployments().unwrap().is_empty());
    }
}
