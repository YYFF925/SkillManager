//! 预设定义文件读写：`presets/<domain>.yaml`（T062，data-model §1.4）。
//!
//! 定义字段：`id`（预设内唯一标识，人写，域内不重复即可）、`name`、`icon`、`order`
//! （预设间顺序）与 `members`（技能名引用，域内唯一可解析；**数组顺序即成员显示顺序**）。
//! 写入按原始键值合并，保留软件不认识的字段与键顺序（与 `.manager/meta.yaml` 同一规则）。
//! 组合定义随 SkillVault 同步；本文件只表达组合，不含部署状态。

use std::path::{Path, PathBuf};

use yaml_serde::{Mapping, Value};

use crate::domain::{Domain, names_equal};
use crate::error::{CoreError, Result};
use crate::manager::files::{read_optional, write_atomic};

/// 预设定义目录名。
pub const PRESETS_DIR: &str = "presets";

/// 预设定义目录。
pub fn presets_dir(root: &Path) -> PathBuf {
    root.join(PRESETS_DIR)
}

/// 某管理域的预设定义文件：`presets/code.yaml` / `presets/lobster.yaml`。
pub fn presets_path(root: &Path, domain: Domain) -> PathBuf {
    presets_dir(root).join(format!("{}.yaml", domain.as_str()))
}

/// 解析后的预设定义。
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    /// 预设间显示顺序。
    pub order: i64,
    /// 成员技能名引用；数组顺序即成员显示顺序。
    pub members: Vec<String>,
}

/// 预设定义文件；保留原始映射以写回未知字段。
#[derive(Clone, Debug)]
pub struct PresetFile {
    path: PathBuf,
    items: Vec<Value>,
}

impl PresetFile {
    /// 加载；文件不存在按空定义。
    pub fn load(root: &Path, domain: Domain) -> Result<Self> {
        let path = presets_path(root, domain);
        let items = match read_optional(&path)? {
            Some(bytes) => {
                let text = String::from_utf8(bytes)
                    .map_err(|err| CoreError::other(format!("{} 不是有效 UTF-8：{err}", path.display())))?;
                let value: Value = yaml_serde::from_str(&text).map_err(|err| CoreError::Yaml {
                    path: path.clone(),
                    message: err.to_string(),
                })?;
                match value {
                    Value::Null => Vec::new(),
                    Value::Sequence(items) => items,
                    _ => {
                        return Err(CoreError::Yaml {
                            path,
                            message: "presets/<domain>.yaml 顶层应为列表".to_string(),
                        });
                    }
                }
            }
            None => Vec::new(),
        };
        Ok(Self { path, items })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 全部定义，按 `order` 与 `id` 排序；形状不合法时报错，不静默丢弃。
    pub fn presets(&self) -> Result<Vec<Preset>> {
        let mut out = Vec::with_capacity(self.items.len());
        for item in &self.items {
            out.push(self.parse_item(item)?);
        }
        out.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
        Ok(out)
    }

    /// 按 id 查找（大小写不敏感，与技能名比较口径一致）。
    pub fn find(&self, id: &str) -> Result<Option<Preset>> {
        for item in &self.items {
            let preset = self.parse_item(item)?;
            if names_equal(&preset.id, id) {
                return Ok(Some(preset));
            }
        }
        Ok(None)
    }

    /// 新建预设：id 由名称派生并在域内去重；初始无成员。
    pub fn create(&mut self, name: &str, icon: Option<&str>) -> Result<Preset> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::other("预设名称不能为空"));
        }
        let id = self.unique_id(name)?;
        let order = self.next_order()?;

        let mut mapping = Mapping::new();
        mapping.insert(Value::String("id".into()), Value::String(id.clone()));
        mapping.insert(Value::String("name".into()), Value::String(name.to_string()));
        if let Some(icon) = icon.map(str::trim).filter(|text| !text.is_empty()) {
            mapping.insert(Value::String("icon".into()), Value::String(icon.to_string()));
        }
        mapping.insert(Value::String("order".into()), Value::from(order));
        mapping.insert(Value::String("members".into()), Value::Sequence(Vec::new()));
        self.items.push(Value::Mapping(mapping));

        Ok(Preset {
            id,
            name: name.to_string(),
            icon: icon.map(str::trim).filter(|text| !text.is_empty()).map(str::to_string),
            order,
            members: Vec::new(),
        })
    }

    /// 删除预设；返回被删除的定义。
    pub fn delete(&mut self, id: &str) -> Result<Preset> {
        let index = self
            .index_of(id)?
            .ok_or_else(|| not_found(id))?;
        let preset = self.parse_item(&self.items[index])?;
        self.items.remove(index);
        Ok(preset)
    }

    /// 重命名；只改 `name`，不触发部署。
    pub fn rename(&mut self, id: &str, name: &str) -> Result<Preset> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::other("预设名称不能为空"));
        }
        self.update(id, |mapping| {
            mapping.insert(Value::String("name".into()), Value::String(name.to_string()));
        })
    }

    /// 设置图标；传 `None` 删除该键。
    pub fn set_icon(&mut self, id: &str, icon: Option<&str>) -> Result<Preset> {
        let icon = icon.map(str::trim).filter(|text| !text.is_empty()).map(str::to_string);
        self.update(id, |mapping| match &icon {
            Some(icon) => {
                mapping.insert(Value::String("icon".into()), Value::String(icon.clone()));
            }
            None => {
                mapping.shift_remove("icon");
            }
        })
    }

    /// 直接替换成员列表（保持给定顺序）；成员增删的校验在 `manage` 层。
    pub fn set_members(&mut self, id: &str, members: &[String]) -> Result<Preset> {
        let members: Vec<Value> = members
            .iter()
            .map(|name| Value::String(name.clone()))
            .collect();
        self.update(id, |mapping| {
            mapping.insert(Value::String("members".into()), Value::Sequence(members));
        })
    }

    /// 按给定 id 顺序重写预设间顺序（`order` = 0..n-1）。
    pub fn set_order(&mut self, ids_in_order: &[String]) -> Result<()> {
        for (position, id) in ids_in_order.iter().enumerate() {
            let index = self
                .index_of(id)?
                .ok_or_else(|| not_found(id))?;
            match &mut self.items[index] {
                Value::Mapping(mapping) => {
                    mapping.insert(Value::String("order".into()), Value::from(position as i64));
                }
                _ => return Err(self.error("预设项应为映射")),
            }
        }
        Ok(())
    }

    /// 写回；内容未变化时也会重写（不做额外差异检测）。
    pub fn save(&self) -> Result<()> {
        let text = yaml_serde::to_string(&Value::Sequence(self.items.clone())).map_err(|err| {
            CoreError::Yaml {
                path: self.path.clone(),
                message: err.to_string(),
            }
        })?;
        write_atomic(&self.path, text.as_bytes())
    }

    // -----------------------------------------------------------------------
    // 内部
    // -----------------------------------------------------------------------

    fn error(&self, message: impl Into<String>) -> CoreError {
        CoreError::Yaml {
            path: self.path.clone(),
            message: message.into(),
        }
    }

    fn parse_item(&self, value: &Value) -> Result<Preset> {
        let mapping = value
            .as_mapping()
            .ok_or_else(|| self.error("预设项应为映射"))?;

        let id = self.required_string(mapping, "id")?;
        let name = self.required_string(mapping, "name")?;
        let icon = self.optional_string(mapping, "icon")?;
        let order = match mapping.get("order") {
            None | Some(Value::Null) => 0,
            Some(Value::Number(_)) => mapping
                .get("order")
                .and_then(Value::as_i64)
                .ok_or_else(|| self.error("预设 order 应为整数"))?,
            Some(_) => return Err(self.error("预设 order 应为整数")),
        };
        let members = match mapping.get("members") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Sequence(items)) => {
                let mut out: Vec<String> = Vec::with_capacity(items.len());
                for item in items {
                    let Value::String(text) = item else {
                        return Err(self.error("预设 members 列表项应为字符串"));
                    };
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if !out.iter().any(|existing| existing == trimmed) {
                        out.push(trimmed.to_string());
                    }
                }
                out
            }
            Some(_) => return Err(self.error("预设 members 应为列表")),
        };

        Ok(Preset {
            id,
            name,
            icon,
            order,
            members,
        })
    }

    fn required_string(&self, mapping: &Mapping, key: &str) -> Result<String> {
        self.optional_string(mapping, key)?
            .ok_or_else(|| self.error(format!("预设缺少 {key}")))
    }

    fn optional_string(&self, mapping: &Mapping, key: &str) -> Result<Option<String>> {
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
            Some(_) => Err(self.error(format!("预设 {key} 应为字符串"))),
        }
    }

    fn index_of(&self, id: &str) -> Result<Option<usize>> {
        for (index, item) in self.items.iter().enumerate() {
            if let Value::Mapping(mapping) = item {
                if let Some(Value::String(existing)) = mapping.get("id") {
                    if names_equal(existing, id) {
                        return Ok(Some(index));
                    }
                }
            }
        }
        Ok(None)
    }

    fn update(
        &mut self,
        id: &str,
        change: impl FnOnce(&mut Mapping),
    ) -> Result<Preset> {
        let index = self.index_of(id)?.ok_or_else(|| not_found(id))?;
        if let Value::Mapping(mapping) = &mut self.items[index] {
            change(mapping);
        }
        self.parse_item(&self.items[index].clone())
    }

    fn next_order(&self) -> Result<i64> {
        let mut max = -1;
        for preset in self.presets()? {
            max = max.max(preset.order);
        }
        Ok(max + 1)
    }

    /// 由名称派生域内唯一的 id：保留 ASCII 字母数字，其余折为 `-`，纯非 ASCII 时回退 `preset`。
    fn unique_id(&self, name: &str) -> Result<String> {
        let base = slugify(name);
        if self.index_of(&base)?.is_none() {
            return Ok(base);
        }
        let mut suffix = 2;
        loop {
            let candidate = format!("{base}-{suffix}");
            if self.index_of(&candidate)?.is_none() {
                return Ok(candidate);
            }
            suffix += 1;
        }
    }
}

fn not_found(id: &str) -> CoreError {
    CoreError::other(format!("预设不存在：{id}"))
}

/// 名称 → id：小写 ASCII 字母数字，其他字符折叠为单个 `-`，首尾 `-` 去除。
fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(ch.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }
    if out.is_empty() {
        "preset".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_presets(root: &Path, domain: Domain, body: &str) {
        let path = presets_path(root, domain);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn reads_definition_with_order_and_icon() {
        let root = tempfile::TempDir::new().unwrap();
        write_presets(
            root.path(),
            Domain::Code,
            "- id: rust\n  name: Rust 依赖检查\n  icon: shield\n  order: 2\n  members: [a, b]\n",
        );
        let file = PresetFile::load(root.path(), Domain::Code).unwrap();
        let presets = file.presets().unwrap();
        assert_eq!(presets.len(), 1);
        let preset = &presets[0];
        assert_eq!(preset.id, "rust");
        assert_eq!(preset.name, "Rust 依赖检查");
        assert_eq!(preset.icon.as_deref(), Some("shield"));
        assert_eq!(preset.order, 2);
        assert_eq!(preset.members, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn missing_file_is_empty_and_domains_are_separate_files() {
        let root = tempfile::TempDir::new().unwrap();
        let file = PresetFile::load(root.path(), Domain::Lobster).unwrap();
        assert!(file.presets().unwrap().is_empty());
        assert!(presets_path(root.path(), Domain::Code).ends_with("presets/code.yaml"));
        assert!(presets_path(root.path(), Domain::Lobster).ends_with("presets/lobster.yaml"));
    }

    #[test]
    fn unknown_fields_survive_write_back() {
        let root = tempfile::TempDir::new().unwrap();
        write_presets(
            root.path(),
            Domain::Code,
            "- id: rust\n  name: Rust\n  order: 0\n  members: []\n  extra: keep-me\n",
        );
        let mut file = PresetFile::load(root.path(), Domain::Code).unwrap();
        file.rename("rust", "Rust 检查").unwrap();
        file.save().unwrap();

        let text = std::fs::read_to_string(presets_path(root.path(), Domain::Code)).unwrap();
        assert!(text.contains("extra: keep-me"), "{text}");
        assert!(text.contains("Rust 检查"), "{text}");
        assert_eq!(
            PresetFile::load(root.path(), Domain::Code)
                .unwrap()
                .find("rust")
                .unwrap()
                .unwrap()
                .name,
            "Rust 检查"
        );
    }

    #[test]
    fn create_derives_unique_slug_and_next_order() {
        let root = tempfile::TempDir::new().unwrap();
        write_presets(
            root.path(),
            Domain::Code,
            "- id: rust\n  name: Rust\n  order: 4\n  members: []\n",
        );
        let mut file = PresetFile::load(root.path(), Domain::Code).unwrap();
        let created = file.create("Rust 依赖检查", Some("shield")).unwrap();
        assert_eq!(created.id, "rust-2");
        assert_eq!(created.order, 5);
        let chinese = file.create("依赖检查", None).unwrap();
        assert_eq!(chinese.id, "preset");
    }

    #[test]
    fn malformed_shape_is_reported_not_swallowed() {
        let root = tempfile::TempDir::new().unwrap();
        write_presets(root.path(), Domain::Code, "id: not-a-list\n");
        let err = PresetFile::load(root.path(), Domain::Code).unwrap_err();
        assert!(err.to_string().contains("顶层应为列表"), "{err}");

        write_presets(root.path(), Domain::Code, "- name: 无名\n");
        let file = PresetFile::load(root.path(), Domain::Code).unwrap();
        let err = file.presets().unwrap_err();
        assert!(err.to_string().contains("缺少 id"), "{err}");
    }
}
