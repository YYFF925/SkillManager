//! Skill Manager 命令行入口（面向 Agent 的主要管理入口）。
//!
//! 全局 `--vault` 指定共享内容真值，`--state-dir` 隔离本机状态；两者一起可在同一台
//! 机器上模拟两台电脑。业务规则全部在 `skill-manager-core` 中。

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use skill_manager_core::{AppContext, GlobalOptions};

mod commands;

/// Skill Manager 命令行接口。
#[derive(Parser, Debug)]
#[command(
    name = "skill-manager",
    version,
    about = "Skill Manager 命令行接口",
    arg_required_else_help = true
)]
struct Cli {
    /// 输出结构化 JSON 结果（result/data/errors）。
    #[arg(long, global = true)]
    json: bool,

    /// SkillVault 根目录。
    #[arg(long, global = true, value_name = "path")]
    vault: Option<PathBuf>,

    /// 本机状态目录（隔离 SQLite、检查缓存与日志）。
    #[arg(long, global = true, value_name = "path")]
    state_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// 技能与库管理。
    Skill(commands::skill::SkillCommand),
    /// 部署技能到本机 Agent、共享目录或 Hermes default 档案。
    Deploy(commands::deploy::DeployCommand),
    /// 预设组合定义管理（新建/删除/重命名/图标/排序/成员）。
    Preset(commands::preset::PresetCommand),
    /// 本机 Agent 配置与本地技能管理。
    Agent(commands::agent::AgentCommand),
    /// Hermes 档案与技能启用管理。
    Hermes(commands::hermes::HermesCommand),
    /// SkillVault 跨机同步（拉取/上传/状态/冲突）。
    Sync(commands::sync::SyncCommand),
    /// Agent 指令源读取与保存。
    Instruction(commands::instruction::InstructionCommand),
    /// 工具中心：工具资料、版本检查、安装事实回写与维护任务。
    Tool(commands::tool::ToolCommand),
    /// 本机运行配置与只读报告（设置读取、共享目录/检查频率、诊断与容量）。
    Settings(commands::settings::SettingsCommand),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let options = GlobalOptions {
        json: cli.json,
        vault: cli.vault,
        state_dir: cli.state_dir,
    };

    let mut ctx = match open_context(&options) {
        Ok(ctx) => ctx,
        Err(message) => {
            let result = skill_manager_core::CliResult::failed_message(message);
            return ExitCode::from(result.emit(options.json, "") as u8);
        }
    };

    let (result, human) = match cli.command {
        Command::Skill(command) => commands::skill::run(command, &mut ctx, &options),
        Command::Deploy(command) => commands::deploy::run(command, &mut ctx, &options),
        Command::Preset(command) => commands::preset::run(command, &mut ctx, &options),
        Command::Agent(command) => commands::agent::run(command, &mut ctx, &options),
        Command::Hermes(command) => commands::hermes::run(command, &mut ctx, &options),
        Command::Sync(command) => commands::sync::run(command, &mut ctx, &options),
        Command::Instruction(command) => commands::instruction::run(command, &mut ctx, &options),
        Command::Tool(command) => commands::tool::run(command, &mut ctx, &options),
        Command::Settings(command) => commands::settings::run(command, &mut ctx, &options),
    };

    ExitCode::from(result.emit(options.json, &human) as u8)
}

/// 建立上下文：本机状态目录必须可用；库路径给了就必须能接入。
fn open_context(options: &GlobalOptions) -> Result<AppContext, String> {
    let mut ctx = AppContext::open(options.state_dir.clone()).map_err(|err| err.to_string())?;
    match &options.vault {
        Some(path) => ctx.attach_vault(path).map_err(|err| err.to_string())?,
        None => {
            ctx.attach_remembered_vault()
                .map_err(|err| err.to_string())?;
        }
    }
    Ok(ctx)
}
