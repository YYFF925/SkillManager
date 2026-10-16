//! US10 增量（T093）：`settings` 命令组的 CLI 断言（quickstart 5.14～5.19）。
//!
//! 全程使用 `TestEnv` 的临时 HOME + `--vault` + `--state-dir`，不触碰真实 `~/.codex`
//! /`~/.zcode`/`~/.hermes`/`~/.agents` 与真实 Application Support。
//!
//! 覆盖：`settings get` 的默认值推导与 null 语义；共享目录自动识别与 `shared` 部署联动；
//! 自动检查频率取值校验；主题/字号/上传授权无 CLI 写入口；诊断导出真实文件；容量空库排行。

mod common;

use common::TestEnv;
use std::path::Path;

/// `settings get` 不带参数应返回的键集合（不含本次移除的 `hermes_root`/`log_dir`）。
const ALL_KEYS: [&str; 8] = [
    "theme",
    "font_scale",
    "auto_check_minutes",
    "shared_dir",
    "git_auto_push",
    "vault_path",
    "last_sync_at",
    "last_check_at",
];

#[test]
fn get_returns_effective_values_with_null_for_unset() {
    let env = TestEnv::new();
    let data = env.cli_json(&["settings", "get"]);

    for key in ALL_KEYS {
        assert!(
            data.get(key).is_some(),
            "settings get 缺少键 {key}（未设置的键也不能省略）：{data}"
        );
    }
    assert_eq!(data["theme"], "system");
    assert_eq!(data["font_scale"], "default");
    assert_eq!(data["auto_check_minutes"], 60);
    // 共享目录始终按临时 HOME 自动解析，不依赖可写设置。
    assert_eq!(data["shared_dir"], env.home.join(".agents/skills").to_string_lossy().as_ref());
    assert_eq!(data["git_auto_push"], false);
    assert!(data["last_sync_at"].is_null(), "{data}");
    assert!(data["last_check_at"].is_null(), "{data}");
    let vault = data["vault_path"].as_str().expect("vault_path 应为已接入库");
    assert!(Path::new(vault).is_absolute(), "{vault}");
}

#[test]
fn get_with_keys_returns_only_requested_keys() {
    let env = TestEnv::new();
    let data = env.cli_json(&["settings", "get", "theme", "shared_dir"]);
    let object = data.as_object().expect("应为对象");
    assert_eq!(object.len(), 2, "{data}");
    assert_eq!(data["theme"], "system");
    assert_eq!(data["shared_dir"], env.home.join(".agents/skills").to_string_lossy().as_ref());
}

#[test]
fn get_does_not_return_removed_keys() {
    let env = TestEnv::new();
    let data = env.cli_json(&["settings", "get"]);
    assert!(data.get("hermes_root").is_none(), "{data}");
    assert!(data.get("log_dir").is_none(), "{data}");
}

#[test]
fn shared_deploy_uses_home_without_path_configuration() {
    let env = TestEnv::new();
    env.write_skill("code", None, "alpha", "# alpha\n");
    let shared = env.home.join(".agents/skills");
    let data = env.cli_json(&["settings", "get", "shared_dir"]);
    assert_eq!(data["shared_dir"], shared.to_string_lossy().as_ref());
    assert!(!shared.exists(), "reading settings must not create the directory");
    let deployed = env.cli_json(&["deploy", "to", "shared", "code/alpha"]);
    assert_eq!(deployed["items"].as_array().map(Vec::len), Some(1));
    assert!(shared.join("alpha/SKILL.md").exists());
    let removed = env.cli(&["settings", "set-shared-dir", "--clear"]);
    assert_eq!(removed.status.code(), Some(2), "obsolete write command must be removed");
}

#[test]
fn set_auto_check_accepts_allowed_and_rejects_other() {
    let env = TestEnv::new();
    for minutes in [0, 30, 60, 360, 1440] {
        let arg = minutes.to_string();
        let written = env.cli_json(&["settings", "set-auto-check", &arg]);
        assert_eq!(written["auto_check_minutes"], minutes);

        let read_back = env.cli_json(&["settings", "get", "auto_check_minutes"]);
        assert_eq!(read_back["auto_check_minutes"], minutes);
    }

    let (code, value) = env.cli_json_raw(&["settings", "set-auto-check", "45"]);
    assert_eq!(code, 1, "{value}");
    let message = value["errors"][0]["message"].as_str().unwrap_or("");
    assert!(message.contains("0") && message.contains("1440"), "{message}");
}

#[test]
fn no_cli_write_entry_for_theme_font_scale_and_auto_push() {
    let env = TestEnv::new();
    for args in [
        ["settings", "set-theme", "dark"],
        ["settings", "set-font-scale", "large"],
        ["settings", "set-git-auto-push", "true"],
    ] {
        let output = env.cli(&args);
        assert!(
            !output.status.success(),
            "不应存在 CLI 写入口 {args:?}：stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // 三者仍可经 settings get 读取。
    let data = env.cli_json(&["settings", "get", "theme", "font_scale", "git_auto_push"]);
    assert_eq!(data["theme"], "system");
    assert_eq!(data["font_scale"], "default");
    assert_eq!(data["git_auto_push"], false);
}

#[test]
fn diagnostics_reports_real_paths_and_exports_file() {
    let env = TestEnv::new();
    let report = env.cli_json(&["settings", "diagnostics"]);
    assert!(report["version"].as_str().unwrap_or("").contains('.'), "{report}");
    assert!(!report["os"].as_str().unwrap_or("").is_empty(), "{report}");
    let log_path = report["log_path"].as_str().expect("log_path");
    let state = env.state_dir.to_str().unwrap();
    assert!(log_path.starts_with(state), "日志应落在状态目录：{log_path}");

    let export = env.cli_json(&["settings", "diagnostics", "--export"]);
    let path = export["path"].as_str().expect("导出路径");
    assert!(Path::new(path).exists(), "导出文件应真实存在：{path}");
    assert!(path.starts_with(state), "导出应落在状态目录：{path}");
    assert!(export["report"]["version"].is_string(), "{export}");
}

#[test]
fn capacity_on_skillless_vault_is_zero_and_empty() {
    let env = TestEnv::new();
    let report = env.cli_json(&["settings", "capacity"]);
    assert_eq!(report["skill_count"], 0, "{report}");
    assert_eq!(report["skill_total_size"], 0, "{report}");
    assert!(report["skills"].as_array().unwrap().is_empty(), "{report}");
    assert!(report["total_size"].as_u64().is_some(), "{report}");
}
