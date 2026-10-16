//! CLI/入口统一输出：`result` + `data` + `errors`，批量逐项给结果与原因。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 整体结果。非 0 退出码用于失败与部分未完成。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Success,
    Partial,
    Failed,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Success => "success",
            Outcome::Partial => "partial",
            Outcome::Failed => "failed",
        }
    }
}

/// 单项失败原因：带上技能/目标/实际路径，便于定位。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ItemError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl ItemError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            skill: None,
            target: None,
            path: None,
        }
    }

    pub fn skill(mut self, skill: impl Into<String>) -> Self {
        self.skill = Some(skill.into());
        self
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// 从核心错误构造，自动带上可定位信息。
    pub fn from_error(error: &crate::error::CoreError) -> Self {
        Self {
            message: error.to_string(),
            skill: error.skill_hint(),
            target: None,
            path: error.path_hint(),
        }
    }
}

impl From<crate::error::CoreError> for ItemError {
    fn from(value: crate::error::CoreError) -> Self {
        ItemError::from_error(&value)
    }
}

/// 统一输出对象。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CliResult {
    pub result: Outcome,
    pub data: Value,
    pub errors: Vec<ItemError>,
}

impl CliResult {
    pub fn success(data: Value) -> Self {
        Self {
            result: Outcome::Success,
            data,
            errors: Vec::new(),
        }
    }

    /// 成功且带数据，用于统一序列化。
    pub fn from_data<T: Serialize>(data: &T) -> Self {
        Self::success(serde_json::to_value(data).unwrap_or(Value::Null))
    }

    /// 部分完成：给出成功数据与未完成项原因。
    pub fn partial(data: Value, errors: Vec<ItemError>) -> Self {
        debug_assert!(!errors.is_empty(), "partial 必须带未完成项原因");
        Self {
            result: Outcome::Partial,
            data,
            errors,
        }
    }

    pub fn failed(errors: Vec<ItemError>) -> Self {
        Self {
            result: Outcome::Failed,
            data: Value::Null,
            errors,
        }
    }

    pub fn failed_message(message: impl Into<String>) -> Self {
        Self::failed(vec![ItemError::new(message)])
    }

    pub fn failed_error(error: crate::error::CoreError) -> Self {
        Self::failed(vec![ItemError::from_error(&error)])
    }

    /// 成功 0；失败 1；部分完成 2。
    pub fn exit_code(&self) -> i32 {
        match self.result {
            Outcome::Success => 0,
            Outcome::Failed => 1,
            Outcome::Partial => 2,
        }
    }

    /// 输出结果并返回进程退出码。`human` 为默认（非 JSON）可读文本。
    pub fn emit(&self, json: bool, human: &str) -> i32 {
        if json {
            match serde_json::to_string_pretty(self) {
                Ok(text) => println!("{text}"),
                Err(err) => {
                    eprintln!("序列化输出失败：{err}");
                    return 1;
                }
            }
        } else {
            if !human.is_empty() {
                println!("{human}");
            }
            for error in &self.errors {
                eprintln!("未完成：{}", format_item_error(error));
            }
        }
        self.exit_code()
    }
}

fn format_item_error(error: &ItemError) -> String {
    let mut prefix = String::new();
    if let Some(skill) = &error.skill {
        prefix.push_str(skill);
        prefix.push(' ');
    }
    if let Some(target) = &error.target {
        prefix.push_str(&format!("[{target}] "));
    }
    if let Some(path) = &error.path {
        prefix.push_str(&format!("{path}："));
    }
    format!("{prefix}{}", error.message)
}

/// 批量逐项结果收集器。
#[derive(Debug, Default)]
pub struct BatchOutcome {
    pub succeeded: Vec<Value>,
    pub failed: Vec<ItemError>,
}

impl BatchOutcome {
    pub fn push_ok(&mut self, item: Value) {
        self.succeeded.push(item);
    }

    pub fn push_err(&mut self, error: ItemError) {
        self.failed.push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.succeeded.is_empty() && self.failed.is_empty()
    }

    /// 全成 / 部分 / 全败三态。
    pub fn into_result(self, prefix: &str) -> CliResult {
        let data = serde_json::json!({
            format!("{prefix}"): self.succeeded,
        });
        if self.failed.is_empty() {
            CliResult::success(data)
        } else if self.succeeded.is_empty() {
            CliResult::failed(self.failed)
        } else {
            CliResult::partial(data, self.failed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_distinguish_partial_from_failed() {
        assert_eq!(CliResult::success(Value::Null).exit_code(), 0);
        assert_eq!(CliResult::failed_message("x").exit_code(), 1);
        assert_eq!(
            CliResult::partial(Value::Null, vec![ItemError::new("x")]).exit_code(),
            2
        );
    }

    #[test]
    fn serialized_shape_is_stable() {
        let result = CliResult::success(serde_json::json!({"skills": []}));
        let value = serde_json::to_value(&result).unwrap();
        assert_eq!(value["result"], "success");
        assert!(value.get("data").is_some());
        assert_eq!(value["errors"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn batch_outcome_reports_per_item_reason() {
        let mut batch = BatchOutcome::default();
        batch.push_ok(serde_json::json!({"skill": "code/a"}));
        batch.push_err(ItemError::new("目标外改暂停").skill("code/b").target("agent:codex"));
        let result = batch.into_result("items");
        assert_eq!(result.result, Outcome::Partial);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].target.as_deref(), Some("agent:codex"));
    }
}
