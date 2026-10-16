//! 上游维护命令（T047）：check/update/update-repo/reject-new/rejected/skip/unskip/skipped。
//!
//! 薄封装 core；检查只展示，应用通过独立显式调用表达用户裁决；自维护 Gitea 来源也无自动采用例外。
//! GUI（T048）由后续任务接入页面。

use serde_json::json;
use skill_manager_core::domain::{Domain, SkillRef};
use skill_manager_core::manager;
use skill_manager_core::upstream::check::CheckScope;
use skill_manager_core::upstream::{decisions, UpstreamService};
use skill_manager_core::vault::acquire::CategoryChoice;

use super::acquire::progress_emitter;
use super::AppState;

/// 检查上游变化（只展示，不应用）。
#[tauri::command]
pub async fn skill_check(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    reference: Option<String>,
    repo: Option<String>,
    all: Option<bool>,
    job_id: String,
) -> Result<skill_manager_core::upstream::CheckReport, String> {
    let scope = match (&reference, &repo, all.unwrap_or(false)) {
        (Some(raw), None, false) => CheckScope::Single(SkillRef::parse(raw).map_err(|e| e.to_string())?),
        (None, Some(repo), false) => CheckScope::Repo(repo.clone()),
        (None, None, true) => CheckScope::All,
        _ => return Err("检查范围需为 ref、repo 或 all 之一".to_string()),
    };
    let cancel = state.register_fetch(&job_id);
    let ctx = state.get()?;
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = UpstreamService::from_context(&ctx, &resolver).map_err(|err| err.to_string())?;
    let mut on_progress = progress_emitter(&app, &job_id);
    let result = service
        .check(&scope, &mut on_progress, &cancel)
        .map_err(|err| err.to_string());
    state.finish_fetch(&job_id);
    result
}

/// 显式单项采用上游更新。
#[tauri::command]
pub async fn skill_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    reference: String,
    to: Option<String>,
    job_id: String,
) -> Result<skill_manager_core::upstream::UpdateItem, String> {
    let reference = SkillRef::parse(&reference).map_err(|err| err.to_string())?;
    let cancel = state.register_fetch(&job_id);
    let ctx = state.get()?;
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = UpstreamService::from_context(&ctx, &resolver).map_err(|err| err.to_string())?;
    let mut on_progress = progress_emitter(&app, &job_id);
    let result = service
        .update(&reference, to, &mut on_progress, &cancel)
        .map_err(|err| err.to_string());
    state.finish_fetch(&job_id);
    result
}

/// 仓库批量更新（新增仅处理显式选定）。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn skill_update_repo(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    repo: String,
    include_new: Option<Vec<String>>,
    domain: Option<String>,
    category: Option<String>,
    uncategorized: Option<bool>,
    job_id: String,
) -> Result<serde_json::Value, String> {
    let domain = match domain {
        Some(raw) => Some(raw.parse::<Domain>().map_err(|err| err.to_string())?),
        None => None,
    };
    let category = if uncategorized.unwrap_or(false) {
        CategoryChoice::Uncategorized
    } else {
        match category {
            Some(name) => CategoryChoice::Given(name),
            None => CategoryChoice::Default,
        }
    };
    let cancel = state.register_fetch(&job_id);
    let ctx = state.get()?;
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = UpstreamService::from_context(&ctx, &resolver).map_err(|err| err.to_string())?;
    let mut on_progress = progress_emitter(&app, &job_id);
    let outcome = service
        .update_repo(
            &repo,
            &include_new.unwrap_or_default(),
            domain,
            category,
            &mut on_progress,
            &cancel,
        )
        .map_err(|err| err.to_string());
    state.finish_fetch(&job_id);
    let outcome = outcome?;
    Ok(json!({
        "repo": outcome.repo,
        "items": outcome.items,
        "pending_deletions": outcome.pending_deletions,
        "imported": outcome.imported,
        "errors": outcome.errors,
    }))
}

/// 明确“不入库并记住”的新增候选。
#[tauri::command]
pub fn skill_reject_new(
    state: tauri::State<'_, AppState>,
    repo: String,
    domain: String,
    skills: Vec<String>,
) -> Result<serde_json::Value, String> {
    let domain = domain.parse::<Domain>().map_err(|err| err.to_string())?;
    let ctx = state.get()?;
    let root = ctx
        .vault()
        .map(|vault| vault.root().to_path_buf())
        .map_err(|err| err.to_string())?;
    decisions::reject(&root, &repo, domain, &skills).map_err(|err| err.to_string())?;
    Ok(json!({ "repo": repo, "domain": domain.to_string(), "rejected": skills }))
}

/// 查看已拒绝新增记录。
#[tauri::command]
pub fn skill_rejected(
    state: tauri::State<'_, AppState>,
    repo: Option<String>,
    domain: Option<String>,
) -> Result<Vec<decisions::DecisionEntry>, String> {
    let domain = match domain {
        Some(raw) => Some(raw.parse::<Domain>().map_err(|err| err.to_string())?),
        None => None,
    };
    let ctx = state.get()?;
    let root = ctx
        .vault()
        .map(|vault| vault.root().to_path_buf())
        .map_err(|err| err.to_string())?;
    decisions::entries(&root, repo.as_deref(), domain).map_err(|err| err.to_string())
}

/// 跳过技能当前上游版本。
#[tauri::command]
pub fn skill_skip(
    state: tauri::State<'_, AppState>,
    reference: String,
    version: Option<String>,
) -> Result<serde_json::Value, String> {
    let ctx = state.get()?;
    let root = ctx
        .vault()
        .map(|vault| vault.root().to_path_buf())
        .map_err(|err| err.to_string())?;
    let reference = SkillRef::parse(&reference).map_err(|err| err.to_string())?;
    let index = skill_manager_core::vault::SkillIndex::build(&root).map_err(|err| err.to_string())?;
    let entry = index
        .get(&reference)
        .ok_or_else(|| format!("技能不存在：{reference}"))?;
    let source = entry
        .source
        .clone()
        .ok_or_else(|| format!("技能没有上游来源记录：{reference}"))?;
    let version = match version {
        Some(version) => version,
        None => ctx
            .state()
            .latest_remote_version(&source.repo, reference.domain, &reference.name)
            .map_err(|err| err.to_string())?
            .ok_or_else(|| {
                format!("尚无检查结果：{reference}；请先执行检查（不在跳过时静默选择另一个版本）")
            })?,
    };
    let mut record = source;
    record.skipped_version = Some(version.clone());
    manager::write_source(&entry.dir, &record).map_err(|err| err.to_string())?;
    Ok(json!({ "skill": reference.to_string(), "skipped_version": version }))
}

/// 恢复技能的上游更新提示。
#[tauri::command]
pub fn skill_unskip(
    state: tauri::State<'_, AppState>,
    reference: String,
) -> Result<serde_json::Value, String> {
    let ctx = state.get()?;
    let root = ctx
        .vault()
        .map(|vault| vault.root().to_path_buf())
        .map_err(|err| err.to_string())?;
    let reference = SkillRef::parse(&reference).map_err(|err| err.to_string())?;
    let index = skill_manager_core::vault::SkillIndex::build(&root).map_err(|err| err.to_string())?;
    let entry = index
        .get(&reference)
        .ok_or_else(|| format!("技能不存在：{reference}"))?;
    let mut record = manager::read_source(&entry.dir)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("技能没有上游来源记录：{reference}"))?;
    record.skipped_version = None;
    manager::write_source(&entry.dir, &record).map_err(|err| err.to_string())?;
    Ok(json!({ "skill": reference.to_string(), "skipped_version": null }))
}

/// 查看已跳过版本。
#[tauri::command]
pub fn skill_skipped(state: tauri::State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    let ctx = state.get()?;
    let root = ctx
        .vault()
        .map(|vault| vault.root().to_path_buf())
        .map_err(|err| err.to_string())?;
    let index = skill_manager_core::vault::SkillIndex::build(&root).map_err(|err| err.to_string())?;
    let mut out = Vec::new();
    for entry in index.all() {
        let Some(source) = &entry.source else { continue };
        let Some(skipped) = &source.skipped_version else {
            continue;
        };
        out.push(json!({
            "skill": entry.reference().to_string(),
            "repo": source.repo,
            "subpath": source.subpath,
            "adopted_version": source.version,
            "skipped_version": skipped,
        }));
    }
    Ok(out)
}
