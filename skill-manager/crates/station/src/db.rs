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
  session_ttl_seconds INTEGER NOT NULL DEFAULT 604800
);
CREATE TABLE IF NOT EXISTS sessions (
  session_id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  last_device_id TEXT,
  pending_rebind INTEGER NOT NULL DEFAULT 0
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
}
