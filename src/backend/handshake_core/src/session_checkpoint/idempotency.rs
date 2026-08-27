//! MT-194 Idempotent recovery semantics + event-deduplication on replay.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;
use surrealdb::types::{RecordId, SurrealValue};
use thiserror::Error;
use uuid::Uuid;

use crate::storage::surreal::{SurrealStorage, SurrealStorageError};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectKind {
    MailboxMessagePost,
    StoreWrite,
    FileSystemWrite,
    ProcessSpawn,
    LeaseAcquisition,
    StoreWriteTarget { table: String },
    FileSystemWriteTarget { path_key: String },
}

impl SideEffectKind {
    pub fn store_write_table(table: &str) -> Self {
        Self::StoreWriteTarget {
            table: table.to_string(),
        }
    }

    pub fn file_system_write_target_key(path_key: &str) -> Self {
        Self::FileSystemWriteTarget {
            path_key: path_key.to_string(),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MailboxMessagePost => "mailbox_message_post",
            Self::StoreWrite | Self::StoreWriteTarget { .. } => "store_write",
            Self::FileSystemWrite | Self::FileSystemWriteTarget { .. } => "file_system_write",
            Self::ProcessSpawn => "process_spawn",
            Self::LeaseAcquisition => "lease_acquisition",
        }
    }

    pub fn storage_key(&self) -> String {
        match self {
            Self::StoreWriteTarget { table } => target_storage_key(self.as_str(), "table", table),
            Self::FileSystemWriteTarget { path_key } => {
                target_storage_key(self.as_str(), "path_key", path_key)
            }
            _ => self.as_str().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey {
    pub session_id: Uuid,
    pub event_seq: i64,
    pub side_effect_kind: SideEffectKind,
}

impl IdempotencyKey {
    pub fn side_effect_storage_key(&self) -> String {
        self.side_effect_kind.storage_key()
    }

    /// The `kernel_idempotency_ledger` record id for this key. Every writer of
    /// that table must go through this so replayed side effects dedupe against
    /// the same record regardless of which code path claimed them.
    pub(crate) fn ledger_record_id(&self) -> String {
        ledger_record_id(
            self.session_id,
            self.event_seq,
            &self.side_effect_storage_key(),
        )
    }
}

fn target_storage_key(kind: &str, target_label: &str, target: &str) -> String {
    let target_bytes = target.as_bytes();
    format!(
        "{kind}|{target_label}:len={}:hex={}",
        target_bytes.len(),
        hex::encode(target_bytes)
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplyOutcome {
    Applied,
    AlreadyApplied,
    Failed { error: String },
}

#[derive(Debug, Error)]
pub enum IdempotencyLedgerError {
    #[error("storage error: {0}")]
    Storage(#[from] SurrealStorageError),
    #[error("idempotency side effect is still pending for {record_id}")]
    Pending { record_id: String },
    #[error("idempotency ledger state is invalid for {record_id}: {state}")]
    InvalidState { record_id: String, state: String },
    #[error("idempotency claim ownership was lost for {record_id}")]
    ClaimLost { record_id: String },
}

#[async_trait]
pub trait IdempotentApply {
    async fn try_apply_idempotent<F, Fut>(
        &self,
        key: IdempotencyKey,
        op: F,
    ) -> Result<ApplyOutcome, IdempotencyLedgerError>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<(), String>> + Send;
}

const IDEMPOTENCY_LEDGER_TABLE: &str = "kernel_idempotency_ledger";
const PENDING_KIND_PREFIX: &str = "pending|claim=";
const PENDING_CLAIM_LEASE_SECONDS: i64 = 30;
const PENDING_CLAIM_HEARTBEAT_SECONDS: u64 = 10;

/// One `kernel_idempotency_ledger` record. The record id is the composite
/// `(session_id, event_seq, side_effect_kind)` key, so single-application is
/// enforced by record-id uniqueness exactly where the previous PRIMARY KEY
/// enforced it.
#[derive(Debug, Clone, SurrealValue)]
struct IdempotencyLedgerRow {
    session_id: Uuid,
    event_seq: i64,
    side_effect_kind: String,
    applied_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, SurrealValue)]
struct ClaimBindings {
    record_id: RecordId,
    session_id: Uuid,
    event_seq: i64,
    applied_kind: String,
    pending_kind: String,
    stale_before: DateTime<Utc>,
}

#[derive(Debug, Clone, SurrealValue)]
struct ClaimTransitionBindings {
    record_id: RecordId,
    applied_kind: String,
    pending_kind: String,
}

const CLAIM_PENDING_ATOMIC: &str = r#"
RETURN {
    LET $existing = SELECT side_effect_kind, applied_at_utc FROM ONLY $record_id;
    IF $existing = NONE {
        CREATE $record_id CONTENT {
            session_id: $session_id,
            event_seq: $event_seq,
            side_effect_kind: $pending_kind,
            applied_at_utc: time::now()
        } RETURN NONE;
        RETURN 'claimed';
    };
    IF $existing.side_effect_kind = $applied_kind {
        RETURN 'applied';
    };
    IF !string::starts_with($existing.side_effect_kind, 'pending|claim=') {
        RETURN 'invalid';
    };
    IF $existing.applied_at_utc > $stale_before {
        RETURN 'pending';
    };
    UPDATE ONLY $record_id SET
        side_effect_kind = $pending_kind,
        applied_at_utc = time::now();
    RETURN 'claimed';
};
"#;

const MARK_APPLIED_ATOMIC: &str = r#"
RETURN {
    LET $current = SELECT VALUE side_effect_kind FROM ONLY $record_id;
    IF $current != $pending_kind {
        RETURN false;
    };
    UPDATE ONLY $record_id SET
        side_effect_kind = $applied_kind,
        applied_at_utc = time::now();
    RETURN true;
};
"#;

const RENEW_PENDING_ATOMIC: &str = r#"
RETURN {
    LET $current = SELECT VALUE side_effect_kind FROM ONLY $record_id;
    IF $current != $pending_kind {
        RETURN false;
    };
    UPDATE ONLY $record_id SET applied_at_utc = time::now();
    RETURN true;
};
"#;

const RELEASE_PENDING_ATOMIC: &str = r#"
RETURN {
    LET $current = SELECT VALUE side_effect_kind FROM ONLY $record_id;
    IF $current != $pending_kind {
        RETURN false;
    };
    DELETE ONLY $record_id;
    RETURN true;
};
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InMemoryClaimState {
    Pending,
    Applied,
}

fn ledger_record_id(session_id: Uuid, event_seq: i64, side_effect_storage_key: &str) -> String {
    format!("{session_id}:{event_seq}:{side_effect_storage_key}")
}

fn pending_storage_kind(applied_kind: &str, claim_token: Uuid) -> String {
    format!(
        "{PENDING_KIND_PREFIX}{claim_token}|kind_hex={}",
        hex::encode(applied_kind.as_bytes())
    )
}

/// Idempotency ledger backed by the Handshake-managed embedded SurrealDB
/// store. The `kernel_idempotency_ledger` record id enforces
/// single-application at the database level.
pub struct IdempotencyLedger {
    storage: Option<SurrealStorage>,
    // In-process fallback for tests that don't open an embedded store.
    in_memory: Mutex<HashMap<(Uuid, i64, String), InMemoryClaimState>>,
}

impl IdempotencyLedger {
    pub fn new(storage: SurrealStorage) -> Self {
        Self {
            storage: Some(storage),
            in_memory: Mutex::new(HashMap::new()),
        }
    }

    pub fn in_memory() -> Self {
        Self {
            storage: None,
            in_memory: Mutex::new(HashMap::new()),
        }
    }

    /// Try to apply a side effect. The closure runs only if the key has not
    /// been applied before; otherwise returns `AlreadyApplied`. On op failure,
    /// the claim row is released so a retry can succeed.
    pub async fn try_apply<F, Fut>(
        &self,
        key: IdempotencyKey,
        op: F,
    ) -> Result<ApplyOutcome, IdempotencyLedgerError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        match &self.storage {
            Some(storage) => self.try_apply_surreal(storage, key, op).await,
            None => self.try_apply_in_memory(key, op).await,
        }
    }

    /// The previous PostgreSQL form held the ledger INSERT open in a
    /// transaction while `op` ran and committed only afterwards. An arbitrary
    /// Rust side effect cannot be held inside an embedded-store transaction, so
    /// this implementation persists an explicit tokenized pending claim. Live
    /// claims fail as pending instead of pretending the effect was applied;
    /// stale claims left by a crash may be atomically reclaimed after the
    /// bounded lease. Only a successful `op` transitions the row to the
    /// canonical applied key. This restores crash/reopen at-least-once recovery
    /// without allowing two live appliers to run the same effect concurrently.
    async fn try_apply_surreal<F, Fut>(
        &self,
        storage: &SurrealStorage,
        key: IdempotencyKey,
        op: F,
    ) -> Result<ApplyOutcome, IdempotencyLedgerError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        let side_effect_storage_key = key.side_effect_storage_key();
        let record_id = ledger_record_id(key.session_id, key.event_seq, &side_effect_storage_key);
        let session_id = key.session_id;
        let event_seq = key.event_seq;
        let claim_token = Uuid::now_v7();
        let pending_kind = pending_storage_kind(&side_effect_storage_key, claim_token);
        let claim_record = RecordId::new(IDEMPOTENCY_LEDGER_TABLE, record_id.clone());
        let claim_outcome: Option<String> = storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_first::<String, _>(
                            CLAIM_PENDING_ATOMIC,
                            ClaimBindings {
                                record_id: claim_record,
                                session_id,
                                event_seq,
                                applied_kind: side_effect_storage_key,
                                pending_kind: pending_kind.clone(),
                                stale_before: Utc::now()
                                    - chrono::Duration::seconds(PENDING_CLAIM_LEASE_SECONDS),
                            },
                        )
                        .await
                })
            })
            .await?;
        match claim_outcome.as_deref() {
            Some("applied") => return Ok(ApplyOutcome::AlreadyApplied),
            Some("pending") => {
                return Err(IdempotencyLedgerError::Pending {
                    record_id: record_id.clone(),
                })
            }
            Some("claimed") => {}
            Some("invalid") => {
                return Err(IdempotencyLedgerError::InvalidState {
                    record_id: record_id.clone(),
                    state: "noncanonical side_effect_kind".to_owned(),
                })
            }
            other => {
                return Err(IdempotencyLedgerError::InvalidState {
                    record_id: record_id.clone(),
                    state: format!("claim outcome {other:?}"),
                })
            }
        }

        let op_result = {
            let mut operation = Box::pin(op());
            loop {
                tokio::select! {
                    result = &mut operation => break result,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(
                        PENDING_CLAIM_HEARTBEAT_SECONDS,
                    )) => {
                        let renew_record = RecordId::new(
                            IDEMPOTENCY_LEDGER_TABLE,
                            record_id.clone(),
                        );
                        let renew_applied_kind = key.side_effect_storage_key();
                        let renew_pending_kind =
                            pending_storage_kind(&renew_applied_kind, claim_token);
                        let renewed: Option<bool> = storage
                            .with_data_operation(move |database| {
                                Box::pin(async move {
                                    database
                                        .query_first::<bool, _>(
                                            RENEW_PENDING_ATOMIC,
                                            ClaimTransitionBindings {
                                                record_id: renew_record,
                                                applied_kind: renew_applied_kind,
                                                pending_kind: renew_pending_kind,
                                            },
                                        )
                                        .await
                                })
                            })
                            .await?;
                        if renewed != Some(true) {
                            return Err(IdempotencyLedgerError::ClaimLost {
                                record_id: record_id.clone(),
                            });
                        }
                    }
                }
            }
        };

        match op_result {
            Ok(()) => {
                let finalize_record = RecordId::new(IDEMPOTENCY_LEDGER_TABLE, record_id.clone());
                let applied_kind = key.side_effect_storage_key();
                let finalize_pending_kind = pending_storage_kind(&applied_kind, claim_token);
                let finalized: Option<bool> = storage
                    .with_data_operation(move |database| {
                        Box::pin(async move {
                            database
                                .query_first::<bool, _>(
                                    MARK_APPLIED_ATOMIC,
                                    ClaimTransitionBindings {
                                        record_id: finalize_record,
                                        applied_kind,
                                        pending_kind: finalize_pending_kind,
                                    },
                                )
                                .await
                        })
                    })
                    .await?;
                if finalized == Some(true) {
                    Ok(ApplyOutcome::Applied)
                } else {
                    Err(IdempotencyLedgerError::ClaimLost { record_id })
                }
            }
            Err(e) => {
                // Release only this invocation's pending token. If release
                // fails, the pending lease remains visible and recoverable;
                // it is never misreported as AlreadyApplied.
                let applied_kind = key.side_effect_storage_key();
                let release_pending_kind = pending_storage_kind(&applied_kind, claim_token);
                let release_record = RecordId::new(IDEMPOTENCY_LEDGER_TABLE, record_id.clone());
                let release = storage
                    .with_data_operation(move |database| {
                        Box::pin(async move {
                            database
                                .query_first::<bool, _>(
                                    RELEASE_PENDING_ATOMIC,
                                    ClaimTransitionBindings {
                                        record_id: release_record,
                                        applied_kind,
                                        pending_kind: release_pending_kind,
                                    },
                                )
                                .await
                        })
                    })
                    .await;
                if let Err(release_error) = release {
                    tracing::error!(
                        target: "session_checkpoint_idempotency",
                        error = %release_error,
                        "failed to release pending idempotency claim after op failure; retry remains recoverable after the lease"
                    );
                    return Ok(ApplyOutcome::Failed {
                        error: format!("{e}; pending claim release failed: {release_error}"),
                    });
                }
                Ok(ApplyOutcome::Failed { error: e })
            }
        }
    }

    async fn try_apply_in_memory<F, Fut>(
        &self,
        key: IdempotencyKey,
        op: F,
    ) -> Result<ApplyOutcome, IdempotencyLedgerError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        let side_effect_storage_key = key.side_effect_storage_key();
        let map_key = (
            key.session_id,
            key.event_seq,
            side_effect_storage_key.clone(),
        );
        let existing = {
            let mut buf = self.in_memory.lock().unwrap();
            match buf.entry(map_key.clone()) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(InMemoryClaimState::Pending);
                    None
                }
                std::collections::hash_map::Entry::Occupied(entry) => Some(*entry.get()),
            }
        };
        match existing {
            Some(InMemoryClaimState::Applied) => return Ok(ApplyOutcome::AlreadyApplied),
            Some(InMemoryClaimState::Pending) => {
                return Err(IdempotencyLedgerError::Pending {
                    record_id: key.ledger_record_id(),
                });
            }
            None => {}
        }
        match op().await {
            Ok(()) => {
                self.in_memory
                    .lock()
                    .unwrap()
                    .insert(map_key, InMemoryClaimState::Applied);
                Ok(ApplyOutcome::Applied)
            }
            Err(e) => {
                // Rollback the in-memory insert so retry can succeed.
                let mut buf = self.in_memory.lock().unwrap();
                buf.remove(&map_key);
                Ok(ApplyOutcome::Failed { error: e })
            }
        }
    }
}

#[async_trait]
impl IdempotentApply for IdempotencyLedger {
    async fn try_apply_idempotent<F, Fut>(
        &self,
        key: IdempotencyKey,
        op: F,
    ) -> Result<ApplyOutcome, IdempotencyLedgerError>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<(), String>> + Send,
    {
        self.try_apply(key, op).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::surreal::{bootstrap_schema, SurrealStorageConfig};
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };

    async fn open(path: &std::path::Path) -> SurrealStorage {
        let storage = SurrealStorage::open(
            SurrealStorageConfig::with_path(path).expect("valid idempotency test path"),
        )
        .await
        .expect("open embedded idempotency store");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap idempotency schema");
        storage
    }

    #[tokio::test]
    async fn first_apply_succeeds_second_returns_already_applied() {
        let ledger = IdempotencyLedger::in_memory();
        let session = Uuid::now_v7();
        let key = IdempotencyKey {
            session_id: session,
            event_seq: 1,
            side_effect_kind: SideEffectKind::MailboxMessagePost,
        };
        let r1 = ledger
            .try_apply(key.clone(), || async { Ok(()) })
            .await
            .unwrap();
        let r2 = ledger.try_apply(key, || async { Ok(()) }).await.unwrap();
        assert_eq!(r1, ApplyOutcome::Applied);
        assert_eq!(r2, ApplyOutcome::AlreadyApplied);
    }

    #[tokio::test]
    async fn op_failure_rolls_back_so_retry_can_succeed() {
        let ledger = IdempotencyLedger::in_memory();
        let session = Uuid::now_v7();
        let key = IdempotencyKey {
            session_id: session,
            event_seq: 1,
            side_effect_kind: SideEffectKind::StoreWrite,
        };
        let r1 = ledger
            .try_apply(key.clone(), || async { Err("transient".to_string()) })
            .await
            .unwrap();
        assert!(matches!(r1, ApplyOutcome::Failed { .. }));
        let r2 = ledger.try_apply(key, || async { Ok(()) }).await.unwrap();
        assert_eq!(r2, ApplyOutcome::Applied);
    }

    #[tokio::test]
    async fn concurrent_in_memory_claim_is_pending_not_already_applied() {
        let ledger = Arc::new(IdempotencyLedger::in_memory());
        let key = IdempotencyKey {
            session_id: Uuid::now_v7(),
            event_seq: 2,
            side_effect_kind: SideEffectKind::MailboxMessagePost,
        };
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let first_ledger = Arc::clone(&ledger);
        let first_key = key.clone();
        let first = tokio::spawn(async move {
            first_ledger
                .try_apply(first_key, || async move {
                    let _ = entered_tx.send(());
                    let _ = release_rx.await;
                    Ok(())
                })
                .await
        });
        entered_rx.await.expect("first operation entered");

        let second_ran = Arc::new(AtomicBool::new(false));
        let second_ran_for_op = Arc::clone(&second_ran);
        let second = ledger
            .try_apply(key.clone(), move || async move {
                second_ran_for_op.store(true, Ordering::SeqCst);
                Ok(())
            })
            .await;
        assert!(matches!(
            second,
            Err(IdempotencyLedgerError::Pending { .. })
        ));
        assert!(!second_ran.load(Ordering::SeqCst));

        release_tx.send(()).expect("release first operation");
        assert_eq!(first.await.unwrap().unwrap(), ApplyOutcome::Applied);
        assert_eq!(
            ledger.try_apply(key, || async { Ok(()) }).await.unwrap(),
            ApplyOutcome::AlreadyApplied
        );
    }

    #[tokio::test]
    async fn stale_surreal_pending_claim_is_recovered_after_reopen() {
        let directory = tempfile::tempdir().expect("temporary idempotency root");
        let path = directory.path().join("store");
        let storage = open(&path).await;
        let key = IdempotencyKey {
            session_id: Uuid::now_v7(),
            event_seq: 3,
            side_effect_kind: SideEffectKind::store_write_table("checkpoint-recovery"),
        };
        let applied_kind = key.side_effect_storage_key();
        let record_id = key.ledger_record_id();
        let pending_kind = pending_storage_kind(&applied_kind, Uuid::now_v7());
        let stale_row = IdempotencyLedgerRow {
            session_id: key.session_id,
            event_seq: key.event_seq,
            side_effect_kind: pending_kind,
            applied_at_utc: Utc::now() - chrono::Duration::seconds(PENDING_CLAIM_LEASE_SECONDS + 1),
        };
        storage
            .with_data_operation({
                let record_id = record_id.clone();
                move |database| {
                    Box::pin(async move {
                        database
                            .upsert_one::<IdempotencyLedgerRow, _>(
                                IDEMPOTENCY_LEDGER_TABLE,
                                &record_id,
                                stale_row,
                            )
                            .await
                            .map(|_| ())
                    })
                }
            })
            .await
            .expect("seed stale pending claim");
        storage.shutdown().await.expect("close first store");
        drop(storage);

        let reopened = open(&path).await;
        let ledger = IdempotencyLedger::new(reopened.clone());
        let runs = Arc::new(AtomicUsize::new(0));
        let runs_for_op = Arc::clone(&runs);
        assert_eq!(
            ledger
                .try_apply(key.clone(), move || async move {
                    runs_for_op.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
                .await
                .expect("stale pending claim is reclaimable"),
            ApplyOutcome::Applied
        );
        assert_eq!(runs.load(Ordering::SeqCst), 1);
        assert_eq!(
            ledger.try_apply(key, || async { Ok(()) }).await.unwrap(),
            ApplyOutcome::AlreadyApplied
        );
        drop(ledger);
        reopened.shutdown().await.expect("close reopened store");
    }
}
