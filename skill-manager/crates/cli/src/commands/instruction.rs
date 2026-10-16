//! `instruction` 子命令：Agent 指令源的读取与保存（T070）。
//!
//! 参数与输出遵循 `specs/002-skill-management/contracts/cli.md`；业务规则全部在 core。
//! 沿用单保存动作：保存源内容后按“全局在前、专属在后”覆盖本机已启用目标；指令路径与技能路径独立。

use std::io::Read;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::json;
use skill_manager_core::output::ItemError;
use skill_manager_core::vault::{InstructionScope, InstructionsService};
use skill_manager_core::{AppContext, CliResult, CoreError, GlobalOptions};

#[derive(Args, Debug)]
pub struct InstructionCommand {
    #[command(subcommand)]
    pub action: InstructionAction,
}

#[derive(Subcommand, Debug)]
pub enum InstructionAction {
    /// 读取某作用域的源内容与合成预览（global 或 Agent key）。
    Get(ScopeArgs),
    /// 保存某作用域的源内容并按拼接规则覆盖本机已启用目标。
    Set(SetArgs),
}

#[derive(Args, Debug)]
pub struct ScopeArgs {
    /// 作用域：`global` 或 Agent key。
    #[arg(value_name = "global|key")]
    pub scope: String,
}

#[derive(Args, Debug)]
pub struct SetArgs {
    /// 作用域：`global` 或 Agent key。
    #[arg(value_name = "global|key")]
    pub scope: String,
    /// 从文件读取内容。
    #[arg(long, value_name = "path", conflicts_with = "stdin")]
    pub file: Option<PathBuf>,
    /// 从标准输入读取内容。
    #[arg(long, conflicts_with = "file")]
    pub stdin: bool,
}

type Result<T> = std::result::Result<T, CoreError>;

/// 分发 `instruction` 子命令。
pub fn run(
    command: InstructionCommand,
    ctx: &mut AppContext,
    _options: &GlobalOptions,
) -> (CliResult, String) {
    let result = match command.action {
        InstructionAction::Get(args) => get(ctx, &args.scope),
        InstructionAction::Set(args) => set(ctx, args),
    };
    match result {
        Ok(pair) => pair,
        Err(err) => (CliResult::failed_error(err), String::new()),
    }
}

fn service(ctx: &AppContext) -> Result<InstructionsService<'_>> {
    let root = ctx.vault()?.root().to_path_buf();
    Ok(InstructionsService::new(
        root,
        ctx.home()?,
        ctx.catalog(),
        ctx.state(),
    ))
}

fn get(ctx: &AppContext, scope: &str) -> Result<(CliResult, String)> {
    let scope = InstructionScope::parse(scope)?;
    let view = service(ctx)?.get(&scope)?;
    let data = json!({
        "scope": view.scope,
        "source": view.source,
        "composed": view.composed,
    });
    let human = format!(
        "作用域 {}（源 {} 字符；合成后 {} 字符）",
        view.scope,
        view.source.chars().count(),
        view.composed.chars().count()
    );
    Ok((CliResult::from_data(&data), human))
}

fn set(ctx: &AppContext, args: SetArgs) -> Result<(CliResult, String)> {
    let scope = InstructionScope::parse(&args.scope)?;
    let content = match (&args.file, args.stdin) {
        (Some(path), false) => std::fs::read_to_string(path)
            .map_err(|err| CoreError::io(path.clone(), err))?,
        (None, true) => {
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|err| CoreError::io(PathBuf::from("<stdin>"), err))?;
            buffer
        }
        _ => {
            return Err(CoreError::other(
                "必须且只能指定 --file <path> 或 --stdin 之一",
            ))
        }
    };

    let results = service(ctx)?.save(&scope, &content)?;
    let failures: Vec<ItemError> = results
        .iter()
        .filter(|result| !result.ok)
        .map(|result| {
            ItemError::new(result.error.clone().unwrap_or_else(|| "写入失败".to_string()))
                .target(result.agent.clone())
                .path(result.path.clone())
        })
        .collect();

    let data = json!({
        "scope": scope.as_str(),
        "targets": results,
    });
    let mut human = format!(
        "已保存 {} 指令源；覆盖 {} 个本机已启用目标",
        scope.as_str(),
        results.len()
    );
    if !failures.is_empty() {
        human.push_str(&format!("，{} 个目标写入失败", failures.len()));
    }

    let result = if failures.is_empty() {
        CliResult::success(data)
    } else {
        CliResult::partial(data, failures)
    };
    Ok((result, human))
}
