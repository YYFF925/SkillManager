//! US1 库维护的文件级断言：分类移动、同域唯一性、标签治理、未知字段保留与译文时效。
//!
//! 这些用例直接对真实文件断言，不经过 CLI。

use std::path::{Path, PathBuf};

use skill_manager_core::domain::{Domain, SkillRef};
use skill_manager_core::manager::{self, SupportDecl};
use skill_manager_core::vault::index::{AgentSupport, CategoryFilter, SearchQuery, SkillIndex};
use skill_manager_core::vault::{manage, scan};

fn write_file(root: &Path, rel: &str, content: &str) -> PathBuf {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
    path
}

fn rel_dir(domain: &str, category: Option<&str>, name: &str) -> String {
    match category {
        Some(c) => format!("{domain}/{c}/{name}"),
        None => format!("{domain}/{name}"),
    }
}

/// 写一个技能：`<domain>/[<category>/]<name>/SKILL.md`。
fn write_skill(
    root: &Path,
    domain: &str,
    category: Option<&str>,
    name: &str,
    body: &str,
) -> PathBuf {
    write_file(root, &format!("{}/SKILL.md", rel_dir(domain, category, name)), body);
    root.join(rel_dir(domain, category, name))
}

fn write_meta(root: &Path, domain: &str, category: Option<&str>, name: &str, meta: &str) {
    write_file(
        root,
        &format!("{}/.manager/meta.yaml", rel_dir(domain, category, name)),
        meta,
    );
}

fn read_meta(root: &Path, domain: &str, category: Option<&str>, name: &str) -> String {
    std::fs::read_to_string(
        root.join(rel_dir(domain, category, name)).join(".manager/meta.yaml"),
    )
    .unwrap()
}

fn meta_of(root: &Path, rel: &str) -> manager::SkillMeta {
    manager::MetaFile::load(&root.join(rel))
        .unwrap()
        .meta()
        .unwrap()
}

fn tags(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

// ---------------------------------------------------------------------------
// 识别谓词（skill-detection.md）
// ---------------------------------------------------------------------------

#[test]
fn only_skill_md_or_lowercase_marker_is_a_skill_dir() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();

    // 命中即不向下递归：技能内部的 references/ 不会被当成独立技能。
    write_file(root, "code/skill-a/SKILL.md", "a");
    write_file(root, "code/skill-a/references/inner/SKILL.md", "inner");
    // README/CLAUDE 不作标记。
    write_file(root, "code/docs-only/README.md", "readme");
    write_file(root, "code/claude-only/CLAUDE.md", "claude");
    // 兼容入口 skill.md 可识别。
    write_file(root, "lobster/lower/skill.md", "lower");
    // 超过一级分类的嵌套不是技能。
    write_file(root, "code/tool/deep/nested/SKILL.md", "deep");
    // `.git` 子树被跳过。
    write_file(root, "code/.git/objects/SKILL.md", "git");

    let scanned = scan::scan_vault(root).unwrap();
    let keys: Vec<String> = scanned
        .iter()
        .map(|s| format!("{}/{}", s.domain.as_str(), s.name))
        .collect();
    assert!(keys.contains(&"code/skill-a".to_string()), "{keys:?}");
    assert!(keys.contains(&"lobster/lower".to_string()), "{keys:?}");
    for absent in ["inner", "docs-only", "claude-only", "nested", "objects"] {
        assert!(
            !keys.iter().any(|key| key.contains(absent)),
            "{absent} 不应被识别为技能：{keys:?}"
        );
    }

    // 目录名即技能身份，分类由目录层级派生。
    let entry = scanned.iter().find(|s| s.name == "skill-a").unwrap();
    assert_eq!(entry.category, None);
    let lower = scanned.iter().find(|s| s.name == "lower").unwrap();
    assert_eq!(lower.domain, Domain::Lobster);
    assert_eq!(lower.entry_file, "skill.md");
}

#[test]
fn directory_name_is_the_skill_identity_not_frontmatter_name() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    write_skill(
        root,
        "code",
        None,
        "dir-name",
        "---\nname: frontmatter-name\ndescription: 来自 frontmatter\n---\n正文\n",
    );

    let index = SkillIndex::build(root).unwrap();
    assert!(index.get(&SkillRef::parse("code/dir-name").unwrap()).is_some());
    assert!(index.get(&SkillRef::parse("code/frontmatter-name").unwrap()).is_none());
    let entry = index.get(&SkillRef::parse("code/dir-name").unwrap()).unwrap();
    assert_eq!(entry.description.as_deref(), Some("来自 frontmatter"));
}

// ---------------------------------------------------------------------------
// 分类移动
// ---------------------------------------------------------------------------

#[test]
fn move_category_moves_directory_without_renaming() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    write_skill(root, "code", Some("工具"), "alpha", "正文");

    let reference = SkillRef::parse("code/alpha").unwrap();
    let moved = manage::move_to_category(root, &reference, Some("脚本")).unwrap();
    assert!(moved.ends_with("code/脚本/alpha"), "{}", moved.display());

    assert!(!root.join("code/工具/alpha").exists());
    assert!(root.join("code/脚本/alpha/SKILL.md").exists());

    let index = SkillIndex::build(root).unwrap();
    let entry = index.get(&reference).expect("移动后技能仍可解析");
    assert_eq!(entry.name, "alpha");
    assert_eq!(entry.category.as_deref(), Some("脚本"));
    assert_eq!(entry.vault_rel_dir, "code/脚本/alpha");

    let query = SearchQuery {
        domain: Some(Domain::Code),
        category: CategoryFilter::Named("脚本".to_string()),
        ..SearchQuery::default()
    };
    assert_eq!(index.search(&query).len(), 1);

    // 移回未分类。
    manage::move_to_category(root, &reference, None).unwrap();
    assert!(root.join("code/alpha/SKILL.md").exists());
    assert_eq!(
        SkillIndex::build(root).unwrap().get(&reference).unwrap().category,
        None
    );
}

#[test]
fn move_to_occupied_directory_is_blocked_with_locatable_reason() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    write_skill(root, "code", Some("工具"), "alpha", "a");
    // 目标目录已存在（非空），直接 rename 会失败或误并。
    write_file(root, "code/脚本/alpha/note.txt", "占位");

    let reference = SkillRef::parse("code/alpha").unwrap();
    let err = manage::move_to_category(root, &reference, Some("脚本")).unwrap_err();
    let message = err.to_string();
    assert!(message.contains("脚本"), "{message}");
    assert!(message.contains("alpha"), "错误应可定位到目标：{message}");
    // 原目录未被破坏。
    assert!(root.join("code/工具/alpha/SKILL.md").exists());
}

// ---------------------------------------------------------------------------
// 同域唯一性
// ---------------------------------------------------------------------------

#[test]
fn same_domain_names_are_unique_case_insensitively_and_cross_domain_allowed() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    write_skill(root, "code", None, "alpha", "a");

    let duplicate = manage::ensure_unique_name(root, Domain::Code, "ALPHA", None).unwrap_err();
    assert!(duplicate.to_string().contains("同名技能"), "{duplicate}");
    // 跨域同名允许。
    manage::ensure_unique_name(root, Domain::Lobster, "alpha", None).unwrap();
    // 新名字可用。
    manage::ensure_unique_name(root, Domain::Code, "beta", None).unwrap();
    // 忽略自身后可用（供原地改写复用）。
    manage::ensure_unique_name(root, Domain::Code, "alpha", Some(&root.join("code/alpha")))
        .unwrap();
}

// ---------------------------------------------------------------------------
// 标签
// ---------------------------------------------------------------------------

#[test]
fn tag_candidates_follow_selected_category() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    write_skill(root, "code", Some("工具"), "alpha", "a");
    write_meta(root, "code", Some("工具"), "alpha", "tags: [y, x]\n");
    write_skill(root, "code", Some("工具"), "beta", "b");
    write_meta(root, "code", Some("工具"), "beta", "tags: [y, z]\n");
    write_skill(root, "code", Some("其他"), "gamma", "c");
    write_meta(root, "code", Some("其他"), "gamma", "tags: [w]\n");
    write_skill(root, "lobster", None, "delta", "d");
    write_meta(root, "lobster", None, "delta", "tags: [l]\n");

    let index = SkillIndex::build(root).unwrap();

    // 选中分类时只含该分类内实际标签。
    assert_eq!(
        index.tag_candidates(Some(Domain::Code), &CategoryFilter::Named("工具".into())),
        tags(&["x", "y", "z"])
    );
    assert_eq!(
        index.tag_candidates(Some(Domain::Code), &CategoryFilter::Named("其他".into())),
        tags(&["w"])
    );
    // "全部分类"恢复该库全部。
    assert_eq!(
        index.tag_candidates(Some(Domain::Code), &CategoryFilter::All),
        tags(&["w", "x", "y", "z"])
    );
    // 各库分别筛选；跨域为并集。
    assert_eq!(
        index.tag_candidates(Some(Domain::Lobster), &CategoryFilter::All),
        tags(&["l"])
    );
    assert_eq!(
        index.tag_candidates(None, &CategoryFilter::All),
        tags(&["l", "w", "x", "y", "z"])
    );
}

#[test]
fn batch_tag_add_and_detach_keep_other_tags() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    write_skill(root, "code", None, "alpha", "a");
    write_meta(root, "code", None, "alpha", "tags: [x, y]\n");
    write_skill(root, "lobster", None, "alpha", "a");
    write_meta(root, "lobster", None, "alpha", "tags: [y]\n");

    let code_ref = SkillRef::parse("code/alpha").unwrap();
    let lobster_ref = SkillRef::parse("lobster/alpha").unwrap();

    let items = manage::add_tags_many(
        root,
        &[code_ref.clone(), lobster_ref.clone()],
        &tags(&["z"]),
    );
    assert!(items.iter().all(|item| item.ok), "{items:?}");
    assert_eq!(meta_of(root, "code/alpha").tags, tags(&["x", "y", "z"]));
    assert_eq!(meta_of(root, "lobster/alpha").tags, tags(&["y", "z"]));

    let items = manage::detach_tags_many(root, &[code_ref.clone()], &tags(&["x"]));
    assert!(items.iter().all(|item| item.ok), "{items:?}");
    assert_eq!(meta_of(root, "code/alpha").tags, tags(&["y", "z"]));

    // 替换集合只在 set 时发生，且不跨域连带。
    manage::set_tags(root, &code_ref, &tags(&["only"])).unwrap();
    assert_eq!(meta_of(root, "code/alpha").tags, tags(&["only"]));
    assert_eq!(meta_of(root, "lobster/alpha").tags, tags(&["y", "z"]));
}

#[test]
fn tag_rename_and_remove_cover_both_domains_without_deleting_skills() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    write_skill(root, "code", None, "alpha", "a");
    write_meta(root, "code", None, "alpha", "tags: [旧, 保留]\n");
    write_skill(root, "lobster", None, "beta", "b");
    write_meta(root, "lobster", None, "beta", "tags: [旧]\n");

    assert_eq!(manage::rename_tag(root, "旧", "新").unwrap(), 2);
    assert_eq!(meta_of(root, "code/alpha").tags, tags(&["新", "保留"]));
    assert_eq!(meta_of(root, "lobster/beta").tags, tags(&["新"]));

    assert_eq!(manage::remove_tag(root, "新").unwrap(), 2);
    assert_eq!(meta_of(root, "code/alpha").tags, tags(&["保留"]));
    assert!(meta_of(root, "lobster/beta").tags.is_empty());

    // 删除标签只解除关联，技能仍在。
    let index = SkillIndex::build(root).unwrap();
    assert!(index.get(&SkillRef::parse("code/alpha").unwrap()).is_some());
    assert!(index.get(&SkillRef::parse("lobster/beta").unwrap()).is_some());
}

// ---------------------------------------------------------------------------
// 支持声明与未知字段保留
// ---------------------------------------------------------------------------

#[test]
fn support_declaration_defaults_to_all_and_empty_is_distinct() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    write_skill(root, "code", None, "alpha", "a");
    let reference = SkillRef::parse("code/alpha").unwrap();

    // 缺省=该域内全支持。
    assert_eq!(meta_of(root, "code/alpha").agents, None);
    let index = SkillIndex::build(root).unwrap();
    let entry = index.get(&reference).unwrap();
    assert!(entry.supports_agent("codex", Domain::Code));
    assert!(entry.supports_agent("zcode", Domain::Code));
    // 缺省不等于任意 Agent：编码库技能不因此支持龙虾 Agent。
    assert!(!entry.supports_agent("hermes", Domain::Lobster));

    // 显式空集合=不支持任何目标。
    manage::set_support(root, &reference, SupportDecl::Empty, SupportDecl::Empty).unwrap();
    let index = SkillIndex::build(root).unwrap();
    let entry = index.get(&reference).unwrap();
    assert!(!entry.supports_agent("codex", Domain::Code));
    assert_eq!(entry.agents, Some(Vec::<String>::new()));

    // 明确列表；恢复"全部"删除键。
    manage::set_support(
        root,
        &reference,
        SupportDecl::List(vec!["codex".into()]),
        SupportDecl::All,
    )
    .unwrap();
    let index = SkillIndex::build(root).unwrap();
    let entry = index.get(&reference).unwrap();
    assert!(entry.supports_agent("codex", Domain::Code));
    assert!(!entry.supports_agent("zcode", Domain::Code));
    assert_eq!(entry.oses, None);

    let raw = read_meta(root, "code", None, "alpha");
    assert!(raw.contains("codex"), "{raw}");
    assert!(!raw.contains("oses"), "恢复全部应删除该键：{raw}");
}

#[test]
fn unknown_yaml_fields_survive_metadata_write_back() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    write_skill(root, "code", None, "alpha", "a");
    write_meta(
        root,
        "code",
        None,
        "alpha",
        "foo: bar\ntags: [x]\ncomment: 旧注释\nfuture_field:\n  nested: 1\n",
    );

    let reference = SkillRef::parse("code/alpha").unwrap();
    manage::set_comment(root, &reference, Some("新注释")).unwrap();

    let raw = read_meta(root, "code", None, "alpha");
    assert!(raw.contains("foo: bar"), "未知字段应保留：{raw}");
    assert!(raw.contains("future_field"), "未知结构应保留：{raw}");
    assert!(raw.contains("nested: 1"), "未知结构应保留：{raw}");
    assert!(raw.contains("新注释"), "{raw}");
    assert!(raw.contains("tags"), "{raw}");

    let meta = meta_of(root, "code/alpha");
    assert_eq!(meta.tags, tags(&["x"]));
    assert_eq!(meta.comment.as_deref(), Some("新注释"));

    // 原地改键，不重排：foo 仍在 tags 之前。
    assert!(raw.find("foo").unwrap() < raw.find("tags").unwrap(), "{raw}");
}

// ---------------------------------------------------------------------------
// 译文时效
// ---------------------------------------------------------------------------

#[test]
fn translation_is_read_only_and_stale_after_original_changes() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    let dir = write_skill(root, "code", None, "alpha", "原文第一版\n");
    let reference = SkillRef::parse("code/alpha").unwrap();

    // 无译文：不提示待更新。
    let index = SkillIndex::build(root).unwrap();
    let entry = index.get(&reference).unwrap();
    assert!(!entry.has_translation);
    assert!(!entry.translation_stale);
    assert!(manager::read_translation(&dir).unwrap().is_none());

    // 保存译文：不改写原文，记录当时原文摘要。
    manager::write_translation(&dir, "译文第一版\n").unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("SKILL.md")).unwrap(),
        "原文第一版\n"
    );
    let index = SkillIndex::build(root).unwrap();
    let entry = index.get(&reference).unwrap();
    assert!(entry.has_translation);
    assert!(!entry.translation_stale, "按当前原文保存后不应提示待更新");
    assert_eq!(
        manager::read_translation(&dir).unwrap().as_deref(),
        Some("译文第一版\n")
    );

    // 原文变化而未维护译文 → 提示待更新，译文保留。
    std::fs::write(dir.join("SKILL.md"), "原文第二版\n").unwrap();
    let index = SkillIndex::build(root).unwrap();
    let entry = index.get(&reference).unwrap();
    assert!(entry.has_translation);
    assert!(entry.translation_stale, "原文变化后应提示译文待更新");
    assert_eq!(
        manager::read_translation(&dir).unwrap().as_deref(),
        Some("译文第一版\n")
    );

    // 按当前原文重新保存译文即解除提示。
    manager::write_translation(&dir, "译文第二版\n").unwrap();
    let index = SkillIndex::build(root).unwrap();
    assert!(!index.get(&reference).unwrap().translation_stale);

    // 删除译文后不再提示，时效依据一并清除。
    manager::remove_translation(&dir).unwrap();
    let index = SkillIndex::build(root).unwrap();
    let entry = index.get(&reference).unwrap();
    assert!(!entry.has_translation);
    assert!(!entry.translation_stale);
    let raw = read_meta(root, "code", None, "alpha");
    assert!(!raw.contains("translation_source_hash"), "{raw}");
}

#[test]
fn only_original_content_changes_mark_translation_stale() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    let dir = write_skill(root, "code", None, "alpha", "原文\n");
    manager::write_translation(&dir, "译文\n").unwrap();
    let reference = SkillRef::parse("code/alpha").unwrap();

    // 只改管理资料不触发提示。
    manage::set_comment(root, &reference, Some("说明")).unwrap();
    manage::add_tags_many(root, std::slice::from_ref(&reference), &tags(&["x"]));
    let index = SkillIndex::build(root).unwrap();
    let entry = index.get(&reference).unwrap();
    assert_eq!(entry.comment.as_deref(), Some("说明"));
    assert_eq!(entry.tags, tags(&["x"]));
    assert!(!entry.translation_stale);

    // 改覆盖层也不触发提示。
    manager::write_overlay(&dir, None, None, "x.sh", b"overlay").unwrap();
    let index = SkillIndex::build(root).unwrap();
    assert!(!index.get(&reference).unwrap().translation_stale);

    // 改原文内容才触发。
    std::fs::write(dir.join("SKILL.md"), "原文改\n").unwrap();
    let index = SkillIndex::build(root).unwrap();
    assert!(index.get(&reference).unwrap().translation_stale);
}

// ---------------------------------------------------------------------------
// 组合搜索
// ---------------------------------------------------------------------------

#[test]
fn combined_search_and_untagged_filter() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    write_skill(
        root,
        "code",
        Some("工具"),
        "alpha",
        "---\nname: alpha\ndescription: 解析依赖\n---\n正文\n",
    );
    write_meta(
        root,
        "code",
        Some("工具"),
        "alpha",
        "tags: [x]\ncomment: 检查依赖\n",
    );
    write_file(
        root,
        "code/工具/alpha/.manager/source.yaml",
        "repo: https://example.invalid/skills.git\nsubpath: tools/alpha\nversion: 0123456789abcdef0123456789abcdef01234567\nregistered_at: 2026-10-15\n",
    );
    write_skill(root, "code", None, "beta", "正文\n");

    let index = SkillIndex::build(root).unwrap();
    let find = |query: SearchQuery| -> Vec<String> {
        index
            .search(&query)
            .iter()
            .map(|entry| format!("{}/{}", entry.domain.as_str(), entry.name))
            .collect()
    };
    let text = |needle: &str| SearchQuery {
        query: Some(needle.to_string()),
        ..SearchQuery::default()
    };

    assert_eq!(find(text("解析依赖")), vec!["code/alpha"]); // description
    assert_eq!(find(text("检查依赖")), vec!["code/alpha"]); // comment
    assert_eq!(find(text("example.invalid")), vec!["code/alpha"]); // 来源
    assert_eq!(find(text("工具")), vec!["code/alpha"]); // 分类
    assert_eq!(find(text("x")), vec!["code/alpha"]); // 标签

    assert_eq!(
        find(SearchQuery {
            require_untagged: true,
            ..SearchQuery::default()
        }),
        vec!["code/beta"]
    );
    assert_eq!(
        find(SearchQuery {
            tags: vec!["x".to_string()],
            ..SearchQuery::default()
        }),
        vec!["code/alpha"]
    );
    assert_eq!(
        find(SearchQuery {
            domain: Some(Domain::Lobster),
            ..SearchQuery::default()
        }),
        Vec::<String>::new()
    );
    // 支持声明筛选：缺省=该域内全支持（未分类的 beta 也在内）。
    assert_eq!(
        find(SearchQuery {
            agent: Some(AgentSupport {
                key: "codex".to_string(),
                domain: Domain::Code,
            }),
            ..SearchQuery::default()
        }),
        vec!["code/beta", "code/alpha"]
    );
    // 龙虾 Agent 不匹配编码库技能。
    assert_eq!(
        find(SearchQuery {
            agent: Some(AgentSupport {
                key: "hermes".to_string(),
                domain: Domain::Lobster,
            }),
            ..SearchQuery::default()
        }),
        Vec::<String>::new()
    );
    // 未分类筛选。
    assert_eq!(
        find(SearchQuery {
            category: CategoryFilter::Uncategorized,
            ..SearchQuery::default()
        }),
        vec!["code/beta"]
    );
}

#[test]
fn entry_metadata_exposes_vault_path_description_and_source() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    write_skill(root, "code", Some("工具"), "alpha", "---\ndescription: 说明\n---\n");
    write_file(
        root,
        "code/工具/alpha/.manager/source.yaml",
        "repo: https://example.invalid/skills.git\nref: main\nsubpath: tools/alpha\nversion: 0123456789abcdef0123456789abcdef01234567\nregistered_at: 2026-10-15\n",
    );

    let index = SkillIndex::build(root).unwrap();
    let entry = index.get(&SkillRef::parse("code/alpha").unwrap()).unwrap();
    assert_eq!(entry.vault_rel_dir, "code/工具/alpha");
    assert_eq!(entry.description.as_deref(), Some("说明"));
    let source = entry.source.as_ref().unwrap();
    assert_eq!(source.repo, "https://example.invalid/skills.git");
    assert_eq!(source.reference.as_deref(), Some("main"));
    assert_eq!(source.subpath, "tools/alpha");
    assert_eq!(source.version.len(), 40);

    // 无上游的自建技能没有 source.yaml。
    write_skill(root, "code", None, "self-made", "正文\n");
    let index = SkillIndex::build(root).unwrap();
    assert!(
        index
            .get(&SkillRef::parse("code/self-made").unwrap())
            .unwrap()
            .source
            .is_none()
    );
}

#[test]
fn unreadable_manager_file_is_reported_not_folded_into_empty_metadata() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    let dir = write_skill(root, "code", None, "alpha", "a");
    // 制造一个目录占住 meta.yaml 的位置：读取失败不能折叠成"无资料"。
    std::fs::create_dir_all(manager::meta_path(&dir)).unwrap();

    // 加载器报错。
    assert!(manager::MetaFile::load(&dir).is_err());

    // 索引仍可用（一个坏文件不让整库打不开），但该技能带有可见的读取错误。
    let index = SkillIndex::build(root).unwrap();
    let reference = SkillRef::parse("code/alpha").unwrap();
    let entry = index.get(&reference).expect("技能仍应列出");
    assert!(entry.manager_error.is_some(), "{entry:?}");

    // 写回被阻止，不覆盖读不出的原始内容。
    assert!(manage::set_comment(root, &reference, Some("x")).is_err());
}

// ---------------------------------------------------------------------------
// 覆盖层布局
// ---------------------------------------------------------------------------

#[test]
fn overlay_layout_uses_underscore_for_generic_dimensions() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    let dir = write_skill(root, "code", None, "alpha", "a");

    manager::write_overlay(&dir, None, None, "scripts/x.sh", b"generic").unwrap();
    manager::write_overlay(&dir, Some("codex"), None, "SKILL.md", b"codex-only").unwrap();
    manager::write_overlay(&dir, None, Some("macos"), "x.sh", b"macos-only").unwrap();

    assert!(dir.join(".manager/overlay/_/_/scripts/x.sh").exists());
    assert!(dir.join(".manager/overlay/codex/_/SKILL.md").exists());
    assert!(dir.join(".manager/overlay/_/macos/x.sh").exists());

    let mut listed = manager::list_overlays(&dir).unwrap();
    listed.sort_by(|a, b| a.rel.cmp(&b.rel));
    assert_eq!(listed.len(), 3, "{listed:?}");
    let skill_md = listed.iter().find(|e| e.rel == "SKILL.md").unwrap();
    assert_eq!(skill_md.agent.as_deref(), Some("codex"));
    assert_eq!(skill_md.os, None);
    let generic = listed.iter().find(|e| &e.rel == "scripts/x.sh").unwrap();
    assert_eq!(generic.agent, None);
    assert_eq!(generic.os, None);
    let macos = listed.iter().find(|e| &e.rel == "x.sh").unwrap();
    assert_eq!(macos.agent, None);
    assert_eq!(macos.os.as_deref(), Some("macos"));

    assert_eq!(
        manager::read_overlay(&dir, Some("codex"), None, "SKILL.md")
            .unwrap()
            .unwrap(),
        b"codex-only".to_vec()
    );
    assert!(manager::read_overlay(&dir, Some("codex"), None, "scripts/x.sh").unwrap().is_none());

    // 路径逃逸被拒绝。
    assert!(manager::write_overlay(&dir, None, None, "../escape.sh", b"x").is_err());
    assert!(manager::write_overlay(&dir, None, None, "/abs.sh", b"x").is_err());
    assert!(manager::write_overlay(&dir, None, None, "a/../../escape.sh", b"x").is_err());

    assert!(manager::remove_overlay(&dir, Some("codex"), None, "SKILL.md").unwrap());
    assert!(!dir.join(".manager/overlay/codex/_/SKILL.md").exists());
    assert!(!manager::remove_overlay(&dir, Some("codex"), None, "SKILL.md").unwrap());
}
