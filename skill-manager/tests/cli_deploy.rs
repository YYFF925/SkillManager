//! quickstart M2 CLI 场景（T023）：部署、接管、同步暂停、撤销、删库清理、多技能×多目标
//! 与 dry-run。全部使用预置库文件，不依赖 US2。

mod common;

#[test]
fn batch_deployment_continues_after_target_write_error() {
    let env = common::TestEnv::new();
    env.write_skill("code", None, "alpha", "alpha\n");
    let blocked = env.root.path().join("blocked");
    std::fs::write(&blocked, "不是目录").unwrap();
    let good = env.root.path().join("good");
    env.cli_json(&["agent", "set-dir", "codex", "--skills", blocked.to_str().unwrap()]);
    env.cli_json(&["agent", "set-dir", "zcode", "--skills", good.to_str().unwrap()]);
    let (code, result) = env.cli_json_raw(&["deploy", "to", "codex", "code/alpha", "--also-target", "zcode"]);
    assert_eq!(code, 2, "{result}");
    assert_eq!(result["data"]["items"][0]["target"], "agent:zcode");
    assert_eq!(result["errors"][0]["target"], "agent:codex");
    assert!(good.join("alpha/SKILL.md").exists());
    let diagnostics = env.cli_json(&["settings", "diagnostics"]);
    let logs = diagnostics["recent_logs"].as_array().unwrap();
    let failure = logs.iter().find(|entry| entry["action"] == "deploy").unwrap();
    let message = failure["message"].as_str().unwrap();
    assert!(message.contains("blocked") && message.contains("agent:codex"), "{failure}");
    assert!(!failure["at"].as_str().unwrap().is_empty());
}

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

#[test]
fn deploy_keeps_quoted_frontmatter_shapes_and_is_byte_idempotent() {
    // 端到端复现 YAML 1.1 类型腐蚀事故场景：引号形态字段与行内注释
    // 部署后逐字节保留，重复部署字节一致。
    let env = TestEnv::new();
    let original = "---\n\
name: probe\n\
description: Probe skill\n\
version: \"1.10\"\n\
date: \"2026-10-08\"\n\
time: \"12:30\"\n\
flag: \"no\" # 上游注释\n\
---\n正文\n";
    env.write_skill("code", None, "probe", original);
    env.write_manager("code", None, "probe", "meta.yaml", "comment: 检查\n");

    let (code, result) = env.cli_json_raw(&["deploy", "to", "codex", "code/probe"]);
    assert_eq!(code, 0, "{result}");
    let entry_path = deployed(&env, ".codex/skills", "probe").join("SKILL.md");
    let first = std::fs::read_to_string(&entry_path).unwrap();
    for kept in [
        "name: probe\n",
        "version: \"1.10\"\n",
        "date: \"2026-10-08\"\n",
        "time: \"12:30\"\n",
        "flag: \"no\" # 上游注释\n",
        "---\n正文\n",
    ] {
        assert!(first.contains(kept), "部署副本应原样保留 {kept:?}：\n{first}");
    }
    assert!(first.contains("description: 检查｜Probe skill\n"), "{first}");

    // 再次部署（sync 全量）后产物字节一致。
    let (code, _) = env.cli_json_raw(&["deploy", "sync"]);
    assert_eq!(code, 0);
    let second = std::fs::read_to_string(&entry_path).unwrap();
    assert_eq!(first, second, "重复部署应字节一致");
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
fn removed_preview_flag_is_rejected() {
    let env = TestEnv::new();
    seed(&env);
    for args in [vec!["deploy", "to", "codex", "code/alpha", "--dry-run"], vec!["deploy", "undo", "codex", "code/alpha", "--dry-run"]] {
        let output = env.cli(&args);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("--dry-run"));
    }
    assert!(env.cli_json(&["deploy", "status"])["deployments"].as_array().unwrap().is_empty());
}
