//! `tool` 子命令：工具资料、版本检查、安装事实回写与维护任务（T079）。
//!
//! 参数与输出遵循 `specs/002-skill-management/contracts/cli.md`；业务规则全部在 core。
//! 登记、资料修改与安装结果回写都从这里可达；生成任务不安装、不执行；所有共享资料变更
//! 沿用“Agent 修改后询问、明确同意才上传”的既有约定，本命令不自动上传。

use std::io::Read;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::json;
use skill_manager_core::domain::SkillRef;
use skill_manager_core::state::ToolInstallRecord;
use skill_manager_core::toolcenter::files;
use skill_manager_core::toolcenter::{self, ToolQuery, ToolStatusFilter, ToolTaskAction};
use skill_manager_core::{AppContext, CliResult, CoreError, GlobalOptions, SkillIndex};

#[derive(Args, Debug)]
pub struct ToolCommand {
    #[command(subcommand)]
    pub action: ToolAction,
}

#[derive(Subcommand, Debug)]
pub enum ToolAction {
    /// 列出工具（含未安装条目）；支持文本、一级分类与状态筛选。
    List(ToolListArgs),
    /// 读取一个工具的详情（简介、本机事实、检查结果、指南、配套 Skill）。
    Get(ToolGetArgs),
    /// 执行登记的版本检查命令（单项或全部）；只写检查结果。
    Check(ToolCheckArgs),
    /// 登记新工具（--dir 提供 tool.yaml/guide.md 及检查脚本，或 --stdin）。
    Register(ToolWriteArgs),
    /// 修改已有工具的共享资料。
    Set(ToolSetArgs),
    /// 回写本机安装事实（只由 Agent 核实后调用）；回写后重置检查结果。
    SetInstall(SetInstallArgs),
    /// 生成工具维护任务说明（复制给外部 Agent；不代表已执行）。
    Task(ToolTaskArgs),
    /// 添加一个配套 Skill 关联（来源资料随 tool set 维护）。
    SkillLink(SkillLinkArgs),
}

#[derive(Args, Debug)]
pub struct ToolListArgs {
    /// 文本搜索（名称、短摘要、标签、分类）。
    #[arg(long, value_name = "text")]
    pub query: Option<String>,
    /// 一级分类。
    #[arg(long, value_name = "category")]
    pub category: Option<String>,
    /// 状态筛选：installed|not-installed|updatable|latest|failed|unchecked|no-command|no-guide。
    #[arg(long, value_name = "status")]
    pub status: Option<String>,
}

#[derive(Args, Debug)]
pub struct ToolGetArgs {
    /// 工具 slug（`tools/<slug>` 目录名）。
    #[arg(value_name = "slug")]
    pub slug: String,
}

#[derive(Args, Debug)]
pub struct ToolCheckArgs {
    /// 单个工具 slug。
    #[arg(value_name = "slug")]
    pub slug: Option<String>,
    /// 检查全部已登记工具。
    #[arg(long)]
    pub all: bool,
}

#[derive(Args, Debug)]
pub struct ToolWriteArgs {
    /// 提供 `tool.yaml`、`guide.md` 及所需检查脚本的目录。
    #[arg(long, value_name = "path", conflicts_with = "stdin")]
    pub dir: Option<PathBuf>,
    /// 从标准输入读取 `{"tool": {...}, "guide": "..."}`。
    #[arg(long, conflicts_with = "dir")]
    pub stdin: bool,
}

#[derive(Args, Debug)]
pub struct ToolSetArgs {
    #[arg(value_name = "slug")]
    pub slug: String,
    #[arg(long, value_name = "path", conflicts_with = "stdin")]
    pub dir: Option<PathBuf>,
    #[arg(long, conflicts_with = "dir")]
    pub stdin: bool,
}

#[derive(Args, Debug)]
pub struct SetInstallArgs {
    #[arg(value_name = "slug")]
    pub slug: String,
    /// Agent 核实后的真实本机版本；传空字符串表示该工具已不在本机（清除安装记录）。
    #[arg(long, value_name = "v")]
    pub version: String,
    /// 安装方式。
    #[arg(long, value_name = "text")]
    pub method: Option<String>,
    /// 安装位置。
    #[arg(long, value_name = "path")]
    pub path: Option<String>,
    /// 配置位置。
    #[arg(long = "config-path", value_name = "path")]
    pub config_path: Option<String>,
}

#[derive(Args, Debug)]
pub struct ToolTaskArgs {
    /// 已登记工具 slug；`--action register` 允许省略。
    #[arg(value_name = "slug")]
    pub slug: Option<String>,
    /// 任务动作：register|edit|install|update|guide。
    #[arg(long, value_name = "action", required = true)]
    pub action: String,
    /// 用户要求。
    #[arg(long, value_name = "text")]
    pub requirement: Option<String>,
}

#[derive(Args, Debug)]
pub struct SkillLinkArgs {
    #[arg(value_name = "slug")]
    pub slug: String,
    /// 配套技能引用 `<domain>/<name>`。
    #[arg(value_name = "ref")]
    pub reference: String,
}

type Result<T> = std::result::Result<T, CoreError>;

/// 分发 `tool` 子命令。
pub fn run(
    command: ToolCommand,
    ctx: &mut AppContext,
    _options: &GlobalOptions,
) -> (CliResult, String) {
    match dispatch(command.action, ctx) {
        Ok(pair) => pair,
        Err(err) => (CliResult::failed_error(err), String::new()),
    }
}

fn dispatch(action: ToolAction, ctx: &AppContext) -> Result<(CliResult, String)> {
    match action {
        ToolAction::List(args) => list(ctx, args),
        ToolAction::Get(args) => get(ctx, args),
        ToolAction::Check(args) => check(ctx, args),
        ToolAction::Register(args) => register(ctx, args),
        ToolAction::Set(args) => set(ctx, args),
        ToolAction::SetInstall(args) => set_install(ctx, args),
        ToolAction::Task(args) => task(ctx, args),
        ToolAction::SkillLink(args) => skill_link(ctx, args),
    }
}

fn root(ctx: &AppContext) -> Result<PathBuf> {
    Ok(ctx.vault()?.root().to_path_buf())
}

fn read_stdin() -> Result<String> {
    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|err| CoreError::other(format!("读取标准输入失败：{err}")))?;
    Ok(buffer)
}

fn parse_stdin_payload() -> Result<serde_json::Value> {
    let text = read_stdin()?;
    serde_json::from_str(&text)
        .map_err(|err| CoreError::other(format!("--stdin 需要 JSON 对象：{err}")))
}

/// 登记时由工具名称派生 slug。
fn slug_of_payload(payload: &serde_json::Value) -> Result<String> {
    let name = payload
        .get("tool")
        .and_then(|tool| tool.get("name"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| CoreError::other("--stdin 登记工具需要 tool.name"))?;
    Ok(files::slugify(name))
}

/// 登记时由目录内 `tool.yaml` 派生 slug。
fn slug_of_dir(dir: &std::path::Path) -> Result<String> {
    let path = dir.join(files::TOOL_FILE);
    let text = std::fs::read_to_string(&path).map_err(|err| CoreError::io(path.clone(), err))?;
    let file = files::ToolFile::from_yaml(&text, &path)?;
    let meta = file.meta()?;
    meta.validate()?;
    Ok(files::slugify(&meta.name))
}

fn ensure_not_registered(root: &std::path::Path, slug: &str) -> Result<()> {
    if files::tool_path(root, slug).is_file() {
        return Err(CoreError::other(format!(
            "工具已登记：{slug}（修改共享资料请用 tool set）"
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 各动作
// ---------------------------------------------------------------------------

fn list(ctx: &AppContext, args: ToolListArgs) -> Result<(CliResult, String)> {
    let root = root(ctx)?;
    let status = match &args.status {
        Some(raw) => Some(ToolStatusFilter::parse(raw)?),
        None => None,
    };
    let query = ToolQuery {
        query: args.query.clone(),
        category: args.category.clone(),
        status,
    };
    let entries = toolcenter::catalog::list(&root, ctx.state(), &query)?;
    let data = json!({ "tools": entries, "total": entries.len() });
    let mut human = format!("工具 {} 个", entries.len());
    for entry in &entries {
        let check_text = match entry.check.status.as_str() {
            "checked" => match (
                entry.comparison.as_deref(),
                entry.check.latest_version.as_deref(),
                entry.install.as_ref(),
            ) {
                (Some("latest"), _, _) => "已检查：本机已是检查所得最新版本".to_string(),
                (Some("updatable"), _, _) => "已检查：有更新".to_string(),
                (Some(_), _, _) => "已检查：无法判定是否可更新".to_string(),
                // 未安装工具只展示上游版本，不显示“本机最新/可更新”。
                (None, Some(latest), None) => {
                    format!("已检查：本机未安装，仅获知上游版本 {latest}")
                }
                _ => "已检查".to_string(),
            },
            "failed" => "检查失败".to_string(),
            "checking" => "检查中".to_string(),
            "no_command" => "未登记检查命令".to_string(),
            _ => "未检查".to_string(),
        };
        human.push_str(&format!(
            "\n- {}（{}）{}｜{}",
            entry.name,
            entry.slug,
            match &entry.install {
                Some(install) => format!("已安装 {}", install.version),
                None => "未安装".to_string(),
            },
            check_text
        ));
        if let Some(error) = &entry.error {
            human.push_str(&format!("｜资料异常：{error}"));
        }
    }
    Ok((CliResult::from_data(&data), human))
}

fn get(ctx: &AppContext, args: ToolGetArgs) -> Result<(CliResult, String)> {
    let root = root(ctx)?;
    let entry = toolcenter::catalog::get(&root, ctx.state(), &args.slug)?;
    let mut human = format!("{}（{}）", entry.name, entry.slug);
    human.push_str(&format!(
        "\n本机安装：{}",
        match &entry.install {
            Some(install) => format!(
                "{}（方式：{}；位置：{}；配置：{}）",
                install.version,
                install.method.as_deref().unwrap_or("未记录"),
                install.path.as_deref().unwrap_or("未记录"),
                install.config_path.as_deref().unwrap_or("未记录"),
            ),
            None => "未安装".to_string(),
        }
    ));
    human.push_str(&format!(
        "\n检查状态：{}{}",
        entry.check.status,
        entry
            .check
            .latest_version
            .as_ref()
            .map(|latest| format!("（上游最新版本：{latest}）"))
            .unwrap_or_default()
    ));
    if let Some(reason) = &entry.comparison_reason {
        human.push_str(&format!("\n版本对比：无法判定——{reason}"));
    } else if let Some(comparison) = &entry.comparison {
        human.push_str(&format!("\n版本对比：{comparison}"));
    }
    human.push_str(&format!("\n资料目录：{}", entry.dir));
    if let Some(intro) = &entry.intro {
        human.push_str(&format!("\n工具简介：\n{intro}"));
    }
    if entry.skills.is_empty() {
        human.push_str("\n配套 Skill：无");
    } else {
        human.push_str("\n配套 Skill：");
        for skill in &entry.skills {
            human.push_str(&format!(
                "\n- {}/{}（{}）",
                skill.domain,
                skill.name,
                if skill.in_vault {
                    "已入库"
                } else {
                    "未入库"
                }
            ));
        }
    }
    match &entry.guide {
        Some(guide) => human.push_str(&format!("\n维护指南：\n{guide}")),
        None => human.push_str("\n维护指南：待补（尚未编写）"),
    }
    Ok((CliResult::from_data(&entry), human))
}

fn check(ctx: &AppContext, args: ToolCheckArgs) -> Result<(CliResult, String)> {
    let root = root(ctx)?;
    let outcomes = match (&args.slug, args.all) {
        (Some(slug), false) => vec![toolcenter::check_tool(&root, ctx.state(), slug)?],
        (None, true) => toolcenter::check_all(&root, ctx.state())?,
        _ => {
            return Err(CoreError::other(
                "tool check 须给出一个 slug 或使用 --all（二者恰好一个）",
            ));
        }
    };

    let data = json!({
        "checked": outcomes.len(),
        "tools": outcomes,
    });
    let mut human = format!("已检查 {} 个工具（只保存检查结果，不安装、不更新）", outcomes.len());
    for outcome in &outcomes {
        human.push_str(&format!(
            "\n- {}（{}）{}",
            if outcome.name.is_empty() {
                outcome.slug.clone()
            } else {
                outcome.name.clone()
            },
            outcome.slug,
            match outcome.status.as_str() {
                "checked" => match (&outcome.installed_version, &outcome.latest_version) {
                    (Some(installed), Some(latest)) => match outcome.comparison.as_ref().map(|item| item.kind()) {
                        Some("latest") => format!("已安装 {installed}；上游 {latest}：已是最新"),
                        Some("updatable") => format!("已安装 {installed}；上游 {latest}：可更新"),
                        Some(_) => format!(
                            "已安装 {installed}；上游 {latest}：{}",
                            outcome
                                .comparison
                                .as_ref()
                                .and_then(|item| item.reason())
                                .unwrap_or("无法判定")
                        ),
                        None => format!("已安装 {installed}；上游 {latest}：无法判定"),
                    },
                    (None, Some(latest)) => format!("未安装；上游最新版本 {latest}（不等于已安装）"),
                    _ => "已检查".to_string(),
                },
                "failed" => format!(
                    "检查失败：{}",
                    outcome.output.as_deref().unwrap_or("未记录原因")
                ),
                "no_command" => "未登记检查命令".to_string(),
                other => other.to_string(),
            }
        ));
    }
    Ok((CliResult::from_data(&data), human))
}

fn register(ctx: &AppContext, args: ToolWriteArgs) -> Result<(CliResult, String)> {
    let root = root(ctx)?;
    let (slug, report) = match (&args.dir, args.stdin) {
        (Some(dir), false) => {
            let slug = slug_of_dir(dir)?;
            ensure_not_registered(&root, &slug)?;
            let report = files::write_from_dir(&root, &slug, dir)?;
            (slug, report)
        }
        (None, true) => {
            let payload = parse_stdin_payload()?;
            let slug = slug_of_payload(&payload)?;
            ensure_not_registered(&root, &slug)?;
            let report = files::write_from_stdin(&root, &slug, &payload)?;
            (slug, report)
        }
        _ => {
            return Err(CoreError::other(
                "tool register 须给出 --dir <path> 或 --stdin 之一",
            ));
        }
    };
    Ok((
        CliResult::from_data(&report),
        format!(
            "已登记工具 {slug}（共享资料写入 {}；属于库修改，请按既有约定在用户明确同意后再上传）",
            report.dir
        ),
    ))
}

fn set(ctx: &AppContext, args: ToolSetArgs) -> Result<(CliResult, String)> {
    let root = root(ctx)?;
    files::ToolFile::load(&root, &args.slug)?;
    let report = match (&args.dir, args.stdin) {
        (Some(dir), false) => files::write_from_dir(&root, &args.slug, dir)?,
        (None, true) => {
            let payload = parse_stdin_payload()?;
            files::write_from_stdin(&root, &args.slug, &payload)?
        }
        _ => {
            return Err(CoreError::other(
                "tool set 须给出 --dir <path> 或 --stdin 之一",
            ));
        }
    };
    Ok((
        CliResult::from_data(&report),
        format!(
            "已更新工具共享资料 {}（不安装、不检查；属于库修改，询问后再上传）",
            args.slug
        ),
    ))
}

fn set_install(ctx: &AppContext, args: SetInstallArgs) -> Result<(CliResult, String)> {
    let root = root(ctx)?;
    files::ToolFile::load(&root, &args.slug)?;
    let version = args.version.trim();

    if version.is_empty() {
        ctx.state().clear_tool_install(&args.slug)?;
        return Ok((
            CliResult::success(json!({
                "tool": args.slug,
                "installed": false,
                "check_reset": true,
            })),
            format!(
                "已清除 {} 的本机安装记录（本机已不存在该工具）；检查结果已重置为未检查",
                args.slug
            ),
        ));
    }

    ctx.state().set_tool_install(&ToolInstallRecord {
        tool_slug: args.slug.clone(),
        version: version.to_string(),
        method: args.method.clone(),
        path: args.path.clone(),
        config_path: args.config_path.clone(),
    })?;
    Ok((
        CliResult::success(json!({
            "tool": args.slug,
            "installed": true,
            "version": version,
            "check_reset": true,
        })),
        format!(
            "已回写 {} 的本机安装版本 {version}；检查结果已重置为未检查（该版本来自 Agent 核实，不是软件安装结果）",
            args.slug
        ),
    ))
}

fn task(ctx: &AppContext, args: ToolTaskArgs) -> Result<(CliResult, String)> {
    let root = root(ctx)?;
    let action = ToolTaskAction::parse(&args.action)?;
    let index = SkillIndex::build(&root).ok();
    let text = toolcenter::tool_task(
        &root,
        ctx.state(),
        index.as_ref(),
        args.slug.as_deref(),
        action,
        args.requirement.as_deref(),
    )?;
    Ok((
        CliResult::success(json!({ "task": text, "action": action.as_str() })),
        text,
    ))
}

fn skill_link(ctx: &AppContext, args: SkillLinkArgs) -> Result<(CliResult, String)> {
    let root = root(ctx)?;
    let reference = SkillRef::parse(&args.reference)?;
    let mut file = files::ToolFile::load(&root, &args.slug)?;
    let link = files::ToolSkillLink {
        domain: reference.domain,
        name: reference.name.clone(),
        repo: None,
        subpath: None,
    };
    let added = file.add_skill_link(&link);
    if added {
        file.save()?;
    }
    Ok((
        CliResult::success(json!({
            "tool": args.slug,
            "skill": reference.to_string(),
            "added": added,
        })),
        if added {
            format!(
                "已添加配套 Skill 关联 {} → {}（未入库时界面显示“未入库”；属于库修改，询问后再上传）",
                args.slug,
                reference
            )
        } else {
            format!("配套 Skill 关联已存在：{} → {}", args.slug, reference)
        },
    ))
}
