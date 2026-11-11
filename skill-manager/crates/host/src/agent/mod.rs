//! 内置管理 Agent 桥：ACP stdio、工具、两层上下文。浏览器不跑模型。

mod acp;
mod bridge;
mod context;
mod tools;

pub use bridge::{cancel_task, detect_and_lock, post_message};
pub use context::{list_messages, list_sessions, set_context};
pub use tools::builtin_skill_dir;

use crate::HostRuntime;
use serde_json::Value;

pub fn post_message_http(runtime: &HostRuntime, body: Value) -> Result<Value, String> {
    post_message(runtime, body)
}

pub fn set_context_http(runtime: &HostRuntime, body: Value) -> Result<Value, String> {
    set_context(runtime, body)
}

pub fn cancel_task_http(
    runtime: &HostRuntime,
    task_id: &str,
    body: Value,
) -> Result<Value, String> {
    cancel_task(runtime, task_id, body)
}
