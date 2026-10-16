//! T094：Hermes 根解析与诊断日志目录清理的 core 断言。
//!
//! 覆盖：Hermes 根按 `HERMES_HOME` 环境变量与平台默认解析、**不读**可写设置键；
//! 自定义日志目录键移除后诊断日志仍固定落在 `<state-dir>/logs`。
//!
//! 环境变量是进程级状态：本文件的用例共用一把锁串行设置/清除 `HERMES_HOME`，
//! 且只做路径解析，不写入任何真实 Hermes 根。

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use skill_manager_core::deploy::TargetResolver;
use skill_manager_core::domain::Target;
use skill_manager_core::hermes::HermesService;
use skill_manager_core::{AppContext, StateStore};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn scheduler_runs_all_three_tasks_and_timer_respects_disabled_setting() {
    use skill_manager_core::scheduler::{run, Trigger};
    let root = tempfile::TempDir::new().unwrap();
    let vault = root.path().join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    assert!(std::process::Command::new("git").args(["init", "--quiet"]).arg(&vault).status().unwrap().success());
    let tool = vault.join("tools/fixture");
    std::fs::create_dir_all(&tool).unwrap();
    std::fs::write(tool.join("tool.yaml"), "name: fixture\ncheck_command: echo command-failed >&2; exit 1\n").unwrap();
    let ctx = AppContext::open_with_vault(&vault, Some(root.path().join("state"))).unwrap();
    ctx.state().set_auto_check_minutes(0).unwrap();
    for trigger in [Trigger::Startup, Trigger::Manual] {
        let report = run(&ctx, trigger).unwrap();
        assert!(report.upstream.executed && report.pull.executed && report.tool_check.executed);
        assert!(report.upstream_report.is_some());
        assert_eq!(report.pull.status, "error");
        assert_eq!(report.tool_check.status, "error");
        assert!(report.tool_check.detail.unwrap().contains("command-failed"));
    }
    let timer = run(&ctx, Trigger::Timer).unwrap();
    assert!(timer.skipped_by_interval);
    assert!(!timer.upstream.executed && !timer.pull.executed && !timer.tool_check.executed);
    ctx.state().set_auto_check_minutes(30).unwrap();
    ctx.state().set_last_check_at("2000-01-01T00:00:00Z").unwrap();
    let timer = run(&ctx, Trigger::Timer).unwrap();
    assert!(timer.upstream.executed && timer.pull.executed && timer.tool_check.executed);
}

#[test]
fn shared_directory_is_derived_from_home_and_ignores_obsolete_setting() {
    let state_dir = tempfile::TempDir::new().unwrap();
    let ctx = AppContext::open(Some(state_dir.path().to_path_buf())).unwrap();
    ctx.state().set_setting("shared_dir", "/tmp/obsolete-shared-path").unwrap();
    let expected = ctx.home().unwrap().join(".agents").join("skills");
    assert_eq!(ctx.shared_dir().unwrap(), expected);
    let resolved = ctx.target_resolver().unwrap().resolve(&Target::Shared).unwrap();
    assert_eq!(PathBuf::from(resolved.path), expected);
}

#[test]
fn hermes_root_follows_env_var_and_ignores_settings_key() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());

    let state_dir = tempfile::TempDir::new().unwrap();
    let custom_root = tempfile::TempDir::new().unwrap();

    // SAFETY: 测试串行持锁，且仅在本进程内设置环境变量。
    unsafe { std::env::set_var("HERMES_HOME", custom_root.path()) };

    let ctx = AppContext::open(Some(state_dir.path().to_path_buf())).unwrap();
    // 写入一个已废弃的可写设置键，解析必须忽略它。
    ctx.state().set_setting("hermes_root", "/tmp/should-be-ignored").unwrap();

    let resolver: TargetResolver<'static> = ctx.target_resolver().unwrap();
    let resolved = resolver.resolve(&Target::HermesDefault).unwrap();
    assert_eq!(
        PathBuf::from(&resolved.path),
        custom_root.path().join("skills"),
        "deploy to hermes:default 应使用 HERMES_HOME 解析"
    );

    let service = HermesService::from_context(&ctx, &resolver).unwrap();
    assert_eq!(service.root(), custom_root.path());
    assert_eq!(service.default_skills_dir(), custom_root.path().join("skills"));

    // SAFETY: 同上传。
    unsafe { std::env::remove_var("HERMES_HOME") };
}

#[test]
fn hermes_root_falls_back_to_platform_default_without_env() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());

    // SAFETY: 测试串行持锁，且仅在本进程内设置环境变量。
    unsafe { std::env::remove_var("HERMES_HOME") };

    let home = Path::new("/tmp/hermes-home-probe");
    let state_dir = tempfile::TempDir::new().unwrap();
    let ctx = AppContext::open(Some(state_dir.path().to_path_buf())).unwrap();
    let resolver = ctx.target_resolver().unwrap();
    // 平台默认为非 Windows 的 `~/.hermes`；此处直接核对解析函数对 home 的取法。
    #[cfg(not(windows))]
    {
        // resolver.home() 是运行环境真实 home；未设 HERMES_HOME 时根为 `<home>/.hermes`。
        let expected = resolver.home().join(".hermes");
        let service = HermesService::from_context(&ctx, &resolver).unwrap();
        assert_eq!(service.root(), expected, "home = {:?}", resolver.home());
    }
    let _ = home;
}

#[test]
fn diagnostics_log_dir_is_fixed_under_state_dir() {
    let state_dir = tempfile::TempDir::new().unwrap();
    let store = StateStore::open(state_dir.path()).unwrap();
    // 已移除的“自定义日志目录”键不再影响落点。
    store.set_setting("log_dir", "/tmp/elsewhere").unwrap();
    assert_eq!(
        skill_manager_core::diagnostics::log_dir(&store),
        state_dir.path().join("logs")
    );

    // 诊断报告同样给出 `<state-dir>/logs` 下的真实路径，而不是残留键的值。
    let ctx = AppContext::open(Some(state_dir.path().to_path_buf())).unwrap();
    let report = skill_manager_core::diagnostics::report(&ctx, 5).unwrap();
    assert_eq!(
        PathBuf::from(report.log_path),
        state_dir.path().join("logs").join("skill-manager.log")
    );
    assert_eq!(
        PathBuf::from(report.crash_path),
        state_dir.path().join("logs").join("crash.log")
    );
}
