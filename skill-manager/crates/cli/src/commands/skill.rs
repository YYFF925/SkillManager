//! `skill` 子命令：技能的查询与库内维护。
//!
//! 参数与输出遵循 `specs/002-skill-management/contracts/cli.md`；业务规则全部在 core，
//! 这里只做参数到核心调用的映射与结果表达。

use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;

use clap::{Args, Subcommand, ValueEnum};
use serde_json::{json, Value};
use skill_manager_core::deploy::DeploymentService;
use skill_manager_core::domain::{names_equal, Domain, SkillRef};
use skill_manager_core::manager::{
    self, list_overlays, read_overlay, read_translation, remove_overlay, remove_translation,
    write_overlay, write_translation, SupportDecl,
};
use skill_manager_core::vault::index::{
    AgentSupport, CategoryFilter, DeploymentView, ReadLang, SearchQuery, SkillEntry, SkillIndex,
    SkillSummary,
};
use skill_manager_core::vault::{self, manage};
use skill_manager_core::{
    AppContext, BatchOutcome, CliResult, CoreError, GlobalOptions, ItemError,
};

use super::deploy;

#[derive(Args, Debug)]
pub struct SkillCommand {
    #[command(subcommand)]
    pub action: SkillAction,
}

#[derive(Subcommand, Debug)]
pub enum SkillAction {
    /// 列出技能（组合筛选）。
    List(ListArgs),
    /// 技能详情：正文/资料/来源/文件树/库内路径/部署位置。
    Get(RefArgs),
    /// 读取正文（有译文默认中文）。
    Read(ReadArgs),
    /// 修改管理资料（注释、标签、支持声明）。
    Set(SetArgs),
    /// 标签治理：批量增删与全库重命名/删除。
    #[command(subcommand)]
    Tag(TagCommand),
    /// 分类治理：重命名一级分类目录。
    #[command(subcommand)]
    Category(CategoryCommand),
    /// 移动到一级分类（不改变技能名）。
    Move(MoveArgs),
    /// 删除库内技能并清理其受管部署（本地外改副本暂停）。
    Delete(DeleteArgs),
    /// 归档：卸载全部受管部署后移出正常技能库。
    #[command(subcommand)]
    Archive(ArchiveCommand),
    /// 中文译文维护。
    #[command(subcommand)]
    Translation(TranslationCommand),
    /// 覆盖层维护。
    #[command(subcommand)]
    Overlay(OverlayCommand),
    /// 收编 Hermes 档案独有技能到龙虾库并部署 default 档案。
    AdoptHermes(AdoptHermesArgs),
    /// 获取并列出来源仓库的技能候选（不入库；真实进度与可取消）。
    Preview(super::acquire::PreviewArgs),
    /// 从来源仓库选择技能部分入库。
    Import(super::acquire::ImportArgs),
    /// 收编本机 Agent 中未受管的技能并接管原部署。
    Adopt(super::acquire::AdoptArgs),
    /// 创建自建技能（不部署）。
    Create(super::acquire::CreateArgs),
    /// 跨管理域复制技能（不部署，两份独立维护）。
    Copy(super::acquire::CopyArgs),
    /// 检查上游变化（单技能/仓库/全库；只展示，不应用）。
    Check(super::acquire::CheckArgs),
    /// 显式单项采用上游更新。
    Update(super::acquire::UpdateArgs),
    /// 仓库批量更新。
    UpdateRepo(super::acquire::UpdateRepoArgs),
    /// 明确“不入库并记住”新增候选（随库共享）。
    RejectNew(super::acquire::RejectNewArgs),
    /// 查看已拒绝新增记录。
    Rejected(super::acquire::RejectedArgs),
    /// 跳过技能当前上游版本。
    Skip(super::acquire::SkipArgs),
    /// 恢复技能的上游更新提示。
    Unskip(super::acquire::RefArg),
    /// 查看已跳过版本。
    Skipped,
    /// 生成技能维护任务说明（单项/多选；复制给外部 Agent，不代表已执行）。
    Task(TaskArgs),
}

#[derive(Args, Debug)]
pub struct TaskArgs {
    /// 技能引用 `<domain>/<name>`（可重复；单项或多选）。
    #[arg(value_name = "ref", required = true)]
    pub references: Vec<String>,
    /// 用户要求（本次希望 Agent 做什么）。
    #[arg(long, value_name = "text")]
    pub requirement: Option<String>,
}

#[derive(Args, Debug)]
pub struct AdoptHermesArgs {
    /// Hermes 档案名（具名档案；default 的技能由 deploy 表达）。
    #[arg(value_name = "profile")]
    pub profile: String,
    /// 档案技能名（可重复）。
    #[arg(value_name = "name", required = true)]
    pub names: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// 管理域。
    #[arg(long, value_name = "domain")]
    pub domain: Option<String>,
    /// 文本搜索（名称、描述、注释、来源、分类、标签）。
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
    /// 预设筛选（须同时给 `--domain`）。
    #[arg(long, value_name = "id")]
    pub preset: Option<String>,
    /// 支持声明筛选（Agent key）。
    #[arg(long, value_name = "key")]
    pub agent: Option<String>,
    /// 部署目标筛选（按本机部署记录）。
    #[arg(long = "deployed-to", value_name = "target")]
    pub deployed_to: Option<String>,
    /// 只列出无标签技能。
    #[arg(long = "no-tag")]
    pub no_tag: bool,
}

#[derive(Args, Debug)]
pub struct RefArgs {
    /// 技能引用 `<domain>/<name>`。
    #[arg(value_name = "ref")]
    pub reference: String,
}

#[derive(Args, Debug)]
pub struct ReadArgs {
    #[arg(value_name = "ref")]
    pub reference: String,
    /// 阅读语言：有译文默认中文。
    #[arg(long, value_name = "zh|orig")]
    pub lang: Option<LangArg>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
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

#[derive(Args, Debug)]
pub struct SetArgs {
    #[arg(value_name = "ref")]
    pub reference: String,
    /// 中文短注释；传空字符串清除。
    #[arg(long, value_name = "text")]
    pub comment: Option<String>,
    /// 替换标签集合（逗号分隔）；传空字符串清空。
    #[arg(long, value_name = "csv")]
    pub tags: Option<String>,
    /// 支持的 Agent：`all`（删除限制键）/`none`（显式空集合）/逗号分隔名单。
    #[arg(long, value_name = "all|none|csv")]
    pub agents: Option<String>,
    /// 支持的 OS：`all`/`none`/逗号分隔名单。
    #[arg(long, value_name = "all|none|csv")]
    pub oses: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum CategoryCommand {
    /// 重命名一级分类（移动实际目录，该分类下全部技能一并移动，技能名不变）。
    Rename {
        /// 管理域。
        #[arg(long, value_name = "code|lobster")]
        domain: String,
        #[arg(value_name = "old")]
        old: String,
        #[arg(value_name = "new")]
        new: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum TagCommand {
    /// 批量添加标签（只增减目标标签）。
    Add(TagBatchArgs),
    /// 批量移除标签（只增减目标标签）。
    Detach(TagBatchArgs),
    /// 全库重命名标签（两域生效）。
    Rename {
        #[arg(value_name = "old")]
        old: String,
        #[arg(value_name = "new")]
        new: String,
    },
    /// 全库删除标签关联（两域生效，不删除技能）。
    Remove {
        #[arg(value_name = "tag")]
        tag: String,
    },
}

#[derive(Args, Debug)]
pub struct TagBatchArgs {
    #[arg(value_name = "ref", required = true)]
    pub references: Vec<String>,
    #[arg(long, value_name = "csv")]
    pub tags: String,
}

#[derive(Args, Debug)]
pub struct MoveArgs {
    #[arg(value_name = "ref")]
    pub reference: String,
    /// 目标一级分类。
    #[arg(long, value_name = "category", conflicts_with = "uncategorized")]
    pub category: Option<String>,
    /// 移动到管理域根下（未分类）。
    #[arg(long, conflicts_with = "category")]
    pub uncategorized: bool,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// 技能引用 `<domain>/<name>`（可重复）。
    #[arg(value_name = "ref", required = true)]
    pub references: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum ArchiveCommand {
    /// 归档技能：先卸载全部受管部署，成功后才移入归档目录。
    Add(DeleteArgs),
    /// 列出已归档技能。
    List {
        #[arg(long, value_name = "domain")]
        domain: Option<String>,
    },
    /// 还原到正常库，保留原分类与标签，不重新部署。
    Restore(DeleteArgs),
    /// 从归档永久删除。
    Delete(DeleteArgs),
}

#[derive(Subcommand, Debug)]
pub enum TranslationCommand {
    /// 读取译文本体（无译文如实为空）。
    Get(RefArgs),
    /// 按当前原文保存译文并更新时效依据。
    Set(TranslationSetArgs),
    /// 删除译文。
    Remove(RefArgs),
}

#[derive(Args, Debug)]
pub struct TranslationSetArgs {
    #[arg(value_name = "ref")]
    pub reference: String,
    /// 从文件读取译文。
    #[arg(long, value_name = "path", conflicts_with = "stdin")]
    pub file: Option<PathBuf>,
    /// 从标准输入读取译文。
    #[arg(long, conflicts_with = "file")]
    pub stdin: bool,
}

#[derive(Subcommand, Debug)]
pub enum OverlayCommand {
    /// 列出覆盖层文件。
    List(RefArgs),
    /// 读取一项覆盖层文件。
    Get(OverlayArgs),
    /// 写入一项覆盖层文件。
    Set(OverlaySetArgs),
    /// 删除一项覆盖层文件。
    Remove(OverlayArgs),
}

#[derive(Args, Debug)]
pub struct OverlayArgs {
    #[arg(value_name = "ref")]
    pub reference: String,
    /// 技能目录内的相对路径。
    #[arg(long, value_name = "relative-path")]
    pub path: String,
    /// 目标 Agent（省略=该维度通用）。
    #[arg(long, value_name = "key")]
    pub agent: Option<String>,
    /// 目标系统（省略=该维度通用）。
    #[arg(long, value_name = "os")]
    pub os: Option<String>,
}

#[derive(Args, Debug)]
pub struct OverlaySetArgs {
    #[arg(value_name = "ref")]
    pub reference: String,
    #[arg(long, value_name = "relative-path")]
    pub path: String,
    #[arg(long, value_name = "key")]
    pub agent: Option<String>,
    #[arg(long, value_name = "os")]
    pub os: Option<String>,
    /// 从文件读取内容。
    #[arg(long, value_name = "path", conflicts_with = "stdin")]
    pub file: Option<PathBuf>,
    /// 从标准输入读取内容。
    #[arg(long, conflicts_with = "file")]
    pub stdin: bool,
}

/// 分发 `skill` 子命令，返回结构化结果与默认（人类可读）文本。
pub fn run(
    command: SkillCommand,
    ctx: &mut AppContext,
    options: &GlobalOptions,
) -> (CliResult, String) {
    match dispatch(command.action, ctx, options) {
        Ok(pair) => pair,
        Err(err) => (CliResult::failed_error(err), String::new()),
    }
}

fn dispatch(
    action: SkillAction,
    ctx: &mut AppContext,
    _options: &GlobalOptions,
) -> Result<(CliResult, String)> {
    match action {
        SkillAction::List(args) => list(ctx, args),
        SkillAction::Get(args) => get(ctx, args),
        SkillAction::Read(args) => read(ctx, args),
        SkillAction::Set(args) => set(ctx, args),
        SkillAction::Tag(command) => tag(ctx, command),
        SkillAction::Category(command) => category(ctx, command),
        SkillAction::Move(args) => move_skill(ctx, args),
        SkillAction::Delete(args) => delete(ctx, args),
        SkillAction::Archive(command) => archive(ctx, command),
        SkillAction::Translation(command) => translation(ctx, command),
        SkillAction::Overlay(command) => overlay(ctx, command),
        SkillAction::AdoptHermes(args) => super::hermes::adopt(ctx, &args.profile, &args.names),
        SkillAction::Preview(args) => super::acquire::preview(ctx, args),
        SkillAction::Import(args) => super::acquire::import(ctx, args),
        SkillAction::Adopt(args) => super::acquire::adopt(ctx, args),
        SkillAction::Create(args) => super::acquire::create(ctx, args),
        SkillAction::Copy(args) => super::acquire::copy(ctx, args),
        SkillAction::Check(args) => super::acquire::check(ctx, args),
        SkillAction::Update(args) => super::acquire::update(ctx, args),
        SkillAction::UpdateRepo(args) => super::acquire::update_repo(ctx, args),
        SkillAction::RejectNew(args) => super::acquire::reject_new(ctx, args),
        SkillAction::Rejected(args) => super::acquire::rejected(ctx, args),
        SkillAction::Skip(args) => super::acquire::skip(ctx, args),
        SkillAction::Unskip(args) => super::acquire::unskip(ctx, args),
        SkillAction::Skipped => super::acquire::skipped(ctx),
        SkillAction::Task(args) => task(ctx, args),
    }
}

type Result<T> = std::result::Result<T, CoreError>;

/// `skill task`：生成可复制的维护任务说明（不改库、不部署、不上传）。
fn task(ctx: &AppContext, args: TaskArgs) -> Result<(CliResult, String)> {
    let root = vault_root(ctx)?;
    let index = load_index(ctx)?;
    let references = parse_refs(&args.references)?;
    let deployments = ctx.state().deployments()?;
    let text = vault::tasks::skill_task(
        &root,
        &index,
        &deployments,
        &references,
        args.requirement.as_deref(),
    )?;
    let data = json!({
        "skills": references.iter().map(|item| item.to_string()).collect::<Vec<_>>(),
        "task": text,
    });
    Ok((CliResult::from_data(&data), text))
}

// ---------------------------------------------------------------------------
// 共用
// ---------------------------------------------------------------------------

fn vault_root(ctx: &AppContext) -> Result<PathBuf> {
    Ok(ctx.vault()?.root().to_path_buf())
}

fn load_index(ctx: &AppContext) -> Result<SkillIndex> {
    SkillIndex::build(vault_root(ctx)?)
}

fn entry_summary(index: &SkillIndex, reference: &SkillRef) -> Result<SkillSummary> {
    index
        .get(reference)
        .map(SkillEntry::to_summary)
        .ok_or_else(|| CoreError::SkillNotFound {
            domain: reference.domain.to_string(),
            name: reference.name.clone(),
        })
}

fn parse_csv(raw: &str) -> Vec<String> {
    raw.split([',', '，'])
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn parse_support(raw: &str) -> Result<SupportDecl> {
    match raw.trim() {
        "" | "none" => Ok(SupportDecl::Empty),
        "all" => Ok(SupportDecl::All),
        other => Ok(SupportDecl::List(parse_csv(other))),
    }
}

fn join(values: &[String]) -> String {
    if values.is_empty() {
        "无".to_string()
    } else {
        values.join("、")
    }
}

fn support_text(value: &Option<Vec<String>>, label: &str) -> String {
    match value {
        None => format!("{label}：全部（未声明）"),
        Some(list) if list.is_empty() => format!("{label}：不支持任何目标（显式空集合）"),
        Some(list) => format!("{label}：{}", join(list)),
    }
}

fn read_stdin_bytes() -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    std::io::stdin()
        .read_to_end(&mut buffer)
        .map_err(|err| CoreError::other(format!("读取标准输入失败：{err}")))?;
    Ok(buffer)
}

fn read_file_bytes(path: &PathBuf) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|err| CoreError::io(path.clone(), err))
}

fn decode_utf8(bytes: Vec<u8>, source: &str) -> Result<String> {
    String::from_utf8(bytes)
        .map_err(|err| CoreError::other(format!("{source} 不是有效 UTF-8：{err}")))
}

fn parse_ref(raw: &str) -> Result<SkillRef> {
    SkillRef::parse(raw)
}

/// 由 CLI 参数构造组合筛选条件（供 `skill list` 与 `hermes list` 复用，分类联动一致）。
pub fn search_query(
    query: Option<String>,
    category: Option<String>,
    uncategorized: bool,
    tags: Vec<String>,
    no_tag: bool,
) -> SearchQuery {
    let category = if uncategorized {
        CategoryFilter::Uncategorized
    } else {
        match category {
            Some(name) => CategoryFilter::Named(name),
            None => CategoryFilter::All,
        }
    };
    SearchQuery {
        domain: None,
        query,
        category,
        tags,
        require_untagged: no_tag,
        agent: None,
        preset: None,
    }
}

fn parse_refs(raw: &[String]) -> Result<Vec<SkillRef>> {
    raw.iter().map(|item| parse_ref(item)).collect()
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

fn list(ctx: &AppContext, args: ListArgs) -> Result<(CliResult, String)> {
    let deployed_target = match &args.deployed_to {
        Some(raw) => Some(deploy::parse_target(raw)?),
        None => None,
    };

    let index = load_index(ctx)?;
    let domain = match &args.domain {
        Some(raw) => Some(Domain::from_str(raw)?),
        None => None,
    };
    // `--preset` 须同时给 `--domain`，避免跨域同 id 歧义（contracts/cli.md）。
    if args.preset.is_some() && domain.is_none() {
        return Err(CoreError::other(
            "--preset 须同时指定 --domain，避免两个管理域同 id 预设之间产生歧义",
        ));
    }
    let category = if args.uncategorized {
        CategoryFilter::Uncategorized
    } else {
        match &args.category {
            Some(name) => CategoryFilter::Named(name.clone()),
            None => CategoryFilter::All,
        }
    };
    let agent = match &args.agent {
        Some(key) => {
            let entry = ctx
                .catalog()
                .get(key)
                .ok_or_else(|| CoreError::InvalidTarget(format!("{key} 不在内置 Agent 名单中")))?;
            Some(AgentSupport {
                key: key.clone(),
                domain: entry.reference_domain,
            })
        }
        None => None,
    };

    let query = SearchQuery {
        domain,
        query: args.query.clone(),
        category: category.clone(),
        tags: args.tags.clone(),
        require_untagged: args.no_tag,
        agent,
        preset: args.preset.clone(),
    };
    let hits = index.search(&query);
    let mut summaries: Vec<SkillSummary> = hits.iter().map(|entry| entry.to_summary()).collect();
    if let Some(target) = &deployed_target {
        let records = ctx.state().deployments()?;
        let target = target.as_str();
        summaries.retain(|summary| {
            records.iter().any(|record| {
                record.target == target
                    && record.domain.as_str() == summary.domain
                    && names_equal(&record.name, &summary.name)
            })
        });
    }
    let tag_candidates = index.tag_candidates(domain, &category);

    let data = json!({
        "skills": summaries,
        "total": summaries.len(),
        "tag_candidates": tag_candidates,
        "categories": index.categories(domain),
    });

    let mut human = format!("共 {} 个技能", summaries.len());
    if !tag_candidates.is_empty() {
        human.push_str(&format!("（标签候选：{}）", join(&tag_candidates)));
    }
    for summary in &summaries {
        human.push('\n');
        human.push_str(&format!("- {}/{}", summary.domain, summary.name));
        if let Some(category) = &summary.category {
            human.push_str(&format!("（分类：{category}）"));
        }
        if let Some(comment) = &summary.comment {
            human.push_str(&format!("（{comment}）"));
        }
        if !summary.tags.is_empty() {
            human.push_str(&format!(" 标签：{}", join(&summary.tags)));
        }
        if !summary.presets.is_empty() {
            let ids: Vec<&str> = summary
                .presets
                .iter()
                .map(|preset| preset.id.as_str())
                .collect();
            human.push_str(&format!(" 预设：{}", ids.join("、")));
        }
        if summary.translation_stale {
            human.push_str(" [译文待更新]");
        }
        if let Some(error) = &summary.manager_error {
            human.push_str(&format!(" [管理资料读取失败：{error}]"));
        }
    }
    Ok((CliResult::from_data(&data), human))
}

// ---------------------------------------------------------------------------
// get / read
// ---------------------------------------------------------------------------

fn get(ctx: &AppContext, args: RefArgs) -> Result<(CliResult, String)> {
    let reference = parse_ref(&args.reference)?;
    let root = vault_root(ctx)?;
    let index = SkillIndex::build(&root)?;
    let entry = index
        .get(&reference)
        .ok_or_else(|| CoreError::SkillNotFound {
            domain: reference.domain.to_string(),
            name: reference.name.clone(),
        })?;

    let detail = vault::index::SkillDetail::load(entry, &root)?;
    let records = ctx
        .state()
        .deployments_for(reference.domain, &reference.name)?;
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
    let detail = detail.with_deployments(deployments, note);
    let data = serde_json::to_value(&detail)
        .map_err(|err| CoreError::other(format!("序列化技能详情失败：{err}")))?;

    let summary = &detail.skill;
    let mut human = format!("{}/{}", summary.domain, summary.name);
    if let Some(category) = &summary.category {
        human.push_str(&format!("（分类：{category}）"));
    }
    human.push('\n');
    human.push_str(&format!(
        "  注释：{}\n",
        summary.comment.as_deref().unwrap_or("无")
    ));
    human.push_str(&format!(
        "  描述：{}\n",
        summary.description.as_deref().unwrap_or("无")
    ));
    human.push_str(&format!("  标签：{}\n", join(&summary.tags)));
    human.push_str(&format!(
        "  {}\n",
        support_text(&summary.agents, "支持的 Agent")
    ));
    human.push_str(&format!(
        "  {}\n",
        support_text(&summary.oses, "支持的系统")
    ));
    match &summary.source {
        Some(source) => human.push_str(&format!(
            "  来源：{}（{}） {}\n",
            source.repo,
            source.reference.as_deref().unwrap_or("远端默认分支"),
            source.subpath
        )),
        None => human.push_str("  来源：无（自建技能）\n"),
    }
    human.push_str(&format!(
        "  译文：{}\n",
        if !summary.has_translation {
            "无".to_string()
        } else if summary.translation_stale {
            "有（待更新：原文已变化）".to_string()
        } else {
            "有（最新）".to_string()
        }
    ));
    human.push_str(&format!("  库内路径：{}\n", summary.path));
    human.push_str(&format!(
        "  文件：{} 项（含 .manager）\n",
        count_nodes(&detail.files)
    ));
    human.push_str(&format!("  部署位置：{}", detail.deployments_note));

    Ok((CliResult::from_data(&data), human))
}

fn count_nodes(nodes: &[vault::FileNode]) -> usize {
    nodes
        .iter()
        .map(|node| 1 + count_nodes(&node.children))
        .sum()
}

fn read(ctx: &AppContext, args: ReadArgs) -> Result<(CliResult, String)> {
    let reference = parse_ref(&args.reference)?;
    let index = load_index(ctx)?;
    let entry = index
        .get(&reference)
        .ok_or_else(|| CoreError::SkillNotFound {
            domain: reference.domain.to_string(),
            name: reference.name.clone(),
        })?;

    let body = vault::index::read_body(entry, args.lang.map(ReadLang::from))?;
    let data = serde_json::to_value(&body)
        .map_err(|err| CoreError::other(format!("序列化正文失败：{err}")))?;

    let mut human = format!("{}（{}）", body.skill, body.lang);
    if body.translation_stale {
        human.push_str("  译文待更新：原文已变化，当前显示的是旧译文");
    }
    if body.lang == "orig" && args.lang.is_some() {
        human.push_str("  无译文，已显示原文");
    }
    human.push_str("\n\n");
    human.push_str(&body.text);
    Ok((CliResult::from_data(&data), human))
}

// ---------------------------------------------------------------------------
// set
// ---------------------------------------------------------------------------

fn set(ctx: &AppContext, args: SetArgs) -> Result<(CliResult, String)> {
    let reference = parse_ref(&args.reference)?;
    let root = vault_root(ctx)?;

    let update = manage::MetaUpdate {
        comment: args.comment.as_ref().map(|text| {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }),
        tags: args.tags.as_ref().map(|raw| parse_csv(raw)),
        agents: match &args.agents {
            Some(raw) => Some(parse_support(raw)?),
            None => None,
        },
        oses: match &args.oses {
            Some(raw) => Some(parse_support(raw)?),
            None => None,
        },
    };
    manage::update_meta(&root, &reference, &update)?;

    let index = SkillIndex::build(&root)?;
    let summary = entry_summary(&index, &reference)?;
    let data = json!({ "skill": summary });

    let mut human = format!("已更新 {reference}");
    human.push_str(&format!(
        "\n  注释：{}",
        summary.comment.as_deref().unwrap_or("无")
    ));
    human.push_str(&format!("\n  标签：{}", join(&summary.tags)));
    human.push_str(&format!(
        "\n  {}",
        support_text(&summary.agents, "支持的 Agent")
    ));
    human.push_str(&format!(
        "\n  {}",
        support_text(&summary.oses, "支持的系统")
    ));
    Ok((CliResult::from_data(&data), human))
}

// ---------------------------------------------------------------------------
// tag
// ---------------------------------------------------------------------------

fn category(ctx: &AppContext, command: CategoryCommand) -> Result<(CliResult, String)> {
    let root = vault_root(ctx)?;
    match command {
        CategoryCommand::Rename { domain, old, new } => {
            let domain = Domain::from_str(&domain)?;
            let affected = manage::rename_category(&root, domain, &old, &new)?;
            let data = json!({
                "domain": domain.as_str(),
                "from": old,
                "to": new,
                "affected": affected,
            });
            let human = format!(
                "已将分类「{old}」重命名为「{new}」，涉及 {affected} 个技能（{}）",
                domain.label()
            );
            Ok((CliResult::from_data(&data), human))
        }
    }
}

fn tag(ctx: &AppContext, command: TagCommand) -> Result<(CliResult, String)> {
    let root = vault_root(ctx)?;
    match command {
        TagCommand::Add(args) => tag_batch(&root, args, true),
        TagCommand::Detach(args) => tag_batch(&root, args, false),
        TagCommand::Rename { old, new } => {
            let affected = manage::rename_tag(&root, &old, &new)?;
            let data = json!({ "from": old, "to": new, "affected": affected });
            let human = format!("已将标签「{old}」重命名为「{new}」，涉及 {affected} 个技能");
            Ok((CliResult::from_data(&data), human))
        }
        TagCommand::Remove { tag } => {
            let affected = manage::remove_tag(&root, &tag)?;
            let data = json!({ "tag": tag, "affected": affected });
            let human =
                format!("已解除标签「{tag}」的关联，涉及 {affected} 个技能（技能未被删除）");
            Ok((CliResult::from_data(&data), human))
        }
    }
}

fn tag_batch(root: &PathBuf, args: TagBatchArgs, add: bool) -> Result<(CliResult, String)> {
    let references = parse_refs(&args.references)?;
    let tags = parse_csv(&args.tags);
    let items = if add {
        manage::add_tags_many(root, &references, &tags)
    } else {
        manage::detach_tags_many(root, &references, &tags)
    };

    let mut batch = BatchOutcome::default();
    for item in items {
        if item.ok {
            batch.push_ok(Value::String(item.skill));
        } else {
            let message = item.message.unwrap_or_else(|| "未知原因".to_string());
            batch.push_err(ItemError::new(message).skill(item.skill));
        }
    }

    let human = if batch.failed.is_empty() {
        format!(
            "已{}标签 {}",
            if add { "添加" } else { "移除" },
            join(&tags)
        )
    } else {
        format!(
            "已{}标签 {}（{} 项未完成）",
            if add { "添加" } else { "移除" },
            join(&tags),
            batch.failed.len()
        )
    };
    Ok((batch.into_result("skills"), human))
}

// ---------------------------------------------------------------------------
// delete
// ---------------------------------------------------------------------------

fn delete(ctx: &AppContext, args: DeleteArgs) -> Result<(CliResult, String)> {
    let references = parse_refs(&args.references)?;
    deploy::delete_skills(ctx, &references)
}

fn archive(ctx: &AppContext, command: ArchiveCommand) -> Result<(CliResult, String)> {
    match command {
        ArchiveCommand::List { domain } => archive_list(ctx, domain.as_deref()),
        ArchiveCommand::Add(args) => archive_add(ctx, &args.references),
        ArchiveCommand::Restore(args) => archive_restore(ctx, &args.references),
        ArchiveCommand::Delete(args) => archive_delete(ctx, &args.references),
    }
}

fn archive_list(ctx: &AppContext, domain: Option<&str>) -> Result<(CliResult, String)> {
    let domain = match domain {
        Some(raw) => Some(Domain::from_str(raw)?),
        None => None,
    };
    let items = vault::list_archived(&vault_root(ctx)?, domain)?;
    let data = json!({ "skills": items, "total": items.len() });
    let human = if items.is_empty() {
        "归档为空".to_string()
    } else {
        let mut text = format!("已归档 {} 项", items.len());
        for item in &items {
            let category = item.category.as_deref().unwrap_or("未分类");
            text.push_str(&format!(
                "\n- {}/{} · {category} · {}",
                item.domain.as_str(),
                item.name,
                item.vault_rel_dir
            ));
        }
        text
    };
    Ok((CliResult::from_data(&data), human))
}

fn with_deploy<T>(
    ctx: &AppContext,
    run: impl FnOnce(&DeploymentService<'_>) -> Result<T>,
) -> Result<T> {
    let root = vault_root(ctx)?;
    let resolver = ctx.target_resolver()?;
    let service = DeploymentService::new(root, &resolver, ctx.state());
    run(&service)
}

fn archive_add(ctx: &AppContext, raw: &[String]) -> Result<(CliResult, String)> {
    let references = parse_refs(raw)?;
    let outcome = with_deploy(ctx, |service| service.archive_skills(&references))?;
    let data = json!({
        "archived": outcome.archived,
        "cleanup": outcome.cleanup,
    });
    let human = format!(
        "已归档 {} 项，卸载副本 {} 项",
        outcome.archived.len(),
        outcome.cleanup.len()
    );
    archive_batch_result(data, outcome.failures, outcome.archived.is_empty(), human)
}

fn archive_restore(ctx: &AppContext, raw: &[String]) -> Result<(CliResult, String)> {
    let references = parse_refs(raw)?;
    let outcome = with_deploy(ctx, |service| service.restore_archived(&references))?;
    let data = json!({ "restored": outcome.restored });
    let human = format!("已还原 {} 项（未重新部署）", outcome.restored.len());
    archive_batch_result(data, outcome.failures, outcome.restored.is_empty(), human)
}

fn archive_delete(ctx: &AppContext, raw: &[String]) -> Result<(CliResult, String)> {
    let references = parse_refs(raw)?;
    let outcome = with_deploy(ctx, |service| service.delete_archived(&references))?;
    let data = json!({ "deleted": outcome.deleted });
    let human = format!("已从归档删除 {} 项", outcome.deleted.len());
    archive_batch_result(data, outcome.failures, outcome.deleted.is_empty(), human)
}

fn archive_batch_result(
    data: Value,
    errors: Vec<ItemError>,
    empty_ok: bool,
    human: String,
) -> Result<(CliResult, String)> {
    let result = if errors.is_empty() {
        CliResult::success(data)
    } else if empty_ok {
        CliResult::failed(errors)
    } else {
        CliResult::partial(data, errors)
    };
    Ok((result, human))
}

// ---------------------------------------------------------------------------
// move
// ---------------------------------------------------------------------------

fn move_skill(ctx: &AppContext, args: MoveArgs) -> Result<(CliResult, String)> {
    let reference = parse_ref(&args.reference)?;
    let category = match (&args.category, args.uncategorized) {
        (Some(name), false) => Some(name.as_str()),
        (None, true) => None,
        (None, false) => {
            return Err(CoreError::other(
                "必须指定 --category <分类> 或 --uncategorized",
            ));
        }
        (Some(_), true) => {
            return Err(CoreError::other(
                "--category 与 --uncategorized 不能同时使用",
            ));
        }
    };

    let root = vault_root(ctx)?;
    manage::move_to_category(&root, &reference, category)?;

    let index = SkillIndex::build(&root)?;
    let summary = entry_summary(&index, &reference)?;
    let data = json!({ "skill": summary });
    let human = format!(
        "已移动 {reference} 到{}",
        summary
            .category
            .as_ref()
            .map(|category| format!("分类「{category}」"))
            .unwrap_or_else(|| "管理域根下（未分类）".to_string())
    );
    Ok((CliResult::from_data(&data), human))
}

// ---------------------------------------------------------------------------
// translation
// ---------------------------------------------------------------------------

fn translation(ctx: &AppContext, command: TranslationCommand) -> Result<(CliResult, String)> {
    let root = vault_root(ctx)?;
    match command {
        TranslationCommand::Get(args) => {
            let reference = parse_ref(&args.reference)?;
            let dir = manage::locate(&root, &reference)?;
            let text = read_translation(&dir)?;
            let index = SkillIndex::build(&root)?;
            let summary = entry_summary(&index, &reference)?;

            let data = json!({
                "skill": reference.to_string(),
                "exists": text.is_some(),
                "text": text,
                "translation_stale": summary.translation_stale,
            });
            let human = match &data["text"] {
                Value::String(text) => format!("{reference} 的中文译文（最新）：\n\n{text}"),
                _ => format!("{reference} 没有中文译文"),
            };
            Ok((CliResult::from_data(&data), human))
        }
        TranslationCommand::Set(args) => {
            let reference = parse_ref(&args.reference)?;
            let content = match (&args.file, args.stdin) {
                (Some(path), false) => {
                    decode_utf8(read_file_bytes(path)?, &path.display().to_string())?
                }
                (None, true) => decode_utf8(read_stdin_bytes()?, "标准输入")?,
                _ => {
                    return Err(CoreError::other("必须指定 --file <path> 或 --stdin 之一"));
                }
            };
            let dir = manage::locate(&root, &reference)?;
            write_translation(&dir, &content)?;

            let index = SkillIndex::build(&root)?;
            let summary = entry_summary(&index, &reference)?;
            let data = json!({
                "skill": reference.to_string(),
                "exists": true,
                "translation_stale": summary.translation_stale,
            });
            let human = format!("已按当前原文保存 {reference} 的中文译文");
            Ok((CliResult::from_data(&data), human))
        }
        TranslationCommand::Remove(args) => {
            let reference = parse_ref(&args.reference)?;
            let dir = manage::locate(&root, &reference)?;
            remove_translation(&dir)?;
            let data = json!({
                "skill": reference.to_string(),
                "exists": false,
            });
            let human = format!("已删除 {reference} 的中文译文");
            Ok((CliResult::from_data(&data), human))
        }
    }
}

// ---------------------------------------------------------------------------
// overlay
// ---------------------------------------------------------------------------

fn overlay(ctx: &AppContext, command: OverlayCommand) -> Result<(CliResult, String)> {
    let root = vault_root(ctx)?;
    match command {
        OverlayCommand::List(args) => {
            let reference = parse_ref(&args.reference)?;
            let dir = manage::locate(&root, &reference)?;
            let entries = list_overlays(&dir)?;
            let data = json!({
                "skill": reference.to_string(),
                "overlays": entries
                    .iter()
                    .map(|entry| json!({
                        "agent": entry.agent,
                        "os": entry.os,
                        "path": entry.rel,
                        "size": entry.size,
                    }))
                    .collect::<Vec<_>>(),
            });
            let mut human = format!("{reference} 的覆盖层 {} 项", entries.len());
            for entry in &entries {
                human.push_str(&format!("\n- {}", overlay_label(entry)));
            }
            Ok((CliResult::from_data(&data), human))
        }
        OverlayCommand::Get(args) => {
            let reference = parse_ref(&args.reference)?;
            let dir = manage::locate(&root, &reference)?;
            let bytes = read_overlay(&dir, args.agent.as_deref(), args.os.as_deref(), &args.path)?;
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
            let data = json!({
                "skill": reference.to_string(),
                "agent": args.agent,
                "os": args.os,
                "path": args.path,
                "exists": text.is_some() || binary,
                "binary": binary,
                "size": size,
                "text": text,
            });
            let human = match &data["text"] {
                Value::String(text) => format!(
                    "{}（{}）\n\n{text}",
                    args.path,
                    dimension_label(&args.agent, &args.os)
                ),
                _ if binary => format!("{} 为二进制文件（{size} 字节）", args.path),
                _ => format!("{} 没有该覆盖层文件", args.path),
            };
            Ok((CliResult::from_data(&data), human))
        }
        OverlayCommand::Set(args) => {
            let reference = parse_ref(&args.reference)?;
            let bytes = match (&args.file, args.stdin) {
                (Some(path), false) => read_file_bytes(path)?,
                (None, true) => read_stdin_bytes()?,
                _ => {
                    return Err(CoreError::other("必须指定 --file <path> 或 --stdin 之一"));
                }
            };
            let dir = manage::locate(&root, &reference)?;
            write_overlay(
                &dir,
                args.agent.as_deref(),
                args.os.as_deref(),
                &args.path,
                &bytes,
            )?;
            let data = json!({
                "skill": reference.to_string(),
                "agent": args.agent,
                "os": args.os,
                "path": args.path,
                "written": true,
            });
            let human = format!(
                "已写入 {reference} 的覆盖层 {}（{}）",
                args.path,
                dimension_label(&args.agent, &args.os)
            );
            Ok((CliResult::from_data(&data), human))
        }
        OverlayCommand::Remove(args) => {
            let reference = parse_ref(&args.reference)?;
            let dir = manage::locate(&root, &reference)?;
            let removed =
                remove_overlay(&dir, args.agent.as_deref(), args.os.as_deref(), &args.path)?;
            let data = json!({
                "skill": reference.to_string(),
                "agent": args.agent,
                "os": args.os,
                "path": args.path,
                "removed": removed,
            });
            let human = if removed {
                format!("已删除 {reference} 的覆盖层 {}", args.path)
            } else {
                format!("{reference} 没有该覆盖层文件（{}）", args.path)
            };
            Ok((CliResult::from_data(&data), human))
        }
    }
}

fn overlay_label(entry: &manager::OverlayEntry) -> String {
    format!(
        "{}（{}）",
        entry.rel,
        dimension_label(&entry.agent, &entry.os)
    )
}

fn dimension_label(agent: &Option<String>, os: &Option<String>) -> String {
    match (agent, os) {
        (Some(agent), Some(os)) => format!("{agent} · {os}"),
        (Some(agent), None) => format!("{agent} · 通用系统"),
        (None, Some(os)) => format!("通用 Agent · {os}"),
        (None, None) => "通用".to_string(),
    }
}
