//! `sync` 子命令：SkillManagerVault 跨机拉取、上传、状态与冲突（T054）。
//!
//! 参数与输出遵循 `specs/002-skill-management/contracts/cli.md`；业务规则全部在 core。
//! CLI 写库/部署同步不构成上传授权：`sync push` 是 Agent 获得明确同意后的显式调用入口。

use clap::{Args, Subcommand};
use skill_manager_core::output::{ItemError, Outcome};
use skill_manager_core::sync::{PullReport, SyncService};
use skill_manager_core::{AppContext, CliResult, CoreError, GlobalOptions};

#[derive(Args, Debug)]
pub struct SyncCommand {
    #[command(subcommand)]
    pub action: SyncAction,
}

#[derive(Subcommand, Debug)]
pub enum SyncAction {
    /// 拉取同一 SkillManagerVault；无冲突静默应用并刷新已有部署/指令目标。
    Pull,
    /// 获明确同意后显式上传工作区全部改动（提交并推送自己的库）。
    Push(PushArgs),
    /// 只读同步事实：连接、未提交、未拉取与冲突。
    Status,
    /// 输出 Git 冲突信息供 Agent 检查。
    Conflicts,
}

#[derive(Args, Debug)]
pub struct PushArgs {
    /// 提交信息；缺省使用固定说明。
    #[arg(long, value_name = "text")]
    pub message: Option<String>,
}

type Result<T> = std::result::Result<T, CoreError>;

/// 分发 `sync` 子命令。
pub fn run(
    command: SyncCommand,
    ctx: &mut AppContext,
    _options: &GlobalOptions,
) -> (CliResult, String) {
    let result = match command.action {
        SyncAction::Pull => pull(ctx),
        SyncAction::Push(args) => push(ctx, args),
        SyncAction::Status => status(ctx),
        SyncAction::Conflicts => conflicts(ctx),
    };
    match result {
        Ok(pair) => pair,
        Err(err) => (CliResult::failed_error(err), String::new()),
    }
}

/// 在已接入的库上构造同步服务并执行闭包（解析器生命周期随闭包）。
fn with_service<T>(
    ctx: &AppContext,
    f: impl FnOnce(&SyncService<'_>) -> Result<T>,
) -> Result<T> {
    let vault = ctx.vault()?;
    let home = ctx.home()?;
    let resolver = ctx.target_resolver()?;
    let service = SyncService::new(vault, home, ctx.catalog(), ctx.state(), &resolver);
    f(&service)
}

fn pull(ctx: &AppContext) -> Result<(CliResult, String)> {
    let report = with_service(ctx, |service| service.pull())?;
    let human = pull_human(&report);
    let data = serde_json::to_value(&report).unwrap_or(serde_json::Value::Null);

    let result = match report.outcome.as_str() {
        "pulled" | "up_to_date" if !report.errors.is_empty() => CliResult::partial(data, report.errors.clone()),
        "pulled" | "up_to_date" => CliResult::success(data),
        "conflict" => {
            let mut errors = vec![ItemError::new(&report.message)];
            for path in &report.conflicts {
                errors.push(ItemError::new("未合并路径").path(path.clone()));
            }
            CliResult::partial(data, errors)
        }
        _ => CliResult {
            result: Outcome::Failed,
            data,
            errors: vec![ItemError::new(&report.message)],
        },
    };
    Ok((result, human))
}

fn pull_human(report: &PullReport) -> String {
    let mut text = report.message.clone();
    for target in &report.instruction_targets {
        text.push_str(&format!(
            "\n- 指令目标 {}：{}{}",
            target.agent,
            if target.ok { "已刷新" } else { "刷新失败" },
            target.path
        ));
    }
    text
}

fn push(ctx: &AppContext, args: PushArgs) -> Result<(CliResult, String)> {
    let report = with_service(ctx, |service| service.push(args.message.as_deref()))?;
    let human = format!(
        "{}（分支 {}，远端 {}）",
        report.message,
        report.branch.clone().unwrap_or_else(|| "-".to_string()),
        report.remote.clone().unwrap_or_else(|| "-".to_string())
    );
    Ok((CliResult::from_data(&report), human))
}

fn status(ctx: &AppContext) -> Result<(CliResult, String)> {
    let status = with_service(ctx, |service| service.status())?;
    let connection = if status.connected {
        "已连接".to_string()
    } else {
        format!(
            "未连接（{}）",
            status.connection_error.clone().unwrap_or_default()
        )
    };
    let human = format!(
        "库：{}\n连接：{}\n分支：{}　上游：{}\n未提交：{} 项　未拉取：{}　领先：{}\n冲突：{}",
        status.vault,
        connection,
        status.branch.clone().unwrap_or_else(|| "-".to_string()),
        status.upstream.clone().unwrap_or_else(|| "-".to_string()),
        status.uncommitted.len(),
        status
            .behind
            .map(|value| value.to_string())
            .unwrap_or_else(|| "未知".to_string()),
        status
            .ahead
            .map(|value| value.to_string())
            .unwrap_or_else(|| "未知".to_string()),
        if status.conflict { "有" } else { "无" }
    );
    Ok((CliResult::from_data(&status), human))
}

fn conflicts(ctx: &AppContext) -> Result<(CliResult, String)> {
    let view = with_service(ctx, |service| service.conflicts())?;
    let human = if view.has_conflict {
        format!(
            "{}\n未合并路径：\n- {}",
            view.message,
            view.conflicts.join("\n- ")
        )
    } else {
        view.message.clone()
    };
    Ok((CliResult::from_data(&view), human))
}
