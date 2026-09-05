//! Store-backed lease/claim coordination for parallel models
//! (WP-KERNEL-005 MT-143), on the embedded SurrealDB store.
//!
//! The kernel claim-lease contract (`kernel::role_mailbox_claim_lease`)
//! validates and projects lease shapes; this store is where lease reality
//! lives. TTL, stale state, and conflict errors are enforced against the
//! store clock:
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
use std::sync::OnceLock;
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use crate::kernel::role_mailbox_claim_lease::{
    ClaimLeaseState, RoleMailboxClaimMode, RoleMailboxExecutorKind,
};

use super::{uuid_from_record_link, AtelierError, AtelierResult, AtelierStore};

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

/// Wire token for an executor kind (shared with the model-ops HTTP surface).
pub fn executor_kind_token(kind: RoleMailboxExecutorKind) -> &'static str {
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

/// Wire token for a claim mode (shared with the model-ops HTTP surface).
pub fn claim_mode_token(mode: RoleMailboxClaimMode) -> &'static str {
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

/// Persisted lease row plus the store-clock-derived TTL view computed at
/// read time. `lease_age_seconds`, `lease_expired`, and `effective_state`
/// come from the store clock, never from the caller.
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
    /// Seconds elapsed since the claim, per the store clock at read time.
    pub lease_age_seconds: i64,
    /// True when the store clock has passed `lease_expires_at_utc`.
    pub lease_expired: bool,
    /// `stored_state` corrected for TTL: an `Active` row whose expiry has
    /// passed reads back as [`ClaimLeaseState::Expired`].
    pub effective_state: ClaimLeaseState,
}

/// One lease row as the store returns it, TTL view included.
#[derive(SurrealValue)]
struct ModelLeaseRow {
    claim_id: SurrealUuid,
    thread_id: String,
    executor_kind: String,
    actor_id: String,
    session_id: String,
    claim_mode: String,
    lease_state: String,
    claimed_at_utc: Datetime,
    ttl_seconds: i64,
    lease_expires_at_utc: Datetime,
    released_at_utc: Option<Datetime>,
    taken_over_at_utc: Option<Datetime>,
    takeover_reason: Option<String>,
    prior_claim_id: Option<RecordId>,
    linked_work_packet_id: String,
    linked_micro_task_id: String,
    lease_age_seconds: i64,
    lease_expired: bool,
}

impl TryFrom<ModelLeaseRow> for ModelLeaseRecord {
    type Error = AtelierError;

    fn try_from(row: ModelLeaseRow) -> AtelierResult<Self> {
        let stored_state = lease_state_from_token(&row.lease_state)?;
        let effective_state = if stored_state == ClaimLeaseState::Active && row.lease_expired {
            ClaimLeaseState::Expired
        } else {
            stored_state
        };
        let prior_claim_id = row
            .prior_claim_id
            .as_ref()
            .map(|link| uuid_from_record_link("prior_claim_id", link))
            .transpose()?;
        Ok(ModelLeaseRecord {
            claim_id: row.claim_id.into(),
            thread_id: row.thread_id,
            executor_kind: executor_kind_from_token(&row.executor_kind)?,
            actor_id: row.actor_id,
            session_id: row.session_id,
            claim_mode: claim_mode_from_token(&row.claim_mode)?,
            stored_state,
            claimed_at_utc: row.claimed_at_utc.into(),
            ttl_seconds: row.ttl_seconds,
            lease_expires_at_utc: row.lease_expires_at_utc.into(),
            released_at_utc: row.released_at_utc.map(Into::into),
            taken_over_at_utc: row.taken_over_at_utc.map(Into::into),
            takeover_reason: row.takeover_reason,
            prior_claim_id,
            linked_work_packet_id: row.linked_work_packet_id,
            linked_micro_task_id: row.linked_micro_task_id,
            lease_age_seconds: row.lease_age_seconds,
            lease_expired: row.lease_expired,
            effective_state,
        })
    }
}

/// The lease read projection: stored fields plus the store-clock TTL view.
/// (The former SQL computed the same two fields with `NOW()`.)
const LEASE_READ_COLUMNS: &str =
    "claim_id, thread_id, executor_kind, actor_id, session_id, claim_mode, lease_state, \
     claimed_at_utc, ttl_seconds, lease_expires_at_utc, released_at_utc, taken_over_at_utc, \
     takeover_reason, prior_claim_id, linked_work_packet_id, linked_micro_task_id, \
     math::max([0, duration::secs(time::now() - claimed_at_utc)]) AS lease_age_seconds, \
     (time::now() >= lease_expires_at_utc) AS lease_expired";

#[derive(Clone, SurrealValue)]
struct ClaimLeaseBindings {
    record_id: RecordId,
    claim_id: SurrealUuid,
    thread_id: String,
    executor_kind: String,
    actor_id: String,
    session_id: String,
    claim_mode: String,
    ttl_seconds: i64,
    linked_work_packet_id: String,
    linked_micro_task_id: String,
    exclusive: bool,
    takeover_reason: String,
}

/// The claim decision as one atomic statement: read the active exclusive
/// holders, refuse while one is live, take over the stale ones, and create
/// the new lease — all in a single transaction, which is what the former
/// row-locked transaction guaranteed.
const CLAIM_LEASE_STATEMENT: &str = concat!(
    "RETURN { \
       LET $now = time::now(); \
       LET $holders = IF $exclusive { \
           (SELECT claim_id, actor_id, lease_expires_at_utc \
            FROM atelier_model_coordination_lease \
            WHERE thread_id = $thread_id AND lease_state = 'active' \
              AND claim_mode IN ['exclusive_lease', 'handoff_reservation']) \
         } ELSE { [] }; \
       LET $live = (SELECT * FROM $holders WHERE lease_expires_at_utc > $now); \
       LET $stale = (SELECT * FROM $holders WHERE lease_expires_at_utc <= $now); \
       LET $can_claim = array::len($live) = 0; \
       LET $prior = IF array::len($stale) = 0 { NONE } ELSE { \
         type::record('atelier_model_coordination_lease', $stale[0].claim_id) \
       }; \
       IF $can_claim { \
         IF $exclusive { \
           UPDATE atelier_model_coordination_lease SET \
             lease_state = 'taken_over', \
             taken_over_at_utc = $now, \
             takeover_reason = $takeover_reason \
           WHERE thread_id = $thread_id AND lease_state = 'active' \
             AND claim_mode IN ['exclusive_lease', 'handoff_reservation'] \
             AND lease_expires_at_utc <= $now; \
         }; \
         CREATE $record_id CONTENT { \
           claim_id: $claim_id, \
           thread_id: $thread_id, \
           executor_kind: $executor_kind, \
           actor_id: $actor_id, \
           session_id: $session_id, \
           claim_mode: $claim_mode, \
           lease_state: 'active', \
           claimed_at_utc: $now, \
           ttl_seconds: $ttl_seconds, \
           lease_expires_at_utc: $now + duration::from::secs($ttl_seconds), \
           prior_claim_id: $prior, \
           linked_work_packet_id: $linked_work_packet_id, \
           linked_micro_task_id: $linked_micro_task_id \
         }; \
       }; \
       RETURN { \
         claimed: $can_claim, \
         live_holder_claim_id: $live[0].claim_id, \
         live_holder_actor_id: $live[0].actor_id, \
         live_holder_expires_at_utc: $live[0].lease_expires_at_utc, \
         taken_over_claim_id: $stale[0].claim_id, \
         record: (SELECT ",
    "claim_id, thread_id, executor_kind, actor_id, session_id, claim_mode, lease_state, \
     claimed_at_utc, ttl_seconds, lease_expires_at_utc, released_at_utc, taken_over_at_utc, \
     takeover_reason, prior_claim_id, linked_work_packet_id, linked_micro_task_id, \
     math::max([0, duration::secs(time::now() - claimed_at_utc)]) AS lease_age_seconds, \
     (time::now() >= lease_expires_at_utc) AS lease_expired",
    " FROM $record_id) \
       }; };"
);

/// The outcome object [`CLAIM_LEASE_STATEMENT`] returns.
#[derive(SurrealValue)]
struct ClaimLeaseOutcome {
    claimed: bool,
    live_holder_claim_id: Option<SurrealUuid>,
    live_holder_actor_id: Option<String>,
    live_holder_expires_at_utc: Option<Datetime>,
    taken_over_claim_id: Option<SurrealUuid>,
    record: Vec<ModelLeaseRow>,
}

#[derive(SurrealValue)]
struct ClaimIdBinding {
    claim_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct ThreadIdBinding {
    thread_id: String,
}

#[derive(Clone, SurrealValue)]
struct RenewLeaseBindings {
    claim_id: SurrealUuid,
    actor_id: String,
    extend_seconds: i64,
}

const GET_LEASE_STATEMENT: &str = concat!(
    "SELECT ",
    "claim_id, thread_id, executor_kind, actor_id, session_id, claim_mode, lease_state, \
     claimed_at_utc, ttl_seconds, lease_expires_at_utc, released_at_utc, taken_over_at_utc, \
     takeover_reason, prior_claim_id, linked_work_packet_id, linked_micro_task_id, \
     math::max([0, duration::secs(time::now() - claimed_at_utc)]) AS lease_age_seconds, \
     (time::now() >= lease_expires_at_utc) AS lease_expired",
    " FROM atelier_model_coordination_lease WHERE claim_id = $claim_id LIMIT 1;"
);

/// Guarded renew: only an unexpired active lease held by the renewing actor
/// matches; zero updated rows is the lost race the caller maps to Conflict.
const RENEW_LEASE_STATEMENT: &str = concat!(
    "RETURN { \
       LET $updated = (UPDATE atelier_model_coordination_lease SET \
           lease_expires_at_utc = time::now() + duration::from::secs($extend_seconds), \
           ttl_seconds = $extend_seconds \
         WHERE claim_id = $claim_id AND actor_id = $actor_id \
           AND lease_state = 'active' AND time::now() < lease_expires_at_utc \
         RETURN AFTER); \
       RETURN (SELECT ",
    "claim_id, thread_id, executor_kind, actor_id, session_id, claim_mode, lease_state, \
     claimed_at_utc, ttl_seconds, lease_expires_at_utc, released_at_utc, taken_over_at_utc, \
     takeover_reason, prior_claim_id, linked_work_packet_id, linked_micro_task_id, \
     math::max([0, duration::secs(time::now() - claimed_at_utc)]) AS lease_age_seconds, \
     (time::now() >= lease_expires_at_utc) AS lease_expired",
    " FROM $updated.id); };"
);

const RELEASE_LEASE_STATEMENT: &str = concat!(
    "RETURN { \
       LET $updated = (UPDATE atelier_model_coordination_lease SET \
           lease_state = 'released', \
           released_at_utc = time::now() \
         WHERE claim_id = $claim_id AND actor_id = $actor_id \
           AND lease_state = 'active' \
         RETURN AFTER); \
       RETURN (SELECT ",
    "claim_id, thread_id, executor_kind, actor_id, session_id, claim_mode, lease_state, \
     claimed_at_utc, ttl_seconds, lease_expires_at_utc, released_at_utc, taken_over_at_utc, \
     takeover_reason, prior_claim_id, linked_work_packet_id, linked_micro_task_id, \
     math::max([0, duration::secs(time::now() - claimed_at_utc)]) AS lease_age_seconds, \
     (time::now() >= lease_expires_at_utc) AS lease_expired",
    " FROM $updated.id); };"
);

const LIST_LEASES_FOR_THREAD_STATEMENT: &str = concat!(
    "SELECT ",
    "claim_id, thread_id, executor_kind, actor_id, session_id, claim_mode, lease_state, \
     claimed_at_utc, ttl_seconds, lease_expires_at_utc, released_at_utc, taken_over_at_utc, \
     takeover_reason, prior_claim_id, linked_work_packet_id, linked_micro_task_id, \
     math::max([0, duration::secs(time::now() - claimed_at_utc)]) AS lease_age_seconds, \
     (time::now() >= lease_expires_at_utc) AS lease_expired",
    " FROM atelier_model_coordination_lease WHERE thread_id = $thread_id \
     ORDER BY created_at_utc DESC;"
);

/// Process-local serialization point for exclusive lease claims.
///
/// The PostgreSQL reference took `pg_advisory_xact_lock(5023022, hashtext(thread_id))`
/// inside the claim transaction. The embedded SurrealDB store has no advisory
/// locks, and its optimistic RocksDB transactions only detect write-write
/// conflicts, so two concurrent first claims on the same thread could both read
/// an empty holder set and both commit. The embedded store is owned by exactly
/// one kernel process, so an in-process async mutex around the claim statement
/// is the equivalent (and sufficient) serialization guarantee: exactly one of N
/// concurrent exclusive claimants wins, the rest observe the committed holder
/// and fail with a typed conflict.
static EXCLUSIVE_CLAIM_SERIALIZER: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

fn exclusive_claim_serializer() -> &'static tokio::sync::Mutex<()> {
    EXCLUSIVE_CLAIM_SERIALIZER.get_or_init(|| tokio::sync::Mutex::new(()))
}

impl AtelierStore {
    /// Claim a coordination thread. Exclusive leases and handoff
    /// reservations enforce one active unexpired claimant per thread; a
    /// conflicting claim fails with [`AtelierError::Conflict`]. An expired
    /// holder is persisted as `taken_over` and the new claim succeeds.
    ///
    /// The claim decision itself (check, takeover, create) is one atomic
    /// statement; the mirror events land immediately after it commits.
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
        // WP-CKC MT-022/MT-062: serialize exclusive claims per process so
        // concurrent first claimants cannot both observe "no active holder"
        // (the reference used a PostgreSQL advisory lock keyed by thread_id).
        let claim_guard = if exclusive {
            Some(exclusive_claim_serializer().lock().await)
        } else {
            None
        };

        let bindings = ClaimLeaseBindings {
            record_id: RecordId::new(
                "atelier_model_coordination_lease",
                SurrealUuid::from(claim_id),
            ),
            claim_id: SurrealUuid::from(claim_id),
            thread_id: input.thread_id.clone(),
            executor_kind: executor_kind_token(input.executor_kind).to_owned(),
            actor_id: input.actor_id.clone(),
            session_id: input.session_id.clone(),
            claim_mode: claim_mode_token(input.claim_mode).to_owned(),
            ttl_seconds: input.ttl_seconds,
            linked_work_packet_id: input.linked_work_packet_id.clone(),
            linked_micro_task_id: input.linked_micro_task_id.clone(),
            exclusive,
            takeover_reason: format!("lease TTL expired; taken over by {}", input.actor_id),
        };
        let outcome = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first::<ClaimLeaseOutcome, _>(CLAIM_LEASE_STATEMENT, bindings)
                        .await
                })
            })
            .await;
        drop(claim_guard);
        let outcome = outcome?.ok_or_else(|| {
            AtelierError::Internal("claiming a model lease returned no outcome".to_owned())
        })?;

        if !outcome.claimed {
            let holder_actor = outcome.live_holder_actor_id.unwrap_or_default();
            let holder_expires = outcome
                .live_holder_expires_at_utc
                .map(|value| DateTime::<Utc>::from(value).to_string())
                .unwrap_or_default();
            let holder_claim = outcome
                .live_holder_claim_id
                .map(|value| Uuid::from(value).to_string())
                .unwrap_or_default();
            return Err(AtelierError::Conflict(format!(
                "thread {} is leased by {} until {} (claim {})",
                input.thread_id, holder_actor, holder_expires, holder_claim
            )));
        }

        let record: ModelLeaseRecord = outcome
            .record
            .into_iter()
            .next()
            .ok_or_else(|| {
                AtelierError::Internal("claimed model lease returned no row".to_owned())
            })?
            .try_into()?;

        if let Some(prior_claim_id) = outcome.taken_over_claim_id {
            let prior_claim_id: Uuid = prior_claim_id.into();
            self.record_event(
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
        self.record_event(
            model_lease_event_family::MODEL_LEASE_CLAIMED,
            "atelier_model_lease",
            &record.claim_id.to_string(),
            model_lease_event_payload(&record),
        )
        .await?;
        Ok(record)
    }

    /// Re-read a lease. TTL fields (`lease_age_seconds`, `lease_expired`,
    /// `effective_state`) are recomputed against the store clock on
    /// every call, so expiry is observable without any writer.
    pub async fn get_model_lease(&self, claim_id: Uuid) -> AtelierResult<ModelLeaseRecord> {
        let bindings = ClaimIdBinding {
            claim_id: SurrealUuid::from(claim_id),
        };
        let row: Option<ModelLeaseRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_LEASE_STATEMENT, bindings).await })
            })
            .await?;
        match row {
            Some(row) => row.try_into(),
            None => Err(AtelierError::NotFound(format!(
                "model lease claim_id={claim_id}"
            ))),
        }
    }

    /// Extend an unexpired active lease held by `actor_id` by
    /// `extend_seconds` from the store clock. Renewing an expired or
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
        let bindings = RenewLeaseBindings {
            claim_id: SurrealUuid::from(claim_id),
            actor_id: actor_id.to_owned(),
            extend_seconds,
        };
        let row: Option<ModelLeaseRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(RENEW_LEASE_STATEMENT, bindings).await })
            })
            .await?;

        let Some(row) = row else {
            let current = self.get_model_lease(claim_id).await?;
            return Err(AtelierError::Conflict(format!(
                "lease {claim_id} cannot be renewed by {actor_id}: state={:?} expired={} holder={}",
                current.effective_state, current.lease_expired, current.actor_id
            )));
        };
        let record: ModelLeaseRecord = row.try_into()?;
        self.record_event(
            model_lease_event_family::MODEL_LEASE_RENEWED,
            "atelier_model_lease",
            &record.claim_id.to_string(),
            model_lease_event_payload(&record),
        )
        .await?;
        Ok(record)
    }

    /// Release an active lease held by `actor_id`. Releasing a lease held
    /// by another actor (or already terminal) is a typed conflict.
    pub async fn release_model_lease(
        &self,
        claim_id: Uuid,
        actor_id: &str,
    ) -> AtelierResult<ModelLeaseRecord> {
        let bindings = RenewLeaseBindings {
            claim_id: SurrealUuid::from(claim_id),
            actor_id: actor_id.to_owned(),
            extend_seconds: 0,
        };
        let row: Option<ModelLeaseRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(RELEASE_LEASE_STATEMENT, bindings).await })
            })
            .await?;

        let Some(row) = row else {
            let current = self.get_model_lease(claim_id).await?;
            return Err(AtelierError::Conflict(format!(
                "lease {claim_id} cannot be released by {actor_id}: state={:?} holder={}",
                current.stored_state, current.actor_id
            )));
        };
        let record: ModelLeaseRecord = row.try_into()?;
        self.record_event(
            model_lease_event_family::MODEL_LEASE_RELEASED,
            "atelier_model_lease",
            &record.claim_id.to_string(),
            model_lease_event_payload(&record),
        )
        .await?;
        Ok(record)
    }

    /// All leases for a thread, newest first, with store-clock TTL view.
    pub async fn list_model_leases_for_thread(
        &self,
        thread_id: &str,
    ) -> AtelierResult<Vec<ModelLeaseRecord>> {
        let bindings = ThreadIdBinding {
            thread_id: thread_id.to_owned(),
        };
        let rows: Vec<ModelLeaseRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_LEASES_FOR_THREAD_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        rows.into_iter().map(ModelLeaseRecord::try_from).collect()
    }
}

fn validate_new_claim(input: &NewModelLeaseClaim) -> AtelierResult<()> {
    for (field, value) in [
        ("thread_id", input.thread_id.as_str()),
        ("actor_id", input.actor_id.as_str()),
        ("session_id", input.session_id.as_str()),
        (
            "linked_work_packet_id",
            input.linked_work_packet_id.as_str(),
        ),
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
