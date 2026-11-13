//! Host HTTP：invoke 同名 POST、SSE、会话、确认计划、对话入口。

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use futures_util::stream;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::invoke::{self, InvokeError, InvokeSource};
use crate::plans;
use crate::HostRuntime;

#[derive(Clone)]
struct App {
    runtime: HostRuntime,
}

/// 绑定回环地址并服务，直到取消。返回实际监听地址。
pub async fn serve(runtime: HostRuntime, addr: SocketAddr) -> Result<SocketAddr, String> {
    let listener = TcpListener::bind(addr).await.map_err(|err| err.to_string())?;
    let bound = listener.local_addr().map_err(|err| err.to_string())?;
    runtime.set_listen(bound.to_string())?;
    let app = router(runtime);
    axum::serve(listener, app)
        .await
        .map_err(|err| err.to_string())?;
    Ok(bound)
}

pub fn router(runtime: HostRuntime) -> Router {
    Router::new()
        .route("/host/status", get(host_status))
        .route("/session", post(create_session))
        .route("/plans", post(create_plan))
        .route("/plans/{id}/confirm", post(confirm_plan))
        .route("/invoke/{command}", post(invoke_command))
        .route("/events", get(events))
        .route("/agent/context", put(agent_context))
        .route("/agent/messages", post(agent_message))
        .route("/agent/tasks/{id}/cancel", post(agent_cancel))
        .route("/agent/sessions", get(agent_sessions))
        .route("/agent/sessions/{id}/messages", get(agent_messages))
        .route("/agent/runtime", get(agent_runtime_get).put(agent_runtime_put))
        .route("/agent/runtime/install", post(agent_runtime_install))
        .route("/agent/runtime/login", post(agent_runtime_login))
        .route("/agent/models", get(agent_models_get).put(agent_set_model))
        .with_state(App { runtime })
}

fn source_from(headers: &HeaderMap) -> InvokeSource {
    match headers
        .get("x-skill-manager-source")
        .and_then(|v| v.to_str().ok())
    {
        Some("cli") => InvokeSource::Cli,
        Some("agent") => InvokeSource::Agent,
        _ => InvokeSource::Gui,
    }
}

fn session_from(headers: &HeaderMap, body: &Value) -> Option<String> {
    body.get("sessionId")
        .or_else(|| body.get("session_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("x-skill-manager-session")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        })
}

async fn host_status(State(app): State<App>) -> Json<Value> {
    Json(serde_json::to_value(app.runtime.status_view()).unwrap_or(json!(null)))
}

async fn create_session() -> Json<Value> {
    Json(json!({ "sessionId": plans::new_session() }))
}

#[derive(Deserialize)]
struct ConfirmBody {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

async fn confirm_plan(
    State(app): State<App>,
    Path(id): Path<String>,
    Json(body): Json<ConfirmBody>,
) -> impl IntoResponse {
    match plans::confirm(&app.runtime, &id, body.session_id.as_deref()) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(err) => (StatusCode::FORBIDDEN, Json(json!({ "error": err }))).into_response(),
    }
}

async fn create_plan() -> impl IntoResponse {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "请对删除或覆盖类命令直接 invoke；缺少 planId 时会返回待确认计划" })),
    )
}

async fn invoke_command(
    State(app): State<App>,
    Path(command): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let source = source_from(&headers);
    let session = session_from(&headers, &body);
    let runtime = app.runtime.clone();
    let result = tokio::task::spawn_blocking(move || {
        invoke::invoke(&runtime, &command, body, source, session.as_deref())
    })
    .await;
    match result {
        Ok(Ok(value)) => (StatusCode::OK, Json(value)).into_response(),
        Ok(Err(InvokeError::NeedsPlan(plan))) => (StatusCode::CONFLICT, Json(plan)).into_response(),
        Ok(Err(err)) => {
            let (status, body) = err.status_and_body();
            (StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_REQUEST), Json(body)).into_response()
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

async fn events(State(app): State<App>) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = app.runtime.sse().subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|item| match item {
        Ok(event) => Some(Ok(Event::default()
            .event(event.name)
            .json_data(event.payload)
            .unwrap_or_else(|_| Event::default()))),
        Err(_) => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

async fn agent_context(
    State(app): State<App>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    match crate::agent::set_context(&app.runtime, body) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response(),
    }
}

async fn agent_message(
    State(app): State<App>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    match crate::agent::post_message(&app.runtime, body) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response(),
    }
}

async fn agent_cancel(
    State(app): State<App>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    match crate::agent::cancel_task(&app.runtime, &id, body) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response(),
    }
}

async fn agent_runtime_get(State(app): State<App>) -> Json<Value> {
    Json(crate::agent::runtime_view(&app.runtime))
}

async fn agent_runtime_install(State(app): State<App>) -> impl IntoResponse {
    match crate::agent::install_default_cli(&app.runtime) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response(),
    }
}

async fn agent_runtime_login(State(app): State<App>) -> impl IntoResponse {
    match crate::agent::launch_login(&app.runtime) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response(),
    }
}

async fn agent_models_get(State(app): State<App>, headers: HeaderMap) -> impl IntoResponse {
    let body = match session_from(&headers, &json!({})) {
        Some(session) => json!({ "sessionId": session }),
        None => json!({}),
    };
    match crate::agent::list_models(&app.runtime, body) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response(),
    }
}

async fn agent_set_model(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let mut body = body;
    if let Some(session) = session_from(&headers, &body) {
        if let Some(obj) = body.as_object_mut() {
            obj.entry("sessionId".to_string())
                .or_insert_with(|| json!(session));
        }
    }
    match crate::agent::set_model(&app.runtime, body) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response(),
    }
}

async fn agent_runtime_put(
    State(app): State<App>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    if body.get("clear").and_then(Value::as_bool) == Some(true) {
        return match crate::agent::clear_user_cli(&app.runtime) {
            Ok(()) => (StatusCode::OK, Json(crate::agent::runtime_view(&app.runtime))).into_response(),
            Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response(),
        };
    }
    let bin = body
        .get("binaryPath")
        .or_else(|| body.get("bin"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if bin.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "缺少 binaryPath。清单外 CLI 用 PUT /agent/runtime { binaryPath, acpArgs } 登记。" })),
        )
            .into_response();
    }
    let args = body
        .get("acpArgs")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let id = body.get("cliId").and_then(Value::as_str);
    match crate::agent::set_user_cli(&app.runtime, bin, args, id) {
        Ok(_) => (StatusCode::OK, Json(crate::agent::runtime_view(&app.runtime))).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response(),
    }
}

async fn agent_sessions(State(app): State<App>) -> impl IntoResponse {
    match crate::agent::list_sessions(&app.runtime) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response(),
    }
}

async fn agent_messages(
    State(app): State<App>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match crate::agent::list_messages(&app.runtime, &id) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response(),
    }
}

/// 供 CLI 探测 Host 是否可转发。
pub fn try_local_client(state_dir: &std::path::Path) -> Option<String> {
    crate::lock::peek_meta(state_dir)?.listen
}

/// CLI 转发：把 JSON 体 POST 到本机 Host。
pub fn forward_invoke(
    listen: &str,
    command: &str,
    body: &Value,
    source: InvokeSource,
    session_id: Option<&str>,
) -> Result<Value, String> {
    let url = format!("http://{listen}/invoke/{command}");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|err| err.to_string())?;
    let mut req = client.post(&url).json(body).header(
        "x-skill-manager-source",
        match source {
            InvokeSource::Gui => "gui",
            InvokeSource::Cli => "cli",
            InvokeSource::Agent => "agent",
        },
    );
    if let Some(session) = session_id {
        req = req.header("x-skill-manager-session", session);
    }
    let response = req.send().map_err(|err| err.to_string())?;
    let status = response.status();
    let value: Value = response.json().unwrap_or(json!(null));
    if status.is_success() || status.as_u16() == 409 {
        Ok(value)
    } else {
        Err(value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("Host 转发失败")
            .to_string())
    }
}

pub fn probe(listen: &str) -> bool {
    let url = format!("http://{listen}/host/status");
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()
        .and_then(|c| c.get(url).send().ok())
        .is_some_and(|r| r.status().is_success())
}

#[allow(dead_code)]
fn _unused_arc(runtime: HostRuntime) -> Arc<HostRuntime> {
    Arc::new(runtime)
}

#[allow(dead_code)]
fn _unused_stream() -> impl tokio_stream::Stream<Item = Result<Event, Infallible>> {
    stream::empty()
}
