//! Model-steering surface for the native Handshake shell (WP-KERNEL-011 MT-027).
//!
//! This module is the WRITE half of the model-vision contract. MT-025 emits the live AccessKit tree,
//! MT-026 projects it to a full nested JSON snapshot (the READ surface); MT-027 adds:
//!
//! 1. an **action channel** ([`action`]) that turns a model's `author_id`-addressed request
//!    (`click` / `focus` / `set_value` / `select` / `scroll`) into a real `accesskit::ActionRequest`
//!    bound to the widget's STABLE `NodeId`, and
//! 2. an **MCP-style tool surface** ([`tools`]) — canonical `argus.inspect`, `argus.click`,
//!    `argus.set_value`, and `argus.screenshot` methods (with legacy aliases) — dispatched through a
//!    JSON-RPC 2.0 subset so an external/in-process agent speaks
//!    the same protocol Claude Code and other MCP clients already use, and
//! 3. a **screenshot adapter** ([`screenshot`]) that captures a focus-safe PNG of the window, and
//! 4. an **out-of-process transport** ([`server`]) — a localhost TCP listener AND a Windows named pipe,
//!    both gated by the per-session HMAC [`SessionToken`], persisting an [`McpBinding`] discovery file —
//!    that newline-frames JSON-RPC and dispatches every request through [`tools::dispatch_request`].
//!
//! ## Transport-agnostic core + real out-of-process server (the contract's mandate)
//!
//! The MT-027 contract mandates an OUT-OF-PROCESS server: a `tokio::net::TcpListener` on `127.0.0.1:0`
//! AND a Windows named pipe, BOTH gated by a per-session HMAC token, with a `swarm_mcp_binding.json`
//! discovery file (owner-only perms) and per-connection rate limiting. That server is implemented in
//! [`server`]. It is built OVER the transport-agnostic [`tools::dispatch_request`]: that function
//! consumes an already-parsed [`McpRequest`] and returns an [`McpResponse`], touching no socket — so the
//! exact steering semantics proven by the in-process unit tests are what the socket/pipe transport
//! exposes byte-for-byte. The over-the-wire integration test BINDS the real TCP listener, CONNECTS a
//! client over the socket, and proves HMAC-authed Argus inspect + click round-trips and
//! steers the running shell.
//!
//! ## Screenshot: two sources
//!
//! The production [`screenshot`] tool grabs the live OS window via focus-safe Win32 `PrintWindow`
//! ([`screenshot::capture_handshake_window`]) — never `SetForegroundWindow`/`BringWindowToTop` (HBR-QUIET).
//! That OS path needs a real on-screen window, so it is genuinely undriveable from a headless `cargo
//! test`; the over-the-wire test injects an offscreen-render closure (`egui_kittest` wgpu render-to-image,
//! focus-safe by construction) to prove a real, decodable PNG flows through the tool. See the handoff
//! DEVIATION notes for what is and is not provable in this headless environment.
//!
//! ## Target-safe `argus.set_value`
//!
//! [`UiAction::SetValue`] dispatches one AccessKit `SetValue` request to the resolved node. Numeric
//! widgets receive `ActionData::NumericValue`; string-valued widgets receive `ActionData::Value` and
//! consume it at their real backing widget. This avoids global keyboard/text events, so concurrent
//! requests cannot overlap and a stale/unmounted target cannot modify whichever field has focus.

pub mod action;
pub mod argus;
pub mod attribution;
pub mod binding;
pub mod layout_guard;
pub mod leases;
pub mod screenshot;
pub mod server;
pub mod session;
pub mod tools;

pub use action::{
    accesskit_string_set_value, build_action_request, resolve_target, ActionChannel, ActionError,
    ActionOutcome, ActionReceipt, ActionReceiptStatus, UiAction, DEFAULT_ACTION_CAPACITY,
    MAX_ACTIONS_PER_BURST,
};
pub use argus::{
    ArgusMethod, ARGUS_CLICK_METHOD, ARGUS_INSPECT_METHOD, ARGUS_SCREENSHOT_METHOD,
    ARGUS_SET_VALUE_METHOD, LEGACY_CLICK_METHOD, LEGACY_INSPECT_METHOD, LEGACY_SCREENSHOT_METHOD,
    LEGACY_SET_VALUE_METHOD,
};
pub use attribution::{
    agent_id_for_token, ActionLog, AttributedAction, ACTION_LOG_CAPACITY, AGENT_ID_HEX_LEN,
};
pub use binding::{
    binding_path, remove_binding, restore_binding_if_current, write_binding, BindingError,
    McpBinding, BINDING_FILE_NAME,
};
pub use layout_guard::LayoutGuard;
pub use leases::{LeaseError, LeaseGuard, LeaseKind, LeaseRegistry, DEFAULT_LEASE_TIMEOUT};
pub use screenshot::{
    capture_handshake_window, ScreenshotError, ScreenshotResult, HANDSHAKE_WINDOW_TITLE,
};
pub use server::{SwarmMcpServer, MAX_LINE_BYTES, MAX_REQUESTS_PER_SEC};
pub use session::{McpSession, SwarmSafetyState, SNAPSHOT_RESOURCE};
pub use tools::{
    dispatch_request, McpError, McpRequest, McpResponse, McpToolError, SessionToken,
    ERR_ACTION_QUEUE_FULL, ERR_INVALID_PARAMS, ERR_LEASE_TIMEOUT, ERR_METHOD_NOT_FOUND,
    ERR_RATE_LIMITED, ERR_TOOL_FAILED, ERR_UNAUTHORIZED,
};
