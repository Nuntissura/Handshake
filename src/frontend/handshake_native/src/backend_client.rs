//! Backend HTTP client. Reuses the EXISTING handshake_core backend over its real HTTP API
//! (GET /health, GET/PUT /workspaces/:id/workbench/layout) — the native app never starts or embeds
//! the backend; it assumes it is running. Deserializes via serde_json::Value to avoid a build
//! dependency on the handshake_core crate.

use crate::error::AppError;
use crate::layout_persistence::{LayoutError, LayoutTransport};
use serde_json::Value;
use std::collections::VecDeque;
use std::fmt;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::Duration;

/// handshake_core listens here (hardcoded in handshake_core/src/main.rs).
pub const BACKEND_BASE_URL: &str = "http://127.0.0.1:37501";

/// The process-wide shared backend [`reqwest::Client`]. `reqwest::Client` owns a connection pool and is
/// cheaply cloneable (an `Arc` internally), so the whole native app should share ONE pool rather than
/// minting an independent pool/TLS stack per sub-client. New `/knowledge/documents/*` transport (the
/// MT-037 consolidated client + the MT-029 find/replace `RichDocClient`, which now delegates to it)
/// resolves its client from here so there is exactly ONE document-transport pool. Lazily initialized on
/// first use; construction is proof-gated so an unbounded fallback client can never enter the app.
static SHARED_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// WP-KERNEL-012 MT-088 (D2 internal_diagnostics — backend-down graceful degradation, Master Spec
/// v02.196 §5.8.5 "Backend-down graceful degradation (HARD)"). Connect timeout for every backend
/// reqwest client: a dead backend (nothing listening on `127.0.0.1:37501`) refuses the TCP connection
/// fast, but a HALF-OPEN backend (a host that accepts the SYN but never completes the handshake, e.g. a
/// firewalled/black-holed port) would otherwise hang the connect attempt for the OS default (tens of
/// seconds). A short connect timeout bounds that worst case so even a TCP-accepting-but-silent backend
/// cannot hang a worker indefinitely (AC-008-5 / defense in depth — the off-thread move in `app.rs`
/// already prevents a UI-thread stall; this prevents a leaked worker on a half-open socket).
pub const BACKEND_CONNECT_TIMEOUT: Duration = Duration::from_millis(1500);

/// WP-KERNEL-012 MT-088: the overall per-request timeout floor applied to the shared backend client so a
/// request that connects but never sends a response body cannot hang a worker forever. Individual call
/// sites may set a tighter per-request `.timeout(..)` (e.g. the 5s layout/health probes); this is the
/// outer bound on the shared pool.
pub const BACKEND_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// WP-KERNEL-012 MT-101: model-session `/jobs` creation is a runtime launch path, not a health or
/// layout probe. The payload carries `timeout_ms=120000`, so the native request must stay alive long
/// enough for the backend to enqueue and return real workflow state instead of cancelling early and
/// risking an orphaned queued job.
pub const MODEL_SESSION_JOBS_REQUEST_TIMEOUT: Duration = Duration::from_secs(130);

/// WP-KERNEL-012 MT-088: build a backend [`reqwest::Client`] carrying the backend-down timeouts
/// ([`BACKEND_CONNECT_TIMEOUT`] + [`BACKEND_REQUEST_TIMEOUT`]). Construction failure is fatal rather
/// than silently installing an unbounded client: every backend transport must retain these hard bounds.
/// Used by the shared pool, the `/health` probe, and the layout transport.
pub fn build_backend_client() -> reqwest::Client {
    build_backend_client_with_request_timeout(BACKEND_REQUEST_TIMEOUT)
}

fn build_backend_client_with_request_timeout(request_timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(BACKEND_CONNECT_TIMEOUT)
        .timeout(request_timeout)
        .build()
        .expect("bounded backend reqwest client construction must succeed")
}

fn build_model_session_backend_client() -> reqwest::Client {
    build_backend_client_with_request_timeout(MODEL_SESSION_JOBS_REQUEST_TIMEOUT)
}

/// Return a clone of the process-wide shared backend [`reqwest::Client`] (one connection pool for the
/// whole app). Cloning is cheap (the pool is shared behind an `Arc`); callers that need to own a client
/// should clone this rather than calling `reqwest::Client::new()` so they share the single pool and its
/// backend-down timeouts ([`build_backend_client`]).
pub fn shared_http_client() -> reqwest::Client {
    SHARED_HTTP_CLIENT.get_or_init(build_backend_client).clone()
}

/// WP-KERNEL-012 MT-100: planned native HTTP terminal-session route. The real PTY runtime exists today
/// in `handshake_core::terminal` and is exposed through legacy Tauri IPC
/// `kernel_terminal_create_session`, but the native Rust frontend is a separate process and has no HTTP
/// `/terminal/*` surface to call.
pub const TERMINAL_LAUNCH_PROBED_PATH: &str = "/terminal/sessions";

/// The existing terminal session spawn channel. This is evidence for the typed blocker: terminal launch
/// is built behind Tauri IPC, not behind a native-reachable HTTP route.
pub const TERMINAL_LAUNCH_IPC_CHANNEL: &str = "kernel_terminal_create_session";

/// Source owner of the currently reachable terminal runtime bridge.
pub const TERMINAL_LAUNCH_IPC_OWNER: &str = "app/src-tauri/src/commands/terminal.rs";

/// WP-KERNEL-012 MT-101: reachable native job-creation path for a model-session launch.
pub const MODEL_SESSION_JOBS_PATH: &str = "/jobs";

/// The protocol id used by existing handshake_core tests and default workflow creation. The native
/// frontend sends this through the real `POST /jobs` surface and keeps the repo-folder binding inside
/// `job_inputs`; the backend NOW enforces the native workspace model-launch contract
/// (`validate_native_workspace_model_launch_contract` in handshake_core `workflows.rs`): when
/// `launch_surface`/`launch_mode` are present it requires explicit `session_id`, `workspace_folder`,
/// `working_dir == workspace_folder`, `wrapper`, `model_provider == backend`, and `model_id` — so a
/// mis-scoped or folder-less launch is rejected server-side rather than silently defaulted.
pub const MODEL_SESSION_PROTOCOL_ID: &str = "protocol-default";

/// Native-declared direct-spawn route that does not exist today. The direct repo-folder session spawn
/// with wrapper is only exposed through Tauri IPC, so probing this path always returns EndpointMissing.
pub const MODEL_SESSION_DIRECT_SPAWN_PROBED_PATH: &str = "/swarm/sessions";

/// The existing direct session spawn channel in the legacy Tauri command layer.
pub const MODEL_SESSION_LAUNCH_IPC_CHANNEL: &str = "kernel_swarm_spawn_session";

/// Source owner of the reachable IPC command for direct repo-folder model session spawn.
pub const MODEL_SESSION_LAUNCH_IPC_OWNER: &str = "app/src-tauri/src/commands/swarm_runtime.rs";

/// The local GGUF/runtime load channel that exists only in the legacy Tauri command layer. A native
/// local-provider model-session launch must name this exact missing bridge until a native HTTP route
/// exists or a live operator-surface proof shows the model is loaded/running through another real path.
pub const MODEL_SESSION_LOCAL_MODEL_LOAD_IPC_CHANNEL: &str = "kernel_model_runtime_load";

/// Source owner of the reachable IPC command for local model runtime loading.
pub const MODEL_SESSION_LOCAL_MODEL_LOAD_IPC_OWNER: &str =
    "app/src-tauri/src/commands/model_runtime.rs";

#[cfg(target_os = "windows")]
const DEFAULT_TERMINAL_SHELL: &str = "pwsh.exe";
#[cfg(not(target_os = "windows"))]
const DEFAULT_TERMINAL_SHELL: &str = "sh";

/// The resolved request the native terminal-launch affordance would send if a native HTTP terminal
/// route existed. Keeping this typed prevents the UI from fabricating a terminal session while still
/// proving that the cwd and shell wrapper are carried explicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalLaunchRequest {
    pub cwd: String,
    pub shell: String,
    pub args: Vec<String>,
    pub rows: u16,
    pub cols: u16,
}

impl TerminalLaunchRequest {
    pub fn workspace_default(cwd: impl Into<String>) -> Self {
        let (shell, args) = platform_terminal_wrapper();
        Self {
            cwd: cwd.into(),
            shell,
            args,
            rows: 24,
            cols: 80,
        }
    }
}

/// Platform shell wrapper for a workspace terminal launch. Windows prefers `pwsh.exe`; the backend PTY
/// path still owns deeper fallback (`powershell.exe` -> `cmd.exe`) once a native route exists.
pub fn platform_terminal_wrapper() -> (String, Vec<String>) {
    (DEFAULT_TERMINAL_SHELL.to_owned(), Vec::new())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalLaunchClient {
    base_url: String,
}

impl TerminalLaunchClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    pub fn production() -> Self {
        Self::new(BACKEND_BASE_URL)
    }

    pub fn probed_url(&self) -> String {
        format!("{}{}", self.base_url, TERMINAL_LAUNCH_PROBED_PATH)
    }

    pub fn request_for_workspace(&self, cwd: impl Into<String>) -> TerminalLaunchRequest {
        TerminalLaunchRequest::workspace_default(cwd)
    }

    /// Attempt to open a terminal session in `cwd`. Today this returns the honest typed blocker because
    /// the PTY session runtime is reachable only through Tauri IPC, not through native HTTP.
    #[allow(clippy::result_large_err)]
    pub fn open_workspace_terminal(
        &self,
        cwd: impl Into<String>,
    ) -> Result<TerminalLaunchSession, TerminalLaunchError> {
        let request = self.request_for_workspace(cwd);
        Err(TerminalLaunchError::EndpointMissing {
            probed_path: TERMINAL_LAUNCH_PROBED_PATH.to_owned(),
            probed_url: self.probed_url(),
            ipc_channel: TERMINAL_LAUNCH_IPC_CHANNEL,
            ipc_owner: TERMINAL_LAUNCH_IPC_OWNER,
            request,
        })
    }
}

/// Placeholder for the future real native terminal-session response. It deliberately has no fake
/// constructor or fallback id; the current production path returns [`TerminalLaunchError`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalLaunchSession {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalLaunchError {
    EndpointMissing {
        probed_path: String,
        probed_url: String,
        ipc_channel: &'static str,
        ipc_owner: &'static str,
        request: TerminalLaunchRequest,
    },
}

impl TerminalLaunchError {
    pub fn is_endpoint_missing(&self) -> bool {
        matches!(self, Self::EndpointMissing { .. })
    }

    pub fn request(&self) -> &TerminalLaunchRequest {
        match self {
            Self::EndpointMissing { request, .. } => request,
        }
    }
}

impl fmt::Display for TerminalLaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EndpointMissing {
                probed_path,
                ipc_channel,
                ipc_owner,
                ..
            } => write!(
                f,
                "EndpointMissing: native terminal launch needs HTTP {probed_path}; current PTY runtime terminal/** is IPC-only via {ipc_channel} in {ipc_owner}"
            ),
        }
    }
}

/// Which backend/provider lane the native model-session launch should request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSessionProvider {
    Local,
    Cloud,
}

impl ModelSessionProvider {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Cloud => "cloud",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Local => "Local",
            Self::Cloud => "Cloud",
        }
    }
}

impl fmt::Display for ModelSessionProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Operator-selected launch inputs. These are explicit because handshake_core's model_run defaults would
/// otherwise mask missing UI state with `default-model` / `default-backend`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSessionLaunchRequest {
    pub provider: ModelSessionProvider,
    pub session_id: String,
    pub workspace_id: String,
    pub workspace_folder: String,
    pub model_id: String,
    pub wrapper: String,
    /// MT-101 REMEDIATION: OPTIONAL governed-work attribution. `None` for an operator launch (the
    /// default — an operator model session is NOT work-packet work and must not be misattributed);
    /// a governed/agent launch surface that really is executing a WP sets it explicitly.
    pub wp_id: Option<String>,
    /// MT-101 REMEDIATION: optional microtask attribution, same contract as `wp_id`.
    pub mt_id: Option<String>,
}

impl ModelSessionLaunchRequest {
    pub fn new(
        provider: ModelSessionProvider,
        workspace_id: impl Into<String>,
        workspace_folder: impl Into<String>,
        model_id: impl Into<String>,
        wrapper: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            session_id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            workspace_folder: workspace_folder.into(),
            model_id: model_id.into(),
            wrapper: wrapper.into(),
            wp_id: None,
            mt_id: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = session_id.into();
        self
    }

    /// MT-101 REMEDIATION: attach governed-work attribution to this launch (a governed/agent surface
    /// executing a real WP/MT). Operator launches never call this — the fields stay absent.
    pub fn with_governed_work(
        mut self,
        wp_id: impl Into<String>,
        mt_id: impl Into<String>,
    ) -> Self {
        self.wp_id = Some(wp_id.into());
        self.mt_id = Some(mt_id.into());
        self
    }

    #[allow(clippy::result_large_err)]
    fn validate(&self) -> Result<(), ModelSessionLaunchError> {
        if self.session_id.trim().is_empty() {
            return Err(ModelSessionLaunchError::InvalidRequest {
                field: "session_id",
                reason: "session id is required".to_owned(),
            });
        }
        if self.workspace_folder.trim().is_empty() {
            return Err(ModelSessionLaunchError::InvalidRequest {
                field: "workspace_folder",
                reason: "workspace folder is required".to_owned(),
            });
        }
        if self.model_id.trim().is_empty() {
            return Err(ModelSessionLaunchError::InvalidRequest {
                field: "model_id",
                reason: "model id or cloud model name is required".to_owned(),
            });
        }
        if self.wrapper.trim().is_empty() {
            return Err(ModelSessionLaunchError::InvalidRequest {
                field: "wrapper",
                reason: "wrapper is required".to_owned(),
            });
        }
        Ok(())
    }

    fn trimmed(&self) -> Self {
        Self {
            provider: self.provider,
            session_id: self.session_id.trim().to_owned(),
            workspace_id: self.workspace_id.trim().to_owned(),
            workspace_folder: self.workspace_folder.trim().to_owned(),
            model_id: self.model_id.trim().to_owned(),
            wrapper: self.wrapper.trim().to_owned(),
            wp_id: self
                .wp_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            mt_id: self
                .mt_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
        }
    }
}

/// The direct-spawn request that would be passed to `kernel_swarm_spawn_session` if a native HTTP bridge
/// existed. It is preserved inside [`ModelSessionLaunchError::EndpointMissing`] for state recovery and
/// no-context model debugging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSessionDirectSpawnRequest {
    pub provider: ModelSessionProvider,
    pub session_id: String,
    pub workspace_id: String,
    pub worktree_id: Option<String>,
    pub working_dir: String,
    pub model_id: String,
    pub wrapper: String,
    pub artifact_path: Option<String>,
    pub sha256_expected: Option<String>,
    pub runtime_binding: Option<String>,
    pub local_model_id: Option<String>,
    pub cloud_model_name: Option<String>,
}

impl From<&ModelSessionLaunchRequest> for ModelSessionDirectSpawnRequest {
    fn from(request: &ModelSessionLaunchRequest) -> Self {
        let request = request.trimmed();
        Self {
            provider: request.provider,
            session_id: request.session_id,
            workspace_id: request.workspace_id,
            worktree_id: None,
            working_dir: request.workspace_folder,
            model_id: request.model_id.clone(),
            wrapper: request.wrapper,
            artifact_path: None,
            sha256_expected: None,
            runtime_binding: None,
            local_model_id: (request.provider == ModelSessionProvider::Local)
                .then_some(request.model_id.clone()),
            cloud_model_name: (request.provider == ModelSessionProvider::Cloud)
                .then_some(request.model_id),
        }
    }
}

/// Parsed result from the real `POST /jobs` path. This deliberately does not expose "model running":
/// creating a backend workflow job is not the same proof as a direct repo-folder-bound live session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSessionJobResult {
    pub job_id: String,
    pub workflow_run_id: Option<String>,
    pub status: Option<String>,
    pub raw: serde_json::Value,
}

impl ModelSessionJobResult {
    fn from_json(raw: serde_json::Value) -> Result<Self, String> {
        let Some(job_id) = raw
            .get("job_id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
        else {
            return Err(
                "invalid /jobs response: missing required job_id; did not claim model session created"
                    .to_owned(),
            );
        };
        let workflow_run_id = raw
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_owned);
        let status = raw
            .get("status")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        Ok(Self {
            job_id,
            workflow_run_id,
            status,
            raw,
        })
    }
}

/// Delivery cell for the off-thread `POST /jobs` model-session request.
pub type ModelSessionLaunchCell = Arc<Mutex<Option<Result<ModelSessionJobResult, String>>>>;

/// Native client for MT-101 model-session launch. It owns the real reachable `POST /jobs` request and
/// separately exposes the typed blocker for direct repo-folder spawn, which is IPC-only today.
#[derive(Clone)]
pub struct ModelSessionLaunchClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl ModelSessionLaunchClient {
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: build_model_session_backend_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    pub fn jobs_url(&self) -> String {
        format!("{}{}", self.base_url, MODEL_SESSION_JOBS_PATH)
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn direct_spawn_probed_url(base_url: &str) -> String {
        format!("{base_url}{MODEL_SESSION_DIRECT_SPAWN_PROBED_PATH}")
    }

    #[allow(clippy::result_large_err)]
    pub fn jobs_request(
        &self,
        request: &ModelSessionLaunchRequest,
    ) -> Result<RequestSpec, ModelSessionLaunchError> {
        model_session_jobs_request(&self.base_url, request)
    }

    /// Enqueue the real reachable `POST /jobs` request off the UI thread. The returned [`RequestSpec`] is
    /// the exact request being sent, so the caller can report "POST /jobs issued" without duplicating
    /// serialization logic.
    #[allow(clippy::result_large_err)]
    pub fn launch_workspace_model_job(
        &self,
        request: ModelSessionLaunchRequest,
        cell: ModelSessionLaunchCell,
    ) -> Result<RequestSpec, ModelSessionLaunchError> {
        let spec = self.jobs_request(&request)?;
        let body = spec.body.clone().unwrap_or_else(|| serde_json::json!({}));
        let client = self.client.clone();
        let url = spec.url.clone();
        self.runtime.spawn(async move {
            let result = match post_json_expect_value(
                &client,
                &url,
                &body,
                MODEL_SESSION_JOBS_REQUEST_TIMEOUT,
            )
            .await
            {
                Ok(raw) => ModelSessionJobResult::from_json(raw),
                Err(e) => Err(e.to_string()),
            };
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result);
            }
        });
        Ok(spec)
    }

    /// The direct repo-folder-bound spawn remains IPC-only. This path intentionally never returns a fake
    /// session id.
    #[allow(clippy::result_large_err)]
    pub fn direct_spawn_workspace(
        base_url: &str,
        request: &ModelSessionLaunchRequest,
    ) -> Result<ModelSessionDirectSpawn, ModelSessionLaunchError> {
        request.validate()?;
        Err(ModelSessionLaunchError::EndpointMissing {
            probed_path: MODEL_SESSION_DIRECT_SPAWN_PROBED_PATH.to_owned(),
            probed_url: Self::direct_spawn_probed_url(base_url),
            ipc_channel: MODEL_SESSION_LAUNCH_IPC_CHANNEL,
            ipc_owner: MODEL_SESSION_LAUNCH_IPC_OWNER,
            request: ModelSessionDirectSpawnRequest::from(request),
        })
    }
}

/// Placeholder for a future native direct-spawn response. There is deliberately no fake constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSessionDirectSpawn {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
// EndpointMissing deliberately retains the full typed launch request for operator recovery.
#[allow(clippy::large_enum_variant)]
pub enum ModelSessionLaunchError {
    InvalidRequest {
        field: &'static str,
        reason: String,
    },
    EndpointMissing {
        probed_path: String,
        probed_url: String,
        ipc_channel: &'static str,
        ipc_owner: &'static str,
        request: ModelSessionDirectSpawnRequest,
    },
}

impl ModelSessionLaunchError {
    pub fn is_endpoint_missing(&self) -> bool {
        matches!(self, Self::EndpointMissing { .. })
    }
}

impl fmt::Display for ModelSessionLaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest { field, reason } => {
                write!(f, "InvalidRequest: {field}: {reason}")
            }
            Self::EndpointMissing {
                ipc_channel,
                ipc_owner,
                request,
                ..
            } => {
                write!(
                    f,
                    "EndpointMissing: direct repo-folder model session spawn with wrapper is IPC-only via {ipc_channel} in {ipc_owner}"
                )?;
                if request.provider == ModelSessionProvider::Local {
                    write!(
                        f,
                        "; LocalModelLoadEndpointMissing: local model load is IPC-only via {} in {}; no native HTTP local-model load route is exposed to the Rust frontend",
                        MODEL_SESSION_LOCAL_MODEL_LOAD_IPC_CHANNEL,
                        MODEL_SESSION_LOCAL_MODEL_LOAD_IPC_OWNER
                    )?;
                }
                Ok(())
            }
        }
    }
}

#[allow(clippy::result_large_err)]
fn model_session_jobs_request(
    base_url: &str,
    request: &ModelSessionLaunchRequest,
) -> Result<RequestSpec, ModelSessionLaunchError> {
    request.validate()?;
    let request = request.trimmed();
    let backend = request.provider.as_str();
    // MT-101 REMEDIATION: an operator launch carries NO governed-work attribution (previously every
    // launch was hardcoded wp_id=WP-KERNEL-012/mt_id=MT-101 — misattributing operator sessions as WP
    // work), NO canned prompt (this is a promptless session bootstrap; operator messages must arrive
    // through a real follow-up message path, never a baked-in string), and NO `simulate_duration_ms` knob
    // (a simulation control has no place in a production launch). The backend treats all three as
    // optional (`string_opt`/`u64_opt`).
    let mut job_inputs = serde_json::json!({
        "launch_surface": "handshake_native",
        "launch_mode": "workspace_model_session",
        "session_id": request.session_id,
        "workspace_id": request.workspace_id,
        "workspace_folder": request.workspace_folder,
        "working_dir": request.workspace_folder,
        "model_provider": backend,
        "model_id": request.model_id,
        "backend": backend,
        "wrapper": request.wrapper,
        "role": "assistant",
        "lane": "PRIMARY",
        "priority": 50,
        "retry_backoff": "exponential",
        "timeout_ms": 120000,
        "max_tokens_budget": 4096,
        "max_retries": 3,
        "parameter_class": "default",
        "execution_mode": "STANDARD",
        "memory_policy": "EPHEMERAL",
        "capability_grants": [],
        "capability_token_ids": [],
        "session_messages": [],
    });
    if let Some(map) = job_inputs.as_object_mut() {
        if let Some(wp_id) = &request.wp_id {
            map.insert("wp_id".to_owned(), serde_json::json!(wp_id));
        }
        if let Some(mt_id) = &request.mt_id {
            map.insert("mt_id".to_owned(), serde_json::json!(mt_id));
        }
    }
    Ok(RequestSpec {
        method: HttpMethod::Post,
        url: format!("{base_url}{MODEL_SESSION_JOBS_PATH}"),
        body: Some(serde_json::json!({
            "job_kind": "model_run",
            "protocol_id": MODEL_SESSION_PROTOCOL_ID,
            "job_inputs": job_inputs,
        })),
    })
}

/// Health probe (CONTROL-2). Kept as a full URL for the existing MT-002 health wiring.
pub const HEALTH_URL: &str = "http://127.0.0.1:37501/health";

/// Per-request timeout for the layout endpoint. A save must not hang the UI thread; on timeout the
/// transport returns a TRANSIENT [`LayoutError::Transport`] the persistence manager retries.
const LAYOUT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct HealthInfo {
    pub status: String,
    pub db_status: String,
    pub migration_version: Option<i64>,
}

/// GET /health with a 5s request timeout (CONTROL-2) plus the MT-088 backend-down connect timeout
/// ([`build_backend_client`]). Non-success status, a refused connection (backend down), or a parse
/// failure is an error, never a panic. The connect timeout bounds a half-open backend; the off-thread
/// spawn (`HandshakeApp::new` / `poll_health`) keeps this off the egui UI thread.
pub async fn fetch_health(url: &str) -> Result<HealthInfo, AppError> {
    let client = build_backend_client();
    let resp = client
        .get(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "non-success status {}",
            resp.status()
        )));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))?;
    let object = v
        .as_object()
        .ok_or_else(|| AppError::Parse("health response must be a JSON object".to_owned()))?;
    let status = object
        .get("status")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            AppError::Parse("health response requires non-empty string field `status`".to_owned())
        })?;
    if !matches!(status, "ok" | "error") {
        return Err(AppError::Parse(format!(
            "health response field `status` must be one of `ok` or `error`, got `{status}`"
        )));
    }
    let db_status = object
        .get("db_status")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            AppError::Parse(
                "health response requires non-empty string field `db_status`".to_owned(),
            )
        })?;
    if !matches!(db_status, "ok" | "error") {
        return Err(AppError::Parse(format!(
            "health response field `db_status` must be one of `ok` or `error`, got `{db_status}`"
        )));
    }
    let expected_status = if db_status == "ok" { "ok" } else { "error" };
    if status != expected_status {
        return Err(AppError::Parse(format!(
            "health response fields are inconsistent: `status` is `{status}` but producer semantics require `{expected_status}` when `db_status` is `{db_status}`"
        )));
    }
    let migration_version = match object.get("migration_version") {
        None | Some(serde_json::Value::Null) => None,
        Some(value) => Some(value.as_i64().ok_or_else(|| {
            AppError::Parse(
                "health response field `migration_version` must be an integer or null".to_owned(),
            )
        })?),
    };
    Ok(HealthInfo {
        status: status.to_owned(),
        db_status: db_status.to_owned(),
        migration_version,
    })
}

/// REST client for the backend's PostgreSQL-authoritative workbench-layout surface
/// (`GET`/`PUT /workspaces/:workspace_id/workbench/layout`, migration `0323_workbench_layout_state`).
///
/// This is the REAL [`LayoutTransport`] the app wires into its [`LayoutPersistenceManager`]: the
/// native layout persists THROUGH this REST endpoint into PostgreSQL/EventLedger — there is no local
/// file authority (CX-503S / Data Posture). The endpoint stores the snapshot as an opaque JSONB
/// `layout_state` blob, so this client speaks `serde_json::Value` directly and never depends on the
/// `handshake_core` crate's types.
///
/// ## Why a blocking transport over an async client
///
/// reqwest is async, but [`LayoutTransport`] is synchronous so the persistence manager stays a pure,
/// directly-unit-testable state machine. This client holds a tokio runtime [`Handle`] and bridges by
/// `Handle::block_on`. The app calls the transport from a short-lived tokio worker (NOT the egui UI
/// thread — see `HandshakeApp`'s save wiring), so the UI thread is never blocked on the network
/// (HBR-QUIET: background work must not stall the operator).
///
/// [`Handle`]: tokio::runtime::Handle
#[derive(Clone)]
pub struct WorkbenchLayoutClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl WorkbenchLayoutClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    ///
    /// WP-KERNEL-012 MT-088: the client carries the backend-down timeouts ([`build_backend_client`] —
    /// connect + request), so the `GET`/`PUT` layout round-trips this transport performs cannot hang a
    /// worker forever on a dead/half-open backend. This is the transport whose UI-thread-reachable
    /// `block_on` was the latent 2026-06-26 freeze; the freeze fix moves the call off the UI thread (in
    /// `app.rs`) AND bounds the network wait here (defense in depth).
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: build_backend_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn layout_url(&self, workspace_id: &str) -> String {
        format!(
            "{}/workspaces/{}/workbench/layout",
            self.base_url, workspace_id
        )
    }
}

/// Which single flag a [`LoomBlockClient::set_flag`] call PATCHes (MT-021 AC#73). Exactly one flag is
/// sent per request, mapping to the verified flattened `LoomBlockUpdate` field, so a typo can never
/// reach the wrong backend field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoomBlockFlag {
    /// `{ "pinned": <bool> }`.
    Pinned,
    /// `{ "favorite": <bool> }`.
    Favorite,
}

/// The HTTP method a [`RequestSpec`] carries. Kept as a tiny typed enum (not a `reqwest::Method`) so a
/// unit test can assert the method without depending on reqwest internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Post,
    Patch,
    Delete,
    Get,
    /// MT-023: the daily-journal Agenda fetch is a get-or-create PUT (`open_daily_journal`).
    Put,
}

/// The fully-resolved `(method, url, body)` a client method is about to send. Returned by the pure
/// `*_request` builders so a unit test asserts the EXACT verified URL + JSON body (MT-021 MAJOR #1/#2/#3
/// proof) without a live backend. The real spawn paths route through these SAME builders, so the test
/// proves the production request construction, not a parallel reimplementation. `body` is `None` for a
/// bodyless request (DELETE / GET) and `query` carries GET query params.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestSpec {
    pub method: HttpMethod,
    pub url: String,
    pub body: Option<serde_json::Value>,
}

/// A `(method, url, query)` spec for a GET request (diff/blame), where the params live in the query
/// string rather than a JSON body. Separate from [`RequestSpec`] so the query is asserted as typed
/// pairs (order-stable) instead of being smuggled into the URL string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetRequestSpec {
    pub method: HttpMethod,
    pub url: String,
    pub query: Vec<(String, String)>,
}

/// Stable identity for one explorer rename dispatch. Both the monotonic id and backend entity key
/// must still match the open dialog before a completion is allowed to mutate UI state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameOperation {
    pub operation_id: u64,
    pub entity_key: String,
}

/// One correlated off-thread rename completion.
#[derive(Debug)]
pub struct RenameDelivery {
    pub operation: RenameOperation,
    pub result: Result<String, String>,
}

/// FIFO delivery queue for explorer rename results. A queue is required because cancel/reopen and
/// reverse completion may leave more than one worker alive; a one-slot cell loses or overwrites one.
pub type RenameDeliveryCell = Arc<Mutex<VecDeque<RenameDelivery>>>;

/// REST client for the Loom-block surface this shell mutates today: the rename PATCH on the VERIFIED
/// backend endpoint `PATCH /workspaces/:workspace_id/loom/blocks/:block_id` (handler
/// `handshake_core::api::loom::patch_loom_block`, body `LoomBlockPatchRequest` whose flattened
/// `LoomBlockUpdate.title` is the rename field). The body sent is `{ "title": "<new title>" }`.
///
/// ## Off-thread (HBR-QUIET)
///
/// The egui UI thread must never block on the network, so [`rename_block`](Self::rename_block) spawns
/// the PATCH on the app's tokio runtime and delivers the result into a [`RenameDeliveryCell`] the UI
/// drains next frame — the MT-009 off-thread + delivery-cell pattern (the same shape
/// `WorkbenchLayoutClient` + the settings cells use). It speaks `serde_json::Value` so it never depends
/// on the `handshake_core` crate's types.
#[derive(Clone)]
pub struct LoomBlockClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomBlockClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn block_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/blocks/{}",
            self.base_url, workspace_id, block_id
        )
    }

    /// PATCH a single Loom-block FLAG (`pinned` or `favorite`) off the UI thread, delivering the result
    /// into `cell` (MT-021 AC#73). The body is `{ "pinned": <bool> }` or `{ "favorite": <bool> }` —
    /// exactly ONE flag per request, flattened into the verified `LoomBlockUpdate` (the same PATCH
    /// endpoint `rename_block` uses). `Ok(())` on a 2xx; `Err(msg)` on failure. This is what the
    /// `loom.pin` / `loom.favorite` menu actions invoke, so the toggled flag actually persists.
    pub fn set_flag(
        &self,
        workspace_id: &str,
        block_id: &str,
        flag: LoomBlockFlag,
        value: bool,
        cell: ScmReceiptCell,
    ) {
        let spec = self.set_flag_request(workspace_id, block_id, flag, value);
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = patch_expect_success(&client, &spec.url, &body).await;
            let delivered = result.map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(delivered);
            }
        });
    }

    /// Pure request builder for [`set_flag`](Self::set_flag): the `(PATCH, url, body)` it sends. Split
    /// out so a unit test asserts the EXACT verified URL + single-flag JSON body without a live backend
    /// (the spawn path above routes through this same builder, so the test proves the production path).
    pub fn set_flag_request(
        &self,
        workspace_id: &str,
        block_id: &str,
        flag: LoomBlockFlag,
        value: bool,
    ) -> RequestSpec {
        let body = match flag {
            LoomBlockFlag::Pinned => serde_json::json!({ "pinned": value }),
            LoomBlockFlag::Favorite => serde_json::json!({ "favorite": value }),
        };
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.block_url(workspace_id, block_id),
            body: Some(body),
        }
    }

    /// Pure request builder for [`rename_block`](Self::rename_block): the `(PATCH, url, body)` it sends.
    pub fn rename_request(
        &self,
        workspace_id: &str,
        block_id: &str,
        new_title: &str,
        expected_updated_at: Option<&str>,
    ) -> RequestSpec {
        let mut body = serde_json::json!({ "title": new_title });
        if let Some(expected) = expected_updated_at {
            body["expected_updated_at"] = serde_json::Value::String(expected.to_owned());
        }
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.block_url(workspace_id, block_id),
            body: Some(body),
        }
    }

    /// PATCH the block's title off the UI thread, delivering the result into `cell`. The egui UI thread
    /// returns immediately; the spawned task writes `Ok(new_title)` / `Err(msg)` into `cell` and the UI
    /// drains it next frame. The repaint is requested by the caller's normal frame loop (the cell is
    /// drained at the top of `update`).
    pub fn rename_block(
        &self,
        workspace_id: &str,
        block_id: &str,
        new_title: &str,
        expected_updated_at: Option<&str>,
        operation: RenameOperation,
        cell: RenameDeliveryCell,
    ) {
        let url = self.block_url(workspace_id, block_id);
        let client = self.client.clone();
        let new_title = new_title.to_owned();
        let expected_updated_at = expected_updated_at.map(str::to_owned);
        self.runtime.spawn(async move {
            let result =
                patch_block_title(&client, &url, &new_title, expected_updated_at.as_deref()).await;
            let delivered = match result {
                Ok(title) => Ok(title),
                Err(e) => Err(e.to_string()),
            };
            if let Ok(mut queue) = cell.lock() {
                queue.push_back(RenameDelivery {
                    operation,
                    result: delivered,
                });
            }
        });
    }
}

/// Off-thread client for the canvas title mutation route. The optional `expected_updated_at` token
/// provides optimistic concurrency: a stale explorer row receives HTTP 409 and stays open with the
/// backend error instead of overwriting another editor's rename.
#[derive(Clone)]
pub struct CanvasTitleClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl CanvasTitleClient {
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    pub fn rename_request(
        &self,
        canvas_id: &str,
        new_title: &str,
        expected_updated_at: Option<&str>,
    ) -> RequestSpec {
        let mut body = serde_json::json!({ "title": new_title });
        if let Some(expected) = expected_updated_at {
            body["expected_updated_at"] = serde_json::Value::String(expected.to_owned());
        }
        RequestSpec {
            method: HttpMethod::Patch,
            url: format!("{}/canvases/{}", self.base_url, canvas_id),
            body: Some(body),
        }
    }

    pub fn rename_canvas(
        &self,
        canvas_id: &str,
        new_title: &str,
        expected_updated_at: Option<&str>,
        operation: RenameOperation,
        cell: RenameDeliveryCell,
    ) {
        let spec = self.rename_request(canvas_id, new_title, expected_updated_at);
        let client = self.client.clone();
        let fallback_title = new_title.to_owned();
        self.runtime.spawn(async move {
            let result = async {
                let response = client
                    .patch(&spec.url)
                    .json(&spec.body.unwrap_or_default())
                    .send()
                    .await
                    .map_err(|error| AppError::Http(error.to_string()))?;
                let status = response.status();
                let value: serde_json::Value = response
                    .json()
                    .await
                    .map_err(|error| AppError::Parse(error.to_string()))?;
                if !status.is_success() {
                    let detail = value
                        .get("error")
                        .and_then(|value| value.as_str())
                        .unwrap_or("canvas rename failed");
                    return Err(AppError::Http(format!("HTTP {status}: {detail}")));
                }
                Ok(value
                    .get("title")
                    .and_then(|value| value.as_str())
                    .unwrap_or(&fallback_title)
                    .to_owned())
            }
            .await
            .map_err(|error| error.to_string());
            if let Ok(mut queue) = cell.lock() {
                queue.push_back(RenameDelivery { operation, result });
            }
        });
    }
}

/// Send `PATCH {url}` with body `{ "title": <new_title> }` and return the renamed block's title from
/// the response (`LoomBlock.title`), falling back to the sent title if the response omits it. A
/// non-success status or a parse failure is an [`AppError`], never a panic.
async fn patch_block_title(
    client: &reqwest::Client,
    url: &str,
    new_title: &str,
    expected_updated_at: Option<&str>,
) -> Result<String, AppError> {
    let mut body = serde_json::json!({ "title": new_title });
    if let Some(expected) = expected_updated_at {
        body["expected_updated_at"] = serde_json::Value::String(expected.to_owned());
    }
    let resp = client
        .patch(url)
        .timeout(Duration::from_secs(5))
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "PATCH block non-success status {}",
            resp.status()
        )));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))?;
    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .unwrap_or(new_title)
        .to_owned();
    Ok(title)
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// MT-021 (C5 part 2) off-thread clients for the source-control + canvas surfaces.
//
// Every endpoint here was VERIFIED READ-ONLY against `src/backend/handshake_core` (the real running
// backend), NOT assumed from the contract body (whose URLs were partly stale):
//   - source-control routes are mounted at `/source-control/{status,diff,stage,unstage,discard,blame}`
//     with NO `/api` prefix (handshake_core::api::source_control::routes_with_event_recorder). The base
//     URL has no `/api` either (BACKEND_BASE_URL + "/source-control/..."), matching the existing health
//     + workbench-layout clients. `stage`/`unstage` POST `{repo_path, paths}` (PathsRequest); `discard`
//     POSTs `{repo_path, paths, confirmed}` (DiscardRequest — the field is `confirmed`, NOT the
//     contract's `force`, and `confirmed:false` returns HTTP 409 with NO mutation); `diff` is a GET with
//     query `repo_path`,`path`,`scope` (scope ∈ {worktree,staged}); `blame` is a GET with
//     `repo_path`,`path`.
//   - canvas placement routes are `PATCH`/`DELETE /workspaces/:ws/loom/canvas-placements/:placement_id`
//     (handshake_core::api::loom — NOT the contract's `.../loom/canvas/{cb}/placements/{p}`). The
//     placement body supports a real `z_index` field (migration 0334 `loom_canvas_placements.z_index`),
//     so canvas bring-to-front / send-to-back PERSIST (not local-only).
//   - canvas visual-edge delete is `DELETE /workspaces/:ws/loom/canvas-visual-edges/:visual_edge_id`
//     (visual-only edges; semantic Loom edges are a different surface and are NEVER touched here — the
//     red-team `remove_edges` control).
//
// All follow the MT-020 `LoomBlockClient` shape: spawn the request on the app's tokio runtime and
// deliver the outcome into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next frame, so
// the render thread is NEVER blocked on the network (HBR-QUIET).

/// One-slot delivery cell for an off-thread source-control receipt result. `Ok(())` on a successful
/// stage/unstage/discard (the receipt body is not needed by the menu — the panel re-fetches status),
/// `Err(msg)` on failure (surfaced on the panel status row).
pub type ScmReceiptCell = Arc<Mutex<Option<Result<(), String>>>>;

/// One-slot delivery cell for an off-thread source-control diff/blame text result. `Ok(text)` carries
/// the patch (diff) or rendered blame the panel displays; `Err(msg)` the failure.
pub type ScmTextCell = Arc<Mutex<Option<Result<String, String>>>>;

/// One-slot delivery cell for an off-thread canvas placement mutation result. `Ok(())` on a successful
/// placement update/remove (the canvas re-fetches its board), `Err(msg)` the failure.
pub type CanvasOpCell = Arc<Mutex<Option<Result<(), String>>>>;

/// REST client for the VERIFIED Handshake-native source-control surface (MT-253 backend). Drives the
/// stage/unstage/discard write ops and the diff/blame read ops the MT-021 source-control change-row
/// context menu dispatches. Speaks `serde_json::Value` so it never depends on the `handshake_core`
/// crate's types.
#[derive(Clone)]
pub struct SourceControlClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl SourceControlClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    /// `POST /source-control/{stage|unstage}` with `{repo_path, paths:[path]}`, off the UI thread.
    /// `op` is `"stage"` or `"unstage"` — the SAME path segment the verified backend route uses.
    pub fn stage_paths(&self, op: ScmWriteOp, repo_path: &str, path: &str, cell: ScmReceiptCell) {
        let spec = self.stage_request(op, repo_path, path);
        self.spawn_receipt(spec.url, spec.body.unwrap_or_default(), cell);
    }

    /// Pure request builder for [`stage_paths`](Self::stage_paths).
    pub fn stage_request(&self, op: ScmWriteOp, repo_path: &str, path: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!("{}/source-control/{}", self.base_url, op.path_segment()),
            body: Some(serde_json::json!({ "repo_path": repo_path, "paths": [path] })),
        }
    }

    /// `POST /source-control/discard` with `{repo_path, paths:[path], confirmed}`, off the UI thread.
    /// `confirmed` MUST be true to mutate: the verified backend returns HTTP 409 (no mutation) when
    /// `confirmed:false`. The MT-021 menu item is a STUB in V1 (no confirm dialog yet), so the panel
    /// passes `confirmed:false` until a real confirm dialog exists — making an accidental dispatch a
    /// safe 409 no-op, never a destructive discard (red-team discard control).
    pub fn discard_paths(
        &self,
        repo_path: &str,
        path: &str,
        confirmed: bool,
        cell: ScmReceiptCell,
    ) {
        let spec = self.discard_request(repo_path, path, confirmed);
        self.spawn_receipt(spec.url, spec.body.unwrap_or_default(), cell);
    }

    /// Pure request builder for [`discard_paths`](Self::discard_paths).
    pub fn discard_request(&self, repo_path: &str, path: &str, confirmed: bool) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!("{}/source-control/discard", self.base_url),
            body: Some(serde_json::json!({
                "repo_path": repo_path,
                "paths": [path],
                "confirmed": confirmed
            })),
        }
    }

    /// `GET /source-control/diff?repo_path&path&scope`, off the UI thread, delivering the patch text.
    pub fn diff(&self, repo_path: &str, path: &str, scope: ScmDiffScope, cell: ScmTextCell) {
        let spec = self.diff_request(repo_path, path, scope);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = get_json_field(&client, &spec.url, &spec.query, "patch").await;
            deliver_text(&cell, result);
        });
    }

    /// Pure request builder for [`diff`](Self::diff).
    pub fn diff_request(&self, repo_path: &str, path: &str, scope: ScmDiffScope) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: format!("{}/source-control/diff", self.base_url),
            query: vec![
                ("repo_path".to_owned(), repo_path.to_owned()),
                ("path".to_owned(), path.to_owned()),
                ("scope".to_owned(), scope.query_value().to_owned()),
            ],
        }
    }

    /// `GET /source-control/blame?repo_path&path`, off the UI thread, delivering a rendered blame text
    /// (each line `"{short_commit}  {content}"`) for the V1 monospace blame display.
    pub fn blame(&self, repo_path: &str, path: &str, cell: ScmTextCell) {
        let spec = self.blame_request(repo_path, path);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_blame_text(&client, &spec.url, &spec.query).await;
            deliver_text(&cell, result);
        });
    }

    /// Pure request builder for [`blame`](Self::blame).
    pub fn blame_request(&self, repo_path: &str, path: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: format!("{}/source-control/blame", self.base_url),
            query: vec![
                ("repo_path".to_owned(), repo_path.to_owned()),
                ("path".to_owned(), path.to_owned()),
            ],
        }
    }

    /// Shared spawn for a write op (stage/unstage/discard): POST the body, deliver `Ok(())`/`Err(msg)`.
    fn spawn_receipt(&self, url: String, body: serde_json::Value, cell: ScmReceiptCell) {
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = post_expect_success(&client, &url, &body).await;
            let delivered = result.map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(delivered);
            }
        });
    }
}

/// Which source-control write op a [`SourceControlClient::stage_paths`] call performs. The variant maps
/// to the verified backend route's path segment, so a typo can never reach a wrong endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScmWriteOp {
    Stage,
    Unstage,
}

impl ScmWriteOp {
    fn path_segment(self) -> &'static str {
        match self {
            ScmWriteOp::Stage => "stage",
            ScmWriteOp::Unstage => "unstage",
        }
    }
}

/// The diff scope a [`SourceControlClient::diff`] call requests — the verified backend `DiffScope`
/// enum's two lowercase query values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScmDiffScope {
    Worktree,
    Staged,
}

impl ScmDiffScope {
    fn query_value(self) -> &'static str {
        match self {
            ScmDiffScope::Worktree => "worktree",
            ScmDiffScope::Staged => "staged",
        }
    }
}

/// REST client for the VERIFIED canvas placement + visual-edge surface (MT-261 backend). Drives the
/// canvas-node context menu's `move_to_front`/`move_to_back` (PATCH `z_index`), `remove` (DELETE
/// placement), and `remove_edges` (DELETE visual edges) off the UI thread.
#[derive(Clone)]
pub struct CanvasClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl CanvasClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn placement_url(&self, workspace_id: &str, placement_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/canvas-placements/{}",
            self.base_url, workspace_id, placement_id
        )
    }

    fn visual_edge_url(&self, workspace_id: &str, visual_edge_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/canvas-visual-edges/{}",
            self.base_url, workspace_id, visual_edge_id
        )
    }

    /// `PATCH /workspaces/:ws/loom/canvas-placements/:placement_id` with `{z_index}`, off the UI thread.
    /// The verified backend persists `z_index`, so bring-to-front / send-to-back survives a reload (the
    /// red-team z-order-persistence concern is resolved by the real backend field, not a local list).
    pub fn set_z_index(
        &self,
        workspace_id: &str,
        placement_id: &str,
        z_index: i32,
        cell: CanvasOpCell,
    ) {
        let spec = self.set_z_index_request(workspace_id, placement_id, z_index);
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = patch_expect_success(&client, &spec.url, &body).await;
            deliver_op(&cell, result);
        });
    }

    /// Pure request builder for [`set_z_index`](Self::set_z_index).
    pub fn set_z_index_request(
        &self,
        workspace_id: &str,
        placement_id: &str,
        z_index: i32,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.placement_url(workspace_id, placement_id),
            body: Some(serde_json::json!({ "z_index": z_index })),
        }
    }

    /// `DELETE /workspaces/:ws/loom/canvas-placements/:placement_id`, off the UI thread. Removes the
    /// placement (the canvas reference), NOT the underlying LoomBlock (the contract's "Remove from
    /// Canvas, NOT the block").
    pub fn remove_placement(&self, workspace_id: &str, placement_id: &str, cell: CanvasOpCell) {
        let spec = self.remove_placement_request(workspace_id, placement_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = delete_expect_success(&client, &spec.url).await;
            deliver_op(&cell, result);
        });
    }

    /// Pure request builder for [`remove_placement`](Self::remove_placement).
    pub fn remove_placement_request(&self, workspace_id: &str, placement_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Delete,
            url: self.placement_url(workspace_id, placement_id),
            body: None,
        }
    }

    /// `DELETE /workspaces/:ws/loom/canvas-visual-edges/:visual_edge_id`, off the UI thread. ONLY a
    /// VISUAL-only edge is ever passed here — the canvas-node menu's `remove_edges` enumerates the
    /// board's `visual_edges` and never touches a semantic Loom edge (red-team `remove_edges` control).
    pub fn remove_visual_edge(&self, workspace_id: &str, visual_edge_id: &str, cell: CanvasOpCell) {
        let spec = self.remove_visual_edge_request(workspace_id, visual_edge_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = delete_expect_success(&client, &spec.url).await;
            deliver_op(&cell, result);
        });
    }

    /// Pure request builder for [`remove_visual_edge`](Self::remove_visual_edge).
    pub fn remove_visual_edge_request(
        &self,
        workspace_id: &str,
        visual_edge_id: &str,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Delete,
            url: self.visual_edge_url(workspace_id, visual_edge_id),
            body: None,
        }
    }
}

/// Write the receipt result into a [`ScmReceiptCell`]/[`CanvasOpCell`]-shaped cell.
fn deliver_op(cell: &CanvasOpCell, result: Result<(), AppError>) {
    if let Ok(mut slot) = cell.lock() {
        *slot = Some(result.map_err(|e| e.to_string()));
    }
}

/// Write a text result into a [`ScmTextCell`].
fn deliver_text(cell: &ScmTextCell, result: Result<String, AppError>) {
    if let Ok(mut slot) = cell.lock() {
        *slot = Some(result.map_err(|e| e.to_string()));
    }
}

/// POST `body` and treat any 2xx as success (the receipt body is not needed by the menu). A
/// non-success status (e.g. discard's 409 when not confirmed) is an [`AppError`], never a panic.
async fn post_expect_success(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<(), AppError> {
    let resp = client
        .post(url)
        .timeout(Duration::from_secs(5))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "POST non-success status {}",
            resp.status()
        )));
    }
    Ok(())
}

/// POST `body`, require a 2xx response, and return the JSON body. Used when a receipt body matters
/// (MT-101 model-session `/jobs` creation); non-2xx and parse failures are surfaced, never treated as a
/// successful launch.
async fn post_json_expect_value(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
    timeout: Duration,
) -> Result<serde_json::Value, AppError> {
    let resp = client
        .post(url)
        .timeout(timeout)
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "POST non-success status {}",
            resp.status()
        )));
    }
    resp.json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))
}

/// PATCH `body` and treat any 2xx as success.
async fn patch_expect_success(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<(), AppError> {
    let resp = client
        .patch(url)
        .timeout(Duration::from_secs(5))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "PATCH non-success status {}",
            resp.status()
        )));
    }
    Ok(())
}

/// DELETE `url` and treat any 2xx as success.
async fn delete_expect_success(client: &reqwest::Client, url: &str) -> Result<(), AppError> {
    let resp = client
        .delete(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "DELETE non-success status {}",
            resp.status()
        )));
    }
    Ok(())
}

/// GET `url?query` and return the string at top-level JSON `field` (e.g. the diff's `patch`).
async fn get_json_field(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
    field: &str,
) -> Result<String, AppError> {
    let v = get_json(client, url, query).await?;
    let text = v
        .get(field)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_owned();
    Ok(text)
}

/// GET `url?query` and render the verified `SourceControlBlame.lines[]` (`{commit_id, content}`) into a
/// monospace `"{short_commit}  {content}"` block for the V1 blame display.
async fn fetch_blame_text(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<String, AppError> {
    let v = get_json(client, url, query).await?;
    let mut out = String::new();
    if let Some(lines) = v.get("lines").and_then(|x| x.as_array()) {
        for line in lines {
            let commit = line.get("commit_id").and_then(|x| x.as_str()).unwrap_or("");
            let short = commit.chars().take(8).collect::<String>();
            let content = line.get("content").and_then(|x| x.as_str()).unwrap_or("");
            out.push_str(&format!("{short}  {content}\n"));
        }
    }
    Ok(out)
}

/// GET `url?query` and parse the JSON body. A non-success status or parse failure is an [`AppError`].
async fn get_json(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<serde_json::Value, AppError> {
    let resp = client
        .get(url)
        .query(query)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "GET non-success status {}",
            resp.status()
        )));
    }
    resp.json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-023 (C6) off-thread client for the bottom-drawer stash-shelf card data.
//
// Every endpoint here was VERIFIED READ-ONLY against `src/backend/handshake_core` (the real running
// backend), NOT taken from the MT-023 contract body (whose `binds_backend_api` was STALE, the MT-022
// lesson):
//   - The contract named `GET /workspaces/:ws/loom/views/table?content_type=list` and
//     `GET /workspaces/:ws/loom/views/calendar` returning `{ "blocks": [...], "total": N }`. NONE of
//     that exists. `parse_view_type` (handshake_core::api::loom) accepts ONLY
//     {all,unlinked,sorted,pins,favorites} — `table`/`calendar` return HTTP 400 HSK-400-LOOM-VIEW-TYPE.
//     `LoomViewResponse` is `#[serde(tag="view_type")]` with NO `total` field; the count is
//     `blocks.len()`. And `content_type=list` is invalid — `LoomBlockContentType` has no `list`
//     variant (valid: note,file,annotated_file,tag_hub,journal,canvas,view_def → HSK-400 otherwise).
//   - The REAL countable surface is `GET /workspaces/:ws/loom/views/all?content_type=<ct>` (handler
//     `query_loom_view`, `LoomViewQuery.content_type`), response `{ "view_type":"all","blocks":[...] }`.
//     Notes card → content_type=note (exists); the contract's "Lists" maps to the saved
//     block-collection views, whose real content_type is `view_def` (MT-262 BlockCollectionViews).
//   - Agenda has no calendar view to read; the contract's own `ports_from_react` directs the daily
//     journal as the data source: `PUT /workspaces/:ws/loom/journals/:date` (handler
//     `open_daily_journal`, returns a single `LoomBlock`). Badge = 1 if today's journal block has a
//     title/content, else 0; subtitle = its title.
//
// All follow the MT-020/021 off-thread shape: spawn on the app's tokio runtime, deliver the outcome
// into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next frame (HBR-QUIET — the render
// thread is NEVER blocked on the network). Speaks `serde_json::Value` so it never depends on the
// `handshake_core` crate's types.

/// The four drawer card kinds whose badge data this client fetches. Mail makes NO backend call (the
/// contract: no mail backend exists yet), so it has no variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawerDataKind {
    /// Today's daily journal (`PUT /loom/journals/{today}`): badge = has-content, subtitle = title.
    Agenda,
    /// Saved block-collection views (`GET /loom/views/all?content_type=view_def`): badge = count.
    Lists,
    /// Note blocks (`GET /loom/views/all?content_type=note`): badge = count.
    Notes,
}

impl DrawerDataKind {
    /// The verified `content_type` query value for the count fetch. `Agenda` uses the journal endpoint
    /// (not a view count), so it has no content_type and returns `None`.
    pub fn content_type(self) -> Option<&'static str> {
        match self {
            DrawerDataKind::Agenda => None,
            // The contract's "Lists" = saved block-collection views; their real content_type is
            // `view_def` (no `list` content_type exists — disclosed MT-023 deviation).
            DrawerDataKind::Lists => Some("view_def"),
            DrawerDataKind::Notes => Some("note"),
        }
    }
}

/// The externally-meaningful result of one drawer card fetch: the badge count plus a one-line subtitle.
/// `Ok` carries the live data; `Err(msg)` a failure the card surfaces without crashing the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawerCardData {
    /// Badge count (CONTROL-023-D: a missing/empty result defaults to 0, never an error).
    pub badge_count: u32,
    /// One-line subtitle (e.g. the journal title, or a "N items" summary).
    pub subtitle: String,
}

/// One-slot delivery cell for an off-thread drawer-card fetch result, keyed by which card it is. The
/// spawned task writes `(kind, Ok(data))` / `(kind, Err(msg))`; the egui UI thread drains it next frame
/// and folds it into the matching card (same `Arc<Mutex<Option<..>>>` pattern as the SCM/rename cells).
pub type DrawerDataCell = Arc<Mutex<Option<(DrawerDataKind, Result<DrawerCardData, String>)>>>;

/// REST client for the VERIFIED Loom view-count + daily-journal surfaces the MT-023 bottom drawer reads.
#[derive(Clone)]
pub struct DrawerDataClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl DrawerDataClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn views_all_url(&self, workspace_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/views/all",
            self.base_url, workspace_id
        )
    }

    fn journal_url(&self, workspace_id: &str, journal_date: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/journals/{}",
            self.base_url, workspace_id, journal_date
        )
    }

    /// Pure request builder for a view-count fetch: `GET /loom/views/all?content_type=<ct>`. Split out so
    /// a unit test asserts the EXACT verified URL + query without a live backend (the spawn path routes
    /// through this same builder). `kind` must be `Lists` or `Notes` (Agenda has no content_type).
    pub fn count_request(&self, workspace_id: &str, kind: DrawerDataKind) -> GetRequestSpec {
        let content_type = kind
            .content_type()
            .expect("count_request requires a content_type kind (Lists/Notes), not Agenda");
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.views_all_url(workspace_id),
            query: vec![("content_type".to_owned(), content_type.to_owned())],
        }
    }

    /// Pure request builder for the Agenda fetch: `PUT /loom/journals/{today}` (the journal endpoint is a
    /// PUT — `open_daily_journal` get-or-creates today's journal block). No body.
    pub fn journal_request(&self, workspace_id: &str, journal_date: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Put,
            url: self.journal_url(workspace_id, journal_date),
            body: None,
        }
    }

    /// Fetch the Lists/Notes badge count off the UI thread, delivering `(kind, Ok/Err)` into `cell`. The
    /// count is `blocks.len()` from the verified `{ "view_type":"all","blocks":[...] }` response
    /// (CONTROL-023-D: an absent/empty `blocks` array yields 0, never an error).
    pub fn fetch_count(&self, workspace_id: &str, kind: DrawerDataKind, cell: DrawerDataCell) {
        let spec = self.count_request(workspace_id, kind);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_view_count(&client, &spec.url, &spec.query).await;
            deliver_drawer(&cell, kind, result.map_err(|e| e.to_string()));
        });
    }

    /// Fetch today's Agenda data off the UI thread, delivering `(Agenda, Ok/Err)` into `cell`. Badge = 1
    /// if today's journal block has a non-empty title, else 0; subtitle = the title (or "No agenda today").
    pub fn fetch_agenda(&self, workspace_id: &str, journal_date: &str, cell: DrawerDataCell) {
        let spec = self.journal_request(workspace_id, journal_date);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_daily_journal(&client, &spec.url).await;
            deliver_drawer(
                &cell,
                DrawerDataKind::Agenda,
                result.map_err(|e| e.to_string()),
            );
        });
    }
}

/// Write a drawer fetch result into a [`DrawerDataCell`].
fn deliver_drawer(
    cell: &DrawerDataCell,
    kind: DrawerDataKind,
    result: Result<DrawerCardData, String>,
) {
    if let Ok(mut slot) = cell.lock() {
        *slot = Some((kind, result));
    }
}

/// `GET {url}?{query}` and count the `blocks` array length from the verified `LoomViewResponse::All`
/// shape `{ "view_type":"all", "blocks":[...] }`. A missing/null `blocks` field counts as 0
/// (CONTROL-023-D — never an error). A non-success status or parse failure is an [`AppError`].
async fn fetch_view_count(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<DrawerCardData, AppError> {
    let v = get_json(client, url, query).await?;
    let count = v
        .get("blocks")
        .and_then(|b| b.as_array())
        .map(|a| a.len())
        .unwrap_or(0) as u32;
    let subtitle = if count == 1 {
        "1 item".to_owned()
    } else {
        format!("{count} items")
    };
    Ok(DrawerCardData {
        badge_count: count,
        subtitle,
    })
}

/// `PUT {url}` (no body) and read today's daily-journal block (the verified `open_daily_journal`
/// response is a single `LoomBlock`). Badge = 1 if the block has a non-empty `title`, else 0; subtitle
/// is the title (or a "No agenda today" fallback). A non-success status or parse failure is an
/// [`AppError`], never a panic.
async fn fetch_daily_journal(
    client: &reqwest::Client,
    url: &str,
) -> Result<DrawerCardData, AppError> {
    let resp = client
        .put(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "PUT journal non-success status {}",
            resp.status()
        )));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))?;
    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty());
    match title {
        Some(t) => Ok(DrawerCardData {
            badge_count: 1,
            subtitle: t.to_owned(),
        }),
        None => Ok(DrawerCardData {
            badge_count: 0,
            subtitle: "No agenda today".to_owned(),
        }),
    }
}

impl LayoutTransport for WorkbenchLayoutClient {
    /// `GET /workspaces/:id/workbench/layout`. The backend's `WorkbenchLayoutResponse` carries the
    /// required `layout_state: Option<Value>` field — explicit `null` means first run (`Ok(None)`), while
    /// a missing field is a malformed response and therefore a transport error.
    /// A non-success status or a transport error is a TRANSIENT [`LayoutError::Transport`].
    fn load(&self, workspace_id: &str) -> Result<Option<Value>, LayoutError> {
        let url = self.layout_url(workspace_id);
        let client = self.client.clone();
        self.runtime.block_on(async move {
            let resp = client
                .get(&url)
                .timeout(LAYOUT_REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(|e| LayoutError::Transport(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(LayoutError::Transport(format!(
                    "GET layout non-success status {}",
                    resp.status()
                )));
            }
            let body: Value = resp
                .json()
                .await
                .map_err(|e| LayoutError::Transport(e.to_string()))?;
            // The response contract requires the field. Only an explicit null means first run; accepting
            // an absent field would misclassify a malformed/backend-incompatible response as success.
            match body.get("layout_state") {
                Some(Value::Null) => Ok(None),
                Some(v) => Ok(Some(v.clone())),
                None => Err(LayoutError::Transport(
                    "GET layout response missing required `layout_state` field".to_owned(),
                )),
            }
        })
    }

    /// `PUT /workspaces/:id/workbench/layout` with `SaveWorkbenchLayoutRequest { layout_state }`.
    /// A non-success status or a transport error is a TRANSIENT [`LayoutError::Transport`] the
    /// manager retries; the in-memory layout is unaffected by a save failure.
    fn save(&self, workspace_id: &str, layout_state: Value) -> Result<(), LayoutError> {
        let url = self.layout_url(workspace_id);
        let client = self.client.clone();
        let request_body = serde_json::json!({ "layout_state": layout_state });
        self.runtime.block_on(async move {
            let resp = client
                .put(&url)
                .timeout(LAYOUT_REQUEST_TIMEOUT)
                .json(&request_body)
                .send()
                .await
                .map_err(|e| LayoutError::Transport(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(LayoutError::Transport(format!(
                    "PUT layout non-success status {}",
                    resp.status()
                )));
            }
            Ok(())
        })
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-024 (C6) off-thread client for the bottom-drawer CARD ACTION backend mutations.
//
// The MT-023 drawer renders four TYPE cards (Agenda/Mail/Lists/Notes). MT-024 wires the typed action
// menu the cards expose. The PERSISTING actions route through THIS client; every endpoint + body was
// VERIFIED READ-ONLY against `src/backend/handshake_core` (the real running backend), NOT taken from
// the MT-024 contract body (whose `binds_backend_api` was partly STALE — the MT-022/MT-023 lesson):
//
//   - PIN: `PUT /workspaces/:ws/loom/blocks/:block_id/pin-order` (handler `set_loom_block_pin_order`,
//     MT-183). VERIFIED body field is `{ "pin_order": <i32|null> }` (struct `SetPinOrderRequest`) —
//     NOT the contract's `{ "ordinal": 0 }`. To bring a card to the top we send `pin_order: 0`.
//   - DISCARD: `DELETE /workspaces/:ws/loom/blocks/:block_id` (handler `delete_loom_block`). Bodyless.
//   - STOW: the contract proposed `PATCH ... { metadata: { stash_state } }` OR `{ content_type }`.
//     NEITHER is patchable: the VERIFIED `LoomBlockUpdate` (storage/loom.rs) has ONLY
//     {title,pinned,favorite,journal_date,pin_order} — there is NO `metadata`, `stash_state`, or
//     `content_type` field on the PATCH. The contract's OWN implementation_note names the fallback:
//     the tag-edge approach. So STOW = `POST /workspaces/:ws/loom/edges` with
//     `{ source_block_id, target_block_id:<stash hub>, edge_type:"tag", created_by:"user",
//        target_title:"stash" }` (VERIFIED `CreateLoomEdgeRequest`; `ensure_edge_target_exists`
//     get-or-creates the `stash` TagHub on first use, so no separate hub-creation call is needed).
//   - ATTACH-EVIDENCE: `POST /diagnostics` (handler `create_diagnostic`, body `DiagnosticInput`).
//     VERIFIED enum values: `source` ∈ {lsp,terminal,validator,engine,connector,system,plugin:*,
//     matcher:*} and `surface` ∈ {monaco,canvas,sheet,terminal,connector,system} — the contract's
//     `source:"user"` + `surface:"drawer"` do NOT exist and would HTTP-400. We send the honest closest
//     valid values `source:"system"`, `surface:"system"`, severity `"info"`, and carry the stashed
//     block id in `evidence_refs.artifact_hashes` (the VERIFIED `EvidenceRefs.artifact_hashes` field).
//   - CONVERT-TO-ARTIFACT: there is NO backend surface to change a block's content_type (no PATCH
//     field, no dedicated endpoint). It therefore CANNOT be wired honestly and remains a disabled V1
//     menu item (same treatment as the existing MT-021 `convert_artifact` stub) — disclosed deviation.
//
// All follow the MT-020/021/023 off-thread shape: spawn on the app's tokio runtime, deliver the
// outcome into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next frame (HBR-QUIET —
// the render thread is NEVER blocked on the network). Speaks `serde_json::Value` so it never depends on
// the `handshake_core` crate's types.

/// One-slot delivery cell for an off-thread drawer-card action result. `Ok(())` on a 2xx (the card
/// removes/refreshes optimistically AFTER the backend confirms), `Err(msg)` on failure (the card stays
/// and the drawer surfaces the error). Same shape as the SCM/canvas receipt cells.
pub type DrawerActionCell = Arc<Mutex<Option<Result<(), String>>>>;

/// The well-known title of the per-workspace "stash" TagHub a Stow action tags blocks into. The
/// backend `ensure_edge_target_exists` get-or-creates a TagHub with this title on first tag-edge
/// creation, so Stow never needs a separate hub-creation round-trip.
pub const STASH_TAG_TITLE: &str = "stash";

/// The deterministic block id of the per-workspace stash TagHub. A stable, content-addressable id
/// (`stash` hub is a singleton per workspace) so repeated Stows tag the SAME hub and a swarm reader can
/// address it. `ensure_edge_target_exists` creates it with `content_type=tag_hub` + `target_title` on
/// the first Stow if absent.
pub const STASH_TAG_HUB_BLOCK_ID: &str = "tag-hub-stash";

/// REST client for the VERIFIED Loom-block + diagnostics surfaces the MT-024 drawer card ACTIONS
/// mutate: pin-order (Pin), block delete (Discard), tag-edge (Stow), and diagnostic create
/// (Attach-evidence). Mirrors the `LoomBlockClient`/`CanvasClient` shape exactly.
#[derive(Clone)]
pub struct DrawerActionClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl DrawerActionClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn pin_order_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/blocks/{}/pin-order",
            self.base_url, workspace_id, block_id
        )
    }

    fn block_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/blocks/{}",
            self.base_url, workspace_id, block_id
        )
    }

    fn edges_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/edges", self.base_url, workspace_id)
    }

    fn diagnostics_url(&self) -> String {
        format!("{}/diagnostics", self.base_url)
    }

    // ── Pin ─────────────────────────────────────────────────────────────────────────────────────────

    /// Pure request builder for [`pin_to_top`](Self::pin_to_top): `PUT /loom/blocks/:id/pin-order` with
    /// `{ "pin_order": <ordinal> }`. The field is `pin_order` (VERIFIED `SetPinOrderRequest`), NOT the
    /// contract's `ordinal`. Bring-to-top sends ordinal 0.
    pub fn pin_order_request(
        &self,
        workspace_id: &str,
        block_id: &str,
        ordinal: i32,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Put,
            url: self.pin_order_url(workspace_id, block_id),
            body: Some(serde_json::json!({ "pin_order": ordinal })),
        }
    }

    /// Bring a card's block to the top of the Pins grid (ordinal 0) off the UI thread, delivering the
    /// result into `cell`.
    pub fn pin_to_top(&self, workspace_id: &str, block_id: &str, cell: DrawerActionCell) {
        let spec = self.pin_order_request(workspace_id, block_id, 0);
        let body = spec.body.unwrap_or_default();
        self.spawn_put_receipt(spec.url, body, cell);
    }

    // ── Discard ─────────────────────────────────────────────────────────────────────────────────────

    /// Pure request builder for [`discard`](Self::discard): `DELETE /loom/blocks/:id`, bodyless.
    pub fn discard_request(&self, workspace_id: &str, block_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Delete,
            url: self.block_url(workspace_id, block_id),
            body: None,
        }
    }

    /// DELETE the card's block off the UI thread, delivering the result into `cell`. DESTRUCTIVE: the
    /// caller MUST only invoke this after the confirm-discard guard is `true` (HBR-STOP / RISK-024-A).
    pub fn discard(&self, workspace_id: &str, block_id: &str, cell: DrawerActionCell) {
        let spec = self.discard_request(workspace_id, block_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = delete_expect_success(&client, &spec.url).await;
            deliver_drawer_action(&cell, result);
        });
    }

    // ── Stow (tag-edge to the stash TagHub) ───────────────────────────────────────────────────────────

    /// Pure request builder for [`stow`](Self::stow): `POST /loom/edges` with a VERIFIED
    /// `CreateLoomEdgeRequest` that tags the card's block into the per-workspace `stash` TagHub. The
    /// `target_title` lets `ensure_edge_target_exists` get-or-create the hub on first use.
    pub fn stow_request(&self, workspace_id: &str, block_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: self.edges_url(workspace_id),
            body: Some(serde_json::json!({
                "source_block_id": block_id,
                "target_block_id": STASH_TAG_HUB_BLOCK_ID,
                "edge_type": "tag",
                "created_by": "user",
                "target_title": STASH_TAG_TITLE,
            })),
        }
    }

    /// Tag the card's block into the `stash` TagHub off the UI thread, delivering the result into `cell`.
    pub fn stow(&self, workspace_id: &str, block_id: &str, cell: DrawerActionCell) {
        let spec = self.stow_request(workspace_id, block_id);
        let body = spec.body.unwrap_or_default();
        self.spawn_post_receipt(spec.url, body, cell);
    }

    // ── Attach evidence (diagnostic create) ──────────────────────────────────────────────────────────

    /// Pure request builder for [`attach_evidence`](Self::attach_evidence): `POST /diagnostics` with a
    /// VERIFIED `DiagnosticInput`. `source:"system"` + `surface:"system"` are the honest closest valid
    /// enum values (the contract's `user`/`drawer` do not exist); the stashed block id is carried in
    /// `evidence_refs.artifact_hashes`. `job_id` is the active job when present (AC-024-9).
    pub fn attach_evidence_request(
        &self,
        workspace_id: &str,
        block_id: &str,
        block_title: &str,
        job_id: Option<&str>,
    ) -> RequestSpec {
        let mut body = serde_json::json!({
            "title": format!("Evidence: {block_title}"),
            "message": "Attached from drawer stash shelf",
            "severity": "info",
            "source": "system",
            "surface": "system",
            "wsid": workspace_id,
            "evidence_refs": { "artifact_hashes": { block_id: block_id } },
        });
        if let Some(job_id) = job_id {
            body["job_id"] = serde_json::Value::String(job_id.to_owned());
        }
        RequestSpec {
            method: HttpMethod::Post,
            url: self.diagnostics_url(),
            body: Some(body),
        }
    }

    /// Record the card's block as an evidence diagnostic off the UI thread, delivering the result into
    /// `cell`. The caller is responsible for the AC-024-9 "no active job" pre-check (it shows a tooltip
    /// and makes NO call when there is no active job).
    pub fn attach_evidence(
        &self,
        workspace_id: &str,
        block_id: &str,
        block_title: &str,
        job_id: Option<&str>,
        cell: DrawerActionCell,
    ) {
        let spec = self.attach_evidence_request(workspace_id, block_id, block_title, job_id);
        let body = spec.body.unwrap_or_default();
        self.spawn_post_receipt(spec.url, body, cell);
    }

    // ── Shared spawns ────────────────────────────────────────────────────────────────────────────────

    fn spawn_post_receipt(&self, url: String, body: serde_json::Value, cell: DrawerActionCell) {
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = post_expect_success(&client, &url, &body).await;
            deliver_drawer_action(&cell, result);
        });
    }

    fn spawn_put_receipt(&self, url: String, body: serde_json::Value, cell: DrawerActionCell) {
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = put_expect_success(&client, &url, &body).await;
            deliver_drawer_action(&cell, result);
        });
    }
}

/// Write a drawer-action receipt result into a [`DrawerActionCell`].
fn deliver_drawer_action(cell: &DrawerActionCell, result: Result<(), AppError>) {
    if let Ok(mut slot) = cell.lock() {
        *slot = Some(result.map_err(|e| e.to_string()));
    }
}

/// PUT `body` and treat any 2xx as success (the pin-order receipt body is not needed by the card).
async fn put_expect_success(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<(), AppError> {
    let resp = client
        .put(url)
        .timeout(Duration::from_secs(5))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "PUT non-success status {}",
            resp.status()
        )));
    }
    Ok(())
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-021 Loom-graph transport (REUSE the existing reqwest/timeout/error shape).
//
// The native Loom GRAPH VIEW (`graph::graph_view::LoomGraphView`) binds the EXISTING handshake_core
// Loom read APIs through THIS client (NOT Tauri — the contract's "Tauri command" reference is the
// legacy React/webview stack; this is the native egui app, so it uses the same HTTP client every other
// MT-008/014/015 surface uses). Two modes, both VERIFIED READ-ONLY against `src/backend/handshake_core`:
//   - GLOBAL: `GET /workspaces/:ws/loom/graph/global?node_limit=5000&hub_degree_threshold=0` ->
//     `LoomGraph`. Disabling hub suppression is required for the MT-021 "all LoomBlocks" graph; a
//     valid backend-capped response remains renderable with an explicit truncation affordance.
//   - LOCAL: `GET /workspaces/:ws/loom/graph/local?start_block_id={id}&max_depth={depth}&node_limit=200`
//     -> `LoomGraph`, the authoritative undirected PostgreSQL neighbourhood with real LoomEdges.
//
// `views/all` remains the independent count oracle used by the managed-PG proof. `graph-search` is a
// heterogeneous retrieval/search surface, not a graph projection, and MUST NOT be used to fabricate
// star edges for this view.
//
// Follows the MT-020/021/023 off-thread shape: spawn on the app's tokio runtime, deliver the parsed
// graph into a queued `Arc<Mutex<VecDeque<LoomGraphDelivery>>>` the UI drains next frame (HBR-QUIET — the
// render thread is NEVER blocked on the network). Speaks `serde_json::Value` so it never depends on the
// `handshake_core` crate's types; the parsed node/edge shapes are the widget's own
// `graph::graph_view::{GraphNode, GraphEdge}` (the field-correct reuse of the verified backend shapes).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use crate::graph::graph_view::{GraphEdge, GraphNode};

/// The externally-meaningful result of a Loom-graph fetch: the node + edge lists the
/// [`crate::graph::graph_view::LoomGraphView`] renders. `Ok` carries the live graph; `Err(msg)` a
/// failure the view surfaces as an error label (AC8) instead of crashing.
pub type LoomGraphCell = Arc<Mutex<VecDeque<LoomGraphDelivery>>>;

/// The exact graph projection a request targets. This is carried through the asynchronous transport so
/// the host can reject deliveries for a previous mode, focus block, depth, or workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoomGraphRequestMode {
    Global,
    Local { focus_block_id: String, depth: u32 },
}

/// Monotonic host generation plus every input that makes one graph response meaningful.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoomGraphRequestIdentity {
    pub generation: u64,
    pub workspace_id: String,
    pub mode: LoomGraphRequestMode,
}

impl LoomGraphRequestIdentity {
    pub fn global(generation: u64, workspace_id: impl Into<String>) -> Self {
        Self {
            generation,
            workspace_id: workspace_id.into(),
            mode: LoomGraphRequestMode::Global,
        }
    }

    pub fn local(
        generation: u64,
        workspace_id: impl Into<String>,
        focus_block_id: impl Into<String>,
        depth: u32,
    ) -> Self {
        Self {
            generation,
            workspace_id: workspace_id.into(),
            mode: LoomGraphRequestMode::Local {
                focus_block_id: focus_block_id.into(),
                depth: depth.clamp(MIN_BACKLINK_DEPTH, MAX_BACKLINK_DEPTH),
            },
        }
    }
}

/// One completed graph request. A queue is required here: an older completion must never overwrite a
/// newer completion before the UI thread gets a chance to validate both identities.
#[derive(Debug)]
pub struct LoomGraphDelivery {
    pub request: LoomGraphRequestIdentity,
    pub result: Result<LoomGraphData, String>,
}

/// A parsed Loom graph (nodes + edges) plus the focus block id the fetch was for (so a stale delivery
/// for a previous mode/block can be detected by the host).
#[derive(Debug, Clone, PartialEq)]
pub struct LoomGraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    /// The backend deliberately capped this projection. The graph remains valid and renderable, but the
    /// view must disclose that the returned node count is not the complete canonical set.
    pub truncated: bool,
    /// Hub ids deliberately excluded by the backend projection policy. Kept as metadata rather than
    /// converting a valid bounded graph into a fatal transport error.
    pub suppressed_hub_ids: Vec<String>,
}

/// REST client for the VERIFIED Loom graph read surfaces the MT-021 graph view binds: `graph/global`
/// and `graph/local`. Mirrors the `DrawerDataClient` off-thread delivery shape.
#[derive(Clone)]
pub struct LoomGraphClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomGraphClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client().clone(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn global_graph_url(&self, workspace_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/graph/global",
            self.base_url, workspace_id
        )
    }

    fn local_graph_url(&self, workspace_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/graph/local",
            self.base_url, workspace_id
        )
    }

    /// Pure request builder for the GLOBAL graph fetch. Hub suppression is disabled so every block is
    /// present up to the backend's explicit hard ceiling; a capped projection remains visibly partial.
    pub fn global_request(&self, workspace_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.global_graph_url(workspace_id),
            query: vec![
                ("node_limit".to_owned(), "5000".to_owned()),
                ("hub_degree_threshold".to_owned(), "0".to_owned()),
            ],
        }
    }

    /// Pure request builder for the LOCAL neighbourhood fetch. The canonical route keys traversal by
    /// stable block id, never by a title search.
    pub fn local_request(&self, workspace_id: &str, block_id: &str) -> GetRequestSpec {
        self.local_request_with_depth(workspace_id, block_id, DEFAULT_BACKLINK_DEPTH)
    }

    /// WP-KERNEL-012 MT-080 (E11 host-mount, AC-080-3 / MT-060 deep wiring): the DEPTH-parameterized
    /// variant of [`local_request`](Self::local_request). The graph view's MT-060 link-depth slider fires
    /// `GraphEvent::DepthChanged { depth }`; the host re-fires the EXISTING `graph/local` endpoint with a
    /// new `max_depth` (NO new endpoint — only the verified query parameter changes). `depth` is
    /// clamped to `[MIN..=MAX]_BACKLINK_DEPTH` so a slider/agent value can never send an out-of-range or
    /// abusive depth to the backend. `local_request` delegates here with the default depth, so the two stay
    /// one builder (no second URL surface to drift).
    pub fn local_request_with_depth(
        &self,
        workspace_id: &str,
        block_id: &str,
        depth: u32,
    ) -> GetRequestSpec {
        let depth = depth.clamp(MIN_BACKLINK_DEPTH, MAX_BACKLINK_DEPTH);
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.local_graph_url(workspace_id),
            query: vec![
                ("start_block_id".to_owned(), block_id.to_owned()),
                ("max_depth".to_owned(), depth.to_string()),
                ("node_limit".to_owned(), "200".to_owned()),
            ],
        }
    }

    /// Fetch the GLOBAL graph (all blocks) off the UI thread, delivering the parsed graph into `cell`.
    pub fn fetch_global(&self, workspace_id: &str, generation: u64, cell: LoomGraphCell) {
        let spec = self.global_request(workspace_id);
        let request = LoomGraphRequestIdentity::global(generation, workspace_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_graph_projection(&client, &spec.url, &spec.query).await;
            deliver_graph(&cell, request, result.map_err(|e| e.to_string()));
        });
    }

    /// Fetch the LOCAL neighbourhood of the focused block off the UI thread, delivering the parsed graph
    /// into `cell`. `focus_block_id` is the stable id the canonical local traversal requires.
    pub fn fetch_local(
        &self,
        workspace_id: &str,
        focus_block_id: &str,
        focus_title: &str,
        generation: u64,
        cell: LoomGraphCell,
    ) {
        self.fetch_local_with_depth(
            workspace_id,
            focus_block_id,
            focus_title,
            DEFAULT_BACKLINK_DEPTH,
            generation,
            cell,
        );
    }

    /// WP-KERNEL-012 MT-080 (AC-080-3 / MT-060): fetch the LOCAL neighbourhood at a specific
    /// `max_depth`, the re-query the host fires on `GraphEvent::DepthChanged`. Same off-thread spawn +
    /// parse path as [`fetch_local`](Self::fetch_local); only the query `max_depth` differs (the
    /// EXISTING endpoint, NO new route). `fetch_local` delegates here with the default depth.
    pub fn fetch_local_with_depth(
        &self,
        workspace_id: &str,
        focus_block_id: &str,
        _focus_title: &str,
        depth: u32,
        generation: u64,
        cell: LoomGraphCell,
    ) {
        let spec = self.local_request_with_depth(workspace_id, focus_block_id, depth);
        let request =
            LoomGraphRequestIdentity::local(generation, workspace_id, focus_block_id, depth);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_graph_projection(&client, &spec.url, &spec.query).await;
            deliver_graph(&cell, request, result.map_err(|e| e.to_string()));
        });
    }
}

/// WP-KERNEL-012 MT-080 (MT-060 link-depth): the default `max_depth` the local neighbourhood fetch
/// uses (the value `local_request` carried before the depth parameter was threaded — unchanged behavior
/// for the non-depth path).
pub const DEFAULT_BACKLINK_DEPTH: u32 = 2;
/// The minimum `max_depth` the depth-parameterized graph re-query will send. A depth of 1 is the
/// focused block plus its direct neighbours (the shallowest useful local view).
pub const MIN_BACKLINK_DEPTH: u32 = 1;
/// The maximum `max_depth` the depth-parameterized graph re-query will send. Clamps a slider/agent
/// value so an out-of-range depth can never reach the backend as an abusive traversal (RISK-080-3 — the
/// re-query stays inside the verified endpoint's safe envelope).
pub const MAX_BACKLINK_DEPTH: u32 = 5;

/// Write a graph fetch result into a [`LoomGraphCell`].
fn deliver_graph(
    cell: &LoomGraphCell,
    request: LoomGraphRequestIdentity,
    result: Result<LoomGraphData, String>,
) {
    if let Ok(mut queue) = cell.lock() {
        queue.push_back(LoomGraphDelivery { request, result });
    }
}

/// Parse one verified `LoomBlock` JSON object into a [`GraphNode`]. `title` falls back to the block id
/// when null/empty so a node is never label-less. `content_type` defaults to "other" (slate) when
/// absent. Returns `None` only when the block has no `block_id` (a malformed row is skipped, not faked).
fn block_to_node(block: &serde_json::Value) -> Option<GraphNode> {
    let block_id = block
        .get("block_id")
        .and_then(|x| x.as_str())
        .filter(|id| !id.trim().is_empty())?
        .to_owned();
    let title = block
        .get("title")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&block_id)
        .to_owned();
    let content_type = block
        .get("content_type")
        .and_then(|x| x.as_str())
        .unwrap_or("other")
        .to_owned();
    Some(GraphNode::new(block_id, title, content_type))
}

/// `GET {url}?{query}` and parse the canonical backend `LoomGraph` projection. Both local and global
/// routes return `{nodes:[{block,...}], edges:[{edge,...}], truncated, suppressed_hub_ids?}`. The
/// backend deliberately omits `suppressed_hub_ids` when it is empty, so absence decodes as an empty
/// list; when present it remains strictly typed. Empty node/edge arrays are a valid workspace (AC7).
async fn fetch_graph_projection(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<LoomGraphData, AppError> {
    let v = get_json(client, url, query).await?;
    parse_graph_projection(v)
}

fn parse_graph_projection(v: Value) -> Result<LoomGraphData, AppError> {
    let truncated = v
        .get("truncated")
        .and_then(Value::as_bool)
        .ok_or_else(|| AppError::Parse("LoomGraph.truncated must be a bool".to_owned()))?;
    let mut suppressed_hub_ids = Vec::new();
    if let Some(suppressed_value) = v.get("suppressed_hub_ids") {
        let suppressed_rows = suppressed_value.as_array().ok_or_else(|| {
            AppError::Parse("LoomGraph.suppressed_hub_ids must be an array".to_owned())
        })?;
        suppressed_hub_ids.reserve(suppressed_rows.len());
        for (index, id) in suppressed_rows.iter().enumerate() {
            let id = id
                .as_str()
                .filter(|id| !id.trim().is_empty())
                .ok_or_else(|| {
                    AppError::Parse(format!(
                        "LoomGraph.suppressed_hub_ids[{index}] must be a non-empty string"
                    ))
                })?;
            suppressed_hub_ids.push(id.to_owned());
        }
    }

    let node_rows = v
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::Parse("LoomGraph.nodes must be an array".to_owned()))?;
    let mut nodes = Vec::with_capacity(node_rows.len());
    for (index, row) in node_rows.iter().enumerate() {
        let node = row.get("block").and_then(block_to_node).ok_or_else(|| {
            AppError::Parse(format!("LoomGraph.nodes[{index}].block is malformed"))
        })?;
        nodes.push(node);
    }

    let edge_rows = v
        .get("edges")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::Parse("LoomGraph.edges must be an array".to_owned()))?;
    let mut edges = Vec::with_capacity(edge_rows.len());
    for (index, row) in edge_rows.iter().enumerate() {
        let edge = row
            .get("edge")
            .ok_or_else(|| AppError::Parse(format!("LoomGraph.edges[{index}].edge is missing")))?;
        let edge_id = edge
            .get("edge_id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| {
                AppError::Parse(format!(
                    "LoomGraph.edges[{index}].edge.edge_id is malformed"
                ))
            })?;
        let source = edge
            .get("source_block_id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| {
                AppError::Parse(format!(
                    "LoomGraph.edges[{index}].edge.source_block_id is malformed"
                ))
            })?;
        let target = edge
            .get("target_block_id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| {
                AppError::Parse(format!(
                    "LoomGraph.edges[{index}].edge.target_block_id is malformed"
                ))
            })?;
        let edge_type = edge
            .get("edge_type")
            .and_then(Value::as_str)
            .filter(|kind| !kind.trim().is_empty())
            .ok_or_else(|| {
                AppError::Parse(format!(
                    "LoomGraph.edges[{index}].edge.edge_type is malformed"
                ))
            })?;
        edges.push(GraphEdge::with_id(edge_id, source, target, edge_type));
    }

    let node_ids: std::collections::HashSet<&str> =
        nodes.iter().map(|node| node.block_id.as_str()).collect();
    if let Some(edge) = edges.iter().find(|edge| {
        !node_ids.contains(edge.source.as_str()) || !node_ids.contains(edge.target.as_str())
    }) {
        return Err(AppError::Parse(format!(
            "LoomGraph edge {} -> {} references a node outside the projection",
            edge.source, edge.target
        )));
    }

    Ok(LoomGraphData {
        nodes,
        edges,
        truncated,
        suppressed_hub_ids,
    })
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-026 Loom CANVAS-BOARD transport (E3 — the Obsidian-Canvas-class surface).
//
// VERIFIED READ-ONLY against `src/backend/handshake_core/src/{api,storage}/loom.rs` (the running
// backend), NOT taken from the MT-026 contract body — whose `binds_backend_api` URLs were STALE
// (the contract named `.../loom/canvas/{cb}/...`; the REAL routes are `.../loom/canvas-boards/...`
// + the placement/visual-edge routes under `.../loom/canvas-placements` / `.../loom/canvas-visual-edges`,
// matching the existing CanvasClient that already verified the placement DELETE/PATCH shape). The
// MT-022/023/024 "verify, don't trust the contract" lesson. The nine routes this client binds:
//   - GET    /workspaces/:ws/loom/canvas-boards/:block_id              get_canvas_board -> LoomCanvasBoardView
//                                                                       { board{board_state{pan_x,pan_y,zoom}},
//                                                                         placements[], visual_edges[] }
//   - PUT    /workspaces/:ws/loom/canvas-boards/:block_id/viewport     update_canvas_board_state
//                                                                       body { board_state:{schema_id,pan_x,pan_y,zoom} }
//   - POST   /workspaces/:ws/loom/canvas-boards/:block_id/placements   place_block_on_canvas
//                                                                       body { placed_block_id,x,y,w,h }
//   - POST   /workspaces/:ws/loom/canvas-boards/:block_id/cards        create_canvas_card
//   - POST   /workspaces/:ws/loom/canvas-boards/:block_id/stage-cards/:placement_id/compensate
//                                                                       body { title,body,x,y,w,h }
//   - PATCH  /workspaces/:ws/loom/canvas-placements/:placement_id      update_canvas_placement
//                                                                       body { group_id } (NOT `.../canvas/{cb}/placements/{p}`)
//   - DELETE /workspaces/:ws/loom/canvas-placements/:placement_id      remove_canvas_placement (source block kept)
//   - POST   /workspaces/:ws/loom/edges                                create_loom_edge
//                                                                       body { source_block_id,target_block_id,
//                                                                              edge_type:"mention",created_by:"user" }
//   - POST   /workspaces/:ws/loom/canvas-boards/:block_id/visual-edges add_canvas_visual_edge
//                                                                       body { from_placement_id,to_placement_id }
//   - GET    /workspaces/:ws/loom/blocks/:block_id                     get_loom_block -> LoomBlock (live title resolve)
//
// Placement x/y/w/h are `f64` on the wire (the storage struct), so the request builders take f64.
// All follow the MT-020/021 off-thread shape: spawn on the app's tokio runtime, deliver the outcome
// into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next frame (HBR-QUIET). Speaks
// `serde_json::Value` so it never depends on the `handshake_core` crate's types; the parsed board
// shape is the widget's own `graph::canvas_board::{CanvasPlacementCard, VisualEdge}`.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use crate::graph::canvas_board::{CanvasCardKind, CanvasPlacementCard, VisualEdge};

/// The parsed canvas board: placements + visual edges + viewport (pan/zoom), plus the live-title resolve
/// map keyed by `placed_block_id` (filled by a follow-up `getLoomBlock` per distinct block). `Ok` carries
/// the projection; `Err(msg)` a failure the board surfaces as an error label instead of crashing.
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasBoardData {
    pub placements: Vec<CanvasPlacementCard>,
    pub visual_edges: Vec<VisualEdge>,
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
}

pub const CANVAS_STAGE_CAPTURE_REF_SCHEMA: &str = "handshake.canvas-stage-capture-ref.v1";

/// Structured Stage reference persisted as the Canvas text-card document body. A reload validates this
/// exact schema instead of scraping display text, while the artifact id remains dereferenceable through
/// the authoritative Stage route for exact bytes and manifest verification.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CanvasStageCaptureReference {
    pub schema_id: String,
    pub artifact_id: String,
    pub sha256: String,
    pub manifest_ref: String,
    pub causal_action_id: String,
}

/// The created placement payload returned by the verified canvas creation routes:
/// `POST .../placements` returns a `LoomCanvasPlacement` directly, while `POST .../cards` wraps it under
/// `placement`. This compact DTO carries only the fields the native host needs to register the MT-035
/// compensating undo.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatedCanvasPlacement {
    pub placement_id: String,
    pub placed_block_id: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// True only when this request has an unambiguous successful create receipt. A placement recovered
    /// from a preflight/fresh-board reconciliation is not owned by this request and must not register a
    /// compensating undo that could delete another action's durable placement.
    pub created_by_request: bool,
}

impl CreatedCanvasPlacement {
    pub fn geometry(&self) -> (f64, f64, f64, f64) {
        (self.x, self.y, self.w, self.h)
    }
}

/// Every input that makes a canvas-board projection meaningful. `pane_generation` changes whenever
/// the mounted `(workspace, canvas)` binding changes (including A -> B -> A), while
/// `request_sequence` changes for every refresh of the same binding. Both are required: workspace and
/// canvas ids alone cannot distinguish an old A response after returning to A, and generation alone
/// cannot distinguish overlapping retries for the same board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasBoardRequestIdentity {
    pub workspace_id: String,
    pub canvas_block_id: String,
    pub pane_generation: u64,
    pub request_sequence: u64,
}

impl CanvasBoardRequestIdentity {
    pub fn new(
        workspace_id: impl Into<String>,
        canvas_block_id: impl Into<String>,
        pane_generation: u64,
        request_sequence: u64,
    ) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            canvas_block_id: canvas_block_id.into(),
            pane_generation,
            request_sequence,
        }
    }
}

/// One completed board request. This is deliberately FIFO: a single Option lets an older completion
/// overwrite a newer one before the UI thread can validate either identity.
#[derive(Debug)]
pub struct CanvasBoardDelivery {
    pub request: CanvasBoardRequestIdentity,
    pub result: Result<CanvasBoardData, String>,
}

pub type CanvasBoardCell = Arc<Mutex<VecDeque<CanvasBoardDelivery>>>;

/// One-slot delivery cell for a canvas creation result whose response body carries the created
/// placement id. Non-create mutations still use [`CanvasBoardOpCell`] because their body is not needed.
pub type CanvasBoardCreateCell = Arc<Mutex<Option<Result<CreatedCanvasPlacement, String>>>>;

/// Exact authoritative receipt returned by `POST /loom/edges`. The backend mints `edge_id`; callers
/// must retain this identity and prove that the subsequent graph projection contains this exact edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedSemanticEdge {
    pub edge_id: String,
    pub workspace_id: String,
    pub source_block_id: String,
    pub target_block_id: String,
    pub edge_type: String,
}

pub type SemanticEdgeCreateCell = Arc<Mutex<Option<Result<CreatedSemanticEdge, String>>>>;

/// Typed outcome for a placement-creation receipt whose ownership matters to compensating redo.
/// Only `MalformedSuccess` proves the server accepted this exact POST; transport ambiguity and a known
/// non-success status are never safe grounds for claiming a placement discovered by reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasCreateReceiptError {
    Transport(String),
    Rejected(String),
    MalformedSuccess(String),
}

impl std::fmt::Display for CanvasCreateReceiptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(error) => write!(f, "transport receipt unavailable: {error}"),
            Self::Rejected(error) => write!(f, "placement POST rejected: {error}"),
            Self::MalformedSuccess(error) => write!(
                f,
                "placement POST succeeded with malformed receipt: {error}"
            ),
        }
    }
}

/// One-slot delivery cell for an off-thread canvas MUTATION (place/card/viewport/group/remove/edge)
/// result. `Ok(())` on a 2xx (the board re-fetches), `Err(msg)` the failure. Same shape as
/// [`CanvasOpCell`].
pub type CanvasBoardOpCell = Arc<Mutex<Option<Result<(), String>>>>;

/// The resolved fields a `getLoomBlock` live-resolve carries: `(title, content_type, content_hash)`.
/// `title` is `Option<String>` (a block can be untitled); `content_hash` is `Option<String>`, the
/// backend-computed canonical-JSON SHA-256 the block carries (WP-KERNEL-012 MT-032 — READ-only, the
/// canvas never writes a hash). `None` content_hash means the backend block omitted it (honestly
/// absent, never fabricated).
pub type LiveBlock = (Option<String>, String, Option<String>);

/// A typed live-block lookup failure. Only [`Missing`](Self::Missing) is proof that the reference no
/// longer exists and may unlock the retained-title "Create note from link" recovery. Transport,
/// timeout, server, and decode failures remain unavailable and must not be presented as missing data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveBlockResolveError {
    Missing,
    Unavailable(String),
}

impl std::fmt::Display for LiveBlockResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => write!(f, "block not found"),
            Self::Unavailable(message) => write!(f, "block resolution unavailable: {message}"),
        }
    }
}

/// One-slot delivery cell for an off-thread `getLoomBlock` live-resolve. Delivers
/// `(placed_block_id, Ok(live_block))` / `(placed_block_id, Err(resolve_error))`. Only a confirmed
/// HTTP 404 is `Missing`; transport, timeout, server, and decode failures remain `Unavailable`.
pub type LiveBlockCell = Arc<Mutex<Option<(String, Result<LiveBlock, LiveBlockResolveError>)>>>;

/// REST client for the VERIFIED Loom canvas-board surface (MT-261 backend). Drives the board read +
/// all canvas mutations off the UI thread. Mirrors the `CanvasClient`/`LoomGraphClient` shape exactly.
#[derive(Clone)]
pub struct CanvasBoardClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl CanvasBoardClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// Build over an explicitly owned HTTP pool. Production uses [`Self::new`]
    /// and its one shared pool; integration proofs use this constructor to
    /// model independent native processes with no shared in-memory state.
    pub fn with_http_client(
        base_url: impl Into<String>,
        runtime: tokio::runtime::Handle,
        client: reqwest::Client,
    ) -> Self {
        Self {
            client,
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn board_url(&self, workspace_id: &str, canvas_block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/canvas-boards/{}",
            self.base_url, workspace_id, canvas_block_id
        )
    }

    fn placement_url(&self, workspace_id: &str, placement_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/canvas-placements/{}",
            self.base_url, workspace_id, placement_id
        )
    }

    fn edges_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/edges", self.base_url, workspace_id)
    }

    fn block_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/blocks/{}",
            self.base_url, workspace_id, block_id
        )
    }

    /// Pure request builder for `GET .../canvas-boards/:block_id` (getCanvasBoard).
    pub fn get_board_request(&self, workspace_id: &str, canvas_block_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.board_url(workspace_id, canvas_block_id),
            query: vec![],
        }
    }

    /// Pure request builder for `PUT .../canvas-boards/:block_id/viewport` (updateCanvasBoardViewport).
    /// The verified body is `{ board_state:{schema_id,pan_x,pan_y,zoom} }` — NOT a top-level
    /// `{pan_x,pan_y,zoom}` (the contract's stale shape).
    pub fn viewport_request(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
        pan_x: f32,
        pan_y: f32,
        zoom: f32,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Put,
            url: format!("{}/viewport", self.board_url(workspace_id, canvas_block_id)),
            body: Some(serde_json::json!({
                "board_state": {
                    "schema_id": LOOM_CANVAS_BOARD_SCHEMA_ID,
                    "pan_x": pan_x,
                    "pan_y": pan_y,
                    "zoom": zoom,
                }
            })),
        }
    }

    /// Pure request builder for `POST .../canvas-boards/:block_id/placements` (placeBlockOnCanvas).
    #[allow(clippy::too_many_arguments)] // x/y/w/h geometry + ids — the verified placement body shape.
    pub fn place_block_request(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
        placed_block_id: &str,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!(
                "{}/placements",
                self.board_url(workspace_id, canvas_block_id)
            ),
            body: Some(serde_json::json!({
                "placed_block_id": placed_block_id,
                "x": x, "y": y, "w": w, "h": h,
            })),
        }
    }

    /// Pure request builder for `POST .../canvas-boards/:block_id/cards` (createCanvasCard).
    #[allow(clippy::too_many_arguments)] // x/y/w/h geometry + title + ids — the verified card body shape.
    pub fn create_card_request(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
        title: &str,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!("{}/cards", self.board_url(workspace_id, canvas_block_id)),
            body: Some(serde_json::json!({
                "title": title,
                "body": "",
                "x": x, "y": y, "w": w, "h": h,
            })),
        }
    }

    /// Pure request builder for a Stage provenance card on a canvas. This reuses the canonical cards
    /// route while retaining the evidence tuple in the persisted markdown body.
    #[allow(clippy::too_many_arguments)]
    pub fn create_stage_capture_card_request(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
        artifact_id: &str,
        sha256: &str,
        manifest_ref: &str,
        causal_action_id: &str,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> RequestSpec {
        let reference = CanvasStageCaptureReference {
            schema_id: CANVAS_STAGE_CAPTURE_REF_SCHEMA.to_owned(),
            artifact_id: artifact_id.to_owned(),
            sha256: sha256.to_owned(),
            manifest_ref: manifest_ref.to_owned(),
            causal_action_id: causal_action_id.to_owned(),
        };
        RequestSpec {
            method: HttpMethod::Post,
            url: format!("{}/cards", self.board_url(workspace_id, canvas_block_id)),
            body: Some(serde_json::json!({
                "title": format!("Stage capture {artifact_id}"),
                "body": serde_json::to_string(&reference)
                    .expect("Canvas Stage reference serialization is infallible"),
                "stage_provenance": reference,
                "x": x, "y": y, "w": w, "h": h,
            })),
        }
    }

    /// Exact owned-card compensation request. The backend re-verifies this
    /// placement/block/provenance tuple under the create advisory lock before
    /// removing any authority row.
    #[allow(clippy::too_many_arguments)]
    pub fn compensate_stage_capture_card_request(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
        placement_id: &str,
        placed_block_id: &str,
        artifact_id: &str,
        sha256: &str,
        manifest_ref: &str,
        causal_action_id: &str,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!(
                "{}/stage-cards/{placement_id}/compensate",
                self.board_url(workspace_id, canvas_block_id)
            ),
            body: Some(serde_json::json!({
                "placed_block_id": placed_block_id,
                "stage_provenance": CanvasStageCaptureReference {
                    schema_id: CANVAS_STAGE_CAPTURE_REF_SCHEMA.to_owned(),
                    artifact_id: artifact_id.to_owned(),
                    sha256: sha256.to_owned(),
                    manifest_ref: manifest_ref.to_owned(),
                    causal_action_id: causal_action_id.to_owned(),
                }
            })),
        }
    }

    /// Pure request builder for `PATCH .../canvas-placements/:placement_id` (updateCanvasPlacement)
    /// with a `group_id` (grouping). The verified body uses `group_id`.
    pub fn group_request(
        &self,
        workspace_id: &str,
        placement_id: &str,
        group_id: &str,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.placement_url(workspace_id, placement_id),
            body: Some(serde_json::json!({ "group_id": group_id })),
        }
    }

    /// Persist a completed canvas card drag in one placement PATCH. The backend accepts geometry and
    /// grouping in the same `UpdatePlacementRequest`; keeping them atomic prevents a refresh between
    /// two requests from exposing a half-applied move. `None` uses the backend's explicit
    /// `clear_group` flag because a JSON null group is intentionally a no-op there.
    pub fn move_request(
        &self,
        workspace_id: &str,
        placement_id: &str,
        x: f64,
        y: f64,
        group_id: Option<&str>,
    ) -> RequestSpec {
        let body = match group_id {
            Some(group_id) => serde_json::json!({
                "x": x,
                "y": y,
                "group_id": group_id,
            }),
            None => serde_json::json!({
                "x": x,
                "y": y,
                "clear_group": true,
            }),
        };
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.placement_url(workspace_id, placement_id),
            body: Some(body),
        }
    }

    /// WP-KERNEL-012 MT-080 (AC-080-2 / MT-061): pure request builder for
    /// `PATCH .../canvas-placements/:placement_id` (updateCanvasPlacement) with the new card `{w, h}`. The
    /// canvas `CanvasEvent::ResizePlacement { placement_id, w, h }` fires ONCE on resize drag-stop
    /// (debounced in the widget); the host maps it to this builder, sends it via [`dispatch`](Self::dispatch),
    /// then re-fetches the board so the persisted geometry replaces the optimistic in-flight size. Same
    /// placement URL + PATCH verb as [`group_request`](Self::group_request); only the body fields differ
    /// (`w`/`h` are the verified placement geometry fields — see [`placement_from_json`]).
    pub fn resize_request(
        &self,
        workspace_id: &str,
        placement_id: &str,
        w: f64,
        h: f64,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.placement_url(workspace_id, placement_id),
            body: Some(serde_json::json!({ "w": w, "h": h })),
        }
    }

    /// WP-KERNEL-012 MT-080 (AC-080-2 / MT-061): pure request builder for
    /// `PATCH .../canvas-placements/:placement_id` clearing the `group_id` (a card dropped OUTSIDE all
    /// section frames). Explicit section-assignment actions use this builder; completed move gestures use
    /// [`move_request`](Self::move_request) so their coordinates and cleared section persist atomically.
    ///
    /// Backend-shape note (verified against `update_canvas_placement` /
    /// `UpdatePlacementRequest` in `src/backend/handshake_core/src/api/loom.rs`): the handler clears the
    /// group ONLY when the separate boolean `clear_group: true` is present
    /// (`let group_id = if payload.clear_group { Some(None) } else { payload.group_id.map(Some) };`). A
    /// `{"group_id": null}` body deserializes to `group_id: None` (serde default) and the storage layer's
    /// `CASE WHEN $8 ...` (with `$8 = update.group_id.is_some() = false`) leaves the group UNCHANGED — i.e.
    /// `{"group_id": null}` is a silent no-op and the card re-snaps into its old section on the next board
    /// refresh. Sending `{"clear_group": true}` is the only body that actually clears the assignment. This
    /// matches the shape the widget already documents at `graph/canvas_board.rs` (`{clear_group:true}`).
    pub fn clear_group_request(&self, workspace_id: &str, placement_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.placement_url(workspace_id, placement_id),
            body: Some(serde_json::json!({ "clear_group": true })),
        }
    }

    /// Pure request builder for `DELETE .../canvas-placements/:placement_id` (removeCanvasPlacement).
    /// Removes the placement REFERENCE; the source block is KEPT (MC-4).
    pub fn remove_placement_request(&self, workspace_id: &str, placement_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Delete,
            url: self.placement_url(workspace_id, placement_id),
            body: None,
        }
    }

    /// Pure request builder for `POST /loom/edges` (createLoomEdge) — a real semantic `mention` edge.
    pub fn semantic_edge_request(
        &self,
        workspace_id: &str,
        source_block_id: &str,
        target_block_id: &str,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: self.edges_url(workspace_id),
            body: Some(serde_json::json!({
                "source_block_id": source_block_id,
                "target_block_id": target_block_id,
                "edge_type": "mention",
                "created_by": "user",
            })),
        }
    }

    /// Pure request builder for `DELETE /loom/edges/:edge_id` (deleteLoomEdge) — removes a semantic
    /// Loom edge by id (WP-KERNEL-012 MT-042/021 REMEDIATION: the `GraphEvent::RemoveEdge` host route;
    /// the VERIFIED backend twin is `delete_loom_edge` in `handshake_core` `api/loom.rs`).
    pub fn remove_semantic_edge_request(&self, workspace_id: &str, edge_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Delete,
            url: format!("{}/{}", self.edges_url(workspace_id), edge_id),
            body: None,
        }
    }

    /// Pure request builder for `DELETE /loom/canvas-visual-edges/:visual_edge_id`
    /// (removeCanvasVisualEdge) — removes a BOARD-LOCAL visual edge by id (WP-KERNEL-012 W3 / MT-026
    /// remediation: the `CanvasEvent::RemoveEdge` host route for an edge id the board's `visual_edges`
    /// projection owns; the VERIFIED backend twin is `remove_canvas_visual_edge` in `handshake_core`
    /// `api/loom.rs`, route `/workspaces/:ws/loom/canvas-visual-edges/:visual_edge_id`). A RemoveEdge
    /// whose id is NOT a board visual edge routes to
    /// [`remove_semantic_edge_request`](Self::remove_semantic_edge_request) instead.
    pub fn remove_visual_edge_request(
        &self,
        workspace_id: &str,
        visual_edge_id: &str,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Delete,
            url: format!(
                "{}/workspaces/{}/loom/canvas-visual-edges/{}",
                self.base_url, workspace_id, visual_edge_id
            ),
            body: None,
        }
    }

    /// Pure request builder for `POST .../canvas-boards/:block_id/visual-edges` (addCanvasVisualEdge).
    pub fn visual_edge_request(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
        from_placement_id: &str,
        to_placement_id: &str,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!(
                "{}/visual-edges",
                self.board_url(workspace_id, canvas_block_id)
            ),
            body: Some(serde_json::json!({
                "from_placement_id": from_placement_id,
                "to_placement_id": to_placement_id,
            })),
        }
    }

    /// Pure request builder for `GET /loom/blocks/:block_id` (getLoomBlock) — live-title resolve.
    pub fn get_block_request(&self, workspace_id: &str, block_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.block_url(workspace_id, block_id),
            query: vec![],
        }
    }

    /// Compatibility fetch for direct client consumers. Mounted hosts must use
    /// [`Self::fetch_board_with_identity`] so stale deliveries cannot cross pane bindings.
    pub fn fetch_board(&self, workspace_id: &str, canvas_block_id: &str, cell: CanvasBoardCell) {
        self.fetch_board_with_identity(
            CanvasBoardRequestIdentity::new(workspace_id, canvas_block_id, 0, 0),
            cell,
        );
    }

    /// Await one canonical board read inside an already-off-thread workflow. Canvas compensating
    /// undo/redo uses this after an ambiguous transport receipt to determine whether the mutation
    /// actually committed before it finalizes the provisional undo-ring transition.
    pub async fn fetch_board_now(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
    ) -> Result<CanvasBoardData, String> {
        fetch_canvas_board(&self.client, &self.board_url(workspace_id, canvas_block_id))
            .await
            .map_err(|error| error.to_string())
    }

    /// Find an already-persisted Stage capture card by its complete provenance tuple. This fresh-read
    /// reconciliation makes a repeated embed converge on the original Canvas placement instead of
    /// minting a second text-card block after a lost response or operator retry.
    pub async fn find_stage_capture_card_now(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
        artifact_id: &str,
        sha256: &str,
        manifest_ref: &str,
        causal_action_id: &str,
    ) -> Result<Option<CreatedCanvasPlacement>, String> {
        let board = self.fetch_board_now(workspace_id, canvas_block_id).await?;
        let expected_title = format!("Stage capture {artifact_id}");
        let expected_reference = CanvasStageCaptureReference {
            schema_id: CANVAS_STAGE_CAPTURE_REF_SCHEMA.to_owned(),
            artifact_id: artifact_id.to_owned(),
            sha256: sha256.to_owned(),
            manifest_ref: manifest_ref.to_owned(),
            causal_action_id: causal_action_id.to_owned(),
        };

        for card in board.placements {
            let block_url = self.block_url(workspace_id, &card.placed_block_id);
            let block = get_json(&self.client, &block_url, &[])
                .await
                .map_err(|error| error.to_string())?;
            if block.get("title").and_then(serde_json::Value::as_str)
                != Some(expected_title.as_str())
            {
                continue;
            }
            let document_id = block
                .get("document_id")
                .and_then(serde_json::Value::as_str)
                .filter(|id| !id.trim().is_empty())
                .or_else(|| {
                    // Imported Markdown notes use the RichDocument's id as the same-id Loom
                    // projection block id; their Loom `document_id` field is intentionally null.
                    // The canonical document route remains the authority for content readback.
                    block
                        .get("block_id")
                        .and_then(serde_json::Value::as_str)
                        .filter(|id| !id.trim().is_empty())
                })
                .ok_or_else(|| {
                    "Stage capture Loom block has neither document_id nor same-id block_id"
                        .to_owned()
                })?;
            let document_client =
                crate::backend::knowledge_documents::KnowledgeDocumentsClient::with_client(
                    self.client.clone(),
                    self.base_url.clone(),
                );
            let headers = crate::backend::knowledge_documents::HskDocumentHeaders::for_read(
                format!("stage-canvas-reconcile-{canvas_block_id}"),
                document_id,
            );
            let document = document_client
                .load_document(&headers, document_id)
                .await
                .map_err(|error| error.to_string())?;
            if find_canvas_stage_capture_reference(&document.document).as_ref()
                == Some(&expected_reference)
            {
                return Ok(Some(CreatedCanvasPlacement {
                    placement_id: card.placement_id,
                    placed_block_id: card.placed_block_id,
                    x: card.x as f64,
                    y: card.y as f64,
                    w: card.w as f64,
                    h: card.h as f64,
                    created_by_request: false,
                }));
            }
        }
        Ok(None)
    }

    /// Converge one Stage capture tuple onto exactly one canonical Canvas placement. The card POST
    /// carries the structured provenance separately from its persisted body; handshake_core validates
    /// both representations and holds a PostgreSQL transaction advisory lock through preflight and
    /// creation. Independent native processes therefore cannot both pass a read-before-create window.
    /// A transport-lost success is still reconciled from canonical board/document state before failure
    /// is exposed.
    #[allow(clippy::too_many_arguments)]
    pub async fn ensure_stage_capture_card_now(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
        artifact_id: &str,
        sha256: &str,
        manifest_ref: &str,
        causal_action_id: &str,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> Result<CreatedCanvasPlacement, String> {
        if let Some(existing) = self
            .find_stage_capture_card_now(
                workspace_id,
                canvas_block_id,
                artifact_id,
                sha256,
                manifest_ref,
                causal_action_id,
            )
            .await?
        {
            return Ok(existing);
        }
        let spec = self.create_stage_capture_card_request(
            workspace_id,
            canvas_block_id,
            artifact_id,
            sha256,
            manifest_ref,
            causal_action_id,
            x,
            y,
            w,
            h,
        );
        match self.create_placement_now(spec).await {
            Ok(created) => Ok(created),
            Err(create_error) => match self
                .find_stage_capture_card_now(
                    workspace_id,
                    canvas_block_id,
                    artifact_id,
                    sha256,
                    manifest_ref,
                    causal_action_id,
                )
                .await
            {
                Ok(Some(existing)) => Ok(existing),
                Ok(None) => Err(create_error.to_string()),
                Err(reconcile_error) => Err(format!(
                    "{create_error}; post-create reconciliation also failed: {reconcile_error}"
                )),
            },
        }
    }

    /// Atomically remove a Stage-created Canvas card after the host discovers
    /// the embed target is gone. One retry is intentional: a lost successful
    /// response reaches the backend's idempotent all-absent reconciliation.
    #[allow(clippy::too_many_arguments)]
    pub async fn compensate_stage_capture_card_now(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
        created: &CreatedCanvasPlacement,
        artifact_id: &str,
        sha256: &str,
        manifest_ref: &str,
        causal_action_id: &str,
    ) -> Result<bool, String> {
        if !created.created_by_request {
            return Err(
                "Stage Canvas compensation denied: this request did not create the placement"
                    .to_owned(),
            );
        }
        let spec = self.compensate_stage_capture_card_request(
            workspace_id,
            canvas_block_id,
            &created.placement_id,
            &created.placed_block_id,
            artifact_id,
            sha256,
            manifest_ref,
            causal_action_id,
        );
        let body = spec
            .body
            .as_ref()
            .expect("compensation request always has a body");
        let mut first_ambiguous_error = None;
        for attempt in 0..2 {
            match self.client.post(&spec.url).json(body).send().await {
                Ok(response) => {
                    let status = response.status();
                    if !status.is_success() {
                        return Err(format!(
                            "Stage Canvas compensation rejected with status {status}"
                        ));
                    }
                    let value = match response.json::<serde_json::Value>().await {
                        Ok(value) => value,
                        Err(error) if attempt == 0 => {
                            first_ambiguous_error =
                                Some(format!("successful response decode failed: {error}"));
                            continue;
                        }
                        Err(error) => {
                            return Err(format!(
                                "Stage Canvas compensation response decode failed after retry: {error}"
                            ));
                        }
                    };
                    let removed = value
                        .get("removed_by_request")
                        .and_then(serde_json::Value::as_bool);
                    if let Some(removed) = removed {
                        return Ok(removed);
                    }
                    if attempt == 0 {
                        first_ambiguous_error =
                            Some("successful response missing removed_by_request".to_owned());
                        continue;
                    }
                    return Err(
                        "Stage Canvas compensation response missing removed_by_request after retry"
                            .to_owned(),
                    );
                }
                Err(error) if attempt == 0 => {
                    first_ambiguous_error = Some(format!("transport failed: {error}"))
                }
                Err(error) => {
                    return Err(format!(
                        "Stage Canvas compensation failed twice (first: {}; retry transport: {error})",
                        first_ambiguous_error.as_deref().unwrap_or("unknown")
                    ));
                }
            }
        }
        unreachable!("bounded compensation retry loop always returns")
    }

    /// Remove a placement synchronously inside an already-off-thread recovery workflow.
    pub async fn remove_placement_now(
        &self,
        workspace_id: &str,
        placement_id: &str,
    ) -> Result<(), String> {
        let spec = self.remove_placement_request(workspace_id, placement_id);
        send_canvas_mutation(&self.client, &spec)
            .await
            .map_err(|error| error.to_string())
    }

    /// Fetch the board off the UI thread, preserving the exact host identity in the FIFO delivery.
    pub fn fetch_board_with_identity(
        &self,
        request: CanvasBoardRequestIdentity,
        cell: CanvasBoardCell,
    ) {
        let spec = self.get_board_request(&request.workspace_id, &request.canvas_block_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_canvas_board(&client, &spec.url).await;
            if let Ok(mut slot) = cell.lock() {
                slot.push_back(CanvasBoardDelivery {
                    request,
                    result: result.map_err(|e| e.to_string()),
                });
            }
        });
    }

    /// Resolve a block's live title + content_type off the UI thread (`getLoomBlock`), delivering
    /// `(placed_block_id, Ok((title, content_type)))` / `(placed_block_id, Err))` into `cell`.
    pub fn resolve_block(&self, workspace_id: &str, placed_block_id: &str, cell: LiveBlockCell) {
        let spec = self.get_block_request(workspace_id, placed_block_id);
        let client = self.client.clone();
        let id = placed_block_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_live_block(&client, &spec.url).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((id, result));
            }
        });
    }

    /// Send a prebuilt mutation [`RequestSpec`] (place/card/viewport/group/remove/edge) off the UI
    /// thread, delivering `Ok(())`/`Err(msg)` into `cell`. The host re-fetches the board after a 2xx.
    pub fn dispatch(&self, spec: RequestSpec, cell: CanvasBoardOpCell) {
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = send_canvas_mutation(&client, &spec).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Send a placement/card creation request off-thread and deliver the created placement payload.
    /// This is deliberately separate from [`dispatch`](Self::dispatch): only creation routes need the
    /// response body so the shell can register a precise compensating undo for the backend-minted id.
    pub fn dispatch_created_placement(&self, spec: RequestSpec, cell: CanvasBoardCreateCell) {
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = send_canvas_created_placement(&client, &spec).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Send a semantic-edge POST and retain the backend-minted edge identity from its response body.
    pub fn dispatch_created_semantic_edge(&self, spec: RequestSpec, cell: SemanticEdgeCreateCell) {
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = send_created_semantic_edge(&client, &spec).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|error| error.to_string()));
            }
        });
    }

    /// Await a placement creation while preserving whether failure happened before a response, at a
    /// known non-2xx response, or while decoding a successful receipt. Compensating redo uses this
    /// distinction to avoid claiming another actor's same-block placement after a conflict.
    pub async fn create_placement_now(
        &self,
        spec: RequestSpec,
    ) -> Result<CreatedCanvasPlacement, CanvasCreateReceiptError> {
        if spec.method != HttpMethod::Post {
            return Err(CanvasCreateReceiptError::Rejected(
                "created-placement dispatch only supports POST".to_owned(),
            ));
        }
        let empty = serde_json::json!({});
        let body = spec.body.as_ref().unwrap_or(&empty);
        let response = self
            .client
            .post(&spec.url)
            .timeout(Duration::from_secs(5))
            .json(body)
            .send()
            .await
            .map_err(|error| CanvasCreateReceiptError::Transport(error.to_string()))?;
        if !response.status().is_success() {
            return Err(CanvasCreateReceiptError::Rejected(format!(
                "POST non-success status {}",
                response.status()
            )));
        }
        let value = response
            .json::<serde_json::Value>()
            .await
            .map_err(|error| CanvasCreateReceiptError::MalformedSuccess(error.to_string()))?;
        let created = created_canvas_placement_from_response(&value)
            .map_err(|error| CanvasCreateReceiptError::MalformedSuccess(error.to_string()))?;
        validate_created_placement_receipt(&spec.url, body, &value, &created)
            .map_err(CanvasCreateReceiptError::MalformedSuccess)?;
        Ok(created)
    }

    /// Resolve an Atelier intake item through the canonical Loom relation returned by the Atelier API,
    /// then place that real block reference on a canvas. Missing relation identity fails closed; this
    /// client never guesses a block id or creates a synthetic projection. Placement retries reconcile
    /// against the durable canvas uniqueness constraint, so no unsupported Atelier field is sent to the
    /// Loom placement route.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_atelier_and_place(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
        atelier_ref: &crate::interop::AtelierRef,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        cell: CanvasBoardCreateCell,
    ) {
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let workspace_id = workspace_id.to_owned();
        let canvas_block_id = canvas_block_id.to_owned();
        let atelier_ref = atelier_ref.clone();
        self.runtime.spawn(async move {
            let result = resolve_atelier_projection_and_place(
                &client,
                &base_url,
                &workspace_id,
                &canvas_block_id,
                &atelier_ref,
                x,
                y,
                w,
                h,
            )
            .await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|error| error.to_string()));
            }
        });
    }
}

fn find_canvas_stage_capture_reference(
    value: &serde_json::Value,
) -> Option<CanvasStageCaptureReference> {
    match value {
        serde_json::Value::String(text) => {
            serde_json::from_str::<CanvasStageCaptureReference>(text).ok()
        }
        serde_json::Value::Array(values) => {
            values.iter().find_map(find_canvas_stage_capture_reference)
        }
        serde_json::Value::Object(values) => values
            .values()
            .find_map(find_canvas_stage_capture_reference),
        _ => None,
    }
}

pub(crate) fn validate_created_placement_receipt(
    request_url: &str,
    request_body: &serde_json::Value,
    response: &serde_json::Value,
    created: &CreatedCanvasPlacement,
) -> Result<(), String> {
    if created.placement_id.trim().is_empty() {
        return Err("creation receipt has an empty placement_id".to_owned());
    }
    let expected_block = request_body
        .get("placed_block_id")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            response
                .get("block")
                .and_then(|block| block.get("block_id"))
                .and_then(serde_json::Value::as_str)
        })
        .ok_or_else(|| "creation receipt cannot prove the requested block identity".to_owned())?;
    if created.placed_block_id != expected_block {
        return Err(format!(
            "creation receipt placed_block_id mismatch: expected {expected_block}, got {}",
            created.placed_block_id
        ));
    }
    let scope = request_url
        .split("/workspaces/")
        .nth(1)
        .map(|tail| tail.split('/').collect::<Vec<_>>())
        .filter(|parts| parts.len() >= 4 && parts[1] == "loom" && parts[2] == "canvas-boards")
        .ok_or_else(|| {
            "placement request URL has no canonical workspace/canvas scope".to_owned()
        })?;
    let placement = response.get("placement").unwrap_or(response);
    for (field, expected) in [("workspace_id", scope[0]), ("canvas_block_id", scope[3])] {
        let actual = placement
            .get(field)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("creation receipt missing {field}"))?;
        if actual != expected {
            return Err(format!(
                "creation receipt {field} mismatch: expected {expected}, got {actual}"
            ));
        }
    }
    if created.created_by_request {
        for (name, actual) in [
            ("x", created.x),
            ("y", created.y),
            ("w", created.w),
            ("h", created.h),
        ] {
            let expected = request_body
                .get(name)
                .and_then(serde_json::Value::as_f64)
                .ok_or_else(|| format!("placement request missing {name}"))?;
            if (actual - expected).abs() > 0.000_001 {
                return Err(format!(
                    "creation receipt {name} mismatch: expected {expected}, got {actual}"
                ));
            }
        }
    }
    Ok(())
}

fn canonical_atelier_projection_block_id(
    atelier_ref: &crate::interop::AtelierRef,
) -> Result<&str, AppError> {
    atelier_ref
        .loom_block_id
        .as_deref()
        .filter(|block_id| !block_id.trim().is_empty())
        .ok_or_else(|| {
            AppError::Parse(format!(
                "Atelier item {} has no canonical Loom projection relation; Canvas placement is unavailable until Atelier publishes loom_block_id",
                atelier_ref.item_id
            ))
        })
}

#[allow(clippy::too_many_arguments)]
async fn resolve_atelier_projection_and_place(
    client: &reqwest::Client,
    base_url: &str,
    workspace_id: &str,
    canvas_block_id: &str,
    atelier_ref: &crate::interop::AtelierRef,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<CreatedCanvasPlacement, AppError> {
    // The Atelier API owns this durable relation. Never derive an identity from workspace/item strings:
    // doing so can make a frontend-only id look canonical and place the wrong or nonexistent block.
    let block_id = canonical_atelier_projection_block_id(atelier_ref)?.to_owned();
    let block_url = format!("{base_url}/workspaces/{workspace_id}/loom/blocks/{block_id}");
    let get_response = client
        .get(&block_url)
        .send()
        .await
        .map_err(|error| AppError::Http(error.to_string()))?;
    let get_status = get_response.status();

    if get_status == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::Parse(format!(
            "Atelier item {} has no canonical Loom projection with a source document/asset; Canvas placement is unsupported until Atelier publishes that durable relation",
            atelier_ref.item_id
        )));
    } else if !get_status.is_success() {
        return Err(AppError::Http(format!(
            "Atelier Loom projection GET non-success status {get_status}"
        )));
    } else {
        verify_atelier_projection_response(get_response, &block_id, atelier_ref.item_kind).await?;
    }

    let board_url =
        format!("{base_url}/workspaces/{workspace_id}/loom/canvas-boards/{canvas_block_id}");
    if let Some(existing) = find_reconciled_canvas_placement(client, &board_url, &block_id).await? {
        return Ok(existing);
    }

    let placement_url = format!("{board_url}/placements");
    let run_id = uuid::Uuid::new_v4().to_string();
    let response = client
        .post(placement_url)
        .header(HSK_HEADER_ACTOR_ID, "handshake-native-atelier-drop")
        .header(HSK_HEADER_ACTOR_KIND, "operator")
        .header(HSK_HEADER_KERNEL_TASK_RUN_ID, &run_id)
        .header(HSK_HEADER_SESSION_RUN_ID, &run_id)
        .json(&serde_json::json!({
            "placed_block_id": block_id,
            "x": x,
            "y": y,
            "w": w,
            "h": h,
        }))
        .send()
        .await;
    let unambiguous_create = response
        .as_ref()
        .map(|response| response.status().is_success())
        .unwrap_or(false);

    // The backend has a durable UNIQUE(canvas_block_id, placed_block_id) constraint. Always reconcile
    // from a fresh board after the POST, including conflict, transport loss, or a committed 2xx whose
    // receipt body is malformed. This makes a retry converge on the one canonical placement.
    if let Some(mut reconciled) =
        find_reconciled_canvas_placement(client, &board_url, &block_id).await?
    {
        reconciled.created_by_request = unambiguous_create;
        return Ok(reconciled);
    }
    match response {
        Ok(response) => Err(AppError::Http(format!(
            "Atelier canvas placement did not appear in fresh board after POST status {}",
            response.status()
        ))),
        Err(error) => Err(AppError::Http(format!(
            "Atelier canvas placement receipt was lost and fresh-board reconciliation found no placement: {error}"
        ))),
    }
}

async fn verify_atelier_projection_response(
    response: reqwest::Response,
    expected_block_id: &str,
    item_kind: crate::interop::AtelierItemKind,
) -> Result<(), AppError> {
    let value = response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| AppError::Parse(error.to_string()))?;
    let actual_id = value
        .get("block_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AppError::Parse("Atelier Loom projection missing block_id".to_owned()))?;
    let content_type = value
        .get("content_type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::Parse("Atelier Loom projection missing content_type".to_owned())
        })?;
    let has_source = value
        .get("document_id")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|id| !id.trim().is_empty())
        || value
            .get("asset_id")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|id| !id.trim().is_empty());
    let expected_content_type = match item_kind {
        crate::interop::AtelierItemKind::Media => "file",
        crate::interop::AtelierItemKind::Character => "ckc_character",
        crate::interop::AtelierItemKind::Moodboard => "ckc_moodboard",
    };
    if actual_id != expected_block_id || content_type != expected_content_type || !has_source {
        return Err(AppError::Parse(format!(
            "Atelier Loom projection identity/source mismatch: expected {expected_block_id}/{expected_content_type} with document_id or asset_id, got {actual_id}/{content_type}"
        )));
    }
    Ok(())
}

async fn find_reconciled_canvas_placement(
    client: &reqwest::Client,
    board_url: &str,
    placed_block_id: &str,
) -> Result<Option<CreatedCanvasPlacement>, AppError> {
    let value = get_json(client, board_url, &[]).await?;
    let Some(placements) = value
        .get("placements")
        .and_then(serde_json::Value::as_array)
    else {
        return Err(AppError::Parse(
            "canvas board reconciliation response missing placements".to_owned(),
        ));
    };
    for placement in placements {
        if placement
            .get("placed_block_id")
            .and_then(serde_json::Value::as_str)
            == Some(placed_block_id)
        {
            let mut created = created_canvas_placement_from_response(placement)?;
            created.created_by_request = false;
            return Ok(Some(created));
        }
    }
    Ok(None)
}

/// The board-state schema id the backend stamps on the canvas viewport JSONB (mirrors
/// `handshake_core::storage::LOOM_CANVAS_BOARD_SCHEMA_ID`). Kept as a const here so the native client
/// never depends on the backend crate.
pub const LOOM_CANVAS_BOARD_SCHEMA_ID: &str = "hsk.loom_canvas_board@1";

/// Send one canvas mutation by method, treating any 2xx as success (the board re-fetches for the body).
async fn send_canvas_mutation(
    client: &reqwest::Client,
    spec: &RequestSpec,
) -> Result<(), AppError> {
    let empty = serde_json::json!({});
    let body = spec.body.as_ref().unwrap_or(&empty);
    match spec.method {
        HttpMethod::Post => post_expect_success(client, &spec.url, body).await,
        HttpMethod::Patch => patch_expect_success(client, &spec.url, body).await,
        HttpMethod::Put => put_expect_success(client, &spec.url, body).await,
        HttpMethod::Delete => delete_expect_success(client, &spec.url).await,
        HttpMethod::Get => Err(AppError::Http("GET is not a mutation".to_owned())),
    }
}

async fn send_canvas_created_placement(
    client: &reqwest::Client,
    spec: &RequestSpec,
) -> Result<CreatedCanvasPlacement, AppError> {
    if spec.method != HttpMethod::Post {
        return Err(AppError::Http(
            "created-placement dispatch only supports POST".to_owned(),
        ));
    }
    let empty = serde_json::json!({});
    let body = spec.body.as_ref().unwrap_or(&empty);
    let value = post_json_expect_value(client, &spec.url, body, Duration::from_secs(5)).await?;
    let created = created_canvas_placement_from_response(&value)?;
    validate_created_placement_receipt(&spec.url, body, &value, &created)
        .map_err(AppError::Parse)?;
    Ok(created)
}

async fn send_created_semantic_edge(
    client: &reqwest::Client,
    spec: &RequestSpec,
) -> Result<CreatedSemanticEdge, AppError> {
    if spec.method != HttpMethod::Post {
        return Err(AppError::Http(
            "created-semantic-edge dispatch only supports POST".to_owned(),
        ));
    }
    let body = spec
        .body
        .as_ref()
        .ok_or_else(|| AppError::Parse("semantic-edge POST body is missing".to_owned()))?;
    let value = post_json_expect_value(client, &spec.url, body, Duration::from_secs(5)).await?;
    created_semantic_edge_from_response(spec, &value)
}

fn created_semantic_edge_from_response(
    spec: &RequestSpec,
    value: &serde_json::Value,
) -> Result<CreatedSemanticEdge, AppError> {
    if spec.method != HttpMethod::Post {
        return Err(AppError::Http(
            "created-semantic-edge dispatch only supports POST".to_owned(),
        ));
    }
    let body = spec
        .body
        .as_ref()
        .ok_or_else(|| AppError::Parse("semantic-edge POST body is missing".to_owned()))?;
    let required = |source: &serde_json::Value, field: &str| {
        source
            .get(field)
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| AppError::Parse(format!("semantic-edge receipt missing {field}")))
    };
    let created = CreatedSemanticEdge {
        edge_id: required(&value, "edge_id")?,
        workspace_id: required(&value, "workspace_id")?,
        source_block_id: required(&value, "source_block_id")?,
        target_block_id: required(&value, "target_block_id")?,
        edge_type: required(&value, "edge_type")?,
    };
    let workspace = spec
        .url
        .split("/workspaces/")
        .nth(1)
        .map(|tail| tail.split('/').collect::<Vec<_>>())
        .filter(|parts| {
            parts.len() == 3 && !parts[0].is_empty() && parts[1] == "loom" && parts[2] == "edges"
        })
        .map(|parts| parts[0])
        .ok_or_else(|| {
            AppError::Parse(
                "semantic-edge URL is not the canonical /workspaces/:id/loom/edges route"
                    .to_owned(),
            )
        })?;
    let expected_source = required(body, "source_block_id")?;
    let expected_target = required(body, "target_block_id")?;
    let expected_edge_type = required(body, "edge_type")?;
    for (field, expected, actual) in [
        ("workspace_id", workspace, created.workspace_id.as_str()),
        (
            "source_block_id",
            expected_source.as_str(),
            created.source_block_id.as_str(),
        ),
        (
            "target_block_id",
            expected_target.as_str(),
            created.target_block_id.as_str(),
        ),
        (
            "edge_type",
            expected_edge_type.as_str(),
            created.edge_type.as_str(),
        ),
    ] {
        if actual != expected {
            return Err(AppError::Parse(format!(
                "semantic-edge receipt {field} mismatch: expected {expected}, got {actual}"
            )));
        }
    }
    Ok(created)
}

/// `GET {url}` and parse the verified `LoomCanvasBoardView` into a [`CanvasBoardData`]. Placements
/// arrive WITHOUT live titles (the host resolves each via `getLoomBlock` after this returns — reference,
/// not copy). A valid empty board has empty required arrays; malformed successful responses fail closed.
async fn fetch_canvas_board(
    client: &reqwest::Client,
    url: &str,
) -> Result<CanvasBoardData, AppError> {
    let v = get_json(client, url, &[]).await?;
    let placements_value = v
        .get("placements")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AppError::Parse("canvas board placements must be an array".to_owned()))?;
    let placements = placements_value
        .iter()
        .enumerate()
        .map(|(index, row)| strict_canvas_placement(row, index))
        .collect::<Result<Vec<_>, _>>()?;
    let visual_edges_value = v
        .get("visual_edges")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AppError::Parse("canvas board visual_edges must be an array".to_owned()))?;
    let visual_edges = visual_edges_value
        .iter()
        .enumerate()
        .map(|(index, row)| {
            visual_edge_from_json(row).ok_or_else(|| {
                AppError::Parse(format!(
                    "canvas board visual_edges[{index}] is missing a required id"
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let board_state = v
        .get("board")
        .and_then(|board| board.get("board_state"))
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| AppError::Parse("canvas board.board_state must be an object".to_owned()))?;
    let schema_id = board_state
        .get("schema_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::Parse("canvas board_state.schema_id must be a string".to_owned())
        })?;
    if schema_id != LOOM_CANVAS_BOARD_SCHEMA_ID {
        return Err(AppError::Parse(format!(
            "canvas board_state.schema_id must be {LOOM_CANVAS_BOARD_SCHEMA_ID}, got {schema_id}"
        )));
    }
    let required_finite = |field: &str| -> Result<f32, AppError> {
        let value = board_state
            .get(field)
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| {
                AppError::Parse(format!("canvas board_state.{field} must be a number"))
            })?;
        if !value.is_finite() {
            return Err(AppError::Parse(format!(
                "canvas board_state.{field} must be finite"
            )));
        }
        Ok(value as f32)
    };
    let pan_x = required_finite("pan_x")?;
    let pan_y = required_finite("pan_y")?;
    let zoom = required_finite("zoom")?;
    Ok(CanvasBoardData {
        placements,
        visual_edges,
        pan_x,
        pan_y,
        zoom,
    })
}

fn strict_canvas_placement(
    row: &serde_json::Value,
    index: usize,
) -> Result<CanvasPlacementCard, AppError> {
    for field in [
        "placement_id",
        "canvas_block_id",
        "workspace_id",
        "placed_block_id",
    ] {
        if row.get(field).and_then(serde_json::Value::as_str).is_none() {
            return Err(AppError::Parse(format!(
                "canvas board placements[{index}].{field} must be a string"
            )));
        }
    }
    for field in ["x", "y", "w", "h"] {
        let value = row
            .get(field)
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| {
                AppError::Parse(format!(
                    "canvas board placements[{index}].{field} must be a number"
                ))
            })?;
        if !value.is_finite() {
            return Err(AppError::Parse(format!(
                "canvas board placements[{index}].{field} must be finite"
            )));
        }
    }
    if row
        .get("z_index")
        .and_then(serde_json::Value::as_i64)
        .is_none()
    {
        return Err(AppError::Parse(format!(
            "canvas board placements[{index}].z_index must be an integer"
        )));
    }
    placement_from_json(row).ok_or_else(|| {
        AppError::Parse(format!(
            "canvas board placements[{index}] is missing a required id"
        ))
    })
}

/// Parse one verified `LoomCanvasPlacement` JSON object into a [`CanvasPlacementCard`] (no live title
/// yet). Returns `None` only when `placement_id` or `placed_block_id` is missing (a malformed row is
/// skipped, not faked).
fn placement_from_json(p: &serde_json::Value) -> Option<CanvasPlacementCard> {
    let placement_id = p.get("placement_id").and_then(|x| x.as_str())?.to_owned();
    let placed_block_id = p
        .get("placed_block_id")
        .and_then(|x| x.as_str())?
        .to_owned();
    let x = p.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
    let y = p.get("y").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
    let w = p.get("w").and_then(|x| x.as_f64()).unwrap_or(200.0) as f32;
    let h = p.get("h").and_then(|x| x.as_f64()).unwrap_or(120.0) as f32;
    let mut card = CanvasPlacementCard::new(placement_id, placed_block_id, x, y, w, h);
    card.z_index = p.get("z_index").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
    card.group_id = p
        .get("group_id")
        .and_then(|x| x.as_str())
        .map(ToOwned::to_owned);
    // WP-KERNEL-012 MT-080: the backend now stamps each placement with a durable `is_text_card` flag
    // (`LoomCanvasPlacement.is_text_card`). When set, mark the card as a free-text [`CanvasCardKind::TextCard`]
    // so the inline editor stays REACHABLE across sessions — independent of the host-origin
    // `canvas_text_card_block_ids` tracking (which only knows about cards created THIS session). serde-style
    // default: an absent/non-bool field reads as `false` (a plain block reference). Reference-not-copy safe:
    // `live_body` is only seeded to an empty buffer so a double-click opens an editable (initially empty)
    // card — the backend field never carries block content.
    if p.get("is_text_card")
        .and_then(|x| x.as_bool())
        .unwrap_or(false)
    {
        card.card_kind = CanvasCardKind::TextCard;
        if card.live_body.is_none() {
            card.live_body = Some(String::new());
        }
    }
    Some(card)
}

/// Parse the response body of either canvas creation route into the placement payload the host needs for
/// compensating undo registration. Accepts both verified shapes:
/// - `POST .../placements` -> `LoomCanvasPlacement`
/// - `POST .../cards` -> `{ block, rich_document_id, placement: LoomCanvasPlacement }`
pub fn created_canvas_placement_from_response(
    value: &serde_json::Value,
) -> Result<CreatedCanvasPlacement, AppError> {
    let placement = value.get("placement").unwrap_or(value);
    let string_field = |field: &str| {
        placement
            .get(field)
            .and_then(|x| x.as_str())
            .map(ToOwned::to_owned)
            .ok_or_else(|| AppError::Parse(format!("canvas placement response missing {field}")))
    };
    let number_field = |field: &str| {
        placement
            .get(field)
            .and_then(|x| x.as_f64())
            .ok_or_else(|| AppError::Parse(format!("canvas placement response missing {field}")))
    };
    Ok(CreatedCanvasPlacement {
        placement_id: string_field("placement_id")?,
        placed_block_id: string_field("placed_block_id")?,
        x: number_field("x")?,
        y: number_field("y")?,
        w: number_field("w")?,
        h: number_field("h")?,
        created_by_request: value
            .get("created_by_request")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
    })
}

/// Parse one verified `LoomCanvasVisualEdge` JSON object into a [`VisualEdge`]. Returns `None` when any
/// required id is missing.
fn visual_edge_from_json(e: &serde_json::Value) -> Option<VisualEdge> {
    Some(VisualEdge {
        visual_edge_id: e.get("visual_edge_id").and_then(|x| x.as_str())?.to_owned(),
        from_placement_id: e
            .get("from_placement_id")
            .and_then(|x| x.as_str())?
            .to_owned(),
        to_placement_id: e
            .get("to_placement_id")
            .and_then(|x| x.as_str())?
            .to_owned(),
    })
}

/// `GET {url}` and read a verified `LoomBlock`'s `(title, content_type, content_hash)` for the
/// live-resolve. `title` is `Option<String>` (a block can be untitled); `content_type` defaults to
/// "note"; `content_hash` is the backend-computed canonical-JSON hash when present (MT-032, READ-only —
/// `Option<String>`, honestly `None` when the backend omits it). A 404 (the block was deleted) is an
/// [`AppError`] so the host shows "(stale reference)" — never a fabricated title.
async fn fetch_live_block(
    client: &reqwest::Client,
    url: &str,
) -> Result<LiveBlock, LiveBlockResolveError> {
    let response = client
        .get(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|error| LiveBlockResolveError::Unavailable(error.to_string()))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(LiveBlockResolveError::Missing);
    }
    if !response.status().is_success() {
        return Err(LiveBlockResolveError::Unavailable(format!(
            "GET non-success status {}",
            response.status()
        )));
    }
    let v = response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| LiveBlockResolveError::Unavailable(format!("decode: {error}")))?;
    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(ToOwned::to_owned);
    let content_type = v
        .get("content_type")
        .and_then(|x| x.as_str())
        .unwrap_or("note")
        .to_owned();
    let content_hash = v
        .get("content_hash")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(ToOwned::to_owned);
    Ok((title, content_type, content_hash))
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-022 Loom FOLDER-TREE transport (REUSE — extends the MT-021 Loom read surface).
//
// VERIFIED READ-ONLY against `src/backend/handshake_core/src/{api,storage}/loom.rs` (the running
// backend), NOT taken from the MT-022 contract body — whose assumed surface (content_type='folder'
// LoomBlocks, color in content_json.metadata.color_label, children via views/sorted?tag_ids=) does NOT
// exist (the MT-022/023 "verify, don't trust the contract" lesson the LoomGraphClient + DrawerDataClient
// already embody). The REAL folder authority is the dedicated `loom_folders` subsystem (MT-181
// FolderTreeAndColorLabels, Master Spec §7.1.4.3), an organizational overlay over LoomBlocks with a
// first-class `color` column. The three routes this client binds (mounted in `loom::routes`):
//   - GET   /workspaces/:ws/loom/folders                    -> Vec<LoomFolder> (the tree rows; the
//     parent/child shape is `parent_folder_id`, so the tree is built CLIENT-side from the flat list).
//   - GET   /workspaces/:ws/loom/folders/:folder_id/blocks  -> Vec<LoomBlock> (the lazy child-block
//     load on expand; supports `limit`/`offset`, default limit 100, capped 500).
//   - PATCH /workspaces/:ws/loom/folders/:folder_id  body { "color": "#rrggbb" } -> LoomFolder (recolor).
//     `LoomFolderUpdate.color` is `Option<Option<String>>` server-side, i.e. a TRUE JSON merge-patch:
//     sending ONLY `color` leaves name/sort/parent untouched (RISK-2/MC-2: no whole-record clobber).
//
// Follows the MT-020/021/023 off-thread shape exactly: spawn on the app's tokio runtime, deliver the
// parsed result into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next frame (HBR-QUIET
// — the render thread is NEVER blocked on the network). Speaks `serde_json::Value` so it never depends
// on the `handshake_core` crate's types; the parsed shapes are the widget's own
// `graph::folder_tree::{FolderRow, LeafBlock}` (the field-correct reuse of the verified backend shapes).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use crate::graph::folder_tree::{FolderRow, LeafBlock};

/// The externally-meaningful result of a folder-list fetch: the flat [`FolderRow`] list the
/// [`crate::graph::folder_tree::LoomFolderTree`] builds its forest from. `Ok` carries the rows (possibly
/// empty -> the "No folders" empty state, AC7); `Err(msg)` a failure the view surfaces as an error
/// banner + Retry (AC8) instead of crashing.
pub type FolderListDelivery = (String, u64, u64, Result<Vec<FolderRow>, String>);
pub type FolderListCell = Arc<Mutex<VecDeque<FolderListDelivery>>>;

/// The externally-meaningful result of a folder-children fetch: the leaf [`LeafBlock`] list for one
/// expanded folder. `Ok` carries the blocks (possibly empty); `Err(msg)` a failure the node surfaces.
/// The host clears the node's `loading` flag when this delivers (the bounded-spinner rule).
pub type FolderChildrenDelivery = (String, String, u64, u64, Result<Vec<LeafBlock>, String>);
pub type FolderChildrenCell = Arc<Mutex<Option<FolderChildrenDelivery>>>;

/// One folder create/update/delete delivery. Create/rename/move return the canonical row; delete
/// returns `None`. Every failure remains typed and visible to the mounted host.
pub type FolderWriteDelivery = (String, u64, u64, Result<Option<FolderRow>, String>);
pub type FolderWriteCell = Arc<Mutex<Option<FolderWriteDelivery>>>;

/// A recolor completion carries all context in-band. The host never couples a generic receipt to a
/// mutable side slot, so an old workspace/operation cannot color the currently mounted tree.
pub type FolderRecolorDelivery = (String, String, u64, u64, Result<(), String>);
pub type FolderRecolorCell = Arc<Mutex<Option<FolderRecolorDelivery>>>;

const FOLDER_CHILD_PAGE_SIZE: u32 = 500;
const MAX_FOLDER_CHILDREN: usize = 100_000;

/// REST client for the VERIFIED Loom folder-tree surface (MT-181 backend) the MT-022 folder tree binds:
/// list folders/children and create, rename, recolor, move, or delete a folder. Mirrors the `LoomGraphClient` /
/// `DrawerDataClient` shape exactly (off-thread + delivery cell). Speaks `serde_json::Value` so it never
/// depends on the `handshake_core` crate's types.
#[derive(Clone)]
pub struct LoomFolderClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomFolderClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            // Reuse the process-wide pool and its hard connect/request deadlines. Timeout failures
            // flow through the typed result cells and become the folder pane's visible error.
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn folders_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/folders", self.base_url, workspace_id)
    }

    fn folder_url(&self, workspace_id: &str, folder_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/folders/{}",
            self.base_url, workspace_id, folder_id
        )
    }

    fn folder_blocks_url(&self, workspace_id: &str, folder_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/folders/{}/blocks",
            self.base_url, workspace_id, folder_id
        )
    }

    /// Pure request builder for the folder-list fetch: `GET /loom/folders` (no query). Split out so a
    /// unit test asserts the EXACT verified URL without a live backend (the spawn path routes through
    /// this same builder, so the test proves the production request construction).
    pub fn list_folders_request(&self, workspace_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.folders_url(workspace_id),
            query: vec![],
        }
    }

    /// Pure request builder for the first child-block page. Production fetching continues with
    /// offset pages until the backend returns fewer than [`FOLDER_CHILD_PAGE_SIZE`] rows.
    pub fn list_folder_blocks_request(
        &self,
        workspace_id: &str,
        folder_id: &str,
    ) -> GetRequestSpec {
        self.list_folder_blocks_page_request(workspace_id, folder_id, 0)
    }

    pub fn list_folder_blocks_page_request(
        &self,
        workspace_id: &str,
        folder_id: &str,
        offset: u32,
    ) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.folder_blocks_url(workspace_id, folder_id),
            query: vec![
                ("limit".to_owned(), FOLDER_CHILD_PAGE_SIZE.to_string()),
                ("offset".to_owned(), offset.to_string()),
            ],
        }
    }

    /// Pure request builder for the recolor PATCH: `PATCH /loom/folders/{id}` body `{ "color": "#hex" }`.
    /// The body carries ONLY the `color` key — a true JSON merge-patch against the verified
    /// `LoomFolderUpdate` (whose `color: Option<Option<String>>` means "set color, leave everything
    /// else"), so a recolor can NEVER clobber the folder's name/sort/parent (RISK-2 / MC-2 / AC4).
    pub fn recolor_request(&self, workspace_id: &str, folder_id: &str, hex: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.folder_url(workspace_id, folder_id),
            body: Some(serde_json::json!({ "color": hex })),
        }
    }

    pub fn create_folder_request(
        &self,
        workspace_id: &str,
        name: &str,
        parent_folder_id: Option<&str>,
        sort_order: Option<i32>,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: self.folders_url(workspace_id),
            body: Some(serde_json::json!({
                "name": name,
                "parent_folder_id": parent_folder_id,
                "sort_order": sort_order,
            })),
        }
    }

    pub fn rename_folder_request(
        &self,
        workspace_id: &str,
        folder_id: &str,
        name: &str,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.folder_url(workspace_id, folder_id),
            body: Some(serde_json::json!({ "name": name })),
        }
    }

    pub fn move_folder_request(
        &self,
        workspace_id: &str,
        folder_id: &str,
        parent_folder_id: Option<&str>,
        sort_order: Option<i32>,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.folder_url(workspace_id, folder_id),
            body: Some(serde_json::json!({
                "parent_folder_id": parent_folder_id,
                "sort_order": sort_order,
            })),
        }
    }

    pub fn delete_folder_request(&self, workspace_id: &str, folder_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Delete,
            url: self.folder_url(workspace_id, folder_id),
            body: None,
        }
    }

    /// Fetch the workspace's folder list off the UI thread, delivering the parsed rows into `cell` (the
    /// initial AC1 tree load). The host sets `loading=true` before calling and clears it on delivery.
    pub fn fetch_folders(
        &self,
        workspace_id: &str,
        workspace_epoch: u64,
        request_sequence: u64,
        cell: FolderListCell,
    ) {
        let spec = self.list_folders_request(workspace_id);
        let client = self.client.clone();
        let workspace_id = workspace_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_folder_rows(&client, &spec.url).await;
            if let Ok(mut slot) = cell.lock() {
                slot.push_back((
                    workspace_id,
                    workspace_epoch,
                    request_sequence,
                    result.map_err(|e| e.to_string()),
                ));
            }
        });
    }

    /// Fetch one folder's child blocks off the UI thread, delivering the parsed leaves into `cell` (the
    /// AC2 lazy child load on expand). The host sets the node's `loading=true` before calling (so the
    /// spinner animates ONLY during this genuine in-flight fetch) and clears it on delivery.
    pub fn fetch_folder_blocks(
        &self,
        workspace_id: &str,
        folder_id: &str,
        workspace_epoch: u64,
        request_sequence: u64,
        cell: FolderChildrenCell,
    ) {
        let url = self.folder_blocks_url(workspace_id, folder_id);
        let client = self.client.clone();
        let workspace_id = workspace_id.to_owned();
        let folder_id = folder_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_all_folder_leaves(&client, &url).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((
                    workspace_id,
                    folder_id,
                    workspace_epoch,
                    request_sequence,
                    result.map_err(|e| e.to_string()),
                ));
            }
        });
    }

    /// Recolor a folder off the UI thread (AC4), delivering the outcome into `cell`. The PATCH body is
    /// the single-`color`-key merge-patch from [`recolor_request`](Self::recolor_request). `Ok(())` on a
    /// 2xx; `Err(msg)` on failure. The host applies the swatch only after this success delivery.
    pub fn recolor_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
        hex: &str,
        workspace_epoch: u64,
        request_sequence: u64,
        cell: FolderRecolorCell,
    ) {
        let spec = self.recolor_request(workspace_id, folder_id, hex);
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        let workspace_id = workspace_id.to_owned();
        let folder_id = folder_id.to_owned();
        self.runtime.spawn(async move {
            let result = patch_expect_success(&client, &spec.url, &body).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((
                    workspace_id,
                    folder_id,
                    workspace_epoch,
                    request_sequence,
                    result.map_err(|e| e.to_string()),
                ));
            }
        });
    }

    fn send_folder_write(
        &self,
        workspace_id: &str,
        workspace_epoch: u64,
        request_sequence: u64,
        spec: RequestSpec,
        cell: FolderWriteCell,
    ) {
        let client = self.client.clone();
        let workspace_id = workspace_id.to_owned();
        self.runtime.spawn(async move {
            let result = match spec.method {
                HttpMethod::Post => {
                    let body = spec.body.as_ref().expect("folder POST body");
                    post_json_expect_value(&client, &spec.url, body, Duration::from_secs(5))
                        .await
                        .and_then(|value| folder_to_row(&value).map(Some))
                }
                HttpMethod::Patch => {
                    let body = spec.body.as_ref().expect("folder PATCH body");
                    patch_json_expect_value(&client, &spec.url, body)
                        .await
                        .and_then(|value| folder_to_row(&value).map(Some))
                }
                HttpMethod::Delete => delete_expect_success(&client, &spec.url)
                    .await
                    .map(|_| None),
                _ => Err(AppError::Http(
                    "folder write requires POST, PATCH, or DELETE".to_owned(),
                )),
            };
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((
                    workspace_id,
                    workspace_epoch,
                    request_sequence,
                    result.map_err(|error| error.to_string()),
                ));
            }
        });
    }

    // Folder mutations retain explicit workspace/generation/sequence authority at the call boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn create_folder(
        &self,
        workspace_id: &str,
        name: &str,
        parent_folder_id: Option<&str>,
        sort_order: Option<i32>,
        workspace_epoch: u64,
        request_sequence: u64,
        cell: FolderWriteCell,
    ) {
        self.send_folder_write(
            workspace_id,
            workspace_epoch,
            request_sequence,
            self.create_folder_request(workspace_id, name, parent_folder_id, sort_order),
            cell,
        );
    }

    pub fn rename_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
        name: &str,
        workspace_epoch: u64,
        request_sequence: u64,
        cell: FolderWriteCell,
    ) {
        self.send_folder_write(
            workspace_id,
            workspace_epoch,
            request_sequence,
            self.rename_folder_request(workspace_id, folder_id, name),
            cell,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn move_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
        parent_folder_id: Option<&str>,
        sort_order: Option<i32>,
        workspace_epoch: u64,
        request_sequence: u64,
        cell: FolderWriteCell,
    ) {
        self.send_folder_write(
            workspace_id,
            workspace_epoch,
            request_sequence,
            self.move_folder_request(workspace_id, folder_id, parent_folder_id, sort_order),
            cell,
        );
    }

    pub fn delete_folder(
        &self,
        workspace_id: &str,
        folder_id: &str,
        workspace_epoch: u64,
        request_sequence: u64,
        cell: FolderWriteCell,
    ) {
        self.send_folder_write(
            workspace_id,
            workspace_epoch,
            request_sequence,
            self.delete_folder_request(workspace_id, folder_id),
            cell,
        );
    }
}

async fn patch_json_expect_value(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let response = client
        .patch(url)
        .timeout(Duration::from_secs(5))
        .json(body)
        .send()
        .await
        .map_err(|error| AppError::Http(error.to_string()))?;
    if !response.status().is_success() {
        return Err(AppError::Http(format!(
            "PATCH non-success status {}",
            response.status()
        )));
    }
    response
        .json()
        .await
        .map_err(|error| AppError::Parse(error.to_string()))
}

/// Parse one verified `LoomFolder` JSON object into a [`FolderRow`]. The complete canonical wire shape
/// is type-checked; malformed successful responses fail closed instead of being partially accepted.
fn folder_to_row(folder: &serde_json::Value) -> Result<FolderRow, AppError> {
    let folder_id = folder
        .get("folder_id")
        .and_then(|x| x.as_str())
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| {
            AppError::Parse("LoomFolder.folder_id must be a non-empty string".to_owned())
        })?
        .to_owned();
    required_nonempty_string(folder, "workspace_id")?;
    let parent_folder_id = optional_nonempty_string(folder, "parent_folder_id")?;
    let name = folder
        .get("name")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| AppError::Parse("LoomFolder.name must be a non-empty string".to_owned()))?
        .to_owned();
    // `loom_folders.color` intentionally accepts CSS-style tokens (including short hex and names).
    // Preserve any non-empty backend value in the row; the widget resolves supported colors and uses
    // the theme-neutral swatch for an unknown token without rejecting every otherwise-valid folder.
    let color = optional_nonempty_string(folder, "color")?;
    if let Some(value) = folder.get("sort_order") {
        if !value.is_null() && value.as_i64().and_then(|n| i32::try_from(n).ok()).is_none() {
            return Err(AppError::Parse(
                "LoomFolder.sort_order must be a 32-bit integer or null".to_owned(),
            ));
        }
    }
    required_nonempty_string(folder, "sort_mode")?;
    optional_nonempty_string(folder, "project_ref")?;
    required_nonempty_string(folder, "created_at")?;
    required_nonempty_string(folder, "updated_at")?;
    Ok(FolderRow::new(folder_id, parent_folder_id, name, color))
}

fn required_nonempty_string<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, AppError> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| AppError::Parse(format!("{field} must be a non-empty string")))
}

fn optional_nonempty_string(
    value: &serde_json::Value,
    field: &str,
) -> Result<Option<String>, AppError> {
    match value.get(field) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(text)) if !text.trim().is_empty() => Ok(Some(text.clone())),
        _ => Err(AppError::Parse(format!(
            "{field} must be a non-empty string or null"
        ))),
    }
}

/// Parse one verified `LoomBlock` JSON object into a folder-tree [`LeafBlock`]. Mirrors the graph
/// view's `block_to_node` field reads (`block_id`/`title`/`content_type`) so the two surfaces agree on
/// the verified block shape. The complete wire shape is validated; only the optional title may fall
/// back to the block id.
fn block_to_leaf(block: &serde_json::Value) -> Result<LeafBlock, AppError> {
    let block_id = block
        .get("block_id")
        .and_then(|x| x.as_str())
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| AppError::Parse("LoomBlock.block_id must be a non-empty string".to_owned()))?
        .to_owned();
    required_nonempty_string(block, "workspace_id")?;
    for field in [
        "document_id",
        "asset_id",
        "original_filename",
        "content_hash",
        "journal_date",
        "imported_at",
    ] {
        optional_nonempty_string(block, field)?;
    }
    let title = match block.get("title") {
        None | Some(serde_json::Value::Null) => block_id.clone(),
        Some(serde_json::Value::String(text)) if text.trim().is_empty() => block_id.clone(),
        Some(serde_json::Value::String(text)) => text.clone(),
        _ => return Err(AppError::Parse("title must be a string or null".to_owned())),
    };
    let content_type = required_nonempty_string(block, "content_type")?.to_owned();
    for field in ["pinned", "favorite"] {
        if block
            .get(field)
            .and_then(serde_json::Value::as_bool)
            .is_none()
        {
            return Err(AppError::Parse(format!("{field} must be a bool")));
        }
    }
    if let Some(value) = block.get("pin_order") {
        if !value.is_null() && value.as_i64().and_then(|n| i32::try_from(n).ok()).is_none() {
            return Err(AppError::Parse(
                "pin_order must be a 32-bit integer or null".to_owned(),
            ));
        }
    }
    required_nonempty_string(block, "created_at")?;
    required_nonempty_string(block, "updated_at")?;
    if !block
        .get("derived")
        .is_some_and(serde_json::Value::is_object)
    {
        return Err(AppError::Parse("derived must be an object".to_owned()));
    }
    Ok(LeafBlock::new(block_id, title, content_type))
}

/// `GET {url}` and parse the verified `Vec<LoomFolder>` into [`FolderRow`]s. A valid empty array yields
/// the "No folders" state (AC7). A successful non-array body or malformed row is a typed parse error
/// (AC8), never silently reinterpreted as an empty workspace.
async fn fetch_folder_rows(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<FolderRow>, AppError> {
    let v = get_json(client, url, &[]).await?;
    let array = v
        .as_array()
        .ok_or_else(|| AppError::Parse("Loom folder list response must be an array".to_owned()))?;
    let mut rows = Vec::with_capacity(array.len());
    for (index, value) in array.iter().enumerate() {
        rows.push(
            folder_to_row(value).map_err(|error| {
                AppError::Parse(format!("Loom folder list row {index}: {error}"))
            })?,
        );
    }
    Ok(rows)
}

/// `GET {url}?{query}` and parse the verified `Vec<LoomBlock>` into folder-tree [`LeafBlock`]s. An
/// empty array yields an empty leaf list (the folder renders "(empty)"). A successful non-array body
/// or malformed block row is a typed parse error, never silently dropped.
async fn fetch_folder_leaves(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<Vec<LeafBlock>, AppError> {
    let v = get_json(client, url, query).await?;
    let array = v.as_array().ok_or_else(|| {
        AppError::Parse("Loom folder children response must be an array".to_owned())
    })?;
    let mut leaves = Vec::with_capacity(array.len());
    for (index, value) in array.iter().enumerate() {
        leaves.push(
            block_to_leaf(value).map_err(|error| {
                AppError::Parse(format!("Loom folder child row {index}: {error}"))
            })?,
        );
    }
    Ok(leaves)
}

/// Fetch every folder-member page without silently truncating large folders. Duplicate ids across
/// pages or a folder beyond the explicit safety ceiling fail closed as a typed pane error.
async fn fetch_all_folder_leaves(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<LeafBlock>, AppError> {
    let mut all = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut offset = 0u32;
    loop {
        let query = vec![
            ("limit".to_owned(), FOLDER_CHILD_PAGE_SIZE.to_string()),
            ("offset".to_owned(), offset.to_string()),
        ];
        let page = fetch_folder_leaves(client, url, &query).await?;
        let page_len = page.len();
        for leaf in page {
            if !seen.insert(leaf.block_id.clone()) {
                return Err(AppError::Parse(format!(
                    "Loom folder pagination repeated block_id {}",
                    leaf.block_id
                )));
            }
            all.push(leaf);
        }
        if page_len < FOLDER_CHILD_PAGE_SIZE as usize {
            return Ok(all);
        }
        if all.len() >= MAX_FOLDER_CHILDREN {
            return Err(AppError::Parse(format!(
                "Loom folder contains at least {MAX_FOLDER_CHILDREN} blocks; refusing an unbounded UI load"
            )));
        }
        offset = offset
            .checked_add(FOLDER_CHILD_PAGE_SIZE)
            .ok_or_else(|| AppError::Parse("Loom folder pagination offset overflow".to_owned()))?;
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-023 Loom TAG-HUB transport (REUSE — extends the MT-021/022 Loom read surface).
//
// VERIFIED READ-ONLY against `src/backend/handshake_core/src/{api,storage}/loom.rs` (the running
// backend), NOT taken from the MT-023 contract body. The generic `views/all?content_type=tag_hub` and
// `views/all?tag_ids={id}` filters exist, but the dedicated tag authority for this surface is the
// stronger MT-182 tag-hub API (tags as first-class blocks):
//   - GET  /workspaces/:ws/loom/tags                       -> Vec<LoomBlock> (every `tag_hub` block; the
//     flat list the panel renders. Because this route ALREADY returns only tag hubs, RISK-5's
//     client-side content_type fallback is unnecessary — there is no `content_type` filter to fall back
//     from). Supports `limit`/`offset` (default 100, capped 500).
//   - GET  /workspaces/:ws/loom/tags/:tag_block_id         -> LoomTagHub { block, sub_tags,
//     tagged_blocks, backlink_count } (the exact hub page/count source: title from block.title, members
//     from tagged_blocks).
//   - GET  /workspaces/:ws/loom/tags/:tag_block_id/blocks  -> Vec<LoomBlock> (members; supports
//     `include_subtags`/`limit`/`offset`; default 100, capped 500). Direct route/live proof source, not
//     the exact list badge count source.
//   - POST /workspaces/:ws/loom/edges  body { source_block_id, target_block_id, edge_type:"tag",
//     created_by:"user" } -> LoomEdge (tag a block with a hub). The backend HARD-rejects a non-tag_hub
//     target with HSK-400-LOOM-TAG-TARGET-MUST-BE-TAG_HUB, so the hub is ALWAYS the edge TARGET and the
//     tagged block the SOURCE (verified `create_loom_edge`). `created_by` is the verified
//     `LoomEdgeCreatedBy` enum ("user"/"ai"); "user" is the operator-initiated tag.
//
// Follows the MT-020/021/022 off-thread shape exactly: spawn on the app's tokio runtime, deliver the
// parsed result into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next frame (HBR-QUIET
// — the render thread is NEVER blocked on the network). Speaks `serde_json::Value` so it never depends
// on the handshake_core crate; the parsed shapes are the widget's own graph::tags_panel types.
//
// AC6 / RISK-2 / MC-2 (the no-fixed-sleep correction): `tag_block` spawns the POST and delivers the
// outcome into a `TagEdgeReceiptCell`; the HOST awaits THAT delivery and only THEN re-queries the hub
// detail/members. There is NO 100ms sleep — the re-query is gated on the edge-create RESPONSE.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use crate::graph::tags_panel::{AddTagCandidate, HubMember, TagEntry};

/// The externally-meaningful result of a tag-list fetch: the flat [`TagEntry`] list the
/// [`crate::graph::tags_panel::LoomTagsPanel`] renders. `Ok` carries the entries (possibly empty -> the
/// "No tags" empty state, AC8); `Err(msg)` a failure the panel surfaces as an error banner + Retry. The
/// leading workspace id and request sequence let the host discard stale async deliveries after a project
/// switch or retry.
pub type TagListDelivery = (String, u64, u64, Result<Vec<TagEntry>, String>);
pub type TagListCell = Arc<Mutex<VecDeque<TagListDelivery>>>;

/// The externally-meaningful result of a hub-detail fetch: `(title, members)` for the hub page. `Ok`
/// carries the resolved title + member list; `Err(msg)` a failure the hub page surfaces. The workspace +
/// hub id + request sequence tuple lets the host reject stale responses from an older hub/page request.
pub type TagHubDetailDelivery = (
    String,
    u64,
    String,
    u64,
    Result<(String, Vec<HubMember>), String>,
);
pub type TagHubDetailCell = Arc<Mutex<VecDeque<TagHubDetailDelivery>>>;

/// The externally-meaningful result of an add-tag candidate search: the candidate blocks to tag. `Ok`
/// carries the candidates (possibly empty); `Err(msg)` a failure (the popup shows nothing rather than
/// crashing). The workspace + query + request sequence tuple lets the host reject stale candidate lists.
pub type AddTagCandidatesDelivery = (
    String,
    u64,
    String,
    u64,
    Result<Vec<AddTagCandidate>, String>,
);
pub type AddTagCandidatesCell = Arc<Mutex<VecDeque<AddTagCandidatesDelivery>>>;

/// The externally-meaningful result of a tag-edge POST. The workspace + hub id + request sequence live in
/// the same FIFO delivery as the receipt, so a stale POST can never consume a newer side-slot context.
pub type TagEdgeReceiptDelivery = (String, u64, String, u64, Result<(), String>);
pub type TagEdgeReceiptCell = Arc<Mutex<VecDeque<TagEdgeReceiptDelivery>>>;

/// The backend caps tag-hub list pages at 500 rows. Fetching pages until the backend returns a short
/// page prevents the Tags pane from silently omitting hubs beyond the route's default first 100 rows.
const TAG_HUB_PAGE_SIZE: u32 = 500;
/// Explicit UI safety ceiling. Reaching it is a typed error, never a successful truncated list.
const MAX_TAG_HUBS: usize = 100_000;

/// REST client for the VERIFIED Loom tag-hub surface (MT-182 backend) the MT-023 tags panel binds: list
/// tag hubs, load a hub's detail + members, search for taggable blocks, and create a `tag` edge. Mirrors
/// the `LoomFolderClient` / `LoomGraphClient` shape exactly (off-thread + delivery cell). Speaks
/// `serde_json::Value` so it never depends on the handshake_core crate's types.
#[derive(Clone)]
pub struct LoomTagClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomTagClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client().clone(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn tags_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/tags", self.base_url, workspace_id)
    }

    fn tag_url(&self, workspace_id: &str, tag_block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/tags/{}",
            self.base_url, workspace_id, tag_block_id
        )
    }

    fn tag_blocks_url(&self, workspace_id: &str, tag_block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/tags/{}/blocks",
            self.base_url, workspace_id, tag_block_id
        )
    }

    fn edges_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/edges", self.base_url, workspace_id)
    }

    fn search_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/search", self.base_url, workspace_id)
    }

    /// Pure request builder for the first tag-list page: `GET /loom/tags?limit=500&offset=0`.
    /// Split out so a unit test asserts the EXACT verified URL without a live backend (the spawn path
    /// routes through this same builder, so the test proves the production request construction).
    pub fn list_tags_request(&self, workspace_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.tags_url(workspace_id),
            query: vec![
                ("limit".to_owned(), TAG_HUB_PAGE_SIZE.to_string()),
                ("offset".to_owned(), "0".to_owned()),
            ],
        }
    }

    /// Pure request builder for the hub-detail fetch: `GET /loom/tags/{id}`.
    pub fn tag_detail_request(&self, workspace_id: &str, tag_block_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.tag_url(workspace_id, tag_block_id),
            query: vec![],
        }
    }

    /// Pure request builder for the member-list fetch: `GET /loom/tags/{id}/blocks?limit=100`.
    pub fn list_members_request(&self, workspace_id: &str, tag_block_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.tag_blocks_url(workspace_id, tag_block_id),
            query: vec![("limit".to_owned(), "100".to_owned())],
        }
    }

    /// Pure request builder for the add-tag candidate search: `GET /loom/search?q={q}&limit=20`. The
    /// verified workspace search route returns blocks matching `q` (the candidate blocks to tag).
    pub fn search_blocks_request(&self, workspace_id: &str, q: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.search_url(workspace_id),
            query: vec![
                ("q".to_owned(), q.to_owned()),
                ("limit".to_owned(), "20".to_owned()),
            ],
        }
    }

    /// Pure request builder for the tag-edge create: `POST /loom/edges` with the verified
    /// `CreateLoomEdgeRequest` body. The tagged block is the edge SOURCE; the hub is the TARGET (the
    /// backend rejects a non-tag_hub target). `created_by:"user"` is the operator-initiated tag (AC6).
    pub fn tag_block_request(
        &self,
        workspace_id: &str,
        source_block_id: &str,
        hub_block_id: &str,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: self.edges_url(workspace_id),
            body: Some(serde_json::json!({
                "source_block_id": source_block_id,
                "target_block_id": hub_block_id,
                "edge_type": "tag",
                "created_by": "user",
            })),
        }
    }

    /// Fetch the workspace's tag-hub list off the UI thread, delivering the parsed entries into `cell`
    /// (the initial AC1 load). The host sets `loading=true` before calling and clears it on delivery.
    pub fn fetch_tags(&self, workspace_id: &str, cell: TagListCell) {
        self.fetch_tags_with_identity(workspace_id, 0, 0, cell);
    }

    /// Sequence-attributed tag-list fetch for host-driven UI state. Older deliveries for the same
    /// workspace are dropped by the host when a retry or workspace rebound supersedes them.
    pub fn fetch_tags_with_sequence(&self, workspace_id: &str, sequence: u64, cell: TagListCell) {
        self.fetch_tags_with_identity(workspace_id, 0, sequence, cell);
    }

    /// Epoch + sequence attributed tag-list fetch. The epoch distinguishes A -> B -> A workspace
    /// generations even when an older A request resolves after the workspace returns to A.
    pub fn fetch_tags_with_identity(
        &self,
        workspace_id: &str,
        workspace_epoch: u64,
        sequence: u64,
        cell: TagListCell,
    ) {
        let spec = self.list_tags_request(workspace_id);
        let client = self.client.clone();
        let delivered_workspace = workspace_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_all_tag_entries(&client, &spec.url).await;
            if let Ok(mut slot) = cell.lock() {
                slot.push_back((
                    delivered_workspace,
                    workspace_epoch,
                    sequence,
                    result.map_err(|e| e.to_string()),
                ));
            }
        });
    }

    /// Fetch one hub's detail (title + members) off the UI thread, delivering into `cell` (AC4). Parses
    /// the verified `LoomTagHub` shape: title from `block.title`, members from `tagged_blocks`.
    pub fn fetch_hub_detail(&self, workspace_id: &str, tag_block_id: &str, cell: TagHubDetailCell) {
        self.fetch_hub_detail_with_identity(workspace_id, 0, tag_block_id, 0, cell);
    }

    /// Sequence-attributed hub-detail fetch for host-driven UI state. The host tracks the latest
    /// sequence per `(workspace, hub)` so an older retry/open/count refresh cannot overwrite newer state.
    pub fn fetch_hub_detail_with_sequence(
        &self,
        workspace_id: &str,
        tag_block_id: &str,
        sequence: u64,
        cell: TagHubDetailCell,
    ) {
        self.fetch_hub_detail_with_identity(workspace_id, 0, tag_block_id, sequence, cell);
    }

    pub fn fetch_hub_detail_with_identity(
        &self,
        workspace_id: &str,
        workspace_epoch: u64,
        tag_block_id: &str,
        sequence: u64,
        cell: TagHubDetailCell,
    ) {
        let spec = self.tag_detail_request(workspace_id, tag_block_id);
        let client = self.client.clone();
        let delivered_workspace = workspace_id.to_owned();
        let delivered_hub = tag_block_id.to_owned();
        let fallback_id = tag_block_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_tag_hub_detail(&client, &spec.url, &fallback_id).await;
            if let Ok(mut slot) = cell.lock() {
                slot.push_back((
                    delivered_workspace,
                    workspace_epoch,
                    delivered_hub,
                    sequence,
                    result.map_err(|e| e.to_string()),
                ));
            }
        });
    }

    /// Fetch a hub's members off the UI thread, delivering into `cell`. Used by live route proofs; the
    /// host uses `fetch_hub_detail_with_sequence` for list badge backfill because this route is capped.
    pub fn fetch_members(&self, workspace_id: &str, tag_block_id: &str, cell: TagHubDetailCell) {
        self.fetch_members_with_identity(workspace_id, 0, tag_block_id, 0, cell);
    }

    /// Sequence-attributed member-list fetch. This route is intentionally not used for exact list badge
    /// counts because the backend caps it; it remains useful for direct route verification.
    pub fn fetch_members_with_sequence(
        &self,
        workspace_id: &str,
        tag_block_id: &str,
        sequence: u64,
        cell: TagHubDetailCell,
    ) {
        self.fetch_members_with_identity(workspace_id, 0, tag_block_id, sequence, cell);
    }

    pub fn fetch_members_with_identity(
        &self,
        workspace_id: &str,
        workspace_epoch: u64,
        tag_block_id: &str,
        sequence: u64,
        cell: TagHubDetailCell,
    ) {
        let spec = self.list_members_request(workspace_id, tag_block_id);
        let client = self.client.clone();
        let delivered_workspace = workspace_id.to_owned();
        let delivered_hub = tag_block_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_tag_members(&client, &spec.url, &spec.query).await;
            if let Ok(mut slot) = cell.lock() {
                // The member-list route carries no hub title; deliver an empty title so the host keeps
                // its existing title and replaces only the members.
                slot.push_back((
                    delivered_workspace,
                    workspace_epoch,
                    delivered_hub,
                    sequence,
                    result
                        .map(|m| (String::new(), m))
                        .map_err(|e| e.to_string()),
                ));
            }
        });
    }

    /// Search for candidate blocks to tag off the UI thread, delivering into `cell` (the add-tag popup).
    pub fn search_blocks(&self, workspace_id: &str, q: &str, cell: AddTagCandidatesCell) {
        self.search_blocks_with_identity(workspace_id, 0, q, 0, cell);
    }

    /// Sequence-attributed candidate search for host-driven UI state.
    pub fn search_blocks_with_sequence(
        &self,
        workspace_id: &str,
        q: &str,
        sequence: u64,
        cell: AddTagCandidatesCell,
    ) {
        self.search_blocks_with_identity(workspace_id, 0, q, sequence, cell);
    }

    pub fn search_blocks_with_identity(
        &self,
        workspace_id: &str,
        workspace_epoch: u64,
        q: &str,
        sequence: u64,
        cell: AddTagCandidatesCell,
    ) {
        let spec = self.search_blocks_request(workspace_id, q);
        let client = self.client.clone();
        let delivered_workspace = workspace_id.to_owned();
        let delivered_query = q.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_add_tag_candidates(&client, &spec.url, &spec.query).await;
            if let Ok(mut slot) = cell.lock() {
                slot.push_back((
                    delivered_workspace,
                    workspace_epoch,
                    delivered_query,
                    sequence,
                    result.map_err(|e| e.to_string()),
                ));
            }
        });
    }

    /// Create a `tag` edge (tag `source_block_id` with the hub `hub_block_id`) off the UI thread,
    /// delivering the outcome into `cell` (AC6). The body is the verified `CreateLoomEdgeRequest`. The
    /// HOST awaits this delivery and only THEN re-queries the members (no fixed sleep — RISK-2/MC-2).
    pub fn tag_block(
        &self,
        workspace_id: &str,
        source_block_id: &str,
        hub_block_id: &str,
        cell: TagEdgeReceiptCell,
    ) {
        self.tag_block_with_identity(workspace_id, 0, source_block_id, hub_block_id, 0, cell);
    }

    /// Sequence-attributed tag-edge POST for host-driven UI state. The host suppresses stale error
    /// banners when a newer add-tag attempt for the same hub has superseded this request.
    pub fn tag_block_with_sequence(
        &self,
        workspace_id: &str,
        source_block_id: &str,
        hub_block_id: &str,
        sequence: u64,
        cell: TagEdgeReceiptCell,
    ) {
        self.tag_block_with_identity(
            workspace_id,
            0,
            source_block_id,
            hub_block_id,
            sequence,
            cell,
        );
    }

    pub fn tag_block_with_identity(
        &self,
        workspace_id: &str,
        workspace_epoch: u64,
        source_block_id: &str,
        hub_block_id: &str,
        sequence: u64,
        cell: TagEdgeReceiptCell,
    ) {
        let spec = self.tag_block_request(workspace_id, source_block_id, hub_block_id);
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        let delivered_workspace = workspace_id.to_owned();
        let delivered_hub = hub_block_id.to_owned();
        self.runtime.spawn(async move {
            let result = post_expect_success(&client, &spec.url, &body).await;
            if let Ok(mut slot) = cell.lock() {
                slot.push_back((
                    delivered_workspace,
                    workspace_epoch,
                    delivered_hub,
                    sequence,
                    result.map_err(|e| e.to_string()),
                ));
            }
        });
    }
}

/// Parse one verified `tag_hub` `LoomBlock` JSON object into a [`TagEntry`]. The canonical LoomBlock
/// shape is validated by the same parser the folder surface uses. A successful malformed row fails the
/// complete list instead of being dropped or converted into a fabricated fallback row.
fn block_to_tag_entry(block: &serde_json::Value) -> Result<TagEntry, AppError> {
    let leaf = block_to_leaf(block)?;
    if leaf.content_type != "tag_hub" {
        return Err(AppError::Parse(format!(
            "tag list row {} has content_type {}, expected tag_hub",
            leaf.block_id, leaf.content_type
        )));
    }
    let member_count = tag_member_count_hint(block)?;
    Ok(TagEntry::new(leaf.block_id, leaf.title, member_count))
}

fn count_value_to_u32(value: &serde_json::Value) -> Option<u32> {
    value
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .or_else(|| value.as_i64().and_then(|n| u32::try_from(n).ok()))
}

fn optional_count_field(block: &serde_json::Value, key: &str) -> Result<Option<u32>, AppError> {
    let value = block
        .get("derived")
        .and_then(|derived| derived.get(key))
        .or_else(|| block.get(key));
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(value) => count_value_to_u32(value).map(Some).ok_or_else(|| {
            AppError::Parse(format!("{key} must be an unsigned 32-bit integer or null"))
        }),
    }
}

fn tag_member_count_hint(block: &serde_json::Value) -> Result<Option<u32>, AppError> {
    if let Some(count) = optional_count_field(block, "member_count")? {
        return Ok(Some(count));
    }
    match block.get("tagged_blocks") {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(value) => value
            .as_array()
            .ok_or_else(|| AppError::Parse("tagged_blocks must be an array or null".to_owned()))
            .and_then(|blocks| {
                u32::try_from(blocks.len())
                    .map(Some)
                    .map_err(|_| AppError::Parse("tagged_blocks length exceeds u32".to_owned()))
            }),
    }
}

/// Parse one verified `LoomBlock` JSON object into a hub-page [`HubMember`]. Mirrors the folder-tree
/// `block_to_leaf` field reads so the surfaces agree on the verified block shape.
fn block_to_hub_member(block: &serde_json::Value) -> Result<HubMember, AppError> {
    let leaf = block_to_leaf(block)?;
    Ok(HubMember::new(leaf.block_id, leaf.title, leaf.content_type))
}

/// `GET {url}` and parse the verified `Vec<LoomBlock>` (tag hubs) into [`TagEntry`]s. An empty array
/// yields an empty list (the "No tags" empty state, AC8), never an error. A non-success status / parse
/// failure is an [`AppError`] (the error banner).
async fn fetch_tag_entries_page(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<Vec<TagEntry>, AppError> {
    let value = get_json(client, url, query).await?;
    parse_tag_entries_page(&value)
}

fn parse_tag_entries_page(value: &serde_json::Value) -> Result<Vec<TagEntry>, AppError> {
    let rows = value
        .as_array()
        .ok_or_else(|| AppError::Parse("Loom tag list response must be an array".to_owned()))?;
    let mut entries = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        entries.push(
            block_to_tag_entry(row)
                .map_err(|error| AppError::Parse(format!("Loom tag list row {index}: {error}")))?,
        );
    }
    Ok(entries)
}

/// Fetch every `/loom/tags` page. Duplicate ids, malformed pages, offset overflow, or reaching the
/// explicit safety ceiling fail closed rather than yielding a partial successful list.
async fn fetch_all_tag_entries(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<TagEntry>, AppError> {
    let mut all = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut offset = 0u32;
    loop {
        let query = vec![
            ("limit".to_owned(), TAG_HUB_PAGE_SIZE.to_string()),
            ("offset".to_owned(), offset.to_string()),
        ];
        let page = fetch_tag_entries_page(client, url, &query).await?;
        let page_len = page.len();
        for entry in page {
            if !seen.insert(entry.block_id.clone()) {
                return Err(AppError::Parse(format!(
                    "Loom tag pagination repeated block_id {}",
                    entry.block_id
                )));
            }
            all.push(entry);
        }
        if page_len < TAG_HUB_PAGE_SIZE as usize {
            return Ok(all);
        }
        if all.len() >= MAX_TAG_HUBS {
            return Err(AppError::Parse(format!(
                "workspace contains at least {MAX_TAG_HUBS} tag hubs; refusing an unbounded UI load"
            )));
        }
        offset = offset
            .checked_add(TAG_HUB_PAGE_SIZE)
            .ok_or_else(|| AppError::Parse("Loom tag pagination offset overflow".to_owned()))?;
    }
}

/// `GET {url}` and parse the verified `LoomTagHub` `{ block, tagged_blocks, .. }` into `(title,
/// members)`. The hub title is `block.title` (falling back to the block id); the members are the
/// `tagged_blocks` array. A non-success status / parse failure is an [`AppError`].
async fn fetch_tag_hub_detail(
    client: &reqwest::Client,
    url: &str,
    expected_id: &str,
) -> Result<(String, Vec<HubMember>), AppError> {
    let v = get_json(client, url, &[]).await?;
    parse_tag_hub_detail(&v, expected_id)
}

fn parse_tag_hub_detail(
    v: &serde_json::Value,
    expected_id: &str,
) -> Result<(String, Vec<HubMember>), AppError> {
    let block = v
        .get("block")
        .ok_or_else(|| AppError::Parse("LoomTagHub.block is missing".to_owned()))?;
    let hub = block_to_leaf(block)
        .map_err(|error| AppError::Parse(format!("LoomTagHub.block: {error}")))?;
    if hub.block_id != expected_id {
        return Err(AppError::Parse(format!(
            "LoomTagHub.block.block_id {} does not match requested hub {expected_id}",
            hub.block_id
        )));
    }
    if hub.content_type != "tag_hub" {
        return Err(AppError::Parse(format!(
            "LoomTagHub.block.content_type must be tag_hub, got {}",
            hub.content_type
        )));
    }
    let sub_tags = v
        .get("sub_tags")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AppError::Parse("LoomTagHub.sub_tags must be an array".to_owned()))?;
    for (index, sub_tag) in sub_tags.iter().enumerate() {
        let parsed = block_to_leaf(sub_tag)
            .map_err(|error| AppError::Parse(format!("LoomTagHub.sub_tags[{index}]: {error}")))?;
        if parsed.content_type != "tag_hub" {
            return Err(AppError::Parse(format!(
                "LoomTagHub.sub_tags[{index}].content_type must be tag_hub"
            )));
        }
    }
    if v.get("backlink_count")
        .and_then(serde_json::Value::as_i64)
        .filter(|count| *count >= 0)
        .is_none()
    {
        return Err(AppError::Parse(
            "LoomTagHub.backlink_count must be a non-negative integer".to_owned(),
        ));
    }
    let member_rows = v
        .get("tagged_blocks")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AppError::Parse("LoomTagHub.tagged_blocks must be an array".to_owned()))?;
    let mut members = Vec::with_capacity(member_rows.len());
    for (index, member) in member_rows.iter().enumerate() {
        members.push(block_to_hub_member(member).map_err(|error| {
            AppError::Parse(format!("LoomTagHub.tagged_blocks[{index}]: {error}"))
        })?);
    }
    Ok((hub.title, members))
}

/// `GET {url}?{query}` and parse the verified `Vec<LoomBlock>` (a hub's members) into [`HubMember`]s. An
/// empty array yields an empty member list, never an error.
async fn fetch_tag_members(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<Vec<HubMember>, AppError> {
    let v = get_json(client, url, query).await?;
    parse_tag_members(&v)
}

fn parse_tag_members(v: &serde_json::Value) -> Result<Vec<HubMember>, AppError> {
    let rows = v
        .as_array()
        .ok_or_else(|| AppError::Parse("Loom tag member response must be an array".to_owned()))?;
    let mut members = Vec::with_capacity(rows.len());
    for (index, member) in rows.iter().enumerate() {
        members.push(
            block_to_hub_member(member).map_err(|error| {
                AppError::Parse(format!("Loom tag member row {index}: {error}"))
            })?,
        );
    }
    Ok(members)
}

/// Pure parser: turn a verified `/loom/search` JSON response into add-tag [`AddTagCandidate`]s. Split out
/// (pure over `serde_json::Value`, no I/O) so the VERIFIED response shape is unit-testable without a live
/// backend — the gap that hid the wrong-shape bug (the only candidate-producing widget test injected
/// candidates directly, never exercising this parse).
///
/// VERIFIED shape (`api::loom::search_loom_blocks` -> `Json<Vec<LoomBlockSearchResult>>`, and
/// `storage::loom::LoomBlockSearchResult { block: LoomBlock, score: f64 }` — NO `#[serde(flatten)]`): each
/// array entry is `{ "block": { "block_id", "title", .. }, "score": f64 }`, so `block_id`/`title` live
/// UNDER the `block` key, NOT at the entry's top level. Parsing is deliberately fail-closed: only the
/// verified bare array of `{block:{block_id,title},score}` rows is accepted, and one malformed row rejects
/// the entire payload. An empty verified array is valid.
fn parse_add_tag_candidates(v: &serde_json::Value) -> Result<Vec<AddTagCandidate>, AppError> {
    let rows = v.as_array().ok_or_else(|| {
        AppError::Parse("Loom add-tag search response must be a bare array".to_owned())
    })?;
    rows.iter()
        .enumerate()
        .map(|(index, entry)| {
            let block = entry
                .get("block")
                .and_then(serde_json::Value::as_object)
                .ok_or_else(|| {
                    AppError::Parse(format!("Loom add-tag row {index}.block must be an object"))
                })?;
            let block_id = block
                .get("block_id")
                .and_then(serde_json::Value::as_str)
                .filter(|id| !id.trim().is_empty())
                .ok_or_else(|| {
                    AppError::Parse(format!(
                        "Loom add-tag row {index}.block.block_id must be a nonblank string"
                    ))
                })?;
            let title = block
                .get("title")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    AppError::Parse(format!(
                        "Loom add-tag row {index}.block.title must be a string"
                    ))
                })?;
            entry
                .get("score")
                .and_then(serde_json::Value::as_f64)
                .ok_or_else(|| {
                    AppError::Parse(format!("Loom add-tag row {index}.score must be a number"))
                })?;
            Ok(AddTagCandidate::new(block_id, title))
        })
        .collect()
}

/// `GET {url}?{query}` against the verified workspace search route and parse the result blocks into
/// add-tag [`AddTagCandidate`]s via [`parse_add_tag_candidates`]. An empty result yields no candidates,
/// never an error.
async fn fetch_add_tag_candidates(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<Vec<AddTagCandidate>, AppError> {
    let v = get_json(client, url, query).await?;
    parse_add_tag_candidates(&v)
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-024 Loom SIDEBAR transport (REUSE — extends the MT-021/022/023 Loom read surface).
//
// The native pins/favorites/backlinks/unlinked sidebar (`graph::sidebar_panel::LoomSidebarPanel`) binds
// the EXISTING handshake_core Loom read + mutation APIs through THIS client (NO Tauri — the same HTTP
// client every MT-008/014/021/022/023 surface uses). Every endpoint + body was VERIFIED READ-ONLY
// against `src/backend/handshake_core/src/{api,storage}/loom.rs` (the running backend), NOT taken from
// the MT-024 contract body (the MT-022/023 "verify, don't trust the contract" lesson):
//
//   - PINS:      `GET /workspaces/:ws/loom/views/pins?limit=100`      -> `LoomViewResponse::Pins
//     { blocks }`. `parse_view_type` accepts `pins` (CONFIRMED real this time, unlike the MT-022/023
//     stale view types). The count is `blocks.len()`; there is NO `total` field.
//   - FAVORITES: `GET /workspaces/:ws/loom/views/favorites?limit=100` -> `LoomViewResponse::Favorites
//     { blocks }` (`parse_view_type` accepts `favorites`).
//   - BACKLINKS (contract correction, disclosed): the contract named `graph-search?mention_ids={id}`.
//     That param IS real, but the DEDICATED `GET /workspaces/:ws/loom/blocks/:id/backlinks` ->
//     `Vec<LoomBacklink>` (MT-178 `get_backlinks_with_context`) is the field-correct surface: each
//     backlink carries the incoming `edge` (with `edge_type`) + the `source_block`. That is exactly the
//     AC4 "source block title + edge_type label", so this client binds THAT route (verified), not a
//     synthesized graph-search star.
//   - UNLINKED (contract correction, disclosed): the contract named `GET /loom/views/unlinked` (a
//     WORKSPACE unlinked view). For the *per-active-block* Unlinked section the correct verified surface
//     is the DEDICATED `GET /workspaces/:ws/loom/blocks/:id/unlinked-mentions` ->
//     `Vec<LoomUnlinkedMention>` (MT-178 `scan_loom_block_unlinked_mentions`): blocks whose text mentions
//     the active block's title with NO edge — exactly the AC5 semantics. The workspace `/views/unlinked`
//     is NOT scoped to the active block, so it would be the wrong list.
//   - REMOVE PIN (two-call, RISK-1 / MC-1): `PUT /workspaces/:ws/loom/blocks/:id/pin-order` body
//     `{ "pin_order": null }` (`SetPinOrderRequest`, MT-183 — the field is `pin_order`, NOT the
//     contract's `ordinal`) THEN `PATCH /workspaces/:ws/loom/blocks/:id` body `{ "pinned": false }`
//     (`LoomBlockUpdate`, MT-022 confirmed `pinned` is an `Option<bool>` PATCH field). Both are issued in
//     sequence (the React WorkspaceSidebar.tsx lines 297-298 flow); on the SECOND failure the host
//     re-fetches Pins to determine true state (RISK-1 recovery).
//   - REMOVE FAVORITE: `PATCH /workspaces/:ws/loom/blocks/:id` body `{ "favorite": false }`.
//
// All follow the MT-020/021/023 off-thread shape: spawn on the app's tokio runtime, deliver the parsed
// result into an identity-stamped FIFO the egui UI thread drains next frame (HBR-QUIET — the render
// thread is NEVER blocked on the network). A FIFO completion carries workspace epoch, operation target,
// and sequence in the same value, so reordering cannot overwrite or misattribute another request. Speaks
// `serde_json::Value` so it never depends on the
// `handshake_core` crate's types; the parsed shapes are the widget's own
// `graph::sidebar_panel::{SidebarBlock, BacklinkRow, UnlinkedRow}` (field-correct reuse of the verified
// backend shapes).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use crate::graph::sidebar_panel::{BacklinkRow, SectionKind, SidebarBlock, UnlinkedRow};

/// The externally-meaningful result of a Pins/Favorites fetch: the [`SidebarBlock`] list the
/// [`crate::graph::sidebar_panel::LoomSidebarPanel`] renders. `Ok` carries the blocks (possibly empty ->
/// the section empty state); `Err(msg)` a failure the section surfaces as an inline banner + Retry (AC9).
pub type SidebarBlockListDelivery = (String, u64, u64, Result<Vec<SidebarBlock>, String>);
pub type SidebarBlockListCell = Arc<Mutex<VecDeque<SidebarBlockListDelivery>>>;

/// The externally-meaningful result of a backlinks fetch: the [`BacklinkRow`] list (source block + edge
/// type). Stamped with the generation the host bumped on dispatch so a stale delivery is dropped (RISK-2).
pub type SidebarBacklinksDelivery = (
    String,
    u64,
    String,
    u64,
    u64,
    Result<Vec<BacklinkRow>, String>,
);
pub type SidebarBacklinksCell = Arc<Mutex<VecDeque<SidebarBacklinksDelivery>>>;

/// The externally-meaningful result of an unlinked-mentions fetch: the [`UnlinkedRow`] list. Stamped with
/// the dispatch generation so a stale delivery is dropped (RISK-2).
pub type SidebarUnlinkedDelivery = (
    String,
    u64,
    String,
    u64,
    u64,
    Result<Vec<UnlinkedRow>, String>,
);
pub type SidebarUnlinkedCell = Arc<Mutex<VecDeque<SidebarUnlinkedDelivery>>>;

/// FIFO mutation result. Identity travels with the completion so a slow older action can never consume
/// a newer side-slot and emit/remove the wrong bookmark after workspace or operation reordering.
pub type SidebarActionDelivery = (String, u64, SectionKind, String, u64, Result<(), String>);
pub type SidebarActionCell = Arc<Mutex<VecDeque<SidebarActionDelivery>>>;

/// REST client for the VERIFIED Loom sidebar surfaces the MT-024 sidebar panel binds: pins/favorites
/// view lists, per-block backlinks + unlinked-mentions, and the two-call pin removal + favorite removal.
/// Mirrors the `LoomTagClient` / `LoomFolderClient` / `LoomGraphClient` shape exactly.
#[derive(Clone)]
pub struct LoomSidebarClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomSidebarClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn view_url(&self, workspace_id: &str, view_type: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/views/{}",
            self.base_url, workspace_id, view_type
        )
    }

    fn block_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/blocks/{}",
            self.base_url, workspace_id, block_id
        )
    }

    /// WP-KERNEL-012 MT-024 FAIL_V2: the single atomic pin-removal endpoint.
    fn remove_pin_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/blocks/{}/remove-pin",
            self.base_url, workspace_id, block_id
        )
    }

    fn backlinks_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/blocks/{}/backlinks",
            self.base_url, workspace_id, block_id
        )
    }

    fn unlinked_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/blocks/{}/unlinked-mentions",
            self.base_url, workspace_id, block_id
        )
    }

    /// Pure request builder for the Pins fetch: `GET /loom/views/pins?limit=100`. Split out so a unit
    /// test asserts the EXACT verified URL + query without a live backend (the spawn path routes through
    /// this same builder, so the test proves the production request construction).
    pub fn pins_request(&self, workspace_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.view_url(workspace_id, "pins"),
            query: vec![("limit".to_owned(), "100".to_owned())],
        }
    }

    /// Pure request builder for the Favorites fetch: `GET /loom/views/favorites?limit=100`.
    pub fn favorites_request(&self, workspace_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.view_url(workspace_id, "favorites"),
            query: vec![("limit".to_owned(), "100".to_owned())],
        }
    }

    /// Pure request builder for the per-block Backlinks fetch: `GET /loom/blocks/{id}/backlinks` (the
    /// verified dedicated MT-178 route — see the module comment for the contract correction).
    pub fn backlinks_request(&self, workspace_id: &str, block_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.backlinks_url(workspace_id, block_id),
            query: vec![],
        }
    }

    /// Pure request builder for the per-block Unlinked-mentions fetch: `GET
    /// /loom/blocks/{id}/unlinked-mentions` (the verified dedicated MT-178 route).
    pub fn unlinked_request(&self, workspace_id: &str, block_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.unlinked_url(workspace_id, block_id),
            query: vec![],
        }
    }

    /// WP-KERNEL-012 MT-024 FAIL_V2: pure request builder for the SINGLE ATOMIC
    /// pin removal. `POST /loom/blocks/{id}/remove-pin` clears pin_order AND
    /// unpins the block in one server transaction alongside the durable
    /// EventLedger receipt, so the running app can never leave the partial
    /// `pin_order cleared but still pinned` state the old two-call flow risked.
    pub fn remove_pin_request(&self, workspace_id: &str, block_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: self.remove_pin_url(workspace_id, block_id),
            body: None,
        }
    }

    /// Pure request builder for the un-favorite PATCH: `PATCH /loom/blocks/{id}` body
    /// `{ "favorite": false }` (AC3).
    pub fn unfavorite_request(&self, workspace_id: &str, block_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.block_url(workspace_id, block_id),
            body: Some(serde_json::json!({ "favorite": false })),
        }
    }

    /// Fetch the Pins list off the UI thread, delivering the parsed blocks into `cell` (AC1). The host
    /// sets the section loading flag before calling and clears it on delivery.
    pub fn fetch_pins(&self, workspace_id: &str, cell: SidebarBlockListCell) {
        self.fetch_pins_with_identity(workspace_id, 0, 0, cell);
    }

    pub fn fetch_pins_with_identity(
        &self,
        workspace_id: &str,
        workspace_epoch: u64,
        sequence: u64,
        cell: SidebarBlockListCell,
    ) {
        let spec = self.pins_request(workspace_id);
        let client = self.client.clone();
        let delivered_workspace = workspace_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_view_blocks(&client, &spec.url, &spec.query).await;
            if let Ok(mut queue) = cell.lock() {
                queue.push_back((
                    delivered_workspace,
                    workspace_epoch,
                    sequence,
                    result.map_err(|e| e.to_string()),
                ));
            }
        });
    }

    /// Fetch the Favorites list off the UI thread, delivering into `cell` (AC3 load).
    pub fn fetch_favorites(&self, workspace_id: &str, cell: SidebarBlockListCell) {
        self.fetch_favorites_with_identity(workspace_id, 0, 0, cell);
    }

    pub fn fetch_favorites_with_identity(
        &self,
        workspace_id: &str,
        workspace_epoch: u64,
        sequence: u64,
        cell: SidebarBlockListCell,
    ) {
        let spec = self.favorites_request(workspace_id);
        let client = self.client.clone();
        let delivered_workspace = workspace_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_view_blocks(&client, &spec.url, &spec.query).await;
            if let Ok(mut queue) = cell.lock() {
                queue.push_back((
                    delivered_workspace,
                    workspace_epoch,
                    sequence,
                    result.map_err(|e| e.to_string()),
                ));
            }
        });
    }

    /// Fetch the active block's Backlinks off the UI thread, delivering `(generation, Ok/Err)` into
    /// `cell` (AC4). `generation` is the value the host bumped on dispatch; the host drops the delivery
    /// if its generation has since advanced (RISK-2 stale-response guard).
    pub fn fetch_backlinks(
        &self,
        workspace_id: &str,
        block_id: &str,
        generation: u64,
        cell: SidebarBacklinksCell,
    ) {
        self.fetch_backlinks_with_identity(workspace_id, 0, block_id, generation, 0, cell);
    }

    pub fn fetch_backlinks_with_identity(
        &self,
        workspace_id: &str,
        workspace_epoch: u64,
        block_id: &str,
        generation: u64,
        sequence: u64,
        cell: SidebarBacklinksCell,
    ) {
        let spec = self.backlinks_request(workspace_id, block_id);
        let client = self.client.clone();
        let delivered_workspace = workspace_id.to_owned();
        let delivered_block = block_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_backlink_rows(&client, &spec.url).await;
            if let Ok(mut queue) = cell.lock() {
                queue.push_back((
                    delivered_workspace,
                    workspace_epoch,
                    delivered_block,
                    generation,
                    sequence,
                    result.map_err(|e| e.to_string()),
                ));
            }
        });
    }

    /// Fetch the active block's Unlinked mentions off the UI thread, delivering `(generation, Ok/Err)`
    /// into `cell` (AC5). Generation-stamped for the same RISK-2 stale-drop guard.
    pub fn fetch_unlinked(
        &self,
        workspace_id: &str,
        block_id: &str,
        generation: u64,
        cell: SidebarUnlinkedCell,
    ) {
        self.fetch_unlinked_with_identity(workspace_id, 0, block_id, generation, 0, cell);
    }

    pub fn fetch_unlinked_with_identity(
        &self,
        workspace_id: &str,
        workspace_epoch: u64,
        block_id: &str,
        generation: u64,
        sequence: u64,
        cell: SidebarUnlinkedCell,
    ) {
        let spec = self.unlinked_request(workspace_id, block_id);
        let client = self.client.clone();
        let delivered_workspace = workspace_id.to_owned();
        let delivered_block = block_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_unlinked_rows(&client, &spec.url).await;
            if let Ok(mut queue) = cell.lock() {
                queue.push_back((
                    delivered_workspace,
                    workspace_epoch,
                    delivered_block,
                    generation,
                    sequence,
                    result.map_err(|e| e.to_string()),
                ));
            }
        });
    }

    /// Remove a pin off the UI thread with the TWO-CALL flow (RISK-1 / MC-1 / AC2): `PUT /pin-order
    /// {pin_order:null}` THEN `PATCH {pinned:false}`. Delivers `Ok(())` only when BOTH succeed; if either
    /// fails, `Err(msg)` (the host rolls the optimistic removal back and re-fetches to find true state).
    /// Both calls are always issued in sequence (the React WorkspaceSidebar.tsx lines 297-298 flow); the
    /// pin-order clear is never skipped.
    pub fn remove_pin(&self, workspace_id: &str, block_id: &str, cell: SidebarActionCell) {
        self.remove_pin_with_identity(workspace_id, 0, block_id, 0, cell);
    }

    pub fn remove_pin_with_identity(
        &self,
        workspace_id: &str,
        workspace_epoch: u64,
        block_id: &str,
        sequence: u64,
        cell: SidebarActionCell,
    ) {
        // WP-KERNEL-012 MT-024 FAIL_V2: ONE atomic POST /remove-pin. The server
        // clears pin_order AND unpins the block in a single transaction with its
        // durable EventLedger receipt, so there is no between-call window that can
        // persist the partial `pin_order cleared but still pinned` state the old
        // two-call PUT-then-PATCH flow risked. On failure the whole mutation rolls
        // back server-side and the host rolls the optimistic row back.
        let remove = self.remove_pin_request(workspace_id, block_id);
        let remove_body = remove.body.unwrap_or_default();
        let client = self.client.clone();
        let delivered_workspace = workspace_id.to_owned();
        let delivered_block = block_id.to_owned();
        self.runtime.spawn(async move {
            let result = post_expect_success(&client, &remove.url, &remove_body).await;
            if let Ok(mut queue) = cell.lock() {
                queue.push_back((
                    delivered_workspace,
                    workspace_epoch,
                    SectionKind::Pins,
                    delivered_block,
                    sequence,
                    result.map_err(|e| e.to_string()),
                ));
            }
        });
    }

    /// Remove a favorite off the UI thread: `PATCH {favorite:false}` (AC3). Single call.
    pub fn remove_favorite(&self, workspace_id: &str, block_id: &str, cell: SidebarActionCell) {
        self.remove_favorite_with_identity(workspace_id, 0, block_id, 0, cell);
    }

    pub fn remove_favorite_with_identity(
        &self,
        workspace_id: &str,
        workspace_epoch: u64,
        block_id: &str,
        sequence: u64,
        cell: SidebarActionCell,
    ) {
        let unfav = self.unfavorite_request(workspace_id, block_id);
        let body = unfav.body.unwrap_or_default();
        let client = self.client.clone();
        let delivered_workspace = workspace_id.to_owned();
        let delivered_block = block_id.to_owned();
        self.runtime.spawn(async move {
            let result = patch_expect_success(&client, &unfav.url, &body).await;
            if let Ok(mut queue) = cell.lock() {
                queue.push_back((
                    delivered_workspace,
                    workspace_epoch,
                    SectionKind::Favorites,
                    delivered_block,
                    sequence,
                    result.map_err(|e| e.to_string()),
                ));
            }
        });
    }
}

/// Parse one verified `LoomBlock` JSON object into a [`SidebarBlock`]. Identity, title, and content type
/// are required and nonblank; malformed rows fail the whole delivery so corrupt backend state cannot be
/// presented as a legitimate partial list.
fn required_sidebar_string(
    value: &serde_json::Value,
    field: &str,
    context: &str,
) -> Result<String, AppError> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|candidate| !candidate.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            AppError::Parse(format!(
                "{context} requires nonblank string field '{field}'"
            ))
        })
}

fn block_to_sidebar_block(
    block: &serde_json::Value,
    context: &str,
) -> Result<SidebarBlock, AppError> {
    let block_id = required_sidebar_string(block, "block_id", context)?;
    let optional_nonblank = |field: &str| -> Result<Option<String>, AppError> {
        match block.get(field) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(value) => {
                let candidate = value.as_str().ok_or_else(|| {
                    AppError::Parse(format!(
                        "{context} field '{field}' must be null or a string"
                    ))
                })?;
                let candidate = candidate.trim();
                Ok((!candidate.is_empty()).then(|| candidate.to_owned()))
            }
        }
    };
    let title = optional_nonblank("title")?
        .or(optional_nonblank("original_filename")?)
        .unwrap_or_else(|| block_id.clone());
    let content_type = required_sidebar_string(block, "content_type", context)?;
    Ok(SidebarBlock::new(block_id, title, content_type))
}

/// Strict parser for the canonical Loom Pins/Favorites view envelope. An empty `blocks` array is a
/// valid empty state; a missing envelope, mismatched view type, malformed row, or duplicate block id is
/// a typed error rather than a false empty list.
pub fn parse_sidebar_view_blocks(
    value: &serde_json::Value,
    expected_view_type: &str,
) -> Result<Vec<SidebarBlock>, AppError> {
    let view_type = required_sidebar_string(value, "view_type", "sidebar view envelope")?;
    if view_type != expected_view_type {
        return Err(AppError::Parse(format!(
            "sidebar view expected view_type '{expected_view_type}', got '{view_type}'"
        )));
    }
    let rows = value
        .get("blocks")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AppError::Parse("sidebar view requires array field 'blocks'".to_owned()))?;
    let mut seen = std::collections::HashSet::with_capacity(rows.len());
    let mut parsed = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let block = block_to_sidebar_block(row, &format!("sidebar view row[{index}]"))?;
        if !seen.insert(block.block_id.clone()) {
            return Err(AppError::Parse(format!(
                "sidebar view contains duplicate block_id '{}'",
                block.block_id
            )));
        }
        parsed.push(block);
    }
    Ok(parsed)
}

/// `GET {url}?{query}` and parse the verified `LoomViewResponse::{Pins,Favorites} { blocks }` shape into
/// [`SidebarBlock`]s. A present empty `blocks` array is the empty state; a missing/mismatched envelope is
/// an [`AppError`] surfaced by the AC9 error banner.
async fn fetch_view_blocks(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<Vec<SidebarBlock>, AppError> {
    let v = get_json(client, url, query).await?;
    let expected_view_type = if url.ends_with("/pins") {
        "pins"
    } else if url.ends_with("/favorites") {
        "favorites"
    } else {
        return Err(AppError::Parse(format!(
            "unsupported sidebar view URL '{url}'"
        )));
    };
    parse_sidebar_view_blocks(&v, expected_view_type)
}

/// `GET {url}` and parse the verified `Vec<LoomBacklink>` shape into [`BacklinkRow`]s. Each backlink is
/// `{ edge:{ edge_type, source_block_id, .. }, source_block:{ block_id, title, .. }, context_snippet }`.
/// The row's open key + title come from `source_block`; the label is `edge.edge_type` (AC4), and the
/// optional context snippet is retained. A present empty array is valid; malformed rows fail closed.
async fn fetch_backlink_rows(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<BacklinkRow>, AppError> {
    let v = get_json(client, url, &[]).await?;
    parse_sidebar_backlinks(&v)
}

/// Strict parser for the dedicated `Vec<LoomBacklink>` response. Missing rows are not silently
/// skipped, and duplicate source ids are rejected because they would collide in the AccessKit row
/// namespace and make a model action ambiguous.
pub fn parse_sidebar_backlinks(value: &serde_json::Value) -> Result<Vec<BacklinkRow>, AppError> {
    let rows = value
        .as_array()
        .ok_or_else(|| AppError::Parse("sidebar backlinks response must be an array".to_owned()))?;
    let mut seen = std::collections::HashSet::with_capacity(rows.len());
    let mut parsed = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let context = format!("sidebar backlink row[{index}]");
        let source = row.get("source_block").ok_or_else(|| {
            AppError::Parse(format!("{context} requires object field 'source_block'"))
        })?;
        let block_id = required_sidebar_string(source, "block_id", &context)?;
        if !seen.insert(block_id.clone()) {
            return Err(AppError::Parse(format!(
                "sidebar backlinks contains duplicate source block_id '{block_id}'"
            )));
        }
        let source_block = block_to_sidebar_block(source, &context)?;
        let title = source_block.title;
        let edge = row
            .get("edge")
            .ok_or_else(|| AppError::Parse(format!("{context} requires object field 'edge'")))?;
        let edge_type = required_sidebar_string(edge, "edge_type", &context)?;
        let edge_source_block_id = required_sidebar_string(edge, "source_block_id", &context)?;
        let _target_block_id = required_sidebar_string(edge, "target_block_id", &context)?;
        if edge_source_block_id != block_id {
            return Err(AppError::Parse(format!(
                "{context} edge.source_block_id '{edge_source_block_id}' does not match source_block.block_id '{block_id}'"
            )));
        }
        let context_snippet = match row.get("context_snippet") {
            None | Some(serde_json::Value::Null) => None,
            Some(value) => Some(
                value
                    .as_str()
                    .filter(|snippet| !snippet.trim().is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| {
                        AppError::Parse(format!(
                            "{context} field 'context_snippet' must be null or nonblank string"
                        ))
                    })?,
            ),
        };
        parsed.push(BacklinkRow::new(block_id, title, edge_type).with_context(context_snippet));
    }
    Ok(parsed)
}

/// `GET {url}` and parse the verified `Vec<LoomUnlinkedMention>` shape into [`UnlinkedRow`]s. Each mention
/// is `{ source_block:{ block_id, title, .. }, matched_term, snippet, match_offset }`; the row's open key
/// The row's open key and title come from `source_block` (AC5); matched term and snippet are retained.
/// A present empty array
/// is valid; malformed rows fail closed.
async fn fetch_unlinked_rows(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<UnlinkedRow>, AppError> {
    let v = get_json(client, url, &[]).await?;
    parse_sidebar_unlinked(&v)
}

/// Strict parser for `Vec<LoomUnlinkedMention>`, retaining the matched term and context snippet the
/// operator needs to judge whether to promote the textual mention to a real edge.
pub fn parse_sidebar_unlinked(value: &serde_json::Value) -> Result<Vec<UnlinkedRow>, AppError> {
    let rows = value
        .as_array()
        .ok_or_else(|| AppError::Parse("sidebar unlinked response must be an array".to_owned()))?;
    let mut seen = std::collections::HashSet::with_capacity(rows.len());
    let mut parsed = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let context = format!("sidebar unlinked row[{index}]");
        let source = row.get("source_block").ok_or_else(|| {
            AppError::Parse(format!("{context} requires object field 'source_block'"))
        })?;
        let block_id = required_sidebar_string(source, "block_id", &context)?;
        if !seen.insert(block_id.clone()) {
            return Err(AppError::Parse(format!(
                "sidebar unlinked response contains duplicate source block_id '{block_id}'"
            )));
        }
        let title = block_to_sidebar_block(source, &context)?.title;
        let matched_term = required_sidebar_string(row, "matched_term", &context)?;
        let snippet = required_sidebar_string(row, "snippet", &context)?;
        let match_offset = row
            .get("match_offset")
            .and_then(serde_json::Value::as_i64)
            .filter(|offset| *offset >= 0)
            .ok_or_else(|| {
                AppError::Parse(format!(
                    "{context} requires nonnegative integer field 'match_offset'"
                ))
            })?;
        let _ = match_offset;
        parsed.push(UnlinkedRow::new(block_id, title).with_match(matched_term, snippet));
    }
    Ok(parsed)
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-008 code-navigation transport (REUSE — not a second HTTP stack).
//
// The native code-editor's CodeNavClient (`code_editor::code_nav`) binds the EXISTING handshake_core
// code-nav GET routes. Those routes (`api::knowledge_code_nav`) require four backend-navigation
// identity headers on EVERY request, verified READ-ONLY against the running backend:
//   x-hsk-actor-id, x-hsk-kernel-task-run-id, x-hsk-session-run-id, x-hsk-actor-kind.
// A missing header is a deterministic HTTP 400 ("<header> header is required"), so the transport must
// attach them or the bind silently 400s. `actor-kind: system` is the verified valid kind for an
// automated UI nav (the same kind the backend's own quiet-nav lane uses). The shared `code_nav_get`
// helper below adds the headers + parses the JSON body via `serde_json::Value`, reusing the SAME
// `reqwest`/timeout/error shape as every other client in this module (NO new HTTP stack, NO dependency
// on the handshake_core crate). CodeNavClient calls THIS helper for all four routes.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// The backend-navigation identity headers required on every code-nav GET (verified against
/// `handshake_core::api::knowledge_code_nav::nav_context`) AND on every knowledge-document request
/// (verified against `handshake_core::api::knowledge_documents::doc_context`). A missing header is a
/// hard 400 ("<header> header is required"). `pub` so the rich-editor save/draft transport reuses the
/// SAME canonical header names rather than re-deriving the strings (the MT-020 missing-headers fix).
pub const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
pub const HSK_HEADER_KERNEL_TASK_RUN_ID: &str = "x-hsk-kernel-task-run-id";
pub const HSK_HEADER_SESSION_RUN_ID: &str = "x-hsk-session-run-id";
pub const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";

/// The stable actor identity the native editor presents to the backend code-nav API. `system` is the
/// verified-valid `x-hsk-actor-kind` for an automated UI navigation (the backend maps it to
/// `KernelActor::System`); the actor id names the native editor surface so the nav receipts are
/// attributable to it (HBR-SWARM attribution).
pub const CODE_NAV_ACTOR_ID: &str = "handshake-native-editor";
pub const CODE_NAV_ACTOR_KIND: &str = "system";

/// The stable identity the native editor presents to the backend KNOWLEDGE-DOCUMENT API (save +
/// draft). `operator` is the verified-valid `x-hsk-actor-kind` for an operator-initiated document
/// edit: the MT-158 permission matrix (`knowledge_document::permission`) grants `operator` the
/// `Write` action, so a save (`PUT /save`), a draft upsert (`PUT /draft`), and a draft clear
/// (`DELETE /draft`) are permitted. A MISSING `x-hsk-actor-kind` defaults to the least-privileged
/// (read-only) kind server-side and a write then 403s — so the kind MUST be asserted. The actor id
/// names the native editor surface so the document receipts are attributable to it (HBR-SWARM).
pub const DOC_ACTOR_ID: &str = "handshake-native-editor";
pub const DOC_ACTOR_KIND: &str = "operator";

/// Stable attributable identity for saved block-collection reads and writes. The Loom routes accept
/// the same canonical `x-hsk-*` identity vocabulary as the adjacent knowledge surfaces; using an
/// operator actor kind keeps create/update/card-move actions write-capable without changing the pure
/// request-builder test seam.
pub const BLOCK_VIEW_ACTOR_ID: &str = "handshake-native-block-collection-view";
pub const BLOCK_VIEW_ACTOR_KIND: &str = "operator";

/// `GET {url}?{query}` against the code-nav API with the four required backend-nav identity headers
/// attached, returning the parsed JSON body. `run_id` is folded into the per-request run ids so each
/// editor nav action is individually traceable (it never reaches the wrong field — the headers are
/// fixed names). A non-success status or a parse failure is an [`AppError`], never a panic — the
/// CodeNavClient turns that into graceful empty results (no completion / no hover), so the editor keeps
/// working when the backend is down (AC-004 graceful-degradation analog for the code-nav path).
///
/// REUSE: the process-wide bounded backend pool. The editor calls this from a spawned tokio task
/// (HBR-QUIET — never the egui UI thread), so a slow request never stalls the operator.
pub async fn code_nav_get(
    url: &str,
    query: &[(String, String)],
    run_id: &str,
) -> Result<serde_json::Value, AppError> {
    let client = shared_http_client();
    let resp = client
        .get(url)
        .query(query)
        .header(HSK_HEADER_ACTOR_ID, CODE_NAV_ACTOR_ID)
        .header(HSK_HEADER_ACTOR_KIND, CODE_NAV_ACTOR_KIND)
        .header(
            HSK_HEADER_KERNEL_TASK_RUN_ID,
            format!("native-editor-{run_id}"),
        )
        .header(
            HSK_HEADER_SESSION_RUN_ID,
            format!("native-editor-session-{run_id}"),
        )
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "GET code-nav non-success status {}",
            resp.status()
        )));
    }
    resp.json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-025 Loom WIKI-PROJECTION transport (REUSE — extends the MT-021/022/024 Loom read
// surface; mirrors the `LoomGraphClient` / `LoomFolderClient` / `LoomSidebarClient` shape exactly:
// off-thread spawn + one-slot delivery cell, speaks `serde_json::Value`, NEVER depends on the
// `handshake_core` crate types).
//
// SPEC-REALISM GATE (the MT-025 KERNEL_BUILDER gate + the MT-008/021/022/023/024 "verify, don't trust
// the contract" rule). VERIFIED READ-ONLY against the running backend
// `src/backend/handshake_core/src/{api,storage}/loom.rs`:
//   - `GET    /workspaces/{ws}/loom/wiki/{projection_id}`            -> `ServedWikiPage` =
//       `LoomWikiProjection { projection_id, workspace_id, title, source_block_ids[], rendered_content,
//       staleness_hash, rebuild_status, page_type?, compile_stamp?, page_links, created_at, updated_at }`
//       FLATTENED with `staleness_verdict` (the MT-242 LM-PWIKI-008 fail-closed verdict). CONFIRMED real
//       — the field shape the MT assumed EXISTS (handler `get_loom_wiki_projection` -> `ServedWikiPage`).
//   - `POST   /workspaces/{ws}/loom/wiki/{projection_id}/regenerate` -> `ServedWikiPage` (handler
//       `regenerate_loom_wiki_projection`). This is the REAL "rebuild" route — NOT the contract's assumed
//       `.../rebuild`. A regenerate recompiles `rendered_content` FROM `source_block_ids` and re-stamps.
//   - `POST   /workspaces/{ws}/loom/wiki/{projection_id}/overlays`   body `{ "annotation", "anchor"? }`
//       -> `LoomWikiOverlay { overlay_id, projection_id, workspace_id, annotation, anchor?, .. }` (handler
//       `add_loom_wiki_overlay`). This is the REAL, PERSISTED, CANONICAL write surface for a wiki page.
//
// THE CRITICAL FINDING (MC-1 / RISK-1, the contract's own doubt confirmed): there is **NO PATCH or PUT
// route that edits `rendered_content`**. The backend storage comment is explicit — `rendered_content` is
// "The rendered wiki markdown (regenerable; never authority)"; it is a DERIVED projection compiled FROM
// `source_block_ids` and is OVERWRITTEN on every regenerate. The ONLY canonical write is an OVERLAY
// annotation, stored in its OWN authority row precisely so "editing it never makes the projection
// canonical" (storage::LoomWikiOverlay doc). Therefore the native panel ships the "Edit overlay" as the
// REAL overlay-annotation write (POST .../overlays) and keeps `rendered_content` READ-ONLY — never a fake
// PATCH that would 404 or be silently clobbered on the next rebuild (Spec-Realism: no silently-broken
// write). The contract's PATCH/PUT-on-rendered_content path is a TYPED LIMITATION, surfaced in the widget.

/// A parsed Loom wiki projection (the `ServedWikiPage` GET/regenerate body), holding ONLY the fields the
/// native panel reads. `staleness_verdict` is the raw flattened verdict object (`serde_json::Value`,
/// typed `unknown` in the React API + `serde_json::Value` here per the MT note) so the "stale" display
/// logic can treat any non-null/non-`{"state":"fresh"}` value as stale without coupling to the verdict
/// schema. Parsing is strict: the response must contain every required projection field and its
/// workspace/projection identity must match the request. A malformed or cross-resource response is an
/// error, never a fabricated page.
#[derive(Debug, Clone, PartialEq)]
pub struct WikiProjection {
    pub projection_id: String,
    pub workspace_id: String,
    pub title: String,
    pub source_block_ids: Vec<String>,
    pub rendered_content: String,
    pub staleness_hash: String,
    pub rebuild_status: String,
    /// Canonical projection-row timestamps. `updated_at` is the source-projection revision bound into
    /// wiki action receipts; it is not inferred from a render or local clock.
    pub created_at: String,
    pub updated_at: String,
    pub page_type: Option<String>,
    /// Persisted operator annotations loaded from the canonical overlay-list route after every
    /// projection GET/regenerate. Keeping them on the delivered snapshot makes the mounted panel—not a
    /// test-only HTTP client—the product surface that proves a saved overlay survived reload.
    pub overlays: Vec<WikiOverlay>,
    /// The raw flattened `staleness_verdict` object (or `Null` when absent). The display treats any
    /// non-null value whose `state` is not `"fresh"` as STALE (the MT RISK-5/MC-5 "treat any non-null
    /// non-fresh verdict as stale" rule; the React type is `unknown`).
    pub staleness_verdict: serde_json::Value,
}

impl WikiProjection {
    fn from_json(
        v: &serde_json::Value,
        requested_workspace_id: &str,
        requested_projection_id: &str,
    ) -> Result<Self, AppError> {
        let required_string = |key: &str| -> Result<String, AppError> {
            v.get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| AppError::Parse(format!("WikiProjection.{key} must be a string")))
        };
        let projection_id = required_string("projection_id")?;
        let workspace_id = required_string("workspace_id")?;
        if projection_id != requested_projection_id || workspace_id != requested_workspace_id {
            return Err(AppError::Parse(format!(
                "wiki response identity mismatch: requested {requested_workspace_id}/{requested_projection_id}, received {workspace_id}/{projection_id}"
            )));
        }
        let source_rows = v
            .get("source_block_ids")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                AppError::Parse("WikiProjection.source_block_ids must be an array".to_owned())
            })?;
        let mut source_block_ids = Vec::with_capacity(source_rows.len());
        for (index, value) in source_rows.iter().enumerate() {
            source_block_ids.push(
                value
                    .as_str()
                    .filter(|id| !id.trim().is_empty())
                    .ok_or_else(|| {
                        AppError::Parse(format!(
                            "WikiProjection.source_block_ids[{index}] must be a non-empty string"
                        ))
                    })?
                    .to_owned(),
            );
        }
        let page_type = match v.get("page_type") {
            None | Some(Value::Null) => None,
            Some(Value::String(value)) if !value.trim().is_empty() => Some(value.clone()),
            _ => {
                return Err(AppError::Parse(
                    "WikiProjection.page_type must be null or a non-empty string".to_owned(),
                ));
            }
        };
        let staleness_verdict = v.get("staleness_verdict").cloned().ok_or_else(|| {
            AppError::Parse("WikiProjection.staleness_verdict is required".to_owned())
        })?;
        Ok(WikiProjection {
            projection_id,
            workspace_id,
            title: required_string("title")?,
            source_block_ids,
            rendered_content: required_string("rendered_content")?,
            staleness_hash: required_string("staleness_hash")?,
            rebuild_status: required_string("rebuild_status")?,
            created_at: required_string("created_at")?,
            updated_at: required_string("updated_at")?,
            page_type,
            overlays: Vec::new(),
            staleness_verdict,
        })
    }
}

/// One persisted annotation returned by `GET .../overlays`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiOverlay {
    pub overlay_id: String,
    pub projection_id: String,
    pub workspace_id: String,
    pub annotation: String,
    pub anchor: Option<String>,
    /// Canonical persisted overlay-row revision returned by POST and confirmed by GET readback.
    pub created_at: String,
    pub updated_at: String,
}

fn parse_wiki_overlay(
    row: &Value,
    requested_workspace_id: &str,
    requested_projection_id: &str,
    location: &str,
) -> Result<WikiOverlay, AppError> {
    let required = |key: &str| -> Result<String, AppError> {
        row.get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| AppError::Parse(format!("{location}.{key} must be a string")))
    };
    let overlay_id = required("overlay_id")?;
    let projection_id = required("projection_id")?;
    let workspace_id = required("workspace_id")?;
    if projection_id != requested_projection_id || workspace_id != requested_workspace_id {
        return Err(AppError::Parse(format!(
            "wiki overlay identity mismatch at {location}: requested {requested_workspace_id}/{requested_projection_id}, received {workspace_id}/{projection_id}"
        )));
    }
    let anchor = match row.get("anchor") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => Some(value.clone()),
        _ => {
            return Err(AppError::Parse(format!(
                "{location}.anchor must be null or a string"
            )))
        }
    };
    Ok(WikiOverlay {
        overlay_id,
        projection_id,
        workspace_id,
        annotation: required("annotation")?,
        anchor,
        created_at: required("created_at")?,
        updated_at: required("updated_at")?,
    })
}

fn parse_wiki_overlays(
    value: &Value,
    requested_workspace_id: &str,
    requested_projection_id: &str,
) -> Result<Vec<WikiOverlay>, AppError> {
    let rows = value
        .as_array()
        .ok_or_else(|| AppError::Parse("wiki overlays response must be an array".to_owned()))?;
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            parse_wiki_overlay(
                row,
                requested_workspace_id,
                requested_projection_id,
                &format!("WikiOverlay[{index}]"),
            )
        })
        .collect()
}

/// One-slot delivery cell for an off-thread wiki-projection GET/regenerate result. `Ok(projection)`
/// carries the parsed page the panel renders; `Err(msg)` the failure the panel surfaces (AC8).
pub type WikiProjectionCell = Arc<Mutex<Option<Result<WikiProjection, String>>>>;

/// Identity stamped by the mounted wiki pane onto every asynchronous load/save/regenerate request.
/// The workspace and projection prevent cross-resource delivery; `pane_generation` also rejects an
/// older A completion after the operator navigates A -> B -> A.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiPaneIdentity {
    pub workspace_id: String,
    pub projection_id: String,
    pub pane_generation: u64,
}

/// Which projection operation produced a delivery. A post-save reload is distinct because the edit
/// buffer must not be cleared until that reload succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WikiProjectionOperation {
    Load,
    Regenerate,
    ReloadAfterSave { action_generation: u64 },
}

#[derive(Debug)]
pub struct WikiProjectionDelivery {
    pub identity: WikiPaneIdentity,
    pub operation: WikiProjectionOperation,
    pub result: Result<WikiProjection, String>,
}

pub type WikiProjectionDeliveryCell = Arc<Mutex<VecDeque<WikiProjectionDelivery>>>;

#[derive(Debug)]
pub struct WikiSaveDelivery {
    pub identity: WikiPaneIdentity,
    pub action_generation: u64,
    pub result: Result<WikiOverlay, String>,
}

pub type WikiSaveDeliveryCell = Arc<Mutex<VecDeque<WikiSaveDelivery>>>;

/// REST client for the VERIFIED Loom wiki-projection surface the MT-025 wiki page panel binds:
/// `GET /loom/wiki/{id}` (load), `POST /loom/wiki/{id}/regenerate` (rebuild), and
/// `POST /loom/wiki/{id}/overlays` (the REAL persisted overlay-annotation write — the "Edit overlay"
/// mechanism, since `rendered_content` itself has NO edit route). Mirrors the `LoomGraphClient` shape.
#[derive(Clone)]
pub struct LoomWikiClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomWikiClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn wiki_url(&self, workspace_id: &str, projection_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/wiki/{}",
            self.base_url, workspace_id, projection_id
        )
    }

    fn overlays_url(&self, workspace_id: &str, projection_id: &str) -> String {
        format!("{}/overlays", self.wiki_url(workspace_id, projection_id))
    }

    /// Pure request builder for the wiki-page LOAD: `GET /loom/wiki/{id}` (no query). Split out so a unit
    /// test asserts the EXACT verified URL without a live backend (the spawn path routes through this same
    /// builder, so the test proves the production request construction — PROOF2 request-shape layer).
    pub fn load_request(&self, workspace_id: &str, projection_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.wiki_url(workspace_id, projection_id),
            query: vec![],
        }
    }

    /// Pure request builder for the REBUILD: `POST /loom/wiki/{id}/regenerate` (no body). This is the REAL
    /// route (`regenerate_loom_wiki_projection`) — the contract's assumed `.../rebuild` does NOT exist.
    pub fn regenerate_request(&self, workspace_id: &str, projection_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!("{}/regenerate", self.wiki_url(workspace_id, projection_id)),
            body: None,
        }
    }

    /// Pure request builder for the OVERLAY-ANNOTATION write (the REAL "Edit overlay" persistence):
    /// `POST /loom/wiki/{id}/overlays` body `{ "annotation": <text> }` (+ optional `anchor`). This is the
    /// ONLY canonical wiki-page write (`add_loom_wiki_overlay`); `rendered_content` itself is read-only.
    /// PROOF3 asserts this exact `(POST, url, body)` is what the Save spawn path sends.
    pub fn add_overlay_request(
        &self,
        workspace_id: &str,
        projection_id: &str,
        annotation: &str,
        anchor: Option<&str>,
    ) -> RequestSpec {
        let mut body = serde_json::Map::new();
        body.insert(
            "annotation".to_owned(),
            serde_json::Value::String(annotation.to_owned()),
        );
        if let Some(anchor) = anchor.filter(|a| !a.is_empty()) {
            body.insert(
                "anchor".to_owned(),
                serde_json::Value::String(anchor.to_owned()),
            );
        }
        RequestSpec {
            method: HttpMethod::Post,
            url: format!("{}/overlays", self.wiki_url(workspace_id, projection_id)),
            body: Some(serde_json::Value::Object(body)),
        }
    }

    /// Fetch one wiki projection off the UI thread, delivering the parsed page into `cell` (AC1 load). The
    /// host sets `loading=true` before calling (so the spinner animates ONLY during this genuine in-flight
    /// fetch — the MT-015 idle-repaint rule) and clears it on delivery.
    pub fn fetch_projection(
        &self,
        workspace_id: &str,
        projection_id: &str,
        cell: WikiProjectionCell,
    ) {
        let spec = self.load_request(workspace_id, projection_id);
        let overlays_url = self.overlays_url(workspace_id, projection_id);
        let client = self.client.clone();
        let workspace_id = workspace_id.to_owned();
        let pid = projection_id.to_owned();
        self.runtime.spawn(async move {
            let result =
                fetch_wiki_projection(&client, &spec.url, &overlays_url, &workspace_id, &pid).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Mounted-host load with complete response identity. Unlike the compatibility one-slot method,
    /// this FIFO cannot let an older completion overwrite a newer one before host filtering runs.
    pub fn fetch_projection_stamped(
        &self,
        identity: WikiPaneIdentity,
        operation: WikiProjectionOperation,
        cell: WikiProjectionDeliveryCell,
    ) {
        let spec = self.load_request(&identity.workspace_id, &identity.projection_id);
        let overlays_url = self.overlays_url(&identity.workspace_id, &identity.projection_id);
        let client = self.client.clone();
        let requested_workspace_id = identity.workspace_id.clone();
        let requested_id = identity.projection_id.clone();
        self.runtime.spawn(async move {
            let result = fetch_wiki_projection(
                &client,
                &spec.url,
                &overlays_url,
                &requested_workspace_id,
                &requested_id,
            )
            .await
            .map_err(|e| e.to_string());
            if let Ok(mut queue) = cell.lock() {
                queue.push_back(WikiProjectionDelivery {
                    identity,
                    operation,
                    result,
                });
            }
        });
    }

    /// Regenerate (rebuild) the projection off the UI thread, delivering the REBUILT page into `cell`
    /// (the optional Rebuild button). The POST returns the fresh `ServedWikiPage`, parsed like the GET.
    pub fn regenerate_projection(
        &self,
        workspace_id: &str,
        projection_id: &str,
        cell: WikiProjectionCell,
    ) {
        let spec = self.regenerate_request(workspace_id, projection_id);
        let overlays_url = self.overlays_url(workspace_id, projection_id);
        let client = self.client.clone();
        let workspace_id = workspace_id.to_owned();
        let pid = projection_id.to_owned();
        self.runtime.spawn(async move {
            let result =
                post_wiki_regenerate(&client, &spec.url, &overlays_url, &workspace_id, &pid).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Mounted-host regenerate with complete response identity and FIFO delivery.
    pub fn regenerate_projection_stamped(
        &self,
        identity: WikiPaneIdentity,
        cell: WikiProjectionDeliveryCell,
    ) {
        let spec = self.regenerate_request(&identity.workspace_id, &identity.projection_id);
        let overlays_url = self.overlays_url(&identity.workspace_id, &identity.projection_id);
        let client = self.client.clone();
        let requested_workspace_id = identity.workspace_id.clone();
        let requested_id = identity.projection_id.clone();
        self.runtime.spawn(async move {
            let result = post_wiki_regenerate(
                &client,
                &spec.url,
                &overlays_url,
                &requested_workspace_id,
                &requested_id,
            )
            .await
            .map_err(|e| e.to_string());
            if let Ok(mut queue) = cell.lock() {
                queue.push_back(WikiProjectionDelivery {
                    identity,
                    operation: WikiProjectionOperation::Regenerate,
                    result,
                });
            }
        });
    }

    /// Add an overlay annotation off the UI thread (the REAL "Save" of the Edit overlay), delivering the
    /// outcome into `cell`. `Ok(())` on a 2xx; `Err(msg)` on failure (AC5/PROOF5 — the host keeps the edit
    /// buffer and shows the error inline). The host re-fetches the projection on success (AC3).
    pub fn add_overlay(
        &self,
        workspace_id: &str,
        projection_id: &str,
        annotation: &str,
        anchor: Option<&str>,
        cell: ScmReceiptCell,
    ) {
        let spec = self.add_overlay_request(workspace_id, projection_id, annotation, anchor);
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = post_expect_success(&client, &spec.url, &body).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Mounted-host overlay save with complete response identity and FIFO delivery.
    pub fn add_overlay_stamped(
        &self,
        identity: WikiPaneIdentity,
        action_generation: u64,
        annotation: &str,
        anchor: Option<&str>,
        cell: WikiSaveDeliveryCell,
    ) {
        let spec = self.add_overlay_request(
            &identity.workspace_id,
            &identity.projection_id,
            annotation,
            anchor,
        );
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = post_json_expect_value(&client, &spec.url, &body, Duration::from_secs(5))
                .await
                .and_then(|value| {
                    parse_wiki_overlay(
                        &value,
                        &identity.workspace_id,
                        &identity.projection_id,
                        "WikiOverlay.POST",
                    )
                })
                .map_err(|e| e.to_string());
            if let Ok(mut queue) = cell.lock() {
                queue.push_back(WikiSaveDelivery {
                    identity,
                    action_generation,
                    result,
                });
            }
        });
    }
}

/// `GET {url}` and parse the verified `ServedWikiPage` into a [`WikiProjection`]. A non-success status or
/// parse failure is an [`AppError`] (AC8). `requested_id` is the GET's projection id (the parse fallback).
async fn fetch_wiki_projection(
    client: &reqwest::Client,
    url: &str,
    overlays_url: &str,
    requested_workspace_id: &str,
    requested_projection_id: &str,
) -> Result<WikiProjection, AppError> {
    let v = get_json(client, url, &[]).await?;
    let mut projection =
        WikiProjection::from_json(&v, requested_workspace_id, requested_projection_id)?;
    let overlays = get_json(client, overlays_url, &[]).await?;
    projection.overlays =
        parse_wiki_overlays(&overlays, requested_workspace_id, requested_projection_id)?;
    Ok(projection)
}

/// `POST {url}` (no body) for the regenerate route and parse the rebuilt `ServedWikiPage`. A non-success
/// status or parse failure is an [`AppError`].
async fn post_wiki_regenerate(
    client: &reqwest::Client,
    url: &str,
    overlays_url: &str,
    requested_workspace_id: &str,
    requested_projection_id: &str,
) -> Result<WikiProjection, AppError> {
    let resp = client
        .post(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "POST regenerate non-success status {}",
            resp.status()
        )));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))?;
    let mut projection =
        WikiProjection::from_json(&v, requested_workspace_id, requested_projection_id)?;
    let overlays = get_json(client, overlays_url, &[]).await?;
    projection.overlays =
        parse_wiki_overlays(&overlays, requested_workspace_id, requested_projection_id)?;
    Ok(projection)
}

#[cfg(test)]
mod wiki_client_tests {
    use super::*;

    fn client() -> LoomWikiClient {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        // Leak the runtime handle for the test's lifetime (the builders are pure; no task is spawned).
        let handle = rt.handle().clone();
        std::mem::forget(rt);
        LoomWikiClient::new("http://test.local:1234", handle)
    }

    /// PROOF2 (request layer) / AC1: the LOAD hits the verified `GET /loom/wiki/{id}` route.
    #[test]
    fn load_request_hits_verified_get_route() {
        let spec = client().load_request("ws1", "proj-001");
        assert_eq!(spec.method, HttpMethod::Get);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/wiki/proj-001"
        );
        assert!(spec.query.is_empty());
    }

    /// AC: the REBUILD hits the verified `POST /loom/wiki/{id}/regenerate` (NOT the contract's
    /// non-existent `.../rebuild`), bodyless.
    #[test]
    fn regenerate_request_hits_verified_regenerate_route() {
        let spec = client().regenerate_request("ws1", "proj-001");
        assert_eq!(spec.method, HttpMethod::Post);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/wiki/proj-001/regenerate"
        );
        assert_eq!(spec.body, None);
    }

    /// PROOF3 (request layer) / AC3: the overlay-annotation SAVE hits the verified
    /// `POST /loom/wiki/{id}/overlays` route with the verified `{ "annotation": <text> }` body — the REAL
    /// persisted wiki-page write (NOT a fake PATCH on rendered_content).
    #[test]
    fn add_overlay_request_hits_verified_overlays_route() {
        let spec = client().add_overlay_request("ws1", "proj-001", "NEW CONTENT", None);
        assert_eq!(spec.method, HttpMethod::Post);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/wiki/proj-001/overlays"
        );
        assert_eq!(
            spec.body,
            Some(serde_json::json!({ "annotation": "NEW CONTENT" }))
        );
    }

    /// The optional anchor is included only when non-empty (a true merge — never sends `anchor:""`).
    #[test]
    fn add_overlay_request_includes_anchor_when_present() {
        let spec = client().add_overlay_request("ws1", "proj-001", "note", Some("block-7"));
        assert_eq!(
            spec.body,
            Some(serde_json::json!({ "annotation": "note", "anchor": "block-7" }))
        );
        let spec_empty = client().add_overlay_request("ws1", "proj-001", "note", Some(""));
        assert_eq!(
            spec_empty.body,
            Some(serde_json::json!({ "annotation": "note" }))
        );
    }

    /// AC1 parse: the verified `ServedWikiPage` shape parses strictly into [`WikiProjection`], including
    /// the flattened `staleness_verdict` and request identity.
    #[test]
    fn parses_served_wiki_page_shape() {
        let body = serde_json::json!({
            "projection_id": "proj-001",
            "workspace_id": "ws1",
            "title": "Ownership model",
            "source_block_ids": ["blk-1", "blk-2", "blk-3"],
            "rendered_content": "# Ownership\nBorrow checker notes.",
            "staleness_hash": "abc123",
            "rebuild_status": "fresh",
            "page_type": "concept",
            "page_links": [],
            "created_at": "2026-06-19T00:00:00Z",
            "updated_at": "2026-06-19T00:00:00Z",
            "staleness_verdict": { "state": "fresh", "stamp_ledger_version": 7 }
        });
        let p = WikiProjection::from_json(&body, "ws1", "proj-001").unwrap();
        assert_eq!(p.projection_id, "proj-001");
        assert_eq!(p.title, "Ownership model");
        assert_eq!(p.source_block_ids.len(), 3);
        assert_eq!(p.rendered_content, "# Ownership\nBorrow checker notes.");
        assert_eq!(p.rebuild_status, "fresh");
        assert_eq!(p.created_at, "2026-06-19T00:00:00Z");
        assert_eq!(p.updated_at, "2026-06-19T00:00:00Z");
        assert_eq!(p.page_type.as_deref(), Some("concept"));
        assert_eq!(p.staleness_verdict["state"], "fresh");
    }

    /// Missing fields are rejected instead of fabricating an accepted projection.
    #[test]
    fn parse_rejects_missing_fields() {
        let error = WikiProjection::from_json(&serde_json::json!({}), "ws1", "proj-xyz")
            .unwrap_err()
            .to_string();
        assert!(error.contains("projection_id"));
    }

    #[test]
    fn parse_rejects_mismatched_projection_identity() {
        let body = serde_json::json!({
            "projection_id": "proj-other",
            "workspace_id": "ws1",
            "title": "Wrong page",
            "source_block_ids": [],
            "rendered_content": "wrong",
            "staleness_hash": "hash",
            "rebuild_status": "fresh",
            "page_type": null,
            "staleness_verdict": { "state": "fresh" }
        });
        let error = WikiProjection::from_json(&body, "ws1", "proj-xyz")
            .unwrap_err()
            .to_string();
        assert!(error.contains("identity mismatch"));
    }

    #[test]
    fn parse_rejects_mismatched_workspace_identity() {
        let body = serde_json::json!({
            "projection_id": "proj-xyz",
            "workspace_id": "ws-other",
            "title": "Wrong workspace",
            "source_block_ids": [],
            "rendered_content": "wrong",
            "staleness_hash": "hash",
            "rebuild_status": "fresh",
            "page_type": null,
            "staleness_verdict": { "state": "fresh" }
        });
        let error = WikiProjection::from_json(&body, "ws1", "proj-xyz")
            .unwrap_err()
            .to_string();
        assert!(error.contains("identity mismatch"));
    }

    #[test]
    fn overlay_parse_rejects_mismatched_projection_identity() {
        let value = serde_json::json!([{
            "overlay_id": "ov-1",
            "projection_id": "proj-other",
            "workspace_id": "ws1",
            "annotation": "must not cross projections",
            "anchor": null
        }]);
        let error = parse_wiki_overlays(&value, "ws1", "proj-001")
            .unwrap_err()
            .to_string();
        assert!(error.contains("overlay identity mismatch"));
    }

    #[test]
    fn overlay_parse_retains_canonical_persisted_revision() {
        let value = serde_json::json!([{
            "overlay_id": "ov-1",
            "projection_id": "proj-001",
            "workspace_id": "ws1",
            "annotation": "persisted note",
            "anchor": null,
            "created_at": "2026-06-19T01:00:00Z",
            "updated_at": "2026-06-19T01:00:01Z"
        }]);
        let parsed = parse_wiki_overlays(&value, "ws1", "proj-001").unwrap();
        assert_eq!(parsed[0].overlay_id, "ov-1");
        assert_eq!(parsed[0].created_at, "2026-06-19T01:00:00Z");
        assert_eq!(parsed[0].updated_at, "2026-06-19T01:00:01Z");
    }

    #[test]
    fn overlay_parse_rejects_mismatched_workspace_identity() {
        let value = serde_json::json!([{
            "overlay_id": "ov-1",
            "projection_id": "proj-001",
            "workspace_id": "ws-other",
            "annotation": "must not cross workspaces",
            "anchor": null
        }]);
        let error = parse_wiki_overlays(&value, "ws1", "proj-001")
            .unwrap_err()
            .to_string();
        assert!(error.contains("overlay identity mismatch"));
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-027 BlockCollectionViews client: the VERIFIED saved-view surface (table / Kanban / calendar).
//
// Verified READ-ONLY against `src/backend/handshake_core/src/{api,storage}/loom.rs` + `app/src/lib/
// api.ts` (the MT-022/023/024/026 lesson — bind only confirmed shapes):
//   - GET    /workspaces/:ws/loom/views/definitions/:block_id          getBlockView -> BlockViewRecord
//   - POST   /workspaces/:ws/loom/views/definitions/:block_id/results  queryBlockViewResults
//       body {limit,offset} -> BlockViewResults{kind,blocks,groups,total_returned}  (POST, RISK-1)
//   - PATCH  /workspaces/:ws/loom/views/definitions/:block_id          updateBlockView body {definition}
//   - PATCH  /workspaces/:ws/loom/blocks/:block_id                     updateLoomBlock body
//       {add_tags,remove_tags} (top-level alongside the flattened update) — Kanban lane move
//   - POST   /workspaces/:ws/loom/views/definitions                    createBlockView body
//       {block_id?,title?,definition}
//
// Mirrors the `CanvasBoardClient` shape: pure `*_request` builders return a [`RequestSpec`] /
// [`GetRequestSpec`] (unit-testable WITHOUT a backend), and the off-thread fetch/dispatch methods
// deliver into `Arc<Mutex<Option<..>>>` cells the egui UI drains next frame (HBR-QUIET). Speaks
// `serde_json::Value` so it never depends on the `handshake_core` crate.
//
// FIELD-TYPE VERIFICATION (adversarial-review hardening, must-fix #1/#2/#3 — the route+method match
// was not enough; the query field VALUE TYPES and the group_by lane dependency had drifted):
//   - `BlockViewQuery.date_from/date_to` are backend type `Option<DateTime<Utc>>` with the DEFAULT
//     chrono serde (RFC3339, full timestamp). `definition_to_json` EXPANDS the calendar `YYYY-MM-DD`
//     to `<date>T00:00:00Z` / `<date>T23:59:59Z` so the PATCH body actually deserializes (a bare date
//     would 400/422). `date_serializes_as_rfc3339_*` prove the produced strings parse as `DateTime<Utc>`
//     (the SAME type+serde the backend field uses) — an adapter-boundary check, not a self-tautology.
//   - The native `BlockViewDefinition`/`BlockViewQuery` now model the FULL backend query
//     (content_type/mime/tag_ids/mention_ids) + `group_by` ({"kind":"tag"} | {"kind":"field","field"}),
//     so a sort/kind/date `updateBlockView` — which the backend persists as a FULL overwrite of
//     `view_definition_json` — never silently drops a server-side filter or a Kanban lane grouping.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use crate::graph::block_collection_view::{
    BlockViewDefinition, BlockViewField, BlockViewGroupBy, BlockViewKind, BlockViewLane,
    BlockViewQuery, BlockViewResults, BlockViewSort, BlockViewSortDirection, LoomBlockRow,
};

/// The parsed result of a `getBlockView` fetch: the loaded definition + the view block id (so the host
/// can confirm identity). The block's own fields are not modeled here — only the definition the
/// sub-views need (the full `LoomBlock` is not required by the MT-027 surfaces).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockViewRecordData {
    pub view_block_id: String,
    pub definition: BlockViewDefinition,
}

/// One-slot delivery cell for an off-thread `getBlockView` result.
pub type BlockViewRecordCell = Arc<Mutex<Option<Result<BlockViewRecordData, String>>>>;

/// One-slot delivery cell for an off-thread `queryBlockViewResults` result.
pub type BlockViewResultsCell = Arc<Mutex<Option<Result<BlockViewResults, String>>>>;

/// One-slot delivery cell for an off-thread view MUTATION (updateBlockView / updateLoomBlock /
/// createBlockView). `Ok(view_block_id)` carries the (possibly new) view block id the host should be
/// on after the mutation; `Err(msg)` the failure. For a create, this is the NEW block id (so the host
/// switches to it); for update/card-move it echoes the current id.
/// Identity-stamped completion for a block-view mutation or create operation. A mutation names the
/// view that was bound when it started; create uses `None` because success intentionally switches to a
/// newly minted id. The mounted host accepts a delivery only while workspace + generation + intended
/// binding are still current.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockViewOpDelivery {
    pub workspace_id: String,
    pub generation: u64,
    pub expected_bound_view_id: Option<String>,
    pub result: Result<String, String>,
}

pub type BlockViewOpCell = Arc<Mutex<Option<BlockViewOpDelivery>>>;

/// REST client for the VERIFIED MT-262 block-collection-view surface. Drives the definition read, the
/// query (POST!), the sort/kind/date persist, the Kanban card-move tag mutation, and view creation off
/// the UI thread.
#[derive(Clone)]
pub struct BlockViewClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl BlockViewClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn definitions_url(&self, workspace_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/views/definitions",
            self.base_url, workspace_id
        )
    }

    fn definition_url(&self, workspace_id: &str, view_block_id: &str) -> String {
        format!("{}/{}", self.definitions_url(workspace_id), view_block_id)
    }

    fn block_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/blocks/{}",
            self.base_url, workspace_id, block_id
        )
    }

    /// Pure request builder for `GET .../views/definitions/:block_id` (getBlockView).
    pub fn get_view_request(&self, workspace_id: &str, view_block_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.definition_url(workspace_id, view_block_id),
            query: vec![],
        }
    }

    /// Pure request builder for `POST .../views/definitions/:block_id/results` (queryBlockViewResults).
    /// The VERIFIED method is POST with a JSON body `{limit, offset}` — NOT a GET with query params
    /// (RISK-1 / MC-1: a GET would 405 or silently send params as a query string).
    pub fn query_results_request(
        &self,
        workspace_id: &str,
        view_block_id: &str,
        limit: u32,
        offset: u32,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!(
                "{}/results",
                self.definition_url(workspace_id, view_block_id)
            ),
            body: Some(serde_json::json!({ "limit": limit, "offset": offset })),
        }
    }

    /// Pure request builder for `PATCH .../views/definitions/:block_id` (updateBlockView). The VERIFIED
    /// body is `{definition: <BlockViewDefinition JSON>}` (NOT the bare definition at top level).
    pub fn update_view_request(
        &self,
        workspace_id: &str,
        view_block_id: &str,
        definition: &BlockViewDefinition,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.definition_url(workspace_id, view_block_id),
            body: Some(serde_json::json!({ "definition": definition_to_json(definition) })),
        }
    }

    /// Pure request builder for `PATCH .../loom/blocks/:block_id` (updateLoomBlock) carrying the Kanban
    /// lane-move tag mutation. The VERIFIED body has `add_tags`/`remove_tags` at the TOP level (the
    /// backend `LoomBlockPatchRequest` reads them alongside the flattened update).
    pub fn card_move_request(
        &self,
        workspace_id: &str,
        block_id: &str,
        add_tags: &[String],
        remove_tags: &[String],
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.block_url(workspace_id, block_id),
            body: Some(serde_json::json!({
                "add_tags": add_tags,
                "remove_tags": remove_tags,
            })),
        }
    }

    /// Pure request builder for `POST .../views/definitions` (createBlockView). The stable
    /// client-generated `block_id` is retained across ambiguous retries, so response loss cannot
    /// create duplicate saved views.
    pub fn create_view_request(
        &self,
        workspace_id: &str,
        block_id: &str,
        title: &str,
        definition: &BlockViewDefinition,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: self.definitions_url(workspace_id),
            body: Some(serde_json::json!({
                "block_id": block_id,
                "title": title,
                "definition": definition_to_json(definition),
            })),
        }
    }

    /// Fetch the view definition off the UI thread, delivering the parsed [`BlockViewRecordData`] into
    /// `cell`.
    pub fn fetch_view(&self, workspace_id: &str, view_block_id: &str, cell: BlockViewRecordCell) {
        self.fetch_view_inner(workspace_id, view_block_id, cell, None);
    }

    /// Fetch only if `generation` is still current when the response resolves. The mounted host uses
    /// this for initial loads, mutation re-queries, and Retry so an older slow request can never publish
    /// over a newer binding.
    pub fn fetch_view_for_generation(
        &self,
        workspace_id: &str,
        view_block_id: &str,
        generation: Arc<AtomicU64>,
        expected_generation: u64,
        cell: BlockViewRecordCell,
    ) {
        self.fetch_view_inner(
            workspace_id,
            view_block_id,
            cell,
            Some((generation, expected_generation)),
        );
    }

    fn fetch_view_inner(
        &self,
        workspace_id: &str,
        view_block_id: &str,
        cell: BlockViewRecordCell,
        generation_guard: Option<(Arc<AtomicU64>, u64)>,
    ) {
        let spec = self.get_view_request(workspace_id, view_block_id);
        let client = self.client.clone();
        let id = view_block_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_block_view(&client, &spec.url, &id)
                .await
                .map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                // Check while holding the publication lock. A rebind either increments first and this
                // stale delivery is discarded, or waits for this publication and then clears it.
                if generation_guard
                    .as_ref()
                    .is_some_and(|(generation, expected)| {
                        generation.load(Ordering::Acquire) != *expected
                    })
                {
                    return;
                }
                *slot = Some(result);
            }
        });
    }

    /// Run the view query (POST!) off the UI thread, delivering the parsed [`BlockViewResults`] into
    /// `cell`.
    pub fn query_results(
        &self,
        workspace_id: &str,
        view_block_id: &str,
        limit: u32,
        offset: u32,
        cell: BlockViewResultsCell,
    ) {
        self.query_results_inner(workspace_id, view_block_id, limit, offset, cell, None);
    }

    /// Query only if `generation` remains current at delivery time. See
    /// [`Self::fetch_view_for_generation`] for the mounted-host race this closes.
    #[allow(clippy::too_many_arguments)]
    pub fn query_results_for_generation(
        &self,
        workspace_id: &str,
        view_block_id: &str,
        limit: u32,
        offset: u32,
        generation: Arc<AtomicU64>,
        expected_generation: u64,
        cell: BlockViewResultsCell,
    ) {
        self.query_results_inner(
            workspace_id,
            view_block_id,
            limit,
            offset,
            cell,
            Some((generation, expected_generation)),
        );
    }

    fn query_results_inner(
        &self,
        workspace_id: &str,
        view_block_id: &str,
        limit: u32,
        offset: u32,
        cell: BlockViewResultsCell,
        generation_guard: Option<(Arc<AtomicU64>, u64)>,
    ) {
        let spec = self.query_results_request(workspace_id, view_block_id, limit, offset);
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = post_block_view_results(&client, &spec.url, &body)
                .await
                .map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                if generation_guard
                    .as_ref()
                    .is_some_and(|(generation, expected)| {
                        generation.load(Ordering::Acquire) != *expected
                    })
                {
                    return;
                }
                *slot = Some(result);
            }
        });
    }

    /// Send a prebuilt update/card-move [`RequestSpec`] off the UI thread, delivering `Ok(echo_id)` /
    /// `Err(msg)` into `cell`. `echo_id` is the view block id the host stays on (the host passes its
    /// current id). The host re-queries after a 2xx.
    pub fn dispatch(
        &self,
        spec: RequestSpec,
        workspace_id: &str,
        echo_id: String,
        generation: Arc<AtomicU64>,
        expected_generation: u64,
        cell: BlockViewOpCell,
    ) {
        let client = self.client.clone();
        let workspace_id = workspace_id.to_owned();
        let expected_bound_view_id = echo_id.clone();
        self.runtime.spawn(async move {
            let result = send_block_view_mutation(&client, &spec)
                .await
                .map(|_| echo_id);
            if let Ok(mut slot) = cell.lock() {
                if generation.load(Ordering::Acquire) != expected_generation {
                    return;
                }
                *slot = Some(BlockViewOpDelivery {
                    workspace_id,
                    generation: expected_generation,
                    expected_bound_view_id: Some(expected_bound_view_id),
                    result: result.map_err(|e| e.to_string()),
                });
            }
        });
    }

    /// Create a new view off the UI thread, delivering the NEW view block id into `cell` (so the host
    /// switches to it). The body is `createBlockView`'s `{title, definition}`.
    pub fn create_view(
        &self,
        workspace_id: &str,
        block_id: &str,
        title: &str,
        definition: &BlockViewDefinition,
        generation: Arc<AtomicU64>,
        expected_generation: u64,
        cell: BlockViewOpCell,
    ) {
        let spec = self.create_view_request(workspace_id, block_id, title, definition);
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        let url = spec.url.clone();
        let workspace_id = workspace_id.to_owned();
        self.runtime.spawn(async move {
            let result = post_create_block_view(&client, &url, &body)
                .await
                .map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                if generation.load(Ordering::Acquire) != expected_generation {
                    return;
                }
                *slot = Some(BlockViewOpDelivery {
                    workspace_id,
                    generation: expected_generation,
                    expected_bound_view_id: None,
                    result,
                });
            }
        });
    }
}

/// Expand a calendar `YYYY-MM-DD` bound to the full RFC3339 instant the backend's
/// `BlockViewQuery.date_from/date_to: Option<DateTime<Utc>>` (default chrono serde) ACCEPTS. The backend
/// REJECTS a bare date-only string (must-fix #1 / backend-shape #4 — `updateBlockView` would 400/422),
/// so `date_from` becomes the start-of-day `<date>T00:00:00Z` and `date_to` the INCLUSIVE end-of-day
/// `<date>T23:59:59Z`. A value already carrying a time component (`T`) is passed through unchanged (so a
/// future full-timestamp input still round-trips). `end_of_day=false` => 00:00:00, `true` => 23:59:59.
fn expand_iso_date_to_rfc3339(date: &str, end_of_day: bool) -> String {
    let trimmed = date.trim();
    if trimmed.contains('T') {
        // Already a full timestamp — leave it (the read path slices, but a caller may pass full).
        return trimmed.to_owned();
    }
    let time = if end_of_day { "23:59:59" } else { "00:00:00" };
    format!("{trimmed}T{time}Z")
}

/// Serialize a [`BlockViewDefinition`] to the VERIFIED wire JSON the backend `BlockViewDefinition`
/// deserializes (snake_case kind/field/direction strings). The FULL query (date window expanded to
/// RFC3339, content_type, mime, tag_ids, mention_ids) and `group_by` are written so a sort/kind/date
/// `updateBlockView` round-trip — which the backend persists as a FULL overwrite of
/// `view_definition_json` — never silently drops a server-side filter or a Kanban grouping (must-fix
/// #1/#2/#3). The backend defaults only genuinely-absent fields (serde `#[serde(default)]`).
fn definition_to_json(def: &BlockViewDefinition) -> serde_json::Value {
    let q = &def.query;
    let mut query = serde_json::Map::new();
    // date_from/date_to: EXPAND the calendar `YYYY-MM-DD` to a full RFC3339 instant — the backend field
    // is `Option<DateTime<Utc>>` and rejects a bare date (must-fix #1).
    if let Some(from) = &q.date_from {
        query.insert(
            "date_from".to_owned(),
            serde_json::Value::String(expand_iso_date_to_rfc3339(from, false)),
        );
    }
    if let Some(to) = &q.date_to {
        query.insert(
            "date_to".to_owned(),
            serde_json::Value::String(expand_iso_date_to_rfc3339(to, true)),
        );
    }
    if let Some(ct) = &q.content_type {
        query.insert(
            "content_type".to_owned(),
            serde_json::Value::String(ct.clone()),
        );
    }
    if let Some(mime) = &q.mime {
        query.insert("mime".to_owned(), serde_json::Value::String(mime.clone()));
    }
    if !q.tag_ids.is_empty() {
        query.insert(
            "tag_ids".to_owned(),
            serde_json::Value::Array(
                q.tag_ids
                    .iter()
                    .map(|t| serde_json::Value::String(t.clone()))
                    .collect(),
            ),
        );
    }
    if !q.mention_ids.is_empty() {
        query.insert(
            "mention_ids".to_owned(),
            serde_json::Value::Array(
                q.mention_ids
                    .iter()
                    .map(|m| serde_json::Value::String(m.clone()))
                    .collect(),
            ),
        );
    }
    let mut obj = serde_json::Map::new();
    obj.insert(
        "kind".to_owned(),
        serde_json::Value::String(def.kind.as_str().to_owned()),
    );
    if !query.is_empty() {
        obj.insert("query".to_owned(), serde_json::Value::Object(query));
    }
    if !def.columns.is_empty() {
        obj.insert(
            "columns".to_owned(),
            serde_json::Value::Array(
                def.columns
                    .iter()
                    .map(|f| serde_json::Value::String(f.as_str().to_owned()))
                    .collect(),
            ),
        );
    }
    // group_by: serialize the verified tagged-enum shape ({"kind":"tag"} | {"kind":"field","field":..})
    // so a Kanban view's lane grouping survives the full-overwrite persist (must-fix #3).
    if let Some(group_by) = &def.group_by {
        obj.insert("group_by".to_owned(), group_by_to_json(group_by));
    }
    if let Some(sort) = def.sort {
        obj.insert(
            "sort".to_owned(),
            serde_json::json!({
                "field": sort.field.as_str(),
                "direction": sort.direction.as_str(),
            }),
        );
    }
    if let Some(field) = def.calendar_date_field {
        obj.insert(
            "calendar_date_field".to_owned(),
            serde_json::Value::String(field.as_str().to_owned()),
        );
    }
    serde_json::Value::Object(obj)
}

/// Serialize the verified `BlockViewGroupBy` tagged-enum wire shape (`#[serde(tag="kind",
/// rename_all="snake_case")]`): `{"kind":"tag"}` or `{"kind":"field","field":"<field>"}`.
fn group_by_to_json(group_by: &BlockViewGroupBy) -> serde_json::Value {
    match group_by {
        BlockViewGroupBy::Tag => serde_json::json!({ "kind": "tag" }),
        BlockViewGroupBy::Field { field } => {
            serde_json::json!({ "kind": "field", "field": field.as_str() })
        }
    }
}

fn block_view_required_string(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    path: &str,
) -> Result<String, String> {
    object
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("{path}.{key} must be a non-empty string"))
}

fn parse_block_view_kind(value: &str, path: &str) -> Result<BlockViewKind, String> {
    match value {
        "table" => Ok(BlockViewKind::Table),
        "kanban" => Ok(BlockViewKind::Kanban),
        "calendar" => Ok(BlockViewKind::Calendar),
        other => Err(format!("{path} has unknown block-view kind `{other}`")),
    }
}

fn parse_block_view_field(value: &serde_json::Value, path: &str) -> Result<BlockViewField, String> {
    let raw = value
        .as_str()
        .ok_or_else(|| format!("{path} must be a string"))?;
    BlockViewField::parse_str(raw)
        .ok_or_else(|| format!("{path} has unknown block-view field `{raw}`"))
}

/// Parse the verified `BlockViewGroupBy` tagged-enum JSON and reject the entire definition on any
/// malformed or unknown member.
fn group_by_from_json(v: &serde_json::Value) -> Result<BlockViewGroupBy, String> {
    let object = v
        .as_object()
        .ok_or_else(|| "definition.group_by must be an object".to_owned())?;
    match block_view_required_string(object, "kind", "definition.group_by")?.as_str() {
        "tag" => Ok(BlockViewGroupBy::Tag),
        "field" => Ok(BlockViewGroupBy::Field {
            field: parse_block_view_field(
                object
                    .get("field")
                    .ok_or_else(|| "definition.group_by.field is required".to_owned())?,
                "definition.group_by.field",
            )?,
        }),
        other => Err(format!(
            "definition.group_by.kind has unknown value `{other}`"
        )),
    }
}

/// Parse the VERIFIED `BlockViewDefinition` JSON into the native projection. Fields omitted by the
/// backend's explicit `skip_serializing_if` rules remain optional; every present field is strict.
pub fn definition_from_json(v: &serde_json::Value) -> Result<BlockViewDefinition, String> {
    let object = v
        .as_object()
        .ok_or_else(|| "definition must be an object".to_owned())?;
    let kind = parse_block_view_kind(
        &block_view_required_string(object, "kind", "definition")?,
        "definition.kind",
    )?;
    let mut seen_columns = std::collections::HashSet::new();
    let columns = match object.get("columns") {
        None => Vec::new(),
        Some(value) => value
            .as_array()
            .ok_or_else(|| "definition.columns must be an array".to_owned())?
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let field = parse_block_view_field(value, &format!("definition.columns[{index}]"))?;
                if !seen_columns.insert(field.as_str()) {
                    return Err(format!(
                        "definition.columns contains duplicate `{}`",
                        field.as_str()
                    ));
                }
                Ok(field)
            })
            .collect::<Result<Vec<_>, String>>()?,
    };
    let sort = match object.get("sort") {
        None | Some(serde_json::Value::Null) => None,
        Some(value) => {
            let sort = value
                .as_object()
                .ok_or_else(|| "definition.sort must be an object or null".to_owned())?;
            let field = parse_block_view_field(
                sort.get("field")
                    .ok_or_else(|| "definition.sort.field is required".to_owned())?,
                "definition.sort.field",
            )?;
            let direction =
                match block_view_required_string(sort, "direction", "definition.sort")?.as_str() {
                    "asc" => BlockViewSortDirection::Asc,
                    "desc" => BlockViewSortDirection::Desc,
                    other => {
                        return Err(format!(
                            "definition.sort.direction has unknown value `{other}`"
                        ));
                    }
                };
            Some(BlockViewSort { field, direction })
        }
    };
    let group_by = match object.get("group_by") {
        None | Some(serde_json::Value::Null) => None,
        Some(value) => Some(group_by_from_json(value)?),
    };
    let calendar_date_field = match object.get("calendar_date_field") {
        None | Some(serde_json::Value::Null) => None,
        Some(value) => Some(parse_block_view_field(
            value,
            "definition.calendar_date_field",
        )?),
    };
    let query = parse_block_view_query(
        object
            .get("query")
            .ok_or_else(|| "definition.query is required".to_owned())?,
    )?;
    Ok(BlockViewDefinition {
        kind,
        query,
        columns,
        group_by,
        sort,
        calendar_date_field,
    })
}

/// Parse the FULL VERIFIED `BlockViewQuery` JSON into the native projection. The backend stores
/// `date_from`/`date_to` as ISO datetimes; the calendar surface only needs the `YYYY-MM-DD` prefix, so
/// the native projection slices it (the write path re-expands it to RFC3339). `content_type`/`mime`/
/// `tag_ids`/`mention_ids` are carried verbatim so a later `updateBlockView` round-trip never drops the
/// user's server-side filters (must-fix #2).
fn parse_block_view_query(v: &serde_json::Value) -> Result<BlockViewQuery, String> {
    let object = v
        .as_object()
        .ok_or_else(|| "definition.query must be an object".to_owned())?;
    let date = |key: &str| -> Result<Option<String>, String> {
        match object.get(key) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(value) => {
                let raw = value
                    .as_str()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| {
                        format!("definition.query.{key} must be a non-empty RFC3339 string or null")
                    })?;
                chrono::DateTime::parse_from_rfc3339(raw)
                    .map_err(|error| format!("definition.query.{key} is not RFC3339: {error}"))?;
                Ok(Some(raw.chars().take(10).collect()))
            }
        }
    };
    let string_array = |key: &str| -> Result<Vec<String>, String> {
        let Some(value) = object.get(key) else {
            return Ok(Vec::new());
        };
        let array = value
            .as_array()
            .ok_or_else(|| format!("definition.query.{key} must be an array"))?;
        let mut seen = std::collections::HashSet::new();
        array
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let item = value
                    .as_str()
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .ok_or_else(|| {
                        format!("definition.query.{key}[{index}] must be a non-empty string")
                    })?
                    .to_owned();
                if !seen.insert(item.clone()) {
                    return Err(format!(
                        "definition.query.{key} contains duplicate `{item}`"
                    ));
                }
                Ok(item)
            })
            .collect()
    };
    let optional_string = |key: &str| -> Result<Option<String>, String> {
        match object.get(key) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(value) => value
                .as_str()
                .map(ToOwned::to_owned)
                .map(Some)
                .ok_or_else(|| format!("definition.query.{key} must be a string or null")),
        }
    };
    let content_type = optional_string("content_type")?;
    if let Some(value) = content_type.as_deref() {
        if !matches!(
            value,
            "note"
                | "file"
                | "annotated_file"
                | "tag_hub"
                | "journal"
                | "canvas"
                | "view_def"
                | "ckc_moodboard"
                | "ckc_character"
        ) {
            return Err(format!(
                "definition.query.content_type has unknown value `{value}`"
            ));
        }
    }
    Ok(BlockViewQuery {
        date_from: date("date_from")?,
        date_to: date("date_to")?,
        content_type,
        mime: optional_string("mime")?,
        tag_ids: string_array("tag_ids")?,
        mention_ids: string_array("mention_ids")?,
    })
}

/// Parse one VERIFIED `LoomBlock` JSON object into a [`LoomBlockRow`] (the cell-value + bucket-key +
/// title fields the sub-views read). Any malformed row rejects the complete delivery; rows are never
/// skipped or completed with invented defaults.
pub fn loom_block_row_from_json(b: &serde_json::Value) -> Result<LoomBlockRow, String> {
    let object = b
        .as_object()
        .ok_or_else(|| "block row must be an object".to_owned())?;
    let block_id = block_view_required_string(object, "block_id", "block")?;
    let _workspace_id = block_view_required_string(object, "workspace_id", "block")?;
    let content_type = block_view_required_string(object, "content_type", "block")?;
    if !matches!(
        content_type.as_str(),
        "note"
            | "file"
            | "annotated_file"
            | "tag_hub"
            | "journal"
            | "canvas"
            | "view_def"
            | "ckc_moodboard"
            | "ckc_character"
    ) {
        return Err(format!(
            "block.content_type has unknown value `{content_type}`"
        ));
    }
    let nullable_string = |key: &str| -> Result<Option<String>, String> {
        match object.get(key) {
            Some(serde_json::Value::Null) => Ok(None),
            Some(value) => value
                .as_str()
                .map(ToOwned::to_owned)
                .map(Some)
                .ok_or_else(|| format!("block.{key} must be a string or null")),
            None => Err(format!("block.{key} is required (string or null)")),
        }
    };
    let timestamp = |key: &str| -> Result<String, String> {
        let raw = block_view_required_string(object, key, "block")?;
        chrono::DateTime::parse_from_rfc3339(&raw)
            .map_err(|error| format!("block.{key} is not RFC3339: {error}"))?;
        Ok(raw)
    };
    let derived = object
        .get("derived")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "block.derived must be an object".to_owned())?;
    let count = |key: &str| -> Result<i64, String> {
        let value = derived
            .get(key)
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| format!("block.derived.{key} must be an integer"))?;
        if value < 0 {
            return Err(format!("block.derived.{key} must be non-negative"));
        }
        Ok(value)
    };
    Ok(LoomBlockRow {
        title: nullable_string("title")?,
        original_filename: nullable_string("original_filename")?,
        content_type,
        journal_date: nullable_string("journal_date")?,
        created_at: timestamp("created_at")?,
        updated_at: timestamp("updated_at")?,
        pinned: object
            .get("pinned")
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| "block.pinned must be a boolean".to_owned())?,
        favorite: object
            .get("favorite")
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| "block.favorite must be a boolean".to_owned())?,
        backlink_count: count("backlink_count")?,
        mention_count: count("mention_count")?,
        tag_count: count("tag_count")?,
        block_id,
    })
}

/// Parse the VERIFIED `BlockViewResults` JSON (`{kind, blocks, groups?, total_returned}`) into the
/// native projection. `blocks: []` is the canonical empty state; a missing/wrong-typed array is a
/// malformed success and therefore an error. `groups` may be omitted only because the backend
/// explicitly skips serialization for an empty vector.
pub fn results_from_json(v: &serde_json::Value) -> Result<BlockViewResults, String> {
    let object = v
        .as_object()
        .ok_or_else(|| "block-view results must be an object".to_owned())?;
    let kind_str = block_view_required_string(object, "kind", "results")?;
    parse_block_view_kind(&kind_str, "results.kind")?;
    let block_values = object
        .get("blocks")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "results.blocks must be an array".to_owned())?;
    let mut block_ids = std::collections::HashSet::new();
    let blocks = block_values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let block = loom_block_row_from_json(value)
                .map_err(|error| format!("results.blocks[{index}]: {error}"))?;
            if !block_ids.insert(block.block_id.clone()) {
                return Err(format!(
                    "results.blocks contains duplicate id `{}`",
                    block.block_id
                ));
            }
            Ok(block)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut lane_keys = std::collections::HashSet::new();
    let groups = match object.get("groups") {
        None => Vec::new(),
        Some(value) => value
            .as_array()
            .ok_or_else(|| "results.groups must be an array when present".to_owned())?
            .iter()
            .enumerate()
            .map(|(lane_index, lane)| {
                let lane = lane
                    .as_object()
                    .ok_or_else(|| format!("results.groups[{lane_index}] must be an object"))?;
                let key = block_view_required_string(
                    lane,
                    "key",
                    &format!("results.groups[{lane_index}]"),
                )?;
                if !lane_keys.insert(key.clone()) {
                    return Err(format!(
                        "results.groups contains duplicate lane key `{key}`"
                    ));
                }
                let values = lane
                    .get("blocks")
                    .and_then(serde_json::Value::as_array)
                    .ok_or_else(|| {
                        format!("results.groups[{lane_index}].blocks must be an array")
                    })?;
                let mut ids = std::collections::HashSet::new();
                let blocks = values
                    .iter()
                    .enumerate()
                    .map(|(block_index, value)| {
                        let block = loom_block_row_from_json(value).map_err(|error| {
                            format!("results.groups[{lane_index}].blocks[{block_index}]: {error}")
                        })?;
                        if !ids.insert(block.block_id.clone()) {
                            return Err(format!(
                                "results.groups[{lane_index}].blocks contains duplicate id `{}`",
                                block.block_id
                            ));
                        }
                        Ok(block)
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(BlockViewLane { key, blocks })
            })
            .collect::<Result<Vec<_>, String>>()?,
    };
    let total = object
        .get("total_returned")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "results.total_returned must be a non-negative integer".to_owned())?;
    let total_returned =
        u32::try_from(total).map_err(|_| "results.total_returned exceeds u32".to_owned())?;
    Ok(BlockViewResults {
        kind_str,
        blocks,
        groups,
        total_returned,
    })
}

/// `GET {url}` and parse the verified `BlockViewRecord` (`{block, definition}`) into a
/// [`BlockViewRecordData`]. The `block.block_id` (or the requested id) identifies the view.
async fn fetch_block_view(
    client: &reqwest::Client,
    url: &str,
    requested_id: &str,
) -> Result<BlockViewRecordData, AppError> {
    let v = block_view_get_json(client, url).await?;
    let definition = v
        .get("definition")
        .ok_or_else(|| AppError::Parse("getBlockView response missing definition".to_owned()))
        .and_then(|value| definition_from_json(value).map_err(AppError::Parse))?;
    let view_block_id = v
        .get("block")
        .and_then(|b| b.get("block_id"))
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Parse("getBlockView response missing block.block_id".to_owned()))?
        .to_owned();
    if view_block_id != requested_id {
        return Err(AppError::Parse(format!(
            "getBlockView identity mismatch: requested `{requested_id}`, received `{view_block_id}`"
        )));
    }
    let content_type = v
        .get("block")
        .and_then(|block| block.get("content_type"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::Parse("getBlockView response missing block.content_type".to_owned())
        })?;
    if content_type != "view_def" {
        return Err(AppError::Parse(format!(
            "getBlockView response block.content_type must be `view_def`, got `{content_type}`"
        )));
    }
    Ok(BlockViewRecordData {
        view_block_id,
        definition,
    })
}

/// `POST {url}` (body `{limit,offset}`) and parse the verified `BlockViewResults` (RISK-1: POST not GET).
async fn post_block_view_results(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<BlockViewResults, AppError> {
    let v = block_view_post_json(client, url, body).await?;
    results_from_json(&v).map_err(AppError::Parse)
}

/// `POST {url}` (createBlockView body) and read the NEW view block id from the returned `BlockViewRecord`.
async fn post_create_block_view(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<String, AppError> {
    let v = block_view_post_json(client, url, body).await?;
    create_block_view_id_from_json(&v).map_err(AppError::Parse)
}

/// Parse the successful create response as the same canonical `BlockViewRecord` shape used by GET.
/// A 2xx response is not success when it cannot identify a real saved-view definition.
pub fn create_block_view_id_from_json(v: &serde_json::Value) -> Result<String, String> {
    let id = v
        .get("block")
        .and_then(|b| b.get("block_id"))
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "createBlockView response missing non-empty block.block_id".to_owned())?
        .to_owned();
    let content_type = v
        .get("block")
        .and_then(|block| block.get("content_type"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "createBlockView response missing block.content_type".to_owned())?;
    if content_type != "view_def" {
        return Err(format!(
            "createBlockView response block.content_type must be `view_def`, got `{content_type}`"
        ));
    }
    let definition = v
        .get("definition")
        .ok_or_else(|| "createBlockView response missing definition".to_owned())?;
    definition_from_json(definition)
        .map_err(|error| format!("createBlockView response malformed definition: {error}"))?;
    Ok(id)
}

/// Send one block-view mutation (update/card-move) by method, treating any 2xx as success (the host
/// re-queries for the body).
async fn send_block_view_mutation(
    client: &reqwest::Client,
    spec: &RequestSpec,
) -> Result<(), AppError> {
    let empty = serde_json::json!({});
    let body = spec.body.as_ref().unwrap_or(&empty);
    match spec.method {
        HttpMethod::Post => block_view_post_expect_success(client, &spec.url, body).await,
        HttpMethod::Patch => block_view_patch_expect_success(client, &spec.url, body).await,
        _ => Err(AppError::Http(
            "block-view mutation must be POST or PATCH".to_owned(),
        )),
    }
}

/// Attach the canonical attributable identity to every saved-view request. Keeping this at the
/// execution seam preserves the pure `RequestSpec` builders used by unit/kittest proofs while the real
/// product transport is never anonymous.
fn block_view_identity(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    request
        .header(HSK_HEADER_ACTOR_ID, BLOCK_VIEW_ACTOR_ID)
        .header(HSK_HEADER_ACTOR_KIND, BLOCK_VIEW_ACTOR_KIND)
        .header(
            HSK_HEADER_KERNEL_TASK_RUN_ID,
            "block-collection-view-runtime",
        )
        .header(HSK_HEADER_SESSION_RUN_ID, "block-collection-view-session")
}

async fn block_view_get_json(
    client: &reqwest::Client,
    url: &str,
) -> Result<serde_json::Value, AppError> {
    let resp = block_view_identity(client.get(url))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Http(format!(
            "GET block view non-success status {status}: {body}"
        )));
    }
    resp.json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))
}

async fn block_view_post_json(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let resp = block_view_identity(client.post(url))
        .timeout(Duration::from_secs(5))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    let status = resp.status();
    if !status.is_success() {
        let response_body = resp.text().await.unwrap_or_default();
        return Err(AppError::Http(format!(
            "POST block view non-success status {status}: {response_body}"
        )));
    }
    resp.json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))
}

async fn block_view_post_expect_success(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<(), AppError> {
    let resp = block_view_identity(client.post(url))
        .timeout(Duration::from_secs(5))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    let status = resp.status();
    if !status.is_success() {
        let response_body = resp.text().await.unwrap_or_default();
        return Err(AppError::Http(format!(
            "POST block-view mutation non-success status {status}: {response_body}"
        )));
    }
    Ok(())
}

async fn block_view_patch_expect_success(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<(), AppError> {
    let resp = block_view_identity(client.patch(url))
        .timeout(Duration::from_secs(5))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    let status = resp.status();
    if !status.is_success() {
        let response_body = resp.text().await.unwrap_or_default();
        return Err(AppError::Http(format!(
            "PATCH block-view mutation non-success status {status}: {response_body}"
        )));
    }
    Ok(())
}

/// `POST {url}` with a JSON body and return the parsed JSON response. A non-success status or a parse
/// failure is an [`AppError`]. Used by the block-view query + create (both POST + read a body).
async fn post_json(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let resp = client
        .post(url)
        .timeout(Duration::from_secs(5))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "POST non-success status {}",
            resp.status()
        )));
    }
    resp.json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-028 LoomSearchV2 client: the VERIFIED hybrid-search + save-as-view surface.
//
// Verified READ-ONLY against `src/backend/handshake_core/src/{api,storage}/loom.rs` (the MT-022..027
// lesson — bind only confirmed shapes; do NOT guess a route):
//   - POST /workspaces/:ws/loom/search-v2  (api/loom.rs route table line 294 + handler `loom_search_v2`)
//       body  `LoomSearchV2Body {query, content_type?, tag_ids?, graph_boost, limit, offset?}`
//       reply `LoomSearchV2Response {hits:[LoomSearchV2Hit{block,score,fts_rank,trgm_sim,vector_sim,
//              edge_degree,highlight}], content_type_facets: {ct->count}, semantic_available, total}`
//       (storage/loom.rs 637-712; LoomBlockContentType serializes `snake_case`, e.g. `tag_hub`).
//   - SAVE-AS-VIEW reuses the MT-027 VERIFIED createBlockView route:
//       POST /workspaces/:ws/loom/views/definitions  body `{title, definition}` -> `{block:{block_id}}`
//     The MT-028 contract's bare `/loom/views` is STALE (RISK-3 / MC-3); MT-027 proved the real route is
//     `/loom/views/definitions` and the body is `{title, definition}` (NOT `{kind, query, columns}` at
//     top level — the React `createBlockView(ws, definition, {title})` flattens to the SAME wire shape).
//     The save-as-view `definition` carries `{kind:"table", query:{content_type?}, columns:[...]}`.
//
// Mirrors the `BlockViewClient` shape: pure `*_request` builders return a [`RequestSpec`] (unit-testable
// WITHOUT a backend), and the off-thread methods deliver into `Arc<Mutex<Option<..>>>` cells the egui UI
// drains next frame (HBR-QUIET: the search HTTP call is NEVER on the UI thread). The deserialized result
// types are local serde structs whose field names match the snake_case backend JSON EXACTLY (RISK-6: a
// rename would silently null a field), modelling only the fields the panel displays.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// A Loom block REFERENCE returned in a search hit (open-in-place, never a content copy). Only the
/// display fields the panel renders are modelled; the backend `LoomBlock` carries more (workspace_id,
/// timestamps, derived counts) that the search surface does not need. `content_type` is a plain
/// `String` (the backend serializes the `LoomBlockContentType` enum as a `snake_case` string, e.g.
/// `"note"`, `"tag_hub"`), so the native struct never re-encodes the enum and an unknown future
/// content_type degrades to its raw string instead of a deserialize error.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct LoomSearchBlock {
    pub block_id: String,
    pub content_type: String,
    #[serde(default)]
    pub document_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

impl LoomSearchBlock {
    /// The display title: the block's own `title`, or the `block_id` as a fallback (the React parity
    /// reference renders `hit.block.title ?? hit.block.block_id`).
    pub fn display_title(&self) -> &str {
        self.title
            .as_deref()
            .filter(|t| !t.is_empty())
            .unwrap_or(&self.block_id)
    }
}

/// One hybrid-search hit. Field names match the backend `storage::LoomSearchV2Hit` snake_case JSON
/// EXACTLY. The per-modality sub-scores (fts_rank/trgm_sim/vector_sim/edge_degree) are retained so a
/// later MT or test can prove which modality matched; the panel itself renders `score` + `highlight`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct LoomSearchV2Hit {
    pub block: LoomSearchBlock,
    pub score: f64,
    #[serde(default)]
    pub fts_rank: f64,
    #[serde(default)]
    pub trgm_sim: f64,
    #[serde(default)]
    pub vector_sim: f64,
    #[serde(default)]
    pub edge_degree: i64,
    /// ts_headline highlight with literal `<mark>…</mark>` markers; rendered as colored runs (NOT raw
    /// HTML) by [`crate::loom_search_v2::parse_highlight_segments`].
    #[serde(default)]
    pub highlight: String,
}

/// A faceted, ranked LoomSearchV2 result set. Field names match the backend `storage::LoomSearchV2Response`
/// snake_case JSON EXACTLY. `content_type_facets` keeps the backend's `BTreeMap<String,i64>` shape so the
/// facet order is deterministic before the panel re-sorts by count.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct LoomSearchV2Response {
    pub hits: Vec<LoomSearchV2Hit>,
    #[serde(default)]
    pub content_type_facets: std::collections::BTreeMap<String, i64>,
    /// `true` => the semantic (pgvector kNN) modality contributed; `false` => typed keyword/trigram
    /// fallback (no embedding model configured). The status line reads this to show `(semantic on)`
    /// vs `(keyword/fuzzy only)` HONESTLY (RISK-7: never claim semantic when it is off).
    #[serde(default)]
    pub semantic_available: bool,
    #[serde(default)]
    pub total: i64,
}

/// The request body for `POST /loom/search-v2`. Matches the backend `LoomSearchV2Body` (snake_case).
/// `graph_boost` is always `1.0` and `limit` `25` for the MT-028 baseline (the React parity reference
/// sends exactly these); `content_type` is the active facet filter (omitted via `skip_serializing_if`
/// when `None`, exactly as the backend's `#[serde(default)] Option` accepts).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct LoomSearchV2Body {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    pub graph_boost: f64,
    pub limit: u32,
    /// Backend-authoritative result offset. The baseline UI request omits zero, while callers that
    /// must exhaust an exact candidate set advance this by `limit` for each subsequent page.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub offset: u32,
}

fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

impl LoomSearchV2Body {
    /// The MT-028 baseline body: graph_boost 1.0, limit 25 (the React parity defaults), with the active
    /// facet (if any) as the `content_type` filter.
    pub fn baseline(query: impl Into<String>, content_type: Option<String>) -> Self {
        Self {
            query: query.into(),
            content_type,
            graph_boost: 1.0,
            limit: 25,
            offset: 0,
        }
    }
}

/// One-slot delivery cell for an off-thread LoomSearchV2 result (the egui UI drains it next frame).
pub type LoomSearchCell = Arc<Mutex<Option<Result<LoomSearchV2Response, String>>>>;

/// One-slot delivery cell for an off-thread save-as-view result. `Ok(block_id)` is the NEW view block
/// id (shown in the panel's view-status label); `Err(msg)` the failure string.
pub type SaveViewCell = Arc<Mutex<Option<Result<String, String>>>>;

/// REST client for the VERIFIED MT-264 LoomSearchV2 surface: the hybrid search POST and the save-results
/// -as-view POST (reusing the MT-027 createBlockView route). Drives both off the UI thread, delivering
/// into the delivery cells the egui panel drains. Speaks `serde_json` so it never depends on the
/// `handshake_core` crate.
#[derive(Clone)]
pub struct LoomSearchV2Client {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomSearchV2Client {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn search_url(&self, workspace_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/search-v2",
            self.base_url, workspace_id
        )
    }

    fn views_definitions_url(&self, workspace_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/views/definitions",
            self.base_url, workspace_id
        )
    }

    /// Pure request builder for `POST .../loom/search-v2`. The VERIFIED body is the snake_case
    /// `LoomSearchV2Body`; asserting it proves the production request construction (the spawn path below
    /// routes through this SAME builder) without a live backend.
    pub fn search_request(&self, workspace_id: &str, body: &LoomSearchV2Body) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: self.search_url(workspace_id),
            body: Some(serde_json::to_value(body).unwrap_or_default()),
        }
    }

    /// Pure request builder for the save-as-view `POST .../loom/views/definitions` (createBlockView).
    /// The VERIFIED body is `{block_id, title, definition}` where `block_id` is caller-stable across
    /// ambiguous retries and `definition = {kind:"table", query, columns}`
    /// (MT-027's proven shape; the MT-028 contract's bare `/loom/views` is stale). `active_content_type`
    /// becomes the saved view's `query.content_type` filter (omitted when `None`).
    pub fn save_view_request(
        &self,
        workspace_id: &str,
        block_id: &str,
        query_text: &str,
        active_content_type: Option<&str>,
    ) -> RequestSpec {
        let query = match active_content_type {
            Some(ct) => serde_json::json!({ "content_type": ct }),
            None => serde_json::json!({}),
        };
        let definition = serde_json::json!({
            "kind": "table",
            "query": query,
            "columns": ["title", "content_type", "updated"],
        });
        RequestSpec {
            method: HttpMethod::Post,
            url: self.views_definitions_url(workspace_id),
            body: Some(serde_json::json!({
                "block_id": block_id,
                "title": format!("Search: {}", query_text.trim()),
                "definition": definition,
            })),
        }
    }

    /// Run the hybrid search (POST) off the UI thread, delivering the parsed [`LoomSearchV2Response`]
    /// into `cell`. The egui UI thread returns immediately; the spawned tokio task does the network I/O
    /// and writes the result, which the UI drains next frame (HBR-QUIET — no UI-thread network block).
    pub fn search(&self, workspace_id: &str, body: &LoomSearchV2Body, cell: LoomSearchCell) {
        let spec = self.search_request(workspace_id, body);
        let req_body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        let url = spec.url;
        self.runtime.spawn(async move {
            let result = post_loom_search_v2(&client, &url, &req_body)
                .await
                .map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result);
            }
        });
    }

    /// Create the saved view (POST) off the UI thread, delivering the NEW view block id into `cell`.
    pub fn save_view(
        &self,
        workspace_id: &str,
        block_id: &str,
        query_text: &str,
        active_content_type: Option<&str>,
        cell: SaveViewCell,
    ) {
        let spec = self.save_view_request(workspace_id, block_id, query_text, active_content_type);
        let req_body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        let url = spec.url;
        self.runtime.spawn(async move {
            let result = post_create_block_view(&client, &url, &req_body)
                .await
                .map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result);
            }
        });
    }
}

/// `POST {url}` (LoomSearchV2 body) and parse the verified [`LoomSearchV2Response`]. A non-success
/// status or a parse failure is an [`AppError`] (NEVER a panic; the panel shows the error string).
async fn post_loom_search_v2(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<LoomSearchV2Response, AppError> {
    let v = post_json(client, url, body).await?;
    serde_json::from_value(v).map_err(|e| AppError::Parse(e.to_string()))
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-029 Find-in-Files + Replace-in-Files transport (E4 Search).
//
// Three clients drive the native WorkspaceSearchPanel port, each binding a backend route VERIFIED
// READ-ONLY against the running `src/backend/handshake_core` + the React reference (api.ts / loom.rs /
// workspaces.rs / knowledge_documents.rs), NOT the MT-029 contract body (whose route names were partly
// stale — the recurring backend-shape lesson):
//
//   - SEARCH binds `GET /workspaces/{ws}/loom/graph-search` (handler `search_loom_graph` ->
//     `Vec<LoomGraphSearchResult>` carrying source_kind/result_kind/ref_id/title/excerpt/metadata/block).
//     This is what the React `searchLoomGraph()` actually calls (api.ts:1320-1341) — NOT the plain
//     `/loom/search` (handler `search_loom_blocks` -> `Vec<{block,score}>` with NO source_kind/ref_id,
//     so it cannot satisfy documentIdFromHit). Verified params (loom.rs `LoomSearchQueryParams` +
//     api.test.ts:771): q, source_kinds (comma-joined), tag_ids, mention_ids, case_sensitive,
//     whole_word, `regex` (NOT isRegex), path, limit (server-capped at 500), offset.
//   - BOOKMARKS bind `GET/PUT /workspaces/{ws}/search-bookmarks` (api/workspaces.rs:61-62,806-869). GET
//     returns `{workspace_id, bookmark_state:Option<Value>, ..}`; PUT body is `{bookmark_state:Value}`.
//     The bookmark blob (carried INSIDE bookmark_state) is `{schema_id:"hsk.workspace_search_bookmark_
//     state@1", bookmarks:[..]}` — the schema_id lives in the blob (RISK-6).
//   - RICH-DOC load binds `GET /knowledge/documents/{id}` -> `{document:{rich_document_id,doc_version,
//     title,content_json,crdt_document_id,..}, tree, code_nodes}`; save binds `PUT /knowledge/documents/
//     {id}/save` `{expected_version, content_json}` -> `{document:{doc_version,..}, save_receipt_event_id}`;
//     409 = optimistic-concurrency conflict (the MT-017/020 VERIFIED routes — NOT the contract's
//     /knowledge/rich-documents PATCH). The four `x-hsk-*` document identity headers are REQUIRED (a
//     missing one is a hard 400 / read-only 403), reusing the canonical DOC_* constants above.
//
// All follow the MT-020/028 off-thread shape: spawn on the app runtime, deliver into an
// `Arc<Mutex<Option<..>>>` cell the egui UI drains next frame (HBR-QUIET). Speaks `serde_json::Value`
// so it never depends on the `handshake_core` crate types.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// One graph-search hit, the native projection of the backend `LoomGraphSearchResult`. Field names
/// match the snake_case JSON EXACTLY. `metadata`/`block` are raw `serde_json::Value` so the
/// documentId-from-hit logic can read whatever keys the backend attaches without coupling to a schema.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct LoomGraphSearchHit {
    pub source_kind: String,
    pub result_kind: String,
    pub ref_id: String,
    pub title: String,
    pub excerpt: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub block: Option<serde_json::Value>,
}

/// Identity attached to every MT-029 asynchronous completion. `epoch` changes on workspace rebind and
/// `sequence` changes per operation, so an A→B→A workspace cycle cannot accept a completion from the
/// earlier A binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindInFilesOperation {
    Search,
    Preview,
    Apply,
    BookmarkLoad,
    BookmarkSave,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindInFilesStamp {
    pub workspace_id: String,
    pub operation: FindInFilesOperation,
    pub epoch: u64,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FindInFilesDelivery<T> {
    pub stamp: FindInFilesStamp,
    pub outcome: T,
}

pub type FindInFilesDeliveryQueue<T> =
    Arc<Mutex<std::collections::VecDeque<FindInFilesDelivery<T>>>>;

/// One-slot delivery cell for an off-thread paginated search: `Ok((hits, result_set_key))` carries the
/// fully-paginated hit set tagged with the search-plan key it was fetched under (the stale-result
/// guard); `Err(msg)` the failure.
pub type GraphSearchCell =
    FindInFilesDeliveryQueue<Result<(Vec<LoomGraphSearchHit>, String), String>>;

/// One-slot delivery cell for an off-thread bookmark op:
/// `Ok((bookmark_state_blob, status?, event_ledger_event_id?))` carries the saved/loaded
/// `bookmark_state` blob (re-parsed by the panel), an optional operator status string, and the
/// producer-issued durable receipt. Empty-state GETs legitimately carry no receipt; successful saves
/// always carry the nonblank event-ledger id validated by [`parse_bookmark_response`].
pub type BookmarkStateCell =
    FindInFilesDeliveryQueue<Result<(serde_json::Value, Option<String>, Option<String>), String>>;

/// The match options the search transport forwards as query params (a copy of the panel's toggles, kept
/// here so backend_client does not depend on the find_in_files module).
#[derive(Debug, Clone, Copy, Default)]
pub struct SearchMatchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub is_regex: bool,
}

/// The page size for the paginated workspace search (the React `SEARCH_PAGE_SIZE`). The backend caps a
/// single page at 500, so requesting 500 and looping until a short page is the find-all contract.
pub const SEARCH_PAGE_SIZE: u32 = 500;

fn parse_graph_search_page(v: &serde_json::Value) -> Result<Vec<LoomGraphSearchHit>, String> {
    const SOURCE_KINDS: &[&str] = &[
        "loom_block",
        "file",
        "tag_hub",
        "document",
        "symbol",
        "work_packet",
        "micro_task",
        "user_manual_page",
        "wiki_page",
    ];
    const RESULT_KINDS: &[&str] = &[
        "loom_block",
        "knowledge_entity",
        "user_manual_page",
        "wiki_page",
    ];
    let rows = v
        .as_array()
        .ok_or_else(|| "graph-search response must be an array".to_owned())?;
    rows.iter()
        .enumerate()
        .map(|(index, hit)| {
            let object = hit
                .as_object()
                .ok_or_else(|| format!("graph-search hit[{index}] must be an object"))?;
            let source_kind = object
                .get("source_kind")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    format!("graph-search hit[{index}].source_kind missing or not a string")
                })?;
            if !SOURCE_KINDS.contains(&source_kind) {
                return Err(format!(
                    "graph-search hit[{index}].source_kind is not a producer enum value"
                ));
            }
            let result_kind = object
                .get("result_kind")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    format!("graph-search hit[{index}].result_kind missing or not a string")
                })?;
            if !RESULT_KINDS.contains(&result_kind) {
                return Err(format!(
                    "graph-search hit[{index}].result_kind is not a producer enum value"
                ));
            }
            if !object
                .get("excerpt")
                .is_some_and(serde_json::Value::is_string)
            {
                return Err(format!(
                    "graph-search hit[{index}].excerpt missing or not a string"
                ));
            }
            let score = object
                .get("score")
                .and_then(serde_json::Value::as_f64)
                .ok_or_else(|| {
                    format!("graph-search hit[{index}].score missing or not a finite number")
                })?;
            if !score.is_finite() {
                return Err(format!("graph-search hit[{index}].score must be finite"));
            }
            if let Some(block) = object.get("block") {
                if !block.is_null() {
                    if !block.is_object() {
                        return Err(format!(
                            "graph-search hit[{index}].block must be absent, null, or a canonical LoomBlock object"
                        ));
                    }
                    block_to_leaf(block).map_err(|error| {
                        format!("graph-search hit[{index}].block is not canonical: {error}")
                    })?;
                }
            }
            serde_json::from_value::<LoomGraphSearchHit>(hit.clone())
                .map_err(|error| format!("graph-search hit[{index}] malformed: {error}"))
                .and_then(|parsed| {
                    if parsed.source_kind.trim().is_empty()
                        || parsed.result_kind.trim().is_empty()
                        || parsed.ref_id.trim().is_empty()
                    {
                        Err(format!(
                            "graph-search hit[{index}] has an empty identity field"
                        ))
                    } else {
                        Ok(parsed)
                    }
                })
        })
        .collect()
}

fn parse_bookmark_response(
    value: &serde_json::Value,
    expected_workspace_id: &str,
    allow_absent_state: bool,
) -> Result<(serde_json::Value, Option<String>), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "search-bookmarks response must be an object".to_owned())?;
    let workspace_id = object
        .get("workspace_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            "search-bookmarks response.workspace_id missing or not a string".to_owned()
        })?;
    if workspace_id != expected_workspace_id {
        return Err(format!(
            "search-bookmarks response workspace mismatch: expected {expected_workspace_id}, got {workspace_id}"
        ));
    }
    let bookmark_state = object
        .get("bookmark_state")
        .ok_or_else(|| "search-bookmarks response.bookmark_state is missing".to_owned())?;
    if bookmark_state.is_null() {
        if !allow_absent_state {
            return Err("search-bookmarks save response.bookmark_state is null".to_owned());
        }
        for field in ["updated_at", "event_ledger_event_id"] {
            if !object.get(field).is_some_and(serde_json::Value::is_null) {
                return Err(format!(
                    "search-bookmarks empty response.{field} must be present and null"
                ));
            }
        }
        return Ok((serde_json::json!({}), None));
    }
    if !bookmark_state.is_object() {
        return Err(
            "search-bookmarks response.bookmark_state must be an object or null".to_owned(),
        );
    }
    let updated_at = object
        .get("updated_at")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "search-bookmarks response.updated_at missing or not a string".to_owned())?;
    if updated_at.trim().is_empty() {
        return Err("search-bookmarks response.updated_at must not be blank".to_owned());
    }
    let event_ledger_event_id = object
        .get("event_ledger_event_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            "search-bookmarks response.event_ledger_event_id missing or not a string".to_owned()
        })?;
    if event_ledger_event_id.trim().is_empty() {
        return Err("search-bookmarks response.event_ledger_event_id must not be blank".to_owned());
    }
    Ok((
        bookmark_state.clone(),
        Some(event_ledger_event_id.trim().to_owned()),
    ))
}

/// REST client for the VERIFIED workspace search + search-bookmark surfaces the MT-029 Find-in-Files
/// panel binds: `GET /loom/graph-search` (paginated) and `GET/PUT /search-bookmarks`.
#[derive(Clone)]
pub struct WorkspaceSearchClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl WorkspaceSearchClient {
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn graph_search_url(&self, workspace_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/graph-search",
            self.base_url, workspace_id
        )
    }

    fn bookmarks_url(&self, workspace_id: &str) -> String {
        format!(
            "{}/workspaces/{}/search-bookmarks",
            self.base_url, workspace_id
        )
    }

    /// Build the query params for ONE search page (the VERIFIED `LoomSearchQueryParams` names; note the
    /// regex param is `regex`, NOT `isRegex`). `source_kind` is omitted for the All filter (AC-4); empty
    /// tag/path filters are omitted. Split out so a unit test asserts the EXACT wire params without a
    /// backend (the spawn path routes through this same builder).
    #[allow(clippy::too_many_arguments)]
    pub fn search_page_query(
        &self,
        query: &str,
        source_kind: Option<&str>,
        tag_filter: &str,
        path_filter: &str,
        opts: SearchMatchOptions,
        offset: u32,
    ) -> Vec<(String, String)> {
        let mut params: Vec<(String, String)> = vec![
            ("q".to_owned(), query.to_owned()),
            ("limit".to_owned(), SEARCH_PAGE_SIZE.to_string()),
            ("offset".to_owned(), offset.to_string()),
        ];
        if let Some(sk) = source_kind {
            params.push(("source_kinds".to_owned(), sk.to_owned()));
        }
        let tags: Vec<&str> = tag_filter
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if !tags.is_empty() {
            params.push(("tag_ids".to_owned(), tags.join(",")));
        }
        if !path_filter.trim().is_empty() {
            params.push(("path".to_owned(), path_filter.trim().to_owned()));
        }
        if opts.case_sensitive {
            params.push(("case_sensitive".to_owned(), "true".to_owned()));
        }
        if opts.whole_word {
            params.push(("whole_word".to_owned(), "true".to_owned()));
        }
        if opts.is_regex {
            params.push(("regex".to_owned(), "true".to_owned()));
        }
        params
    }

    /// Run the paginated workspace search off the UI thread, delivering `(all_hits, result_set_key)`
    /// into `cell`. Loops `offset += 500` until a page returns `< 500` hits (the find-all contract), so
    /// a large workspace returns the WHOLE result set (the React pagination — a partial first page would
    /// silently truncate replace-all, RISK-7).
    #[allow(clippy::too_many_arguments)]
    pub fn search_paginated(
        &self,
        workspace_id: &str,
        query: &str,
        source_kind: Option<&str>,
        tag_filter: &str,
        path_filter: &str,
        opts: SearchMatchOptions,
        result_set_key: String,
        stamp: FindInFilesStamp,
        cell: GraphSearchCell,
    ) {
        let url = self.graph_search_url(workspace_id);
        let client = self.client.clone();
        let query = query.to_owned();
        let source_kind = source_kind.map(str::to_owned);
        let tag_filter = tag_filter.to_owned();
        let path_filter = path_filter.to_owned();
        let this = self.clone();
        let operation_handle = crate::diagnostics::register_backend_operation();
        self.runtime.spawn(async move {
            let mut all: Vec<LoomGraphSearchHit> = Vec::new();
            let mut offset = 0u32;
            let result = loop {
                let params = this.search_page_query(
                    &query,
                    source_kind.as_deref(),
                    &tag_filter,
                    &path_filter,
                    opts,
                    offset,
                );
                match get_json(&client, &url, &params).await {
                    Ok(v) => {
                        let page = match parse_graph_search_page(&v) {
                            Ok(page) => page,
                            Err(error) => break Err(error),
                        };
                        let page_len = page.len();
                        all.extend(page);
                        operation_handle.tick();
                        if page_len < SEARCH_PAGE_SIZE as usize {
                            break Ok((all, result_set_key));
                        }
                        let Some(next_offset) = offset.checked_add(SEARCH_PAGE_SIZE) else {
                            break Err("graph-search pagination offset overflow".to_owned());
                        };
                        offset = next_offset;
                    }
                    Err(e) => break Err(e.to_string()),
                }
            };
            if let Ok(mut queue) = cell.lock() {
                queue.push_back(FindInFilesDelivery {
                    stamp,
                    outcome: result,
                });
            }
        });
    }

    /// Load the saved-search bookmark state off the UI thread, delivering
    /// `(bookmark_state_blob, None, producer_receipt?)` into `cell`. An absent `bookmark_state` (no
    /// bookmarks saved yet) yields an empty blob with no receipt, never an error.
    pub fn load_bookmarks(
        &self,
        workspace_id: &str,
        stamp: FindInFilesStamp,
        cell: BookmarkStateCell,
    ) {
        let url = self.bookmarks_url(workspace_id);
        let expected_workspace_id = workspace_id.to_owned();
        let client = self.client.clone();
        let operation_handle = crate::diagnostics::register_backend_operation();
        self.runtime.spawn(async move {
            let result = get_json(&client, &url, &[])
                .await
                .map_err(|e| e.to_string())
                .and_then(|v| {
                    parse_bookmark_response(&v, &expected_workspace_id, true)
                        .map(|(blob, receipt)| (blob, None, receipt))
                });
            operation_handle.tick();
            if let Ok(mut queue) = cell.lock() {
                queue.push_back(FindInFilesDelivery {
                    stamp,
                    outcome: result,
                });
            }
        });
    }

    /// Build the bookmark-save request (`PUT /search-bookmarks` with `{bookmark_state: <blob>}`). Split
    /// out so a unit test asserts the EXACT wrapper without a backend.
    pub fn save_bookmarks_request(
        &self,
        workspace_id: &str,
        bookmark_state: serde_json::Value,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Put,
            url: self.bookmarks_url(workspace_id),
            body: Some(serde_json::json!({ "bookmark_state": bookmark_state })),
        }
    }

    /// Save the bookmark state off the UI thread, delivering
    /// `(saved_blob, Some(status), Some(event_ledger_event_id))` into `cell`. The saved blob and durable
    /// receipt are re-read from the PUT response so the panel renders canonical persisted state and can
    /// attribute the terminal mutation after a workspace rebind.
    pub fn save_bookmarks(
        &self,
        workspace_id: &str,
        bookmark_state: serde_json::Value,
        status: String,
        stamp: FindInFilesStamp,
        cell: BookmarkStateCell,
    ) {
        let spec = self.save_bookmarks_request(workspace_id, bookmark_state.clone());
        let body = spec.body.unwrap_or_default();
        let expected_workspace_id = workspace_id.to_owned();
        let client = self.client.clone();
        let operation_handle = crate::diagnostics::register_backend_operation();
        self.runtime.spawn(async move {
            let result = put_json(&client, &spec.url, &body)
                .await
                .map_err(|e| e.to_string())
                .and_then(|v| {
                    parse_bookmark_response(&v, &expected_workspace_id, false)
                        .map(|(blob, receipt)| (blob, Some(status), receipt))
                });
            operation_handle.tick();
            if let Ok(mut queue) = cell.lock() {
                queue.push_back(FindInFilesDelivery {
                    stamp,
                    outcome: result,
                });
            }
        });
    }
}

/// `PUT {url}` with a JSON body, returning the parsed response body. A non-success status or parse
/// failure is an [`AppError`]. Mirrors [`post_json`] for the bookmark-save path.
async fn put_json(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let resp = client
        .put(url)
        .timeout(Duration::from_secs(10))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "PUT non-success status {}",
            resp.status()
        )));
    }
    resp.json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-029 rich-document load + save transport for the replace pipeline. Reuses the MT-020 VERIFIED
// `/knowledge/documents/{id}` + `/save` routes + the four required identity headers. The preview +
// apply orchestration (load each doc, walk content_json, save with expected_version, 409-no-overwrite,
// partial-failure receipts) is owned by the find_in_files module; this client provides the raw load +
// save primitives the module's off-thread pipeline calls.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// A loaded rich document's verified fields the native editor consumes.
#[derive(Debug, Clone, PartialEq)]
pub struct RichDocBody {
    pub document_id: String,
    pub workspace_id: String,
    pub doc_version: u64,
    pub title: String,
    pub content_json: serde_json::Value,
    pub crdt_document_id: Option<String>,
    pub authority_label: String,
    pub owner_actor_kind: Option<String>,
    pub owner_actor_id: Option<String>,
    pub project_ref: Option<String>,
    pub folder_ref: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn required_doc_string(doc: &serde_json::Value, field: &str) -> Result<String, AppError> {
    doc.get(field)
        .and_then(|x| x.as_str())
        .map(str::to_owned)
        .ok_or_else(|| AppError::Parse(format!("document.{field} missing or not a string")))
}

fn optional_doc_string(doc: &serde_json::Value, field: &str) -> Result<Option<String>, AppError> {
    match doc.get(field) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .map(|s| Some(s.to_owned()))
            .ok_or_else(|| AppError::Parse(format!("document.{field} is not a string/null"))),
    }
}

fn required_doc_u64(doc: &serde_json::Value, field: &str) -> Result<u64, AppError> {
    doc.get(field)
        .and_then(|x| x.as_u64())
        .ok_or_else(|| AppError::Parse(format!("document.{field} missing or not a u64")))
}

fn required_doc_value(doc: &serde_json::Value, field: &str) -> Result<serde_json::Value, AppError> {
    doc.get(field)
        .cloned()
        .ok_or_else(|| AppError::Parse(format!("document.{field} missing")))
}

/// The outcome of one document save: the receipt event id, or a typed conflict / error.
#[derive(Debug, Clone, PartialEq)]
pub enum DocSaveOutcome {
    /// 200: the save committed; carries the `save_receipt_event_id`.
    Saved(String),
    /// 200: content committed, but EventLedger receipt append failed or returned no usable id.
    CommittedWithoutReceipt { receipt_error: Option<String> },
    /// 409: the document changed since preview — NOT overwritten (RISK-2 data-loss control).
    Conflict,
    /// A non-409 failure (network / server / schema).
    Failed(String),
}

/// Fold the per-document `(document_id, outcome)` results of the apply pipeline into the typed
/// [`crate::find_in_files::ReplaceDelivery`] (RISK-1/MC-1). PURE so the partial-failure receipt-
/// preservation control is unit-provable without a live backend: every `Saved` receipt seen BEFORE the
/// first `Conflict`/`Failed` is preserved in `AppliedPartial`; an all-`Saved` run yields `Applied`.
/// `plan_count` is the original number of plans (carried for the success status line).
pub fn fold_apply_outcomes(
    outcomes: &[(String, DocSaveOutcome)],
    plans: &[crate::find_in_files::ReplacementPlan],
) -> crate::find_in_files::ReplaceDelivery {
    let mut receipts: Vec<String> = Vec::new();
    let mut audit_receipts = Vec::new();
    let mut failure: Option<String> = None;
    for (index, (document_id, outcome)) in outcomes.iter().enumerate() {
        let plan = plans
            .get(index)
            .filter(|plan| plan.document_id == *document_id);
        let before_sha256 = plan
            .map(|plan| plan.before_sha256.clone())
            .unwrap_or_default();
        let after_sha256 = plan
            .map(|plan| plan.after_sha256.clone())
            .unwrap_or_default();
        match outcome {
            DocSaveOutcome::Saved(receipt) => {
                receipts.push(receipt.clone());
                audit_receipts.push(crate::find_in_files::ReplaceAuditReceipt {
                    document_id: document_id.clone(),
                    before_sha256,
                    after_sha256,
                    outcome: crate::find_in_files::ReplaceAuditOutcome::Saved,
                    save_receipt_event_id: Some(receipt.clone()),
                    error: None,
                });
            }
            DocSaveOutcome::CommittedWithoutReceipt { receipt_error } => {
                audit_receipts.push(crate::find_in_files::ReplaceAuditReceipt {
                    document_id: document_id.clone(),
                    before_sha256,
                    after_sha256,
                    outcome: crate::find_in_files::ReplaceAuditOutcome::CommittedWithoutReceipt,
                    save_receipt_event_id: None,
                    error: receipt_error.clone().or_else(|| {
                        Some("document committed without a save receipt event id".to_owned())
                    }),
                });
            }
            DocSaveOutcome::Conflict => {
                let error = format!(
                    "Document {document_id} changed since preview (version conflict); not overwritten."
                );
                audit_receipts.push(crate::find_in_files::ReplaceAuditReceipt {
                    document_id: document_id.clone(),
                    before_sha256,
                    after_sha256,
                    outcome: crate::find_in_files::ReplaceAuditOutcome::Conflict,
                    save_receipt_event_id: None,
                    error: Some(error.clone()),
                });
                failure = Some(error);
                break;
            }
            DocSaveOutcome::Failed(msg) => {
                let error = format!("Save of {document_id} failed: {msg}");
                audit_receipts.push(crate::find_in_files::ReplaceAuditReceipt {
                    document_id: document_id.clone(),
                    before_sha256,
                    after_sha256,
                    outcome: crate::find_in_files::ReplaceAuditOutcome::Failed,
                    save_receipt_event_id: None,
                    error: Some(error.clone()),
                });
                failure = Some(error);
                break;
            }
        }
    }
    match failure {
        // RISK-1/MC-1: a partial failure preserves the receipts already collected.
        Some(error) => crate::find_in_files::ReplaceDelivery::AppliedPartial {
            receipts,
            audit_receipts,
            error,
        },
        None => crate::find_in_files::ReplaceDelivery::Applied {
            receipts,
            audit_receipts,
            plan_count: plans.len(),
        },
    }
}

/// Convert a cooperatively cancelled prefix into its truthful delivery. Every completed outcome is
/// folded first, so committed saves and committed-without-receipt rows remain visible; a terminal
/// conflict/failure still wins over a concurrent cancel request.
fn fold_cancelled_apply_outcomes(
    outcomes: &[(String, DocSaveOutcome)],
    plans: &[crate::find_in_files::ReplacementPlan],
) -> crate::find_in_files::ReplaceDelivery {
    let processed_plan_count = outcomes.len().min(plans.len());
    let folded = fold_apply_outcomes(outcomes, &plans[..processed_plan_count]);
    match folded {
        crate::find_in_files::ReplaceDelivery::Applied {
            receipts,
            audit_receipts,
            ..
        } => crate::find_in_files::ReplaceDelivery::Cancelled {
            receipts,
            audit_receipts,
            skipped_plan_count: plans.len().saturating_sub(processed_plan_count),
        },
        terminal => terminal,
    }
}

/// The typed delivery the find_in_files replace pipeline emits. Kept as a `serde_json::Value`-free
/// enum so the module's `ReplaceDelivery` can be built from it without backend_client depending on the
/// module. (The module defines its own `ReplaceDelivery`; this is the transport-level result feeding it
/// via the closure the module passes — see `RichDocClient::preview_replace`/`apply_plans`.)
pub type FindReplaceCell = FindInFilesDeliveryQueue<crate::find_in_files::ReplaceDelivery>;

/// REST client for the VERIFIED rich-document load + save routes the MT-029 replace pipeline drives.
#[derive(Clone)]
pub struct RichDocClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
    session_run_id: String,
}

impl RichDocClient {
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            // Share the ONE process-wide pool rather than minting an independent connection pool/TLS
            // stack: load/save now delegate to the consolidated MT-037 client (see `load_document` /
            // `save_document`), so the find/replace pipeline and the editor client share one transport.
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
            session_run_id: format!("native-editor-fif-{}", std::process::id()),
        }
    }

    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    /// The consolidated MT-037 client bound to the SAME shared pool + base URL. load/save delegate
    /// through this so there is exactly ONE document load/save wire path with ONE conflict semantic
    /// (the REUSE-NOT-DUPLICATE gate): `RichDocClient` no longer forks its own load/save transport.
    fn consolidated(&self) -> crate::backend::knowledge_documents::KnowledgeDocumentsClient {
        crate::backend::knowledge_documents::KnowledgeDocumentsClient::with_client(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    /// Run the PREVIEW pipeline off the UI thread: for each `document_id`, load the doc, walk its
    /// content_json with `regex`/`replacement`/`opts`, and accumulate a [`crate::find_in_files::
    /// ReplacementPlan`] for every doc with >= 1 match. Delivers a `ReplaceDelivery::Preview{plans,key}`
    /// (or `PreviewError`) into `cell`. A load failure aborts with `PreviewError` (no partial preview).
    #[allow(clippy::too_many_arguments)]
    pub fn preview_replace(
        &self,
        workspace_id: &str,
        document_ids: Vec<String>,
        regex: regex::Regex,
        replacement: String,
        opts: crate::find_in_files::MatchOptions,
        key: String,
        stamp: FindInFilesStamp,
        cell: FindReplaceCell,
    ) {
        let workspace_id = workspace_id.to_owned();
        let this = self.clone();
        let operation_handle = crate::diagnostics::register_backend_operation();
        self.runtime.spawn(async move {
            let mut plans = Vec::new();
            let mut error: Option<String> = None;
            for document_id in &document_ids {
                match this.load_document(document_id).await {
                    Ok(doc) => {
                        operation_handle.tick();
                        if doc.document_id != *document_id {
                            error = Some(format!(
                                "Replace preview rejected document identity mismatch: requested {document_id}, loaded {}",
                                doc.document_id
                            ));
                            break;
                        }
                        if doc.workspace_id != workspace_id {
                            error = Some(format!(
                                "Replace preview rejected cross-workspace document {document_id}: expected workspace {workspace_id}, loaded {}",
                                doc.workspace_id
                            ));
                            break;
                        }
                        let replaced = crate::find_in_files::replace_in_content(
                            &doc.content_json,
                            &regex,
                            &replacement,
                            opts,
                        );
                        if replaced.count == 0 {
                            continue;
                        }
                        let before_sha256 =
                            crate::find_in_files::content_json_sha256(&doc.content_json);
                        let after_sha256 =
                            crate::find_in_files::content_json_sha256(&replaced.content);
                        plans.push(crate::find_in_files::ReplacementPlan {
                            workspace_id: doc.workspace_id,
                            document_id: doc.document_id,
                            title: doc.title,
                            expected_version: doc.doc_version,
                            content_json_after: replaced.content,
                            before_sha256,
                            after_sha256,
                            crdt_document_id: doc.crdt_document_id,
                            match_count: replaced.count,
                            before_preview: replaced.before_preview,
                            after_preview: replaced.after_preview,
                            match_previews: replaced.match_previews,
                        });
                    }
                    Err(e) => {
                        error = Some(format!("Replace preview failed: {e}"));
                        break;
                    }
                }
            }
            let delivery = match error {
                Some(msg) => crate::find_in_files::ReplaceDelivery::PreviewError(msg),
                None => crate::find_in_files::ReplaceDelivery::Preview { plans, key },
            };
            if let Ok(mut queue) = cell.lock() {
                queue.push_back(FindInFilesDelivery {
                    stamp,
                    outcome: delivery,
                });
            }
        });
    }

    /// Run the APPLY pipeline off the UI thread: save each plan with its captured `expected_version`
    /// (optimistic concurrency). On a 409 or error, STOP but PRESERVE the receipts already collected
    /// (RISK-1/MC-1 — never a silent partial loss): delivers `AppliedPartial{receipts,error}`. On full
    /// success delivers `Applied{receipts,plan_count}`. The fold from per-document outcomes to the typed
    /// delivery lives in the pure [`fold_apply_outcomes`] so MC-1 is unit-provable without a backend.
    pub fn apply_plans(
        &self,
        workspace_id: &str,
        plans: Vec<crate::find_in_files::ReplacementPlan>,
        stamp: FindInFilesStamp,
        cell: FindReplaceCell,
        cancel: Arc<AtomicBool>,
    ) {
        let workspace_id = workspace_id.to_owned();
        let this = self.clone();
        let operation_handle = crate::diagnostics::register_backend_operation();
        self.runtime.spawn(async move {
            // Save sequentially, capturing each (document_id, outcome) and STOPPING at the first
            // non-success so a since-edited later doc is never overwritten on a stale plan. The break is
            // realized inside the pure fold below by feeding outcomes only up to (and including) the first
            // failure.
            let mut outcomes: Vec<(String, DocSaveOutcome)> = Vec::with_capacity(plans.len());
            for plan in &plans {
                if cancel.load(Ordering::Acquire) {
                    break;
                }
                if plan.workspace_id != workspace_id {
                    outcomes.push((
                        plan.document_id.clone(),
                        DocSaveOutcome::Failed(format!(
                            "replacement plan workspace mismatch: expected {workspace_id}, plan carries {}",
                            plan.workspace_id
                        )),
                    ));
                    break;
                }
                let loaded = match this.load_document(&plan.document_id).await {
                    Ok(loaded) => loaded,
                    Err(error) => {
                        outcomes.push((
                            plan.document_id.clone(),
                            DocSaveOutcome::Failed(format!(
                                "pre-save workspace revalidation failed: {error}"
                            )),
                        ));
                        break;
                    }
                };
                operation_handle.tick();
                if loaded.document_id != plan.document_id || loaded.workspace_id != workspace_id {
                    outcomes.push((
                        plan.document_id.clone(),
                        DocSaveOutcome::Failed(format!(
                            "pre-save authority mismatch: expected {workspace_id}/{}, loaded {}/{}",
                            plan.document_id,
                            loaded.workspace_id,
                            loaded.document_id
                        )),
                    ));
                    break;
                }
                if cancel.load(Ordering::Acquire) {
                    break;
                }
                let outcome = this
                    .save_document(
                        &plan.document_id,
                        &plan.content_json_after,
                        plan.expected_version,
                    )
                    .await;
                operation_handle.tick();
                let is_terminal = matches!(
                    outcome,
                    DocSaveOutcome::Conflict | DocSaveOutcome::Failed(_)
                );
                outcomes.push((plan.document_id.clone(), outcome));
                if is_terminal {
                    break;
                }
            }
            let delivery = if cancel.load(Ordering::Acquire) && outcomes.len() < plans.len() {
                fold_cancelled_apply_outcomes(&outcomes, &plans)
            } else {
                fold_apply_outcomes(&outcomes, &plans)
            };
            if let Ok(mut queue) = cell.lock() {
                queue.push_back(FindInFilesDelivery {
                    stamp,
                    outcome: delivery,
                });
            }
        });
    }

    /// `GET /knowledge/documents/{id}` -> the verified `{document:{..}}` body, narrowed into a
    /// [`RichDocBody`] for the native editor. DELEGATES to the consolidated MT-037 client (ONE wire
    /// load path — the REUSE-NOT-DUPLICATE gate); the rich [`crate::backend::knowledge_documents::
    /// DocumentLoadResponse`] is narrowed here to the fields the editor/runtime consumes. A non-success
    /// status or parse failure is an [`AppError`].
    pub async fn load_document(&self, document_id: &str) -> Result<RichDocBody, AppError> {
        let headers = crate::backend::knowledge_documents::HskDocumentHeaders::for_operator(
            self.session_run_id.clone(),
            document_id,
        );
        let resp = self
            .consolidated()
            .load_document(&headers, document_id)
            .await
            .map_err(|e| AppError::Http(e.to_string()))?;
        let doc = &resp.document;
        Ok(RichDocBody {
            document_id: required_doc_string(doc, "rich_document_id")?,
            workspace_id: required_doc_string(doc, "workspace_id")?,
            doc_version: required_doc_u64(doc, "doc_version")?,
            title: required_doc_string(doc, "title")?,
            content_json: required_doc_value(doc, "content_json")?,
            crdt_document_id: optional_doc_string(doc, "crdt_document_id")?,
            authority_label: required_doc_string(doc, "authority_label")?,
            owner_actor_kind: optional_doc_string(doc, "owner_actor_kind")?,
            owner_actor_id: optional_doc_string(doc, "owner_actor_id")?,
            project_ref: optional_doc_string(doc, "project_ref")?,
            folder_ref: optional_doc_string(doc, "folder_ref")?,
            created_at: required_doc_string(doc, "created_at")?,
            updated_at: required_doc_string(doc, "updated_at")?,
        })
    }

    /// `PUT /knowledge/documents/{id}/save` with `{expected_version, content_json}` -> the
    /// [`DocSaveOutcome`]. DELEGATES to the consolidated MT-037 client (ONE wire save path with ONE
    /// conflict semantic — the REUSE-NOT-DUPLICATE gate): a 200 returns the `save_receipt_event_id`;
    /// the consolidated [`crate::backend::knowledge_documents::KnowledgeDocumentsError::SaveConflict`]
    /// (409) maps to [`DocSaveOutcome::Conflict`] (NEVER an overwrite — so the find/replace pipeline's
    /// `AppliedPartial` / [`fold_apply_outcomes`] receipt-preservation control is unchanged); any other
    /// failure is [`DocSaveOutcome::Failed`].
    pub async fn save_document(
        &self,
        document_id: &str,
        content_json: &serde_json::Value,
        expected_version: u64,
    ) -> DocSaveOutcome {
        let headers = crate::backend::knowledge_documents::HskDocumentHeaders::for_operator(
            self.session_run_id.clone(),
            document_id,
        );
        let body = crate::backend::knowledge_documents::SaveDocumentRequest {
            // The replace pipeline carries `expected_version` as a u64; the backend optimistic-
            // concurrency token is an i64. Versions are small non-negative monotone counters, so the
            // saturating cast is lossless in practice and never produces a negative token.
            expected_version: i64::try_from(expected_version).unwrap_or(i64::MAX),
            content_json: content_json.clone(),
            crdt_document_id: None,
            crdt_snapshot_id: None,
            promotion_receipt_event_id: None,
        };
        match self
            .consolidated()
            .save_document(&headers, document_id, &body)
            .await
        {
            Ok(saved) => match saved
                .save_receipt_event_id
                .filter(|receipt| !receipt.trim().is_empty())
            {
                Some(receipt) => DocSaveOutcome::Saved(receipt),
                None => DocSaveOutcome::CommittedWithoutReceipt {
                    receipt_error: saved.receipt_error,
                },
            },
            Err(crate::backend::knowledge_documents::KnowledgeDocumentsError::SaveConflict {
                ..
            }) => DocSaveOutcome::Conflict,
            Err(e) => DocSaveOutcome::Failed(e.to_string()),
        }
    }
}

/// Save backend used by the mounted Notes editor when a document has been opened through the running
/// shell. It adapts the MT-020 [`SaveBackend`](crate::rich_editor::save::save_manager::SaveBackend)
/// state machine onto the consolidated MT-037 `/knowledge/documents/*` client, so the app path and the
/// find/replace `RichDocClient` path share the same route/header construction instead of drifting.
#[derive(Clone)]
pub struct RichDocSaveBackend {
    client: crate::backend::knowledge_documents::KnowledgeDocumentsClient,
    session_run_id: String,
    actor_id: String,
}

impl RichDocSaveBackend {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            // This constructor is mounted by `HandshakeApp` with its configured backend URL, so it is a
            // production transport even though the URL is injectable. Keep the injectable URL while
            // retaining the process-wide bounded pool; `KnowledgeDocumentsClient::with_base_url` remains
            // the isolated fresh-pool seam for its direct HTTP tests.
            client: crate::backend::knowledge_documents::KnowledgeDocumentsClient::with_client(
                shared_http_client(),
                base_url,
            ),
            session_run_id: crate::rich_editor::save::save_manager::new_session_run_id(),
            actor_id: crate::backend_client::DOC_ACTOR_ID.to_owned(),
        }
    }

    pub fn production() -> Self {
        Self {
            client: crate::backend::knowledge_documents::KnowledgeDocumentsClient::production(),
            session_run_id: crate::rich_editor::save::save_manager::new_session_run_id(),
            actor_id: crate::backend_client::DOC_ACTOR_ID.to_owned(),
        }
    }

    /// Production transport with an explicit operator/agent identity. Parallel mounted hosts use this
    /// so canonical save receipts remain attributable instead of collapsing onto one process constant.
    pub fn new_with_actor(base_url: impl Into<String>, actor_id: impl Into<String>) -> Self {
        let mut backend = Self::new(base_url);
        backend.actor_id = actor_id.into();
        backend
    }

    fn headers(
        &self,
        document_id: &str,
    ) -> crate::backend::knowledge_documents::HskDocumentHeaders {
        crate::backend::knowledge_documents::HskDocumentHeaders {
            actor_id: self.actor_id.clone(),
            kernel_task_run_id: format!("native-editor-doc-{document_id}"),
            session_run_id: self.session_run_id.clone(),
            actor_kind: Some(crate::backend_client::DOC_ACTOR_KIND.to_owned()),
            correlation_id: Some(format!(
                "native-editor-save-{document_id}-{}",
                self.session_run_id
            )),
        }
    }
}

impl crate::rich_editor::save::save_manager::SaveBackend for RichDocSaveBackend {
    fn save_document(
        &self,
        document_id: &str,
        content_json: serde_json::Value,
        expected_version: u64,
    ) -> crate::rich_editor::save::save_manager::SaveFuture {
        let client = self.client.clone();
        let headers = self.headers(document_id);
        let read_headers = headers.clone();
        let document_id = document_id.to_owned();
        Box::pin(async move {
            let body = crate::backend::knowledge_documents::SaveDocumentRequest {
                expected_version: i64::try_from(expected_version).unwrap_or(i64::MAX),
                content_json,
                crdt_document_id: None,
                crdt_snapshot_id: None,
                promotion_receipt_event_id: None,
            };
            match client.save_document(&headers, &document_id, &body).await {
                Ok(resp) => {
                    let document = rich_doc_load_from_value(
                        resp.document,
                        &document_id,
                        expected_version.saturating_add(1),
                    );
                    Ok(crate::rich_editor::save::save_manager::RichDocSaveResult {
                        document,
                        backlinks_persisted: resp.backlinks_persisted,
                        backlinks_error: resp.backlinks_error,
                        backlinks_skipped_reason: resp.backlinks_skipped_reason,
                        save_receipt_event_id: resp.save_receipt_event_id,
                        attribution: Some(
                            crate::rich_editor::save::save_manager::SaveAttribution {
                                actor_id: headers.actor_id,
                                actor_kind: headers.actor_kind.unwrap_or_default(),
                                kernel_task_run_id: headers.kernel_task_run_id,
                                session_run_id: headers.session_run_id,
                                correlation_id: headers.correlation_id,
                            },
                        ),
                    })
                }
                Err(
                    crate::backend::knowledge_documents::KnowledgeDocumentsError::SaveConflict {
                        ..
                    },
                ) => {
                    let server = match client.load_document(&read_headers, &document_id).await {
                        Ok(resp) => rich_doc_load_from_value(
                            resp.document,
                            &document_id,
                            expected_version.saturating_add(1),
                        ),
                        Err(_) => crate::rich_editor::save::save_manager::RichDocLoad {
                            rich_document_id: document_id.clone(),
                            doc_version: expected_version.saturating_add(1),
                            title: String::new(),
                            content_json: None,
                            updated_at: None,
                        },
                    };
                    Err(
                        crate::rich_editor::save::save_manager::SaveError::VersionConflict(
                            Box::new(server),
                        ),
                    )
                }
                Err(e) => Err(doc_error_to_save_error(e)),
            }
        })
    }
}

fn rich_doc_load_from_value(
    value: serde_json::Value,
    fallback_document_id: &str,
    fallback_version: u64,
) -> crate::rich_editor::save::save_manager::RichDocLoad {
    serde_json::from_value(value).unwrap_or_else(|_| {
        crate::rich_editor::save::save_manager::RichDocLoad {
            rich_document_id: fallback_document_id.to_owned(),
            doc_version: fallback_version,
            title: String::new(),
            content_json: None,
            updated_at: None,
        }
    })
}

fn doc_error_to_save_error(
    error: crate::backend::knowledge_documents::KnowledgeDocumentsError,
) -> crate::rich_editor::save::save_manager::SaveError {
    use crate::backend::knowledge_documents::KnowledgeDocumentsError as E;
    use crate::rich_editor::save::save_manager::SaveError;
    match error {
        E::Transport(msg) => SaveError::Network(msg),
        E::BadRequest(msg) => SaveError::SchemaRejected(msg),
        E::Forbidden(msg) => SaveError::Server(403, msg),
        E::NotFound(msg) => SaveError::Server(404, msg),
        E::Server(msg) => SaveError::Server(500, msg),
        E::UnexpectedStatus { status, body } => SaveError::Server(status, body),
        E::Parse(msg) => SaveError::Server(200, msg),
        E::BatchTooLarge { len, max } => {
            SaveError::SchemaRejected(format!("batch too large: {len} operations (max {max})"))
        }
        E::BatchEmpty => SaveError::SchemaRejected("batch is empty".to_owned()),
        E::TitleAmbiguous { detail } => SaveError::Server(409, detail),
        E::SaveConflict { .. } => SaveError::Server(409, "unexpected save conflict".to_owned()),
    }
}

/// Draft backend paired with [`RichDocSaveBackend`]. It reuses the same consolidated document client
/// for `GET`/`PUT`/`DELETE /draft`, preserving MT-020 recovery while keeping MT-099's authoritative
/// save proof distinct from draft writes.
#[derive(Clone)]
pub struct RichDocDraftBackend {
    client: crate::backend::knowledge_documents::KnowledgeDocumentsClient,
    session_run_id: String,
}

impl RichDocDraftBackend {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            // Draft autosave is mounted beside `RichDocSaveBackend`; it must carry the same bounded
            // connect/request policy instead of turning an injectable production URL into an unbounded
            // private client.
            client: crate::backend::knowledge_documents::KnowledgeDocumentsClient::with_client(
                shared_http_client(),
                base_url,
            ),
            session_run_id: crate::rich_editor::save::save_manager::new_session_run_id(),
        }
    }

    fn headers(
        &self,
        document_id: &str,
    ) -> crate::backend::knowledge_documents::HskDocumentHeaders {
        crate::backend::knowledge_documents::HskDocumentHeaders::for_operator(
            self.session_run_id.clone(),
            document_id,
        )
    }
}

impl crate::rich_editor::save::draft_manager::DraftBackend for RichDocDraftBackend {
    fn load_draft(
        &self,
        document_id: &str,
    ) -> crate::rich_editor::save::draft_manager::DraftLoadFuture {
        let client = self.client.clone();
        let headers = self.headers(document_id);
        let document_id = document_id.to_owned();
        Box::pin(async move {
            let resp = client
                .load_document_draft(&headers, &document_id)
                .await
                .map_err(doc_error_to_draft_error)?;
            let draft = match resp.draft {
                Some(value) if !value.is_null() => serde_json::from_value(value).ok(),
                _ => None,
            };
            Ok(
                crate::rich_editor::save::draft_manager::RichDocumentDraftLoad {
                    current_doc_version: u64::try_from(resp.current_doc_version).unwrap_or(0),
                    draft,
                },
            )
        })
    }

    fn upsert_draft(
        &self,
        document_id: &str,
        base_doc_version: u64,
        base_content_sha256: String,
        content_json: serde_json::Value,
    ) -> crate::rich_editor::save::draft_manager::DraftWriteFuture {
        let client = self.client.clone();
        let headers = self.headers(document_id);
        let document_id = document_id.to_owned();
        Box::pin(async move {
            let body = crate::backend::knowledge_documents::UpsertDraftRequest {
                base_doc_version: i64::try_from(base_doc_version).unwrap_or(i64::MAX),
                base_content_sha256,
                content_json,
            };
            client
                .upsert_document_draft(&headers, &document_id, &body)
                .await
                .map(|_| ())
                .map_err(doc_error_to_draft_error)
        })
    }

    fn clear_draft(
        &self,
        document_id: &str,
    ) -> crate::rich_editor::save::draft_manager::DraftWriteFuture {
        let client = self.client.clone();
        let headers = self.headers(document_id);
        let document_id = document_id.to_owned();
        Box::pin(async move {
            client
                .clear_document_draft(&headers, &document_id)
                .await
                .map(|_| ())
                .map_err(doc_error_to_draft_error)
        })
    }
}

fn doc_error_to_draft_error(
    error: crate::backend::knowledge_documents::KnowledgeDocumentsError,
) -> crate::rich_editor::save::draft_manager::DraftError {
    use crate::backend::knowledge_documents::KnowledgeDocumentsError as E;
    use crate::rich_editor::save::draft_manager::DraftError;
    match error {
        E::Transport(msg) => DraftError::Network(msg),
        E::BadRequest(msg) => DraftError::Server(400, msg),
        E::Forbidden(msg) => DraftError::Server(403, msg),
        E::NotFound(msg) => DraftError::Server(404, msg),
        E::SaveConflict { .. } => DraftError::Server(409, "draft conflict".to_owned()),
        E::Server(msg) => DraftError::Server(500, msg),
        E::UnexpectedStatus { status, body } => DraftError::Server(status, body),
        E::Parse(msg) => DraftError::Server(200, msg),
        E::BatchTooLarge { len, max } => DraftError::Server(
            400,
            format!("batch too large: {len} operations (max {max})"),
        ),
        E::BatchEmpty => DraftError::Server(400, "batch is empty".to_owned()),
        E::TitleAmbiguous { detail } => DraftError::Server(409, detail),
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-033 (E5 — CKC embeds / drag-in): the atelier read surface the AtelierSidePanel loads.
//
// VERIFIED READ-ONLY against the REAL running backend (`src/backend/handshake_core/src/api/atelier.rs`,
// WP-KERNEL-005), NOT taken from the MT contract body:
//   - GET /atelier/intake/batches                 -> Vec<{batch_id(uuid), source_label, source_ref,
//                                                          mode, status, created_at_utc, ...}>
//   - GET /atelier/intake/batches/{batch_id}/items -> { lane_counts, items:[{item_id(uuid), file_name,
//                                                          source_path, lane, byte_len}] }
//   - GET /atelier/command-corpus                  -> Vec<{entry_id(uuid), action_id, owner,
//                                                          execution_class, foreground_flag,
//                                                          manual_anchor}>
// These three reads are the ONLY atelier endpoints the side panel needs (the contract's two list reads +
// the per-batch items read used to expand a batch into draggable item rows). No backend edit; a gap is a
// typed blocker. Follows the MT-020/021/023/026 off-thread shape: spawn on the app's tokio runtime,
// deliver the parsed projection into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next
// frame (HBR-QUIET — the render thread is NEVER blocked on the network). Speaks `serde_json::Value` so it
// never depends on the `handshake_core` crate's types.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// One intake batch row the AtelierSidePanel lists (the verified subset of `IntakeBatchResponse`). The
/// `batch_id` is the path arg for the items read; `source_label` is the row label; `status` is a muted
/// chip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtelierBatchRow {
    pub batch_id: String,
    pub source_label: String,
    pub status: String,
}

/// One intake item row inside an expanded batch (the verified subset of `IntakeItemResponse`). The
/// `item_id` is the atelier item id used as the embed `refValue`; `file_name` is the draggable row label;
/// `source_path` is the thumbnail/path hint the contract asks the row to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtelierItemRow {
    pub item_id: String,
    pub file_name: String,
    pub source_path: String,
    pub lane: String,
    pub loom_block_id: Option<String>,
}

/// One command-corpus entry row (the verified subset of `CommandCorpusEntryResponse`). `action_id` is the
/// row label; `owner` + `execution_class` are muted detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtelierCorpusRow {
    pub entry_id: String,
    pub action_id: String,
    pub owner: String,
    pub execution_class: String,
}

/// The externally-meaningful result of one atelier side-panel load: the batches + the command corpus
/// (the two top-level sections). Items are loaded per-batch on demand via [`AtelierClient::fetch_items`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtelierSidePanelData {
    pub batches: Vec<AtelierBatchRow>,
    pub corpus: Vec<AtelierCorpusRow>,
}

/// FIFO deliveries for off-thread side-panel loads. A queue is required because a slower stale request
/// may finish after the newest request but before the UI polls; a one-slot cell would let it overwrite
/// the newer completion before generation filtering can run.
pub type AtelierSidePanelCell =
    Arc<Mutex<std::collections::VecDeque<(u64, Result<AtelierSidePanelData, String>)>>>;

/// FIFO deliveries for off-thread per-batch item loads, keyed by generation + batch id so reordered
/// completions are all observed and stale ones discarded without overwriting a newer result.
pub type AtelierItemsCell =
    Arc<Mutex<std::collections::VecDeque<(u64, String, Result<Vec<AtelierItemRow>, String>)>>>;

/// REST client for the VERIFIED atelier read surface the MT-033 AtelierSidePanel consumes.
#[derive(Clone)]
pub struct AtelierClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl AtelierClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    /// Pure request builder for `GET /atelier/intake/batches`.
    pub fn batches_request(&self) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: format!("{}/atelier/intake/batches", self.base_url),
            query: vec![],
        }
    }

    /// Pure request builder for `GET /atelier/command-corpus`.
    pub fn corpus_request(&self) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: format!("{}/atelier/command-corpus", self.base_url),
            query: vec![],
        }
    }

    /// Pure request builder for `GET /atelier/intake/batches/{batch_id}/items`.
    pub fn items_request(&self, batch_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: format!(
                "{}/atelier/intake/batches/{}/items",
                self.base_url, batch_id
            ),
            query: vec![],
        }
    }

    /// Load the batches + command corpus off the UI thread, delivering the parsed projection into `cell`.
    /// A failure of EITHER read fails the whole load (the panel surfaces the error text, never a blank
    /// half-loaded panel). The two reads run concurrently on the runtime.
    pub fn fetch_side_panel(&self, generation: u64, cell: AtelierSidePanelCell) {
        let batches_url = self.batches_request().url;
        let corpus_url = self.corpus_request().url;
        let client = self.client.clone();
        let operation_handle = crate::diagnostics::register_backend_operation();
        self.runtime.spawn(async move {
            let result = load_atelier_side_panel(&client, &batches_url, &corpus_url).await;
            operation_handle.tick();
            if let Ok(mut slot) = cell.lock() {
                slot.push_back((generation, result.map_err(|e| e.to_string())));
            }
        });
    }

    /// Load one batch's items off the UI thread, delivering `(batch_id, Ok(items))` / `(batch_id, Err)`
    /// into `cell`. The host matches the delivered `batch_id` against the currently-expanded batch so a
    /// stale response for a since-collapsed batch is discarded.
    pub fn fetch_items(&self, generation: u64, batch_id: &str, cell: AtelierItemsCell) {
        let url = self.items_request(batch_id).url;
        let client = self.client.clone();
        let id = batch_id.to_owned();
        let operation_handle = crate::diagnostics::register_backend_operation();
        self.runtime.spawn(async move {
            let result = fetch_atelier_items(&client, &url)
                .await
                .map_err(|e| e.to_string());
            operation_handle.tick();
            if let Ok(mut slot) = cell.lock() {
                slot.push_back((generation, id, result));
            }
        });
    }
}

/// GET the two side-panel reads and assemble the projection. Either read failing fails the whole load.
async fn load_atelier_side_panel(
    client: &reqwest::Client,
    batches_url: &str,
    corpus_url: &str,
) -> Result<AtelierSidePanelData, AppError> {
    let (batches, corpus) = tokio::try_join!(
        fetch_atelier_batches(client, batches_url),
        fetch_atelier_corpus(client, corpus_url)
    )?;
    Ok(AtelierSidePanelData { batches, corpus })
}

#[derive(serde::Deserialize)]
struct AtelierBatchWire {
    batch_id: String,
    source_label: String,
    status: String,
}

/// `GET {url}` and strictly parse the required `IntakeBatchResponse` identity/display fields. A malformed
/// row fails the load visibly instead of silently disappearing or acquiring a fabricated default.
async fn fetch_atelier_batches(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<AtelierBatchRow>, AppError> {
    let wire: Vec<AtelierBatchWire> = serde_json::from_value(get_json(client, url, &[]).await?)
        .map_err(|error| AppError::Parse(format!("atelier batches response malformed: {error}")))?;
    let rows: Vec<AtelierBatchRow> = wire
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            if row.batch_id.trim().is_empty()
                || row.source_label.trim().is_empty()
                || row.status.trim().is_empty()
            {
                return Err(AppError::Parse(format!(
                    "atelier batches row[{index}] has an empty required field"
                )));
            }
            uuid::Uuid::parse_str(&row.batch_id).map_err(|error| {
                AppError::Parse(format!(
                    "atelier batches row[{index}] has invalid batch_id: {error}"
                ))
            })?;
            Ok(AtelierBatchRow {
                batch_id: row.batch_id,
                source_label: row.source_label,
                status: row.status,
            })
        })
        .collect::<Result<_, _>>()?;
    let mut seen = std::collections::HashSet::with_capacity(rows.len());
    if let Some(duplicate) = rows
        .iter()
        .map(|row| row.batch_id.as_str())
        .find(|id| !seen.insert((*id).to_owned()))
    {
        return Err(AppError::Parse(format!(
            "atelier batches response contains duplicate batch_id {duplicate}"
        )));
    }
    Ok(rows)
}

#[derive(serde::Deserialize)]
struct AtelierCorpusWire {
    entry_id: String,
    action_id: String,
    owner: String,
    execution_class: String,
}

/// `GET {url}` and parse the `Vec<CommandCorpusEntryResponse>` into [`AtelierCorpusRow`]s.
async fn fetch_atelier_corpus(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<AtelierCorpusRow>, AppError> {
    let wire: Vec<AtelierCorpusWire> = serde_json::from_value(get_json(client, url, &[]).await?)
        .map_err(|error| {
            AppError::Parse(format!(
                "atelier command-corpus response malformed: {error}"
            ))
        })?;
    let rows: Vec<AtelierCorpusRow> = wire
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            if row.entry_id.trim().is_empty()
                || row.action_id.trim().is_empty()
                || row.owner.trim().is_empty()
                || row.execution_class.trim().is_empty()
            {
                return Err(AppError::Parse(format!(
                    "atelier command-corpus row[{index}] has an empty required field"
                )));
            }
            uuid::Uuid::parse_str(&row.entry_id).map_err(|error| {
                AppError::Parse(format!(
                    "atelier command-corpus row[{index}] has invalid entry_id: {error}"
                ))
            })?;
            Ok(AtelierCorpusRow {
                entry_id: row.entry_id,
                action_id: row.action_id,
                owner: row.owner,
                execution_class: row.execution_class,
            })
        })
        .collect::<Result<_, _>>()?;
    let mut seen = std::collections::HashSet::with_capacity(rows.len());
    if let Some(duplicate) = rows
        .iter()
        .map(|row| row.entry_id.as_str())
        .find(|id| !seen.insert((*id).to_owned()))
    {
        return Err(AppError::Parse(format!(
            "atelier command-corpus response contains duplicate entry_id {duplicate}"
        )));
    }
    Ok(rows)
}

#[derive(serde::Deserialize)]
struct AtelierItemsEnvelopeWire {
    items: Vec<AtelierItemWire>,
}

#[derive(serde::Deserialize)]
struct AtelierItemWire {
    item_id: String,
    file_name: String,
    source_path: String,
    lane: String,
    #[serde(default)]
    loom_block_id: Option<String>,
}

/// `GET {url}` and parse the `IntakeBatchItemsResponse.items[]` into [`AtelierItemRow`]s.
async fn fetch_atelier_items(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<AtelierItemRow>, AppError> {
    parse_atelier_items(get_json(client, url, &[]).await?)
}

fn parse_atelier_items(value: serde_json::Value) -> Result<Vec<AtelierItemRow>, AppError> {
    let envelope: AtelierItemsEnvelopeWire = serde_json::from_value(value).map_err(|error| {
        AppError::Parse(format!("atelier batch-items response malformed: {error}"))
    })?;
    let rows: Vec<AtelierItemRow> = envelope
        .items
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            if row.item_id.trim().is_empty()
                || row.file_name.trim().is_empty()
                || row.source_path.trim().is_empty()
                || row.lane.trim().is_empty()
            {
                return Err(AppError::Parse(format!(
                    "atelier batch-items row[{index}] has an empty required field"
                )));
            }
            uuid::Uuid::parse_str(&row.item_id).map_err(|error| {
                AppError::Parse(format!(
                    "atelier batch-items row[{index}] has invalid item_id: {error}"
                ))
            })?;
            Ok(AtelierItemRow {
                item_id: row.item_id,
                file_name: row.file_name,
                source_path: row.source_path,
                lane: row.lane,
                loom_block_id: row
                    .loom_block_id
                    .filter(|block_id| !block_id.trim().is_empty()),
            })
        })
        .collect::<Result<_, _>>()?;
    let mut seen = std::collections::HashSet::with_capacity(rows.len());
    if let Some(duplicate) = rows
        .iter()
        .map(|row| row.item_id.as_str())
        .find(|id| !seen.insert((*id).to_owned()))
    {
        return Err(AppError::Parse(format!(
            "atelier batch-items response contains duplicate item_id {duplicate}"
        )));
    }
    Ok(rows)
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-021 hardening tests (MAJOR #1/#2/#3): prove every menu-action backend call constructs the EXACT
// verified URL + JSON body. Two layers:
//   1. Pure request-builder assertions (`*_request`) — deterministic, no port flakiness. Because the
//      real spawn methods (`stage_paths`, `set_z_index`, `set_flag`, …) route through these SAME
//      builders, asserting the builder asserts the production request construction.
//   2. A live in-process HTTP CAPTURE server (std::net::TcpListener, no new deps) that the REAL spawn
//      path of one representative write op (stage) actually sends to — proving the client is genuinely
//      CONSUMED end-to-end (the dispatch -> client -> reqwest -> wire path is real, not just arithmetic).
// ═════════════════════════════════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    /// A current-thread runtime whose handle the clients bridge onto. Kept alive for the test scope.
    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build current-thread runtime")
    }

    const BASE: &str = "http://test.local:1234";

    #[test]
    fn atelier_items_malformed_response_fails_then_valid_response_recovers() {
        let malformed = parse_atelier_items(serde_json::json!({
            "items": [{"item_id": "not-a-uuid", "file_name": "x.png"}]
        }));
        assert!(
            malformed.is_err(),
            "missing required fields and an invalid id must fail loudly"
        );

        let rows = parse_atelier_items(serde_json::json!({
            "items": [{
                "item_id": "01900000-0000-7000-8000-000000000033",
                "file_name": "recovered.png",
                "source_path": "/atelier/recovered.png",
                "lane": "accept"
            }]
        }))
        .expect("a later valid response must recover independently");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].file_name, "recovered.png");
    }

    #[test]
    fn atelier_projection_requires_canonical_backend_relation() {
        let unresolved = crate::interop::AtelierRef::new(
            "item-without-relation",
            crate::interop::AtelierItemKind::Media,
            "Unresolved",
        );
        let error = canonical_atelier_projection_block_id(&unresolved)
            .expect_err("a frontend must not invent canonical Loom identity");
        assert!(error
            .to_string()
            .contains("no canonical Loom projection relation"));

        let mut resolved = unresolved;
        resolved.loom_block_id = Some("loom-canonical-1".to_owned());
        assert_eq!(
            canonical_atelier_projection_block_id(&resolved).unwrap(),
            "loom-canonical-1"
        );
    }

    // ── SourceControlClient: stage / unstage / discard / diff / blame ────────────────────────────────

    #[test]
    fn scm_stage_request_url_and_body() {
        let rt = rt();
        let c = SourceControlClient::new(BASE, rt.handle().clone());
        let spec = c.stage_request(ScmWriteOp::Stage, "/repo", "src/x.rs");
        assert_eq!(spec.method, HttpMethod::Post);
        assert_eq!(spec.url, "http://test.local:1234/source-control/stage");
        assert_eq!(
            spec.body.unwrap(),
            serde_json::json!({ "repo_path": "/repo", "paths": ["src/x.rs"] })
        );
    }

    #[test]
    fn scm_unstage_request_uses_unstage_segment() {
        let rt = rt();
        let c = SourceControlClient::new(BASE, rt.handle().clone());
        let spec = c.stage_request(ScmWriteOp::Unstage, "/repo", "src/x.rs");
        assert_eq!(spec.url, "http://test.local:1234/source-control/unstage");
        assert_eq!(
            spec.body.unwrap(),
            serde_json::json!({ "repo_path": "/repo", "paths": ["src/x.rs"] })
        );
    }

    #[test]
    fn scm_discard_request_carries_confirmed_flag() {
        let rt = rt();
        let c = SourceControlClient::new(BASE, rt.handle().clone());
        // confirmed:false is the V1 stub default — a safe 409 no-op, never a destructive discard.
        let spec = c.discard_request("/repo", "src/x.rs", false);
        assert_eq!(spec.url, "http://test.local:1234/source-control/discard");
        assert_eq!(
            spec.body.unwrap(),
            serde_json::json!({ "repo_path": "/repo", "paths": ["src/x.rs"], "confirmed": false })
        );
    }

    #[test]
    fn scm_diff_request_query_carries_scope() {
        let rt = rt();
        let c = SourceControlClient::new(BASE, rt.handle().clone());
        let worktree = c.diff_request("/repo", "src/x.rs", ScmDiffScope::Worktree);
        assert_eq!(worktree.method, HttpMethod::Get);
        assert_eq!(worktree.url, "http://test.local:1234/source-control/diff");
        assert_eq!(
            worktree.query,
            vec![
                ("repo_path".to_owned(), "/repo".to_owned()),
                ("path".to_owned(), "src/x.rs".to_owned()),
                ("scope".to_owned(), "worktree".to_owned()),
            ]
        );
        let staged = c.diff_request("/repo", "src/x.rs", ScmDiffScope::Staged);
        assert_eq!(staged.query.last().unwrap().1, "staged");
    }

    #[test]
    fn scm_blame_request_url_and_query() {
        let rt = rt();
        let c = SourceControlClient::new(BASE, rt.handle().clone());
        let spec = c.blame_request("/repo", "src/x.rs");
        assert_eq!(spec.url, "http://test.local:1234/source-control/blame");
        assert_eq!(
            spec.query,
            vec![
                ("repo_path".to_owned(), "/repo".to_owned()),
                ("path".to_owned(), "src/x.rs".to_owned()),
            ]
        );
    }

    // ── CanvasClient: set_z_index (front/back) / remove placement / remove visual edge ───────────────

    #[test]
    fn canvas_set_z_index_request_url_and_body() {
        let rt = rt();
        let c = CanvasClient::new(BASE, rt.handle().clone());
        let spec = c.set_z_index_request("ws1", "p9", 1_000_000);
        assert_eq!(spec.method, HttpMethod::Patch);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/canvas-placements/p9"
        );
        assert_eq!(
            spec.body.unwrap(),
            serde_json::json!({ "z_index": 1_000_000 })
        );
    }

    #[test]
    fn canvas_remove_placement_request_is_delete_no_body() {
        let rt = rt();
        let c = CanvasClient::new(BASE, rt.handle().clone());
        let spec = c.remove_placement_request("ws1", "p9");
        assert_eq!(spec.method, HttpMethod::Delete);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/canvas-placements/p9"
        );
        assert!(spec.body.is_none());
    }

    #[test]
    fn canvas_remove_visual_edge_request_targets_visual_edge_endpoint() {
        let rt = rt();
        let c = CanvasClient::new(BASE, rt.handle().clone());
        let spec = c.remove_visual_edge_request("ws1", "ve7");
        assert_eq!(spec.method, HttpMethod::Delete);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/canvas-visual-edges/ve7"
        );
        assert!(spec.body.is_none());
    }

    // ── LoomBlockClient: set_flag (AC#73) + rename ───────────────────────────────────────────────────

    #[test]
    fn loom_set_flag_pinned_body_contains_pinned() {
        let rt = rt();
        let c = LoomBlockClient::new(BASE, rt.handle().clone());
        let spec = c.set_flag_request("ws1", "b3", LoomBlockFlag::Pinned, true);
        assert_eq!(spec.method, HttpMethod::Patch);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/blocks/b3"
        );
        let body = spec.body.unwrap();
        // AC#73: the serialized body contains the `pinned` flag, and ONLY that flag (not favorite).
        assert_eq!(body, serde_json::json!({ "pinned": true }));
        assert!(body.get("favorite").is_none());
    }

    #[test]
    fn loom_set_flag_favorite_body_contains_favorite() {
        let rt = rt();
        let c = LoomBlockClient::new(BASE, rt.handle().clone());
        let spec = c.set_flag_request("ws1", "b3", LoomBlockFlag::Favorite, false);
        let body = spec.body.unwrap();
        assert_eq!(body, serde_json::json!({ "favorite": false }));
        assert!(body.get("pinned").is_none());
    }

    #[test]
    fn loom_rename_request_body_contains_title() {
        let rt = rt();
        let c = LoomBlockClient::new(BASE, rt.handle().clone());
        let spec = c.rename_request("ws1", "b3", "New Title", Some("2026-07-16T10:20:30Z"));
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/blocks/b3"
        );
        assert_eq!(
            spec.body.unwrap(),
            serde_json::json!({
                "title": "New Title",
                "expected_updated_at": "2026-07-16T10:20:30Z"
            })
        );
    }

    #[test]
    fn canvas_title_rename_request_carries_concurrency_token() {
        let rt = rt();
        let client = CanvasTitleClient::new(BASE, rt.handle().clone());
        let spec =
            client.rename_request("canvas-7", "Architecture map", Some("2026-07-16T10:20:30Z"));
        assert_eq!(spec.method, HttpMethod::Patch);
        assert_eq!(spec.url, "http://test.local:1234/canvases/canvas-7");
        assert_eq!(
            spec.body.unwrap(),
            serde_json::json!({
                "title": "Architecture map",
                "expected_updated_at": "2026-07-16T10:20:30Z",
            })
        );
    }

    // ── MT-023 DrawerDataClient: verified view-count + daily-journal requests ────────────────────────

    #[test]
    fn drawer_notes_count_request_targets_views_all_with_note_content_type() {
        let rt = rt();
        let c = DrawerDataClient::new(BASE, rt.handle().clone());
        let spec = c.count_request("ws1", DrawerDataKind::Notes);
        assert_eq!(spec.method, HttpMethod::Get);
        // VERIFIED endpoint: /loom/views/all (NOT the contract's stale /loom/views/table).
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/views/all"
        );
        // VERIFIED content_type: note (the contract's `list` does not exist as a content_type).
        assert_eq!(
            spec.query,
            vec![("content_type".to_owned(), "note".to_owned())]
        );
    }

    #[test]
    fn drawer_lists_count_request_maps_to_view_def_content_type() {
        let rt = rt();
        let c = DrawerDataClient::new(BASE, rt.handle().clone());
        let spec = c.count_request("ws1", DrawerDataKind::Lists);
        // The contract's "Lists" maps to saved block-collection views → content_type=view_def (the
        // real, countable surface; `list` is not a valid LoomBlockContentType — disclosed deviation).
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/views/all"
        );
        assert_eq!(
            spec.query,
            vec![("content_type".to_owned(), "view_def".to_owned())]
        );
    }

    #[test]
    fn drawer_agenda_request_is_put_to_daily_journal() {
        let rt = rt();
        let c = DrawerDataClient::new(BASE, rt.handle().clone());
        let spec = c.journal_request("ws1", "2026-06-20");
        // VERIFIED endpoint: PUT /loom/journals/{date} (open_daily_journal, get-or-create, no body).
        assert_eq!(spec.method, HttpMethod::Put);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/journals/2026-06-20"
        );
        assert!(spec.body.is_none());
    }

    #[test]
    #[should_panic(expected = "count_request requires a content_type")]
    fn drawer_count_request_rejects_agenda_kind() {
        let rt = rt();
        let c = DrawerDataClient::new(BASE, rt.handle().clone());
        // Agenda has no content_type (it uses the journal PUT); building a count request for it is a
        // programmer error, caught loudly rather than silently sending a malformed query.
        let _ = c.count_request("ws1", DrawerDataKind::Agenda);
    }

    // ── End-to-end live capture: the REAL spawn path sends the real request on the wire ─────────────

    /// Captured raw HTTP request: the request line (`METHOD path HTTP/1.1`) + the body after the blank
    /// line. Proves the client genuinely CONSTRUCTED and SENT the request (not just built a spec).
    struct Captured {
        request_line: String,
        body: String,
    }

    /// Bind an ephemeral localhost port, accept ONE connection, read the request, reply `200 {}`, and
    /// return the captured request line + body. No new deps — raw std::net + a tiny manual HTTP read.
    fn capture_one_request(listener: std::net::TcpListener) -> Captured {
        use std::io::{Read, Write};
        let (mut stream, _) = listener.accept().expect("accept connection");
        let mut buf = [0u8; 8192];
        let mut data = Vec::new();
        // Read until we have headers + the (small) JSON body. One read is enough for these tiny bodies,
        // but loop until a blank line is seen and the declared Content-Length is satisfied.
        loop {
            let n = stream.read(&mut buf).expect("read request");
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&data);
            if let Some(hdr_end) = text.find("\r\n\r\n") {
                let header = &text[..hdr_end];
                let body_so_far = &text[hdr_end + 4..];
                let content_len = header
                    .lines()
                    .find_map(|l| {
                        let l = l.to_ascii_lowercase();
                        l.strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().ok())
                    })
                    .flatten()
                    .unwrap_or(0);
                if body_so_far.len() >= content_len {
                    break;
                }
            }
        }
        let text = String::from_utf8_lossy(&data).into_owned();
        let request_line = text.lines().next().unwrap_or("").to_owned();
        let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_owned();
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}");
        let _ = stream.flush();
        Captured { request_line, body }
    }

    #[test]
    fn scm_stage_spawn_sends_real_request_on_the_wire() {
        // Real multi-thread runtime so the spawned task actually runs while the test thread blocks on
        // the capture server (the production off-thread path).
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build multi-thread runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");

        let client = SourceControlClient::new(base, rt.handle().clone());
        let cell: ScmReceiptCell = Arc::new(Mutex::new(None));
        // Drive the REAL spawn path (the same call apply_source_control_event makes).
        client.stage_paths(ScmWriteOp::Stage, "/repo", "src/x.rs", cell.clone());

        // Capture the request the spawned task sends on the wire.
        let captured = capture_one_request(listener);
        assert_eq!(captured.request_line, "POST /source-control/stage HTTP/1.1");
        let body: serde_json::Value =
            serde_json::from_str(captured.body.trim()).expect("json body");
        assert_eq!(
            body,
            serde_json::json!({ "repo_path": "/repo", "paths": ["src/x.rs"] })
        );

        // The delivery cell receives Ok(()) after the 200 — proving the full round-trip is consumed.
        rt.block_on(async {
            for _ in 0..50 {
                if cell.lock().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        let delivered = cell.lock().unwrap().take();
        assert_eq!(delivered, Some(Ok(())), "stage round-trip delivered Ok(())");
    }

    /// Accept ONE connection, read the request, reply `200` with `reply_body`, and return the captured
    /// request line. Variant of [`capture_one_request`] that lets the test control the response body so
    /// the client's parse path (e.g. counting `blocks`) is proven end-to-end, not just the request line.
    fn capture_one_request_reply(listener: std::net::TcpListener, reply_body: &str) -> String {
        use std::io::{Read, Write};
        let (mut stream, _) = listener.accept().expect("accept connection");
        let mut buf = [0u8; 8192];
        let mut data = Vec::new();
        loop {
            let n = stream.read(&mut buf).expect("read request");
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&data);
            if let Some(hdr_end) = text.find("\r\n\r\n") {
                let header = &text[..hdr_end];
                let body_so_far = &text[hdr_end + 4..];
                let content_len = header
                    .lines()
                    .find_map(|l| {
                        let l = l.to_ascii_lowercase();
                        l.strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().ok())
                    })
                    .flatten()
                    .unwrap_or(0);
                if body_so_far.len() >= content_len {
                    break;
                }
            }
        }
        let text = String::from_utf8_lossy(&data).into_owned();
        let request_line = text.lines().next().unwrap_or("").to_owned();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            reply_body.len(),
            reply_body
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
        request_line
    }

    #[test]
    fn drawer_count_spawn_sends_real_get_and_parses_blocks_len() {
        // The REAL fetch_count spawn path: it must GET /loom/views/all?content_type=note on the wire,
        // parse `blocks.len()` from the verified LoomViewResponse::All shape, and deliver the count. This
        // proves the client is genuinely CONSUMED end-to-end (dispatch → spawn → reqwest → parse → cell),
        // not just that the request builder is correct.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build multi-thread runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");

        let client = DrawerDataClient::new(base, rt.handle().clone());
        let cell: DrawerDataCell = Arc::new(Mutex::new(None));
        client.fetch_count("ws1", DrawerDataKind::Notes, cell.clone());

        // The verified response shape: two note blocks → badge_count 2.
        let reply = r#"{"view_type":"all","blocks":[{"block_id":"b1"},{"block_id":"b2"}]}"#;
        let request_line = capture_one_request_reply(listener, reply);
        assert_eq!(
            request_line, "GET /workspaces/ws1/loom/views/all?content_type=note HTTP/1.1",
            "REAL spawn path hits the VERIFIED /loom/views/all endpoint with content_type=note"
        );

        rt.block_on(async {
            for _ in 0..50 {
                if cell.lock().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        let delivered = cell.lock().unwrap().take().expect("drawer count delivered");
        assert_eq!(
            delivered,
            (
                DrawerDataKind::Notes,
                Ok(DrawerCardData {
                    badge_count: 2,
                    subtitle: "2 items".to_owned()
                })
            ),
            "blocks.len() parsed as the badge count from the verified response shape"
        );
    }

    // ── MT-024 DrawerActionClient: verified pin / discard / stow / attach-evidence requests ──────────

    #[test]
    fn drawer_action_pin_request_uses_pin_order_field_not_ordinal() {
        let rt = rt();
        let c = DrawerActionClient::new(BASE, rt.handle().clone());
        let spec = c.pin_order_request("ws1", "b3", 0);
        assert_eq!(spec.method, HttpMethod::Put);
        // VERIFIED endpoint: /pin-order (MT-183 set_loom_block_pin_order).
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/blocks/b3/pin-order"
        );
        // VERIFIED body field: pin_order (NOT the contract's `ordinal`).
        assert_eq!(spec.body.unwrap(), serde_json::json!({ "pin_order": 0 }));
    }

    #[test]
    fn drawer_action_discard_request_is_delete_no_body() {
        let rt = rt();
        let c = DrawerActionClient::new(BASE, rt.handle().clone());
        let spec = c.discard_request("ws1", "b3");
        assert_eq!(spec.method, HttpMethod::Delete);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/blocks/b3"
        );
        assert!(spec.body.is_none(), "DELETE carries no body");
    }

    #[test]
    fn drawer_action_stow_request_posts_a_tag_edge_to_the_stash_hub() {
        let rt = rt();
        let c = DrawerActionClient::new(BASE, rt.handle().clone());
        let spec = c.stow_request("ws1", "b3");
        assert_eq!(spec.method, HttpMethod::Post);
        // VERIFIED endpoint: /loom/edges (the contract's metadata/content_type PATCH does not exist).
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws1/loom/edges");
        assert_eq!(
            spec.body.unwrap(),
            serde_json::json!({
                "source_block_id": "b3",
                "target_block_id": STASH_TAG_HUB_BLOCK_ID,
                "edge_type": "tag",
                "created_by": "user",
                "target_title": STASH_TAG_TITLE,
            })
        );
    }

    #[test]
    fn drawer_action_attach_evidence_request_uses_valid_enums_and_carries_block_id() {
        let rt = rt();
        let c = DrawerActionClient::new(BASE, rt.handle().clone());
        let spec = c.attach_evidence_request("ws1", "b3", "My Note", Some("job-9"));
        assert_eq!(spec.method, HttpMethod::Post);
        assert_eq!(spec.url, "http://test.local:1234/diagnostics");
        let body = spec.body.unwrap();
        assert_eq!(body["title"], serde_json::json!("Evidence: My Note"));
        // VERIFIED enums: source/surface "user"/"drawer" do not exist; "system" is the honest valid value.
        assert_eq!(body["source"], serde_json::json!("system"));
        assert_eq!(body["surface"], serde_json::json!("system"));
        assert_eq!(body["severity"], serde_json::json!("info"));
        assert_eq!(body["job_id"], serde_json::json!("job-9"));
        // The stashed block id is carried in the VERIFIED evidence_refs.artifact_hashes map.
        assert_eq!(
            body["evidence_refs"]["artifact_hashes"]["b3"],
            serde_json::json!("b3")
        );
    }

    #[test]
    fn drawer_action_attach_evidence_omits_job_id_when_none() {
        let rt = rt();
        let c = DrawerActionClient::new(BASE, rt.handle().clone());
        let spec = c.attach_evidence_request("ws1", "b3", "My Note", None);
        let body = spec.body.unwrap();
        assert!(
            body.get("job_id").is_none(),
            "no job_id key when there is no active job"
        );
    }

    #[test]
    fn drawer_action_discard_spawn_sends_real_delete_on_the_wire() {
        // The REAL discard spawn path: it must DELETE /workspaces/ws1/loom/blocks/b3 on the wire and
        // deliver Ok(()) after the 200. Proves the client is genuinely CONSUMED end-to-end (the
        // dispatch -> spawn -> reqwest -> wire -> cell path is real), the MT-021 capture pattern.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build multi-thread runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");

        let client = DrawerActionClient::new(base, rt.handle().clone());
        let cell: DrawerActionCell = Arc::new(Mutex::new(None));
        client.discard("ws1", "b3", cell.clone());

        let captured = capture_one_request(listener);
        assert_eq!(
            captured.request_line,
            "DELETE /workspaces/ws1/loom/blocks/b3 HTTP/1.1"
        );

        rt.block_on(async {
            for _ in 0..50 {
                if cell.lock().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        assert_eq!(
            cell.lock().unwrap().take(),
            Some(Ok(())),
            "discard round-trip delivered Ok(())"
        );
    }

    #[test]
    fn drawer_action_stow_spawn_sends_real_tag_edge_post_on_the_wire() {
        // The REAL stow spawn path: POST /workspaces/ws1/loom/edges with the verified tag-edge body.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build multi-thread runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");

        let client = DrawerActionClient::new(base, rt.handle().clone());
        let cell: DrawerActionCell = Arc::new(Mutex::new(None));
        client.stow("ws1", "b3", cell.clone());

        let captured = capture_one_request(listener);
        assert_eq!(
            captured.request_line,
            "POST /workspaces/ws1/loom/edges HTTP/1.1"
        );
        let body: serde_json::Value =
            serde_json::from_str(captured.body.trim()).expect("json body");
        assert_eq!(body["source_block_id"], serde_json::json!("b3"));
        assert_eq!(body["edge_type"], serde_json::json!("tag"));
        assert_eq!(
            body["target_block_id"],
            serde_json::json!(STASH_TAG_HUB_BLOCK_ID)
        );

        rt.block_on(async {
            for _ in 0..50 {
                if cell.lock().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        assert_eq!(
            cell.lock().unwrap().take(),
            Some(Ok(())),
            "stow round-trip delivered Ok(())"
        );
    }

    #[test]
    fn drawer_count_missing_blocks_field_defaults_to_zero() {
        // CONTROL-023-D: a response that omits `blocks` (or has it null) must default the count to 0
        // without erroring the card. Proven through the REAL spawn + parse path.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build multi-thread runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");

        let client = DrawerDataClient::new(base, rt.handle().clone());
        let cell: DrawerDataCell = Arc::new(Mutex::new(None));
        client.fetch_count("ws1", DrawerDataKind::Lists, cell.clone());

        // A response with NO blocks field (the red-team "API omits the field" case).
        let _ = capture_one_request_reply(listener, r#"{"view_type":"all"}"#);

        rt.block_on(async {
            for _ in 0..50 {
                if cell.lock().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        let delivered = cell.lock().unwrap().take().expect("drawer count delivered");
        assert_eq!(
            delivered,
            (
                DrawerDataKind::Lists,
                Ok(DrawerCardData {
                    badge_count: 0,
                    subtitle: "0 items".to_owned()
                })
            ),
            "missing blocks field defaults to 0 (CONTROL-023-D), never an error"
        );
    }

    // ── MT-022 LoomFolderClient: malformed-success responses fail closed ───────────────────────────

    #[test]
    fn loom_folder_list_rejects_successful_non_array_body() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("bounded folder client runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let base = format!(
            "http://{}",
            listener.local_addr().expect("listener address")
        );
        let client = LoomFolderClient::new(base, rt.handle().clone());
        let cell: FolderListCell = Arc::new(Mutex::new(VecDeque::new()));
        client.fetch_folders("ws-malformed", 7, 11, Arc::clone(&cell));
        let _ = capture_one_request_reply(listener, r#"{"folders":[]}"#);
        rt.block_on(async {
            for _ in 0..100 {
                if !cell.lock().unwrap().is_empty() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            panic!("folder-list malformed response was not delivered within 1 second");
        });
        let (workspace, epoch, sequence, result) = cell
            .lock()
            .unwrap()
            .pop_front()
            .expect("folder list delivery");
        assert_eq!(
            (workspace.as_str(), epoch, sequence),
            ("ws-malformed", 7, 11)
        );
        assert!(result
            .expect_err("successful non-array folder body must fail closed")
            .contains("must be an array"));
    }

    #[test]
    fn loom_folder_list_rejects_malformed_array_row() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("bounded folder client runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let base = format!(
            "http://{}",
            listener.local_addr().expect("listener address")
        );
        let client = LoomFolderClient::new(base, rt.handle().clone());
        let cell: FolderListCell = Arc::new(Mutex::new(VecDeque::new()));
        client.fetch_folders("ws-malformed", 8, 12, Arc::clone(&cell));
        let _ = capture_one_request_reply(listener, r#"[{"folder_id":"", "name":"empty id"}]"#);
        rt.block_on(async {
            for _ in 0..100 {
                if !cell.lock().unwrap().is_empty() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            panic!("folder-list malformed row was not delivered within 1 second");
        });
        let (_, _, _, result) = cell
            .lock()
            .unwrap()
            .pop_front()
            .expect("folder list delivery");
        assert!(result
            .expect_err("malformed folder row must fail closed")
            .contains("folder_id must be a non-empty string"));
    }

    #[test]
    fn loom_folder_children_fetches_every_offset_page_without_truncation() {
        fn wire_block(index: usize) -> serde_json::Value {
            serde_json::json!({
                "block_id": format!("block-{index:04}"),
                "workspace_id": "ws-paged",
                "content_type": "note",
                "title": format!("Block {index}"),
                "pinned": false,
                "favorite": false,
                "pin_order": null,
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z",
                "derived": {}
            })
        }

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("bounded folder client runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let first_listener = listener.try_clone().expect("clone page listener");
        let base = format!(
            "http://{}",
            listener.local_addr().expect("listener address")
        );
        let client = LoomFolderClient::new(base, rt.handle().clone());
        let cell: FolderChildrenCell = Arc::new(Mutex::new(None));
        client.fetch_folder_blocks("ws-paged", "folder-paged", 3, 7, Arc::clone(&cell));

        let first_page = serde_json::to_string(&(0..500).map(wire_block).collect::<Vec<_>>())
            .expect("encode full first page");
        let first_request = capture_one_request_reply(first_listener, &first_page);
        let second_page = serde_json::to_string(&vec![wire_block(500)]).expect("encode final page");
        let second_request = capture_one_request_reply(listener, &second_page);

        rt.block_on(async {
            for _ in 0..100 {
                if cell.lock().unwrap().is_some() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            panic!("paged folder children were not delivered within 1 second");
        });
        assert!(
            first_request.contains("limit=500&offset=0"),
            "{first_request}"
        );
        assert!(
            second_request.contains("limit=500&offset=500"),
            "{second_request}"
        );
        let (_, _, epoch, sequence, result) = cell.lock().unwrap().take().expect("paged delivery");
        assert_eq!((epoch, sequence), (3, 7));
        let leaves = result.expect("all pages parse");
        assert_eq!(leaves.len(), 501);
        assert_eq!(
            leaves.first().map(|leaf| leaf.block_id.as_str()),
            Some("block-0000")
        );
        assert_eq!(
            leaves.last().map(|leaf| leaf.block_id.as_str()),
            Some("block-0500")
        );
    }

    #[test]
    fn loom_folder_children_rejects_malformed_array_row() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("bounded folder client runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let base = format!(
            "http://{}",
            listener.local_addr().expect("listener address")
        );
        let client = LoomFolderClient::new(base, rt.handle().clone());
        let cell: FolderChildrenCell = Arc::new(Mutex::new(None));
        client.fetch_folder_blocks("ws-malformed", "folder-1", 9, 13, Arc::clone(&cell));
        let _ = capture_one_request_reply(listener, r#"[{"title":"missing id"}]"#);
        rt.block_on(async {
            for _ in 0..100 {
                if cell.lock().unwrap().is_some() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            panic!("folder-children malformed response was not delivered within 1 second");
        });
        let (workspace, folder, epoch, sequence, result) = cell
            .lock()
            .unwrap()
            .take()
            .expect("folder children delivery");
        assert_eq!(
            (workspace.as_str(), folder.as_str(), epoch, sequence),
            ("ws-malformed", "folder-1", 9, 13)
        );
        assert!(result
            .expect_err("malformed folder child row must fail closed")
            .contains("block_id must be a non-empty string"));
    }

    #[test]
    fn loom_folder_parsers_reject_wrong_types_but_allow_absent_optional_fields() {
        let row = folder_to_row(&serde_json::json!({
            "folder_id": "folder-1",
            "workspace_id": "ws-1",
            "name": "Inbox",
            "sort_mode": "manual",
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }))
        .expect("absent optional folder fields are legitimate");
        assert_eq!(row.parent_folder_id, None);
        assert_eq!(row.color, None);

        for (field, bad) in [
            ("name", serde_json::json!(7)),
            ("parent_folder_id", serde_json::json!(7)),
            ("color", serde_json::json!(false)),
            ("sort_order", serde_json::json!("first")),
            ("project_ref", serde_json::json!([])),
            ("created_at", serde_json::json!(false)),
        ] {
            let mut malformed = serde_json::json!({
                "folder_id":"folder-1", "workspace_id":"ws-1", "name":"Inbox",
                "sort_mode":"manual", "created_at":"2026-01-01T00:00:00Z",
                "updated_at":"2026-01-01T00:00:00Z"
            });
            malformed[field] = bad;
            assert!(
                folder_to_row(&malformed).is_err(),
                "must fail closed: {malformed}"
            );
        }

        let named_color = folder_to_row(&serde_json::json!({
            "folder_id":"folder-color", "workspace_id":"ws-1", "name":"Color",
            "color":"red", "sort_mode":"manual", "created_at":"2026-01-01T00:00:00Z",
            "updated_at":"2026-01-01T00:00:00Z"
        }))
        .expect("named colors are legal backend values");
        assert_eq!(named_color.color.as_deref(), Some("red"));

        let leaf = block_to_leaf(&serde_json::json!({
            "block_id": "block-1",
            "workspace_id": "ws-1",
            "title": null,
            "content_type": "note",
            "pinned": false,
            "favorite": false,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "derived": {}
        }))
        .expect("nullable optional title is legitimate");
        assert_eq!(leaf.title, "block-1");
        for (field, bad) in [
            ("title", serde_json::json!(9)),
            ("content_type", serde_json::json!(false)),
            ("pinned", serde_json::json!("false")),
            ("pin_order", serde_json::json!([])),
            ("derived", serde_json::json!(null)),
            ("document_id", serde_json::json!({})),
        ] {
            let mut malformed = serde_json::json!({
                "block_id":"block-1", "workspace_id":"ws-1", "title":"Note",
                "content_type":"note", "pinned":false, "favorite":false,
                "created_at":"2026-01-01T00:00:00Z", "updated_at":"2026-01-01T00:00:00Z",
                "derived":{}
            });
            malformed[field] = bad;
            assert!(
                block_to_leaf(&malformed).is_err(),
                "must fail closed: {malformed}"
            );
        }
    }

    #[test]
    fn loom_folder_crud_builders_match_verified_routes_and_partial_bodies() {
        let rt = rt();
        let client = LoomFolderClient::new(BASE, rt.handle().clone());
        let create = client.create_folder_request("ws-7", "Child", Some("parent-1"), Some(4));
        assert_eq!(create.method, HttpMethod::Post);
        assert_eq!(
            create.url,
            "http://test.local:1234/workspaces/ws-7/loom/folders"
        );
        assert_eq!(
            create.body,
            Some(serde_json::json!({"name":"Child","parent_folder_id":"parent-1","sort_order":4}))
        );

        let rename = client.rename_folder_request("ws-7", "folder-1", "Renamed");
        assert_eq!(rename.method, HttpMethod::Patch);
        assert_eq!(rename.body, Some(serde_json::json!({"name":"Renamed"})));

        let move_root = client.move_folder_request("ws-7", "folder-1", None, None);
        assert_eq!(move_root.method, HttpMethod::Patch);
        assert_eq!(
            move_root.body,
            Some(serde_json::json!({"parent_folder_id":null,"sort_order":null}))
        );

        let delete = client.delete_folder_request("ws-7", "folder-1");
        assert_eq!(delete.method, HttpMethod::Delete);
        assert_eq!(delete.body, None);
    }

    // ── MT-021 LoomGraphClient: verified URL/query builders + a live round-trip parse ────────────────

    #[test]
    fn loom_graph_global_request_url() {
        let rt = rt();
        let c = LoomGraphClient::new(BASE, rt.handle().clone());
        let spec = c.global_request("ws-7");
        assert_eq!(spec.method, HttpMethod::Get);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws-7/loom/graph/global"
        );
        assert_eq!(
            spec.query,
            vec![
                ("node_limit".to_owned(), "5000".to_owned()),
                ("hub_degree_threshold".to_owned(), "0".to_owned()),
            ],
            "global graph disables hub suppression and requests the backend hard ceiling"
        );
    }

    #[test]
    fn loom_graph_local_request_url_and_query() {
        let rt = rt();
        let c = LoomGraphClient::new(BASE, rt.handle().clone());
        let spec = c.local_request("ws-7", "block-42");
        assert_eq!(spec.method, HttpMethod::Get);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws-7/loom/graph/local"
        );
        // The focused block's stable id drives canonical traversal; title search is not involved.
        assert_eq!(
            spec.query,
            vec![
                ("start_block_id".to_owned(), "block-42".to_owned()),
                ("max_depth".to_owned(), "2".to_owned()),
                ("node_limit".to_owned(), "200".to_owned()),
            ]
        );
    }

    /// WP-KERNEL-012 MT-080 (AC-080-3 / MT-060): the depth-parameterized builder carries the NEW
    /// `max_depth` (the re-query the host fires on `GraphEvent::DepthChanged`) on the SAME verified
    /// endpoint, and clamps an out-of-range depth into `[MIN..=MAX]_BACKLINK_DEPTH` (RISK-080-3).
    #[test]
    fn loom_graph_local_request_with_depth_carries_and_clamps_max_depth() {
        let rt = rt();
        let c = LoomGraphClient::new(BASE, rt.handle().clone());

        // A valid in-range depth is carried verbatim on the SAME graph/local URL (NO new endpoint).
        let spec = c.local_request_with_depth("ws-7", "block-42", 4);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws-7/loom/graph/local"
        );
        assert_eq!(
            spec.query,
            vec![
                ("start_block_id".to_owned(), "block-42".to_owned()),
                ("max_depth".to_owned(), "4".to_owned()),
                ("node_limit".to_owned(), "200".to_owned()),
            ],
            "the new depth replaces the default max_depth on the verified endpoint"
        );

        // An abusive over-range depth clamps DOWN to MAX (never reaches the backend as an abusive
        // traversal); a zero/under-range depth clamps UP to MIN.
        let too_deep = c.local_request_with_depth("ws-7", "T", 99);
        assert_eq!(
            too_deep.query[1],
            ("max_depth".to_owned(), MAX_BACKLINK_DEPTH.to_string())
        );
        let too_shallow = c.local_request_with_depth("ws-7", "T", 0);
        assert_eq!(
            too_shallow.query[1],
            ("max_depth".to_owned(), MIN_BACKLINK_DEPTH.to_string())
        );

        // The non-depth `local_request` still equals the default-depth path (one builder, no drift).
        assert_eq!(
            c.local_request("ws-7", "X").query,
            c.local_request_with_depth("ws-7", "X", DEFAULT_BACKLINK_DEPTH)
                .query
        );
    }

    #[test]
    fn loom_graph_projection_requires_truncation_and_accepts_omitted_empty_suppression() {
        let projection = parse_graph_projection(serde_json::json!({
            "nodes": [], "edges": [], "truncated": false
        }))
        .expect("the backend omits an empty suppressed_hub_ids list");
        assert!(projection.suppressed_hub_ids.is_empty());

        for (value, expected) in [
            (
                serde_json::json!({
                    "nodes": [], "edges": [], "suppressed_hub_ids": []
                }),
                "LoomGraph.truncated must be a bool",
            ),
            (
                serde_json::json!({
                    "nodes": [], "edges": [], "truncated": "false", "suppressed_hub_ids": []
                }),
                "LoomGraph.truncated must be a bool",
            ),
            (
                serde_json::json!({
                    "nodes": [], "edges": [], "truncated": false,
                    "suppressed_hub_ids": "hub-1"
                }),
                "LoomGraph.suppressed_hub_ids must be an array",
            ),
        ] {
            let error = parse_graph_projection(value).expect_err("malformed metadata must fail");
            assert!(
                error.to_string().contains(expected),
                "expected '{expected}', got '{error}'"
            );
        }
    }

    #[test]
    fn loom_graph_projection_rejects_non_string_suppressed_hub_ids() {
        for invalid in [
            serde_json::Value::Null,
            serde_json::json!(7),
            serde_json::json!(""),
            serde_json::json!("   "),
        ] {
            let error = parse_graph_projection(serde_json::json!({
                "nodes": [],
                "edges": [],
                "truncated": false,
                "suppressed_hub_ids": [invalid]
            }))
            .expect_err("every suppressed hub id must be a non-empty string");
            assert!(error
                .to_string()
                .contains("suppressed_hub_ids[0] must be a non-empty string"));
        }
    }

    #[test]
    fn loom_graph_projection_rejects_whitespace_node_and_edge_identities() {
        let nodes = serde_json::json!([
            {"block":{"block_id":"b1","title":"One","content_type":"note"}},
            {"block":{"block_id":"b2","title":"Two","content_type":"note"}}
        ]);
        let edge = |source: &str, target: &str, edge_type: &str| {
            serde_json::json!({
                "edge": {
                    "source_block_id": source,
                    "target_block_id": target,
                    "edge_type": edge_type
                }
            })
        };

        let malformed = [
            serde_json::json!({
                "nodes": [{"block":{"block_id":"   ","title":"Blank","content_type":"note"}}],
                "edges": [],
                "truncated": false
            }),
            serde_json::json!({
                "nodes": nodes,
                "edges": [edge("   ", "b2", "mention")],
                "truncated": false
            }),
            serde_json::json!({
                "nodes": nodes,
                "edges": [edge("b1", "\t", "mention")],
                "truncated": false
            }),
            serde_json::json!({
                "nodes": nodes,
                "edges": [edge("b1", "b2", "   ")],
                "truncated": false
            }),
        ];

        for projection in malformed {
            parse_graph_projection(projection)
                .expect_err("blank/whitespace graph identities must fail closed");
        }
    }

    /// WP-KERNEL-012 MT-080 (AC-080-2 / MT-061): the canvas resize + clear-group request builders PATCH the
    /// SAME verified placement URL the `group_request` uses; only the body differs (`{w,h}` for a resize,
    /// `{clear_group: true}` for a clear). The clear body is asserted against the REAL backend's accepted
    /// contract (`UpdatePlacementRequest.clear_group` in `src/backend/handshake_core/src/api/loom.rs`),
    /// NOT the serializer's own historical output: `{"group_id": null}` is a verified backend no-op (it
    /// deserializes to `group_id: None` and leaves the group unchanged), so only `{"clear_group": true}`
    /// actually clears the section assignment end-to-end.
    #[test]
    fn canvas_board_resize_move_and_clear_group_requests() {
        let rt = rt();
        let c = CanvasBoardClient::new(BASE, rt.handle().clone());

        let resize = c.resize_request("ws-7", "p-9", 320.0, 180.0);
        assert_eq!(resize.method, HttpMethod::Patch);
        assert_eq!(
            resize.url,
            "http://test.local:1234/workspaces/ws-7/loom/canvas-placements/p-9"
        );
        assert_eq!(
            resize.body,
            Some(serde_json::json!({ "w": 320.0, "h": 180.0 }))
        );

        let clear = c.clear_group_request("ws-7", "p-9");
        assert_eq!(clear.method, HttpMethod::Patch);
        assert_eq!(
            clear.url,
            "http://test.local:1234/workspaces/ws-7/loom/canvas-placements/p-9"
        );
        // The backend clears the group ONLY on `clear_group: true`; `{"group_id": null}` is a no-op.
        assert_eq!(clear.body, Some(serde_json::json!({ "clear_group": true })));

        // The assign (Some group) arm reuses the existing verified group_request (same URL + verb).
        let assign = c.group_request("ws-7", "p-9", "section-2");
        assert_eq!(
            assign.url, resize.url,
            "assign-section reuses the same placement PATCH URL"
        );
        assert_eq!(
            assign.body,
            Some(serde_json::json!({ "group_id": "section-2" }))
        );

        let moved_into = c.move_request("ws-7", "p-9", 85.0, 110.0, Some("section-2"));
        assert_eq!(moved_into.method, HttpMethod::Patch);
        assert_eq!(moved_into.url, resize.url);
        assert_eq!(
            moved_into.body,
            Some(serde_json::json!({
                "x": 85.0,
                "y": 110.0,
                "group_id": "section-2"
            }))
        );

        let moved_out = c.move_request("ws-7", "p-9", 12.0, -4.0, None);
        assert_eq!(moved_out.method, HttpMethod::Patch);
        assert_eq!(moved_out.url, resize.url);
        assert_eq!(
            moved_out.body,
            Some(serde_json::json!({
                "x": 12.0,
                "y": -4.0,
                "clear_group": true
            }))
        );
    }

    #[test]
    fn stage_capture_canvas_card_persists_complete_provenance() {
        let rt = rt();
        let client = CanvasBoardClient::new(BASE, rt.handle().clone());
        let spec = client.create_stage_capture_card_request(
            "ws-7",
            "canvas-2",
            "artifact-9",
            "abc123",
            "artifact://sha256/abc123",
            "stage-route-node-canvas-2-node-9",
            40.0,
            50.0,
            320.0,
            180.0,
        );

        assert_eq!(spec.method, HttpMethod::Post);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws-7/loom/canvas-boards/canvas-2/cards"
        );
        let body = spec.body.as_ref().expect("stage card request has a body");
        let encoded_reference = body["body"]
            .as_str()
            .expect("stage card body carries the encoded provenance reference");
        let decoded_reference: CanvasStageCaptureReference =
            serde_json::from_str(encoded_reference).expect("stage provenance decodes");
        let expected_reference = CanvasStageCaptureReference {
            schema_id: CANVAS_STAGE_CAPTURE_REF_SCHEMA.to_owned(),
            artifact_id: "artifact-9".to_owned(),
            sha256: "abc123".to_owned(),
            manifest_ref: "artifact://sha256/abc123".to_owned(),
            causal_action_id: "stage-route-node-canvas-2-node-9".to_owned(),
        };
        assert_eq!(decoded_reference, expected_reference);
        let expected_wire = serde_json::json!({
            "schema_id": CANVAS_STAGE_CAPTURE_REF_SCHEMA,
            "artifact_id": "artifact-9",
            "sha256": "abc123",
            "manifest_ref": "artifact://sha256/abc123",
            "causal_action_id": "stage-route-node-canvas-2-node-9",
        });
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(encoded_reference)
                .expect("encoded provenance is JSON"),
            expected_wire,
            "the nested persisted reference pins the external wire keys"
        );
        assert_eq!(
            body["stage_provenance"], expected_wire,
            "the structured mirror pins the same external wire keys"
        );
        assert_eq!(
            (
                &body["title"],
                &body["x"],
                &body["y"],
                &body["w"],
                &body["h"]
            ),
            (
                &serde_json::json!("Stage capture artifact-9"),
                &serde_json::json!(40.0),
                &serde_json::json!(50.0),
                &serde_json::json!(320.0),
                &serde_json::json!(180.0),
            )
        );

        let compensate = client.compensate_stage_capture_card_request(
            "ws-7",
            "canvas-2",
            "placement-3",
            "block-4",
            "artifact-9",
            "abc123",
            "artifact://sha256/abc123",
            "stage-route-node-canvas-2-node-9",
        );
        assert_eq!(compensate.method, HttpMethod::Post);
        assert_eq!(
            compensate.url,
            "http://test.local:1234/workspaces/ws-7/loom/canvas-boards/canvas-2/stage-cards/placement-3/compensate"
        );
        assert_eq!(
            compensate.body,
            Some(serde_json::json!({
                "placed_block_id": "block-4",
                "stage_provenance": {
                    "schema_id": CANVAS_STAGE_CAPTURE_REF_SCHEMA,
                    "artifact_id": "artifact-9",
                    "sha256": "abc123",
                    "manifest_ref": "artifact://sha256/abc123",
                    "causal_action_id": "stage-route-node-canvas-2-node-9",
                }
            }))
        );
    }

    /// WP-KERNEL-012 MT-080: `placement_from_json` reads the backend's durable `is_text_card` flag.
    /// `true` yields a [`CanvasCardKind::TextCard`] (inline-editable across sessions), `false`/absent a
    /// [`CanvasCardKind::BlockRef`] — so a reloaded text card no longer depends on same-session host-origin
    /// tracking to stay inline-editable.
    #[test]
    fn placement_from_json_reads_is_text_card_flag() {
        let text = placement_from_json(&serde_json::json!({
            "placement_id": "p-text",
            "placed_block_id": "blk-text",
            "x": 10.0, "y": 20.0, "w": 200.0, "h": 120.0,
            "is_text_card": true
        }))
        .expect("text placement parses");
        assert_eq!(
            text.card_kind,
            CanvasCardKind::TextCard,
            "is_text_card:true marks a TextCard"
        );
        assert!(
            text.card_kind.is_text_card(),
            "TextCard is inline-editable across sessions"
        );
        // The editor buffer is seeded (empty) so a double-click opens an editable card — never block content.
        assert_eq!(text.live_body.as_deref(), Some(""));

        let block_ref = placement_from_json(&serde_json::json!({
            "placement_id": "p-ref",
            "placed_block_id": "blk-ref",
            "x": 0.0, "y": 0.0, "w": 200.0, "h": 120.0,
            "is_text_card": false
        }))
        .expect("block-ref placement parses");
        assert_eq!(
            block_ref.card_kind,
            CanvasCardKind::BlockRef,
            "is_text_card:false stays a BlockRef"
        );

        // Absent field is the serde-default false -> BlockRef (never a fabricated TextCard).
        let defaulted = placement_from_json(&serde_json::json!({
            "placement_id": "p-none",
            "placed_block_id": "blk-none",
            "x": 0.0, "y": 0.0, "w": 200.0, "h": 120.0
        }))
        .expect("defaulted placement parses");
        assert_eq!(defaulted.card_kind, CanvasCardKind::BlockRef);
        assert!(!defaulted.card_kind.is_text_card());
    }

    /// End-to-end: the REAL `fetch_global` spawn path hits a live capture server and parses the verified
    /// canonical `LoomGraph` payload into real nodes + edges (proves the client is genuinely CONSUMED —
    /// dispatch -> reqwest -> wire -> parse — not just arithmetic).
    #[test]
    fn loom_graph_global_fetch_parses_blocks() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build multi-thread runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");

        let client = LoomGraphClient::new(base, rt.handle().clone());
        let cell: LoomGraphCell = Arc::new(Mutex::new(VecDeque::new()));
        client.fetch_global("ws1", 41, cell.clone());

        let body = r#"{"nodes":[
            {"block":{"block_id":"b1","title":"Alpha","content_type":"note"},"depth":0,"degree":1,"stale":false},
            {"block":{"block_id":"b2","title":"Beta","content_type":"file"},"depth":0,"degree":2,"stale":false},
            {"block":{"block_id":"b3","title":null,"content_type":"tag_hub"},"depth":0,"degree":1,"stale":false}
        ],"edges":[
            {"edge":{"edge_id":"e1","source_block_id":"b1","target_block_id":"b2","edge_type":"mention"},"stale":false},
            {"edge":{"edge_id":"e2","source_block_id":"b2","target_block_id":"b3","edge_type":"tag"},"stale":false}
        ],"truncated":true,"suppressed_hub_ids":["hub-capped"]}"#;
        let request_line = capture_one_request_reply(listener, body);
        assert!(
            request_line.contains("GET /workspaces/ws1/loom/graph/global?"),
            "global fetch hits graph/global (got '{request_line}')"
        );

        rt.block_on(async {
            for _ in 0..50 {
                if !cell.lock().unwrap().is_empty() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        let delivery = cell.lock().unwrap().pop_front().expect("graph delivered");
        assert_eq!(
            delivery.request,
            LoomGraphRequestIdentity::global(41, "ws1"),
            "the completion preserves its host generation and workspace identity"
        );
        let data = delivery.result.expect("parse ok");
        assert_eq!(data.nodes.len(), 3, "3 seeded blocks -> 3 nodes");
        assert_eq!(data.nodes[0].block_id, "b1");
        assert_eq!(data.nodes[0].content_type, "note");
        // A null title falls back to the block id (never label-less).
        assert_eq!(
            data.nodes[2].title, "b3",
            "null title falls back to block_id"
        );
        assert_eq!(
            data.edges.len(),
            2,
            "canonical projection carries real edges"
        );
        assert_eq!(data.edges[0].source, "b1");
        assert_eq!(data.edges[0].target, "b2");
        assert_eq!(data.edges[0].edge_type, "mention");
        assert!(
            data.truncated,
            "a valid capped projection remains renderable and carries honest truncation metadata"
        );
        assert_eq!(data.suppressed_hub_ids, vec!["hub-capped"]);
    }

    /// AC8 binding: a backend 5xx (non-success status) delivers Err, NOT a panic and NOT a fake graph.
    #[test]
    fn loom_graph_global_fetch_error_on_5xx() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build multi-thread runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");

        let client = LoomGraphClient::new(base, rt.handle().clone());
        let cell: LoomGraphCell = Arc::new(Mutex::new(VecDeque::new()));
        client.fetch_global("ws1", 42, cell.clone());

        // Reply 503 (backend unreachable analog).
        std::thread::spawn(move || {
            use std::io::{Read, Write};
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let resp = "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n";
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
        });

        rt.block_on(async {
            for _ in 0..50 {
                if !cell.lock().unwrap().is_empty() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        let delivery = cell.lock().unwrap().pop_front().expect("graph delivered");
        assert_eq!(
            delivery.request,
            LoomGraphRequestIdentity::global(42, "ws1")
        );
        let delivered = delivery.result;
        assert!(
            delivered.is_err(),
            "AC8: a 5xx must deliver Err (got {delivered:?}), never a fake graph"
        );
    }

    // ── LoomTagClient: tag-list and add-tag parsers (verified backend response shapes) ────────────────

    fn valid_tag_test_block(id: &str, title: &str, content_type: &str) -> serde_json::Value {
        serde_json::json!({
            "block_id": id,
            "workspace_id": "ws1",
            "document_id": null,
            "asset_id": null,
            "original_filename": null,
            "content_hash": null,
            "journal_date": null,
            "imported_at": null,
            "title": title,
            "content_type": content_type,
            "pinned": false,
            "favorite": false,
            "pin_order": null,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "derived": {}
        })
    }

    #[test]
    fn tag_entries_do_not_label_backlink_count_as_member_count() {
        let mut row = valid_tag_test_block("tag-rust", "Rust", "tag_hub");
        row["derived"]["backlink_count"] = serde_json::json!(4);
        let entry = block_to_tag_entry(&row).expect("tag row parses");

        assert_eq!(
            entry.member_count, None,
            "AC1: backlink_count includes non-member backlinks and must not be labeled as member_count"
        );
    }

    #[test]
    fn tag_entries_parse_member_count_from_explicit_member_evidence() {
        let mut explicit_json = valid_tag_test_block("tag-rust", "Rust", "tag_hub");
        explicit_json["member_count"] = serde_json::json!(4);
        let explicit = block_to_tag_entry(&explicit_json).expect("tag row parses");
        assert_eq!(explicit.member_count, Some(4));

        let mut tagged_json = valid_tag_test_block("tag-design", "Design", "tag_hub");
        tagged_json["tagged_blocks"] = serde_json::json!([{}, {}]);
        let tagged_blocks = block_to_tag_entry(&tagged_json).expect("tag row parses");
        assert_eq!(
            tagged_blocks.member_count,
            Some(2),
            "tagged_blocks.len() is exact member evidence"
        );
    }

    #[test]
    fn tag_transport_parsers_fail_closed_on_malformed_success_payloads() {
        assert!(parse_tag_entries_page(&serde_json::json!({})).is_err());

        let mut valid = valid_tag_test_block("tag-valid", "Valid", "tag_hub");
        let malformed = serde_json::json!([valid.clone(), { "block_id": "partial" }]);
        assert!(
            parse_tag_entries_page(&malformed).is_err(),
            "one malformed successful row rejects the complete list"
        );

        valid["member_count"] = serde_json::json!("four");
        assert!(
            block_to_tag_entry(&valid).is_err(),
            "wrong-typed explicit counts fail closed"
        );

        let hub_block = valid_tag_test_block("tag-valid", "Valid", "tag_hub");
        let member = valid_tag_test_block("member-1", "Member", "note");
        let valid_detail = serde_json::json!({
            "block": hub_block,
            "sub_tags": [],
            "tagged_blocks": [member],
            "backlink_count": 0
        });
        assert_eq!(
            parse_tag_hub_detail(&valid_detail, "tag-valid")
                .expect("canonical detail parses")
                .1
                .len(),
            1
        );
        let mut malformed_detail = valid_detail;
        malformed_detail["tagged_blocks"] = serde_json::json!([{"block_id":"partial"}]);
        assert!(
            parse_tag_hub_detail(&malformed_detail, "tag-valid").is_err(),
            "malformed detail members are not silently dropped"
        );
        assert!(
            parse_tag_members(&serde_json::json!({"blocks": []})).is_err(),
            "wrong top-level member shape is not reinterpreted as empty"
        );
    }

    #[test]
    fn tag_list_fetches_complete_pagination_beyond_first_hundred() {
        use std::io::{Read, Write};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind tag paging server");
        let base = format!("http://{}", listener.local_addr().expect("server address"));
        let server = std::thread::spawn(move || {
            let mut request_lines = Vec::new();
            for (page_index, count) in [(0usize, 500usize), (1usize, 1usize)] {
                let (mut stream, _) = listener.accept().expect("accept tag page request");
                let mut bytes = [0u8; 4096];
                let read = stream.read(&mut bytes).expect("read tag page request");
                let request = String::from_utf8_lossy(&bytes[..read]);
                request_lines.push(request.lines().next().unwrap_or_default().to_owned());
                let rows: Vec<_> = (0..count)
                    .map(|index| {
                        let id = format!("tag-{:04}", page_index * 500 + index);
                        valid_tag_test_block(&id, &id, "tag_hub")
                    })
                    .collect();
                let body = serde_json::to_string(&rows).expect("serialize tag page");
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write tag page response");
            }
            request_lines
        });

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tag paging runtime");
        let client = reqwest::Client::new();
        let tags = rt
            .block_on(fetch_all_tag_entries(
                &client,
                &format!("{base}/workspaces/ws1/loom/tags"),
            ))
            .expect("all tag pages parse");
        assert_eq!(
            tags.len(),
            501,
            "every tag beyond the first 100 is retained"
        );
        assert_eq!(tags[500].block_id, "tag-0500");
        let request_lines = server.join().expect("tag paging server joins");
        assert!(request_lines[0].contains("limit=500&offset=0"));
        assert!(request_lines[1].contains("limit=500&offset=500"));
    }

    /// AC6 / Spec-Realism Gate: the add-tag candidate parser MUST read the VERIFIED `/loom/search` shape
    /// `Vec<LoomBlockSearchResult>` = `[{ "block": { "block_id", "title" }, "score" }]` — `block_id`/`title`
    /// are nested UNDER `block`, not at the entry top level. This feeds the exact wrapper shape and asserts
    /// a candidate is produced; a top-level-only read (the prior bug) would yield ZERO candidates and an
    /// always-empty add-tag popup against the real backend. This exercises the parse directly (NOT the
    /// widget's own `AddTagCandidate::new`, which the existing PROOF5 test injects).
    #[test]
    fn add_tag_candidates_parse_verified_search_result_shape() {
        // The verified LoomBlockSearchResult wrapper: { block: {...}, score }.
        let response = serde_json::json!([
            { "block": { "block_id": "blk-1", "title": "Rust notes" }, "score": 0.9 },
            { "block": { "block_id": "blk-2", "title": "" }, "score": 0.4 },
        ]);
        let candidates = parse_add_tag_candidates(&response).expect("verified shape parses");
        assert_eq!(
            candidates.len(),
            2,
            "verified [{{block,score}}] shape must yield one candidate per entry (got {candidates:?})"
        );
        assert_eq!(candidates[0].block_id, "blk-1");
        assert_eq!(candidates[0].title, "Rust notes");
        // The backend-provided title is preserved exactly.
        assert_eq!(candidates[1].block_id, "blk-2");
        assert_eq!(candidates[1].title, "");
    }

    #[test]
    fn graph_search_page_parser_rejects_partial_or_wrapped_payloads() {
        let valid = serde_json::json!([{
            "source_kind": "document", "result_kind": "knowledge_entity", "ref_id": "KRD-1",
            "title": "A", "excerpt": "B", "score": 1.0, "metadata": {}
        }]);
        assert_eq!(parse_graph_search_page(&valid).unwrap().len(), 1);
        assert!(parse_graph_search_page(&serde_json::json!({"results": valid})).is_err());
        assert!(
            parse_graph_search_page(&serde_json::json!([
                {"source_kind":"document","result_kind":"knowledge_entity","ref_id":"KRD-1","title":"A","excerpt":"B","score":1.0},
                {"source_kind":"document","result_kind":"knowledge_entity","title":"A","excerpt":"B","score":1.0}
            ]))
            .is_err(),
            "one malformed row rejects the whole page"
        );
        assert!(parse_graph_search_page(&serde_json::json!([{
            "source_kind":"unknown", "result_kind":"knowledge_entity", "ref_id":"KRD-1",
            "title":"A", "excerpt":"B", "score":1.0
        }]))
        .is_err());
        assert!(
            parse_graph_search_page(&serde_json::json!([{
                "result_kind":"knowledge_entity", "ref_id":"KRD-1",
                "title":"A", "excerpt":"B", "score":1.0
            }]))
            .is_err(),
            "producer-required source_kind must not be defaulted"
        );
        assert!(
            parse_graph_search_page(&serde_json::json!([{
                "source_kind":"document", "result_kind":"knowledge_entity", "ref_id":"KRD-1",
                "excerpt":"B", "score":1.0
            }]))
            .is_err(),
            "producer-required title must not be defaulted by the consumer"
        );
        assert!(
            parse_graph_search_page(&serde_json::json!([{
                "source_kind":"document", "result_kind":"knowledge_entity", "ref_id":"KRD-1",
                "title":"A", "score":1.0
            }]))
            .is_err(),
            "producer-required excerpt must not be defaulted by the consumer"
        );
        assert!(
            parse_graph_search_page(&serde_json::json!([{
                "source_kind":"document", "result_kind":"knowledge_entity", "ref_id":"KRD-1",
                "title":"A", "excerpt":42, "score":1.0
            }]))
            .is_err(),
            "non-string excerpt fails closed"
        );
        assert!(
            parse_graph_search_page(&serde_json::json!([{
                "source_kind":"document", "result_kind":"knowledge_entity", "ref_id":"KRD-1",
                "title":"A", "excerpt":"B"
            }]))
            .is_err(),
            "producer-required score must not be defaulted by the consumer"
        );
        assert!(
            parse_graph_search_page(&serde_json::json!([{
                "source_kind":"document", "result_kind":"knowledge_entity", "ref_id":"KRD-1",
                "title":"A", "excerpt":"B", "score":"NaN"
            }]))
            .is_err(),
            "non-numeric/non-finite score spellings fail closed"
        );
        for invalid_block in [serde_json::json!(7), serde_json::json!([{}])] {
            assert!(
                parse_graph_search_page(&serde_json::json!([{
                    "source_kind":"loom_block", "result_kind":"loom_block", "ref_id":"blk-1",
                    "title":"A", "excerpt":"B", "score":1.0, "block": invalid_block
                }]))
                .is_err(),
                "primitive/array optional block must fail closed"
            );
        }
        let canonical_block = serde_json::json!({
            "block_id":"blk-1", "workspace_id":"WS-1", "content_type":"note",
            "title":"A", "pinned":false, "favorite":false, "pin_order":null,
            "created_at":"2026-07-15T00:00:00Z", "updated_at":"2026-07-15T00:00:00Z",
            "derived":{}
        });
        assert!(parse_graph_search_page(&serde_json::json!([{
            "source_kind":"loom_block", "result_kind":"loom_block", "ref_id":"blk-1",
            "title":"A", "excerpt":"B", "score":1.0, "block": canonical_block
        }]))
        .is_ok());
        assert!(parse_graph_search_page(&serde_json::json!([{
            "source_kind":"document", "result_kind":"knowledge_entity", "ref_id":"KRD-1",
            "title":"A", "excerpt":"B", "score":1.0, "block":null
        }]))
        .is_ok());
    }

    #[test]
    fn bookmark_response_parser_rejects_wrong_or_partial_producer_shapes() {
        let valid = serde_json::json!({
            "workspace_id": "WS-1",
            "bookmark_state": {"schema_id":"hsk.workspace_search_bookmark_state@1","bookmarks":[]},
            "updated_at": "2026-07-15T00:00:00Z",
            "event_ledger_event_id": "evt-1"
        });
        let (parsed_state, receipt) =
            parse_bookmark_response(&valid, "WS-1", false).expect("valid producer response");
        assert_eq!(parsed_state, valid["bookmark_state"]);
        assert_eq!(receipt.as_deref(), Some("evt-1"));
        assert!(
            parse_bookmark_response(&serde_json::json!({"bookmark_state": {}}), "WS-1", true)
                .is_err()
        );
        assert!(parse_bookmark_response(
            &serde_json::json!({"workspace_id":"WS-2","bookmark_state":{}}),
            "WS-1",
            true
        )
        .is_err());
        assert!(
            parse_bookmark_response(&serde_json::json!({"workspace_id":"WS-1"}), "WS-1", true)
                .is_err()
        );
        let (empty_state, empty_receipt) = parse_bookmark_response(
            &serde_json::json!({
                "workspace_id":"WS-1",
                "bookmark_state":null,
                "updated_at":null,
                "event_ledger_event_id":null
            }),
            "WS-1",
            true,
        )
        .expect("absent GET state");
        assert_eq!(empty_state, serde_json::json!({}));
        assert_eq!(empty_receipt, None);
        assert!(parse_bookmark_response(
            &serde_json::json!({"workspace_id":"WS-1","bookmark_state":null}),
            "WS-1",
            false
        )
        .is_err());
    }

    #[test]
    fn add_tag_candidates_reject_unverified_wrappers_and_malformed_rows() {
        let bare = serde_json::json!([{ "block_id": "b9", "title": "Bare", "score": 1.0 }]);
        assert!(parse_add_tag_candidates(&bare).is_err());
        let wrapped = serde_json::json!({
            "results": [{ "block": { "block_id": "b10", "title": "Wrapped" }, "score": 1.0 }]
        });
        assert!(parse_add_tag_candidates(&wrapped).is_err());
        assert!(parse_add_tag_candidates(&serde_json::json!([
            { "block": { "block_id": "", "title": "Bad" }, "score": 1.0 }
        ]))
        .is_err());
        assert!(parse_add_tag_candidates(&serde_json::json!([
            { "block": { "block_id": "b1", "title": "Bad" }, "score": "high" }
        ]))
        .is_err());
        assert_eq!(
            parse_add_tag_candidates(&serde_json::json!([])).unwrap(),
            Vec::new()
        );
    }

    // ── Apply pipeline partial-failure fold (RISK-1/MC-1) ─────────────────────────────────────────────

    fn audit_plans(ids: &[&str]) -> Vec<crate::find_in_files::ReplacementPlan> {
        ids.iter()
            .enumerate()
            .map(|(index, id)| crate::find_in_files::ReplacementPlan {
                workspace_id: "WS-1".to_owned(),
                document_id: (*id).to_owned(),
                title: (*id).to_owned(),
                expected_version: 1,
                content_json_after: serde_json::json!({"index": index, "state": "after"}),
                before_sha256: format!("{:064x}", index + 1),
                after_sha256: format!("{:064x}", index + 101),
                crdt_document_id: None,
                match_count: 1,
                before_preview: String::new(),
                after_preview: String::new(),
                match_previews: Vec::new(),
            })
            .collect()
    }

    /// MC-1: a SECOND document save returning 409 (Conflict) must PRESERVE the first document's receipt
    /// and STOP — `AppliedPartial{receipts:[r1], ..}`, never a silent loss of the first receipt. This is
    /// the red_team-named control that previously had no standalone (un-ignored) coverage.
    #[test]
    fn apply_fold_preserves_first_receipt_on_second_doc_conflict() {
        let outcomes = vec![
            (
                "KRD-1".to_owned(),
                DocSaveOutcome::Saved("evt-1".to_owned()),
            ),
            ("KRD-2".to_owned(), DocSaveOutcome::Conflict),
        ];
        let plans = audit_plans(&["KRD-1", "KRD-2"]);
        match fold_apply_outcomes(&outcomes, &plans) {
            crate::find_in_files::ReplaceDelivery::AppliedPartial {
                receipts,
                audit_receipts,
                error,
            } => {
                assert_eq!(
                    receipts,
                    vec!["evt-1".to_owned()],
                    "first receipt must survive the conflict"
                );
                assert!(
                    error.contains("KRD-2"),
                    "error names the conflicting doc: {error}"
                );
                assert!(
                    error.contains("conflict"),
                    "error states the version conflict: {error}"
                );
                assert_eq!(audit_receipts.len(), 2);
                assert_eq!(audit_receipts[0].before_sha256, plans[0].before_sha256);
                assert_eq!(audit_receipts[1].after_sha256, plans[1].after_sha256);
                assert_eq!(
                    audit_receipts[1].outcome,
                    crate::find_in_files::ReplaceAuditOutcome::Conflict
                );
            }
            other => panic!("expected AppliedPartial preserving the first receipt, got {other:?}"),
        }
    }

    /// MC-1 (Failed variant): a non-409 failure on the second doc also preserves the first receipt.
    #[test]
    fn apply_fold_preserves_first_receipt_on_second_doc_failure() {
        let outcomes = vec![
            (
                "KRD-1".to_owned(),
                DocSaveOutcome::Saved("evt-1".to_owned()),
            ),
            (
                "KRD-2".to_owned(),
                DocSaveOutcome::Failed("status 500".to_owned()),
            ),
        ];
        let plans = audit_plans(&["KRD-1", "KRD-2"]);
        match fold_apply_outcomes(&outcomes, &plans) {
            crate::find_in_files::ReplaceDelivery::AppliedPartial {
                receipts, error, ..
            } => {
                assert_eq!(receipts, vec!["evt-1".to_owned()]);
                assert!(
                    error.contains("status 500"),
                    "error carries the failure detail: {error}"
                );
            }
            other => panic!("expected AppliedPartial, got {other:?}"),
        }
    }

    /// An all-success run folds to `Applied{receipts, plan_count}` with every receipt.
    #[test]
    fn apply_fold_all_success_yields_applied_with_all_receipts() {
        let outcomes = vec![
            (
                "KRD-1".to_owned(),
                DocSaveOutcome::Saved("evt-1".to_owned()),
            ),
            (
                "KRD-2".to_owned(),
                DocSaveOutcome::Saved("evt-2".to_owned()),
            ),
        ];
        let plans = audit_plans(&["KRD-1", "KRD-2"]);
        match fold_apply_outcomes(&outcomes, &plans) {
            crate::find_in_files::ReplaceDelivery::Applied {
                receipts,
                plan_count,
                audit_receipts,
            } => {
                assert_eq!(receipts, vec!["evt-1".to_owned(), "evt-2".to_owned()]);
                assert_eq!(plan_count, 2);
                assert_eq!(audit_receipts.len(), 2);
            }
            other => panic!("expected Applied, got {other:?}"),
        }
    }

    #[test]
    fn apply_fold_keeps_committed_without_receipt_explicit_and_never_invents_blank_id() {
        let outcomes = vec![
            (
                "KRD-1".to_owned(),
                DocSaveOutcome::CommittedWithoutReceipt {
                    receipt_error: Some("event ledger unavailable".to_owned()),
                },
            ),
            (
                "KRD-2".to_owned(),
                DocSaveOutcome::Saved("evt-2".to_owned()),
            ),
        ];
        let plans = audit_plans(&["KRD-1", "KRD-2"]);
        match fold_apply_outcomes(&outcomes, &plans) {
            crate::find_in_files::ReplaceDelivery::Applied {
                receipts,
                audit_receipts,
                plan_count,
            } => {
                assert_eq!(receipts, vec!["evt-2".to_owned()]);
                assert!(receipts.iter().all(|receipt| !receipt.trim().is_empty()));
                assert_eq!(plan_count, 2);
                assert_eq!(
                    audit_receipts[0].outcome,
                    crate::find_in_files::ReplaceAuditOutcome::CommittedWithoutReceipt
                );
                assert_eq!(audit_receipts[0].save_receipt_event_id, None);
                assert_eq!(
                    audit_receipts[0].error.as_deref(),
                    Some("event ledger unavailable")
                );
            }
            other => panic!("expected committed-without-receipt Applied outcome, got {other:?}"),
        }
    }

    #[test]
    fn cancelled_apply_fold_preserves_every_committed_outcome_and_counts_unsent_plans() {
        let outcomes = vec![
            (
                "KRD-1".to_owned(),
                DocSaveOutcome::Saved("evt-1".to_owned()),
            ),
            (
                "KRD-2".to_owned(),
                DocSaveOutcome::CommittedWithoutReceipt {
                    receipt_error: Some("event ledger unavailable".to_owned()),
                },
            ),
        ];
        let plans = audit_plans(&["KRD-1", "KRD-2", "KRD-3"]);
        match fold_cancelled_apply_outcomes(&outcomes, &plans) {
            crate::find_in_files::ReplaceDelivery::Cancelled {
                receipts,
                audit_receipts,
                skipped_plan_count,
            } => {
                assert_eq!(receipts, vec!["evt-1".to_owned()]);
                assert!(receipts.iter().all(|receipt| !receipt.trim().is_empty()));
                assert_eq!(audit_receipts.len(), 2);
                assert_eq!(
                    audit_receipts[1].outcome,
                    crate::find_in_files::ReplaceAuditOutcome::CommittedWithoutReceipt
                );
                assert_eq!(
                    audit_receipts[1].error.as_deref(),
                    Some("event ledger unavailable")
                );
                assert_eq!(skipped_plan_count, 1);
            }
            other => panic!("expected truthful Cancelled delivery, got {other:?}"),
        }
    }

    #[test]
    fn apply_worker_rejects_plan_workspace_mismatch_before_any_http() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let client = RichDocClient::new("http://127.0.0.1:9", runtime.handle().clone());
        let cell: FindReplaceCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
        client.apply_plans(
            "WS-2",
            audit_plans(&["KRD-1"]),
            FindInFilesStamp {
                workspace_id: "WS-2".to_owned(),
                operation: FindInFilesOperation::Apply,
                epoch: 1,
                sequence: 1,
            },
            Arc::clone(&cell),
            Arc::new(AtomicBool::new(false)),
        );
        let delivery = runtime.block_on(async {
            for _ in 0..100 {
                if let Some(delivery) = cell.lock().unwrap().pop_front() {
                    return delivery.outcome;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            panic!("workspace mismatch Apply did not complete")
        });
        match delivery {
            crate::find_in_files::ReplaceDelivery::AppliedPartial {
                receipts,
                audit_receipts,
                error,
            } => {
                assert!(receipts.is_empty());
                assert_eq!(audit_receipts.len(), 1);
                assert_eq!(
                    audit_receipts[0].outcome,
                    crate::find_in_files::ReplaceAuditOutcome::Failed
                );
                assert!(error.contains("workspace mismatch"));
            }
            other => panic!("workspace mismatch must fail before HTTP, got {other:?}"),
        }
    }

    #[test]
    fn rich_doc_body_required_metadata_is_not_defaulted_blank() {
        let doc = serde_json::json!({
            "rich_document_id": "KRD-1",
            "workspace_id": "WS-1",
            "title": "T",
            "doc_version": 1,
            "content_json": {"type": "doc", "content": []},
            "created_at": "2026-06-29T09:00:00Z",
            "updated_at": "2026-06-29T10:00:00Z"
        });

        let error = required_doc_string(&doc, "authority_label")
            .expect_err("missing required metadata must fail instead of rendering a blank badge");
        assert!(
            error.to_string().contains("authority_label"),
            "parse error names the missing field: {error}"
        );
    }

    #[test]
    fn created_semantic_edge_receipt_is_exact_and_fails_closed() {
        let spec = RequestSpec {
            method: HttpMethod::Post,
            url: "http://test.local/workspaces/ws-a/loom/edges".to_owned(),
            body: Some(serde_json::json!({
                "source_block_id": "source-a",
                "target_block_id": "target-a",
                "edge_type": "mention",
                "created_by": "user",
            })),
        };
        let receipt = serde_json::json!({
            "edge_id": "edge-minted-a",
            "workspace_id": "ws-a",
            "source_block_id": "source-a",
            "target_block_id": "target-a",
            "edge_type": "mention",
        });
        assert_eq!(
            created_semantic_edge_from_response(&spec, &receipt).unwrap(),
            CreatedSemanticEdge {
                edge_id: "edge-minted-a".to_owned(),
                workspace_id: "ws-a".to_owned(),
                source_block_id: "source-a".to_owned(),
                target_block_id: "target-a".to_owned(),
                edge_type: "mention".to_owned(),
            }
        );

        for field in [
            "edge_id",
            "workspace_id",
            "source_block_id",
            "target_block_id",
            "edge_type",
        ] {
            let mut malformed = receipt.clone();
            malformed.as_object_mut().unwrap().remove(field);
            assert!(
                created_semantic_edge_from_response(&spec, &malformed).is_err(),
                "missing receipt field {field} must fail closed"
            );
            malformed[field] = serde_json::json!("   ");
            assert!(
                created_semantic_edge_from_response(&spec, &malformed).is_err(),
                "blank receipt field {field} must fail closed"
            );
            malformed[field] = serde_json::json!(42);
            assert!(
                created_semantic_edge_from_response(&spec, &malformed).is_err(),
                "non-string receipt field {field} must fail closed"
            );
        }
        for (field, wrong) in [
            ("workspace_id", "ws-b"),
            ("source_block_id", "source-b"),
            ("target_block_id", "target-b"),
            ("edge_type", "tag"),
        ] {
            let mut mismatched = receipt.clone();
            mismatched[field] = serde_json::json!(wrong);
            assert!(
                created_semantic_edge_from_response(&spec, &mismatched).is_err(),
                "receipt {field} mismatch must fail closed"
            );
        }

        let mut invalid_url = spec.clone();
        invalid_url.url = "http://test.local/loom/edges".to_owned();
        assert!(created_semantic_edge_from_response(&invalid_url, &receipt).is_err());
        let mut unrelated_url = spec.clone();
        unrelated_url.url = "http://test.local/workspaces/ws-a/unrelated".to_owned();
        assert!(created_semantic_edge_from_response(&unrelated_url, &receipt).is_err());
        let mut no_body = spec.clone();
        no_body.body = None;
        assert!(created_semantic_edge_from_response(&no_body, &receipt).is_err());
        let mut wrong_method = spec.clone();
        wrong_method.method = HttpMethod::Patch;
        assert!(created_semantic_edge_from_response(&wrong_method, &receipt).is_err());
        for field in ["source_block_id", "target_block_id", "edge_type"] {
            let mut missing_request_field = spec.clone();
            missing_request_field
                .body
                .as_mut()
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(
                created_semantic_edge_from_response(&missing_request_field, &receipt).is_err(),
                "missing request field {field} must fail closed"
            );
            for invalid in [serde_json::json!("   "), serde_json::json!(42)] {
                let mut invalid_request_field = spec.clone();
                invalid_request_field.body.as_mut().unwrap()[field] = invalid;
                assert!(
                    created_semantic_edge_from_response(&invalid_request_field, &receipt).is_err(),
                    "blank/non-string request field {field} must fail closed"
                );
            }
        }
    }

    #[test]
    fn created_semantic_edge_transport_never_mints_identity_on_error_or_malformed_json() {
        fn serve_once(status: &str, body: &str) -> (String, std::thread::JoinHandle<()>) {
            use std::io::{Read, Write};
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let status = status.to_owned();
            let body = body.to_owned();
            let join = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0_u8; 4096];
                let _ = stream.read(&mut request);
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            });
            (format!("http://{address}"), join)
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let client = shared_http_client();
        for (status, body) in [
            (
                "500 Internal Server Error",
                r#"{"error":"database unavailable"}"#,
            ),
            ("200 OK", "not-json"),
        ] {
            let (base_url, join) = serve_once(status, body);
            let spec = RequestSpec {
                method: HttpMethod::Post,
                url: format!("{base_url}/workspaces/ws-a/loom/edges"),
                body: Some(serde_json::json!({
                    "source_block_id": "source-a",
                    "target_block_id": "target-a",
                    "edge_type": "mention",
                    "created_by": "user",
                })),
            };
            assert!(
                runtime
                    .block_on(send_created_semantic_edge(&client, &spec))
                    .is_err(),
                "{status} must never yield a created edge identity"
            );
            join.join().unwrap();
        }
    }

    #[test]
    fn live_block_resolution_classifies_only_404_as_missing() {
        fn serve(
            status: &str,
            body: &str,
            delay: std::time::Duration,
        ) -> (String, std::thread::JoinHandle<()>) {
            use std::io::{Read, Write};
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let status = status.to_owned();
            let body = body.to_owned();
            let join = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0_u8; 2048];
                let _ = stream.read(&mut request);
                std::thread::sleep(delay);
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            });
            (format!("http://{address}/block"), join)
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let client = shared_http_client();

        let (url, join) = serve("404 Not Found", "{}", std::time::Duration::ZERO);
        assert_eq!(
            runtime.block_on(fetch_live_block(&client, &url)),
            Err(LiveBlockResolveError::Missing)
        );
        join.join().unwrap();

        let (url, join) = serve("500 Internal Server Error", "{}", std::time::Duration::ZERO);
        assert!(matches!(runtime.block_on(fetch_live_block(&client, &url)),
            Err(LiveBlockResolveError::Unavailable(message)) if message.contains("500")));
        join.join().unwrap();

        let (url, join) = serve("200 OK", "not-json", std::time::Duration::ZERO);
        assert!(matches!(runtime.block_on(fetch_live_block(&client, &url)),
            Err(LiveBlockResolveError::Unavailable(message)) if message.contains("decode")));
        join.join().unwrap();

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let refused = listener.local_addr().unwrap();
        drop(listener);
        assert!(matches!(
            runtime.block_on(fetch_live_block(
                &client,
                &format!("http://{refused}/block")
            )),
            Err(LiveBlockResolveError::Unavailable(_))
        ));

        let (url, join) = serve("200 OK", "{}", std::time::Duration::from_millis(5_100));
        assert!(matches!(
            runtime.block_on(fetch_live_block(&client, &url)),
            Err(LiveBlockResolveError::Unavailable(_))
        ));
        join.join().unwrap();
    }
}
