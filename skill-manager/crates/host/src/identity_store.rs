//! host_identity 表的写入。表由 T020 的 migration 创建。

use skill_manager_core::state::StateStore;
use skill_manager_core::CoreError;

use crate::identity::HostIdentity;

const CREATE: &str = r#"
CREATE TABLE IF NOT EXISTS host_identity (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  device_id TEXT NOT NULL,
  display_name TEXT NOT NULL,
  station_url TEXT,
  device_key TEXT,
  paired_at TEXT
);
"#;

pub fn ensure_table(store: &StateStore) -> Result<(), String> {
    store
        .connection()
        .execute_batch(CREATE)
        .map_err(|err| err.to_string())
}

pub fn upsert(store: &StateStore, identity: &HostIdentity) -> Result<(), String> {
    ensure_table(store)?;
    store
        .connection()
        .execute(
            "INSERT INTO host_identity (id, device_id, display_name, station_url, device_key, paired_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
               device_id = excluded.device_id,
               display_name = excluded.display_name,
               station_url = excluded.station_url,
               device_key = excluded.device_key,
               paired_at = excluded.paired_at",
            rusqlite::params![
                identity.device_id,
                identity.display_name,
                identity.station_url,
                identity.device_key,
                identity.paired_at,
            ],
        )
        .map_err(|err| CoreError::State(err.to_string()).to_string())?;
    Ok(())
}
