//! US6：通过 Hermes 档案使用龙虾技能（T057）。
//!
//! 夹具按 research.md §五 对**当前真实 Hermes 安装**的只读核实结论构造：
//! 根 = `<临时 home>/.hermes`（default 技能目录 `<root>/skills`），具名档案 =
//! `<root>/profiles/<名>`（含身份标记 `config.yaml`），墓碑在 `profiles/.deleted/<名>`。
//! 全程只在临时目录内读写；不触碰真实 `~/.hermes`。
//!
//! 场景：明确启用、不自动扩散、内容更新保持选择、支持收窄保留已有启用且阻止新增、
//! 档案独有技能收编至龙虾库和 default、批量启停。

mod common;

use common::TestEnv;
use serde_json::Value;

fn names(items: &Value) -> Vec<String> {
    items
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|item| item.get("name").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn find_item<'a>(items: &'a Value, name: &str) -> Option<&'a Value> {
    items
        .as_array()?
        .iter()
        .find(|item| item.get("name").and_then(Value::as_str) == Some(name))
}

fn profile_item<'a>(skills: &'a Value, name: &str) -> &'a Value {
    find_item(skills, name).unwrap_or_else(|| panic!("档案技能列表缺少 {name}：{skills}"))
}

/// 在临时 HOME 内搭好假 Hermes 根与两个具名档案。
fn hermes_env() -> TestEnv {
    let env = TestEnv::new();
    env.hermes_init();
    env.hermes_profile("blank");
    env.hermes_profile("drawing");
    env
}

#[test]
fn read_profile_local_skill_does_not_adopt_or_change_enable_state() {
    let env = hermes_env();
    let dir = env.hermes_profile_skill("blank", "local-only", "# 档案独有正文\n");
    std::fs::create_dir_all(dir.join("references")).unwrap();
    std::fs::write(dir.join("references/guide.md"), "独有文件").unwrap();
    let before = env.cli_json(&["hermes", "list", "blank"]);
    let body = env.cli_json(&["hermes", "read", "blank", "local-only"]);
    assert_eq!(body["text"], "# 档案独有正文\n");
    let file = env.cli_json(&["hermes", "read", "blank", "local-only", "--file", "references/guide.md"]);
    assert_eq!(file["text"], "独有文件");
    assert_ne!(env.cli_json_raw(&["hermes", "read", "blank", "local-only", "--file", "../../config.yaml"]).0, 0);
    assert!(!env.path("lobster/local-only").exists());
    assert!(!env.hermes_root().join("skills/local-only").exists());
    assert_eq!(env.cli_json(&["hermes", "list", "blank"]), before);
}

#[test]
fn profiles_lists_default_and_identity_profiles_only() {
    let env = hermes_env();
    // 无身份标记的副作用目录与已删除（有墓碑）的目录都不应被认作档案。
    env.hermes_stray_dir("sideeffect");
    let env2 = hermes_env();
    env2.hermes_profile("gone");
    env2.hermes_tombstone("gone");

    let data = env.cli_json(&["hermes", "profiles"]);
    let listed = names(&data["profiles"]);
    assert!(listed.contains(&"default".to_string()), "{listed:?}");
    assert!(listed.contains(&"blank".to_string()), "{listed:?}");
    assert!(listed.contains(&"drawing".to_string()), "{listed:?}");
    assert!(!listed.contains(&"sideeffect".to_string()), "{listed:?}");
    assert!(
        data["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["name"] == "default" && item["kind"] == "default"),
        "{data}"
    );

    let data2 = env2.cli_json(&["hermes", "profiles"]);
    let listed2 = names(&data2["profiles"]);
    assert!(!listed2.contains(&"gone".to_string()), "{listed2:?}");
}

#[test]
fn enable_is_explicit_and_does_not_spread() {
    let env = hermes_env();
    env.write_skill("lobster", None, "alpha", "# alpha\n");
    env.write_skill("lobster", None, "beta", "# beta\n");
    env.commit_all("加入龙虾技能");
    env.cli_json(&["deploy", "to", "hermes:default", "lobster/alpha"]);
    env.cli_json(&["deploy", "to", "hermes:default", "lobster/beta"]);

    // 明确为 blank 启用 alpha。
    let result = env.cli_json(&["hermes", "enable", "blank", "alpha"]);
    assert_eq!(result["items"][0]["ok"], true, "{result}");

    let link = env
        .hermes_root()
        .join("profiles/blank/skills/alpha");
    assert!(TestEnv::is_symlink(&link), "应建立指向 default 副本的链接");
    assert_eq!(
        std::fs::canonicalize(&link).unwrap(),
        std::fs::canonicalize(env.hermes_root().join("skills/alpha")).unwrap(),
        "链接应指向 default 档案共享副本"
    );

    // beta 已部署到 default，但未对 blank 启用：不自动扩散。
    assert!(
        !env.hermes_root()
            .join("profiles/blank/skills/beta")
            .exists(),
        "新部署到 default 的技能不应自动出现在其他档案"
    );

    let list = env.cli_json(&["hermes", "list", "blank"]);
    assert_eq!(profile_item(&list["skills"], "alpha")["enabled"], true, "{list}");
    assert_eq!(profile_item(&list["skills"], "beta")["enabled"], false, "{list}");
}

#[test]
fn content_update_keeps_selection() {
    let env = hermes_env();
    env.write_skill("lobster", None, "alpha", "# alpha v1\n");
    env.commit_all("加入龙虾技能");
    env.cli_json(&["deploy", "to", "hermes:default", "lobster/alpha"]);
    env.cli_json(&["hermes", "enable", "blank", "alpha"]);

    // 库内内容更新并同步（档案共用同一份内容）。
    env.write_skill("lobster", None, "alpha", "# alpha v2\n");
    env.commit_all("更新龙虾技能");
    env.cli_json(&["deploy", "sync"]);

    let list = env.cli_json(&["hermes", "list", "blank"]);
    assert_eq!(
        profile_item(&list["skills"], "alpha")["enabled"],
        true,
        "内容更新不得重置启用选择：{list}"
    );
    assert!(
        env.hermes_read("skills/alpha/SKILL.md")
            .is_some_and(|text| text.contains("v2")),
        "default 副本应更新到同一份新内容"
    );
    let link = env.hermes_root().join("profiles/blank/skills/alpha");
    assert!(TestEnv::is_symlink(&link), "启用链接应保持");
}

#[test]
fn support_narrowing_keeps_existing_enable_and_blocks_new() {
    let env = hermes_env();
    env.write_skill("lobster", None, "gamma", "# gamma\n");
    env.commit_all("加入龙虾技能");
    env.cli_json(&["deploy", "to", "hermes:default", "lobster/gamma"]);
    env.cli_json(&["hermes", "enable", "blank", "gamma"]);

    // 收窄支持范围：不再声明支持 hermes。
    env.cli_json(&["skill", "set", "lobster/gamma", "--agents", "codex"]);

    // 已有启用保留并警告。
    let list = env.cli_json(&["hermes", "list", "blank"]);
    let item = profile_item(&list["skills"], "gamma");
    assert_eq!(item["enabled"], true, "已有启用应保留：{list}");
    assert!(
        item["warning"].as_str().is_some_and(|text| text.contains("支持")),
        "应给出支持收窄警告：{item}"
    );

    // 新增到另一档案被阻止。
    let (code, result) = env.cli_json_raw(&["hermes", "enable", "drawing", "gamma"]);
    assert_ne!(code, 0, "不支持的目标不得新增启用：{result}");
    assert_eq!(result["result"], "failed", "{result}");
    assert!(
        result["errors"]
            .as_array()
            .is_some_and(|errors| errors
                .iter()
                .any(|error| error["message"]
                    .as_str()
                    .is_some_and(|text| text.contains("禁止新增启用")))),
        "应报告禁止新增启用：{result}"
    );
    assert!(
        !env.hermes_root()
            .join("profiles/drawing/skills/gamma")
            .exists()
    );
}

#[test]
fn adopt_profile_skill_into_lobster_and_default() {
    let env = hermes_env();
    env.hermes_profile_skill("blank", "mydog", "# mydog\n");

    let result = env.cli_json(&["skill", "adopt-hermes", "blank", "mydog"]);
    assert_eq!(result["items"][0]["ok"], true, "{result}");

    // 进入龙虾库并部署到 default。
    assert!(env.vault.join("lobster/mydog/SKILL.md").exists(), "应收编进龙虾库");
    let list = env.cli_json(&["hermes", "list", "default"]);
    assert_eq!(profile_item(&list["skills"], "mydog")["enabled"], true, "{list}");

    // 原档案启用状态保持（仍可用）。
    let blank = env.cli_json(&["hermes", "list", "blank"]);
    assert_eq!(profile_item(&blank["skills"], "mydog")["enabled"], true, "{blank}");

    // 同名阻止：再次收编失败。
    let (code, again) = env.cli_json_raw(&["skill", "adopt-hermes", "blank", "mydog"]);
    assert_ne!(code, 0, "同域同名应阻止：{again}");
}

#[test]
fn batch_enable_disable() {
    let env = hermes_env();
    for name in ["a", "b", "c"] {
        env.write_skill("lobster", None, name, &format!("# {name}\n"));
    }
    env.commit_all("加入龙虾技能");
    env.cli_json(&[
        "deploy",
        "to",
        "hermes:default",
        "lobster/a",
        "lobster/b",
        "lobster/c",
    ]);

    let enabled = env.cli_json(&["hermes", "enable", "blank", "a", "b"]);
    assert_eq!(enabled["items"].as_array().unwrap().len(), 2, "{enabled}");
    let list = env.cli_json(&["hermes", "list", "blank"]);
    assert_eq!(profile_item(&list["skills"], "a")["enabled"], true);
    assert_eq!(profile_item(&list["skills"], "b")["enabled"], true);
    assert_eq!(profile_item(&list["skills"], "c")["enabled"], false);

    let disabled = env.cli_json(&["hermes", "disable", "blank", "a", "b"]);
    assert_eq!(disabled["items"].as_array().unwrap().len(), 2, "{disabled}");
    let list = env.cli_json(&["hermes", "list", "blank"]);
    assert_eq!(profile_item(&list["skills"], "a")["enabled"], false);
    assert_eq!(profile_item(&list["skills"], "b")["enabled"], false);
    assert!(
        !env.hermes_root().join("profiles/blank/skills/a").exists(),
        "禁用应移除档案内的启用链接"
    );
}
