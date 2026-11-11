//! 预设命令（T065）：定义 CRUD、两种显示排序、成员增删与预设叠加部署/撤销。
//!
//! 薄封装 core；组合定义不触发部署，前端不重复业务判断。

use skill_manager_core::deploy::{parse_target_arg, DeploymentService};
use skill_manager_core::domain::{Domain, Target};
use skill_manager_core::preset;
use skill_manager_core::PresetView;

use super::deploy::DeployReportView;
use crate::HostRuntime;

fn parse_domain(raw: &str) -> Result<Domain, String> {
    raw.parse::<Domain>().map_err(|err| err.to_string())
}

fn parse_targets(primary: &str, also: Option<Vec<String>>) -> Result<Vec<Target>, String> {
    let mut targets = vec![parse_target_arg(primary).map_err(|err| err.to_string())?];
    for raw in also.unwrap_or_default() {
        targets.push(parse_target_arg(&raw).map_err(|err| err.to_string())?);
    }
    Ok(targets)
}

/// 列出预设；不给域时两域分别列出。
pub fn preset_list(
    runtime: &HostRuntime,
    domain: Option<String>,
) -> Result<Vec<PresetView>, String> {
    let ctx = runtime.get()?;
    let root = ctx.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    let domain = match domain {
        Some(raw) => Some(parse_domain(&raw)?),
        None => None,
    };
    preset::list(&root, domain).map_err(|err| err.to_string())
}

/// 读取一个预设（完整引用 domain / id）。
pub fn preset_get(
    runtime: &HostRuntime,
    domain: String,
    id: String,
) -> Result<PresetView, String> {
    let root = runtime.get()?.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    preset::get(&root, parse_domain(&domain)?, &id).map_err(|err| err.to_string())
}

/// 新建预设；id 由名称派生并在域内去重。
pub fn preset_create(
    runtime: &HostRuntime,
    domain: String,
    name: String,
    icon: Option<String>,
) -> Result<PresetView, String> {
    let root = runtime.get()?.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    preset::create(&root, parse_domain(&domain)?, &name, icon.as_deref())
        .map_err(|err| err.to_string())
}

/// 删除预设（只删组合定义，不影响实际部署）。
pub fn preset_delete(
    runtime: &HostRuntime,
    domain: String,
    id: String,
) -> Result<PresetView, String> {
    let root = runtime.get()?.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    preset::delete(&root, parse_domain(&domain)?, &id).map_err(|err| err.to_string())
}

/// 重命名预设。
pub fn preset_rename(
    runtime: &HostRuntime,
    domain: String,
    id: String,
    name: String,
) -> Result<PresetView, String> {
    let root = runtime.get()?.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    preset::rename(&root, parse_domain(&domain)?, &id, &name).map_err(|err| err.to_string())
}

/// 设置或清除预设图标（空串表示清除）。
pub fn preset_set_icon(
    runtime: &HostRuntime,
    domain: String,
    id: String,
    icon: Option<String>,
) -> Result<PresetView, String> {
    let root = runtime.get()?.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    let icon = icon
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    preset::set_icon(&root, parse_domain(&domain)?, &id, icon).map_err(|err| err.to_string())
}

/// 调整预设间显示顺序；`order` 必须是该域全部预设 id 的一个排列。
pub fn preset_reorder(
    runtime: &HostRuntime,
    domain: String,
    order: Vec<String>,
) -> Result<Vec<PresetView>, String> {
    let root = runtime.get()?.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    preset::reorder(&root, parse_domain(&domain)?, &order).map_err(|err| err.to_string())
}

/// 添加成员（限定同域已存在技能）。
pub fn preset_members_add(
    runtime: &HostRuntime,
    domain: String,
    id: String,
    names: Vec<String>,
) -> Result<PresetView, String> {
    let root = runtime.get()?.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    preset::add_members(&root, parse_domain(&domain)?, &id, &names).map_err(|err| err.to_string())
}

/// 移除成员。
pub fn preset_members_remove(
    runtime: &HostRuntime,
    domain: String,
    id: String,
    names: Vec<String>,
) -> Result<PresetView, String> {
    let root = runtime.get()?.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    preset::remove_members(&root, parse_domain(&domain)?, &id, &names)
        .map_err(|err| err.to_string())
}

/// 调整成员显示顺序；`order` 必须是当前成员的一个排列。
pub fn preset_reorder_members(
    runtime: &HostRuntime,
    domain: String,
    id: String,
    order: Vec<String>,
) -> Result<PresetView, String> {
    let root = runtime.get()?.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    preset::reorder_members(&root, parse_domain(&domain)?, &id, &order)
        .map_err(|err| err.to_string())
}

// ---------------------------------------------------------------------------
// 预设叠加部署与撤销
// ---------------------------------------------------------------------------

/// 按预设成员叠加部署到目标（可多目标）。
pub fn deploy_preset(
    runtime: &HostRuntime,
    domain: String,
    preset_id: String,
    target: String,
    also_targets: Option<Vec<String>>,
) -> Result<DeployReportView, String> {
    let ctx = runtime.get()?;
    let root = ctx.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = DeploymentService::new(root, &resolver, ctx.state());

    let targets = parse_targets(&target, also_targets)?;
    let report = service
        .deploy_preset(parse_domain(&domain)?, &preset_id, &targets)
        .map_err(|err| err.to_string())?;
    super::deploy::log_failures(&ctx, "deploy-preset", &report);
    Ok(DeployReportView {
        items: report.items,
        errors: report.failures,
    })
}

/// 按预设当前成员逐个撤销（保留库内技能）。
pub fn deploy_undo_preset(
    runtime: &HostRuntime,
    domain: String,
    preset_id: String,
    target: String,
    also_targets: Option<Vec<String>>,
) -> Result<DeployReportView, String> {
    let ctx = runtime.get()?;
    let root = ctx.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = DeploymentService::new(root, &resolver, ctx.state());

    let targets = parse_targets(&target, also_targets)?;
    let report = service
        .undo_preset(parse_domain(&domain)?, &preset_id, &targets)
        .map_err(|err| err.to_string())?;
    super::deploy::log_failures(&ctx, "undo-preset", &report);
    Ok(DeployReportView {
        items: report.items,
        errors: report.failures,
    })
}
