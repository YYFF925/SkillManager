//! 部署命令：薄封装 core 的部署、撤销、同步与状态查询。
//!
//! 字段与 CLI 的同名操作保持一致；前端只调用这里，不重复业务判断。

use serde::Serialize;
use skill_manager_core::deploy::{
    parse_target_arg, DeployItemView, DeployReport, DeploymentService, DeploymentStatusView,
};
use skill_manager_core::domain::{SkillRef, Target};
use skill_manager_core::output::ItemError;
use skill_manager_core::AppContext;

use super::AppState;

/// 部署/撤销/同步结果。
#[derive(Clone, Debug, Serialize)]
pub struct DeployReportView {
    pub items: Vec<DeployItemView>,
    pub errors: Vec<ItemError>,
}

/// 删库清理结果。
#[derive(Clone, Debug, Serialize)]
pub struct SkillDeleteView {
    pub deleted: Vec<String>,
    pub cleanup: Vec<DeployItemView>,
    pub errors: Vec<ItemError>,
}

/// 把本次未完成项写入本机运行日志（供诊断导出定位部署异常）。
pub fn log_failures(ctx: &AppContext, action: &str, report: &DeployReport) {
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

fn parse_refs(raw: &[String]) -> Result<Vec<SkillRef>, String> {
    raw.iter()
        .map(|value| SkillRef::parse(value).map_err(|err| err.to_string()))
        .collect()
}

fn parse_targets(primary: &str, also: Option<Vec<String>>) -> Result<Vec<Target>, String> {
    let mut targets = vec![parse_target_arg(primary).map_err(|err| err.to_string())?];
    for raw in also.unwrap_or_default() {
        targets.push(parse_target_arg(&raw).map_err(|err| err.to_string())?);
    }
    Ok(targets)
}

/// 部署技能到目标（可多技能 × 多目标）。
#[tauri::command]
pub fn deploy_to(
    state: tauri::State<'_, AppState>,
    target: String,
    references: Vec<String>,
    also_targets: Option<Vec<String>>,
) -> Result<DeployReportView, String> {
    let ctx = state.get()?;
    let root = ctx.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = DeploymentService::new(root, &resolver, ctx.state());

    let references = parse_refs(&references)?;
    let targets = parse_targets(&target, also_targets)?;
    let report = service
        .deploy_to(&references, &targets)
        .map_err(|err| err.to_string())?;
    log_failures(&ctx, "deploy", &report);
    Ok(DeployReportView {
        items: report.items,
        errors: report.failures,
    })
}

/// 撤销部署（保留库内技能）。
#[tauri::command]
pub fn deploy_undo(
    state: tauri::State<'_, AppState>,
    target: String,
    references: Vec<String>,
    also_targets: Option<Vec<String>>,
    discard_local_changes: Option<bool>,
) -> Result<DeployReportView, String> {
    let ctx = state.get()?;
    let root = ctx.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = DeploymentService::new(root, &resolver, ctx.state());

    let references = parse_refs(&references)?;
    let targets = parse_targets(&target, also_targets)?;
    let report = service
        .undo(
            &references,
            &targets,
            discard_local_changes.unwrap_or(false),
        )
        .map_err(|err| err.to_string())?;
    log_failures(&ctx, "undo", &report);
    Ok(DeployReportView {
        items: report.items,
        errors: report.failures,
    })
}

/// 同步已有部署（不新增目标）。
#[tauri::command]
pub fn deploy_sync(
    state: tauri::State<'_, AppState>,
    reference: Option<String>,
    target: Option<String>,
    overwrite_local: Option<bool>,
) -> Result<DeployReportView, String> {
    let ctx = state.get()?;
    let root = ctx.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = DeploymentService::new(root, &resolver, ctx.state());

    let reference = match reference {
        Some(raw) => Some(SkillRef::parse(&raw).map_err(|err| err.to_string())?),
        None => None,
    };
    let target = match target {
        Some(raw) => Some(parse_target_arg(&raw).map_err(|err| err.to_string())?),
        None => None,
    };
    let report = service
        .sync(reference.as_ref(), target.as_ref(), overwrite_local.unwrap_or(false))
        .map_err(|err| err.to_string())?;
    log_failures(&ctx, "deploy-sync", &report);
    Ok(DeployReportView {
        items: report.items,
        errors: report.failures,
    })
}

/// 本机部署记录；`paused` 只列出被外部修改而暂停的项。
#[tauri::command]
pub fn deploy_status(
    state: tauri::State<'_, AppState>,
    target: Option<String>,
    paused: Option<bool>,
) -> Result<Vec<DeploymentStatusView>, String> {
    let ctx = state.get()?;
    let root = ctx.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = DeploymentService::new(root, &resolver, ctx.state());

    let target = match target {
        Some(raw) => Some(parse_target_arg(&raw).map_err(|err| err.to_string())?),
        None => None,
    };
    service
        .status(target.as_ref(), paused.unwrap_or(false))
        .map_err(|err| err.to_string())
}

/// 删除库内技能并清理其受管部署（本地外改副本暂停并保留定位记录）。
#[tauri::command]
pub fn skill_delete(
    state: tauri::State<'_, AppState>,
    references: Vec<String>,
) -> Result<SkillDeleteView, String> {
    let ctx = state.get()?;
    let root = ctx.vault().map_err(|err| err.to_string())?.root().to_path_buf();
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = DeploymentService::new(root, &resolver, ctx.state());

    let references = parse_refs(&references)?;
    let outcome = service
        .delete_skills(&references)
        .map_err(|err| err.to_string())?;
    let mut errors = outcome.paused;
    errors.extend(outcome.failures);
    Ok(SkillDeleteView {
        deleted: outcome.deleted,
        cleanup: outcome.removed,
        errors,
    })
}
