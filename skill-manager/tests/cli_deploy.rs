//! quickstart M2 CLI 场景（T023）：部署、接管、同步暂停、撤销、删库清理、多技能×多目标
//! 与 dry-run。全部使用预置库文件，不依赖 US2。

mod common;

use common::TestEnv;
use serde_json::Value;

fn seed(env: &TestEnv) {
    env.write_skill(
        "code",
        Some("工具"),
        "alpha",
        "---\nname: alpha\ndescription: 解析依赖\n---\n原库内正文\n",
    );
    env.write_manager("code", Some("工具"), "alpha", "meta.yaml", "comment: 检查依赖\n");
    env.write_manager("code", Some("工具"), "alpha", "overlay/_/_/notes.txt", "通用补充\n");

    env.write_skill("code", None, "beta", "beta 正文\n");
    env.write_skill("lobster", None, "omega", "龙虾技能\n");
}

fn deployed(env: &TestEnv, rel: &str, name: &str) -> std::path::PathBuf {
    env.home.join(rel).join(name)
}

fn item_skills(items: &Value) -> Vec<String> {
    items
        .as_array()
        .expect("items 应为数组")
        .iter()
        .map(|item| {
            format!(
                "{}/{}",
                item["skill"].as_str().unwrap(),
                item["target"].as_str().unwrap()
            )
        })
        .collect()
}

fn failures(result: &Value) -> Vec<String> {
    result["errors"]
        .as_array()
        .expect("errors 应为数组")
        .iter()
        .map(|error| {
            let skill = error["skill"].as_str().unwrap_or("-");
            let message = error["message"].as_str().unwrap();
            format!("{skill}: {message}")
        })
        .collect()
}

#[test]
fn unmanaged_same_name_is_overwritten_and_recorded() {
    let env = TestEnv::new();
    seed(&env);
    let copy = env.agent_skill(".codex/skills", "alpha", "旧的未受管副本\n");
    assert!(copy.join("SKILL.md").exists());

    let (code, result) = env.cli_json_raw(&["deploy", "to", "codex", "code/alpha"]);
    assert_eq!(code, 0, "{result}");
    let items = result["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["action"], "deployed");
    assert_eq!(items[0]["status"], "ok");

    let entry = std::fs::read_to_string(copy.join("SKILL.md")).unwrap();
    assert!(entry.contains("检查依赖｜解析依赖"), "{entry}");
    assert!(!copy.join(".manager").exists());
    assert!(copy.join("notes.txt").exists());

    let status = env.cli_json(&["deploy", "status"]);
    let records = status["deployments"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["target"], "agent:codex");
    assert_eq!(records[0]["status"], "ok");
    assert_eq!(records[0]["library_exists"], true);
}

#[test]
fn sync_updates_existing_deployments_without_adding_targets() {
    let env = TestEnv::new();
    seed(&env);
    let (code, _) = env.cli_json_raw(&["deploy", "to", "codex", "code/alpha"]);
    assert_eq!(code, 0);

    // 库内正文变化后同步。
    env.write_skill(
        "code",
        Some("工具"),
        "alpha",
        "---\nname: alpha\ndescription: 解析依赖\n---\n新库内正文\n",
    );
    let (code, result) = env.cli_json_raw(&["deploy", "sync"]);
    assert_eq!(code, 0, "{result}");
    let entry = std::fs::read_to_string(deployed(&env, ".codex/skills", "alpha").join("SKILL.md")).unwrap();
    assert!(entry.contains("新库内正文"), "{entry}");

    // beta 无记录：同步不得产生首次部署。
    assert!(!deployed(&env, ".codex/skills", "beta").exists());
    let status = env.cli_json(&["deploy", "status"]);
    assert_eq!(status["deployments"].as_array().unwrap().len(), 1);
}

#[test]
fn local_change_pauses_that_item_and_the_rest_continue() {
    let env = TestEnv::new();
    seed(&env);
    let (code, _) = env.cli_json_raw(&[
        "deploy", "to", "codex", "code/alpha", "code/beta",
    ]);
    assert_eq!(code, 0, "批量部署应成功");

    // 外部修改 alpha 的受管副本。
    let alpha_copy = deployed(&env, ".codex/skills", "alpha");
    std::fs::write(alpha_copy.join("SKILL.md"), "用户手改的内容\n").unwrap();

    let (code, result) = env.cli_json_raw(&["deploy", "sync"]);
    assert_eq!(code, 2, "存在暂停项时应为部分完成：{result}");
    let messages = failures(&result);
    assert!(
        messages.iter().any(|m| m.contains("外部修改") && m.contains("alpha")),
        "{messages:?}"
    );
    // 手改副本未被覆盖。
    assert_eq!(
        std::fs::read_to_string(alpha_copy.join("SKILL.md")).unwrap(),
        "用户手改的内容\n"
    );
    // beta 继续同步（这里无变化，仍是被处理项）。
    assert!(
        item_skills(&result["data"]["items"]).contains(&"code/beta/agent:codex".to_string()),
        "{:?}",
        result["data"]["items"]
    );

    let paused = env.cli_json(&["deploy", "status", "--paused"]);
    let records = paused["deployments"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["status"], "paused_local_change");

    // 用户裁决后显式按库覆盖。
    let (code, result) = env.cli_json_raw(&[
        "deploy", "sync", "--ref", "code/alpha", "--target", "codex", "--overwrite-local",
    ]);
    assert_eq!(code, 0, "{result}");
    assert!(
        std::fs::read_to_string(alpha_copy.join("SKILL.md"))
            .unwrap()
            .contains("检查依赖｜解析依赖")
    );
    let after = env.cli_json(&["deploy", "status", "--paused"]);
    assert!(after["deployments"].as_array().unwrap().is_empty());
}

#[test]
fn undo_removes_copy_and_keeps_library_skill() {
    let env = TestEnv::new();
    seed(&env);
    let (code, _) = env.cli_json_raw(&["deploy", "to", "codex", "code/alpha"]);
    assert_eq!(code, 0);

    let (code, result) = env.cli_json_raw(&["deploy", "undo", "codex", "code/alpha"]);
    assert_eq!(code, 0, "{result}");
    assert!(!deployed(&env, ".codex/skills", "alpha").exists());
    assert!(env.path("code/工具/alpha/SKILL.md").exists(), "库内技能必须保留");
    let status = env.cli_json(&["deploy", "status"]);
    assert!(status["deployments"].as_array().unwrap().is_empty());
}

#[test]
fn skill_delete_cleans_copies_and_library_entry() {
    let env = TestEnv::new();
    seed(&env);
    let (code, _) = env.cli_json_raw(&[
        "deploy", "to", "codex", "code/alpha", "--also-target", "zcode",
    ]);
    assert_eq!(code, 0, "多目标部署应成功");

    let (code, result) = env.cli_json_raw(&["skill", "delete", "code/alpha"]);
    assert_eq!(code, 0, "{result}");
    assert!(!env.path("code/工具/alpha").exists(), "库内条目应删除");
    assert!(!deployed(&env, ".codex/skills", "alpha").exists());
    assert!(!deployed(&env, ".zcode/skills", "alpha").exists());
    let status = env.cli_json(&["deploy", "status"]);
    assert!(status["deployments"].as_array().unwrap().is_empty());
}

#[test]
fn skill_delete_keeps_externally_changed_copy_and_reports_pause() {
    let env = TestEnv::new();
    seed(&env);
    let (code, _) = env.cli_json_raw(&["deploy", "to", "codex", "code/alpha"]);
    assert_eq!(code, 0);
    let copy = deployed(&env, ".codex/skills", "alpha");
    std::fs::write(copy.join("SKILL.md"), "用户手改的内容\n").unwrap();

    let (code, result) = env.cli_json_raw(&["skill", "delete", "code/alpha"]);
    assert_eq!(code, 2, "清理暂停应如实报告为部分完成：{result}");
    let messages = failures(&result);
    assert!(
        messages.iter().any(|m| m.contains("库已删除") && m.contains("暂停")),
        "{messages:?}"
    );
    assert!(!env.path("code/工具/alpha").exists(), "库内条目仍应删除");
    assert!(copy.join("SKILL.md").exists(), "外改副本必须保留");

    let status = env.cli_json(&["deploy", "status", "--paused"]);
    let records = status["deployments"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["library_exists"], false);

    // 用户裁决后显式丢弃待清理副本。
    let (code, result) = env.cli_json_raw(&[
        "deploy", "undo", "codex", "code/alpha", "--discard-local-changes",
    ]);
    assert_eq!(code, 0, "{result}");
    assert!(!copy.exists());
    let status = env.cli_json(&["deploy", "status"]);
    assert!(status["deployments"].as_array().unwrap().is_empty());
}

#[test]
fn multi_skill_multi_target_filters_each_pair_and_continues() {
    let env = TestEnv::new();
    seed(&env);
    env.write_manager("code", Some("工具"), "alpha", "meta.yaml", "comment: 检查依赖\nagents: [zcode]\n");

    let (code, result) = env.cli_json_raw(&[
        "deploy", "to", "codex", "code/alpha", "code/beta", "--also-target", "zcode",
    ]);
    assert_eq!(code, 2, "逐项过滤应整体报告部分完成：{result}");
    let items = item_skills(&result["data"]["items"]);
    assert!(items.contains(&"code/beta/agent:codex".to_string()), "{items:?}");
    assert!(items.contains(&"code/beta/agent:zcode".to_string()), "{items:?}");
    assert!(items.contains(&"code/alpha/agent:zcode".to_string()), "{items:?}");
    assert!(!items.contains(&"code/alpha/agent:codex".to_string()), "{items:?}");
    let messages = failures(&result);
    assert!(
        messages.iter().any(|m| m.contains("禁止新增部署")),
        "{messages:?}"
    );
    assert!(!deployed(&env, ".codex/skills", "alpha").exists());
    assert!(deployed(&env, ".zcode/skills", "alpha").exists());
    assert!(deployed(&env, ".codex/skills", "beta").exists());
}

#[test]
fn dry_run_reports_paths_without_changing_files_or_records() {
    let env = TestEnv::new();
    seed(&env);
    let before = env.cli_json(&["deploy", "status"]);

    let (code, result) = env.cli_json_raw(&["deploy", "to", "codex", "code/alpha", "--dry-run"]);
    assert_eq!(code, 0, "{result}");
    assert_eq!(result["data"]["dry_run"], true);
    let items = result["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["action"], "would_deploy");
    assert!(items[0]["path"].as_str().unwrap().ends_with(".codex/skills/alpha"));
    assert!(!deployed(&env, ".codex/skills", "alpha").exists(), "预演不得落文件");

    let after = env.cli_json(&["deploy", "status"]);
    assert_eq!(before, after, "预演不得改本机记录");

    // 撤销预演：先真实部署，再预演撤销。
    let (code, _) = env.cli_json_raw(&["deploy", "to", "codex", "code/alpha"]);
    assert_eq!(code, 0);
    let (code, result) = env.cli_json_raw(&["deploy", "undo", "codex", "code/alpha", "--dry-run"]);
    assert_eq!(code, 0, "{result}");
    assert_eq!(result["data"]["dry_run"], true);
    assert!(deployed(&env, ".codex/skills", "alpha").exists(), "预演不得删副本");
    let status = env.cli_json(&["deploy", "status"]);
    assert_eq!(status["deployments"].as_array().unwrap().len(), 1);
}
