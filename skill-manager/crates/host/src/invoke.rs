//! 与 Tauri invoke 同名的 Host 命令分发（研究决策 D3）。

use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::commands;
use crate::commands::acquire::MetaInput;
use crate::commands::skill::{LangArg, SupportArg};
use crate::commands::tool::ToolListParams;
use crate::plans::{self, DestructiveAction};
use crate::HostRuntime;

/// 调用来源：图形界面、CLI、内置管理 Agent。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvokeSource {
    Gui,
    Cli,
    Agent,
}

#[derive(Debug, thiserror::Error)]
pub enum InvokeError {
    #[error("{0}")]
    Failed(String),
    #[error("未知命令：{0}")]
    Unknown(String),
    #[error("需要确认")]
    NeedsPlan(Value),
}

impl InvokeError {
    pub fn status_and_body(&self) -> (u16, Value) {
        match self {
            InvokeError::NeedsPlan(plan) => (409, plan.clone()),
            InvokeError::Unknown(name) => (404, serde_json::json!({"error": format!("未知命令：{name}")})),
            InvokeError::Failed(message) => (400, serde_json::json!({"error": message})),
        }
    }
}

/// 执行与现有 Tauri command 同名的操作。
pub fn invoke(
    runtime: &HostRuntime,
    command: &str,
    body: Value,
    source: InvokeSource,
    session_id: Option<&str>,
) -> Result<Value, InvokeError> {
    if command == "settings_set_git_auto_push" && source != InvokeSource::Gui {
        return Err(InvokeError::Failed(
            "自动上传授权只能在图形界面中修改；请到设置中开启".to_string(),
        ));
    }
    if let Some(action) = destructive_action(command, &body) {
        let plan_id = opt_string(&body, "planId")?
            .or_else(|| opt_string(&body, "plan_id").ok().flatten());
        match plan_id {
            Some(id) => plans::assert_confirmed(runtime, &id, session_id, action)
                .map_err(InvokeError::Failed)?,
            None => {
                let plan = plans::issue(runtime, session_id, action, &body).map_err(InvokeError::Failed)?;
                return Err(InvokeError::NeedsPlan(plan));
            }
        }
    }
    dispatch(runtime, command, &body)
}

fn destructive_action(command: &str, body: &Value) -> Option<DestructiveAction> {
    match command {
        "skill_delete" => Some(DestructiveAction::SkillDelete),
        "deploy_sync" if opt_bool(body, "overwriteLocal").ok().flatten().unwrap_or(false)
            || opt_bool(body, "overwrite_local").ok().flatten().unwrap_or(false) =>
        {
            Some(DestructiveAction::DeployOverwrite)
        }
        "deploy_undo" if opt_bool(body, "discardLocalChanges").ok().flatten().unwrap_or(false)
            || opt_bool(body, "discard_local_changes").ok().flatten().unwrap_or(false) =>
        {
            Some(DestructiveAction::DeployOverwrite)
        }
        _ => None,
    }
}

fn dispatch(runtime: &HostRuntime, command: &str, body: &Value) -> Result<Value, InvokeError> {
    match command {
        "skill_preview" => commands::acquire::skill_preview(runtime, req_string(body, "repo")?, opt_string(body, "reference")?, req_string(body, "job_id")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_fetch_cancel" => Ok(serde_json::json!(commands::acquire::skill_fetch_cancel(runtime, req_string(body, "job_id")?))),
        "skill_import" => commands::acquire::skill_import(runtime, req_string(body, "repo")?, req_string(body, "domain")?, req_string_vec(body, "skills")?, opt_string(body, "reference")?, opt_string(body, "category")?, opt_bool(body, "uncategorized")?, req_string(body, "job_id")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_adopt" => commands::acquire::skill_adopt(runtime, req_string(body, "agent")?, req_string_vec(body, "names")?, opt_string(body, "category")?, opt_bool(body, "uncategorized")?, opt_string(body, "comment")?, opt_string_vec(body, "tags")?, opt_json::<HashMap<String, MetaInput>>(body, "metadata")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_create" => commands::acquire::skill_create(runtime, req_string(body, "reference")?, opt_string(body, "category")?, opt_bool(body, "uncategorized")?, opt_string(body, "comment")?, opt_string_vec(body, "tags")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_copy" => commands::acquire::skill_copy(runtime, req_string(body, "reference")?, req_string(body, "to")?, opt_string(body, "category")?, opt_bool(body, "uncategorized")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "agent_list" => commands::agent::agent_list(runtime).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "agent_detect" => commands::agent::agent_detect(runtime).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "agent_enable" => commands::agent::agent_enable(runtime, req_string(body, "key")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "agent_disable" => commands::agent::agent_disable(runtime, req_string(body, "key")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "agent_reorder" => commands::agent::agent_reorder(runtime, req_string_vec(body, "order")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "agent_set_dir" => commands::agent::agent_set_dir(runtime, req_string(body, "key")?, req_string(body, "kind")?, req_string(body, "path")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "agent_reset_dir" => commands::agent::agent_reset_dir(runtime, req_string(body, "key")?, req_string(body, "kind")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "agent_local_skills" => commands::agent::agent_local_skills(runtime, req_string(body, "target")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "agent_read_local" => commands::agent::agent_read_local(runtime, req_string(body, "target")?, req_string(body, "name")?, opt_string(body, "file")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "agent_delete_local" => commands::agent::agent_delete_local(runtime, req_string(body, "target")?, req_string_vec(body, "names")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "shared_dir_get" => commands::agent::shared_dir_get(runtime).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "instruction_get" => commands::agent::instruction_get(runtime, req_string(body, "scope")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "instruction_set" => commands::agent::instruction_set(runtime, req_string(body, "scope")?, req_string(body, "content")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "deploy_to" => commands::deploy::deploy_to(runtime, req_string(body, "target")?, req_string_vec(body, "references")?, opt_string_vec(body, "also_targets")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "deploy_undo" => commands::deploy::deploy_undo(runtime, req_string(body, "target")?, req_string_vec(body, "references")?, opt_string_vec(body, "also_targets")?, opt_bool(body, "discard_local_changes")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "deploy_sync" => commands::deploy::deploy_sync(runtime, opt_string(body, "reference")?, opt_string(body, "target")?, opt_bool(body, "overwrite_local")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "deploy_status" => commands::deploy::deploy_status(runtime, opt_string(body, "target")?, opt_bool(body, "paused")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_delete" => commands::deploy::skill_delete(runtime, req_string_vec(body, "references")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "environment_check" => commands::environment::environment_check(runtime).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "environment_schedule_current" => commands::environment::environment_schedule_current(runtime).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "environment_os" => Ok(serde_json::json!(commands::environment::environment_os())),
        "environment_diagnostics" => commands::environment::environment_diagnostics(runtime, opt_usize(body, "recent")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "environment_diagnostics_export" => commands::environment::environment_diagnostics_export(runtime, opt_usize(body, "recent")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "environment_capacity" => commands::environment::environment_capacity(runtime, opt_usize(body, "top")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "hermes_read" => commands::hermes::hermes_read(runtime, req_string(body, "profile")?, req_string(body, "name")?, opt_string(body, "file")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "hermes_profiles" => commands::hermes::hermes_profiles(runtime).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "hermes_list" => commands::hermes::hermes_list(runtime, req_string(body, "profile")?, opt_string(body, "query")?, opt_string(body, "category")?, opt_bool(body, "uncategorized")?, opt_string_vec(body, "tags")?, opt_bool(body, "no_tag")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "hermes_enable" => commands::hermes::hermes_enable(runtime, req_string(body, "profile")?, req_string_vec(body, "names")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "hermes_disable" => commands::hermes::hermes_disable(runtime, req_string(body, "profile")?, req_string_vec(body, "names")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "hermes_adopt" => commands::hermes::hermes_adopt(runtime, req_string(body, "profile")?, req_string_vec(body, "names")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "preset_list" => commands::preset::preset_list(runtime, opt_string(body, "domain")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "preset_get" => commands::preset::preset_get(runtime, req_string(body, "domain")?, req_string(body, "id")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "preset_create" => commands::preset::preset_create(runtime, req_string(body, "domain")?, req_string(body, "name")?, opt_string(body, "icon")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "preset_delete" => commands::preset::preset_delete(runtime, req_string(body, "domain")?, req_string(body, "id")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "preset_rename" => commands::preset::preset_rename(runtime, req_string(body, "domain")?, req_string(body, "id")?, req_string(body, "name")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "preset_set_icon" => commands::preset::preset_set_icon(runtime, req_string(body, "domain")?, req_string(body, "id")?, opt_string(body, "icon")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "preset_reorder" => commands::preset::preset_reorder(runtime, req_string(body, "domain")?, req_string_vec(body, "order")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "preset_members_add" => commands::preset::preset_members_add(runtime, req_string(body, "domain")?, req_string(body, "id")?, req_string_vec(body, "names")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "preset_members_remove" => commands::preset::preset_members_remove(runtime, req_string(body, "domain")?, req_string(body, "id")?, req_string_vec(body, "names")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "preset_reorder_members" => commands::preset::preset_reorder_members(runtime, req_string(body, "domain")?, req_string(body, "id")?, req_string_vec(body, "order")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "deploy_preset" => commands::preset::deploy_preset(runtime, req_string(body, "domain")?, req_string(body, "preset_id")?, req_string(body, "target")?, opt_string_vec(body, "also_targets")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "deploy_undo_preset" => commands::preset::deploy_undo_preset(runtime, req_string(body, "domain")?, req_string(body, "preset_id")?, req_string(body, "target")?, opt_string_vec(body, "also_targets")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "settings_get" => commands::settings::settings_get(runtime).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "settings_set_theme" => commands::settings::settings_set_theme(runtime, req_string(body, "theme")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "settings_set_font_scale" => commands::settings::settings_set_font_scale(runtime, req_string(body, "scale")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "settings_set_auto_check" => commands::settings::settings_set_auto_check(runtime, req_i64(body, "minutes")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "settings_set_git_auto_push" => commands::settings::settings_set_git_auto_push(runtime, req_bool(body, "enabled")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_list" => commands::skill::skill_list(runtime, opt_string(body, "domain")?, opt_string(body, "query")?, opt_string(body, "category")?, opt_bool(body, "uncategorized")?, opt_string_vec(body, "tags")?, opt_string(body, "agent")?, opt_bool(body, "no_tag")?, opt_string(body, "preset")?, opt_string(body, "deployed_to")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_get" => commands::skill::skill_get(runtime, req_string(body, "reference")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_read" => commands::skill::skill_read(runtime, req_string(body, "reference")?, opt_json::<LangArg>(body, "lang")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_file_read" => commands::skill::skill_file_read(runtime, req_string(body, "reference")?, req_string(body, "path")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_set" => commands::skill::skill_set(runtime, req_string(body, "reference")?, opt_string(body, "comment")?, opt_string_vec(body, "tags")?, opt_json::<SupportArg>(body, "agents")?, opt_json::<SupportArg>(body, "oses")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_tag_add" => commands::skill::skill_tag_add(runtime, req_string_vec(body, "references")?, req_string_vec(body, "tags")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_tag_detach" => commands::skill::skill_tag_detach(runtime, req_string_vec(body, "references")?, req_string_vec(body, "tags")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_tag_rename" => commands::skill::skill_tag_rename(runtime, req_string(body, "old")?, req_string(body, "new")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_tag_remove" => commands::skill::skill_tag_remove(runtime, req_string(body, "tag")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_move" => commands::skill::skill_move(runtime, req_string(body, "reference")?, opt_string(body, "category")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_category_rename" => commands::skill::skill_category_rename(runtime, req_string(body, "domain")?, req_string(body, "old")?, req_string(body, "new")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_archive_list" => commands::skill::skill_archive_list(runtime, opt_string(body, "domain")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_archive" => commands::skill::skill_archive(runtime, req_string_vec(body, "references")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_archive_restore" => commands::skill::skill_archive_restore(runtime, req_string_vec(body, "references")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_archive_delete" => commands::skill::skill_archive_delete(runtime, req_string_vec(body, "references")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_translation_get" => commands::skill::skill_translation_get(runtime, req_string(body, "reference")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_translation_set" => commands::skill::skill_translation_set(runtime, req_string(body, "reference")?, req_string(body, "content")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_translation_remove" => commands::skill::skill_translation_remove(runtime, req_string(body, "reference")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_overlay_list" => commands::skill::skill_overlay_list(runtime, req_string(body, "reference")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_overlay_get" => commands::skill::skill_overlay_get(runtime, req_string(body, "reference")?, req_string(body, "path")?, opt_string(body, "agent")?, opt_string(body, "os")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_overlay_set" => commands::skill::skill_overlay_set(runtime, req_string(body, "reference")?, req_string(body, "path")?, req_string(body, "content")?, opt_string(body, "agent")?, opt_string(body, "os")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_overlay_remove" => commands::skill::skill_overlay_remove(runtime, req_string(body, "reference")?, req_string(body, "path")?, opt_string(body, "agent")?, opt_string(body, "os")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_task" => commands::skill::skill_task(runtime, req_string_vec(body, "references")?, opt_string(body, "requirement")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "sync_pull" => commands::sync::sync_pull(runtime).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "sync_push" => commands::sync::sync_push(runtime, opt_string(body, "message")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "sync_status" => commands::sync::sync_status(runtime).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "sync_conflicts" => commands::sync::sync_conflicts(runtime).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "tool_list" => commands::tool::tool_list(runtime, opt_json::<ToolListParams>(body, "params")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "tool_get" => commands::tool::tool_get(runtime, req_string(body, "slug")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "tool_check" => commands::tool::tool_check(runtime, opt_string(body, "slug")?, opt_bool(body, "all")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "tool_task" => commands::tool::tool_task(runtime, opt_string(body, "slug")?, req_string(body, "action")?, opt_string(body, "requirement")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "tool_skill_candidates" => commands::tool::tool_skill_candidates(runtime, req_string(body, "slug")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "tool_register" => commands::tool::tool_register(runtime, req_string(body, "dir")?).map_err(InvokeError::Failed),
        "tool_set" => commands::tool::tool_set(runtime, req_string(body, "slug")?, req_string(body, "dir")?).map_err(InvokeError::Failed),
        "tool_set_install" => commands::tool::tool_set_install(runtime, req_string(body, "slug")?, req_string(body, "version")?, opt_string(body, "method")?, opt_string(body, "path")?, opt_string(body, "config_path")?).map_err(InvokeError::Failed),
        "tool_skill_link" => commands::tool::tool_skill_link(runtime, req_string(body, "slug")?, req_string(body, "reference")?).map_err(InvokeError::Failed),
        "skill_check" => commands::upstream::skill_check(runtime, opt_string(body, "reference")?, opt_string(body, "repo")?, opt_bool(body, "all")?, req_string(body, "job_id")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_update" => commands::upstream::skill_update(runtime, req_string(body, "reference")?, opt_string(body, "to")?, req_string(body, "job_id")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_update_repo" => commands::upstream::skill_update_repo(runtime, req_string(body, "repo")?, opt_string_vec(body, "include_new")?, opt_string(body, "domain")?, opt_string(body, "category")?, opt_bool(body, "uncategorized")?, req_string(body, "job_id")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_reject_new" => commands::upstream::skill_reject_new(runtime, req_string(body, "repo")?, req_string(body, "domain")?, req_string_vec(body, "skills")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_rejected" => commands::upstream::skill_rejected(runtime, opt_string(body, "repo")?, opt_string(body, "domain")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_skip" => commands::upstream::skill_skip(runtime, req_string(body, "reference")?, opt_string(body, "version")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_unskip" => commands::upstream::skill_unskip(runtime, req_string(body, "reference")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "skill_skipped" => commands::upstream::skill_skipped(runtime).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "vault_open" => commands::vault::vault_open(runtime, req_string(body, "path")?).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "vault_current" => commands::vault::vault_current(runtime).map(|value| serde_json::to_value(value).unwrap_or(serde_json::Value::Null)).map_err(InvokeError::Failed),
        "host_status" => Ok(serde_json::to_value(runtime.status_view()).unwrap_or(Value::Null)),
        "__session__" => Ok(serde_json::json!({ "sessionId": plans::new_session() })),
        "plans_confirm" => plans::confirm(runtime, &req_string(body, "planId")?, opt_string(body, "sessionId")?.as_deref()).map_err(InvokeError::Failed),
        "agent_set_context" => crate::agent::set_context(runtime, body.clone()).map_err(InvokeError::Failed),
        "agent_post_message" => crate::agent::post_message(runtime, body.clone()).map_err(InvokeError::Failed),
        "agent_cancel_task" => crate::agent::cancel_task(runtime, &req_string(body, "taskId")?, body.clone()).map_err(InvokeError::Failed),
        "agent_list_messages" => crate::agent::list_messages(runtime, &req_string(body, "sessionId")?).map_err(InvokeError::Failed),
        "agent_list_sessions" => crate::agent::list_sessions(runtime).map_err(InvokeError::Failed),
        _ => Err(InvokeError::Unknown(command.to_string())),
    }
}

fn field<'a>(body: &'a Value, snake: &str) -> Option<&'a Value> {
    body.get(snake).or_else(|| body.get(&to_camel(snake)))
}

fn to_camel(snake: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for ch in snake.chars() {
        if ch == '_' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn req_string(body: &Value, key: &str) -> Result<String, InvokeError> {
    field(body, key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| InvokeError::Failed(format!("缺少参数 {key}")))
}

fn opt_string(body: &Value, key: &str) -> Result<Option<String>, InvokeError> {
    match field(body, key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(InvokeError::Failed(format!("参数 {key} 应为字符串，实际为 {other}"))),
    }
}

fn req_bool(body: &Value, key: &str) -> Result<bool, InvokeError> {
    field(body, key)
        .and_then(Value::as_bool)
        .ok_or_else(|| InvokeError::Failed(format!("缺少布尔参数 {key}")))
}

fn opt_bool(body: &Value, key: &str) -> Result<Option<bool>, InvokeError> {
    match field(body, key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(v)) => Ok(Some(*v)),
        Some(other) => Err(InvokeError::Failed(format!("参数 {key} 应为布尔，实际为 {other}"))),
    }
}

fn req_i64(body: &Value, key: &str) -> Result<i64, InvokeError> {
    field(body, key)
        .and_then(Value::as_i64)
        .ok_or_else(|| InvokeError::Failed(format!("缺少整数参数 {key}")))
}

fn opt_i64(body: &Value, key: &str) -> Result<Option<i64>, InvokeError> {
    match field(body, key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_i64().map(Some).ok_or_else(|| InvokeError::Failed(format!("参数 {key} 应为整数"))),
    }
}

fn opt_usize(body: &Value, key: &str) -> Result<Option<usize>, InvokeError> {
    Ok(opt_i64(body, key)?.map(|v| v as usize))
}

fn req_string_vec(body: &Value, key: &str) -> Result<Vec<String>, InvokeError> {
    match field(body, key) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| v.as_str().map(str::to_string).ok_or_else(|| InvokeError::Failed(format!("参数 {key} 含非字符串"))))
            .collect(),
        _ => Err(InvokeError::Failed(format!("缺少字符串数组参数 {key}"))),
    }
}

fn opt_string_vec(body: &Value, key: &str) -> Result<Option<Vec<String>>, InvokeError> {
    match field(body, key) {
        None | Some(Value::Null) => Ok(None),
        _ => req_string_vec(body, key).map(Some),
    }
}

fn req_json<T: DeserializeOwned>(body: &Value, key: &str) -> Result<T, InvokeError> {
    let value = field(body, key).cloned().ok_or_else(|| InvokeError::Failed(format!("缺少参数 {key}")))?;
    serde_json::from_value(value).map_err(|err| InvokeError::Failed(format!("参数 {key} 无法解析：{err}")))
}

fn opt_json<T: DeserializeOwned>(body: &Value, key: &str) -> Result<Option<T>, InvokeError> {
    match field(body, key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => serde_json::from_value(value.clone())
            .map(Some)
            .map_err(|err| InvokeError::Failed(format!("参数 {key} 无法解析：{err}"))),
    }
}

#[allow(dead_code)]
fn object(body: &Value) -> Map<String, Value> {
    body.as_object().cloned().unwrap_or_default()
}
