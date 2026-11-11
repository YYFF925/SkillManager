//! ACP 工具：映射现有 JSON CLI，经 Host 执行。第一版不做 MCP。

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::invoke::{self, InvokeSource};
use crate::HostRuntime;

/// 内置管理说明目录（应用资产，不入库）。
pub fn builtin_skill_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let candidate = exe
            .parent()
            .unwrap_or(Path::new("."))
            .join("assets/skills/skill-manager-builtin-agent");
        if candidate.is_dir() {
            return candidate;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/skills/skill-manager-builtin-agent")
}

/// 把工具调用转到 Host invoke / 现有 JSON CLI。
pub fn run_tool(runtime: &HostRuntime, name: &str, args: Value, session_id: Option<&str>) -> Result<Value, String> {
    let command = match name {
        "skill_list" | "skill.list" => "skill_list",
        "skill_get" | "skill.get" => "skill_get",
        "skill_read" | "skill.read" => "skill_read",
        "skill_set" | "skill.set" => "skill_set",
        "skill_delete" | "skill.delete" => "skill_delete",
        "deploy_to" | "deploy.to" => "deploy_to",
        "deploy_sync" | "deploy.sync" => "deploy_sync",
        "sync_push" | "sync.push" => "sync_push",
        "sync_status" | "sync.status" => "sync_status",
        other => other,
    };
    if command == "settings_set_git_auto_push" {
        return Err("内置管理 Agent 不能修改自动上传授权；请在设置中开启".to_string());
    }
    if command == "sync_push" {
        let ctx = runtime.get()?;
        if !ctx.state().git_auto_push().map_err(|err| err.to_string())? {
            return Err("未开启自动上传。请在设置中打开 GUI 保存后自动上传，然后再试。".to_string());
        }
    }
    match invoke::invoke(runtime, command, args, InvokeSource::Agent, session_id) {
        Ok(value) => Ok(value),
        Err(err) => {
            let (_, body) = err.status_and_body();
            Err(body
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| body.to_string()))
        }
    }
}

pub fn tool_descriptors() -> Value {
    json!([
        {"name": "skill_list", "description": "列出当前 Host 的技能摘要（含 translation_stale 等已有字段）"},
        {"name": "skill_get", "description": "读取一个技能的详情"},
        {"name": "skill_read", "description": "读取技能正文片段"},
        {"name": "skill_set", "description": "修改注释、分类、标签或支持声明"},
        {"name": "skill_delete", "description": "删除技能；须经确认计划，口头同意无效"},
        {"name": "deploy_to", "description": "普通部署，不逐步确认"},
        {"name": "deploy_sync", "description": "同步已有部署；overwrite_local 须确认计划"},
        {"name": "sync_status", "description": "库同步状态"},
        {"name": "sync_push", "description": "上传库；仅当 GUI 已打开自动上传"}
    ])
}
