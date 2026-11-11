//! 确认计划绑定签发时的 session_id；其它会话不得确认。

use serde_json::json;
use skill_manager_core::AppContext;
use skill_manager_host::plans::{self, DestructiveAction};
use skill_manager_host::HostRuntime;

#[test]
fn other_session_cannot_confirm() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::open(Some(dir.path().to_path_buf())).unwrap();
    let runtime = HostRuntime::from_context(ctx);
    let plan = plans::issue(
        &runtime,
        Some("session-a"),
        DestructiveAction::SkillDelete,
        &json!({"references": ["code/demo"]}),
    )
    .unwrap();
    let plan_id = plan.get("planId").and_then(|v| v.as_str()).unwrap();
    let denied = plans::confirm(&runtime, plan_id, Some("session-b"));
    assert!(denied.is_err(), "other session must be rejected");
    plans::confirm(&runtime, plan_id, Some("session-a")).expect("owner can confirm");
    plans::assert_confirmed(
        &runtime,
        plan_id,
        Some("session-a"),
        DestructiveAction::SkillDelete,
    )
    .expect("confirmed plan is usable");
    let mismatch = plans::assert_confirmed(
        &runtime,
        plan_id,
        Some("session-b"),
        DestructiveAction::SkillDelete,
    );
    assert!(mismatch.is_err());
}
