//! `station serve`：HTTP 入口，供已有 HTTPS 反代指过来。第一版不申请证书。

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::auth;
use crate::db::StationDb;
use crate::devices::LiveMap;
use crate::http::{self, AppState};

pub fn default_listen() -> SocketAddr {
    "127.0.0.1:8787".parse().expect("loopback")
}

pub fn default_data_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("SkillManager").join("station")
}

pub fn frontend_dir() -> PathBuf {
    if let Ok(path) = std::env::var("SKILL_MANAGER_WEB") {
        return PathBuf::from(path);
    }
    if let Ok(exe) = std::env::current_exe() {
        let next_to_exe = exe
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("web");
        if next_to_exe.is_dir() {
            return next_to_exe;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dist")
}

/// 阻塞运行管理站。无管理员且 stdin 是终端时走本机创建。
pub fn run_blocking(listen: SocketAddr, data_dir: Option<PathBuf>) -> Result<(), String> {
    let dir = data_dir.unwrap_or_else(default_data_dir);
    let db = StationDb::open(&dir)?;
    if !auth::has_admin(&db)? {
        if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            auth::interactive_setup(&db)?;
        } else {
            eprintln!(
                "尚未创建管理员。请在本机打开终端运行 `skill-manager station serve`，或 POST /auth/setup。"
            );
        }
    }
    let state = AppState {
        db: Arc::new(Mutex::new(db)),
        live: LiveMap::default(),
        web_dir: frontend_dir(),
    };
    let rt = tokio::runtime::Runtime::new().map_err(|err| err.to_string())?;
    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind(listen)
            .await
            .map_err(|err| err.to_string())?;
        let addr = listener.local_addr().map_err(|err| err.to_string())?;
        eprintln!("Skill Manager 管理站正在监听 {addr}");
        let app = http::router(state);
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await
            .map_err(|err| err.to_string())
    })
}

pub fn reset_admin(data_dir: Option<PathBuf>) -> Result<String, String> {
    let dir = data_dir.unwrap_or_else(default_data_dir);
    let db = StationDb::open(&dir)?;
    auth::reset_admin(&db)?;
    Ok("已清除管理员，下次 station serve 将重新创建。".to_string())
}

/// 读取或设置管理员会话保持时长。
pub fn session_ttl(data_dir: Option<PathBuf>, seconds: Option<i64>) -> Result<i64, String> {
    let dir = data_dir.unwrap_or_else(default_data_dir);
    let db = StationDb::open(&dir)?;
    match seconds {
        Some(value) => auth::set_ttl(&db, value),
        None => auth::ttl(&db),
    }
}
