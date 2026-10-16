//! `agent` 子命令：本机 Agent 配置与本地技能管理（T067/T068）。
//!
//! 参数与输出遵循 `specs/002-skill-management/contracts/cli.md`；业务规则全部在 core。
//! 检测只读本机存在事实；启停、顺序与目录只写本机状态，不进入 SkillManagerVault。

use clap::{Args, Subcommand};
use serde_json::{Value, json};
use skill_manager_core::deploy::{
    AgentConfigService, DirKind, LocalSkillService, TargetResolver, parse_target_arg,
};
use skill_manager_core::output::BatchOutcome;
use skill_manager_core::{AppContext, CliResult, CoreError, GlobalOptions};

#[derive(Args, Debug)]
pub struct AgentCommand {
    #[command(subcommand)]
    pub action: AgentAction,
}

#[derive(Subcommand, Debug)]
pub enum AgentAction {
    /// 列出全量内置 Agent 名单与本机状态。
    List,
    /// 重新检测本机 Agent（只读目录存在性）。
    Detect,
    /// 启用一个 Agent（只写本机状态）。
    Enable(KeyArgs),
    /// 停用一个 Agent（只写本机状态）。
    Disable(KeyArgs),
    /// 按给定顺序调整显示顺序（只写本机状态）。
    Reorder(ReorderArgs),
    /// 修改技能目录 / 项目相对路径 / 指令目标（只写本机状态）。
    SetDir(SetDirArgs),
    /// 恢复一类目录为内置默认。
    ResetDir(ResetDirArgs),
    /// 列出该目标目录下的本地技能（受管/仅本地）；目标为内置 Agent key、`shared` 或 `hermes:default`。
    LocalSkills(LocalTargetArgs),
    /// 只读：本机技能正文或指定文件，不用库内原件代替副本。
    ReadLocal(ReadLocalArgs),
    /// 删除该目标目录下的本地技能（未受管删文件不改库；受管走撤销与暂停规则）；目标为内置 Agent key、`shared` 或 `hermes:default`。
    DeleteLocal(DeleteLocalArgs),
}

#[derive(Args, Debug)]
pub struct KeyArgs {
    /// 内置 Agent key。
    #[arg(value_name = "key")]
    pub key: String,
}

#[derive(Args, Debug)]
pub struct LocalTargetArgs {
    /// 内置 Agent key、`shared`（共享目录是本机部署目标）或 `hermes:default`（Hermes default 档案）。
    #[arg(value_name = "target")]
    pub target: String,
}

#[derive(Args, Debug)]
pub struct ReadLocalArgs {
    #[arg(value_name = "target")]
    pub target: String,
    #[arg(value_name = "name")]
    pub name: String,
    /// 技能内相对文件路径；省略时阅读 SKILL.md 或 skill.md。
    #[arg(long, value_name = "path")]
    pub file: Option<String>,
}

#[derive(Args, Debug)]
pub struct ReorderArgs {
    /// 顺序，逗号分隔的 Agent key。
    #[arg(long = "order", value_name = "key,...")]
    pub order: String,
}

#[derive(Args, Debug)]
pub struct SetDirArgs {
    #[arg(value_name = "key")]
    pub key: String,
    /// 技能目录（绝对路径）。
    #[arg(long, value_name = "path")]
    pub skills: Option<String>,
    /// 项目技能相对路径（仅配置保留）。
    #[arg(long = "project-rel", value_name = "path")]
    pub project_rel: Option<String>,
    /// 指令分发目标（`AGENTS.md` 位置，绝对路径）。
    #[arg(long, value_name = "path")]
    pub instructions: Option<String>,
}

#[derive(Args, Debug)]
pub struct ResetDirArgs {
    #[arg(value_name = "key")]
    pub key: String,
    /// 目录种类：skills、project-rel 或 instructions。
    #[arg(long, value_name = "skills|project-rel|instructions")]
    pub kind: String,
}

#[derive(Args, Debug)]
pub struct DeleteLocalArgs {
    /// 内置 Agent key、`shared`（共享目录是本机部署目标）或 `hermes:default`（Hermes default 档案）。
    #[arg(value_name = "target")]
    pub target: String,
    /// 技能目录名（可重复）。
    #[arg(value_name = "name", required = true)]
    pub names: Vec<String>,
}

type Result<T> = std::result::Result<T, CoreError>;

/// 分发 `agent` 子命令。
pub fn run(
    command: AgentCommand,
    ctx: &mut AppContext,
    _options: &GlobalOptions,
) -> (CliResult, String) {
    match dispatch(command.action, ctx) {
        Ok(pair) => pair,
        Err(err) => (CliResult::failed_error(err), String::new()),
    }
}

fn dispatch(action: AgentAction, ctx: &AppContext) -> Result<(CliResult, String)> {
    match action {
        AgentAction::List => list(ctx, false),
        AgentAction::Detect => list(ctx, true),
        AgentAction::Enable(args) => set_enabled(ctx, &args.key, true),
        AgentAction::Disable(args) => set_enabled(ctx, &args.key, false),
        AgentAction::Reorder(args) => reorder(ctx, &args.order),
        AgentAction::SetDir(args) => set_dir(ctx, args),
        AgentAction::ResetDir(args) => reset_dir(ctx, args),
        AgentAction::LocalSkills(args) => local_skills(ctx, &args.target),
        AgentAction::ReadLocal(args) => {
            let resolver = ctx.target_resolver()?;
            let target = parse_target_arg(&args.target)?;
            let view = local_service(ctx, &resolver)?.read(&target, &args.name, args.file.as_deref())?;
            let human = view.text.clone().unwrap_or_else(|| format!("非 UTF-8 文本（{} 字节）", view.size));
            Ok((CliResult::from_data(&view), human))
        }
        AgentAction::DeleteLocal(args) => delete_local(ctx, &args.target, &args.names),
    }
}

fn service<'a>(ctx: &'a AppContext) -> Result<AgentConfigService<'a>> {
    Ok(AgentConfigService::new(
        ctx.home()?,
        ctx.catalog(),
        ctx.state(),
    ))
}

fn local_service<'a>(
    ctx: &'a AppContext,
    resolver: &'a TargetResolver<'static>,
) -> Result<LocalSkillService<'a>> {
    Ok(LocalSkillService::new(
        resolver,
        ctx.state(),
        ctx.vault_opt().map(|vault| vault.root().to_path_buf()),
    ))
}

// ---------------------------------------------------------------------------
// list / detect
// ---------------------------------------------------------------------------

fn list(ctx: &AppContext, detect: bool) -> Result<(CliResult, String)> {
    let service = service(ctx)?;
    let views = if detect {
        service.detect()?
    } else {
        service.list()?
    };
    let detected = views.iter().filter(|view| view.detected).count();
    let enabled = views.iter().filter(|view| view.detected && view.enabled).count();

    let data = json!({
        "agents": views,
        "total": views.len(),
        "detected": detected,
        "enabled": enabled,
    });

    let mut human = format!(
        "内置 Agent {} 项：已检测 {detected}，已启用 {enabled}{}",
        views.len(),
        if detect { "（本次为重新检测）" } else { "" }
    );
    for view in views.iter().filter(|view| view.detected) {
        human.push_str(&format!(
            "\n- {}（{}）{} {}",
            view.display_name,
            view.key,
            if view.enabled { "已启用" } else { "已停用" },
            view.skills_dir
        ));
    }
    Ok((CliResult::from_data(&data), human))
}

// ---------------------------------------------------------------------------
// enable / disable / reorder
// ---------------------------------------------------------------------------

fn set_enabled(ctx: &AppContext, key: &str, enabled: bool) -> Result<(CliResult, String)> {
    let view = service(ctx)?.set_enabled(key, enabled)?;
    let data = json!({ "agent": view });
    let human = format!(
        "已{} {}（本机配置；技能目录：{}）",
        if enabled { "启用" } else { "停用" },
        view.display_name,
        view.skills_dir
    );
    Ok((CliResult::from_data(&data), human))
}

fn reorder(ctx: &AppContext, order: &str) -> Result<(CliResult, String)> {
    let keys: Vec<String> = order
        .split([',', '，'])
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect();
    if keys.is_empty() {
        return Err(CoreError::other("--order 不能为空"));
    }
    let views = service(ctx)?.reorder(&keys)?;
    let data = json!({ "agents": views });
    let human = format!("已按给定顺序调整 {} 个 Agent 的显示顺序", keys.len());
    Ok((CliResult::from_data(&data), human))
}

// ---------------------------------------------------------------------------
// set-dir / reset-dir
// ---------------------------------------------------------------------------

fn set_dir(ctx: &AppContext, args: SetDirArgs) -> Result<(CliResult, String)> {
    let provided: Vec<(&str, &String)> = [
        ("skills", &args.skills),
        ("project-rel", &args.project_rel),
        ("instructions", &args.instructions),
    ]
    .into_iter()
    .filter_map(|(kind, value)| value.as_ref().map(|value| (kind, value)))
    .collect();
    if provided.len() != 1 {
        return Err(CoreError::other(
            "必须且只能指定 --skills、--project-rel、--instructions 之一",
        ));
    }
    let (kind_raw, value) = provided[0];
    let kind = DirKind::parse(kind_raw)?;
    let view = service(ctx)?.set_dir(&args.key, kind, value)?;
    let data = json!({ "agent": view });
    let human = format!(
        "已更新 {} 的{}（本机配置）",
        view.display_name,
        dir_label(kind)
    );
    Ok((CliResult::from_data(&data), human))
}

fn reset_dir(ctx: &AppContext, args: ResetDirArgs) -> Result<(CliResult, String)> {
    let kind = DirKind::parse(&args.kind)?;
    let view = service(ctx)?.reset_dir(&args.key, kind)?;
    let data = json!({ "agent": view });
    let human = format!(
        "已恢复 {} 的{}为内置默认",
        view.display_name,
        dir_label(kind)
    );
    Ok((CliResult::from_data(&data), human))
}

fn dir_label(kind: DirKind) -> &'static str {
    match kind {
        DirKind::Skills => "技能目录",
        DirKind::ProjectRel => "项目相对路径",
        DirKind::Instructions => "指令目标",
    }
}

// ---------------------------------------------------------------------------
// local-skills / delete-local
// ---------------------------------------------------------------------------

fn local_skills(ctx: &AppContext, raw_target: &str) -> Result<(CliResult, String)> {
    let target = parse_target_arg(raw_target)?;
    let resolver = ctx.target_resolver()?;
    let service = local_service(ctx, &resolver)?;
    let dir = service.skills_dir(&target)?;
    let views = service.list(&target)?;
    let managed = views.iter().filter(|view| view.managed).count();
    let label = raw_target.trim();

    let data = json!({
        "agent": label,
        "skills_dir": dir.display().to_string(),
        "skills": views,
        "managed": managed,
        "total": views.len(),
    });

    let mut human = format!(
        "{}：{} · {} 项（受管理 {}）",
        label,
        dir.display(),
        views.len(),
        managed
    );
    for view in &views {
        human.push_str(&format!(
            "\n- {}（{}）{}",
            view.name,
            match view.status {
                "managed" => "已同步",
                "paused_local_change" => "本地有变更",
                _ => "仅本地",
            },
            view.path
        ));
    }
    Ok((CliResult::from_data(&data), human))
}

fn delete_local(ctx: &AppContext, raw_target: &str, names: &[String]) -> Result<(CliResult, String)> {
    let target = parse_target_arg(raw_target)?;
    let resolver = ctx.target_resolver()?;
    let service = local_service(ctx, &resolver)?;
    let outcome = service.delete(&target, names)?;

    let mut batch = BatchOutcome::default();
    for item in &outcome.removed {
        batch.push_ok(serde_json::to_value(item).unwrap_or(Value::Null));
    }
    for error in outcome.errors {
        batch.push_err(error);
    }
    let result = batch.into_result("removed");

    let mut human = format!(
        "已删除 {} 项本机副本（未受管项不改库；受管项走撤销规则）",
        outcome.removed.len()
    );
    if !result.errors.is_empty() {
        human.push_str(&format!("；{} 项未完成", result.errors.len()));
    }
    Ok((result, human))
}
