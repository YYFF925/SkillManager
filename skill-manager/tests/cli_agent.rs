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

// ---------------------------------------------------------------------------
// 共享目录技能浏览（FR-042/FR-055）：`shared` 是编码域的本机部署目标，
// 与 Agent 目录同构——列出、标受管、删除未受管/撤销受管。
// ---------------------------------------------------------------------------

/// 在任意目录下放一个技能副本（共享目录不在 home 下，不能用 agent_skill）。
fn write_skill_at(dir: &std::path::Path, name: &str, body: &str) {
    let skill = dir.join(name);
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), body).unwrap();
}

#[test]
fn local_skills_shared_resolves_missing_directory_without_creating_it() {
    let env = TestEnv::new();
    let data = env.cli_json(&["agent", "local-skills", "shared"]);
    let shared = env.home.join(".agents/skills");
    assert_eq!(data["skills_dir"], shared.to_string_lossy().as_ref());
    assert_eq!(data["total"], 0);
    assert!(!shared.exists());
}

#[test]
fn shared_local_skill_can_be_adopted_and_original_target_is_managed() {
    let env = TestEnv::new();
    env.agent_skill(".agents/skills", "localonly", "---\ndescription: local\n---\nlocal body\n");
    let result = env.cli_json(&["skill", "adopt", "shared", "localonly", "--comment", "管理本地技能"]);
    assert_eq!(result["adopted"], 1, "{result}");
    assert!(env.path("code/localonly/SKILL.md").exists());
    let local = env.cli_json(&["agent", "local-skills", "shared"]);
    assert_eq!(local["skills"][0]["managed"], true);
    assert_eq!(local["skills"][0]["library_skill"], "code/localonly");
    let status = env.cli_json(&["deploy", "status"]);
    assert_eq!(status["deployments"].as_array().unwrap().len(), 1);
    assert_eq!(status["deployments"][0]["target"], "shared");
    assert!(!env.home.join(".codex/skills/localonly").exists());
}

#[test]
fn read_local_uses_actual_copy_supports_nested_files_and_rejects_outside_paths() {
    let env = TestEnv::new();
    env.write_skill("code", None, "alpha", "library body\n");
    let local = env.agent_skill(".agents/skills", "alpha", "local body\n");
    std::fs::create_dir_all(local.join("references")).unwrap();
    std::fs::write(local.join("references/usage.md"), "local reference\n").unwrap();
    std::fs::write(local.join("image.bin"), [0xff_u8]).unwrap();
    std::fs::write(local.parent().unwrap().join("outside.txt"), "outside\n").unwrap();
    let body = env.cli_json(&["agent", "read-local", "shared", "alpha"]);
    assert_eq!(body["text"], "local body\n");
    assert!(body["files"].as_array().unwrap().iter().any(|p| p == "references/usage.md"));
    let file = env.cli_json(&["agent", "read-local", "shared", "alpha", "--file", "references/usage.md"]);
    assert_eq!(file["text"], "local reference\n");
    let binary = env.cli_json(&["agent", "read-local", "shared", "alpha", "--file", "image.bin"]);
    assert!(binary["text"].is_null());
    let (code, _) = env.cli_json_raw(&["agent", "read-local", "shared", "alpha", "--file", "../outside.txt"]);
    assert_ne!(code, 0);
    assert_eq!(env.read("code/alpha/SKILL.md").unwrap(), "library body\n");
}

#[test]
fn local_skills_shared_lists_and_marks_managed() {
    let env = TestEnv::new();
    seed(&env);
    let shared = env.home.join(".agents/skills");
    std::fs::create_dir_all(&shared).unwrap();
    let shared_arg = shared.to_str().unwrap().to_string();
    env.cli_json(&["deploy", "to", "shared", "code/alpha"]);
    write_skill_at(&shared, "localonly", "仅本地\n");

    let data = env.cli_json(&["agent", "local-skills", "shared"]);
    assert_eq!(data["skills_dir"], shared_arg);
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
fn delete_local_shared_removes_unmanaged_and_undoes_managed() {
    let env = TestEnv::new();
    seed(&env);
    let shared = env.home.join(".agents/skills");
    std::fs::create_dir_all(&shared).unwrap();
    env.cli_json(&["deploy", "to", "shared", "code/alpha"]);
    write_skill_at(&shared, "localonly", "仅本地\n");

    // 未受管项：删除实际文件，不动库。
    let (code, result) = env.cli_json_raw(&["agent", "delete-local", "shared", "localonly"]);
    assert_eq!(code, 0, "{result}");
    assert!(!shared.join("localonly").exists(), "未受管副本应删除实际文件");
    assert!(env.path("code/alpha/SKILL.md").exists(), "库内技能不得受影响");

    // 受管项：按撤销部署规则移除副本并清理记录，库内技能保留。
    let (code, result) = env.cli_json_raw(&["agent", "delete-local", "shared", "alpha"]);
    assert_eq!(code, 0, "{result}");
    assert!(!shared.join("alpha").exists(), "受管副本按撤销规则移除");
    assert!(env.path("code/alpha/SKILL.md").exists(), "删本机副本不得删库");
    let status = env.cli_json(&["deploy", "status"]);
    assert!(status["deployments"].as_array().unwrap().is_empty());
}

#[test]
fn local_skills_for_agent_key_is_unaffected_by_shared_support() {
    let env = TestEnv::new();
    seed(&env);
    env.agent_skill(".codex/skills", "localonly", "仅本地\n");
    let shared = env.home.join(".agents/skills");
    std::fs::create_dir_all(&shared).unwrap();

    // 设置共享目录不改变内置 Agent key 的浏览结果。
    let data = env.cli_json(&["agent", "local-skills", "codex"]);
    assert_eq!(data["total"], 1);
    assert_eq!(data["skills"][0]["name"], "localonly");

    // 其它命令仍只接受内置 Agent key。
    let (code, value) = env.cli_json_raw(&["agent", "local-skills", "not-an-agent"]);
    assert_ne!(code, 0, "{value}");
}

// ---------------------------------------------------------------------------
// Hermes default 档案（`hermes:default`）：与 `shared` 同批登记为正式取值，
// 复用同一 `LocalSkillService` 与 `parse_target_arg`，行为同构（列出、标受管、删未受管）。
// default 档案即 Hermes 根自身，技能目录为 `<Hermes 根>/skills`。
// ---------------------------------------------------------------------------

#[test]
fn local_skills_and_delete_local_support_hermes_default_target() {
    let env = TestEnv::new();
    env.hermes_init();
    let default_skills = env.hermes_root().join("skills");
    seed(&env);
    env.write_skill("lobster", None, "beta", "---\nname: beta\ndescription: 龙虾技能\n---\n正文\n");
    env.cli_json(&["deploy", "to", "hermes:default", "lobster/beta"]);
    write_skill_at(&default_skills, "localonly", "仅本地\n");

    // 列出：与 Agent 目录同构，区分受管（存在 `target=hermes:default` 记录）与仅本地。
    let data = env.cli_json(&["agent", "local-skills", "hermes:default"]);
    assert_eq!(data["skills_dir"], default_skills.display().to_string());
    assert_eq!(data["managed"], 1);
    let skills = data["skills"].as_array().unwrap();
    let managed = skills.iter().find(|s| s["name"] == "beta").unwrap();
    assert_eq!(managed["managed"], true);
    assert_eq!(managed["status"], "managed");
    let local = skills.iter().find(|s| s["name"] == "localonly").unwrap();
    assert_eq!(local["managed"], false);
    assert_eq!(local["status"], "local");

    // 未受管项：删除实际文件，不改库。
    let (code, result) = env.cli_json_raw(&["agent", "delete-local", "hermes:default", "localonly"]);
    assert_eq!(code, 0, "{result}");
    assert!(!default_skills.join("localonly").exists(), "未受管副本应删除实际文件");
    assert!(env.path("lobster/beta/SKILL.md").exists(), "库内技能不得受影响");
}
