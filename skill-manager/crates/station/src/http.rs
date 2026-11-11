//! 管理站 HTTP：登录、设备、转发与静态前端。未登录不加载技能数据。

use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post, put};
use axum::{Json, Router};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::{broadcast, mpsc};
use tower_http::services::{ServeDir, ServeFile};

use crate::auth;
use crate::db::StationDb;
use crate::devices::{self, LiveLink, LiveMap, PairBody};
use crate::proxy;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<StationDb>>,
    pub live: LiveMap,
    pub web_dir: PathBuf,
}

fn with_db<T>(
    state: &AppState,
    f: impl FnOnce(&StationDb) -> Result<T, String>,
) -> Result<T, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "管理站数据库锁已损坏".to_string())?;
    f(&db)
}

fn session_id(jar: &CookieJar, headers: &HeaderMap) -> Option<String> {
    jar.get(auth::cookie_name())
        .map(|c| c.value().to_string())
        .or_else(|| {
            headers
                .get("x-station-session")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        })
}

fn require_session(
    state: &AppState,
    jar: &CookieJar,
    headers: &HeaderMap,
) -> Result<String, (StatusCode, Json<Value>)> {
    let Some(id) = session_id(jar, headers) else {
        return Err((StatusCode::UNAUTHORIZED, Json(json!({ "error": "未登录" }))));
    };
    let ok = with_db(state, |db| auth::session_valid(db, &id)).unwrap_or(false);
    if ok {
        Ok(id)
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "登录已过期" })),
        ))
    }
}

fn require_full_session(
    state: &AppState,
    jar: &CookieJar,
    headers: &HeaderMap,
) -> Result<String, (StatusCode, Json<Value>)> {
    let id = require_session(state, jar, headers)?;
    let pending = with_db(state, |db| auth::session_pending_rebind(db, &id)).unwrap_or(false);
    if pending {
        Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "请先完成一次性密码绑定" })),
        ))
    } else {
        Ok(id)
    }
}

fn err_status(message: String) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message })))
}

pub fn router(state: AppState) -> Router {
    let mut router = Router::new()
        .route("/auth/setup", post(auth_setup))
        .route("/auth/totp/setup", get(auth_totp_setup))
        .route("/auth/totp/verify", post(auth_totp_verify))
        .route("/auth/login", post(auth_login))
        .route("/auth/logout", post(auth_logout))
        .route("/auth/session", get(auth_session))
        .route("/auth/session-ttl", get(auth_ttl_get).put(auth_ttl))
        .route("/auth/totp/rebind", post(auth_totp_rebind))
        .route("/devices/pairing-codes", post(pairing_codes))
        .route("/devices", get(list_devices))
        .route("/devices/{id}", patch(rename_device))
        .route("/devices/{id}/revoke", post(revoke_device))
        .route("/pair", post(pair_host))
        .route("/host/ws/{id}", get(host_ws))
        .route("/current", put(set_current))
        .route("/current/invoke/{command}", post(current_invoke))
        .route("/current/events", get(current_events))
        .route("/current/session", post(current_session))
        .route("/current/plans/{id}/confirm", post(current_confirm))
        .route("/current/agent/context", put(current_agent_context))
        .route("/current/agent/messages", post(current_agent_messages))
        .route(
            "/current/agent/tasks/{id}/cancel",
            post(current_agent_cancel),
        )
        .route("/current/host/status", get(current_host_status));

    if state.web_dir.is_dir() {
        let index = state.web_dir.join("index.html");
        router = router.fallback_service(
            ServeDir::new(&state.web_dir).not_found_service(ServeFile::new(index)),
        );
    } else {
        router = router.fallback(frontend_missing);
    }

    router.with_state(state)
}

async fn frontend_missing() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(
            json!({ "error": "未找到前端静态文件。请先构建 skill-manager 前端，或设置 SKILL_MANAGER_WEB。" }),
        ),
    )
}

#[derive(Deserialize)]
struct SetupBody {
    username: String,
    password: String,
}

async fn auth_setup(
    State(state): State<AppState>,
    Json(body): Json<SetupBody>,
) -> impl IntoResponse {
    match with_db(&state, |db| {
        auth::create_admin(db, &body.username, &body.password)
    }) {
        Ok(shown) => Json(json!({
            "username": shown.username,
            "totpSecret": shown.totp_secret,
            "otpauth": shown.otpauth,
            "recoveryCodes": shown.recovery_codes,
            "qr": shown.qr_unicode,
        }))
        .into_response(),
        Err(err) => err_status(err).into_response(),
    }
}

async fn auth_totp_setup(State(state): State<AppState>) -> impl IntoResponse {
    match with_db(&state, auth::load_admin) {
        Ok(Some(admin)) if admin.totp_secret.is_some() => Json(json!({
            "bound": true,
            "username": admin.username,
        }))
        .into_response(),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "请先 POST /auth/setup" })),
        )
            .into_response(),
        Err(err) => err_status(err).into_response(),
    }
}

#[derive(Deserialize)]
struct OtpBody {
    otp: Option<String>,
    code: Option<String>,
}

async fn auth_totp_verify(
    State(state): State<AppState>,
    Json(body): Json<OtpBody>,
) -> impl IntoResponse {
    let otp = body.otp.or(body.code).unwrap_or_default();
    match with_db(&state, |db| auth::verify_totp(db, &otp)) {
        Ok(value) => Json(value).into_response(),
        Err(err) => err_status(err).into_response(),
    }
}

#[derive(Deserialize)]
struct LoginBody {
    username: String,
    password: String,
    otp: Option<String>,
    #[serde(rename = "recoveryCode")]
    recovery_code: Option<String>,
}

async fn auth_login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginBody>,
) -> impl IntoResponse {
    let otp = body.otp.or(body.recovery_code).unwrap_or_default();
    match with_db(&state, |db| {
        auth::login(db, &body.username, &body.password, &otp)
    }) {
        Ok(result) => {
            let cookie = Cookie::build((auth::cookie_name(), result.session_id.clone()))
                .path("/")
                .http_only(true)
                .same_site(SameSite::Lax)
                .build();
            let mut body = json!({
                "ok": true,
                "sessionId": result.session_id,
                "mustRebindTotp": result.must_rebind_totp,
            });
            if let Some(rebind) = result.rebind {
                body["totpSecret"] = json!(rebind.totp_secret);
                body["otpauth"] = json!(rebind.otpauth);
                body["qr"] = json!(rebind.qr_unicode);
                body["recoveryCodes"] = json!(rebind.recovery_codes);
            }
            (jar.add(cookie), Json(body)).into_response()
        }
        Err(err) => err_status(err).into_response(),
    }
}

async fn auth_logout(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(id) = session_id(&jar, &headers) {
        let _ = with_db(&state, |db| auth::delete_session(db, &id));
    }
    let cookie = Cookie::build((auth::cookie_name(), ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();
    (jar.remove(cookie), Json(json!({ "ok": true }))).into_response()
}

async fn auth_session(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> impl IntoResponse {
    match require_session(&state, &jar, &headers) {
        Ok(id) => {
            let device = with_db(&state, |db| auth::session_device(db, &id))
                .ok()
                .flatten();
            Json(json!({ "ok": true, "sessionId": id, "deviceId": device })).into_response()
        }
        Err(err) => err.into_response(),
    }
}

#[derive(Deserialize)]
struct TtlBody {
    seconds: i64,
}

async fn auth_ttl_get(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(err) = require_full_session(&state, &jar, &headers) {
        return err.into_response();
    }
    match with_db(&state, auth::ttl) {
        Ok(seconds) => Json(json!({ "ok": true, "seconds": seconds })).into_response(),
        Err(err) => err_status(err).into_response(),
    }
}

async fn auth_ttl(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<TtlBody>,
) -> impl IntoResponse {
    if let Err(err) = require_full_session(&state, &jar, &headers) {
        return err.into_response();
    }
    match with_db(&state, |db| auth::set_ttl(db, body.seconds)) {
        Ok(seconds) => Json(json!({ "ok": true, "seconds": seconds })).into_response(),
        Err(err) => err_status(err).into_response(),
    }
}

async fn auth_totp_rebind(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<OtpBody>,
) -> impl IntoResponse {
    if let Err(err) = require_session(&state, &jar, &headers) {
        return err.into_response();
    }
    let otp = body.otp.or(body.code).unwrap_or_default();
    match with_db(&state, |db| auth::confirm_totp_rebind(db, &otp)) {
        Ok(value) => Json(value).into_response(),
        Err(err) => err_status(err).into_response(),
    }
}

async fn pairing_codes(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(err) = require_full_session(&state, &jar, &headers) {
        return err.into_response();
    }
    match with_db(&state, devices::issue_pairing_code) {
        Ok(value) => Json(value).into_response(),
        Err(err) => err_status(err).into_response(),
    }
}

async fn list_devices(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(err) = require_full_session(&state, &jar, &headers) {
        return err.into_response();
    }
    match with_db(&state, |db| devices::list_devices(db, &state.live)) {
        Ok(list) => Json(json!({ "devices": list })).into_response(),
        Err(err) => err_status(err).into_response(),
    }
}

#[derive(Deserialize)]
struct RenameBody {
    #[serde(rename = "displayName")]
    display_name: String,
}

async fn rename_device(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<RenameBody>,
) -> impl IntoResponse {
    if let Err(err) = require_full_session(&state, &jar, &headers) {
        return err.into_response();
    }
    match with_db(&state, |db| devices::rename(db, &id, &body.display_name)) {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(err) => err_status(err).into_response(),
    }
}

async fn revoke_device(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(err) = require_full_session(&state, &jar, &headers) {
        return err.into_response();
    }
    match with_db(&state, |db| devices::revoke(db, &state.live, &id)) {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(err) => err_status(err).into_response(),
    }
}

async fn pair_host(State(state): State<AppState>, Json(body): Json<PairBody>) -> impl IntoResponse {
    match with_db(&state, |db| devices::consume_pair(db, body)) {
        Ok(value) => Json(value).into_response(),
        Err(err) => err_status(err).into_response(),
    }
}

async fn host_ws(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let key = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("")
        .to_string();
    let ok = with_db(&state, |db| devices::verify_device_key(db, &id, &key)).unwrap_or(false);
    if !ok {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "设备密钥无效或已解除" })),
        )
            .into_response();
    }
    ws.on_upgrade(move |socket| handle_host_socket(state, id, socket))
        .into_response()
}

async fn handle_host_socket(state: AppState, device_id: String, socket: WebSocket) {
    let (mut sink, mut stream) = StreamExt::split(socket);
    let (invoke_tx, mut invoke_rx) = mpsc::unbounded_channel();
    let (events, _) = broadcast::channel(64);
    state.live.insert(
        device_id.clone(),
        LiveLink {
            last_seen: std::time::Instant::now(),
            invoke_tx,
            events: events.clone(),
        },
    );
    let _ = with_db(&state, |db| {
        devices::touch_seen(db, &device_id);
        Ok(())
    });

    let mut pending: std::collections::HashMap<
        String,
        tokio::sync::oneshot::Sender<crate::devices::InvokeReply>,
    > = std::collections::HashMap::new();

    loop {
        tokio::select! {
            incoming = stream.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        state.live.touch(&device_id);
                        if let Ok(value) = serde_json::from_str::<Value>(&text) {
                            match value.get("type").and_then(Value::as_str) {
                                Some("invoke_result") => {
                                    if let Some(id) = value.get("id").and_then(Value::as_str) {
                                        if let Some(tx) = pending.remove(id) {
                                            let _ = tx.send(proxy::reply_from_host_message(&value));
                                        }
                                    }
                                }
                                Some("sse") => {
                                    let _ = events.send(value);
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {
                        state.live.touch(&device_id);
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
            req = invoke_rx.recv() => {
                let Some(req) = req else { break; };
                let payload = json!({
                    "type": "invoke",
                    "id": req.id,
                    "command": req.command,
                    "body": req.body,
                    "sessionId": req.session_id,
                });
                pending.insert(req.id.clone(), req.reply);
                if sink.send(Message::Text(payload.to_string().into())).await.is_err() {
                    break;
                }
            }
        }
    }
    state.live.remove(&device_id);
}

#[derive(Deserialize)]
struct CurrentBody {
    #[serde(rename = "deviceId")]
    device_id: String,
}

async fn set_current(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<CurrentBody>,
) -> impl IntoResponse {
    let session = match require_full_session(&state, &jar, &headers) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };
    if !state.live.is_online(&body.device_id) {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": format!("设备 {} 离线，不能切换为当前设备来操作", body.device_id) })),
        )
            .into_response();
    }
    match with_db(&state, |db| {
        auth::set_session_device(db, &session, Some(&body.device_id))
    }) {
        Ok(()) => Json(json!({ "ok": true, "deviceId": body.device_id })).into_response(),
        Err(err) => err_status(err).into_response(),
    }
}

fn current_device(state: &AppState, session: &str) -> Result<String, (StatusCode, Json<Value>)> {
    let device = with_db(state, |db| auth::session_device(db, session)).map_err(err_status)?;
    match device {
        Some(id) if state.live.is_online(&id) => Ok(id),
        Some(id) => Err((StatusCode::CONFLICT, Json(proxy::offline_error(Some(&id))))),
        None => Err((StatusCode::CONFLICT, Json(proxy::offline_error(None)))),
    }
}

async fn current_invoke(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(command): Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let session = match require_full_session(&state, &jar, &headers) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };
    let device = match current_device(&state, &session) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };
    let host_session = body
        .get("sessionId")
        .or_else(|| body.get("session_id"))
        .and_then(Value::as_str)
        .map(str::to_string);
    match proxy::invoke(&state.live, &device, &command, body, host_session).await {
        Ok(reply) => invoke_response(reply),
        Err(err) => (StatusCode::CONFLICT, Json(json!({ "error": err }))).into_response(),
    }
}

async fn current_events(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match require_full_session(&state, &jar, &headers) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };
    let device = match current_device(&state, &session) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };
    let rx = match proxy::subscribe(&state.live, &device) {
        Ok(rx) => rx,
        Err(err) => return (StatusCode::CONFLICT, Json(json!({ "error": err }))).into_response(),
    };
    let stream = tokio_stream::StreamExt::filter_map(
        tokio_stream::wrappers::BroadcastStream::new(rx),
        |item| match item {
            Ok(value) => {
                let name = value
                    .get("event")
                    .and_then(Value::as_str)
                    .unwrap_or("message")
                    .to_string();
                let data = value.get("data").cloned().unwrap_or(value);
                Some(Ok::<_, Infallible>(
                    Event::default().event(name).data(data.to_string()),
                ))
            }
            Err(_) => None,
        },
    );
    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

async fn current_session(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match require_full_session(&state, &jar, &headers) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };
    let device = match current_device(&state, &session) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };
    match proxy::invoke(&state.live, &device, "__session__", json!({}), None).await {
        Ok(reply) => invoke_response(reply),
        Err(_) => {
            // Host 没有 __session__ 命令时，由本站生成并让后续 invoke 带上。
            Json(json!({ "sessionId": uuid::Uuid::new_v4().to_string() })).into_response()
        }
    }
}

#[derive(Deserialize)]
struct ConfirmBody {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

async fn current_confirm(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ConfirmBody>,
) -> impl IntoResponse {
    let session = match require_full_session(&state, &jar, &headers) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };
    let device = match current_device(&state, &session) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };
    // 通过 Host 的 plans confirm：借用 invoke 通道发一条内部命令，失败则说明离线。
    let payload = json!({ "planId": id, "sessionId": body.session_id });
    match proxy::invoke(
        &state.live,
        &device,
        "plans_confirm",
        payload,
        body.session_id,
    )
    .await
    {
        Ok(reply) => invoke_response(reply),
        Err(err) => (StatusCode::CONFLICT, Json(json!({ "error": err }))).into_response(),
    }
}

async fn current_agent_context(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    forward_named(&state, &jar, &headers, "agent_set_context", body).await
}

async fn current_agent_messages(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    forward_named(&state, &jar, &headers, "agent_post_message", body).await
}

async fn current_agent_cancel(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let mut body = body;
    if let Some(obj) = body.as_object_mut() {
        obj.insert("taskId".into(), json!(id));
    }
    forward_named(&state, &jar, &headers, "agent_cancel_task", body).await
}

async fn current_host_status(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> impl IntoResponse {
    forward_named(&state, &jar, &headers, "host_status", json!({})).await
}

async fn forward_named(
    state: &AppState,
    jar: &CookieJar,
    headers: &HeaderMap,
    command: &str,
    body: Value,
) -> Response {
    let session = match require_full_session(state, jar, headers) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };
    let device = match current_device(state, &session) {
        Ok(id) => id,
        Err(err) => return err.into_response(),
    };
    match proxy::invoke(&state.live, &device, command, body, None).await {
        Ok(reply) => invoke_response(reply),
        Err(err) => (StatusCode::CONFLICT, Json(json!({ "error": err }))).into_response(),
    }
}

fn invoke_response(reply: crate::devices::InvokeReply) -> Response {
    let status = StatusCode::from_u16(reply.status).unwrap_or(StatusCode::BAD_REQUEST);
    (status, Json(reply.body)).into_response()
}

/// 给 Host WS 增加 plans_confirm / agent_* / host_status 的本地翻译。
/// 实际执行仍在 Host：这些名字会落到 Host invoke Unknown。
/// 因此 Host 侧应接受它们，或管理站应走 Host HTTP。出站长连接已覆盖 invoke。
#[allow(dead_code)]
fn _note() {}
