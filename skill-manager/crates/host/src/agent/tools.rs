//! ACP 工具：映射现有 JSON CLI，经 Host 执行。第一版不做 MCP。

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::HostRuntime;
use crate::invoke::{self, InvokeError, InvokeSource};

const ALLOWED: &[&str] = &[
    "skill_list",
    "skill_get",
    "skill_read",
    "skill_file_read",
    "skill_set",
    "skill_delete",
    "skill_archive_list",
    "skill_translation_get",
    "skill_overlay_list",
    "skill_overlay_get",
    "skill_rejected",
    "skill_skipped",
    "skill_task",
    "deploy_to",
    "deploy_sync",
    "deploy_status",
    "sync_push",
    "sync_status",
    "sync_conflicts",
    "preset_list",
    "preset_get",
    "hermes_profiles",
    "hermes_list",
    "hermes_read",
    "tool_list",
    "tool_get",
    "tool_skill_candidates",
    "agent_list",
    "agent_local_skills",
    "agent_read_local",
    "shared_dir_get",
    "instruction_get",
    "settings_get",
    "environment_schedule_current",
    "environment_diagnostics",
    "environment_capacity",
    "vault_current",
    "host_status",
];

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

fn canonical_name(name: &str) -> String {
    name.replace('.', "_")
}

/// 把工具调用转到 Host invoke / 现有 JSON CLI。
pub fn run_tool(
    runtime: &HostRuntime,
    name: &str,
    args: Value,
    session_id: Option<&str>,
) -> Result<Value, String> {
    let command = canonical_name(name);
    if command == "plans_confirm" || command == "plans.confirm" {
        return Err("内置管理 Agent 不能代替用户确认破坏性操作。请在界面确认。".to_string());
    }
    if command == "settings_set_git_auto_push" {
        return Err("内置管理 Agent 不能修改自动上传授权；请在设置中开启".to_string());
    }
    if !ALLOWED.contains(&command.as_str()) {
        return Err(format!("未知或不允许的管理工具：{command}"));
    }
    if command == "sync_push" {
        let ctx = runtime.get()?;
        if !ctx.state().git_auto_push().map_err(|err| err.to_string())? {
            return Err(
                "未开启自动上传。请在设置中打开 GUI 保存后自动上传，然后再试。".to_string(),
            );
        }
    }
    match invoke::invoke(runtime, &command, args, InvokeSource::Agent, session_id) {
        Ok(value) => Ok(value),
        Err(InvokeError::NeedsPlan(plan)) => Err(format!(
            "需要在界面确认后才能继续（确认前不会改库）。planId={}",
            plan.get("planId").and_then(Value::as_str).unwrap_or("")
        )),
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
    json!(
        ALLOWED
            .iter()
            .map(|name| json!({ "name": name, "description": tool_description(name) }))
            .collect::<Vec<_>>()
    )
}

fn tool_description(name: &str) -> &'static str {
    match name {
        "skill_list" => "列出当前 Host 的技能摘要（含 translation_stale 等已有字段）",
        "skill_get" => "读取一个技能的详情（含文件树与部署位置；正文请用 skill_read）",
        "skill_read" => "读取技能正文片段",
        "skill_file_read" => "读取技能目录内指定相对路径文件",
        "skill_set" => "修改注释、分类、标签或支持声明",
        "skill_delete" => "删除技能；须经界面确认计划，口头同意无效",
        "skill_archive_list" => "列出已归档技能",
        "skill_translation_get" => "读取中文译文",
        "skill_overlay_list" => "列出覆盖层文件",
        "skill_overlay_get" => "读取一项覆盖层",
        "skill_rejected" => "已拒绝入库的新增候选",
        "skill_skipped" => "已跳过的上游版本",
        "skill_task" => "生成技能维护任务说明（不改库）",
        "deploy_to" => "普通部署，不逐步确认",
        "deploy_sync" => "同步已有部署；overwrite_local 须界面确认计划",
        "deploy_status" => "本机部署记录；paused 只列出 paused_local_change",
        "sync_status" => "库同步状态",
        "sync_push" => "上传库；仅当 GUI 已打开自动上传",
        "sync_conflicts" => "拉取冲突路径与 git status 原文",
        "preset_list" => "列出预设摘要",
        "preset_get" => "读取一个预设的成员",
        "hermes_profiles" => "本机 Hermes 档案列表",
        "hermes_list" => "列出某档案技能",
        "hermes_read" => "读取档案技能文件",
        "tool_list" => "工具中心清单（不含指南正文）",
        "tool_get" => "工具详情",
        "tool_skill_candidates" => "工具配套 Skill 候选",
        "agent_list" => "本机 Agent 检测与启用状态",
        "agent_local_skills" => "某目标目录下的本机技能（含 paused_local_change）",
        "agent_read_local" => "读取本机技能副本文件",
        "shared_dir_get" => "共享 Agent 目录",
        "instruction_get" => "读取指令源与合成预览",
        "settings_get" => "主题、检查频率、自动上传等本机设置",
        "environment_schedule_current" => "最近一次调度报告（含可入库新增）",
        "environment_diagnostics" => "最近命令失败原文与诊断路径",
        "environment_capacity" => "库容量排行",
        "vault_current" => "当前接入的 Vault 路径",
        "host_status" => "Host 状态",
        _ => "现有 JSON CLI 经 Host 转发",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HostRuntime;
    use crate::invoke::InvokeSource;
    use serde_json::json;
    use skill_manager_core::AppContext;

    #[test]
    fn agent_cannot_confirm_or_call_unknown_tools() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        let denied = run_tool(
            &runtime,
            "plans_confirm",
            json!({ "planId": "x" }),
            Some("agent"),
        );
        assert!(denied.unwrap_err().contains("不能代替用户确认"));
        let unknown = run_tool(&runtime, "settings_set_theme", json!({}), Some("agent"));
        assert!(unknown.unwrap_err().contains("未知或不允许"));
        let invoke_denied = invoke::invoke(
            &runtime,
            "plans_confirm",
            json!({ "planId": "x" }),
            InvokeSource::Agent,
            Some("agent"),
        );
        assert!(invoke_denied.is_err());
    }
}
