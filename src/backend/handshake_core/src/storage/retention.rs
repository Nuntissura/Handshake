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
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use crate::storage::{ArtifactKind, Database, PruneReport, RetentionPolicy};

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
    Database(String),
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
    storage: Arc<dyn Database>,
    flight_recorder: Arc<dyn FlightRecorder>,
    config: JanitorConfig,
}

impl Janitor {
    pub fn new(
        storage: Arc<dyn Database>,
        flight_recorder: Arc<dyn FlightRecorder>,
        config: JanitorConfig,
    ) -> Self {
        Self {
            storage,
            flight_recorder,
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
        self.emit_gc_summary(&report).await?;
        self.flight_recorder
            .enforce_retention()
            .await
            .map_err(|e| RetentionError::FlightRecorder(e.to_string()))?;

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

        let job_report = self
            .storage
            .prune_ai_jobs(cutoff, policy.min_versions, self.config.dry_run)
            .await
            .map_err(|e| RetentionError::Database(e.to_string()))?;

        report.items_scanned += job_report.items_scanned;
        report.items_pruned += job_report.items_pruned;
        report.items_spared_pinned += job_report.items_spared_pinned;
        report.items_spared_window += job_report.items_spared_window;
        report.total_bytes_freed += job_report.total_bytes_freed;

        Ok(())
    }

    /// [HSK-GC-003] Emit meta.gc_summary event to Flight Recorder.
    async fn emit_gc_summary(&self, report: &PruneReport) -> Result<(), RetentionError> {
        let payload = serde_json::json!({
            "timestamp": report.timestamp.to_rfc3339(),
            "items_scanned": report.items_scanned,
            "items_pruned": report.items_pruned,
            "items_spared_pinned": report.items_spared_pinned,
            "items_spared_window": report.items_spared_window,
            "total_bytes_freed": report.total_bytes_freed,
            "dry_run": self.config.dry_run
        });

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            Uuid::new_v4(),
            payload,
        )
        .with_actor_id("janitor");

        self.flight_recorder
            .record_event(event)
            .await
            .map_err(|e| RetentionError::FlightRecorder(e.to_string()))
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
    use crate::flight_recorder::{EventFilter, RecorderError};
    use crate::storage::sqlite::SqliteDatabase;
    use crate::storage::{AccessMode, JobKind, JobMetrics, NewAiJob, SafetyMode};
    use async_trait::async_trait;
    use std::error::Error;
    use std::sync::Mutex;
    use tempfile::tempdir;

    #[derive(Clone)]
    struct MemoryRecorder {
        events: Arc<Mutex<Vec<FlightRecorderEvent>>>,
        retention_days: u32,
    }

    impl MemoryRecorder {
        fn new(retention_days: u32) -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
                retention_days,
            }
        }

        fn events(&self) -> Arc<Mutex<Vec<FlightRecorderEvent>>> {
            self.events.clone()
        }
    }

    #[async_trait]
    impl FlightRecorder for MemoryRecorder {
        async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
            self.events
                .lock()
                .map_err(|_| RecorderError::LockError)?
                .push(event);
            Ok(())
        }

        async fn enforce_retention(&self) -> Result<u64, RecorderError> {
            let cutoff = Utc::now() - Duration::days(self.retention_days as i64);
            let mut guard = self.events.lock().map_err(|_| RecorderError::LockError)?;
            let before = guard.len();
            guard.retain(|evt| evt.timestamp >= cutoff);
            Ok((before.saturating_sub(guard.len())) as u64)
        }

        async fn list_events(
            &self,
            _filter: EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            Ok(self
                .events
                .lock()
                .map_err(|_| RecorderError::LockError)?
                .clone())
        }
    }

    async fn setup_test_db() -> Result<
        (
            Arc<dyn Database>,
            Arc<dyn FlightRecorder>,
            Arc<Mutex<Vec<FlightRecorderEvent>>>,
            tempfile::TempDir,
        ),
        Box<dyn Error>,
    > {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let db = SqliteDatabase::connect(&db_url, 1).await?;
        db.run_migrations().await?;

        // In-memory recorder for tests
        let recorder = MemoryRecorder::new(7);
        let events = recorder.events();

        Ok((db.into_arc(), Arc::new(recorder), events, dir))
    }

    async fn create_test_job(
        db: &Arc<dyn Database>,
        created_at: DateTime<Utc>,
        status: &str,
        is_pinned: bool,
    ) -> Result<String, Box<dyn Error>> {
        let trace_id = uuid::Uuid::new_v4();
        let job = db
            .create_ai_job(NewAiJob {
                trace_id,
                job_kind: JobKind::WorkflowRun,
                protocol_id: "proto".into(),
                profile_id: "profile".into(),
                capability_profile_id: "cap".into(),
                access_mode: AccessMode::AnalysisOnly,
                safety_mode: SafetyMode::Normal,
                entity_refs: vec![],
                planned_operations: vec![],
                status_reason: "test".into(),
                metrics: JobMetrics::zero(),
                job_inputs: None,
            })
            .await?;

        if let Some(sqlite_db) = db.as_any().downcast_ref::<SqliteDatabase>() {
            let id_str = job.job_id.to_string();
            let created_at_str = created_at.to_rfc3339();
            let pinned = if is_pinned { 1i32 } else { 0i32 };
            // Use sqlx::query instead of query! to avoid compilation issues with missing metadata in tests
            sqlx::query(
                "UPDATE ai_jobs SET status = ?, created_at = ?, is_pinned = ? WHERE id = ?",
            )
            .bind(status)
            .bind(created_at_str)
            .bind(pinned)
            .bind(id_str)
            .execute(sqlite_db.pool())
            .await?;
        }

        Ok(job.job_id.to_string())
    }

    #[tokio::test]
    async fn test_prune_respects_pinned_items() -> Result<(), Box<dyn Error>> {
        let (db, flight_recorder, _events, _dir) = setup_test_db().await?;

        // Create an old, completed, pinned job (should be spared)
        let old_date = Utc::now() - Duration::days(60);
        let pinned_id = create_test_job(&db, old_date, "completed", true).await?;

        // Create an old, completed, unpinned job (should be deleted)
        let _unpinned_id = create_test_job(&db, old_date, "completed", false).await?;

        // Create 3 more unpinned jobs to satisfy min_versions
        for _ in 0..3 {
            create_test_job(&db, Utc::now(), "completed", false).await?;
        }

        let config = JanitorConfig {
            policies: vec![RetentionPolicy::default_result()],
            dry_run: false,
            interval_secs: 3600,
            batch_size: 100,
        };

        let janitor = Janitor::new(db.clone(), flight_recorder, config);
        let report = janitor.prune().await?;

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
        let job = db.get_ai_job(&pinned_id).await?;
        assert_eq!(job.job_id.to_string(), pinned_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_prune_respects_window() -> Result<(), Box<dyn Error>> {
        let (db, flight_recorder, _events, _dir) = setup_test_db().await?;

        // Create a recent job (within window, should be spared)
        let recent_date = Utc::now() - Duration::days(5);
        let _recent_id = create_test_job(&db, recent_date, "completed", false).await?;

        // Create an old job (outside window, should be deleted)
        let old_date = Utc::now() - Duration::days(60);
        let _old_id = create_test_job(&db, old_date, "completed", false).await?;

        // Create more recent jobs to satisfy min_versions
        for _ in 0..3 {
            create_test_job(&db, Utc::now(), "completed", false).await?;
        }

        let config = JanitorConfig {
            policies: vec![RetentionPolicy::default_result()],
            dry_run: false,
            interval_secs: 3600,
            batch_size: 100,
        };

        let janitor = Janitor::new(db.clone(), flight_recorder, config);
        let report = janitor.prune().await?;

        // Old job should be pruned
        assert!(report.items_pruned >= 1, "Old item should be pruned");

        Ok(())
    }

    #[tokio::test]
    async fn test_dry_run_does_not_delete() -> Result<(), Box<dyn Error>> {
        let (db, flight_recorder, _events, _dir) = setup_test_db().await?;

        // Create an old job
        let old_date = Utc::now() - Duration::days(60);
        let job_id = create_test_job(&db, old_date, "completed", false).await?;

        // Create enough jobs to satisfy min_versions
        for _ in 0..3 {
            create_test_job(&db, Utc::now(), "completed", false).await?;
        }

        let config = JanitorConfig {
            policies: vec![RetentionPolicy::default_result()],
            dry_run: true, // DRY RUN MODE
            interval_secs: 3600,
            batch_size: 100,
        };

        let janitor = Janitor::new(db.clone(), flight_recorder, config);
        let report = janitor.prune().await?;

        // Report should show items would be pruned
        assert!(
            report.items_pruned >= 1,
            "Dry run should report prunable items"
        );

        // But job should still exist
        let exists = db.get_ai_job(&job_id).await;
        assert!(exists.is_ok(), "Job should still exist after dry run");
        Ok(())
    }

    #[tokio::test]
    async fn test_min_versions_constraint() -> Result<(), Box<dyn Error>> {
        let (db, flight_recorder, _events, _dir) = setup_test_db().await?;

        // Create exactly 3 old jobs (min_versions = 3)
        let old_date = Utc::now() - Duration::days(60);
        for _ in 0..3 {
            create_test_job(&db, old_date, "completed", false).await?;
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

        let janitor = Janitor::new(db.clone(), flight_recorder, config);
        let report = janitor.prune().await?;

        // All 3 should be spared due to min_versions
        assert_eq!(
            report.items_pruned, 0,
            "No items should be pruned (min_versions)"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_flight_recorder_event_emitted() -> Result<(), Box<dyn Error>> {
        let (db, flight_recorder, events, _dir) = setup_test_db().await?;

        let config = JanitorConfig::default();
        let janitor = Janitor::new(db, flight_recorder, config);

        janitor.prune().await?;

        // Check Flight Recorder for gc_summary event
        let count = events
            .lock()
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to lock FR events: {e}"),
                )
            })?
            .iter()
            .filter(|evt| matches!(evt.event_type, FlightRecorderEventType::System))
            .count();

        assert_eq!(
            count, 1,
            "GC summary event should be emitted to Flight Recorder"
        );
        Ok(())
    }
}
