//! `settings` 子命令：本机运行配置的读取、检查频率写入与两条只读报告。
//!
//! 参数与边界遵循 `specs/002-skill-management/contracts/cli.md` 的 `settings` 命令组与
//! spec.md FR-050「本机运行侧」/FR-052/FR-053：只暴露自动检查频率写入，
//! 共享目录自动定位；主题/字号与上传长期授权没有写入口，诊断与容量复用 core。
//! 只读写本机状态，不改 SkillManagerVault 内容、不触发上传。

use clap::{Args, Subcommand};
use serde_json::{Value, json};
use skill_manager_core::diagnostics;
use skill_manager_core::vault::capacity;
use skill_manager_core::{AppContext, CliResult, CoreError, GlobalOptions};

#[derive(Args, Debug)]
pub struct SettingsCommand {
    #[command(subcommand)]
    pub action: SettingsAction,
}

#[derive(Subcommand, Debug)]
pub enum SettingsAction {
    /// 只读：运行配置与调度事实的当前生效值。
    Get(GetArgs),
    /// 设置统一的自动检查频率（FR-029 三项共用）。
    #[command(name = "set-auto-check")]
    SetAutoCheck(SetAutoCheckArgs),
    /// 诊断报告与运行日志/崩溃记录的真实路径。
    Diagnostics(DiagnosticsArgs),
    /// 技能库容量：总占用与较大技能排行。
    Capacity,
}

#[derive(Args, Debug)]
pub struct GetArgs {
    /// 只读取指定键（可重复）；省略时返回全部键的当前生效值。
    #[arg(value_name = "key")]
    pub keys: Vec<String>,
}

#[derive(Args, Debug)]
pub struct SetAutoCheckArgs {
    /// 自动检查频率（分钟）：0=关闭 / 30 / 60 / 360 / 1440。
    #[arg(value_name = "minutes")]
    pub minutes: i64,
}

#[derive(Args, Debug)]
pub struct DiagnosticsArgs {
    /// 追加写出可交给 Agent 的诊断文件并返回其真实路径。
    #[arg(long)]
    pub export: bool,
}

type Result<T> = std::result::Result<T, CoreError>;

/// `settings get` 支持的键（顺序即人类可读输出顺序）。
const SETTING_KEYS: [&str; 8] = [
    "theme",
    "font_scale",
    "auto_check_minutes",
    "shared_dir",
    "git_auto_push",
    "vault_path",
    "last_sync_at",
    "last_check_at",
];

/// 分发 `settings` 子命令。
pub fn run(
    command: SettingsCommand,
    ctx: &mut AppContext,
    _options: &GlobalOptions,
) -> (CliResult, String) {
    match dispatch(command.action, ctx) {
        Ok(pair) => pair,
        Err(err) => (CliResult::failed_error(err), String::new()),
    }
}

fn dispatch(action: SettingsAction, ctx: &AppContext) -> Result<(CliResult, String)> {
    match action {
        SettingsAction::Get(args) => get(ctx, args),
        SettingsAction::SetAutoCheck(args) => set_auto_check(ctx, args),
        SettingsAction::Diagnostics(args) => diagnostics_report(ctx, args),
        SettingsAction::Capacity => capacity_report(ctx),
    }
}

// ---------------------------------------------------------------------------
// get
// ---------------------------------------------------------------------------

/// 当前生效值：未设置的键返回 `null`，不省略键、不用空字符串冒充。
fn effective_value(ctx: &AppContext, key: &str) -> Result<Value> {
    let state = ctx.state();
    Ok(match key {
        "theme" => json!(state.theme()?),
        "font_scale" => json!(state.font_scale()?),
        "auto_check_minutes" => json!(state.auto_check_minutes()?),
        "shared_dir" => json!(ctx.shared_dir()?.to_string_lossy()),
        "git_auto_push" => json!(state.git_auto_push()?),
        "vault_path" => path_value(state.vault_path()?),
        "last_sync_at" => text_value(state.last_sync_at()?),
        "last_check_at" => text_value(state.last_check_at()?),
        other => {
            return Err(CoreError::other(format!(
                "不支持的设置键 {other}（可用：{}）",
                SETTING_KEYS.join("、")
            )));
        }
    })
}

fn path_value(value: Option<std::path::PathBuf>) -> Value {
    match value {
        Some(path) => json!(path.to_string_lossy()),
        None => Value::Null,
    }
}

fn text_value(value: Option<String>) -> Value {
    value.map_or(Value::Null, Value::String)
}

fn get(ctx: &AppContext, args: GetArgs) -> Result<(CliResult, String)> {
    let keys: Vec<&str> = if args.keys.is_empty() {
        SETTING_KEYS.to_vec()
    } else {
        args.keys.iter().map(String::as_str).collect()
    };

    let mut data = serde_json::Map::new();
    let mut human = String::new();
    for key in &keys {
        let value = effective_value(ctx, key)?;
        data.insert((*key).to_string(), value.clone());
        human.push_str(&format!("{key} = {}\n", human_value(&value)));
    }
    Ok((CliResult::success(Value::Object(data)), human.trim_end().to_string()))
}

fn human_value(value: &Value) -> String {
    match value {
        Value::Null => "（未设置）".to_string(),
        Value::Bool(flag) => if *flag { "是" } else { "否" }.to_string(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

// ---------------------------------------------------------------------------
// set-auto-check
// ---------------------------------------------------------------------------

fn set_auto_check(ctx: &AppContext, args: SetAutoCheckArgs) -> Result<(CliResult, String)> {
    ctx.state().set_auto_check_minutes(args.minutes)?;
    let current = ctx.state().auto_check_minutes()?;
    let human = if current == 0 {
        "已关闭自动检查（统一周期为 0）".to_string()
    } else {
        format!("自动检查频率已设为 {current} 分钟")
    };
    Ok((
        CliResult::success(json!({ "auto_check_minutes": current })),
        human,
    ))
}

// ---------------------------------------------------------------------------
// diagnostics / capacity
// ---------------------------------------------------------------------------

fn diagnostics_report(ctx: &AppContext, args: DiagnosticsArgs) -> Result<(CliResult, String)> {
    if args.export {
        let export = diagnostics::export(ctx, 200)?;
        let report = &export.report;
        let human = format!(
            "已导出诊断资料：{}\n版本：{}；平台：{} / {}\n运行日志：{}\n崩溃记录：{}",
            export.path, report.version, report.os, report.arch, report.log_path, report.crash_path
        );
        return Ok((CliResult::from_data(&export), human));
    }

    let report = diagnostics::report(ctx, 100)?;
    let human = format!(
        "诊断报告\n版本：{}；平台：{} / {}\nSkillManagerVault：{}\n状态目录：{}\n运行日志：{}\n崩溃记录：{}\n自动检查频率：{} 分钟\n最近检查：{}\n最近同步：{}",
        report.version,
        report.os,
        report.arch,
        report.vault.as_deref().unwrap_or("（尚未接入）"),
        report.state_dir,
        report.log_path,
        report.crash_path,
        report.auto_check_minutes,
        report.last_check_at.as_deref().unwrap_or("（无记录）"),
        report.last_sync_at.as_deref().unwrap_or("（无记录）"),
    );
    Ok((CliResult::from_data(&report), human))
}

fn capacity_report(ctx: &AppContext) -> Result<(CliResult, String)> {
    let root = ctx.vault()?.root().to_path_buf();
    let report = capacity::scan(&root, 10)?;
    let mut human = format!(
        "库总占用：{} 字节 / {} 文件\n技能占用：{} 字节（{} 项）",
        report.total_size, report.file_count, report.skill_total_size, report.skill_count
    );
    for skill in &report.skills {
        human.push_str(&format!(
            "\n- {}（{} 字节，{} 文件）{}",
            skill.name, skill.size, skill.file_count, skill.path
        ));
    }
    Ok((CliResult::from_data(&report), human))
}
