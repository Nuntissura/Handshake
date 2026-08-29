//! Per-connection MCP session: attribution + leasing applied consistently (WP-KERNEL-011 MT-028).
//!
//! MT-027's [`crate::mcp::tools::dispatch_request`] turns one parsed request into one response with auth
//! + the four tools. [`McpSession`] is the MT-028 wrapper that makes the SAME dispatch safe under N concurrent agents.
//!
//! 1. Every accepted connection gets one `McpSession`. The token-derived id is the base identity;
//!    production qualifies it by a stable request `client_session_id` when supplied, otherwise by the
//!    connection sequence, so every action is attributable without logging the token.
//! 2. A mutating tool (`click_widget` / `set_value`) acquires an EXCLUSIVE lease on its target widget
//!    key before the action is enqueued, and a reading tool (`list_widgets`) acquires a SHARED lease on
//!    the snapshot resource — so two agents cannot drive the same widget at once, but many can read
//!    concurrently (the contract's lease granularity).
//! 3. After a mutating tool successfully enqueues, the action is APPENDED to the shared
//!    [`crate::mcp::attribution::ActionLog`] with this session's `agent_id` — the post-hoc audit trail.
//!
//! The registry lease protects the bounded resolve+enqueue wait. [`ActionChannel`] keeps a per-target
//! transaction in flight through fresh-tree revalidation and post-render acknowledgement; a second
//! write to that target waits above the channel mutex until the transaction terminalizes or the single
//! request deadline returns typed -32004. Non-conflicting targets remain independently queueable.
//!
//! ## Why the lease key is the widget `author_id`
//!
//! A model addresses a widget by its stable `author_id` (the MT-025 convention); that same string is the
//! lease resource key, so two agents targeting the SAME widget contend on the SAME lease, while agents
//! targeting DIFFERENT widgets never contend (fine-grained, low-contention — the contract's design).
//! `list_widgets` is a whole-tree read, so it leases the single [`SNAPSHOT_RESOURCE`] key shared, which
//! only blocks while some op holds it exclusively (none currently does; reserved for a future snapshot
//! write).

use std::sync::{Arc, Mutex, TryLockError};
use std::time::Duration;

use crate::accessibility::UiTreeSnapshot;
use crate::mcp::action::ActionChannel;
use crate::mcp::argus::ArgusMethod;
use crate::mcp::attribution::{agent_id_for_token, ActionLog};
use crate::mcp::leases::{LeaseError, LeaseKind, LeaseRegistry, DEFAULT_LEASE_TIMEOUT};
use crate::mcp::screenshot::{ScreenshotError, ScreenshotResult};
use crate::mcp::tools::{dispatch_request, McpError, McpRequest, McpResponse, SessionToken};

#[cfg(test)]
use crate::mcp::tools::ERR_LEASE_TIMEOUT;

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

/// How long to wait between re-attempts when the exclusive lease was WON but the ActionChannel still
/// holds an un-acknowledged transaction for the same target (MT-134).
///
/// Short enough that a freed slot is taken promptly, long enough that a contended target does not
/// spin the tokio worker. The retry is bounded by the LEASE deadline, never by this interval.
const CHANNEL_BUSY_RETRY_INTERVAL: Duration = Duration::from_millis(2);

/// The capture closure is unreachable on the mutating lease path.
///
/// decide() routes ONLY click_widget / set_value to ExclusiveWrite, and dispatch_request invokes its
/// capture argument solely for the screenshot method. Passing this makes that reachability argument
/// explicit, and lets the enqueue be RETRIED - which a moved-in FnOnce could not be. If the routing
/// ever changed, this returns a typed error rather than silently producing an empty capture.
fn unreachable_capture() -> Result<ScreenshotResult, ScreenshotError> {
    Err(ScreenshotError(
        "screenshot capture is not reachable from the mutating lease path".to_owned(),
    ))
}

/// The leasing + attribution decision for one request, computed ONCE from the auth-gated request so the
/// synchronous [`McpSession::dispatch`] and the async [`McpSession::dispatch_shared_async`] entry points
/// share IDENTICAL auth-gate + target-extraction + lease-kind-selection + attribute-vs-passthrough logic
/// (DRY — the two paths differ ONLY in how they acquire the lease and lock the channel, never in the
/// decision). [`McpSession::decide`] returns this; each entry point then does its own sync/async lease
/// acquire + enqueue.
enum DispatchPlan {
    /// Auth failed, no `target`/lease applies, or an unknown method: dispatch directly with NO lease and
    /// NO attribution (auth errors, malformed-param errors, unknown methods). The canonical
    /// error/response shape comes from [`dispatch_request`].
    Direct,
    /// A screenshot request: run the potentially blocking OS capture off the async worker and never
    /// hold the shared action-channel mutex while it executes.
    Screenshot,
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

/// One MCP connection's session: a deterministic `agent_id`, plus clones of the shared lease registry,
/// action log, session token, and the dispatch state (snapshot + channel) the tools act on.
///
/// Cloneable-by-construction over `Arc`s: the server builds one per accepted connection from the shared
/// `ServerState`, so all sessions contend on the SAME [`LeaseRegistry`] and append to the SAME
/// [`ActionLog`].
#[derive(Clone)]
pub struct McpSession {
    /// The short deterministic per-session id (first 8 hex of SHA-256(token)).
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
        Self::new_with_agent_id(token, leases, log, agent_id)
    }

    /// Build a connection-attributed session while retaining the server token as the authentication
    /// authority. Production uses this so parallel clients discovering one canonical app binding never
    /// collapse into one audit identity merely because they authenticate with the same app token.
    pub(crate) fn new_with_agent_id(
        token: SessionToken,
        leases: LeaseRegistry,
        log: ActionLog,
        agent_id: String,
    ) -> Self {
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
    /// - `screenshot` -> [`DispatchPlan::Screenshot`]; unknown methods -> [`DispatchPlan::Direct`].
    fn decide(&self, request: &McpRequest) -> DispatchPlan {
        // Auth-gate BEFORE any lease/channel work so an unauthorized flood cannot even contend for leases.
        if !self.token.matches(&request.session_token) {
            return DispatchPlan::Direct;
        }
        match ArgusMethod::from_wire_name(&request.method) {
            Some(ArgusMethod::Click | ArgusMethod::SetValue) => {
                match request.params.get("target").and_then(|v| v.as_str()) {
                    Some(t) if !t.is_empty() => DispatchPlan::ExclusiveWrite {
                        target: t.to_owned(),
                    },
                    // Missing/empty target is malformed: no lease, let dispatch_request emit -32602.
                    _ => DispatchPlan::Direct,
                }
            }
            Some(ArgusMethod::Inspect) => DispatchPlan::SharedRead,
            Some(ArgusMethod::Screenshot) => DispatchPlan::Screenshot,
            // Unknown methods: no shared-widget mutation, so no lease.
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
    /// The registry lease guard drops at the end of this call. A successful mutation remains protected
    /// by the action channel's per-target in-flight transaction until post-render acknowledgement.
    pub fn dispatch(
        &self,
        request: &McpRequest,
        snapshot: &UiTreeSnapshot,
        channel: &mut ActionChannel,
        capture: impl FnOnce() -> Result<ScreenshotResult, ScreenshotError>,
    ) -> McpResponse {
        match self.decide(request) {
            DispatchPlan::Screenshot => {
                // The synchronous/in-process path retains its caller-defined capture semantics. The
                // async server path below adds the production timeout boundary.
                dispatch_request(request, &self.token, snapshot, channel, capture)
            }
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
        snapshot: &Arc<Mutex<UiTreeSnapshot>>,
        channel: &Arc<Mutex<ActionChannel>>,
        capture: impl FnOnce() -> Result<ScreenshotResult, ScreenshotError> + Send + 'static,
    ) -> McpResponse {
        match self.decide(request) {
            DispatchPlan::Screenshot => {
                // Windows PrintWindow/WM_PRINT/GDI capture is inherently synchronous and can stall on
                // driver/window-manager behavior. Never execute it on the app's sole Tokio worker and
                // never hold the shared ActionChannel while it runs: either the blocking worker returns
                // within the boundary or the request receives a typed error while inspect/click/value
                // traffic remains serviceable.
                const SCREENSHOT_CAPTURE_TIMEOUT: Duration = Duration::from_secs(2);
                let capture_result = match tokio::time::timeout(
                    SCREENSHOT_CAPTURE_TIMEOUT,
                    tokio::task::spawn_blocking(capture),
                )
                .await
                {
                    Ok(Ok(result)) => result,
                    Ok(Err(error)) => Err(ScreenshotError(format!(
                        "screenshot capture worker failed: {error}"
                    ))),
                    Err(_) => Err(ScreenshotError(format!(
                        "screenshot capture exceeded the {} ms production boundary",
                        SCREENSHOT_CAPTURE_TIMEOUT.as_millis()
                    ))),
                };
                let snapshot = clone_snapshot(snapshot);
                // Screenshot does not resolve or enqueue an action, so a private empty channel preserves
                // the canonical dispatch/auth/response shape without contending with live UI actions.
                let mut private_channel = ActionChannel::new();
                dispatch_request(
                    request,
                    &self.token,
                    &snapshot,
                    &mut private_channel,
                    move || capture_result,
                )
            }
            DispatchPlan::Direct => {
                let snapshot = clone_snapshot(snapshot);
                let mut ch = lock_channel(channel);
                dispatch_request(request, &self.token, &snapshot, &mut ch, capture)
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
                let snapshot = clone_snapshot(snapshot);
                let mut ch = lock_channel(channel);
                dispatch_request(request, &self.token, &snapshot, &mut ch, capture)
            }
            DispatchPlan::ExclusiveWrite { target } => {
                // One request deadline covers BOTH serializer windows. Starting it before registry
                // acquisition prevents a request from consuming `lease_timeout` in the registry and
                // then receiving a second full timeout while the ActionChannel is still busy.
                let deadline = std::time::Instant::now() + self.lease_timeout;
                // LEASE FIRST (async wait — yields the worker thread; never holds the channel lock here).
                let _guard = match self
                    .leases
                    .acquire_async(
                        &target,
                        LeaseKind::Exclusive,
                        deadline.saturating_duration_since(std::time::Instant::now()),
                    )
                    .await
                {
                    Ok(g) => g,
                    Err(e) => return Self::lease_timeout_response(request, e),
                };
                // LeaseRegistry performs a final grant attempt before its own elapsed check. Enforce
                // this request's absolute deadline after acquisition as well, then let the guard drop.
                if std::time::Instant::now() >= deadline {
                    return Self::lease_timeout_response(
                        request,
                        LeaseError::Timeout {
                            resource: target,
                            kind: LeaseKind::Exclusive,
                        },
                    );
                }
                // MT-134: the lease is not the only serializer, and the two do not share a window.
                //
                // Winning the lease is NOT sufficient to enqueue. The ActionChannel keeps its own
                // per-target in-flight transaction until POST-RENDER acknowledgement, and it fails
                // FAST: `enqueue` scans queue + in_flight and returns `TargetBusy` immediately
                // (mcp/action.rs), which maps to the same -32004 as a lease timeout. Because the
                // lease guard is released as soon as this arm returns, the NEXT agent could win the
                // lease while the previous action was still un-acknowledged, and then be rejected by
                // the channel. The wait covered the wrong window, which is why five concurrent
                // agents on one widget lost actions despite a generous lease timeout.
                //
                // Retrying under the HELD lease closes the gap without changing either mechanism's
                // semantics: this agent already owns the target exclusively, so no other agent can
                // interleave, and the channel lock is released between attempts so the egui frame
                // loop can actually acknowledge the in-flight action and free the slot. The retry is
                // bounded by the SAME lease deadline, so a genuinely wedged target still yields
                // -32004 rather than hanging.
                //
                // The channel lock is never held across an await — that would deadlock against the
                // frame loop that must acknowledge.
                let response = loop {
                    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                    if remaining.is_zero() {
                        break Self::lease_timeout_response(
                            request,
                            LeaseError::Timeout {
                                resource: target.clone(),
                                kind: LeaseKind::Exclusive,
                            },
                        );
                    }

                    let response = match try_dispatch_mutation(
                        request,
                        &self.token,
                        snapshot,
                        channel,
                        deadline,
                    ) {
                        MutationDispatchAttempt::SnapshotContended => {
                            tokio::time::sleep(CHANNEL_BUSY_RETRY_INTERVAL.min(remaining)).await;
                            continue;
                        }
                        MutationDispatchAttempt::ChannelContended => {
                            tokio::time::sleep(CHANNEL_BUSY_RETRY_INTERVAL.min(remaining)).await;
                            continue;
                        }
                        MutationDispatchAttempt::DeadlineElapsed => {
                            break Self::lease_timeout_response(
                                request,
                                LeaseError::Timeout {
                                    resource: target.clone(),
                                    kind: LeaseKind::Exclusive,
                                },
                            );
                        }
                        MutationDispatchAttempt::Response(response) => response,
                    };
                    if !Self::is_target_busy(&response) {
                        break response;
                    }
                    // No channel guard survives this await. The next loop iteration performs both an
                    // absolute-deadline check and a fresh snapshot read before retrying.
                    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                    if remaining.is_zero() {
                        break response;
                    }
                    tokio::time::sleep(CHANNEL_BUSY_RETRY_INTERVAL.min(remaining)).await;
                };
                self.attribute_and_stamp(response, request, &target)
                // _guard drops here.
            }
        }
    }

    /// Does this response mean the ActionChannel still holds an un-acknowledged transaction for the
    /// target? The internal producer discriminator survives mapping to public wire code -32004, so
    /// registry timeouts or future same-code errors cannot silently become retryable.
    fn is_target_busy(response: &McpResponse) -> bool {
        matches!(response.result_ref(), Err(error) if error.is_action_target_busy())
    }
    /// Build the typed [`crate::mcp::tools::ERR_LEASE_TIMEOUT`] (-32004) response for a contended lease.
    fn lease_timeout_response(
        request: &McpRequest,
        e: crate::mcp::leases::LeaseError,
    ) -> McpResponse {
        McpResponse::error(request.id.clone(), McpError::lease_timeout(e.to_string()))
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
        let op_name = ArgusMethod::from_wire_name(&request.method)
            .map(ArgusMethod::canonical_name)
            .unwrap_or(&request.method);
        self.log.record(&self.agent_id, op_name, target, node_id);
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

/// A synchronous mutation attempt owns every `std::sync::MutexGuard` it creates and returns only an
/// owned outcome. Keeping this boundary non-async makes it structurally impossible for a channel guard
/// to be retained in `dispatch_shared_async` across its retry sleep.
enum MutationDispatchAttempt {
    SnapshotContended,
    ChannelContended,
    DeadlineElapsed,
    Response(McpResponse),
}

fn try_dispatch_mutation(
    request: &McpRequest,
    token: &SessionToken,
    snapshot: &Arc<Mutex<UiTreeSnapshot>>,
    channel: &Arc<Mutex<ActionChannel>>,
    deadline: std::time::Instant,
) -> MutationDispatchAttempt {
    // Hold the snapshot guard through channel acquisition + dispatch. This makes the old-tree
    // resolution and enqueue one snapshot->channel critical section with the UI's post-render
    // publication/acknowledgement handoff; the UI can linearize before or after it, never between a
    // stale clone and enqueue. Both mutexes are nonblocking so contention remains inside the caller's
    // absolute request deadline, and every guard drops before the retry await.
    let current_snapshot = match snapshot.try_lock() {
        Ok(snapshot) => snapshot,
        Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
        Err(TryLockError::WouldBlock) => return MutationDispatchAttempt::SnapshotContended,
    };
    let mut ch = match channel.try_lock() {
        Ok(ch) => ch,
        Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
        Err(TryLockError::WouldBlock) => return MutationDispatchAttempt::ChannelContended,
    };
    // A screenshot/capture or frame holder may have owned the global channel mutex through the
    // deadline. Acquiring it late must not authorize a late enqueue.
    if std::time::Instant::now() >= deadline {
        return MutationDispatchAttempt::DeadlineElapsed;
    }
    MutationDispatchAttempt::Response(dispatch_request(
        request,
        token,
        &current_snapshot,
        &mut ch,
        unreachable_capture,
    ))
}

/// Lock the shared channel for the minimum span, recovering a poisoned lock (a prior holder panicked
/// while holding it) so one agent's panic cannot wedge every other connection's enqueue path.
fn lock_channel(channel: &Arc<Mutex<ActionChannel>>) -> std::sync::MutexGuard<'_, ActionChannel> {
    channel
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Clone the latest complete UI snapshot while preserving poison recovery. Mutation retries call this
/// for every attempt so a target changed by the previous terminalized action is never resolved against
/// the pre-wait tree.
fn clone_snapshot(snapshot: &Arc<Mutex<UiTreeSnapshot>>) -> UiTreeSnapshot {
    snapshot
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
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

    pub(crate) fn session_with_agent_id(&self, agent_id: String) -> McpSession {
        McpSession::new_with_agent_id(
            self.token.clone(),
            self.leases.clone(),
            self.log.clone(),
            agent_id,
        )
        .with_lease_timeout(self.lease_timeout)
    }

    pub(crate) fn session_for_connection(&self, connection_id: u64) -> McpSession {
        let token_agent_id = agent_id_for_token(self.token.as_hex());
        self.session_with_agent_id(format!("{token_agent_id}:connection-{connection_id}"))
    }

    pub(crate) fn session_for_client(&self, client_session_id: &str) -> McpSession {
        let token_agent_id = agent_id_for_token(self.token.as_hex());
        self.session_with_agent_id(format!("{token_agent_id}:client:{client_session_id}"))
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
            viewport: None,
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
                "argus.click",
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
        assert_eq!(entries[0].op_name, "argus.click");
        assert_eq!(entries[0].target_key, "btn");
        assert_eq!(entries[0].node_id, 10);
        // The lease is released after dispatch (no resource left held).
        assert_eq!(state.leases().active_resource_count(), 0);
    }

    #[test]
    fn released_registry_lease_does_not_admit_same_target_before_render_ack() {
        let token = SessionToken::from_hex("secret");
        let state = SwarmSafetyState::new(
            token.clone(),
            Arc::new(Mutex::new(snap())),
            Arc::new(Mutex::new(ActionChannel::new())),
        );
        let session = state.session();
        let mut snapshot = snap();
        let target = &mut snapshot.root.children[0];
        target.id = "field".to_owned();
        target.author_id = Some("field".to_owned());
        target.role = "TextInput".to_owned();
        target.value = Some("before".to_owned());
        target.actions = vec!["SetValue".to_owned()];
        let mut channel = ActionChannel::new();

        let first = session.dispatch(
            &req(
                "argus.set_value",
                serde_json::json!({"target": "field", "value": "one"}),
                "secret",
            ),
            &snapshot,
            &mut channel,
            no_capture,
        );
        assert_eq!(first.to_json()["result"]["queued"], true);
        assert_eq!(state.leases().active_resource_count(), 0);

        let overlapping = session.dispatch(
            &req(
                "argus.set_value",
                serde_json::json!({"target": "field", "value": "two"}),
                "secret",
            ),
            &snapshot,
            &mut channel,
            no_capture,
        );
        assert_eq!(overlapping.to_json()["error"]["code"], ERR_LEASE_TIMEOUT);
        assert_eq!(channel.drain_revalidated_into_events(&snapshot).len(), 1);
        snapshot.root.children[0].value = Some("one".to_owned());
        channel.acknowledge_after_render(&snapshot);

        let after_ack = session.dispatch(
            &req(
                "argus.set_value",
                serde_json::json!({"target": "field", "value": "two"}),
                "secret",
            ),
            &snapshot,
            &mut channel,
            no_capture,
        );
        assert_eq!(after_ack.to_json()["result"]["queued"], true);
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
            &req("argus.inspect", serde_json::json!({}), "secret"),
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
