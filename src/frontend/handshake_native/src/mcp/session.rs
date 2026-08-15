//! Per-connection MCP session: attribution + leasing applied consistently (WP-KERNEL-011 MT-028).
//!
//! MT-027's [`crate::mcp::tools::dispatch_request`] turns one parsed request into one response with auth
//! + the four tools. [`McpSession`] is the MT-028 wrapper that makes the SAME dispatch safe under N concurrent agents.
//!
//! 1. Production clients exchange the root discovery token for a broker-minted agent credential.
//!    Every mutation is attributed to the credential-derived `agent_id`; the caller-controlled
//!    `agent_label` remains display metadata and cannot select that identity. Legacy non-durable
//!    in-process sessions retain the deterministic root-token fallback.
//! 2. A mutating tool (`click_widget` / `set_value`) acquires an EXCLUSIVE lease on its target widget
//!    key before the action is enqueued, and a reading tool (`list_widgets`) acquires a SHARED lease on
//!    the snapshot resource — so two agents cannot drive the same widget at once, but many can read
//!    concurrently (the contract's lease granularity).
//! 3. After a mutating tool successfully enqueues, the action is APPENDED to the shared
//!    [`crate::mcp::attribution::ActionLog`] with this session's `agent_id` — the post-hoc audit trail.
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

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::accessibility::UiTreeSnapshot;
use crate::mcp::action::ActionChannel;
use crate::mcp::argus::{
    validate_agent_label, ArgusError, ArgusWindowDescriptor, WindowSnapshotRegistry,
    ACTION_RECEIPT_TIMEOUT, MAIN_WINDOW_ID,
};
use crate::mcp::attribution::{agent_id_for_token, ActionLog};
use crate::mcp::leases::{LeaseKind, LeaseRegistry, DEFAULT_LEASE_TIMEOUT};
use crate::mcp::screenshot::{ScreenshotError, ScreenshotResult, HANDSHAKE_WINDOW_TITLE};
use crate::mcp::tools::{
    canonical_method, dispatch_request, dispatch_windowed_request, McpError, McpRequest,
    McpResponse, SessionToken, ERR_LEASE_TIMEOUT,
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

#[derive(Clone)]
pub struct ArgusReceiptProvenance {
    pub diagnostics_session_id: uuid::Uuid,
    signing_secret:
        std::sync::Arc<dyn Fn() -> Option<zeroize::Zeroizing<[u8; 32]>> + Send + Sync + 'static>,
}

impl ArgusReceiptProvenance {
    pub fn new(
        diagnostics_session_id: uuid::Uuid,
        signing_secret: zeroize::Zeroizing<[u8; 32]>,
    ) -> Self {
        let signing_secret = std::sync::Arc::new(signing_secret);
        Self {
            diagnostics_session_id,
            signing_secret: std::sync::Arc::new(move || Some((*signing_secret).clone())),
        }
    }

    /// Build provenance over the live Palmistry secret slot. The provider is
    /// evaluated for every durable receipt so a backend restart can rotate the
    /// secret without rebinding the MCP listener or leaving existing sessions
    /// permanently signed with the pre-restart key.
    pub fn dynamic(
        diagnostics_session_id: uuid::Uuid,
        signing_secret: std::sync::Arc<
            dyn Fn() -> Option<zeroize::Zeroizing<[u8; 32]>> + Send + Sync + 'static,
        >,
    ) -> Self {
        Self {
            diagnostics_session_id,
            signing_secret,
        }
    }
}

impl std::fmt::Debug for ArgusReceiptProvenance {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ArgusReceiptProvenance")
            .field("diagnostics_session_id", &self.diagnostics_session_id)
            .field("signing_secret", &"[REDACTED]")
            .finish()
    }
}

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

/// One MCP connection's session: a legacy fallback `agent_id`, the production credential broker, plus
/// clones of the shared lease registry, action log, session token, and dispatch state.
///
/// Cloneable-by-construction over `Arc`s: the server builds one per accepted connection from the shared
/// `ServerState`, so all sessions contend on the SAME [`LeaseRegistry`] and append to the SAME
/// [`ActionLog`].
#[derive(Clone)]
pub struct McpSession {
    /// Legacy non-durable fallback identity; production dispatches use a broker-minted credential.
    agent_id: String,
    /// The per-session HMAC token (the dispatch auth-gates every request against this).
    token: SessionToken,
    /// The shared registry every session contends on for widget/pane leases.
    leases: LeaseRegistry,
    /// The shared append-only audit log of attributed actions.
    log: ActionLog,
    /// Per-acquire lease timeout (configurable so the concurrent test can force the timeout path).
    lease_timeout: Duration,
    connection_id: String,
    durable_receipts: bool,
    receipt_provenance: Option<ArgusReceiptProvenance>,
    agent_credentials: AgentCredentialBroker,
    require_agent_credentials: bool,
    /// Production-only wake for the egui frame loop. A queued action cannot be
    /// drained by `raw_input_hook` while every viewport is idle unless enqueue
    /// requests a frame. The callback is deliberately narrower than a window
    /// activation API: it may schedule repaint, but cannot focus or foreground.
    action_wake: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArgusDurabilityReceipt {
    event_ledger_event_id: String,
    flight_recorder_event_id: Option<uuid::Uuid>,
    flight_recorder_mirrored: bool,
    durable: bool,
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
            connection_id: format!(
                "mcp-connection-{}",
                NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed)
            ),
            durable_receipts: false,
            receipt_provenance: None,
            agent_credentials: AgentCredentialBroker::default(),
            require_agent_credentials: false,
            action_wake: None,
        }
    }

    /// Override the lease timeout (the concurrent test uses a short value to exercise the timeout path
    /// deterministically; production uses [`DEFAULT_LEASE_TIMEOUT`]).
    pub fn with_lease_timeout(mut self, timeout: Duration) -> Self {
        self.lease_timeout = timeout;
        self
    }

    fn with_durable_receipts(mut self, enabled: bool) -> Self {
        self.durable_receipts = enabled;
        self
    }

    fn with_receipt_provenance(mut self, provenance: Option<ArgusReceiptProvenance>) -> Self {
        self.receipt_provenance = provenance;
        self
    }

    fn with_agent_credentials(mut self, broker: AgentCredentialBroker, required: bool) -> Self {
        self.agent_credentials = broker;
        self.require_agent_credentials = required;
        self
    }

    fn with_action_wake(mut self, wake: Option<Arc<dyn Fn(&str) + Send + Sync>>) -> Self {
        self.action_wake = wake;
        self
    }

    /// This session's deterministic agent id.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
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
        if validate_agent_label(request.agent_label()).is_err() {
            return DispatchPlan::Direct;
        }
        match canonical_method(&request.method) {
            Some("argus.click" | "argus.show_context_menu" | "argus.set_value") => {
                match request
                    .params
                    .get("author_id")
                    .or_else(|| request.params.get("target"))
                    .and_then(|v| v.as_str())
                {
                    Some(t) if !t.is_empty() => DispatchPlan::ExclusiveWrite {
                        target: format!(
                            "{}::{t}",
                            request
                                .params
                                .get("window_id")
                                .and_then(|value| value.as_str())
                                .unwrap_or(MAIN_WINDOW_ID)
                        ),
                    },
                    // Missing/empty target is malformed: no lease, let dispatch_request emit -32602.
                    _ => DispatchPlan::Direct,
                }
            }
            Some("argus.inspect" | "argus.list_windows") => DispatchPlan::SharedRead,
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

    /// Live production dispatch: canonical/alias method normalization, window-aware resolution,
    /// per-target fencing, request-level attribution, and an applied/failed receipt after the target
    /// viewport consumes the action and publishes a newer snapshot.
    pub async fn dispatch_argus_shared_async(
        &self,
        request: &McpRequest,
        windows: &WindowSnapshotRegistry,
        channel: &Arc<Mutex<ActionChannel>>,
        capture: impl FnOnce(&ArgusWindowDescriptor) -> Result<ScreenshotResult, ScreenshotError>
            + Send
            + 'static,
    ) -> McpResponse {
        if !self.token.matches(&request.session_token) {
            return McpResponse::error(
                request.id.clone(),
                crate::mcp::tools::McpError {
                    code: crate::mcp::ERR_UNAUTHORIZED,
                    message: "Invalid session token".to_owned(),
                },
            );
        }
        if request.method == "argus.authenticate_agent" {
            if validate_agent_label(request.agent_label()).is_err() {
                return McpResponse::error(
                    request.id.clone(),
                    crate::mcp::tools::McpError {
                        code: crate::mcp::tools::ERR_INVALID_PARAMS,
                        message: "agent_label must be non-empty bounded ASCII graphic text"
                            .to_owned(),
                    },
                );
            }
            let (agent_id, agent_token) = self.agent_credentials.mint();
            return McpResponse::ok_value(
                request.id.clone(),
                serde_json::json!({
                    "agent_id": agent_id,
                    "agent_token": agent_token,
                    "agent_label": request.agent_label(),
                }),
            );
        }
        let authenticated_agent_id = if self.require_agent_credentials {
            match self
                .agent_credentials
                .authenticate(request.agent_credential())
            {
                Some(agent_id) => agent_id,
                None => {
                    return McpResponse::error(
                        request.id.clone(),
                        crate::mcp::tools::McpError {
                            code: crate::mcp::ERR_UNAUTHORIZED,
                            message: "Missing or invalid broker-minted agent token".to_owned(),
                        },
                    )
                }
            }
        } else {
            self.agent_id.clone()
        };
        // Screenshot capture is a compositor/GPU operation, never an ActionChannel mutation. Run it
        // on Tokio's blocking pool and give dispatch_windowed_request a private unused channel so the
        // global mutation-admission mutex is not held across capture. The production capture adapter
        // independently bounds its whole WGC lifecycle to eight seconds.
        if canonical_method(&request.method) == Some("argus.screenshot") {
            let owned_request = request.clone();
            let token = self.token.clone();
            let windows = windows.clone();
            let connection_id = self.connection_id.clone();
            let blocking_capture = tokio::task::spawn_blocking(move || {
                let mut unused_channel = ActionChannel::new();
                dispatch_windowed_request(
                    &owned_request,
                    &token,
                    &windows,
                    &mut unused_channel,
                    &connection_id,
                    &authenticated_agent_id,
                    capture,
                )
            });
            return match tokio::time::timeout(Duration::from_secs(9), blocking_capture).await {
                Ok(Ok(response)) => response,
                Ok(Err(error)) => McpResponse::error(
                    request.id.clone(),
                    ScreenshotError(format!("screenshot blocking worker failed: {error}")).into(),
                ),
                Err(_) => McpResponse::error(
                    request.id.clone(),
                    ScreenshotError(
                        "screenshot request exceeded the 9-second RPC deadline".to_owned(),
                    )
                    .into(),
                ),
            };
        }
        match self.decide(request) {
            DispatchPlan::Direct => {
                let mut channel = lock_channel(channel);
                dispatch_windowed_request(
                    request,
                    &self.token,
                    windows,
                    &mut channel,
                    &self.connection_id,
                    request.agent_label(),
                    capture,
                )
            }
            DispatchPlan::SharedRead => {
                let _guard = match self
                    .leases
                    .acquire_async(SNAPSHOT_RESOURCE, LeaseKind::Shared, self.lease_timeout)
                    .await
                {
                    Ok(guard) => guard,
                    Err(error) => return Self::lease_timeout_response(request, error),
                };
                let mut channel = lock_channel(channel);
                dispatch_windowed_request(
                    request,
                    &self.token,
                    windows,
                    &mut channel,
                    &self.connection_id,
                    request.agent_label(),
                    capture,
                )
            }
            DispatchPlan::ExclusiveWrite { target } => {
                let _guard = match self
                    .leases
                    .acquire_async(&target, LeaseKind::Exclusive, self.lease_timeout)
                    .await
                {
                    Ok(guard) => guard,
                    Err(error) => return Self::lease_timeout_response(request, error),
                };
                let response = {
                    let mut channel = lock_channel(channel);
                    dispatch_windowed_request(
                        request,
                        &self.token,
                        windows,
                        &mut channel,
                        &self.connection_id,
                        request.agent_label(),
                        capture,
                    )
                };
                self.attribute_and_wait(response, request, channel, &authenticated_agent_id)
                    .await
            }
        }
    }

    async fn attribute_and_wait(
        &self,
        response: McpResponse,
        request: &McpRequest,
        channel: &Arc<Mutex<ActionChannel>>,
        authenticated_agent_id: &str,
    ) -> McpResponse {
        let result = match response.result_ref() {
            Ok(result) if result.get("queued").and_then(|value| value.as_bool()) == Some(true) => {
                result.clone()
            }
            _ => return response,
        };
        let Some(action_id) = result.get("action_id").and_then(|value| value.as_str()) else {
            return response;
        };
        // Enqueue occurs on the transport runtime, which may be the only active
        // thread while the native shell is idle. Wake one egui pass immediately
        // so raw_input_hook can drain and causally acknowledge this exact action.
        let window_id = result
            .get("window_id")
            .and_then(|value| value.as_str())
            .unwrap_or(MAIN_WINDOW_ID);
        if let Some(wake) = &self.action_wake {
            wake(window_id);
        }
        let target = result
            .get("target")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let node_id = result
            .get("node_id")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let seq = self.log.record_with_label(
            authenticated_agent_id,
            request.agent_label(),
            &request.method,
            target,
            node_id,
        );
        let tracker = lock_channel(channel).receipt_tracker();
        tracker.set_evidence_ref(action_id, format!("native-action-log://{seq}"));
        let action_id_owned = action_id.to_owned();
        let waiting_tracker = tracker.clone();
        let waited = tokio::task::spawn_blocking(move || {
            waiting_tracker.wait(&action_id_owned, ACTION_RECEIPT_TIMEOUT)
        })
        .await;
        let mut receipt = match waited {
            Ok(Ok(receipt)) => receipt,
            Ok(Err(error)) => {
                let receipt_timed_out = matches!(error, ArgusError::ReceiptTimeout { .. });
                tracker.failed(action_id, error.to_string());
                if receipt_timed_out {
                    // Reclaim synchronously by receipt id. The target viewport may have closed after
                    // enqueue, in which case no matching raw-input pass exists to prune this mutation.
                    lock_channel(channel).discard_action(action_id);
                }
                let receipt = match tracker.wait(action_id, Duration::ZERO) {
                    Ok(receipt) => receipt,
                    Err(_) => {
                        tracker.release_terminal(action_id);
                        return McpResponse::error(request.id.clone(), error.into());
                    }
                };
                if receipt_timed_out {
                    // Atomic timeout has already terminalized + cleaned the receipt. Wake the native
                    // shell so its next pass drops terminal queue/consumed bookkeeping.
                    if let Some(wake) = &self.action_wake {
                        wake(window_id);
                    }
                }
                receipt
            }
            Err(error) => {
                tracker.failed(action_id, format!("receipt waiter failed: {error}"));
                match tracker.wait(action_id, Duration::ZERO) {
                    Ok(receipt) => receipt,
                    Err(receipt_error) => {
                        tracker.release_terminal(action_id);
                        return McpResponse::error(request.id.clone(), receipt_error.into());
                    }
                }
            }
        };
        if !self.durable_receipts {
            let response = McpResponse::ok_value(
                request.id.clone(),
                attributed_receipt_value(&receipt, authenticated_agent_id),
            );
            tracker.release_terminal(action_id);
            return response;
        }
        let Some(provenance) = self.receipt_provenance.clone() else {
            let error = "Argus receipt has no authenticated diagnostics provenance".to_owned();
            tracker.set_durability_error(action_id, error.clone());
            receipt.durability_error = Some(error);
            let response = McpResponse::ok_value(
                request.id.clone(),
                attributed_receipt_value(&receipt, authenticated_agent_id),
            );
            tracker.release_terminal(action_id);
            return response;
        };
        match persist_argus_receipt(&receipt, authenticated_agent_id, provenance).await {
            Ok(durable) if durable.durable => {
                let evidence_ref = durable
                    .flight_recorder_event_id
                    .filter(|_| durable.flight_recorder_mirrored)
                    .map(|event_id| {
                        format!(
                            "eventledger://kernel/{}#flight-recorder/{event_id}",
                            durable.event_ledger_event_id
                        )
                    })
                    .unwrap_or_else(|| {
                        format!("eventledger://kernel/{}", durable.event_ledger_event_id)
                    });
                tracker.set_evidence_ref(action_id, evidence_ref.clone());
                receipt.evidence_ref = Some(evidence_ref);
                crate::internal_diagnostics::record_open(
                    crate::internal_diagnostics::InternalDiagnosticEvent::mechanical(
                        crate::internal_diagnostics::DiagnosticMechanism::GuiAction,
                        crate::internal_diagnostics::DiagnosticEventState::Healthy,
                        None,
                    ),
                );
            }
            Ok(_) => {
                let error = "Argus receipt durability was not acknowledged".to_owned();
                tracker.set_durability_error(action_id, error.clone());
                receipt.durability_error = Some(error);
            }
            Err(error) => {
                let error = format!("Argus durable receipt append failed: {error}");
                tracker.set_durability_error(action_id, error.clone());
                receipt.durability_error = Some(error);
                crate::internal_diagnostics::record_open(
                    crate::internal_diagnostics::InternalDiagnosticEvent::mechanical(
                        crate::internal_diagnostics::DiagnosticMechanism::GuiAction,
                        crate::internal_diagnostics::DiagnosticEventState::Degraded,
                        Some(crate::internal_diagnostics::DiagnosticCode::BackendUnavailable),
                    ),
                );
            }
        }
        let response = McpResponse::ok_value(
            request.id.clone(),
            attributed_receipt_value(&receipt, authenticated_agent_id),
        );
        tracker.release_terminal(action_id);
        response
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
        self.log.record_with_label(
            &self.agent_id,
            request.agent_label(),
            &request.method,
            target,
            node_id,
        );
        // Rebuild the result Value with the acting agent_id added (AC#2).
        let mut stamped = result;
        if let Some(obj) = stamped.as_object_mut() {
            obj.insert(
                "agent_id".to_owned(),
                serde_json::Value::String(self.agent_id.clone()),
            );
        }
        McpResponse::ok_value(request.id.clone(), stamped)
    }
}

fn attributed_receipt_value(
    receipt: &crate::mcp::argus::ArgusActionReceipt,
    authenticated_agent_id: &str,
) -> serde_json::Value {
    let mut value = serde_json::to_value(receipt).unwrap_or_else(
        |_| serde_json::json!({"status": "failed", "error": "receipt serialize failed"}),
    );
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "agent_id".to_owned(),
            serde_json::Value::String(authenticated_agent_id.to_owned()),
        );
    }
    value
}

#[derive(Clone, Default)]
struct AgentCredentialBroker {
    credentials: Arc<Mutex<Vec<(SessionToken, String)>>>,
}

impl AgentCredentialBroker {
    const MAX_ACTIVE_CREDENTIALS: usize = 1_024;

    fn mint(&self) -> (String, String) {
        let token = SessionToken::generate();
        let agent_id = agent_id_for_token(token.as_hex());
        let mut credentials = self
            .credentials
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if credentials.len() >= Self::MAX_ACTIVE_CREDENTIALS {
            credentials.remove(0);
        }
        credentials.push((token.clone(), agent_id.clone()));
        (agent_id, token.as_hex().to_owned())
    }

    fn authenticate(&self, presented: &str) -> Option<String> {
        // Map lookup does not itself provide a constant-time miss path, so first
        // bound the credential shape and then validate the selected token with
        // SessionToken::matches before returning its broker-owned principal.
        if presented.len() != 64 || !presented.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        self.credentials
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .find_map(|(token, principal)| token.matches(presented).then(|| principal.clone()))
    }
}

async fn persist_argus_receipt(
    receipt: &crate::mcp::argus::ArgusActionReceipt,
    authenticated_agent_id: &str,
    provenance: ArgusReceiptProvenance,
) -> Result<ArgusDurabilityReceipt, String> {
    let status = match &receipt.status {
        crate::mcp::argus::ActionReceiptStatus::Applied => "applied",
        crate::mcp::argus::ActionReceiptStatus::Failed => "failed",
        crate::mcp::argus::ActionReceiptStatus::Queued => {
            return Err("queued receipt cannot be persisted as final evidence".to_owned())
        }
    };
    let action = canonical_receipt_action(&receipt.action)?;
    let proof_bytes = argus_proof_bytes(
        provenance.diagnostics_session_id,
        receipt,
        action,
        authenticated_agent_id,
        status,
    );
    let signing_secret = (provenance.signing_secret)()
        .ok_or_else(|| "Palmistry signing secret is not currently authenticated".to_owned())?;
    let mut mac = Hmac::<Sha256>::new_from_slice(signing_secret.as_ref())
        .map_err(|_| "invalid Argus proof key".to_owned())?;
    mac.update(&proof_bytes);
    let proof = lower_hex(&mac.finalize().into_bytes());
    let payload = argus_durable_receipt_payload(
        provenance.diagnostics_session_id,
        receipt,
        action,
        authenticated_agent_id,
        status,
        &proof,
    );
    let response = reqwest::Client::new()
        .post(format!(
            "{}/internal-diagnostics/argus/action-receipt",
            crate::backend_client::BACKEND_BASE_URL
        ))
        .timeout(Duration::from_secs(5))
        .json(&payload)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(format!(
            "HTTP {status}: {}",
            String::from_utf8_lossy(&bytes)
        ));
    }
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn argus_durable_receipt_payload(
    diagnostics_session_id: uuid::Uuid,
    receipt: &crate::mcp::argus::ArgusActionReceipt,
    action: &str,
    authenticated_agent_id: &str,
    status: &str,
    proof: &str,
) -> serde_json::Value {
    serde_json::json!({
        "diagnostics_session_id": diagnostics_session_id,
        "action_id": receipt.action_id,
        "action": action,
        "connection_id": receipt.connection_id,
        "agent_id": authenticated_agent_id,
        "agent_label": receipt.agent_label,
        "window_id": receipt.window_id,
        "author_id": receipt.author_id,
        "before_revision": receipt.before_revision,
        "after_revision": receipt.after_revision,
        "status": status,
        "proof": proof,
    })
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn argus_proof_bytes(
    diagnostics_session_id: uuid::Uuid,
    receipt: &crate::mcp::argus::ArgusActionReceipt,
    action: &str,
    agent_id: &str,
    status: &str,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        diagnostics_session_id.to_string(),
        receipt.action_id.clone(),
        action.to_owned(),
        receipt.connection_id.clone(),
        agent_id.to_owned(),
        receipt.agent_label.clone(),
        receipt.window_id.clone(),
        receipt.author_id.clone(),
        receipt.before_revision.to_string(),
        receipt
            .after_revision
            .map(|value| value.to_string())
            .unwrap_or_default(),
        status.to_owned(),
    ] {
        bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    bytes
}

fn canonical_receipt_action(action: &str) -> Result<&'static str, String> {
    match action {
        "Click" => Ok("argus.click"),
        "ShowContextMenu" => Ok("argus.show_context_menu"),
        "SetValue" => Ok("argus.set_value"),
        other => Err(format!("unsupported final Argus receipt action `{other}`")),
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
    /// Window-keyed live snapshots used by the production Argus transport.
    pub windows: WindowSnapshotRegistry,
    /// The bounded action channel (shared with the egui frame loop).
    pub channel: Arc<Mutex<ActionChannel>>,
    /// Per-acquire lease timeout every connection's [`McpSession`] inherits. Defaults to
    /// [`DEFAULT_LEASE_TIMEOUT`]; the concurrent harness overrides it with a short value to exercise the
    /// lease-timeout path deterministically over the wire.
    pub lease_timeout: Duration,
    durable_receipts: bool,
    receipt_provenance: Option<ArgusReceiptProvenance>,
    agent_credentials: AgentCredentialBroker,
    require_agent_credentials: bool,
    action_wake: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

impl SwarmSafetyState {
    /// Build the shared safety state for a server. Each connection derives its own [`McpSession`] from
    /// this via [`Self::session`]. The lease registry + attribution log are fresh (per-server).
    pub fn new(
        token: SessionToken,
        snapshot: Arc<Mutex<UiTreeSnapshot>>,
        channel: Arc<Mutex<ActionChannel>>,
    ) -> Self {
        let windows = WindowSnapshotRegistry::new();
        let initial_snapshot = snapshot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        windows.publish(
            ArgusWindowDescriptor {
                window_id: MAIN_WINDOW_ID.to_owned(),
                viewport_id: "ROOT".to_owned(),
                title: HANDSHAKE_WINDOW_TITLE.to_owned(),
            },
            initial_snapshot,
        );
        Self {
            token,
            leases: LeaseRegistry::new(),
            log: ActionLog::new(),
            snapshot,
            windows,
            channel,
            lease_timeout: DEFAULT_LEASE_TIMEOUT,
            durable_receipts: false,
            receipt_provenance: None,
            agent_credentials: AgentCredentialBroker::default(),
            require_agent_credentials: false,
            action_wake: None,
        }
    }

    /// Override the per-connection lease timeout (the concurrent harness uses a short value so the
    /// lease-contention path times out deterministically). Returns `self` for chaining.
    pub fn with_lease_timeout(mut self, timeout: Duration) -> Self {
        self.lease_timeout = timeout;
        self
    }

    pub fn with_durable_receipts(mut self, provenance: ArgusReceiptProvenance) -> Self {
        self.durable_receipts = true;
        self.receipt_provenance = Some(provenance);
        self.require_agent_credentials = true;
        self
    }

    /// Install the non-intrusive production frame wake used after a mutation is
    /// actually queued. Tests and headless callers remain wake-free by default.
    pub fn with_action_wake(mut self, wake: Arc<dyn Fn(&str) + Send + Sync>) -> Self {
        self.action_wake = Some(wake);
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
        let windows = WindowSnapshotRegistry::new();
        let initial_snapshot = snapshot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        windows.publish(
            ArgusWindowDescriptor {
                window_id: MAIN_WINDOW_ID.to_owned(),
                viewport_id: "ROOT".to_owned(),
                title: HANDSHAKE_WINDOW_TITLE.to_owned(),
            },
            initial_snapshot,
        );
        Self {
            token,
            leases,
            log,
            snapshot,
            windows,
            channel,
            lease_timeout: DEFAULT_LEASE_TIMEOUT,
            durable_receipts: false,
            receipt_provenance: None,
            agent_credentials: AgentCredentialBroker::default(),
            require_agent_credentials: false,
            action_wake: None,
        }
    }

    pub fn with_window_registry(
        token: SessionToken,
        snapshot: Arc<Mutex<UiTreeSnapshot>>,
        windows: WindowSnapshotRegistry,
        channel: Arc<Mutex<ActionChannel>>,
    ) -> Self {
        Self {
            token,
            leases: LeaseRegistry::new(),
            log: ActionLog::new(),
            snapshot,
            windows,
            channel,
            lease_timeout: DEFAULT_LEASE_TIMEOUT,
            durable_receipts: false,
            receipt_provenance: None,
            agent_credentials: AgentCredentialBroker::default(),
            require_agent_credentials: false,
            action_wake: None,
        }
    }

    /// A session for one accepted connection (shares the registry + log + dispatch state, and inherits
    /// this state's [`Self::lease_timeout`]).
    pub fn session(&self) -> McpSession {
        McpSession::new(self.token.clone(), self.leases.clone(), self.log.clone())
            .with_lease_timeout(self.lease_timeout)
            .with_durable_receipts(self.durable_receipts)
            .with_receipt_provenance(self.receipt_provenance.clone())
            .with_agent_credentials(
                self.agent_credentials.clone(),
                self.require_agent_credentials,
            )
            .with_action_wake(self.action_wake.clone())
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

    #[test]
    fn dynamic_receipt_provenance_reads_rotated_secret_without_rebind() {
        let slot = std::sync::Arc::new(std::sync::Mutex::new(Some(zeroize::Zeroizing::new(
            [1_u8; 32],
        ))));
        let provider_slot = std::sync::Arc::clone(&slot);
        let provenance = ArgusReceiptProvenance::dynamic(
            uuid::Uuid::now_v7(),
            std::sync::Arc::new(move || {
                provider_slot
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .as_ref()
                    .cloned()
            }),
        );
        assert_eq!((provenance.signing_secret)().unwrap().as_ref(), &[1_u8; 32]);
        *slot.lock().unwrap_or_else(|error| error.into_inner()) =
            Some(zeroize::Zeroizing::new([2_u8; 32]));
        assert_eq!((provenance.signing_secret)().unwrap().as_ref(), &[2_u8; 32]);
    }

    fn req(method: &str, params: serde_json::Value, token: &str) -> McpRequest {
        McpRequest {
            id: serde_json::json!(1),
            method: method.to_owned(),
            params,
            session_token: token.to_owned(),
        }
    }

    fn no_capture() -> Result<ScreenshotResult, ScreenshotError> {
        Ok(screenshot_from_png(b"x", 1, 1))
    }

    fn targeted_no_capture(_: &ArgusWindowDescriptor) -> Result<ScreenshotResult, ScreenshotError> {
        no_capture()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn relabelled_mutation_retains_authenticated_principal_in_all_evidence() {
        let token = SessionToken::from_hex("production-root");
        let snapshot = Arc::new(Mutex::new(snap()));
        let channel = Arc::new(Mutex::new(ActionChannel::default()));
        let mut state = SwarmSafetyState::new(token.clone(), snapshot, channel);
        // Enforce broker credentials without making a unit test call the live durability backend. The
        // exact production durable payload builder is asserted after the real mutation completes.
        state.require_agent_credentials = true;
        let first_session = state.session();
        let second_session = state.session();
        let auth = |id: u64, label: &str| {
            McpRequest::from_json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "argus.authenticate_agent",
                "params": {},
                "session_token": token.as_hex(),
                "agent_label": label,
            }))
            .unwrap()
        };
        let first = first_session
            .dispatch_argus_shared_async(
                &auth(1, "agent-one"),
                &state.windows,
                &state.channel,
                targeted_no_capture,
            )
            .await;
        let second = second_session
            .dispatch_argus_shared_async(
                &auth(2, "agent-two"),
                &state.windows,
                &state.channel,
                targeted_no_capture,
            )
            .await;
        let first_result = first.result_ref().unwrap();
        let second_result = second.result_ref().unwrap();
        let first_id = first_result["agent_id"].as_str().unwrap().to_owned();
        let second_id = second_result["agent_id"].as_str().unwrap().to_owned();
        let first_token = first_result["agent_token"].as_str().unwrap().to_owned();
        let second_token = second_result["agent_token"].as_str().unwrap().to_owned();
        assert_ne!(first_id, second_id);
        assert_ne!(first_token, second_token);
        assert_eq!(
            state
                .agent_credentials
                .authenticate(&first_token)
                .as_deref(),
            Some(first_id.as_str())
        );
        assert_eq!(
            state
                .agent_credentials
                .authenticate(&second_token)
                .as_deref(),
            Some(second_id.as_str())
        );
        assert_ne!(
            state
                .agent_credentials
                .authenticate(&first_token)
                .as_deref(),
            Some(second_id.as_str())
        );

        // Principal A intentionally presents the display label used by principal B and performs a real
        // mutation. Credential identity must win everywhere; the label remains display metadata only.
        let spoofed = McpRequest::from_json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "argus.click",
            "params": {
                "window_id": MAIN_WINDOW_ID,
                "author_id": "btn",
                "expected_snapshot_revision": 1
            },
            "session_token": token.as_hex(),
            "agent_token": first_token,
            "agent_label": "agent-two",
        }))
        .unwrap();
        assert_eq!(
            state
                .agent_credentials
                .authenticate(spoofed.agent_credential())
                .as_deref(),
            Some(first_id.as_str()),
            "a caller-controlled display label cannot spoof another broker-minted principal"
        );
        let windows = state.windows.clone();
        let channel = state.channel.clone();
        let dispatch_windows = windows.clone();
        let dispatch_channel = channel.clone();
        let dispatched = tokio::spawn(async move {
            second_session
                .dispatch_argus_shared_async(
                    &spoofed,
                    &dispatch_windows,
                    &dispatch_channel,
                    targeted_no_capture,
                )
                .await
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while channel.lock().unwrap().pending() == 0 {
            assert!(
                std::time::Instant::now() < deadline,
                "relabelled mutation queued"
            );
            tokio::task::yield_now().await;
        }
        let (batch, tracker) = {
            let mut channel = channel.lock().unwrap();
            let tracker = channel.receipt_tracker();
            (channel.drain_for_window(MAIN_WINDOW_ID), tracker)
        };
        assert_eq!(batch.action_ids.len(), 1);
        let action_id = batch.action_ids[0].clone();
        let mut receipt_for_payload = tracker.get(&action_id).expect("queued receipt retained");
        let current = windows.get(MAIN_WINDOW_ID).unwrap();
        let snapshot = current.snapshot.clone();
        let revision = windows.publish(current.window, current.snapshot);
        receipt_for_payload.after_revision = Some(revision);
        receipt_for_payload.status = crate::mcp::argus::ActionReceiptStatus::Applied;
        tracker.acknowledge_effect(&action_id);
        tracker.observe_postcondition(&action_id, revision, &snapshot);

        let response_json = dispatched
            .await
            .expect("relabelled dispatch task")
            .to_json();
        assert_eq!(response_json["result"]["status"], "applied");
        assert_eq!(response_json["result"]["agent_id"], first_id);
        assert_eq!(response_json["result"]["agent_label"], "agent-two");

        let log = state.log.drain_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].agent_id, first_id);
        assert_eq!(log[0].agent_label, "agent-two");
        assert_eq!(log[0].op_name, "argus.click");
        assert_eq!(log[0].target_key, "btn");

        assert!(
            tracker.get(&action_id).is_none(),
            "session releases the terminal receipt after building its response"
        );
        assert_eq!(receipt_for_payload.agent_label, "agent-two");
        let durable_payload = argus_durable_receipt_payload(
            uuid::Uuid::nil(),
            &receipt_for_payload,
            canonical_receipt_action(&receipt_for_payload.action).unwrap(),
            &first_id,
            "applied",
            "proof-canary",
        );
        assert_eq!(durable_payload["agent_id"], first_id);
        assert_eq!(durable_payload["agent_label"], "agent-two");

        let missing_credential = McpRequest::from_json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "argus.list_windows",
            "params": {},
            "session_token": token.as_hex(),
            "agent_label": "agent-one",
        }))
        .unwrap();
        let rejected = state
            .session()
            .dispatch_argus_shared_async(
                &missing_credential,
                &state.windows,
                &state.channel,
                targeted_no_capture,
            )
            .await;
        assert!(
            rejected.is_error_code(crate::mcp::ERR_UNAUTHORIZED),
            "a caller-controlled label is not an authentication credential"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn queued_idle_mutation_wakes_and_drains_the_real_viewport_channel() {
        let token = SessionToken::from_hex("secret");
        let snapshot = Arc::new(Mutex::new(snap()));
        let channel = Arc::new(Mutex::new(ActionChannel::new()));
        let windows = WindowSnapshotRegistry::new();
        windows.publish(
            ArgusWindowDescriptor {
                window_id: MAIN_WINDOW_ID.to_owned(),
                viewport_id: "ROOT".to_owned(),
                title: HANDSHAKE_WINDOW_TITLE.to_owned(),
            },
            snap(),
        );
        let wakes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let wake_channel = Arc::clone(&channel);
        let wake_windows = windows.clone();
        let wake_count = Arc::clone(&wakes);
        let state =
            SwarmSafetyState::with_window_registry(token, snapshot, windows, Arc::clone(&channel))
                .with_action_wake(Arc::new(move |window_id| {
                    assert_eq!(
                        window_id, MAIN_WINDOW_ID,
                        "wake must carry the queued target window"
                    );
                    wake_count.fetch_add(1, Ordering::SeqCst);
                    let (batch, tracker) = {
                        let mut channel = wake_channel
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        let tracker = channel.receipt_tracker();
                        (
                            channel.drain_for_viewport(MAIN_WINDOW_ID, egui::ViewportId::ROOT),
                            tracker,
                        )
                    };
                    assert_eq!(batch.action_ids.len(), 1, "wake drains the queued action");
                    let current = wake_windows.get(MAIN_WINDOW_ID).expect("main snapshot");
                    let rendered = current.snapshot.clone();
                    let revision = wake_windows.publish(current.window, current.snapshot);
                    for action_id in batch.action_ids {
                        crate::mcp::argus::acknowledge_action_effect(
                            &egui::Context::default(),
                            "btn",
                        );
                        tracker.observe_postcondition(&action_id, revision, &rendered);
                    }
                }));

        let response = state
            .session()
            .dispatch_argus_shared_async(
                &argus_req(1, "argus.click", "btn", "idle-agent"),
                &state.windows,
                &state.channel,
                targeted_no_capture,
            )
            .await
            .to_json();

        assert_eq!(response["result"]["status"], "applied");
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
        assert_eq!(channel.lock().unwrap().pending(), 0);

        let mut unauthorized_request = argus_req(2, "argus.click", "btn", "unauthorized-agent");
        unauthorized_request.session_token = "wrong-secret".to_owned();
        let unauthorized = state
            .session()
            .dispatch_argus_shared_async(
                &unauthorized_request,
                &state.windows,
                &state.channel,
                targeted_no_capture,
            )
            .await;
        assert!(unauthorized.is_error_code(crate::mcp::ERR_UNAUTHORIZED));
        assert_eq!(wakes.load(Ordering::SeqCst), 1, "rejection must not wake");

        let full_channel = Arc::new(Mutex::new(ActionChannel::with_capacity(1)));
        full_channel
            .lock()
            .unwrap()
            .enqueue(&snap(), "btn", crate::mcp::action::UiAction::Click)
            .expect("fill bounded action queue");
        let rejected_wakes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let rejected_wakes_for_callback = Arc::clone(&rejected_wakes);
        let full_state = SwarmSafetyState::new(
            SessionToken::from_hex("secret"),
            Arc::new(Mutex::new(snap())),
            full_channel,
        )
        .with_action_wake(Arc::new(move |_| {
            rejected_wakes_for_callback.fetch_add(1, Ordering::SeqCst);
        }));
        let queue_full = full_state
            .session()
            .dispatch_argus_shared_async(
                &argus_req(3, "argus.click", "btn", "queued-agent"),
                &full_state.windows,
                &full_state.channel,
                targeted_no_capture,
            )
            .await;
        assert!(queue_full.is_error_code(crate::mcp::ERR_ACTION_QUEUE_FULL));
        assert_eq!(
            rejected_wakes.load(Ordering::SeqCst),
            0,
            "queue-full rejection must not wake"
        );
    }

    #[test]
    fn accesskit_receipt_actions_map_to_canonical_durable_actions() {
        assert_eq!(canonical_receipt_action("Click").unwrap(), "argus.click");
        assert_eq!(
            canonical_receipt_action("ShowContextMenu").unwrap(),
            "argus.show_context_menu"
        );
        assert_eq!(
            canonical_receipt_action("SetValue").unwrap(),
            "argus.set_value"
        );
        assert!(canonical_receipt_action("Focus").is_err());
    }

    fn argus_req(id: u64, method: &str, author_id: &str, agent_label: &str) -> McpRequest {
        McpRequest::from_json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": {
                "window_id": MAIN_WINDOW_ID,
                "author_id": author_id,
                "expected_snapshot_revision": 1
            },
            "session_token": "secret",
            "agent_label": agent_label
        }))
        .expect("valid Argus request")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn set_value_receipt_timeout_terminalizes_and_drops_the_orphan_action() {
        let wakes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let wake_targets = Arc::new(Mutex::new(Vec::<String>::new()));
        let timeout_wakes = Arc::clone(&wakes);
        let timeout_wake_targets = Arc::clone(&wake_targets);
        let state = SwarmSafetyState::new(
            SessionToken::from_hex("secret"),
            Arc::new(Mutex::new(snap())),
            Arc::new(Mutex::new(ActionChannel::new())),
        )
        .with_action_wake(Arc::new(move |window_id| {
            timeout_wakes.fetch_add(1, Ordering::SeqCst);
            timeout_wake_targets
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(window_id.to_owned());
        }));
        state.windows.publish(
            ArgusWindowDescriptor {
                window_id: "popout-settings".to_owned(),
                viewport_id: format!("{:?}", egui::ViewportId::from_hash_of("settings")),
                title: "Handshake – Settings".to_owned(),
            },
            snap(),
        );
        let mut request = argus_req(1, "argus.set_value", "btn", "timeout-agent");
        request.params.as_object_mut().unwrap().insert(
            "window_id".to_owned(),
            serde_json::Value::String("popout-settings".to_owned()),
        );
        request.params.as_object_mut().unwrap().insert(
            "value".to_owned(),
            serde_json::Value::String("never-rendered".to_owned()),
        );

        // No viewport drains this action. The production-bounded receipt waiter must terminalize it;
        // a later viewport pass must then discard it without injecting Focus or Text.
        let response = state
            .session()
            .dispatch_argus_shared_async(
                &request,
                &state.windows,
                &state.channel,
                targeted_no_capture,
            )
            .await
            .to_json();
        assert_eq!(response["result"]["status"], "failed", "{response}");
        assert!(
            response["result"]["error"]
                .as_str()
                .is_some_and(|error| error.contains("timed out")),
            "{response}"
        );
        let action_id = response["result"]["action_id"]
            .as_str()
            .expect("failed receipt action id");
        let tracker = lock_channel(&state.channel).receipt_tracker();
        assert!(tracker.is_terminal(action_id));
        assert_eq!(tracker.pending_postcondition_count(), 0);
        assert_eq!(crate::mcp::argus::registered_action_effect_count(), 0);
        assert_eq!(
            wakes.load(Ordering::SeqCst),
            2,
            "enqueue and atomic timeout each wake the native shell"
        );
        assert_eq!(
            *wake_targets
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec!["popout-settings", "popout-settings"],
            "both enqueue and timeout target the exact detached viewport"
        );

        assert!(
            tracker.get(action_id).is_none(),
            "session releases terminal timeout receipt after response correlation"
        );

        assert_eq!(
            lock_channel(&state.channel).pending(),
            0,
            "timeout synchronously reclaims a mutation even when its detached viewport vanished"
        );
        let batch =
            lock_channel(&state.channel).drain_for_viewport(MAIN_WINDOW_ID, egui::ViewportId::ROOT);
        assert!(batch.events.is_empty(), "timed-out SetValue was injected");
        assert!(batch.action_ids.is_empty());
        assert_eq!(lock_channel(&state.channel).pending(), 0);
        assert_eq!(crate::mcp::argus::registered_action_effect_count(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn same_target_mutation_is_fenced_until_first_receipt_completes() {
        let state = SwarmSafetyState::new(
            SessionToken::from_hex("secret"),
            Arc::new(Mutex::new(snap())),
            Arc::new(Mutex::new(ActionChannel::new())),
        )
        .with_lease_timeout(Duration::from_millis(20));
        let windows = state.windows.clone();
        let channel = state.channel.clone();
        let first_session = state.session();
        let second_session = state.session();
        let first_request = argus_req(1, "argus.click", "btn", "first-agent");
        let second_request = argus_req(2, "argus.click", "btn", "second-agent");

        let first_windows = windows.clone();
        let first_channel = channel.clone();
        let first = tokio::spawn(async move {
            first_session
                .dispatch_argus_shared_async(
                    &first_request,
                    &first_windows,
                    &first_channel,
                    targeted_no_capture,
                )
                .await
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while channel.lock().unwrap().pending() == 0 {
            assert!(std::time::Instant::now() < deadline, "first action queued");
            tokio::task::yield_now().await;
        }

        let second = second_session
            .dispatch_argus_shared_async(&second_request, &windows, &channel, targeted_no_capture)
            .await;
        assert!(second.is_error_code(ERR_LEASE_TIMEOUT));

        let (batch, tracker) = {
            let mut channel = channel.lock().unwrap();
            let tracker = channel.receipt_tracker();
            (channel.drain_for_window(MAIN_WINDOW_ID), tracker)
        };
        let current = windows.get(MAIN_WINDOW_ID).unwrap();
        let snapshot = current.snapshot.clone();
        let revision = windows.publish(current.window, current.snapshot);
        for action_id in batch.action_ids {
            tracker.acknowledge_effect(&action_id);
            tracker.observe_postcondition(&action_id, revision, &snapshot);
        }
        let first = first.await.expect("first dispatch task");
        assert_eq!(first.to_json()["result"]["status"], "applied");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn different_targets_can_wait_for_applied_receipts_concurrently() {
        let mut snapshot = snap();
        let mut second = snapshot.root.children[0].clone();
        second.id = "btn-2".to_owned();
        second.author_id = Some("btn-2".to_owned());
        second.node_id = 11;
        snapshot.root.children.push(second);
        snapshot.widget_count += 1;
        let state = SwarmSafetyState::new(
            SessionToken::from_hex("secret"),
            Arc::new(Mutex::new(snapshot)),
            Arc::new(Mutex::new(ActionChannel::new())),
        );
        let windows = state.windows.clone();
        let channel = state.channel.clone();
        let requests = [
            argus_req(1, "argus.click", "btn", "agent-one"),
            argus_req(2, "argus.click", "btn-2", "agent-two"),
        ];
        let mut tasks = Vec::new();
        for request in requests {
            let session = state.session();
            let windows = windows.clone();
            let channel = channel.clone();
            tasks.push(tokio::spawn(async move {
                session
                    .dispatch_argus_shared_async(&request, &windows, &channel, targeted_no_capture)
                    .await
            }));
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while channel.lock().unwrap().pending() < 2 {
            assert!(std::time::Instant::now() < deadline, "both actions queued");
            tokio::task::yield_now().await;
        }
        let (batch, tracker) = {
            let mut channel = channel.lock().unwrap();
            let tracker = channel.receipt_tracker();
            (channel.drain_for_window(MAIN_WINDOW_ID), tracker)
        };
        assert_eq!(batch.action_ids.len(), 2);
        let current = windows.get(MAIN_WINDOW_ID).unwrap();
        let snapshot = current.snapshot.clone();
        let revision = windows.publish(current.window, current.snapshot);
        for action_id in batch.action_ids {
            tracker.acknowledge_effect(&action_id);
            tracker.observe_postcondition(&action_id, revision, &snapshot);
        }
        for task in tasks {
            assert_eq!(
                task.await.expect("dispatch task").to_json()["result"]["status"],
                "applied"
            );
        }
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
