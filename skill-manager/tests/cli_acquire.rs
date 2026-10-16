//! T031：quickstart M3 获取场景（US2）。
//! 默认分类、部分入库、继承 `.manager`、同名阻止、跨域同名、自建/复制不部署、单项/批量收编接管。

mod common;

use common::TestEnv;
use serde_json::Value;

#[test]
fn failed_manual_acquisition_is_available_in_diagnostics() {
    let env = TestEnv::new();
    let missing = env.root.path().join("missing-source");
    for action in ["preview", "import"] {
        let mut args = vec!["skill", action, missing.to_str().unwrap()];
        if action == "import" {
            args.extend(["--domain", "code", "--skills", "alpha"]);
        }
        let (code, _) = env.cli_json_raw(&args);
        assert_ne!(code, 0);
    }
    let report = env.cli_json(&["settings", "diagnostics"]);
    let logs = report["recent_logs"].as_array().unwrap();
    for action in ["acquire.preview", "acquire.import"] {
        let entry = logs.iter().find(|entry| entry["action"] == action).unwrap();
        assert!(!entry["at"].as_str().unwrap().is_empty());
        assert!(entry["message"].as_str().unwrap().contains("missing-source"));
    }
}

fn skill_exists(env: &TestEnv, rel: &str) -> bool {
    env.path(rel).join("SKILL.md").exists()
}

fn multi_skill_repo(env: &TestEnv) -> common::UpstreamRepo {
    let up = env.upstream("upstream");
    up.skill(Some("tools"), "alpha", "---\nname: alpha\ndescription: 甲\n---\nalpha 正文\n");
    up.skill(Some("tools"), "beta", "---\nname: beta\ndescription: 乙\n---\nbeta 正文\n");
    up.commit("初始");
    up
}

#[test]
fn multi_skill_repo_defaults_to_repo_name_category_and_imports_only_selected() {
    let env = TestEnv::new();
    let up = multi_skill_repo(&env);

    let data = env.cli_json(&[
        "skill", "import", &up.url(),
        "--domain", "code",
        "--skills", "tools/alpha",
    ]);
    let items = data["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["skill"], "code/alpha");
    assert_eq!(items[0]["category"], "upstream", "多技能仓库默认按仓库名分类");
    assert_eq!(items[0]["name"], "alpha", "保留上游原名（目录身份）");
    assert!(!skill_exists(&env, "code/upstream/beta"), "未选中的技能不入库");
}

#[test]
fn single_skill_repo_is_not_auto_categorized() {
    let env = TestEnv::new();
    let up = env.upstream("single");
    up.skill(None, "solo", "---\nname: solo\n---\n正文\n");
    up.commit("初始");

    let data = env.cli_json(&[
        "skill", "import", &up.url(),
        "--domain", "code",
        "--skills", "solo",
    ]);
    assert_eq!(data["items"][0]["category"], Value::Null, "单技能仓库不自动分类");
    assert!(skill_exists(&env, "code/solo"));
}

#[test]
fn import_inherits_upstream_manager_material() {
    let env = TestEnv::new();
    let up = env.upstream("managed");
    up.skill(Some("tools"), "alpha", "---\nname: alpha\ndescription: 甲\n---\n正文\n");
    up.write("tools/alpha/.manager/meta.yaml", "comment: 上游注释\ntags: [上游标签]\n");
    up.write("tools/alpha/.manager/translation.md", "上游中文译文\n");
    up.write("tools/alpha/.manager/overlay/_/_/notes.txt", "覆盖层补充\n");
    up.commit("初始");

    let preview = env.cli_json(&["skill", "preview", &up.url()]);
    assert_eq!(preview["candidates"][0]["comment"], "上游注释");
    assert_eq!(preview["candidates"][0]["description"], "甲");
    assert_eq!(preview["candidates"][0]["tags"], serde_json::json!(["上游标签"]));
    assert!(!skill_exists(&env, "code/alpha"), "阅读来源资料不入库");

    env.cli_json(&[
        "skill", "import", &up.url(),
        "--domain", "code",
        "--skills", "tools/alpha",
    ]);

    let detail = env.cli_json(&["skill", "get", "code/alpha"]);
    assert_eq!(detail["skill"]["comment"], "上游注释");
    assert_eq!(detail["skill"]["tags"], serde_json::json!(["上游标签"]));
    assert_eq!(detail["content"]["has_translation"], true);
    assert_eq!(detail["skill"]["agents"], Value::Null, "缺声明=域内全支持");
    assert!(env.path("code/alpha/.manager/overlay/_/_/notes.txt").exists());
}

#[test]
fn same_domain_case_insensitive_name_is_blocked() {
    let env = TestEnv::new();
    env.write_skill("code", None, "Alpha", "已有的库内技能\n");
    let up = env.upstream("dup");
    up.skill(Some("tools"), "alpha", "---\nname: alpha\n---\n正文\n");
    up.commit("初始");

    let (code, result) = env.cli_json_raw(&[
        "skill", "import", &up.url(),
        "--domain", "code",
        "--skills", "tools/alpha",
    ]);
    assert_eq!(code, 1, "{result}");
    assert_eq!(result["result"], "failed");
    let message = result["errors"][0]["message"].as_str().unwrap();
    assert!(message.contains("同名"), "{message}");
}

#[test]
fn cross_domain_same_name_is_allowed_and_copy_is_independent() {
    let env = TestEnv::new();
    let up = multi_skill_repo(&env);
    env.cli_json(&[
        "skill", "import", &up.url(),
        "--domain", "code",
        "--skills", "tools/alpha",
    ]);

    let copied = env.cli_json(&["skill", "copy", "code/alpha", "--to", "lobster"]);
    assert_eq!(copied["skill"]["domain"], "lobster");
    assert_eq!(copied["skill"]["name"], "alpha");
    assert_eq!(copied["skill"]["source"]["subpath"], "tools/alpha", "副本保留来源记录");
    assert!(skill_exists(&env, "code/upstream/alpha"), "原件保留");

    // 两份独立维护：改副本不影响原件。
    let copy_dir = env.path("lobster/alpha");
    std::fs::write(copy_dir.join("SKILL.md"), "副本被独立修改\n").unwrap();
    let original = std::fs::read_to_string(env.path("code/upstream/alpha/SKILL.md")).unwrap();
    assert!(original.contains("alpha 正文"), "跨域副本不互相同步：{original}");
}

#[test]
fn create_and_copy_do_not_deploy() {
    let env = TestEnv::new();
    env.cli_json(&[
        "skill", "create", "code/new-skill",
        "--comment", "自建技能",
        "--tags", "a,b",
    ]);
    assert!(skill_exists(&env, "code/new-skill"));
    assert_eq!(
        std::fs::read_to_string(env.path("code/new-skill/.manager/meta.yaml")).unwrap().contains("自建技能"),
        true
    );

    let up = multi_skill_repo(&env);
    env.cli_json(&[
        "skill", "import", &up.url(),
        "--domain", "code",
        "--skills", "tools/alpha",
    ]);
    env.cli_json(&["skill", "copy", "code/alpha", "--to", "lobster"]);

    let status = env.cli_json(&["deploy", "status"]);
    assert!(
        status["deployments"].as_array().unwrap().is_empty(),
        "自建与复制都不部署"
    );
}

#[test]
fn adopt_single_and_batch_take_over_existing_deployment_only() {
    let env = TestEnv::new();
    env.agent_skill(".codex/skills", "local1", "---\nname: local1\n---\n本地一\n");
    env.agent_skill(".codex/skills", "local2", "---\nname: local2\n---\n本地二\n");

    let data = env.cli_json(&[
        "skill", "adopt", "codex", "local1", "local2",
        "--comment", "收编注释",
    ]);
    assert_eq!(data["adopted"], 2);
    assert!(skill_exists(&env, "code/local1"));
    assert!(skill_exists(&env, "code/local2"));

    let status = env.cli_json(&["deploy", "status"]);
    let records = status["deployments"].as_array().unwrap();
    assert_eq!(records.len(), 2, "仅接管原 Agent 部署，不新增其他目标：{records:?}");
    for record in records {
        assert_eq!(record["target"], "agent:codex");
        assert_eq!(record["skill"].as_str().unwrap().starts_with("code/"), true);
    }

    // 接管后的 Agent 副本是受管产物：注释已注入，且不含 .manager。
    let copy = env.home.join(".codex/skills/local1");
    let entry = std::fs::read_to_string(copy.join("SKILL.md")).unwrap();
    assert!(entry.contains("收编注释"), "{entry}");
    assert!(!copy.join(".manager").exists());
}
