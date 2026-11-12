//! Host 出站对接管理站：一次性码、长连接、解除。

use std::time::Duration;

use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::HostRuntime;
use crate::identity::{self, HostIdentity};

/// 出站 WebSocket Ping 间隔。须短于管理站曾用的 45s 空闲 TTL。
const WS_HEARTBEAT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairRequest {
    code: String,
    device_id: String,
    display_name: String,
    device_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairResponse {
    #[serde(default)]
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

fn random_key() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// `host pair --station --code`。
pub fn pair(
    runtime: &HostRuntime,
    station_url: &str,
    code: &str,
    display_name: Option<&str>,
) -> Result<HostIdentity, String> {
    let name = display_name
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(identity::default_display_name);
    let device_id = Uuid::new_v4().to_string();
    let device_key = random_key();
    let url = format!("{}/pair", station_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| err.to_string())?;
    let response = client
        .post(&url)
        .json(&PairRequest {
            code: code.to_string(),
            device_id: device_id.clone(),
            display_name: name.clone(),
            device_key: device_key.clone(),
        })
        .send()
        .map_err(|err| format!("无法连接管理站：{err}"))?;
    if !response.status().is_success() {
        let text = response.text().unwrap_or_default();
        return Err(format!("对接失败：{text}"));
    }
    let body: PairResponse = response.json().unwrap_or(PairResponse {
        ok: true,
        error: None,
    });
    if let Some(error) = body.error {
        return Err(error);
    }
    persist(
        runtime,
        HostIdentity {
            device_id: device_id.clone(),
            display_name: name,
            station_url: station_url.trim_end_matches('/').to_string(),
            device_key: device_key.clone(),
            paired_at: Some(chrono::Utc::now().to_rfc3339()),
        },
    )?;
    spawn_link(
        runtime.clone(),
        station_url.trim_end_matches('/').to_string(),
        device_id,
        device_key,
    );
    identity::read(runtime.get()?.state())
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "对接已完成但本机身份未写入".to_string())
}

/// `host unpair`：清本机 station_url / device_key。
pub fn unpair(runtime: &HostRuntime) -> Result<(), String> {
    let ctx = runtime.get()?;
    let store = ctx.state();
    let conn = store.connection();
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='host_identity'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if exists == 0 {
        return Ok(());
    }
    conn.execute(
        "UPDATE host_identity SET station_url = '', device_key = '', paired_at = NULL",
        [],
    )
    .map_err(|err| err.to_string())?;
    Ok(())
}

fn persist(runtime: &HostRuntime, identity: HostIdentity) -> Result<(), String> {
    let ctx = runtime.get()?;
    crate::identity_store::upsert(ctx.state(), &identity)
}

fn spawn_link(runtime: HostRuntime, station_url: String, device_id: String, device_key: String) {
    // 已在 `host serve` 的运行时里时，把出站长连接挂在同一执行器上，
    // 不要再 `Runtime::new()`：双运行时上 `tokio::spawn` 的 invoke 任务可能永不被轮询。
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
            link_loop(runtime, station_url, device_id, device_key).await;
        });
        return;
    }
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(_) => return,
        };
        rt.block_on(link_loop(runtime, station_url, device_id, device_key));
    });
}

async fn link_loop(
    runtime: HostRuntime,
    station_url: String,
    device_id: String,
    device_key: String,
) {
    let ws_url = station_ws_url(&station_url, &device_id);
    loop {
        match connect_once(&runtime, &ws_url, &device_key).await {
            Ok(()) => {}
            Err(err) => {
                eprintln!("管理站长连接中断：{err}");
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
        // 解除对接后 identity.station_url 为空，停止。
        let still_paired = runtime.get().ok().and_then(|ctx| {
            identity::read(ctx.state())
                .ok()
                .flatten()
                .map(|id| !id.station_url.is_empty() && id.device_id == device_id)
        });
        if still_paired != Some(true) {
            break;
        }
    }
}

/// Host 启动时若已对接，恢复出站长连接。
pub fn resume_if_paired(runtime: &HostRuntime) {
    let Some((station_url, device_id, device_key)) = runtime.get().ok().and_then(|ctx| {
        identity::read(ctx.state()).ok().flatten().and_then(|id| {
            if id.station_url.is_empty() || id.device_key.is_empty() {
                None
            } else {
                Some((id.station_url, id.device_id, id.device_key))
            }
        })
    }) else {
        return;
    };
    spawn_link(runtime.clone(), station_url, device_id, device_key);
}

fn station_ws_url(station_url: &str, device_id: &str) -> String {
    let http = station_url.trim_end_matches('/');
    let ws = if let Some(rest) = http.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = http.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        format!("ws://{http}")
    };
    format!("{ws}/host/ws/{device_id}")
}

async fn connect_once(runtime: &HostRuntime, ws_url: &str, device_key: &str) -> Result<(), String> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    let mut request = ws_url
        .into_client_request()
        .map_err(|err| err.to_string())?;
    request.headers_mut().insert(
        "authorization",
        format!("Bearer {device_key}")
            .parse()
            .map_err(|err: axum::http::header::InvalidHeaderValue| err.to_string())?,
    );
    let (ws, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|err| err.to_string())?;
    let (mut sink, mut stream) = ws.split();
    let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    let mut events = runtime.sse().subscribe();
    let mut heartbeat = tokio::time::interval(WS_HEARTBEAT);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    heartbeat.tick().await;
    loop {
        tokio::select! {
            biased;
            incoming = stream.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let runtime = runtime.clone();
                        let out_tx = out_tx.clone();
                        let payload = text.to_string();
                        std::thread::spawn(move || {
                            dispatch_station_invoke(runtime, out_tx, payload);
                        });
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = out_tx.send(Message::Pong(data));
                    }
                    Some(Ok(Message::Close(_))) | None => return Ok(()),
                    Some(Err(err)) => return Err(err.to_string()),
                    _ => {}
                }
            }
            outgoing = out_rx.recv() => {
                let Some(message) = outgoing else {
                    return Err("管理站出站通道已关闭".to_string());
                };
                if sink.send(message).await.is_err() {
                    return Err("管理站长连接已断开".to_string());
                }
            }
            event = events.recv() => {
                if let Ok(event) = event {
                    let payload = json!({
                        "type": "sse",
                        "event": event.name,
                        "data": event.payload,
                    });
                    let _ = out_tx.send(Message::Text(payload.to_string().into()));
                }
            }
            _ = heartbeat.tick() => {
                if out_tx.send(Message::Ping(Default::default())).is_err() {
                    return Err("管理站心跳发送失败".to_string());
                }
            }
        }
    }
}

fn dispatch_station_invoke(
    runtime: HostRuntime,
    out_tx: tokio::sync::mpsc::UnboundedSender<tokio_tungstenite::tungstenite::Message>,
    text: String,
) {
    use crate::invoke::{self, InvokeSource};
    use tokio_tungstenite::tungstenite::Message;

    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(err) => {
            let reply = json!({
                "type": "invoke_result",
                "id": "",
                "ok": false,
                "status": 400,
                "body": { "error": format!("无法解析管理站请求：{err}") },
            });
            let _ = out_tx.send(Message::Text(reply.to_string().into()));
            return;
        }
    };
    if value.get("type").and_then(|v| v.as_str()) != Some("invoke") {
        return;
    }
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let command = value
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let body = value.get("body").cloned().unwrap_or(json!({}));
    let session = value
        .get("sessionId")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let result = invoke::invoke(
        &runtime,
        &command,
        body,
        InvokeSource::Gui,
        session.as_deref(),
    );
    let (ok, status, payload) = match result {
        Ok(v) => (true, 200u16, v),
        Err(err) => {
            let (status, body) = err.status_and_body();
            (false, status, body)
        }
    };
    let reply = json!({
        "type": "invoke_result",
        "id": id,
        "ok": ok,
        "status": status,
        "body": payload,
    });
    let _ = out_tx.send(Message::Text(reply.to_string().into()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_tungstenite::tungstenite::Message;

    #[test]
    fn host_ws_heartbeat_is_shorter_than_former_idle_ttl() {
        assert!(WS_HEARTBEAT < Duration::from_secs(45));
        assert_eq!(WS_HEARTBEAT, Duration::from_secs(15));
    }

    #[test]
    fn station_invoke_replies_on_the_outbox_without_tokio_spawn() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = skill_manager_core::AppContext::open(Some(dir.path().to_path_buf())).unwrap();
        let runtime = HostRuntime::from_context(ctx);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        dispatch_station_invoke(
            runtime,
            tx,
            json!({
                "type": "invoke",
                "id": "req-1",
                "command": "agent_runtime_get",
                "sessionId": "gui-session",
                "body": {},
            })
            .to_string(),
        );
        let Message::Text(text) = rx.try_recv().expect("invoke_result") else {
            panic!("expected text reply");
        };
        let reply: serde_json::Value = serde_json::from_str(text.as_str()).unwrap();
        assert_eq!(reply["type"], "invoke_result");
        assert_eq!(reply["id"], "req-1");
        assert_eq!(reply["ok"], true);
        assert_eq!(reply["status"], 200);
        assert_eq!(reply["body"]["cliId"], "opencode");
    }
}
