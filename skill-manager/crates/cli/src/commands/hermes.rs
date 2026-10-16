//! `hermes` 子命令：本机单 Hermes 根下的档案与技能启用（T060）。
//!
//! 参数与输出遵循 `specs/002-skill-management/contracts/cli.md`；业务规则全部在 core。
//! default 档案由 `hermes:default` 部署记录表达（`deploy` 维护）；其他档案在此明确启停。
//! 不提供档案创建/删除/改名、活动切换、多实例或 `SOUL.md` 管理。

use clap::{Args, Subcommand};
use serde_json::{Value, json};
use skill_manager_core::hermes::HermesService;
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
    /// 列出某档案的技能（组合筛选）。
    List(ListArgs),
    /// 为某具名档案明确启用技能（可多个）。
    Enable(NamesArgs),
    /// 为某具名档案禁用技能（可多个）。
    Disable(NamesArgs),
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
        HermesAction::List(args) => list(ctx, args),
        HermesAction::Enable(args) => set_enabled(ctx, args, true),
        HermesAction::Disable(args) => set_enabled(ctx, args, false),
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
        human.push_str(&format!("\n[档案配置读取失败，启用状态不作有效结论：{error}]"));
    }
    for skill in &view.skills {
        human.push_str(&format!(
            "\n- {} [{}]{}",
            skill.name,
            if skill.enabled { "已启用" } else { "未启用" },
            if skill.needs_adoption { "（档案独有，可收编）" } else { "" }
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

fn set_enabled(ctx: &AppContext, args: NamesArgs, enabled: bool) -> Result<(CliResult, String)> {
    let items = with_service(ctx, |service| service.set_enabled(&args.profile, &args.names, enabled))?;

    let mut batch = BatchOutcome::default();
    for item in &items {
        if item.ok {
            batch.push_ok(serde_json::to_value(item).unwrap_or(Value::Null));
        } else {
            batch.push_err(
                skill_manager_core::ItemError::new(
                    item.message.clone().unwrap_or_else(|| "未知原因".to_string()),
                )
                .skill(format!("{}/{}", args.profile, item.name)),
            );
        }
    }
    let ok = items.iter().filter(|item| item.ok).count();
    let data = json!({
        "profile": args.profile,
        "enabled": enabled,
        "items": items,
        "changed": ok,
    });
    let result = if batch.failed.is_empty() {
        CliResult::success(data)
    } else if ok == 0 {
        CliResult::failed(batch.failed)
    } else {
        CliResult::partial(data, batch.failed)
    };

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
                    item.message.clone().unwrap_or_else(|| "未知原因".to_string()),
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
            human.push_str(&format!("\n- {} → {}（default）", item.name, item.library_skill.as_deref().unwrap_or("-")));
        }
    }
    Ok((result, human))
}
