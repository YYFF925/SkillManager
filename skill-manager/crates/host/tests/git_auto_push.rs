//! Agent 不得写 git_auto_push；GUI 可以。

use serde_json::json;
use skill_manager_core::AppContext;
use skill_manager_host::invoke::{self, InvokeSource};
use skill_manager_host::HostRuntime;

#[test]
fn agent_cannot_set_git_auto_push() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::open(Some(dir.path().to_path_buf())).unwrap();
    let runtime = HostRuntime::from_context(ctx);
    let err = invoke::invoke(
        &runtime,
        "settings_set_git_auto_push",
        json!({ "enabled": true }),
        InvokeSource::Agent,
        None,
    )
    .unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains("图形界面") || message.contains("自动上传"),
        "{message}"
    );
}

#[test]
fn gui_can_set_git_auto_push() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = AppContext::open(Some(dir.path().to_path_buf())).unwrap();
    let runtime = HostRuntime::from_context(ctx);
    invoke::invoke(
        &runtime,
        "settings_set_git_auto_push",
        json!({ "enabled": true }),
        InvokeSource::Gui,
        None,
    )
    .expect("GUI may write git_auto_push");
}
