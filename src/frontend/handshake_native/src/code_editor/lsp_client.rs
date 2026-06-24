//! LSP (Language Server Protocol) client over a stdio JSON-RPC transport (WP-KERNEL-012 MT-008 — E1
//! code editor).
//!
//! [`LspClient`] launches a language-server subprocess lazily on the first [`LspClient::did_open`],
//! speaks LSP 3.17 JSON-RPC over its stdin/stdout, and exposes the editor-facing requests the MT
//! names: completion, hover, go-to-definition, references, and a `publishDiagnostics` notification
//! stream. When no language server is configured (or the process fails to start), EVERY method
//! degrades gracefully — empty `Vec` / `None`, never a panic (AC-004).
//!
//! ## Why a hand-rolled stdio transport over `lsp-types`, not tower-lsp
//!
//! RESEARCH (wf_ffa74d6d): `tower-lsp` is the SERVER framework and orders client-side notifications
//! incorrectly; the field-correct choice for a CLIENT is a lightweight raw JSON-RPC-over-stdio
//! transport plus the `lsp-types` protocol type definitions. This module is exactly that: `lsp-types`
//! supplies the request/response/notification param + result structs (serde-derived), and a small
//! [`transport`] layer frames them as `Content-Length`-delimited JSON-RPC messages on the server's
//! stdio.
//!
//! ## Message routing (impl note 1 + 4)
//!
//! One tokio reader task owns the server's stdout. It parses each LSP message and routes it: a message
//! WITH an `id` and a `result`/`error` goes to the pending request future waiting on that id (via a
//! `HashMap<RequestId, oneshot::Sender<Value>>`); a message WITHOUT an `id` (a notification, e.g.
//! `textDocument/publishDiagnostics`) goes to the notification channel the editor drains for
//! diagnostics. Malformed / non-JSON stdout lines (a server's stray debug print) are SKIPPED, never
//! panicked on (RISK-003).
//!
//! ## Focus-safe, leak-free supervision (MC-001 / RISK-001 / HBR-QUIET)
//!
//! The subprocess is spawned with piped stdio and (on Windows) the `CREATE_NO_WINDOW` flag so it never
//! pops a console window or steals OS focus (HBR-QUIET — the same focus-safe stance the rest of the
//! shell holds). [`LspClient`]'s [`Drop`] sends `shutdown` + `exit` and waits with a bounded
//! [`SHUTDOWN_TIMEOUT`] before killing the child, so closing the editor never leaks a zombie
//! language-server process (RISK-001 / MC-001).

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot};

use lsp_types::{
    CompletionResponse, GotoDefinitionResponse, Hover, HoverContents, Location, MarkedString,
    Position, PublishDiagnosticsParams, ServerCapabilities, SignatureHelp, TextDocumentIdentifier,
    TextDocumentPositionParams, Url,
};

/// How long [`LspClient::drop`] waits for the server to honor `shutdown`/`exit` before force-killing
/// the child (RISK-001 / MC-001). Bounded so closing the editor is never blocked on a wedged server.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// How long a single LSP request waits for its response before giving up with `None`/empty. A server
/// that never replies must not hang the editor's result-delivery task.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Why an LSP request that the editor treats as fallible (currently only `textDocument/rename` — MT-048)
/// could not produce a usable result. A no-server / timeout case is NOT an error for rename (it maps to an
/// empty WorkspaceEdit so the editor falls back to the single-file path); an error is reserved for an
/// unparseable URI or a garbled response body, so the caller can show a message instead of silently
/// dropping a real rename (AC-008 — never a panic).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspError {
    /// The document URI could not be parsed into an LSP `Url`.
    BadUri,
    /// The server's response body could not be deserialized into the expected type.
    Parse,
}

impl std::fmt::Display for LspError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LspError::BadUri => write!(f, "the document URI is not a valid LSP Url"),
            LspError::Parse => write!(f, "the LSP response body could not be parsed"),
        }
    }
}

impl std::error::Error for LspError {}

/// The completion items a single completion response yields (the popup list). Mirrors the LSP
/// `CompletionItem` but flattened to just the fields the popup renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspCompletionItem {
    /// The label shown in the popup (and the text inserted when no `insert_text` is given).
    pub label: String,
    /// The text inserted at the cursor on accept (falls back to `label`).
    pub insert_text: String,
    /// The server-provided detail line (type/signature), if any.
    pub detail: Option<String>,
    /// The numeric LSP `CompletionItemKind` (1..=25), or `None` when the server omitted it.
    pub kind: Option<i32>,
}

/// A hover result: the rendered markdown/plain text the server returned for the cursor position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverResult {
    /// The hover body (markdown when the server sent `MarkupContent`, else the plain string).
    pub value: String,
}

/// One diagnostic from a `textDocument/publishDiagnostics` notification, mapped to the editor's gutter
/// vocabulary. `line` is 0-based (the LSP `range.start.line` is already 0-based, matching the gutter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDiagnostic {
    /// The 0-based line the diagnostic applies to (LSP `range.start.line`).
    pub line: usize,
    /// The LSP severity (1=Error, 2=Warning, 3=Information, 4=Hint), defaulting to Error when omitted
    /// (the LSP convention: an omitted severity is client-interpreted; we treat it as the strongest).
    pub severity: i32,
    /// The human-readable diagnostic message.
    pub message: String,
}

/// A `textDocument/publishDiagnostics` payload routed to the editor: the document URI + its current
/// diagnostics. The editor maps `uri` to a panel and calls `push_diagnostics` with the markers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedDiagnostics {
    /// The document URI the diagnostics apply to (as the server sent it).
    pub uri: String,
    /// The diagnostics for that document (replaces the document's previous set — LSP semantics).
    pub diagnostics: Vec<LspDiagnostic>,
}

/// The configuration for launching a language server: the executable + its args. `None` everywhere
/// means "no server configured" and the client degrades to empty results (AC-004).
#[derive(Debug, Clone, Default)]
pub struct LspServerConfig {
    /// The language-server executable (e.g. `rust-analyzer`). Empty -> no server.
    pub command: String,
    /// Arguments passed to the server on spawn.
    pub args: Vec<String>,
}

impl LspServerConfig {
    /// A config that launches `command` with no args.
    pub fn command(command: impl Into<String>) -> Self {
        Self { command: command.into(), args: Vec::new() }
    }

    /// Whether a server is actually configured (a non-empty command).
    pub fn is_configured(&self) -> bool {
        !self.command.trim().is_empty()
    }
}

/// The live transport to a running language server: the writer half of stdin + the pending-request
/// table the reader task fulfills. Held inside [`LspClient`] behind a `Mutex` once the server is up.
///
/// `stdin` is behind an ASYNC ([`tokio::sync::Mutex`]) lock, NOT the outer std `Mutex<Option<Transport>>`:
/// the write path awaits the framed write, so it must hold an await-safe lock (a std `MutexGuard` held
/// across `.await` risks a same-thread deadlock — clippy `await_holding_lock`). The outer std mutex is
/// only ever held briefly + non-async (clone the `Arc<stdin>` + the `pending` `Arc`, then release).
struct Transport {
    /// The server's stdin writer (LSP requests are framed + written here), behind an async mutex so the
    /// framed write can be awaited without holding a std lock across the await. Generic over the writer
    /// type so a test can install a duplex-pipe writer (no real process) on the SAME request path.
    stdin: Arc<tokio::sync::Mutex<Box<dyn tokio::io::AsyncWrite + Send + Unpin>>>,
    /// The spawned child process handle (kept so `Drop` can wait/kill it — RISK-001). `None` for a
    /// test transport backed by an in-memory pipe (no OS process to reap).
    child: Option<Child>,
    /// Pending request id -> the oneshot sender the reader task fulfills when the matching response
    /// arrives. Shared with the reader task.
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>,
}

/// The LSP client. Owns the (lazily-spawned) server transport, the diagnostics notification channel,
/// and the monotonic request-id counter. Cheap to construct (no process until `did_open`); graceful
/// when no server is configured.
pub struct LspClient {
    /// The server launch config. When not [`LspServerConfig::is_configured`], every method is a
    /// graceful no-op (AC-004).
    config: LspServerConfig,
    /// The live transport, or `None` before the first `did_open` / when the spawn failed (graceful).
    /// Behind a `Mutex` so the `&self` editor methods can lazily spawn + send under shared ownership.
    transport: Mutex<Option<Transport>>,
    /// Monotonic JSON-RPC request id source.
    next_id: AtomicI64,
    /// The receive half of the `publishDiagnostics` channel — taken ONCE by the editor via
    /// [`LspClient::take_diagnostics_receiver`] so the editor drains diagnostics each frame.
    diagnostics_rx: Mutex<Option<mpsc::UnboundedReceiver<PublishedDiagnostics>>>,
    /// The send half, cloned into the reader task on spawn so notifications reach the editor.
    diagnostics_tx: mpsc::UnboundedSender<PublishedDiagnostics>,
    /// The tokio runtime handle captured at spawn, used by the [`Drop`] path to drive the bounded
    /// graceful shutdown (RISK-001). `None` until the server is spawned (a never-spawned client has no
    /// child to shut down). Behind a `Mutex` so the `&self` lazy-spawn path can set it.
    runtime: Mutex<Option<Handle>>,
    /// WP-KERNEL-012 MT-047: the `ServerCapabilities` cached from the `initialize` response. The
    /// signature-help path reads `signature_help_provider` (presence + the server-declared trigger
    /// characters) so the editor only issues a `textDocument/signatureHelp` request when the server
    /// supports it (otherwise it falls back to the code-nav signature). `None` until `initialize`
    /// completes (or when no server is configured — the graceful disabled path).
    server_capabilities: Mutex<Option<ServerCapabilities>>,
    /// TEST-ONLY: the mock server's read half of the in-memory transport (installed by
    /// `install_test_transport`). Production never sets this; it exists so a test can read the framed
    /// request the client emitted before writing the framed response on the SAME real request path.
    test_server_read: Mutex<Option<tokio::io::DuplexStream>>,
}

impl LspClient {
    /// Build a client for `config`. No process is launched yet (it spawns lazily on the first
    /// [`did_open`](Self::did_open)). With an unconfigured `config` every method degrades gracefully.
    pub fn new(config: LspServerConfig) -> Self {
        let (diagnostics_tx, diagnostics_rx) = mpsc::unbounded_channel();
        Self {
            config,
            transport: Mutex::new(None),
            next_id: AtomicI64::new(1),
            diagnostics_rx: Mutex::new(Some(diagnostics_rx)),
            diagnostics_tx,
            runtime: Mutex::new(None),
            server_capabilities: Mutex::new(None),
            test_server_read: Mutex::new(None),
        }
    }

    /// A client with NO server configured — every method returns empty/None without spawning anything
    /// (the AC-004 graceful path + the default the editor uses until a server is configured).
    pub fn disabled() -> Self {
        Self::new(LspServerConfig::default())
    }

    /// Whether a language server is configured (does not imply it has been spawned or is healthy).
    pub fn is_configured(&self) -> bool {
        self.config.is_configured()
    }

    /// Whether a server process is currently spawned + attached.
    pub fn is_running(&self) -> bool {
        self.transport.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    /// Take the diagnostics receiver (once) so the editor can drain `publishDiagnostics` each frame.
    /// Returns the receiver the first time and `None` afterward (one consumer per channel). The editor
    /// wires this to `push_diagnostics`.
    pub fn take_diagnostics_receiver(
        &self,
    ) -> Option<mpsc::UnboundedReceiver<PublishedDiagnostics>> {
        self.diagnostics_rx.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    /// Allocate the next JSON-RPC request id.
    fn alloc_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// TEST HOOK (AC-008): run the SAME production reader loop against an arbitrary `AsyncRead` (an
    /// in-memory pipe in a test, a real mock language server's stdout in an integration test), wired to
    /// THIS client's diagnostics channel. This drives the exact `transport::read_loop` +
    /// `route_message` the production reader uses, so a test feeding a `publishDiagnostics` frame proves
    /// the real notification-routing path (not a parallel reimplementation). Returns immediately; the
    /// loop runs on the current tokio runtime until the reader hits EOF.
    pub fn spawn_reader_for_test<R>(&self, reader: R)
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let diagnostics_tx = self.diagnostics_tx.clone();
        tokio::spawn(async move {
            transport::read_loop(reader, pending, diagnostics_tx).await;
        });
    }

    /// TEST HOOK (MT-047): install a `signatureHelpProvider` capability so [`supports_signature_help`]
    /// returns `true` without a live `initialize` handshake. `triggers` becomes the declared
    /// trigger-character set (empty -> the `( ,` default). Used by the signature-help request test +
    /// the capability-gating test.
    pub fn set_signature_help_capability_for_test(&self, triggers: &[char]) {
        let trigger_characters = if triggers.is_empty() {
            None
        } else {
            Some(triggers.iter().map(|c| c.to_string()).collect())
        };
        let caps = ServerCapabilities {
            signature_help_provider: Some(lsp_types::SignatureHelpOptions {
                trigger_characters,
                retrigger_characters: None,
                work_done_progress_options: lsp_types::WorkDoneProgressOptions::default(),
            }),
            ..Default::default()
        };
        *self.server_capabilities.lock().unwrap_or_else(|e| e.into_inner()) = Some(caps);
    }

    /// TEST HOOK (MT-047 / AC-001): install an in-memory duplex-pipe transport (NO OS process) wired to
    /// THIS client's real `request()` pending table + reader loop, and return the SERVER side of the
    /// pipe (read the framed request, write the framed response). This drives the EXACT production
    /// `request()` -> framed write -> `read_loop` -> `route_message` path a real stdio server would,
    /// so a test feeding a `textDocument/signatureHelp` response proves the real request/response
    /// routing (not a parallel reimplementation). The returned duplex half is the mock server's
    /// stdin(write)/stdout(read): the test reads the client's request bytes from it and writes the
    /// response frame back.
    pub fn install_test_transport(&self) -> tokio::io::DuplexStream {
        // client_to_server: the client writes requests here; the mock server reads them.
        // server_to_client: the mock server writes responses here; the client's reader reads them.
        let (client_write, server_read) = tokio::io::duplex(64 * 1024);
        let (server_write, client_read) = tokio::io::duplex(64 * 1024);
        let pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        // Spawn the REAL reader loop on the client_read half wired to this client's pending table +
        // diagnostics channel — identical to the production reader.
        let reader_pending = Arc::clone(&pending);
        let diagnostics_tx = self.diagnostics_tx.clone();
        tokio::spawn(async move {
            transport::read_loop(client_read, reader_pending, diagnostics_tx).await;
        });
        *self.runtime.lock().unwrap_or_else(|e| e.into_inner()) = Some(Handle::current());
        *self.transport.lock().unwrap_or_else(|e| e.into_inner()) = Some(Transport {
            stdin: Arc::new(tokio::sync::Mutex::new(
                Box::new(client_write) as Box<dyn tokio::io::AsyncWrite + Send + Unpin>,
            )),
            child: None, // no OS process for an in-memory transport.
            pending,
        });
        // The mock server reads requests from `server_read` and writes responses to `server_write`;
        // join them into one duplex-like pair the test drives. We return the write half here and the
        // read half via the companion accessor; to keep a single return, splice them with a small
        // pump task is overkill — instead return the server_write and stash server_read for the test
        // to read through `read_test_request`. Simpler: return server_write; the test uses
        // `read_test_request` to pull the request bytes.
        *self.test_server_read.lock().unwrap_or_else(|e| e.into_inner()) = Some(server_read);
        server_write
    }

    /// TEST HOOK (MT-047): read the next framed JSON-RPC request the client wrote over the test
    /// transport (installed by [`install_test_transport`]). Returns the parsed request `Value`, or
    /// `None` on EOF. The test calls this to observe the request before writing the response.
    pub async fn read_test_request(&self) -> Option<Value> {
        let mut reader = self.test_server_read.lock().unwrap_or_else(|e| e.into_inner()).take()?;
        let result = read_one_frame(&mut reader).await;
        // Park the reader back for any subsequent reads.
        *self.test_server_read.lock().unwrap_or_else(|e| e.into_inner()) = Some(reader);
        result
    }

    /// TEST HOOK (AC-008): frame a JSON-RPC message exactly as the production transport does
    /// (`Content-Length` header + body), so a test can write a real LSP `publishDiagnostics`
    /// notification frame into the reader's pipe.
    pub fn frame_message_for_test(message: &Value) -> Vec<u8> {
        let body = serde_json::to_vec(message).unwrap_or_default();
        let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        out.extend_from_slice(&body);
        out
    }

    /// Lazily spawn the server (if configured and not already running) and run the `initialize`
    /// handshake. Returns `true` when a transport is live afterward, `false` when no server is
    /// configured or the spawn/handshake failed (graceful — AC-004 / RISK-003). Idempotent.
    pub async fn initialize(&self, root_uri: Option<Url>) -> bool {
        if !self.config.is_configured() {
            return false; // AC-004: no server -> graceful no-op.
        }
        if self.is_running() {
            return true;
        }
        // Spawn focus-safe (RISK-001 / HBR-QUIET): piped stdio + CREATE_NO_WINDOW on Windows so the
        // server never pops a console or steals focus. A spawn failure is graceful (AC-004).
        let mut command = Command::new(&self.config.command);
        command
            .args(&self.config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        {
            // CREATE_NO_WINDOW (0x0800_0000): the server never flashes a console window (HBR-QUIET).
            command.creation_flags(0x0800_0000);
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(_) => return false, // server binary missing / not launchable -> graceful (AC-004).
        };
        let Some(stdin) = child.stdin.take() else { return false };
        let Some(stdout) = child.stdout.take() else { return false };

        let pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        // Spawn the reader task: it owns stdout, routes responses to `pending` + notifications to the
        // diagnostics channel, and SKIPS malformed lines (RISK-003).
        let reader_pending = Arc::clone(&pending);
        let diagnostics_tx = self.diagnostics_tx.clone();
        tokio::spawn(async move {
            transport::read_loop(stdout, reader_pending, diagnostics_tx).await;
        });

        // Capture the runtime handle now (we are on a runtime in `initialize`) so the Drop path can
        // drive the bounded graceful shutdown (RISK-001).
        *self.runtime.lock().unwrap_or_else(|e| e.into_inner()) = Some(Handle::current());
        *self.transport.lock().unwrap_or_else(|e| e.into_inner()) = Some(Transport {
            stdin: Arc::new(tokio::sync::Mutex::new(
                Box::new(stdin) as Box<dyn tokio::io::AsyncWrite + Send + Unpin>,
            )),
            child: Some(child),
            pending,
        });

        // The LSP `initialize` request (a minimal client capabilities object is enough for the
        // editor-facing requests). A handshake failure tears the transport back down (graceful).
        let params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri.map(|u| u.to_string()),
            "capabilities": {},
            "clientInfo": { "name": "handshake-native-editor" }
        });
        let init_result = self.request("initialize", params).await;
        if let Some(result) = init_result {
            // Cache the server's declared capabilities (MT-047: the signature-help path reads
            // `signature_help_provider`). The `initialize` result is `{ capabilities, serverInfo }`; a
            // missing/garbled `capabilities` degrades to `None` (the editor then treats every optional
            // capability as unsupported — graceful, never a panic).
            if let Some(caps) = result.get("capabilities").cloned() {
                if let Ok(parsed) = serde_json::from_value::<ServerCapabilities>(caps) {
                    *self.server_capabilities.lock().unwrap_or_else(|e| e.into_inner()) = Some(parsed);
                }
            }
            // Per the spec the client sends `initialized` after the handshake. Best-effort.
            self.notify("initialized", serde_json::json!({})).await;
            true
        } else {
            // Handshake failed: drop the half-initialized transport so methods stay graceful.
            *self.transport.lock().unwrap_or_else(|e| e.into_inner()) = None;
            false
        }
    }

    /// `textDocument/didOpen`: tell the server a document is open. Spawns + initializes the server
    /// lazily on the first call (impl note "launched lazily on first did_open"). A graceful no-op when
    /// no server is configured / the spawn failed (AC-004).
    pub async fn did_open(&self, uri: &str, language_id: &str, text: &str) {
        // Lazy spawn: derive a root uri from the document's directory when possible.
        if !self.is_running() {
            let root = Url::parse(uri).ok();
            if !self.initialize(root).await {
                return; // graceful (AC-004).
            }
        }
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": 1,
                "text": text,
            }
        });
        self.notify("textDocument/didOpen", params).await;
    }

    /// `textDocument/didChange`: push a full-document change (the editor sends the whole new text — the
    /// simplest correct sync mode, matching the lazy editor wiring). A graceful no-op when no server.
    pub async fn did_change(&self, uri: &str, version: i64, text: &str) {
        if !self.is_running() {
            return; // AC-004: no server -> graceful no-op.
        }
        let params = serde_json::json!({
            "textDocument": { "uri": uri, "version": version },
            "contentChanges": [ { "text": text } ]
        });
        self.notify("textDocument/didChange", params).await;
    }

    /// `textDocument/completion`: request completions at `position`. Returns the popup items (empty on
    /// no server / no response / a server error — AC-004).
    pub async fn completion(&self, uri: &str, position: Position) -> Vec<LspCompletionItem> {
        let Some(params) = position_params(uri, position) else { return Vec::new() };
        let Some(result) = self.request("textDocument/completion", params).await else {
            return Vec::new();
        };
        // The response is `CompletionResponse` (a list, an array, or null).
        match serde_json::from_value::<CompletionResponse>(result) {
            Ok(CompletionResponse::Array(items)) => {
                items.into_iter().map(completion_item_from_lsp).collect()
            }
            Ok(CompletionResponse::List(list)) => {
                list.items.into_iter().map(completion_item_from_lsp).collect()
            }
            Err(_) => Vec::new(),
        }
    }

    /// `textDocument/hover`: request hover docs at `position`. Returns the rendered hover text, or
    /// `None` (no server / no hover / a server error — AC-004).
    pub async fn hover(&self, uri: &str, position: Position) -> Option<HoverResult> {
        let params = position_params(uri, position)?;
        let result = self.request("textDocument/hover", params).await?;
        // A null hover deserializes to `None`; a present one to `Some(Hover)`.
        let hover: Option<Hover> = serde_json::from_value(result).ok()?;
        let hover = hover?;
        let value = hover_contents_to_string(hover.contents);
        if value.trim().is_empty() {
            None
        } else {
            Some(HoverResult { value })
        }
    }

    /// `textDocument/definition`: request the definition location of the symbol at `position`. Returns
    /// the first location (the editor opens/scrolls to it), or `None` (AC-004).
    pub async fn goto_definition(&self, uri: &str, position: Position) -> Option<Location> {
        let params = position_params(uri, position)?;
        let result = self.request("textDocument/definition", params).await?;
        let response: Option<GotoDefinitionResponse> = serde_json::from_value(result).ok()?;
        match response? {
            GotoDefinitionResponse::Scalar(loc) => Some(loc),
            GotoDefinitionResponse::Array(locs) => locs.into_iter().next(),
            GotoDefinitionResponse::Link(links) => links.into_iter().next().map(|l| Location {
                uri: l.target_uri,
                range: l.target_selection_range,
            }),
        }
    }

    /// `textDocument/references`: request the reference locations of the symbol at `position`. Returns
    /// every location (empty on no server / no response — AC-004).
    pub async fn references(&self, uri: &str, position: Position) -> Vec<Location> {
        let Some(base) = position_params(uri, position) else { return Vec::new() };
        // The references request adds a `context.includeDeclaration` field to the position params.
        let mut params = base;
        params["context"] = serde_json::json!({ "includeDeclaration": true });
        let Some(result) = self.request("textDocument/references", params).await else {
            return Vec::new();
        };
        serde_json::from_value::<Option<Vec<Location>>>(result)
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    /// WP-KERNEL-012 MT-048: request `textDocument/rename` at `position` over the EXISTING MT-008 stdio
    /// JSON-RPC transport (NO second transport — AC-007). Serializes the params
    /// `{ textDocument: { uri }, position: { line, character }, newName }` and deserializes the
    /// `WorkspaceEdit` response (the `lsp_types::WorkspaceEdit` type covers BOTH the `changes` map form
    /// `{ [uri]: TextEdit[] }` AND the `documentChanges` array form `[{ textDocument, edits }]` — RISK-003
    /// / MC-003 / AC-007). A null/empty WorkspaceEdit response maps to `Ok(WorkspaceEdit::default())` (the
    /// no-op rename — the caller shows "no changes"); any LSP error / no server / no transport / a garbled
    /// body maps to `Err(LspError)` or `Ok(empty)` so the editor never panics (AC-008). Reuses the same
    /// `request()` -> framed write -> `read_loop` -> `route_message` path every other LSP request uses.
    pub async fn rename(
        &self,
        uri: &str,
        position: Position,
        new_name: &str,
    ) -> Result<lsp_types::WorkspaceEdit, LspError> {
        let Some(base) = position_params(uri, position) else {
            return Err(LspError::BadUri);
        };
        // The rename request adds `newName` to the position params.
        let mut params = base;
        params["newName"] = serde_json::json!(new_name);
        let Some(result) = self.request("textDocument/rename", params).await else {
            // No server / no transport / timeout: a graceful no-op rename (empty WorkspaceEdit), NOT an
            // error — the caller falls back to the single-file path when no LSP is attached (AC-003).
            return Ok(lsp_types::WorkspaceEdit::default());
        };
        // A null response (the server declined the rename) deserializes to `None` -> an empty edit (no-op).
        match serde_json::from_value::<Option<lsp_types::WorkspaceEdit>>(result) {
            Ok(Some(edit)) => Ok(edit),
            Ok(None) => Ok(lsp_types::WorkspaceEdit::default()),
            Err(_) => Err(LspError::Parse),
        }
    }

    /// WP-KERNEL-012 MT-047: whether the attached server declared `signatureHelpProvider` in its
    /// `initialize` capabilities. The editor uses this to SKIP the LSP signature-help request (and fall
    /// back to the code-nav signature) when the server cannot serve it. `false` when no server is
    /// configured / `initialize` has not run / the server omitted the capability — all graceful.
    pub fn supports_signature_help(&self) -> bool {
        self.server_capabilities
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|c| c.signature_help_provider.is_some())
            .unwrap_or(false)
    }

    /// WP-KERNEL-012 MT-047: the server-declared signature-help trigger characters (the chars that
    /// initiate a request while typing a call), defaulting to `(` and `,` when the server did not
    /// declare a set (or no server is attached). The editor prefers the server's set when present so it
    /// matches the language's call syntax; otherwise the contract's `( ,` default.
    pub fn signature_help_trigger_chars(&self) -> Vec<char> {
        let declared = self
            .server_capabilities
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .and_then(|c| c.signature_help_provider.as_ref())
            .and_then(|p| p.trigger_characters.clone());
        match declared {
            Some(chars) if !chars.is_empty() => chars
                .iter()
                .filter_map(|s| s.chars().next())
                .collect(),
            _ => vec!['(', ','],
        }
    }

    /// WP-KERNEL-012 MT-047: request `textDocument/signatureHelp` (parameter hints) at `position` over
    /// the EXISTING MT-008 stdio JSON-RPC transport (no second transport — AC-006). Returns the parsed
    /// `lsp_types::SignatureHelp`, or `None` when: no server is attached, the server did not declare
    /// `signatureHelpProvider` (the caller then falls back to the code-nav signature — AC-003), the
    /// request times out, or the response is null/garbled (AC-008 — never a panic). The caller converts
    /// the result into the transport-agnostic `SignatureHelpState` via `SignatureHelpState::from_lsp`.
    pub async fn signature_help(&self, uri: &str, position: Position) -> Option<SignatureHelp> {
        // Skip the round-trip when the server cannot serve signature help (let the caller fall back).
        if !self.supports_signature_help() {
            return None;
        }
        let params = position_params(uri, position)?;
        let result = self.request("textDocument/signatureHelp", params).await?;
        // A null response (the cursor is not inside a call) deserializes to `None`; a present one to
        // `Some(SignatureHelp)`. A malformed body degrades to `None` (AC-008).
        serde_json::from_value::<Option<SignatureHelp>>(result).ok().flatten()
    }

    /// Send a JSON-RPC REQUEST and await its `result` (or `None` on no transport / timeout / a server
    /// `error`). The request id is registered in the pending table before the bytes are written, so a
    /// fast response can never race ahead of the registration.
    async fn request(&self, method: &str, params: Value) -> Option<Value> {
        let id = self.alloc_id();
        let (tx, rx) = oneshot::channel();
        // Clone the stdin (async-locked) + pending handles out of the std transport guard, then RELEASE
        // the std guard BEFORE the await (clippy await_holding_lock — a std MutexGuard must not be held
        // across `.await`). If there is no transport, this is a graceful None.
        let (stdin, pending) = {
            let guard = self.transport.lock().unwrap_or_else(|e| e.into_inner());
            let transport = guard.as_ref()?;
            (Arc::clone(&transport.stdin), Arc::clone(&transport.pending))
        };
        // Register the pending sender before the bytes go out so a fast response cannot race ahead.
        pending.lock().unwrap_or_else(|e| e.into_inner()).insert(id, tx);
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        // Write under the ASYNC stdin lock (await-safe); serialized sends preserve JSON-RPC framing.
        {
            let mut stdin = stdin.lock().await;
            if transport::write_message(&mut **stdin, &message).await.is_err() {
                pending.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
                return None;
            }
        }
        // Await the response with a bound so a silent server cannot hang the editor's delivery task.
        match tokio::time::timeout(REQUEST_TIMEOUT, rx).await {
            Ok(Ok(value)) => Some(value),
            _ => {
                // Timeout / channel closed: drop the pending entry so it does not leak.
                pending.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
                None
            }
        }
    }

    /// Send a JSON-RPC NOTIFICATION (no id, no response). A graceful no-op when there is no transport.
    async fn notify(&self, method: &str, params: Value) {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        // Clone the async-locked stdin out of the std guard, release the std guard, then write.
        let stdin = {
            let guard = self.transport.lock().unwrap_or_else(|e| e.into_inner());
            match guard.as_ref() {
                Some(transport) => Arc::clone(&transport.stdin),
                None => return, // graceful no-op.
            }
        };
        let mut stdin = stdin.lock().await;
        let _ = transport::write_message(&mut **stdin, &message).await;
    }

    /// Send `shutdown` + `exit` and wait (bounded) for the child to exit, else kill it. The drop path
    /// used by [`Drop`] (RISK-001 / MC-001). Idempotent: a no-transport client is a no-op.
    ///
    /// Graceful-then-forced: when a tokio runtime handle is available AND we are not already ON a
    /// runtime worker thread (so `block_on` is legal), it writes `shutdown`+`exit` and waits up to
    /// [`SHUTDOWN_TIMEOUT`] for a clean exit. Regardless of the graceful path, the child is force-killed
    /// at the end if still alive, and `kill_on_drop(true)` on the spawned `Command` is the final safety
    /// net so the OS process can never outlive the [`LspClient`] (RISK-001 — no zombie).
    fn shutdown_now(&mut self) {
        let runtime = self.runtime.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let transport = self.transport.lock().unwrap_or_else(|e| e.into_inner()).take();
        let Some(mut transport) = transport else { return };

        let graceful = |handle: &Handle, transport: &mut Transport| {
            handle.block_on(async {
                let shutdown =
                    serde_json::json!({"jsonrpc":"2.0","id":0,"method":"shutdown","params":null});
                let exit = serde_json::json!({"jsonrpc":"2.0","method":"exit","params":null});
                {
                    let mut stdin = transport.stdin.lock().await;
                    let _ = transport::write_message(&mut **stdin, &shutdown).await;
                    let _ = transport::write_message(&mut **stdin, &exit).await;
                }
                // Wait (bounded) for the server to exit on its own (a test transport has no child).
                if let Some(child) = transport.child.as_mut() {
                    let _ = tokio::time::timeout(SHUTDOWN_TIMEOUT, child.wait()).await;
                }
            });
        };

        // `block_on` panics if called from inside a runtime worker thread, so only take the graceful
        // path when we hold a handle AND are not on a runtime thread.
        if let Some(handle) = runtime {
            if Handle::try_current().is_err() {
                graceful(&handle, &mut transport);
            }
        }

        // Force-kill if still alive (the bounded graceful wait may have timed out, or no runtime was
        // available). `start_kill` is non-blocking + safe on an already-exited child; `kill_on_drop`
        // on the Command finishes the reap when `transport.child` drops here (RISK-001 — no zombie).
        // A test transport has no child (nothing to kill).
        if let Some(child) = transport.child.as_mut() {
            let _ = child.start_kill();
        }
    }
}

impl Drop for LspClient {
    /// RISK-001 / MC-001: on drop, send shutdown/exit (bounded) and kill the child, so closing the
    /// editor never leaks a language-server process.
    fn drop(&mut self) {
        self.shutdown_now();
    }
}

/// Build the `TextDocumentPositionParams` JSON for a request, or `None` if the uri is unparseable.
fn position_params(uri: &str, position: Position) -> Option<Value> {
    let url = Url::parse(uri).ok()?;
    let params = TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: url },
        position,
    };
    serde_json::to_value(params).ok()
}

/// Map an `lsp_types::CompletionItem` to our flattened popup item. `CompletionItemKind` is a
/// `#[serde(transparent)]` newtype over `i32`; round-trip it through serde to read the numeric kind
/// (the only API the crate exposes for the inner value).
fn completion_item_from_lsp(item: lsp_types::CompletionItem) -> LspCompletionItem {
    let insert_text = item.insert_text.clone().unwrap_or_else(|| item.label.clone());
    let kind = item
        .kind
        .and_then(|k| serde_json::to_value(k).ok())
        .and_then(|v| v.as_i64())
        .map(|n| n as i32);
    LspCompletionItem {
        label: item.label,
        insert_text,
        detail: item.detail,
        kind,
    }
}

/// Flatten `HoverContents` (scalar / array / markup) to a single display string (impl note 3).
fn hover_contents_to_string(contents: HoverContents) -> String {
    match contents {
        HoverContents::Scalar(s) => marked_string_to_text(s),
        HoverContents::Array(items) => items
            .into_iter()
            .map(marked_string_to_text)
            .collect::<Vec<_>>()
            .join("\n"),
        HoverContents::Markup(markup) => markup.value,
    }
}

/// Flatten a `MarkedString` (plain or language-tagged code) to display text.
fn marked_string_to_text(s: MarkedString) -> String {
    match s {
        MarkedString::String(text) => text,
        MarkedString::LanguageString(ls) => ls.value,
    }
}

/// Map a `PublishDiagnosticsParams` to the editor's [`PublishedDiagnostics`] (0-based gutter lines).
pub fn published_diagnostics_from_lsp(params: PublishDiagnosticsParams) -> PublishedDiagnostics {
    let diagnostics = params
        .diagnostics
        .into_iter()
        .map(|d| LspDiagnostic {
            line: d.range.start.line as usize,
            // An omitted severity is client-interpreted; treat it as Error (severity 1).
            severity: d.severity.map(severity_to_i32).unwrap_or(1),
            message: d.message,
        })
        .collect();
    PublishedDiagnostics { uri: params.uri.to_string(), diagnostics }
}

/// Map an `lsp_types::DiagnosticSeverity` to its i32 wire value (the crate stores it opaquely).
fn severity_to_i32(severity: lsp_types::DiagnosticSeverity) -> i32 {
    match severity {
        lsp_types::DiagnosticSeverity::ERROR => 1,
        lsp_types::DiagnosticSeverity::WARNING => 2,
        lsp_types::DiagnosticSeverity::INFORMATION => 3,
        lsp_types::DiagnosticSeverity::HINT => 4,
        _ => 1,
    }
}

/// TEST HELPER: read exactly one `Content-Length`-framed JSON-RPC message from `reader` and parse it,
/// or `None` on EOF / a malformed frame. Used by [`LspClient::read_test_request`] so a mock server can
/// observe the client's request before responding. Mirrors the production `read_loop` framing.
async fn read_one_frame<R>(reader: &mut R) -> Option<Value>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut buf = BufReader::new(reader);
    let mut content_length: Option<usize> = None;
    let mut header_line = String::new();
    loop {
        header_line.clear();
        match buf.read_line(&mut header_line).await {
            Ok(0) => return None, // EOF.
            Ok(_) => {}
            Err(_) => return None,
        }
        let trimmed = header_line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse::<usize>().ok();
        }
    }
    let len = content_length?;
    let mut body = vec![0u8; len];
    buf.read_exact(&mut body).await.ok()?;
    serde_json::from_slice(&body).ok()
}

/// The JSON-RPC-over-stdio framing transport. Kept in its own module so the framing (Content-Length
/// header + body) is testable in isolation from the process plumbing.
mod transport {
    use super::*;

    /// Serialize `message` and write it to the server's stdin with the LSP `Content-Length` frame
    /// header. Async (the request/notify paths await it). Generic over the writer so the production
    /// `ChildStdin` and a test duplex-pipe writer share the SAME framing path.
    pub async fn write_message<W>(stdin: &mut W, message: &Value) -> std::io::Result<()>
    where
        W: tokio::io::AsyncWrite + Unpin + ?Sized,
    {
        let body = serde_json::to_vec(message)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(&body).await?;
        stdin.flush().await
    }

    /// The reader loop: own the server's stdout, parse each `Content-Length`-framed JSON-RPC message,
    /// and route it. A message with an `id` + `result`/`error` fulfills the pending request; a message
    /// without an `id` is a notification (publishDiagnostics is routed to `diagnostics_tx`). A malformed
    /// frame / non-JSON body is SKIPPED, never panicked on (RISK-003).
    pub async fn read_loop<R>(
        stdout: R,
        pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>,
        diagnostics_tx: mpsc::UnboundedSender<PublishedDiagnostics>,
    ) where
        R: tokio::io::AsyncRead + Unpin,
    {
        let mut reader = BufReader::new(stdout);
        loop {
            // Read the header block (lines until a blank line), extracting Content-Length.
            let mut content_length: Option<usize> = None;
            let mut header_line = String::new();
            loop {
                header_line.clear();
                match reader.read_line(&mut header_line).await {
                    Ok(0) => return, // EOF: server closed stdout.
                    Ok(_) => {}
                    Err(_) => return, // unrecoverable read error.
                }
                let trimmed = header_line.trim_end_matches(['\r', '\n']);
                if trimmed.is_empty() {
                    break; // end of headers.
                }
                if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
                    content_length = rest.trim().parse::<usize>().ok();
                }
                // RISK-003: an unexpected non-header line (a server debug print) without a colon is
                // ignored; we keep reading until the blank line / a Content-Length appears.
            }
            let Some(len) = content_length else {
                // A header block ended with no Content-Length (e.g. a stray blank line from a server
                // debug print). Skip it and resync on the next header block (RISK-003 — never panic on
                // malformed framing).
                continue;
            };
            // Read exactly `len` bytes of body.
            let mut body = vec![0u8; len];
            if reader.read_exact(&mut body).await.is_err() {
                return;
            }
            // Parse the body; a non-JSON / malformed body is SKIPPED (RISK-003).
            let value: Value = match serde_json::from_slice(&body) {
                Ok(v) => v,
                Err(_) => continue,
            };
            route_message(value, &pending, &diagnostics_tx);
        }
    }

    /// Route one parsed JSON-RPC message to a pending request or a notification handler.
    fn route_message(
        value: Value,
        pending: &Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>,
        diagnostics_tx: &mpsc::UnboundedSender<PublishedDiagnostics>,
    ) {
        // A RESPONSE has an `id`. (Server-to-client REQUESTS also have an id + a `method`; we do not
        // implement server->client requests, so a message with both id+method is treated as a
        // notification-style message we ignore unless it is publishDiagnostics — which has no id.)
        let has_method = value.get("method").is_some();
        if let Some(id) = value.get("id").and_then(numeric_id) {
            if !has_method {
                // It's a response to one of our requests.
                if let Some(tx) = pending.lock().unwrap_or_else(|e| e.into_inner()).remove(&id) {
                    // Deliver the `result` (or, on an error, a Null so the request resolves to None).
                    let result = value.get("result").cloned().unwrap_or(Value::Null);
                    let _ = tx.send(result);
                }
                return;
            }
        }
        // A NOTIFICATION (no id, or id+method we ignore). Route publishDiagnostics.
        if value.get("method").and_then(|m| m.as_str()) == Some("textDocument/publishDiagnostics") {
            if let Some(params) = value.get("params").cloned() {
                if let Ok(parsed) =
                    serde_json::from_value::<PublishDiagnosticsParams>(params)
                {
                    let _ = diagnostics_tx.send(published_diagnostics_from_lsp(parsed));
                }
            }
        }
    }

    /// Parse a JSON-RPC id (number or numeric string) to an i64 (our ids are numeric).
    fn numeric_id(v: &Value) -> Option<i64> {
        v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_client_is_not_configured() {
        let client = LspClient::disabled();
        assert!(!client.is_configured());
        assert!(!client.is_running());
    }

    #[test]
    fn published_diagnostics_map_zero_based_lines_and_severity() {
        use lsp_types::{Diagnostic, DiagnosticSeverity, Range};
        let params = PublishDiagnosticsParams {
            uri: Url::parse("file:///x.rs").unwrap(),
            version: None,
            diagnostics: vec![
                Diagnostic {
                    range: Range {
                        start: Position { line: 4, character: 0 },
                        end: Position { line: 4, character: 3 },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: "boom".to_owned(),
                    ..Default::default()
                },
                Diagnostic {
                    range: Range {
                        start: Position { line: 9, character: 2 },
                        end: Position { line: 9, character: 8 },
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    message: "careful".to_owned(),
                    ..Default::default()
                },
            ],
        };
        let mapped = published_diagnostics_from_lsp(params);
        assert_eq!(mapped.uri, "file:///x.rs");
        assert_eq!(mapped.diagnostics.len(), 2);
        assert_eq!(mapped.diagnostics[0].line, 4);
        assert_eq!(mapped.diagnostics[0].severity, 1);
        assert_eq!(mapped.diagnostics[0].message, "boom");
        assert_eq!(mapped.diagnostics[1].line, 9);
        assert_eq!(mapped.diagnostics[1].severity, 2);
    }

    #[test]
    fn hover_contents_flatten_markup_and_scalar() {
        let markup = HoverContents::Markup(lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: "**add**\nAdds".to_owned(),
        });
        assert_eq!(hover_contents_to_string(markup), "**add**\nAdds");
        let scalar = HoverContents::Scalar(MarkedString::String("plain".to_owned()));
        assert_eq!(hover_contents_to_string(scalar), "plain");
    }

    #[tokio::test]
    async fn unconfigured_methods_degrade_gracefully() {
        // AC-004: every method returns empty/None without a server, without panicking.
        let client = LspClient::disabled();
        assert!(!client.initialize(None).await);
        client.did_open("file:///x.rs", "rust", "fn main() {}").await; // no panic
        client.did_change("file:///x.rs", 2, "fn main() {}").await; // no panic
        let pos = Position { line: 0, character: 0 };
        assert!(client.completion("file:///x.rs", pos).await.is_empty());
        assert!(client.hover("file:///x.rs", pos).await.is_none());
        assert!(client.goto_definition("file:///x.rs", pos).await.is_none());
        assert!(client.references("file:///x.rs", pos).await.is_empty());
        assert!(!client.is_running());
    }
}
