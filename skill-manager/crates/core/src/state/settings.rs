//! 应用设置（data-model §2.6）：主题、字号、自动检查频率、最近同步时间与上传确认。
//!
//! 只存本机状态；自动检查频率三项共用（FR-029），上传确认只表达 GUI 保存这一条已授权
//! 来源，不是逐次同意账本（T050）。

use crate::error::{CoreError, Result};
use crate::state::{
    setting_key, StateStore, AUTO_CHECK_MINUTES_ALLOWED, AUTO_CHECK_MINUTES_DEFAULT,
    FONT_SCALE_ALLOWED, THEME_ALLOWED,
};

impl StateStore {
    /// 设置主题；只接受 light/dark/system（默认 system，不继承旧版 dark）。
    pub fn set_theme(&self, theme: &str) -> Result<()> {
        let theme = theme.trim();
        if !THEME_ALLOWED.contains(&theme) {
            return Err(CoreError::other(format!(
                "主题只支持 light/dark/system，收到 {theme}"
            )));
        }
        self.set_setting(setting_key::THEME, theme)
    }

    /// 设置字号；只接受 small/default/large/xlarge。
    pub fn set_font_scale(&self, scale: &str) -> Result<()> {
        let scale = scale.trim();
        if !FONT_SCALE_ALLOWED.contains(&scale) {
            return Err(CoreError::other(format!(
                "字号只支持 small/default/large/xlarge，收到 {scale}"
            )));
        }
        self.set_setting(setting_key::FONT_SCALE, scale)
    }
    /// 自动检查频率（分钟）：默认 60（FR-029，三项共用）。
    pub fn auto_check_minutes(&self) -> Result<i64> {
        let raw = self.setting(setting_key::AUTO_CHECK_MINUTES)?;
        match raw {
            Some(value) => Ok(value.parse().unwrap_or(AUTO_CHECK_MINUTES_DEFAULT)),
            None => Ok(AUTO_CHECK_MINUTES_DEFAULT),
        }
    }

    /// 设置自动检查频率；只接受“关闭/30/60/360/1440”五个取值。
    pub fn set_auto_check_minutes(&self, minutes: i64) -> Result<()> {
        if !AUTO_CHECK_MINUTES_ALLOWED.contains(&minutes) {
            return Err(CoreError::other(format!(
                "自动检查频率只支持 0（关闭）/30/60/360/1440 分钟，收到 {minutes}"
            )));
        }
        self.set_setting(
            setting_key::AUTO_CHECK_MINUTES,
            &minutes.to_string(),
        )
    }

    /// 最近一次成功 `sync pull` 的时间；无记录为 None。
    pub fn last_sync_at(&self) -> Result<Option<String>> {
        Ok(self
            .setting(setting_key::LAST_SYNC_AT)?
            .filter(|value| !value.trim().is_empty()))
    }

    /// 记录最近一次成功 `sync pull` 的时间。
    pub fn set_last_sync_at(&self, value: &str) -> Result<()> {
        self.set_setting(setting_key::LAST_SYNC_AT, value)
    }

    /// 最近一次完成统一调度（上游检查/库拉取/工具检查）的时间；无记录为 None。
    pub fn last_check_at(&self) -> Result<Option<String>> {
        Ok(self
            .setting(setting_key::LAST_CHECK_AT)?
            .filter(|value| !value.trim().is_empty()))
    }

    /// 记录最近一次完成统一调度的时间。
    pub fn set_last_check_at(&self, value: &str) -> Result<()> {
        self.set_setting(setting_key::LAST_CHECK_AT, value)
    }

    /// GUI 保存后是否已确认自动上传；缺省为 false（尚未确认）。
    pub fn git_auto_push(&self) -> Result<bool> {
        Ok(matches!(
            self.setting(setting_key::GIT_AUTO_PUSH)?.as_deref(),
            Some("true")
        ))
    }

    /// 写入 GUI 保存自动上传的既有确认（首次确认后为 true）。
    pub fn set_git_auto_push(&self, enabled: bool) -> Result<()> {
        self.set_setting(
            setting_key::GIT_AUTO_PUSH,
            if enabled { "true" } else { "false" },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_check_frequency_defaults_and_validates() {
        let store = StateStore::open_in_memory().unwrap();
        assert_eq!(store.auto_check_minutes().unwrap(), 60);

        for value in AUTO_CHECK_MINUTES_ALLOWED {
            store.set_auto_check_minutes(value).unwrap();
            assert_eq!(store.auto_check_minutes().unwrap(), value);
        }
        assert!(store.set_auto_check_minutes(15).is_err());
    }

    #[test]
    fn last_sync_and_auto_push_round_trip() {
        let store = StateStore::open_in_memory().unwrap();
        assert_eq!(store.last_sync_at().unwrap(), None);
        assert!(!store.git_auto_push().unwrap());

        store.set_last_sync_at("2026-10-15T00:00:00Z").unwrap();
        assert_eq!(
            store.last_sync_at().unwrap().as_deref(),
            Some("2026-10-15T00:00:00Z")
        );

        store.set_git_auto_push(true).unwrap();
        assert!(store.git_auto_push().unwrap());
        store.set_git_auto_push(false).unwrap();
        assert!(!store.git_auto_push().unwrap());
    }
}
