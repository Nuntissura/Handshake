//! Per-connection MCP session: attribution + leasing applied consistently (WP-KERNEL-011 MT-028).
//!
//! MT-027's [`crate::mcp::tools::dispatch_request`] turns one parsed request into one response with auth
//! + the Argus/MCP tools. [`McpSession`] is the MT-028 wrapper that makes the SAME dispatch safe under N concurrent agents.
//!
//! 1. Every accepted connection gets one `McpSession` holding a deterministic default `agent_id` derived
//!    from its session token (see [`crate::mcp::attribution::agent_id_for_token`]). A request may also
//!    carry top-level `agent_label`; then receipts derive the effective `agent_id` from
//!    `session_token + agent_label` so parallel clients sharing one binding token remain attributable.
//! 2. A mutating tool (`argus.click` / `argus.set_value`) acquires an EXCLUSIVE lease on its target widget
//!    key before the action is enqueued, and a reading tool (`argus.inspect`) acquires a SHARED lease on
//!    the snapshot resource — so two agents cannot drive the same widget at once, but many can read
//!    concurrently (the contract's lease granularity).
//! 3. After a mutating tool successfully enqueues, the action is APPENDED to the shared
//!    [`crate::mcp::attribution::ActionLog`] with the effective `agent_id` and optional `agent_label` —
//!    the post-hoc audit trail.
//!
//! The lease is held ONLY for the dispatch span (acquire -> dispatch -> append -> drop), which is the
//! synchronous, await-free window MT-027's `dispatch_locked` already runs in. Holding it longer would
//! serialize the swarm; holding it shorter would not protect the resolve+enqueue against a racing agent.
//!
//! ## Why the lease key is the widget `author_id`
//!
//! A model addresses a widget by its stable `author_id` (the MT-025 convention); that same string is the
//! lease resource key, so two agents targeting the SAME widget contend on the SAME lease, while agents
//! targeting DIFFERENT widgets never contend (fine-grained, low-contention — the contract's design).
//! `list_widgets` is a whole-tree read, so it leases the single [`SNAPSHOT_RESOURCE`] key shared, which
//! only blocks while some op holds it exclusively (none currently does; reserved for a future snapshot
//! write).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::accessibility::UiTreeSnapshot;
use crate::mcp::action::ActionChannel;
use crate::mcp::attribution::{agent_id_for_token, agent_id_for_token_and_label, ActionLog};
use crate::mcp::leases::{LeaseKind, LeaseRegistry, DEFAULT_LEASE_TIMEOUT};
use crate::mcp::screenshot::{ScreenshotError, ScreenshotResult};
use crate::mcp::tools::{
    dispatch_request, McpError, McpRequest, McpResponse, SessionToken, ERR_LEASE_TIMEOUT,
};

/// The lease resource key for a whole-tree read (`list_widgets`). A read takes this SHARED, so many
/// reads coexist; reserved exclusive use would be a future snapshot-rewrite op.
///
/// NOTE (deliberate future tradeoff): `list_widgets` takes this key SHARED — many reads coexist and
/// nothing currently takes it EXCLUSIVE, so the read lease never actually blocks today. If a future
/// snapshot WRITER (a whole-tree rewrite) is introduced, it would take this single global key EXCLUSIVE,
/// which would act as a COARSE global read-gate: every `list_widgets` across the whole swarm would block
/// for the writer's span, regardless of which subtree changed. That coarse granularity is an accepted
/// tradeoff for the single-snapshot model (the whole tree is rebuilt atomically); finer-grained
/// per-subtree snapshot leasing would be the alternative if read throughput under a writer ever matters.
pub const SNAPSHOT_RESOURCE: &str = "ui.snapshot";

/// The leasing + attribution decision for one request, computed ONCE from the auth-gated request so the
/// synchronous [`McpSession::dispatch`] and the async [`McpSession::dispatch_shared_async`] entry points
/// share IDENTICAL auth-gate + target-extraction + lease-kind-selection + attribute-vs-passthrough logic
/// (DRY — the two paths differ ONLY in how they acquire the lease and lock the channel, never in the
/// decision). [`McpSession::decide`] returns this; each entry point then does its own sync/async lease
/// acquire + enqueue.
enum DispatchPlan {
    /// Auth failed, no `target`/lease applies, or an unknown method: dispatch directly with NO lease and
    /// NO attribution (auth errors, malformed-param errors, `screenshot`, unknown methods). The canonical
    /// error/response shape comes from [`dispatch_request`].
    Direct,
    /// A reading tool (`list_widgets`): take a SHARED lease on [`SNAPSHOT_RESOURCE`], dispatch, NO
    /// attribution (reads are not logged).
    SharedRead,
    /// A mutating tool (`click_widget` / `set_value`): take an EXCLUSIVE lease on the carried target key,
    /// dispatch, then attribute + stamp the success result with the acting `agent_id`.
    ExclusiveWrite {
        /// The widget `author_id` that is BOTH the lease resource key and the attribution target.
        target: String,
    },
}

/// One MCP connection's session: a deterministic default `agent_id`, plus clones of the shared lease
/// registry, action log, session token, and the dispatch state (snapshot + channel) the tools act on.
///
/// Cloneable-by-construction over `Arc`s: the server builds one per accepted connection from the shared
/// `ServerState`, so all sessions contend on the SAME [`LeaseRegistry`] and append to the SAME
/// [`ActionLog`].
#[derive(Clone)]
pub struct McpSession {
    /// The short deterministic default per-session id (first 8 hex of SHA-256(token)).
    agent_id: String,
    /// The per-session HMAC token (the dispatch auth-gates every request against this).
    token: SessionToken,
    /// The shared registry every session contends on for widget/pane leases.
    leases: LeaseRegistry,
    /// The shared append-only audit log of attributed actions.
    log: ActionLog,
    /// Per-acquire lease timeout (configurable so the concurrent test can force the timeout path).
    lease_timeout: Duration,
}

impl McpSession {
    /// Build a session for a connection authenticated by `token`. The `agent_id` is derived from the
    /// token's hex (deterministic per session). Shares the given registry + log with all other sessions.
    pub fn new(token: SessionToken, leases: LeaseRegistry, log: ActionLog) -> Self {
        let agent_id = agent_id_for_token(token.as_hex());
        Self {
            agent_id,
            token,
            leases,
            log,
            lease_timeout: DEFAULT_LEASE_TIMEOUT,
        }
    }

    /// Override the lease timeout (the concurrent test uses a short value to exercise the timeout path
    /// deterministically; production uses [`DEFAULT_LEASE_TIMEOUT`]).
    pub fn with_lease_timeout(mut self, timeout: Duration) -> Self {
        self.lease_timeout = timeout;
        self
    }

    /// This session's deterministic default agent id.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    fn effective_agent<'a>(&self, request: &'a McpRequest) -> (String, Option<&'a str>) {
        let label = request.agent_label.as_deref();
        (
            agent_id_for_token_and_label(self.token.as_hex(), label),
            label,
        )
    }

    /// The ONE shared decision both entry points call: auth-gate + target-extraction + lease-kind
    /// selection + attribute-vs-passthrough, computed from the request WITHOUT acquiring any lease or
    /// touching the channel. Returns the [`DispatchPlan`] each entry point then executes with its own
    /// sync/async lease acquire + sync/locked enqueue (so the two paths can NEVER drift on the decision).
    ///
    /// - Auth fail -> [`DispatchPlan::Direct`] (the caller dispatches for the canonical -32001 shape; no
    ///   lease, no attribution).
    /// - `click_widget` / `set_value` with a non-empty `target` -> [`DispatchPlan::ExclusiveWrite`]; a
    ///   missing/empty `target` is malformed -> [`DispatchPlan::Direct`] so `dispatch_request` produces
    ///   the canonical -32602 (nothing to lease).
    /// - `list_widgets` -> [`DispatchPlan::SharedRead`].
    /// - `screenshot` + unknown methods -> [`DispatchPlan::Direct`] (no shared-widget mutation).
    fn decide(&self, request: &McpRequest) -> DispatchPlan {
        // Auth-gate BEFORE any lease/channel work so an unauthorized flood cannot even contend for leases.
        if !self.token.matches(&request.session_token) {
            return DispatchPlan::Direct;
        }
        match crate::mcp::argus::primitive_method(request.method.as_str()) {
            "click_widget" | "set_value" => {
                match request.params.get("target").and_then(|v| v.as_str()) {
                    Some(t) if !t.is_empty() => DispatchPlan::ExclusiveWrite {
                        target: t.to_owned(),
                    },
                    // Missing/empty target is malformed: no lease, let dispatch_request emit -32602.
                    _ => DispatchPlan::Direct,
                }
            }
            "list_widgets" => DispatchPlan::SharedRead,
            // screenshot + unknown methods: no shared-widget mutation, so no lease.
            _ => DispatchPlan::Direct,
        }
    }

    /// Dispatch one request WITH leasing + attribution applied (the SYNCHRONOUS path, for in-process
    /// callers that already hold an exclusive `&mut ActionChannel` — the unit tests, and any future
    /// single-threaded driver). The async server connection task uses [`Self::dispatch_shared_async`]
    /// instead, so it never blocks a tokio worker on the lease wait (see that method).
    ///
    /// The leasing/attribution DECISION is shared with the async path via [`Self::decide`]; this method
    /// differs only in acquiring the lease synchronously ([`LeaseRegistry::try_acquire`]) and dispatching
    /// against the exclusive `&mut ActionChannel` the caller already holds.
    ///
    /// - `click_widget` / `set_value`: acquire an EXCLUSIVE lease on the target key; on timeout return a
    ///   typed [`ERR_LEASE_TIMEOUT`] error instead of racing. On a successful enqueue, append an
    ///   attributed entry to the shared log AND stamp the acting `agent_id` into the result (AC#2).
    /// - `list_widgets`: acquire a SHARED lease on [`SNAPSHOT_RESOURCE`] (coexists with other reads).
    /// - `screenshot` + unknown methods + auth failure: no lease (no shared-widget mutation), dispatched
    ///   directly.
    ///
    /// The lease guard is dropped at the end of this call, releasing the resource for the next agent.
    pub fn dispatch(
        &self,
        request: &McpRequest,
        snapshot: &UiTreeSnapshot,
        channel: &mut ActionChannel,
        capture: impl FnOnce() -> Result<ScreenshotResult, ScreenshotError>,
    ) -> McpResponse {
        match self.decide(request) {
            DispatchPlan::Direct => {
                dispatch_request(request, &self.token, snapshot, channel, capture)
            }
            DispatchPlan::SharedRead => {
                // Shared read lease: blocks only under an exclusive holder; many reads coexist.
                match self.leases.try_acquire(
                    SNAPSHOT_RESOURCE,
                    LeaseKind::Shared,
                    self.lease_timeout,
                ) {
                    Ok(_read_guard) => {
                        dispatch_request(request, &self.token, snapshot, channel, capture)
                    }
                    Err(e) => Self::lease_timeout_response(request, e),
                }
                // _read_guard drops here, releasing the shared read lease.
            }
            DispatchPlan::ExclusiveWrite { target } => {
                // Acquire the EXCLUSIVE per-widget lease FIRST (the gate). Loser -> typed -32004.
                let _guard =
                    match self
                        .leases
                        .try_acquire(&target, LeaseKind::Exclusive, self.lease_timeout)
                    {
                        Ok(g) => g,
                        Err(e) => return Self::lease_timeout_response(request, e),
                    };
                let response = dispatch_request(request, &self.token, snapshot, channel, capture);
                self.attribute_and_stamp(response, request, &target)
                // _guard drops here, releasing the exclusive widget lease for the next agent.
            }
        }
    }

    /// Dispatch one request WITH leasing + attribution applied, holding the shared
    /// `Arc<Mutex<ActionChannel>>` lock ONLY for the brief resolve+enqueue span — NOT across the lease
    /// wait. This is the MAJOR fix that makes the per-widget lease the REAL contention point under the
    /// swarm (WP-KERNEL-011 MT-028):
    ///
    /// - The exclusive per-widget LEASE is acquired FIRST (gating inter-agent access). Two agents
    ///   targeting the SAME widget serialize HERE (one waits, or times out with -32004); agents on
    ///   DIFFERENT widgets never contend on the lease, so they proceed concurrently.
    /// - The global channel `Mutex` is locked ONLY for the `dispatch_request` (resolve + enqueue) call,
    ///   then released immediately — it is NEVER held while an agent waits for a lease. So two agents on
    ///   different widgets serialize on the channel lock only for the sub-microsecond enqueue, not for the
    ///   whole (potentially blocking) dispatch, and shared reads interleave freely.
    /// - The lease wait is `tokio::time::sleep`-based ([`LeaseRegistry::acquire_async`]), so a waiting
    ///   agent YIELDS its tokio worker thread instead of blocking it (the MINOR fix).
    ///
    /// `snapshot` is a CLONE the caller already took (cheap, lock-free here); the channel is locked
    /// per-call below.
    ///
    /// The leasing/attribution DECISION is shared with the sync path via [`Self::decide`]; this method
    /// differs only in acquiring the lease asynchronously ([`LeaseRegistry::acquire_async`], which yields
    /// the worker thread) and locking the shared `Arc<Mutex<ActionChannel>>` for ONLY the brief enqueue.
    pub async fn dispatch_shared_async(
        &self,
        request: &McpRequest,
        snapshot: &UiTreeSnapshot,
        channel: &Arc<Mutex<ActionChannel>>,
        capture: impl FnOnce() -> Result<ScreenshotResult, ScreenshotError>,
    ) -> McpResponse {
        match self.decide(request) {
            DispatchPlan::Direct => {
                let mut ch = lock_channel(channel);
                dispatch_request(request, &self.token, snapshot, &mut ch, capture)
            }
            DispatchPlan::SharedRead => {
                let _read_guard = match self
                    .leases
                    .acquire_async(SNAPSHOT_RESOURCE, LeaseKind::Shared, self.lease_timeout)
                    .await
                {
                    Ok(g) => g,
                    Err(e) => return Self::lease_timeout_response(request, e),
                };
                let mut ch = lock_channel(channel);
                dispatch_request(request, &self.token, snapshot, &mut ch, capture)
            }
            DispatchPlan::ExclusiveWrite { target } => {
                // LEASE FIRST (async wait — yields the worker thread; never holds the channel lock here).
                let _guard = match self
                    .leases
                    .acquire_async(&target, LeaseKind::Exclusive, self.lease_timeout)
                    .await
                {
                    Ok(g) => g,
                    Err(e) => return Self::lease_timeout_response(request, e),
                };
                // Now lock the channel ONLY for the brief resolve+enqueue, under the held lease.
                let response = {
                    let mut ch = lock_channel(channel);
                    dispatch_request(request, &self.token, snapshot, &mut ch, capture)
                    // channel lock drops here — released before we attribute / drop the lease.
                };
                self.attribute_and_stamp(response, request, &target)
                // _guard drops here.
            }
        }
    }

    /// Build the typed [`ERR_LEASE_TIMEOUT`] (-32004) response for a contended lease.
    fn lease_timeout_response(
        request: &McpRequest,
        e: crate::mcp::leases::LeaseError,
    ) -> McpResponse {
        McpResponse::error(
            request.id.clone(),
            McpError {
                code: ERR_LEASE_TIMEOUT,
                message: e.to_string(),
            },
        )
    }

    /// On a successful mutating enqueue: append the attributed action to the shared log AND rebuild the
    /// result so it carries the acting `agent_id` (AC#2 — a swarm reader must see WHICH agent's action
    /// was queued, over the wire). Non-success responses pass through unchanged.
    fn attribute_and_stamp(
        &self,
        response: McpResponse,
        request: &McpRequest,
        target: &str,
    ) -> McpResponse {
        // Attribute + stamp ONLY a successful enqueue (result carries `queued: true` + node_id).
        let queued = matches!(
            response.result_ref(),
            Ok(result) if result.get("queued").and_then(|v| v.as_bool()) == Some(true)
        );
        if !queued {
            return response;
        }
        let result = match response.result_ref() {
            Ok(r) => r.clone(),
            Err(_) => return response, // unreachable given `queued`, but keep the type total.
        };
        let node_id = result.get("node_id").and_then(|v| v.as_u64()).unwrap_or(0);
        let (agent_id, agent_label) = self.effective_agent(request);
        self.log
            .record(&agent_id, agent_label, &request.method, target, node_id);
        // Rebuild the result Value with the acting agent_id added (AC#2).
        let mut stamped = result;
        if let Some(obj) = stamped.as_object_mut() {
            obj.insert("agent_id".to_owned(), serde_json::Value::String(agent_id));
            if let Some(label) = agent_label {
                obj.insert(
                    "agent_label".to_owned(),
                    serde_json::Value::String(label.to_owned()),
                );
            }
        }
        McpResponse::ok_value(request.id.clone(), stamped)
    }
}

/// Lock the shared channel for the minimum span, recovering a poisoned lock (a prior holder panicked
/// while holding it) so one agent's panic cannot wedge every other connection's enqueue path.
fn lock_channel(channel: &Arc<Mutex<ActionChannel>>) -> std::sync::MutexGuard<'_, ActionChannel> {
    channel
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The shared steering + safety state the server hands to every [`McpSession`]. Built once at server
/// init from the per-session token; the registry + log are SHARED across all connections (so leasing
/// and attribution are global), while the snapshot + channel are the same `Arc<Mutex<_>>` the egui frame
/// loop owns.
#[derive(Clone)]
pub struct SwarmSafetyState {
    /// The per-session HMAC token (one server = one session token, per MT-027).
    pub token: SessionToken,
    /// The shared lease registry every connection contends on.
    pub leases: LeaseRegistry,
    /// The shared attributed-action audit log.
    pub log: ActionLog,
    /// The live UI-tree snapshot slot (shared with the egui frame loop).
    pub snapshot: Arc<Mutex<UiTreeSnapshot>>,
    /// The bounded action channel (shared with the egui frame loop).
    pub channel: Arc<Mutex<ActionChannel>>,
    /// Per-acquire lease timeout every connection's [`McpSession`] inherits. Defaults to
    /// [`DEFAULT_LEASE_TIMEOUT`]; the concurrent harness overrides it with a short value to exercise the
    /// lease-timeout path deterministically over the wire.
    pub lease_timeout: Duration,
}

impl SwarmSafetyState {
    /// Build the shared safety state for a server. Each connection derives its own [`McpSession`] from
    /// this via [`Self::session`]. The lease registry + attribution log are fresh (per-server).
    pub fn new(
        token: SessionToken,
        snapshot: Arc<Mutex<UiTreeSnapshot>>,
        channel: Arc<Mutex<ActionChannel>>,
    ) -> Self {
        Self {
            token,
            leases: LeaseRegistry::new(),
            log: ActionLog::new(),
            snapshot,
            channel,
            lease_timeout: DEFAULT_LEASE_TIMEOUT,
        }
    }

    /// Override the per-connection lease timeout (the concurrent harness uses a short value so the
    /// lease-contention path times out deterministically). Returns `self` for chaining.
    pub fn with_lease_timeout(mut self, timeout: Duration) -> Self {
        self.lease_timeout = timeout;
        self
    }

    /// Build a safety state that SHARES a given lease registry + attribution log across servers. Used by
    /// the concurrent harness (and any multi-token swarm topology) where N agents each have a DISTINCT
    /// session token — so each gets a distinct `agent_id` — yet must contend on ONE global lease registry
    /// and append to ONE global attribution log. Each per-token server is bound with its own
    /// `SwarmSafetyState` built here from the same shared `leases` + `log`.
    pub fn with_shared(
        token: SessionToken,
        snapshot: Arc<Mutex<UiTreeSnapshot>>,
        channel: Arc<Mutex<ActionChannel>>,
        leases: LeaseRegistry,
        log: ActionLog,
    ) -> Self {
        Self {
            token,
            leases,
            log,
            snapshot,
            channel,
            lease_timeout: DEFAULT_LEASE_TIMEOUT,
        }
    }

    /// A session for one accepted connection (shares the registry + log + dispatch state, and inherits
    /// this state's [`Self::lease_timeout`]).
    pub fn session(&self) -> McpSession {
        McpSession::new(self.token.clone(), self.leases.clone(), self.log.clone())
            .with_lease_timeout(self.lease_timeout)
    }

    /// The shared action log (for diagnostics / tests).
    pub fn log(&self) -> &ActionLog {
        &self.log
    }

    /// The shared lease registry (for diagnostics / tests).
    pub fn leases(&self) -> &LeaseRegistry {
        &self.leases
    }
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
            agent_label: None,
        }
    }

    fn no_capture() -> Result<ScreenshotResult, ScreenshotError> {
        Ok(screenshot_from_png(b"x", 1, 1))
    }

    #[test]
    fn click_through_session_enqueues_and_attributes() {
        let token = SessionToken::from_hex("secret");
        let state = SwarmSafetyState::new(
            token.clone(),
            Arc::new(Mutex::new(snap())),
            Arc::new(Mutex::new(ActionChannel::new())),
        );
        let session = state.session();
        let snapshot = snap();
        let mut channel = ActionChannel::new();

        let resp = session.dispatch(
            &req(
                "click_widget",
                serde_json::json!({ "target": "btn" }),
                "secret",
            ),
            &snapshot,
            &mut channel,
            no_capture,
        );
        assert_eq!(resp.to_json()["result"]["queued"], true);
        // MAJOR #2 / AC#2: the success result is stamped with the acting agent_id.
        assert_eq!(
            resp.to_json()["result"]["agent_id"],
            session.agent_id(),
            "the click result carries the acting agent_id"
        );
        assert_eq!(channel.pending(), 1, "action enqueued under the lease");

        // The action is attributed in the shared log with THIS session's agent_id.
        let entries = state.log().drain_log();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].agent_id, session.agent_id());
        assert_eq!(entries[0].op_name, "click_widget");
        assert_eq!(entries[0].target_key, "btn");
        assert_eq!(entries[0].node_id, 10);
        // The lease is released after dispatch (no resource left held).
        assert_eq!(state.leases().active_resource_count(), 0);
    }

    #[test]
    fn argus_click_through_session_enqueues_attributes_and_uses_exclusive_lease() {
        let token = SessionToken::from_hex("secret");
        let state = SwarmSafetyState::new(
            token.clone(),
            Arc::new(Mutex::new(snap())),
            Arc::new(Mutex::new(ActionChannel::new())),
        );
        let session = state.session();
        let snapshot = snap();
        let mut channel = ActionChannel::new();

        let resp = session.dispatch(
            &req(
                "argus.click",
                serde_json::json!({ "target": "btn" }),
                "secret",
            ),
            &snapshot,
            &mut channel,
            no_capture,
        );
        let json = resp.to_json();
        assert_eq!(json["result"]["queued"], true);
        assert_eq!(json["result"]["agent_id"], session.agent_id());
        assert_eq!(json["result"]["argus"]["method"], "argus.click");
        assert_eq!(channel.pending(), 1, "argus click enqueued under the lease");

        let entries = state.log().drain_log();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].agent_id, session.agent_id());
        assert_eq!(entries[0].op_name, "argus.click");
        assert_eq!(entries[0].target_key, "btn");
        assert_eq!(entries[0].node_id, 10);
        assert_eq!(state.leases().active_resource_count(), 0);
    }

    #[test]
    fn same_token_agent_labels_stamp_distinct_agent_ids() {
        let token = SessionToken::from_hex("secret");
        let state = SwarmSafetyState::new(
            token.clone(),
            Arc::new(Mutex::new(snap())),
            Arc::new(Mutex::new(ActionChannel::new())),
        );
        let session = state.session();
        let snapshot = snap();
        let mut channel = ActionChannel::new();
        let mut a = req(
            "argus.click",
            serde_json::json!({ "target": "btn" }),
            "secret",
        );
        a.agent_label = Some("codex-a".to_owned());
        let mut b = req(
            "argus.click",
            serde_json::json!({ "target": "btn" }),
            "secret",
        );
        b.agent_label = Some("codex-b".to_owned());

        let resp_a = session.dispatch(&a, &snapshot, &mut channel, no_capture);
        let resp_b = session.dispatch(&b, &snapshot, &mut channel, no_capture);
        let json_a = resp_a.to_json();
        let json_b = resp_b.to_json();

        assert_eq!(json_a["result"]["agent_label"], "codex-a");
        assert_eq!(json_b["result"]["agent_label"], "codex-b");
        assert_ne!(
            json_a["result"]["agent_id"], json_b["result"]["agent_id"],
            "same live token plus different labels must not collapse attribution"
        );
        let entries = state.log().drain_log();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].agent_label.as_deref(), Some("codex-a"));
        assert_eq!(entries[1].agent_label.as_deref(), Some("codex-b"));
        assert_ne!(entries[0].agent_id, entries[1].agent_id);
    }

    #[test]
    fn unauthorized_session_takes_no_lease_and_logs_nothing() {
        let token = SessionToken::from_hex("secret");
        let state = SwarmSafetyState::new(
            token,
            Arc::new(Mutex::new(snap())),
            Arc::new(Mutex::new(ActionChannel::new())),
        );
        let session = state.session();
        let snapshot = snap();
        let mut channel = ActionChannel::new();

        let resp = session.dispatch(
            &req(
                "click_widget",
                serde_json::json!({ "target": "btn" }),
                "WRONG",
            ),
            &snapshot,
            &mut channel,
            no_capture,
        );
        assert_eq!(resp.to_json()["error"]["code"], -32001);
        assert_eq!(
            channel.pending(),
            0,
            "no action enqueued for an unauthorized caller"
        );
        assert!(
            state.log().is_empty(),
            "no attribution for an unauthorized caller"
        );
        assert_eq!(state.leases().active_resource_count(), 0, "no lease taken");
    }

    #[test]
    fn contended_exclusive_lease_returns_lease_timeout() {
        let token = SessionToken::from_hex("secret");
        let state = SwarmSafetyState::new(
            token,
            Arc::new(Mutex::new(snap())),
            Arc::new(Mutex::new(ActionChannel::new())),
        );
        // Hold the "btn" exclusive lease out-of-band, then a session click on "btn" must time out.
        let _held = state
            .leases()
            .try_acquire("btn", LeaseKind::Exclusive, Duration::from_millis(10))
            .expect("hold btn lease");

        let session = state
            .session()
            .with_lease_timeout(Duration::from_millis(30));
        let snapshot = snap();
        let mut channel = ActionChannel::new();
        let resp = session.dispatch(
            &req(
                "click_widget",
                serde_json::json!({ "target": "btn" }),
                "secret",
            ),
            &snapshot,
            &mut channel,
            no_capture,
        );
        assert_eq!(
            resp.to_json()["error"]["code"],
            ERR_LEASE_TIMEOUT,
            "a contended widget lease yields a typed lease-timeout error"
        );
        assert_eq!(
            channel.pending(),
            0,
            "no action enqueued when the lease could not be acquired"
        );
    }

    #[test]
    fn list_widgets_takes_a_shared_lease_and_succeeds() {
        let token = SessionToken::from_hex("secret");
        let state = SwarmSafetyState::new(
            token,
            Arc::new(Mutex::new(snap())),
            Arc::new(Mutex::new(ActionChannel::new())),
        );
        let session = state.session();
        let snapshot = snap();
        let mut channel = ActionChannel::new();
        let resp = session.dispatch(
            &req("list_widgets", serde_json::json!({}), "secret"),
            &snapshot,
            &mut channel,
            no_capture,
        );
        assert_eq!(resp.to_json()["result"]["widget_count"], 2);
        assert_eq!(
            state.leases().active_resource_count(),
            0,
            "shared read lease released after"
        );
    }
}
