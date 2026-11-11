//! Agent 命令：本机 Agent 配置、本地技能列表、阅读与删除，以及共享目录自动定位。
//!
//! 薄封装 core：检测只读事实；启停/顺序/目录只写本机状态；删除受管项走 US3 撤销与暂停规则。

use serde::Serialize;
use skill_manager_core::deploy::{
    AgentConfigService, AgentView, DirKind, LocalDeleteOutcome, LocalSkillRead, LocalSkillService,
    LocalSkillView, TargetResolver, parse_target_arg,
};
use skill_manager_core::vault::{
    InstructionScope, InstructionSourceView, InstructionTargetResult, InstructionsService,
};

use crate::HostRuntime;

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
pub fn agent_list(runtime: &HostRuntime) -> Result<Vec<AgentView>, String> {
    let ctx = runtime.get()?;
    config_service(&ctx)?.list().map_err(|err| err.to_string())
}

/// 重新检测本机 Agent（只读目录存在性）。
pub fn agent_detect(runtime: &HostRuntime) -> Result<Vec<AgentView>, String> {
    let ctx = runtime.get()?;
    config_service(&ctx)?.detect().map_err(|err| err.to_string())
}

/// 启用一个 Agent；只写本机状态。
pub fn agent_enable(
    runtime: &HostRuntime,
    key: String,
) -> Result<AgentView, String> {
    let ctx = runtime.get()?;
    config_service(&ctx)?
        .set_enabled(&key, true)
        .map_err(|err| err.to_string())
}

/// 停用一个 Agent；只写本机状态。
pub fn agent_disable(
    runtime: &HostRuntime,
    key: String,
) -> Result<AgentView, String> {
    let ctx = runtime.get()?;
    config_service(&ctx)?
        .set_enabled(&key, false)
        .map_err(|err| err.to_string())
}

/// 调整显示顺序；只写本机状态。
pub fn agent_reorder(
    runtime: &HostRuntime,
    order: Vec<String>,
) -> Result<Vec<AgentView>, String> {
    let ctx = runtime.get()?;
    config_service(&ctx)?
        .reorder(&order)
        .map_err(|err| err.to_string())
}

/// 修改一类目录；`kind` 为 `skills` / `project-rel` / `instructions`。
pub fn agent_set_dir(
    runtime: &HostRuntime,
    key: String,
    kind: String,
    path: String,
) -> Result<AgentView, String> {
    let ctx = runtime.get()?;
    let kind = DirKind::parse(&kind).map_err(|err| err.to_string())?;
    config_service(&ctx)?
        .set_dir(&key, kind, &path)
        .map_err(|err| err.to_string())
}

/// 恢复一类目录为内置默认。
pub fn agent_reset_dir(
    runtime: &HostRuntime,
    key: String,
    kind: String,
) -> Result<AgentView, String> {
    let ctx = runtime.get()?;
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

/// 列出目标技能目录下的本地技能（受管/仅本地）；目标为内置 Agent key、`shared` 或 `hermes:default`。
pub fn agent_local_skills(
    runtime: &HostRuntime,
    target: String,
) -> Result<LocalSkillsView, String> {
    let ctx = runtime.get()?;
    let target = parse_target_arg(&target).map_err(|err| err.to_string())?;
    let resolver: TargetResolver<'static> = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = LocalSkillService::new(
        &resolver,
        ctx.state(),
        ctx.vault_opt().map(|vault| vault.root().to_path_buf()),
    );
    let skills_dir = service
        .skills_dir(&target)
        .map_err(|err| err.to_string())?;
    let skills = service.list(&target).map_err(|err| err.to_string())?;
    let managed = skills.iter().filter(|skill| skill.managed).count();
    Ok(LocalSkillsView {
        agent: target.as_str(),
        skills_dir: skills_dir.display().to_string(),
        total: skills.len(),
        managed,
        skills,
    })
}

/// 只读本机技能正文或目录内文件。
pub fn agent_read_local(
    runtime: &HostRuntime,
    target: String,
    name: String,
    file: Option<String>,
) -> Result<LocalSkillRead, String> {
    let ctx = runtime.get()?;
    let target = parse_target_arg(&target).map_err(|err| err.to_string())?;
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    LocalSkillService::new(&resolver, ctx.state(), ctx.vault_opt().map(|vault| vault.root().to_path_buf()))
        .read(&target, &name, file.as_deref())
        .map_err(|err| err.to_string())
}

/// 删除该目标本机技能副本；未受管直接删文件且不改库，受管走撤销与暂停规则。
pub fn agent_delete_local(
    runtime: &HostRuntime,
    target: String,
    names: Vec<String>,
) -> Result<LocalDeleteOutcome, String> {
    let ctx = runtime.get()?;
    let target = parse_target_arg(&target).map_err(|err| err.to_string())?;
    let resolver: TargetResolver<'static> = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = LocalSkillService::new(
        &resolver,
        ctx.state(),
        ctx.vault_opt().map(|vault| vault.root().to_path_buf()),
    );
    service
        .delete(&target, &names)
        .map_err(|err| err.to_string())
}

/// 读取按当前系统用户主目录自动定位的共享部署路径。
pub fn shared_dir_get(runtime: &HostRuntime) -> Result<String, String> {
    let ctx = runtime.get()?;
    Ok(ctx.shared_dir().map_err(|err| err.to_string())?.display().to_string())
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
pub fn instruction_get(
    runtime: &HostRuntime,
    scope: String,
) -> Result<InstructionSourceView, String> {
    let ctx = runtime.get()?;
    let scope = InstructionScope::parse(&scope).map_err(|err| err.to_string())?;
    instructions_service(&ctx)?
        .get(&scope)
        .map_err(|err| err.to_string())
}

/// 保存某作用域的指令源，并按“全局+专属”覆盖本机已启用目标。
pub fn instruction_set(
    runtime: &HostRuntime,
    scope: String,
    content: String,
) -> Result<Vec<InstructionTargetResult>, String> {
    let ctx = runtime.get()?;
    let scope = InstructionScope::parse(&scope).map_err(|err| err.to_string())?;
    instructions_service(&ctx)?
        .save(&scope, &content)
        .map_err(|err| err.to_string())
}
