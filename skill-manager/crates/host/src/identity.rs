//! 本机作为管理站设备的身份（data-model `host_identity`）。

use skill_manager_core::state::StateStore;

/// 本机设备行。
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostIdentity {
    pub device_id: String,
    pub display_name: String,
    pub station_url: String,
    pub device_key: String,
    pub paired_at: Option<String>,
}

/// 读取本机身份。表尚未创建时视为未对接。
pub fn read(store: &StateStore) -> skill_manager_core::Result<Option<HostIdentity>> {
    let conn = store.connection();
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='host_identity'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if exists == 0 {
        return Ok(None);
    }
    let row = conn
        .query_row(
            "SELECT device_id, display_name, station_url, device_key, paired_at FROM host_identity LIMIT 1",
            [],
            |row| {
                Ok(HostIdentity {
                    device_id: row.get(0)?,
                    display_name: row.get(1)?,
                    station_url: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    device_key: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    paired_at: row.get(4)?,
                })
            },
        )
        .optional_row();
    match row {
        Ok(v) => Ok(v),
        Err(err) => Err(skill_manager_core::CoreError::State(err.to_string())),
    }
}

trait OptionalRow<T> {
    fn optional_row(self) -> rusqlite::Result<Option<T>>;
}

impl<T> OptionalRow<T> for rusqlite::Result<T> {
    fn optional_row(self) -> rusqlite::Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

pub fn default_display_name() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Skill Manager Host".to_string())
}
