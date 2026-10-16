//! US3 部署合成与准入的核心断言（T022）。
//!
//! 覆盖：按 Agent/OS 的覆盖层与整份 `SKILL.md` 替换、中文注释只进产物且库内原件未变、
//! 译文与 `.manager` 不部署、域边界与支持声明过滤、显式空支持集合、支持收窄保留已有
//! 部署并警告、共享目录通用产物。全部使用预置库文件，不依赖 US2。

use std::path::{Path, PathBuf};

use skill_manager_core::deploy::compose::compose_skill;
use skill_manager_core::deploy::targets::{AgentCatalog, TargetResolver};
use skill_manager_core::deploy::DeploymentService;
use skill_manager_core::domain::{Domain, SkillRef, Target};
use skill_manager_core::state::StateStore;

/// 临时环境：预置库、独立状态目录、模拟 home 与共享目录。
struct Env {
    _root: tempfile::TempDir,
    vault: PathBuf,
    state: StateStore,
    resolver: TargetResolver<'static>,
}

impl Env {
    fn new() -> Self {
        let root = tempfile::TempDir::new().unwrap();
        let vault = root.path().join("vault");
        let state_dir = root.path().join("state");
        let home = root.path().join("home");
        for dir in [&vault, &state_dir, &home] {
            std::fs::create_dir_all(dir).unwrap();
        }
        let state = StateStore::open(&state_dir).unwrap();
        let resolver = TargetResolver::new(&home, AgentCatalog::embedded());
        Self {
            _root: root,
            vault,
            state,
            resolver,
        }
    }

    fn service(&self) -> DeploymentService<'_> {
        DeploymentService::new(self.vault.clone(), &self.resolver, &self.state)
    }

    fn write(&self, rel: &str, content: &str) {
        let path = self.vault.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    /// 写一个技能：`<domain>/[<category>/]<name>/SKILL.md`，返回技能目录。
    fn skill(&self, domain: &str, category: Option<&str>, name: &str, body: &str) -> PathBuf {
        let rel = match category {
            Some(c) => format!("{domain}/{c}/{name}"),
            None => format!("{domain}/{name}"),
        };
        self.write(&format!("{rel}/SKILL.md"), body);
        self.vault.join(rel)
    }

    fn manager(&self, domain: &str, category: Option<&str>, name: &str, rel: &str, content: &str) {
        let base = match category {
            Some(c) => format!("{domain}/{c}/{name}"),
            None => format!("{domain}/{name}"),
        };
        self.write(&format!("{base}/.manager/{rel}"), content);
    }

    fn target_path(&self, target: &Target, name: &str) -> PathBuf {
        PathBuf::from(self.resolver.resolve(target).unwrap().path).join(name)
    }
}

fn code_ref(name: &str) -> SkillRef {
    SkillRef::new(Domain::Code, name).unwrap()
}

fn lobster_ref(name: &str) -> SkillRef {
    SkillRef::new(Domain::Lobster, name).unwrap()
}

fn agent(key: &str) -> Target {
    Target::Agent {
        key: key.to_string(),
    }
}

fn text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

// ---------------------------------------------------------------------------
// 合成产物
// ---------------------------------------------------------------------------

#[test]
fn agent_overlay_and_comment_enter_artifact_while_library_original_is_untouched() {
    let env = Env::new();
    let dir = env.skill(
        "code",
        Some("工具"),
        "alpha",
        "---\nname: alpha\ndescription: 解析依赖\n---\n原文正文\n",
    );
    env.manager(
        "code",
        Some("工具"),
        "alpha",
        "meta.yaml",
        "comment: 检查依赖\n",
    );
    env.manager(
        "code",
        Some("工具"),
        "alpha",
        "overlay/_/_/notes.txt",
        "通用补充\n",
    );
    env.manager(
        "code",
        Some("工具"),
        "alpha",
        "overlay/codex/_/scripts/x.sh",
        "#!/bin/sh\necho codex\n",
    );

    let artifact = compose_skill(&dir, Some("codex"), "macos", Some("检查依赖")).unwrap();
    let entry = artifact.text("SKILL.md").expect("产物应含 SKILL.md");
    assert!(
        entry.contains("description: 检查依赖｜解析依赖"),
        "注释应注入 description 前缀：{entry}"
    );
    assert!(artifact.text("notes.txt").is_some(), "通用覆盖层应进入产物");
    assert!(
        artifact.text("scripts/x.sh").is_some(),
        "Agent 专属覆盖层应进入产物"
    );

    // 库内原件未被注释或覆盖层改写。
    let original = text(&dir.join("SKILL.md"));
    assert_eq!(original, "---\nname: alpha\ndescription: 解析依赖\n---\n原文正文\n");
    assert!(!dir.join("notes.txt").exists());
}

#[test]
fn os_specific_overlay_applies_only_for_the_matching_system() {
    let env = Env::new();
    let dir = env.skill("code", None, "alpha", "正文\n");
    env.manager("code", None, "alpha", "overlay/_/macos/tool.sh", "macos\n");
    env.manager("code", None, "alpha", "overlay/_/linux/tool.sh", "linux\n");
    env.manager("code", None, "alpha", "overlay/_/_/base.txt", "base\n");

    let macos = compose_skill(&dir, None, "macos", None).unwrap();
    assert_eq!(macos.text("tool.sh").as_deref(), Some("macos\n"));
    assert_eq!(macos.text("base.txt").as_deref(), Some("base\n"));

    let linux = compose_skill(&dir, None, "linux", None).unwrap();
    assert_eq!(linux.text("tool.sh").as_deref(), Some("linux\n"));
}

#[test]
fn whole_skill_md_overlay_replaces_original_without_merge() {
    let env = Env::new();
    let dir = env.skill(
        "code",
        None,
        "alpha",
        "---\nname: alpha\ndescription: 原描述\n---\n原库内正文\n",
    );
    env.manager(
        "code",
        None,
        "alpha",
        "overlay/codex/_/SKILL.md",
        "---\nname: alpha\ndescription: Codex 专用\n---\n覆盖层正文\n",
    );

    let artifact = compose_skill(&dir, Some("codex"), "macos", None).unwrap();
    let entry = artifact.text("SKILL.md").unwrap();
    assert!(entry.contains("覆盖层正文"), "{entry}");
    assert!(!entry.contains("原库内正文"), "整份替换不应合并原文：{entry}");
    // 另一 Agent 不受该覆盖层影响。
    let other = compose_skill(&dir, Some("zcode"), "macos", None).unwrap();
    assert!(other.text("SKILL.md").unwrap().contains("原库内正文"));
}

#[test]
fn translation_and_manager_layer_are_not_deployed() {
    let env = Env::new();
    let dir = env.skill("code", None, "alpha", "正文\n");
    env.manager("code", None, "alpha", "meta.yaml", "tags: [x]\n");
    env.manager("code", None, "alpha", "translation.md", "中文译文正文\n");
    env.manager(
        "code",
        None,
        "alpha",
        "source.yaml",
        "repo: https://example.invalid/skills.git\nsubpath: alpha\nversion: 0123456789abcdef0123456789abcdef01234567\nregistered_at: 2026-10-15\n",
    );

    let artifact = compose_skill(&dir, Some("codex"), "macos", None).unwrap();
    let paths = artifact.paths();
    assert!(
        paths.iter().all(|rel| !rel.starts_with(".manager")),
        "{paths:?}"
    );
    assert!(paths.iter().all(|rel| !rel.contains("translation")), "{paths:?}");
    assert_eq!(paths, vec!["SKILL.md".to_string()]);
}

#[test]
fn comment_prefix_is_not_stacked_when_overlay_already_carries_it() {
    let env = Env::new();
    let dir = env.skill("code", None, "alpha", "正文\n");
    env.manager(
        "code",
        None,
        "alpha",
        "overlay/_/_/SKILL.md",
        "---\ndescription: 检查依赖｜已处理\n---\n正文\n",
    );

    let artifact = compose_skill(&dir, None, "macos", Some("检查依赖")).unwrap();
    let entry = artifact.text("SKILL.md").unwrap();
    assert_eq!(entry.matches("检查依赖｜").count(), 1, "{entry}");
}

// ---------------------------------------------------------------------------
// 准入：管理域边界与支持声明
// ---------------------------------------------------------------------------

#[test]
fn domain_boundary_blocks_cross_domain_deploy() {
    let env = Env::new();
    env.skill("lobster", None, "beta", "龙虾技能\n");
    let report = env
        .service()
        .deploy_to(&[lobster_ref("beta")], &[agent("codex")])
        .unwrap();
    assert!(report.items.is_empty());
    assert_eq!(report.failures.len(), 1);
    assert!(
        report.failures[0].message.contains("管理域"),
        "{}",
        report.failures[0].message
    );

    // 同域目标正常部署。
    env.skill("lobster", None, "gamma", "龙虾技能\n");
    let ok = env
        .service()
        .deploy_to(&[lobster_ref("gamma")], &[Target::HermesDefault])
        .unwrap();
    assert_eq!(ok.failures.len(), 0, "{:?}", ok.failures);
    assert_eq!(ok.items.len(), 1);
}

#[test]
fn support_declaration_blocks_new_deploy_to_unsupported_agent() {
    let env = Env::new();
    env.skill("code", None, "alpha", "正文\n");
    env.manager("code", None, "alpha", "meta.yaml", "agents: [zcode]\n");

    let blocked = env
        .service()
        .deploy_to(&[code_ref("alpha")], &[agent("codex")])
        .unwrap();
    assert_eq!(blocked.failures.len(), 1);
    assert!(
        blocked.failures[0].message.contains("支持"),
        "{}",
        blocked.failures[0].message
    );
    assert!(!env.target_path(&agent("codex"), "alpha").exists());

    let allowed = env
        .service()
        .deploy_to(&[code_ref("alpha")], &[agent("zcode")])
        .unwrap();
    assert_eq!(allowed.failures.len(), 0, "{:?}", allowed.failures);
    assert!(env.target_path(&agent("zcode"), "alpha").exists());
}

#[test]
fn explicit_empty_support_has_no_target_at_all() {
    let env = Env::new();
    env.skill("code", None, "alpha", "正文\n");
    env.manager("code", None, "alpha", "meta.yaml", "agents: []\n");

    let report = env
        .service()
        .deploy_to(&[code_ref("alpha")], &[agent("codex")])
        .unwrap();
    assert_eq!(report.failures.len(), 1);
    assert!(
        report.failures[0].message.contains("不支持"),
        "{}",
        report.failures[0].message
    );
}

#[test]
fn narrowed_support_keeps_existing_deployment_and_warns() {
    let env = Env::new();
    env.skill("code", None, "alpha", "正文\n");
    env.manager("code", None, "alpha", "meta.yaml", "agents: [zcode]\n");
    let first = env
        .service()
        .deploy_to(&[code_ref("alpha")], &[agent("zcode")])
        .unwrap();
    assert_eq!(first.failures.len(), 0, "{:?}", first.failures);

    // 收窄声明：移除 zcode。
    env.manager("code", None, "alpha", "meta.yaml", "agents: [some-other-agent]\n");
    let again = env
        .service()
        .deploy_to(&[code_ref("alpha")], &[agent("zcode")])
        .unwrap();
    assert_eq!(again.failures.len(), 0, "已有部署应保留而不是失败：{:?}", again.failures);
    assert_eq!(again.items.len(), 1);
    assert!(
        again.items[0]
            .message
            .as_deref()
            .is_some_and(|message| message.contains("收窄")),
        "应有支持收窄警告：{:?}",
        again.items[0].message
    );
    assert!(env.target_path(&agent("zcode"), "alpha").exists());

    // 新目标仍被禁止。
    let new_target = env
        .service()
        .deploy_to(&[code_ref("alpha")], &[agent("codex")])
        .unwrap();
    assert_eq!(new_target.failures.len(), 1);
}

// ---------------------------------------------------------------------------
// 共享目录通用产物
// ---------------------------------------------------------------------------

#[test]
fn shared_gets_generic_content_and_is_not_rejected_for_having_overlays() {
    let env = Env::new();
    env.skill(
        "code",
        None,
        "alpha",
        "---\ndescription: 原描述\n---\n正文\n",
    );
    env.manager("code", None, "alpha", "meta.yaml", "comment: 通用技能\n");
    env.manager("code", None, "alpha", "overlay/_/_/common.txt", "通用\n");
    env.manager("code", None, "alpha", "overlay/codex/_/only-codex.txt", "codex\n");

    let report = env
        .service()
        .deploy_to(&[code_ref("alpha")], &[Target::Shared])
        .unwrap();
    assert_eq!(report.failures.len(), 0, "{:?}", report.failures);

    let shared = env.target_path(&Target::Shared, "alpha");
    assert!(shared.join("common.txt").exists(), "共享产物应包含通用覆盖层");
    assert!(
        !shared.join("only-codex.txt").exists(),
        "Agent 专属覆盖层不应进入共享产物"
    );
    let entry = text(&shared.join("SKILL.md"));
    assert!(entry.contains("通用技能｜原描述"), "{entry}");
}

#[test]
fn shared_requires_all_consumers_supported() {
    let env = Env::new();
    env.skill("code", None, "alpha", "正文\n");
    env.manager("code", None, "alpha", "meta.yaml", "agents: [codex]\n");

    let report = env
        .service()
        .deploy_to(&[code_ref("alpha")], &[Target::Shared])
        .unwrap();
    assert_eq!(report.failures.len(), 1);
    assert!(
        report.failures[0].message.contains("共享"),
        "{}",
        report.failures[0].message
    );
    assert!(!env.target_path(&Target::Shared, "alpha").exists());
}
