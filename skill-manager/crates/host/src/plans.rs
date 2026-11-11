//! 破坏性确认计划：删除技能、覆盖部署。绑定签发时的 session_id。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use uuid::Uuid;

use crate::HostRuntime;

const PLAN_TTL: Duration = Duration::from_secs(15 * 60);

/// 须走确认计划的动作。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DestructiveAction {
    SkillDelete,
    DeployOverwrite,
}

impl DestructiveAction {
    pub fn as_str(self) -> &'static str {
        match self {
            DestructiveAction::SkillDelete => "skill.delete",
            DestructiveAction::DeployOverwrite => "deploy.overwrite",
        }
    }
}

#[derive(Clone, Debug)]
struct Plan {
    plan_id: String,
    session_id: String,
    action: DestructiveAction,
    payload: Value,
    expires_at: u64,
    confirmed: bool,
    state_dir: String,
}

fn store() -> &'static Mutex<HashMap<String, Plan>> {
    static STORE: OnceLock<Mutex<HashMap<String, Plan>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 签发计划并返回给界面的 JSON。
pub fn issue(
    runtime: &HostRuntime,
    session_id: Option<&str>,
    action: DestructiveAction,
    payload: &Value,
) -> Result<Value, String> {
    let session_id = session_id
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "破坏性操作需要有效的界面会话".to_string())?
        .to_string();
    let plan_id = Uuid::new_v4().to_string();
    let expires_at = now_secs() + PLAN_TTL.as_secs();
    let preview = payload.clone();
    let plan = Plan {
        plan_id: plan_id.clone(),
        session_id: session_id.clone(),
        action,
        payload: payload.clone(),
        expires_at,
        confirmed: false,
        state_dir: runtime.state_dir().display().to_string(),
    };
    store()
        .lock()
        .map_err(|_| "确认计划锁已损坏".to_string())?
        .insert(plan_id.clone(), plan);
    Ok(json!({
        "needsConfirmation": true,
        "planId": plan_id,
        "action": action.as_str(),
        "preview": preview,
        "expiresAt": expires_at,
        "sessionId": session_id,
    }))
}

/// 确认计划。必须是签发该计划的 session_id。
pub fn confirm(runtime: &HostRuntime, plan_id: &str, session_id: Option<&str>) -> Result<Value, String> {
    let session_id = session_id.unwrap_or("");
    let mut guard = store().lock().map_err(|_| "确认计划锁已损坏".to_string())?;
    let plan = guard
        .get_mut(plan_id)
        .ok_or_else(|| "确认计划不存在或已过期".to_string())?;
    if plan.state_dir != runtime.state_dir().display().to_string() {
        return Err("确认计划不属于当前 Host".to_string());
    }
    if now_secs() > plan.expires_at {
        return Err("确认计划已过期".to_string());
    }
    if plan.session_id != session_id {
        return Err("确认计划不属于当前界面会话".to_string());
    }
    plan.confirmed = true;
    Ok(json!({
        "planId": plan.plan_id,
        "action": plan.action.as_str(),
        "confirmed": true,
    }))
}

/// invoke 在带 planId 执行前调用：未确认、过期或会话不匹配则拒绝。
pub fn assert_confirmed(
    runtime: &HostRuntime,
    plan_id: &str,
    session_id: Option<&str>,
    action: DestructiveAction,
) -> Result<(), String> {
    let session_id = session_id.unwrap_or("");
    let guard = store().lock().map_err(|_| "确认计划锁已损坏".to_string())?;
    let plan = guard
        .get(plan_id)
        .ok_or_else(|| "确认计划不存在或已过期".to_string())?;
    if plan.state_dir != runtime.state_dir().display().to_string() {
        return Err("确认计划不属于当前 Host".to_string());
    }
    if now_secs() > plan.expires_at {
        return Err("确认计划已过期".to_string());
    }
    if plan.session_id != session_id {
        return Err("确认计划不属于当前界面会话".to_string());
    }
    if plan.action != action {
        return Err("确认计划动作与本次操作不符".to_string());
    }
    if !plan.confirmed {
        return Err("确认计划尚未确认".to_string());
    }
    Ok(())
}

/// 创建界面会话。
pub fn new_session() -> String {
    Uuid::new_v4().to_string()
}
