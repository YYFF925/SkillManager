//! `skill preview/import/adopt/create/copy` 与 `skill check/update/update-repo/reject-new/
//! rejected/skip/unskip/skipped`（T037/T047）。
//!
//! 参数与输出遵循 `specs/002-skill-management/contracts/cli.md`；业务规则全部在 core。
//! 进度写标准错误、最终 JSON 结果写标准输出；所有选择由显式参数传入，不要求回 GUI 补裁决。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use clap::Args;
use serde_json::{json, Value};
use skill_manager_core::domain::{Domain, SkillRef};
use skill_manager_core::manager::SupportDecl;
use skill_manager_core::sync::GitProgress;
use skill_manager_core::upstream::check::CheckScope;
use skill_manager_core::upstream::{decisions, UpdateRepoOutcome, UpstreamService};
use skill_manager_core::vault::acquire::CategoryChoice;
use skill_manager_core::vault::{adopt::AdoptMeta, copy_skill, create_skill};
use skill_manager_core::{AppContext, CliResult, CoreError};

type Result<T> = std::result::Result<T, CoreError>;

// ---------------------------------------------------------------------------
// 参数
// ---------------------------------------------------------------------------

#[derive(Args, Debug)]
pub struct PreviewArgs {
    /// 来源仓库地址（本机 Git 可用形式）。
    #[arg(value_name = "repo-url")]
    pub repo: String,
    /// 所选分支或标签（缺省跟踪远端默认分支）。
    #[arg(long = "ref", value_name = "branch-or-tag")]
    pub reference: Option<String>,
}

#[derive(Args, Debug)]
pub struct ImportArgs {
    #[arg(value_name = "repo-url")]
    pub repo: String,
    /// 上游子路径（逗号分隔，可重复）。
    #[arg(long = "skills", value_name = "subpath", value_delimiter = ',', required = true)]
    pub skills: Vec<String>,
    #[arg(long, value_name = "domain")]
    pub domain: String,
    #[arg(long = "ref", value_name = "branch-or-tag")]
    pub reference: Option<String>,
    #[arg(long, value_name = "category", conflicts_with = "uncategorized")]
    pub category: Option<String>,
    #[arg(long, conflicts_with = "category")]
    pub uncategorized: bool,
}

#[derive(Args, Debug)]
pub struct AdoptArgs {
    /// 内置 Agent key 或共享目录目标 shared。
    #[arg(value_name = "target")]
    pub agent: String,
    #[arg(value_name = "name", required = true)]
    pub names: Vec<String>,
    #[arg(long, value_name = "category", conflicts_with = "uncategorized")]
    pub category: Option<String>,
    #[arg(long, conflicts_with = "category")]
    pub uncategorized: bool,
    #[arg(long, value_name = "text")]
    pub comment: Option<String>,
    #[arg(long, value_name = "csv")]
    pub tags: Option<String>,
    /// 按技能名给出各自管理资料的 YAML 文件。
    #[arg(long = "metadata-file", value_name = "path")]
    pub metadata_file: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(value_name = "ref")]
    pub reference: String,
    #[arg(long, value_name = "category", conflicts_with = "uncategorized")]
    pub category: Option<String>,
    #[arg(long, conflicts_with = "category")]
    pub uncategorized: bool,
    #[arg(long, value_name = "text")]
    pub comment: Option<String>,
    #[arg(long, value_name = "csv")]
    pub tags: Option<String>,
}

#[derive(Args, Debug)]
pub struct CopyArgs {
    #[arg(value_name = "ref")]
    pub reference: String,
    /// 目标管理域。
    #[arg(long = "to", value_name = "domain")]
    pub to: String,
    #[arg(long, value_name = "category", conflicts_with = "uncategorized")]
    pub category: Option<String>,
    #[arg(long, conflicts_with = "category")]
    pub uncategorized: bool,
}

#[derive(Args, Debug)]
pub struct CheckArgs {
    /// 单个技能 `<domain>/<name>`。
    #[arg(value_name = "ref")]
    pub reference: Option<String>,
    /// 指定来源仓库地址。
    #[arg(long = "repo", value_name = "url")]
    pub repo: Option<String>,
    /// 全库登记来源。
    #[arg(long)]
    pub all: bool,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    #[arg(value_name = "ref")]
    pub reference: String,
    /// 明确采用的完整提交 OID（缺省用最近检查所得）。
    #[arg(long = "to", value_name = "commit-oid")]
    pub to: Option<String>,
}

#[derive(Args, Debug)]
pub struct UpdateRepoArgs {
    #[arg(value_name = "repo-url")]
    pub repo: String,
    /// 本次选定入库的新增候选子路径（逗号分隔，可重复）。
    #[arg(long = "include-new", value_name = "subpath", value_delimiter = ',')]
    pub include_new: Vec<String>,
    /// 新增入库到哪个管理域。
    #[arg(long, value_name = "domain")]
    pub domain: Option<String>,
    #[arg(long, value_name = "category", conflicts_with = "uncategorized")]
    pub category: Option<String>,
    #[arg(long, conflicts_with = "category")]
    pub uncategorized: bool,
}

#[derive(Args, Debug)]
pub struct RejectNewArgs {
    #[arg(value_name = "repo-url")]
    pub repo: String,
    #[arg(long, value_name = "domain")]
    pub domain: String,
    #[arg(long = "skills", value_name = "subpath", value_delimiter = ',', required = true)]
    pub skills: Vec<String>,
}

#[derive(Args, Debug)]
pub struct RejectedArgs {
    #[arg(long = "repo", value_name = "url")]
    pub repo: Option<String>,
    #[arg(long, value_name = "domain")]
    pub domain: Option<String>,
}

#[derive(Args, Debug)]
pub struct SkipArgs {
    #[arg(value_name = "ref")]
    pub reference: String,
    /// 跳过的完整提交 OID（缺省用最近检查所得）。
    #[arg(long = "version", value_name = "commit-oid")]
    pub version: Option<String>,
}

/// metadata-file 中单个技能的补充资料。
#[derive(Debug, Default, serde::Deserialize)]
struct MetaSpec {
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    agents: Option<Vec<String>>,
    #[serde(default)]
    oses: Option<Vec<String>>,
    #[serde(default)]
    translation: Option<String>,
}

// ---------------------------------------------------------------------------
// 共用
// ---------------------------------------------------------------------------

fn parse_csv(raw: &str) -> Vec<String> {
    raw.split([',', '，'])
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn category_choice(category: Option<String>, uncategorized: bool) -> CategoryChoice {
    if uncategorized {
        CategoryChoice::Uncategorized
    } else {
        match category {
            Some(name) => CategoryChoice::Given(name),
            None => CategoryChoice::Default,
        }
    }
}

/// CLI 的 `--category` / `--uncategorized` 二者都不给 = 默认（按仓库技能总数）。
fn explicit_category(category: Option<String>, uncategorized: bool) -> Option<CategoryChoice> {
    if uncategorized {
        Some(CategoryChoice::Uncategorized)
    } else {
        category.map(CategoryChoice::Given)
    }
}

fn with_upstream<T>(
    ctx: &AppContext,
    run: impl FnOnce(&UpstreamService<'_>) -> Result<T>,
) -> Result<T> {
    let resolver = ctx.target_resolver()?;
    let service = UpstreamService::from_context(ctx, &resolver)?;
    run(&service)
}

/// 建立取消标记并登记 Ctrl-C 处理；取消即停止本次获取（不误报入库）。
fn cancel_flag() -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    let handle = flag.clone();
    let _ = ctrlc::set_handler(move || {
        handle.store(true, Ordering::SeqCst);
    });
    flag
}

/// 把真实进度写到标准错误（阶段 + 可得的进展量），最终结果仍到标准输出。
fn stderr_progress(progress: &GitProgress) {
    let mut line = progress.raw.clone();
    if let (Some(stage), Some(percent)) = (&progress.stage, progress.percent) {
        if let Some((done, total)) = progress.counts {
            line = format!("{stage}: {percent}% ({done}/{total})");
        }
    }
    eprint!("\r\x1b[K{line}");
}

fn finish_progress() {
    eprintln!();
}

// ---------------------------------------------------------------------------
// preview / import
// ---------------------------------------------------------------------------

pub fn preview(ctx: &AppContext, args: PreviewArgs) -> Result<(CliResult, String)> {
    let cancel = cancel_flag();
    let mut on_progress = |p: &GitProgress| stderr_progress(p);
    let report = with_upstream(ctx, |service| {
        service.preview(&args.repo, args.reference.as_deref(), &mut on_progress, &cancel)
    })?;
    finish_progress();

    let data = json!({
        "repo": report.repo,
        "ref": report.reference,
        "remote_version": report.remote_version,
        "skill_count": report.skill_count,
        "default_category": report.default_category,
        "candidates": report.candidates,
    });
    let mut human = format!(
        "来源 {} 共 {} 个技能候选（默认分类：{}）",
        report.repo,
        report.skill_count,
        report.default_category.as_deref().unwrap_or("未分类")
    );
    for candidate in &report.candidates {
        human.push_str(&format!(
            "\n- {}（{}）{}",
            candidate.name,
            candidate.subpath,
            candidate.description.as_deref().unwrap_or("")
        ));
    }
    Ok((CliResult::from_data(&data), human))
}

pub fn import(ctx: &AppContext, args: ImportArgs) -> Result<(CliResult, String)> {
    let domain = Domain::from_str(&args.domain)?;
    let category = category_choice(args.category.clone(), args.uncategorized);
    let cancel = cancel_flag();
    let mut on_progress = |p: &GitProgress| stderr_progress(p);
    let repo = args.repo.clone();
    let reference = args.reference.clone();
    let subpaths = args.skills.clone();
    let outcome = with_upstream(ctx, |service| {
        service.import(
            &repo,
            reference.as_deref(),
            domain,
            &subpaths,
            category,
            &mut on_progress,
            &cancel,
        )
    })?;
    finish_progress();

    let data = json!({
        "repo": args.repo,
        "domain": args.domain,
        "items": outcome.items,
        "count": outcome.items.len(),
    });
    let result = if outcome.errors.is_empty() {
        CliResult::success(data)
    } else if outcome.items.is_empty() {
        CliResult::failed(outcome.errors)
    } else {
        CliResult::partial(data, outcome.errors)
    };
    let mut human = format!(
        "已从 {} 入库 {} 项到 {}",
        args.repo,
        outcome.items.len(),
        args.domain
    );
    if !result.errors.is_empty() {
        human.push_str(&format!("；{} 项未完成", result.errors.len()));
    }
    for item in &outcome.items {
        human.push_str(&format!(
            "\n- {} → {}（分类：{}）",
            item.subpath,
            item.skill,
            item.category.as_deref().unwrap_or("未分类")
        ));
    }
    Ok((result, human))
}

// ---------------------------------------------------------------------------
// adopt / create / copy
// ---------------------------------------------------------------------------

pub fn adopt(ctx: &AppContext, args: AdoptArgs) -> Result<(CliResult, String)> {
    let resolver = ctx.target_resolver()?;
    let root = ctx.vault()?.root().to_path_buf();

    let common = AdoptMeta {
        comment: args.comment.clone(),
        tags: args.tags.as_ref().map(|raw| parse_csv(raw)),
        category: match explicit_category(args.category.clone(), args.uncategorized) {
            Some(CategoryChoice::Given(name)) => Some(name),
            _ => None,
        },
        ..AdoptMeta::default()
    };
    let per_skill = match &args.metadata_file {
        Some(path) => load_metadata_file(path)?,
        None => BTreeMap::new(),
    };

    let outcome = skill_manager_core::vault::adopt::adopt_skills(
        &root,
        &resolver,
        ctx.state(),
        &args.agent,
        &args.names,
        &common,
        &per_skill,
    )?;

    let data = json!({
        "agent": outcome.agent,
        "domain": outcome.domain,
        "items": outcome.items,
        "adopted": outcome.adopted,
    });
    let result = if outcome.errors.is_empty() {
        CliResult::success(data)
    } else if outcome.adopted == 0 {
        CliResult::failed(outcome.errors)
    } else {
        CliResult::partial(data, outcome.errors)
    };
    let mut human = format!(
        "已从 {} 收编 {} 项到{}并接管原部署",
        args.agent,
        outcome.adopted,
        if outcome.domain == "code" { "编码库" } else { "龙虾库" }
    );
    if !result.errors.is_empty() {
        human.push_str(&format!("；{} 项未完成", result.errors.len()));
    }
    for item in &outcome.items {
        if item.ok {
            human.push_str(&format!(
                "\n- {} → {}",
                item.name,
                item.skill.as_deref().unwrap_or("-")
            ));
        }
    }
    Ok((result, human))
}

fn load_metadata_file(path: &PathBuf) -> Result<BTreeMap<String, AdoptMeta>> {
    let bytes = std::fs::read(path).map_err(|err| CoreError::io(path.clone(), err))?;
    let text = String::from_utf8(bytes)
        .map_err(|err| CoreError::other(format!("{} 不是有效 UTF-8：{err}", path.display())))?;
    let specs: BTreeMap<String, MetaSpec> =
        yaml_serde::from_str(&text).map_err(|err| CoreError::Yaml {
            path: path.clone(),
            message: err.to_string(),
        })?;
    let mut out = BTreeMap::new();
    for (name, spec) in specs {
        out.insert(
            name,
            AdoptMeta {
                comment: spec.comment,
                tags: spec.tags,
                category: spec.category,
                agents: spec.agents.map(support_decl),
                oses: spec.oses.map(support_decl),
                translation: spec.translation,
            },
        );
    }
    Ok(out)
}

fn support_decl(list: Vec<String>) -> SupportDecl {
    if list.is_empty() {
        SupportDecl::Empty
    } else {
        SupportDecl::List(list)
    }
}

pub fn create(ctx: &AppContext, args: CreateArgs) -> Result<(CliResult, String)> {
    let reference = SkillRef::parse(&args.reference)?;
    let root = ctx.vault()?.root().to_path_buf();
    let category = match explicit_category(args.category.clone(), args.uncategorized) {
        Some(CategoryChoice::Given(name)) => Some(name),
        _ => None,
    };
    let tags = args.tags.as_ref().map(|raw| parse_csv(raw)).unwrap_or_default();
    create_skill(
        &root,
        &reference,
        category.as_deref(),
        args.comment.as_deref(),
        &tags,
    )?;
    let index = skill_manager_core::vault::SkillIndex::build(&root)?;
    let summary = index
        .get(&reference)
        .map(|entry| entry.to_summary())
        .ok_or_else(|| CoreError::SkillNotFound {
            domain: reference.domain.to_string(),
            name: reference.name.clone(),
        })?;
    let data = json!({ "skill": summary });
    let human = format!("已创建自建技能 {reference}（未部署）");
    Ok((CliResult::from_data(&data), human))
}

pub fn copy(ctx: &AppContext, args: CopyArgs) -> Result<(CliResult, String)> {
    let reference = SkillRef::parse(&args.reference)?;
    let to = Domain::from_str(&args.to)?;
    let root = ctx.vault()?.root().to_path_buf();
    let category = match explicit_category(args.category.clone(), args.uncategorized) {
        Some(CategoryChoice::Given(name)) => Some(name),
        _ => None,
    };
    let target_ref = copy_skill(&root, &reference, to, category.as_deref())?;
    let index = skill_manager_core::vault::SkillIndex::build(&root)?;
    let summary = index
        .get(&target_ref)
        .map(|entry| entry.to_summary())
        .ok_or_else(|| CoreError::SkillNotFound {
            domain: target_ref.domain.to_string(),
            name: target_ref.name.clone(),
        })?;
    let data = json!({ "skill": summary, "from": reference.to_string() });
    let human = format!(
        "已复制 {reference} → {target_ref}（原件保留、副本独立维护、未部署）"
    );
    Ok((CliResult::from_data(&data), human))
}

// ---------------------------------------------------------------------------
// check / update / update-repo
// ---------------------------------------------------------------------------

pub fn check(ctx: &AppContext, args: CheckArgs) -> Result<(CliResult, String)> {
    let scope = match (&args.reference, &args.repo, args.all) {
        (Some(raw), None, false) => CheckScope::Single(SkillRef::parse(raw)?),
        (None, Some(repo), false) => CheckScope::Repo(repo.clone()),
        (None, None, true) => CheckScope::All,
        _ => {
            return Err(CoreError::other(
                "检查范围需为 <ref>、--repo <url> 或 --all 之一",
            ));
        }
    };
    let cancel = cancel_flag();
    let mut on_progress = |p: &GitProgress| stderr_progress(p);
    let report = with_upstream(ctx, |service| service.check(&scope, &mut on_progress, &cancel))?;
    finish_progress();

    let data = serde_json::to_value(&report)
        .map_err(|err| CoreError::other(format!("序列化检查结果失败：{err}")))?;
    let mut human = format!(
        "检查完成：仓库 {} 个，技能 {} 个，新增候选 {} 个",
        report.repos.len(),
        report.skills.len(),
        report.candidates.len()
    );
    for stamp in &report.repos {
        human.push_str(&format!(
            "\n- {}（{}）",
            stamp.repo,
            if stamp.ok {
                stamp.remote_version.as_deref().unwrap_or("-")
            } else {
                stamp.error.as_deref().unwrap_or("检查失败")
            }
        ));
    }
    for skill in &report.skills {
        human.push_str(&format!("\n- {}：{}", skill.skill, result_label(&skill.result)));
    }
    for candidate in &report.candidates {
        human.push_str(&format!(
            "\n- 候选 {}/{}（{}{}）",
            candidate.domain,
            candidate.name,
            candidate.subpath,
            if candidate.is_new { "，后来新增" } else { "" }
        ));
    }
    Ok((CliResult::from_data(&data), human))
}

fn result_label(result: &str) -> &'static str {
    match result {
        "up_to_date" => "已是最新",
        "update_available" => "有更新",
        "skipped" => "已跳过该版本",
        "upstream_missing" => "上游已删除该技能",
        "error" => "检查失败",
        _ => "未知",
    }
}

pub fn update(ctx: &AppContext, args: UpdateArgs) -> Result<(CliResult, String)> {
    let reference = SkillRef::parse(&args.reference)?;
    let cancel = cancel_flag();
    let mut on_progress = |p: &GitProgress| stderr_progress(p);
    let item = with_upstream(ctx, |service| {
        service.update(&reference, args.to.clone(), &mut on_progress, &cancel)
    })?;
    finish_progress();
    let data = serde_json::to_value(&item)
        .map_err(|err| CoreError::other(format!("序列化更新结果失败：{err}")))?;
    let human = match item.action.as_str() {
        "removed" => format!(
            "{}：上游已删除该技能，已移除库内条目并清理受管部署",
            item.skill
        ),
        _ => format!(
            "{}：已从 {} 更新到 {}",
            item.skill, item.from_version, item.to_version
        ),
    };
    Ok((CliResult::from_data(&data), human))
}

pub fn update_repo(ctx: &AppContext, args: UpdateRepoArgs) -> Result<(CliResult, String)> {
    let domain = match &args.domain {
        Some(raw) => Some(Domain::from_str(raw)?),
        None => None,
    };
    let category = category_choice(args.category.clone(), args.uncategorized);
    let cancel = cancel_flag();
    let mut on_progress = |p: &GitProgress| stderr_progress(p);
    let repo = args.repo.clone();
    let include_new = args.include_new.clone();
    let outcome = with_upstream(ctx, |service| {
        service.update_repo(
            &repo,
            &include_new,
            domain,
            category,
            &mut on_progress,
            &cancel,
        )
    })?;
    finish_progress();
    Ok(update_repo_result(outcome))
}

fn update_repo_result(outcome: UpdateRepoOutcome) -> (CliResult, String) {
    let data = json!({
        "repo": outcome.repo,
        "items": outcome.items,
        "pending_deletions": outcome.pending_deletions,
        "imported": outcome.imported,
    });
    let result = if outcome.errors.is_empty() {
        CliResult::success(data)
    } else if outcome.items.is_empty() && outcome.imported.is_empty() {
        CliResult::failed(outcome.errors)
    } else {
        CliResult::partial(data, outcome.errors)
    };
    let mut human = format!(
        "仓库 {} 批量更新：已更新 {} 项，新增入库 {} 项",
        outcome.repo,
        outcome.items.len(),
        outcome.imported.len()
    );
    if !outcome.pending_deletions.is_empty() {
        human.push_str(&format!(
            "；{} 个技能被上游删除（批量不自动删除，请用 skill update <ref> 显式确认）",
            outcome.pending_deletions.len()
        ));
    }
    if !result.errors.is_empty() {
        human.push_str(&format!("；{} 项未完成", result.errors.len()));
    }
    (result, human)
}

// ---------------------------------------------------------------------------
// reject-new / rejected / skip / unskip / skipped
// ---------------------------------------------------------------------------

pub fn reject_new(ctx: &AppContext, args: RejectNewArgs) -> Result<(CliResult, String)> {
    let domain = Domain::from_str(&args.domain)?;
    let root = ctx.vault()?.root().to_path_buf();
    decisions::reject(&root, &args.repo, domain, &args.skills)?;
    let data = json!({
        "repo": args.repo,
        "domain": args.domain,
        "rejected": args.skills,
    });
    let human = format!(
        "已记住拒绝 {} 的 {} 个子路径进入 {}（之后不再提示，随库共享）",
        args.repo,
        args.skills.len(),
        args.domain
    );
    Ok((CliResult::from_data(&data), human))
}

pub fn rejected(ctx: &AppContext, args: RejectedArgs) -> Result<(CliResult, String)> {
    let domain = match &args.domain {
        Some(raw) => Some(Domain::from_str(raw)?),
        None => None,
    };
    let root = ctx.vault()?.root().to_path_buf();
    let entries = decisions::entries(&root, args.repo.as_deref(), domain)?;
    let data = json!({ "entries": entries });
    let mut human = format!("已拒绝新增记录 {} 条", entries.len());
    for entry in &entries {
        human.push_str(&format!(
            "\n- {} [{}]: {}",
            entry.repo,
            entry.domain,
            entry.rejected_subpaths.join("、")
        ));
    }
    Ok((CliResult::from_data(&data), human))
}

pub fn skip(ctx: &AppContext, args: SkipArgs) -> Result<(CliResult, String)> {
    let reference = SkillRef::parse(&args.reference)?;
    let root = ctx.vault()?.root().to_path_buf();
    let index = skill_manager_core::vault::SkillIndex::build(&root)?;
    let entry = index
        .get(&reference)
        .ok_or_else(|| CoreError::SkillNotFound {
            domain: reference.domain.to_string(),
            name: reference.name.clone(),
        })?;
    let source = entry
        .source
        .clone()
        .ok_or_else(|| skill_manager_core::upstream::no_source(&reference))?;
    let version = match args.version {
        Some(version) => version,
        None => ctx
            .state()
            .latest_remote_version(&source.repo, reference.domain, &reference.name)?
            .ok_or_else(|| {
                CoreError::other(format!(
                    "尚无检查结果：{reference}；请先执行 skill check（不在跳过时静默选择另一个版本）"
                ))
            })?,
    };
    let mut record = source.clone();
    record.skipped_version = Some(version.clone());
    skill_manager_core::manager::write_source(&entry.dir, &record)?;

    let data = json!({ "skill": reference.to_string(), "skipped_version": version });
    let human = format!("已跳过 {reference} 的上游版本 {version}（不影响其他技能/域）");
    Ok((CliResult::from_data(&data), human))
}

pub fn unskip(ctx: &AppContext, args: RefArg) -> Result<(CliResult, String)> {
    let reference = SkillRef::parse(&args.reference)?;
    let root = ctx.vault()?.root().to_path_buf();
    let index = skill_manager_core::vault::SkillIndex::build(&root)?;
    let entry = index
        .get(&reference)
        .ok_or_else(|| CoreError::SkillNotFound {
            domain: reference.domain.to_string(),
            name: reference.name.clone(),
        })?;
    let mut record = skill_manager_core::manager::read_source(&entry.dir)?
        .ok_or_else(|| skill_manager_core::upstream::no_source(&reference))?;
    record.skipped_version = None;
    skill_manager_core::manager::write_source(&entry.dir, &record)?;
    let data = json!({ "skill": reference.to_string(), "skipped_version": Value::Null });
    let human = format!("已恢复 {reference} 的上游更新提示");
    Ok((CliResult::from_data(&data), human))
}

pub fn skipped(ctx: &AppContext) -> Result<(CliResult, String)> {
    let root = ctx.vault()?.root().to_path_buf();
    let index = skill_manager_core::vault::SkillIndex::build(&root)?;
    let mut skills = Vec::new();
    for entry in index.all() {
        let Some(source) = &entry.source else { continue };
        let Some(skipped) = &source.skipped_version else {
            continue;
        };
        skills.push(json!({
            "skill": entry.reference().to_string(),
            "repo": source.repo,
            "subpath": source.subpath,
            "adopted_version": source.version,
            "skipped_version": skipped,
        }));
    }
    let data = json!({ "skills": skills });
    let mut human = format!("已跳过版本 {} 项", skills.len());
    for skill in &skills {
        human.push_str(&format!(
            "\n- {}（跳过 {}）",
            skill["skill"].as_str().unwrap_or("-"),
            skill["skipped_version"].as_str().unwrap_or("-")
        ));
    }
    Ok((CliResult::from_data(&data), human))
}

/// 单技能引用参数。
#[derive(Args, Debug)]
pub struct RefArg {
    #[arg(value_name = "ref")]
    pub reference: String,
}
