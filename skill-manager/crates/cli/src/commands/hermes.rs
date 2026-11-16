//! `hermes` 子命令：本机单 Hermes 根下的档案、技能启用与人格源（T060 / T145）。
//!
//! 参数与输出遵循 `specs/002-skill-management/contracts/cli.md`；业务规则全部在 core。
//! default 档案由 `hermes:default` 部署记录表达（`deploy` 维护）；其他档案在此明确启停。
//! 不提供档案创建/删除/改名、活动切换或多实例。`SOUL.md` 由库内 `hermes-identity` 源无条件覆盖。

use std::io::Read;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::{json, Value};
use skill_manager_core::hermes::{HermesService, IdentityScope, IdentityService, LONG_SOUL_CHARS};
use skill_manager_core::output::BatchOutcome;
use skill_manager_core::{AppContext, CliResult, CoreError, GlobalOptions};

#[derive(Args, Debug)]
pub struct HermesCommand {
    #[command(subcommand)]
    pub action: HermesAction,
}

#[derive(Subcommand, Debug)]
pub enum HermesAction {
    /// 列出本机现有档案（default 与具名档案）。
    Profiles,
    /// 只读查看档案实际技能正文或文件，不收编。
    Read(ReadArgs),
    /// 列出某档案的技能（组合筛选）。
    List(ListArgs),
    /// 为某具名档案明确启用技能（可多个）。
    Enable(NamesArgs),
    /// 为某具名档案禁用技能（可多个）。
    Disable(NamesArgs),
    /// 为某具名档案按龙虾预设一键启用（未安装到 default 则静默补装）。
    EnablePreset(PresetArgs),
    /// 仅在当前具名档案禁用龙虾预设成员。
    DisablePreset(PresetArgs),
    /// 档案人格源（SOUL.md）：读取、保存并覆盖本机全部已发现档案。
    Identity(IdentityCommand),
}

#[derive(Args, Debug)]
pub struct IdentityCommand {
    #[command(subcommand)]
    pub action: IdentityAction,
}

#[derive(Subcommand, Debug)]
pub enum IdentityAction {
    /// 读取某作用域的源内容与合成预览（global 或档案 id）。
    Get(IdentityScopeArgs),
    /// 保存某作用域的源内容并覆盖本机全部已发现档案的 SOUL.md。
    Set(IdentitySetArgs),
    /// 按当前源内容刷新本机全部已发现档案（不改源）。
    Distribute,
}

#[derive(Args, Debug)]
pub struct IdentityScopeArgs {
    /// 作用域：`global` 或档案 id。
    #[arg(value_name = "global|profile")]
    pub scope: String,
}

#[derive(Args, Debug)]
pub struct IdentitySetArgs {
    /// 作用域：`global` 或档案 id。
    #[arg(value_name = "global|profile")]
    pub scope: String,
    /// 从文件读取内容。
    #[arg(long, value_name = "path", conflicts_with = "stdin")]
    pub file: Option<PathBuf>,
    /// 从标准输入读取内容。
    #[arg(long, conflicts_with = "file")]
    pub stdin: bool,
}

#[derive(Args, Debug)]
pub struct ReadArgs {
    pub profile: String,
    pub name: String,
    #[arg(long)]
    pub file: Option<String>,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(value_name = "profile")]
    pub profile: String,
    /// 文本搜索（名称、描述、注释、分类、标签）。
    #[arg(long, value_name = "text")]
    pub query: Option<String>,
    /// 一级分类。
    #[arg(long, value_name = "category", conflicts_with = "uncategorized")]
    pub category: Option<String>,
    /// 只列出未分类技能。
    #[arg(long, conflicts_with = "category")]
    pub uncategorized: bool,
    /// 标签筛选（可重复；命中任一标签即可）。
    #[arg(long = "tag", value_name = "tag")]
    pub tags: Vec<String>,
    /// 只列出无标签技能。
    #[arg(long = "no-tag")]
    pub no_tag: bool,
}

#[derive(Args, Debug)]
pub struct NamesArgs {
    #[arg(value_name = "profile")]
    pub profile: String,
    /// 技能名（可重复）。
    #[arg(value_name = "name", required = true)]
    pub names: Vec<String>,
}

#[derive(Args, Debug)]
pub struct PresetArgs {
    #[arg(value_name = "profile")]
    pub profile: String,
    /// 龙虾预设 id。
    #[arg(value_name = "preset-id")]
    pub preset_id: String,
}

type Result<T> = std::result::Result<T, CoreError>;

/// 分发 `hermes` 子命令。
pub fn run(
    command: HermesCommand,
    ctx: &mut AppContext,
    _options: &GlobalOptions,
) -> (CliResult, String) {
    match dispatch(command.action, ctx) {
        Ok(pair) => pair,
        Err(err) => (CliResult::failed_error(err), String::new()),
    }
}

fn dispatch(action: HermesAction, ctx: &AppContext) -> Result<(CliResult, String)> {
    match action {
        HermesAction::Profiles => profiles(ctx),
        HermesAction::Read(args) => {
            let view = with_service(ctx, |service| {
                service.read(&args.profile, &args.name, args.file.as_deref())
            })?;
            let human = view
                .text
                .clone()
                .unwrap_or_else(|| format!("该文件不是 UTF-8 文本（{} 字节）", view.size));
            Ok((CliResult::from_data(&view), human))
        }
        HermesAction::List(args) => list(ctx, args),
        HermesAction::Enable(args) => set_enabled(ctx, args, true),
        HermesAction::Disable(args) => set_enabled(ctx, args, false),
        HermesAction::EnablePreset(args) => set_preset_enabled(ctx, args, true),
        HermesAction::DisablePreset(args) => set_preset_enabled(ctx, args, false),
        HermesAction::Identity(command) => identity(ctx, command),
    }
}

/// 建立档案服务并执行；闭包内使用。
fn with_service<T>(
    ctx: &AppContext,
    run: impl FnOnce(&HermesService<'_>) -> Result<T>,
) -> Result<T> {
    let resolver = ctx.target_resolver()?;
    let service = HermesService::from_context(ctx, &resolver)?;
    run(&service)
}

// ---------------------------------------------------------------------------
// profiles
// ---------------------------------------------------------------------------

fn profiles(ctx: &AppContext) -> Result<(CliResult, String)> {
    let (root, views) = with_service(ctx, |service| {
        Ok((service.root().display().to_string(), service.profiles()?))
    })?;
    let data = json!({ "root": root, "profiles": views });
    let mut human = format!("本机 Hermes 档案 {} 个（根：{root}）", views.len());
    for view in &views {
        human.push_str(&format!(
            "\n- {}（{}，{} 个已启用）{}",
            view.name, view.kind, view.enabled_count, view.path
        ));
    }
    Ok((CliResult::from_data(&data), human))
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

fn list(ctx: &AppContext, args: ListArgs) -> Result<(CliResult, String)> {
    let query = super::skill::search_query(
        args.query.clone(),
        args.category.clone(),
        args.uncategorized,
        args.tags.clone(),
        args.no_tag,
    );
    let view = with_service(ctx, |service| service.list(&args.profile, &query))?;
    let data = serde_json::to_value(&view)
        .map_err(|err| CoreError::other(format!("序列化档案技能列表失败：{err}")))?;

    let enabled = view.skills.iter().filter(|skill| skill.enabled).count();
    let mut human = format!(
        "档案 {}（{}）：{} 项，其中已启用 {enabled}{}",
        view.profile,
        view.kind,
        view.skills.len(),
        if view.default_skills_dir.is_empty() {
            String::new()
        } else {
            format!(" · default 共享副本 {}", view.default_skills_dir)
        }
    );
    if let Some(error) = &view.config_error {
        human.push_str(&format!(
            "\n[档案配置读取失败，启用状态不作有效结论：{error}]"
        ));
    }
    for skill in &view.skills {
        human.push_str(&format!(
            "\n- {} [{}]{}",
            skill.name,
            if skill.enabled {
                "已启用"
            } else {
                "未启用"
            },
            if skill.needs_adoption {
                "（档案独有，可收编）"
            } else {
                ""
            }
        ));
        if let Some(warning) = &skill.warning {
            human.push_str(&format!(" {warning}"));
        }
        if let Some(blocked) = &skill.blocked {
            human.push_str(&format!(" {blocked}"));
        }
    }
    Ok((CliResult::from_data(&data), human))
}

// ---------------------------------------------------------------------------
// enable / disable
// ---------------------------------------------------------------------------

fn collect_items_outcome(
    profile: &str,
    items: &[skill_manager_core::hermes::HermesItem],
    data: Value,
) -> CliResult {
    let mut batch = BatchOutcome::default();
    for item in items {
        if item.ok {
            batch.push_ok(serde_json::to_value(item).unwrap_or(Value::Null));
        } else {
            batch.push_err(
                skill_manager_core::ItemError::new(
                    item.message
                        .clone()
                        .unwrap_or_else(|| "未知原因".to_string()),
                )
                .skill(format!("{profile}/{}", item.name)),
            );
        }
    }
    let ok = items.iter().filter(|item| item.ok).count();
    if batch.failed.is_empty() {
        CliResult::success(data)
    } else if ok == 0 {
        CliResult::failed(batch.failed)
    } else {
        CliResult::partial(data, batch.failed)
    }
}

fn set_enabled(ctx: &AppContext, args: NamesArgs, enabled: bool) -> Result<(CliResult, String)> {
    let items = with_service(ctx, |service| {
        service.set_enabled(&args.profile, &args.names, enabled)
    })?;

    let ok = items.iter().filter(|item| item.ok).count();
    let data = json!({
        "profile": args.profile,
        "enabled": enabled,
        "items": items,
        "changed": ok,
    });
    let result = collect_items_outcome(&args.profile, &items, data);

    let mut human = format!(
        "已{}档案 {} 的技能 {} 项",
        if enabled { "启用" } else { "禁用" },
        args.profile,
        ok
    );
    if !result.errors.is_empty() {
        human.push_str(&format!("；{} 项未完成", result.errors.len()));
    }
    for item in &items {
        if let Some(message) = &item.message {
            human.push_str(&format!("\n- {}：{message}", item.name));
        }
    }
    Ok((result, human))
}

fn set_preset_enabled(
    ctx: &AppContext,
    args: PresetArgs,
    enabled: bool,
) -> Result<(CliResult, String)> {
    let outcome = with_service(ctx, |service| {
        service.set_preset_enabled(&args.profile, &args.preset_id, enabled)
    })?;
    let data = serde_json::to_value(&outcome)
        .map_err(|err| CoreError::other(format!("序列化预设结果失败：{err}")))?;
    let result = collect_items_outcome(&outcome.profile, &outcome.items, data);
    let mut human = if enabled {
        if outcome.installed > 0 {
            format!(
                "已为档案 {} 启用预设「{}」（自动安装 {} 项到 default，启用 {} 项）",
                outcome.profile, outcome.preset_name, outcome.installed, outcome.changed
            )
        } else {
            format!(
                "已为档案 {} 启用预设「{}」（{} 项）",
                outcome.profile, outcome.preset_name, outcome.changed
            )
        }
    } else {
        format!(
            "已在档案 {} 禁用预设「{}」（{} 项，default 与其他档案不受影响）",
            outcome.profile, outcome.preset_name, outcome.changed
        )
    };
    if !result.errors.is_empty() {
        human.push_str(&format!("；{} 项未完成", result.errors.len()));
    }
    for item in &outcome.items {
        if let Some(message) = &item.message {
            human.push_str(&format!("\n- {}：{message}", item.name));
        }
    }
    Ok((result, human))
}

// ---------------------------------------------------------------------------
// skill adopt-hermes 共用入口
// ---------------------------------------------------------------------------

/// `skill adopt-hermes <profile> <name>...`：收编档案独有技能到龙虾库并部署 default。
pub fn adopt(ctx: &AppContext, profile: &str, names: &[String]) -> Result<(CliResult, String)> {
    let items = with_service(ctx, |service| service.adopt(profile, names))?;

    let mut batch = BatchOutcome::default();
    for item in &items {
        if item.ok {
            batch.push_ok(serde_json::to_value(item).unwrap_or(Value::Null));
        } else {
            batch.push_err(
                skill_manager_core::ItemError::new(
                    item.message
                        .clone()
                        .unwrap_or_else(|| "未知原因".to_string()),
                )
                .skill(format!("{profile}/{}", item.name)),
            );
        }
    }
    let ok = items.iter().filter(|item| item.ok).count();
    let data = json!({ "profile": profile, "items": items, "adopted": ok });
    let result = if batch.failed.is_empty() {
        CliResult::success(data)
    } else if ok == 0 {
        CliResult::failed(batch.failed)
    } else {
        CliResult::partial(data, batch.failed)
    };

    let mut human = format!("已从档案 {profile} 收编 {} 项到龙虾库并部署 default", ok);
    if !result.errors.is_empty() {
        human.push_str(&format!("；{} 项未完成", result.errors.len()));
    }
    for item in &items {
        if item.ok {
            human.push_str(&format!(
                "\n- {} → {}（default）",
                item.name,
                item.library_skill.as_deref().unwrap_or("-")
            ));
        }
    }
    Ok((result, human))
}

// ---------------------------------------------------------------------------
// identity
// ---------------------------------------------------------------------------

fn identity_service(ctx: &AppContext) -> Result<IdentityService> {
    let root = ctx.vault()?.root().to_path_buf();
    Ok(IdentityService::new(root, ctx.home()?))
}

fn identity(ctx: &AppContext, command: IdentityCommand) -> Result<(CliResult, String)> {
    match command.action {
        IdentityAction::Get(args) => identity_get(ctx, &args.scope),
        IdentityAction::Set(args) => identity_set(ctx, args),
        IdentityAction::Distribute => identity_distribute(ctx),
    }
}

fn identity_get(ctx: &AppContext, scope: &str) -> Result<(CliResult, String)> {
    let scope = IdentityScope::parse(scope)?;
    let view = identity_service(ctx)?.get(&scope)?;
    let data = json!({
        "scope": view.scope,
        "source": view.source,
        "composed": view.composed,
    });
    let human = format!(
        "作用域 {}（源 {} 字符；合成后 {} 字符）",
        view.scope,
        view.source.chars().count(),
        view.composed.chars().count()
    );
    Ok((CliResult::from_data(&data), human))
}

fn identity_content(args: &IdentitySetArgs) -> Result<String> {
    match (&args.file, args.stdin) {
        (Some(path), false) => {
            std::fs::read_to_string(path).map_err(|err| CoreError::io(path.clone(), err))
        }
        (None, true) => {
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|err| CoreError::io(PathBuf::from("<stdin>"), err))?;
            Ok(buffer)
        }
        _ => Err(CoreError::other(
            "必须且只能指定 --file <path> 或 --stdin 之一",
        )),
    }
}

fn identity_set(ctx: &AppContext, args: IdentitySetArgs) -> Result<(CliResult, String)> {
    let scope = IdentityScope::parse(&args.scope)?;
    let content = identity_content(&args)?;
    let results = identity_service(ctx)?.save(&scope, &content)?;
    identity_results(scope.as_str(), results, true)
}

fn identity_distribute(ctx: &AppContext) -> Result<(CliResult, String)> {
    let results = identity_service(ctx)?.distribute()?;
    identity_results("current", results, false)
}

fn identity_results(
    scope: &str,
    results: Vec<skill_manager_core::hermes::IdentityTargetResult>,
    saved: bool,
) -> Result<(CliResult, String)> {
    let failures: Vec<skill_manager_core::ItemError> = results
        .iter()
        .filter(|result| !result.ok)
        .map(|result| {
            skill_manager_core::ItemError::new(
                result
                    .error
                    .clone()
                    .unwrap_or_else(|| "写入失败".to_string()),
            )
            .target(result.profile.clone())
            .path(result.path.clone())
        })
        .collect();
    let long = results
        .iter()
        .filter(|result| result.ok && result.chars >= LONG_SOUL_CHARS)
        .count();
    let data = json!({
        "scope": scope,
        "targets": results,
    });
    let mut human = if saved {
        format!(
            "已保存 {} 人格源；覆盖 {} 个本机已发现档案",
            scope,
            results.len()
        )
    } else {
        format!("已按当前人格源覆盖 {} 个本机已发现档案", results.len())
    };
    if !failures.is_empty() {
        human.push_str(&format!("，{} 个档案写入失败", failures.len()));
    }
    if long > 0 {
        human.push_str(&format!(
            "；{long} 个合成稿达到约 {LONG_SOUL_CHARS} 字符，Hermes 可能截断中段"
        ));
    }
    let result = if failures.is_empty() {
        CliResult::success(data)
    } else {
        CliResult::partial(data, failures)
    };
    Ok((result, human))
}
