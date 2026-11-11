//! 桌面 IPC 薄壳：全部业务走 Host invoke，确认计划绑定桌面会话。

use serde_json::{Value, json};
use skill_manager_host::{InvokeError, InvokeSource};

use super::AppState;

#[tauri::command]
pub fn host_invoke(
    state: tauri::State<'_, AppState>,
    command: String,
    args: Option<Value>,
    session_id: Option<String>,
) -> Result<Value, String> {
    let body = args.unwrap_or(json!({}));
    match skill_manager_host::invoke::invoke(
        &state.runtime,
        &command,
        body,
        InvokeSource::Gui,
        session_id.as_deref(),
    ) {
        Ok(data) => Ok(json!({ "ok": true, "status": 200, "data": data })),
        Err(InvokeError::NeedsPlan(plan)) => {
            Ok(json!({ "ok": false, "status": 409, "data": plan }))
        }
        Err(err) => {
            let (status, body) = err.status_and_body();
            if status == 409 {
                Ok(json!({ "ok": false, "status": 409, "data": body }))
            } else {
                Err(body
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("invoke 失败")
                    .to_string())
            }
        }
    }
}
