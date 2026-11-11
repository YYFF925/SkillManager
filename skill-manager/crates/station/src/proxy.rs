//! 当前设备转发：invoke 与 SSE 转到在线 Host。离线不得用缓存正文。

use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::devices::{InvokeReq, LiveMap};

pub async fn invoke(
    live: &LiveMap,
    device_id: &str,
    command: &str,
    body: Value,
    session_id: Option<String>,
) -> Result<Value, String> {
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

pub fn subscribe(live: &LiveMap, device_id: &str) -> Result<tokio::sync::broadcast::Receiver<Value>, String> {
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
