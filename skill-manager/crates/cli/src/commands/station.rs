//! `station` 子命令：管理站 HTTP 与本机重置管理员。

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use skill_manager_core::{CliResult, GlobalOptions};

#[derive(Args, Debug, Clone)]
pub struct StationCommand {
    #[command(subcommand)]
    pub action: StationAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum StationAction {
    /// 启动管理站 HTTP。首次无管理员时在本机创建。
    Serve(ServeArgs),
    /// 仅本机：清除管理员，下次 serve 重新创建。
    ResetAdmin,
}

#[derive(Args, Debug, Clone)]
pub struct ServeArgs {
    #[arg(long, default_value = "127.0.0.1:8787")]
    pub listen: String,
    /// 管理站本机数据目录（station.db）。
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
}

pub fn run(command: StationCommand, _options: &GlobalOptions) -> (CliResult, String) {
    match command.action {
        StationAction::Serve(args) => serve_cmd(args),
        StationAction::ResetAdmin => match skill_manager_station::serve::reset_admin(None) {
            Ok(msg) => (CliResult::success(serde_json::json!({ "message": msg })), msg),
            Err(err) => (CliResult::failed_message(err.clone()), err),
        },
    }
}

fn serve_cmd(args: ServeArgs) -> (CliResult, String) {
    let listen: SocketAddr = match args.listen.parse() {
        Ok(addr) => addr,
        Err(err) => {
            let msg = format!("无法解析监听地址：{err}");
            return (CliResult::failed_message(msg.clone()), msg);
        }
    };
    if let Err(err) = skill_manager_station::serve::run_blocking(listen, args.data_dir) {
        return (CliResult::failed_message(err.clone()), err);
    }
    (
        CliResult::success(serde_json::json!({ "stopped": true })),
        "管理站已停止".to_string(),
    )
}
