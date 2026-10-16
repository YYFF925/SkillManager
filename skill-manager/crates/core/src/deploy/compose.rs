//! 部署产物合成（FR-016、FR-008）：原始文件 → 目标/系统覆盖层 → 中文注释。
//!
//! 合成输入永远是**库内原始文件 + 覆盖层**，从不以已注入注释的产物为输入，
//! 因此库内原始正文不会被注释改写（FR-006）。设计依据见 research.md §四 D10/D11/D14。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;
use yaml_serde::{Mapping, Value};

use crate::error::{CoreError, Result};
use crate::manager::{self, GENERIC_DIMENSION};

/// 中文注释与原 `description` 之间的分隔符。
pub const COMMENT_SEPARATOR: char = '｜';

/// 合成基础树时跳过的目录名。
pub const SKIP_DIRS: [&str; 2] = [".git", ".manager"];
/// 合成与指纹都忽略的平台噪声文件。
pub const SKIP_FILE: &str = ".DS_Store";
/// 入口文件候选，`SKILL.md` 优先（与扫描谓词一致）。
pub const ENTRY_FILES: [&str; 2] = ["SKILL.md", "skill.md"];

/// 产物中的一个文件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedFile {
    pub rel: String,
    pub bytes: Vec<u8>,
}

/// 合成结果。
#[derive(Clone, Debug)]
pub struct ComposedArtifact {
    pub files: Vec<ComposedFile>,
    /// 合格产物指纹（已含注释注入与覆盖层）。
    pub artifact_hash: String,
    /// 合成输入指纹（基础树 + 生效覆盖层 + 注释）。
    pub source_hash: String,
    /// 本次实际生效的覆盖层条目（`<agent>/<os>:<相对路径>`）。
    pub applied_overlays: Vec<String>,
    /// 注释未注入时的如实说明。
    pub comment_note: Option<String>,
}

impl ComposedArtifact {
    pub fn paths(&self) -> Vec<String> {
        self.files.iter().map(|file| file.rel.clone()).collect()
    }

    pub fn read(&self, rel: &str) -> Option<&[u8]> {
        self.files
            .iter()
            .find(|file| file.rel == rel)
            .map(|file| file.bytes.as_slice())
    }

    pub fn text(&self, rel: &str) -> Option<String> {
        self.read(rel)
            .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
    }
}

/// 一项生效的覆盖层文件，`rank` 越大越具体。
struct OverlayApplication {
    rank: u8,
    rel: String,
    bytes: Vec<u8>,
    agent: Option<String>,
    os: Option<String>,
}

/// 合成技能产物。
///
/// `agent` 为 `None` 表示只取通用 Agent 维度（共享目录）；`os` 为当前系统标识。
pub fn compose_skill(
    skill_dir: &Path,
    agent: Option<&str>,
    os: &str,
    comment: Option<&str>,
) -> Result<ComposedArtifact> {
    let base = collect_base_files(skill_dir)?;
    let overlays = effective_overlays(skill_dir, agent, os)?;

    let mut merged: BTreeMap<String, Vec<u8>> = base.iter().cloned().collect();
    let mut applied_overlays = Vec::new();
    for overlay in &overlays {
        apply_override(&mut merged, &overlay.rel, overlay.bytes.clone());
        applied_overlays.push(format!(
            "{}/{}:{}",
            overlay.agent.as_deref().unwrap_or(GENERIC_DIMENSION),
            overlay.os.as_deref().unwrap_or(GENERIC_DIMENSION),
            overlay.rel
        ));
    }

    let source_hash = input_hash(&base, &overlays, comment);

    let mut comment_note = None;
    if let Some(comment) = comment.map(str::trim).filter(|text| !text.is_empty()) {
        match find_entry(&merged) {
            Some((rel, bytes)) => match inject_comment(bytes, comment)? {
                Injection::Changed(new_bytes) => {
                    merged.insert(rel.to_string(), new_bytes);
                }
                Injection::Unchanged => {}
                Injection::Skipped(note) => comment_note = Some(note),
            },
            None => {
                comment_note = Some("产物中没有 SKILL.md/skill.md，未注入中文注释".to_string())
            }
        }
    }

    let files: Vec<ComposedFile> = merged
        .into_iter()
        .map(|(rel, bytes)| ComposedFile { rel, bytes })
        .collect();
    let artifact_hash = hash_files(&files);

    Ok(ComposedArtifact {
        files,
        artifact_hash,
        source_hash,
        applied_overlays,
        comment_note,
    })
}

// ---------------------------------------------------------------------------
// 基础树与覆盖层
// ---------------------------------------------------------------------------

/// 收集技能基础树（排除 `.manager`/`.git`/`.DS_Store`，不跟随符号链接）。
pub fn collect_base_files(skill_dir: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    collect_dir_files(skill_dir, true)
}

fn collect_dir_files(root: &Path, skip_manager: bool) -> Result<Vec<(String, Vec<u8>)>> {
    let mut out = Vec::new();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !is_skipped_dir(entry, skip_manager));
    for entry in walker {
        let entry = entry.map_err(|err| {
            CoreError::other(format!("读取技能目录失败（{}）：{err}", root.display()))
        })?;
        // 符号链接与目录都跳过；只复制普通文件。
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.file_name() == SKIP_FILE {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(root) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if rel.is_empty() {
            continue;
        }
        let bytes =
            std::fs::read(entry.path()).map_err(|err| CoreError::io(entry.path(), err))?;
        out.push((rel, bytes));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn is_skipped_dir(entry: &walkdir::DirEntry, skip_manager: bool) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    name == ".git" || (skip_manager && name == SKIP_DIRS[1])
}

fn effective_overlays(
    skill_dir: &Path,
    agent: Option<&str>,
    os: &str,
) -> Result<Vec<OverlayApplication>> {
    let mut by_rel: BTreeMap<String, OverlayApplication> = BTreeMap::new();
    for entry in manager::list_overlays(skill_dir)? {
        if entry.rel.split('/').any(|part| part.is_empty() || part == "..") {
            continue;
        }
        if let Some(declared) = &entry.agent {
            if Some(declared.as_str()) != agent {
                continue;
            }
        }
        if let Some(declared) = &entry.os {
            if declared != os {
                continue;
            }
        }
        let bytes = manager::read_overlay(
            skill_dir,
            entry.agent.as_deref(),
            entry.os.as_deref(),
            &entry.rel,
        )?
        .ok_or_else(|| {
            CoreError::other(format!("覆盖层文件不可读：{}", entry.rel))
        })?;
        // 具体维度数量多者优先；同数量时 Agent 维度优先于 OS 维度（D11）。
        let rank = u8::from(entry.agent.is_some()) * 2 + u8::from(entry.os.is_some());
        by_rel.insert(
            entry.rel.clone(),
            OverlayApplication {
                rank,
                rel: entry.rel,
                bytes,
                agent: entry.agent,
                os: entry.os,
            },
        );
    }

    let mut overlays: Vec<OverlayApplication> = by_rel.into_values().collect();
    // 按具体程度升序应用，后者覆盖前者。
    overlays.sort_by(|a, b| a.rank.cmp(&b.rank).then(a.rel.cmp(&b.rel)));
    Ok(overlays)
}

/// 覆盖层“同路径文件一项”：写入 `rel` 时同时移除与之无法共存的旧条目，
/// 保证产物仍是合法目录树（D11 同名文件/目录）。
fn apply_override(map: &mut BTreeMap<String, Vec<u8>>, rel: &str, bytes: Vec<u8>) {
    map.remove(rel);
    let dir_prefix = format!("{rel}/");
    map.retain(|existing, _| !existing.starts_with(&dir_prefix));
    map.retain(|existing, _| !rel.starts_with(&format!("{existing}/")));
    map.insert(rel.to_string(), bytes);
}

fn find_entry(files: &BTreeMap<String, Vec<u8>>) -> Option<(&str, &[u8])> {
    for candidate in ENTRY_FILES {
        if let Some(bytes) = files.get(candidate) {
            return Some((candidate, bytes.as_slice()));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// 中文注释注入
// ---------------------------------------------------------------------------

enum Injection {
    Changed(Vec<u8>),
    Unchanged,
    Skipped(String),
}

fn inject_comment(bytes: &[u8], comment: &str) -> Result<Injection> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Ok(Injection::Skipped(
            "入口文件不是有效 UTF-8，未注入中文注释".to_string(),
        ));
    };
    let (changed, note) = inject_comment_text(text, comment)?;
    Ok(match (changed, note) {
        (Some(text), _) => Injection::Changed(text.into_bytes()),
        (None, Some(note)) => Injection::Skipped(note),
        (None, None) => Injection::Unchanged,
    })
}

/// 把注释作为一个前缀写进 frontmatter 的 `description`。
///
/// 返回 `(替换后的全文, 未注入原因)`：`(None, None)` 表示无需改动。
fn inject_comment_text(text: &str, comment: &str) -> Result<(Option<String>, Option<String>)> {
    let prefix = format!("{comment}{COMMENT_SEPARATOR}");
    match split_frontmatter(text) {
        Some((block, body)) => {
            let mut map = match yaml_serde::from_str::<Value>(block) {
                Ok(Value::Mapping(mapping)) => mapping,
                Ok(Value::Null) => Mapping::new(),
                Ok(_) => {
                    return Ok((
                        None,
                        Some("frontmatter 顶层不是映射，未注入中文注释".to_string()),
                    ));
                }
                Err(err) => {
                    return Ok((
                        None,
                        Some(format!("frontmatter 无法解析，未注入中文注释：{err}")),
                    ));
                }
            };
            let value = match map.get("description") {
                None | Some(Value::Null) => comment.to_string(),
                Some(Value::String(existing)) => {
                    if existing.starts_with(&prefix) {
                        return Ok((None, None));
                    }
                    format!("{prefix}{existing}")
                }
                Some(_) => {
                    return Ok((
                        None,
                        Some("description 不是字符串，未注入中文注释".to_string()),
                    ));
                }
            };
            map.insert(
                Value::String("description".to_string()),
                Value::String(value),
            );
            let serialized = serialize_mapping(&map)?;
            let mut out = String::from("---\n");
            out.push_str(&serialized);
            out.push_str("---\n");
            out.push_str(body);
            Ok((Some(out), None))
        }
        None => {
            // 没有 frontmatter：新增一个最小块承载注释，正文保持在块之后。
            let mut map = Mapping::new();
            map.insert(
                Value::String("description".to_string()),
                Value::String(comment.to_string()),
            );
            let serialized = serialize_mapping(&map)?;
            let mut out = String::from("---\n");
            out.push_str(&serialized);
            out.push_str("---\n");
            out.push_str(text);
            Ok((Some(out), None))
        }
    }
}

fn serialize_mapping(map: &Mapping) -> Result<String> {
    let text = yaml_serde::to_string(&Value::Mapping(map.clone()))
        .map_err(|err| CoreError::other(format!("序列化 frontmatter 失败：{err}")))?;
    if text.ends_with('\n') {
        Ok(text)
    } else {
        Ok(format!("{text}\n"))
    }
}

/// 拆出 frontmatter 块（不含分隔行）与其后的正文；无完整 frontmatter 返回 `None`。
fn split_frontmatter(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = rest.split_inclusive('\n');
    let first = lines.next()?;
    if first.trim_end_matches(['\r', '\n']).trim_end() != "---" {
        return None;
    }
    let mut offset = first.len();
    for line in lines {
        let content = line.trim_end_matches(['\r', '\n']);
        if content.trim_end() == "---" {
            let block = &rest[first.len()..offset];
            let body = &rest[offset + line.len()..];
            return Some((block, body));
        }
        offset += line.len();
    }
    None
}

// ---------------------------------------------------------------------------
// 指纹与落盘
// ---------------------------------------------------------------------------

/// 产物文件集指纹。
pub fn hash_files(files: &[ComposedFile]) -> String {
    let mut hasher = Sha256::new();
    for file in files {
        hasher.update(file.rel.as_bytes());
        hasher.update([0]);
        hasher.update(&file.bytes);
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

fn input_hash(
    base: &[(String, Vec<u8>)],
    overlays: &[OverlayApplication],
    comment: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"base\0");
    for (rel, bytes) in base {
        hasher.update(rel.as_bytes());
        hasher.update([0]);
        hasher.update(bytes);
        hasher.update([0]);
    }
    hasher.update(b"overlay\0");
    for overlay in overlays {
        hasher.update(overlay.agent.as_deref().unwrap_or(GENERIC_DIMENSION).as_bytes());
        hasher.update(b"/");
        hasher.update(overlay.os.as_deref().unwrap_or(GENERIC_DIMENSION).as_bytes());
        hasher.update([0]);
        hasher.update(overlay.rel.as_bytes());
        hasher.update([0]);
        hasher.update(&overlay.bytes);
        hasher.update([0]);
    }
    hasher.update(b"comment\0");
    hasher.update(comment.unwrap_or("").as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 目标目录当前内容指纹；不存在返回 `None`。
///
/// 只用于识别“外部变更”，因此不跳过 `.manager`——用户自行放入的目录同样是改动。
pub fn hash_dir(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    if !path.is_dir() {
        let bytes = std::fs::read(path).map_err(|err| CoreError::io(path, err))?;
        let mut hasher = Sha256::new();
        hasher.update(b"file\0");
        hasher.update(bytes);
        return Ok(Some(format!("{:x}", hasher.finalize())));
    }
    let files: Vec<ComposedFile> = collect_dir_files(path, false)?
        .into_iter()
        .map(|(rel, bytes)| ComposedFile { rel, bytes })
        .collect();
    Ok(Some(hash_files(&files)))
}

/// 把产物写入目标目录：先写同父目录临时目录，再整体替换。
pub fn write_artifact(target_dir: &Path, files: &[ComposedFile]) -> Result<()> {
    let parent = target_dir.parent().ok_or_else(|| {
        CoreError::other(format!("部署目标没有父目录：{}", target_dir.display()))
    })?;
    std::fs::create_dir_all(parent).map_err(|err| CoreError::io(parent.to_path_buf(), err))?;

    let name = target_dir
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "skill".to_string());
    let temp = parent.join(format!(".{name}.tmp.{}", std::process::id()));
    if temp.exists() {
        remove_path(&temp)?;
    }
    std::fs::create_dir_all(&temp).map_err(|err| CoreError::io(temp.clone(), err))?;

    for file in files {
        let path = temp.join(file.rel.split('/').collect::<PathBuf>());
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|err| CoreError::io(dir.to_path_buf(), err))?;
        }
        std::fs::write(&path, &file.bytes).map_err(|err| CoreError::io(path, err))?;
    }

    if target_dir.exists() {
        remove_path(target_dir)?;
    }
    if let Err(err) = std::fs::rename(&temp, target_dir) {
        let _ = remove_path(&temp);
        return Err(CoreError::io(target_dir, err));
    }
    Ok(())
}

/// 删除一个文件或目录；不存在视为已达成。
pub fn remove_path(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => {
            std::fs::remove_dir_all(path).map_err(|err| CoreError::io(path, err))
        }
        Ok(_) => std::fs::remove_file(path).map_err(|err| CoreError::io(path, err)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(CoreError::io(path, err)),
    }
}
