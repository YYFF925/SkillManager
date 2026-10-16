//! 工具中心命令（T079/T080）：薄封装 core，不承载业务规则。
//!
//! GUI 只提供**清单/状态/检查、维护任务生成与复制、配套 Skill 候选入库**；工具登记、
//! 共享资料修改与本机安装结果回写由外部 Agent 经 CLI 核实完成（FR-047/050），因此这里
//! 不提供直接写 `tool.yaml` 或安装事实的入口，避免绕过“真实核实后回写”。

use serde::Serialize;
use skill_manager_core::toolcenter::{self, ToolQuery, ToolStatusFilter, ToolTaskAction};
use skill_manager_core::vault::index::SkillIndex;
use skill_manager_core::{ToolCheckOutcome, ToolEntry, ToolSkillCandidates};

use super::AppState;

/// 工具清单查询参数。
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct ToolListParams {
    pub query: Option<String>,
    pub category: Option<String>,
    pub status: Option<String>,
}

/// 一次检查的结果集合。
#[derive(Clone, Debug, Serialize)]
pub struct ToolCheckReport {
    pub checked: usize,
    pub tools: Vec<ToolCheckOutcome>,
}

fn parse_status(raw: Option<&str>) -> Result<Option<ToolStatusFilter>, String> {
    match raw.map(str::trim).filter(|text| !text.is_empty()) {
        Some(value) => Ok(Some(
            ToolStatusFilter::parse(value).map_err(|err| err.to_string())?,
        )),
        None => Ok(None),
    }
}

/// 工具清单（含未安装条目）。
#[tauri::command]
pub fn tool_list(
    state: tauri::State<'_, AppState>,
    params: Option<ToolListParams>,
) -> Result<Vec<ToolEntry>, String> {
    let ctx = state.get()?;
    let root = ctx
        .vault()
        .map_err(|err| err.to_string())?
        .root()
        .to_path_buf();
    let params = params.unwrap_or_default();
    let status = parse_status(params.status.as_deref())?;
    let query = ToolQuery {
        query: params.query.filter(|text| !text.trim().is_empty()),
        category: params.category.filter(|text| !text.trim().is_empty()),
        status,
    };
    toolcenter::catalog::list(&root, ctx.state(), &query).map_err(|err| err.to_string())
}

/// 工具详情：简介、本机事实、检查结果、指南与配套 Skill。
#[tauri::command]
pub fn tool_get(
    state: tauri::State<'_, AppState>,
    slug: String,
) -> Result<ToolEntry, String> {
    let ctx = state.get()?;
    let root = ctx
        .vault()
        .map_err(|err| err.to_string())?
        .root()
        .to_path_buf();
    toolcenter::catalog::get(&root, ctx.state(), &slug).map_err(|err| err.to_string())
}

/// 执行版本检查：`slug` 给定时检查单项，否则检查全部（`all` 为真）。
#[tauri::command]
pub fn tool_check(
    state: tauri::State<'_, AppState>,
    slug: Option<String>,
    all: Option<bool>,
) -> Result<ToolCheckReport, String> {
    let ctx = state.get()?;
    let root = ctx
        .vault()
        .map_err(|err| err.to_string())?
        .root()
        .to_path_buf();
    let tools = match (slug, all.unwrap_or(false)) {
        (Some(slug), false) => {
            vec![toolcenter::check_tool(&root, ctx.state(), &slug).map_err(|err| err.to_string())?]
        }
        (None, true) => toolcenter::check_all(&root, ctx.state()).map_err(|err| err.to_string())?,
        _ => {
            return Err("工具检查须给出一个 slug 或全部检查（二者恰好一个）".to_string());
        }
    };
    Ok(ToolCheckReport {
        checked: tools.len(),
        tools,
    })
}

/// 生成工具维护任务说明（复制给外部 Agent；不代表已执行）。
#[tauri::command]
pub fn tool_task(
    state: tauri::State<'_, AppState>,
    slug: Option<String>,
    action: String,
    requirement: Option<String>,
) -> Result<String, String> {
    let ctx = state.get()?;
    let root = ctx
        .vault()
        .map_err(|err| err.to_string())?
        .root()
        .to_path_buf();
    let action = ToolTaskAction::parse(action.trim()).map_err(|err| err.to_string())?;
    let index = SkillIndex::build(&root).ok();
    toolcenter::tool_task(
        &root,
        ctx.state(),
        index.as_ref(),
        slug.as_deref(),
        action,
        requirement.as_deref(),
    )
    .map_err(|err| err.to_string())
}

/// 配套 Skill 候选（含工具版本概况；入库仍走 US2 的预览与选择流程）。
#[tauri::command]
pub fn tool_skill_candidates(
    state: tauri::State<'_, AppState>,
    slug: String,
) -> Result<ToolSkillCandidates, String> {
    let ctx = state.get()?;
    let root = ctx
        .vault()
        .map_err(|err| err.to_string())?
        .root()
        .to_path_buf();
    toolcenter::skills::candidates(&root, ctx.state(), &slug).map_err(|err| err.to_string())
}
