//! 本机 Agent 配置与本地技能删除的核心断言（T067/T068）。
//!
//! 覆盖：检测只读事实且不写状态、启停/排序只写本机状态、技能目录覆盖被部署采用、
//! 项目相对路径归一化、未受管删除不改库、受管删除走撤销与暂停规则、路径包含性校验。

use std::path::PathBuf;

use skill_manager_core::deploy::{
    AgentCatalog, AgentConfigService, DeploymentService, DirKind, LocalSkillService, TargetResolver,
};
use skill_manager_core::domain::{Domain, SkillRef, Target};
use skill_manager_core::state::StateStore;

struct Env {
    _root: tempfile::TempDir,
    vault: PathBuf,
    home: PathBuf,
    state: StateStore,
}

impl Env {
    fn new() -> Self {
        let root = tempfile::TempDir::new().unwrap();
        let vault = root.path().join("vault");
        let home = root.path().join("home");
        let state_dir = root.path().join("state");
        for dir in [&vault, &home, &state_dir] {
            std::fs::create_dir_all(dir).unwrap();
        }
        let state = StateStore::open(&state_dir).unwrap();
        Self {
            _root: root,
            vault,
            home,
            state,
        }
    }

    fn resolver(&self) -> TargetResolver<'static> {
        let mut resolver = TargetResolver::new(self.home.clone(), AgentCatalog::embedded(), None);
        let dirs: Vec<(String, PathBuf)> = self
            .state
            .agents()
            .unwrap()
            .into_iter()
            .filter_map(|config| config.skills_dir.map(|dir| (config.key, PathBuf::from(dir))))
            .collect();
        resolver.set_agent_dirs(dirs);
        resolver
    }

    fn config(&self) -> AgentConfigService<'_> {
        AgentConfigService::new(self.home.clone(), AgentCatalog::embedded(), &self.state)
    }

    fn write(&self, rel: &str, content: &str) -> PathBuf {
        let path = self.vault.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
        path
    }

    fn home_file(&self, rel: &str, content: &str) -> PathBuf {
        let path = self.home.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
        path
    }

    fn deploy(&self, resolver: &TargetResolver<'static>, reference: &SkillRef, target: Target) {
        let service = DeploymentService::new(self.vault.clone(), resolver, &self.state);
        let report = service
            .deploy_to(std::slice::from_ref(reference), std::slice::from_ref(&target), false)
            .unwrap();
        assert!(report.failures.is_empty(), "{:?}", report.failures);
    }
}

fn code_ref(name: &str) -> SkillRef {
    SkillRef::new(Domain::Code, name).unwrap()
}

fn agent(key: &str) -> Target {
    Target::Agent {
        key: key.to_string(),
    }
}

#[test]
fn detection_reads_existence_without_writing_state() {
    let env = Env::new();
    let service = env.config();
    let all = service.list().unwrap();
    assert_eq!(all.len(), 53);
    assert!(all.iter().all(|view| !view.detected));

    std::fs::create_dir_all(env.home.join(".codex")).unwrap();
    let detected = service.detect().unwrap();
    let codex = detected.iter().find(|view| view.key == "codex").unwrap();
    assert!(codex.detected);
    // 无本机记录时默认启用；且检测不写任何本机状态。
    assert!(codex.enabled);
    assert_eq!(env.state.agent("codex").unwrap(), None);
}

#[test]
fn enable_disable_and_reorder_write_only_local_state() {
    let env = Env::new();
    let service = env.config();

    service.set_enabled("codex", false).unwrap();
    assert!(!env.state.agent("codex").unwrap().unwrap().enabled);
    service.set_enabled("codex", true).unwrap();
    assert!(env.state.agent("codex").unwrap().unwrap().enabled);

    let views = service.reorder(&["zcode".to_string(), "codex".to_string()]).unwrap();
    let order = |key: &str| views.iter().find(|view| view.key == key).unwrap().order;
    assert_eq!(order("zcode"), 0);
    assert_eq!(order("codex"), 1);

    // 未知 key 不写入。
    assert!(service.set_enabled("nope", true).is_err());
}

#[test]
fn skills_dir_override_is_used_by_deploy_and_reset_restores_default() {
    let env = Env::new();
    let custom = env._root.path().join("custom-skills");
    std::fs::create_dir_all(&custom).unwrap();
    env.write("code/alpha/SKILL.md", "正文\n");

    let service = env.config();
    let view = service
        .set_dir("codex", DirKind::Skills, &custom.display().to_string())
        .unwrap();
    assert_eq!(view.skills_dir, custom.display().to_string());
    assert!(view.skills_dir_custom);

    let resolver = env.resolver();
    env.deploy(&resolver, &code_ref("alpha"), agent("codex"));
    assert!(custom.join("alpha/SKILL.md").exists(), "部署应使用新目录");

    let view = service.reset_dir("codex", DirKind::Skills).unwrap();
    assert!(!view.skills_dir_custom);
    assert!(view.skills_dir.ends_with(".codex/skills"));

    // 相对路径被拒绝，避免随工作目录漂移。
    assert!(service.set_dir("codex", DirKind::Skills, "relative/dir").is_err());
}

#[test]
fn project_relative_path_is_normalized_and_cleared() {
    let env = Env::new();
    let service = env.config();
    let view = service
        .set_dir("codex", DirKind::ProjectRel, ".codex//skills")
        .unwrap();
    assert_eq!(view.project_rel_dir.as_deref(), Some(".codex/skills"));

    assert!(service.set_dir("codex", DirKind::ProjectRel, "a/../b").is_err());
    assert!(service.set_dir("codex", DirKind::ProjectRel, "/abs").is_err());

    let cleared = service.set_dir("codex", DirKind::ProjectRel, "").unwrap();
    assert_eq!(cleared.project_rel_dir, None);
}

#[test]
fn local_list_marks_managed_and_unmanaged() {
    let env = Env::new();
    env.write("code/alpha/SKILL.md", "正文\n");
    env.home_file(".codex/skills/localonly/SKILL.md", "仅本地\n");

    let resolver = env.resolver();
    env.deploy(&resolver, &code_ref("alpha"), agent("codex"));

    let service = LocalSkillService::new(&resolver, &env.state, Some(env.vault.clone()));
    let skills = service.list("codex").unwrap();
    let alpha = skills.iter().find(|skill| skill.name == "alpha").unwrap();
    assert!(alpha.managed);
    assert_eq!(alpha.status, "managed");
    assert_eq!(alpha.library_skill.as_deref(), Some("code/alpha"));

    let local = skills.iter().find(|skill| skill.name == "localonly").unwrap();
    assert!(!local.managed);
    assert_eq!(local.status, "local");
}

#[test]
fn unmanaged_delete_removes_files_without_touching_library() {
    let env = Env::new();
    env.write("code/alpha/SKILL.md", "正文\n");
    let dir = env.home_file(".codex/skills/localonly/SKILL.md", "仅本地\n");
    let skill_dir = dir.parent().unwrap().to_path_buf();

    let resolver = env.resolver();
    let service = LocalSkillService::new(&resolver, &env.state, Some(env.vault.clone()));
    let outcome = service.delete("codex", &["localonly".to_string()]).unwrap();

    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    assert_eq!(outcome.removed.len(), 1);
    assert!(!skill_dir.exists());
    assert!(env.vault.join("code/alpha/SKILL.md").exists(), "库内技能必须不变");
    assert!(env.state.deployments().unwrap().is_empty());
}

#[test]
fn managed_delete_uses_undo_and_keeps_library_skill() {
    let env = Env::new();
    env.write("code/alpha/SKILL.md", "正文\n");
    let resolver = env.resolver();
    env.deploy(&resolver, &code_ref("alpha"), agent("codex"));
    let copy = env.home.join(".codex/skills/alpha");
    assert!(copy.exists());

    let service = LocalSkillService::new(&resolver, &env.state, Some(env.vault.clone()));
    let outcome = service.delete("codex", &["alpha".to_string()]).unwrap();

    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    assert_eq!(outcome.removed.len(), 1);
    assert!(!copy.exists(), "受管副本应被移除");
    assert!(env.vault.join("code/alpha/SKILL.md").exists(), "删本机副本不得删库");
    assert!(env.state.deployments().unwrap().is_empty());
}

#[test]
fn managed_delete_pauses_when_copy_changed_externally() {
    let env = Env::new();
    env.write("code/alpha/SKILL.md", "正文\n");
    let resolver = env.resolver();
    env.deploy(&resolver, &code_ref("alpha"), agent("codex"));
    let copy = env.home.join(".codex/skills/alpha");
    std::fs::write(copy.join("SKILL.md"), "手改内容\n").unwrap();

    let service = LocalSkillService::new(&resolver, &env.state, Some(env.vault.clone()));
    let outcome = service.delete("codex", &["alpha".to_string()]).unwrap();

    assert!(outcome.removed.is_empty());
    assert_eq!(outcome.errors.len(), 1);
    assert!(outcome.errors[0].message.contains("暂停"), "{:?}", outcome.errors);
    assert!(copy.join("SKILL.md").exists(), "外改副本必须保留");
    assert_eq!(env.state.deployments().unwrap().len(), 1);
    assert!(env.vault.join("code/alpha/SKILL.md").exists());
}

#[test]
fn delete_rejects_paths_outside_the_agents_skills_dir() {
    let env = Env::new();
    // 技能目录内的符号链接指向技能目录之外：包含性校验必须拒绝。
    let outside = env._root.path().join("outside/evil");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("SKILL.md"), "外部\n").unwrap();
    let skills = env.home.join(".codex/skills");
    std::fs::create_dir_all(&skills).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, skills.join("evil")).unwrap();

    let resolver = env.resolver();
    let service = LocalSkillService::new(&resolver, &env.state, Some(env.vault.clone()));
    let outcome = service.delete("codex", &["evil".to_string()]).unwrap();

    assert!(outcome.removed.is_empty());
    assert_eq!(outcome.errors.len(), 1);
    assert!(
        outcome.errors[0].message.contains("技能目录内") || outcome.errors[0].message.contains("不合法"),
        "{:?}",
        outcome.errors
    );
    assert!(outside.join("SKILL.md").exists(), "外部文件不得被删除");

    // 带路径分隔符的名字也不接受。
    let escaped = service.delete("codex", &["../../etc".to_string()]).unwrap();
    assert!(escaped.removed.is_empty());
    assert_eq!(escaped.errors.len(), 1);
}
