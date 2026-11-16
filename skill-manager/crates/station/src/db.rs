//! `station.db`：管理员、会话、设备、对接码。无技能表。

use std::path::{Path, PathBuf};

use rusqlite::Connection;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS admin (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  username TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  totp_secret TEXT,
  recovery_code_hashes TEXT NOT NULL DEFAULT '[]',
  session_ttl_seconds INTEGER NOT NULL DEFAULT 31104000
);
CREATE TABLE IF NOT EXISTS sessions (
  session_id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  last_device_id TEXT,
  pending_rebind INTEGER NOT NULL DEFAULT 0,
  pending_totp_secret TEXT,
  pending_recovery_hashes TEXT
);
CREATE TABLE IF NOT EXISTS devices (
  device_id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  device_key_hash TEXT NOT NULL,
  last_seen_at TEXT,
  revoked INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS pairing_codes (
  code_hash TEXT PRIMARY KEY,
  expires_at TEXT NOT NULL,
  consumed_at TEXT
);
"#;

/// 管理站状态目录下的 SQLite。
pub struct StationDb {
    conn: Connection,
    dir: PathBuf,
}

impl StationDb {
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, String> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
        let conn = Connection::open(dir.join("station.db")).map_err(|err| err.to_string())?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|err| err.to_string())?;
        conn.pragma_update(None, "busy_timeout", 5_000)
            .map_err(|err| err.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|err| err.to_string())?;
        let _ = conn.execute(
            "ALTER TABLE sessions ADD COLUMN pending_rebind INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE sessions ADD COLUMN pending_totp_secret TEXT",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE sessions ADD COLUMN pending_recovery_hashes TEXT",
            [],
        );
        let user_version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|err| err.to_string())?;
        if user_version < 1 {
            // 将旧默认值 7 天（604800 秒）一次性升级为 360 天（31104000 秒）；不覆盖用户已自定义的值。
            conn.execute(
                "UPDATE admin SET session_ttl_seconds = 31104000 WHERE session_ttl_seconds = 604800",
                [],
            )
            .map_err(|err| err.to_string())?;
            conn.pragma_update(None, "user_version", 1)
                .map_err(|err| err.to_string())?;
        }
        Ok(Self { conn, dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_has_no_skill_tables() {
        let dir = tempfile::tempdir().unwrap();
        let db = StationDb::open(dir.path()).unwrap();
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('skills','deployments','agent_sessions')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn upgrades_legacy_7_day_default_ttl_once() {
        let dir = tempfile::tempdir().unwrap();
        {
            let db = StationDb::open(dir.path()).unwrap();
            db.conn().pragma_update(None, "user_version", 0).unwrap();
            db.conn()
                .execute(
                    "INSERT INTO admin (id, username, password_hash, totp_secret, recovery_code_hashes, session_ttl_seconds)
                     VALUES (1, 'admin', 'hash', 'sec', '[]', 604800)",
                    [],
                )
                .unwrap();
        }
        let db = StationDb::open(dir.path()).unwrap();
        let ttl: i64 = db
            .conn()
            .query_row("SELECT session_ttl_seconds FROM admin WHERE id = 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(ttl, 31_104_000);

        // 用户之后若显式改回 604800，再次打开库不得覆盖。
        db.conn()
            .execute("UPDATE admin SET session_ttl_seconds = 604800 WHERE id = 1", [])
            .unwrap();
        drop(db);
        let db = StationDb::open(dir.path()).unwrap();
        let ttl: i64 = db
            .conn()
            .query_row("SELECT session_ttl_seconds FROM admin WHERE id = 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(ttl, 604_800);
    }
}
