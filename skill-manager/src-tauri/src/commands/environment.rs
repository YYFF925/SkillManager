//! 本机环境与诊断只读命令：当前系统、诊断报告与导出、技能库容量报告。
//!
//! Agent 名单与本机状态由 `commands::agent` 提供；业务计算全部在 core。
//! 诊断资料由用户主动获取，不自动上传；导出写出真实文件并返回其路径。

use skill_manager_core::diagnostics::{DiagnosticsExport, DiagnosticsReport};
use skill_manager_core::vault::capacity::CapacityReport;

use super::AppState;

/// 当前操作系统标识；取值与 `.manager/meta.yaml` 的 `oses` 一致。
#[tauri::command]
pub fn environment_os() -> &'static str {
    skill_manager_core::deploy::current_os()
}

/// 诊断报告：版本、平台、有关路径、最近日志与崩溃记录（已脱敏）。
#[tauri::command]
pub fn environment_diagnostics(
    state: tauri::State<'_, AppState>,
    recent: Option<usize>,
) -> Result<DiagnosticsReport, String> {
    let ctx = state.get()?;
    skill_manager_core::diagnostics::report(&ctx, recent.unwrap_or(100))
        .map_err(|err| err.to_string())
}

/// 导出诊断资料到本机日志目录下的真实文件，返回文件路径。
#[tauri::command]
pub fn environment_diagnostics_export(
    state: tauri::State<'_, AppState>,
    recent: Option<usize>,
) -> Result<DiagnosticsExport, String> {
    let ctx = state.get()?;
    skill_manager_core::diagnostics::export(&ctx, recent.unwrap_or(200))
        .map_err(|err| err.to_string())
}

/// 技能库容量报告：总占用与较大技能排行（按当前库文件，排除 `.git`）。
#[tauri::command]
pub fn environment_capacity(
    state: tauri::State<'_, AppState>,
    top: Option<usize>,
) -> Result<CapacityReport, String> {
    let ctx = state.get()?;
    let root = ctx.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    skill_manager_core::vault::capacity::scan(&root, top.unwrap_or(10))
        .map_err(|err| err.to_string())
}
