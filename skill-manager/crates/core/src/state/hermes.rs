//! Hermes 档案启用名单存储（本机状态，data-model §2.2）。
//!
//! 名单是 `(profile, domain, name)` → 启用关系；default 档案由 `hermes:default`
//! 部署记录表达，不重复存一条启用记录。内容更新不重置选择（FR-027）。

use crate::domain::{Domain, names_equal};
use crate::error::{CoreError, Result};
use crate::state::StateStore;

/// 一条档案启用记录。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HermesEnable {
    pub profile: String,
    pub domain: Domain,
    pub name: String,
}

impl StateStore {
    /// 某档案的启用技能引用，按名称排序。
    pub fn hermes_enables(&self, profile: &str) -> Result<Vec<(Domain, String)>> {
        let mut stmt = self
            .connection()
            .prepare("SELECT domain, name FROM hermes_enables WHERE profile = ?1 ORDER BY domain, name")?;
        let rows = stmt.query_map([profile], |row| {
            let domain_raw: String = row.get(0)?;
            let name: String = row.get(1)?;
            Ok((domain_raw, name))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (domain_raw, name) = row?;
            let domain = domain_raw.parse::<Domain>().map_err(|err| {
                CoreError::State(format!("hermes_enables 中存在未知管理域 {domain_raw}：{err}"))
            })?;
            out.push((domain, name));
        }
        Ok(out)
    }

    /// 某档案是否已启用某技能。
    pub fn hermes_enabled(&self, profile: &str, domain: Domain, name: &str) -> Result<bool> {
        Ok(self
            .hermes_enables(profile)?
            .iter()
            .any(|(d, n)| *d == domain && names_equal(n, name)))
    }

    /// 某档案启用条目数量。
    pub fn hermes_enable_count(&self, profile: &str) -> Result<usize> {
        Ok(self.hermes_enables(profile)?.len())
    }

    /// 写入一条启用记录（幂等）。
    pub fn set_hermes_enable(&self, profile: &str, domain: Domain, name: &str) -> Result<()> {
        self.connection().execute(
            "INSERT INTO hermes_enables (profile, domain, name) VALUES (?1, ?2, ?3)
             ON CONFLICT(profile, domain, name) DO NOTHING",
            rusqlite::params![profile, domain.as_str(), name],
        )?;
        Ok(())
    }

    /// 删除一条启用记录；返回是否实际删除。
    pub fn remove_hermes_enable(
        &self,
        profile: &str,
        domain: Domain,
        name: &str,
    ) -> Result<bool> {
        let affected = self.connection().execute(
            "DELETE FROM hermes_enables WHERE profile = ?1 AND domain = ?2 AND name = ?3",
            rusqlite::params![profile, domain.as_str(), name],
        )?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enables_round_trip_per_profile() {
        let store = StateStore::open_in_memory().unwrap();
        assert!(store.hermes_enables("blank").unwrap().is_empty());

        store
            .set_hermes_enable("blank", Domain::Lobster, "alpha")
            .unwrap();
        store
            .set_hermes_enable("blank", Domain::Lobster, "alpha")
            .unwrap();
        store
            .set_hermes_enable("drawing", Domain::Lobster, "beta")
            .unwrap();

        assert_eq!(store.hermes_enables("blank").unwrap().len(), 1);
        assert!(store.hermes_enabled("blank", Domain::Lobster, "ALPHA").unwrap());
        assert_eq!(store.hermes_enable_count("drawing").unwrap(), 1);

        assert!(
            store
                .remove_hermes_enable("blank", Domain::Lobster, "alpha")
                .unwrap()
        );
        assert!(!store.hermes_enabled("blank", Domain::Lobster, "alpha").unwrap());
    }
}
