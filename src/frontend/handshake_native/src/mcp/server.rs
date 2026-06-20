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

use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use crate::accessibility::UiTreeSnapshot;
use crate::mcp::action::ActionChannel;
use crate::mcp::binding::{self, McpBinding};
use crate::mcp::screenshot::{capture_handshake_window, ScreenshotError, ScreenshotResult};
use crate::mcp::tools::{dispatch_request, McpRequest, McpResponse, SessionToken, ERR_INVALID_PARAMS, ERR_RATE_LIMITED};

/// Max JSON-RPC requests one connection may issue per second before the server replies `-32003`
/// (`Rate limited`). 100/sec is generous for multi-step steering yet bounds an adversarial flood.
pub const MAX_REQUESTS_PER_SEC: u32 = 100;

/// Max bytes in a single newline-delimited request line. A request larger than this is rejected (the
/// connection is closed) so a malicious client cannot exhaust memory with one unbounded line.
pub const MAX_LINE_BYTES: usize = 1 << 20; // 1 MiB

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
}

/// The shared steering state the server's connection tasks read/write, cloned into each task.
#[derive(Clone)]
struct ServerState {
    token: SessionToken,
    snapshot: Arc<Mutex<UiTreeSnapshot>>,
    channel: Arc<Mutex<ActionChannel>>,
    /// The screenshot capture used by the `screenshot` tool. Boxed so tests can inject an
    /// offscreen-render closure in place of the OS-window grab.
    capture: Arc<dyn Fn() -> Result<ScreenshotResult, ScreenshotError> + Send + Sync>,
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
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let tcp_addr = listener.local_addr()?.to_string();
        let (shutdown_tx, _) = broadcast::channel(1);

        let state = ServerState {
            token,
            snapshot,
            channel,
            capture,
        };

        // Spawn the TCP accept loop (detached background task — HBR-QUIET).
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
                            Err(e) => {
                                tracing::warn!(error = %e, "mcp tcp accept failed");
                            }
                        }
                    }
                }
                tracing::debug!("mcp tcp accept loop stopped");
            });
        }

        // Windows named pipe (non-fatal on failure — TCP-only fallback).
        let pipe_name = Self::spawn_named_pipe(&state, &shutdown_tx);

        let binding = McpBinding {
            tcp_addr,
            pipe_name,
            token: state.token.as_hex().to_owned(),
            pid: std::process::id(),
        };
        match binding::write_binding(&binding) {
            Ok(path) => tracing::info!(path = %path.display(), tcp = %binding.tcp_addr, "mcp binding written"),
            Err(e) => tracing::warn!(error = %e, "mcp binding file write failed (server still running)"),
        }

        Ok(Self {
            binding,
            shutdown_tx,
            binding_removed: false,
        })
    }

    /// The production OS-window screenshot capture (focus-safe). Pass to [`Self::bind`] in the live app.
    pub fn os_window_capture() -> Arc<dyn Fn() -> Result<ScreenshotResult, ScreenshotError> + Send + Sync> {
        Arc::new(capture_handshake_window)
    }

    /// Spawn the Windows named-pipe accept loop. Returns the pipe name on success, `None` (TCP-only) on
    /// any bind failure (non-fatal — red-team: named-pipe exhaustion must not crash the server).
    #[cfg(target_os = "windows")]
    fn spawn_named_pipe(state: &ServerState, shutdown_tx: &broadcast::Sender<()>) -> Option<String> {
        use tokio::net::windows::named_pipe::ServerOptions;

        let pipe_name = format!(r"\\.\pipe\handshake_swarm_{}", std::process::id());
        // Try to create the first pipe instance up front so a bind failure is reported as TCP-only now.
        let first = match ServerOptions::new()
            .first_pipe_instance(true)
            .create(&pipe_name)
        {
            Ok(server) => server,
            Err(e) => {
                tracing::warn!(error = %e, pipe = %pipe_name, "named pipe bind failed; running TCP-only");
                return None;
            }
        };

        let state = state.clone();
        let name = pipe_name.clone();
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
        Some(pipe_name)
    }

    /// No named pipe on non-Windows builds (TCP-only).
    #[cfg(not(target_os = "windows"))]
    fn spawn_named_pipe(_state: &ServerState, _shutdown_tx: &broadcast::Sender<()>) -> Option<String> {
        None
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

    /// Stop the accept loops and remove the discovery file. Idempotent.
    pub fn shutdown(&mut self) {
        // A send error just means there are no live receivers (loops already stopped) — fine.
        let _ = self.shutdown_tx.send(());
        if !self.binding_removed {
            if let Err(e) = binding::remove_binding() {
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

        let response_json = handle_line(trimmed, &state, &mut limiter);
        let mut out = serde_json::to_string(&response_json).unwrap_or_else(|_| {
            "{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32603,\"message\":\"serialize failed\"}}".to_owned()
        });
        out.push('\n');
        write_half.write_all(out.as_bytes()).await?;
        write_half.flush().await?;
    }
    Ok(())
}

/// Parse + dispatch a single request line into a JSON-RPC response value. Pure (no IO) so it is unit
/// testable. Rate-limit and envelope-parse failures map to well-formed JSON-RPC errors.
fn handle_line(line: &str, state: &ServerState, limiter: &mut RateLimiter) -> serde_json::Value {
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

    dispatch_locked(&request, state).to_json()
}

/// Take the shared snapshot + channel locks for the minimum span needed to dispatch one request. The
/// locks are released before the response is serialized/written so a slow client never blocks the egui
/// frame loop's drain.
fn dispatch_locked(request: &McpRequest, state: &ServerState) -> McpResponse {
    let snapshot = state
        .snapshot
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|poisoned| poisoned.into_inner().clone());
    let mut channel = state
        .channel
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let capture = state.capture.clone();
    dispatch_request(request, &state.token, &snapshot, &mut channel, move || capture())
}

/// Read one `\n`-terminated line into `buf`, but error out (rather than buffer unboundedly) once the
/// pending line exceeds `max_bytes` (red-team: unbounded-line OOM). Returns bytes read (0 on EOF).
async fn read_line_bounded<R>(reader: &mut R, buf: &mut String, max_bytes: usize) -> std::io::Result<usize>
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
        UiTreeSnapshot { root, captured_at_utc: "0Z".to_owned(), widget_count: 2 }
    }

    fn test_state(token: &str) -> ServerState {
        ServerState {
            token: SessionToken::from_hex(token),
            snapshot: Arc::new(Mutex::new(snap())),
            channel: Arc::new(Mutex::new(ActionChannel::new())),
            capture: Arc::new(|| {
                Ok(crate::mcp::screenshot::screenshot_from_png(b"foobar", 4, 3))
            }),
        }
    }

    #[test]
    fn handle_line_dispatches_authed_list_widgets() {
        let state = test_state("secret-token-1234567890");
        let mut limiter = RateLimiter::new(MAX_REQUESTS_PER_SEC);
        let line = r#"{"jsonrpc":"2.0","id":1,"method":"list_widgets","params":{},"session_token":"secret-token-1234567890"}"#;
        let resp = handle_line(line, &state, &mut limiter);
        assert_eq!(resp["result"]["widget_count"], 2);
        assert_eq!(resp["result"]["root"]["role"], "Window");
    }

    #[test]
    fn handle_line_rejects_bad_token_over_wire_shape() {
        let state = test_state("secret-token-1234567890");
        let mut limiter = RateLimiter::new(MAX_REQUESTS_PER_SEC);
        let line = r#"{"jsonrpc":"2.0","id":2,"method":"list_widgets","params":{},"session_token":"WRONG"}"#;
        let resp = handle_line(line, &state, &mut limiter);
        assert_eq!(resp["error"]["code"], -32001);
        assert_eq!(resp["error"]["message"], "Unauthorized");
        assert!(resp.get("result").is_none());
    }

    #[test]
    fn handle_line_click_enqueues_into_shared_channel() {
        let state = test_state("secret-token-1234567890");
        let mut limiter = RateLimiter::new(MAX_REQUESTS_PER_SEC);
        let line = r#"{"jsonrpc":"2.0","id":3,"method":"click_widget","params":{"target":"btn"},"session_token":"secret-token-1234567890"}"#;
        let resp = handle_line(line, &state, &mut limiter);
        assert_eq!(resp["result"]["queued"], true);
        assert_eq!(state.channel.lock().unwrap().pending(), 1, "action landed in the shared channel");
    }

    #[test]
    fn handle_line_invalid_json_is_minus_32602() {
        let state = test_state("secret-token-1234567890");
        let mut limiter = RateLimiter::new(MAX_REQUESTS_PER_SEC);
        let resp = handle_line("not json at all", &state, &mut limiter);
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
        assert!(allowed >= 5, "the initial full bucket is honored; allowed {allowed}");
    }
}
