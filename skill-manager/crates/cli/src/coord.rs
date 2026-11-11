//! Host 在场时写入类命令转发到本机 Host；不在场时直连 core 并占同一把锁。

use serde_json::{json, Value};
use skill_manager_core::GlobalOptions;
use skill_manager_host::lock::HostLock;
use skill_manager_host::{http, InvokeSource};

use crate::commands::deploy::DeployAction;
use crate::commands::settings::SettingsAction;
use crate::commands::skill::SkillAction;
use crate::commands::sync::SyncAction;
use crate::Command;

pub enum Coordination {
    Forward { listen: String },
    Local { _lock: Option<HostLock> },
}

pub fn coordinate(options: &GlobalOptions, command: &Command) -> Result<Coordination, String> {
    if !is_write(command) {
        return Ok(Coordination::Local { _lock: None });
    }
    let state_dir = options
        .state_dir
        .clone()
        .unwrap_or_else(skill_manager_core::AppContext::default_state_dir);
    if let Some(listen) = http::try_local_client(&state_dir) {
        if http::probe(&listen) {
            return Ok(Coordination::Forward { listen });
        }
    }
    match HostLock::try_acquire(&state_dir) {
        Ok(lock) => Ok(Coordination::Local { _lock: Some(lock) }),
        Err(err) => Err(err.to_string()),
    }
}

pub fn forward(listen: &str, command: &str, body: &Value) -> Result<Value, String> {
    http::forward_invoke(listen, command, body, InvokeSource::Cli, None)
}

pub fn to_invoke(command: &Command) -> Option<(String, Value)> {
    match command {
        Command::Sync(cmd) => match &cmd.action {
            SyncAction::Pull => Some(("sync_pull".into(), json!({}))),
            SyncAction::Push(args) => Some(("sync_push".into(), json!({ "message": args.message }))),
            _ => None,
        },
        Command::Skill(cmd) => match &cmd.action {
            SkillAction::Set(args) => Some((
                "skill_set".into(),
                json!({
                    "reference": args.reference,
                    "comment": args.comment,
                    "tags": args.tags.as_ref().map(|s| csv(s)),
                }),
            )),
            SkillAction::Delete(args) => Some((
                "skill_delete".into(),
                json!({ "references": args.references }),
            )),
            _ => None,
        },
        Command::Deploy(cmd) => match &cmd.action {
            DeployAction::To(args) => Some((
                "deploy_to".into(),
                json!({
                    "target": args.target,
                    "references": args.references,
                    "also_targets": args.also_target,
                }),
            )),
            DeployAction::Undo(args) => Some((
                "deploy_undo".into(),
                json!({
                    "target": args.target,
                    "references": args.references,
                    "also_targets": args.also_target,
                    "discard_local_changes": args.discard_local_changes,
                }),
            )),
            DeployAction::Sync(args) => Some((
                "deploy_sync".into(),
                json!({
                    "reference": args.reference,
                    "target": args.target,
                    "overwrite_local": args.overwrite_local,
                }),
            )),
            _ => None,
        },
        Command::Settings(cmd) => match &cmd.action {
            SettingsAction::SetAutoCheck(args) => Some((
                "settings_set_auto_check".into(),
                json!({ "minutes": args.minutes }),
            )),
            _ => None,
        },
        _ => None,
    }
}

pub fn is_write(command: &Command) -> bool {
    match command {
        Command::Host(_) | Command::Station(_) => false,
        Command::Vault(_) => true,
        Command::Sync(cmd) => matches!(cmd.action, SyncAction::Pull | SyncAction::Push(_)),
        Command::Settings(cmd) => matches!(cmd.action, SettingsAction::SetAutoCheck(_)),
        Command::Deploy(cmd) => !matches!(cmd.action, DeployAction::Status(_)),
        Command::Skill(cmd) => !matches!(
            cmd.action,
            SkillAction::List(_)
                | SkillAction::Get(_)
                | SkillAction::Read(_)
                | SkillAction::Skipped
                | SkillAction::Rejected(_)
                | SkillAction::Task(_)
        ),
        Command::Preset(_) | Command::Agent(_) | Command::Hermes(_) | Command::Instruction(_) | Command::Tool(_) => true,
    }
}

fn csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}
