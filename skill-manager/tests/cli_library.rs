//! quickstart M1 1.4～1.8 的 CLI 场景：JSON schema、正文选择、分类移动、标签治理与退出码。
//!
//! 全部使用预置好的库文件，不依赖尚未实现的 `skill import`。

mod common;

use std::path::PathBuf;

use common::{find_skill, TestEnv};
use serde_json::Value;

/// 预置一个双域、带分类与标签的库。
fn seed(env: &TestEnv) {
    env.write_skill(
        "code",
        Some("工具"),
        "alpha",
        "---\nname: alpha\ndescription: 解析依赖\n---\n原文正文\n",
    );
    env.write_manager(
        "code",
        Some("工具"),
        "alpha",
        "meta.yaml",
        "tags: [x, y]\ncomment: 检查依赖\n",
    );
    env.write_manager(
        "code",
        Some("工具"),
        "alpha",
        "source.yaml",
        "repo: https://example.invalid/skills.git\nref: main\nsubpath: tools/alpha\nversion: 0123456789abcdef0123456789abcdef01234567\nregistered_at: 2026-10-15\n",
    );

    env.write_skill("code", Some("工具"), "beta", "beta 正文\n");
    env.write_manager("code", Some("工具"), "beta", "meta.yaml", "tags: [y]\n");

    env.write_skill("code", None, "gamma", "gamma 正文\n");

    env.write_skill("lobster", None, "alpha", "龙虾 alpha 正文\n");
    env.write_manager("lobster", None, "alpha", "meta.yaml", "tags: [l]\n");

    env.write_skill("lobster", None, "omega", "omega 正文\n");
}

fn names(data: &Value) -> Vec<String> {
    data["skills"]
        .as_array()
        .expect("skills 应为数组")
        .iter()
        .map(|entry| {
            format!(
                "{}/{}",
                entry["domain"].as_str().unwrap(),
                entry["name"].as_str().unwrap()
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 1.8 稳定 JSON schema
// ---------------------------------------------------------------------------

#[test]
fn list_json_has_stable_schema_and_filters() {
    let env = TestEnv::new();
    seed(&env);

    let data = env.cli_json(&["skill", "list"]);
    let skills = data["skills"].as_array().expect("skills 数组");
    assert_eq!(skills.len(), 5, "{data}");
    assert_eq!(data["total"].as_u64(), Some(5));
    assert!(data["tag_candidates"].as_array().is_some(), "{data}");

    // 稳定字段（缺资料如实为 null，不省略键）。
    let alpha = find_skill(&data["skills"], "code", "alpha").expect("code/alpha");
    for key in [
        "domain",
        "name",
        "category",
        "comment",
        "tags",
        "description",
        "agents",
        "oses",
        "has_translation",
        "translation_stale",
        "vault_rel_dir",
        "path",
        "entry_file",
        "source",
        "manager_error",
    ] {
        assert!(alpha.get(key).is_some(), "缺少字段 {key}：{alpha}");
    }
    assert_eq!(alpha["category"], "工具");
    assert_eq!(alpha["comment"], "检查依赖");
    assert_eq!(alpha["description"], "解析依赖");
    assert_eq!(alpha["tags"], serde_json::json!(["x", "y"]));
    assert!(alpha["agents"].is_null(), "缺省声明应如实为 null");
    assert!(alpha["oses"].is_null());
    assert_eq!(alpha["has_translation"], false);
    assert_eq!(alpha["translation_stale"], false);
    assert_eq!(alpha["vault_rel_dir"], "code/工具/alpha");
    assert_eq!(alpha["entry_file"], "SKILL.md");
    assert_eq!(alpha["source"]["repo"], "https://example.invalid/skills.git");
    assert_eq!(alpha["source"]["ref"], "main");
    assert_eq!(alpha["source"]["subpath"], "tools/alpha");
    assert_eq!(alpha["source"]["version"].as_str().unwrap().len(), 40);

    // 未分类技能：category 为 null，source 为 null。
    let gamma = find_skill(&data["skills"], "code", "gamma").expect("code/gamma");
    assert!(gamma["category"].is_null());
    assert!(gamma["source"].is_null());
    assert_eq!(gamma["tags"], serde_json::json!([]));

    // 域筛选。
    let code_only = env.cli_json(&["skill", "list", "--domain", "code"]);
    assert_eq!(names(&code_only).len(), 3);
    let lobster_only = env.cli_json(&["skill", "list", "--domain", "lobster"]);
    assert_eq!(
        names(&lobster_only),
        vec!["lobster/alpha".to_string(), "lobster/omega".into()]
    );

    // 分类 / 未分类。
    let in_tools = env.cli_json(&["skill", "list", "--category", "工具"]);
    assert_eq!(
        names(&in_tools),
        vec!["code/alpha".to_string(), "code/beta".into()]
    );
    let uncategorized = env.cli_json(&["skill", "list", "--uncategorized"]);
    assert_eq!(
        names(&uncategorized),
        vec!["code/gamma".to_string(), "lobster/alpha".into(), "lobster/omega".into()]
    );

    // 文本搜索覆盖描述、注释、来源、标签。
    for needle in ["解析依赖", "检查依赖", "example.invalid"] {
        let found = env.cli_json(&["skill", "list", "--query", needle]);
        assert_eq!(names(&found), vec!["code/alpha".to_string()], "{needle}");
    }
    let q = env.cli_json(&["skill", "list", "--query", "点名"]);
    assert!(names(&q).is_empty(), "{q}");

    // 标签筛选（多标签匹配任一）与无标签筛选。
    let tagged = env.cli_json(&["skill", "list", "--tag", "y"]);
    assert_eq!(
        names(&tagged),
        vec!["code/alpha".to_string(), "code/beta".into()]
    );
    let untagged = env.cli_json(&["skill", "list", "--no-tag"]);
    assert_eq!(
        names(&untagged),
        vec!["code/gamma".to_string(), "lobster/omega".into()]
    );

    // 分类联动标签候选：选中分类只含该分类内实际标签；全部分类恢复全部。
    assert_eq!(
        in_tools["tag_candidates"],
        serde_json::json!(["x", "y"]),
        "{in_tools}"
    );
    assert_eq!(
        data["tag_candidates"],
        serde_json::json!(["l", "x", "y"]),
        "{data}"
    );

    // 支持声明筛选：预置技能都未声明=域内全支持。
    let by_agent = env.cli_json(&["skill", "list", "--agent", "codex"]);
    assert_eq!(names(&by_agent).len(), 3);
}

#[test]
fn preset_filter_requires_domain_and_deployed_filter_uses_records() {
    let env = TestEnv::new();
    seed(&env);

    // `--preset` 必须同时给 `--domain`，避免跨域同 id 歧义；给了域但预设无成员时返回空。
    let (code, value) = env.cli_json_raw(&["skill", "list", "--preset", "rust"]);
    assert_eq!(code, 1, "{value}");
    assert_eq!(value["result"], "failed");
    assert!(
        value["errors"][0]["message"]
            .as_str()
            .unwrap()
            .contains("--domain"),
        "{value}"
    );
    let data = env.cli_json(&["skill", "list", "--domain", "code", "--preset", "rust"]);
    assert_eq!(data["total"], 0, "{data}");

    // M2 起 `--deployed-to` 读取真实部署记录：没有记录时不返回任何技能。
    let (code, value) = env.cli_json_raw(&["skill", "list", "--deployed-to", "codex"]);
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["data"]["total"], 0, "{value}");
    assert!(value["data"]["skills"].as_array().unwrap().is_empty());
}

#[test]
fn get_json_returns_detail_and_empty_deployments() {
    let env = TestEnv::new();
    seed(&env);

    let data = env.cli_json(&["skill", "get", "code/alpha"]);
    assert_eq!(data["skill"]["name"], "alpha");
    assert_eq!(data["skill"]["vault_rel_dir"], "code/工具/alpha");
    assert_eq!(data["content"]["original"], "---\nname: alpha\ndescription: 解析依赖\n---\n原文正文\n");
    assert!(data["content"]["translation"].is_null());
    assert_eq!(data["content"]["has_translation"], false);
    assert_eq!(data["content"]["translation_stale"], false);
    assert_eq!(data["content"]["default_lang"], "orig");

    assert_eq!(data["vault_path"].as_str().unwrap().ends_with("vault"), true);
    // 文件树：带目录层级，含 .manager 与技能文件。
    let root = data["files"].as_array().expect("files 数组");
    assert!(
        root.iter().any(|node| node["path"] == "SKILL.md" && node["is_dir"] == false),
        "{data}"
    );
    let manager = root.iter().find(|node| node["path"] == ".manager").expect(".manager 目录");
    assert_eq!(manager["is_dir"], true);
    assert!(
        manager["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|child| child["path"] == ".manager/meta.yaml"),
        "{manager}"
    );

    // M2 起部署位置来自本机记录；此处没有部署，如实为空数组并说明。
    assert_eq!(data["deployments"], serde_json::json!([]));
    assert!(
        data["deployments_note"]
            .as_str()
            .unwrap()
            .contains("没有部署记录"),
        "{data}"
    );
}

// ---------------------------------------------------------------------------
// 1.3 / 1.8 正文选择
// ---------------------------------------------------------------------------

#[test]
fn read_prefers_translation_and_falls_back_to_original() {
    let env = TestEnv::new();
    seed(&env);

    // 无译文：显示原文。
    let read = env.cli_json(&["skill", "read", "code/alpha"]);
    assert_eq!(read["lang"], "orig");
    assert_eq!(read["text"], "---\nname: alpha\ndescription: 解析依赖\n---\n原文正文\n");
    assert_eq!(read["translation_stale"], false);

    // 保存译文后默认中文。
    env.write(
        "code/工具/alpha/.manager/translation.md",
        "中文译文\n",
    );
    // 直接写文件不会更新时效依据，因此先按当前原文保存。
    let set = env.cli_json_stdin(&["skill", "translation", "set", "code/alpha", "--stdin"], "中文译文\n");
    assert_eq!(set["exists"], true);
    assert_eq!(set["translation_stale"], false);

    let read = env.cli_json(&["skill", "read", "code/alpha"]);
    assert_eq!(read["lang"], "zh");
    assert_eq!(read["text"], "中文译文\n");

    // 显式要原文。
    let read = env.cli_json(&["skill", "read", "code/alpha", "--lang", "orig"]);
    assert_eq!(read["lang"], "orig");
    assert!(read["text"].as_str().unwrap().contains("原文正文"));

    // 原文变化 → 提示待更新，译文保留。
    env.write("code/工具/alpha/SKILL.md", "改过的原文\n");
    let read = env.cli_json(&["skill", "read", "code/alpha"]);
    assert_eq!(read["text"], "中文译文\n");
    assert_eq!(read["translation_stale"], true);

    let get = env.cli_json(&["skill", "get", "code/alpha"]);
    assert_eq!(get["content"]["translation_stale"], true);
    assert_eq!(get["content"]["translation"], "中文译文\n");

    // 按当前原文重新保存即解除。
    let set = env.cli_json_stdin(&["skill", "translation", "set", "code/alpha", "--stdin"], "新译文\n");
    assert_eq!(set["translation_stale"], false);

    // 删除译文后顺事实为空且不再提示。
    let removed = env.cli_json(&["skill", "translation", "remove", "code/alpha"]);
    assert_eq!(removed["exists"], false);
    let read = env.cli_json(&["skill", "read", "code/alpha"]);
    assert_eq!(read["lang"], "orig");
    assert_eq!(read["translation_stale"], false);

    // 无译文的技能 get 时译文为 null。
    let get = env.cli_json(&["skill", "get", "code/beta"]);
    assert!(get["content"]["translation"].is_null());
    assert_eq!(get["content"]["default_lang"], "orig");
}

#[test]
fn translation_file_write_via_file_flag() {
    let env = TestEnv::new();
    seed(&env);
    let source = env.root.path().join("译文.md");
    std::fs::write(&source, "来自文件的译文\n").unwrap();

    let data = env.cli_json(&[
        "skill",
        "translation",
        "set",
        "code/gamma",
        "--file",
        source.to_str().unwrap(),
    ]);
    assert_eq!(data["exists"], true);
    assert_eq!(
        env.read("code/gamma/.manager/translation.md").as_deref(),
        Some("来自文件的译文\n")
    );
}

// ---------------------------------------------------------------------------
// 1.4 分类移动
// ---------------------------------------------------------------------------

#[test]
fn move_category_moves_directory_and_stays_findable() {
    let env = TestEnv::new();
    seed(&env);

    let data = env.cli_json(&["skill", "move", "code/gamma", "--category", "脚本"]);
    assert_eq!(data["skill"]["name"], "gamma");
    assert_eq!(data["skill"]["category"], "脚本");
    assert_eq!(data["skill"]["vault_rel_dir"], "code/脚本/gamma");

    assert!(!env.path("code/gamma").exists());
    assert!(env.path("code/脚本/gamma/SKILL.md").exists());

    let found = env.cli_json(&["skill", "list", "--category", "脚本"]);
    assert_eq!(names(&found), vec!["code/gamma".to_string()]);

    // 移回未分类。
    let data = env.cli_json(&["skill", "move", "code/gamma", "--uncategorized"]);
    assert!(data["skill"]["category"].is_null());
    assert!(env.path("code/gamma/SKILL.md").exists());
    assert!(!env.path("code/脚本/gamma").exists());

    // 管理资料随目录一起移动。
    env.cli_json(&["skill", "move", "code/alpha", "--category", "运维"]);
    assert!(env.path("code/运维/alpha/.manager/meta.yaml").exists());
    assert!(env.path("code/运维/alpha/.manager/source.yaml").exists());
}

// ---------------------------------------------------------------------------
// 1.5 / 1.6 管理资料写入
// ---------------------------------------------------------------------------

#[test]
fn set_updates_tags_comment_and_support_keeping_unknown_fields() {
    let env = TestEnv::new();
    seed(&env);
    env.write_manager(
        "code",
        Some("工具"),
        "alpha",
        "meta.yaml",
        "foo: bar\ntags: [x, y]\ncomment: 检查依赖\n",
    );

    let data = env.cli_json(&[
        "skill", "set", "code/alpha", "--comment", "新注释", "--tags", "a,b", "--agents", "codex",
        "--oses", "all",
    ]);
    assert_eq!(data["skill"]["comment"], "新注释");
    assert_eq!(data["skill"]["tags"], serde_json::json!(["a", "b"]));
    assert_eq!(data["skill"]["agents"], serde_json::json!(["codex"]));
    assert!(data["skill"]["oses"].is_null());

    let raw = env.read("code/工具/alpha/.manager/meta.yaml").unwrap();
    assert!(raw.contains("foo: bar"), "未知字段应保留：{raw}");
    assert!(raw.contains("codex"), "{raw}");
    assert!(!raw.contains("oses"), "all 应删除该键：{raw}");

    // 未传的参数保持原声明；本机未显示的声明不会被清掉。
    let data = env.cli_json(&["skill", "set", "code/alpha", "--comment", "再改注释"]);
    assert_eq!(data["skill"]["agents"], serde_json::json!(["codex"]));
    assert_eq!(data["skill"]["tags"], serde_json::json!(["a", "b"]));

    // none 写显式空集合；all 删除键恢复全支持。
    let data = env.cli_json(&["skill", "set", "code/alpha", "--agents", "none"]);
    assert_eq!(data["skill"]["agents"], serde_json::json!([]));
    let raw = env.read("code/工具/alpha/.manager/meta.yaml").unwrap();
    assert!(raw.contains("agents: []"), "{raw}");

    let data = env.cli_json(&["skill", "set", "code/alpha", "--agents", "all"]);
    assert!(data["skill"]["agents"].is_null());
    let raw = env.read("code/工具/alpha/.manager/meta.yaml").unwrap();
    assert!(!raw.contains("agents"), "{raw}");
    assert!(raw.contains("foo: bar"), "{raw}");

    // 空字符串清空标签集合。
    let data = env.cli_json(&["skill", "set", "code/alpha", "--tags", ""]);
    assert_eq!(data["skill"]["tags"], serde_json::json!([]));
}

#[test]
fn external_file_edits_are_visible_on_next_invocation() {
    let env = TestEnv::new();
    seed(&env);

    env.write_manager(
        "code",
        Some("工具"),
        "alpha",
        "meta.yaml",
        "tags: [外部标签]\ncomment: 外部改的注释\n",
    );
    let data = env.cli_json(&["skill", "list", "--query", "外部改的注释"]);
    assert_eq!(names(&data), vec!["code/alpha".to_string()]);
    let alpha = find_skill(&data["skills"], "code", "alpha").unwrap();
    assert_eq!(alpha["tags"], serde_json::json!(["外部标签"]));
}

// ---------------------------------------------------------------------------
// 1.5 全库标签治理
// ---------------------------------------------------------------------------

#[test]
fn tag_rename_and_remove_cover_both_domains() {
    let env = TestEnv::new();
    seed(&env);

    let data = env.cli_json(&["skill", "tag", "rename", "x", "公用"]);
    assert_eq!(data["affected"], 1);
    assert!(env.read("code/工具/alpha/.manager/meta.yaml").unwrap().contains("公用"));

    // 跨域重命名：两库同名标签都生效。
    let data = env.cli_json(&["skill", "tag", "add", "code/beta", "lobster/alpha", "--tags", "共用"]);
    assert_eq!(data["skills"], serde_json::json!(["code/beta", "lobster/alpha"]));
    let data = env.cli_json(&["skill", "tag", "rename", "共用", "共享"]);
    assert_eq!(data["affected"], 2);
    assert!(env.read("code/工具/beta/.manager/meta.yaml").unwrap().contains("共享"));
    assert!(env.read("lobster/alpha/.manager/meta.yaml").unwrap().contains("共享"));

    // 删除只解除关联，不删技能。
    let data = env.cli_json(&["skill", "tag", "remove", "共享"]);
    assert_eq!(data["affected"], 2);
    assert!(!env.read("code/工具/beta/.manager/meta.yaml").unwrap().contains("共享"));
    let list = env.cli_json(&["skill", "list"]);
    assert!(find_skill(&list["skills"], "code", "beta").is_some());
    assert!(find_skill(&list["skills"], "lobster", "alpha").is_some());

    // detach 只减目标标签。
    env.cli_json(&["skill", "tag", "add", "code/alpha", "--tags", "保留,去掉"]);
    let data = env.cli_json(&["skill", "tag", "detach", "code/alpha", "--tags", "去掉"]);
    assert_eq!(data["skills"], serde_json::json!(["code/alpha"]));
    let meta = env.read("code/工具/alpha/.manager/meta.yaml").unwrap();
    assert!(meta.contains("保留"), "{meta}");
    assert!(!meta.contains("去掉"), "{meta}");

    // 部分失败：逐项报告并返回非零。
    let (code, value) = env.cli_json_raw(&["skill", "tag", "add", "code/alpha", "code/不存在", "--tags", "z"]);
    assert_eq!(code, 2, "{value}");
    assert_eq!(value["result"], "partial");
    assert_eq!(value["data"]["skills"], serde_json::json!(["code/alpha"]));
    assert_eq!(value["errors"][0]["skill"], "code/不存在");
}

#[test]
fn overlay_commands_manage_files_under_manager() {
    let env = TestEnv::new();
    seed(&env);

    let data = env.cli_json_stdin(
        &["skill", "overlay", "set", "code/alpha", "--path", "scripts/x.sh", "--stdin"],
        "通用脚本\n",
    );
    assert_eq!(data["written"], true);
    assert_eq!(data["agent"], Value::Null);
    assert_eq!(data["os"], Value::Null);
    assert!(env.path("code/工具/alpha/.manager/overlay/_/_/scripts/x.sh").exists());

    // 指定维度。
    let data = env.cli_json_stdin(
        &["skill", "overlay", "set", "code/alpha", "--path", "SKILL.md", "--agent", "codex", "--stdin"],
        "codex 专用正文\n",
    );
    assert_eq!(data["agent"], "codex");
    assert!(env.path("code/工具/alpha/.manager/overlay/codex/_/SKILL.md").exists());

    let listed = env.cli_json(&["skill", "overlay", "list", "code/alpha"]);
    let overlays = listed["overlays"].as_array().unwrap();
    assert_eq!(overlays.len(), 2, "{listed}");

    let got = env.cli_json(&[
        "skill", "overlay", "get", "code/alpha", "--path", "SKILL.md", "--agent", "codex",
    ]);
    assert_eq!(got["exists"], true);
    assert_eq!(got["text"], "codex 专用正文\n");

    // 不存在的覆盖层如实为空。
    let got = env.cli_json(&[
        "skill", "overlay", "get", "code/alpha", "--path", "nope.sh",
    ]);
    assert_eq!(got["exists"], false);
    assert!(got["text"].is_null());

    let removed = env.cli_json(&[
        "skill", "overlay", "remove", "code/alpha", "--path", "SKILL.md", "--agent", "codex",
    ]);
    assert_eq!(removed["removed"], true);
    assert!(!env.path("code/工具/alpha/.manager/overlay/codex/_/SKILL.md").exists());
    let removed = env.cli_json(&[
        "skill", "overlay", "remove", "code/alpha", "--path", "SKILL.md", "--agent", "codex",
    ]);
    assert_eq!(removed["removed"], false);

    // 逃逸路径被拒绝。
    let (code, value) = env.cli_json_raw(&[
        "skill", "overlay", "set", "code/alpha", "--path", "../escape.sh", "--stdin",
    ]);
    assert_ne!(code, 0, "{value}");
    assert_eq!(value["result"], "failed");
}

// ---------------------------------------------------------------------------
// 1.7 同名约束与跨域同名
// ---------------------------------------------------------------------------

#[test]
fn same_domain_name_conflict_blocks_move_and_cross_domain_same_name_coexists() {
    let env = TestEnv::new();
    seed(&env);

    // 目标分类下已有同名技能 → 阻止并给出可定位原因。
    env.write_skill("code", None, "epsilon", "epsilon 正文\n");
    env.write_skill("code", Some("工具"), "epsilon", "已存在的同名技能\n");
    let (code, value) = env.cli_json_raw(&["skill", "move", "code/epsilon", "--category", "工具"]);
    assert_eq!(code, 1, "{value}");
    assert_eq!(value["result"], "failed");
    assert!(
        value["errors"][0]["message"].as_str().unwrap().contains("epsilon"),
        "{value}"
    );
    assert!(env.path("code/epsilon/SKILL.md").exists(), "阻止后不应移动");

    // 跨域同名允许，且各自可解析。
    let list = env.cli_json(&["skill", "list"]);
    let code_alpha = find_skill(&list["skills"], "code", "alpha").unwrap();
    let lobster_alpha = find_skill(&list["skills"], "lobster", "alpha").unwrap();
    assert_eq!(code_alpha["vault_rel_dir"], "code/工具/alpha");
    assert_eq!(lobster_alpha["vault_rel_dir"], "lobster/alpha");

    let got = env.cli_json(&["skill", "get", "lobster/alpha"]);
    assert_eq!(got["skill"]["domain"], "lobster");
    assert_eq!(got["content"]["original"], "龙虾 alpha 正文\n");
}

// ---------------------------------------------------------------------------
// 错误退出码
// ---------------------------------------------------------------------------

#[test]
fn errors_use_nonzero_exit_codes_with_locatable_reasons() {
    let env = TestEnv::new();
    seed(&env);

    let (code, value) = env.cli_json_raw(&["skill", "get", "code/不存在"]);
    assert_eq!(code, 1, "{value}");
    assert_eq!(value["result"], "failed");
    assert_eq!(value["errors"][0]["skill"], "code/不存在");
    assert!(
        value["errors"][0]["message"].as_str().unwrap().contains("技能不存在"),
        "{value}"
    );

    let (code, value) = env.cli_json_raw(&["skill", "read", "lobster/不存在"]);
    assert_eq!(code, 1, "{value}");

    // 引用缺少域。
    let (code, value) = env.cli_json_raw(&["skill", "get", "alpha"]);
    assert_eq!(code, 1, "{value}");
    assert!(
        value["errors"][0]["message"].as_str().unwrap().contains("domain"),
        "{value}"
    );

    // 未知域。
    let (code, value) = env.cli_json_raw(&["skill", "list", "--domain", "other"]);
    assert_eq!(code, 1, "{value}");

    // 分类与未分类互斥（clap 阻止）。
    let output = env.cli(&["skill", "list", "--category", "工具", "--uncategorized"]);
    assert!(!output.status.success());

    // 移动缺少目标分类参数。
    let output = env.cli(&["skill", "move", "code/alpha"]);
    assert!(!output.status.success());

    // 译文 set 缺少输入来源。
    let output = env.cli(&["skill", "translation", "set", "code/alpha"]);
    assert!(!output.status.success());

    // 非 JSON 模式下人类可读文本为简体中文，且错误写标准错误。
    let output = env.cli(&["skill", "get", "code/不存在"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("未完成"), "{stderr}");
}

#[test]
fn human_output_is_chinese_and_json_mode_is_structured_only() {
    let env = TestEnv::new();
    seed(&env);

    let output = env.cli(&["skill", "list", "--domain", "code"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("code/alpha"), "{stdout}");

    let output = env.cli(&["--json", "skill", "list", "--domain", "code"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(parsed["result"], "success");
}

// ---------------------------------------------------------------------------
// 库内路径与文件树（额外断言）
// ---------------------------------------------------------------------------

#[test]
fn get_reports_vault_path_and_nested_file_tree() {
    let env = TestEnv::new();
    seed(&env);
    env.write("code/工具/alpha/references/deep.md", "参考\n");

    let data = env.cli_json(&["skill", "get", "code/alpha"]);
    let expected = env.vault.canonicalize().unwrap();
    assert_eq!(
        PathBuf::from(data["vault_path"].as_str().unwrap())
            .canonicalize()
            .unwrap(),
        expected
    );

    let references = data["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["path"] == "references")
        .expect("references 目录");
    assert_eq!(references["is_dir"], true);
    assert_eq!(references["children"][0]["path"], "references/deep.md");
}
