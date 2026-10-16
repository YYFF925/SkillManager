//! 统一调度（T053，FR-029/FR-037）：上游检查、库拉取与工具版本检查共用同一“自动检查频率”。
//!
//! 触发方式：启动（`Startup`）、手动（`Manual`）与定时（`Timer`）。启动与手动总是执行；
//! 定时仅在频率不为“关闭”（0）时到达后执行，关闭定时不禁用手动操作。三类结果分别表达，
//! 不合并成单一“同步成功”。工具版本检查由 US9（T077）补齐，本期只登记入口，不冒充已执行。

use std::sync::atomic::AtomicBool;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::context::AppContext;
use crate::error::Result;
use crate::sync::SyncService;
use crate::upstream::{CheckScope, UpstreamService};

/// 定时是否到期：无上次记录视为到期；否则距上次已过至少 `interval_minutes`。
///
/// 仅用于 `Trigger::Timer` 的门控；启动与手动不受影响。
pub fn is_due(last: Option<DateTime<Utc>>, interval_minutes: i64, now: DateTime<Utc>) -> bool {
    match last {
        None => true,
        Some(last) => (now - last).num_minutes() >= interval_minutes,
    }
}

/// 一次调度的触发方式。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    Startup,
    Manual,
    Timer,
}

impl Trigger {
    pub fn as_str(self) -> &'static str {
        match self {
            Trigger::Startup => "startup",
            Trigger::Manual => "manual",
            Trigger::Timer => "timer",
        }
    }

    /// 定时触发才受“自动检查频率”的门控；启动与手动不受影响。
    pub fn respects_interval(self) -> bool {
        matches!(self, Trigger::Timer)
    }
}

/// 单个任务的结果（分别表达）。
#[derive(Clone, Debug, Serialize)]
pub struct TaskOutcome {
    pub executed: bool,
    /// `ok` / `error` / `skipped`。
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl TaskOutcome {
    fn ok(detail: impl Into<String>) -> Self {
        Self {
            executed: true,
            status: "ok".to_string(),
            detail: Some(detail.into()),
        }
    }

    fn error(detail: impl Into<String>) -> Self {
        Self {
            executed: true,
            status: "error".to_string(),
            detail: Some(detail.into()),
        }
    }

    fn skipped(detail: impl Into<String>) -> Self {
        Self {
            executed: false,
            status: "skipped".to_string(),
            detail: Some(detail.into()),
        }
    }
}

/// 一次调度的整体结果。
#[derive(Clone, Debug, Serialize)]
pub struct ScheduleReport {
    pub trigger: String,
    pub interval_minutes: i64,
    /// 定时是否启用（频率不为 0）。
    pub timer_enabled: bool,
    /// 本次是否因“定时已关闭”而未执行。
    pub skipped_by_interval: bool,
    /// 本次是否因“未到检查周期”而未执行（仅定时）。
    pub not_due: bool,
    pub upstream: TaskOutcome,
    pub pull: TaskOutcome,
    pub tool_check: TaskOutcome,
}

/// 统一调度未执行时的报告（定时关闭或未到周期）。
fn skipped_report(
    trigger: Trigger,
    interval: i64,
    timer_enabled: bool,
    skipped_by_interval: bool,
    note: &str,
) -> ScheduleReport {
    ScheduleReport {
        trigger: trigger.as_str().to_string(),
        interval_minutes: interval,
        timer_enabled,
        skipped_by_interval,
        not_due: !skipped_by_interval,
        upstream: TaskOutcome::skipped(note),
        pull: TaskOutcome::skipped(note),
        tool_check: TaskOutcome::skipped(note),
    }
}

fn parse_time(raw: Option<String>) -> Option<DateTime<Utc>> {
    raw.and_then(|value| {
        DateTime::parse_from_rfc3339(&value)
            .ok()
            .map(|parsed| parsed.with_timezone(&Utc))
    })
}

/// 执行一次统一调度。
pub fn run(ctx: &AppContext, trigger: Trigger) -> Result<ScheduleReport> {
    let interval = ctx.state().auto_check_minutes()?;
    let timer_enabled = interval != 0;
    let now = Utc::now();

    // 定时门控：关闭或未到周期时跳过；启动与手动不受影响。
    if trigger.respects_interval() {
        if !timer_enabled {
            return Ok(skipped_report(
                trigger,
                interval,
                false,
                true,
                "自动检查频率为“关闭”，本次定时未执行；手动操作仍可用",
            ));
        }
        let last = parse_time(ctx.state().last_check_at()?);
        if !is_due(last, interval, now) {
            return Ok(skipped_report(
                trigger,
                interval,
                true,
                false,
                "尚未到达自动检查周期，本次未执行",
            ));
        }
    }

    let Some(vault) = ctx.vault_opt() else {
        let note = "尚未接入 SkillManagerVault，未执行；接入后可用";
        return Ok(ScheduleReport {
            trigger: trigger.as_str().to_string(),
            interval_minutes: interval,
            timer_enabled,
            skipped_by_interval: false,
            not_due: false,
            upstream: TaskOutcome::skipped(note),
            pull: TaskOutcome::skipped(note),
            tool_check: TaskOutcome::skipped(note),
        });
    };

    let resolver = ctx.target_resolver()?;

    // 1) 上游检查（只展示，不应用）。
    let upstream = {
        let cancel = AtomicBool::new(false);
        let service = UpstreamService::from_context(ctx, &resolver)?;
        match service.check(&CheckScope::All, &mut |_progress| {}, &cancel) {
            Ok(report) => TaskOutcome::ok(format!(
                "检查 {} 个来源仓库、{} 个技能，发现 {} 个新增候选",
                report.repos.len(),
                report.skills.len(),
                report.candidates.len()
            )),
            Err(err) => {
                let _ = crate::diagnostics::record(
                    ctx.state(),
                    "error",
                    "scheduler.upstream",
                    &err.to_string(),
                );
                TaskOutcome::error(err.to_string())
            }
        }
    };

    // 2) 库拉取（无冲突静默应用并刷新已有部署；不推送）。
    let home = ctx.home()?;
    let pull = {
        let service = SyncService::new(vault, home, ctx.catalog(), ctx.state(), &resolver);
        match service.pull() {
            Ok(report) => {
                if report.outcome == "pulled" || report.outcome == "up_to_date" {
                    TaskOutcome::ok(report.message)
                } else {
                    // 未连接/冲突/无上游都如实表达为未成功，不冒充成功。
                    let _ = crate::diagnostics::record(
                        ctx.state(),
                        "error",
                        "scheduler.pull",
                        &report.message,
                    );
                    TaskOutcome::error(report.message)
                }
            }
            Err(err) => {
                let _ = crate::diagnostics::record(
                    ctx.state(),
                    "error",
                    "scheduler.pull",
                    &err.to_string(),
                );
                TaskOutcome::error(err.to_string())
            }
        }
    };

    // 3) 工具版本检查（US9/T077）：执行已登记工具的检查命令，只写检查结果，
    //    不安装、不更新、不改本机安装事实；成功/失败/未登记命令分别表达。
    let tool_check = match crate::toolcenter::check_all(vault.root(), ctx.state()) {
        Ok(outcomes) => {
            let count = |status: &str| {
                outcomes
                    .iter()
                    .filter(|outcome| outcome.status == status)
                    .count()
            };
            TaskOutcome::ok(format!(
                "检查 {} 个工具：成功 {}，失败 {}，未登记命令 {}",
                outcomes.len(),
                count("checked"),
                count("failed"),
                count("no_command")
            ))
        }
        Err(err) => {
            let _ = crate::diagnostics::record(
                ctx.state(),
                "error",
                "scheduler.tool-check",
                &err.to_string(),
            );
            TaskOutcome::error(err.to_string())
        }
    };

    ctx.state().set_last_check_at(&now.to_rfc3339())?;

    // 记录“何时执行过哪次调度”便于诊断定位操作与时间；成功与否按各任务分别表达。
    let _ = crate::diagnostics::record(
        ctx.state(),
        "info",
        "scheduler",
        &format!(
            "{} 调度：上游 {}，库拉取 {}，工具检查 {}",
            trigger.as_str(),
            upstream.status,
            pull.status,
            tool_check.status
        ),
    );

    Ok(ScheduleReport {
        trigger: trigger.as_str().to_string(),
        interval_minutes: interval,
        timer_enabled,
        skipped_by_interval: false,
        not_due: false,
        upstream,
        pull,
        tool_check,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn only_timer_respects_interval() {
        assert!(Trigger::Timer.respects_interval());
        assert!(!Trigger::Manual.respects_interval());
        assert!(!Trigger::Startup.respects_interval());
    }

    #[test]
    fn due_uses_last_run_and_interval() {
        let now = Utc::now();
        assert!(is_due(None, 60, now));
        assert!(!is_due(Some(now - Duration::minutes(30)), 60, now));
        assert!(is_due(Some(now - Duration::minutes(60)), 60, now));
    }
}
