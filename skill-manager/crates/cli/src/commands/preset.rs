//! `preset` 子命令：预设定义管理（T063）。
//!
//! 参数与输出遵循 `specs/002-skill-management/contracts/cli.md`；业务规则全部在 core。
//! 完整引用为 `(domain, id)`，两域同 id 时不静默选域。定义编辑不触发部署。

use clap::{Args, Subcommand};
use serde_json::json;
use skill_manager_core::domain::Domain;
use skill_manager_core::preset;
use skill_manager_core::{AppContext, CliResult, CoreError, GlobalOptions, PresetView};

#[derive(Args, Debug)]
pub struct PresetCommand {
    #[command(subcommand)]
    pub action: PresetAction,
}

#[derive(Subcommand, Debug)]
pub enum PresetAction {
    /// 列出预设（不给 --domain 时两域分别列出）。
    List(ListArgs),
    /// 读取一个预设（完整引用 domain / id）。
    Get(GetArgs),
    /// 新建预设；id 由名称派生并在域内去重。
    Create(CreateArgs),
    /// 删除预设（只删组合定义，不影响实际部署）。
    Delete(GetArgs),
    /// 添加或移除成员（二者互斥）。
    Members(MembersArgs),
    /// 重命名预设。
    Rename(RenameArgs),
    /// 设置或清除预设图标。
    Icon(IconArgs),
    /// 调整预设间显示顺序。
    Reorder(ReorderArgs),
    /// 调整某预设内成员的显示顺序。
    ReorderMembers(ReorderMembersArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// 只列出该管理域；不给则两域分别列出。
    #[arg(long, value_name = "domain")]
    pub domain: Option<String>,
}

#[derive(Args, Debug)]
pub struct GetArgs {
    /// 管理域：code 或 lobster。
    #[arg(value_name = "domain")]
    pub domain: String,
    /// 预设 id。
    #[arg(value_name = "id")]
    pub id: String,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(value_name = "domain")]
    pub domain: String,
    /// 预设名称（中文可读名）。
    #[arg(value_name = "name")]
    pub name: String,
    /// 图标标识（见 `docs/technical-reference/preset-icons.json`）。
    #[arg(long, value_name = "icon")]
    pub icon: Option<String>,
}

#[derive(Args, Debug)]
pub struct MembersArgs {
    #[arg(value_name = "domain")]
    pub domain: String,
    #[arg(value_name = "id")]
    pub id: String,
    /// 添加成员（逗号分隔的技能名；须同域已存在）。
    #[arg(long, value_name = "csv", conflicts_with = "remove")]
    pub add: Option<String>,
    /// 移除成员（逗号分隔的技能名）。
    #[arg(long, value_name = "csv", conflicts_with = "add")]
    pub remove: Option<String>,
}

#[derive(Args, Debug)]
pub struct RenameArgs {
    #[arg(value_name = "domain")]
    pub domain: String,
    #[arg(value_name = "id")]
    pub id: String,
    /// 新名称。
    #[arg(value_name = "new")]
    pub name: String,
}

#[derive(Args, Debug)]
pub struct IconArgs {
    #[arg(value_name = "domain")]
    pub domain: String,
    #[arg(value_name = "id")]
    pub id: String,
    /// 图标标识；传空字符串清除图标。
    #[arg(value_name = "icon")]
    pub icon: String,
}

#[derive(Args, Debug)]
pub struct ReorderArgs {
    #[arg(value_name = "domain")]
    pub domain: String,
    /// 该域全部预设 id 的顺序（逗号分隔，须为当前集合的一个排列）。
    #[arg(long = "order", value_name = "id,...")]
    pub order: String,
}

#[derive(Args, Debug)]
pub struct ReorderMembersArgs {
    #[arg(value_name = "domain")]
    pub domain: String,
    #[arg(value_name = "id")]
    pub id: String,
    /// 该预设全部成员技能名的顺序（逗号分隔，须为当前成员的一个排列）。
    #[arg(long = "order", value_name = "name,...")]
    pub order: String,
}

type Result<T> = std::result::Result<T, CoreError>;

/// 分发 `preset` 子命令。
pub fn run(
    command: PresetCommand,
    ctx: &mut AppContext,
    _options: &GlobalOptions,
) -> (CliResult, String) {
    match dispatch(command.action, ctx) {
        Ok(pair) => pair,
        Err(err) => (CliResult::failed_error(err), String::new()),
    }
}

fn dispatch(action: PresetAction, ctx: &AppContext) -> Result<(CliResult, String)> {
    match action {
        PresetAction::List(args) => list(ctx, args),
        PresetAction::Get(args) => get(ctx, args),
        PresetAction::Create(args) => create(ctx, args),
        PresetAction::Delete(args) => delete(ctx, args),
        PresetAction::Members(args) => members(ctx, args),
        PresetAction::Rename(args) => rename(ctx, args),
        PresetAction::Icon(args) => icon(ctx, args),
        PresetAction::Reorder(args) => reorder(ctx, args),
        PresetAction::ReorderMembers(args) => reorder_members(ctx, args),
    }
}

fn root(ctx: &AppContext) -> Result<std::path::PathBuf> {
    Ok(ctx.vault()?.root().to_path_buf())
}

fn parse_domain(raw: &str) -> Result<Domain> {
    raw.parse::<Domain>()
}

/// 逗号分隔列表（同时接受中文逗号）。
fn split_csv(raw: &str) -> Vec<String> {
    raw.split([',', '，'])
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn views_data(views: &[PresetView]) -> serde_json::Value {
    json!({ "presets": views, "total": views.len() })
}

fn preset_human(view: &PresetView) -> String {
    let mut out = format!("{}/{}（{}）", view.domain, view.id, view.name);
    if let Some(icon) = &view.icon {
        out.push_str(&format!(" 图标：{icon}"));
    }
    out.push_str(&format!(" 顺序：{}", view.order));
    if view.members.is_empty() {
        out.push_str(" 成员：无");
    } else {
        let names: Vec<String> = view
            .members
            .iter()
            .map(|member| {
                if member.missing {
                    format!("{}（缺失成员）", member.name)
                } else {
                    member.name.clone()
                }
            })
            .collect();
        out.push_str(&format!(" 成员：{}", names.join("、")));
    }
    out
}

// ---------------------------------------------------------------------------
// 各动作
// ---------------------------------------------------------------------------

fn list(ctx: &AppContext, args: ListArgs) -> Result<(CliResult, String)> {
    let domain = match &args.domain {
        Some(raw) => Some(parse_domain(raw)?),
        None => None,
    };
    let views = preset::list(&root(ctx)?, domain)?;
    let mut human = format!("预设 {} 个", views.len());
    for view in &views {
        human.push_str("\n- ");
        human.push_str(&preset_human(view));
    }
    if views.iter().any(|view| !view.missing_members.is_empty()) {
        human.push_str("\n（标注“缺失成员”的技能已不在库内，不影响其他成员）");
    }
    Ok((CliResult::from_data(&views_data(&views)), human))
}

fn get(ctx: &AppContext, args: GetArgs) -> Result<(CliResult, String)> {
    let domain = parse_domain(&args.domain)?;
    let view = preset::get(&root(ctx)?, domain, &args.id)?;
    Ok((CliResult::from_data(&view), preset_human(&view)))
}

fn create(ctx: &AppContext, args: CreateArgs) -> Result<(CliResult, String)> {
    let domain = parse_domain(&args.domain)?;
    let view = preset::create(&root(ctx)?, domain, &args.name, args.icon.as_deref())?;
    let human = format!(
        "已新建预设 {}（不部署任何技能；id 由名称派生）",
        preset_human(&view)
    );
    Ok((CliResult::from_data(&view), human))
}

fn delete(ctx: &AppContext, args: GetArgs) -> Result<(CliResult, String)> {
    let domain = parse_domain(&args.domain)?;
    let view = preset::delete(&root(ctx)?, domain, &args.id)?;
    let human = format!("已删除预设 {}（实际部署未改变）", preset_human(&view));
    Ok((CliResult::from_data(&view), human))
}

fn members(ctx: &AppContext, args: MembersArgs) -> Result<(CliResult, String)> {
    let domain = parse_domain(&args.domain)?;
    let root = root(ctx)?;
    let (view, action) = match (&args.add, &args.remove) {
        (Some(csv), None) => {
            let names = split_csv(csv);
            (preset::add_members(&root, domain, &args.id, &names)?, "添加")
        }
        (None, Some(csv)) => {
            let names = split_csv(csv);
            (preset::remove_members(&root, domain, &args.id, &names)?, "移除")
        }
        _ => {
            return Err(CoreError::other(
                "preset members 须且只能指定 --add 或 --remove 之一",
            ));
        }
    };
    let human = format!("已{action}成员后的预设 {}", preset_human(&view));
    Ok((CliResult::from_data(&view), human))
}

fn rename(ctx: &AppContext, args: RenameArgs) -> Result<(CliResult, String)> {
    let domain = parse_domain(&args.domain)?;
    let view = preset::rename(&root(ctx)?, domain, &args.id, &args.name)?;
    let human = format!("已重命名预设（成员与部署未改变）：{}", preset_human(&view));
    Ok((CliResult::from_data(&view), human))
}

fn icon(ctx: &AppContext, args: IconArgs) -> Result<(CliResult, String)> {
    let domain = parse_domain(&args.domain)?;
    let icon = args.icon.trim();
    let icon = if icon.is_empty() { None } else { Some(icon) };
    let view = preset::set_icon(&root(ctx)?, domain, &args.id, icon)?;
    let human = match &view.icon {
        Some(icon) => format!("已设置预设图标为 {icon}：{}", preset_human(&view)),
        None => format!("已清除预设图标：{}", preset_human(&view)),
    };
    Ok((CliResult::from_data(&view), human))
}

fn reorder(ctx: &AppContext, args: ReorderArgs) -> Result<(CliResult, String)> {
    let domain = parse_domain(&args.domain)?;
    let order = split_csv(&args.order);
    let views = preset::reorder(&root(ctx)?, domain, &order)?;
    let human = format!(
        "已调整 {} 个预设的显示顺序（不改变成员集合、分类目录或部署）",
        views.len()
    );
    Ok((CliResult::from_data(&views_data(&views)), human))
}

fn reorder_members(ctx: &AppContext, args: ReorderMembersArgs) -> Result<(CliResult, String)> {
    let domain = parse_domain(&args.domain)?;
    let order = split_csv(&args.order);
    let view = preset::reorder_members(&root(ctx)?, domain, &args.id, &order)?;
    let human = format!(
        "已调整成员显示顺序（不增减成员、不动目录、不改变部署）：{}",
        preset_human(&view)
    );
    Ok((CliResult::from_data(&view), human))
}
