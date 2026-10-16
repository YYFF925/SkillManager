//! 工具共享资料读写：`tools/<tool-slug>/tool.yaml` 与 `guide.md`（T075，data-model §1.5）。
//!
//! **工具 slug（目录名）即工具身份**，不引入数字 ID。`summary` 是卡片短摘要，`intro` 是
//! 概览长简介（可读、多段），两者不能互相代替。写入按原始键值合并，保留软件不认识的
//! 字段与键顺序（与 `.manager/meta.yaml`、`presets/<domain>.yaml` 同一规则）。
//!
//! 资料随 SkillManagerVault 同步（FR-044）；本机安装记录不在库内（见 `state::tool_installs`）。

use std::path::{Path, PathBuf};

use serde::Serialize;
use yaml_serde::{Mapping, Value};

use crate::domain::Domain;
use crate::error::{CoreError, Result};
use crate::manager::files::{normalize_tags, read_optional, write_atomic};

/// 工具资料目录名。
pub const TOOLS_DIR: &str = "tools";
/// 工具资料文件名。
pub const TOOL_FILE: &str = "tool.yaml";
/// 维护指南文件名。
pub const GUIDE_FILE: &str = "guide.md";

/// 工具资料根目录。
pub fn tools_dir(root: &Path) -> PathBuf {
    root.join(TOOLS_DIR)
}

/// 一个工具的资料目录（slug 即目录名）。
pub fn tool_dir(root: &Path, slug: &str) -> PathBuf {
    tools_dir(root).join(slug)
}

/// `tool.yaml` 路径。
pub fn tool_path(root: &Path, slug: &str) -> PathBuf {
    tool_dir(root, slug).join(TOOL_FILE)
}

/// `guide.md` 路径。
pub fn guide_path(root: &Path, slug: &str) -> PathBuf {
    tool_dir(root, slug).join(GUIDE_FILE)
}

/// 校验工具 slug 可以作为一个单层目录名。
///
/// slug 直接拼进库内路径，必须挡住路径分隔符与空段；错误信息面向用户且指出这是 slug。
pub fn validate_slug(slug: &str) -> Result<()> {
    let fail = |reason: &str| CoreError::other(format!("工具 slug 不合法（{slug}）：{reason}"));
    if slug.trim() != slug || slug.is_empty() {
        return Err(fail("不能为空或首尾含空白"));
    }
    if slug == "." || slug == ".." {
        return Err(fail("不能是 . 或 .."));
    }
    if slug.contains('/') || slug.contains('\\') {
        return Err(fail("不能包含路径分隔符"));
    }
    if slug.chars().any(char::is_control) {
        return Err(fail("不能包含控制字符"));
    }
    Ok(())
}

/// 名称 → slug：保留 ASCII 字母数字，其余折为单个 `-`（与预设 id 派生同一口径）。
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut pending = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending && !out.is_empty() {
                out.push('-');
            }
            pending = false;
            out.push(ch.to_ascii_lowercase());
        } else {
            pending = true;
        }
    }
    if out.is_empty() {
        "tool".to_string()
    } else {
        out
    }
}

/// 配套 Skill 关联：技能名引用 + 候选来源（未入库时仍携带来源，供候选入库复用）。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ToolSkillLink {
    pub domain: Domain,
    pub name: String,
    /// 候选来源仓库地址（可选；未入库时可复用技能候选发现流程）。
    pub repo: Option<String>,
    /// 来源仓库内子路径（可选）。
    pub subpath: Option<String>,
}

/// 解析后的工具资料。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToolMeta {
    /// 工具名（必填）。
    pub name: String,
    /// 卡片短摘要。
    pub summary: Option<String>,
    /// 概览长简介（可读、多段）。
    pub intro: Option<String>,
    pub tags: Vec<String>,
    /// 一级分类。
    pub category: Option<String>,
    /// 版本检查命令（Agent 编写；工作目录为该工具资料目录）。
    pub check_command: Option<String>,
    /// 配套 Skill 关联。
    pub skills: Vec<ToolSkillLink>,
}

impl ToolMeta {
    /// 是否可直接写入：名称不能为空。
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(CoreError::other("工具资料缺少 name（工具名称不能为空）"));
        }
        Ok(())
    }
}

/// `tool.yaml` 的保留原始映射读写器。
#[derive(Clone, Debug)]
pub struct ToolFile {
    path: PathBuf,
    raw: Mapping,
}

impl ToolFile {
    /// 加载已登记工具；文件不存在按“未登记”报错，而不是当作空资料。
    pub fn load(root: &Path, slug: &str) -> Result<Self> {
        validate_slug(slug)?;
        let path = tool_path(root, slug);
        match read_optional(&path)? {
            Some(bytes) => {
                let text = String::from_utf8(bytes).map_err(|err| {
                    CoreError::other(format!("{} 不是有效 UTF-8：{err}", path.display()))
                })?;
                Self::from_yaml(&text, &path)
            }
            None => Err(CoreError::other(format!(
                "工具未登记：{slug}（缺少 {}）",
                path.display()
            ))),
        }
    }

    /// 从 YAML 文本解析；顶层必须是映射。
    pub fn from_yaml(text: &str, path: &Path) -> Result<Self> {
        let value: Value = yaml_serde::from_str(text).map_err(|err| CoreError::Yaml {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
        match value {
            Value::Mapping(mapping) => Ok(Self {
                path: path.to_path_buf(),
                raw: mapping,
            }),
            _ => Err(CoreError::Yaml {
                path: path.to_path_buf(),
                message: format!("{TOOL_FILE} 顶层应为映射"),
            }),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 由单个 JSON 对象构造（`--stdin` 的 `{"tool": {...}}`）。
    pub fn from_json_object(
        object: &serde_json::Map<String, serde_json::Value>,
        path: &Path,
    ) -> Result<Self> {
        let text = yaml_serde::to_string(object).map_err(|err| CoreError::Yaml {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
        Self::from_yaml(&text, path)
    }

    fn error(&self, message: impl Into<String>) -> CoreError {
        CoreError::Yaml {
            path: self.path.clone(),
            message: message.into(),
        }
    }

    /// 解析工具资料；字段形状不合法时报错，不静默当作缺省。
    pub fn meta(&self) -> Result<ToolMeta> {
        Ok(ToolMeta {
            name: self.optional_string("name")?.unwrap_or_default(),
            summary: self.optional_string("summary")?,
            intro: self.optional_string("intro")?,
            tags: self.string_list("tags")?.unwrap_or_default(),
            category: self.optional_string("category")?,
            check_command: self.optional_string("check_command")?,
            skills: self.skill_links()?,
        })
    }

    fn optional_string(&self, key: &str) -> Result<Option<String>> {
        match self.raw.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(text)) => {
                let trimmed = text.trim();
                Ok(if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                })
            }
            Some(_) => Err(self.error(format!("{key} 应为字符串"))),
        }
    }

    fn string_list(&self, key: &str) -> Result<Option<Vec<String>>> {
        match self.raw.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Sequence(items)) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        Value::String(text) => out.push(text.trim().to_string()),
                        _ => return Err(self.error(format!("{key} 列表项应为字符串"))),
                    }
                }
                Ok(Some(normalize_tags(&out)))
            }
            Some(_) => Err(self.error(format!("{key} 应为列表"))),
        }
    }

    fn skill_links(&self) -> Result<Vec<ToolSkillLink>> {
        let Some(value) = self.raw.get("skills") else {
            return Ok(Vec::new());
        };
        let Value::Sequence(items) = value else {
            return Err(self.error("skills 应为列表"));
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            let mapping = item
                .as_mapping()
                .ok_or_else(|| self.error("skills 列表项应为映射"))?;
            let domain_raw = match mapping.get("domain") {
                Some(Value::String(text)) => text.trim().to_string(),
                _ => return Err(self.error("skills 项的 domain 应为字符串")),
            };
            let domain = domain_raw
                .parse::<Domain>()
                .map_err(|err| self.error(err.to_string()))?;
            let name = match mapping.get("name") {
                Some(Value::String(text)) if !text.trim().is_empty() => text.trim().to_string(),
                _ => return Err(self.error("skills 项的 name 应为非空字符串")),
            };
            let optional = |key: &str| -> Result<Option<String>> {
                match mapping.get(key) {
                    None | Some(Value::Null) => Ok(None),
                    Some(Value::String(text)) => {
                        let trimmed = text.trim();
                        Ok(if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        })
                    }
                    Some(_) => Err(self.error(format!("skills 项的 {key} 应为字符串"))),
                }
            };
            out.push(ToolSkillLink {
                domain,
                name,
                repo: optional("repo")?,
                subpath: optional("subpath")?,
            });
        }
        Ok(out)
    }

    /// 直接替换整套工具资料（保留映射中的未知字段）。
    pub fn set_meta(&mut self, meta: &ToolMeta) {
        self.raw
            .insert(Value::String("name".into()), Value::String(meta.name.clone()));
        self.set_optional("summary", meta.summary.as_deref());
        self.set_optional("intro", meta.intro.as_deref());
        self.set_optional("category", meta.category.as_deref());
        self.set_optional("check_command", meta.check_command.as_deref());

        let tags = normalize_tags(&meta.tags);
        if tags.is_empty() {
            self.raw.shift_remove("tags");
        } else {
            self.raw.insert(
                Value::String("tags".into()),
                Value::Sequence(tags.into_iter().map(Value::String).collect()),
            );
        }

        let skills: Vec<Value> = meta
            .skills
            .iter()
            .map(|link| {
                let mut mapping = Mapping::new();
                mapping.insert(
                    Value::String("domain".into()),
                    Value::String(link.domain.as_str().to_string()),
                );
                mapping.insert(Value::String("name".into()), Value::String(link.name.clone()));
                if let Some(repo) = &link.repo {
                    mapping.insert(Value::String("repo".into()), Value::String(repo.clone()));
                }
                if let Some(subpath) = &link.subpath {
                    mapping.insert(Value::String("subpath".into()), Value::String(subpath.clone()));
                }
                Value::Mapping(mapping)
            })
            .collect();
        if skills.is_empty() {
            self.raw.shift_remove("skills");
        } else {
            self.raw
                .insert(Value::String("skills".into()), Value::Sequence(skills));
        }
    }

    fn set_optional(&mut self, key: &str, value: Option<&str>) {
        match value.map(str::trim).filter(|text| !text.is_empty()) {
            Some(text) => {
                self.raw
                    .insert(Value::String(key.into()), Value::String(text.to_string()));
            }
            None => {
                self.raw.shift_remove(key);
            }
        }
    }

    /// 追加一个配套 Skill 关联（`tool skill-link`）；已存在同引用时不重复添加。
    pub fn add_skill_link(&mut self, link: &ToolSkillLink) -> bool {
        let mut meta = match self.meta() {
            Ok(meta) => meta,
            Err(_) => return false,
        };
        if meta
            .skills
            .iter()
            .any(|existing| existing.domain == link.domain && existing.name == link.name)
        {
            return false;
        }
        meta.skills.push(link.clone());
        self.set_meta(&meta);
        true
    }

    /// 序列化文本（保留未知字段与键顺序）。
    pub fn to_yaml(&self) -> Result<String> {
        yaml_serde::to_string(&Value::Mapping(self.raw.clone())).map_err(|err| CoreError::Yaml {
            path: self.path.clone(),
            message: err.to_string(),
        })
    }

    /// 写回 `tool.yaml`。
    pub fn save(&self) -> Result<()> {
        let text = self.to_yaml()?;
        write_atomic(&self.path, text.as_bytes())
    }
}

/// 读取维护指南；不存在返回 `None`（界面表达“待补维护指南”）。
pub fn read_guide(root: &Path, slug: &str) -> Result<Option<String>> {
    let path = guide_path(root, slug);
    match read_optional(&path)? {
        Some(bytes) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|err| CoreError::other(format!("{} 不是有效 UTF-8：{err}", path.display()))),
        None => Ok(None),
    }
}

/// 写入维护指南。
pub fn write_guide(root: &Path, slug: &str, content: &str) -> Result<()> {
    write_atomic(&guide_path(root, slug), content.as_bytes())
}

/// 写入结果：为入口层提供可读的落地位置。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ToolWriteReport {
    pub slug: String,
    pub dir: String,
    pub wrote_tool: bool,
    pub wrote_guide: bool,
    /// `--dir` 一并落地的其他文件（如检查脚本）。
    pub copied_files: Vec<String>,
}

fn is_hidden_component(rel: &Path) -> bool {
    rel.components().any(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .starts_with('.')
    })
}

/// 从一个源目录写入工具资料（`--dir`）。
///
/// 要求源目录存在 `tool.yaml`；`guide.md` 与其他普通文件（检查脚本等）存在才写入，
/// 缺失表示保持原值、不表示清空（与“未给的编辑参数保持原值”同一约定）。
pub fn write_from_dir(root: &Path, slug: &str, source: &Path) -> Result<ToolWriteReport> {
    validate_slug(slug)?;
    if !source.is_dir() {
        return Err(CoreError::NotADirectory(source.to_path_buf()));
    }
    let source_tool = source.join(TOOL_FILE);
    let text = std::fs::read_to_string(&source_tool)
        .map_err(|err| CoreError::io(source_tool.clone(), err))?;
    let path = tool_path(root, slug);
    let file = ToolFile::from_yaml(&text, &path)?;
    let meta = file
        .meta()
        .map_err(|err| CoreError::other(format!("{}：{err}", source_tool.display())))?;
    meta.validate()?;

    std::fs::create_dir_all(tool_dir(root, slug))
        .map_err(|err| CoreError::io(tool_dir(root, slug), err))?;
    file.save()?;

    let mut copied = Vec::new();
    let mut wrote_guide = false;
    for entry in walkdir::WalkDir::new(source).min_depth(1) {
        let entry = entry
            .map_err(|err| CoreError::other(format!("读取 {} 失败：{err}", source.display())))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(source)
            .map_err(|_| CoreError::other("源目录遍历越界"))?
            .to_path_buf();
        if is_hidden_component(&relative) {
            continue;
        }
        let bytes =
            std::fs::read(entry.path()).map_err(|err| CoreError::io(entry.path(), err))?;
        write_atomic(&tool_dir(root, slug).join(&relative), &bytes)?;
        let display = relative.to_string_lossy().replace('\\', "/");
        if display == GUIDE_FILE {
            wrote_guide = true;
        } else if display != TOOL_FILE {
            copied.push(display);
        }
    }
    copied.sort();

    Ok(ToolWriteReport {
        slug: slug.to_string(),
        dir: tool_dir(root, slug).to_string_lossy().into_owned(),
        wrote_tool: true,
        wrote_guide,
        copied_files: copied,
    })
}

/// 从 `{"tool": {...}, "guide": "..."}` 写入工具资料（`--stdin`）。
///
/// 只更新传入的字段：`tool` 出现的键由该对象决定，`guide` 缺失表示保持原值。
pub fn write_from_stdin(root: &Path, slug: &str, payload: &serde_json::Value) -> Result<ToolWriteReport> {
    validate_slug(slug)?;
    let object = payload
        .get("tool")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| CoreError::other("--stdin 需要对象 {\"tool\": {...}, \"guide\": \"...\"}"))?;
    let path = tool_path(root, slug);

    let existing = read_optional(&path)?;
    let mut file = match existing {
        Some(bytes) => {
            let text = String::from_utf8(bytes).map_err(|err| {
                CoreError::other(format!("{} 不是有效 UTF-8：{err}", path.display()))
            })?;
            ToolFile::from_yaml(&text, &path)?
        }
        None => ToolFile::from_json_object(&serde_json::Map::new(), &path)?,
    };

    let incoming = ToolFile::from_json_object(object, &path)?;
    // 合并：传入字段覆盖，未传字段保持原值（含未知字段）。
    for (key, value) in incoming.raw.iter() {
        file.raw.insert(key.clone(), value.clone());
    }
    file.meta()?.validate()?;

    std::fs::create_dir_all(tool_dir(root, slug))
        .map_err(|err| CoreError::io(tool_dir(root, slug), err))?;
    file.save()?;

    let mut wrote_guide = false;
    if let Some(guide) = payload.get("guide").and_then(serde_json::Value::as_str) {
        write_guide(root, slug, guide)?;
        wrote_guide = true;
    }

    Ok(ToolWriteReport {
        slug: slug.to_string(),
        dir: tool_dir(root, slug).to_string_lossy().into_owned(),
        wrote_tool: true,
        wrote_guide,
        copied_files: Vec::new(),
    })
}

/// 已登记的 slug（存在 `tools/<slug>/tool.yaml` 的目录名），按名称排序。
pub fn registered_slugs(root: &Path) -> Result<Vec<String>> {
    Ok(all_slugs(root)?
        .into_iter()
        .filter(|slug| tool_path(root, slug).is_file())
        .collect())
}

/// `tools/` 下的全部目录名（含缺少 `tool.yaml` 的目录），按名称排序。
///
/// 缺少 `tool.yaml` 的目录按“资料损坏”在清单中如实呈现，而不是静默隐藏。
pub fn all_slugs(root: &Path) -> Result<Vec<String>> {
    let dir = tools_dir(root);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|err| CoreError::io(dir.clone(), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| CoreError::io(dir.clone(), err))?;
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        out.push(name);
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tool(root: &Path, slug: &str, body: &str) -> PathBuf {
        let path = tool_path(root, slug);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn reads_meta_with_summary_intro_and_skills() {
        let root = tempfile::TempDir::new().unwrap();
        write_tool(
            root.path(),
            "ripgrep",
            "name: ripgrep\nsummary: 极快的正则搜索工具\nintro: |\n  ripgrep 是 ...\n  第二段。\ntags: [搜索, cli]\ncategory: 命令行工具\ncheck_command: sh ./check-latest.sh\nskills:\n  - domain: code\n    name: ripgrep-usage\n    repo: https://example.invalid/rg.git\n    subpath: rg\n",
        );
        let meta = ToolFile::load(root.path(), "ripgrep").unwrap().meta().unwrap();
        assert_eq!(meta.name, "ripgrep");
        assert_eq!(meta.summary.as_deref(), Some("极快的正则搜索工具"));
        assert!(meta.intro.as_deref().unwrap().contains("第二段"));
        assert_eq!(meta.tags, vec!["搜索".to_string(), "cli".to_string()]);
        assert_eq!(meta.category.as_deref(), Some("命令行工具"));
        assert_eq!(meta.check_command.as_deref(), Some("sh ./check-latest.sh"));
        assert_eq!(meta.skills.len(), 1);
        assert_eq!(meta.skills[0].domain, Domain::Code);
        assert_eq!(meta.skills[0].name, "ripgrep-usage");
    }

    #[test]
    fn missing_tool_file_is_not_registered() {
        let root = tempfile::TempDir::new().unwrap();
        let err = ToolFile::load(root.path(), "absent").unwrap_err();
        assert!(err.to_string().contains("未登记"), "{err}");
        assert!(registered_slugs(root.path()).unwrap().is_empty());
    }

    #[test]
    fn unknown_fields_survive_write_back() {
        let root = tempfile::TempDir::new().unwrap();
        write_tool(
            root.path(),
            "uv",
            "name: uv\nsummary: 旧摘要\nfoo: keep-me\n",
        );
        let mut file = ToolFile::load(root.path(), "uv").unwrap();
        let mut meta = file.meta().unwrap();
        meta.summary = Some("新摘要".to_string());
        file.set_meta(&meta);
        file.save().unwrap();

        let text = std::fs::read_to_string(tool_path(root.path(), "uv")).unwrap();
        assert!(text.contains("foo: keep-me"), "{text}");
        assert!(text.contains("新摘要"), "{text}");
    }

    #[test]
    fn malformed_shape_is_reported_not_swallowed() {
        let root = tempfile::TempDir::new().unwrap();
        write_tool(root.path(), "bad", "- not-a-mapping\n");
        let err = ToolFile::load(root.path(), "bad").unwrap_err();
        assert!(err.to_string().contains("顶层应为映射"), "{err}");
    }

    #[test]
    fn write_from_dir_copies_guide_and_scripts() {
        let root = tempfile::TempDir::new().unwrap();
        let source = tempfile::TempDir::new().unwrap();
        std::fs::write(
            source.path().join(TOOL_FILE),
            "name: ripgrep\nsummary: 搜索\n",
        )
        .unwrap();
        std::fs::write(source.path().join(GUIDE_FILE), "# 维护指南\n").unwrap();
        std::fs::write(source.path().join("check-latest.sh"), "echo 14.1.1\n").unwrap();

        let report = write_from_dir(root.path(), "ripgrep", source.path()).unwrap();
        assert!(report.wrote_tool && report.wrote_guide);
        assert_eq!(report.copied_files, vec!["check-latest.sh".to_string()]);
        assert_eq!(
            read_guide(root.path(), "ripgrep").unwrap().as_deref(),
            Some("# 维护指南\n")
        );
        assert!(tool_dir(root.path(), "ripgrep").join("check-latest.sh").is_file());
    }

    #[test]
    fn write_from_stdin_merges_and_keeps_unknown_fields() {
        let root = tempfile::TempDir::new().unwrap();
        write_tool(root.path(), "uv", "name: uv\nsummary: 旧\nfoo: keep-me\n");
        let payload = serde_json::json!({
            "tool": {"summary": "新"},
            "guide": "# 指南\n",
        });
        let report = write_from_stdin(root.path(), "uv", &payload).unwrap();
        assert!(report.wrote_guide);
        let text = std::fs::read_to_string(tool_path(root.path(), "uv")).unwrap();
        assert!(text.contains("foo: keep-me"), "{text}");
        assert!(text.contains("新"), "{text}");
        assert!(text.contains("uv"), "{text}");
    }

    #[test]
    fn slug_validation_blocks_path_segments() {
        assert!(validate_slug("ripgrep").is_ok());
        assert!(validate_slug("a/b").is_err());
        assert!(validate_slug("..").is_err());
        assert!(validate_slug("").is_err());
        assert_eq!(slugify("Docker CLI"), "docker-cli");
        assert_eq!(slugify("媒体处理"), "tool");
    }
}
