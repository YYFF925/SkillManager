//! YAML 安全写回：防止字符串被 YAML 1.1 隐式类型改写。
//!
//! `yaml_serde` 的 emitter 按 YAML 1.2 core 判定 plain 标量安全性，
//! `2026-10-08`、`12:30`、`no`、`on` 这类形状按 1.2 是字符串，不加引号输出；
//! 但 PyYAML 等下游消费者按 YAML 1.1 解析会得到 date/int/bool，键与值都被改型
//! （值被改写、类型消费方崩溃）。本模块负责两件事：
//!
//! - [`needs_quotes`]：判定一个字符串以 plain 标量输出时是否会被 1.1 消费者类型化；
//! - [`to_yaml_string`]：写回入口。树内不含危险字符串时直接走 `yaml_serde`
//!   （输出与历史行为逐字节一致）；含危险字符串时改用本模块的保守发射器，
//!   字符串全部带引号输出。含 `Value::Tagged` 时回退 `yaml_serde`
//!   （标签值无法安全重发射；与危险字符串并存的理论组合仍有改型窗口，仓库内无此用例）。
//!
//! 隐式类型规则对照 PyYAML 的 resolver（YAML 1.1），另收严：单字母 `y`/`n`、
//! 合并键 `<<`、值标记 `=`。

use std::sync::LazyLock;

use regex::Regex;
use yaml_serde::{Mapping, Value};

/// PyYAML 的 1.1 隐式 int/float（含六十进制、十六进制、前导零八进制、`.inf`/`.nan`）。
static NUMBER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        ^(?:
            [-+]?(?:0b[0-1_]+|0x[0-9a-fA-F_]+|0[0-7_]+|0|[1-9][0-9_]*(?::[0-5]?[0-9])*)
            |[-+]?[0-9][0-9_]*\.[0-9_]*(?:[eE][-+][0-9]+)?
            |\.[0-9_]+(?:[eE][-+][0-9]+)?
            |[-+]?[0-9][0-9_]*(?::[0-5]?[0-9])+\.[0-9_]*
            |[-+]?\.(?:inf|Inf|INF)
            |\.(?:nan|NaN|NAN)
        )$",
    )
    .expect("常量正则必然合法")
});

/// PyYAML 的 1.1 隐式 timestamp（日期与完整/带时区的时间形式）。
static TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        ^(?:
            [0-9]{4}-[0-9]{2}-[0-9]{2}
            |[0-9]{4}-[0-9]{1,2}-[0-9]{1,2}(?:[Tt]|[\x20\t]+)[0-9]{1,2}:[0-9]{2}:[0-9]{2}
            (?:\.[0-9]*)?(?:[\x20\t]*(?:Z|[-+][0-9]{1,2}(?::[0-9]{2})?))?
        )$",
    )
    .expect("常量正则必然合法")
});

/// 字符串以 plain 标量输出时，会被 YAML 1.1 消费者解析成非字符串（含空串→null）。
pub(crate) fn needs_quotes(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    matches!(
        text,
        "~" | "null"
            | "Null"
            | "NULL"
            | "y" | "Y"
            | "n" | "N"
            | "yes" | "Yes" | "YES"
            | "no" | "No" | "NO"
            | "true" | "True" | "TRUE"
            | "false" | "False" | "FALSE"
            | "on" | "On" | "ON"
            | "off" | "Off" | "OFF"
            | "<<"
            | "="
    ) || NUMBER.is_match(text)
        || TIMESTAMP.is_match(text)
}

/// 字符串能否安全地以 plain 标量输出（不含指示符、`: `、` #`、控制字符等）。
pub(crate) fn can_be_plain(text: &str) -> bool {
    if text.is_empty() || text.trim() != text {
        return false;
    }
    let Some(first) = text.chars().next() else {
        return false;
    };
    if matches!(
        first,
        '-' | '?' | ':' | ',' | '[' | ']' | '{' | '}' | '#' | '&' | '*' | '!' | '|' | '>'
            | '\'' | '"' | '%' | '@' | '`'
    ) {
        return false;
    }
    if text.contains(": ") || text.contains(" #") || text.ends_with(':') {
        return false;
    }
    !text.chars().any(char::is_control)
}

/// 字符串带引号输出：常规内容用单引号（`'` 转义为 `''`），
/// 含换行/控制字符时改用双引号转义。
pub(crate) fn format_string(text: &str) -> String {
    if text.chars().any(char::is_control) {
        double_quoted(text)
    } else {
        format!("'{}'", text.replace('\'', "''"))
    }
}

/// 能 plain 则 plain、否则带引号——用于注入的 description 值，
/// 与历史输出保持一致（普通中文注释仍为 plain 标量）。
pub(crate) fn format_plain_if_safe(text: &str) -> String {
    if can_be_plain(text) && !needs_quotes(text) {
        text.to_string()
    } else {
        format_string(text)
    }
}

fn double_quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            ch if ch.is_control() => out.push_str(&format!("\\x{:02x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// 标量输出：字符串带引号；Null/Bool/Number 本身就是目标类型，交给 `yaml_serde`。
fn format_scalar(value: &Value) -> Result<String, yaml_serde::Error> {
    match value {
        Value::String(text) => Ok(format_string(text)),
        other => Ok(yaml_serde::to_string(other)?.trim_end().to_string()),
    }
}

fn key_text(key: &Value) -> Result<String, yaml_serde::Error> {
    match key {
        Value::String(text) => Ok(format_string(text)),
        other => format_scalar(other),
    }
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push(' ');
    }
}

/// 块风格映射（2 空格缩进，按插入序）；首行缩进已剥离，便于悬挂到 `- ` 或 `key:` 之后。
fn emit_mapping(map: &Mapping, indent: usize, out: &mut String) -> Result<(), yaml_serde::Error> {
    for (key, value) in map {
        push_indent(out, indent);
        out.push_str(&key_text(key)?);
        out.push(':');
        match value {
            Value::Mapping(nested) if !nested.is_empty() => {
                out.push('\n');
                emit_mapping(nested, indent + 2, out)?;
            }
            Value::Sequence(items) if !items.is_empty() => {
                out.push('\n');
                emit_sequence(items, indent + 2, out)?;
            }
            scalar_or_empty => {
                out.push(' ');
                out.push_str(&format_scalar(scalar_or_empty)?);
                out.push('\n');
            }
        }
    }
    Ok(())
}

/// 块风格序列；映射项悬挂为标准的 `- key: value` 形式。
fn emit_sequence(items: &[Value], indent: usize, out: &mut String) -> Result<(), yaml_serde::Error> {
    for item in items {
        push_indent(out, indent);
        out.push('-');
        match item {
            Value::Mapping(nested) if !nested.is_empty() => {
                out.push(' ');
                out.push_str(&emit_block(item, indent + 2)?);
            }
            Value::Sequence(nested) if !nested.is_empty() => {
                out.push(' ');
                out.push_str(&emit_block(item, indent + 2)?);
            }
            scalar_or_empty => {
                out.push(' ');
                out.push_str(&format_scalar(scalar_or_empty)?);
                out.push('\n');
            }
        }
    }
    Ok(())
}

fn emit_block(value: &Value, indent: usize) -> Result<String, yaml_serde::Error> {
    let mut block = String::new();
    match value {
        Value::Mapping(map) => emit_mapping(map, indent, &mut block)?,
        Value::Sequence(items) => emit_sequence(items, indent, &mut block)?,
        other => {
            block.push_str(&format_scalar(other)?);
            block.push('\n');
        }
    }
    Ok(block.trim_start_matches(' ').to_string())
}

fn contains_tagged(value: &Value) -> bool {
    match value {
        Value::Tagged(_) => true,
        Value::Sequence(items) => items.iter().any(contains_tagged),
        Value::Mapping(map) => map
            .iter()
            .any(|(key, item)| contains_tagged(key) || contains_tagged(item)),
        _ => false,
    }
}

/// 树内任何位置（含键）存在会被 1.1 消费者类型化的字符串。
fn contains_dangerous_string(value: &Value) -> bool {
    match value {
        Value::String(text) => needs_quotes(text),
        Value::Sequence(items) => items.iter().any(contains_dangerous_string),
        Value::Mapping(map) => map.iter().any(|(key, item)| {
            matches!(key, Value::String(text) if needs_quotes(text)) || contains_dangerous_string(item)
        }),
        _ => false,
    }
}

/// 安全写回入口。错误类型与 `yaml_serde::to_string` 一致，调用方沿用既有 `map_err`。
pub fn to_yaml_string(value: &Value) -> Result<String, yaml_serde::Error> {
    if contains_tagged(value) || !contains_dangerous_string(value) {
        return yaml_serde::to_string(value);
    }
    let mut out = String::new();
    match value {
        Value::Mapping(map) if !map.is_empty() => emit_mapping(map, 0, &mut out)?,
        Value::Sequence(items) if !items.is_empty() => emit_sequence(items, 0, &mut out)?,
        other => {
            out.push_str(&format_scalar(other)?);
            out.push('\n');
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_quotes_covers_yaml_1_1_implicit_types() {
        for text in [
            "2026-10-08",
            "2026-10-08T12:30:00Z",
            "2026-10-08 12:30:00",
            "2026-10-08T12:30:00.5+08:00",
            "12:30",
            "1:2:3",
            "12:30.5",
            "12:30:00",
            "no",
            "on",
            "y",
            "N",
            "TRUE",
            "Off",
            "null",
            "~",
            "",
            "0755",
            "00",
            "0_1",
            "0x1F",
            "0b101",
            "-1_000",
            "750",
            "1.10",
            "5.",
            ".5",
            "1.5e+10",
            ".inf",
            "-.Inf",
            ".nan",
            "<<",
            "=",
        ] {
            assert!(needs_quotes(text), "应判定需要引号：{text:?}");
        }
        for text in [
            "probe-skill",
            "中文注释",
            "v1.2",
            "2026-1-8",
            "12:345",
            "1e5",
            "a#b",
            "https://example.invalid/skills.git",
            "检查依赖｜解析依赖",
        ] {
            assert!(!needs_quotes(text), "不应误判需要引号：{text:?}");
        }
    }

    #[test]
    fn to_yaml_string_keeps_clean_trees_byte_identical() {
        // 普通文件（无危险字符串）必须与历史输出逐字节一致，避免无谓扰动。
        let text = "tags: [x]\ncomment: 旧注释\nfuture_field:\n  nested: 1\n";
        let value = yaml_serde::from_str::<Value>(text).unwrap();
        assert_eq!(
            to_yaml_string(&value).unwrap(),
            yaml_serde::to_string(&value).unwrap()
        );
    }

    #[test]
    fn to_yaml_string_quotes_dangerous_values_and_keys() {
        let mut map = Mapping::new();
        map.insert(Value::String("date".into()), Value::String("2026-10-08".into()));
        map.insert(Value::String("on".into()), Value::String("no".into()));
        map.insert(Value::String("comment".into()), Value::String("新注释".into()));
        let out = to_yaml_string(&Value::Mapping(map)).unwrap();
        assert!(out.contains("'date': '2026-10-08'"), "{out}");
        assert!(out.contains("'on': 'no'"), "{out}");
        // 键与值重解析后都仍是字符串。
        let reparsed = yaml_serde::from_str::<Value>(&out).unwrap();
        let map = reparsed.as_mapping().unwrap();
        assert_eq!(
            map.get(&Value::String("date".into())),
            Some(&Value::String("2026-10-08".into()))
        );
        assert_eq!(
            map.get(&Value::String("on".into())),
            Some(&Value::String("no".into()))
        );
    }

    #[test]
    fn conservative_emit_round_trips_nested_structures() {
        let text = "\
order: 3
agents: []
skills:
  - name: ego
    repo: https://example.invalid/a.git
    subpath: skills/ego
intro: |-
  第一行
  第二行
nested:
  deep:
    flag: \"no\"
    count: 7
";
        let value = yaml_serde::from_str::<Value>(text).unwrap();
        let out = to_yaml_string(&value).unwrap();
        let reparsed = yaml_serde::from_str::<Value>(&out).unwrap();
        assert_eq!(reparsed, value, "保守发射必须无损往返：\n{out}");
        // 显式空序列保持空字面量。
        assert!(out.contains("'agents': []"), "{out}");
        // 嵌套的危险字符串被引号保护。
        assert!(out.contains("'flag': 'no'"), "{out}");
    }

    #[test]
    fn conservative_emit_handles_non_string_keys_and_top_level_sequence() {
        // 布尔键：发射时带引号，重解析仍是字符串语义等价的布尔键。
        let mut inner = Mapping::new();
        inner.insert(Value::Bool(true), Value::String("x".into()));
        let seq = Value::Sequence(vec![
            Value::Mapping(inner),
            Value::Number(42i64.into()),
            Value::String("12:30".into()),
        ]);
        let out = to_yaml_string(&seq).unwrap();
        let reparsed = yaml_serde::from_str::<Value>(&out).unwrap();
        assert_eq!(reparsed, seq, "非字符串键与顶层序列必须无损往返：\n{out}");
    }

    #[test]
    fn tagged_values_fall_back_to_yaml_serde() {
        // yaml_serde 解析 Value 时会剥掉标签（历史行为，与本修复无关），
        // 实际到达写回点的树不再含 Tagged；这里直接构造验证回退分支本身。
        let tagged = Value::Tagged(Box::new(yaml_serde::value::TaggedValue {
            tag: yaml_serde::value::Tag::new("!!str"),
            value: Value::String("2026-10-08".into()),
        }));
        let mut map = Mapping::new();
        map.insert(Value::String("custom".into()), tagged);
        let value = Value::Mapping(map);
        assert_eq!(
            to_yaml_string(&value).unwrap(),
            yaml_serde::to_string(&value).unwrap()
        );
    }
}
