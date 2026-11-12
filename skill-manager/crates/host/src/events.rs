//! Host 向界面推送事件：桌面走 Tauri，无窗口走 SSE。core 不依赖任何入口。

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;

/// 库内变化事件名（与现有桌面一致）。
pub const VAULT_CHANGED_EVENT: &str = "vault-changed";
/// 调度完成事件名。
pub const SCHEDULE_FINISHED_EVENT: &str = "schedule-finished";
/// 仓库获取进度事件名。
pub const FETCH_PROGRESS_EVENT: &str = "skill-fetch-progress";
/// 内置 Agent 触发的破坏性确认计划，须由图形界面确认。
pub const HOST_PLAN_EVENT: &str = "host-plan";
/// ACP `session/update` 的助手可见文本块。
pub const AGENT_MESSAGE_CHUNK_EVENT: &str = "agent_message_chunk";

/// 一条发给界面的事件。
#[derive(Clone, Debug)]
pub struct HostEvent {
    pub name: String,
    pub payload: Value,
}

/// 入口层事件槽。Host 只调用这个接口，不引用 Tauri。
pub trait EventSink: Send + Sync {
    fn emit(&self, event: &str, payload: &Value);
}

/// 不向任何界面推送。
pub struct NoopSink;

impl EventSink for NoopSink {
    fn emit(&self, _event: &str, _payload: &Value) {}
}

/// 广播给 SSE / 长连接订阅者。
#[derive(Clone)]
pub struct BroadcastSink {
    tx: broadcast::Sender<HostEvent>,
}

impl BroadcastSink {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<HostEvent> {
        self.tx.subscribe()
    }

    pub fn sender(&self) -> broadcast::Sender<HostEvent> {
        self.tx.clone()
    }
}

impl EventSink for BroadcastSink {
    fn emit(&self, event: &str, payload: &Value) {
        let _ = self.tx.send(HostEvent {
            name: event.to_string(),
            payload: payload.clone(),
        });
    }
}

/// 多个入口同时接收（例如桌面 Tauri + 本机 HTTP）。
pub struct FanoutSink {
    sinks: Vec<Arc<dyn EventSink>>,
}

impl FanoutSink {
    pub fn new(sinks: Vec<Arc<dyn EventSink>>) -> Self {
        Self { sinks }
    }
}

impl EventSink for FanoutSink {
    fn emit(&self, event: &str, payload: &Value) {
        for sink in &self.sinks {
            sink.emit(event, payload);
        }
    }
}
