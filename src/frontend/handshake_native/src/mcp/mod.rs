//! Model-steering surface for the native Handshake shell (WP-KERNEL-011 MT-027).
//!
//! This module is the WRITE half of the model-vision contract. MT-025 emits the live AccessKit tree,
//! MT-026 projects it to a full nested JSON snapshot (the READ surface); MT-027 adds:
//!
//! 1. an **action channel** ([`action`]) that turns a model's `author_id`-addressed request
//!    (`click` / `focus` / `set_value` / `select` / `scroll`) into a real `accesskit::ActionRequest`
//!    bound to the widget's STABLE `NodeId`, and
//! 2. an **MCP-style tool surface** ([`tools`]) — `list_widgets`, `click_widget`, `set_value`,
//!    `screenshot` — dispatched through a JSON-RPC 2.0 subset so an external/in-process agent speaks
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
//! client over the socket, and proves an HMAC-authed `list_widgets` + `click_widget` round-trips and
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
//! ## Why `set_value` is Focus + characters, NOT `Action::SetValue`
//!
//! The contract body asked the `set_value` tool to dispatch `accesskit::Action::SetValue`. MT-026
//! already proved (and its test asserts) that **egui 0.33 text inputs do not emit `SetValue`** — they
//! are steered out-of-process by FOCUSING the field and feeding synthetic characters (the path the
//! MT-001 toolkit spike proved: "typed 10 synthetic chars"). Dispatching `SetValue` to an egui text
//! input is a no-op. So [`UiAction::SetValue`] resolves to a Focus action plus a text payload the
//! caller feeds as `egui::Event::Text`; this is the steering path that actually changes the widget,
//! honoring the contract's INTENT (set a text widget's value by stable id) over its mistaken mechanic.

pub mod action;
pub mod attribution;
pub mod binding;
pub mod layout_guard;
pub mod leases;
pub mod screenshot;
pub mod server;
pub mod session;
pub mod tools;

pub use action::{
    build_action_request, resolve_target, ActionChannel, ActionError, ActionOutcome, UiAction,
    DEFAULT_ACTION_CAPACITY, MAX_ACTIONS_PER_BURST,
};
pub use attribution::{agent_id_for_token, ActionLog, AttributedAction, ACTION_LOG_CAPACITY, AGENT_ID_HEX_LEN};
pub use binding::{binding_path, remove_binding, write_binding, BindingError, McpBinding, BINDING_FILE_NAME};
pub use layout_guard::LayoutGuard;
pub use leases::{LeaseError, LeaseGuard, LeaseKind, LeaseRegistry, DEFAULT_LEASE_TIMEOUT};
pub use screenshot::{capture_handshake_window, ScreenshotError, ScreenshotResult, HANDSHAKE_WINDOW_TITLE};
pub use server::{SwarmMcpServer, MAX_LINE_BYTES, MAX_REQUESTS_PER_SEC};
pub use session::{McpSession, SwarmSafetyState, SNAPSHOT_RESOURCE};
pub use tools::{
    dispatch_request, McpError, McpRequest, McpResponse, McpToolError, SessionToken, ERR_ACTION_QUEUE_FULL,
    ERR_INVALID_PARAMS, ERR_LEASE_TIMEOUT, ERR_METHOD_NOT_FOUND, ERR_RATE_LIMITED, ERR_TOOL_FAILED,
    ERR_UNAUTHORIZED,
};
