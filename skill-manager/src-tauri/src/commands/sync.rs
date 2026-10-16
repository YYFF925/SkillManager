//! SkillManagerVault 同步命令：拉取、上传、状态与冲突（T054）。
//!
//! 薄封装 core。GUI 保存后按已确认策略调用 `sync_push`；文件监听与 `deploy sync` 不调用它，
//! 后台路径不提交、不推送。`sync_push` 也是 Agent 获明确同意后的显式入口。

use skill_manager_core::sync::{ConflictView, PullReport, PushReport, SyncService, SyncStatus};

use super::AppState;

/// 拉取同一 SkillManagerVault；无冲突静默应用并刷新已有部署/指令目标。
#[tauri::command]
pub fn sync_pull(state: tauri::State<'_, AppState>) -> Result<PullReport, String> {
    let ctx = state.get()?;
    let vault = ctx.vault().map_err(|err| err.to_string())?;
    let home = ctx.home().map_err(|err| err.to_string())?;
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = SyncService::new(vault, home, ctx.catalog(), ctx.state(), &resolver);
    service.pull().map_err(|err| err.to_string())
}

/// 提交并推送本机 SkillManagerVault（工作区全部改动）。
#[tauri::command]
pub fn sync_push(
    state: tauri::State<'_, AppState>,
    message: Option<String>,
) -> Result<PushReport, String> {
    let ctx = state.get()?;
    let vault = ctx.vault().map_err(|err| err.to_string())?;
    let home = ctx.home().map_err(|err| err.to_string())?;
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = SyncService::new(vault, home, ctx.catalog(), ctx.state(), &resolver);
    service
        .push(message.as_deref())
        .map_err(|err| err.to_string())
}

/// 只读同步状态。
#[tauri::command]
pub fn sync_status(state: tauri::State<'_, AppState>) -> Result<SyncStatus, String> {
    let ctx = state.get()?;
    let vault = ctx.vault().map_err(|err| err.to_string())?;
    let home = ctx.home().map_err(|err| err.to_string())?;
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = SyncService::new(vault, home, ctx.catalog(), ctx.state(), &resolver);
    service.status().map_err(|err| err.to_string())
}

/// Git 冲突信息（供 Agent 检查）。
#[tauri::command]
pub fn sync_conflicts(state: tauri::State<'_, AppState>) -> Result<ConflictView, String> {
    let ctx = state.get()?;
    let vault = ctx.vault().map_err(|err| err.to_string())?;
    let home = ctx.home().map_err(|err| err.to_string())?;
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = SyncService::new(vault, home, ctx.catalog(), ctx.state(), &resolver);
    service.conflicts().map_err(|err| err.to_string())
}
