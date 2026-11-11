//! Host 在场时写入类命令转发到本机 Host；不在场时直连 core 并占同一把锁。

use serde_json::{Value, json};
use skill_manager_core::GlobalOptions;
use skill_manager_host::lock::HostLock;
use skill_manager_host::{InvokeSource, http};

use crate::Command;
use crate::commands::agent::AgentAction;
use crate::commands::deploy::DeployAction;
use crate::commands::hermes::HermesAction;
use crate::commands::instruction::InstructionAction;
use crate::commands::preset::PresetAction;
use crate::commands::settings::SettingsAction;
use crate::commands::skill::SkillAction;
use crate::commands::sync::SyncAction;
use crate::commands::tool::ToolAction;

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
            SyncAction::Push(args) => {
                Some(("sync_push".into(), json!({ "message": args.message })))
            }
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
        Command::Preset(cmd) => match &cmd.action {
            PresetAction::Create(args) => Some((
                "preset_create".into(),
                json!({
                    "domain": args.domain,
                    "name": args.name,
                    "icon": args.icon,
                }),
            )),
            PresetAction::Delete(args) => Some((
                "preset_delete".into(),
                json!({ "domain": args.domain, "id": args.id }),
            )),
            PresetAction::Members(args) => {
                if let Some(add) = &args.add {
                    Some((
                        "preset_members_add".into(),
                        json!({
                            "domain": args.domain,
                            "id": args.id,
                            "names": csv(add),
                        }),
                    ))
                } else if let Some(remove) = &args.remove {
                    Some((
                        "preset_members_remove".into(),
                        json!({
                            "domain": args.domain,
                            "id": args.id,
                            "names": csv(remove),
                        }),
                    ))
                } else {
                    None
                }
            }
            PresetAction::Rename(args) => Some((
                "preset_rename".into(),
                json!({
                    "domain": args.domain,
                    "id": args.id,
                    "name": args.name,
                }),
            )),
            PresetAction::Icon(args) => Some((
                "preset_set_icon".into(),
                json!({
                    "domain": args.domain,
                    "id": args.id,
                    "icon": args.icon,
                }),
            )),
            PresetAction::Reorder(args) => Some((
                "preset_reorder".into(),
                json!({
                    "domain": args.domain,
                    "order": csv(&args.order),
                }),
            )),
            PresetAction::ReorderMembers(args) => Some((
                "preset_reorder_members".into(),
                json!({
                    "domain": args.domain,
                    "id": args.id,
                    "order": csv(&args.order),
                }),
            )),
            PresetAction::List(_) | PresetAction::Get(_) => None,
        },
        Command::Agent(cmd) => match &cmd.action {
            AgentAction::Enable(args) => Some(("agent_enable".into(), json!({ "key": args.key }))),
            AgentAction::Disable(args) => {
                Some(("agent_disable".into(), json!({ "key": args.key })))
            }
            AgentAction::Reorder(args) => {
                Some(("agent_reorder".into(), json!({ "order": csv(&args.order) })))
            }
            AgentAction::SetDir(args) => {
                let (kind, path) = if let Some(path) = &args.skills {
                    ("skills", path)
                } else if let Some(path) = &args.project_rel {
                    ("project-rel", path)
                } else if let Some(path) = &args.instructions {
                    ("instructions", path)
                } else {
                    return None;
                };
                Some((
                    "agent_set_dir".into(),
                    json!({ "key": args.key, "kind": kind, "path": path }),
                ))
            }
            AgentAction::ResetDir(args) => Some((
                "agent_reset_dir".into(),
                json!({ "key": args.key, "kind": args.kind }),
            )),
            AgentAction::DeleteLocal(args) => Some((
                "agent_delete_local".into(),
                json!({ "target": args.target, "names": args.names }),
            )),
            AgentAction::List
            | AgentAction::Detect
            | AgentAction::LocalSkills(_)
            | AgentAction::ReadLocal(_) => None,
        },
        Command::Hermes(cmd) => match &cmd.action {
            HermesAction::Enable(args) => Some((
                "hermes_enable".into(),
                json!({ "profile": args.profile, "names": args.names }),
            )),
            HermesAction::Disable(args) => Some((
                "hermes_disable".into(),
                json!({ "profile": args.profile, "names": args.names }),
            )),
            HermesAction::Profiles | HermesAction::Read(_) | HermesAction::List(_) => None,
        },
        Command::Instruction(cmd) => match &cmd.action {
            InstructionAction::Set(args) => {
                let content = if let Some(path) = &args.file {
                    std::fs::read_to_string(path).ok()?
                } else if args.stdin {
                    let mut buffer = String::new();
                    std::io::Read::read_to_string(&mut std::io::stdin(), &mut buffer).ok()?;
                    buffer
                } else {
                    return None;
                };
                Some((
                    "instruction_set".into(),
                    json!({ "scope": args.scope, "content": content }),
                ))
            }
            InstructionAction::Get(_) => None,
        },
        Command::Tool(cmd) => match &cmd.action {
            ToolAction::Check(args) => Some((
                "tool_check".into(),
                json!({ "slug": args.slug, "all": args.all }),
            )),
            ToolAction::Register(args) => {
                if let Some(dir) = &args.dir {
                    Some(("tool_register".into(), json!({ "dir": dir })))
                } else {
                    None
                }
            }
            ToolAction::Set(args) => {
                if let Some(dir) = &args.dir {
                    Some(("tool_set".into(), json!({ "slug": args.slug, "dir": dir })))
                } else {
                    None
                }
            }
            ToolAction::SetInstall(args) => Some((
                "tool_set_install".into(),
                json!({
                    "slug": args.slug,
                    "version": args.version,
                    "method": args.method,
                    "path": args.path,
                    "config_path": args.config_path,
                }),
            )),
            ToolAction::Task(args) => Some((
                "tool_task".into(),
                json!({
                    "slug": args.slug,
                    "action": args.action,
                    "requirement": args.requirement,
                }),
            )),
            ToolAction::SkillLink(args) => Some((
                "tool_skill_link".into(),
                json!({ "slug": args.slug, "reference": args.reference }),
            )),
            ToolAction::List(_) | ToolAction::Get(_) => None,
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
        Command::Preset(cmd) => !matches!(cmd.action, PresetAction::List(_) | PresetAction::Get(_)),
        Command::Agent(cmd) => !matches!(
            cmd.action,
            AgentAction::List
                | AgentAction::Detect
                | AgentAction::LocalSkills(_)
                | AgentAction::ReadLocal(_)
        ),
        Command::Hermes(cmd) => matches!(
            cmd.action,
            HermesAction::Enable(_) | HermesAction::Disable(_)
        ),
        Command::Instruction(cmd) => matches!(cmd.action, InstructionAction::Set(_)),
        Command::Tool(cmd) => !matches!(cmd.action, ToolAction::List(_) | ToolAction::Get(_)),
    }
}

fn csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::agent::{AgentAction, AgentCommand, KeyArgs};
    use crate::commands::preset::{CreateArgs, PresetAction, PresetCommand};

    #[test]
    fn preset_and_agent_writes_map_to_host_invoke() {
        let preset = Command::Preset(PresetCommand {
            action: PresetAction::Create(CreateArgs {
                domain: "code".into(),
                name: "日常".into(),
                icon: None,
            }),
        });
        let (name, body) = to_invoke(&preset).expect("preset create must forward");
        assert_eq!(name, "preset_create");
        assert_eq!(body["domain"], "code");
        assert!(is_write(&preset));

        let agent = Command::Agent(AgentCommand {
            action: AgentAction::Enable(KeyArgs {
                key: "codex".into(),
            }),
        });
        let (name, body) = to_invoke(&agent).expect("agent enable must forward");
        assert_eq!(name, "agent_enable");
        assert_eq!(body["key"], "codex");
    }
}
