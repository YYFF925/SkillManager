//! 工具中心命令（T079/T080）：薄封装 core，不承载业务规则。
//!
//! GUI 只提供**清单/状态/检查、维护任务生成与复制、配套 Skill 候选入库**；工具登记、
//! 共享资料修改与本机安装结果回写由外部 Agent 经 CLI 核实完成（FR-047/050），因此这里
//! 不提供直接写 `tool.yaml` 或安装事实的入口，避免绕过“真实核实后回写”。

use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;
use skill_manager_core::domain::SkillRef;
use skill_manager_core::state::ToolInstallRecord;
use skill_manager_core::toolcenter::files;
use skill_manager_core::toolcenter::{self, ToolQuery, ToolStatusFilter, ToolTaskAction};
use skill_manager_core::vault::index::SkillIndex;
use skill_manager_core::{ToolCheckOutcome, ToolEntry, ToolSkillCandidates};

use crate::HostRuntime;

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
pub fn tool_list(
    runtime: &HostRuntime,
    params: Option<ToolListParams>,
) -> Result<Vec<ToolEntry>, String> {
    let ctx = runtime.get()?;
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
pub fn tool_get(runtime: &HostRuntime, slug: String) -> Result<ToolEntry, String> {
    let ctx = runtime.get()?;
    let root = ctx
        .vault()
        .map_err(|err| err.to_string())?
        .root()
        .to_path_buf();
    toolcenter::catalog::get(&root, ctx.state(), &slug).map_err(|err| err.to_string())
}

/// 执行版本检查：`slug` 给定时检查单项，否则检查全部（`all` 为真）。
pub fn tool_check(
    runtime: &HostRuntime,
    slug: Option<String>,
    all: Option<bool>,
) -> Result<ToolCheckReport, String> {
    let ctx = runtime.get()?;
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
pub fn tool_task(
    runtime: &HostRuntime,
    slug: Option<String>,
    action: String,
    requirement: Option<String>,
) -> Result<String, String> {
    let ctx = runtime.get()?;
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
pub fn tool_skill_candidates(
    runtime: &HostRuntime,
    slug: String,
) -> Result<ToolSkillCandidates, String> {
    let ctx = runtime.get()?;
    let root = ctx
        .vault()
        .map_err(|err| err.to_string())?
        .root()
        .to_path_buf();
    toolcenter::skills::candidates(&root, ctx.state(), &slug).map_err(|err| err.to_string())
}

fn vault_root(runtime: &HostRuntime) -> Result<PathBuf, String> {
    Ok(runtime
        .get()?
        .vault()
        .map_err(|err| err.to_string())?
        .root()
        .to_path_buf())
}

/// 从本机目录登记工具（CLI 转发；Host 与 CLI 同机）。
pub fn tool_register(runtime: &HostRuntime, dir: String) -> Result<Value, String> {
    let root = vault_root(runtime)?;
    let dir = PathBuf::from(dir);
    let path = dir.join(files::TOOL_FILE);
    let text = std::fs::read_to_string(&path).map_err(|err| err.to_string())?;
    let file = files::ToolFile::from_yaml(&text, &path).map_err(|err| err.to_string())?;
    let meta = file.meta().map_err(|err| err.to_string())?;
    meta.validate().map_err(|err| err.to_string())?;
    let slug = files::slugify(&meta.name);
    if files::tool_path(&root, &slug).is_file() {
        return Err(format!("工具已登记：{slug}"));
    }
    let report = files::write_from_dir(&root, &slug, &dir).map_err(|err| err.to_string())?;
    serde_json::to_value(report).map_err(|err| err.to_string())
}

/// 从本机目录更新已登记工具资料。
pub fn tool_set(runtime: &HostRuntime, slug: String, dir: String) -> Result<Value, String> {
    let root = vault_root(runtime)?;
    files::ToolFile::load(&root, &slug).map_err(|err| err.to_string())?;
    let report =
        files::write_from_dir(&root, &slug, &PathBuf::from(dir)).map_err(|err| err.to_string())?;
    serde_json::to_value(report).map_err(|err| err.to_string())
}

/// 回写本机安装事实。
pub fn tool_set_install(
    runtime: &HostRuntime,
    slug: String,
    version: String,
    method: Option<String>,
    path: Option<String>,
    config_path: Option<String>,
) -> Result<Value, String> {
    let ctx = runtime.get()?;
    let root = ctx
        .vault()
        .map_err(|err| err.to_string())?
        .root()
        .to_path_buf();
    files::ToolFile::load(&root, &slug).map_err(|err| err.to_string())?;
    if version.trim().is_empty() {
        ctx.state()
            .clear_tool_install(&slug)
            .map_err(|err| err.to_string())?;
        return Ok(serde_json::json!({ "tool": slug, "installed": false, "check_reset": true }));
    }
    ctx.state()
        .set_tool_install(&ToolInstallRecord {
            tool_slug: slug.clone(),
            version: version.clone(),
            method,
            path,
            config_path,
        })
        .map_err(|err| err.to_string())?;
    Ok(serde_json::json!({
        "tool": slug,
        "installed": true,
        "version": version,
        "check_reset": true,
    }))
}

/// 添加配套 Skill 关联。
pub fn tool_skill_link(
    runtime: &HostRuntime,
    slug: String,
    reference: String,
) -> Result<Value, String> {
    let root = vault_root(runtime)?;
    let parsed = SkillRef::parse(&reference).map_err(|err| err.to_string())?;
    let mut file = files::ToolFile::load(&root, &slug).map_err(|err| err.to_string())?;
    let added = file.add_skill_link(&files::ToolSkillLink {
        domain: parsed.domain,
        name: parsed.name.clone(),
        repo: None,
        subpath: None,
    });
    if added {
        file.save().map_err(|err| err.to_string())?;
    }
    Ok(serde_json::json!({
        "tool": slug,
        "skill": parsed.to_string(),
        "added": added,
    }))
}
