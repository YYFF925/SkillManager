//! Agent 命令：本机 Agent 配置、本地技能列表与删除，以及共享部署目录设置。
//!
//! 薄封装 core：检测只读事实；启停/顺序/目录只写本机状态；删除受管项走 US3 撤销与暂停规则。

use serde::Serialize;
use skill_manager_core::deploy::{
    AgentConfigService, AgentView, DirKind, LocalDeleteOutcome, LocalSkillService,
    LocalSkillView, TargetResolver,
};
use skill_manager_core::vault::{
    InstructionScope, InstructionSourceView, InstructionTargetResult, InstructionsService,
};

use super::AppState;

fn config_service<'a>(
    ctx: &'a skill_manager_core::AppContext,
) -> Result<AgentConfigService<'a>, String> {
    Ok(AgentConfigService::new(
        ctx.home().map_err(|err| err.to_string())?,
        ctx.catalog(),
        ctx.state(),
    ))
}

/// 列出全量内置 Agent 名单与本机状态。
#[tauri::command]
pub fn agent_list(state: tauri::State<'_, AppState>) -> Result<Vec<AgentView>, String> {
    let ctx = state.get()?;
    config_service(&ctx)?.list().map_err(|err| err.to_string())
}

/// 重新检测本机 Agent（只读目录存在性）。
#[tauri::command]
pub fn agent_detect(state: tauri::State<'_, AppState>) -> Result<Vec<AgentView>, String> {
    let ctx = state.get()?;
    config_service(&ctx)?.detect().map_err(|err| err.to_string())
}

/// 启用一个 Agent；只写本机状态。
#[tauri::command]
pub fn agent_enable(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<AgentView, String> {
    let ctx = state.get()?;
    config_service(&ctx)?
        .set_enabled(&key, true)
        .map_err(|err| err.to_string())
}

/// 停用一个 Agent；只写本机状态。
#[tauri::command]
pub fn agent_disable(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<AgentView, String> {
    let ctx = state.get()?;
    config_service(&ctx)?
        .set_enabled(&key, false)
        .map_err(|err| err.to_string())
}

/// 调整显示顺序；只写本机状态。
#[tauri::command]
pub fn agent_reorder(
    state: tauri::State<'_, AppState>,
    order: Vec<String>,
) -> Result<Vec<AgentView>, String> {
    let ctx = state.get()?;
    config_service(&ctx)?
        .reorder(&order)
        .map_err(|err| err.to_string())
}

/// 修改一类目录；`kind` 为 `skills` / `project-rel` / `instructions`。
#[tauri::command]
pub fn agent_set_dir(
    state: tauri::State<'_, AppState>,
    key: String,
    kind: String,
    path: String,
) -> Result<AgentView, String> {
    let ctx = state.get()?;
    let kind = DirKind::parse(&kind).map_err(|err| err.to_string())?;
    config_service(&ctx)?
        .set_dir(&key, kind, &path)
        .map_err(|err| err.to_string())
}

/// 恢复一类目录为内置默认。
#[tauri::command]
pub fn agent_reset_dir(
    state: tauri::State<'_, AppState>,
    key: String,
    kind: String,
) -> Result<AgentView, String> {
    let ctx = state.get()?;
    let kind = DirKind::parse(&kind).map_err(|err| err.to_string())?;
    config_service(&ctx)?
        .reset_dir(&key, kind)
        .map_err(|err| err.to_string())
}

/// 本地技能列表结果。
#[derive(Clone, Debug, Serialize)]
pub struct LocalSkillsView {
    pub agent: String,
    pub skills_dir: String,
    pub skills: Vec<LocalSkillView>,
    pub managed: usize,
    pub total: usize,
}

/// 列出该 Agent 技能目录下的本地技能（受管/仅本地）。
#[tauri::command]
pub fn agent_local_skills(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<LocalSkillsView, String> {
    let ctx = state.get()?;
    let resolver: TargetResolver<'static> = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = LocalSkillService::new(
        &resolver,
        ctx.state(),
        ctx.vault_opt().map(|vault| vault.root().to_path_buf()),
    );
    let skills_dir = service.skills_dir(&key).map_err(|err| err.to_string())?;
    let skills = service.list(&key).map_err(|err| err.to_string())?;
    let managed = skills.iter().filter(|skill| skill.managed).count();
    Ok(LocalSkillsView {
        agent: key,
        skills_dir: skills_dir.display().to_string(),
        total: skills.len(),
        managed,
        skills,
    })
}

/// 删除该 Agent 本机技能副本；未受管直接删文件且不改库，受管走撤销与暂停规则。
#[tauri::command]
pub fn agent_delete_local(
    state: tauri::State<'_, AppState>,
    key: String,
    names: Vec<String>,
) -> Result<LocalDeleteOutcome, String> {
    let ctx = state.get()?;
    let resolver: TargetResolver<'static> = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = LocalSkillService::new(
        &resolver,
        ctx.state(),
        ctx.vault_opt().map(|vault| vault.root().to_path_buf()),
    );
    service
        .delete(&key, &names)
        .map_err(|err| err.to_string())
}

/// 读取共享部署目录设置；未设置时为 null。
#[tauri::command]
pub fn shared_dir_get(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    let ctx = state.get()?;
    Ok(ctx
        .state()
        .shared_dir()
        .map_err(|err| err.to_string())?
        .map(|path| path.display().to_string()))
}

/// 设置或清除共享部署目录；传空字符串表示清除。
#[tauri::command]
pub fn shared_dir_set(
    state: tauri::State<'_, AppState>,
    path: Option<String>,
) -> Result<Option<String>, String> {
    let ctx = state.get()?;
    let value = path.map(|value| value.trim().to_string()).filter(|value| !value.is_empty());
    match &value {
        Some(value) => ctx
            .state()
            .set_shared_dir(value)
            .map_err(|err| err.to_string())?,
        None => ctx
            .state()
            .clear_shared_dir()
            .map_err(|err| err.to_string())?,
    }
    Ok(ctx
        .state()
        .shared_dir()
        .map_err(|err| err.to_string())?
        .map(|path| path.display().to_string()))
}

fn instructions_service<'a>(
    ctx: &'a skill_manager_core::AppContext,
) -> Result<InstructionsService<'a>, String> {
    let root = ctx
        .vault()
        .map_err(|err| err.to_string())?
        .root()
        .to_path_buf();
    Ok(InstructionsService::new(
        root,
        ctx.home().map_err(|err| err.to_string())?,
        ctx.catalog(),
        ctx.state(),
    ))
}

/// 读取某作用域的指令源与合成预览（`scope` 为 `global` 或 Agent key）。
#[tauri::command]
pub fn instruction_get(
    state: tauri::State<'_, AppState>,
    scope: String,
) -> Result<InstructionSourceView, String> {
    let ctx = state.get()?;
    let scope = InstructionScope::parse(&scope).map_err(|err| err.to_string())?;
    instructions_service(&ctx)?
        .get(&scope)
        .map_err(|err| err.to_string())
}

/// 保存某作用域的指令源，并按“全局+专属”覆盖本机已启用目标。
#[tauri::command]
pub fn instruction_set(
    state: tauri::State<'_, AppState>,
    scope: String,
    content: String,
) -> Result<Vec<InstructionTargetResult>, String> {
    let ctx = state.get()?;
    let scope = InstructionScope::parse(&scope).map_err(|err| err.to_string())?;
    instructions_service(&ctx)?
        .save(&scope, &content)
        .map_err(|err| err.to_string())
}
