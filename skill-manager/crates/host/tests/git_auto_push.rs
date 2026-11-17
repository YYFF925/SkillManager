//! GUI, Agent, and CLI may write git_auto_push.

use serde_json::json;
use skill_manager_core::AppContext;
use skill_manager_host::invoke::{self, InvokeSource};
use skill_manager_host::HostRuntime;

fn runtime() -> HostRuntime {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::open(Some(dir.path().to_path_buf())).unwrap();
    HostRuntime::from_context(ctx)
}

#[test]
fn agent_can_set_git_auto_push() {
    let runtime = runtime();
    invoke::invoke(
        &runtime,
        "settings_set_git_auto_push",
        json!({ "enabled": true }),
        InvokeSource::Agent,
        None,
    )
    .expect("Agent may write git_auto_push");
}

#[test]
fn gui_can_set_git_auto_push() {
    let runtime = runtime();
    invoke::invoke(
        &runtime,
        "settings_set_git_auto_push",
        json!({ "enabled": true }),
        InvokeSource::Gui,
        None,
    )
    .expect("GUI may write git_auto_push");
}

#[test]
fn cli_can_set_git_auto_push() {
    let runtime = runtime();
    invoke::invoke(
        &runtime,
        "settings_set_git_auto_push",
        json!({ "enabled": true }),
        InvokeSource::Cli,
        None,
    )
    .expect("CLI may write git_auto_push");
}
