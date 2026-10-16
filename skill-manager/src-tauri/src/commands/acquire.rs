//! 获取入口命令（T037）：preview/import/adopt/create/copy 与真实进度/取消事件。
//!
//! 薄封装 core；所有选择由显式参数传入（不要求回 GUI 补裁决）。GUI（T038）由后续任务接入页面，
//! 这里只提供同一核心的命令与进度事件，结构化结果与进度事件可区分。

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use skill_manager_core::domain::{Domain, SkillRef};
use skill_manager_core::manager::SupportDecl;
use skill_manager_core::sync::GitProgress;
use skill_manager_core::upstream::UpstreamService;
use skill_manager_core::vault::acquire::CategoryChoice;
use skill_manager_core::vault::adopt::AdoptMeta;
use skill_manager_core::vault::{copy_skill, create_skill};
use tauri::Emitter;

use super::AppState;

/// 进度事件名。
pub const FETCH_PROGRESS_EVENT: &str = "skill-fetch-progress";

/// 一次获取的进度事件负载。
#[derive(Clone, Debug, Serialize)]
pub struct FetchProgress {
    pub job_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    pub raw: String,
}

/// 收编的逐技能资料输入（metadata 参数）。
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct MetaInput {
    pub comment: Option<String>,
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
    pub agents: Option<Vec<String>>,
    pub oses: Option<Vec<String>>,
    pub translation: Option<String>,
}

impl MetaInput {
    fn to_meta(&self) -> AdoptMeta {
        AdoptMeta {
            comment: self.comment.clone(),
            tags: self.tags.clone(),
            category: self.category.clone(),
            agents: self.agents.clone().map(support_decl),
            oses: self.oses.clone().map(support_decl),
            translation: self.translation.clone(),
        }
    }
}

fn support_decl(list: Vec<String>) -> SupportDecl {
    if list.is_empty() {
        SupportDecl::Empty
    } else {
        SupportDecl::List(list)
    }
}

fn category_choice(category: Option<String>, uncategorized: Option<bool>) -> CategoryChoice {
    if uncategorized.unwrap_or(false) {
        CategoryChoice::Uncategorized
    } else {
        match category {
            Some(name) => CategoryChoice::Given(name),
            None => CategoryChoice::Default,
        }
    }
}

/// 在获取期间把真实进度作为事件发出；调用方负责登记/清理取消标记。
pub(super) fn progress_emitter<'a>(
    app: &'a tauri::AppHandle,
    job_id: &'a str,
) -> impl FnMut(&GitProgress) + 'a {
    move |progress: &GitProgress| {
        let payload = FetchProgress {
            job_id: job_id.to_string(),
            stage: progress.stage.clone(),
            percent: progress.percent,
            done: progress.counts.map(|(done, _)| done),
            total: progress.counts.map(|(_, total)| total),
            raw: progress.raw.clone(),
        };
        let _ = app.emit(FETCH_PROGRESS_EVENT, payload);
    }
}

/// 列出（并获取）来源仓库的技能候选，不入库。
#[tauri::command]
pub async fn skill_preview(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    repo: String,
    reference: Option<String>,
    job_id: String,
) -> Result<skill_manager_core::upstream::PreviewReport, String> {
    let cancel = state.register_fetch(&job_id);
    let ctx = state.get()?;
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = UpstreamService::from_context(&ctx, &resolver).map_err(|err| err.to_string())?;
    let mut on_progress = progress_emitter(&app, &job_id);
    let result = service
        .preview(&repo, reference.as_deref(), &mut on_progress, &cancel)
        .map_err(|err| err.to_string());
    state.finish_fetch(&job_id);
    result
}

/// 取消一次进行中的获取。
#[tauri::command]
pub fn skill_fetch_cancel(state: tauri::State<'_, AppState>, job_id: String) -> bool {
    state.cancel_fetch(&job_id)
}

/// 选定候选入库。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn skill_import(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    repo: String,
    domain: String,
    skills: Vec<String>,
    reference: Option<String>,
    category: Option<String>,
    uncategorized: Option<bool>,
    job_id: String,
) -> Result<serde_json::Value, String> {
    let domain = domain.parse::<Domain>().map_err(|err| err.to_string())?;
    let cancel = state.register_fetch(&job_id);
    let ctx = state.get()?;
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let service = UpstreamService::from_context(&ctx, &resolver).map_err(|err| err.to_string())?;
    let mut on_progress = progress_emitter(&app, &job_id);
    let outcome = service
        .import(
            &repo,
            reference.as_deref(),
            domain,
            &skills,
            category_choice(category, uncategorized),
            &mut on_progress,
            &cancel,
        )
        .map_err(|err| err.to_string());
    state.finish_fetch(&job_id);
    let outcome = outcome?;
    Ok(serde_json::json!({
        "items": outcome.items,
        "errors": outcome.errors,
    }))
}

/// 收编本机 Agent 未受管技能（逐技能资料用 `metadata`）。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn skill_adopt(
    state: tauri::State<'_, AppState>,
    agent: String,
    names: Vec<String>,
    category: Option<String>,
    uncategorized: Option<bool>,
    comment: Option<String>,
    tags: Option<Vec<String>>,
    metadata: Option<HashMap<String, MetaInput>>,
) -> Result<skill_manager_core::vault::AdoptOutcome, String> {
    let ctx = state.get()?;
    let resolver = ctx.target_resolver().map_err(|err| err.to_string())?;
    let root = ctx
        .vault()
        .map(|vault| vault.root().to_path_buf())
        .map_err(|err| err.to_string())?;
    let common = AdoptMeta {
        comment,
        tags,
        category: match category_choice(category, uncategorized) {
            CategoryChoice::Given(name) => Some(name),
            _ => None,
        },
        ..AdoptMeta::default()
    };
    let per_skill: BTreeMap<String, AdoptMeta> = metadata
        .unwrap_or_default()
        .into_iter()
        .map(|(name, input)| (name, input.to_meta()))
        .collect();
    skill_manager_core::vault::adopt::adopt_skills(
        &root,
        &resolver,
        ctx.state(),
        &agent,
        &names,
        &common,
        &per_skill,
    )
    .map_err(|err| err.to_string())
}

/// 创建自建技能（不部署）。
#[tauri::command]
pub fn skill_create(
    state: tauri::State<'_, AppState>,
    reference: String,
    category: Option<String>,
    uncategorized: Option<bool>,
    comment: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<skill_manager_core::vault::index::SkillSummary, String> {
    let ctx = state.get()?;
    let root = ctx
        .vault()
        .map(|vault| vault.root().to_path_buf())
        .map_err(|err| err.to_string())?;
    let reference = SkillRef::parse(&reference).map_err(|err| err.to_string())?;
    let category = match category_choice(category, uncategorized) {
        CategoryChoice::Given(name) => Some(name),
        _ => None,
    };
    create_skill(
        &root,
        &reference,
        category.as_deref(),
        comment.as_deref(),
        &tags.unwrap_or_default(),
    )
    .map_err(|err| err.to_string())?;
    summary_of(&root, &reference)
}

/// 跨域复制（不部署）。
#[tauri::command]
pub fn skill_copy(
    state: tauri::State<'_, AppState>,
    reference: String,
    to: String,
    category: Option<String>,
    uncategorized: Option<bool>,
) -> Result<skill_manager_core::vault::index::SkillSummary, String> {
    let ctx = state.get()?;
    let root = ctx
        .vault()
        .map(|vault| vault.root().to_path_buf())
        .map_err(|err| err.to_string())?;
    let reference = SkillRef::parse(&reference).map_err(|err| err.to_string())?;
    let to = to.parse::<Domain>().map_err(|err| err.to_string())?;
    let category = match category_choice(category, uncategorized) {
        CategoryChoice::Given(name) => Some(name),
        _ => None,
    };
    let target = copy_skill(&root, &reference, to, category.as_deref())
        .map_err(|err| err.to_string())?;
    summary_of(&root, &target)
}

fn summary_of(
    root: &std::path::Path,
    reference: &SkillRef,
) -> Result<skill_manager_core::vault::index::SkillSummary, String> {
    let index = skill_manager_core::vault::SkillIndex::build(root).map_err(|err| err.to_string())?;
    index
        .get(reference)
        .map(|entry| entry.to_summary())
        .ok_or_else(|| format!("技能不存在：{reference}"))
}
