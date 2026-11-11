//! 当前设备转发：invoke 与 SSE 转到在线 Host。离线不得用缓存正文。

use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::devices::{InvokeReply, InvokeReq, LiveMap};

pub async fn invoke(
    live: &LiveMap,
    device_id: &str,
    command: &str,
    body: Value,
    session_id: Option<String>,
) -> Result<InvokeReply, String> {
    let link = live
        .get(device_id)
        .filter(|_| live.is_online(device_id))
        .ok_or_else(|| format!("设备 {device_id} 离线，无法操作。不会用缓存技能内容顶上。"))?;
    let (tx, rx) = oneshot::channel();
    let req = InvokeReq {
        id: Uuid::new_v4().to_string(),
        command: command.to_string(),
        body,
        session_id,
        reply: tx,
    };
    link.invoke_tx
        .send(req)
        .map_err(|_| "设备连接已断开".to_string())?;
    tokio::time::timeout(Duration::from_secs(600), rx)
        .await
        .map_err(|_| "等待设备响应超时".to_string())?
        .map_err(|_| "设备未返回结果".to_string())
}

/// 把 Host 的 invoke_result 还原成 HTTP 状态：确认计划必须是 409，不得当 200。
pub fn reply_from_host_message(value: &Value) -> InvokeReply {
    let body = value.get("body").cloned().unwrap_or(json!(null));
    let mut status = value
        .get("status")
        .and_then(Value::as_u64)
        .map(|n| n as u16)
        .unwrap_or_else(|| {
            if value.get("ok").and_then(Value::as_bool).unwrap_or(true) {
                200
            } else {
                400
            }
        });
    if body
        .get("needsConfirmation")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && body.get("planId").is_some()
    {
        status = 409;
    }
    InvokeReply { status, body }
}

pub fn subscribe(
    live: &LiveMap,
    device_id: &str,
) -> Result<tokio::sync::broadcast::Receiver<Value>, String> {
    let link = live
        .get(device_id)
        .filter(|_| live.is_online(device_id))
        .ok_or_else(|| format!("设备 {device_id} 离线，无法订阅事件。"))?;
    Ok(link.events.subscribe())
}

pub fn offline_error(device_id: Option<&str>) -> Value {
    match device_id {
        Some(id) => json!({ "error": format!("设备 {id} 离线或未选择，无法操作") }),
        None => json!({ "error": "尚未选择当前设备" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_plan_is_http_409_even_if_host_omits_status() {
        let reply = reply_from_host_message(&json!({
            "type": "invoke_result",
            "ok": true,
            "body": {
                "needsConfirmation": true,
                "planId": "plan-1",
                "action": "skill.delete",
                "preview": { "references": ["code/demo"] }
            }
        }));
        assert_eq!(reply.status, 409);
        assert_eq!(reply.body["planId"], "plan-1");
    }

    #[test]
    fn host_status_is_preserved_when_present() {
        let reply = reply_from_host_message(&json!({
            "type": "invoke_result",
            "ok": false,
            "status": 409,
            "body": { "needsConfirmation": true, "planId": "p2" }
        }));
        assert_eq!(reply.status, 409);
    }
}
