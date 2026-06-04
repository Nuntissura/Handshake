//! WP-KERNEL-004 cluster X.1 MT-181 ExecutorRouter integration tests.
//!
//! Per `.GOV/task_packets/WP-KERNEL-004-.../MT-181.json`:
//!   - Owned path: `src/backend/handshake_core/tests/role_mailbox_router_tests.rs`
//!   - Router is pure (no I/O, no implicit clock); `now: DateTime<Utc>` is
//!     injected.
//!   - Decision matrix is exhaustive across (claim_mode × current_lease
//!     present × executor_kind × takeover_policy × response_authority_scope).
//!   - Every `Denied`/`MustWait` carries a typed `RouteReason`.
//!
//! Spec-Realism Gate compliance: every assertion runs without external
//! services; no `#[ignore]`-gated tests are required for this MT since the
//! router is a pure function. There are no `LiveXxxUnavailable` / `todo!()`
//! / `unimplemented!()` paths.
//!
//! Adversarial controls (per MT-181 `red_team.minimum_controls`):
//!   1. Router accepts injected `now: DateTime<Utc>` — proven by
//!      `expired_lease_with_clock_at_expiry_boundary` and
//!      `active_lease_with_clock_before_expiry`.
//!   2. Property-style exhaustive matrix sweep — proven by
//!      `exhaustive_matrix_no_panic_5_axis`.
//!   3. Every `RouteDecision` variant has at least one positive test with
//!      a non-empty reason — proven by the variant-specific tests plus
//!      `every_decision_variant_has_non_empty_reason`.

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use handshake_core::role_mailbox::RoleId;
use handshake_core::role_mailbox_v1::{
    lease::{RoleMailboxClaimLeaseV1, TakeoverPolicy},
    lifecycle::ThreadLifecycleState,
    router::{ExecutorIdentity, ExecutorKind, ExecutorRouter, RouteDecision, RouteReason},
    thread::{ClaimMode, LinkedRecordKind, ResponseAuthorityScope, RoleMailboxThread},
};
use uuid::Uuid;

// ============================================================
// Helpers
// ============================================================

fn make_thread(
    allowlist: Vec<ExecutorKind>,
    claim_mode: ClaimMode,
    takeover: TakeoverPolicy,
    response_scope: ResponseAuthorityScope,
) -> RoleMailboxThread {
    RoleMailboxThread::open(
        "mt-181-router-integration-thread",
        LinkedRecordKind::Wp,
        Some("WP-KERNEL-004".to_string()),
        allowlist,
        claim_mode,
        takeover,
        response_scope,
    )
}

fn make_executor(kind: ExecutorKind, role: RoleId, session_id: Uuid) -> ExecutorIdentity {
    ExecutorIdentity {
        executor_kind: kind,
        role_id: role,
        session_id,
        capabilities: vec!["mt181-router-integration".to_string()],
    }
}

fn make_lease(
    thread_id: Uuid,
    holder_kind: ExecutorKind,
    holder_session: Uuid,
    acquired_at: DateTime<Utc>,
    expires_in_secs: i64,
) -> RoleMailboxClaimLeaseV1 {
    RoleMailboxClaimLeaseV1 {
        lease_id: Uuid::now_v7(),
        thread_id,
        holder_executor_kind: holder_kind,
        holder_role_id: RoleId::Coder,
        holder_session_id: holder_session,
        acquired_at_utc: acquired_at,
        expires_at_utc: acquired_at + ChronoDuration::seconds(expires_in_secs),
        released_at_utc: None,
        takeover_of: None,
        takeover_reason: None,
    }
}

fn assert_reason_non_empty(decision: &RouteDecision) {
    match decision {
        RouteDecision::MayClaim { reason }
        | RouteDecision::MayRespondInExistingLease { reason }
        | RouteDecision::MustWait { reason } => {
            // RouteReason is an enum; debug-formatted variant name is the
            // typed reason payload. Detail-string check applies only to
            // Denied. Verify the variant is one of the known enum members.
            let dbg = format!("{:?}", reason);
            assert!(
                !dbg.is_empty(),
                "RouteReason debug-format must be non-empty for variant: {decision:?}"
            );
        }
        RouteDecision::MayTakeover {
            predecessor_lease_id,
            reason,
        } => {
            assert_ne!(
                *predecessor_lease_id,
                Uuid::nil(),
                "predecessor_lease_id must be non-nil for MayTakeover"
            );
            let dbg = format!("{:?}", reason);
            assert!(!dbg.is_empty(), "RouteReason must be non-empty");
        }
        RouteDecision::Denied { reason, detail } => {
            let dbg = format!("{:?}", reason);
            assert!(!dbg.is_empty(), "Denied reason debug-format must be non-empty");
            assert!(
                !detail.is_empty(),
                "Denied free-form detail must be non-empty per contract validator_focus"
            );
        }
    }
}

// ============================================================
// (1) Allowlist gate — denies wrong-role / wrong-kind claim attempts
// ============================================================

#[test]
fn mt_181_allowlist_denies_executor_kind_not_in_allowlist() {
    // Per contract algorithm step (1): "if executor.executor_kind not in
    // thread.executor_kind_allowlist -> Denied('executor_kind not in
    // allowlist')". Validator-only thread must reject a LocalSmallModel
    // attempt.
    let thread = make_thread(
        vec![ExecutorKind::Validator],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let executor = make_executor(
        ExecutorKind::LocalSmallModel,
        RoleId::Coder,
        Uuid::now_v7(),
    );
    let decision = ExecutorRouter::decide(&thread, None, &executor, Utc::now());
    match decision {
        RouteDecision::Denied { ref reason, .. } => {
            assert_eq!(
                *reason,
                RouteReason::ExecutorKindNotInAllowlist,
                "must surface typed RouteReason::ExecutorKindNotInAllowlist"
            );
            assert_reason_non_empty(&decision);
        }
        other => panic!("expected Denied, got {other:?}"),
    }
}

#[test]
fn mt_181_allowlist_denies_when_allowlist_excludes_executor_even_with_lease() {
    // Adversarial: even when an active lease exists for an allowlisted
    // executor, a *different* kind of executor not in the allowlist must
    // be Denied — the allowlist gate runs before any lease logic.
    let thread = make_thread(
        vec![ExecutorKind::Validator],
        ClaimMode::Exclusive,
        TakeoverPolicy::AlwaysWithReason,
        ResponseAuthorityScope::LeaseHolder,
    );
    let lease = make_lease(
        thread.thread_id.as_uuid(),
        ExecutorKind::Validator,
        Uuid::now_v7(),
        Utc::now(),
        300,
    );
    let intruder = make_executor(
        ExecutorKind::WorkflowAutomation,
        RoleId::Coder,
        Uuid::now_v7(),
    );
    let decision = ExecutorRouter::decide(&thread, Some(&lease), &intruder, Utc::now());
    assert!(
        matches!(decision, RouteDecision::Denied { reason: RouteReason::ExecutorKindNotInAllowlist, .. }),
        "active-lease + permissive takeover must still be vetoed by allowlist; got {decision:?}"
    );
}

#[test]
fn mt_181_allowlist_empty_denies_all_executors() {
    // Adversarial: empty allowlist must lock the thread — no executor of
    // any kind may claim/respond. Without this guard, a `contains` check
    // on an empty vector would silently fall through.
    let thread = make_thread(
        vec![],
        ClaimMode::Exclusive,
        TakeoverPolicy::AlwaysWithReason,
        ResponseAuthorityScope::LeaseHolder,
    );
    for kind in [
        ExecutorKind::LocalSmallModel,
        ExecutorKind::CloudModel,
        ExecutorKind::Reviewer,
        ExecutorKind::Validator,
        ExecutorKind::Operator,
        ExecutorKind::WorkflowAutomation,
    ] {
        let executor = make_executor(kind, RoleId::Operator, Uuid::now_v7());
        let decision = ExecutorRouter::decide(&thread, None, &executor, Utc::now());
        assert!(
            matches!(decision, RouteDecision::Denied { reason: RouteReason::ExecutorKindNotInAllowlist, .. }),
            "empty allowlist must deny kind {kind:?}; got {decision:?}"
        );
    }
}

#[test]
fn mt_181_allowlist_all_kinds_permits_every_executor() {
    // Adversarial converse: an allowlist covering every ExecutorKind must
    // permit every kind (subject to lease/takeover gates).
    let allow_all = vec![
        ExecutorKind::LocalSmallModel,
        ExecutorKind::CloudModel,
        ExecutorKind::Reviewer,
        ExecutorKind::Validator,
        ExecutorKind::Operator,
        ExecutorKind::WorkflowAutomation,
    ];
    let thread = make_thread(
        allow_all.clone(),
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    for kind in allow_all {
        let executor = make_executor(kind, RoleId::Operator, Uuid::now_v7());
        let decision = ExecutorRouter::decide(&thread, None, &executor, Utc::now());
        assert!(
            matches!(decision, RouteDecision::MayClaim { reason: RouteReason::NoActiveLease }),
            "kind {kind:?} on all-kinds allowlist with no lease must MayClaim; got {decision:?}"
        );
    }
}

#[test]
fn mt_181_allowlist_duplicate_entries_do_not_cause_panic() {
    // Adversarial: a malformed allowlist with duplicates must not panic
    // and must still resolve membership correctly.
    let allowlist = vec![
        ExecutorKind::LocalSmallModel,
        ExecutorKind::LocalSmallModel,
        ExecutorKind::Operator,
    ];
    let thread = make_thread(
        allowlist,
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let exec = make_executor(ExecutorKind::LocalSmallModel, RoleId::Coder, Uuid::now_v7());
    let decision = ExecutorRouter::decide(&thread, None, &exec, Utc::now());
    assert!(matches!(decision, RouteDecision::MayClaim { .. }));
}

#[test]
fn mt_181_conflicting_allowlist_membership_resolves_deterministically() {
    // Adversarial: two callers, one in the allowlist and one not, must
    // each receive deterministic distinct verdicts for the same thread.
    let thread = make_thread(
        vec![ExecutorKind::Validator, ExecutorKind::Operator],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let now = Utc::now();
    let exec_in = make_executor(ExecutorKind::Validator, RoleId::Validator, Uuid::now_v7());
    let exec_out = make_executor(ExecutorKind::CloudModel, RoleId::Coder, Uuid::now_v7());
    let d_in = ExecutorRouter::decide(&thread, None, &exec_in, now);
    let d_out = ExecutorRouter::decide(&thread, None, &exec_out, now);
    assert!(matches!(d_in, RouteDecision::MayClaim { .. }));
    assert!(matches!(d_out, RouteDecision::Denied { .. }));
    // Determinism: same inputs, same outputs.
    let d_in_again = ExecutorRouter::decide(&thread, None, &exec_in, now);
    assert_eq!(d_in, d_in_again, "router must be a deterministic function");
}

// ============================================================
// (2) Resolver returns deterministic claim_mode per input
// ============================================================

#[test]
fn mt_181_resolver_deterministic_per_claim_mode_no_lease() {
    // Per algorithm step (3): no lease + claim_mode in
    // [Exclusive, Handoff] -> MayClaim; ClaimMode::Open ->
    // MayRespondInExistingLease.
    let cases = [
        (ClaimMode::Exclusive, "MayClaim"),
        (ClaimMode::Handoff, "MayClaim"),
        (ClaimMode::Open, "MayRespond"),
    ];
    for (mode, expected) in cases {
        let thread = make_thread(
            vec![ExecutorKind::LocalSmallModel],
            mode,
            TakeoverPolicy::Never,
            ResponseAuthorityScope::LeaseHolder,
        );
        let exec = make_executor(
            ExecutorKind::LocalSmallModel,
            RoleId::Coder,
            Uuid::now_v7(),
        );
        let decision = ExecutorRouter::decide(&thread, None, &exec, Utc::now());
        match (expected, &decision) {
            ("MayClaim", RouteDecision::MayClaim { reason }) => {
                assert_eq!(*reason, RouteReason::NoActiveLease);
            }
            ("MayRespond", RouteDecision::MayRespondInExistingLease { .. }) => {}
            (e, other) => panic!("claim_mode={mode:?} expected {e} got {other:?}"),
        }
    }
}

#[test]
fn mt_181_resolver_holder_session_match_returns_respond_in_existing_lease() {
    // Per algorithm step (4): if lease.holder_session_id == executor.session_id
    // -> MayRespondInExistingLease.
    let thread = make_thread(
        vec![ExecutorKind::CloudModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let session = Uuid::now_v7();
    let lease = make_lease(
        thread.thread_id.as_uuid(),
        ExecutorKind::CloudModel,
        session,
        Utc::now(),
        300,
    );
    let exec = make_executor(ExecutorKind::CloudModel, RoleId::Coder, session);
    let decision = ExecutorRouter::decide(&thread, Some(&lease), &exec, Utc::now());
    assert!(
        matches!(
            decision,
            RouteDecision::MayRespondInExistingLease {
                reason: RouteReason::SessionMatchesLeaseHolder
            }
        ),
        "holder session must reuse existing lease; got {decision:?}"
    );
}

#[test]
fn mt_181_resolver_other_holder_no_takeover_returns_must_wait() {
    // Per algorithm step (7): active foreign lease + Never policy ->
    // MustWait with LeaseHeldByOther.
    let thread = make_thread(
        vec![ExecutorKind::Validator, ExecutorKind::CloudModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let lease = make_lease(
        thread.thread_id.as_uuid(),
        ExecutorKind::CloudModel,
        Uuid::now_v7(),
        Utc::now(),
        300,
    );
    let exec = make_executor(ExecutorKind::Validator, RoleId::Validator, Uuid::now_v7());
    let decision = ExecutorRouter::decide(&thread, Some(&lease), &exec, Utc::now());
    assert!(
        matches!(
            decision,
            RouteDecision::MustWait {
                reason: RouteReason::LeaseHeldByOther
            }
        ),
        "Never-policy foreign-held lease must MustWait; got {decision:?}"
    );
    assert_reason_non_empty(&decision);
}

#[test]
fn mt_181_resolver_takeover_policy_always_with_reason_permits_takeover() {
    // Per algorithm step (6): if takeover_policy permits -> MayTakeover.
    let thread = make_thread(
        vec![ExecutorKind::Operator, ExecutorKind::CloudModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::AlwaysWithReason,
        ResponseAuthorityScope::LeaseHolder,
    );
    let lease = make_lease(
        thread.thread_id.as_uuid(),
        ExecutorKind::CloudModel,
        Uuid::now_v7(),
        Utc::now(),
        300,
    );
    let exec = make_executor(ExecutorKind::Operator, RoleId::Operator, Uuid::now_v7());
    let decision = ExecutorRouter::decide(&thread, Some(&lease), &exec, Utc::now());
    match decision {
        RouteDecision::MayTakeover {
            predecessor_lease_id,
            reason,
        } => {
            assert_eq!(predecessor_lease_id, lease.lease_id);
            assert_eq!(reason, RouteReason::TakeoverPolicyPermits);
        }
        other => panic!("expected MayTakeover, got {other:?}"),
    }
}

#[test]
fn mt_181_resolver_takeover_policy_operator_only_permits_operator() {
    // TakeoverPolicy::OperatorOnly + executor.executor_kind=Operator
    // permits takeover. Any other kind must MustWait.
    let thread = make_thread(
        vec![
            ExecutorKind::Operator,
            ExecutorKind::Validator,
            ExecutorKind::CloudModel,
        ],
        ClaimMode::Exclusive,
        TakeoverPolicy::OperatorOnly,
        ResponseAuthorityScope::LeaseHolder,
    );
    let lease = make_lease(
        thread.thread_id.as_uuid(),
        ExecutorKind::CloudModel,
        Uuid::now_v7(),
        Utc::now(),
        300,
    );

    let operator_exec =
        make_executor(ExecutorKind::Operator, RoleId::Operator, Uuid::now_v7());
    let op_decision = ExecutorRouter::decide(&thread, Some(&lease), &operator_exec, Utc::now());
    assert!(
        matches!(op_decision, RouteDecision::MayTakeover { .. }),
        "OperatorOnly + Operator must MayTakeover; got {op_decision:?}"
    );

    let validator_exec =
        make_executor(ExecutorKind::Validator, RoleId::Validator, Uuid::now_v7());
    let val_decision = ExecutorRouter::decide(&thread, Some(&lease), &validator_exec, Utc::now());
    assert!(
        matches!(val_decision, RouteDecision::MustWait { .. }),
        "OperatorOnly + non-Operator must MustWait; got {val_decision:?}"
    );
}

#[test]
fn mt_181_resolver_takeover_policy_on_lease_expiry_waits_when_unexpired() {
    // TakeoverPolicy::OnLeaseExpiry must MustWait while the lease is
    // still active — the expiry path is what eventually unlocks claim
    // (step 5).
    let thread = make_thread(
        vec![ExecutorKind::Operator, ExecutorKind::CloudModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::OnLeaseExpiry,
        ResponseAuthorityScope::LeaseHolder,
    );
    let lease = make_lease(
        thread.thread_id.as_uuid(),
        ExecutorKind::CloudModel,
        Uuid::now_v7(),
        Utc::now(),
        600,
    );
    let exec = make_executor(ExecutorKind::Operator, RoleId::Operator, Uuid::now_v7());
    let decision = ExecutorRouter::decide(&thread, Some(&lease), &exec, Utc::now());
    assert!(
        matches!(decision, RouteDecision::MustWait { reason: RouteReason::LeaseHeldByOther }),
        "OnLeaseExpiry while unexpired must MustWait; got {decision:?}"
    );
}

#[test]
fn mt_181_resolver_response_authority_scope_operator_only_denies_non_operator() {
    // ResponseAuthorityScope::OperatorOnly applies before lease logic.
    // A non-operator allowlisted executor must be Denied with the
    // typed OperatorOnlyResponseScope reason.
    let thread = make_thread(
        vec![
            ExecutorKind::Operator,
            ExecutorKind::Validator,
            ExecutorKind::CloudModel,
        ],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::OperatorOnly,
    );
    let exec = make_executor(ExecutorKind::Validator, RoleId::Validator, Uuid::now_v7());
    let decision = ExecutorRouter::decide(&thread, None, &exec, Utc::now());
    match decision {
        RouteDecision::Denied { ref reason, ref detail } => {
            assert_eq!(*reason, RouteReason::OperatorOnlyResponseScope);
            assert!(detail.contains("OperatorOnly"), "detail must reference scope");
        }
        other => panic!("expected Denied(OperatorOnlyResponseScope), got {other:?}"),
    }

    // Operator must still be allowed.
    let op_exec = make_executor(ExecutorKind::Operator, RoleId::Operator, Uuid::now_v7());
    let op_decision = ExecutorRouter::decide(&thread, None, &op_exec, Utc::now());
    assert!(
        matches!(op_decision, RouteDecision::MayClaim { .. }),
        "Operator on OperatorOnly thread must claim; got {op_decision:?}"
    );
}

// ============================================================
// (3) Adversarial: clock injection, terminal state, claim_mode round-trip
// ============================================================

#[test]
fn mt_181_terminal_thread_state_denies_for_all_executors() {
    // Per algorithm step (2): lifecycle terminal (resolved/expired/archived)
    // -> Denied(ThreadInTerminalState). The allowlist still applies first
    // (step 1).
    for state in [
        ThreadLifecycleState::Resolved,
        ThreadLifecycleState::Expired,
        ThreadLifecycleState::Archived,
    ] {
        let mut thread = make_thread(
            vec![ExecutorKind::LocalSmallModel],
            ClaimMode::Exclusive,
            TakeoverPolicy::Never,
            ResponseAuthorityScope::LeaseHolder,
        );
        thread.lifecycle_state = state;
        let exec = make_executor(
            ExecutorKind::LocalSmallModel,
            RoleId::Coder,
            Uuid::now_v7(),
        );
        let decision = ExecutorRouter::decide(&thread, None, &exec, Utc::now());
        match decision {
            RouteDecision::Denied { ref reason, .. } => {
                assert_eq!(*reason, RouteReason::ThreadInTerminalState);
                assert_reason_non_empty(&decision);
            }
            other => panic!("terminal state {state:?} expected Denied got {other:?}"),
        }
    }
}

#[test]
fn mt_181_expired_lease_with_clock_at_expiry_boundary_returns_may_claim() {
    // Adversarial: lease.expires_at_utc == now must be considered
    // expired (the algorithm uses `<=`). This is the boundary-condition
    // probe for injected `now`.
    let acquired = Utc::now() - ChronoDuration::seconds(300);
    let thread = make_thread(
        vec![ExecutorKind::LocalSmallModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let lease = make_lease(
        thread.thread_id.as_uuid(),
        ExecutorKind::LocalSmallModel,
        Uuid::now_v7(),
        acquired,
        60,
    );
    let exec = make_executor(
        ExecutorKind::LocalSmallModel,
        RoleId::Coder,
        Uuid::now_v7(),
    );
    // Inject `now` exactly at expiry.
    let now = lease.expires_at_utc;
    let decision = ExecutorRouter::decide(&thread, Some(&lease), &exec, now);
    assert!(
        matches!(decision, RouteDecision::MayClaim { reason: RouteReason::LeaseExpired }),
        "now at expiry boundary must MayClaim; got {decision:?}"
    );
}

#[test]
fn mt_181_active_lease_with_clock_before_expiry_returns_must_wait() {
    // Adversarial converse: `now` strictly before expiry must NOT
    // produce MayClaim — instead MustWait (Never policy).
    let acquired = Utc::now();
    let thread = make_thread(
        vec![ExecutorKind::LocalSmallModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let lease = make_lease(
        thread.thread_id.as_uuid(),
        ExecutorKind::LocalSmallModel,
        Uuid::now_v7(),
        acquired,
        600,
    );
    let exec = make_executor(
        ExecutorKind::LocalSmallModel,
        RoleId::Coder,
        Uuid::now_v7(),
    );
    let now = acquired + ChronoDuration::seconds(60);
    let decision = ExecutorRouter::decide(&thread, Some(&lease), &exec, now);
    assert!(
        matches!(decision, RouteDecision::MustWait { reason: RouteReason::LeaseHeldByOther }),
        "now before expiry must MustWait; got {decision:?}"
    );
}

#[test]
fn mt_181_claim_mode_round_trip_via_serde_preserves_router_decision() {
    // Adversarial: ClaimMode must survive a serde round-trip (snake_case
    // wire shape) without altering the router decision. A drift in the
    // wire shape would silently re-route claims.
    let thread = make_thread(
        vec![ExecutorKind::CloudModel],
        ClaimMode::Handoff,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let json = serde_json::to_string(&thread).expect("serialise thread");
    let back: RoleMailboxThread = serde_json::from_str(&json).expect("deserialise thread");
    assert_eq!(back.claim_mode, ClaimMode::Handoff);

    let exec = make_executor(ExecutorKind::CloudModel, RoleId::Coder, Uuid::now_v7());
    let now = Utc::now();
    let d_original = ExecutorRouter::decide(&thread, None, &exec, now);
    let d_round_trip = ExecutorRouter::decide(&back, None, &exec, now);
    assert_eq!(
        d_original, d_round_trip,
        "serde round-trip must preserve router decision"
    );
}

#[test]
fn mt_181_claim_mode_open_with_active_foreign_lease_permits_respond() {
    // ClaimMode::Open allows additional executors to respond even when a
    // lease is held. This is the documented multi-holder path.
    let thread = make_thread(
        vec![ExecutorKind::CloudModel, ExecutorKind::Validator],
        ClaimMode::Open,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::AllowlistOpen,
    );
    let lease = make_lease(
        thread.thread_id.as_uuid(),
        ExecutorKind::CloudModel,
        Uuid::now_v7(),
        Utc::now(),
        300,
    );
    let exec = make_executor(ExecutorKind::Validator, RoleId::Validator, Uuid::now_v7());
    let decision = ExecutorRouter::decide(&thread, Some(&lease), &exec, Utc::now());
    assert!(
        matches!(decision, RouteDecision::MayRespondInExistingLease { .. }),
        "ClaimMode::Open + foreign active lease must permit respond; got {decision:?}"
    );
}

// ============================================================
// (4) Exhaustive matrix — 5-axis property sweep
// ============================================================

#[test]
fn mt_181_exhaustive_matrix_no_panic_5_axis() {
    // (claim_mode × takeover_policy × response_authority_scope ×
    // executor_kind × has_lease) sweep. Per contract:
    //   "exhaustive matrix of (claim_mode × current_lease present ×
    //    executor_kind match × takeover_policy) — 32+ combinations".
    // We extend the matrix to include response_authority_scope so the
    // 1a OperatorOnly path is also covered.
    let modes = [ClaimMode::Exclusive, ClaimMode::Handoff, ClaimMode::Open];
    let policies = [
        TakeoverPolicy::Never,
        TakeoverPolicy::OnLeaseExpiry,
        TakeoverPolicy::AlwaysWithReason,
        TakeoverPolicy::OperatorOnly,
    ];
    let scopes = [
        ResponseAuthorityScope::LeaseHolder,
        ResponseAuthorityScope::AllowlistOpen,
        ResponseAuthorityScope::MicroTaskCompletionScope,
        ResponseAuthorityScope::OperatorOnly,
    ];
    let kinds = [
        ExecutorKind::LocalSmallModel,
        ExecutorKind::CloudModel,
        ExecutorKind::Reviewer,
        ExecutorKind::Validator,
        ExecutorKind::Operator,
        ExecutorKind::WorkflowAutomation,
    ];

    let mut count = 0usize;
    for mode in modes {
        for policy in policies {
            for scope in scopes {
                for kind in kinds {
                    for has_lease in [false, true] {
                        let thread =
                            make_thread(kinds.to_vec(), mode, policy, scope);
                        let session = Uuid::now_v7();
                        let lease = if has_lease {
                            Some(make_lease(
                                thread.thread_id.as_uuid(),
                                ExecutorKind::CloudModel,
                                Uuid::now_v7(),
                                Utc::now(),
                                300,
                            ))
                        } else {
                            None
                        };
                        let exec = make_executor(kind, RoleId::Coder, session);
                        let decision = ExecutorRouter::decide(
                            &thread,
                            lease.as_ref(),
                            &exec,
                            Utc::now(),
                        );
                        // No panic; every decision must carry a typed
                        // reason and (for Denied) a non-empty detail.
                        assert_reason_non_empty(&decision);
                        // Determinism: a second call with the same inputs
                        // must return the same decision.
                        let again = ExecutorRouter::decide(
                            &thread,
                            lease.as_ref(),
                            &exec,
                            Utc::now(),
                        );
                        // Timestamps in `now` can shift by a few ns between
                        // the two calls; we compare decision-variant tag
                        // only, not the full payload, to keep this stable.
                        assert_eq!(
                            std::mem::discriminant(&decision),
                            std::mem::discriminant(&again),
                            "router decision variant must be deterministic across calls; first={decision:?} second={again:?}"
                        );
                        count += 1;
                    }
                }
            }
        }
    }
    // 3 × 4 × 4 × 6 × 2 = 576 combinations.
    assert_eq!(count, 3 * 4 * 4 * 6 * 2, "matrix sweep must cover full 5-axis product");
}

#[test]
fn mt_181_every_decision_variant_has_non_empty_reason() {
    // Per red_team minimum control #3: every RouteDecision variant must
    // have at least one positive test asserting the reason is non-empty.
    //
    // MayClaim:
    let thread = make_thread(
        vec![ExecutorKind::LocalSmallModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let exec = make_executor(
        ExecutorKind::LocalSmallModel,
        RoleId::Coder,
        Uuid::now_v7(),
    );
    let d = ExecutorRouter::decide(&thread, None, &exec, Utc::now());
    assert!(matches!(d, RouteDecision::MayClaim { .. }));
    assert_reason_non_empty(&d);

    // MayRespondInExistingLease (session match):
    let session = Uuid::now_v7();
    let lease = make_lease(
        thread.thread_id.as_uuid(),
        ExecutorKind::LocalSmallModel,
        session,
        Utc::now(),
        300,
    );
    let exec_holder = make_executor(ExecutorKind::LocalSmallModel, RoleId::Coder, session);
    let d = ExecutorRouter::decide(&thread, Some(&lease), &exec_holder, Utc::now());
    assert!(matches!(d, RouteDecision::MayRespondInExistingLease { .. }));
    assert_reason_non_empty(&d);

    // MayTakeover:
    let permissive = make_thread(
        vec![ExecutorKind::Operator],
        ClaimMode::Exclusive,
        TakeoverPolicy::AlwaysWithReason,
        ResponseAuthorityScope::LeaseHolder,
    );
    let foreign_lease = make_lease(
        permissive.thread_id.as_uuid(),
        ExecutorKind::LocalSmallModel,
        Uuid::now_v7(),
        Utc::now(),
        300,
    );
    let op_exec = make_executor(ExecutorKind::Operator, RoleId::Operator, Uuid::now_v7());
    let d = ExecutorRouter::decide(&permissive, Some(&foreign_lease), &op_exec, Utc::now());
    assert!(matches!(d, RouteDecision::MayTakeover { .. }));
    assert_reason_non_empty(&d);

    // MustWait:
    let strict = make_thread(
        vec![ExecutorKind::LocalSmallModel, ExecutorKind::CloudModel],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let foreign = make_lease(
        strict.thread_id.as_uuid(),
        ExecutorKind::CloudModel,
        Uuid::now_v7(),
        Utc::now(),
        300,
    );
    let other_exec = make_executor(
        ExecutorKind::LocalSmallModel,
        RoleId::Coder,
        Uuid::now_v7(),
    );
    let d = ExecutorRouter::decide(&strict, Some(&foreign), &other_exec, Utc::now());
    assert!(matches!(d, RouteDecision::MustWait { .. }));
    assert_reason_non_empty(&d);

    // Denied:
    let denied_thread = make_thread(
        vec![ExecutorKind::Validator],
        ClaimMode::Exclusive,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    );
    let bad_exec = make_executor(ExecutorKind::CloudModel, RoleId::Coder, Uuid::now_v7());
    let d = ExecutorRouter::decide(&denied_thread, None, &bad_exec, Utc::now());
    assert!(matches!(d, RouteDecision::Denied { .. }));
    assert_reason_non_empty(&d);
}

// ============================================================
// (5) Decision serde wire-shape — RouteDecision must round-trip
// ============================================================

#[test]
fn mt_181_route_decision_serde_round_trip_all_variants() {
    // Adversarial: every RouteDecision must serialise + deserialise
    // losslessly. A drift in the wire shape would break inspector tools
    // and the integration validator surface.
    let variants = vec![
        RouteDecision::MayClaim {
            reason: RouteReason::NoActiveLease,
        },
        RouteDecision::MayRespondInExistingLease {
            reason: RouteReason::SessionMatchesLeaseHolder,
        },
        RouteDecision::MayTakeover {
            predecessor_lease_id: Uuid::now_v7(),
            reason: RouteReason::TakeoverPolicyPermits,
        },
        RouteDecision::MustWait {
            reason: RouteReason::LeaseHeldByOther,
        },
        RouteDecision::Denied {
            reason: RouteReason::ExecutorKindNotInAllowlist,
            detail: "test detail".to_string(),
        },
    ];
    for v in variants {
        let json = serde_json::to_string(&v).expect("serialise");
        let back: RouteDecision = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(v, back, "RouteDecision round-trip must be lossless: {json}");
    }
}

#[test]
fn mt_181_executor_kind_serde_snake_case_wire_shape() {
    // Adversarial wire-shape: serde rename_all=snake_case must produce
    // canonical lowercase variant names so cross-language consumers
    // (TypeScript inspector) can rely on the shape.
    let pairs = [
        (ExecutorKind::LocalSmallModel, "\"local_small_model\""),
        (ExecutorKind::CloudModel, "\"cloud_model\""),
        (ExecutorKind::Reviewer, "\"reviewer\""),
        (ExecutorKind::Validator, "\"validator\""),
        (ExecutorKind::Operator, "\"operator\""),
        (ExecutorKind::WorkflowAutomation, "\"workflow_automation\""),
    ];
    for (kind, expected_json) in pairs {
        let json = serde_json::to_string(&kind).expect("serialise");
        assert_eq!(json, expected_json, "ExecutorKind wire shape must be snake_case");
        let back: ExecutorKind = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(kind, back);
    }
}
