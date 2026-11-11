//! Tauri 2 桌面壳：持有 Host runtime；IPC 经 `host_invoke` 转发 Host，不直打 core。

mod commands;
mod diagnostics;
mod tray;

use std::path::PathBuf;
use std::sync::Arc;

use commands::AppState;
use skill_manager_host::EventSink;
use tauri::{Emitter, Manager};

struct TauriSink(tauri::AppHandle);

impl EventSink for TauriSink {
    fn emit(&self, event: &str, payload: &serde_json::Value) {
        let _ = self.0.emit(event, payload);
    }
}

/// 启动桌面应用。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::boot().unwrap_or_else(|err| {
            eprintln!("无法启动本机 Host：{err}");
            std::process::exit(1);
        }))
        .setup(|app| {
            let state: tauri::State<'_, AppState> = app.state();
            state
                .runtime
                .add_event_sink(Arc::new(TauriSink(app.handle().clone())));
            skill_manager_host::pair::resume_if_paired(&state.runtime);
            let http_runtime = state.runtime.clone();
            std::thread::spawn(move || {
                if let Err(err) = skill_manager_host::serve::run_http_blocking(
                    http_runtime,
                    skill_manager_host::serve::default_bind(),
                ) {
                    eprintln!("Host HTTP 退出：{err}");
                }
            });
            // 启动时若已记住 SkillManagerVault，接入后即开始监听外部改动。
            // 监听启动失败不影响库的使用（各命令都会重新读取库文件），只记录原因。
            if let Err(err) = state.watch_vault(app.handle()) {
                eprintln!("启动库文件监听失败：{err}");
            }

            // 诊断：先确定日志目录并安装 panic hook，崩溃线索落到真实文件（不自动上传）。
            let log_dir = match state.get() {
                Ok(ctx) => skill_manager_core::diagnostics::log_dir(ctx.state()),
                Err(_) => PathBuf::new(),
            };
            diagnostics::init(log_dir);

            // 系统托盘与关窗行为（FR-052）：关窗收托盘，仅托盘“退出”才终止。
            if let Some(window) = app.get_webview_window("main") {
                tray::intercept_close(&window);
            }
            if let Err(err) = tray::install(app.handle()) {
                eprintln!("安装系统托盘失败：{err}");
            }

            // 启动执行一次统一调度（上游检查 + 库拉取），并在后台按“自动检查频率”定时执行。
            // 后台只拉取、不提交不推送（FR-029/FR-037/FR-038）；隐藏到托盘后该调度继续运行。
            spawn_scheduler(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::host_invoke::host_invoke])
        .run(tauri::generate_context!())
        .expect("启动 Skill Manager 桌面应用失败");
}

/// 后台统一调度：启动执行一次，此后每 60 秒检查是否到达“自动检查频率”。
///
/// 关闭定时（频率 0）或未到周期时跳过；只做上游检查与库拉取，不提交、不推送。
/// 窗口隐藏到托盘不影响该线程，因此“托盘隐藏后检查仍运行”。
fn spawn_scheduler(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        run_schedule(&app, skill_manager_core::Trigger::Startup);
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
            run_schedule(&app, skill_manager_core::Trigger::Timer);
        }
    });
}

fn run_schedule(app: &tauri::AppHandle, trigger: skill_manager_core::Trigger) {
    if let Err(err) = commands::run_schedule(app, trigger) {
        eprintln!("统一调度失败：{err}");
    }
}
