//! CLI / Agent skip the GUI confirmation gate; GUI still needs a plan.

use serde_json::json;
use skill_manager_core::AppContext;
use skill_manager_host::invoke::{self, InvokeError, InvokeSource};
use skill_manager_host::HostRuntime;

fn runtime() -> HostRuntime {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::open(Some(dir.path().to_path_buf())).unwrap();
    HostRuntime::from_context(ctx)
}

fn message(err: &InvokeError) -> String {
    match err {
        InvokeError::NeedsPlan(_) => "needs-plan".into(),
        other => other.to_string(),
    }
}

#[test]
fn cli_and_agent_delete_do_not_need_gui_session() {
    let runtime = runtime();
    for source in [InvokeSource::Cli, InvokeSource::Agent] {
        let result = invoke::invoke(
            &runtime,
            "skill_delete",
            json!({ "references": ["code/zzz-nonexistent-probe"] }),
            source,
            None,
        );
        match result {
            Ok(_) => {}
            Err(InvokeError::NeedsPlan(_)) => panic!("{source:?} must not return a confirmation plan"),
            Err(err) => {
                let text = message(&err);
                assert!(
                    !text.contains("界面会话") && !text.contains("needs-plan"),
                    "{source:?} still gated: {text}"
                );
            }
        }
    }
}

#[test]
fn gui_delete_still_needs_plan() {
    let runtime = runtime();
    let err = invoke::invoke(
        &runtime,
        "skill_delete",
        json!({ "references": ["code/zzz-nonexistent-probe"] }),
        InvokeSource::Gui,
        Some("session-a"),
    )
    .unwrap_err();
    assert!(
        matches!(err, InvokeError::NeedsPlan(_)),
        "GUI delete without planId must need confirmation, got {err:?}"
    );
}
