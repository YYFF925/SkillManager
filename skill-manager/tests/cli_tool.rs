//! quickstart 5.4～5.7、5.13 与 US9 场景的 CLI 集成测试（T075～T079）。
//!
//! 断言工具共享资料、本机安装事实与检查结果的边界：未安装保留显示、各检查状态分别表达、
//! 检查不改安装事实、回写后重置为未检查、任务保留必要上下文并引用配套维护 Skill、
//! 未入库时如实提示。全部在临时 HOME/库/状态目录内执行。

mod common;

use common::TestEnv;
use serde_json::Value;

/// 写一个工具资料源目录（`--dir` 的输入）。
fn write_tool_source(env: &TestEnv, slug_dir: &str, tool_yaml: &str, guide: Option<&str>) {
    let dir = env.root.path().join(slug_dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("tool.yaml"), tool_yaml).unwrap();
    if let Some(guide) = guide {
        std::fs::write(dir.join("guide.md"), guide).unwrap();
    }
}

fn write_script(env: &TestEnv, slug_dir: &str, name: &str, body: &str) {
    let dir = env.root.path().join(slug_dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(name), body).unwrap();
}

fn tool_entry<'a>(tools: &'a Value, slug: &str) -> &'a Value {
    tools
        .as_array()
        .expect("tools 应为数组")
        .iter()
        .find(|item| item["slug"] == slug)
        .unwrap_or_else(|| panic!("工具清单缺少 {slug}：{tools}"))
}

fn plain(env: &TestEnv, args: &[&str]) -> (i32, String, String) {
    let output = env.cli(args);
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn uninstalled_tool_stays_visible_and_check_does_not_create_install() {
    let env = TestEnv::new();
    write_tool_source(
        &env,
        "toolsrc/ripgrep",
        "name: ripgrep\nsummary: 极快的正则搜索工具\nintro: |\n  第一段说明用途。\n  第二段说明场景。\ntags: [搜索, cli]\ncategory: 命令行工具\ncheck_command: sh ./check-latest.sh\nskills:\n  - domain: code\n    name: ripgrep-usage\n    repo: https://example.invalid/rg.git\n    subpath: ripgrep-usage\n",
        Some("# ripgrep 维护指南\n用途与调用入口：rg\n"),
    );
    write_script(&env, "toolsrc/ripgrep", "check-latest.sh", "echo 14.1.2\n");

    let registered = env.cli_json(&["tool", "register", "--dir", &env.root.path().join("toolsrc/ripgrep").to_string_lossy()]);
    assert_eq!(registered["slug"], "ripgrep");
    assert!(registered["wrote_guide"].as_bool().unwrap());
    assert_eq!(registered["copied_files"][0], "check-latest.sh");

    // 5.4：新电脑接入已有工具的库 → 未安装工具保留显示，可暗显但不隐藏。
    let listed = env.cli_json(&["tool", "list"]);
    assert_eq!(listed["total"], 1);
    let entry = tool_entry(&listed["tools"], "ripgrep");
    assert!(entry["install"].is_null(), "{entry}");
    assert_eq!(entry["check"]["status"], "unchecked");
    // 未安装 + 未检查：不派生“本机最新/可更新”。
    assert!(entry["comparison"].is_null(), "{entry}");
    // 配套引用缺失时标记“未入库”。
    assert_eq!(entry["skills"][0]["in_vault"], false);

    // 5.5：检查只保存检查结果；未安装工具显示上游版本，仍不派生“本机最新”。
    let checked = env.cli_json(&["tool", "check", "ripgrep"]);
    assert_eq!(checked["tools"][0]["status"], "checked");
    assert_eq!(checked["tools"][0]["latest_version"], "14.1.2");
    assert!(checked["tools"][0]["installed_version"].is_null());
    assert!(checked["tools"][0]["comparison"].is_null());

    let listed = env.cli_json(&["tool", "list"]);
    let entry = tool_entry(&listed["tools"], "ripgrep");
    assert!(entry["install"].is_null());
    assert_eq!(entry["check"]["latest_version"], "14.1.2");

    // 仍未安装：清单与状态都没把检查结果当成安装事实。
    let (code, ..) = plain(&env, &["tool", "check", "--all"]);
    assert_eq!(code, 0);
}

#[test]
fn check_states_are_expressed_separately_and_never_touch_install() {
    let env = TestEnv::new();
    // 未登记命令的工具。
    write_tool_source(&env, "toolsrc/no-cmd", "name: no-command-tool\n", None);
    // 检查失败：非零退出。
    write_tool_source(
        &env,
        "toolsrc/failing",
        "name: failing-tool\ncheck_command: sh ./fail.sh\n",
        None,
    );
    write_script(&env, "toolsrc/failing", "fail.sh", "exit 3\n");
    // 检查成功。
    write_tool_source(
        &env,
        "toolsrc/ok",
        "name: ok-tool\ncheck_command: sh ./ok.sh\n",
        None,
    );
    write_script(&env, "toolsrc/ok", "ok.sh", "echo 2.0\n");

    for slug in ["no-cmd", "failing", "ok"] {
        env.cli_json(&[
            "tool",
            "register",
            "--dir",
            &env.root.path().join(format!("toolsrc/{slug}")).to_string_lossy(),
        ]);
    }

    // slug 由工具名称派生（ASCII 字母数字，其余折为 -）。
    let listed = env.cli_json(&["tool", "list"]);
    assert_eq!(listed["total"], 3);
    let slugs: Vec<String> = listed["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["slug"].as_str().unwrap().to_string())
        .collect();
    assert!(slugs.contains(&"no-command-tool".to_string()), "{slugs:?}");
    assert!(slugs.contains(&"failing-tool".to_string()), "{slugs:?}");
    assert!(slugs.contains(&"ok-tool".to_string()), "{slugs:?}");

    // 回写安装事实（仅 Agent 核实后调用）。
    env.cli_json(&[
        "tool",
        "set-install",
        "ok-tool",
        "--version",
        "1.0",
        "--method",
        "brew",
    ]);

    let listed = env.cli_json(&["tool", "list"]);
    let tools = &listed["tools"];
    // 未登记命令与未检查分别表达。
    assert_eq!(tool_entry(tools, "no-command-tool")["check"]["status"], "no_command");
    assert_eq!(tool_entry(tools, "failing-tool")["check"]["status"], "unchecked");
    assert_eq!(tool_entry(tools, "ok-tool")["check"]["status"], "unchecked");

    // 状态筛选可用。
    let only_no_command = env.cli_json(&["tool", "list", "--status", "no-command"]);
    assert_eq!(only_no_command["total"], 1);
    let only_unchecked = env.cli_json(&["tool", "list", "--status", "unchecked"]);
    assert_eq!(only_unchecked["total"], 2);
    let installed = env.cli_json(&["tool", "list", "--status", "installed"]);
    assert_eq!(installed["total"], 1);

    let report = env.cli_json(&["tool", "check", "--all"]);
    assert_eq!(report["checked"], 3);
    let outcomes = report["tools"].as_array().unwrap();
    let status_of = |slug: &str| {
        outcomes
            .iter()
            .find(|item| item["slug"] == slug)
            .unwrap()["status"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert_eq!(status_of("no-command-tool"), "no_command");
    assert_eq!(status_of("failing-tool"), "failed");
    assert_eq!(status_of("ok-tool"), "checked");

    // ok-tool：已安装 1.0，上游 2.0 → 可更新。
    let ok = outcomes.iter().find(|item| item["slug"] == "ok-tool").unwrap();
    assert_eq!(ok["comparison"]["kind"], "updatable");
    // 检查不改安装事实（method 仍是回写值）。
    let detail = env.cli_json(&["tool", "get", "ok-tool"]);
    assert_eq!(detail["install"]["version"], "1.0");
    assert_eq!(detail["install"]["method"], "brew");

    let failed = env.cli_json(&["tool", "list", "--status", "failed"]);
    assert_eq!(failed["total"], 1);
}

#[test]
fn install_write_back_resets_check_and_incomparable_versions_do_not_pretend() {
    let env = TestEnv::new();
    write_tool_source(
        &env,
        "toolsrc/dated",
        "name: dated-tool\ncheck_command: sh ./latest.sh\n",
        None,
    );
    // 上游返回日期样式版本，与本机 semver 记录无法比较。
    write_script(&env, "toolsrc/dated", "latest.sh", "echo 2024-10-15\n");
    env.cli_json(&[
        "tool",
        "register",
        "--dir",
        &env.root.path().join("toolsrc/dated").to_string_lossy(),
    ]);
    env.cli_json(&[
        "tool",
        "set-install",
        "dated-tool",
        "--version",
        "1.2.3",
    ]);

    let report = env.cli_json(&["tool", "check", "dated-tool"]);
    assert_eq!(report["tools"][0]["status"], "checked");
    assert_eq!(report["tools"][0]["latest_version"], "2024-10-15");
    // 无法比较：不冒充“最新/可更新”。
    assert_eq!(report["tools"][0]["comparison"]["kind"], "unknown");
    assert!(
        report["tools"][0]["comparison"]["reason"]
            .as_str()
            .unwrap()
            .contains("无法比较"),
        "{report}"
    );

    // 回写后检查结果重置为未检查（先制造一个 checked 状态）。
    env.cli_json(&["tool", "check", "dated-tool"]);
    let before = env.cli_json(&["tool", "get", "dated-tool"]);
    assert_eq!(before["check"]["status"], "checked");
    env.cli_json(&["tool", "set-install", "dated-tool", "--version", "1.3.0"]);
    let after = env.cli_json(&["tool", "get", "dated-tool"]);
    assert_eq!(after["check"]["status"], "unchecked");
    assert!(after["check"]["latest_version"].is_null());
    assert!(after["comparison"].is_null());

    // 上游低于本机记录时如实说明（可比较但不判定为可更新）。
    env.write("tools/dated-tool/latest.sh", "echo 1.0.0\n");
    let report = env.cli_json(&["tool", "check", "dated-tool"]);
    assert_eq!(report["tools"][0]["comparison"]["kind"], "unknown");
    assert!(
        report["tools"][0]["comparison"]["reason"]
            .as_str()
            .unwrap()
            .contains("低于本机安装版本"),
        "{report}"
    );
}

#[test]
fn maintenance_task_keeps_context_and_references_skill_path() {
    let env = TestEnv::new();
    write_tool_source(
        &env,
        "toolsrc/rg",
        "name: rg\nsummary: 搜索工具\nintro: |\n  简介第一段。\ncheck_command: sh ./latest.sh\nskills:\n  - domain: code\n    name: rg-usage\n",
        Some("# 维护指南\n更新步骤：brew upgrade rg\n"),
    );
    write_script(&env, "toolsrc/rg", "latest.sh", "echo 9.9\n");
    env.cli_json(&[
        "tool",
        "register",
        "--dir",
        &env.root.path().join("toolsrc/rg").to_string_lossy(),
    ]);
    env.cli_json(&["tool", "set-install", "rg", "--version", "9.0"]);

    // 配套维护 Skill 未入库：如实提示，不编造路径、不含“按其规范执行”。
    let (code, stdout, _) = plain(&env, &["tool", "task", "rg", "--action", "update", "--requirement", "更新到最新"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("配套维护 Skill：未入库"), "{stdout}");
    assert!(!stdout.contains("按其规范执行"), "{stdout}");
    assert!(stdout.contains("用户要求：更新到最新"), "{stdout}");
    assert!(stdout.contains("简介第一段"), "{stdout}");
    assert!(stdout.contains("当前安装版本：9.0"), "{stdout}");
    assert!(stdout.contains("维护指南全文："), "{stdout}");
    assert!(stdout.contains("不代表已开始或已完成"), "{stdout}");

    // 配套维护 Skill 入库后引用真实库内路径。
    env.write_skill("code", Some("维护"), "skill-manager-maintenance", "# 维护 Skill\n");
    let (code, stdout, _) = plain(&env, &["tool", "task", "rg", "--action", "update"]);
    assert_eq!(code, 0);
    let expected = env
        .vault
        .join("code/维护/skill-manager-maintenance")
        .to_string_lossy()
        .into_owned();
    assert!(stdout.contains(&expected), "{stdout}");
    assert!(stdout.contains("按其规范执行"), "{stdout}");

    // register 动作允许尚无 slug，用 --requirement 说明名称与来源。
    let (code, stdout, _) = plain(&env, &["tool", "task", "--action", "register", "--requirement", "工具名称：自建命令行工具"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("tool register"), "{stdout}");
}

#[test]
fn skill_task_single_and_multiple_keep_context_and_only_reference_the_skill() {
    let env = TestEnv::new();
    env.write_skill("code", Some("matt"), "matt-pr", "# PR\n");
    env.write_manager(
        "code",
        Some("matt"),
        "matt-pr",
        "meta.yaml",
        "tags: [评审]\ncomment: 写 PR\n",
    );
    env.write_skill("lobster", None, "lobster-helper", "# helper\n");

    let single = env.cli_json(&[
        "skill",
        "task",
        "code/matt-pr",
        "--requirement",
        "补充中文译文",
    ]);
    let task = single["task"].as_str().unwrap();
    assert!(task.contains("SkillManagerVault 本机目录："), "{task}");
    assert!(task.contains("我的维护要求：\n补充中文译文"), "{task}");
    assert!(task.contains("库内路径："), "{task}");
    assert!(task.contains("中文注释：写 PR"), "{task}");
    assert!(task.contains("分类：matt；标签：评审"), "{task}");
    assert!(task.contains("本机已部署：无"), "{task}");
    // 规范只引用配套维护 Skill；未入库时如实提示。
    assert!(task.contains("配套维护 Skill：未入库"), "{task}");
    // 不内联第二份规范全文。
    assert!(!task.contains("按其规范执行"), "{task}");

    let multiple = env.cli_json(&[
        "skill",
        "task",
        "code/matt-pr",
        "lobster/lobster-helper",
        "--requirement",
        "统一整理注释",
    ]);
    let task = multiple["task"].as_str().unwrap();
    assert!(task.contains("matt-pr（编码库）"), "{task}");
    assert!(task.contains("lobster-helper（龙虾库）"), "{task}");
    assert_eq!(multiple["skills"].as_array().unwrap().len(), 2);

    // 入库配套维护 Skill 后引用其真实库内路径。
    env.write_skill("code", None, "skill-manager-maintenance", "# 维护 Skill\n");
    let after = env.cli_json(&["skill", "task", "code/matt-pr"]);
    let task = after["task"].as_str().unwrap();
    let expected = env
        .vault
        .join("code/skill-manager-maintenance")
        .to_string_lossy()
        .into_owned();
    assert!(task.contains(&expected), "{task}");
    assert!(task.contains("按其规范执行"), "{task}");

    // 生成任务不改库（技能文件与部署记录均未新增）。
    assert!(env.read("code/matt/matt-pr/SKILL.md").is_some());
}

#[test]
fn set_from_stdin_and_set_install_clear_are_honest() {
    let env = TestEnv::new();
    let registered = env.cli_json_stdin(
        &["tool", "register", "--stdin"],
        &serde_json::json!({
            "tool": {
                "name": "uv",
                "summary": "Python 环境管理",
                "category": "开发工具",
                "check_command": "sh ./latest.sh",
                "unknown_field": "keep-me"
            },
            "guide": "# uv 维护指南\n"
        })
        .to_string(),
    );
    assert_eq!(registered["slug"], "uv");

    // set --stdin 只更新传入字段，未知字段保留；其他文件通过资料目录维护。
    env.cli_json_stdin(
        &["tool", "set", "uv", "--stdin"],
        &serde_json::json!({"tool": {"summary": "新摘要"}}).to_string(),
    );
    let text = env.read("tools/uv/tool.yaml").unwrap();
    assert!(text.contains("unknown_field: keep-me"), "{text}");
    assert!(text.contains("新摘要"), "{text}");

    // skill-link 追加配套关联（此时未入库）。
    let linked = env.cli_json(&["tool", "skill-link", "uv", "code/uv-usage"]);
    assert_eq!(linked["added"], true);
    let detail = env.cli_json(&["tool", "get", "uv"]);
    assert_eq!(detail["skills"][0]["name"], "uv-usage");
    assert_eq!(detail["skills"][0]["in_vault"], false);

    // set-install 空版本表示本机已不存在该工具：清除记录。
    env.cli_json(&["tool", "set-install", "uv", "--version", "0.9.0"]);
    assert_eq!(env.cli_json(&["tool", "get", "uv"])["install"]["version"], "0.9.0");
    env.cli_json(&["tool", "set-install", "uv", "--version", ""]);
    assert!(env.cli_json(&["tool", "get", "uv"])["install"].is_null());

    // 未登记工具回写被阻止（不能凭空造安装事实）。
    let (code, result) = env.cli_json_raw(&["tool", "set-install", "ghost", "--version", "1.0"]);
    assert_ne!(code, 0);
    assert_eq!(result["result"], "failed");
}

#[test]
fn tool_check_all_does_not_require_tools_and_reports_readably() {
    let env = TestEnv::new();
    // 空库：检查全部正常返回 0 项，不报成功安装。
    let report = env.cli_json(&["tool", "check", "--all"]);
    assert_eq!(report["checked"], 0);

    // 资料损坏的目录如实标记，其余工具仍可浏览。
    std::fs::create_dir_all(env.vault.join("tools/stray")).unwrap();
    let listed = env.cli_json(&["tool", "list"]);
    assert_eq!(listed["total"], 1);
    assert!(tool_entry(&listed["tools"], "stray")["error"].is_string());
}
