//! quickstart M2 的 Agent 侧 CLI 场景（T067/T068）：检测只读事实、启停/排序/目录只写本机状态、
//! 本地技能列表与删除（未受管不改库、受管走撤销与暂停、路径包含性校验）。

mod common;

use common::TestEnv;
use serde_json::Value;

fn seed(env: &TestEnv) {
    env.write_skill("code", None, "alpha", "---\nname: alpha\ndescription: 解析依赖\n---\n正文\n");
}

fn agent_entry<'a>(agents: &'a Value, key: &str) -> &'a Value {
    agents
        .as_array()
        .expect("agents 应为数组")
        .iter()
        .find(|entry| entry["key"] == key)
        .unwrap_or_else(|| panic!("缺少 Agent {key}"))
}

#[test]
fn list_and_detect_report_current_installation_without_side_effects() {
    let env = TestEnv::new();
    let data = env.cli_json(&["agent", "list"]);
    assert_eq!(data["total"], 53);
    assert_eq!(data["detected"], 0, "空 home 不应检测到任何 Agent");

    env.agent_dir(".codex");
    let detected = env.cli_json(&["agent", "detect"]);
    assert_eq!(detected["detected"], 1);
    assert_eq!(agent_entry(&detected["agents"], "codex")["detected"], true);
    // 检测不写本机状态：再次 list 仍是同一事实，且未产生部署。
    let status = env.cli_json(&["deploy", "status"]);
    assert!(status["deployments"].as_array().unwrap().is_empty());
}

#[test]
fn enable_disable_and_reorder_only_change_local_state() {
    let env = TestEnv::new();
    let view = env.cli_json(&["agent", "disable", "codex"]);
    assert_eq!(view["agent"]["enabled"], false);

    let listed = env.cli_json(&["agent", "list"]);
    assert_eq!(agent_entry(&listed["agents"], "codex")["enabled"], false);

    env.cli_json(&["agent", "reorder", "--order", "zcode,codex"]);
    let listed = env.cli_json(&["agent", "list"]);
    assert_eq!(agent_entry(&listed["agents"], "zcode")["order"], 0);
    assert_eq!(agent_entry(&listed["agents"], "codex")["order"], 1);

    let (code, _) = env.cli_json_raw(&["agent", "enable", "not-an-agent"]);
    assert_ne!(code, 0, "未知 Agent key 应被拒绝");
}

#[test]
fn set_dir_and_reset_dir_are_local_and_used_by_deploy() {
    let env = TestEnv::new();
    seed(&env);
    let custom = env.root.path().join("custom-skills");
    std::fs::create_dir_all(&custom).unwrap();

    let view = env.cli_json(&[
        "agent",
        "set-dir",
        "codex",
        "--skills",
        &custom.display().to_string(),
    ]);
    assert_eq!(view["agent"]["skills_dir"], custom.display().to_string());
    assert_eq!(view["agent"]["skills_dir_custom"], true);

    let (code, result) = env.cli_json_raw(&["deploy", "to", "codex", "code/alpha"]);
    assert_eq!(code, 0, "{result}");
    assert!(
        custom.join("alpha/SKILL.md").exists(),
        "部署应使用修改后的技能目录"
    );

    let view = env.cli_json(&["agent", "reset-dir", "codex", "--kind", "skills"]);
    assert_eq!(view["agent"]["skills_dir_custom"], false);

    let (code, _) = env.cli_json_raw(&["agent", "set-dir", "codex", "--project-rel", "a/../b"]);
    assert_ne!(code, 0, "项目相对路径禁止 ..");
    let (code, _) = env.cli_json_raw(&[
        "agent",
        "set-dir",
        "codex",
        "--skills",
        "relative/path",
    ]);
    assert_ne!(code, 0, "技能目录必须是绝对路径");
}

#[test]
fn local_skills_marks_managed_and_unmanaged() {
    let env = TestEnv::new();
    seed(&env);
    env.agent_skill(".codex/skills", "localonly", "仅本地\n");
    env.cli_json(&["deploy", "to", "codex", "code/alpha"]);

    let data = env.cli_json(&["agent", "local-skills", "codex"]);
    assert_eq!(data["managed"], 1);
    let skills = data["skills"].as_array().unwrap();
    let alpha = skills.iter().find(|s| s["name"] == "alpha").unwrap();
    assert_eq!(alpha["managed"], true);
    assert_eq!(alpha["status"], "managed");
    let local = skills.iter().find(|s| s["name"] == "localonly").unwrap();
    assert_eq!(local["managed"], false);
    assert_eq!(local["status"], "local");
}

#[test]
fn delete_local_unmanaged_does_not_touch_library() {
    let env = TestEnv::new();
    seed(&env);
    let dir = env.agent_skill(".codex/skills", "localonly", "仅本地\n");

    let (code, result) = env.cli_json_raw(&["agent", "delete-local", "codex", "localonly"]);
    assert_eq!(code, 0, "{result}");
    assert!(!dir.exists(), "未受管副本应删除实际文件");
    assert!(env.path("code/alpha/SKILL.md").exists(), "库内技能不得受影响");
}

#[test]
fn delete_local_managed_uses_undo_rule_and_keeps_library() {
    let env = TestEnv::new();
    seed(&env);
    env.cli_json(&["deploy", "to", "codex", "code/alpha"]);
    let copy = env.home.join(".codex/skills/alpha");
    assert!(copy.exists());

    let (code, result) = env.cli_json_raw(&["agent", "delete-local", "codex", "alpha"]);
    assert_eq!(code, 0, "{result}");
    assert!(!copy.exists(), "受管副本按撤销规则移除");
    assert!(env.path("code/alpha/SKILL.md").exists(), "删本机副本不得删库");
    let status = env.cli_json(&["deploy", "status"]);
    assert!(status["deployments"].as_array().unwrap().is_empty());
}

#[test]
fn delete_local_managed_pauses_when_copy_changed_externally() {
    let env = TestEnv::new();
    seed(&env);
    env.cli_json(&["deploy", "to", "codex", "code/alpha"]);
    let copy = env.home.join(".codex/skills/alpha");
    std::fs::write(copy.join("SKILL.md"), "手改内容\n").unwrap();

    let (code, result) = env.cli_json_raw(&["agent", "delete-local", "codex", "alpha"]);
    assert_eq!(code, 1, "外改副本应拒绝删除：{result}");
    assert!(copy.join("SKILL.md").exists(), "外改副本必须保留");
    assert!(env.path("code/alpha/SKILL.md").exists(), "库内技能不得受影响");
    let status = env.cli_json(&["deploy", "status"]);
    assert_eq!(status["deployments"].as_array().unwrap().len(), 1);
}

#[test]
fn delete_local_rejects_paths_outside_skills_dir() {
    let env = TestEnv::new();
    seed(&env);
    let (code, result) = env.cli_json_raw(&["agent", "delete-local", "codex", "../vault"]);
    assert_ne!(code, 0, "越权路径必须拒绝：{result}");
    assert!(env.path("code/alpha/SKILL.md").exists());
}

#[test]
fn local_skills_report_empty_when_agent_dir_absent() {
    let env = TestEnv::new();
    let data = env.cli_json(&["agent", "local-skills", "codex"]);
    assert_eq!(data["total"], 0);
    assert!(data["skills"].as_array().unwrap().is_empty());
}
