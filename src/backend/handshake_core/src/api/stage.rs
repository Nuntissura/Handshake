//! Privileged Stage capture/create and exact-byte embed-back API.
//!
//! A capture is one bounded, idempotent operation: strict DTO + operator/system
//! identity -> Job History row -> exact bytes + portable manifest -> EventLedger
//! receipt -> Flight Recorder projection. The content endpoint returns the exact
//! stored bytes; the native client verifies SHA-256 before embedding.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::models::ErrorResponse;
use crate::storage::{
    NewStageCaptureArtifact, StageArtifactStore, StageCaptureArtifact, StorageError,
};
use crate::AppState;
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use hmac::{Hmac, Mac};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<T, ApiError>;

pub const STAGE_CAPTURE_SCHEMA: &str = "hsk.stage.capture.create@1";
pub const STAGE_CAPTURE_MAX_BYTES: usize = 16 * 1024;
const STAGE_CAPTURE_MAX_JSON_BYTES: usize = 32 * 1024;
const STAGE_CAPTURE_MAX_CONCURRENT: usize = 4;
const STAGE_CAPTURE_MAX_PER_MINUTE: u32 = 30;
const STAGE_CAPTURE_CAPABILITY: &str = "stage.jobs.enqueue";
const PRE_AUTH_DENIAL_BUCKET_COUNT: u8 = 64;
const PRE_AUTH_DENIAL_AGGREGATE_AT: u32 = 8;
const PRE_AUTH_DENIAL_WINDOW: Duration = Duration::from_secs(60);

const HSK_HEADER_SESSION_TOKEN: &str = "x-hsk-session-token";

static CAPTURE_RATE: Lazy<Mutex<HashMap<String, (Instant, u32)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static CAPTURE_CONCURRENCY: Lazy<Mutex<HashMap<String, usize>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static PRE_AUTH_DENIAL_RATE: Lazy<Mutex<HashMap<u8, PreAuthDenialBucket>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static STAGE_FLIGHT_EVENT_LOCK: Lazy<tokio::sync::Mutex<()>> =
    Lazy::new(|| tokio::sync::Mutex::new(()));

#[derive(Clone, Copy)]
struct PreAuthDenialBucket {
    window: u64,
    observed: u32,
    aggregate_emitted: bool,
}

enum PreAuthDenialAdmission {
    Detail { bucket: u8, window: u64 },
    Aggregate { bucket: u8, window: u64, count: u32 },
    Suppress,
}

fn api_error(status: StatusCode, code: &'static str) -> ApiError {
    (status, Json(ErrorResponse { error: code }))
}

fn bad_request(code: &'static str) -> ApiError {
    api_error(StatusCode::BAD_REQUEST, code)
}

fn not_found(code: &'static str) -> ApiError {
    api_error(StatusCode::NOT_FOUND, code)
}

fn internal_error(err: impl std::fmt::Display) -> ApiError {
    tracing::error!(target: "handshake_core::stage_api", error = %err, "stage_api_error");
    api_error(StatusCode::INTERNAL_SERVER_ERROR, "HSK-500-STAGE")
}

fn map_storage_error(err: StorageError) -> ApiError {
    match err {
        StorageError::NotFound(code) => not_found(code),
        StorageError::Validation(_) => bad_request("HSK-400-STAGE"),
        StorageError::Conflict(_) => api_error(StatusCode::CONFLICT, "HSK-409-STAGE-IDEMPOTENCY"),
        other => internal_error(other),
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

async fn ensure_workspace_exists(state: &AppState, workspace_id: &str) -> ApiResult<()> {
    match state.storage.get_workspace(workspace_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(not_found("workspace_not_found")),
        Err(err) => Err(map_storage_error(err)),
    }
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/workspaces/:workspace_id/stage/artifacts",
            post(create_stage_artifact).layer(DefaultBodyLimit::max(STAGE_CAPTURE_MAX_JSON_BYTES)),
        )
        .route(
            "/workspaces/:workspace_id/stage/artifacts/:artifact_id",
            get(get_stage_artifact),
        )
        .route(
            "/workspaces/:workspace_id/stage/artifacts/:artifact_id/content",
            get(get_stage_artifact_content),
        )
        .with_state(state)
}

#[derive(Clone)]
pub(crate) struct CaptureContext {
    pub(crate) actor_kind: String,
    pub(crate) actor_id: String,
    pub(crate) limiter_principal: String,
    pub(crate) actor: KernelActor,
    pub(crate) kernel_task_run_id: String,
    pub(crate) session_run_id: String,
    pub(crate) binding_token: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ProcessBirthIdentity {
    Windows {
        creation_time_100ns: u64,
    },
    Linux {
        boot_id: String,
        start_time_ticks: u64,
    },
    MacOs {
        start_time_seconds: u64,
        start_time_microseconds: u64,
    },
}

#[derive(Deserialize)]
struct NativeMcpBinding {
    token: String,
    pid: u32,
    process_birth: ProcessBirthIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaptureContextFailure {
    InvalidSession,
    StaleBinding,
    CapabilityDenied,
}

impl CaptureContextFailure {
    fn records_denial(self) -> bool {
        match self {
            Self::InvalidSession | Self::StaleBinding | Self::CapabilityDenied => true,
        }
    }

    fn into_api_error(self) -> ApiError {
        match self {
            Self::InvalidSession | Self::StaleBinding => {
                api_error(StatusCode::UNAUTHORIZED, "HSK-401-STAGE-SESSION")
            }
            Self::CapabilityDenied => api_error(StatusCode::FORBIDDEN, "HSK-403-STAGE-CAPABILITY"),
        }
    }

    fn denial_reason(self) -> &'static str {
        match self {
            Self::InvalidSession => "invalid_session",
            Self::StaleBinding => "stale_binding",
            Self::CapabilityDenied => "capability_denied",
        }
    }
}

fn native_mcp_binding_path() -> PathBuf {
    if let Some(path) = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE") {
        return PathBuf::from(path);
    }
    #[cfg(target_os = "windows")]
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(path)
            .join("handshake")
            .join("swarm_mcp_binding.json");
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(path)
                .join("handshake")
                .join("swarm_mcp_binding.json");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("handshake")
                .join("swarm_mcp_binding.json");
        }
    }
    PathBuf::from("handshake").join("swarm_mcp_binding.json")
}

fn token_matches(stored: &str, presented: &str) -> bool {
    type HmacSha256 = Hmac<Sha256>;
    let Ok(mut expected) = HmacSha256::new_from_slice(stored.as_bytes()) else {
        return false;
    };
    expected.update(stored.as_bytes());
    let expected_tag = expected.finalize().into_bytes();
    let Ok(mut actual) = HmacSha256::new_from_slice(stored.as_bytes()) else {
        return false;
    };
    actual.update(presented.as_bytes());
    actual.verify_slice(&expected_tag).is_ok()
}

/// Shared guard for the process-global `HANDSHAKE_STAGE_BINDING_FILE` test binding.
///
/// Every test module that installs a native-MCP binding MUST hold this while it runs. The env var
/// is process-global, so two suites in the same test binary (for example `api::memory` and
/// `api::flight_recorder`) would otherwise authenticate against each other's token and fail
/// non-deterministically.
#[cfg(test)]
pub(crate) static NATIVE_BINDING_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn current_process_native_binding(token: &str) -> serde_json::Value {
    let pid = std::process::id();
    let process_birth = process_birth_identity(pid)
        .expect("the current test process has a verifiable birth identity");
    serde_json::json!({
        "token": token,
        "pid": pid,
        "process_birth": process_birth,
    })
}

#[cfg(windows)]
fn process_birth_identity(pid: u32) -> Option<ProcessBirthIdentity> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    const SYNCHRONIZE_RIGHT: u32 = 0x0010_0000;
    if pid == 0 {
        return None;
    }
    // SAFETY: OpenProcess receives a PID from the local binding file and a documented query/synchronize
    // access mask. The handle is checked before use and closed exactly once after the zero-time wait.
    let handle = unsafe {
        OpenProcess(
            SYNCHRONIZE_RIGHT | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        )
    };
    if handle.is_null() {
        return None;
    }
    let live = unsafe { WaitForSingleObject(handle, 0) } == WAIT_TIMEOUT;
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    let queried = live
        && unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) }
            != 0;
    unsafe {
        let _ = CloseHandle(handle);
    }
    queried.then(|| ProcessBirthIdentity::Windows {
        creation_time_100ns: (u64::from(creation.dwHighDateTime) << 32)
            | u64::from(creation.dwLowDateTime),
    })
}

#[cfg(target_os = "linux")]
fn process_birth_identity(pid: u32) -> Option<ProcessBirthIdentity> {
    if pid == 0 {
        return None;
    }
    let boot_id = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()?
        .trim()
        .to_owned();
    if boot_id.is_empty() {
        return None;
    }
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, tail) = stat.rsplit_once(") ")?;
    let fields: Vec<&str> = tail.split_whitespace().collect();
    let state = fields.first()?.as_bytes().first().copied()?;
    if matches!(state, b'Z' | b'X' | b'x') {
        return None;
    }
    let start_time_ticks = fields.get(19)?.parse().ok()?;
    Some(ProcessBirthIdentity::Linux {
        boot_id,
        start_time_ticks,
    })
}

#[cfg(target_os = "macos")]
fn process_birth_identity(pid: u32) -> Option<ProcessBirthIdentity> {
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Default)]
    struct ProcBsdInfo {
        pbi_flags: u32,
        pbi_status: u32,
        pbi_xstatus: u32,
        pbi_pid: u32,
        pbi_ppid: u32,
        pbi_uid: u32,
        pbi_gid: u32,
        pbi_ruid: u32,
        pbi_rgid: u32,
        pbi_svuid: u32,
        pbi_svgid: u32,
        pbi_reserved: u32,
        pbi_comm: [u8; 16],
        pbi_name: [u8; 32],
        pbi_nfiles: u32,
        pbi_pgid: u32,
        pbi_pjobc: u32,
        e_tdev: u32,
        e_tpgid: u32,
        pbi_nice: i32,
        pbi_start_tvsec: u64,
        pbi_start_tvusec: u64,
    }

    #[link(name = "proc")]
    extern "C" {
        fn proc_pidinfo(
            pid: i32,
            flavor: i32,
            arg: u64,
            buffer: *mut c_void,
            buffer_size: i32,
        ) -> i32;
    }

    const PROC_PIDTBSDINFO: i32 = 3;
    const SZOMB: u32 = 5;
    const PROC_FLAG_INEXIT: u32 = 4;
    if pid == 0 {
        return None;
    }
    let mut info = ProcBsdInfo::default();
    let expected_size = std::mem::size_of::<ProcBsdInfo>();
    let queried = unsafe {
        proc_pidinfo(
            i32::try_from(pid).ok()?,
            PROC_PIDTBSDINFO,
            0,
            std::ptr::from_mut(&mut info).cast::<c_void>(),
            i32::try_from(expected_size).ok()?,
        )
    };
    if queried != i32::try_from(expected_size).ok()?
        || info.pbi_pid != pid
        || info.pbi_status == SZOMB
        || info.pbi_flags & PROC_FLAG_INEXIT != 0
        || info.pbi_start_tvsec == 0
        || info.pbi_start_tvusec >= 1_000_000
    {
        return None;
    }
    Some(ProcessBirthIdentity::MacOs {
        start_time_seconds: info.pbi_start_tvsec,
        start_time_microseconds: info.pbi_start_tvusec,
    })
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn process_birth_identity(_pid: u32) -> Option<ProcessBirthIdentity> {
    None
}

#[cfg(all(
    unix,
    not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))
))]
fn process_birth_identity(_pid: u32) -> Option<ProcessBirthIdentity> {
    None
}

#[cfg(not(any(unix, windows)))]
fn process_birth_identity(_pid: u32) -> Option<ProcessBirthIdentity> {
    None
}

pub(crate) fn capture_context(
    headers: &HeaderMap,
) -> Result<CaptureContext, CaptureContextFailure> {
    let presented = header_str(headers, HSK_HEADER_SESSION_TOKEN)
        .filter(|value| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or(CaptureContextFailure::InvalidSession)?;
    let binding_bytes = std::fs::read(native_mcp_binding_path())
        .map_err(|_| CaptureContextFailure::InvalidSession)?;
    let binding: NativeMcpBinding = serde_json::from_slice(&binding_bytes)
        .map_err(|_| CaptureContextFailure::InvalidSession)?;
    if binding.token.len() != 64 || !token_matches(&binding.token, presented) {
        return Err(CaptureContextFailure::InvalidSession);
    }
    if process_birth_identity(binding.pid).as_ref() != Some(&binding.process_birth) {
        return Err(CaptureContextFailure::StaleBinding);
    }
    let birth_fingerprint = hex::encode(Sha256::digest(
        serde_json::to_vec(&binding.process_birth)
            .map_err(|_| CaptureContextFailure::InvalidSession)?,
    ));
    let actor_id = format!("handshake-native:{}:{birth_fingerprint}", binding.pid);
    let session_run_id = format!("native-mcp-session:{}:{birth_fingerprint}", binding.pid);
    let limiter_principal = hex::encode(Sha256::digest(binding.token.as_bytes()));
    Ok(CaptureContext {
        actor_kind: "operator".to_owned(),
        actor: KernelActor::Operator(actor_id.clone()),
        kernel_task_run_id: format!("native-stage-task:{}:{birth_fingerprint}", binding.pid),
        actor_id,
        limiter_principal,
        session_run_id,
        binding_token: binding.token,
    })
}

impl CaptureContext {
    fn approval_id(&self, workspace_id: &str, body: &CreateStageArtifactBody) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(self.binding_token.as_bytes())
            .expect("non-empty validated MCP token is a valid HMAC key");
        mac.update(workspace_id.as_bytes());
        mac.update(&[0]);
        mac.update(body.idempotency_key.as_bytes());
        mac.update(&[0]);
        mac.update(body.correlation_id.as_bytes());
        format!(
            "native-mcp-stage:{}",
            hex::encode(mac.finalize().into_bytes())
        )
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum StageContentKindWire {
    Document,
    Selection,
    CanvasNode,
    AtelierItem,
}

impl StageContentKindWire {
    fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Selection => "selection",
            Self::CanvasNode => "canvas_node",
            Self::AtelierItem => "atelier_item",
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CreateStageArtifactBody {
    schema_version: String,
    idempotency_key: String,
    correlation_id: String,
    content_kind: StageContentKindWire,
    #[serde(default)]
    label: String,
    content_type: String,
    content_base64: String,
    #[serde(default)]
    source_ref: Option<String>,
}

impl CreateStageArtifactBody {
    fn validate_and_decode(&self) -> ApiResult<Vec<u8>> {
        if self.schema_version != STAGE_CAPTURE_SCHEMA
            || self.idempotency_key.trim().is_empty()
            || self.idempotency_key.len() > 256
            || self.correlation_id.trim().is_empty()
            || self.correlation_id.len() > 256
            || self.label.len() > 256
            || self.content_type.trim().is_empty()
            || self.content_type.len() > 128
            || !self.content_type.is_ascii()
            || HeaderValue::from_str(&self.content_type).is_err()
            || self
                .source_ref
                .as_deref()
                .is_some_and(|value| value.trim().is_empty() || value.len() > 2048)
        {
            return Err(bad_request("HSK-400-STAGE-CAPTURE-SCHEMA"));
        }
        let bytes = BASE64
            .decode(self.content_base64.as_bytes())
            .map_err(|_| bad_request("HSK-400-STAGE-CAPTURE-BASE64"))?;
        if bytes.is_empty() || bytes.len() > STAGE_CAPTURE_MAX_BYTES {
            return Err(api_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "HSK-413-STAGE-CAPTURE",
            ));
        }
        Ok(bytes)
    }
}

#[derive(Debug, Serialize)]
struct StageManifestWire {
    sha256: String,
    manifest_ref: String,
    content_type: String,
    size_bytes: i64,
}

#[derive(Debug, Serialize)]
struct StageArtifactRefWire {
    artifact_id: String,
    workspace_id: String,
    sha256: String,
    manifest: StageManifestWire,
    label: String,
    content_path: String,
    size_bytes: i64,
    correlation_id: String,
    job_id: Option<String>,
    event_ledger_event_id: Option<String>,
    replayed: bool,
}

fn artifact_to_wire(artifact: StageCaptureArtifact, replayed: bool) -> StageArtifactRefWire {
    let content_path = format!(
        "/workspaces/{}/stage/artifacts/{}/content",
        encode_path_segment(&artifact.workspace_id),
        encode_path_segment(&artifact.artifact_id)
    );
    StageArtifactRefWire {
        artifact_id: artifact.artifact_id,
        workspace_id: artifact.workspace_id,
        sha256: artifact.content_sha256.clone(),
        manifest: StageManifestWire {
            sha256: artifact.content_sha256,
            manifest_ref: artifact.manifest_ref,
            content_type: artifact.content_type,
            size_bytes: artifact.size_bytes,
        },
        label: artifact.label,
        content_path,
        size_bytes: artifact.size_bytes,
        correlation_id: artifact.correlation_id,
        job_id: artifact.job_id,
        event_ledger_event_id: artifact.event_ledger_event_id,
        replayed,
    }
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(*byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

fn valid_stage_artifact_id(value: &str) -> bool {
    value.len() == 37
        && value.starts_with("STGA-")
        && value[5..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn authority_key(ctx: &CaptureContext, workspace_id: &str) -> String {
    format!("{}\u{0}{}", ctx.limiter_principal, workspace_id)
}

fn check_rate(authority_key: &str) -> bool {
    let Ok(mut rates) = CAPTURE_RATE.lock() else {
        return false;
    };
    let now = Instant::now();
    rates.retain(|_, (window_start, _)| now.duration_since(*window_start).as_secs() < 120);
    if rates.len() >= 4096 && !rates.contains_key(authority_key) {
        return false;
    }
    let entry = rates.entry(authority_key.to_owned()).or_insert((now, 0));
    if now.duration_since(entry.0).as_secs() >= 60 {
        *entry = (now, 0);
    }
    if entry.1 >= STAGE_CAPTURE_MAX_PER_MINUTE {
        return false;
    }
    entry.1 += 1;
    true
}

struct CaptureConcurrencyGuard(String);

impl CaptureConcurrencyGuard {
    fn acquire(key: String) -> Option<Self> {
        let mut counts = CAPTURE_CONCURRENCY.lock().ok()?;
        let count = counts.entry(key.clone()).or_default();
        if *count >= STAGE_CAPTURE_MAX_CONCURRENT {
            return None;
        }
        *count += 1;
        Some(Self(key))
    }
}

impl Drop for CaptureConcurrencyGuard {
    fn drop(&mut self) {
        if let Ok(mut counts) = CAPTURE_CONCURRENCY.lock() {
            if let Some(count) = counts.get_mut(&self.0) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    counts.remove(&self.0);
                }
            }
        }
    }
}

fn deterministic_uuid(scope: &str, value: &str) -> Uuid {
    let digest = Sha256::digest(format!("stage:{scope}:{value}").as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn pre_auth_denial_admission(fingerprint: &[u8]) -> PreAuthDenialAdmission {
    let bucket = fingerprint.first().copied().unwrap_or_default() % PRE_AUTH_DENIAL_BUCKET_COUNT;
    let window = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / PRE_AUTH_DENIAL_WINDOW.as_secs();
    let Ok(mut buckets) = PRE_AUTH_DENIAL_RATE.lock() else {
        return PreAuthDenialAdmission::Suppress;
    };
    let state = buckets.entry(bucket).or_insert(PreAuthDenialBucket {
        window,
        observed: 0,
        aggregate_emitted: false,
    });
    if state.window != window {
        *state = PreAuthDenialBucket {
            window,
            observed: 0,
            aggregate_emitted: false,
        };
    }
    state.observed = state.observed.saturating_add(1);
    if state.observed == 1 {
        return PreAuthDenialAdmission::Detail { bucket, window };
    }
    if state.observed >= PRE_AUTH_DENIAL_AGGREGATE_AT && !state.aggregate_emitted {
        state.aggregate_emitted = true;
        return PreAuthDenialAdmission::Aggregate {
            bucket,
            window,
            count: state.observed,
        };
    }
    PreAuthDenialAdmission::Suppress
}

fn generated_denial_correlation(
    ctx: &CaptureContext,
    workspace_id: &str,
    request_hint: &[u8],
    reason: &str,
) -> String {
    let digest = hex::encode(Sha256::digest(
        [
            ctx.actor_id.as_bytes(),
            b"\0",
            workspace_id.as_bytes(),
            b"\0",
            reason.as_bytes(),
            b"\0",
            &Sha256::digest(request_hint),
        ]
        .concat(),
    ));
    format!("stage-denial:{}", deterministic_uuid("authenticated-denial", &digest))
}

fn rate_denial_correlation(ctx: &CaptureContext, workspace_id: &str) -> String {
    let window = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 60;
    format!(
        "stage-rate-denial:{}",
        deterministic_uuid(
            "authenticated-rate-denial",
            &format!("{}:{workspace_id}:{window}", ctx.actor_id)
        )
    )
}

async fn record_flight_event_once(
    state: &AppState,
    mut event: FlightRecorderEvent,
    event_id: Uuid,
) -> ApiResult<()> {
    // FlightRecorder does not promise sink-level event-id uniqueness. Serialize the check+record
    // healing transaction so concurrent replays cannot both observe a missing projection and append it.
    let _write_guard = STAGE_FLIGHT_EVENT_LOCK.lock().await;
    event.event_id = event_id;
    let existing = state
        .flight_recorder
        .list_events(crate::flight_recorder::EventFilter {
            event_id: Some(event_id),
            ..Default::default()
        })
        .await
        .map_err(internal_error)?;
    if existing.is_empty() {
        if let Err(error) = state.flight_recorder.record_event(event).await {
            let healed = state
                .flight_recorder
                .list_events(crate::flight_recorder::EventFilter {
                    event_id: Some(event_id),
                    ..Default::default()
                })
                .await
                .map_err(internal_error)?;
            if healed.is_empty() {
                return Err(internal_error(error));
            }
        }
    }
    Ok(())
}

async fn record_stage_flight_event(
    state: &AppState,
    artifact: &StageCaptureArtifact,
    replayed: bool,
) -> ApiResult<()> {
    let actor = if artifact.actor_kind == "operator" {
        FlightRecorderActor::Human
    } else {
        FlightRecorderActor::System
    };
    let trace_id = artifact
        .job_id
        .as_deref()
        .and_then(|id| Uuid::parse_str(id).ok())
        .unwrap_or_else(Uuid::now_v7);
    let mut capability_event = FlightRecorderEvent::new(
        FlightRecorderEventType::CapabilityAction,
        actor.clone(),
        trace_id,
        json!({
            "capability_id": STAGE_CAPTURE_CAPABILITY,
            "actor_id": artifact.actor_id,
            "job_id": artifact.job_id,
            "decision_outcome": "allow",
        }),
    )
    .with_actor_id(artifact.actor_id.clone())
    .with_capability_id(STAGE_CAPTURE_CAPABILITY)
    .with_policy_decision_id(artifact.approval_id.clone())
    .with_wsids(vec![artifact.workspace_id.clone()]);
    if let Some(job_id) = &artifact.job_id {
        capability_event = capability_event.with_job_id(job_id.clone());
    }
    record_flight_event_once(
        state,
        capability_event,
        deterministic_uuid("allow-capability", &artifact.artifact_id),
    )
    .await?;

    let mut event = FlightRecorderEvent::new(
        FlightRecorderEventType::System,
        actor,
        trace_id,
        json!({
            "type": "stage.capture",
            "event_family": "stage",
            "action": "capture",
            "decision_outcome": "allow",
            "artifact_id": artifact.artifact_id,
            "artifact_ref": format!("artifact://sha256/{}", artifact.content_sha256),
            "manifest_ref": artifact.manifest_ref,
            "sha256": artifact.content_sha256,
            "size_bytes": artifact.size_bytes,
            "correlation_id": artifact.correlation_id,
            "approval_id": artifact.approval_id,
            "event_ledger_event_id": artifact.event_ledger_event_id,
            "replayed_request": replayed,
        }),
    )
    .with_actor_id(artifact.actor_id.clone())
    .with_capability_id(STAGE_CAPTURE_CAPABILITY)
    .with_policy_decision_id(artifact.approval_id.clone())
    .with_wsids(vec![artifact.workspace_id.clone()]);
    if let Some(job_id) = &artifact.job_id {
        event = event.with_job_id(job_id.clone());
    }
    record_flight_event_once(
        state,
        event,
        deterministic_uuid("allow-capture", &artifact.artifact_id),
    )
    .await
}

async fn record_stage_denial(
    state: &AppState,
    ctx: &CaptureContext,
    workspace_id: &str,
    correlation_id: &str,
    reason: &'static str,
) -> ApiResult<()> {
    let denial_key = format!("{}:{workspace_id}:{correlation_id}:{reason}", ctx.actor_id);
    let nonce = deterministic_uuid("deny", &denial_key);
    let correlation_id = if correlation_id.trim().is_empty() || correlation_id.len() > 256 {
        format!("stage-denial:{nonce}")
    } else {
        correlation_id.to_owned()
    };
    let approval_id = format!("stage-denial:{nonce}");
    let event = NewKernelEvent::builder(
        ctx.kernel_task_run_id.clone(),
        ctx.session_run_id.clone(),
        KernelEventType::ToolDecisionRecorded,
        ctx.actor.clone(),
    )
    .aggregate("stage_capture_authorization", workspace_id)
    .idempotency_key(format!("stage-capture-deny:{nonce}"))
    .correlation_id(correlation_id)
    .source_component("stage_capture_api")
    .payload(json!({
        "capability_id": STAGE_CAPTURE_CAPABILITY,
        "decision_outcome": "deny",
        "reason": reason,
        "actor_kind": ctx.actor_kind,
        "actor_id": ctx.actor_id,
        "approval_id": approval_id,
    }))
    .build()
    .map_err(internal_error)?;
    state
        .storage
        .append_kernel_event(event)
        .await
        .map_err(internal_error)?;

    let actor = if ctx.actor_kind == "operator" {
        FlightRecorderActor::Human
    } else {
        FlightRecorderActor::System
    };
    let fr_event = FlightRecorderEvent::new(
        FlightRecorderEventType::CapabilityAction,
        actor,
        nonce,
        json!({
            "capability_id": STAGE_CAPTURE_CAPABILITY,
            "actor_id": ctx.actor_id,
            "job_id": null,
            "decision_outcome": "deny",
        }),
    )
    .with_actor_id(ctx.actor_id.clone())
    .with_capability_id(STAGE_CAPTURE_CAPABILITY)
    .with_policy_decision_id(approval_id)
    .with_wsids(vec![workspace_id.to_owned()]);
    record_flight_event_once(state, fr_event, deterministic_uuid("deny-fr", &denial_key)).await
}

async fn record_pre_workspace_denial(
    state: &AppState,
    headers: &HeaderMap,
    workspace_hint: &str,
    request_hint: &[u8],
    reason: &'static str,
) {
    let presented = header_str(headers, HSK_HEADER_SESSION_TOKEN).unwrap_or("missing");
    let fingerprint = Sha256::digest(
        [
            format!("{reason}\u{0}{workspace_hint}\u{0}{presented}\u{0}").as_bytes(),
            &Sha256::digest(request_hint),
        ]
        .concat(),
    );
    // Admission happens before the global Flight Recorder check+append lock.
    // A hostile unauthenticated caller can therefore create at most two
    // redacted durable events per fixed bucket/window, while the in-memory
    // state itself is capped at PRE_AUTH_DENIAL_BUCKET_COUNT entries.
    let (bucket, window, aggregate_count, receipt_scope) =
        match pre_auth_denial_admission(&fingerprint) {
            PreAuthDenialAdmission::Detail { bucket, window } => {
                (bucket, window, None, "detail")
            }
            PreAuthDenialAdmission::Aggregate {
                bucket,
                window,
                count,
            } => (bucket, window, Some(count), "aggregate"),
            PreAuthDenialAdmission::Suppress => return,
        };
    let receipt_id = deterministic_uuid(
        "pre-workspace-deny",
        &format!("{bucket}:{window}:{receipt_scope}"),
    );
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::CapabilityAction,
        FlightRecorderActor::System,
        receipt_id,
        json!({
            "capability_id": STAGE_CAPTURE_CAPABILITY,
            "actor_id": "unauthenticated",
            "job_id": null,
            "decision_outcome": "deny",
            "denial_reason": reason,
            "denial_bucket": bucket,
            "window": window,
            "coalesced_count": aggregate_count,
        }),
    )
    .with_actor_id("unauthenticated")
    .with_capability_id(STAGE_CAPTURE_CAPABILITY)
    .with_policy_decision_id(format!("stage-denial:{receipt_id}"));
    if let Err(error) = record_flight_event_once(state, event, receipt_id).await {
        tracing::error!(
            target: "handshake_core::stage_api",
            status = %error.0,
            code = error.1.error,
            %receipt_id,
            "stage denial receipt persistence failed"
        );
    }
}

async fn create_stage_artifact(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
    raw: Bytes,
) -> ApiResult<(StatusCode, Json<StageArtifactRefWire>)> {
    let ctx = match capture_context(&headers) {
        Ok(ctx) => ctx,
        Err(failure) => {
            record_pre_workspace_denial(
                &state,
                &headers,
                &workspace_id,
                &raw,
                failure.denial_reason(),
            )
            .await;
            return Err(failure.into_api_error());
        }
    };
    let principal_workspace_key = authority_key(&ctx, &workspace_id);
    if !check_rate(&principal_workspace_key) {
        record_stage_denial(
            &state,
            &ctx,
            &workspace_id,
            &rate_denial_correlation(&ctx, &workspace_id),
            "actor_rate_limit",
        )
        .await?;
        return Err(api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "HSK-429-STAGE-CAPTURE",
        ));
    }
    let _permit = match CaptureConcurrencyGuard::acquire(principal_workspace_key) {
        Some(permit) => permit,
        None => {
            record_stage_denial(
                &state,
                &ctx,
                &workspace_id,
                &rate_denial_correlation(&ctx, &workspace_id),
                "principal_workspace_concurrency_limit",
            )
            .await?;
            return Err(api_error(
                StatusCode::TOO_MANY_REQUESTS,
                "HSK-429-STAGE-CONCURRENCY",
            ));
        }
    };
    if !crate::capabilities::CapabilityRegistry::new()
        .profile_can("Operator", STAGE_CAPTURE_CAPABILITY)
        .unwrap_or(false)
    {
        record_stage_denial(
            &state,
            &ctx,
            &workspace_id,
            &generated_denial_correlation(&ctx, &workspace_id, &raw, "capability_denied"),
            "capability_denied",
        )
        .await?;
        return Err(CaptureContextFailure::CapabilityDenied.into_api_error());
    }
    if headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        != Some("application/json")
    {
        record_stage_denial(
            &state,
            &ctx,
            &workspace_id,
            &generated_denial_correlation(&ctx, &workspace_id, &raw, "invalid_dto"),
            "invalid_dto",
        )
        .await?;
        return Err(bad_request("HSK-400-STAGE-CAPTURE-CONTENT-TYPE"));
    }
    let body: CreateStageArtifactBody = match serde_json::from_slice(&raw) {
        Ok(body) => body,
        Err(_) => {
            record_stage_denial(
                &state,
                &ctx,
                &workspace_id,
                &generated_denial_correlation(&ctx, &workspace_id, &raw, "invalid_dto"),
                "invalid_dto",
            )
            .await?;
            return Err(bad_request("HSK-400-STAGE-CAPTURE-SCHEMA"));
        }
    };
    let content_bytes = match body.validate_and_decode() {
        Ok(bytes) => bytes,
        Err(error) => {
            record_stage_denial(
                &state,
                &ctx,
                &workspace_id,
                &generated_denial_correlation(&ctx, &workspace_id, &raw, "invalid_dto"),
                "invalid_dto",
            )
            .await?;
            return Err(error);
        }
    };
    ensure_workspace_exists(&state, &workspace_id).await?;
    let approval_id = ctx.approval_id(&workspace_id, &body);

    let request_hash = crate::kernel::context_bundle::sha256_hex(
        &crate::kernel::context_bundle::canonical_json_bytes(&json!({
            "workspace_id": workspace_id,
            "schema_version": body.schema_version,
            "correlation_id": body.correlation_id,
            "content_kind": body.content_kind.as_str(),
            "label": body.label,
            "content_type": body.content_type,
            "content_sha256": hex::encode(Sha256::digest(&content_bytes)),
            "source_ref": body.source_ref,
        })),
    );
    let receipt = NewKernelEvent::builder(
        ctx.kernel_task_run_id.clone(),
        ctx.session_run_id.clone(),
        KernelEventType::ArtifactStored,
        ctx.actor.clone(),
    )
    .aggregate("stage_capture_artifact", "pending")
    .idempotency_key(format!(
        "stage-capture:{}:{}",
        workspace_id, body.idempotency_key
    ))
    .correlation_id(body.correlation_id.clone())
    .source_component("stage_capture_api")
    .payload(json!({"pending": true}))
    .build()
    .map_err(internal_error)?;
    let decision_receipt = NewKernelEvent::builder(
        ctx.kernel_task_run_id.clone(),
        ctx.session_run_id.clone(),
        KernelEventType::ToolDecisionRecorded,
        ctx.actor.clone(),
    )
    .aggregate("stage_capture_authorization", "pending")
    .idempotency_key(format!(
        "stage-capture-decision:{}:{}",
        workspace_id, body.idempotency_key
    ))
    .correlation_id(body.correlation_id.clone())
    .source_component("stage_capture_api")
    .payload(json!({"pending": true}))
    .build()
    .map_err(internal_error)?;
    let content_json = json!({
        "encoding": "base64",
        "content_base64": body.content_base64,
        "size_bytes": content_bytes.len(),
    });
    let inserted = StageArtifactStore::new(state.postgres_pool.clone())
        .insert_stage_artifact(NewStageCaptureArtifact {
            workspace_id: workspace_id.clone(),
            content_kind: body.content_kind.as_str().to_owned(),
            label: body.label,
            content_type: body.content_type,
            content_json,
            content_bytes,
            source_ref: body.source_ref,
            idempotency_key: body.idempotency_key,
            request_hash,
            actor_kind: ctx.actor_kind.clone(),
            actor_id: ctx.actor_id.clone(),
            correlation_id: body.correlation_id,
            approval_id,
            decision_receipt,
            receipt,
        })
        .await
        .map_err(map_storage_error)?;
    if let Err(error) =
        record_stage_flight_event(&state, &inserted.artifact, inserted.replayed).await
    {
        tracing::error!(
            target: "handshake_core::stage_api",
            status = %error.0,
            code = error.1.error,
            artifact_id = %inserted.artifact.artifact_id,
            "Stage capture committed; Flight Recorder projection remains retry-healable"
        );
        return Err(error);
    }
    let status = if inserted.replayed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((
        status,
        Json(artifact_to_wire(inserted.artifact, inserted.replayed)),
    ))
}

async fn get_stage_artifact(
    State(state): State<AppState>,
    Path((workspace_id, artifact_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<StageArtifactRefWire>> {
    let ctx = match capture_context(&headers) {
        Ok(ctx) => ctx,
        Err(failure) => {
            if failure.records_denial() {
                record_pre_workspace_denial(
                    &state,
                    &headers,
                    &workspace_id,
                    artifact_id.as_bytes(),
                    "invalid_session_read",
                )
                .await;
            }
            return Err(failure.into_api_error());
        }
    };
    if !crate::capabilities::CapabilityRegistry::new()
        .profile_can("Operator", STAGE_CAPTURE_CAPABILITY)
        .unwrap_or(false)
    {
        record_stage_denial(
            &state,
            &ctx,
            &workspace_id,
            &generated_denial_correlation(
                &ctx,
                &workspace_id,
                artifact_id.as_bytes(),
                "capability_denied_read",
            ),
            "capability_denied",
        )
        .await?;
        return Err(CaptureContextFailure::CapabilityDenied.into_api_error());
    }
    if !valid_stage_artifact_id(&artifact_id) {
        return Err(bad_request("HSK-400-STAGE-ARTIFACT-ID"));
    }
    ensure_workspace_exists(&state, &workspace_id).await?;
    let artifact = StageArtifactStore::new(state.postgres_pool.clone())
        .get_stage_artifact(&workspace_id, &artifact_id)
        .await
        .map_err(map_storage_error)?
        .ok_or_else(|| not_found("stage_artifact_not_found"))?;
    Ok(Json(artifact_to_wire(artifact, false)))
}

async fn get_stage_artifact_content(
    State(state): State<AppState>,
    Path((workspace_id, artifact_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let ctx = match capture_context(&headers) {
        Ok(ctx) => ctx,
        Err(failure) => {
            if failure.records_denial() {
                record_pre_workspace_denial(
                    &state,
                    &headers,
                    &workspace_id,
                    artifact_id.as_bytes(),
                    "invalid_session_read",
                )
                .await;
            }
            return Err(failure.into_api_error());
        }
    };
    if !crate::capabilities::CapabilityRegistry::new()
        .profile_can("Operator", STAGE_CAPTURE_CAPABILITY)
        .unwrap_or(false)
    {
        record_stage_denial(
            &state,
            &ctx,
            &workspace_id,
            &generated_denial_correlation(
                &ctx,
                &workspace_id,
                artifact_id.as_bytes(),
                "capability_denied_read",
            ),
            "capability_denied",
        )
        .await?;
        return Err(CaptureContextFailure::CapabilityDenied.into_api_error());
    }
    if !valid_stage_artifact_id(&artifact_id) {
        return Err(bad_request("HSK-400-STAGE-ARTIFACT-ID"));
    }
    ensure_workspace_exists(&state, &workspace_id).await?;
    let artifact = StageArtifactStore::new(state.postgres_pool.clone())
        .get_stage_artifact(&workspace_id, &artifact_id)
        .await
        .map_err(map_storage_error)?
        .ok_or_else(|| not_found("stage_artifact_not_found"))?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&artifact.content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        header::ETAG,
        HeaderValue::from_str(&format!("\"sha256:{}\"", artifact.content_sha256))
            .map_err(internal_error)?,
    );
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&artifact.size_bytes.to_string()).map_err(internal_error)?,
    );
    Ok((headers, artifact.content_bytes).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_id_requires_exact_canonical_shape() {
        assert!(valid_stage_artifact_id(
            "STGA-0123456789abcdef0123456789abcdef"
        ));
        for invalid in [
            "STGA-0123456789abcdef0123456789abcde",
            "STGA-0123456789abcdef0123456789abcdef0",
            "stga-0123456789abcdef0123456789abcdef",
            "STGA-0123456789ABCDEF0123456789ABCDEF",
            "STGA-0123456789abcdef0123456789abcdeg",
            "ART-0123456789abcdef0123456789abcdef",
        ] {
            assert!(!valid_stage_artifact_id(invalid), "accepted {invalid}");
        }
    }

    #[test]
    fn rate_limit_accepts_thirty_and_rejects_thirty_first() {
        let key = format!("stage-rate-boundary-{}", Uuid::now_v7());
        for attempt in 1..=STAGE_CAPTURE_MAX_PER_MINUTE {
            assert!(check_rate(&key), "attempt {attempt} must pass");
        }
        assert!(!check_rate(&key), "attempt 31 must fail");
    }

    #[test]
    fn concurrency_limit_accepts_four_rejects_fifth_and_drop_releases() {
        let key = format!("stage-concurrency-boundary-{}", Uuid::now_v7());
        let mut guards = Vec::new();
        for attempt in 1..=STAGE_CAPTURE_MAX_CONCURRENT {
            guards.push(
                CaptureConcurrencyGuard::acquire(key.clone())
                    .unwrap_or_else(|| panic!("guard {attempt} must pass")),
            );
        }
        assert!(CaptureConcurrencyGuard::acquire(key.clone()).is_none());
        guards.pop();
        assert!(CaptureConcurrencyGuard::acquire(key).is_some());
    }

    #[test]
    fn pre_auth_denial_limiter_is_fixed_size_and_coalesces_repeats() {
        PRE_AUTH_DENIAL_RATE.lock().unwrap().clear();
        let fingerprint = [7_u8; 32];
        assert!(matches!(
            pre_auth_denial_admission(&fingerprint),
            PreAuthDenialAdmission::Detail { bucket: 7, .. }
        ));
        for _ in 2..PRE_AUTH_DENIAL_AGGREGATE_AT {
            assert!(matches!(
                pre_auth_denial_admission(&fingerprint),
                PreAuthDenialAdmission::Suppress
            ));
        }
        assert!(matches!(
            pre_auth_denial_admission(&fingerprint),
            PreAuthDenialAdmission::Aggregate {
                bucket: 7,
                count: PRE_AUTH_DENIAL_AGGREGATE_AT,
                ..
            }
        ));
        assert!(matches!(
            pre_auth_denial_admission(&fingerprint),
            PreAuthDenialAdmission::Suppress
        ));

        for first_byte in u8::MIN..=u8::MAX {
            let mut distinct = [0_u8; 32];
            distinct[0] = first_byte;
            let _ = pre_auth_denial_admission(&distinct);
        }
        assert!(
            PRE_AUTH_DENIAL_RATE.lock().unwrap().len()
                <= usize::from(PRE_AUTH_DENIAL_BUCKET_COUNT)
        );
    }

    #[test]
    fn limiter_authority_is_stable_when_actor_pid_changes() {
        fn context(actor_id: &str) -> CaptureContext {
            CaptureContext {
                actor_kind: "operator".to_owned(),
                actor_id: actor_id.to_owned(),
                limiter_principal: "verified-binding-token-digest".to_owned(),
                actor: KernelActor::Operator(actor_id.to_owned()),
                kernel_task_run_id: "test-task".to_owned(),
                session_run_id: "test-session".to_owned(),
                binding_token: "test-token".to_owned(),
            }
        }

        assert_eq!(
            authority_key(&context("handshake-native:100"), "WS-1"),
            authority_key(&context("handshake-native:200"), "WS-1")
        );
        assert_ne!(
            authority_key(&context("handshake-native:100"), "WS-1"),
            authority_key(&context("handshake-native:100"), "WS-2")
        );
    }
}
