//! `deploy` 子命令：部署、撤销、同步、预设叠加部署与本机部署状态。
//!
//! 参数与输出遵循 `specs/002-skill-management/contracts/cli.md`；业务规则全部在 core。

use clap::{Args, Subcommand};
use serde_json::{Value, json};
use skill_manager_core::deploy::{
    DeployReport, DeploymentService, TargetResolver,
};
use skill_manager_core::domain::{Domain, SkillRef, Target};
use skill_manager_core::output::BatchOutcome;
use skill_manager_core::{AppContext, CliResult, CoreError, GlobalOptions, ItemError};

#[derive(Args, Debug)]
pub struct DeployCommand {
    #[command(subcommand)]
    pub action: DeployAction,
}

#[derive(Subcommand, Debug)]
pub enum DeployAction {
    /// 部署技能到目标（可指定多个技能与多个目标）。
    To(DeployToArgs),
    /// 按预设成员叠加部署到目标。
    Preset(DeployPresetArgs),
    /// 撤销部署（保留库内技能）。
    Undo(DeployUndoArgs),
    /// 按预设当前成员逐个撤销（保留库内技能）。
    #[command(name = "undo-preset")]
    UndoPreset(DeployUndoPresetArgs),
    /// 同步本机已有部署（不新增目标）。
    Sync(DeploySyncArgs),
    /// 查看本机部署状态。
    Status(DeployStatusArgs),
}

#[derive(Args, Debug)]
pub struct DeployPresetArgs {
    /// 预设所属管理域：code 或 lobster。
    #[arg(value_name = "domain")]
    pub domain: String,
    /// 预设 id。
    #[arg(value_name = "preset-id")]
    pub preset_id: String,
    /// 目标：Agent key、`shared` 或 `hermes:default`。
    #[arg(long = "to", value_name = "target", required = true)]
    pub to: String,
    /// 本次同时处理的其他目标（可重复）。
    #[arg(long = "also-target", value_name = "target")]
    pub also_target: Vec<String>,
    /// 只报告将处理的技能、目标与路径，不修改文件或记录。
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct DeployUndoPresetArgs {
    #[arg(value_name = "domain")]
    pub domain: String,
    #[arg(value_name = "preset-id")]
    pub preset_id: String,
    /// 撤销来源目标：Agent key、`shared` 或 `hermes:default`。
    #[arg(long = "from", value_name = "target", required = true)]
    pub from: String,
    /// 本次同时处理的其他目标（可重复）。
    #[arg(long = "also-target", value_name = "target")]
    pub also_target: Vec<String>,
    /// 只报告将处理的技能、目标与路径，不修改文件或记录。
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct DeployToArgs {
    /// 目标：Agent key、`shared` 或 `hermes:default`。
    #[arg(value_name = "target")]
    pub target: String,
    /// 技能引用 `<domain>/<name>`（可重复）。
    #[arg(value_name = "ref", required = true)]
    pub references: Vec<String>,
    /// 本次同时处理的其他目标（可重复）。
    #[arg(long = "also-target", value_name = "target")]
    pub also_target: Vec<String>,
    /// 只报告将处理的技能、目标与路径，不修改文件或记录。
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct DeployUndoArgs {
    #[arg(value_name = "target")]
    pub target: String,
    #[arg(value_name = "ref", required = true)]
    pub references: Vec<String>,
    #[arg(long = "also-target", value_name = "target")]
    pub also_target: Vec<String>,
    #[arg(long = "dry-run")]
    pub dry_run: bool,
    /// 确认丢弃已被外部修改的待清理副本。
    #[arg(long = "discard-local-changes")]
    pub discard_local_changes: bool,
}

#[derive(Args, Debug)]
pub struct DeploySyncArgs {
    /// 收窄到单个技能引用。
    #[arg(long = "ref", value_name = "ref")]
    pub reference: Option<String>,
    /// 收窄到单个目标。
    #[arg(long = "target", value_name = "target")]
    pub target: Option<String>,
    /// 用户裁决后显式按库覆盖被外部修改的副本。
    #[arg(long = "overwrite-local")]
    pub overwrite_local: bool,
}

#[derive(Args, Debug)]
pub struct DeployStatusArgs {
    #[arg(long = "target", value_name = "target")]
    pub target: Option<String>,
    /// 只列出被外部修改而暂停的项。
    #[arg(long = "paused")]
    pub paused: bool,
}

/// 解析 CLI 目标参数：Agent key、`shared` 或 `hermes:default`。
pub fn parse_target(raw: &str) -> Result<Target> {
    skill_manager_core::deploy::parse_target_arg(raw)
}

/// 分发 `deploy` 子命令。
pub fn run(
    command: DeployCommand,
    ctx: &mut AppContext,
    _options: &GlobalOptions,
) -> (CliResult, String) {
    match dispatch(command.action, ctx) {
        Ok(pair) => pair,
        Err(err) => (CliResult::failed_error(err), String::new()),
    }
}

type Result<T> = std::result::Result<T, CoreError>;

fn dispatch(action: DeployAction, ctx: &mut AppContext) -> Result<(CliResult, String)> {
    match action {
        DeployAction::To(args) => deploy_to(ctx, args),
        DeployAction::Preset(args) => deploy_preset(ctx, args),
        DeployAction::Undo(args) => undo(ctx, args),
        DeployAction::UndoPreset(args) => undo_preset(ctx, args),
        DeployAction::Sync(args) => sync(ctx, args),
        DeployAction::Status(args) => status(ctx, args),
    }
}

/// 建立部署服务（库根 + 目标解析器 + 本机状态）。
fn with_service<T>(
    ctx: &AppContext,
    run: impl FnOnce(&DeploymentService<'_>) -> Result<T>,
) -> Result<T> {
    let root = ctx.vault()?.root().to_path_buf();
    let resolver: TargetResolver<'static> = ctx.target_resolver()?;
    let service = DeploymentService::new(root, &resolver, ctx.state());
    run(&service)
}

fn parse_refs(raw: &[String]) -> Result<Vec<SkillRef>> {
    raw.iter().map(|value| SkillRef::parse(value)).collect()
}

/// 把本次未完成项写入本机运行日志（供诊断导出定位部署异常）。
fn log_failures(ctx: &AppContext, action: &str, report: &DeployReport) {
    for error in &report.failures {
        let mut message = error.message.clone();
        if let Some(skill) = &error.skill {
            message = format!("{skill}：{message}");
        }
        if let Some(target) = &error.target {
            message = format!("[{target}] {message}");
        }
        if let Some(path) = &error.path {
            message = format!("{path}：{message}");
        }
        let _ = skill_manager_core::diagnostics::record(ctx.state(), "error", action, &message);
    }
}

fn parse_targets(primary: &str, also: &[String]) -> Result<Vec<Target>> {
    let mut targets = vec![parse_target(primary)?];
    for raw in also {
        targets.push(parse_target(raw)?);
    }
    Ok(targets)
}

// ---------------------------------------------------------------------------
// to
// ---------------------------------------------------------------------------

fn deploy_to(ctx: &AppContext, args: DeployToArgs) -> Result<(CliResult, String)> {
    let references = parse_refs(&args.references)?;
    let targets = parse_targets(&args.target, &args.also_target)?;
    let report = with_service(ctx, |service| service.deploy_to(&references, &targets, args.dry_run))?;
    log_failures(ctx, "deploy", &report);
    Ok(report_result(report, Some(args.dry_run)))
}

// ---------------------------------------------------------------------------
// preset / undo-preset
// ---------------------------------------------------------------------------

fn deploy_preset(ctx: &AppContext, args: DeployPresetArgs) -> Result<(CliResult, String)> {
    let domain = args.domain.parse::<Domain>()?;
    let targets = parse_targets(&args.to, &args.also_target)?;
    let report = with_service(ctx, |service| {
        service.deploy_preset(domain, &args.preset_id, &targets, args.dry_run)
    })?;
    log_failures(ctx, "deploy-preset", &report);
    Ok(report_result(report, Some(args.dry_run)))
}

fn undo_preset(ctx: &AppContext, args: DeployUndoPresetArgs) -> Result<(CliResult, String)> {
    let domain = args.domain.parse::<Domain>()?;
    let targets = parse_targets(&args.from, &args.also_target)?;
    let report = with_service(ctx, |service| {
        service.undo_preset(domain, &args.preset_id, &targets, args.dry_run)
    })?;
    log_failures(ctx, "undo-preset", &report);
    Ok(report_result(report, Some(args.dry_run)))
}

// ---------------------------------------------------------------------------
// undo
// ---------------------------------------------------------------------------

fn undo(ctx: &AppContext, args: DeployUndoArgs) -> Result<(CliResult, String)> {
    let references = parse_refs(&args.references)?;
    let targets = parse_targets(&args.target, &args.also_target)?;
    let report = with_service(ctx, |service| {
        service.undo(&references, &targets, args.dry_run, args.discard_local_changes)
    })?;
    log_failures(ctx, "undo", &report);
    Ok(report_result(report, Some(args.dry_run)))
}

// ---------------------------------------------------------------------------
// sync
// ---------------------------------------------------------------------------

fn sync(ctx: &AppContext, args: DeploySyncArgs) -> Result<(CliResult, String)> {
    let reference = match &args.reference {
        Some(raw) => Some(SkillRef::parse(raw)?),
        None => None,
    };
    let target = match &args.target {
        Some(raw) => Some(parse_target(raw)?),
        None => None,
    };
    let report = with_service(ctx, |service| {
        service.sync(reference.as_ref(), target.as_ref(), args.overwrite_local)
    })?;
    log_failures(ctx, "deploy-sync", &report);
    Ok(report_result(report, None))
}

// ---------------------------------------------------------------------------
// status
// ---------------------------------------------------------------------------

fn status(ctx: &AppContext, args: DeployStatusArgs) -> Result<(CliResult, String)> {
    let target = match &args.target {
        Some(raw) => Some(parse_target(raw)?),
        None => None,
    };
    let views = with_service(ctx, |service| service.status(target.as_ref(), args.paused))?;
    let data = json!({ "deployments": views });

    let mut human = format!("本机部署记录 {} 项", views.len());
    for view in &views {
        human.push_str(&format!(
            "\n- {} → {}（{}）{}",
            view.skill, view.target, view.status, view.target_path
        ));
        if !view.library_exists {
            human.push_str(" [库内条目已删除，副本待处置]");
        }
        if let Some(warning) = &view.warning {
            human.push_str(&format!(" [{warning}]"));
        }
    }
    Ok((CliResult::from_data(&data), human))
}

// ---------------------------------------------------------------------------
// skill delete 共用入口
// ---------------------------------------------------------------------------

/// `skill delete <ref>...`：删库并清理受管部署。
pub fn delete_skills(ctx: &AppContext, references: &[SkillRef]) -> Result<(CliResult, String)> {
    let outcome = with_service(ctx, |service| service.delete_skills(references))?;
    let mut errors: Vec<ItemError> = outcome.paused.clone();
    errors.extend(outcome.failures.clone());
    let data = json!({
        "deleted": outcome.deleted,
        "cleanup": outcome.removed,
    });

    let mut human = format!(
        "已删除库内技能 {} 项，清理副本 {} 项",
        outcome.deleted.len(),
        outcome.removed.len()
    );
    if !outcome.paused.is_empty() {
        human.push_str(&format!(
            "；{} 项副本清理暂停（已保留定位记录）",
            outcome.paused.len()
        ));
    }

    let result = if errors.is_empty() {
        CliResult::success(data)
    } else if outcome.deleted.is_empty() {
        CliResult::failed(errors)
    } else {
        CliResult::partial(data, errors)
    };
    Ok((result, human))
}

// ---------------------------------------------------------------------------
// 输出
// ---------------------------------------------------------------------------

fn report_result(report: DeployReport, dry_run: Option<bool>) -> (CliResult, String) {
    let mut batch = BatchOutcome::default();
    for item in &report.items {
        batch.push_ok(serde_json::to_value(item).unwrap_or(Value::Null));
    }
    for error in report.failures {
        batch.push_err(error);
    }

    let mut result = batch.into_result("items");
    if let (Some(dry_run), Value::Object(map)) = (dry_run, &mut result.data) {
        map.insert("dry_run".to_string(), Value::Bool(dry_run));
    }

    let mut human = String::new();
    if dry_run == Some(true) {
        human.push_str("（预演：只报告，不修改文件与本机记录）\n");
    }
    human.push_str(&format!(
        "处理 {} 项，未完成 {} 项",
        report_items_len(&result),
        result.errors.len()
    ));
    for item in &report.items {
        human.push_str(&format!(
            "\n- {}/{} → {}（{}）{}",
            item.skill, item.target, item.action, item.path, item.status
        ));
        if let Some(message) = &item.message {
            human.push_str(&format!(" [{message}]"));
        }
    }
    (result, human)
}

fn report_items_len(result: &CliResult) -> usize {
    result
        .data
        .get("items")
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}
