//! 系统托盘与关窗行为（T084，FR-052）。
//!
//! 正常关窗只隐藏窗口并继续后台运行；托盘提供「重新打开窗口 / 检查上游更新 / 退出」，
//! 只有「退出」才终止进程。业务检查仍调用 core 的统一调度（与启动/定时同一入口），
//! 不恢复旧版的托盘预设子菜单，也不新增托盘专属业务规则。
//!
//! 采用 Tauri 2 官方 API：`tauri::tray::TrayIconBuilder`、`tauri::menu::*` 与
//! `WebviewWindow::on_window_event`（核对 tauri 2.12）。托盘菜单是唯一入口：左键点击图标
//! 即展开菜单（Tauri 2 默认），不另建左键快速恢复等额外分支。

use tauri::menu::MenuBuilder;
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime, WindowEvent};

/// 托盘菜单项标识。
const MENU_SHOW: &str = "skill-manager-show";
const MENU_CHECK: &str = "skill-manager-check-updates";
const MENU_QUIT: &str = "skill-manager-quit";

/// 主窗口标签（与 `tauri.conf.json` 的窗口 label 一致）。
const MAIN_WINDOW: &str = "main";

/// 安装托盘图标与菜单；失败时返回原因，由调用方记录。
pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text(MENU_SHOW, "重新打开窗口")
        .text(MENU_CHECK, "检查上游更新")
        .separator()
        .text(MENU_QUIT, "退出")
        .build()?;

    let mut builder = TrayIconBuilder::with_id("skill-manager-tray")
        .menu(&menu)
        .tooltip("Skill Manager")
        .on_menu_event(|app, event| handle_menu(app, event.id().0.as_str()));

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

/// 拦截关闭请求：正常关窗只隐藏窗口，后台调度继续运行。
pub fn intercept_close<R: Runtime>(window: &tauri::WebviewWindow<R>) {
    let handle = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = handle.hide();
        }
    });
}

fn handle_menu<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        MENU_SHOW => show_main_window(app),
        MENU_CHECK => check_updates(app),
        MENU_QUIT => app.exit(0),
        _ => {}
    }
}

/// 显示并聚焦主窗口。
fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// 托盘「检查上游更新」：直接调用 core 的启动级调度（不打开主窗口）。
///
/// 与启动/定时同一 `scheduler::run` 入口，后台只检查与拉取，不提交、不推送；
/// 结果显示在运行日志与下次打开窗口后的状态中，不在此处另建托盘专属业务逻辑。
fn check_updates<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    std::thread::spawn(move || {
        let state: tauri::State<'_, crate::commands::AppState> = app.state();
        let Ok(ctx) = state.get() else {
            return;
        };
        match skill_manager_core::scheduler::run(&ctx, skill_manager_core::Trigger::Manual) {
            Ok(report) => {
                let _ = skill_manager_core::diagnostics::record(
                    ctx.state(),
                    "info",
                    "tray.check",
                    &format!(
                        "托盘检查完成：上游 {}，库拉取 {}",
                        report.upstream.status, report.pull.status
                    ),
                );
            }
            Err(err) => {
                let _ = skill_manager_core::diagnostics::record(
                    ctx.state(),
                    "error",
                    "tray.check",
                    &err.to_string(),
                );
            }
        }
    });
}
