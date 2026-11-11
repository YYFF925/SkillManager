//! 设备登记、一次性对接码、在线长连接。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rand::RngCore;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::db::StationDb;

const PAIR_TTL: Duration = Duration::from_secs(10 * 60);
const ONLINE_TTL: Duration = Duration::from_secs(45);

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceView {
    pub device_id: String,
    pub display_name: String,
    pub online: bool,
    pub revoked: bool,
    pub last_seen_at: Option<String>,
}

#[derive(Clone)]
pub struct LiveLink {
    pub last_seen: Instant,
    pub invoke_tx: mpsc::UnboundedSender<InvokeReq>,
    pub events: broadcast::Sender<Value>,
}

/// Host 经 WebSocket 返回的 invoke 结果，保留 HTTP 状态（含 409 确认计划）。
#[derive(Debug, Clone)]
pub struct InvokeReply {
    pub status: u16,
    pub body: Value,
}

#[derive(Debug)]
pub struct InvokeReq {
    pub id: String,
    pub command: String,
    pub body: Value,
    pub session_id: Option<String>,
    pub reply: oneshot::Sender<InvokeReply>,
}

#[derive(Clone, Default)]
pub struct LiveMap {
    inner: Arc<Mutex<HashMap<String, LiveLink>>>,
}

impl LiveMap {
    pub fn insert(&self, device_id: String, link: LiveLink) {
        if let Ok(mut g) = self.inner.lock() {
            g.insert(device_id, link);
        }
    }

    pub fn remove(&self, device_id: &str) {
        if let Ok(mut g) = self.inner.lock() {
            g.remove(device_id);
        }
    }

    pub fn touch(&self, device_id: &str) {
        if let Ok(mut g) = self.inner.lock() {
            if let Some(link) = g.get_mut(device_id) {
                link.last_seen = Instant::now();
            }
        }
    }

    pub fn is_online(&self, device_id: &str) -> bool {
        self.inner.lock().ok().is_some_and(|g| {
            g.get(device_id)
                .is_some_and(|l| l.last_seen.elapsed() < ONLINE_TTL)
        })
    }

    pub fn get(&self, device_id: &str) -> Option<LiveLink> {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.get(device_id).cloned())
    }
}

fn hash_code(code: &str) -> String {
    hex::encode(Sha256::digest(code.as_bytes()))
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// 生成一次性对接码，明文只返回一次。
pub fn issue_pairing_code(db: &StationDb) -> Result<Value, String> {
    let mut bytes = [0u8; 6];
    rand::thread_rng().fill_bytes(&mut bytes);
    let code = hex::encode(bytes);
    let expires = chrono::Utc::now() + chrono::Duration::seconds(PAIR_TTL.as_secs() as i64);
    db.conn()
        .execute(
            "INSERT INTO pairing_codes (code_hash, expires_at, consumed_at) VALUES (?1, ?2, NULL)",
            rusqlite::params![hash_code(&code), expires.to_rfc3339()],
        )
        .map_err(|err| err.to_string())?;
    Ok(json!({
        "code": code,
        "expiresAt": expires.to_rfc3339(),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairBody {
    pub code: String,
    pub device_id: String,
    pub display_name: String,
    pub device_key: String,
}

/// Host 出站对接：码过期/已用/填错失败，不影响已对接设备。
pub fn consume_pair(db: &StationDb, body: PairBody) -> Result<Value, String> {
    let hash = hash_code(&body.code);
    let now_s = now();
    let row: Option<(String, Option<String>)> = db
        .conn()
        .query_row(
            "SELECT expires_at, consumed_at FROM pairing_codes WHERE code_hash = ?1",
            [&hash],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|err| err.to_string())?;
    let Some((expires_at, consumed_at)) = row else {
        return Err("对接码不正确".to_string());
    };
    if consumed_at.is_some() {
        return Err("对接码已使用".to_string());
    }
    if expires_at < now_s {
        return Err("对接码已过期".to_string());
    }
    db.conn()
        .execute(
            "UPDATE pairing_codes SET consumed_at = ?1 WHERE code_hash = ?2",
            rusqlite::params![now_s, hash],
        )
        .map_err(|err| err.to_string())?;
    db.conn()
        .execute(
            "INSERT INTO devices (device_id, display_name, device_key_hash, last_seen_at, revoked)
             VALUES (?1, ?2, ?3, ?4, 0)
             ON CONFLICT(device_id) DO UPDATE SET
               display_name = excluded.display_name,
               device_key_hash = excluded.device_key_hash,
               revoked = 0,
               last_seen_at = excluded.last_seen_at",
            rusqlite::params![
                body.device_id,
                body.display_name,
                hash_code(&body.device_key),
                now_s
            ],
        )
        .map_err(|err| err.to_string())?;
    Ok(json!({ "ok": true, "deviceId": body.device_id }))
}

pub fn list_devices(db: &StationDb, live: &LiveMap) -> Result<Vec<DeviceView>, String> {
    let mut stmt = db
        .conn()
        .prepare("SELECT device_id, display_name, last_seen_at, revoked FROM devices")
        .map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            Ok(DeviceView {
                device_id: id.clone(),
                display_name: row.get(1)?,
                last_seen_at: row.get(2)?,
                revoked: row.get::<_, i64>(3)? != 0,
                online: live.is_online(&id),
            })
        })
        .map_err(|err| err.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn rename(db: &StationDb, device_id: &str, display_name: &str) -> Result<(), String> {
    let n = db
        .conn()
        .execute(
            "UPDATE devices SET display_name = ?1 WHERE device_id = ?2",
            rusqlite::params![display_name, device_id],
        )
        .map_err(|err| err.to_string())?;
    if n == 0 {
        Err("设备不存在".to_string())
    } else {
        Ok(())
    }
}

pub fn revoke(db: &StationDb, live: &LiveMap, device_id: &str) -> Result<(), String> {
    db.conn()
        .execute(
            "UPDATE devices SET revoked = 1, device_key_hash = ?1 WHERE device_id = ?2",
            rusqlite::params![hash_code(&format!("revoked-{device_id}")), device_id],
        )
        .map_err(|err| err.to_string())?;
    live.remove(device_id);
    Ok(())
}

pub fn verify_device_key(db: &StationDb, device_id: &str, key: &str) -> Result<bool, String> {
    let row: Option<(String, i64)> = db
        .conn()
        .query_row(
            "SELECT device_key_hash, revoked FROM devices WHERE device_id = ?1",
            [device_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|err| err.to_string())?;
    Ok(row.is_some_and(|(hash, revoked)| revoked == 0 && hash == hash_code(key)))
}

pub fn touch_seen(db: &StationDb, device_id: &str) {
    let _ = db.conn().execute(
        "UPDATE devices SET last_seen_at = ?1 WHERE device_id = ?2",
        rusqlite::params![now(), device_id],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::StationDb;

    #[test]
    fn wrong_or_reused_code_does_not_affect_existing_device() {
        let dir = tempfile::tempdir().unwrap();
        let db = StationDb::open(dir.path()).unwrap();
        let issued = issue_pairing_code(&db).unwrap();
        let code = issued["code"].as_str().unwrap().to_string();
        consume_pair(
            &db,
            PairBody {
                code: code.clone(),
                device_id: "dev-1".into(),
                display_name: "A".into(),
                device_key: "key-1".into(),
            },
        )
        .unwrap();
        let reused = consume_pair(
            &db,
            PairBody {
                code,
                device_id: "dev-2".into(),
                display_name: "B".into(),
                device_key: "key-2".into(),
            },
        );
        assert!(reused.is_err());
        let wrong = consume_pair(
            &db,
            PairBody {
                code: "deadbeef".into(),
                device_id: "dev-2".into(),
                display_name: "B".into(),
                device_key: "key-2".into(),
            },
        );
        assert!(wrong.is_err());
        let devices = list_devices(&db, &LiveMap::default()).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_id, "dev-1");
    }
}
