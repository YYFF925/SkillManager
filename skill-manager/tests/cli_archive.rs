//! 技能归档 CLI：移出正常库、保留分类/标签、还原不部署、列表隐藏。

mod common;

use common::TestEnv;

fn seed(env: &TestEnv) {
    env.write_skill("code", Some("工具"), "alpha", "alpha 正文\n");
    env.write_manager(
        "code",
        Some("工具"),
        "alpha",
        "meta.yaml",
        "tags: [x, y]\ncomment: 检查依赖\n",
    );
    env.write_skill("code", None, "gamma", "gamma 正文\n");
}

#[test]
fn archive_hides_from_list_and_restore_keeps_meta() {
    let env = TestEnv::new();
    seed(&env);

    let archived = env.cli_json(&["skill", "archive", "add", "code/alpha"]);
    assert_eq!(archived["archived"][0], "code/alpha");
    assert!(env.path("archive/code/工具/alpha/SKILL.md").exists());
    assert!(!env.path("code/工具/alpha").exists());

    let listed = env.cli_json(&["skill", "list", "--domain", "code"]);
    let names: Vec<_> = listed["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["gamma"]);

    let archive_list = env.cli_json(&["skill", "archive", "list", "--domain", "code"]);
    assert_eq!(archive_list["total"], 1);
    assert_eq!(archive_list["skills"][0]["name"], "alpha");
    assert_eq!(archive_list["skills"][0]["category"], "工具");
    assert_eq!(archive_list["skills"][0]["comment"], "检查依赖");
    assert_eq!(
        archive_list["skills"][0]["vault_rel_dir"],
        "archive/code/工具/alpha"
    );

    let restored = env.cli_json(&["skill", "archive", "restore", "code/alpha"]);
    assert_eq!(restored["restored"][0], "code/alpha");
    assert!(env.path("code/工具/alpha/SKILL.md").exists());
    let skill = env.cli_json(&["skill", "get", "code/alpha"]);
    assert_eq!(skill["skill"]["category"], "工具");
    assert_eq!(skill["skill"]["comment"], "检查依赖");
}

#[test]
fn archive_delete_removes_files() {
    let env = TestEnv::new();
    seed(&env);
    env.cli_json(&["skill", "archive", "add", "code/gamma"]);
    env.cli_json(&["skill", "archive", "delete", "code/gamma"]);
    assert!(!env.path("archive/code/gamma").exists());
    let listed = env.cli_json(&["skill", "archive", "list"]);
    assert_eq!(listed["total"], 0);
}

#[test]
fn archive_uninstalls_copies_and_restore_does_not_redeploy() {
    let env = TestEnv::new();
    seed(&env);
    env.cli_json(&["deploy", "to", "codex", "code/alpha"]);
    assert!(env.home.join(".codex/skills/alpha/SKILL.md").exists());

    let archived = env.cli_json(&["skill", "archive", "add", "code/alpha"]);
    assert_eq!(archived["archived"][0], "code/alpha");
    assert_eq!(archived["cleanup"].as_array().unwrap().len(), 1);
    assert!(!env.home.join(".codex/skills/alpha").exists());
    let status = env.cli_json(&["deploy", "status"]);
    assert!(status["deployments"].as_array().unwrap().is_empty());

    env.cli_json(&["skill", "archive", "restore", "code/alpha"]);
    assert!(env.path("code/工具/alpha/SKILL.md").exists());
    assert!(!env.home.join(".codex/skills/alpha").exists());
    let status = env.cli_json(&["deploy", "status"]);
    assert!(status["deployments"].as_array().unwrap().is_empty());
}

#[test]
fn archive_aborts_when_copy_has_local_changes() {
    let env = TestEnv::new();
    seed(&env);
    env.cli_json(&["deploy", "to", "codex", "code/alpha"]);
    let copy = env.home.join(".codex/skills/alpha");
    std::fs::write(copy.join("SKILL.md"), "用户手改的内容\n").unwrap();

    let (code, result) = env.cli_json_raw(&["skill", "archive", "add", "code/alpha"]);
    assert_eq!(code, 1, "{result}");
    assert_eq!(result["result"], "failed");
    assert!(
        env.path("code/工具/alpha/SKILL.md").exists(),
        "卸载失败不得归档"
    );
    assert!(copy.join("SKILL.md").exists(), "外改副本必须保留");
    assert!(!env.path("archive/code/工具/alpha").exists());
    let listed = env.cli_json(&["skill", "list", "--domain", "code"]);
    let names: Vec<_> = listed["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"alpha".to_string()), "{names:?}");
}
