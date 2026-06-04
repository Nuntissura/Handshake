//! MT-181 ExecutorRouter — pure routing decision over thread + lease + identity.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::lease::{RoleMailboxClaimLeaseV1, TakeoverPolicy};
use super::thread::{ClaimMode, ResponseAuthorityScope, RoleMailboxThread};
use crate::role_mailbox::RoleId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorKind {
    LocalSmallModel,
    CloudModel,
    Reviewer,
    Validator,
    Operator,
    WorkflowAutomation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutorIdentity {
    pub executor_kind: ExecutorKind,
    pub role_id: RoleId,
    pub session_id: Uuid,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteReason {
    AllowlistOk,
    ExecutorKindNotInAllowlist,
    ThreadInTerminalState,
    NoActiveLease,
    LeaseHeldByOther,
    LeaseExpired,
    SessionMatchesLeaseHolder,
    TakeoverPolicyPermits,
    TakeoverPolicyForbids,
    OperatorOnlyResponseScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", content = "detail")]
pub enum RouteDecision {
    MayClaim {
        reason: RouteReason,
    },
    MayRespondInExistingLease {
        reason: RouteReason,
    },
    MayTakeover {
        predecessor_lease_id: Uuid,
        reason: RouteReason,
    },
    MustWait {
        reason: RouteReason,
    },
    Denied {
        reason: RouteReason,
        detail: String,
    },
}

pub struct ExecutorRouter;

impl ExecutorRouter {
    /// Pure decision function. Clock is injected for testability.
    pub fn decide(
        thread: &RoleMailboxThread,
        current_lease: Option<&RoleMailboxClaimLeaseV1>,
        executor: &ExecutorIdentity,
        now: DateTime<Utc>,
    ) -> RouteDecision {
        // (0) Empty allowlist gate: an empty allowlist locks the thread
        // to all executors. Surface a typed Denied rather than appearing
        // to satisfy a "contains" check on an empty vector.
        if thread.executor_kind_allowlist.is_empty() {
            return RouteDecision::Denied {
                reason: RouteReason::ExecutorKindNotInAllowlist,
                detail: format!(
                    "thread executor_kind_allowlist is empty; no executor may claim/respond. executor_kind={:?}",
                    executor.executor_kind
                ),
            };
        }

        // (1) Allowlist gate.
        if !thread
            .executor_kind_allowlist
            .contains(&executor.executor_kind)
        {
            return RouteDecision::Denied {
                reason: RouteReason::ExecutorKindNotInAllowlist,
                detail: format!(
                    "executor_kind {:?} not in thread allowlist {:?}",
                    executor.executor_kind, thread.executor_kind_allowlist
                ),
            };
        }

        // (1a) ResponseAuthorityScope::OperatorOnly applies before lease
        // logic — only the operator role may respond regardless of who
        // holds the lease.
        if matches!(
            thread.response_authority_scope,
            ResponseAuthorityScope::OperatorOnly
        ) && executor.executor_kind != ExecutorKind::Operator
        {
            return RouteDecision::Denied {
                reason: RouteReason::OperatorOnlyResponseScope,
                detail: format!(
                    "thread response_authority_scope=OperatorOnly; executor_kind={:?} is not Operator",
                    executor.executor_kind
                ),
            };
        }

        // (2) Terminal-state gate.
        if thread.is_terminal() {
            return RouteDecision::Denied {
                reason: RouteReason::ThreadInTerminalState,
                detail: format!(
                    "thread lifecycle_state={:?} is terminal",
                    thread.lifecycle_state
                ),
            };
        }

        // (3) Lease decision.
        match current_lease {
            None => {
                if matches!(thread.claim_mode, ClaimMode::Exclusive | ClaimMode::Handoff) {
                    RouteDecision::MayClaim {
                        reason: RouteReason::NoActiveLease,
                    }
                } else {
                    // ClaimMode::Open allows respond without lease.
                    RouteDecision::MayRespondInExistingLease {
                        reason: RouteReason::AllowlistOk,
                    }
                }
            }
            Some(lease) => {
                // (4) Holder is us.
                if lease.holder_session_id == executor.session_id {
                    return RouteDecision::MayRespondInExistingLease {
                        reason: RouteReason::SessionMatchesLeaseHolder,
                    };
                }
                // (5) Stale lease.
                if lease.expires_at_utc <= now {
                    return RouteDecision::MayClaim {
                        reason: RouteReason::LeaseExpired,
                    };
                }
                // (5a) ClaimMode::Open permits multiple holders responding
                // alongside the existing lease holder when allowlisted.
                if matches!(thread.claim_mode, ClaimMode::Open) {
                    return RouteDecision::MayRespondInExistingLease {
                        reason: RouteReason::AllowlistOk,
                    };
                }
                // (6) Active lease, takeover policy gate.
                match thread.takeover_policy {
                    TakeoverPolicy::AlwaysWithReason => RouteDecision::MayTakeover {
                        predecessor_lease_id: lease.lease_id,
                        reason: RouteReason::TakeoverPolicyPermits,
                    },
                    TakeoverPolicy::OperatorOnly
                        if executor.executor_kind == ExecutorKind::Operator =>
                    {
                        RouteDecision::MayTakeover {
                            predecessor_lease_id: lease.lease_id,
                            reason: RouteReason::TakeoverPolicyPermits,
                        }
                    }
                    // (7) Otherwise wait — OnLeaseExpiry waits because the
                    // lease is not yet expired (step 5 covers expiry).
                    // Never always waits. OperatorOnly waits for non-operators.
                    _ => RouteDecision::MustWait {
                        reason: RouteReason::LeaseHeldByOther,
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::role_mailbox_v1::lease::TakeoverPolicy;
    use crate::role_mailbox_v1::lifecycle::ThreadLifecycleState;
    use crate::role_mailbox_v1::thread::{LinkedRecordKind, ResponseAuthorityScope};

    fn make_thread(
        allowlist: Vec<ExecutorKind>,
        claim_mode: ClaimMode,
        takeover: TakeoverPolicy,
    ) -> RoleMailboxThread {
        RoleMailboxThread::open(
            "t",
            LinkedRecordKind::Wp,
            None,
            allowlist,
            claim_mode,
            takeover,
            ResponseAuthorityScope::LeaseHolder,
        )
    }

    fn make_exec(kind: ExecutorKind, session_id: Uuid) -> ExecutorIdentity {
        ExecutorIdentity {
            executor_kind: kind,
            role_id: RoleId::Coder,
            session_id,
            capabilities: vec![],
        }
    }

    #[test]
    fn denied_when_executor_kind_not_in_allowlist() {
        let t = make_thread(
            vec![ExecutorKind::Validator],
            ClaimMode::Exclusive,
            TakeoverPolicy::Never,
        );
        let e = make_exec(ExecutorKind::LocalSmallModel, Uuid::now_v7());
        let d = ExecutorRouter::decide(&t, None, &e, Utc::now());
        assert!(matches!(d, RouteDecision::Denied { .. }));
    }

    #[test]
    fn may_claim_when_no_active_lease() {
        let t = make_thread(
            vec![ExecutorKind::LocalSmallModel],
            ClaimMode::Exclusive,
            TakeoverPolicy::Never,
        );
        let e = make_exec(ExecutorKind::LocalSmallModel, Uuid::now_v7());
        let d = ExecutorRouter::decide(&t, None, &e, Utc::now());
        assert!(matches!(d, RouteDecision::MayClaim { .. }));
    }

    #[test]
    fn must_wait_when_lease_held_by_other_no_takeover() {
        let t = make_thread(
            vec![ExecutorKind::LocalSmallModel],
            ClaimMode::Exclusive,
            TakeoverPolicy::Never,
        );
        let me = Uuid::now_v7();
        let other = Uuid::now_v7();
        let lease = RoleMailboxClaimLeaseV1 {
            lease_id: Uuid::now_v7(),
            thread_id: t.thread_id.as_uuid(),
            holder_executor_kind: ExecutorKind::LocalSmallModel,
            holder_role_id: RoleId::Coder,
            holder_session_id: other,
            acquired_at_utc: Utc::now(),
            expires_at_utc: Utc::now() + chrono::Duration::seconds(120),
            released_at_utc: None,
            takeover_of: None,
            takeover_reason: None,
        };
        let e = make_exec(ExecutorKind::LocalSmallModel, me);
        let d = ExecutorRouter::decide(&t, Some(&lease), &e, Utc::now());
        assert!(matches!(d, RouteDecision::MustWait { .. }));
    }

    #[test]
    fn may_respond_in_existing_lease_when_session_matches() {
        let t = make_thread(
            vec![ExecutorKind::LocalSmallModel],
            ClaimMode::Exclusive,
            TakeoverPolicy::Never,
        );
        let me = Uuid::now_v7();
        let lease = RoleMailboxClaimLeaseV1 {
            lease_id: Uuid::now_v7(),
            thread_id: t.thread_id.as_uuid(),
            holder_executor_kind: ExecutorKind::LocalSmallModel,
            holder_role_id: RoleId::Coder,
            holder_session_id: me,
            acquired_at_utc: Utc::now(),
            expires_at_utc: Utc::now() + chrono::Duration::seconds(120),
            released_at_utc: None,
            takeover_of: None,
            takeover_reason: None,
        };
        let e = make_exec(ExecutorKind::LocalSmallModel, me);
        let d = ExecutorRouter::decide(&t, Some(&lease), &e, Utc::now());
        assert!(matches!(d, RouteDecision::MayRespondInExistingLease { .. }));
    }

    #[test]
    fn may_claim_when_lease_expired() {
        let t = make_thread(
            vec![ExecutorKind::LocalSmallModel],
            ClaimMode::Exclusive,
            TakeoverPolicy::Never,
        );
        let other = Uuid::now_v7();
        let lease = RoleMailboxClaimLeaseV1 {
            lease_id: Uuid::now_v7(),
            thread_id: t.thread_id.as_uuid(),
            holder_executor_kind: ExecutorKind::LocalSmallModel,
            holder_role_id: RoleId::Coder,
            holder_session_id: other,
            acquired_at_utc: Utc::now() - chrono::Duration::seconds(600),
            expires_at_utc: Utc::now() - chrono::Duration::seconds(60),
            released_at_utc: None,
            takeover_of: None,
            takeover_reason: None,
        };
        let e = make_exec(ExecutorKind::LocalSmallModel, Uuid::now_v7());
        let d = ExecutorRouter::decide(&t, Some(&lease), &e, Utc::now());
        assert!(matches!(d, RouteDecision::MayClaim { .. }));
    }

    #[test]
    fn may_takeover_with_permissive_policy() {
        let t = make_thread(
            vec![ExecutorKind::Operator],
            ClaimMode::Exclusive,
            TakeoverPolicy::AlwaysWithReason,
        );
        let other = Uuid::now_v7();
        let lease = RoleMailboxClaimLeaseV1 {
            lease_id: Uuid::now_v7(),
            thread_id: t.thread_id.as_uuid(),
            holder_executor_kind: ExecutorKind::LocalSmallModel,
            holder_role_id: RoleId::Coder,
            holder_session_id: other,
            acquired_at_utc: Utc::now(),
            expires_at_utc: Utc::now() + chrono::Duration::seconds(120),
            released_at_utc: None,
            takeover_of: None,
            takeover_reason: None,
        };
        let e = make_exec(ExecutorKind::Operator, Uuid::now_v7());
        let d = ExecutorRouter::decide(&t, Some(&lease), &e, Utc::now());
        assert!(matches!(d, RouteDecision::MayTakeover { .. }));
    }

    #[test]
    fn denied_when_thread_terminal() {
        let mut t = make_thread(
            vec![ExecutorKind::LocalSmallModel],
            ClaimMode::Exclusive,
            TakeoverPolicy::Never,
        );
        t.lifecycle_state = ThreadLifecycleState::Resolved;
        let e = make_exec(ExecutorKind::LocalSmallModel, Uuid::now_v7());
        let d = ExecutorRouter::decide(&t, None, &e, Utc::now());
        assert!(matches!(d, RouteDecision::Denied { .. }));
    }

    #[test]
    fn exhaustive_matrix_does_not_panic() {
        use crate::role_mailbox_v1::lease::TakeoverPolicy::*;
        let modes = [ClaimMode::Exclusive, ClaimMode::Handoff, ClaimMode::Open];
        let policies = [Never, OnLeaseExpiry, AlwaysWithReason, OperatorOnly];
        let kinds = [
            ExecutorKind::LocalSmallModel,
            ExecutorKind::CloudModel,
            ExecutorKind::Reviewer,
            ExecutorKind::Validator,
            ExecutorKind::Operator,
            ExecutorKind::WorkflowAutomation,
        ];
        let mut decisions = 0usize;
        for mode in modes {
            for pol in policies {
                for kind in kinds {
                    for has_lease in [false, true] {
                        let t = make_thread(kinds.to_vec(), mode, pol);
                        let me = Uuid::now_v7();
                        let lease = if has_lease {
                            Some(RoleMailboxClaimLeaseV1 {
                                lease_id: Uuid::now_v7(),
                                thread_id: t.thread_id.as_uuid(),
                                holder_executor_kind: ExecutorKind::CloudModel,
                                holder_role_id: RoleId::Coder,
                                holder_session_id: Uuid::now_v7(),
                                acquired_at_utc: Utc::now(),
                                expires_at_utc: Utc::now() + chrono::Duration::seconds(120),
                                released_at_utc: None,
                                takeover_of: None,
                                takeover_reason: None,
                            })
                        } else {
                            None
                        };
                        let e = make_exec(kind, me);
                        let _ = ExecutorRouter::decide(&t, lease.as_ref(), &e, Utc::now());
                        decisions += 1;
                    }
                }
            }
        }
        assert_eq!(decisions, 3 * 4 * 6 * 2);
    }
}
