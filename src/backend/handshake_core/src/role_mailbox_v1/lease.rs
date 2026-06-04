//! MT-180 RoleMailboxClaimLeaseV1 + LeaseManager.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

use super::router::ExecutorKind;
use super::thread::ClaimMode;
use crate::role_mailbox::RoleId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TakeoverPolicy {
    Never,
    OnLeaseExpiry,
    AlwaysWithReason,
    OperatorOnly,
}

impl TakeoverPolicy {
    pub fn permits_takeover_with_reason(self) -> bool {
        matches!(self, Self::AlwaysWithReason)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMailboxClaimLeaseV1 {
    pub lease_id: Uuid,
    pub thread_id: Uuid,
    pub holder_executor_kind: ExecutorKind,
    pub holder_role_id: RoleId,
    pub holder_session_id: Uuid,
    pub acquired_at_utc: DateTime<Utc>,
    pub expires_at_utc: DateTime<Utc>,
    pub released_at_utc: Option<DateTime<Utc>>,
    pub takeover_of: Option<Uuid>,
    pub takeover_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseRequest {
    pub executor_kind: ExecutorKind,
    pub role_id: RoleId,
    pub session_id: Uuid,
    pub lease_duration_secs: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LeaseError {
    #[error("lease held by other holder")]
    LeaseHeldByOther { current_holder: Uuid },
    #[error("executor kind not allowed")]
    ExecutorKindNotAllowed,
    #[error("thread in terminal state")]
    ThreadInTerminalState,
    #[error("conflict — concurrent acquire")]
    Conflict,
    #[error("takeover not permitted by policy")]
    TakeoverNotPermitted,
    #[error("lease not found")]
    NotFound,
    #[error("lease already released")]
    AlreadyReleased,
    #[error("lease expired")]
    Expired,
}

/// In-process lease manager for unit tests + offline workflows. Production
/// uses `crate::role_mailbox_v1::repo::RoleMailboxRepository::acquire_lease`
/// which executes the same algorithm inside a Postgres `SELECT ... FOR UPDATE`
/// transaction.
#[derive(Default)]
pub struct LeaseManager {
    leases: Arc<Mutex<HashMap<Uuid, RoleMailboxClaimLeaseV1>>>,
}

impl LeaseManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Acquire a lease for `thread_id`. Returns `LeaseHeldByOther` if an
    /// unexpired non-released lease already exists.
    pub fn acquire(
        &self,
        thread_id: Uuid,
        allowlist: &[ExecutorKind],
        claim_mode: ClaimMode,
        request: LeaseRequest,
        now: DateTime<Utc>,
    ) -> Result<RoleMailboxClaimLeaseV1, LeaseError> {
        if !allowlist.contains(&request.executor_kind) {
            return Err(LeaseError::ExecutorKindNotAllowed);
        }
        let mut leases = self.leases.lock().unwrap();
        let active_for_thread = leases.values().find(|l| {
            l.thread_id == thread_id && l.released_at_utc.is_none() && l.expires_at_utc > now
        });
        if let Some(active) = active_for_thread {
            // ClaimMode::Open permits multiple holders.
            if claim_mode != ClaimMode::Open {
                return Err(LeaseError::LeaseHeldByOther {
                    current_holder: active.holder_session_id,
                });
            }
        }
        let lease = RoleMailboxClaimLeaseV1 {
            lease_id: Uuid::now_v7(),
            thread_id,
            holder_executor_kind: request.executor_kind,
            holder_role_id: request.role_id,
            holder_session_id: request.session_id,
            acquired_at_utc: now,
            expires_at_utc: now + chrono::Duration::seconds(request.lease_duration_secs as i64),
            released_at_utc: None,
            takeover_of: None,
            takeover_reason: None,
        };
        leases.insert(lease.lease_id, lease.clone());
        Ok(lease)
    }

    pub fn extend(
        &self,
        lease_id: Uuid,
        extra_secs: u32,
        now: DateTime<Utc>,
    ) -> Result<RoleMailboxClaimLeaseV1, LeaseError> {
        let mut leases = self.leases.lock().unwrap();
        let lease = leases.get_mut(&lease_id).ok_or(LeaseError::NotFound)?;
        if lease.released_at_utc.is_some() {
            return Err(LeaseError::AlreadyReleased);
        }
        if lease.expires_at_utc <= now {
            return Err(LeaseError::Expired);
        }
        lease.expires_at_utc = lease.expires_at_utc + chrono::Duration::seconds(extra_secs as i64);
        Ok(lease.clone())
    }

    pub fn release(&self, lease_id: Uuid, now: DateTime<Utc>) -> Result<(), LeaseError> {
        let mut leases = self.leases.lock().unwrap();
        let lease = leases.get_mut(&lease_id).ok_or(LeaseError::NotFound)?;
        if lease.released_at_utc.is_none() {
            lease.released_at_utc = Some(now);
        }
        Ok(())
    }

    pub fn takeover(
        &self,
        thread_id: Uuid,
        allowlist: &[ExecutorKind],
        request: LeaseRequest,
        predecessor_lease_id: Uuid,
        takeover_policy: TakeoverPolicy,
        reason: String,
        now: DateTime<Utc>,
    ) -> Result<RoleMailboxClaimLeaseV1, LeaseError> {
        if matches!(takeover_policy, TakeoverPolicy::Never) {
            return Err(LeaseError::TakeoverNotPermitted);
        }
        if matches!(takeover_policy, TakeoverPolicy::OperatorOnly)
            && request.executor_kind != ExecutorKind::Operator
        {
            return Err(LeaseError::TakeoverNotPermitted);
        }
        if !allowlist.contains(&request.executor_kind) {
            return Err(LeaseError::ExecutorKindNotAllowed);
        }
        let mut leases = self.leases.lock().unwrap();
        let predecessor = leases
            .get_mut(&predecessor_lease_id)
            .ok_or(LeaseError::NotFound)?;
        if matches!(takeover_policy, TakeoverPolicy::OnLeaseExpiry)
            && predecessor.expires_at_utc > now
        {
            return Err(LeaseError::TakeoverNotPermitted);
        }
        predecessor.released_at_utc = Some(now);
        let new_lease = RoleMailboxClaimLeaseV1 {
            lease_id: Uuid::now_v7(),
            thread_id,
            holder_executor_kind: request.executor_kind,
            holder_role_id: request.role_id,
            holder_session_id: request.session_id,
            acquired_at_utc: now,
            expires_at_utc: now + chrono::Duration::seconds(request.lease_duration_secs as i64),
            released_at_utc: None,
            takeover_of: Some(predecessor_lease_id),
            takeover_reason: Some(reason),
        };
        leases.insert(new_lease.lease_id, new_lease.clone());
        Ok(new_lease)
    }

    pub fn lookup(&self, lease_id: Uuid) -> Option<RoleMailboxClaimLeaseV1> {
        self.leases.lock().unwrap().get(&lease_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_on_fresh_thread_succeeds() {
        let mgr = LeaseManager::new();
        let thread_id = Uuid::now_v7();
        let req = LeaseRequest {
            executor_kind: ExecutorKind::LocalSmallModel,
            role_id: RoleId::Coder,
            session_id: Uuid::now_v7(),
            lease_duration_secs: 60,
        };
        let res = mgr.acquire(
            thread_id,
            &[ExecutorKind::LocalSmallModel],
            ClaimMode::Exclusive,
            req,
            Utc::now(),
        );
        assert!(res.is_ok());
    }

    #[test]
    fn second_acquire_on_same_thread_returns_held_by_other() {
        let mgr = LeaseManager::new();
        let thread_id = Uuid::now_v7();
        let r1 = LeaseRequest {
            executor_kind: ExecutorKind::LocalSmallModel,
            role_id: RoleId::Coder,
            session_id: Uuid::now_v7(),
            lease_duration_secs: 120,
        };
        let _ = mgr
            .acquire(
                thread_id,
                &[ExecutorKind::LocalSmallModel],
                ClaimMode::Exclusive,
                r1,
                Utc::now(),
            )
            .unwrap();
        let r2 = LeaseRequest {
            executor_kind: ExecutorKind::LocalSmallModel,
            role_id: RoleId::Coder,
            session_id: Uuid::now_v7(),
            lease_duration_secs: 120,
        };
        let res = mgr.acquire(
            thread_id,
            &[ExecutorKind::LocalSmallModel],
            ClaimMode::Exclusive,
            r2,
            Utc::now(),
        );
        assert!(matches!(res, Err(LeaseError::LeaseHeldByOther { .. })));
    }

    #[test]
    fn takeover_never_policy_returns_error() {
        let mgr = LeaseManager::new();
        let thread_id = Uuid::now_v7();
        let r1 = LeaseRequest {
            executor_kind: ExecutorKind::LocalSmallModel,
            role_id: RoleId::Coder,
            session_id: Uuid::now_v7(),
            lease_duration_secs: 120,
        };
        let l1 = mgr
            .acquire(
                thread_id,
                &[ExecutorKind::LocalSmallModel, ExecutorKind::Operator],
                ClaimMode::Exclusive,
                r1,
                Utc::now(),
            )
            .unwrap();
        let r2 = LeaseRequest {
            executor_kind: ExecutorKind::Operator,
            role_id: RoleId::Operator,
            session_id: Uuid::now_v7(),
            lease_duration_secs: 120,
        };
        let res = mgr.takeover(
            thread_id,
            &[ExecutorKind::LocalSmallModel, ExecutorKind::Operator],
            r2,
            l1.lease_id,
            TakeoverPolicy::Never,
            "no".to_string(),
            Utc::now(),
        );
        assert!(matches!(res, Err(LeaseError::TakeoverNotPermitted)));
    }

    #[test]
    fn parallel_acquire_exactly_one_winner() {
        use std::sync::Arc;
        let mgr = Arc::new(LeaseManager::new());
        let thread_id = Uuid::now_v7();
        let allowlist = vec![ExecutorKind::LocalSmallModel];
        let now = Utc::now();
        let mut handles = Vec::new();
        for _ in 0..8 {
            let mgr = Arc::clone(&mgr);
            let allowlist = allowlist.clone();
            handles.push(std::thread::spawn(move || {
                let req = LeaseRequest {
                    executor_kind: ExecutorKind::LocalSmallModel,
                    role_id: RoleId::Coder,
                    session_id: Uuid::now_v7(),
                    lease_duration_secs: 120,
                };
                mgr.acquire(thread_id, &allowlist, ClaimMode::Exclusive, req, now)
            }));
        }
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let wins = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(wins, 1, "exactly one parallel acquire must win");
    }

    #[test]
    fn extend_after_expiry_returns_error() {
        let mgr = LeaseManager::new();
        let thread_id = Uuid::now_v7();
        let req = LeaseRequest {
            executor_kind: ExecutorKind::LocalSmallModel,
            role_id: RoleId::Coder,
            session_id: Uuid::now_v7(),
            lease_duration_secs: 1,
        };
        let now = Utc::now();
        let l = mgr
            .acquire(
                thread_id,
                &[ExecutorKind::LocalSmallModel],
                ClaimMode::Exclusive,
                req,
                now,
            )
            .unwrap();
        let later = now + chrono::Duration::seconds(2);
        let res = mgr.extend(l.lease_id, 60, later);
        assert!(matches!(res, Err(LeaseError::Expired)));
    }

    #[test]
    fn release_then_acquire_succeeds() {
        let mgr = LeaseManager::new();
        let thread_id = Uuid::now_v7();
        let now = Utc::now();
        let r1 = LeaseRequest {
            executor_kind: ExecutorKind::LocalSmallModel,
            role_id: RoleId::Coder,
            session_id: Uuid::now_v7(),
            lease_duration_secs: 120,
        };
        let l1 = mgr
            .acquire(
                thread_id,
                &[ExecutorKind::LocalSmallModel],
                ClaimMode::Exclusive,
                r1,
                now,
            )
            .unwrap();
        mgr.release(l1.lease_id, now).unwrap();
        let r2 = LeaseRequest {
            executor_kind: ExecutorKind::LocalSmallModel,
            role_id: RoleId::Coder,
            session_id: Uuid::now_v7(),
            lease_duration_secs: 120,
        };
        let res = mgr.acquire(
            thread_id,
            &[ExecutorKind::LocalSmallModel],
            ClaimMode::Exclusive,
            r2,
            now,
        );
        assert!(res.is_ok(), "after release a fresh acquire must succeed");
    }
}
