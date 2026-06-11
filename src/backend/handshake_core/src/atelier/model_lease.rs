//! PostgreSQL-backed lease/claim store for parallel model coordination
//! (WP-KERNEL-005 MT-143).
//!
//! The kernel claim-lease contract (`kernel::role_mailbox_claim_lease`)
//! validates and projects lease shapes; this store is where lease reality
//! lives. TTL, stale state, and conflict errors are enforced against the
//! managed PostgreSQL clock:
//!
//! - `claim_model_lease` rejects a claim while an unexpired exclusive lease
//!   or handoff reservation holds the thread (typed [`AtelierError::Conflict`]).
//! - Once `lease_expires_at_utc` passes, the stale lease is observable on
//!   re-read (`lease_expired` / [`ClaimLeaseState::Expired`]) without any
//!   writer, and a new claimant takes the thread over, persisting the prior
//!   row as `taken_over`.
//! - `renew_model_lease` extends only an unexpired active lease held by the
//!   renewing actor; `release_model_lease` requires the holding actor.
//!
//! Every mutation mirrors through the canonical Atelier EventLedger family.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::kernel::role_mailbox_claim_lease::{
    ClaimLeaseState, RoleMailboxClaimMode, RoleMailboxExecutorKind,
};

use super::{AtelierError, AtelierResult, AtelierStore};

pub mod model_lease_event_family {
    pub const MODEL_LEASE_CLAIMED: &str = "atelier.model_lease.claimed";
    pub const MODEL_LEASE_RENEWED: &str = "atelier.model_lease.renewed";
    pub const MODEL_LEASE_RELEASED: &str = "atelier.model_lease.released";
    pub const MODEL_LEASE_TAKEN_OVER: &str = "atelier.model_lease.taken_over";

    pub const ALL: &[&str] = &[
        MODEL_LEASE_CLAIMED,
        MODEL_LEASE_RENEWED,
        MODEL_LEASE_RELEASED,
        MODEL_LEASE_TAKEN_OVER,
    ];
}

fn executor_kind_token(kind: RoleMailboxExecutorKind) -> &'static str {
    match kind {
        RoleMailboxExecutorKind::LocalSmallModel => "local_small_model",
        RoleMailboxExecutorKind::LocalLargeModel => "local_large_model",
        RoleMailboxExecutorKind::CloudModel => "cloud_model",
        RoleMailboxExecutorKind::Reviewer => "reviewer",
        RoleMailboxExecutorKind::Validator => "validator",
        RoleMailboxExecutorKind::Operator => "operator",
        RoleMailboxExecutorKind::WorkflowAutomation => "workflow_automation",
    }
}

fn executor_kind_from_token(token: &str) -> AtelierResult<RoleMailboxExecutorKind> {
    match token {
        "local_small_model" => Ok(RoleMailboxExecutorKind::LocalSmallModel),
        "local_large_model" => Ok(RoleMailboxExecutorKind::LocalLargeModel),
        "cloud_model" => Ok(RoleMailboxExecutorKind::CloudModel),
        "reviewer" => Ok(RoleMailboxExecutorKind::Reviewer),
        "validator" => Ok(RoleMailboxExecutorKind::Validator),
        "operator" => Ok(RoleMailboxExecutorKind::Operator),
        "workflow_automation" => Ok(RoleMailboxExecutorKind::WorkflowAutomation),
        other => Err(AtelierError::Validation(format!(
            "unknown model lease executor kind: {other}"
        ))),
    }
}

fn claim_mode_token(mode: RoleMailboxClaimMode) -> &'static str {
    match mode {
        RoleMailboxClaimMode::ExclusiveLease => "exclusive_lease",
        RoleMailboxClaimMode::SharedObserver => "shared_observer",
        RoleMailboxClaimMode::BroadcastRequest => "broadcast_request",
        RoleMailboxClaimMode::HandoffReservation => "handoff_reservation",
    }
}

fn claim_mode_from_token(token: &str) -> AtelierResult<RoleMailboxClaimMode> {
    match token {
        "exclusive_lease" => Ok(RoleMailboxClaimMode::ExclusiveLease),
        "shared_observer" => Ok(RoleMailboxClaimMode::SharedObserver),
        "broadcast_request" => Ok(RoleMailboxClaimMode::BroadcastRequest),
        "handoff_reservation" => Ok(RoleMailboxClaimMode::HandoffReservation),
        other => Err(AtelierError::Validation(format!(
            "unknown model lease claim mode: {other}"
        ))),
    }
}

fn lease_state_token(state: ClaimLeaseState) -> AtelierResult<&'static str> {
    match state {
        ClaimLeaseState::Active => Ok("active"),
        ClaimLeaseState::Released => Ok("released"),
        ClaimLeaseState::Expired => Ok("expired"),
        ClaimLeaseState::TakenOver => Ok("taken_over"),
        ClaimLeaseState::Unclaimed => Err(AtelierError::Validation(
            "unclaimed lease state is never persisted".into(),
        )),
    }
}

fn lease_state_from_token(token: &str) -> AtelierResult<ClaimLeaseState> {
    match token {
        "active" => Ok(ClaimLeaseState::Active),
        "released" => Ok(ClaimLeaseState::Released),
        "expired" => Ok(ClaimLeaseState::Expired),
        "taken_over" => Ok(ClaimLeaseState::TakenOver),
        other => Err(AtelierError::Validation(format!(
            "unknown model lease state: {other}"
        ))),
    }
}

/// Claim request for a coordination thread.
#[derive(Clone, Debug)]
pub struct NewModelLeaseClaim {
    pub thread_id: String,
    pub executor_kind: RoleMailboxExecutorKind,
    pub actor_id: String,
    pub session_id: String,
    pub claim_mode: RoleMailboxClaimMode,
    pub ttl_seconds: i64,
    pub linked_work_packet_id: String,
    pub linked_micro_task_id: String,
}

/// Persisted lease row plus the database-clock-derived TTL view computed at
/// read time. `lease_age_seconds`, `lease_expired`, and `effective_state`
/// come from PostgreSQL's `NOW()`, never from the caller.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLeaseRecord {
    pub claim_id: Uuid,
    pub thread_id: String,
    pub executor_kind: RoleMailboxExecutorKind,
    pub actor_id: String,
    pub session_id: String,
    pub claim_mode: RoleMailboxClaimMode,
    /// Last persisted state token.
    pub stored_state: ClaimLeaseState,
    pub claimed_at_utc: DateTime<Utc>,
    pub ttl_seconds: i64,
    pub lease_expires_at_utc: DateTime<Utc>,
    pub released_at_utc: Option<DateTime<Utc>>,
    pub taken_over_at_utc: Option<DateTime<Utc>>,
    pub takeover_reason: Option<String>,
    pub prior_claim_id: Option<Uuid>,
    pub linked_work_packet_id: String,
    pub linked_micro_task_id: String,
    /// Seconds elapsed since the claim, per the database clock at read time.
    pub lease_age_seconds: i64,
    /// True when the database clock has passed `lease_expires_at_utc`.
    pub lease_expired: bool,
    /// `stored_state` corrected for TTL: an `Active` row whose expiry has
    /// passed reads back as [`ClaimLeaseState::Expired`].
    pub effective_state: ClaimLeaseState,
}

const LEASE_READ_COLUMNS: &str = r#"claim_id, thread_id, executor_kind, actor_id, session_id,
       claim_mode, lease_state, claimed_at_utc, ttl_seconds,
       lease_expires_at_utc, released_at_utc, taken_over_at_utc,
       takeover_reason, prior_claim_id, linked_work_packet_id,
       linked_micro_task_id,
       GREATEST(
           0,
           FLOOR(EXTRACT(EPOCH FROM (NOW() - claimed_at_utc)))
       )::BIGINT AS lease_age_seconds,
       (NOW() >= lease_expires_at_utc) AS lease_expired"#;

impl AtelierStore {
    /// Claim a coordination thread. Exclusive leases and handoff
    /// reservations enforce one active unexpired claimant per thread; a
    /// conflicting claim fails with [`AtelierError::Conflict`]. An expired
    /// holder is persisted as `taken_over` and the new claim succeeds.
    pub async fn claim_model_lease(
        &self,
        input: &NewModelLeaseClaim,
    ) -> AtelierResult<ModelLeaseRecord> {
        validate_new_claim(input)?;
        let claim_id = Uuid::now_v7();
        let exclusive = matches!(
            input.claim_mode,
            RoleMailboxClaimMode::ExclusiveLease | RoleMailboxClaimMode::HandoffReservation
        );

        let mut tx = self.pool().begin().await?;
        let mut taken_over_claim_id: Option<Uuid> = None;

        if exclusive {
            let holders = sqlx::query(
                r#"SELECT claim_id, actor_id, lease_expires_at_utc,
                          (NOW() >= lease_expires_at_utc) AS lease_expired
                   FROM atelier_model_coordination_lease
                   WHERE thread_id = $1
                     AND lease_state = 'active'
                     AND claim_mode IN ('exclusive_lease', 'handoff_reservation')
                   FOR UPDATE"#,
            )
            .bind(&input.thread_id)
            .fetch_all(&mut *tx)
            .await?;

            for holder in &holders {
                let holder_claim_id: Uuid = holder.get("claim_id");
                let holder_actor_id: String = holder.get("actor_id");
                let holder_expires_at: DateTime<Utc> = holder.get("lease_expires_at_utc");
                let holder_expired: bool = holder.get("lease_expired");
                if !holder_expired {
                    tx.rollback().await?;
                    return Err(AtelierError::Conflict(format!(
                        "thread {} is leased by {} until {} (claim {})",
                        input.thread_id, holder_actor_id, holder_expires_at, holder_claim_id
                    )));
                }
                // Stale holder: persist the takeover so the prior lease's
                // terminal state is durable, not inferred.
                sqlx::query(
                    r#"UPDATE atelier_model_coordination_lease
                       SET lease_state = 'taken_over',
                           taken_over_at_utc = NOW(),
                           takeover_reason = $2
                       WHERE claim_id = $1"#,
                )
                .bind(holder_claim_id)
                .bind(format!(
                    "lease TTL expired; taken over by {}",
                    input.actor_id
                ))
                .execute(&mut *tx)
                .await?;
                taken_over_claim_id = Some(holder_claim_id);
            }
        }

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_model_coordination_lease (
                   claim_id, thread_id, executor_kind, actor_id, session_id,
                   claim_mode, lease_state, claimed_at_utc, ttl_seconds,
                   lease_expires_at_utc, prior_claim_id,
                   linked_work_packet_id, linked_micro_task_id
               )
               VALUES ($1, $2, $3, $4, $5, $6, 'active', NOW(), $7,
                       NOW() + make_interval(secs => $7::double precision),
                       $8, $9, $10)
               RETURNING {LEASE_READ_COLUMNS}"#
        ))
        .bind(claim_id)
        .bind(&input.thread_id)
        .bind(executor_kind_token(input.executor_kind))
        .bind(&input.actor_id)
        .bind(&input.session_id)
        .bind(claim_mode_token(input.claim_mode))
        .bind(input.ttl_seconds)
        .bind(taken_over_claim_id)
        .bind(&input.linked_work_packet_id)
        .bind(&input.linked_micro_task_id)
        .fetch_one(&mut *tx)
        .await?;
        let record = model_lease_from_row(&row)?;

        if let Some(prior_claim_id) = taken_over_claim_id {
            self.record_event_in_tx(
                &mut tx,
                model_lease_event_family::MODEL_LEASE_TAKEN_OVER,
                "atelier_model_lease",
                &prior_claim_id.to_string(),
                serde_json::json!({
                    "claim_id": prior_claim_id,
                    "thread_id": record.thread_id,
                    "taken_over_by_claim_id": record.claim_id,
                    "taken_over_by_actor_id": record.actor_id,
                    "schema": "hsk.atelier.model_lease@1",
                }),
            )
            .await?;
        }
        self.record_event_in_tx(
            &mut tx,
            model_lease_event_family::MODEL_LEASE_CLAIMED,
            "atelier_model_lease",
            &record.claim_id.to_string(),
            model_lease_event_payload(&record),
        )
        .await?;
        tx.commit().await?;
        Ok(record)
    }

    /// Re-read a lease. TTL fields (`lease_age_seconds`, `lease_expired`,
    /// `effective_state`) are recomputed against the database clock on
    /// every call, so expiry is observable without any writer.
    pub async fn get_model_lease(&self, claim_id: Uuid) -> AtelierResult<ModelLeaseRecord> {
        let row = sqlx::query(&format!(
            r#"SELECT {LEASE_READ_COLUMNS}
               FROM atelier_model_coordination_lease
               WHERE claim_id = $1"#
        ))
        .bind(claim_id)
        .fetch_optional(self.pool())
        .await?;

        match row {
            Some(row) => model_lease_from_row(&row),
            None => Err(AtelierError::NotFound(format!(
                "model lease claim_id={claim_id}"
            ))),
        }
    }

    /// Extend an unexpired active lease held by `actor_id` by
    /// `extend_seconds` from the database clock. Renewing an expired or
    /// foreign lease is a typed conflict.
    pub async fn renew_model_lease(
        &self,
        claim_id: Uuid,
        actor_id: &str,
        extend_seconds: i64,
    ) -> AtelierResult<ModelLeaseRecord> {
        if extend_seconds <= 0 {
            return Err(AtelierError::Validation(
                "extend_seconds must be positive".into(),
            ));
        }
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            r#"UPDATE atelier_model_coordination_lease
               SET lease_expires_at_utc =
                       NOW() + make_interval(secs => $3::double precision),
                   ttl_seconds = $3
               WHERE claim_id = $1
                 AND actor_id = $2
                 AND lease_state = 'active'
                 AND NOW() < lease_expires_at_utc
               RETURNING {LEASE_READ_COLUMNS}"#
        ))
        .bind(claim_id)
        .bind(actor_id)
        .bind(extend_seconds)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.rollback().await?;
            let current = self.get_model_lease(claim_id).await?;
            return Err(AtelierError::Conflict(format!(
                "lease {claim_id} cannot be renewed by {actor_id}: state={:?} expired={} holder={}",
                current.effective_state, current.lease_expired, current.actor_id
            )));
        };
        let record = model_lease_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            model_lease_event_family::MODEL_LEASE_RENEWED,
            "atelier_model_lease",
            &record.claim_id.to_string(),
            model_lease_event_payload(&record),
        )
        .await?;
        tx.commit().await?;
        Ok(record)
    }

    /// Release an active lease held by `actor_id`. Releasing a lease held
    /// by another actor (or already terminal) is a typed conflict.
    pub async fn release_model_lease(
        &self,
        claim_id: Uuid,
        actor_id: &str,
    ) -> AtelierResult<ModelLeaseRecord> {
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            r#"UPDATE atelier_model_coordination_lease
               SET lease_state = 'released',
                   released_at_utc = NOW()
               WHERE claim_id = $1
                 AND actor_id = $2
                 AND lease_state = 'active'
               RETURNING {LEASE_READ_COLUMNS}"#
        ))
        .bind(claim_id)
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.rollback().await?;
            let current = self.get_model_lease(claim_id).await?;
            return Err(AtelierError::Conflict(format!(
                "lease {claim_id} cannot be released by {actor_id}: state={:?} holder={}",
                current.stored_state, current.actor_id
            )));
        };
        let record = model_lease_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            model_lease_event_family::MODEL_LEASE_RELEASED,
            "atelier_model_lease",
            &record.claim_id.to_string(),
            model_lease_event_payload(&record),
        )
        .await?;
        tx.commit().await?;
        Ok(record)
    }

    /// All leases for a thread, newest first, with database-clock TTL view.
    pub async fn list_model_leases_for_thread(
        &self,
        thread_id: &str,
    ) -> AtelierResult<Vec<ModelLeaseRecord>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {LEASE_READ_COLUMNS}
               FROM atelier_model_coordination_lease
               WHERE thread_id = $1
               ORDER BY created_at_utc DESC"#
        ))
        .bind(thread_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(model_lease_from_row).collect()
    }
}

fn validate_new_claim(input: &NewModelLeaseClaim) -> AtelierResult<()> {
    for (field, value) in [
        ("thread_id", input.thread_id.as_str()),
        ("actor_id", input.actor_id.as_str()),
        ("session_id", input.session_id.as_str()),
        ("linked_work_packet_id", input.linked_work_packet_id.as_str()),
        ("linked_micro_task_id", input.linked_micro_task_id.as_str()),
    ] {
        if value.trim().is_empty() || value.trim() != value {
            return Err(AtelierError::Validation(format!(
                "{field} must not be empty or padded"
            )));
        }
    }
    if input.ttl_seconds <= 0 {
        return Err(AtelierError::Validation(
            "ttl_seconds must be positive".into(),
        ));
    }
    Ok(())
}

fn model_lease_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<ModelLeaseRecord> {
    let executor_kind: String = row.get("executor_kind");
    let claim_mode: String = row.get("claim_mode");
    let lease_state: String = row.get("lease_state");
    let stored_state = lease_state_from_token(&lease_state)?;
    let lease_expired: bool = row.get("lease_expired");
    let effective_state = if stored_state == ClaimLeaseState::Active && lease_expired {
        ClaimLeaseState::Expired
    } else {
        stored_state
    };
    Ok(ModelLeaseRecord {
        claim_id: row.get("claim_id"),
        thread_id: row.get("thread_id"),
        executor_kind: executor_kind_from_token(&executor_kind)?,
        actor_id: row.get("actor_id"),
        session_id: row.get("session_id"),
        claim_mode: claim_mode_from_token(&claim_mode)?,
        stored_state,
        claimed_at_utc: row.get("claimed_at_utc"),
        ttl_seconds: row.get("ttl_seconds"),
        lease_expires_at_utc: row.get("lease_expires_at_utc"),
        released_at_utc: row.get("released_at_utc"),
        taken_over_at_utc: row.get("taken_over_at_utc"),
        takeover_reason: row.get("takeover_reason"),
        prior_claim_id: row.get("prior_claim_id"),
        linked_work_packet_id: row.get("linked_work_packet_id"),
        linked_micro_task_id: row.get("linked_micro_task_id"),
        lease_age_seconds: row.get("lease_age_seconds"),
        lease_expired,
        effective_state,
    })
}

fn model_lease_event_payload(record: &ModelLeaseRecord) -> serde_json::Value {
    serde_json::json!({
        "claim_id": record.claim_id,
        "thread_id": record.thread_id,
        "executor_kind": executor_kind_token(record.executor_kind),
        "actor_id": record.actor_id,
        "session_id": record.session_id,
        "claim_mode": claim_mode_token(record.claim_mode),
        "lease_state": lease_state_token(record.stored_state).unwrap_or("active"),
        "claimed_at_utc": record.claimed_at_utc,
        "ttl_seconds": record.ttl_seconds,
        "lease_expires_at_utc": record.lease_expires_at_utc,
        "prior_claim_id": record.prior_claim_id,
        "linked_work_packet_id": record.linked_work_packet_id,
        "linked_micro_task_id": record.linked_micro_task_id,
        "schema": "hsk.atelier.model_lease@1",
    })
}
