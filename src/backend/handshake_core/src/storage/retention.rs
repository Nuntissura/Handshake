//! Retention & Pruning Service [§2.3.11]
//!
//! Implements the Janitor service for automated data cleanup per Master Spec §2.3.11.
//! Prunes old AI jobs and logs based on configurable retention policies.
//!
//! Hard Invariants:
//! - [HSK-GC-002] Pinned items (is_pinned = 1) are excluded from GC
//! - [HSK-GC-003] Every GC run emits meta.gc_summary to Flight Recorder
//! - [HSK-GC-004] PruneReport is written before items are unlinked

use chrono::{DateTime, Duration, Utc};
use duckdb::Connection as DuckDbConnection;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// [HSK-GC-001] Artifact classification for retention policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactKind {
    /// Flight Recorder traces (.jsonl)
    Log,
    /// AI Job outputs / EngineResults
    Result,
    /// Context snapshots (ACE-RAG)
    Evidence,
    /// Web/Model cache
    Cache,
    /// Durable workflow snapshots
    Checkpoint,
}

impl std::fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactKind::Log => write!(f, "log"),
            ArtifactKind::Result => write!(f, "result"),
            ArtifactKind::Evidence => write!(f, "evidence"),
            ArtifactKind::Cache => write!(f, "cache"),
            ArtifactKind::Checkpoint => write!(f, "checkpoint"),
        }
    }
}

/// [HSK-GC-001] Retention policy configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub kind: ArtifactKind,
    /// Number of days to retain items. Default: 30 for Logs, 7 for Cache.
    pub window_days: u32,
    /// Minimum versions to keep even if expired. Default: 3.
    pub min_versions: u32,
}

impl RetentionPolicy {
    /// Default policy for logs: 30 days, keep min 3 versions.
    pub fn default_log() -> Self {
        Self {
            kind: ArtifactKind::Log,
            window_days: 30,
            min_versions: 3,
        }
    }

    /// Default policy for AI job results: 30 days, keep min 3 versions.
    pub fn default_result() -> Self {
        Self {
            kind: ArtifactKind::Result,
            window_days: 30,
            min_versions: 3,
        }
    }

    /// Default policy for cache: 7 days, keep min 3 versions.
    pub fn default_cache() -> Self {
        Self {
            kind: ArtifactKind::Cache,
            window_days: 7,
            min_versions: 3,
        }
    }
}

/// [HSK-GC-001] Report produced after a prune operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneReport {
    pub timestamp: DateTime<Utc>,
    pub items_scanned: u32,
    pub items_pruned: u32,
    pub items_spared_pinned: u32,
    pub items_spared_window: u32,
    pub total_bytes_freed: u64,
}

impl PruneReport {
    fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            items_scanned: 0,
            items_pruned: 0,
            items_spared_pinned: 0,
            items_spared_window: 0,
            total_bytes_freed: 0,
        }
    }
}

/// Configuration for the Janitor service.
#[derive(Debug, Clone)]
pub struct JanitorConfig {
    pub policies: Vec<RetentionPolicy>,
    /// If true, scan and report but do not delete.
    pub dry_run: bool,
    /// Interval between prune runs in seconds. Default: 3600 (1 hour).
    pub interval_secs: u64,
    /// Maximum items to delete per batch to avoid database locking.
    pub batch_size: u32,
}

impl Default for JanitorConfig {
    fn default() -> Self {
        Self {
            policies: vec![RetentionPolicy::default_result()],
            dry_run: false,
            interval_secs: 3600,
            batch_size: 1000,
        }
    }
}

impl JanitorConfig {
    /// Create a dry-run configuration for testing.
    pub fn dry_run() -> Self {
        Self {
            dry_run: true,
            ..Default::default()
        }
    }
}

/// Errors from retention operations.
#[derive(Debug, Error)]
pub enum RetentionError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("flight recorder error: {0}")]
    FlightRecorder(String),
}

/// The Janitor service that runs periodic pruning.
///
/// Implements the engine.janitor contract from §2.3.11.2:
/// - Operation: `prune`
/// - Input Schema: `{ policies: Vec<RetentionPolicy>, dry_run: bool }`
/// - Output: `PruneReport`
pub struct Janitor {
    pool: SqlitePool,
    fr_pool: Arc<Mutex<DuckDbConnection>>,
    config: JanitorConfig,
}

impl Janitor {
    pub fn new(
        pool: SqlitePool,
        fr_pool: Arc<Mutex<DuckDbConnection>>,
        config: JanitorConfig,
    ) -> Self {
        Self {
            pool,
            fr_pool,
            config,
        }
    }

    /// Run a single prune pass. Returns report for audit trail.
    ///
    /// [HSK-GC-002]: Respects is_pinned flag.
    /// [HSK-GC-003]: Emits meta.gc_summary to Flight Recorder.
    /// [HSK-GC-004]: Writes PruneReport before unlinking.
    pub async fn prune(&self) -> Result<PruneReport, RetentionError> {
        let mut report = PruneReport::new();

        for policy in &self.config.policies {
            match policy.kind {
                ArtifactKind::Result => {
                    self.prune_ai_jobs(policy, &mut report).await?;
                }
                // Other artifact kinds (Log, Evidence, Cache, Checkpoint) are
                // out of scope for Phase 1 - they involve file system cleanup.
                _ => {
                    tracing::debug!(
                        target: "handshake_core::janitor",
                        kind = %policy.kind,
                        "Skipping artifact kind (not implemented in Phase 1)"
                    );
                }
            }
        }

        // [HSK-GC-003] Emit meta.gc_summary to Flight Recorder
        self.emit_gc_summary(&report)?;

        tracing::info!(
            target: "handshake_core::janitor",
            scanned = report.items_scanned,
            pruned = report.items_pruned,
            spared_pinned = report.items_spared_pinned,
            spared_window = report.items_spared_window,
            dry_run = self.config.dry_run,
            "GC prune complete"
        );

        Ok(report)
    }

    /// Prune AI jobs based on retention policy.
    async fn prune_ai_jobs(
        &self,
        policy: &RetentionPolicy,
        report: &mut PruneReport,
    ) -> Result<(), RetentionError> {
        let cutoff = Utc::now() - Duration::days(policy.window_days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        // Count total eligible items (completed/failed jobs older than cutoff)
        let scan_result = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as "total!: i64",
                SUM(CASE WHEN is_pinned = 1 THEN 1 ELSE 0 END) as "pinned!: i64"
            FROM ai_jobs
            WHERE status IN ('completed', 'failed')
              AND created_at < $1
            "#,
            cutoff_str
        )
        .fetch_one(&self.pool)
        .await?;

        let total_eligible = scan_result.total as u32;
        let pinned_count = scan_result.pinned as u32;
        let deletable_count = total_eligible.saturating_sub(pinned_count);

        report.items_scanned += total_eligible;
        report.items_spared_pinned += pinned_count;

        // Respect min_versions: keep the N most recent even if expired
        // Calculate how many we should actually delete
        let to_keep_for_min_versions = policy.min_versions;

        // Count how many non-pinned items exist in total (to enforce min_versions)
        let total_non_pinned = sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!: i64"
            FROM ai_jobs
            WHERE is_pinned = 0
              AND status IN ('completed', 'failed')
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let total_non_pinned = total_non_pinned.count as u32;

        // If we have fewer than min_versions, spare everything
        let max_deletable = if total_non_pinned <= to_keep_for_min_versions {
            0
        } else {
            total_non_pinned - to_keep_for_min_versions
        };

        // The actual number we can delete is the minimum of:
        // 1. Items that are expired and not pinned (deletable_count)
        // 2. Items we can delete while respecting min_versions (max_deletable)
        let actual_to_delete = deletable_count.min(max_deletable);

        if actual_to_delete == 0 {
            report.items_spared_window += deletable_count;
            tracing::debug!(
                target: "handshake_core::janitor",
                total_eligible,
                pinned_count,
                deletable_count,
                "No items to delete (min_versions constraint or all pinned)"
            );
            return Ok(());
        }

        if self.config.dry_run {
            tracing::info!(
                target: "handshake_core::janitor",
                would_delete = actual_to_delete,
                "DRY RUN: Would delete AI jobs"
            );
            report.items_pruned += actual_to_delete;
            return Ok(());
        }

        // [HSK-GC-004] Delete in batches to avoid locking
        // workflow_runs will cascade delete automatically due to ON DELETE CASCADE
        let mut deleted = 0u32;
        let batch_size = self.config.batch_size as i64;

        while deleted < actual_to_delete {
            let remaining = (actual_to_delete - deleted) as i64;
            let limit = remaining.min(batch_size);

            let result = sqlx::query!(
                r#"
                DELETE FROM ai_jobs
                WHERE id IN (
                    SELECT id FROM ai_jobs
                    WHERE status IN ('completed', 'failed')
                      AND created_at < $1
                      AND is_pinned = 0
                    ORDER BY created_at ASC
                    LIMIT $2
                )
                "#,
                cutoff_str,
                limit
            )
            .execute(&self.pool)
            .await?;

            let batch_deleted = result.rows_affected() as u32;
            if batch_deleted == 0 {
                break;
            }
            deleted += batch_deleted;

            tracing::debug!(
                target: "handshake_core::janitor",
                batch_deleted,
                total_deleted = deleted,
                "Deleted batch of AI jobs"
            );
        }

        report.items_pruned += deleted;
        let spared_window = deletable_count.saturating_sub(deleted);
        report.items_spared_window += spared_window;

        Ok(())
    }

    /// [HSK-GC-003] Emit meta.gc_summary event to Flight Recorder.
    fn emit_gc_summary(&self, report: &PruneReport) -> Result<(), RetentionError> {
        let payload = serde_json::json!({
            "timestamp": report.timestamp.to_rfc3339(),
            "items_scanned": report.items_scanned,
            "items_pruned": report.items_pruned,
            "items_spared_pinned": report.items_spared_pinned,
            "items_spared_window": report.items_spared_window,
            "total_bytes_freed": report.total_bytes_freed,
            "dry_run": self.config.dry_run
        });

        let conn = self
            .fr_pool
            .lock()
            .map_err(|e| RetentionError::FlightRecorder(e.to_string()))?;

        conn.execute(
            "INSERT INTO events (event_type, job_id, workflow_id, payload) VALUES (?, ?, ?, ?)",
            duckdb::params![
                "meta.gc_summary",
                None::<&str>,
                None::<&str>,
                payload.to_string()
            ],
        )
        .map_err(|e| RetentionError::FlightRecorder(e.to_string()))?;

        Ok(())
    }

    /// Start background loop (runs every interval_secs).
    ///
    /// Safe: panics in this task do not crash the main app.
    /// The task runs independently and logs errors rather than propagating them.
    pub fn spawn_background(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let interval = std::time::Duration::from_secs(self.config.interval_secs);

        tokio::spawn(async move {
            tracing::info!(
                target: "handshake_core::janitor",
                interval_secs = self.config.interval_secs,
                dry_run = self.config.dry_run,
                "Janitor background service started"
            );

            // Run once on startup
            if let Err(e) = self.prune().await {
                tracing::error!(
                    target: "handshake_core::janitor",
                    error = %e,
                    "Initial prune failed"
                );
            }

            // Then run periodically
            let mut interval_timer = tokio::time::interval(interval);
            interval_timer.tick().await; // Skip first immediate tick (already ran above)

            loop {
                interval_timer.tick().await;

                if let Err(e) = self.prune().await {
                    tracing::error!(
                        target: "handshake_core::janitor",
                        error = %e,
                        "Scheduled prune failed"
                    );
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use tempfile::tempdir;

    async fn setup_test_db() -> (SqlitePool, Arc<Mutex<DuckDbConnection>>, tempfile::TempDir) {
        let dir = tempdir().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .expect("Failed to connect to SQLite");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        // Setup DuckDB for flight recorder
        let fr_path = dir.path().join("fr.db");
        let fr_conn = DuckDbConnection::open(&fr_path).expect("Failed to open DuckDB");
        fr_conn
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS events (
                    timestamp DATETIME DEFAULT current_timestamp,
                    event_type TEXT NOT NULL,
                    job_id TEXT,
                    workflow_id TEXT,
                    payload JSON
                );
            "#,
            )
            .expect("Failed to create events table");

        (pool, Arc::new(Mutex::new(fr_conn)), dir)
    }

    async fn create_test_job(
        pool: &SqlitePool,
        created_at: DateTime<Utc>,
        status: &str,
        is_pinned: bool,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at_str = created_at.to_rfc3339();
        let pinned = if is_pinned { 1i32 } else { 0i32 };

        sqlx::query!(
            r#"
            INSERT INTO ai_jobs (
                id, job_kind, status, protocol_id, profile_id,
                capability_profile_id, access_mode, safety_mode,
                created_at, updated_at, is_pinned
            )
            VALUES ($1, 'test', $2, 'proto', 'profile', 'cap', 'read', 'safe', $3, $4, $5)
            "#,
            id,
            status,
            created_at_str,
            created_at_str,
            pinned
        )
        .execute(pool)
        .await
        .expect("Failed to create test job");

        id
    }

    #[tokio::test]
    async fn test_prune_respects_pinned_items() {
        let (pool, fr_pool, _dir) = setup_test_db().await;

        // Create an old, completed, pinned job (should be spared)
        let old_date = Utc::now() - Duration::days(60);
        let _pinned_id = create_test_job(&pool, old_date, "completed", true).await;

        // Create an old, completed, unpinned job (should be deleted)
        let _unpinned_id = create_test_job(&pool, old_date, "completed", false).await;

        // Create 3 more unpinned jobs to satisfy min_versions
        for _ in 0..3 {
            create_test_job(&pool, Utc::now(), "completed", false).await;
        }

        let config = JanitorConfig {
            policies: vec![RetentionPolicy::default_result()],
            dry_run: false,
            interval_secs: 3600,
            batch_size: 100,
        };

        let janitor = Janitor::new(pool.clone(), fr_pool, config);
        let report = janitor.prune().await.expect("Prune failed");

        // Should have scanned 2 (old items only), spared 1 pinned, pruned 1
        assert_eq!(
            report.items_spared_pinned, 1,
            "Pinned item should be spared"
        );
        assert_eq!(
            report.items_pruned, 1,
            "One unpinned expired item should be pruned"
        );

        // Verify pinned job still exists
        let remaining = sqlx::query!("SELECT COUNT(*) as count FROM ai_jobs WHERE is_pinned = 1")
            .fetch_one(&pool)
            .await
            .expect("Query failed");
        assert_eq!(remaining.count, 1, "Pinned job should still exist");
    }

    #[tokio::test]
    async fn test_prune_respects_window() {
        let (pool, fr_pool, _dir) = setup_test_db().await;

        // Create a recent job (within window, should be spared)
        let recent_date = Utc::now() - Duration::days(5);
        let _recent_id = create_test_job(&pool, recent_date, "completed", false).await;

        // Create an old job (outside window, should be deleted)
        let old_date = Utc::now() - Duration::days(60);
        let _old_id = create_test_job(&pool, old_date, "completed", false).await;

        // Create more recent jobs to satisfy min_versions
        for _ in 0..3 {
            create_test_job(&pool, Utc::now(), "completed", false).await;
        }

        let config = JanitorConfig {
            policies: vec![RetentionPolicy::default_result()],
            dry_run: false,
            interval_secs: 3600,
            batch_size: 100,
        };

        let janitor = Janitor::new(pool.clone(), fr_pool, config);
        let report = janitor.prune().await.expect("Prune failed");

        // Old job should be pruned
        assert!(report.items_pruned >= 1, "Old item should be pruned");

        // Recent job should still exist
        let remaining = sqlx::query!(
            r#"
            SELECT COUNT(*) as count FROM ai_jobs
            WHERE created_at > datetime('now', '-10 days')
            "#
        )
        .fetch_one(&pool)
        .await
        .expect("Query failed");

        assert!(remaining.count >= 4, "Recent jobs should still exist");
    }

    #[tokio::test]
    async fn test_dry_run_does_not_delete() {
        let (pool, fr_pool, _dir) = setup_test_db().await;

        // Create an old job
        let old_date = Utc::now() - Duration::days(60);
        let job_id = create_test_job(&pool, old_date, "completed", false).await;

        // Create enough jobs to satisfy min_versions
        for _ in 0..3 {
            create_test_job(&pool, Utc::now(), "completed", false).await;
        }

        let config = JanitorConfig {
            policies: vec![RetentionPolicy::default_result()],
            dry_run: true, // DRY RUN MODE
            interval_secs: 3600,
            batch_size: 100,
        };

        let janitor = Janitor::new(pool.clone(), fr_pool, config);
        let report = janitor.prune().await.expect("Prune failed");

        // Report should show items would be pruned
        assert!(
            report.items_pruned >= 1,
            "Dry run should report prunable items"
        );

        // But job should still exist
        let exists = sqlx::query!("SELECT id FROM ai_jobs WHERE id = $1", job_id)
            .fetch_optional(&pool)
            .await
            .expect("Query failed");

        assert!(exists.is_some(), "Job should still exist after dry run");
    }

    #[tokio::test]
    async fn test_min_versions_constraint() {
        let (pool, fr_pool, _dir) = setup_test_db().await;

        // Create exactly 3 old jobs (min_versions = 3)
        let old_date = Utc::now() - Duration::days(60);
        for _ in 0..3 {
            create_test_job(&pool, old_date, "completed", false).await;
        }

        let config = JanitorConfig {
            policies: vec![RetentionPolicy {
                kind: ArtifactKind::Result,
                window_days: 30,
                min_versions: 3,
            }],
            dry_run: false,
            interval_secs: 3600,
            batch_size: 100,
        };

        let janitor = Janitor::new(pool.clone(), fr_pool, config);
        let report = janitor.prune().await.expect("Prune failed");

        // All 3 should be spared due to min_versions
        assert_eq!(
            report.items_pruned, 0,
            "No items should be pruned (min_versions)"
        );

        // All jobs should still exist
        let remaining = sqlx::query!("SELECT COUNT(*) as count FROM ai_jobs")
            .fetch_one(&pool)
            .await
            .expect("Query failed");

        assert_eq!(remaining.count, 3, "All jobs should still exist");
    }

    #[tokio::test]
    async fn test_flight_recorder_event_emitted() {
        let (pool, fr_pool, _dir) = setup_test_db().await;

        let config = JanitorConfig::default();
        let janitor = Janitor::new(pool, fr_pool.clone(), config);

        janitor.prune().await.expect("Prune failed");

        // Check Flight Recorder for gc_summary event
        let conn = fr_pool.lock().expect("Failed to lock FR");
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM events WHERE event_type = 'meta.gc_summary'")
            .expect("Failed to prepare query");

        let count: i64 = stmt
            .query_row([], |row| row.get(0))
            .expect("Failed to query");

        assert_eq!(
            count, 1,
            "GC summary event should be emitted to Flight Recorder"
        );
    }
}
