//! quickstart M5 预设场景（T062～T064）：组合定义、两种排序、叠加部署与按成员撤销、
//! 缺失成员、跨域同 id 不误选、按预设筛选与归属查看。全部通过 CLI 断言库文件与本机记录。

mod common;

use common::TestEnv;
use serde_json::Value;

fn seed(env: &TestEnv) {
    env.write_skill("code", None, "alpha", "alpha 正文\n");
    env.write_skill("code", None, "beta", "beta 正文\n");
    env.write_skill("code", None, "gamma", "gamma 正文\n");
    env.write_skill("lobster", None, "omega", "龙虾正文\n");
}

fn target_copy(env: &TestEnv, rel: &str, name: &str) -> std::path::PathBuf {
    env.home.join(rel).join(name)
}

fn item_skills(items: &Value) -> Vec<String> {
    items
        .as_array()
        .expect("items 应为数组")
        .iter()
        .map(|item| item["skill"].as_str().unwrap().to_string())
        .collect()
}

fn member_names(result: &Value) -> Vec<String> {
    result["members"]
        .as_array()
        .expect("members 应为数组")
        .iter()
        .map(|member| member["name"].as_str().unwrap().to_string())
        .collect()
}

fn preset_ids(list: &Value) -> Vec<String> {
    list["presets"]
        .as_array()
        .expect("presets 应为数组")
        .iter()
        .map(|preset| preset["id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn preset_deploy_stacks_and_undo_removes_members_only() {
    let env = TestEnv::new();
    seed(&env);

    // 已有 A(alpha) 部署在目标上，预设含 B(beta)、C(gamma)。
    let (code, result) = env.cli_json_raw(&["deploy", "to", "codex", "code/alpha"]);
    assert_eq!(code, 0, "{result}");

    let created = env.cli_json(&["preset", "create", "code", "release"]);
    assert_eq!(created["id"], "release");
    let added = env.cli_json(&[
        "preset", "members", "code", "release", "--add", "beta,gamma",
    ]);
    assert_eq!(member_names(&added), vec!["beta", "gamma"]);

    // FR-024：按组叠加部署（保留目标已有其他技能）。
    let (code, result) = env.cli_json_raw(&["deploy", "preset", "code", "release", "--to", "codex"]);
    assert_eq!(code, 0, "{result}");
    for name in ["alpha", "beta", "gamma"] {
        assert!(
            target_copy(&env, ".codex/skills", name).join("SKILL.md").exists(),
            "{name} 应存在于目标（叠加）"
        );
    }

    // FR-024：按成员撤销，保留库内技能与已存在的 alpha。
    let (code, result) = env.cli_json_raw(&[
        "deploy", "undo-preset", "code", "release", "--from", "codex",
    ]);
    assert_eq!(code, 0, "{result}");
    assert!(!target_copy(&env, ".codex/skills", "beta").exists());
    assert!(!target_copy(&env, ".codex/skills", "gamma").exists());
    assert!(target_copy(&env, ".codex/skills", "alpha").exists(), "非成员不受影响");
    for name in ["alpha", "beta", "gamma"] {
        assert!(env.path(&format!("code/{name}/SKILL.md")).exists(), "库内技能必须保留");
    }
}

#[test]
fn undo_preset_removes_members_shared_with_other_presets() {
    let env = TestEnv::new();
    seed(&env);

    env.cli_json(&["preset", "create", "code", "release"]);
    env.cli_json(&["preset", "create", "code", "audit"]);
    env.cli_json(&["preset", "members", "code", "release", "--add", "beta"]);
    env.cli_json(&["preset", "members", "code", "audit", "--add", "beta,gamma"]);

    // beta 由 release 部署，但 beta 同时属于 audit。
    env.cli_json(&["deploy", "preset", "code", "release", "--to", "codex"]);
    assert!(target_copy(&env, ".codex/skills", "beta").exists());

    // 按成员撤销 audit：不追踪来源预设，属于其他预设的 beta 也一并撤销；
    // gamma 从未部署，如实报告为未完成项而不是冒充已撤销。
    let (code, result) = env.cli_json_raw(&[
        "deploy", "undo-preset", "code", "audit", "--from", "codex",
    ]);
    assert_eq!(code, 2, "{result}");
    assert!(!target_copy(&env, ".codex/skills", "beta").exists());
    assert_eq!(item_skills(&result["data"]["items"]), vec!["code/beta"]);
    assert!(result["errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| error["skill"] == "code/gamma"
            && error["message"].as_str().unwrap().contains("未部署")));
}

#[test]
fn editing_members_changes_definition_only_not_deployment() {
    let env = TestEnv::new();
    seed(&env);
    env.cli_json(&["deploy", "to", "codex", "code/beta"]);
    let before = env.cli_json(&["deploy", "status"]);

    env.cli_json(&["preset", "create", "code", "release"]);
    // 定义编辑：加成员、改图标、重命名、排序都不触发部署。
    env.cli_json(&["preset", "members", "code", "release", "--add", "gamma"]);
    env.cli_json(&["preset", "icon", "code", "release", "shield"]);
    env.cli_json(&["preset", "rename", "code", "release", "发布流程"]);
    env.cli_json(&[
        "preset", "reorder-members", "code", "release", "--order", "gamma",
    ]);

    let after = env.cli_json(&["deploy", "status"]);
    assert_eq!(before, after, "编辑预设定义不得改变实际部署");
    assert!(target_copy(&env, ".codex/skills", "beta").exists());
    assert!(!target_copy(&env, ".codex/skills", "gamma").exists());

    let view = env.cli_json(&["preset", "get", "code", "release"]);
    assert_eq!(view["name"], "发布流程");
    assert_eq!(view["icon"], "shield");
}

#[test]
fn missing_member_is_marked_and_does_not_block_others() {
    let env = TestEnv::new();
    seed(&env);
    env.cli_json(&["preset", "create", "code", "release"]);
    env.cli_json(&["preset", "members", "code", "release", "--add", "beta,gamma"]);

    // 删除库内 gamma：成员变为“缺失成员”。
    std::fs::remove_dir_all(env.path("code/gamma")).unwrap();
    let view = env.cli_json(&["preset", "get", "code", "release"]);
    assert_eq!(view["missing_members"], serde_json::json!(["gamma"]));
    let members = view["members"].as_array().unwrap();
    assert_eq!(members[0]["missing"], false);
    assert_eq!(members[1]["missing"], true);

    // 部署：缺失成员逐项报告，其余继续（Outcome::Partial）。
    let (code, result) = env.cli_json_raw(&["deploy", "preset", "code", "release", "--to", "codex"]);
    assert_eq!(code, 2, "缺失成员应如实报告为部分完成：{result}");
    assert_eq!(item_skills(&result["data"]["items"]), vec!["code/beta"]);
    assert!(target_copy(&env, ".codex/skills", "beta").exists());
    assert!(result["errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| error["message"].as_str().unwrap().contains("技能不存在")));
}

#[test]
fn two_display_orders_are_shared_and_keep_members_and_deployment() {
    let env = TestEnv::new();
    seed(&env);
    env.cli_json(&["preset", "create", "code", "release"]);
    env.cli_json(&["preset", "create", "code", "audit"]);
    env.cli_json(&["preset", "members", "code", "release", "--add", "alpha,beta,gamma"]);

    // FR-025：预设间排序。
    let list = env.cli_json(&["preset", "reorder", "code", "--order", "audit,release"]);
    assert_eq!(preset_ids(&list), vec!["audit", "release"]);
    let ordered = env.cli_json(&["preset", "list", "--domain", "code"]);
    assert_eq!(preset_ids(&ordered), vec!["audit", "release"]);

    // 成员显示顺序复用数组顺序，不另建字段。
    let view = env.cli_json(&[
        "preset", "reorder-members", "code", "release", "--order", "gamma,alpha,beta",
    ]);
    assert_eq!(member_names(&view), vec!["gamma", "alpha", "beta"]);

    // 两种排序都不改变成员集合、不动目录、不改变部署。
    assert_eq!(member_names(&env.cli_json(&["preset", "get", "code", "release"])).len(), 3);
    for name in ["alpha", "beta", "gamma"] {
        assert!(env.path(&format!("code/{name}/SKILL.md")).exists());
    }
    let status = env.cli_json(&["deploy", "status"]);
    assert!(status["deployments"].as_array().unwrap().is_empty());

    // 排序必须是当前集合的排列；缺项或未知项被拒绝。
    let (code, result) = env.cli_json_raw(&[
        "preset", "reorder", "code", "--order", "audit",
    ]);
    assert_ne!(code, 0, "{result}");
    let (code, result) = env.cli_json_raw(&[
        "preset", "reorder-members", "code", "release", "--order", "alpha,beta,gamma,delta",
    ]);
    assert_ne!(code, 0, "{result}");
}

#[test]
fn same_id_across_domains_is_not_resolved_silently() {
    let env = TestEnv::new();
    seed(&env);

    // 两域各有一个 id 相同的预设。
    env.cli_json(&["preset", "create", "code", "release"]);
    env.cli_json(&["preset", "create", "lobster", "release"]);
    env.cli_json(&["preset", "members", "code", "release", "--add", "alpha"]);
    env.cli_json(&["preset", "members", "lobster", "release", "--add", "omega"]);

    // 完整引用 (domain, id) 各自解析，不串域。
    let code_view = env.cli_json(&["preset", "get", "code", "release"]);
    assert_eq!(code_view["domain"], "code");
    assert_eq!(member_names(&code_view), vec!["alpha"]);
    let lobster_view = env.cli_json(&["preset", "get", "lobster", "release"]);
    assert_eq!(lobster_view["domain"], "lobster");
    assert_eq!(member_names(&lobster_view), vec!["omega"]);

    // 按预设筛选必须同时给 --domain。
    let (code_exit, result) = env.cli_json_raw(&["skill", "list", "--preset", "release"]);
    assert_ne!(code_exit, 0, "{result}");
    assert!(result["errors"][0]["message"]
        .as_str()
        .unwrap()
        .contains("--domain"));

    let filtered = env.cli_json(&[
        "skill", "list", "--domain", "code", "--preset", "release",
    ]);
    let names: Vec<String> = filtered["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|skill| skill["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["alpha"]);
    // 归属查看：技能带出所属预设。
    assert_eq!(filtered["skills"][0]["presets"][0]["id"], "release");

    // 预设部署同样显式传域：龙虾预设成员不能部署到编码目标。
    let (code_exit, result) = env.cli_json_raw(&[
        "deploy", "preset", "lobster", "release", "--to", "codex",
    ]);
    assert_eq!(code_exit, 1, "{result}");
    assert!(result["errors"][0]["message"]
        .as_str()
        .unwrap()
        .contains("管理域边界"));
}

#[test]
fn removed_preview_flag_is_rejected() {
    let env = TestEnv::new();
    seed(&env);
    for args in [vec!["deploy", "preset", "code", "release", "--to", "codex", "--dry-run"], vec!["deploy", "undo-preset", "code", "release", "--from", "codex", "--dry-run"]] {
        let output = env.cli(&args);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("--dry-run"));
    }
    assert!(env.cli_json(&["deploy", "status"])["deployments"].as_array().unwrap().is_empty());
}

#[test]
fn preset_members_must_resolve_within_the_same_domain() {
    let env = TestEnv::new();
    seed(&env);
    env.cli_json(&["preset", "create", "code", "release"]);

    let (code, result) = env.cli_json_raw(&[
        "preset", "members", "code", "release", "--add", "omega",
    ]);
    assert_ne!(code, 0, "{result}");
    assert!(result["errors"][0]["message"]
        .as_str()
        .unwrap()
        .contains("编码库中不存在"));

    // 未知 id 的完整引用同样报错，不静默创建。
    let (code, result) = env.cli_json_raw(&["preset", "get", "code", "nope"]);
    assert_ne!(code, 0, "{result}");
}
