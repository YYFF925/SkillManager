//! 技能库管理命令：薄封装 core，不承载业务规则。
//!
//! JSON 字段与 CLI 的同名操作保持一致，前端只调用这里。

use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use skill_manager_core::domain::{Domain, SkillRef, names_equal};
use skill_manager_core::error::CoreError;
use skill_manager_core::manager::{
    SupportDecl, list_overlays, read_overlay, read_translation, remove_overlay, remove_translation,
    write_overlay, write_translation,
};
use skill_manager_core::vault::index::{
    AgentSupport, CategoryFilter, DeploymentView, ReadLang, SearchQuery, SkillBody, SkillDetail,
    SkillIndex, SkillSummary,
};
use skill_manager_core::vault::{ManageItem, manage};
use skill_manager_core::AppContext;

use super::AppState;

// ---------------------------------------------------------------------------
// 参数与视图类型
// ---------------------------------------------------------------------------

/// 支持声明参数：与 CLI 的 `all` / `none` / 名单对应。
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportArg {
    /// 删除限制键，恢复该域内全支持。
    All,
    /// 显式空集合（不支持任何目标）。
    Empty,
    /// 明确名单。
    List(Vec<String>),
}

impl SupportArg {
    fn to_decl(&self) -> SupportDecl {
        match self {
            SupportArg::All => SupportDecl::All,
            SupportArg::Empty => SupportDecl::Empty,
            SupportArg::List(list) => SupportDecl::List(list.clone()),
        }
    }
}

/// 阅读语言。
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LangArg {
    Zh,
    Orig,
}

impl From<LangArg> for ReadLang {
    fn from(value: LangArg) -> Self {
        match value {
            LangArg::Zh => ReadLang::Zh,
            LangArg::Orig => ReadLang::Orig,
        }
    }
}

/// 技能列表结果。
#[derive(Clone, Debug, Serialize)]
pub struct SkillListResult {
    pub skills: Vec<SkillSummary>,
    pub total: usize,
    /// 分类联动的标签候选。
    pub tag_candidates: Vec<String>,
    pub categories: Vec<String>,
}

/// 标签重命名结果。
#[derive(Clone, Debug, Serialize)]
pub struct TagRenameResult {
    pub from: String,
    pub to: String,
    pub affected: usize,
}

/// 标签删除结果。
#[derive(Clone, Debug, Serialize)]
pub struct TagRemoveResult {
    pub tag: String,
    pub affected: usize,
}

/// 分类重命名结果。
#[derive(Clone, Debug, Serialize)]
pub struct CategoryRenameResult {
    pub domain: String,
    pub from: String,
    pub to: String,
    pub affected: usize,
}

/// 译文状态视图。
#[derive(Clone, Debug, Serialize)]
pub struct TranslationView {
    pub skill: String,
    pub exists: bool,
    pub text: Option<String>,
    pub translation_stale: bool,
}

/// 技能目录内一项文件的阅读结果。
#[derive(Clone, Debug, Serialize)]
pub struct SkillFileView {
    pub skill: String,
    pub path: String,
    /// 非 UTF-8 内容不伪装成文本。
    pub binary: bool,
    pub size: u64,
    pub text: Option<String>,
}

/// 覆盖层一项。
#[derive(Clone, Debug, Serialize)]
pub struct OverlayEntryView {
    pub agent: Option<String>,
    pub os: Option<String>,
    pub path: String,
    pub size: u64,
}

/// 覆盖层读取结果。
#[derive(Clone, Debug, Serialize)]
pub struct OverlayContentView {
    pub skill: String,
    pub agent: Option<String>,
    pub os: Option<String>,
    pub path: String,
    pub exists: bool,
    /// 非 UTF-8 内容不伪装成文本。
    pub binary: bool,
    pub size: u64,
    pub text: Option<String>,
}

/// 覆盖层写入结果。
#[derive(Clone, Debug, Serialize)]
pub struct OverlayWriteView {
    pub skill: String,
    pub agent: Option<String>,
    pub os: Option<String>,
    pub path: String,
    pub written: bool,
}

/// 覆盖层删除结果。
#[derive(Clone, Debug, Serialize)]
pub struct OverlayRemoveView {
    pub skill: String,
    pub agent: Option<String>,
    pub os: Option<String>,
    pub path: String,
    pub removed: bool,
}

// ---------------------------------------------------------------------------
// 共用
// ---------------------------------------------------------------------------

fn root_of(ctx: &AppContext) -> Result<PathBuf, String> {
    ctx.vault()
        .map(|vault| vault.root().to_path_buf())
        .map_err(|err| err.to_string())
}

fn parse_ref(raw: &str) -> Result<SkillRef, String> {
    SkillRef::parse(raw).map_err(|err| err.to_string())
}

fn summary_of(root: &Path, reference: &SkillRef) -> Result<SkillSummary, String> {
    let index = SkillIndex::build(root).map_err(|err| err.to_string())?;
    index
        .get(reference)
        .map(|entry| entry.to_summary())
        .ok_or_else(|| {
            CoreError::SkillNotFound {
                domain: reference.domain.to_string(),
                name: reference.name.clone(),
            }
            .to_string()
        })
}

// ---------------------------------------------------------------------------
// 查询
// ---------------------------------------------------------------------------

/// 组合筛选列出技能。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn skill_list(
    state: tauri::State<'_, AppState>,
    domain: Option<String>,
    query: Option<String>,
    category: Option<String>,
    uncategorized: Option<bool>,
    tags: Option<Vec<String>>,
    agent: Option<String>,
    no_tag: Option<bool>,
    preset: Option<String>,
    deployed_to: Option<String>,
) -> Result<SkillListResult, String> {
    let deployed_target = match &deployed_to {
        Some(raw) => Some(
            skill_manager_core::deploy::parse_target_arg(raw).map_err(|err| err.to_string())?,
        ),
        None => None,
    };

    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let index = SkillIndex::build(&root).map_err(|err| err.to_string())?;

    let domain = match &domain {
        Some(raw) => Some(raw.parse::<Domain>().map_err(|err| err.to_string())?),
        None => None,
    };
    let category = if uncategorized.unwrap_or(false) {
        CategoryFilter::Uncategorized
    } else {
        match &category {
            Some(name) => CategoryFilter::Named(name.clone()),
            None => CategoryFilter::All,
        }
    };
    let agent = match &agent {
        Some(key) => {
            let entry = ctx.catalog().get(key).ok_or_else(|| {
                format!("{key} 不在内置 Agent 名单中")
            })?;
            Some(AgentSupport {
                key: key.clone(),
                domain: entry.reference_domain,
            })
        }
        None => None,
    };

    let search = SearchQuery {
        domain,
        query,
        category: category.clone(),
        tags: tags.unwrap_or_default(),
        require_untagged: no_tag.unwrap_or(false),
        agent,
        preset,
    };
    let mut skills: Vec<SkillSummary> = index
        .search(&search)
        .iter()
        .map(|entry| entry.to_summary())
        .collect();
    if let Some(target) = &deployed_target {
        let target = target.as_str();
        let records = ctx.state().deployments().map_err(|err| err.to_string())?;
        skills.retain(|summary| {
            records.iter().any(|record| {
                record.target == target
                    && record.domain.as_str() == summary.domain
                    && names_equal(&record.name, &summary.name)
            })
        });
    }

    Ok(SkillListResult {
        total: skills.len(),
        tag_candidates: index.tag_candidates(domain, &category),
        categories: index.categories(domain),
        skills,
    })
}

/// 技能详情：正文、资料、来源、文件树、库内路径与部署位置。
#[tauri::command]
pub fn skill_get(
    state: tauri::State<'_, AppState>,
    reference: String,
) -> Result<SkillDetail, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let reference = parse_ref(&reference)?;
    let index = SkillIndex::build(&root).map_err(|err| err.to_string())?;
    let entry = index.get(&reference).ok_or_else(|| {
        CoreError::SkillNotFound {
            domain: reference.domain.to_string(),
            name: reference.name.clone(),
        }
        .to_string()
    })?;
    let detail = SkillDetail::load(entry, &root).map_err(|err| err.to_string())?;
    let records = ctx
        .state()
        .deployments_for(reference.domain, &reference.name)
        .map_err(|err| err.to_string())?;
    let deployments: Vec<DeploymentView> = records
        .iter()
        .map(|record| DeploymentView {
            target: record.target.clone(),
            target_path: record.target_path.clone(),
            status: record.status.as_str().to_string(),
        })
        .collect();
    let note = if deployments.is_empty() {
        "本机没有部署记录".to_string()
    } else {
        format!("本机部署 {} 项", deployments.len())
    };
    Ok(detail.with_deployments(deployments, note))
}

/// 读取正文；有译文默认中文。
#[tauri::command]
pub fn skill_read(
    state: tauri::State<'_, AppState>,
    reference: String,
    lang: Option<LangArg>,
) -> Result<SkillBody, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let reference = parse_ref(&reference)?;
    let index = SkillIndex::build(&root).map_err(|err| err.to_string())?;
    let entry = index.get(&reference).ok_or_else(|| {
        CoreError::SkillNotFound {
            domain: reference.domain.to_string(),
            name: reference.name.clone(),
        }
        .to_string()
    })?;
    skill_manager_core::vault::index::read_body(entry, lang.map(ReadLang::from))
        .map_err(|err| err.to_string())
}

// ---------------------------------------------------------------------------
// 资料写入
// ---------------------------------------------------------------------------

/// 读取技能目录内的一项文件（文件树点击阅读）。
///
/// 只允许技能目录内的路径；相对路径由文件树提供（`/` 分隔）。
#[tauri::command]
pub fn skill_file_read(
    state: tauri::State<'_, AppState>,
    reference: String,
    path: String,
) -> Result<SkillFileView, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let reference = parse_ref(&reference)?;
    let dir = manage::locate(&root, &reference).map_err(|err| err.to_string())?;

    let base = dir
        .canonicalize()
        .map_err(|err| CoreError::io(dir.clone(), err).to_string())?;
    let candidate = dir.join(&path);
    let real = candidate
        .canonicalize()
        .map_err(|err| CoreError::io(candidate.clone(), err).to_string())?;
    if !real.starts_with(&base) {
        return Err(format!("只能阅读技能目录内的文件：{path}"));
    }

    let bytes = std::fs::read(&real).map_err(|err| CoreError::io(real.clone(), err).to_string())?;
    let size = bytes.len() as u64;
    let (text, binary) = match String::from_utf8(bytes) {
        Ok(text) => (Some(text), false),
        Err(_) => (None, true),
    };
    Ok(SkillFileView {
        skill: reference.to_string(),
        path,
        binary,
        size,
        text,
    })
}

/// 修改管理资料；未传的参数保持原值。
#[tauri::command]
pub fn skill_set(
    state: tauri::State<'_, AppState>,
    reference: String,
    comment: Option<String>,
    tags: Option<Vec<String>>,
    agents: Option<SupportArg>,
    oses: Option<SupportArg>,
) -> Result<SkillSummary, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let reference = parse_ref(&reference)?;

    let update = manage::MetaUpdate {
        comment: comment.map(|text| {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }),
        tags,
        agents: agents.map(|arg| arg.to_decl()),
        oses: oses.map(|arg| arg.to_decl()),
    };
    manage::update_meta(&root, &reference, &update).map_err(|err| err.to_string())?;
    summary_of(&root, &reference)
}

/// 批量添加标签。
#[tauri::command]
pub fn skill_tag_add(
    state: tauri::State<'_, AppState>,
    references: Vec<String>,
    tags: Vec<String>,
) -> Result<Vec<ManageItem>, String> {
    tag_batch(state, references, tags, true)
}

/// 批量移除标签。
#[tauri::command]
pub fn skill_tag_detach(
    state: tauri::State<'_, AppState>,
    references: Vec<String>,
    tags: Vec<String>,
) -> Result<Vec<ManageItem>, String> {
    tag_batch(state, references, tags, false)
}

fn tag_batch(
    state: tauri::State<'_, AppState>,
    references: Vec<String>,
    tags: Vec<String>,
    add: bool,
) -> Result<Vec<ManageItem>, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let references: Vec<SkillRef> = references
        .iter()
        .map(|raw| parse_ref(raw))
        .collect::<Result<_, _>>()?;
    Ok(if add {
        manage::add_tags_many(&root, &references, &tags)
    } else {
        manage::detach_tags_many(&root, &references, &tags)
    })
}

/// 全库重命名标签（两域生效）。
#[tauri::command]
pub fn skill_tag_rename(
    state: tauri::State<'_, AppState>,
    old: String,
    new: String,
) -> Result<TagRenameResult, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let affected = manage::rename_tag(&root, &old, &new).map_err(|err| err.to_string())?;
    Ok(TagRenameResult {
        from: old,
        to: new,
        affected,
    })
}

/// 全库删除标签关联（两域生效，不删除技能）。
#[tauri::command]
pub fn skill_tag_remove(
    state: tauri::State<'_, AppState>,
    tag: String,
) -> Result<TagRemoveResult, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let affected = manage::remove_tag(&root, &tag).map_err(|err| err.to_string())?;
    Ok(TagRemoveResult { tag, affected })
}

/// 移动到一级分类（不改变技能名）。
#[tauri::command]
pub fn skill_move(
    state: tauri::State<'_, AppState>,
    reference: String,
    category: Option<String>,
) -> Result<SkillSummary, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let reference = parse_ref(&reference)?;
    manage::move_to_category(&root, &reference, category.as_deref())
        .map_err(|err| err.to_string())?;
    summary_of(&root, &reference)
}

/// 重命名一级分类目录（该分类下全部技能一并移动，技能名不变）。
#[tauri::command]
pub fn skill_category_rename(
    state: tauri::State<'_, AppState>,
    domain: String,
    old: String,
    new: String,
) -> Result<CategoryRenameResult, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let domain = Domain::from_str(&domain).map_err(|err| err.to_string())?;
    let affected = manage::rename_category(&root, domain, &old, &new).map_err(|err| err.to_string())?;
    Ok(CategoryRenameResult {
        domain: domain.to_string(),
        from: old,
        to: new,
        affected,
    })
}

// ---------------------------------------------------------------------------
// 译文与覆盖层
// ---------------------------------------------------------------------------

fn translation_view(root: &Path, reference: &SkillRef) -> Result<TranslationView, String> {
    let dir = manage::locate(root, reference).map_err(|err| err.to_string())?;
    let text = read_translation(&dir).map_err(|err| err.to_string())?;
    let index = SkillIndex::build(root).map_err(|err| err.to_string())?;
    let stale = index
        .get(reference)
        .map(|entry| entry.translation_stale)
        .unwrap_or(false);
    Ok(TranslationView {
        skill: reference.to_string(),
        exists: text.is_some(),
        text,
        translation_stale: stale,
    })
}

/// 读取译文本体（无译文如实为空）。
#[tauri::command]
pub fn skill_translation_get(
    state: tauri::State<'_, AppState>,
    reference: String,
) -> Result<TranslationView, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let reference = parse_ref(&reference)?;
    translation_view(&root, &reference)
}

/// 按当前原文保存译文并更新时效依据。
#[tauri::command]
pub fn skill_translation_set(
    state: tauri::State<'_, AppState>,
    reference: String,
    content: String,
) -> Result<TranslationView, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let reference = parse_ref(&reference)?;
    let dir = manage::locate(&root, &reference).map_err(|err| err.to_string())?;
    write_translation(&dir, &content).map_err(|err| err.to_string())?;
    translation_view(&root, &reference)
}

/// 删除译文。
#[tauri::command]
pub fn skill_translation_remove(
    state: tauri::State<'_, AppState>,
    reference: String,
) -> Result<TranslationView, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let reference = parse_ref(&reference)?;
    let dir = manage::locate(&root, &reference).map_err(|err| err.to_string())?;
    remove_translation(&dir).map_err(|err| err.to_string())?;
    translation_view(&root, &reference)
}

/// 列出覆盖层文件。
#[tauri::command]
pub fn skill_overlay_list(
    state: tauri::State<'_, AppState>,
    reference: String,
) -> Result<Vec<OverlayEntryView>, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let reference = parse_ref(&reference)?;
    let dir = manage::locate(&root, &reference).map_err(|err| err.to_string())?;
    let entries = list_overlays(&dir).map_err(|err| err.to_string())?;
    Ok(entries
        .into_iter()
        .map(|entry| OverlayEntryView {
            agent: entry.agent,
            os: entry.os,
            path: entry.rel,
            size: entry.size,
        })
        .collect())
}

/// 读取一项覆盖层文件。
#[tauri::command]
pub fn skill_overlay_get(
    state: tauri::State<'_, AppState>,
    reference: String,
    path: String,
    agent: Option<String>,
    os: Option<String>,
) -> Result<OverlayContentView, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let reference = parse_ref(&reference)?;
    let dir = manage::locate(&root, &reference).map_err(|err| err.to_string())?;
    let bytes = read_overlay(&dir, agent.as_deref(), os.as_deref(), &path)
        .map_err(|err| err.to_string())?;

    let (text, binary, size) = match bytes {
        Some(bytes) => {
            let size = bytes.len() as u64;
            match String::from_utf8(bytes) {
                Ok(text) => (Some(text), false, size),
                Err(_) => (None, true, size),
            }
        }
        None => (None, false, 0),
    };
    Ok(OverlayContentView {
        skill: reference.to_string(),
        agent,
        os,
        path,
        exists: text.is_some() || binary,
        binary,
        size,
        text,
    })
}

/// 写入一项覆盖层文件。
#[tauri::command]
pub fn skill_overlay_set(
    state: tauri::State<'_, AppState>,
    reference: String,
    path: String,
    content: String,
    agent: Option<String>,
    os: Option<String>,
) -> Result<OverlayWriteView, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let reference = parse_ref(&reference)?;
    let dir = manage::locate(&root, &reference).map_err(|err| err.to_string())?;
    write_overlay(
        &dir,
        agent.as_deref(),
        os.as_deref(),
        &path,
        content.as_bytes(),
    )
    .map_err(|err| err.to_string())?;
    Ok(OverlayWriteView {
        skill: reference.to_string(),
        agent,
        os,
        path,
        written: true,
    })
}

/// 删除一项覆盖层文件。
#[tauri::command]
pub fn skill_overlay_remove(
    state: tauri::State<'_, AppState>,
    reference: String,
    path: String,
    agent: Option<String>,
    os: Option<String>,
) -> Result<OverlayRemoveView, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let reference = parse_ref(&reference)?;
    let dir = manage::locate(&root, &reference).map_err(|err| err.to_string())?;
    let removed = remove_overlay(&dir, agent.as_deref(), os.as_deref(), &path)
        .map_err(|err| err.to_string())?;
    Ok(OverlayRemoveView {
        skill: reference.to_string(),
        agent,
        os,
        path,
        removed,
    })
}

// ---------------------------------------------------------------------------
// 维护任务
// ---------------------------------------------------------------------------

/// 生成技能维护任务说明：单项或多选，保留用户要求、库/技能路径、管理资料概况与已有部署；
/// 规范只引用配套维护 Skill。生成任务不改库、不部署、不上传。
#[tauri::command]
pub fn skill_task(
    state: tauri::State<'_, AppState>,
    references: Vec<String>,
    requirement: Option<String>,
) -> Result<String, String> {
    let ctx = state.get()?;
    let root = root_of(&ctx)?;
    let refs: Vec<SkillRef> = references
        .iter()
        .map(|raw| parse_ref(raw))
        .collect::<Result<_, _>>()?;
    let index = SkillIndex::build(&root).map_err(|err| err.to_string())?;
    let deployments = ctx.state().deployments().map_err(|err| err.to_string())?;
    skill_manager_core::vault::tasks::skill_task(
        &root,
        &index,
        &deployments,
        &refs,
        requirement.as_deref(),
    )
    .map_err(|err| err.to_string())
}
