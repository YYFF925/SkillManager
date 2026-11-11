//! 应用设置命令（T055/T083）：主题、字号、共同检查频率与 GUI 自动上传确认。
//!
//! 薄封装 core；主题（light/dark/system，默认 system）与字号取值由 core 校验，这里只读写本机状态。

use serde::Serialize;

use crate::HostRuntime;

/// GUI 需要的本机设置项。
#[derive(Clone, Debug, Serialize)]
pub struct SettingsView {
    /// 主题：`light` / `dark` / `system`（默认跟随系统）。
    pub theme: String,
    /// 字号：`small` / `default` / `large` / `xlarge`。
    pub font_scale: String,
    /// 自动检查频率（分钟）；0 表示关闭。三项检查共用（FR-029）。
    pub auto_check_minutes: i64,
    /// 是否已确认“GUI 保存后自动上传”；缺省为 false（尚未确认）。
    pub git_auto_push: bool,
}

fn read(runtime: &HostRuntime) -> Result<SettingsView, String> {
    let ctx = runtime.get()?;
    Ok(SettingsView {
        theme: ctx.state().theme().map_err(|err| err.to_string())?,
        font_scale: ctx.state().font_scale().map_err(|err| err.to_string())?,
        auto_check_minutes: ctx
            .state()
            .auto_check_minutes()
            .map_err(|err| err.to_string())?,
        git_auto_push: ctx.state().git_auto_push().map_err(|err| err.to_string())?,
    })
}

/// 读取主题、字号、共同检查频率与自动上传确认状态。
pub fn settings_get(runtime: &HostRuntime) -> Result<SettingsView, String> {
    read(runtime)
}

/// 设置主题；只接受 light/dark/system（不继承旧版 dark 默认）。
pub fn settings_set_theme(
    runtime: &HostRuntime,
    theme: String,
) -> Result<SettingsView, String> {
    runtime
        .get()?
        .state()
        .set_theme(&theme)
        .map_err(|err| err.to_string())?;
    read(runtime)
}

/// 设置字号；只接受 small/default/large/xlarge。
pub fn settings_set_font_scale(
    runtime: &HostRuntime,
    scale: String,
) -> Result<SettingsView, String> {
    runtime
        .get()?
        .state()
        .set_font_scale(&scale)
        .map_err(|err| err.to_string())?;
    read(runtime)
}

/// 设置共同检查频率；只接受“关闭/30/60/360/1440”五个取值。
pub fn settings_set_auto_check(
    runtime: &HostRuntime,
    minutes: i64,
) -> Result<SettingsView, String> {
    runtime
        .get()?
        .state()
        .set_auto_check_minutes(minutes)
        .map_err(|err| err.to_string())?;
    read(runtime)
}

/// 写入 GUI 保存自动上传的既有确认；首次确认后沿用。
pub fn settings_set_git_auto_push(
    runtime: &HostRuntime,
    enabled: bool,
) -> Result<SettingsView, String> {
    runtime
        .get()?
        .state()
        .set_git_auto_push(enabled)
        .map_err(|err| err.to_string())?;
    read(runtime)
}
