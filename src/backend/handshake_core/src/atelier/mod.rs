//! Atelier/Lens domain (WP-KERNEL-005 CastKit Codex fold-in).
//!
//! Storage authority is PostgreSQL + EventLedger + ArtifactStore + CRDT only.
//! SQLite is FORBIDDEN in any form (runtime, tests, fixtures, cache, fallback);
//! see [`assert_postgres_url`] (MT-004) and the kernel `no_sqlite_tripwire`.
//!
//! Module boundaries (MT-003): `core` (character identity + append-only sheet
//! versions), `media` (DAM), with `intake`/`collections`/`search`/`exports`
//! folded in by later microtasks. Every mutation is intended to emit an
//! EventLedger / Flight Recorder event from the [`event_family`] set (MT-005).

use sqlx::postgres::{PgPool, PgPoolOptions};
use thiserror::Error;

pub mod annotation;
pub mod collections;
pub mod core;
pub mod exports;
pub mod intake;
pub mod media;
pub mod search;
pub mod settings;
pub mod sheet;

pub use self::core::{Character, NewCharacter};
pub use self::media::{MediaAsset, NewMediaAsset};
pub use self::sheet::{NewSheetVersion, SheetVersion};

/// Errors surfaced by the atelier domain.
#[derive(Debug, Error)]
pub enum AtelierError {
    #[error("atelier database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("atelier entity not found: {0}")]
    NotFound(String),
    #[error("forbidden storage backend: {0}")]
    ForbiddenStorage(String),
    #[error("atelier validation error: {0}")]
    Validation(String),
}

pub type AtelierResult<T> = Result<T, AtelierError>;

/// Atelier EventLedger / Flight Recorder event families (MT-005).
///
/// These are the canonical seams every Core/Data mutation must emit so the
/// operator surface, Locus, and replay can reconstruct atelier history.
pub mod event_family {
    pub const CHARACTER_CREATED: &str = "atelier.character.created";
    pub const SHEET_VERSION_APPENDED: &str = "atelier.sheet.version_appended";
    pub const MEDIA_ASSET_MATERIALIZED: &str = "atelier.media.asset_materialized";

    /// All known atelier event families (used by parity/coverage checks).
    pub const ALL: &[&str] = &[
        CHARACTER_CREATED,
        SHEET_VERSION_APPENDED,
        MEDIA_ASSET_MATERIALIZED,
    ];
}

/// Runtime rejection of forbidden CKC storage assumptions (MT-004).
///
/// SQLite is forbidden in any form; only `postgres://` / `postgresql://`
/// connection strings are accepted as atelier storage authority.
pub fn assert_postgres_url(url: &str) -> AtelierResult<()> {
    let normalized = url.trim().to_ascii_lowercase();
    let is_postgres =
        normalized.starts_with("postgres://") || normalized.starts_with("postgresql://");
    if is_postgres {
        return Ok(());
    }
    if normalized.starts_with("sqlite:")
        || normalized.ends_with(".sqlite")
        || normalized.ends_with(".sqlite3")
        || normalized.ends_with(".db")
    {
        return Err(AtelierError::ForbiddenStorage(
            "SQLite is forbidden in Handshake; atelier requires PostgreSQL".to_string(),
        ));
    }
    Err(AtelierError::ForbiddenStorage(
        "atelier requires a PostgreSQL DATABASE_URL (postgres:// or postgresql://)".to_string(),
    ))
}

/// PostgreSQL-backed atelier data store. Wraps a shared [`PgPool`].
#[derive(Clone)]
pub struct AtelierStore {
    pool: PgPool,
}

impl AtelierStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Connect to a PostgreSQL DATABASE_URL and build a store. Rejects SQLite
    /// and any non-PostgreSQL backend (MT-004).
    pub async fn connect(database_url: &str) -> AtelierResult<Self> {
        assert_postgres_url(database_url)?;
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Idempotent, concurrency-safe bootstrap of the atelier schema from the
    /// canonical migration files (0030 foundation + 0031 core-data). A
    /// transaction-scoped advisory lock serializes concurrent bootstrap so
    /// parallel governed sessions / swarm agents never race on CREATE TABLE
    /// (the IF NOT EXISTS race). The lock auto-releases on commit. Safe to call
    /// repeatedly and from many connections at once.
    pub async fn ensure_schema(&self) -> AtelierResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(7305441001::bigint)")
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(include_str!("../../migrations/0030_atelier_foundation.sql"))
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(include_str!("../../migrations/0031_atelier_core_data.sql"))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Append an atelier domain event to the event ledger table (MT-005).
    pub async fn record_event(
        &self,
        event_family: &str,
        aggregate_type: &str,
        aggregate_id: &str,
        payload: serde_json::Value,
    ) -> AtelierResult<()> {
        sqlx::query(
            r#"INSERT INTO atelier_event (event_family, aggregate_type, aggregate_id, payload)
               VALUES ($1, $2, $3, $4)"#,
        )
        .bind(event_family)
        .bind(aggregate_type)
        .bind(aggregate_id)
        .bind(payload)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Count events of a given family (used by tests / coverage proofs).
    pub async fn count_events(&self, event_family: &str) -> AtelierResult<i64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM atelier_event WHERE event_family = $1")
                .bind(event_family)
                .fetch_one(&self.pool)
                .await?;
        Ok(count)
    }
}

#[cfg(test)]
mod guard_tests {
    use super::*;

    #[test]
    fn rejects_sqlite_urls() {
        assert!(assert_postgres_url("sqlite://./x.db").is_err());
        assert!(assert_postgres_url("/var/lib/handshake.sqlite").is_err());
        assert!(assert_postgres_url("foo.db").is_err());
    }

    #[test]
    fn accepts_postgres_urls() {
        assert!(assert_postgres_url("postgres://postgres@127.0.0.1:5544/handshake").is_ok());
        assert!(assert_postgres_url("postgresql://u:p@host/db").is_ok());
    }
}
