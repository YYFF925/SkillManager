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
use crate::commands::skill::{
    ArchiveCommand, CategoryCommand, OverlayCommand, SkillAction, TagCommand, TranslationCommand,
};
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
            SkillAction::Tag(TagCommand::Add(args)) => Some((
                "skill_tag_add".into(),
                json!({ "references": args.references, "tags": csv(&args.tags) }),
            )),
            SkillAction::Tag(TagCommand::Detach(args)) => Some((
                "skill_tag_detach".into(),
                json!({ "references": args.references, "tags": csv(&args.tags) }),
            )),
            SkillAction::Tag(TagCommand::Rename { old, new }) => {
                Some(("skill_tag_rename".into(), json!({ "old": old, "new": new })))
            }
            SkillAction::Tag(TagCommand::Remove { tag }) => {
                Some(("skill_tag_remove".into(), json!({ "tag": tag })))
            }
            SkillAction::Category(CategoryCommand::Rename { domain, old, new }) => Some((
                "skill_category_rename".into(),
                json!({ "domain": domain, "old": old, "new": new }),
            )),
            SkillAction::Move(args) => Some((
                "skill_move".into(),
                json!({ "reference": args.reference, "category": args.category }),
            )),
            SkillAction::Archive(ArchiveCommand::Add(args)) => Some((
                "skill_archive".into(),
                json!({ "references": args.references }),
            )),
            SkillAction::Archive(ArchiveCommand::Restore(args)) => Some((
                "skill_archive_restore".into(),
                json!({ "references": args.references }),
            )),
            SkillAction::Archive(ArchiveCommand::Delete(args)) => Some((
                "skill_archive_delete".into(),
                json!({ "references": args.references }),
            )),
            SkillAction::Translation(TranslationCommand::Set(args)) => Some((
                "skill_translation_set".into(),
                json!({
                    "reference": args.reference,
                    "content": read_maybe_file(args.file.as_ref(), args.stdin)?,
                }),
            )),
            SkillAction::Translation(TranslationCommand::Remove(args)) => Some((
                "skill_translation_remove".into(),
                json!({ "reference": args.reference }),
            )),
            SkillAction::Overlay(OverlayCommand::Set(args)) => Some((
                "skill_overlay_set".into(),
                json!({
                    "reference": args.reference,
                    "path": args.path,
                    "content": read_maybe_file(args.file.as_ref(), args.stdin)?,
                    "agent": args.agent,
                    "os": args.os,
                }),
            )),
            SkillAction::Overlay(OverlayCommand::Remove(args)) => Some((
                "skill_overlay_remove".into(),
                json!({
                    "reference": args.reference,
                    "path": args.path,
                    "agent": args.agent,
                    "os": args.os,
                }),
            )),
            SkillAction::Import(args) => Some((
                "skill_import".into(),
                json!({
                    "repo": args.repo,
                    "skills": args.skills,
                    "domain": args.domain,
                    "reference": args.reference,
                    "category": args.category,
                    "uncategorized": args.uncategorized,
                    "job_id": job_id(),
                }),
            )),
            SkillAction::Adopt(args) => Some((
                "skill_adopt".into(),
                json!({
                    "agent": args.agent,
                    "names": args.names,
                    "category": args.category,
                    "uncategorized": args.uncategorized,
                    "comment": args.comment,
                    "tags": args.tags.as_ref().map(|s| csv(s)),
                }),
            )),
            SkillAction::Create(args) => Some((
                "skill_create".into(),
                json!({
                    "reference": args.reference,
                    "category": args.category,
                    "uncategorized": args.uncategorized,
                    "comment": args.comment,
                    "tags": args.tags.as_ref().map(|s| csv(s)),
                }),
            )),
            SkillAction::Copy(args) => Some((
                "skill_copy".into(),
                json!({
                    "reference": args.reference,
                    "to": args.to,
                    "category": args.category,
                    "uncategorized": args.uncategorized,
                }),
            )),
            SkillAction::Update(args) => Some((
                "skill_update".into(),
                json!({ "reference": args.reference, "to": args.to, "job_id": job_id() }),
            )),
            SkillAction::UpdateRepo(args) => Some((
                "skill_update_repo".into(),
                json!({
                    "repo": args.repo,
                    "include_new": args.include_new,
                    "domain": args.domain,
                    "category": args.category,
                    "uncategorized": args.uncategorized,
                    "job_id": job_id(),
                }),
            )),
            SkillAction::RejectNew(args) => Some((
                "skill_reject_new".into(),
                json!({ "repo": args.repo, "domain": args.domain, "skills": args.skills }),
            )),
            SkillAction::Skip(args) => Some((
                "skill_skip".into(),
                json!({ "reference": args.reference, "version": args.version }),
            )),
            SkillAction::Unskip(args) => Some((
                "skill_unskip".into(),
                json!({ "reference": args.reference }),
            )),
            SkillAction::AdoptHermes(args) => Some((
                "hermes_adopt".into(),
                json!({ "profile": args.profile, "names": args.names }),
            )),
            SkillAction::Preview(args) => Some((
                "skill_preview".into(),
                json!({
                    "repo": args.repo,
                    "reference": args.reference,
                    "job_id": job_id(),
                }),
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
                | SkillAction::Check(_)
                | SkillAction::Archive(ArchiveCommand::List { .. })
                | SkillAction::Translation(TranslationCommand::Get(_))
                | SkillAction::Overlay(OverlayCommand::List(_) | OverlayCommand::Get(_))
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
        Command::Tool(cmd) => !matches!(
            cmd.action,
            ToolAction::List(_) | ToolAction::Get(_) | ToolAction::Check(_)
        ),
    }
}

fn job_id() -> String {
    format!(
        "cli-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

fn read_maybe_file(path: Option<&std::path::PathBuf>, stdin: bool) -> Option<String> {
    if let Some(path) = path {
        std::fs::read_to_string(path).ok()
    } else if stdin {
        let mut buffer = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buffer).ok()?;
        Some(buffer)
    } else {
        None
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

    #[test]
    fn skill_tag_and_archive_writes_map_to_host_invoke() {
        use crate::commands::skill::{
            ArchiveCommand, DeleteArgs, SkillCommand, TagBatchArgs, TagCommand,
        };
        let tag = Command::Skill(SkillCommand {
            action: SkillAction::Tag(TagCommand::Add(TagBatchArgs {
                references: vec!["code/demo".into()],
                tags: "a,b".into(),
            })),
        });
        let (name, body) = to_invoke(&tag).expect("tag add must forward");
        assert_eq!(name, "skill_tag_add");
        assert_eq!(body["references"][0], "code/demo");
        assert!(is_write(&tag));

        let archive = Command::Skill(SkillCommand {
            action: SkillAction::Archive(ArchiveCommand::Add(DeleteArgs {
                references: vec!["code/demo".into()],
            })),
        });
        let (name, _) = to_invoke(&archive).expect("archive add must forward");
        assert_eq!(name, "skill_archive");
    }
}
