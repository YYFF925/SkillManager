//! Tauri 2 桌面壳：持有 Host runtime；command 层转发到 core，不承载业务规则。

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
        .invoke_handler(tauri::generate_handler![
            commands::vault::vault_open,
            commands::vault::vault_current,
            commands::environment::environment_os,
            commands::environment::environment_check,
            commands::environment::environment_schedule_current,
            commands::environment::environment_diagnostics,
            commands::environment::environment_diagnostics_export,
            commands::environment::environment_capacity,
            commands::skill::skill_list,
            commands::skill::skill_get,
            commands::skill::skill_read,
            commands::skill::skill_file_read,
            commands::skill::skill_set,
            commands::skill::skill_tag_add,
            commands::skill::skill_tag_detach,
            commands::skill::skill_tag_rename,
            commands::skill::skill_tag_remove,
            commands::skill::skill_move,
            commands::skill::skill_category_rename,
            commands::skill::skill_archive_list,
            commands::skill::skill_archive,
            commands::skill::skill_archive_restore,
            commands::skill::skill_archive_delete,
            commands::skill::skill_translation_get,
            commands::skill::skill_translation_set,
            commands::skill::skill_translation_remove,
            commands::skill::skill_overlay_list,
            commands::skill::skill_overlay_get,
            commands::skill::skill_overlay_set,
            commands::skill::skill_overlay_remove,
            commands::deploy::deploy_to,
            commands::deploy::deploy_undo,
            commands::deploy::deploy_sync,
            commands::deploy::deploy_status,
            commands::deploy::skill_delete,
            commands::preset::preset_list,
            commands::preset::preset_get,
            commands::preset::preset_create,
            commands::preset::preset_delete,
            commands::preset::preset_rename,
            commands::preset::preset_set_icon,
            commands::preset::preset_reorder,
            commands::preset::preset_members_add,
            commands::preset::preset_members_remove,
            commands::preset::preset_reorder_members,
            commands::preset::deploy_preset,
            commands::preset::deploy_undo_preset,
            commands::agent::agent_list,
            commands::agent::agent_detect,
            commands::agent::agent_enable,
            commands::agent::agent_disable,
            commands::agent::agent_reorder,
            commands::agent::agent_set_dir,
            commands::agent::agent_reset_dir,
            commands::agent::agent_local_skills,
            commands::agent::agent_read_local,
            commands::agent::agent_delete_local,
            commands::agent::shared_dir_get,
            commands::hermes::hermes_profiles,
            commands::hermes::hermes_list,
            commands::hermes::hermes_read,
            commands::hermes::hermes_enable,
            commands::hermes::hermes_disable,
            commands::hermes::hermes_adopt,
            commands::acquire::skill_preview,
            commands::acquire::skill_import,
            commands::acquire::skill_fetch_cancel,
            commands::acquire::skill_adopt,
            commands::acquire::skill_create,
            commands::acquire::skill_copy,
            commands::upstream::skill_check,
            commands::upstream::skill_update,
            commands::upstream::skill_update_repo,
            commands::upstream::skill_reject_new,
            commands::upstream::skill_rejected,
            commands::upstream::skill_skip,
            commands::upstream::skill_unskip,
            commands::upstream::skill_skipped,
            commands::sync::sync_pull,
            commands::sync::sync_push,
            commands::sync::sync_status,
            commands::sync::sync_conflicts,
            commands::agent::instruction_get,
            commands::agent::instruction_set,
            commands::settings::settings_get,
            commands::settings::settings_set_theme,
            commands::settings::settings_set_font_scale,
            commands::settings::settings_set_auto_check,
            commands::settings::settings_set_git_auto_push,
            commands::skill::skill_task,
            commands::tool::tool_list,
            commands::tool::tool_get,
            commands::tool::tool_check,
            commands::tool::tool_task,
            commands::tool::tool_skill_candidates,
            commands::builtin_agent::agent_set_context,
            commands::builtin_agent::agent_post_message,
            commands::builtin_agent::agent_cancel_task,
            commands::builtin_agent::agent_list_messages,
            commands::builtin_agent::agent_list_sessions,
        ])
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
