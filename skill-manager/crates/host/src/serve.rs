//! `host serve`：前台运行 Host，占锁，本机 HTTP，调度继续按 002 跑。

use std::net::SocketAddr;
use std::path::PathBuf;

use crate::HostRuntime;

/// 绑定地址；默认本机回环随机端口，真实端口写入锁文件供 CLI 发现。
pub fn default_bind() -> SocketAddr {
    "127.0.0.1:0".parse().expect("loopback")
}

/// 在已经占锁的运行时上提供 HTTP（桌面内嵌 Host 用）。
pub fn run_http_blocking(runtime: HostRuntime, bind: SocketAddr) -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|err| err.to_string())?;
    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind(bind)
            .await
            .map_err(|err| err.to_string())?;
        let addr = listener.local_addr().map_err(|err| err.to_string())?;
        runtime.set_listen(addr.to_string())?;
        eprintln!("Skill Manager Host 正在监听 {addr}");
        let app = crate::http::router(runtime);
        axum::serve(listener, app)
            .await
            .map_err(|err| err.to_string())
    })
}

/// 阻塞运行直到收到 SIGINT。已有实例占锁则失败。
pub fn run_blocking(
    state_dir: Option<PathBuf>,
    bind: SocketAddr,
) -> Result<(), String> {
    let runtime = HostRuntime::open(state_dir)?;
    runtime.acquire_lock()?;
    let _ = runtime.watch_vault();
    runtime.spawn_scheduler();

    let rt = tokio::runtime::Runtime::new().map_err(|err| err.to_string())?;
    rt.block_on(async {
        crate::pair::resume_if_paired(&runtime);
        let listener = tokio::net::TcpListener::bind(bind)
            .await
            .map_err(|err| err.to_string())?;
        let addr = listener.local_addr().map_err(|err| err.to_string())?;
        runtime.set_listen(addr.to_string())?;
        eprintln!("Skill Manager Host 正在监听 {addr}");
        let app = crate::http::router(runtime.clone());
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|err| err.to_string())
    })
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
