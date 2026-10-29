//! `.manager` 文件读写：meta/translation/overlay/source。
//!
//! 写入一律按**原始键值合并**，保留软件不认识的字段与键顺序（D3）；
//! 读取区分“文件不存在”（合法空值）与“读取失败”（报错，不折叠成空资料）。

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;
use yaml_serde::{Mapping, Value};

use crate::error::{CoreError, Result};

/// 管理层目录名。
pub const MANAGER_DIR: &str = ".manager";
pub const META_FILE: &str = "meta.yaml";
pub const TRANSLATION_FILE: &str = "translation.md";
pub const OVERLAY_DIR: &str = "overlay";
pub const SOURCE_FILE: &str = "source.yaml";

/// 覆盖层维度目录中表示“该维度通用”的字面值。
pub const GENERIC_DIMENSION: &str = "_";

/// 软件已知键：写入译文时记录的原文内容摘要。
pub const TRANSLATION_SOURCE_HASH_KEY: &str = "translation_source_hash";

pub fn manager_dir(skill_dir: &Path) -> PathBuf {
    skill_dir.join(MANAGER_DIR)
}

pub fn meta_path(skill_dir: &Path) -> PathBuf {
    manager_dir(skill_dir).join(META_FILE)
}

pub fn translation_path(skill_dir: &Path) -> PathBuf {
    manager_dir(skill_dir).join(TRANSLATION_FILE)
}

pub fn overlay_dir(skill_dir: &Path) -> PathBuf {
    manager_dir(skill_dir).join(OVERLAY_DIR)
}

pub fn source_path(skill_dir: &Path) -> PathBuf {
    manager_dir(skill_dir).join(SOURCE_FILE)
}

// ---------------------------------------------------------------------------
// 基础读写
// ---------------------------------------------------------------------------

/// 读取可选文件：不存在返回 `Ok(None)`，其他读取失败返回 `Err`。
pub fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(CoreError::io(path, err)),
    }
}

fn read_text_optional(path: &Path) -> Result<Option<String>> {
    match read_optional(path)? {
        Some(bytes) => String::from_utf8(bytes).map(Some).map_err(|err| {
            CoreError::other(format!("{} 不是有效 UTF-8：{err}", path.display()))
        }),
        None => Ok(None),
    }
}

/// 同目录临时文件 + rename，避免中断留下半个资料文件。
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| CoreError::other(format!("路径没有父目录：{}", path.display())))?;
    std::fs::create_dir_all(parent).map_err(|err| CoreError::io(parent, err))?;

    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "manager".to_string());
    let temp = parent.join(format!(".{file_name}.tmp.{}", std::process::id()));

    std::fs::write(&temp, bytes).map_err(|err| CoreError::io(temp.clone(), err))?;
    if let Err(err) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(CoreError::io(path, err));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// meta.yaml
// ---------------------------------------------------------------------------

/// 支持声明：`All` 表示删除该键（缺省=该域内全支持）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SupportDecl {
    All,
    Empty,
    List(Vec<String>),
}

/// 解析后的管理资料。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkillMeta {
    /// 标签；空=无标签。
    pub tags: Vec<String>,
    /// 中文短注释（一般 ≤20 字，不设强制上限）。
    pub comment: Option<String>,
    /// 支持的 Agent；`None`=缺省（该域内全支持）。
    pub agents: Option<Vec<String>>,
    /// 支持的 OS；`None`=缺省（全支持）。
    pub oses: Option<Vec<String>>,
    /// 写入译文时的原文内容摘要。
    pub translation_source_hash: Option<String>,
}

/// 规范化标签：去首尾空白、去空、去重（保留首次出现顺序）。
pub fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(tags.len());
    for tag in tags {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !out.iter().any(|existing| existing == trimmed) {
            out.push(trimmed.to_string());
        }
    }
    out
}

/// `meta.yaml`：保留原始映射的读写器。
#[derive(Clone, Debug)]
pub struct MetaFile {
    raw: Mapping,
}

impl MetaFile {
    /// 加载；文件不存在视为空资料。
    pub fn load(skill_dir: &Path) -> Result<Self> {
        let path = meta_path(skill_dir);
        match read_text_optional(&path)? {
            Some(text) => Self::from_yaml(&text, &path),
            None => Ok(Self {
                raw: Mapping::new(),
            }),
        }
    }

    /// 从 YAML 文本解析；顶层必须是映射（或空文件）。
    pub fn from_yaml(text: &str, path: &Path) -> Result<Self> {
        let value: Value = yaml_serde::from_str(text)
            .map_err(|err| CoreError::Yaml {
                path: path.to_path_buf(),
                message: err.to_string(),
            })?;
        match value {
            Value::Mapping(mapping) => Ok(Self { raw: mapping }),
            Value::Null => Ok(Self {
                raw: Mapping::new(),
            }),
            _ => Err(CoreError::Yaml {
                path: path.to_path_buf(),
                message: "meta.yaml 顶层应为映射".to_string(),
            }),
        }
    }

    /// 解析管理资料；字段形状不合法时报错，不静默当作缺省。
    pub fn meta(&self) -> Result<SkillMeta> {
        Ok(SkillMeta {
            tags: self.string_list("tags")?.unwrap_or_default(),
            comment: self.optional_string("comment")?,
            agents: self.string_list("agents")?,
            oses: self.string_list("oses")?,
            translation_source_hash: self.optional_string(TRANSLATION_SOURCE_HASH_KEY)?,
        })
    }

    fn value(&self, key: &str) -> Option<&Value> {
        self.raw.get(key)
    }

    fn optional_string(&self, key: &str) -> Result<Option<String>> {
        match self.value(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(text)) => {
                let trimmed = text.trim();
                Ok(if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                })
            }
            Some(_) => Err(self.shape_error(key, "应为字符串")),
        }
    }

    fn string_list(&self, key: &str) -> Result<Option<Vec<String>>> {
        match self.value(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Sequence(items)) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        Value::String(text) => out.push(text.trim().to_string()),
                        _ => return Err(self.shape_error(key, "列表项应为字符串")),
                    }
                }
                Ok(Some(normalize_tags(&out)))
            }
            Some(_) => Err(self.shape_error(key, "应为列表")),
        }
    }

    fn shape_error(&self, key: &str, reason: &str) -> CoreError {
        CoreError::Yaml {
            path: PathBuf::from(format!("{MANAGER_DIR}/{META_FILE}")),
            message: format!("{key} {reason}"),
        }
    }

    /// 替换标签集合；空集合删除该键。
    pub fn set_tags(&mut self, tags: &[String]) {
        let normalized = normalize_tags(tags);
        if normalized.is_empty() {
            self.raw.shift_remove("tags");
        } else {
            self.raw.insert(
                Value::String("tags".to_string()),
                Value::Sequence(
                    normalized
                        .into_iter()
                        .map(Value::String)
                        .collect::<Vec<_>>(),
                ),
            );
        }
    }

    /// 设置或清除中文注释。
    pub fn set_comment(&mut self, comment: Option<&str>) {
        self.set_optional_string("comment", comment);
    }

    pub fn set_translation_source_hash(&mut self, hash: Option<&str>) {
        self.set_optional_string(TRANSLATION_SOURCE_HASH_KEY, hash);
    }

    fn set_optional_string(&mut self, key: &str, value: Option<&str>) {
        match value.map(str::trim).filter(|text| !text.is_empty()) {
            Some(text) => {
                self.raw
                    .insert(Value::String(key.to_string()), Value::String(text.to_string()));
            }
            None => {
                self.raw.shift_remove(key);
            }
        }
    }

    /// 设置支持声明；`All` 删除该键，`Empty` 写显式空集合。
    pub fn set_agents(&mut self, decl: &SupportDecl) {
        self.set_support("agents", decl);
    }

    pub fn set_oses(&mut self, decl: &SupportDecl) {
        self.set_support("oses", decl);
    }

    fn set_support(&mut self, key: &str, decl: &SupportDecl) {
        match decl {
            SupportDecl::All => {
                self.raw.shift_remove(key);
            }
            SupportDecl::Empty => {
                self.raw
                    .insert(Value::String(key.to_string()), Value::Sequence(Vec::new()));
            }
            SupportDecl::List(list) => {
                let normalized = normalize_tags(list);
                let value = Value::Sequence(
                    normalized
                        .into_iter()
                        .map(Value::String)
                        .collect::<Vec<_>>(),
                );
                self.raw.insert(Value::String(key.to_string()), value);
            }
        }
    }

    /// 原始映射中的一项（供合并与变化汇总按原始值比较）。
    pub fn raw_value(&self, key: &str) -> Option<&Value> {
        self.raw.get(key)
    }

    /// 原始映射中的字符串键。
    pub fn raw_keys(&self) -> Vec<String> {
        self.raw
            .keys()
            .filter_map(|key| key.as_str().map(str::to_string))
            .collect()
    }

    /// 用上游同名字段的原始值覆盖本地（D3「同项上游值优先」，未提供则保留本地）；
    /// 返回是否发生了修改。
    pub fn merge_from(&mut self, upstream: &MetaFile) -> bool {
        let mut changed = false;
        for (key, value) in upstream.raw.iter() {
            if self.raw.get(key) != Some(value) {
                self.raw.insert(key.clone(), value.clone());
                changed = true;
            }
        }
        changed
    }

    /// 序列化文本（保留未知字段与键顺序，值与其字符串形态不被 YAML 隐式类型改写）。
    pub fn to_yaml(&self) -> Result<String> {
        crate::yaml_safe::to_yaml_string(&Value::Mapping(self.raw.clone())).map_err(|err| {
            CoreError::Yaml {
                path: PathBuf::from(format!("{MANAGER_DIR}/{META_FILE}")),
                message: err.to_string(),
            }
        })
    }

    /// 写回；内容未变化时仍会重写（不额外做差异检测）。
    pub fn save(&self, skill_dir: &Path) -> Result<()> {
        let text = self.to_yaml()?;
        write_atomic(&meta_path(skill_dir), text.as_bytes())
    }
}

// ---------------------------------------------------------------------------
// translation.md
// ---------------------------------------------------------------------------

/// 读取译文；无译文返回 `None`。
pub fn read_translation(skill_dir: &Path) -> Result<Option<String>> {
    read_text_optional(&translation_path(skill_dir))
}

/// 入口文件的 SHA-256 十六进制摘要。
pub fn entry_content_hash(skill_dir: &Path) -> Result<String> {
    let (path, _) = crate::vault::scan::entry_file(skill_dir).ok_or_else(|| {
        CoreError::other(format!(
            "技能目录缺少 SKILL.md/skill.md：{}",
            skill_dir.display()
        ))
    })?;
    let bytes = std::fs::read(&path).map_err(|err| CoreError::io(path.clone(), err))?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

/// 保存译文：写入 `translation.md` 并把时效依据更新为**当前原文**的摘要。
pub fn write_translation(skill_dir: &Path, content: &str) -> Result<()> {
    let hash = entry_content_hash(skill_dir)?;
    write_atomic(&translation_path(skill_dir), content.as_bytes())?;

    let mut meta = MetaFile::load(skill_dir)?;
    meta.set_translation_source_hash(Some(&hash));
    meta.save(skill_dir)
}

/// 删除译文；存在时效依据时一并清除。
pub fn remove_translation(skill_dir: &Path) -> Result<()> {
    let path = translation_path(skill_dir);
    match read_optional(&path)? {
        Some(_) => std::fs::remove_file(&path).map_err(|err| CoreError::io(path, err))?,
        None => {}
    }

    let mut meta = MetaFile::load(skill_dir)?;
    if meta.value(TRANSLATION_SOURCE_HASH_KEY).is_some() {
        meta.set_translation_source_hash(None);
        meta.save(skill_dir)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// overlay/
// ---------------------------------------------------------------------------

/// 一项覆盖层文件：`<agent>/<os>/<相对路径>`，`None` 表示该维度通用。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayEntry {
    pub agent: Option<String>,
    pub os: Option<String>,
    pub rel: String,
    pub size: u64,
}

fn validate_overlay_rel(rel: &str) -> Result<String> {
    let trimmed = rel.trim();
    if trimmed.is_empty() {
        return Err(CoreError::other("覆盖层相对路径不能为空"));
    }
    if trimmed.starts_with('/') || trimmed.contains('\\') {
        return Err(CoreError::other(format!(
            "覆盖层相对路径必须使用 / 分隔且为相对路径：{rel}"
        )));
    }
    let mut parts = Vec::new();
    for part in trimmed.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(CoreError::other(format!(
                "覆盖层相对路径不能包含空段、. 或 ..：{rel}"
            )));
        }
        parts.push(part);
    }
    Ok(parts.join("/"))
}

fn dimension_dir(value: Option<&str>) -> String {
    match value.map(str::trim) {
        None | Some("") => GENERIC_DIMENSION.to_string(),
        Some(GENERIC_DIMENSION) => GENERIC_DIMENSION.to_string(),
        Some(other) => other.to_string(),
    }
}

fn dimension_value(value: &str) -> Option<String> {
    if value == GENERIC_DIMENSION {
        None
    } else {
        Some(value.to_string())
    }
}

/// 一项覆盖层的实际路径。
pub fn overlay_path(
    skill_dir: &Path,
    agent: Option<&str>,
    os: Option<&str>,
    rel: &str,
) -> Result<PathBuf> {
    let rel = validate_overlay_rel(rel)?;
    let mut path = overlay_dir(skill_dir);
    path.push(dimension_dir(agent));
    path.push(dimension_dir(os));
    for part in rel.split('/') {
        path.push(part);
    }
    Ok(path)
}

/// 列出覆盖层全部文件。
pub fn list_overlays(skill_dir: &Path) -> Result<Vec<OverlayEntry>> {
    let base = overlay_dir(skill_dir);
    if !base.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for entry in WalkDir::new(&base).min_depth(3) {
        let entry = entry.map_err(|err| {
            CoreError::other(format!("读取覆盖层目录失败（{}）：{err}", base.display()))
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = match entry.path().strip_prefix(&base) {
            Ok(relative) => relative,
            Err(_) => continue,
        };
        let parts: Vec<String> = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect();
        if parts.len() < 3 {
            continue;
        }
        out.push(OverlayEntry {
            agent: dimension_value(&parts[0]),
            os: dimension_value(&parts[1]),
            rel: parts[2..].join("/"),
            size: entry.metadata().map(|meta| meta.len()).unwrap_or(0),
        });
    }
    out.sort_by(|a, b| {
        a.agent
            .cmp(&b.agent)
            .then(a.os.cmp(&b.os))
            .then(a.rel.cmp(&b.rel))
    });
    Ok(out)
}

/// 读取一项覆盖层文件；不存在返回 `None`。
pub fn read_overlay(
    skill_dir: &Path,
    agent: Option<&str>,
    os: Option<&str>,
    rel: &str,
) -> Result<Option<Vec<u8>>> {
    read_optional(&overlay_path(skill_dir, agent, os, rel)?)
}

/// 写入一项覆盖层文件。
pub fn write_overlay(
    skill_dir: &Path,
    agent: Option<&str>,
    os: Option<&str>,
    rel: &str,
    bytes: &[u8],
) -> Result<()> {
    write_atomic(&overlay_path(skill_dir, agent, os, rel)?, bytes)
}

/// 删除一项覆盖层文件；返回是否实际删除。
pub fn remove_overlay(
    skill_dir: &Path,
    agent: Option<&str>,
    os: Option<&str>,
    rel: &str,
) -> Result<bool> {
    let path = overlay_path(skill_dir, agent, os, rel)?;
    match read_optional(&path)? {
        Some(_) => {
            std::fs::remove_file(&path).map_err(|err| CoreError::io(path, err))?;
            Ok(true)
        }
        None => Ok(false),
    }
}

// ---------------------------------------------------------------------------
// source.yaml
// ---------------------------------------------------------------------------

/// 上游来源记录；无上游的自建技能没有该文件。
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceRecord {
    pub repo: String,
    /// 所选分支/引用；缺省跟踪远端默认分支。
    #[serde(rename = "ref", default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    pub subpath: String,
    /// 当前采用提交的完整 Git OID。
    pub version: String,
    /// 已跳过的完整 OID（含删除类）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped_version: Option<String>,
    pub registered_at: String,
}

impl SourceRecord {
    /// 仓库地址去重键：去首尾空白与末尾 `/`。
    pub fn repo_key(repo: &str) -> String {
        repo.trim().trim_end_matches('/').to_string()
    }
}

/// 读取来源记录；无 `source.yaml` 返回 `None`。
pub fn read_source(skill_dir: &Path) -> Result<Option<SourceRecord>> {
    let path = source_path(skill_dir);
    let Some(text) = read_text_optional(&path)? else {
        return Ok(None);
    };
    let record: SourceRecord = yaml_serde::from_str(&text).map_err(|err| CoreError::Yaml {
        path,
        message: err.to_string(),
    })?;
    Ok(Some(record))
}

/// 写入来源记录。
pub fn write_source(skill_dir: &Path, record: &SourceRecord) -> Result<()> {
    let path = source_path(skill_dir);
    let value = yaml_serde::to_value(record).map_err(|err| CoreError::Yaml {
        path: path.clone(),
        message: err.to_string(),
    })?;
    let text = crate::yaml_safe::to_yaml_string(&value).map_err(|err| CoreError::Yaml {
        path: path.clone(),
        message: err.to_string(),
    })?;
    write_atomic(&path, text.as_bytes())
}
