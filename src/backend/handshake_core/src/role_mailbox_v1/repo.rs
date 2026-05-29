//! MT-177 Role Mailbox Postgres repository with transactional lifecycle enforcement.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use super::handoff::{MailboxHandoffBundleV1, TranscriptPointer};
use super::lease::{LeaseError, LeaseRequest, RoleMailboxClaimLeaseV1, TakeoverPolicy};
use super::lifecycle::{
    transition_message_state, transition_thread_state, InvalidTransition, MessageDeliveryState,
    ThreadLifecycleState,
};
use super::message::{MessageType, RoleMailboxMessage, RoleMailboxMessageId};
use super::router::ExecutorKind;
use super::thread::{
    ClaimMode, LinkedRecordKind, ResponseAuthorityScope, RoleMailboxThread, RoleMailboxThreadId,
};
use crate::role_mailbox::RoleId;

#[derive(Debug, Error)]
pub enum MailboxError {
    #[error("invalid transition: {0}")]
    InvalidTransition(#[from] InvalidTransition),
    #[error("thread not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("thread in terminal lifecycle state")]
    TerminalState,
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("parse error: {0}")]
    Parse(String),
    /// MT-183: caller-supplied `content_hash` did not match the canonical-JSON
    /// recomputed hash for the handoff bundle. The repo recomputes on insert
    /// and refuses tampered input per `red_team.minimum_controls`.
    #[error("handoff bundle content_hash mismatch (expected {expected}, got {got})")]
    HashMismatch { expected: String, got: String },
}

/// Postgres-backed transactional repository. Production binds to a `PgPool`;
/// tests construct via `RoleMailboxRepository::new(pool)`.
///
/// CX-503R: type bound on `PgPool` only — SQLite is rejected at the type level.
pub struct RoleMailboxRepository {
    pool: PgPool,
}

impl RoleMailboxRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Apply the cluster-X.1 schema migration. Idempotent.
    pub async fn ensure_schema(&self) -> Result<(), MailboxError> {
        sqlx::raw_sql(SCHEMA_SQL).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn create_thread(
        &self,
        thread: RoleMailboxThread,
    ) -> Result<RoleMailboxThread, MailboxError> {
        let allowlist_json = serde_json::to_value(&thread.executor_kind_allowlist)?;
        let claim_mode = serde_json::to_value(&thread.claim_mode)?
            .as_str()
            .ok_or_else(|| MailboxError::Parse("claim_mode encode".to_string()))?
            .to_string();
        let takeover = serde_json::to_value(&thread.takeover_policy)?
            .as_str()
            .ok_or_else(|| MailboxError::Parse("takeover encode".to_string()))?
            .to_string();
        let scope = serde_json::to_value(&thread.response_authority_scope)?
            .as_str()
            .ok_or_else(|| MailboxError::Parse("scope encode".to_string()))?
            .to_string();
        let linked = serde_json::to_value(&thread.linked_record_kind)?
            .as_str()
            .ok_or_else(|| MailboxError::Parse("linked encode".to_string()))?
            .to_string();
        sqlx::query(
            r#"INSERT INTO role_mailbox_thread
               (thread_id, title, linked_record_kind, linked_record_id, lifecycle_state,
                executor_kind_allowlist, claim_mode, lease_duration_secs, takeover_policy,
                response_authority_scope, created_at_utc, updated_at_utc, expires_at_utc, archived_at_utc)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)"#,
        )
        .bind(thread.thread_id.as_uuid())
        .bind(&thread.title)
        .bind(linked)
        .bind(thread.linked_record_id.as_deref())
        .bind(thread.lifecycle_state.as_str())
        .bind(allowlist_json)
        .bind(claim_mode)
        .bind(thread.lease_duration_secs.map(|v| v as i64))
        .bind(takeover)
        .bind(scope)
        .bind(thread.created_at_utc)
        .bind(thread.updated_at_utc)
        .bind(thread.expires_at_utc)
        .bind(thread.archived_at_utc)
        .execute(&self.pool)
        .await?;
        Ok(thread)
    }

    pub async fn get_thread(
        &self,
        thread_id: RoleMailboxThreadId,
    ) -> Result<Option<RoleMailboxThread>, MailboxError> {
        let row: Option<(
            Uuid,
            String,
            String,
            Option<String>,
            String,
            Value,
            String,
            Option<i64>,
            String,
            String,
            DateTime<Utc>,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            Option<DateTime<Utc>>,
        )> = sqlx::query_as(
            r#"SELECT thread_id, title, linked_record_kind, linked_record_id, lifecycle_state,
                       executor_kind_allowlist, claim_mode, lease_duration_secs, takeover_policy,
                       response_authority_scope, created_at_utc, updated_at_utc, expires_at_utc, archived_at_utc
               FROM role_mailbox_thread WHERE thread_id = $1"#,
        )
        .bind(thread_id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;
        let Some(r) = row else { return Ok(None) };
        let allowlist: Vec<ExecutorKind> = serde_json::from_value(r.5)?;
        let linked: LinkedRecordKind = serde_json::from_value(Value::String(r.2))?;
        let lifecycle: ThreadLifecycleState = serde_json::from_value(Value::String(r.4))?;
        let claim_mode: ClaimMode = serde_json::from_value(Value::String(r.6))?;
        let takeover: TakeoverPolicy = serde_json::from_value(Value::String(r.8))?;
        let scope: ResponseAuthorityScope = serde_json::from_value(Value::String(r.9))?;
        Ok(Some(RoleMailboxThread {
            thread_id: RoleMailboxThreadId(r.0),
            title: r.1,
            linked_record_kind: linked,
            linked_record_id: r.3,
            lifecycle_state: lifecycle,
            executor_kind_allowlist: allowlist,
            claim_mode,
            lease_duration_secs: r.7.map(|v| v as u32),
            takeover_policy: takeover,
            response_authority_scope: scope,
            created_at_utc: r.10,
            updated_at_utc: r.11,
            expires_at_utc: r.12,
            archived_at_utc: r.13,
        }))
    }

    /// Transition `thread_id`'s lifecycle to `requested` inside a
    /// `SELECT ... FOR UPDATE` transaction. Concurrent callers see exactly-one-winner semantics.
    pub async fn update_thread_lifecycle(
        &self,
        thread_id: RoleMailboxThreadId,
        requested: ThreadLifecycleState,
    ) -> Result<RoleMailboxThread, MailboxError> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT lifecycle_state, updated_at_utc FROM role_mailbox_thread WHERE thread_id = $1 FOR UPDATE",
        )
        .bind(thread_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        let Some((current_str, _)) = row else {
            return Err(MailboxError::NotFound);
        };
        let current: ThreadLifecycleState = serde_json::from_value(Value::String(current_str))?;
        let next = transition_thread_state(current, requested)?;
        sqlx::query(
            r#"UPDATE role_mailbox_thread SET lifecycle_state = $1, updated_at_utc = NOW() WHERE thread_id = $2"#,
        )
        .bind(next.as_str())
        .bind(thread_id.as_uuid())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.get_thread(thread_id)
            .await?
            .ok_or(MailboxError::NotFound)
    }

    /// Append a new message. Rejects with `TerminalState` if the thread is
    /// resolved/expired/archived.
    pub async fn append_message(
        &self,
        thread_id: RoleMailboxThreadId,
        message_type: MessageType,
        from_role: RoleId,
        to_roles: Vec<RoleId>,
        body: Value,
    ) -> Result<RoleMailboxMessage, MailboxError> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT lifecycle_state FROM role_mailbox_thread WHERE thread_id = $1 FOR UPDATE",
        )
        .bind(thread_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        let Some((current_str,)) = row else {
            return Err(MailboxError::NotFound);
        };
        let current: ThreadLifecycleState = serde_json::from_value(Value::String(current_str))?;
        if current.is_terminal() {
            return Err(MailboxError::TerminalState);
        }
        let msg_id = RoleMailboxMessageId::new_v7();
        let to_roles_json =
            serde_json::to_value(&to_roles.iter().map(|r| r.to_string()).collect::<Vec<_>>())?;
        let from_role_str = from_role.to_string();
        let now = Utc::now();
        sqlx::query(
            r#"INSERT INTO role_mailbox_message
                (message_id, thread_id, message_type, from_role, to_roles,
                 expected_response, expires_at_utc, delivery_state, body,
                 parent_message_id, created_at_utc)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)"#,
        )
        .bind(msg_id.as_uuid())
        .bind(thread_id.as_uuid())
        .bind(message_type.as_str())
        .bind(&from_role_str)
        .bind(to_roles_json)
        .bind(None::<Value>)
        .bind(None::<DateTime<Utc>>)
        .bind(MessageDeliveryState::Queued.as_str())
        .bind(&body)
        .bind(None::<Uuid>)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(RoleMailboxMessage {
            message_id: msg_id,
            thread_id,
            message_type,
            from_role,
            to_roles,
            expected_response: None,
            expires_at_utc: None,
            delivery_state: MessageDeliveryState::Queued,
            body,
            parent_message_id: None,
            created_at_utc: now,
        })
    }

    /// List messages for a thread, chronological order.
    pub async fn list_thread_messages(
        &self,
        thread_id: RoleMailboxThreadId,
    ) -> Result<Vec<RoleMailboxMessage>, MailboxError> {
        let rows: Vec<(
            Uuid,
            Uuid,
            String,
            String,
            Value,
            String,
            Value,
            Option<Uuid>,
            DateTime<Utc>,
        )> = sqlx::query_as(
            r#"SELECT message_id, thread_id, message_type, from_role, to_roles,
                       delivery_state, body, parent_message_id, created_at_utc
               FROM role_mailbox_message WHERE thread_id = $1
               ORDER BY created_at_utc ASC, message_id ASC"#,
        )
        .bind(thread_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let message_type: MessageType = serde_json::from_value(Value::String(r.2))?;
            let from_role =
                RoleId::parse(&r.3).map_err(|e| MailboxError::Parse(format!("from_role: {e}")))?;
            let to_role_strings: Vec<String> = serde_json::from_value(r.4)?;
            let to_roles: Result<Vec<RoleId>, _> =
                to_role_strings.iter().map(|s| RoleId::parse(s)).collect();
            let to_roles = to_roles.map_err(|e| MailboxError::Parse(format!("to_role: {e}")))?;
            let delivery: MessageDeliveryState = serde_json::from_value(Value::String(r.5))?;
            out.push(RoleMailboxMessage {
                message_id: RoleMailboxMessageId(r.0),
                thread_id: RoleMailboxThreadId(r.1),
                message_type,
                from_role,
                to_roles,
                expected_response: None,
                expires_at_utc: None,
                delivery_state: delivery,
                body: r.6,
                parent_message_id: r.7.map(RoleMailboxMessageId),
                created_at_utc: r.8,
            });
        }
        Ok(out)
    }

    pub async fn list_threads_by_state(
        &self,
        state: ThreadLifecycleState,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<RoleMailboxThread>, MailboxError> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"SELECT thread_id FROM role_mailbox_thread WHERE lifecycle_state = $1
               ORDER BY updated_at_utc DESC LIMIT $2 OFFSET $3"#,
        )
        .bind(state.as_str())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        let mut threads = Vec::with_capacity(rows.len());
        for r in rows {
            if let Some(t) = self.get_thread(RoleMailboxThreadId(r.0)).await? {
                threads.push(t);
            }
        }
        Ok(threads)
    }

    /// Dead-letter a message: transition delivery_state to DeadLettered with
    /// `reason` recorded in `audit_reason`.
    pub async fn dead_letter_message(
        &self,
        message_id: RoleMailboxMessageId,
        reason: String,
    ) -> Result<(), MailboxError> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT delivery_state FROM role_mailbox_message WHERE message_id = $1 FOR UPDATE",
        )
        .bind(message_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        let Some((current_str,)) = row else {
            return Err(MailboxError::NotFound);
        };
        let current: MessageDeliveryState = serde_json::from_value(Value::String(current_str))?;
        let next = transition_message_state(current, MessageDeliveryState::DeadLettered)?;
        sqlx::query(
            r#"UPDATE role_mailbox_message SET delivery_state = $1, audit_reason = $2 WHERE message_id = $3"#,
        )
        .bind(next.as_str())
        .bind(&reason)
        .bind(message_id.as_uuid())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Count pending (queued / delivered) messages for `to_role` — used by
    /// MT-182 backpressure inbox-cap check.
    pub async fn count_pending_messages_for_role(
        &self,
        role: &RoleId,
    ) -> Result<u32, MailboxError> {
        let row: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*)::BIGINT FROM role_mailbox_message
               WHERE delivery_state IN ('queued', 'delivered')
                 AND to_roles @> to_jsonb(ARRAY[$1::text])"#,
        )
        .bind(role.to_string())
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0 as u32)
    }

    // ---------- MT-180 Lease primitive ----------

    /// MT-180: Acquire a `RoleMailboxClaimLeaseV1` for `thread_id`.
    ///
    /// Per spec v02.186 §02-system-architecture.md role mailbox subsection
    /// [ADD v02.176] lines 6151-6156 the algorithm runs inside a Postgres
    /// transaction:
    ///   1. `SELECT thread FOR UPDATE` — serialises concurrent acquirers
    ///      against the same thread row.
    ///   2. Verify `thread.claim_mode` allows `request.executor_kind` via the
    ///      `executor_kind_allowlist`.
    ///   3. Reject if the thread is in a terminal lifecycle state.
    ///   4. If an unexpired non-released lease exists, return
    ///      `LeaseError::LeaseHeldByOther` (except for `ClaimMode::Open`).
    ///      For `ClaimMode::Handoff`, callers should use [`Self::takeover`]
    ///      with the explicit predecessor lease id.
    ///   5. Sweep expired-but-unreleased leases by marking them released
    ///      with `released_at_utc = now()`. This keeps the partial unique
    ///      index `WHERE released_at_utc IS NULL` honest for the new INSERT.
    ///   6. INSERT the new lease row. The partial unique index `WHERE
    ///      released_at_utc IS NULL` enforces exactly-one-active-lease-per-
    ///      thread at the database level (spec line 6156). A concurrent
    ///      caller that bypasses this method via a direct INSERT will see
    ///      a `23505 unique_violation`, which surfaces as
    ///      `LeaseError::Conflict`.
    pub async fn acquire_lease(
        &self,
        thread_id: RoleMailboxThreadId,
        request: LeaseRequest,
    ) -> Result<RoleMailboxClaimLeaseV1, LeaseError> {
        let mut tx = self.pool.begin().await.map_err(|_| LeaseError::Conflict)?;
        // (1) Lock the thread row.
        let row: Option<(String, String, Value)> = sqlx::query_as(
            r#"SELECT lifecycle_state, claim_mode, executor_kind_allowlist
               FROM role_mailbox_thread WHERE thread_id = $1 FOR UPDATE"#,
        )
        .bind(thread_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| LeaseError::Conflict)?;
        let Some((lifecycle_str, claim_mode_str, allowlist_json)) = row else {
            return Err(LeaseError::NotFound);
        };
        let lifecycle: ThreadLifecycleState = serde_json::from_value(Value::String(lifecycle_str))
            .map_err(|_| LeaseError::Conflict)?;
        if lifecycle.is_terminal() {
            return Err(LeaseError::ThreadInTerminalState);
        }
        let claim_mode: ClaimMode = serde_json::from_value(Value::String(claim_mode_str))
            .map_err(|_| LeaseError::Conflict)?;
        let allowlist: Vec<ExecutorKind> =
            serde_json::from_value(allowlist_json).map_err(|_| LeaseError::Conflict)?;
        // (2) Allowlist gate.
        if !allowlist.contains(&request.executor_kind) {
            return Err(LeaseError::ExecutorKindNotAllowed);
        }
        let now = Utc::now();
        // (5) Sweep expired-but-unreleased leases so the partial unique
        // index admits a new INSERT.
        sqlx::query(
            r#"UPDATE role_mailbox_claim_lease
               SET released_at_utc = $1
               WHERE thread_id = $2
                 AND released_at_utc IS NULL
                 AND expires_at_utc <= $1"#,
        )
        .bind(now)
        .bind(thread_id.as_uuid())
        .execute(&mut *tx)
        .await
        .map_err(|_| LeaseError::Conflict)?;
        // (4) For exclusive/handoff claim modes, check for an active lease.
        // ClaimMode::Open permits multiple holders so we skip this check.
        if !matches!(claim_mode, ClaimMode::Open) {
            let active: Option<(Uuid,)> = sqlx::query_as(
                r#"SELECT holder_session_id FROM role_mailbox_claim_lease
                   WHERE thread_id = $1
                     AND released_at_utc IS NULL
                     AND expires_at_utc > $2
                   LIMIT 1"#,
            )
            .bind(thread_id.as_uuid())
            .bind(now)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| LeaseError::Conflict)?;
            if let Some((holder,)) = active {
                return Err(LeaseError::LeaseHeldByOther {
                    current_holder: holder,
                });
            }
        }
        // (6) INSERT the new lease. Partial unique index converts any
        // concurrent INSERT that slipped through application checks into
        // a 23505 unique_violation, which we surface as Conflict.
        let lease = RoleMailboxClaimLeaseV1 {
            lease_id: Uuid::now_v7(),
            thread_id: thread_id.as_uuid(),
            holder_executor_kind: request.executor_kind,
            holder_role_id: request.role_id.clone(),
            holder_session_id: request.session_id,
            acquired_at_utc: now,
            expires_at_utc: now + chrono::Duration::seconds(request.lease_duration_secs as i64),
            released_at_utc: None,
            takeover_of: None,
            takeover_reason: None,
        };
        let insert_res = sqlx::query(
            r#"INSERT INTO role_mailbox_claim_lease
               (lease_id, thread_id, holder_executor_kind, holder_role_id,
                holder_session_id, acquired_at_utc, expires_at_utc,
                released_at_utc, takeover_of, takeover_reason)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)"#,
        )
        .bind(lease.lease_id)
        .bind(lease.thread_id)
        .bind(executor_kind_str(lease.holder_executor_kind))
        .bind(lease.holder_role_id.to_string())
        .bind(lease.holder_session_id)
        .bind(lease.acquired_at_utc)
        .bind(lease.expires_at_utc)
        .bind(lease.released_at_utc)
        .bind(lease.takeover_of)
        .bind(lease.takeover_reason.as_deref())
        .execute(&mut *tx)
        .await;
        match insert_res {
            Ok(_) => {
                tx.commit().await.map_err(|_| LeaseError::Conflict)?;
                Ok(lease)
            }
            Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("23505") => {
                // Partial unique index fired — another acquirer raced us.
                Err(LeaseError::Conflict)
            }
            Err(_) => Err(LeaseError::Conflict),
        }
    }

    /// MT-180: Extend a lease by `extra_secs`. Idempotent in the sense that
    /// calling extend after a successful release returns `AlreadyReleased`
    /// and never silently extends a dead lease.
    ///
    /// Per `red_team.minimum_controls` #3: extension cannot bypass expiry.
    /// If the current `expires_at_utc <= now()` the lease is dead and the
    /// caller must `acquire` afresh.
    pub async fn extend_lease(
        &self,
        lease_id: Uuid,
        extra_secs: u32,
    ) -> Result<RoleMailboxClaimLeaseV1, LeaseError> {
        let mut tx = self.pool.begin().await.map_err(|_| LeaseError::Conflict)?;
        let row: Option<(
            Uuid,
            String,
            String,
            Uuid,
            DateTime<Utc>,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            Option<Uuid>,
            Option<String>,
        )> = sqlx::query_as(
            r#"SELECT thread_id, holder_executor_kind, holder_role_id, holder_session_id,
                       acquired_at_utc, expires_at_utc, released_at_utc, takeover_of, takeover_reason
               FROM role_mailbox_claim_lease WHERE lease_id = $1 FOR UPDATE"#,
        )
        .bind(lease_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| LeaseError::Conflict)?;
        let Some((
            thread_id,
            kind_str,
            role_str,
            session_id,
            acquired_at_utc,
            current_expires,
            released_at_utc,
            takeover_of,
            takeover_reason,
        )) = row
        else {
            return Err(LeaseError::NotFound);
        };
        if released_at_utc.is_some() {
            return Err(LeaseError::AlreadyReleased);
        }
        let now = Utc::now();
        if current_expires <= now {
            return Err(LeaseError::Expired);
        }
        let new_expires = current_expires + chrono::Duration::seconds(extra_secs as i64);
        sqlx::query(
            r#"UPDATE role_mailbox_claim_lease SET expires_at_utc = $1 WHERE lease_id = $2"#,
        )
        .bind(new_expires)
        .bind(lease_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| LeaseError::Conflict)?;
        tx.commit().await.map_err(|_| LeaseError::Conflict)?;
        let holder_executor_kind = parse_executor_kind(&kind_str).ok_or(LeaseError::Conflict)?;
        let holder_role_id = RoleId::parse(&role_str).map_err(|_| LeaseError::Conflict)?;
        Ok(RoleMailboxClaimLeaseV1 {
            lease_id,
            thread_id,
            holder_executor_kind,
            holder_role_id,
            holder_session_id: session_id,
            acquired_at_utc,
            expires_at_utc: new_expires,
            released_at_utc: None,
            takeover_of,
            takeover_reason,
        })
    }

    /// MT-180: Release a lease. Idempotent — releasing an already-released
    /// lease is a no-op that returns `Ok(())` (mirrors the in-process
    /// LeaseManager's contract and the spec's "release() ... is a no-op if
    /// already released").
    pub async fn release_lease(&self, lease_id: Uuid) -> Result<(), LeaseError> {
        let mut tx = self.pool.begin().await.map_err(|_| LeaseError::Conflict)?;
        let row: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
            r#"SELECT released_at_utc FROM role_mailbox_claim_lease
               WHERE lease_id = $1 FOR UPDATE"#,
        )
        .bind(lease_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| LeaseError::Conflict)?;
        let Some((released,)) = row else {
            return Err(LeaseError::NotFound);
        };
        if released.is_some() {
            // Idempotent no-op.
            tx.commit().await.map_err(|_| LeaseError::Conflict)?;
            return Ok(());
        }
        sqlx::query(
            r#"UPDATE role_mailbox_claim_lease SET released_at_utc = NOW() WHERE lease_id = $1"#,
        )
        .bind(lease_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| LeaseError::Conflict)?;
        tx.commit().await.map_err(|_| LeaseError::Conflict)?;
        Ok(())
    }

    /// MT-180: Take over a thread's lease per the thread's `takeover_policy`.
    /// Atomically force-releases the predecessor and writes a new lease row
    /// with `takeover_of` and `takeover_reason` populated.
    pub async fn takeover_lease(
        &self,
        thread_id: RoleMailboxThreadId,
        request: LeaseRequest,
        predecessor_lease_id: Uuid,
        reason: String,
    ) -> Result<RoleMailboxClaimLeaseV1, LeaseError> {
        let mut tx = self.pool.begin().await.map_err(|_| LeaseError::Conflict)?;
        // Lock the thread row to read its takeover_policy + allowlist.
        let trow: Option<(String, String, Value, String)> = sqlx::query_as(
            r#"SELECT lifecycle_state, claim_mode, executor_kind_allowlist, takeover_policy
               FROM role_mailbox_thread WHERE thread_id = $1 FOR UPDATE"#,
        )
        .bind(thread_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| LeaseError::Conflict)?;
        let Some((lifecycle_str, _claim_mode_str, allowlist_json, takeover_str)) = trow else {
            return Err(LeaseError::NotFound);
        };
        let lifecycle: ThreadLifecycleState = serde_json::from_value(Value::String(lifecycle_str))
            .map_err(|_| LeaseError::Conflict)?;
        if lifecycle.is_terminal() {
            return Err(LeaseError::ThreadInTerminalState);
        }
        let takeover_policy: TakeoverPolicy = serde_json::from_value(Value::String(takeover_str))
            .map_err(|_| LeaseError::Conflict)?;
        let allowlist: Vec<ExecutorKind> =
            serde_json::from_value(allowlist_json).map_err(|_| LeaseError::Conflict)?;
        if !allowlist.contains(&request.executor_kind) {
            return Err(LeaseError::ExecutorKindNotAllowed);
        }
        // Policy gate.
        if matches!(takeover_policy, TakeoverPolicy::Never) {
            return Err(LeaseError::TakeoverNotPermitted);
        }
        if matches!(takeover_policy, TakeoverPolicy::OperatorOnly)
            && request.executor_kind != ExecutorKind::Operator
        {
            return Err(LeaseError::TakeoverNotPermitted);
        }
        // Lock the predecessor row and apply OnLeaseExpiry gate.
        let prow: Option<(DateTime<Utc>, Option<DateTime<Utc>>)> = sqlx::query_as(
            r#"SELECT expires_at_utc, released_at_utc
               FROM role_mailbox_claim_lease WHERE lease_id = $1 AND thread_id = $2 FOR UPDATE"#,
        )
        .bind(predecessor_lease_id)
        .bind(thread_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| LeaseError::Conflict)?;
        let Some((pred_expires, _pred_released)) = prow else {
            return Err(LeaseError::NotFound);
        };
        let now = Utc::now();
        if matches!(takeover_policy, TakeoverPolicy::OnLeaseExpiry) && pred_expires > now {
            return Err(LeaseError::TakeoverNotPermitted);
        }
        // Force-release the predecessor (idempotent if already released).
        sqlx::query(
            r#"UPDATE role_mailbox_claim_lease
               SET released_at_utc = COALESCE(released_at_utc, $1)
               WHERE lease_id = $2"#,
        )
        .bind(now)
        .bind(predecessor_lease_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| LeaseError::Conflict)?;
        // INSERT the new lease with takeover provenance.
        let lease = RoleMailboxClaimLeaseV1 {
            lease_id: Uuid::now_v7(),
            thread_id: thread_id.as_uuid(),
            holder_executor_kind: request.executor_kind,
            holder_role_id: request.role_id.clone(),
            holder_session_id: request.session_id,
            acquired_at_utc: now,
            expires_at_utc: now + chrono::Duration::seconds(request.lease_duration_secs as i64),
            released_at_utc: None,
            takeover_of: Some(predecessor_lease_id),
            takeover_reason: Some(reason.clone()),
        };
        let res = sqlx::query(
            r#"INSERT INTO role_mailbox_claim_lease
               (lease_id, thread_id, holder_executor_kind, holder_role_id,
                holder_session_id, acquired_at_utc, expires_at_utc,
                released_at_utc, takeover_of, takeover_reason)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)"#,
        )
        .bind(lease.lease_id)
        .bind(lease.thread_id)
        .bind(executor_kind_str(lease.holder_executor_kind))
        .bind(lease.holder_role_id.to_string())
        .bind(lease.holder_session_id)
        .bind(lease.acquired_at_utc)
        .bind(lease.expires_at_utc)
        .bind(lease.released_at_utc)
        .bind(lease.takeover_of)
        .bind(lease.takeover_reason.as_deref())
        .execute(&mut *tx)
        .await;
        match res {
            Ok(_) => {
                tx.commit().await.map_err(|_| LeaseError::Conflict)?;
                Ok(lease)
            }
            Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("23505") => {
                Err(LeaseError::Conflict)
            }
            Err(_) => Err(LeaseError::Conflict),
        }
    }

    /// MT-180: Look up the currently-active lease for `thread_id`. Returns
    /// `None` if no unreleased unexpired lease exists. Useful for routing
    /// decisions (MT-181 ExecutorRouter).
    pub async fn get_active_lease_for_thread(
        &self,
        thread_id: RoleMailboxThreadId,
    ) -> Result<Option<RoleMailboxClaimLeaseV1>, LeaseError> {
        let row: Option<(
            Uuid,
            Uuid,
            String,
            String,
            Uuid,
            DateTime<Utc>,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            Option<Uuid>,
            Option<String>,
        )> = sqlx::query_as(
            r#"SELECT lease_id, thread_id, holder_executor_kind, holder_role_id,
                       holder_session_id, acquired_at_utc, expires_at_utc,
                       released_at_utc, takeover_of, takeover_reason
               FROM role_mailbox_claim_lease
               WHERE thread_id = $1
                 AND released_at_utc IS NULL
                 AND expires_at_utc > NOW()
               LIMIT 1"#,
        )
        .bind(thread_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| LeaseError::Conflict)?;
        let Some(r) = row else { return Ok(None) };
        let holder_executor_kind = parse_executor_kind(&r.2).ok_or(LeaseError::Conflict)?;
        let holder_role_id = RoleId::parse(&r.3).map_err(|_| LeaseError::Conflict)?;
        Ok(Some(RoleMailboxClaimLeaseV1 {
            lease_id: r.0,
            thread_id: r.1,
            holder_executor_kind,
            holder_role_id,
            holder_session_id: r.4,
            acquired_at_utc: r.5,
            expires_at_utc: r.6,
            released_at_utc: r.7,
            takeover_of: r.8,
            takeover_reason: r.9,
        }))
    }

    /// MT-180: Return the full ancestry chain of leases for `thread_id`
    /// ordered chronologically. Satisfies `red_team.minimum_controls` #2:
    /// the takeover audit chain must be queryable so an auditor can
    /// reconstruct the full ownership history.
    pub async fn list_lease_chain_for_thread(
        &self,
        thread_id: RoleMailboxThreadId,
    ) -> Result<Vec<RoleMailboxClaimLeaseV1>, LeaseError> {
        let rows: Vec<(
            Uuid,
            Uuid,
            String,
            String,
            Uuid,
            DateTime<Utc>,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            Option<Uuid>,
            Option<String>,
        )> = sqlx::query_as(
            r#"WITH RECURSIVE chain AS (
                 SELECT lease_id, thread_id, holder_executor_kind, holder_role_id,
                        holder_session_id, acquired_at_utc, expires_at_utc,
                        released_at_utc, takeover_of, takeover_reason
                   FROM role_mailbox_claim_lease
                   WHERE thread_id = $1 AND takeover_of IS NULL
                 UNION ALL
                 SELECT l.lease_id, l.thread_id, l.holder_executor_kind, l.holder_role_id,
                        l.holder_session_id, l.acquired_at_utc, l.expires_at_utc,
                        l.released_at_utc, l.takeover_of, l.takeover_reason
                   FROM role_mailbox_claim_lease l
                   JOIN chain c ON l.takeover_of = c.lease_id
                   WHERE l.thread_id = $1
               )
               SELECT lease_id, thread_id, holder_executor_kind, holder_role_id,
                      holder_session_id, acquired_at_utc, expires_at_utc,
                      released_at_utc, takeover_of, takeover_reason
               FROM chain
               ORDER BY acquired_at_utc ASC, lease_id ASC"#,
        )
        .bind(thread_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|_| LeaseError::Conflict)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let holder_executor_kind = parse_executor_kind(&r.2).ok_or(LeaseError::Conflict)?;
            let holder_role_id = RoleId::parse(&r.3).map_err(|_| LeaseError::Conflict)?;
            out.push(RoleMailboxClaimLeaseV1 {
                lease_id: r.0,
                thread_id: r.1,
                holder_executor_kind,
                holder_role_id,
                holder_session_id: r.4,
                acquired_at_utc: r.5,
                expires_at_utc: r.6,
                released_at_utc: r.7,
                takeover_of: r.8,
                takeover_reason: r.9,
            });
        }
        Ok(out)
    }

    // ------------------------------------------------------------------
    // MT-183 handoff bundle persistence
    // ------------------------------------------------------------------

    /// MT-183: Insert a `MailboxHandoffBundleV1` row. Recomputes the
    /// canonical-JSON content_hash and rejects the insert with
    /// `MailboxError::HashMismatch` if the caller-supplied hash does not
    /// match. This satisfies `red_team.minimum_controls`:
    ///   1. tampered-bundle insert returns a typed error rather than a soft
    ///      warning.
    ///   2. defense-in-depth — even if the application layer skips
    ///      `recompute_hash`, the repo re-runs it before persisting.
    pub async fn insert_handoff_bundle(
        &self,
        bundle: &MailboxHandoffBundleV1,
    ) -> Result<(), MailboxError> {
        let recomputed = bundle.recompute_hash();
        if recomputed != bundle.content_hash {
            return Err(MailboxError::HashMismatch {
                expected: recomputed,
                got: bundle.content_hash.clone(),
            });
        }
        let target_role_str = bundle.target_role.to_string();
        let executor_kind_str = executor_kind_str(bundle.target_executor_kind);
        let linked_artifacts_json = serde_json::to_value(&bundle.linked_artifacts)?;
        let transcript_json = match &bundle.transcript_pointer {
            Some(p) => Some(serde_json::to_value(p)?),
            None => None,
        };
        let capability_grants_json = serde_json::to_value(&bundle.capability_grants)?;
        sqlx::query(
            r#"INSERT INTO role_mailbox_handoff_bundle
                 (bundle_id, source_thread_id, source_message_id, target_role,
                  target_executor_kind, context_summary, linked_artifacts,
                  transcript_pointer, capability_grants, expires_at_utc,
                  content_hash, created_at_utc, created_by_session)
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)"#,
        )
        .bind(bundle.bundle_id)
        .bind(bundle.source_thread_id)
        .bind(bundle.source_message_id)
        .bind(&target_role_str)
        .bind(executor_kind_str)
        .bind(&bundle.context_summary)
        .bind(linked_artifacts_json)
        .bind(transcript_json)
        .bind(capability_grants_json)
        .bind(bundle.expires_at_utc)
        .bind(&recomputed)
        .bind(bundle.created_at_utc)
        .bind(bundle.created_by_session)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// MT-183: Fetch a stored handoff bundle by `bundle_id`. Returns `None`
    /// when no row matches. The returned bundle includes the
    /// stored-as-canonical `content_hash` so callers can re-verify if they
    /// want defence-in-depth.
    pub async fn get_handoff_bundle(
        &self,
        bundle_id: Uuid,
    ) -> Result<Option<MailboxHandoffBundleV1>, MailboxError> {
        let row: Option<(
            Uuid,
            Uuid,
            Uuid,
            String,
            String,
            String,
            Value,
            Option<Value>,
            Value,
            Option<DateTime<Utc>>,
            String,
            DateTime<Utc>,
            Uuid,
        )> = sqlx::query_as(
            r#"SELECT bundle_id, source_thread_id, source_message_id, target_role,
                       target_executor_kind, context_summary, linked_artifacts,
                       transcript_pointer, capability_grants, expires_at_utc,
                       content_hash, created_at_utc, created_by_session
               FROM role_mailbox_handoff_bundle
               WHERE bundle_id = $1"#,
        )
        .bind(bundle_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(r) = row else { return Ok(None) };
        let target_role =
            RoleId::parse(&r.3).map_err(|e| MailboxError::Parse(format!("target_role: {e}")))?;
        let target_executor_kind = parse_executor_kind(&r.4)
            .ok_or_else(|| MailboxError::Parse(format!("target_executor_kind: {}", r.4)))?;
        let linked_artifacts = serde_json::from_value(r.6)?;
        let transcript_pointer = match r.7 {
            Some(v) => Some(serde_json::from_value::<TranscriptPointer>(v)?),
            None => None,
        };
        let capability_grants = serde_json::from_value(r.8)?;
        Ok(Some(MailboxHandoffBundleV1 {
            bundle_id: r.0,
            source_thread_id: r.1,
            source_message_id: r.2,
            target_role,
            target_executor_kind,
            context_summary: r.5,
            linked_artifacts,
            transcript_pointer,
            capability_grants,
            expires_at_utc: r.9,
            content_hash: r.10,
            created_at_utc: r.11,
            created_by_session: r.12,
        }))
    }

    /// MT-183: List handoff bundles for a given thread, chronological by
    /// `created_at_utc`. Useful for auditing the handoff chain a thread
    /// produced.
    pub async fn list_handoff_bundles_for_thread(
        &self,
        thread_id: RoleMailboxThreadId,
    ) -> Result<Vec<MailboxHandoffBundleV1>, MailboxError> {
        let rows: Vec<(
            Uuid,
            Uuid,
            Uuid,
            String,
            String,
            String,
            Value,
            Option<Value>,
            Value,
            Option<DateTime<Utc>>,
            String,
            DateTime<Utc>,
            Uuid,
        )> = sqlx::query_as(
            r#"SELECT bundle_id, source_thread_id, source_message_id, target_role,
                       target_executor_kind, context_summary, linked_artifacts,
                       transcript_pointer, capability_grants, expires_at_utc,
                       content_hash, created_at_utc, created_by_session
               FROM role_mailbox_handoff_bundle
               WHERE source_thread_id = $1
               ORDER BY created_at_utc ASC, bundle_id ASC"#,
        )
        .bind(thread_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let target_role = RoleId::parse(&r.3)
                .map_err(|e| MailboxError::Parse(format!("target_role: {e}")))?;
            let target_executor_kind = parse_executor_kind(&r.4)
                .ok_or_else(|| MailboxError::Parse(format!("target_executor_kind: {}", r.4)))?;
            let linked_artifacts = serde_json::from_value(r.6)?;
            let transcript_pointer = match r.7 {
                Some(v) => Some(serde_json::from_value::<TranscriptPointer>(v)?),
                None => None,
            };
            let capability_grants = serde_json::from_value(r.8)?;
            out.push(MailboxHandoffBundleV1 {
                bundle_id: r.0,
                source_thread_id: r.1,
                source_message_id: r.2,
                target_role,
                target_executor_kind,
                context_summary: r.5,
                linked_artifacts,
                transcript_pointer,
                capability_grants,
                expires_at_utc: r.9,
                content_hash: r.10,
                created_at_utc: r.11,
                created_by_session: r.12,
            });
        }
        Ok(out)
    }
}

fn executor_kind_str(kind: ExecutorKind) -> &'static str {
    match kind {
        ExecutorKind::LocalSmallModel => "local_small_model",
        ExecutorKind::CloudModel => "cloud_model",
        ExecutorKind::Reviewer => "reviewer",
        ExecutorKind::Validator => "validator",
        ExecutorKind::Operator => "operator",
        ExecutorKind::WorkflowAutomation => "workflow_automation",
    }
}

fn parse_executor_kind(s: &str) -> Option<ExecutorKind> {
    match s {
        "local_small_model" => Some(ExecutorKind::LocalSmallModel),
        "cloud_model" => Some(ExecutorKind::CloudModel),
        "reviewer" => Some(ExecutorKind::Reviewer),
        "validator" => Some(ExecutorKind::Validator),
        "operator" => Some(ExecutorKind::Operator),
        "workflow_automation" => Some(ExecutorKind::WorkflowAutomation),
        _ => None,
    }
}

pub const SCHEMA_SQL: &str =
    include_str!("../../migrations/0022_role_mailbox_threads_messages.sql");
