//! ACP `session/new.mcpServers` 用的 stdio MCP：把 tools/list、tools/call 转到运行中的 Host。
//! 这是向 CLI 声明 Host 工具的协议通道，不是给用户用的平行 MCP 产品，也不占 host.lock。

use std::io::{BufRead, BufReader, Read, Write};
use std::time::Duration;

use serde_json::{Value, json};

use super::tools;
use crate::invoke::InvokeSource;

pub fn run_stdio() -> Result<(), String> {
    let listen = std::env::var("SKILL_MANAGER_HOST_LISTEN").map_err(|_| {
        "缺少 SKILL_MANAGER_HOST_LISTEN。host mcp 只能由已运行的 Host 经 ACP 拉起。".to_string()
    })?;
    let session = std::env::var("SKILL_MANAGER_AGENT_SESSION").unwrap_or_default();
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = std::io::stdout();
    loop {
        let msg = match read_msg(&mut reader) {
            Ok(value) => value,
            Err(err) if err == "eof" => return Ok(()),
            Err(err) => return Err(err),
        };
        if let Some(id) = msg.get("id").cloned() {
            let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
            let params = msg.get("params").cloned().unwrap_or(json!({}));
            match handle(&listen, &session, method, params) {
                Ok(result) => write_msg(
                    &mut stdout,
                    &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                )?,
                Err(err) => write_msg(
                    &mut stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32000, "message": err }
                    }),
                )?,
            }
        }
    }
}

pub(crate) fn handle(
    listen: &str,
    session: &str,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": params
                .get("protocolVersion")
                .cloned()
                .unwrap_or(json!("2024-11-05")),
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "skill-manager", "version": env!("CARGO_PKG_VERSION") },
        })),
        "notifications/initialized" | "initialized" | "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools::mcp_tools() })),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| "tools/call 缺少 name".to_string())?;
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            call_host(listen, session, name, args)
        }
        other => Err(format!("未知 MCP 方法：{other}")),
    }
}

fn call_host(listen: &str, session: &str, name: &str, args: Value) -> Result<Value, String> {
    let Some(canonical) = tools::tool_command_name(name) else {
        return Ok(mcp_error(format!("未知或不允许的管理工具：{name}")));
    };
    let args = tools::normalize_args(&canonical, args);
    match crate::http::forward_invoke(
        listen,
        &canonical,
        &args,
        InvokeSource::Agent,
        if session.is_empty() {
            None
        } else {
            Some(session)
        },
    ) {
        Ok(value) => Ok(json!({
            "content": [{ "type": "text", "text": value.to_string() }],
            "isError": false
        })),
        Err(err) => Ok(mcp_error(err)),
    }
}

fn mcp_error(message: String) -> Value {
    json!({
        "content": [{ "type": "text", "text": message }],
        "isError": true
    })
}

fn read_msg(stdout: &mut BufReader<impl Read>) -> Result<Value, String> {
    let mut header = String::new();
    let n = stdout
        .read_line(&mut header)
        .map_err(|err| format!("读取 MCP stdin 失败：{err}"))?;
    if n == 0 {
        return Err("eof".into());
    }
    let trimmed = header.trim();
    if trimmed.is_empty() {
        return read_msg(stdout);
    }
    if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
        let len: usize = rest
            .trim()
            .parse()
            .map_err(|_| "无效的 Content-Length".to_string())?;
        let mut blank = String::new();
        stdout
            .read_line(&mut blank)
            .map_err(|err| err.to_string())?;
        let mut buf = vec![0u8; len];
        stdout.read_exact(&mut buf).map_err(|err| err.to_string())?;
        return serde_json::from_slice(&buf).map_err(|err| err.to_string());
    }
    serde_json::from_str(trimmed).map_err(|err| err.to_string())
}

fn write_msg(out: &mut impl Write, value: &Value) -> Result<(), String> {
    let body = serde_json::to_vec(value).map_err(|err| err.to_string())?;
    write!(out, "Content-Length: {}\r\n\r\n", body.len()).map_err(|err| err.to_string())?;
    out.write_all(&body).map_err(|err| err.to_string())?;
    out.flush().map_err(|err| err.to_string())
}

#[allow(dead_code)]
fn _timeout() -> Duration {
    Duration::from_secs(600)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_has_schema() {
        let listed = handle("unused", "", "tools/list", json!({})).unwrap();
        let tools = listed["tools"].as_array().unwrap();
        assert!(
            tools
                .iter()
                .any(|t| t["name"] == "skill_list" && t["inputSchema"]["type"] == "object")
        );
        assert!(tools.iter().any(|t| t["name"] == "skill_set"));
    }

    #[test]
    fn unknown_tool_is_error_content_not_success_write() {
        let result = handle(
            "127.0.0.1:9",
            "s",
            "tools/call",
            json!({
                "name": "bash",
                "arguments": { "command": "rm -rf /" }
            }),
        )
        .unwrap();
        assert_eq!(result["isError"], true);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("不允许") || text.contains("未知"));
    }
}
