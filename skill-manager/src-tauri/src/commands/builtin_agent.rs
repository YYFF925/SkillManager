//! 内置管理 Agent 的桌面入口：经 Host runtime 转发，不直打 core。

use serde_json::{json, Value};

use super::AppState;

#[tauri::command]
pub fn agent_set_context(
    state: tauri::State<'_, AppState>,
    session_id: String,
    view: Value,
) -> Result<Value, String> {
    skill_manager_host::agent::set_context(
        &state.runtime,
        json!({ "sessionId": session_id, "view": view }),
    )
}

#[tauri::command]
pub fn agent_post_message(
    state: tauri::State<'_, AppState>,
    session_id: String,
    text: String,
) -> Result<Value, String> {
    skill_manager_host::agent::post_message(
        &state.runtime,
        json!({ "sessionId": session_id, "text": text }),
    )
}

#[tauri::command]
pub fn agent_cancel_task(
    state: tauri::State<'_, AppState>,
    task_id: String,
    session_id: Option<String>,
) -> Result<Value, String> {
    skill_manager_host::agent::cancel_task(
        &state.runtime,
        &task_id,
        json!({ "sessionId": session_id, "taskId": task_id }),
    )
}

#[tauri::command]
pub fn agent_list_messages(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> Result<Value, String> {
    skill_manager_host::agent::list_messages(&state.runtime, &session_id)
}

#[tauri::command]
pub fn agent_list_sessions(state: tauri::State<'_, AppState>) -> Result<Value, String> {
    skill_manager_host::agent::list_sessions(&state.runtime)
}
