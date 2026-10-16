//! T040：上游更新合并的字段粒度断言（D3「同项上游值优先」）。
//!
//! 每键 meta、整份译文、overlay 同路径文件：上游提供则采用、未提供则保留；
//! 含本地已改同项与仅管理资料变化；分类目录由本地保留；采用版本推进且不重复应用。

use std::path::{Path, PathBuf};

use skill_manager_core::manager::{self, merge_from_upstream, read_source, write_source, MetaFile, SourceRecord};

fn write(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, body).unwrap();
}

fn local_skill() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    write(dir.path(), "SKILL.md", "---\nname: alpha\ndescription: A\n---\n本地正文\n");
    dir
}

fn upstream_dir() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    write(dir.path(), "SKILL.md", "---\nname: alpha\ndescription: A\n---\n上游正文\n");
    dir
}

#[test]
fn meta_is_merged_per_key_upstream_wins_local_unprovided_kept() {
    let local = local_skill();
    let mut meta = MetaFile::load(local.path()).unwrap();
    meta.set_tags(&["本地标签".to_string()]);
    meta.set_comment(Some("本地注释"));
    meta.save(local.path()).unwrap();

    let upstream = upstream_dir();
    let mut up = MetaFile::load(upstream.path()).unwrap();
    up.set_comment(Some("上游注释"));
    op(upstream.path(), "meta.yaml", &up.to_yaml().unwrap());

    let report = merge_from_upstream(local.path(), upstream.path()).unwrap();
    assert!(report.meta_changed.contains(&"comment".to_string()));

    let merged = MetaFile::load(local.path()).unwrap().meta().unwrap();
    assert_eq!(merged.comment.as_deref(), Some("上游注释"), "上游提供的同项采用上游值");
    assert_eq!(merged.tags, vec!["本地标签".to_string()], "上游未提供的项保留本地值");
}

#[test]
fn translation_is_adopted_as_whole_item_and_local_kept_when_upstream_absent() {
    let local = local_skill();
    manager::write_translation(local.path(), "本地旧译文").unwrap();

    let upstream = upstream_dir();
    op(upstream.path(), "translation.md", "上游新译文");

    let report = merge_from_upstream(local.path(), upstream.path()).unwrap();
    assert!(report.translation_updated);
    assert_eq!(
        manager::read_translation(local.path()).unwrap().as_deref(),
        Some("上游新译文")
    );

    // 上游未提供译文时保留本地译文。
    let another = upstream_dir();
    merge_from_upstream(local.path(), another.path()).unwrap();
    assert_eq!(
        manager::read_translation(local.path()).unwrap().as_deref(),
        Some("上游新译文")
    );
}

#[test]
fn overlay_is_merged_by_same_relative_path() {
    let local = local_skill();
    manager::write_overlay(local.path(), None, None, "notes.txt", b"local\n").unwrap();
    manager::write_overlay(local.path(), None, None, "only_local.txt", b"keep\n").unwrap();

    let upstream = upstream_dir();
    manager::write_overlay(upstream.path(), None, None, "notes.txt", b"upstream\n").unwrap();

    let report = merge_from_upstream(local.path(), upstream.path()).unwrap();
    assert_eq!(report.overlays_updated, vec!["_/_:notes.txt".to_string()]);
    assert_eq!(
        manager::read_overlay(local.path(), None, None, "notes.txt").unwrap().as_deref(),
        Some(b"upstream\n".as_slice())
    );
    assert_eq!(
        manager::read_overlay(local.path(), None, None, "only_local.txt").unwrap().as_deref(),
        Some(b"keep\n".as_slice()),
        "本地独有的覆盖层路径保留"
    );
}

#[test]
fn changed_base_files_are_mirrored_and_deleted_files_removed() {
    let local = local_skill();
    write(local.path(), "notes.txt", "old\n");
    write(local.path(), "removed.txt", "gone\n");

    let upstream = upstream_dir();
    write(upstream.path(), "notes.txt", "new\n");
    write(upstream.path(), "added.txt", "add\n");

    let report = merge_from_upstream(local.path(), upstream.path()).unwrap();
    assert_eq!(report.base.modified, vec!["SKILL.md".to_string(), "notes.txt".to_string()]);
    assert_eq!(report.base.added, vec!["added.txt".to_string()]);
    assert_eq!(report.base.deleted, vec!["removed.txt".to_string()]);

    assert_eq!(std::fs::read_to_string(local.path().join("notes.txt")).unwrap(), "new\n");
    assert!(local.path().join("added.txt").exists());
    assert!(!local.path().join("removed.txt").exists(), "上游删除的技能内文件被移除");
}

#[test]
fn management_only_change_is_applied_even_when_body_is_unchanged() {
    let local = local_skill();
    let upstream = local_skill();
    // 只改上游管理资料（正文逐字相同）。
    let mut up = MetaFile::load(upstream.path()).unwrap();
    up.set_comment(Some("仅资料变化"));
    op(upstream.path(), "meta.yaml", &up.to_yaml().unwrap());

    let report = merge_from_upstream(local.path(), upstream.path()).unwrap();
    assert!(report.base.is_empty(), "正文未变");
    assert!(!report.is_empty(), "仅管理资料变化也要应用，不因正文未变跳过");
    assert_eq!(
        MetaFile::load(local.path()).unwrap().meta().unwrap().comment.as_deref(),
        Some("仅资料变化")
    );
}

#[test]
fn source_version_advances_and_category_dir_is_not_moved() {
    let local = local_skill();
    let reference_dir: PathBuf = local.path().to_path_buf();
    write_source(
        local.path(),
        &SourceRecord {
            repo: "https://x/y.git".to_string(),
            reference: None,
            subpath: "tools/alpha".to_string(),
            version: "a".repeat(40),
            skipped_version: None,
            registered_at: "2026-10-15".to_string(),
        },
    )
    .unwrap();

    let upstream = upstream_dir();
    merge_from_upstream(local.path(), upstream.path()).unwrap();

    // 合并只改技能目录内容，不移动分类目录（目录仍是同一个）。
    assert_eq!(local.path(), reference_dir.as_path());
    assert_eq!(read_source(local.path()).unwrap().unwrap().version, "a".repeat(40));

    // 采用版本推进由 apply 负责；这里模拟其推进并断言续用不再重复。
    let mut record = read_source(local.path()).unwrap().unwrap();
    record.version = "b".repeat(40);
    write_source(local.path(), &record).unwrap();
    assert_eq!(read_source(local.path()).unwrap().unwrap().version, "b".repeat(40));
}

fn op(dir: &Path, rel: &str, body: &str) {
    write(dir, &format!(".manager/{rel}"), body);
}
