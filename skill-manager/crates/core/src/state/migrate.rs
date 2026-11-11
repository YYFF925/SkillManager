//! 本机状态 schema 的后续迁移钩子。共享资产不入库。

use rusqlite::Connection;

use crate::error::Result;

/// 003 入口层表：设备身份、确认计划、对话。不存技能正文、不进 Vault。
const MIGRATIONS: &[(&str, &str)] = &[
    (
        "host_identity",
        r#"
CREATE TABLE IF NOT EXISTS host_identity (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  device_id TEXT NOT NULL,
  display_name TEXT NOT NULL,
  station_url TEXT,
  device_key TEXT,
  paired_at TEXT
);
"#,
    ),
    (
        "operation_plans",
        r#"
CREATE TABLE IF NOT EXISTS operation_plans (
  plan_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  action TEXT NOT NULL,
  payload TEXT NOT NULL,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  confirmed_at TEXT
);
"#,
    ),
    (
        "agent_runtime",
        r#"
CREATE TABLE IF NOT EXISTS agent_sessions (
  session_id TEXT PRIMARY KEY,
  view_ref TEXT,
  snapshot TEXT,
  created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS agent_messages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL,
  role TEXT NOT NULL,
  text TEXT NOT NULL,
  task_id TEXT,
  created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS agent_runtime (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  cli_id TEXT,
  binary_path TEXT,
  detected_at TEXT,
  installed_by_app INTEGER NOT NULL DEFAULT 0
);
"#,
    ),
    (
        "agent_runtime_any_acp",
        r#"
-- 占位：列在 apply() 里按 PRAGMA 补，避免重复 ADD COLUMN 失败。
SELECT 1;
"#,
    ),
];

/// 在基础 schema 之后执行附加 SQL。
pub fn apply(conn: &Connection) -> Result<()> {
    for (_name, sql) in MIGRATIONS {
        conn.execute_batch(sql)?;
    }
    ensure_column(
        conn,
        "agent_runtime",
        "acp_args",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(
        conn,
        "agent_runtime",
        "user_selected",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, decl: &str) -> Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|row| row.ok())
        .any(|name| name == column);
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"),
            [],
        )?;
    }
    Ok(())
}
