//! The MCP-style tool surface: a JSON-RPC 2.0 subset that maps four method names to the steering /
//! vision tools and validates a per-session token on every request.
//!
//! ## Tools
//!
//! | method         | params                                  | result                                            |
//! |----------------|-----------------------------------------|---------------------------------------------------|
//! | `list_widgets` | `{}`                                    | the MT-026 [`UiTreeSnapshot`] JSON                 |
//! | `click_widget` | `{ "target": "<author_id>" }`           | `{ "queued": true, "action": "Click", "node_id": N }` |
//! | `set_value`    | `{ "target": "<author_id>", "value": "…" }` | `{ "queued": true, "action": "Focus", "node_id": N }` (Focus + text — see [`super::action`]) |
//! | `screenshot`   | `{}`                                    | `{ png_base64, width, height, captured_at_utc }`  |
//!
//! `click_widget` / `set_value` ENQUEUE an action onto the [`ActionChannel`]; the egui frame loop (or
//! the live test) drains it and feeds it to egui the next frame. The result reports what was queued,
//! NOT the post-action UI state — a reader takes a fresh `list_widgets` after a frame to observe the
//! effect (the contract's "one frame latency" note; the live test advances a frame between the two).
//!
//! ## Transport independence
//!
//! [`dispatch_request`] consumes an already-parsed [`McpRequest`] and returns an [`McpResponse`]; it
//! never touches a socket. The same function serves the in-process API proven here AND a future
//! `tokio` TCP/named-pipe transport that just newline-frames JSON on the way in/out. This is why the
//! steering semantics can be proven headlessly today without committing to a transport.
//!
//! ## Session token (per-session HMAC, constant-time compare)
//!
//! The contract mandates a per-session HMAC token validated by constant-time compare via `hmac` +
//! `sha2`. [`SessionToken`] holds a 32-byte secret generated from the OS CSPRNG (`rand::rngs::OsRng`)
//! and exposed as 64 lowercase hex chars (written into the binding file, presented by the client in
//! every request's top-level `session_token` field). [`SessionToken::matches`] validates the presented
//! token by computing `HMAC-SHA256(stored_secret, presented_bytes)` and `HMAC-SHA256(stored_secret,
//! stored_bytes)` and comparing the two tags with `hmac`'s constant-time `verify_slice` — so the
//! comparison time does not leak how many leading bytes of the token matched (red-team: token-compare
//! timing side channel). A request missing or mismatching `session_token` is rejected with `-32001`.

use egui::accesskit;

use crate::accessibility::{is_sensitive_author_id, UiTreeSnapshot};
use crate::mcp::action::{ActionChannel, ActionError, UiAction};
use crate::mcp::argus::{
    validate_agent_label, ArgusError, ArgusWindowDescriptor, WindowSnapshotRegistry, MAIN_WINDOW_ID,
};
use crate::mcp::screenshot::{ScreenshotError, ScreenshotResult};

/// JSON-RPC error: the `session_token` was missing or did not match (red-team: unauthorized caller).
pub const ERR_UNAUTHORIZED: i64 = -32001;
/// JSON-RPC error: the bounded action queue is full (back-pressure).
pub const ERR_ACTION_QUEUE_FULL: i64 = -32002;
/// JSON-RPC error: the caller exceeded the action rate limit (reserved for the transport MT's
/// per-connection token bucket; the in-process channel enforces a per-frame burst cap instead).
pub const ERR_RATE_LIMITED: i64 = -32003;
/// JSON-RPC standard error: the method name is not one of the four tools.
pub const ERR_METHOD_NOT_FOUND: i64 = -32601;
/// JSON-RPC standard error: params were missing or malformed for the method.
pub const ERR_INVALID_PARAMS: i64 = -32602;
/// JSON-RPC error: the tool ran but failed (e.g. unknown/disabled target, screenshot capture error).
pub const ERR_TOOL_FAILED: i64 = -32000;
/// JSON-RPC error (MT-028): an exclusive/shared lease on the target resource could not be acquired
/// within the lease timeout because a concurrent agent held it. The caller should retry. The code
/// `-32004` matches the MT-028 contract's `{error:{code:-32004,message:"Lease timeout"}}` acceptance.
pub const ERR_LEASE_TIMEOUT: i64 = -32004;
/// JSON-RPC error: the addressed live window/revision/target is unknown, stale, or ambiguous.
pub const ERR_ARGUS_CONFLICT: i64 = -32005;
const AGENT_LABEL_CONTEXT_KEY: &str = "__argus_agent_label";
const AGENT_CREDENTIAL_CONTEXT_KEY: &str = "__argus_agent_credential";

/// A per-session HMAC secret a caller must present (as 64 hex chars) in every request's
/// `session_token` field. The secret bytes are the HMAC-SHA256 KEY; validation HMACs both the stored
/// and presented tokens under that key and constant-time compares the tags (see module docs).
#[derive(Clone)]
pub struct SessionToken {
    /// The 32 secret bytes (the HMAC key) rendered as 64 lowercase hex chars for transport/discovery.
    hex: String,
    /// The raw secret bytes used as the HMAC key for constant-time validation.
    key: [u8; 32],
}

// Custom Debug so the secret never leaks into logs/panics (red-team: token exfiltration via Debug).
impl std::fmt::Debug for SessionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionToken")
            .field("hex", &"<redacted>")
            .finish()
    }
}

impl PartialEq for SessionToken {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for SessionToken {}

impl SessionToken {
    /// Wrap an existing 64-hex-char token (e.g. a test fixture, or a value read back from the binding
    /// file). Non-hex / wrong-length input is hashed into a 32-byte key so the type is still usable as a
    /// shared secret in tests that pass short strings; production tokens are always 64 hex chars via
    /// [`Self::generate`].
    pub fn from_hex(hex: impl Into<String>) -> Self {
        let hex = hex.into();
        let key = key_from_hex_or_hash(&hex);
        Self { hex, key }
    }

    /// Generate a 32-byte token from the OS CSPRNG (`rand::rngs::OsRng`), rendered as 64 lowercase hex
    /// chars. 256 bits from a cryptographic RNG makes blind guessing infeasible; the bytes double as the
    /// HMAC key used for constant-time validation.
    pub fn generate() -> Self {
        use rand::TryRngCore;
        let mut key = [0u8; 32];
        // `OsRng` is the OS CSPRNG; `try_fill_bytes` is rand 0.9's fallible fill. A failure here means
        // the OS RNG is unavailable, which is catastrophic for a security token — fail loudly.
        rand::rngs::OsRng
            .try_fill_bytes(&mut key)
            .expect("OS CSPRNG available for session token");
        let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
        Self { hex, key }
    }

    /// The token's hex string (written into the discovery/binding artifact, presented by the client).
    pub fn as_hex(&self) -> &str {
        &self.hex
    }

    /// Constant-time validation of a presented token. Computes `HMAC-SHA256(key, presented)` and
    /// `HMAC-SHA256(key, stored_hex)` and compares the tags via `hmac`'s `verify_slice`, whose
    /// comparison is constant-time — so timing does not leak how many leading bytes matched (red-team:
    /// token-compare timing side channel). An empty/short/long presented token simply produces a
    /// different tag and is rejected.
    pub fn matches(&self, presented: &str) -> bool {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        // Tag of the stored canonical token (what an authorized client must reproduce by presenting the
        // same hex). Keyed by the secret so the tag itself is not guessable from the public hex alone.
        let mut expected = HmacSha256::new_from_slice(&self.key).expect("hmac accepts 32-byte key");
        expected.update(self.hex.as_bytes());
        let expected_tag = expected.finalize().into_bytes();

        // Tag of the presented token under the same key.
        let mut presented_mac =
            HmacSha256::new_from_slice(&self.key).expect("hmac accepts 32-byte key");
        presented_mac.update(presented.as_bytes());
        // `verify_slice` is the constant-time compare; it consumes the computed tag and checks it against
        // the expected tag bytes without early-out on the first differing byte.
        presented_mac.verify_slice(&expected_tag).is_ok()
    }
}

/// Decode 64 hex chars into a 32-byte HMAC key; if the input is not exactly 64 hex chars, derive a
/// stable 32-byte key by SHA-256 hashing the raw string (used only for non-production test fixtures).
fn key_from_hex_or_hash(hex: &str) -> [u8; 32] {
    if hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        let mut key = [0u8; 32];
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap_or(0);
        }
        key
    } else {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(hex.as_bytes());
        let mut key = [0u8; 32];
        key.copy_from_slice(&digest);
        key
    }
}

/// A parsed JSON-RPC request for one tool call. `session_token` is a top-level field (per the
/// contract), NOT inside `params`.
#[derive(Debug, Clone)]
pub struct McpRequest {
    /// JSON-RPC id echoed back in the response (number or string; kept as the raw JSON value).
    pub id: serde_json::Value,
    /// The tool name (`list_widgets` / `click_widget` / `set_value` / `screenshot`).
    pub method: String,
    /// The method params object (`{}` for the no-arg tools).
    pub params: serde_json::Value,
    /// The presented per-session token.
    pub session_token: String,
}

impl McpRequest {
    /// Parse a JSON-RPC request from a raw JSON value, validating the `jsonrpc` version and required
    /// fields. Returns an [`McpToolError`] (mapped to `-32600`/`-32602`) on a malformed envelope so a
    /// transport can reply with a well-formed error rather than dropping the connection.
    pub fn from_json(value: &serde_json::Value) -> Result<Self, McpToolError> {
        let obj = value.as_object().ok_or_else(|| {
            McpToolError::new(ERR_INVALID_PARAMS, "request must be a JSON object")
        })?;
        if obj.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
            return Err(McpToolError::new(
                ERR_INVALID_PARAMS,
                "jsonrpc must be \"2.0\"",
            ));
        }
        let method = obj
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpToolError::new(ERR_INVALID_PARAMS, "missing method"))?
            .to_owned();
        let id = obj.get("id").cloned().unwrap_or(serde_json::Value::Null);
        let mut params = obj.get("params").cloned().unwrap_or(serde_json::json!({}));
        let session_token = obj
            .get("session_token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        let agent_label = obj
            .get("agent_label")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        let agent_credential = obj
            .get("agent_token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        if !params.is_object() {
            params = serde_json::json!({});
        }
        params.as_object_mut().expect("params normalized").insert(
            AGENT_LABEL_CONTEXT_KEY.to_owned(),
            serde_json::Value::String(agent_label),
        );
        params.as_object_mut().expect("params normalized").insert(
            AGENT_CREDENTIAL_CONTEXT_KEY.to_owned(),
            serde_json::Value::String(agent_credential),
        );
        Ok(Self {
            id,
            method,
            params,
            session_token,
        })
    }

    /// Bounded attribution label parsed from the top-level request envelope. It is kept in a
    /// reserved transport-context slot so the long-standing public request struct remains source
    /// compatible with in-process callers; it is never used as authentication.
    pub fn agent_label(&self) -> &str {
        self.params
            .get(AGENT_LABEL_CONTEXT_KEY)
            .and_then(|value| value.as_str())
            .unwrap_or("")
    }

    /// Broker-minted credential proving the stable agent principal. This is
    /// deliberately distinct from the caller-controlled display label.
    pub fn agent_credential(&self) -> &str {
        self.params
            .get(AGENT_CREDENTIAL_CONTEXT_KEY)
            .and_then(|value| value.as_str())
            .unwrap_or("")
    }
}

/// A JSON-RPC response: either a `result` value or an `error`. Serializes to the standard envelope.
#[derive(Debug, Clone)]
pub struct McpResponse {
    pub id: serde_json::Value,
    pub payload: Result<serde_json::Value, McpError>,
}

impl McpResponse {
    fn ok(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            id,
            payload: Ok(result),
        }
    }

    fn err(id: serde_json::Value, error: McpError) -> Self {
        Self {
            id,
            payload: Err(error),
        }
    }

    /// Public constructor for an error response (MT-028): the [`crate::mcp::session::McpSession`] wrapper
    /// builds a lease-timeout response without going through [`dispatch_request`]. Same shape as the
    /// internal [`Self::err`].
    pub fn error(id: serde_json::Value, error: McpError) -> Self {
        Self {
            id,
            payload: Err(error),
        }
    }

    /// Public constructor for a success response (MT-028): the [`crate::mcp::session::McpSession`] wrapper
    /// rebuilds a mutating result Value to add the acting `agent_id` (AC#2) after a successful enqueue.
    /// Same shape as the internal [`Self::ok`].
    pub fn ok_value(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            id,
            payload: Ok(result),
        }
    }

    /// Borrow the success `result` value, or the error (MT-028): the session wrapper inspects a
    /// successful enqueue's `{queued, node_id}` to decide whether to append an attribution entry, without
    /// re-serializing to JSON.
    pub fn result_ref(&self) -> Result<&serde_json::Value, &McpError> {
        self.payload.as_ref()
    }

    /// Serialize to the JSON-RPC 2.0 response envelope a transport writes back.
    pub fn to_json(&self) -> serde_json::Value {
        match &self.payload {
            Ok(result) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": self.id,
                "result": result,
            }),
            Err(error) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": self.id,
                "error": { "code": error.code, "message": error.message },
            }),
        }
    }

    /// Convenience: true when this response carries an error with the given code.
    pub fn is_error_code(&self, code: i64) -> bool {
        matches!(&self.payload, Err(e) if e.code == code)
    }
}

/// The JSON-RPC `error` object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpError {
    pub code: i64,
    pub message: String,
}

/// An error raised while parsing/handling a tool call, before a response id is necessarily known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolError {
    pub code: i64,
    pub message: String,
}

impl McpToolError {
    fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<ActionError> for McpError {
    fn from(e: ActionError) -> Self {
        let code = match e {
            ActionError::QueueFull => ERR_ACTION_QUEUE_FULL,
            _ => ERR_TOOL_FAILED,
        };
        McpError {
            code,
            message: e.to_string(),
        }
    }
}

impl From<ScreenshotError> for McpError {
    fn from(e: ScreenshotError) -> Self {
        McpError {
            code: ERR_TOOL_FAILED,
            message: e.to_string(),
        }
    }
}

impl From<ArgusError> for McpError {
    fn from(e: ArgusError) -> Self {
        let code = if matches!(e, ArgusError::InvalidAgentLabel) {
            ERR_INVALID_PARAMS
        } else {
            ERR_ARGUS_CONFLICT
        };
        Self {
            code,
            message: e.to_string(),
        }
    }
}

/// Canonicalize compatibility aliases at the request boundary. All aliases execute the same code.
pub fn canonical_method(method: &str) -> Option<&'static str> {
    match method {
        "argus.list_windows" => Some("argus.list_windows"),
        "argus.inspect" | "list_widgets" => Some("argus.inspect"),
        "argus.click" | "click_widget" => Some("argus.click"),
        "argus.show_context_menu" => Some("argus.show_context_menu"),
        "argus.set_value" | "set_value" => Some("argus.set_value"),
        "argus.screenshot" | "screenshot" => Some("argus.screenshot"),
        _ => None,
    }
}

/// Dispatch a parsed JSON-RPC request to the right tool.
///
/// - `token`: the session's secret; the request's `session_token` is checked against it FIRST (a bad
///   token never reaches a tool — red-team: unauthorized caller cannot enumerate or steer).
/// - `snapshot`: a current-frame [`UiTreeSnapshot`] (the READ surface). `list_widgets` returns it;
///   `click_widget`/`set_value` resolve the target against it.
/// - `channel`: the action queue `click_widget`/`set_value` enqueue onto.
/// - `capture`: a closure that produces a [`ScreenshotResult`] (the live test wires `Harness::render()`
///   + PNG encode). Taken as a closure so this dispatch stays transport- AND renderer-agnostic.
///
/// Returns an [`McpResponse`] (never panics): every failure path is a typed JSON-RPC error.
pub fn dispatch_request(
    request: &McpRequest,
    token: &SessionToken,
    snapshot: &UiTreeSnapshot,
    channel: &mut ActionChannel,
    capture: impl FnOnce() -> Result<ScreenshotResult, ScreenshotError>,
) -> McpResponse {
    let windows = WindowSnapshotRegistry::new();
    windows.publish(
        ArgusWindowDescriptor {
            window_id: MAIN_WINDOW_ID.to_owned(),
            viewport_id: "ROOT".to_owned(),
            title: crate::mcp::screenshot::HANDSHAKE_WINDOW_TITLE.to_owned(),
        },
        snapshot.clone(),
    );
    let mut compatible = request.clone();
    if !compatible.params.is_object() {
        compatible.params = serde_json::json!({});
    }
    let params = compatible
        .params
        .as_object_mut()
        .expect("params normalized");
    if params
        .get(AGENT_LABEL_CONTEXT_KEY)
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .is_empty()
    {
        params.insert(
            AGENT_LABEL_CONTEXT_KEY.to_owned(),
            serde_json::Value::String("legacy-client".to_owned()),
        );
    }
    params
        .entry("window_id")
        .or_insert_with(|| serde_json::json!(MAIN_WINDOW_ID));
    params
        .entry("expected_snapshot_revision")
        .or_insert_with(|| serde_json::json!(1));
    dispatch_windowed_request(
        &compatible,
        token,
        &windows,
        channel,
        "legacy-in-process",
        "legacy-authenticated-session",
        |_| capture(),
    )
}

/// Dispatch through the canonical window-aware Argus implementation used by the live transport.
pub fn dispatch_windowed_request(
    request: &McpRequest,
    token: &SessionToken,
    windows: &WindowSnapshotRegistry,
    channel: &mut ActionChannel,
    connection_id: &str,
    authenticated_agent_id: &str,
    capture: impl FnOnce(&ArgusWindowDescriptor) -> Result<ScreenshotResult, ScreenshotError>,
) -> McpResponse {
    if !token.matches(&request.session_token) {
        return McpResponse::err(
            request.id.clone(),
            McpError {
                code: ERR_UNAUTHORIZED,
                message: "Unauthorized".to_owned(),
            },
        );
    }
    if let Err(error) = validate_agent_label(request.agent_label()) {
        return McpResponse::err(request.id.clone(), error.into());
    }
    let Some(method) = canonical_method(&request.method) else {
        return McpResponse::err(
            request.id.clone(),
            McpError {
                code: ERR_METHOD_NOT_FOUND,
                message: format!("unknown method '{}'", request.method),
            },
        );
    };
    if method == "argus.list_windows" {
        return McpResponse::ok(
            request.id.clone(),
            serde_json::json!({ "windows": windows.list() }),
        );
    }
    let window_id = match parse_window_id(&request.params) {
        Ok(value) => value,
        Err(error) => return tool_error_response(request, error),
    };
    match method {
        "argus.inspect" => match windows.get(&window_id) {
            Ok(window) => McpResponse::ok(
                request.id.clone(),
                serde_json::to_value(window)
                    .unwrap_or_else(|_| serde_json::json!({"error": "snapshot serialize failed"})),
            ),
            Err(error) => McpResponse::err(request.id.clone(), error.into()),
        },
        "argus.click" => {
            let target = match parse_target(&request.params) {
                Ok(value) => value,
                Err(error) => return tool_error_response(request, error),
            };
            dispatch_argus_mutation(
                request,
                windows,
                channel,
                connection_id,
                authenticated_agent_id,
                &window_id,
                &target,
                UiAction::Click,
            )
        }
        "argus.show_context_menu" => {
            let target = match parse_target(&request.params) {
                Ok(value) => value,
                Err(error) => return tool_error_response(request, error),
            };
            dispatch_argus_mutation(
                request,
                windows,
                channel,
                connection_id,
                authenticated_agent_id,
                &window_id,
                &target,
                UiAction::ShowContextMenu,
            )
        }
        "argus.set_value" => {
            let target = match parse_target(&request.params) {
                Ok(value) => value,
                Err(error) => return tool_error_response(request, error),
            };
            // Reject before reading or cloning `params.value`: generic Argus transport and action
            // queues are not secret-bearing boundaries. BYOK keys use the dedicated keychain route.
            if is_sensitive_author_id(&target) {
                return tool_error_response(
                    request,
                    McpToolError::new(
                        ERR_INVALID_PARAMS,
                        "argus.set_value is prohibited for secret-bearing inputs; use the dedicated credential workflow",
                    ),
                );
            }
            let value = match parse_value(&request.params) {
                Ok(value) => value,
                Err(error) => return tool_error_response(request, error),
            };
            dispatch_argus_mutation(
                request,
                windows,
                channel,
                connection_id,
                authenticated_agent_id,
                &window_id,
                &target,
                UiAction::SetValue { text: value },
            )
        }
        "argus.screenshot" => {
            let descriptor = match windows.descriptor(&window_id) {
                Ok(value) => value,
                Err(error) => return McpResponse::err(request.id.clone(), error.into()),
            };
            match capture(&descriptor) {
                Ok(shot) => McpResponse::ok(request.id.clone(), shot.to_json()),
                Err(error) => McpResponse::err(request.id.clone(), error.into()),
            }
        }
        _ => unreachable!("canonical method is closed"),
    }
}

fn dispatch_argus_mutation(
    request: &McpRequest,
    windows: &WindowSnapshotRegistry,
    channel: &mut ActionChannel,
    connection_id: &str,
    authenticated_agent_id: &str,
    window_id: &str,
    target: &str,
    action: UiAction,
) -> McpResponse {
    let expected_revision = match parse_expected_revision(&request.params) {
        Ok(value) => value,
        Err(error) => return tool_error_response(request, error),
    };
    let window = match windows.validate_target(window_id, target, expected_revision) {
        Ok(value) => value,
        Err(error) => return McpResponse::err(request.id.clone(), error.into()),
    };
    let action_name = format!("{:?}", action.accesskit_action());
    match channel.enqueue_argus(
        &window.snapshot,
        window_id,
        target,
        action,
        connection_id,
        authenticated_agent_id,
        expected_revision,
    ) {
        Ok((outcome, receipt)) => McpResponse::ok(
            request.id.clone(),
            serde_json::json!({
                "queued": true,
                "action": action_name,
                "node_id": node_id_u64(&outcome.request.target),
                "target": target,
                "author_id": target,
                "action_id": receipt.action_id,
                "window_id": window_id,
                "before_revision": expected_revision,
            }),
        ),
        Err(error) => McpResponse::err(request.id.clone(), error.into()),
    }
}

fn tool_error_response(request: &McpRequest, error: McpToolError) -> McpResponse {
    McpResponse::err(
        request.id.clone(),
        McpError {
            code: error.code,
            message: error.message,
        },
    )
}

fn parse_window_id(params: &serde_json::Value) -> Result<String, McpToolError> {
    params
        .get("window_id")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| McpToolError::new(ERR_INVALID_PARAMS, "params.window_id required"))
}

fn parse_expected_revision(params: &serde_json::Value) -> Result<u64, McpToolError> {
    params
        .get("expected_snapshot_revision")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            McpToolError::new(
                ERR_INVALID_PARAMS,
                "params.expected_snapshot_revision (u64) required",
            )
        })
}

/// AccessKit `NodeId` is a newtype over u64; pull the inner value for the JSON result.
fn node_id_u64(id: &accesskit::NodeId) -> u64 {
    id.0
}

/// Parse the `target` author_id from a tool's params object.
fn parse_target(params: &serde_json::Value) -> Result<String, McpToolError> {
    params
        .get("author_id")
        .or_else(|| params.get("target"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .ok_or_else(|| {
            McpToolError::new(
                ERR_INVALID_PARAMS,
                "params.author_id (or legacy params.target) string required",
            )
        })
}

/// Parse the non-secret value for `set_value`. Sensitive targets are rejected before this function
/// is called so their payload never enters the generic Argus action queue.
fn parse_value(params: &serde_json::Value) -> Result<String, McpToolError> {
    params
        .get("value")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned())
        .ok_or_else(|| McpToolError::new(ERR_INVALID_PARAMS, "params.value (string) required"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::{UiTreeNode, UiTreeSnapshot};
    use crate::mcp::screenshot::screenshot_from_png;

    fn snap() -> UiTreeSnapshot {
        let button = UiTreeNode {
            id: "btn".to_owned(),
            author_id: Some("btn".to_owned()),
            node_id: 10,
            role: "Button".to_owned(),
            label: Some("Go".to_owned()),
            value: None,
            disabled: false,
            actions: vec![
                "Click".to_owned(),
                "Focus".to_owned(),
                "ShowContextMenu".to_owned(),
            ],
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

    fn req(method: &str, params: serde_json::Value, token: &str) -> McpRequest {
        McpRequest {
            id: serde_json::json!(1),
            method: method.to_owned(),
            params,
            session_token: token.to_owned(),
        }
    }

    fn ok_capture() -> Result<ScreenshotResult, ScreenshotError> {
        Ok(screenshot_from_png(b"foobar", 4, 3))
    }

    #[test]
    fn constant_time_token_matches_and_rejects() {
        let t = SessionToken::from_hex("deadbeef");
        assert!(t.matches("deadbeef"));
        assert!(!t.matches("deadbee0"));
        assert!(!t.matches("deadbee")); // too short
        assert!(!t.matches("deadbeeff")); // too long
        assert!(!t.matches(""));
    }

    #[test]
    fn generated_token_is_64_hex_chars_and_self_matches() {
        let t = SessionToken::generate();
        assert_eq!(t.as_hex().len(), 64);
        assert!(t.as_hex().bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(t.matches(t.as_hex()));
    }

    #[test]
    fn unauthorized_request_is_rejected_with_minus_32001() {
        let token = SessionToken::from_hex("secret");
        let mut chan = ActionChannel::new();
        let r = dispatch_request(
            &req("list_widgets", serde_json::json!({}), "wrong"),
            &token,
            &snap(),
            &mut chan,
            ok_capture,
        );
        assert!(r.is_error_code(ERR_UNAUTHORIZED));
        let v = r.to_json();
        assert_eq!(v["error"]["code"], -32001);
        assert_eq!(v["error"]["message"], "Unauthorized");
    }

    #[test]
    fn list_widgets_returns_snapshot_json() {
        let token = SessionToken::from_hex("secret");
        let mut chan = ActionChannel::new();
        let r = dispatch_request(
            &req("list_widgets", serde_json::json!({}), "secret"),
            &token,
            &snap(),
            &mut chan,
            ok_capture,
        );
        let v = r.to_json();
        assert_eq!(v["result"]["snapshot"]["widget_count"], 2);
        assert_eq!(v["result"]["snapshot"]["root"]["role"], "Window");
        assert_eq!(v["result"]["window_id"], MAIN_WINDOW_ID);
        assert_eq!(v["result"]["revision"], 1);
    }

    #[test]
    fn argus_list_windows_requires_auth_and_no_window_id() {
        let token = SessionToken::from_hex("secret");
        let windows = WindowSnapshotRegistry::new();
        windows.publish(
            ArgusWindowDescriptor {
                window_id: MAIN_WINDOW_ID.to_owned(),
                viewport_id: "ROOT".to_owned(),
                title: "Handshake".to_owned(),
            },
            snap(),
        );
        windows.register(ArgusWindowDescriptor {
            window_id: "popout-pane-a".to_owned(),
            viewport_id: "PANE_A".to_owned(),
            title: "Handshake – Workspace".to_owned(),
        });
        let request = McpRequest::from_json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 41,
            "method": "argus.list_windows",
            "params": {},
            "session_token": "secret",
            "agent_label": "window-list-test"
        }))
        .unwrap();
        let mut channel = ActionChannel::new();
        let response = dispatch_windowed_request(
            &request,
            &token,
            &windows,
            &mut channel,
            "connection-test",
            "agent-test",
            |_| ok_capture(),
        )
        .to_json();
        assert_eq!(response["result"]["windows"][0]["window_id"], "main");
        assert_eq!(
            response["result"]["windows"][1]["window_id"],
            "popout-pane-a"
        );
        assert_eq!(
            response["result"]["windows"][1]["snapshot_available"],
            false
        );

        let unauthorized = McpRequest::from_json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "argus.list_windows",
            "params": {},
            "session_token": "wrong",
            "agent_label": "window-list-test"
        }))
        .unwrap();
        assert!(dispatch_windowed_request(
            &unauthorized,
            &token,
            &windows,
            &mut channel,
            "connection-test",
            "agent-test",
            |_| ok_capture(),
        )
        .is_error_code(ERR_UNAUTHORIZED));
    }

    #[test]
    fn argus_show_context_menu_enqueues_accesskit_action() {
        let token = SessionToken::from_hex("secret");
        let windows = WindowSnapshotRegistry::new();
        windows.publish(
            ArgusWindowDescriptor {
                window_id: MAIN_WINDOW_ID.to_owned(),
                viewport_id: "ROOT".to_owned(),
                title: "Handshake".to_owned(),
            },
            snap(),
        );
        let request = McpRequest::from_json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 43,
            "method": "argus.show_context_menu",
            "params": {
                "window_id": "main",
                "author_id": "btn",
                "expected_snapshot_revision": 1
            },
            "session_token": "secret",
            "agent_label": "context-menu-test"
        }))
        .unwrap();
        let mut channel = ActionChannel::new();
        let response = dispatch_windowed_request(
            &request,
            &token,
            &windows,
            &mut channel,
            "connection-test",
            "agent-test",
            |_| ok_capture(),
        )
        .to_json();
        assert_eq!(response["result"]["queued"], true);
        assert_eq!(response["result"]["action"], "ShowContextMenu");
        let events = channel.drain_for_window(MAIN_WINDOW_ID).events;
        assert!(matches!(
            events.as_slice(),
            [egui::Event::AccessKitActionRequest(request)]
                if request.action == accesskit::Action::ShowContextMenu
        ));
    }

    #[test]
    fn click_widget_enqueues_and_reports_node_id() {
        let token = SessionToken::from_hex("secret");
        let mut chan = ActionChannel::new();
        let r = dispatch_request(
            &req(
                "click_widget",
                serde_json::json!({"target": "btn"}),
                "secret",
            ),
            &token,
            &snap(),
            &mut chan,
            ok_capture,
        );
        let v = r.to_json();
        assert_eq!(v["result"]["queued"], true);
        assert_eq!(v["result"]["action"], "Click");
        assert_eq!(v["result"]["node_id"], 10);
        assert_eq!(chan.pending(), 1);
    }

    #[test]
    fn click_unknown_target_is_tool_failure() {
        let token = SessionToken::from_hex("secret");
        let mut chan = ActionChannel::new();
        let r = dispatch_request(
            &req(
                "click_widget",
                serde_json::json!({"target": "ghost"}),
                "secret",
            ),
            &token,
            &snap(),
            &mut chan,
            ok_capture,
        );
        assert!(r.is_error_code(ERR_TOOL_FAILED));
    }

    #[test]
    fn set_value_requires_value_param() {
        let token = SessionToken::from_hex("secret");
        let mut chan = ActionChannel::new();
        let r = dispatch_request(
            &req("set_value", serde_json::json!({"target": "btn"}), "secret"),
            &token,
            &snap(),
            &mut chan,
            ok_capture,
        );
        assert!(r.is_error_code(ERR_INVALID_PARAMS));
    }

    #[test]
    fn set_value_rejects_secret_target_without_enqueuing_or_echoing_canary() {
        let token = SessionToken::from_hex("secret");
        let mut channel = ActionChannel::new();
        let canary = "argus-set-value-secret-canary";
        let response = dispatch_request(
            &req(
                "argus.set_value",
                serde_json::json!({
                    "author_id": "settings.cloud.byok.openai.key",
                    "value": canary
                }),
                "secret",
            ),
            &token,
            &snap(),
            &mut channel,
            ok_capture,
        );

        assert!(response.is_error_code(ERR_INVALID_PARAMS));
        assert_eq!(channel.pending(), 0);
        assert!(!response.to_json().to_string().contains(canary));
    }

    #[test]
    fn screenshot_returns_visual_capture_shape() {
        let token = SessionToken::from_hex("secret");
        let mut chan = ActionChannel::new();
        let r = dispatch_request(
            &req("screenshot", serde_json::json!({}), "secret"),
            &token,
            &snap(),
            &mut chan,
            ok_capture,
        );
        let v = r.to_json();
        assert_eq!(v["result"]["png_base64"], "Zm9vYmFy");
        assert_eq!(v["result"]["width"], 4);
        assert_eq!(v["result"]["height"], 3);
        assert_eq!(v["result"]["sha256"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn unknown_method_is_minus_32601() {
        let token = SessionToken::from_hex("secret");
        let mut chan = ActionChannel::new();
        let r = dispatch_request(
            &req("nope", serde_json::json!({}), "secret"),
            &token,
            &snap(),
            &mut chan,
            ok_capture,
        );
        assert!(r.is_error_code(ERR_METHOD_NOT_FOUND));
    }

    #[test]
    fn request_envelope_parses_from_json() {
        let raw = serde_json::json!({
            "jsonrpc": "2.0", "id": 7, "method": "click_widget",
            "params": {"target": "btn"}, "session_token": "secret",
            "agent_label": "parser-test"
        });
        let parsed = McpRequest::from_json(&raw).expect("valid envelope");
        assert_eq!(parsed.method, "click_widget");
        assert_eq!(parsed.session_token, "secret");
        assert_eq!(parsed.id, serde_json::json!(7));
        assert_eq!(parsed.agent_label(), "parser-test");
    }

    #[test]
    fn canonical_and_legacy_names_resolve_to_one_implementation() {
        assert_eq!(
            canonical_method("argus.inspect"),
            canonical_method("list_widgets")
        );
        assert_eq!(
            canonical_method("argus.click"),
            canonical_method("click_widget")
        );
        assert_eq!(
            canonical_method("argus.list_windows"),
            Some("argus.list_windows")
        );
        assert_eq!(
            canonical_method("argus.show_context_menu"),
            Some("argus.show_context_menu")
        );
        assert_eq!(
            canonical_method("argus.set_value"),
            canonical_method("set_value")
        );
        assert_eq!(
            canonical_method("argus.screenshot"),
            canonical_method("screenshot")
        );
    }

    #[test]
    fn bad_jsonrpc_version_is_rejected() {
        let raw = serde_json::json!({ "jsonrpc": "1.0", "method": "x", "id": 1 });
        assert!(McpRequest::from_json(&raw).is_err());
    }
}
