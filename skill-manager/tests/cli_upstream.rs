//! T041：quickstart M3 检查/更新场景（US4）。
//! 跨域同仓库只获取一次、文件增改删汇总、检查不应用、单项范围隔离、仓库批量与跳过、
//! 拒绝新增共享、技能内文件删除 vs 整个技能删除、首次未选与后来新增、暂停副本不被覆盖。

mod common;

use common::{TestEnv, UpstreamRepo};
use serde_json::Value;

fn alpha_entry(env: &TestEnv) -> String {
    std::fs::read_to_string(env.path("code/alpha/SKILL.md")).unwrap()
}

fn repo(env: &TestEnv) -> UpstreamRepo {
    let up = env.upstream("upstream");
    up.skill(Some("tools"), "alpha", "---\nname: alpha\ndescription: 甲\n---\nalpha v1\n");
    up.write("tools/alpha/notes.txt", "note v1\n");
    up.skill(Some("tools"), "beta", "---\nname: beta\ndescription: 乙\n---\nbeta v1\n");
    up.commit("v1");
    up
}

fn import(env: &TestEnv, up: &UpstreamRepo, subpath: &str, domain: &str) {
    env.cli_json(&[
        "skill", "import", &up.url(),
        "--domain", domain,
        "--skills", subpath,
        "--uncategorized",
    ]);
}

fn check_all(env: &TestEnv) -> Value {
    env.cli_json(&["skill", "check", "--all"])
}

fn skill_result(report: &Value, skill: &str) -> Value {
    report["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|view| view["skill"] == skill)
        .cloned()
        .unwrap_or(Value::Null)
}

#[test]
fn same_repo_across_two_domains_is_checked_once() {
    let env = TestEnv::new();
    let up = repo(&env);
    import(&env, &up, "tools/alpha", "code");
    import(&env, &up, "tools/alpha", "lobster");

    let report = check_all(&env);
    assert_eq!(
        report["repos"].as_array().unwrap().len(),
        1,
        "跨两域同仓库只获取/检查一次：{report}"
    );
    assert!(skill_result(&report, "code/alpha")["result"].is_string());
    assert!(skill_result(&report, "lobster/alpha")["result"].is_string());
}

#[test]
fn file_changes_are_summarized_without_line_diff() {
    let env = TestEnv::new();
    let up = repo(&env);
    import(&env, &up, "tools/alpha", "code");

    up.skill(Some("tools"), "alpha", "---\nname: alpha\ndescription: 甲\n---\nalpha v2\n");
    up.write("tools/alpha/added.txt", "add\n");
    std::fs::remove_file(up.path.join("tools/alpha/notes.txt")).unwrap();
    up.commit("v2");

    let report = check_all(&env);
    let changes = &skill_result(&report, "code/alpha")["changes"];
    assert!(changes["modified"].as_array().unwrap().iter().any(|v| v == "SKILL.md"));
    assert!(changes["added"].as_array().unwrap().iter().any(|v| v == "added.txt"));
    assert!(changes["deleted"].as_array().unwrap().iter().any(|v| v == "notes.txt"));
}

#[test]
fn check_does_not_apply_changes() {
    let env = TestEnv::new();
    let up = repo(&env);
    import(&env, &up, "tools/alpha", "code");
    let before_version = env.cli_json(&["skill", "get", "code/alpha"])["skill"]["source"]["version"].clone();

    up.skill(Some("tools"), "alpha", "---\nname: alpha\ndescription: 甲\n---\nalpha v2\n");
    up.commit("v2");

    let report = check_all(&env);
    assert_eq!(skill_result(&report, "code/alpha")["result"], "update_available");
    assert!(alpha_entry(&env).contains("alpha v1"), "检查不应用变化");
    let after = env.cli_json(&["skill", "get", "code/alpha"]);
    assert_eq!(after["skill"]["source"]["version"], before_version, "检查不改采用版本");
}

#[test]
fn single_skill_update_is_isolated() {
    let env = TestEnv::new();
    let up = repo(&env);
    import(&env, &up, "tools/alpha", "code");
    import(&env, &up, "tools/beta", "code");

    up.skill(Some("tools"), "alpha", "---\nname: alpha\ndescription: 甲\n---\nalpha v2\n");
    up.skill(Some("tools"), "beta", "---\nname: beta\ndescription: 乙\n---\nbeta v2\n");
    up.commit("v2");
    check_all(&env);

    let updated = env.cli_json(&["skill", "update", "code/alpha"]);
    assert_eq!(updated["action"], "updated");
    assert!(alpha_entry(&env).contains("alpha v2"));
    let beta = std::fs::read_to_string(env.path("code/beta/SKILL.md")).unwrap();
    assert!(beta.contains("beta v1"), "同仓库其他技能不受影响：{beta}");

    let report = check_all(&env);
    assert_eq!(skill_result(&report, "code/beta")["result"], "update_available");
}

#[test]
fn repo_batch_update_respects_skip() {
    let env = TestEnv::new();
    let up = repo(&env);
    import(&env, &up, "tools/alpha", "code");
    import(&env, &up, "tools/beta", "code");

    up.skill(Some("tools"), "alpha", "---\nname: alpha\ndescription: 甲\n---\nalpha v2\n");
    up.skill(Some("tools"), "beta", "---\nname: beta\ndescription: 乙\n---\nbeta v2\n");
    up.commit("v2");
    check_all(&env);

    env.cli_json(&["skill", "skip", "code/beta"]);
    let skipped = env.cli_json(&["skill", "skipped"]);
    assert_eq!(skipped["skills"][0]["skill"], "code/beta");

    let outcome = env.cli_json(&["skill", "update-repo", &up.url()]);
    let updated: Vec<&str> = outcome["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["skill"].as_str().unwrap())
        .collect();
    assert!(updated.contains(&"code/alpha"), "{updated:?}");
    assert!(!updated.contains(&"code/beta"), "跳过决定被批量更新尊重：{updated:?}");
    assert!(std::fs::read_to_string(env.path("code/beta/SKILL.md")).unwrap().contains("beta v1"));

    let report = check_all(&env);
    assert_eq!(skill_result(&report, "code/beta")["result"], "skipped", "跳过只影响该域该技能该版本");
}

#[test]
fn reject_new_is_shared_and_not_prompted_again() {
    let env = TestEnv::new();
    let up = repo(&env);
    import(&env, &up, "tools/alpha", "code");

    // carol 首次已存在但未选，不冒充后来新增。
    let first = check_all(&env);
    let carol = first["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["subpath"] == "tools/beta")
        .cloned()
        .unwrap();
    assert_eq!(carol["is_new"], false, "首次已存在的未选候选不算后来新增");

    env.cli_json(&[
        "skill", "reject-new", &up.url(),
        "--domain", "code",
        "--skills", "tools/beta",
    ]);
    let after = check_all(&env);
    assert!(
        !after["candidates"].as_array().unwrap().iter().any(|c| c["subpath"] == "tools/beta"),
        "明确拒绝新增后不再提示：{after}"
    );
}

#[test]
fn internal_file_deletion_and_whole_skill_deletion_are_distinct() {
    let env = TestEnv::new();
    let up = repo(&env);
    import(&env, &up, "tools/alpha", "code");
    import(&env, &up, "tools/beta", "code");

    // 仅删除技能内文件：移除该文件，技能与其他变化照常应用。
    std::fs::remove_file(up.path.join("tools/alpha/notes.txt")).unwrap();
    up.skill(Some("tools"), "alpha", "---\nname: alpha\ndescription: 甲\n---\nalpha v2\n");
    up.commit("delete one file");
    check_all(&env);
    let updated = env.cli_json(&["skill", "update", "code/alpha"]);
    assert_eq!(updated["action"], "updated");
    assert!(!env.path("code/alpha/notes.txt").exists(), "技能内被删除的文件被移除");
    assert!(env.path("code/alpha/SKILL.md").exists(), "技能仍然存在");

    // 整个技能被上游删除：check 报告缺失，单项 update 确认后才清库。
    std::fs::remove_dir_all(up.path.join("tools/alpha")).unwrap();
    up.commit("remove whole skill");
    let report = check_all(&env);
    assert_eq!(skill_result(&report, "code/alpha")["result"], "upstream_missing");
    assert!(env.path("code/alpha/SKILL.md").exists(), "检查不删除技能");

    let removed = env.cli_json(&["skill", "update", "code/alpha"]);
    assert_eq!(removed["action"], "removed");
    assert!(!env.path("code/alpha").exists(), "确认后移除整个技能");
    assert!(env.path("code/beta/SKILL.md").exists(), "其他技能不受影响");
}

#[test]
fn first_unselected_stays_out_of_new_and_later_addition_keeps_hint() {
    let env = TestEnv::new();
    let up = repo(&env);
    import(&env, &up, "tools/alpha", "code");

    // 后来新增：beta 在基线之后出现，仍未被决定。
    up.skill(Some("tools"), "gamma", "---\nname: gamma\n---\ngamma v1\n");
    up.commit("add gamma");

    let first = check_all(&env);
    let gamma = first["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["subpath"] == "tools/gamma")
        .cloned()
        .unwrap();
    assert_eq!(gamma["is_new"], true, "基线之后出现且仍存在的候选属于后来新增");
    let beta = first["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["subpath"] == "tools/beta")
        .cloned()
        .unwrap();
    assert_eq!(beta["is_new"], false, "首次已存在但未选的候选不算后来新增");

    // 更新 alpha 不应使未决定的 gamma 丢失提示。
    up.skill(Some("tools"), "alpha", "---\nname: alpha\ndescription: 甲\n---\nalpha v2\n");
    up.commit("update alpha");
    check_all(&env);
    env.cli_json(&["skill", "update", "code/alpha"]);

    let after = check_all(&env);
    let gamma_after = after["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["subpath"] == "tools/gamma")
        .cloned()
        .unwrap();
    assert_eq!(gamma_after["is_new"], true, "更新已有技能不等于处理未决定的新增候选");
}

#[test]
fn paused_copy_is_not_overwritten_by_update() {
    let env = TestEnv::new();
    let up = repo(&env);
    import(&env, &up, "tools/alpha", "code");
    env.cli_json(&["deploy", "to", "codex", "code/alpha"]);

    let copy = env.home.join(".codex/skills/alpha");
    std::fs::write(copy.join("SKILL.md"), "用户手改的副本\n").unwrap();

    up.skill(Some("tools"), "alpha", "---\nname: alpha\ndescription: 甲\n---\nalpha v2\n");
    up.commit("v2");
    check_all(&env);

    let (code, result) = env.cli_json_raw(&["skill", "update", "code/alpha"]);
    assert_ne!(code, 0, "部署同步暂停应如实报告未完成：{result}");
    let message = result["errors"][0]["message"].as_str().unwrap();
    assert!(message.contains("部署") && message.contains("未完成"), "{message}");
    assert_eq!(
        std::fs::read_to_string(copy.join("SKILL.md")).unwrap(),
        "用户手改的副本\n",
        "暂停副本不被覆盖"
    );
    let status = env.cli_json(&["deploy", "status", "--paused"]);
    assert_eq!(status["deployments"].as_array().unwrap().len(), 1);
}
