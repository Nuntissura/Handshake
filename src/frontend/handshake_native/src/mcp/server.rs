//! The out-of-process MCP transport (WP-KERNEL-011 MT-027).
//!
//! [`SwarmMcpServer`] binds a localhost TCP listener (`127.0.0.1:0`, OS-picked ephemeral port) and, on
//! Windows, a named pipe (`\\.\pipe\handshake_swarm_<pid>`). Each accepted connection reads
//! newline-delimited JSON-RPC 2.0 requests and writes newline-delimited JSON responses, dispatching
//! every request through the transport-agnostic [`crate::mcp::tools::dispatch_request`] — the SAME
//! function the in-process unit tests prove — so the steering semantics are identical across transports.
//!
//! ## Shared state the server reads/writes (thread-safe)
//!
//! The server tasks run on the app's tokio runtime, concurrently with the egui UI thread, so the state
//! shared with the app is behind `Arc<Mutex<_>>`:
//!
//! - `snapshot: Arc<Mutex<UiTreeSnapshot>>` — the latest UI-tree snapshot the egui frame loop publishes
//!   each frame. `list_widgets` clones it; `click_widget`/`set_value` resolve their target against it.
//! - `channel: Arc<Mutex<ActionChannel>>` — the bounded action queue. The server ENQUEUES resolved
//!   actions; the egui frame loop DRAINS them via `drain_into_events` and feeds them to egui.
//! - `token: SessionToken` — the per-session HMAC secret; checked on EVERY request before any tool runs.
//!
//! The screenshot capture is the focus-safe OS-window grab ([`crate::mcp::screenshot::capture_handshake_window`]).
//! Over-the-wire tests inject a closure instead (the OS grab is undriveable headless — see that module).
//!
//! ## Lifecycle
//!
//! [`SwarmMcpServer::bind`] binds the listeners, writes the discovery [`McpBinding`] file (owner-only),
//! and spawns the accept loops as detached tokio tasks (HBR-QUIET: background, never blocks the UI).
//! [`SwarmMcpServer::shutdown`] signals the accept loops to stop and removes the binding file so an
//! agent does not connect to a closed port. Dropping the server also fires the shutdown signal.
//!
//! ## Red-team controls implemented here
//!
//! - Auth gate FIRST (constant-time HMAC) — an unauthorized caller cannot enumerate or steer.
//! - Per-connection rate limit (token bucket, [`MAX_REQUESTS_PER_SEC`]) — an action flood is rejected
//!   with JSON-RPC `-32003` instead of saturating the egui frame loop.
//! - Bounded line length on reads — a malicious client cannot OOM the server with one huge line.
//! - Named-pipe bind failure is non-fatal — the server continues TCP-only and records that honestly.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use crate::accessibility::UiTreeSnapshot;
use crate::mcp::action::ActionChannel;
use crate::mcp::argus::{ArgusWindowDescriptor, WindowSnapshotRegistry};
use crate::mcp::attribution::ActionLog;
use crate::mcp::binding::{self, McpBinding};
use crate::mcp::leases::LeaseRegistry;
use crate::mcp::screenshot::{capture_handshake_window_target, ScreenshotError, ScreenshotResult};
use crate::mcp::session::{ArgusReceiptProvenance, McpSession, SwarmSafetyState};
use crate::mcp::tools::{
    McpRequest, McpResponse, SessionToken, ERR_INVALID_PARAMS, ERR_RATE_LIMITED,
};

/// Max JSON-RPC requests one connection may issue per second before the server replies `-32003`
/// (`Rate limited`). 100/sec is generous for multi-step steering yet bounds an adversarial flood.
pub const MAX_REQUESTS_PER_SEC: u32 = 100;

/// Max bytes in a single newline-delimited request line. A request larger than this is rejected (the
/// connection is closed) so a malicious client cannot exhaust memory with one unbounded line.
pub const MAX_LINE_BYTES: usize = 1 << 20; // 1 MiB

#[cfg(not(test))]
const BINDING_PUBLICATION_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(test)]
const BINDING_PUBLICATION_TIMEOUT: Duration = Duration::from_millis(100);
static BINDING_PUBLICATION_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static TEST_DELAY_AFTER_BINDING_VERIFY_MS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

struct BindingPublicationAdmission;

impl BindingPublicationAdmission {
    fn acquire() -> std::io::Result<Self> {
        BINDING_PUBLICATION_IN_FLIGHT
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "mcp binding publication is already in flight",
                )
            })?;
        Ok(Self)
    }
}

impl Drop for BindingPublicationAdmission {
    fn drop(&mut self) {
        BINDING_PUBLICATION_IN_FLIGHT.store(false, Ordering::Release);
    }
}

/// A handle to the running MCP transport. Holds the bound endpoint info (for tests/discovery) and the
/// shutdown signal. Dropping it (or calling [`Self::shutdown`]) stops the accept loops + removes the
/// binding file.
pub struct SwarmMcpServer {
    /// The resolved binding (tcp addr, pipe name, token) — also persisted to the discovery file.
    binding: McpBinding,
    /// Broadcast sender the accept loops select on; sending (or dropping) signals shutdown.
    shutdown_tx: broadcast::Sender<()>,
    /// Whether the binding file has already been removed (so shutdown is idempotent).
    binding_removed: bool,
    /// MT-028: the shared swarm-safety state (lease registry + attribution log) every connection uses.
    /// Exposed so the live shell / diagnostics can read the attributed action log and the concurrent
    /// harness test can assert leasing + attribution after driving N clients over the wire.
    safety: SwarmSafetyState,
}

/// The shared steering state the server's connection tasks read/write, cloned into each task. MT-028:
/// it now carries the [`SwarmSafetyState`] (lease registry + attribution log + token + shared snapshot/
/// channel) so every connection dispatches through a per-connection [`McpSession`] that applies leasing
/// and attribution consistently.
#[derive(Clone)]
struct ServerState {
    /// The shared swarm-safety state. Each connection derives its own [`McpSession`] from this; the
    /// lease registry + attribution log are SHARED across connections (so leasing/attribution are
    /// global), while `snapshot` + `channel` are the same `Arc<Mutex<_>>` the egui frame loop owns.
    safety: SwarmSafetyState,
    /// The screenshot capture used by the `screenshot` tool. Boxed so tests can inject an
    /// offscreen-render closure in place of the OS-window grab.
    capture: Arc<
        dyn Fn(&ArgusWindowDescriptor) -> Result<ScreenshotResult, ScreenshotError> + Send + Sync,
    >,
}

impl SwarmMcpServer {
    /// Bind the TCP listener (and, on Windows, the named pipe), write the owner-only discovery file, and
    /// spawn the accept loops on the CURRENT tokio runtime. Returns the server handle.
    ///
    /// `capture` is the screenshot source; production passes [`Self::os_window_capture`], tests pass an
    /// offscreen-render closure. Must be called from within a tokio runtime context (the live app calls
    /// it on its multi-thread runtime; the over-the-wire test uses `#[tokio::test]`).
    pub async fn bind(
        token: SessionToken,
        snapshot: Arc<Mutex<UiTreeSnapshot>>,
        channel: Arc<Mutex<ActionChannel>>,
        capture: Arc<dyn Fn() -> Result<ScreenshotResult, ScreenshotError> + Send + Sync>,
    ) -> std::io::Result<Self> {
        // MT-028: build the per-server swarm-safety state (fresh lease registry + attribution log) over
        // the same token + shared snapshot/channel MT-027 used, then bind through the shared-safety path.
        let safety = SwarmSafetyState::new(token, snapshot, channel);
        let compatible = Arc::new(move |_: &ArgusWindowDescriptor| capture());
        Self::bind_with_targeted_safety(safety, compatible).await
    }

    /// Bind the production window-aware transport over the registry published by the live app.
    pub async fn bind_with_windows(
        token: SessionToken,
        snapshot: Arc<Mutex<UiTreeSnapshot>>,
        windows: WindowSnapshotRegistry,
        channel: Arc<Mutex<ActionChannel>>,
        receipt_provenance: ArgusReceiptProvenance,
        action_wake: Arc<dyn Fn(&str) + Send + Sync>,
        capture: Arc<
            dyn Fn(&ArgusWindowDescriptor) -> Result<ScreenshotResult, ScreenshotError>
                + Send
                + Sync,
        >,
    ) -> std::io::Result<Self> {
        let safety = SwarmSafetyState::with_window_registry(token, snapshot, windows, channel)
            .with_durable_receipts(receipt_provenance)
            .with_action_wake(action_wake);
        Self::bind_with_targeted_safety(safety, capture).await
    }

    /// Bind a server over an EXISTING [`SwarmSafetyState`] (MT-028). Use this when multiple per-token
    /// servers must SHARE one lease registry + attribution log (e.g. the concurrent harness binds N
    /// servers — one per agent token — that all contend on one registry and append to one log). The
    /// single-token live shell uses [`Self::bind`], which builds a per-server safety state.
    pub async fn bind_with_safety(
        safety: SwarmSafetyState,
        capture: Arc<dyn Fn() -> Result<ScreenshotResult, ScreenshotError> + Send + Sync>,
    ) -> std::io::Result<Self> {
        let compatible = Arc::new(move |_: &ArgusWindowDescriptor| capture());
        Self::bind_with_targeted_safety(safety, compatible).await
    }

    async fn bind_with_targeted_safety(
        safety: SwarmSafetyState,
        capture: Arc<
            dyn Fn(&ArgusWindowDescriptor) -> Result<ScreenshotResult, ScreenshotError>
                + Send
                + Sync,
        >,
    ) -> std::io::Result<Self> {
        let publication_admission = BindingPublicationAdmission::acquire()?;
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let tcp_addr = listener.local_addr()?.to_string();
        let (shutdown_tx, _) = broadcast::channel(1);

        let state = ServerState {
            safety: safety.clone(),
            capture,
        };

        // Reserve the named-pipe endpoint before publication, but do not accept any connection until
        // the discovery artifact is verified. TCP is likewise held as an unspawned listener here.
        #[cfg(target_os = "windows")]
        let (pipe_name, first_pipe) = Self::prepare_named_pipe();
        #[cfg(not(target_os = "windows"))]
        let pipe_name = None;

        let binding = McpBinding {
            tcp_addr,
            pipe_name: pipe_name.clone(),
            token: state.safety.token.as_hex().to_owned(),
            pid: std::process::id(),
        };
        let binding_for_write = binding.clone();
        let mut publication_task = tokio::task::spawn_blocking(move || {
            let path = binding::write_binding(&binding_for_write)?;
            let observed = binding::read_binding()?;
            if observed != binding_for_write {
                return Err(binding::BindingError(format!(
                    "canonical reread mismatch after publishing {}",
                    path.display()
                )));
            }
            #[cfg(test)]
            {
                let delay_ms = TEST_DELAY_AFTER_BINDING_VERIFY_MS.swap(0, Ordering::AcqRel);
                if delay_ms > 0 {
                    std::thread::sleep(Duration::from_millis(delay_ms));
                }
            }
            Ok(path)
        });
        let publication =
            tokio::time::timeout(BINDING_PUBLICATION_TIMEOUT, &mut publication_task).await;
        let published_path = match publication {
            Ok(Ok(Ok(path))) => path,
            Ok(Ok(Err(error))) => {
                let binding_for_cleanup = binding.clone();
                tokio::spawn(async move {
                    let _admission = publication_admission;
                    let _ = tokio::task::spawn_blocking(move || {
                        binding::remove_binding_if_owned(&binding_for_cleanup)
                    })
                    .await;
                });
                return Err(std::io::Error::other(format!(
                    "mcp binding publication failed: {error}"
                )));
            }
            Ok(Err(error)) => {
                let binding_for_cleanup = binding.clone();
                tokio::spawn(async move {
                    let _admission = publication_admission;
                    let _ = tokio::task::spawn_blocking(move || {
                        binding::remove_binding_if_owned(&binding_for_cleanup)
                    })
                    .await;
                });
                return Err(std::io::Error::other(format!(
                    "mcp binding publisher task failed: {error}"
                )));
            }
            Err(_) => {
                let binding_for_cleanup = binding.clone();
                tokio::spawn(async move {
                    let _admission = publication_admission;
                    let _ = publication_task.await;
                    let _ = tokio::task::spawn_blocking(move || {
                        binding::remove_binding_if_owned(&binding_for_cleanup)
                    })
                    .await;
                });
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "mcp binding publication exceeded the 5 second startup deadline",
                ));
            }
        };
        drop(publication_admission);
        tracing::info!(path = %published_path.display(), tcp = %binding.tcp_addr, "mcp binding written and verified");

        // Only a verified, discoverable server may begin accepting connections.
        {
            let state = state.clone();
            let mut shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = shutdown_rx.recv() => break,
                        accepted = listener.accept() => match accepted {
                            Ok((stream, _peer)) => {
                                let state = state.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = serve_connection(stream, state).await {
                                        tracing::debug!(error = %e, "mcp tcp connection closed with error");
                                    }
                                });
                            }
                            Err(e) => tracing::warn!(error = %e, "mcp tcp accept failed"),
                        }
                    }
                }
                tracing::debug!("mcp tcp accept loop stopped");
            });
        }
        #[cfg(target_os = "windows")]
        Self::spawn_named_pipe(&state, &shutdown_tx, pipe_name.as_deref(), first_pipe);

        Ok(Self {
            binding,
            shutdown_tx,
            binding_removed: false,
            safety,
        })
    }

    /// The production OS-window screenshot capture (focus-safe). Pass to [`Self::bind`] in the live app.
    pub fn os_window_capture() -> Arc<
        dyn Fn(&ArgusWindowDescriptor) -> Result<ScreenshotResult, ScreenshotError> + Send + Sync,
    > {
        Arc::new(capture_handshake_window_target)
    }

    /// Spawn the Windows named-pipe accept loop. Returns the pipe name on success, `None` (TCP-only) on
    /// any bind failure (non-fatal — red-team: named-pipe exhaustion must not crash the server).
    #[cfg(target_os = "windows")]
    fn prepare_named_pipe() -> (
        Option<String>,
        Option<tokio::net::windows::named_pipe::NamedPipeServer>,
    ) {
        use tokio::net::windows::named_pipe::ServerOptions;

        let pipe_name = format!(r"\\.\pipe\handshake_swarm_{}", std::process::id());
        match ServerOptions::new()
            .first_pipe_instance(true)
            .create(&pipe_name)
        {
            Ok(server) => (Some(pipe_name), Some(server)),
            Err(error) => {
                tracing::warn!(error = %error, pipe = %pipe_name, "named pipe bind failed; running TCP-only");
                (None, None)
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn spawn_named_pipe(
        state: &ServerState,
        shutdown_tx: &broadcast::Sender<()>,
        pipe_name: Option<&str>,
        first: Option<tokio::net::windows::named_pipe::NamedPipeServer>,
    ) {
        use tokio::net::windows::named_pipe::ServerOptions;

        let (Some(pipe_name), Some(first)) = (pipe_name, first) else {
            return;
        };

        let state = state.clone();
        let name = pipe_name.to_owned();
        let mut shutdown_rx = shutdown_tx.subscribe();
        tokio::spawn(async move {
            let mut server = first;
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => break,
                    connected = server.connect() => {
                        match connected {
                            Ok(()) => {
                                // Hand the connected instance to a task; create the next instance to keep
                                // listening (the standard tokio named-pipe accept pattern).
                                let this = std::mem::replace(
                                    &mut server,
                                    match ServerOptions::new().create(&name) {
                                        Ok(s) => s,
                                        Err(e) => {
                                            tracing::warn!(error = %e, "named pipe re-create failed; stopping pipe loop");
                                            break;
                                        }
                                    },
                                );
                                let state = state.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = serve_connection(this, state).await {
                                        tracing::debug!(error = %e, "mcp pipe connection closed with error");
                                    }
                                });
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "named pipe connect failed");
                            }
                        }
                    }
                }
            }
            tracing::debug!("mcp named-pipe accept loop stopped");
        });
    }

    /// The bound localhost TCP address (e.g. `127.0.0.1:54321`).
    pub fn tcp_addr(&self) -> &str {
        &self.binding.tcp_addr
    }

    /// The Windows named-pipe path, if a pipe was bound.
    pub fn pipe_name(&self) -> Option<&str> {
        self.binding.pipe_name.as_deref()
    }

    /// The discovery binding (tcp/pipe/token/pid).
    pub fn binding(&self) -> &McpBinding {
        &self.binding
    }

    /// MT-028: the shared attributed-action audit log every connection appends to. The live shell /
    /// diagnostics drain this for a post-hoc trace of which agent steered which widget; the concurrent
    /// harness test asserts the entry count + per-agent attribution after driving N clients.
    pub fn action_log(&self) -> &ActionLog {
        self.safety.log()
    }

    /// MT-028: the shared lease registry every connection contends on (for diagnostics / tests).
    pub fn leases(&self) -> &LeaseRegistry {
        self.safety.leases()
    }

    /// MT-028: the full shared swarm-safety state (lease registry + attribution log + token + shared
    /// snapshot/channel). Exposed so a diagnostic surface or test can reach all of it from one handle.
    pub fn safety(&self) -> &SwarmSafetyState {
        &self.safety
    }

    /// Stop the accept loops and remove the discovery file. Idempotent.
    pub fn shutdown(&mut self) {
        // A send error just means there are no live receivers (loops already stopped) — fine.
        let _ = self.shutdown_tx.send(());
        if !self.binding_removed {
            if let Err(e) = binding::remove_binding_if_owned(&self.binding) {
                tracing::warn!(error = %e, "mcp binding file removal failed on shutdown");
            }
            self.binding_removed = true;
        }
    }
}

impl Drop for SwarmMcpServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Serve one connection: read newline-delimited JSON-RPC requests, dispatch, write newline-delimited
/// responses, until EOF or a fatal framing/IO error. Each connection has its own rate-limit bucket.
async fn serve_connection<S>(stream: S, state: ServerState) -> std::io::Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);
    let mut limiter = RateLimiter::new(MAX_REQUESTS_PER_SEC);

    // MT-028: one McpSession PER CONNECTION, so its agent_id (derived from the session token) is stable
    // for every request on this connection and the shared lease registry + attribution log are reused.
    let session = state.safety.session();

    let mut line = String::new();
    loop {
        line.clear();
        let n = read_line_bounded(&mut reader, &mut line, MAX_LINE_BYTES).await?;
        if n == 0 {
            break; // EOF
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response_json = handle_line(trimmed, &state, &session, &mut limiter).await;
        let mut out = serde_json::to_string(&response_json).unwrap_or_else(|_| {
            "{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32603,\"message\":\"serialize failed\"}}".to_owned()
        });
        out.push('\n');
        write_half.write_all(out.as_bytes()).await?;
        write_half.flush().await?;
    }
    Ok(())
}

/// Parse + dispatch a single request line into a JSON-RPC response value. `async` because the dispatch
/// now AWAITS the per-widget lease (yielding the tokio worker instead of blocking it); the parse + rate
/// limit path stays synchronous. Rate-limit and envelope-parse failures map to well-formed JSON-RPC
/// errors. Tests `await` it on a current-thread runtime.
async fn handle_line(
    line: &str,
    state: &ServerState,
    session: &McpSession,
    limiter: &mut RateLimiter,
) -> serde_json::Value {
    // Rate limit BEFORE parsing/dispatch so a flood cannot even reach the auth/tool path.
    if !limiter.allow() {
        return serde_json::json!({
            "jsonrpc": "2.0",
            "id": serde_json::Value::Null,
            "error": { "code": ERR_RATE_LIMITED, "message": "Rate limited" },
        });
    }

    let value: serde_json::Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "jsonrpc": "2.0",
                "id": serde_json::Value::Null,
                "error": { "code": ERR_INVALID_PARAMS, "message": format!("invalid JSON: {e}") },
            });
        }
    };
    let request = match McpRequest::from_json(&value) {
        Ok(r) => r,
        Err(e) => {
            // Preserve the request id if present so the client can correlate the error.
            let id = value.get("id").cloned().unwrap_or(serde_json::Value::Null);
            return serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": e.code, "message": e.message },
            });
        }
    };

    dispatch_with_session(&request, session, state)
        .await
        .to_json()
}

/// Dispatch one request through `session` so MT-028 leasing + attribution are applied, taking each
/// shared lock for the MINIMUM span:
///
/// - the snapshot lock is taken only to CLONE the current-frame snapshot (a cheap, lock-free-thereafter
///   read surface), then released immediately;
/// - the channel lock is NOT taken here — [`McpSession::dispatch_shared_async`] takes it ONLY for the
///   brief resolve+enqueue, AFTER acquiring the per-widget lease, and never across the lease wait. This
///   is the MAJOR fix: the global channel lock no longer serializes all dispatch, so the per-widget
///   lease is the real contention point (same widget serializes; different widgets proceed concurrently;
///   reads interleave).
///
/// `async` because the lease wait is now `tokio::time::sleep`-based, yielding the worker thread.
///
/// The per-CONNECTION `session` is built once when the connection is accepted (so its `agent_id` is
/// stable for the connection's whole lifetime) and reused for every request on that connection.
async fn dispatch_with_session(
    request: &McpRequest,
    session: &McpSession,
    state: &ServerState,
) -> McpResponse {
    let capture = state.capture.clone();
    session
        .dispatch_argus_shared_async(
            request,
            &state.safety.windows,
            &state.safety.channel,
            move |window| capture(window),
        )
        .await
}

/// Read one `\n`-terminated line into `buf`, but error out (rather than buffer unboundedly) once the
/// pending line exceeds `max_bytes` (red-team: unbounded-line OOM). Returns bytes read (0 on EOF).
async fn read_line_bounded<R>(
    reader: &mut R,
    buf: &mut String,
    max_bytes: usize,
) -> std::io::Result<usize>
where
    R: AsyncBufReadExt + Unpin,
{
    let mut bytes = Vec::new();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            break; // EOF
        }
        if let Some(pos) = available.iter().position(|&b| b == b'\n') {
            bytes.extend_from_slice(&available[..=pos]);
            reader.consume(pos + 1);
            break;
        } else {
            bytes.extend_from_slice(available);
            let consumed = available.len();
            reader.consume(consumed);
            if bytes.len() > max_bytes {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "request line exceeds max length",
                ));
            }
        }
    }
    let n = bytes.len();
    match String::from_utf8(bytes) {
        Ok(s) => {
            buf.push_str(&s);
            Ok(n)
        }
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "request line is not valid UTF-8",
        )),
    }
}

/// A simple per-connection token-bucket rate limiter: refills `rate` tokens per second, one token per
/// request. Used to reject an action flood with `-32003` before it reaches the egui frame loop.
struct RateLimiter {
    capacity: f64,
    tokens: f64,
    refill_per_sec: f64,
    last: std::time::Instant,
}

impl RateLimiter {
    fn new(rate_per_sec: u32) -> Self {
        let cap = rate_per_sec.max(1) as f64;
        Self {
            capacity: cap,
            tokens: cap,
            refill_per_sec: cap,
            last: std::time::Instant::now(),
        }
    }

    /// Try to consume one token; returns true if allowed. Refills based on elapsed wall time.
    fn allow(&mut self) -> bool {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.capacity);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::{UiTreeNode, UiTreeSnapshot};
    use crate::mcp::MAIN_WINDOW_ID;

    fn snap() -> UiTreeSnapshot {
        let button = UiTreeNode {
            id: "btn".to_owned(),
            author_id: Some("btn".to_owned()),
            node_id: 10,
            role: "Button".to_owned(),
            label: Some("Go".to_owned()),
            value: None,
            disabled: false,
            actions: vec!["Click".to_owned(), "Focus".to_owned()],
            bounds: None,
            children: Vec::new(),
        };
        let root = UiTreeNode {
            id: "node:1".to_owned(),
            author_id: None,
            node_id: 1,
            role: "Window".to_owned(),
            label: None,
            value: None,
            disabled: false,
            actions: Vec::new(),
            bounds: None,
            children: vec![button],
        };
        UiTreeSnapshot {
            root,
            captured_at_utc: "0Z".to_owned(),
            widget_count: 2,
        }
    }

    fn test_state(token: &str) -> ServerState {
        let safety = SwarmSafetyState::new(
            SessionToken::from_hex(token),
            Arc::new(Mutex::new(snap())),
            Arc::new(Mutex::new(ActionChannel::new())),
        );
        ServerState {
            safety,
            capture: Arc::new(|_| Ok(crate::mcp::screenshot::screenshot_from_png(b"foobar", 4, 3))),
        }
    }

    fn complete_next_main_action(state: &ServerState) -> std::thread::JoinHandle<()> {
        let channel = state.safety.channel.clone();
        let windows = state.safety.windows.clone();
        std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
            loop {
                let (batch, tracker) = {
                    let mut channel = channel
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    let tracker = channel.receipt_tracker();
                    (channel.drain_for_window(MAIN_WINDOW_ID), tracker)
                };
                if !batch.action_ids.is_empty() {
                    let current = windows.get(MAIN_WINDOW_ID).expect("main snapshot");
                    let snapshot = current.snapshot.clone();
                    let revision = windows.publish(current.window, current.snapshot);
                    for action_id in batch.action_ids {
                        tracker.acknowledge_effect(&action_id);
                        tracker.observe_postcondition(&action_id, revision, &snapshot);
                    }
                    return;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "timed out waiting for queued Argus action"
                );
                std::thread::yield_now();
            }
        })
    }

    #[tokio::test]
    async fn handle_line_dispatches_authed_list_widgets() {
        let state = test_state("secret-token-1234567890");
        let session = state.safety.session();
        let mut limiter = RateLimiter::new(MAX_REQUESTS_PER_SEC);
        let line = r#"{"jsonrpc":"2.0","id":1,"method":"argus.inspect","params":{"window_id":"main"},"session_token":"secret-token-1234567890","agent_label":"server-test"}"#;
        let resp = handle_line(line, &state, &session, &mut limiter).await;
        assert_eq!(resp["result"]["snapshot"]["widget_count"], 2);
        assert_eq!(resp["result"]["snapshot"]["root"]["role"], "Window");
    }

    #[tokio::test]
    async fn handle_line_lists_registered_windows_over_canonical_method() {
        let state = test_state("secret-token-1234567890");
        state.safety.windows.register(ArgusWindowDescriptor {
            window_id: "popout-pane-a".to_owned(),
            viewport_id: "PANE_A".to_owned(),
            title: "Handshake – Workspace".to_owned(),
        });
        let session = state.safety.session();
        let mut limiter = RateLimiter::new(MAX_REQUESTS_PER_SEC);
        let line = r#"{"jsonrpc":"2.0","id":11,"method":"argus.list_windows","params":{},"session_token":"secret-token-1234567890","agent_label":"server-test"}"#;
        let response = handle_line(line, &state, &session, &mut limiter).await;
        assert_eq!(response["result"]["windows"][0]["window_id"], "main");
        assert_eq!(
            response["result"]["windows"][1]["window_id"],
            "popout-pane-a"
        );
        assert_eq!(
            response["result"]["windows"][1]["snapshot_available"],
            false
        );
    }

    #[tokio::test]
    async fn handle_line_rejects_bad_token_over_wire_shape() {
        let state = test_state("secret-token-1234567890");
        let session = state.safety.session();
        let mut limiter = RateLimiter::new(MAX_REQUESTS_PER_SEC);
        let line = r#"{"jsonrpc":"2.0","id":2,"method":"list_widgets","params":{},"session_token":"WRONG"}"#;
        let resp = handle_line(line, &state, &session, &mut limiter).await;
        assert_eq!(resp["error"]["code"], -32001);
        assert_eq!(resp["error"]["message"], "Unauthorized");
        assert!(resp.get("result").is_none());
    }

    #[tokio::test]
    async fn handle_line_click_enqueues_into_shared_channel_and_attributes() {
        let state = test_state("secret-token-1234567890");
        let session = state.safety.session();
        let mut limiter = RateLimiter::new(MAX_REQUESTS_PER_SEC);
        let completer = complete_next_main_action(&state);
        let line = r#"{"jsonrpc":"2.0","id":3,"method":"argus.click","params":{"window_id":"main","author_id":"btn","expected_snapshot_revision":1},"session_token":"secret-token-1234567890","agent_label":"server-test"}"#;
        let resp = handle_line(line, &state, &session, &mut limiter).await;
        completer.join().expect("UI completion worker");
        assert_eq!(resp["result"]["status"], "applied");
        assert_eq!(resp["result"]["action"], "Click");
        assert_eq!(resp["result"]["agent_label"], "server-test");
        assert_eq!(resp["result"]["window_id"], MAIN_WINDOW_ID);
        assert_eq!(resp["result"]["author_id"], "btn");
        assert_eq!(resp["result"]["before_revision"], 1);
        assert_eq!(resp["result"]["after_revision"], 2);
        assert_eq!(
            state.safety.channel.lock().unwrap().pending(),
            0,
            "applied action was consumed from the shared channel"
        );
        // MT-028: the click is attributed in the shared log with this connection's agent_id.
        let entries = state.safety.log().drain_log();
        assert_eq!(
            entries.len(),
            1,
            "the click is recorded in the attribution log"
        );
        // ADVERSARIAL FIX: production attribution records the AUTHENTICATED principal
        // (token-derived `agent_id`, or a broker principal), NEVER the caller-supplied
        // display label. "server-test" is the `agent_label` display metadata; it must NOT
        // become the principal. The prior assertion (`agent_id == "server-test"`) asserted
        // the wrong invariant — it would have masked a label-becomes-principal regression.
        let expected_principal =
            crate::mcp::attribution::agent_id_for_token("secret-token-1234567890");
        assert_eq!(
            entries[0].agent_id, expected_principal,
            "agent_id is the token-derived authenticated principal"
        );
        assert_ne!(
            entries[0].agent_id, "server-test",
            "the display label must never become the authenticated principal"
        );
        assert_eq!(
            entries[0].agent_label, "server-test",
            "the display label is retained separately in agent_label"
        );
        assert_eq!(entries[0].target_key, "btn");
    }

    #[tokio::test]
    async fn handle_line_invalid_json_is_minus_32602() {
        let state = test_state("secret-token-1234567890");
        let session = state.safety.session();
        let mut limiter = RateLimiter::new(MAX_REQUESTS_PER_SEC);
        let resp = handle_line("not json at all", &state, &session, &mut limiter).await;
        assert_eq!(resp["error"]["code"], -32602);
    }

    #[test]
    fn rate_limiter_rejects_burst_beyond_capacity() {
        let mut rl = RateLimiter::new(5);
        let mut allowed = 0;
        for _ in 0..20 {
            if rl.allow() {
                allowed += 1;
            }
        }
        // The bucket starts full at capacity (5) and barely refills within a tight loop, so far fewer
        // than 20 are allowed.
        assert!(allowed <= 6, "burst was rate-limited; allowed {allowed}");
        assert!(
            allowed >= 5,
            "the initial full bucket is honored; allowed {allowed}"
        );
    }

    #[test]
    fn timed_out_publication_removes_late_binding_and_releases_admission() {
        let _env_guard = binding::BINDING_ENV_GUARD
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root =
            std::env::temp_dir().join(format!("hsk_mcp_late_publication_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create late-publication test root");
        #[cfg(target_os = "windows")]
        let app_data_var = "LOCALAPPDATA";
        #[cfg(not(target_os = "windows"))]
        let app_data_var = "XDG_DATA_HOME";
        let previous = std::env::var_os(app_data_var);
        std::env::set_var(app_data_var, &root);

        TEST_DELAY_AFTER_BINDING_VERIFY_MS.store(250, Ordering::Release);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("build publication-timeout runtime");
        let state = test_state("secret-token-1234567890");
        let result = runtime.block_on(SwarmMcpServer::bind_with_targeted_safety(
            state.safety,
            state.capture,
        ));
        let error = match result {
            Ok(_) => panic!("late publication must time out"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);

        runtime.block_on(async {
            let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
            loop {
                if !BINDING_PUBLICATION_IN_FLIGHT.load(Ordering::Acquire)
                    && !binding::binding_path().exists()
                {
                    break;
                }
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "late publication cleanup did not remove the binding and release admission"
                );
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
        drop(BindingPublicationAdmission::acquire().expect("admission is reusable after cleanup"));

        match previous {
            Some(value) => std::env::set_var(app_data_var, value),
            None => std::env::remove_var(app_data_var),
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
