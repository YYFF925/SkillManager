//! 读取档案 `config.yaml` 的 `skills.disabled` 禁用名单（只读，T056）。
//!
//! Hermes 的 `disabled` 是**禁用名单（黑名单）而非白名单**；未列出的可扫描技能即进入加载范围。
//! 本项目只读取该名单用于状态判定与诚实报告，**不写用户配置**（原因见 research.md §5.4）。
//! 区分“文件不存在”（合法空名单）与“文件存在但读不了”（报错，不折叠为全未禁用）。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// 档案 `skills.disabled` 的读取结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisabledState {
    /// `config.yaml` 不存在：合法空名单。
    Missing,
    /// 已读取的禁用名单（可能为空）。
    Names(BTreeSet<String>),
    /// 文件存在但读不了或不是有效 YAML：状态不作有效结论。
    Unreadable(String),
}

impl DisabledState {
    /// 该技能名是否被显式禁用。
    pub fn contains(&self, name: &str) -> bool {
        match self {
            DisabledState::Names(names) => names.iter().any(|item| item == name),
            _ => false,
        }
    }

    /// 读取失败原因（如有）。
    pub fn error(&self) -> Option<&str> {
        match self {
            DisabledState::Unreadable(reason) => Some(reason.as_str()),
            _ => None,
        }
    }
}

/// 读取档案 home 下的 `config.yaml`，取出 `skills.disabled`。
pub fn read_disabled(profile_home: &Path) -> DisabledState {
    let path: PathBuf = profile_home.join("config.yaml");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return DisabledState::Missing,
        Err(err) => return DisabledState::Unreadable(format!("{}：{err}", path.display())),
    };

    let value: yaml_serde::Value = match yaml_serde::from_str(&text) {
        Ok(value) => value,
        Err(err) => return DisabledState::Unreadable(format!("{} 解析失败：{err}", path.display())),
    };

    let disabled = value
        .get("skills")
        .and_then(|skills| skills.get("disabled"));

    DisabledState::Names(parse_disabled_value(disabled))
}

/// 解析 `skills.disabled` 值：兼容 YAML 列表、JSON/Python 风格列表字符串与单个标量字符串。
fn parse_disabled_value(value: Option<&yaml_serde::Value>) -> BTreeSet<String> {
    use yaml_serde::Value;

    let mut out = BTreeSet::new();
    match value {
        None | Some(Value::Null) => {}
        Some(Value::Sequence(items)) => {
            for item in items {
                if let Value::String(text) = item {
                    push_names(&mut out, text);
                }
            }
        }
        Some(Value::String(text)) => push_names(&mut out, text),
        // 形状不认识时按 Hermes 行为视为空名单（`_normalize_string_set` 吞掉类型错误）。
        Some(_) => {}
    }
    out
}

/// 一个字符串可能是单个技能名，也可能是 JSON/Python 风格列表字符串。
fn push_names(out: &mut BTreeSet<String>, raw: &str) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return;
    }
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .map(str::trim);
    if let Some(inner) = inner {
        for part in inner.split(',') {
            let name = part.trim().trim_matches(|c| c == '"' || c == '\'').trim();
            if !name.is_empty() {
                out.insert(name.to_string());
            }
        }
        return;
    }
    let name = trimmed.trim_matches(|c| c == '"' || c == '\'').trim();
    if !name.is_empty() {
        out.insert(name.to_string());
    }
}

/// 档案的 `config.yaml` 路径。
pub fn config_path(profile_home: &Path) -> PathBuf {
    profile_home.join("config.yaml")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(text: &str) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("config.yaml"), text).unwrap();
        dir
    }

    #[test]
    fn missing_file_is_empty_not_error() {
        let dir = tempfile::TempDir::new().unwrap();
        assert_eq!(read_disabled(dir.path()), DisabledState::Missing);
    }

    #[test]
    fn block_list_and_inline_forms_are_read() {
        let block = write("skills:\n  disabled:\n    - alpha\n    - beta\n");
        assert!(read_disabled(block.path()).contains("alpha"));
        assert!(!read_disabled(block.path()).contains("gamma"));

        let json = write("skills:\n  disabled: '[\"alpha\", \"beta\"]'\n");
        assert!(read_disabled(json.path()).contains("beta"));

        let python = write("skills:\n  disabled: \"['alpha']\"\n");
        assert!(read_disabled(python.path()).contains("alpha"));

        let scalar = write("skills:\n  disabled: alpha\n");
        assert!(read_disabled(scalar.path()).contains("alpha"));

        let empty = write("skills:\n  disabled:\n");
        assert_eq!(
            read_disabled(empty.path()),
            DisabledState::Names(BTreeSet::new())
        );
    }

    #[test]
    fn unreadable_is_distinct_from_missing() {
        let broken = write("skills: [this is: not valid\n");
        assert!(matches!(
            read_disabled(broken.path()),
            DisabledState::Unreadable(_)
        ));
    }
}
