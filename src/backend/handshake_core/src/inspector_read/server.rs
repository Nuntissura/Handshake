#![cfg(feature = "inspector")]

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::{
    extract::{
        ws::{Message, WebSocketUpgrade},
        ConnectInfo, Path, Query, Request, State,
    },
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

use super::replay_drive::{
    PerRunSecret, ReplayDriveError, ReplayDriveResponse, ReplayDriveService,
    PER_RUN_SECRET_HEADER,
};
use super::trace_projection::TraceProjection;
use super::trait_def::{
    EventLedgerRow, InspectorReadV1, ModelLoadedRow, ProcessRow, SessionId, SessionStateRead,
    SessionSummary, WorkspaceId, WorkspaceStateRead,
};

pub const KERNEL_INSPECTOR_PORT_COMMAND_REF: &str = "kernel.inspector.port";
const DEFAULT_TAIL_LIMIT: usize = 100;

#[derive(Debug, Error)]
pub enum InspectorServerError {
    #[error("inspector server must bind to 127.0.0.1 only, got {0}")]
    NonLoopbackBind(SocketAddr),
    #[error("inspector server bind failed: {0}")]
    Bind(std::io::Error),
    #[error("inspector server local_addr failed: {0}")]
    LocalAddr(std::io::Error),
}

pub struct InspectorServer;

pub struct InspectorServerHandle {
    addr: SocketAddr,
    secret: Arc<PerRunSecret>,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl std::fmt::Debug for InspectorServerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InspectorServerHandle")
            .field("addr", &self.addr)
            .field("port_command_ref", &KERNEL_INSPECTOR_PORT_COMMAND_REF)
            .finish_non_exhaustive()
    }
}

impl InspectorServerHandle {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    pub fn port_command_ref(&self) -> &'static str {
        KERNEL_INSPECTOR_PORT_COMMAND_REF
    }

    /// The per-launch shared secret operators and tests must present in the
    /// `X-Handshake-Inspector-Secret` header and use as the HMAC key when
    /// signing replay-drive envelopes. Rotates on every `start` /
    /// `bind_reader` call.
    pub fn per_run_secret(&self) -> &PerRunSecret {
        &self.secret
    }
}

impl Drop for InspectorServerHandle {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        self.task.abort();
    }
}

pub fn kernel_inspector_port(handle: &InspectorServerHandle) -> u16 {
    handle.port()
}

impl InspectorServer {
    pub async fn start(
        reader: Arc<dyn InspectorReadV1>,
    ) -> Result<InspectorServerHandle, InspectorServerError> {
        Self::bind_reader_with_secret(
            SocketAddr::from(([127, 0, 0, 1], 0)),
            reader,
            Arc::new(PerRunSecret::generate()),
        )
        .await
    }

    pub async fn bind_reader(
        addr: SocketAddr,
        reader: Arc<dyn InspectorReadV1>,
    ) -> Result<InspectorServerHandle, InspectorServerError> {
        Self::bind_reader_with_secret(addr, reader, Arc::new(PerRunSecret::generate())).await
    }

    /// Bind with a caller-supplied per-run secret. Tests use this to inject
    /// a deterministic secret; production callers should normally use
    /// [`start`] / [`bind_reader`], which generate a fresh UUIDv7 per launch.
    pub async fn bind_reader_with_secret(
        addr: SocketAddr,
        reader: Arc<dyn InspectorReadV1>,
        secret: Arc<PerRunSecret>,
    ) -> Result<InspectorServerHandle, InspectorServerError> {
        if addr.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) {
            return Err(InspectorServerError::NonLoopbackBind(addr));
        }

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(InspectorServerError::Bind)?;
        let local_addr = listener
            .local_addr()
            .map_err(InspectorServerError::LocalAddr)?;
        let app = inspector_router(reader, secret.clone());
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        // Operator-visible launch announcement: the secret is intended to
        // be read by the operator from this log line and supplied to any
        // tool that calls the inspector plane.
        tracing::info!(
            target: "handshake_core::inspector_read",
            addr = %local_addr,
            per_run_secret = %secret.to_hex(),
            "inspector server bound; clients must present X-Handshake-Inspector-Secret"
        );
        let task = tokio::spawn(async move {
            // `into_make_service_with_connect_info` makes the peer
            // `SocketAddr` available to handlers/middleware via
            // `ConnectInfo<SocketAddr>` so every per-run-secret reject can
            // be audit-logged with the rejected peer address (MT-029
            // §6.5.5 reject log contract: timestamp, route, peer_addr,
            // reason).
            let _ = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await;
        });

        Ok(InspectorServerHandle {
            addr: local_addr,
            secret,
            shutdown: Some(shutdown_tx),
            task,
        })
    }
}

#[derive(Clone)]
struct InspectorState {
    reader: Arc<dyn InspectorReadV1>,
    replay_drive: Arc<ReplayDriveService>,
}

fn inspector_router(reader: Arc<dyn InspectorReadV1>, secret: Arc<PerRunSecret>) -> Router {
    let state = InspectorState {
        reader,
        replay_drive: Arc::new(ReplayDriveService::with_per_run_secret(secret.clone())),
    };
    Router::new()
        .route("/inspector/v1/sessions", get(list_sessions))
        .route("/inspector/v1/sessions/:id", get(session_state))
        .route("/inspector/v1/event-ledger/tail", get(event_ledger_tail))
        .route(
            "/inspector/v1/process-ledger/active",
            get(process_ledger_active),
        )
        .route("/inspector/v1/workspace/:ws_id", get(workspace_state))
        .route("/inspector/v1/trace/:session_id", get(trace_projection))
        .route("/inspector/v1/models", get(loaded_models))
        .route("/inspector/v1/event-stream", get(event_stream))
        .route("/inspector/v1/replay-drive", post(replay_drive))
        // MT-029 spec §6.5.5: the per-run shared-secret header is required
        // on EVERY inspector request — reads, the event-stream upgrade, and
        // the replay-drive write. Enforcing it as a single router-wide
        // middleware layer (rather than per-route) means a newly added
        // route is authenticated by default and cannot silently ship
        // unauthenticated. Missing/wrong header -> 401, audit-logged with
        // (timestamp via tracing, route, peer_addr, reason).
        .layer(middleware::from_fn_with_state(
            secret.clone(),
            require_per_run_secret,
        ))
        .with_state(state)
}

/// Router-wide authentication middleware: rejects any request whose
/// `X-Handshake-Inspector-Secret` header is missing or does not
/// constant-time-match the per-run secret. Every reject is audit-logged
/// with the request route, the peer socket address, and a machine-readable
/// reason. `tracing` stamps each event with a timestamp, satisfying the
/// §6.5.5 (timestamp, route, peer_addr, reason) reject-log contract.
async fn require_per_run_secret(
    State(secret): State<Arc<PerRunSecret>>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let route = request.uri().path().to_string();
    let headers = request.headers();
    match classify_per_run_secret(headers, &secret) {
        SecretCheck::Ok => next.run(request).await,
        SecretCheck::Missing => {
            audit_reject(&route, peer_addr, "missing_per_run_secret_header");
            InspectorHttpError::unauthorized("missing_or_invalid_per_run_secret").into_response()
        }
        SecretCheck::Mismatch => {
            audit_reject(&route, peer_addr, "per_run_secret_header_mismatch");
            InspectorHttpError::unauthorized("missing_or_invalid_per_run_secret").into_response()
        }
    }
}

/// Emit a structured 401 reject audit record. `tracing` attaches the
/// timestamp; we attach the route, peer_addr, and reason. Centralised so
/// the read paths, the event-stream upgrade, and the replay-drive write all
/// log identically.
fn audit_reject(route: &str, peer_addr: SocketAddr, reason: &'static str) {
    tracing::warn!(
        target: "handshake_core::inspector_read",
        route = %route,
        peer_addr = %peer_addr,
        reason = reason,
        "rejected inspector request: per-run shared-secret check failed"
    );
}

enum SecretCheck {
    Ok,
    Missing,
    Mismatch,
}

fn classify_per_run_secret(headers: &HeaderMap, expected: &PerRunSecret) -> SecretCheck {
    let Some(raw) = headers.get(PER_RUN_SECRET_HEADER) else {
        return SecretCheck::Missing;
    };
    if per_run_secret_value_matches(raw, expected) {
        SecretCheck::Ok
    } else {
        SecretCheck::Mismatch
    }
}

async fn list_sessions(State(state): State<InspectorState>) -> Json<Vec<SessionSummary>> {
    Json(state.reader.list_sessions())
}

async fn session_state(
    Path(id): Path<String>,
    State(state): State<InspectorState>,
) -> Result<Json<SessionStateRead>, InspectorHttpError> {
    state
        .reader
        .session_state(SessionId::new(id))
        .map(Json)
        .ok_or(InspectorHttpError::not_found("session_not_found"))
}

#[derive(Debug, Deserialize)]
struct TailQuery {
    n: Option<usize>,
}

async fn event_ledger_tail(
    Query(query): Query<TailQuery>,
    State(state): State<InspectorState>,
) -> Json<Vec<EventLedgerRow>> {
    Json(
        state
            .reader
            .event_ledger_tail(query.n.unwrap_or(DEFAULT_TAIL_LIMIT)),
    )
}

async fn process_ledger_active(State(state): State<InspectorState>) -> Json<Vec<ProcessRow>> {
    Json(state.reader.process_ledger_active())
}

async fn workspace_state(
    Path(ws_id): Path<String>,
    State(state): State<InspectorState>,
) -> Result<Json<WorkspaceStateRead>, InspectorHttpError> {
    state
        .reader
        .workspace_state_read(WorkspaceId::new(ws_id))
        .map(Json)
        .ok_or(InspectorHttpError::not_found("workspace_not_found"))
}

async fn trace_projection(
    Path(session_id): Path<String>,
    State(state): State<InspectorState>,
) -> Result<Json<TraceProjection>, InspectorHttpError> {
    state
        .reader
        .trace_projection(SessionId::new(session_id))
        .map(Json)
        .ok_or(InspectorHttpError::not_found("trace_not_found"))
}

async fn loaded_models(State(state): State<InspectorState>) -> Json<Vec<ModelLoadedRow>> {
    Json(state.reader.loaded_models())
}

async fn event_stream(
    ws: WebSocketUpgrade,
    State(state): State<InspectorState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |mut socket| async move {
        let payload = json!({
            "schema_id": "hsk.inspector.event_stream@1",
            "events": state.reader.event_ledger_tail(DEFAULT_TAIL_LIMIT),
        });
        let _ = socket.send(Message::Text(payload.to_string())).await;
    })
}

async fn replay_drive(
    State(state): State<InspectorState>,
    body: String,
) -> Result<Json<ReplayDriveResponse>, InspectorHttpError> {
    // MT-029 spec §6.5.5: the per-run shared-secret header is enforced for
    // this route (and every other inspector route) by the router-wide
    // `require_per_run_secret` middleware, so by the time this handler runs
    // the header is already validated. The HMAC envelope signature check
    // below is the second, independent factor for the write path.
    state
        .replay_drive
        .handle_body(&body)
        .map(Json)
        .map_err(InspectorHttpError::from_replay_drive)
}

/// Constant-time comparison of a raw header value against the expected
/// per-run secret hex. Reused by the router-wide middleware. Length is
/// compared first (non-secret) then bytes are compared in constant time so
/// a matching-length wrong secret does not leak via timing.
fn per_run_secret_value_matches(
    raw: &axum::http::HeaderValue,
    expected: &PerRunSecret,
) -> bool {
    use subtle::ConstantTimeEq;
    let Ok(value) = raw.to_str() else {
        return false;
    };
    let expected_hex = expected.to_hex();
    if value.len() != expected_hex.len() {
        return false;
    }
    value.as_bytes().ct_eq(expected_hex.as_bytes()).into()
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
}

#[derive(Debug)]
struct InspectorHttpError {
    status: StatusCode,
    code: &'static str,
}

impl InspectorHttpError {
    fn bad_request(code: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
        }
    }

    fn forbidden(code: &'static str) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code,
        }
    }

    fn not_found(code: &'static str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code,
        }
    }

    fn unauthorized(code: &'static str) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code,
        }
    }

    fn internal(code: &'static str) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code,
        }
    }

    fn from_replay_drive(error: ReplayDriveError) -> Self {
        match error {
            ReplayDriveError::MalformedJson => Self::bad_request("malformed_json"),
            ReplayDriveError::ForbiddenShape | ReplayDriveError::InvalidSignature => {
                Self::forbidden("replay_drive_forbidden")
            }
            ReplayDriveError::UnknownAction { .. } => Self::not_found("catalog_action_not_found"),
            ReplayDriveError::Dispatch { .. } => Self::internal("replay_drive_dispatch_failed"),
            ReplayDriveError::EventLedger { .. } => Self::internal("event_ledger_append_failed"),
        }
    }
}

impl IntoResponse for InspectorHttpError {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(ErrorBody { code: self.code })).into_response()
    }
}
