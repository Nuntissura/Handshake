//! The Argus/MCP tool surface: a JSON-RPC 2.0 subset that maps product-facing Argus methods to the
//! native steering/vision primitives and validates a per-session token on every request.
//!
//! ## Tools
//!
//! | method              | params                                      | result                                            |
//! |---------------------|---------------------------------------------|---------------------------------------------------|
//! | `argus.inspect`    | `{}`                                        | the MT-026 [`UiTreeSnapshot`] JSON                 |
//! | `argus.click`      | `{ "target": "<author_id>" }`               | `{ "queued": true, "action": "Click", "node_id": N }` |
//! | `argus.set_value`  | `{ "target": "<TextInput author_id>", "value": "…" }` | `{ "queued": true, "action": "Focus", "node_id": N }` (TextInput only; Focus + select-all + text — see [`super::action`]) |
//! | `argus.screenshot` | `{}`                                        | `{ png_base64, width, height, captured_at_utc }`  |
//!
//! `list_widgets`, `click_widget`, `set_value`, and `screenshot` remain compatibility aliases for older
//! clients. New model/operator workflows should use the Argus names and include optional top-level
//! `agent_label` when multiple parallel clients share the live binding token.
//!
//! `argus.click` / `argus.set_value` ENQUEUE an action onto the [`ActionChannel`]; the egui frame loop
//! (or the live test) drains it and feeds it to egui the next frame. The result reports what was queued,
//! NOT the post-action UI state — a reader takes a fresh `argus.inspect` after a frame to observe the
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

use crate::accessibility::UiTreeSnapshot;
use crate::mcp::action::{ActionChannel, ActionError, UiAction};
use crate::mcp::argus::{self, ArgusRoute};
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
    /// The tool name (`argus.inspect` / `argus.click` / `argus.set_value` / `argus.screenshot`; older
    /// primitive names are compatibility aliases).
    pub method: String,
    /// The method params object (`{}` for the no-arg tools).
    pub params: serde_json::Value,
    /// The presented per-session token.
    pub session_token: String,
    /// Optional model/operator attribution label. This is not an auth credential; the session token is
    /// still the authorization gate. When present, receipts derive a distinct `agent_id` from
    /// `session_token + agent_label` so multiple clients sharing the live binding token remain
    /// attributable.
    pub agent_label: Option<String>,
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
        let params = obj.get("params").cloned().unwrap_or(serde_json::json!({}));
        let session_token = obj
            .get("session_token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        let agent_label = obj
            .get("agent_label")
            .or_else(|| obj.get("agent_id"))
            .and_then(|v| v.as_str())
            .and_then(normalize_agent_label);
        Ok(Self {
            id,
            method,
            params,
            session_token,
            agent_label,
        })
    }
}

/// Normalize an authenticated client-supplied attribution label. It is deliberately permissive enough
/// for model names and worker ids, bounded so it cannot bloat logs, and never treated as authorization.
pub fn normalize_agent_label(label: &str) -> Option<String> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut normalized = String::with_capacity(trimmed.len().min(64));
    for c in trimmed.chars() {
        if normalized.len() >= 64 {
            break;
        }
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '@' | ':') {
            normalized.push(c);
        } else if c.is_whitespace() {
            normalized.push('-');
        }
    }
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
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

/// Dispatch a parsed JSON-RPC request to the right tool.
///
/// - `token`: the session's secret; the request's `session_token` is checked against it FIRST (a bad
///   token never reaches a tool — red-team: unauthorized caller cannot enumerate or steer).
/// - `snapshot`: a current-frame [`UiTreeSnapshot`] (the READ surface). `argus.inspect` returns it;
///   `argus.click`/`argus.set_value` resolve the target against it.
/// - `channel`: the action queue `argus.click`/`argus.set_value` enqueue onto.
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
    // 1. Auth gate (constant-time). A missing/wrong token is rejected before any tool runs.
    if !token.matches(&request.session_token) {
        return McpResponse::err(
            request.id.clone(),
            McpError {
                code: ERR_UNAUTHORIZED,
                message: "Unauthorized".to_owned(),
            },
        );
    }

    let argus_route = argus::route(request.method.as_str());
    let method = argus_route
        .map(|route| route.primitive)
        .unwrap_or(request.method.as_str());

    // 2. Method dispatch.
    match method {
        "list_widgets" => {
            let mut value = serde_json::to_value(snapshot)
                .unwrap_or_else(|_| serde_json::json!({ "error": "snapshot serialize failed" }));
            argus::stamp_result(&mut value, argus_route);
            McpResponse::ok(request.id.clone(), value)
        }
        "click_widget" => match parse_target(&request.params) {
            Ok(target) => enqueue_response(
                request,
                snapshot,
                channel,
                &target,
                UiAction::Click,
                argus_route,
            ),
            Err(e) => McpResponse::err(
                request.id.clone(),
                McpError {
                    code: e.code,
                    message: e.message,
                },
            ),
        },
        "set_value" => match parse_target_and_value(&request.params) {
            Ok((target, value)) => enqueue_response(
                request,
                snapshot,
                channel,
                &target,
                UiAction::SetValue { text: value },
                argus_route,
            ),
            Err(e) => McpResponse::err(
                request.id.clone(),
                McpError {
                    code: e.code,
                    message: e.message,
                },
            ),
        },
        "screenshot" => match capture() {
            Ok(shot) => {
                let mut value = shot.to_json();
                argus::stamp_result(&mut value, argus_route);
                McpResponse::ok(request.id.clone(), value)
            }
            Err(e) => McpResponse::err(request.id.clone(), e.into()),
        },
        other => McpResponse::err(
            request.id.clone(),
            McpError {
                code: ERR_METHOD_NOT_FOUND,
                message: format!("unknown method '{other}'"),
            },
        ),
    }
}

/// Resolve + enqueue an action and build the `{queued, action, node_id}` result (or the typed error).
fn enqueue_response(
    request: &McpRequest,
    snapshot: &UiTreeSnapshot,
    channel: &mut ActionChannel,
    target: &str,
    action: UiAction,
    argus_route: Option<ArgusRoute>,
) -> McpResponse {
    let action_name = format!("{:?}", action.accesskit_action());
    match channel.enqueue(snapshot, target, action) {
        Ok(outcome) => {
            let mut value = serde_json::json!({
                "queued": true,
                "action": action_name,
                "node_id": node_id_u64(&outcome.request.target),
                "target": target,
            });
            argus::stamp_result(&mut value, argus_route);
            McpResponse::ok(request.id.clone(), value)
        }
        Err(e) => McpResponse::err(request.id.clone(), e.into()),
    }
}

/// AccessKit `NodeId` is a newtype over u64; pull the inner value for the JSON result.
fn node_id_u64(id: &accesskit::NodeId) -> u64 {
    id.0
}

/// Parse the `target` author_id from a tool's params object.
fn parse_target(params: &serde_json::Value) -> Result<String, McpToolError> {
    params
        .get("target")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .ok_or_else(|| {
            McpToolError::new(
                ERR_INVALID_PARAMS,
                "params.target (author_id string) required",
            )
        })
}

/// Parse `target` + `value` for `set_value`.
fn parse_target_and_value(params: &serde_json::Value) -> Result<(String, String), McpToolError> {
    let target = parse_target(params)?;
    let value = params
        .get("value")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned())
        .ok_or_else(|| McpToolError::new(ERR_INVALID_PARAMS, "params.value (string) required"))?;
    Ok((target, value))
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
            actions: vec!["Click".to_owned(), "Focus".to_owned()],
            bounds: None,
            children: Vec::new(),
        };
        let input = UiTreeNode {
            id: "field".to_owned(),
            author_id: Some("field".to_owned()),
            node_id: 11,
            role: "TextInput".to_owned(),
            label: None,
            value: Some(String::new()),
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
            children: vec![button, input],
        };
        UiTreeSnapshot {
            root,
            captured_at_utc: "0Z".to_owned(),
            widget_count: 3,
        }
    }

    fn req(method: &str, params: serde_json::Value, token: &str) -> McpRequest {
        McpRequest {
            id: serde_json::json!(1),
            method: method.to_owned(),
            params,
            session_token: token.to_owned(),
            agent_label: None,
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
        assert_eq!(v["error"]["code"], ERR_UNAUTHORIZED);
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
        assert_eq!(v["result"]["widget_count"], 3);
        assert_eq!(v["result"]["root"]["role"], "Window");
    }

    #[test]
    fn argus_inspect_returns_snapshot_json_with_argus_metadata() {
        let token = SessionToken::from_hex("secret");
        let mut chan = ActionChannel::new();
        let r = dispatch_request(
            &req("argus.inspect", serde_json::json!({}), "secret"),
            &token,
            &snap(),
            &mut chan,
            ok_capture,
        );
        let v = r.to_json();
        assert_eq!(v["result"]["widget_count"], 2);
        assert_eq!(v["result"]["root"]["role"], "Window");
        assert_eq!(v["result"]["argus"]["method"], "argus.inspect");
        assert_eq!(v["result"]["argus"]["primitive"], "list_widgets");
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
    fn argus_click_enqueues_through_existing_action_channel() {
        let token = SessionToken::from_hex("secret");
        let mut chan = ActionChannel::new();
        let r = dispatch_request(
            &req(
                "argus.click",
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
        assert_eq!(v["result"]["argus"]["method"], "argus.click");
        assert_eq!(v["result"]["argus"]["primitive"], "click_widget");
        assert_eq!(chan.pending(), 1);
    }

    #[test]
    fn argus_set_value_enqueues_through_existing_action_channel() {
        let token = SessionToken::from_hex("secret");
        let mut chan = ActionChannel::new();
        let r = dispatch_request(
            &req(
                "argus.set_value",
                serde_json::json!({"target": "field", "value": "typed"}),
                "secret",
            ),
            &token,
            &snap(),
            &mut chan,
            ok_capture,
        );
        let v = r.to_json();
        assert_eq!(v["result"]["queued"], true);
        assert_eq!(v["result"]["action"], "Focus");
        assert_eq!(v["result"]["node_id"], 11);
        assert_eq!(v["result"]["argus"]["method"], "argus.set_value");
        assert_eq!(v["result"]["argus"]["primitive"], "set_value");
        assert_eq!(chan.pending(), 1);
    }

    #[test]
    fn argus_set_value_rejects_non_text_targets() {
        let token = SessionToken::from_hex("secret");
        let mut chan = ActionChannel::new();
        let r = dispatch_request(
            &req(
                "argus.set_value",
                serde_json::json!({"target": "btn", "value": "typed"}),
                "secret",
            ),
            &token,
            &snap(),
            &mut chan,
            ok_capture,
        );
        let v = r.to_json();
        assert_eq!(v["error"]["code"], ERR_TOOL_FAILED);
        assert!(
            v["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("SetValue"),
            "set_value rejection must name the unsupported SetValue action: {v}"
        );
        assert_eq!(chan.pending(), 0);
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
    }

    #[test]
    fn argus_screenshot_returns_visual_capture_shape_with_argus_metadata() {
        let token = SessionToken::from_hex("secret");
        let mut chan = ActionChannel::new();
        let r = dispatch_request(
            &req("argus.screenshot", serde_json::json!({}), "secret"),
            &token,
            &snap(),
            &mut chan,
            ok_capture,
        );
        let v = r.to_json();
        assert_eq!(v["result"]["png_base64"], "Zm9vYmFy");
        assert_eq!(v["result"]["width"], 4);
        assert_eq!(v["result"]["height"], 3);
        assert_eq!(v["result"]["argus"]["method"], "argus.screenshot");
        assert_eq!(v["result"]["argus"]["primitive"], "screenshot");
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
            "params": {"target": "btn"}, "session_token": "secret"
        });
        let parsed = McpRequest::from_json(&raw).expect("valid envelope");
        assert_eq!(parsed.method, "click_widget");
        assert_eq!(parsed.session_token, "secret");
        assert_eq!(parsed.id, serde_json::json!(7));
    }

    #[test]
    fn request_envelope_parses_optional_agent_label() {
        let raw = serde_json::json!({
            "jsonrpc": "2.0", "id": 7, "method": "argus.click",
            "params": {"target": "btn"}, "session_token": "secret",
            "agent_label": "codex worker 1"
        });
        let parsed = McpRequest::from_json(&raw).expect("valid envelope");
        assert_eq!(parsed.agent_label.as_deref(), Some("codex-worker-1"));
    }

    #[test]
    fn bad_jsonrpc_version_is_rejected() {
        let raw = serde_json::json!({ "jsonrpc": "1.0", "method": "x", "id": 1 });
        assert!(McpRequest::from_json(&raw).is_err());
    }
}
