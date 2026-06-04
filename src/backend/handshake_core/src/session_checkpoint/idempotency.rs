//! MT-194 Idempotent recovery semantics + event-deduplication on replay.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashSet;
use std::future::Future;
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectKind {
    MailboxMessagePost,
    PostgresWrite,
    FileSystemWrite,
    ProcessSpawn,
    LeaseAcquisition,
    PostgresWriteTarget { table: String },
    FileSystemWriteTarget { path_key: String },
}

impl SideEffectKind {
    pub fn postgres_write_table(table: &str) -> Self {
        Self::PostgresWriteTarget {
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
            Self::PostgresWrite | Self::PostgresWriteTarget { .. } => "postgres_write",
            Self::FileSystemWrite | Self::FileSystemWriteTarget { .. } => "file_system_write",
            Self::ProcessSpawn => "process_spawn",
            Self::LeaseAcquisition => "lease_acquisition",
        }
    }

    pub fn storage_key(&self) -> String {
        match self {
            Self::PostgresWriteTarget { table } => {
                target_storage_key(self.as_str(), "table", table)
            }
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
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
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

/// Postgres-backed idempotency ledger. The
/// `kernel_idempotency_ledger` table primary key enforces single-application
/// at the database level.
pub struct IdempotencyLedger {
    pool: Option<PgPool>,
    // In-process fallback for tests that don't bind Postgres.
    in_memory: Mutex<HashSet<(Uuid, i64, String)>>,
}

impl IdempotencyLedger {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: Some(pool),
            in_memory: Mutex::new(HashSet::new()),
        }
    }

    pub fn in_memory() -> Self {
        Self {
            pool: None,
            in_memory: Mutex::new(HashSet::new()),
        }
    }

    /// Try to apply a side effect. The closure runs only if the key has not
    /// been applied before; otherwise returns `AlreadyApplied`. On op failure,
    /// the row is rolled back so a retry can succeed.
    pub async fn try_apply<F, Fut>(
        &self,
        key: IdempotencyKey,
        op: F,
    ) -> Result<ApplyOutcome, IdempotencyLedgerError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        match &self.pool {
            Some(pool) => self.try_apply_postgres(pool, key, op).await,
            None => Ok(self.try_apply_in_memory(key, op).await),
        }
    }

    async fn try_apply_postgres<F, Fut>(
        &self,
        pool: &PgPool,
        key: IdempotencyKey,
        op: F,
    ) -> Result<ApplyOutcome, IdempotencyLedgerError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        let side_effect_storage_key = key.side_effect_storage_key();
        let mut tx = pool.begin().await?;
        let res = sqlx::query(
            r#"INSERT INTO kernel_idempotency_ledger
                (session_id, event_seq, side_effect_kind)
               VALUES ($1, $2, $3)
               ON CONFLICT DO NOTHING"#,
        )
        .bind(key.session_id)
        .bind(key.event_seq)
        .bind(side_effect_storage_key)
        .execute(&mut *tx)
        .await?;
        if res.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(ApplyOutcome::AlreadyApplied);
        }
        let op_result = op().await;
        match op_result {
            Ok(()) => {
                tx.commit().await?;
                Ok(ApplyOutcome::Applied)
            }
            Err(e) => {
                tx.rollback().await?;
                Ok(ApplyOutcome::Failed { error: e })
            }
        }
    }

    async fn try_apply_in_memory<F, Fut>(&self, key: IdempotencyKey, op: F) -> ApplyOutcome
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        let side_effect_storage_key = key.side_effect_storage_key();
        let inserted = {
            let mut buf = self.in_memory.lock().unwrap();
            buf.insert((
                key.session_id,
                key.event_seq,
                side_effect_storage_key.clone(),
            ))
        };
        if !inserted {
            return ApplyOutcome::AlreadyApplied;
        }
        match op().await {
            Ok(()) => ApplyOutcome::Applied,
            Err(e) => {
                // Rollback the in-memory insert so retry can succeed.
                let mut buf = self.in_memory.lock().unwrap();
                buf.remove(&(key.session_id, key.event_seq, side_effect_storage_key));
                ApplyOutcome::Failed { error: e }
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
            side_effect_kind: SideEffectKind::PostgresWrite,
        };
        let r1 = ledger
            .try_apply(key.clone(), || async { Err("transient".to_string()) })
            .await
            .unwrap();
        assert!(matches!(r1, ApplyOutcome::Failed { .. }));
        let r2 = ledger.try_apply(key, || async { Ok(()) }).await.unwrap();
        assert_eq!(r2, ApplyOutcome::Applied);
    }
}
