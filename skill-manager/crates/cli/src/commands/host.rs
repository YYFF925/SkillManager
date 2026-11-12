//! `host` 子命令：serve / status / 开机项 / 对接。

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use skill_manager_core::{CliResult, GlobalOptions};
use skill_manager_host::{HostLock, HostRuntime, serve, service};

#[derive(Args, Debug, Clone)]
pub struct HostCommand {
    #[command(subcommand)]
    pub action: HostAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum HostAction {
    /// 前台运行 Host。已有实例占锁则失败。
    Serve(ServeArgs),
    /// 报告是否在跑、Vault、对接与锁。
    Status,
    /// 登记当前用户开机启动（启动命令即 host serve）。
    InstallService,
    /// 清除开机启动项。
    UninstallService,
    /// 出站对接管理站。
    Pair(PairArgs),
    /// 解除对接，清本机 station_url 与 device_key。
    Unpair,
    /// ACP 子进程用的 Host 工具通道（stdio）。不占 host.lock，不单独作为用户 MCP 产品。
    Mcp,
    /// 查看、安装、登录或登记内置管理 Agent 所用的 ACP CLI。默认钉死 OpenCode。
    AgentCli(AgentCliArgs),
}

#[derive(Args, Debug, Clone)]
pub struct AgentCliArgs {
    /// 可执行文件路径或 PATH 中的名字。
    #[arg(long)]
    pub bin: Option<String>,
    /// ACP 启动参数，空格分隔，例如 `acp` 或 `--acp`。省略则按常见约定尝试。
    #[arg(long)]
    pub args: Option<String>,
    /// 写入 agent_runtime 的标识，默认取文件名。
    #[arg(long)]
    pub id: Option<String>,
    /// 清除用户指定，下次按钉死的默认 OpenCode（已安装或 PATH 上的 opencode）解析。
    #[arg(long)]
    pub clear: bool,
    /// 安装默认管理 CLI OpenCode 到本机 Host 目录并钉死，不打进 Skill Manager 安装包。
    #[arg(long)]
    pub install: bool,
    /// 打开默认（或已登记）管理 CLI 的本机登录/授权。
    #[arg(long)]
    pub login: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ServeArgs {
    /// 监听地址，默认本机回环随机端口。
    #[arg(long)]
    pub listen: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct PairArgs {
    #[arg(long)]
    pub station: String,
    #[arg(long)]
    pub code: String,
    #[arg(long)]
    pub name: Option<String>,
}

pub fn run(command: HostCommand, options: &GlobalOptions) -> (CliResult, String) {
    match command.action {
        HostAction::Serve(args) => serve_cmd(options, args),
        HostAction::Status => status_cmd(options),
        HostAction::InstallService => match service::install(options.state_dir.as_deref()) {
            Ok(msg) => (
                CliResult::success(serde_json::json!({ "message": msg })),
                msg,
            ),
            Err(err) => (CliResult::failed_message(err.clone()), err),
        },
        HostAction::UninstallService => match service::uninstall() {
            Ok(msg) => (
                CliResult::success(serde_json::json!({ "message": msg })),
                msg,
            ),
            Err(err) => (CliResult::failed_message(err.clone()), err),
        },
        HostAction::Pair(args) => pair_cmd(options, args),
        HostAction::Unpair => unpair_cmd(options),
        HostAction::Mcp => match skill_manager_host::agent::run_mcp_stdio() {
            Ok(()) => (
                CliResult::success(serde_json::json!({ "ok": true })),
                String::new(),
            ),
            Err(err) => (CliResult::failed_message(err.clone()), err),
        },
        HostAction::AgentCli(args) => agent_cli_cmd(options, args),
    }
}

fn serve_cmd(options: &GlobalOptions, args: ServeArgs) -> (CliResult, String) {
    let bind: SocketAddr = match args.listen.as_deref() {
        Some(raw) => match raw.parse() {
            Ok(addr) => addr,
            Err(err) => {
                let msg = format!("无法解析监听地址：{err}");
                return (CliResult::failed_message(msg.clone()), msg);
            }
        },
        None => serve::default_bind(),
    };
    if let Err(err) = serve::run_blocking(options.state_dir.clone(), bind) {
        return (CliResult::failed_message(err.clone()), err);
    }
    (
        CliResult::success(serde_json::json!({ "stopped": true })),
        "Host 已停止".to_string(),
    )
}

fn status_cmd(options: &GlobalOptions) -> (CliResult, String) {
    let state_dir = options
        .state_dir
        .clone()
        .unwrap_or_else(skill_manager_core::AppContext::default_state_dir);
    if let Some(listen) = skill_manager_host::http::try_local_client(&state_dir) {
        if skill_manager_host::http::probe(&listen) {
            if let Ok(value) = fetch_status(&listen) {
                let human = status_human(&value);
                return (CliResult::success(value), human);
            }
        }
    }
    if let Some(meta) = skill_manager_host::lock::peek_meta(&state_dir) {
        let value = serde_json::json!({
            "running": true,
            "pid": meta.pid,
            "listen": meta.listen,
            "vault": meta.vault,
            "lockHeld": true,
            "stateDir": state_dir.display().to_string(),
        });
        return (CliResult::success(value.clone()), status_human(&value));
    }
    // 锁空闲：尝试打开状态目录读 Vault / 对接，不占锁。
    match HostRuntime::open(Some(state_dir.clone())) {
        Ok(runtime) => {
            let mut view = serde_json::to_value(runtime.status_view()).unwrap_or_default();
            if let Some(obj) = view.as_object_mut() {
                obj.insert("running".into(), serde_json::json!(false));
                obj.insert("lockHeld".into(), serde_json::json!(false));
            }
            (CliResult::success(view.clone()), status_human(&view))
        }
        Err(err) => {
            let value = serde_json::json!({
                "running": false,
                "lockHeld": false,
                "stateDir": state_dir.display().to_string(),
                "error": err,
            });
            (CliResult::success(value.clone()), "Host 未运行".to_string())
        }
    }
}

fn fetch_status(listen: &str) -> Result<serde_json::Value, String> {
    skill_manager_host::http::forward_invoke(
        listen,
        "host_status",
        &serde_json::json!({}),
        skill_manager_host::InvokeSource::Cli,
        None,
    )
}

fn status_human(value: &serde_json::Value) -> String {
    let running = value
        .get("running")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let vault = value
        .get("vault")
        .and_then(|v| v.as_str())
        .unwrap_or("未接入");
    let paired = value
        .get("paired")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    format!(
        "Host {}；库 {}；{}",
        if running { "在运行" } else { "未运行" },
        vault,
        if paired {
            "已对接管理站"
        } else {
            "未对接"
        }
    )
}

fn agent_cli_cmd(options: &GlobalOptions, args: AgentCliArgs) -> (CliResult, String) {
    let runtime = match HostRuntime::open(options.state_dir.clone()) {
        Ok(rt) => rt,
        Err(err) => return (CliResult::failed_message(err.clone()), err),
    };
    if args.clear {
        return match skill_manager_host::agent::clear_user_cli(&runtime) {
            Ok(()) => {
                let view = skill_manager_host::agent::runtime_view(&runtime);
                (
                    CliResult::success(view.clone()),
                    "已清除指定的管理 CLI；下次使用钉死的默认 OpenCode（已安装或 PATH 上的 opencode）。".to_string(),
                )
            }
            Err(err) => (CliResult::failed_message(err.clone()), err),
        };
    }
    if args.install {
        return match skill_manager_host::agent::install_default_cli(&runtime) {
            Ok(view) => (
                CliResult::success(view.clone()),
                "已安装并钉死默认管理 CLI OpenCode。请执行 host agent-cli --login 完成本机授权。".to_string(),
            ),
            Err(err) => (CliResult::failed_message(err.clone()), err),
        };
    }
    if args.login {
        return match skill_manager_host::agent::launch_login(&runtime) {
            Ok(view) => {
                let msg = view
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("已打开管理 CLI 登录")
                    .to_string();
                (CliResult::success(view), msg)
            }
            Err(err) => (CliResult::failed_message(err.clone()), err),
        };
    }
    if let Some(bin) = args.bin.as_deref() {
        let acp_args = args
            .args
            .as_deref()
            .unwrap_or("")
            .split_whitespace()
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect();
        return match skill_manager_host::agent::set_user_cli(
            &runtime,
            bin,
            acp_args,
            args.id.as_deref(),
        ) {
            Ok(cli) => {
                let msg = format!(
                    "已登记管理 CLI {}（{}）。模型由该 CLI 决定，不限于 OpenCode / Codex / Gemini。",
                    cli.cli_id,
                    cli.binary_path.display()
                );
                (
                    CliResult::success(skill_manager_host::agent::runtime_view(&runtime)),
                    msg,
                )
            }
            Err(err) => (CliResult::failed_message(err.clone()), err),
        };
    }
    let view = skill_manager_host::agent::runtime_view(&runtime);
    let human = if view.get("ready").and_then(|v| v.as_bool()) == Some(true) {
        format!(
            "当前管理 CLI {}（{}）。之后对话使用这一家，不再扫描命令行清单。可改：host agent-cli --bin <路径> [--args acp]。",
            view.get("displayName").and_then(|v| v.as_str()).unwrap_or("OpenCode"),
            view.get("binaryPath").and_then(|v| v.as_str()).unwrap_or("")
        )
    } else if view.get("needsInstall").and_then(|v| v.as_bool()) == Some(true) {
        "尚未安装默认管理 CLI OpenCode。请执行 host agent-cli --install，或在对话面板点「安装 OpenCode」。".to_string()
    } else {
        view.get("error")
            .and_then(|v| v.as_str())
            .or_else(|| view.get("installHint").and_then(|v| v.as_str()))
            .unwrap_or("默认管理 CLI OpenCode 尚未就绪")
            .to_string()
    };
    (CliResult::success(view), human)
}

fn pair_cmd(options: &GlobalOptions, args: PairArgs) -> (CliResult, String) {
    let runtime = match HostRuntime::open(options.state_dir.clone()) {
        Ok(rt) => rt,
        Err(err) => return (CliResult::failed_message(err.clone()), err),
    };
    match skill_manager_host::pair::pair(&runtime, &args.station, &args.code, args.name.as_deref())
    {
        Ok(id) => {
            let msg = format!("已对接管理站 {}（设备 {}）", args.station, id.device_id);
            (
                CliResult::success(serde_json::to_value(&id).unwrap_or_default()),
                msg,
            )
        }
        Err(err) => (CliResult::failed_message(err.clone()), err),
    }
}

fn unpair_cmd(options: &GlobalOptions) -> (CliResult, String) {
    let runtime = match HostRuntime::open(options.state_dir.clone()) {
        Ok(rt) => rt,
        Err(err) => return (CliResult::failed_message(err.clone()), err),
    };
    match skill_manager_host::pair::unpair(&runtime) {
        Ok(()) => (
            CliResult::success(serde_json::json!({ "ok": true })),
            "已解除对接".to_string(),
        ),
        Err(err) => (CliResult::failed_message(err.clone()), err),
    }
}

#[allow(dead_code)]
fn _lock_path(dir: &PathBuf) -> PathBuf {
    HostLock::path(dir)
}
