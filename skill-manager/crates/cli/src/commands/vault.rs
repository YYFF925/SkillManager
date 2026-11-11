//! `vault clone`：用本机 git 接入已有远端库。

use std::path::PathBuf;

use clap::{Args, Subcommand};
use skill_manager_core::sync::Git;
use skill_manager_core::{AppContext, CliResult, GlobalOptions};

#[derive(Args, Debug, Clone)]
pub struct VaultCommand {
    #[command(subcommand)]
    pub action: VaultAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum VaultAction {
    /// 调用本机 git clone；目标已存在且非空则失败。成功后记为本机 Vault。
    Clone(CloneArgs),
}

#[derive(Args, Debug, Clone)]
pub struct CloneArgs {
    pub remote_url: String,
    #[arg(long)]
    pub path: PathBuf,
}

pub fn run(command: VaultCommand, options: &GlobalOptions) -> (CliResult, String) {
    match command.action {
        VaultAction::Clone(args) => clone_cmd(options, args),
    }
}

fn clone_cmd(options: &GlobalOptions, args: CloneArgs) -> (CliResult, String) {
    match Git::clone_into(&args.remote_url, &args.path) {
        Ok(path) => {
            let mut ctx = match AppContext::open(options.state_dir.clone()) {
                Ok(ctx) => ctx,
                Err(err) => {
                    let msg = format!(
                        "库已 clone 到 {}，但无法写入本机状态：{err}。请稍后用桌面或 host 接入该路径。",
                        path.display()
                    );
                    return (CliResult::failed_message(msg.clone()), msg);
                }
            };
            if let Err(err) = ctx.attach_vault(&path) {
                let msg = format!("库已 clone 到 {}，接入失败：{err}", path.display());
                return (CliResult::failed_message(msg.clone()), msg);
            }
            let msg = format!("已 clone 并接入 {}", path.display());
            (
                CliResult::success(serde_json::json!({
                    "path": path,
                    "remote": args.remote_url,
                })),
                msg,
            )
        }
        Err(err) => {
            let msg = err.to_string();
            (CliResult::failed_message(msg.clone()), msg)
        }
    }
}
