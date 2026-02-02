//! Retention & Pruning Service [§2.3.11]
//!
//! Implements the Janitor service for automated data cleanup per Master Spec §2.3.11.
//! Prunes old AI jobs and logs based on configurable retention policies.
//!
//! Hard Invariants:
//! - [HSK-GC-002] Pinned items (is_pinned = 1) are excluded from GC
//! - [HSK-GC-003] Every GC run emits meta.gc_summary to Flight Recorder
//! - [HSK-GC-004] PruneReport is written before items are unlinked

#[cfg(test)]
use chrono::DateTime;
use chrono::Duration;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

use crate::ace::ArtifactHandle;
use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use crate::storage::artifacts::{
    artifact_root_dir, artifact_root_rel, artifact_store_root, read_artifact_manifest,
    resolve_workspace_root, write_file_artifact, ArtifactClassification, ArtifactLayer,
    ArtifactManifest, ArtifactPayloadKind,
};
use crate::storage::{ArtifactKind, Database, PruneReport, RetentionPolicy};
use sha2::{Digest, Sha256};

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

#[derive(Debug, Clone, Serialize)]
struct DeletedArtifactRecord {
    artifact_id: Uuid,
    layer: ArtifactLayer,
    reason: String,
    size_bytes: u64,
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
        let workspace_root = resolve_workspace_root()
            .map_err(|e| RetentionError::Database(format!("workspace root resolve failed: {e}")))?;
        self.prune_in_workspace(&workspace_root).await
    }

    /// Run a single prune pass within an explicit workspace root.
    ///
    /// This avoids reliance on the process-global `HANDSHAKE_WORKSPACE_ROOT` environment variable
    /// and is primarily intended for tests and callers that already have a workspace context.
    pub async fn prune_in_workspace(
        &self,
        workspace_root: &Path,
    ) -> Result<PruneReport, RetentionError> {
        let mut report = PruneReport::new();
        let now = report.timestamp;
        let mut deleted_artifacts: Vec<DeletedArtifactRecord> = Vec::new();

        // Phase 1 ordering (HSK-GC-004): materialize report artifact before deletions.
        // 1) Plan pass (dry_run=true) to compute deterministic PruneReport.
        // 2) Write PruneReport as an artifact.
        // 3) Execute deletions (dry_run=false) if configured.
        let mut planned: Vec<(RetentionPolicy, chrono::DateTime<chrono::Utc>)> = Vec::new();
        for policy in &self.config.policies {
            let cutoff = now - Duration::days(policy.window_days as i64);
            planned.push((policy.clone(), cutoff));

            match policy.kind {
                ArtifactKind::Result => {
                    self.prune_ai_jobs_with_mode(policy, cutoff, true, &mut report)
                        .await?;
                }
                _ => {
                    tracing::debug!(
                        target: "handshake_core::janitor",
                        kind = %policy.kind,
                        "Skipping artifact kind (not implemented in Phase 1)"
                    );
                }
            }
        }

        self.plan_ttl_artifact_gc(workspace_root, now, &mut report, &mut deleted_artifacts)?;
        let gc_report_artifact =
            self.write_prune_report_artifact(workspace_root, &report, &deleted_artifacts)?;

        if !self.config.dry_run {
            for (policy, cutoff) in planned {
                match policy.kind {
                    ArtifactKind::Result => {
                        let _ = self
                            .storage
                            .prune_ai_jobs(cutoff, policy.min_versions, false)
                            .await
                            .map_err(|e| RetentionError::Database(e.to_string()))?;
                    }
                    _ => {}
                }
            }

            for record in &deleted_artifacts {
                let artifact_root =
                    artifact_root_dir(workspace_root, record.layer, record.artifact_id);
                fs::remove_dir_all(&artifact_root).map_err(|e| {
                    RetentionError::Database(format!(
                        "failed to delete artifact {}: {e}",
                        artifact_root.to_string_lossy()
                    ))
                })?;
            }
        }

        // [HSK-GC-003] Emit meta.gc_summary to Flight Recorder
        self.emit_gc_summary(&report, &gc_report_artifact, &deleted_artifacts)
            .await?;
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
    async fn prune_ai_jobs_with_mode(
        &self,
        policy: &RetentionPolicy,
        cutoff: chrono::DateTime<chrono::Utc>,
        dry_run: bool,
        report: &mut PruneReport,
    ) -> Result<(), RetentionError> {
        let job_report = self
            .storage
            .prune_ai_jobs(cutoff, policy.min_versions, dry_run)
            .await
            .map_err(|e| RetentionError::Database(e.to_string()))?;

        report.items_scanned += job_report.items_scanned;
        report.items_pruned += job_report.items_pruned;
        report.items_spared_pinned += job_report.items_spared_pinned;
        report.items_spared_window += job_report.items_spared_window;
        report.total_bytes_freed += job_report.total_bytes_freed;

        Ok(())
    }

    fn write_prune_report_artifact(
        &self,
        workspace_root: &Path,
        report: &PruneReport,
        deleted_artifacts: &[DeletedArtifactRecord],
    ) -> Result<ArtifactHandle, RetentionError> {
        let payload = serde_json::json!({
            "timestamp": report.timestamp,
            "items_scanned": report.items_scanned,
            "items_pruned": report.items_pruned,
            "items_spared_pinned": report.items_spared_pinned,
            "items_spared_window": report.items_spared_window,
            "total_bytes_freed": report.total_bytes_freed,
            "deleted_artifacts": deleted_artifacts,
        });

        let payload_bytes = serde_json::to_vec_pretty(&payload)
            .map_err(|e| RetentionError::Database(format!("serialize PruneReport: {e}")))?;

        let mut h = Sha256::new();
        h.update(&payload_bytes);
        let content_hash = hex::encode(h.finalize());

        let artifact_id = Uuid::new_v4();
        let manifest = ArtifactManifest {
            artifact_id,
            layer: ArtifactLayer::L3,
            kind: ArtifactPayloadKind::Report,
            mime: "application/json".to_string(),
            filename_hint: Some("gc_report.json".to_string()),
            created_at: report.timestamp,
            created_by_job_id: None,
            source_entity_refs: vec![crate::storage::EntityRef {
                entity_id: "janitor".to_string(),
                entity_kind: "system".to_string(),
            }],
            source_artifact_refs: Vec::new(),
            content_hash,
            size_bytes: payload_bytes.len() as u64,
            classification: ArtifactClassification::Low,
            exportable: false,
            retention_ttl_days: None,
            pinned: None,
            hash_basis: None,
            hash_exclude_paths: Vec::new(),
        };

        write_file_artifact(&workspace_root, &manifest, &payload_bytes)
            .map_err(|e| RetentionError::Database(e.to_string()))?;

        Ok(ArtifactHandle::new(
            artifact_id,
            artifact_root_rel(ArtifactLayer::L3, artifact_id),
        ))
    }

    /// [HSK-GC-003] Emit meta.gc_summary event to Flight Recorder.
    async fn emit_gc_summary(
        &self,
        report: &PruneReport,
        gc_report_artifact: &ArtifactHandle,
        deleted_artifacts: &[DeletedArtifactRecord],
    ) -> Result<(), RetentionError> {
        let payload = serde_json::json!({
            "event_type": "meta.gc_summary",
            "event_name": "gc_summary",
            "timestamp": report.timestamp.to_rfc3339(),
            "items_scanned": report.items_scanned,
            "items_pruned": report.items_pruned,
            "items_spared_pinned": report.items_spared_pinned,
            "items_spared_window": report.items_spared_window,
            "total_bytes_freed": report.total_bytes_freed,
            "dry_run": self.config.dry_run,
            "gc_report_artifact": gc_report_artifact,
            "deleted_artifacts": deleted_artifacts,
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

    fn plan_ttl_artifact_gc(
        &self,
        workspace_root: &Path,
        now: chrono::DateTime<chrono::Utc>,
        report: &mut PruneReport,
        deleted_artifacts: &mut Vec<DeletedArtifactRecord>,
    ) -> Result<(), RetentionError> {
        let layer = ArtifactLayer::L3;
        let layer_root = artifact_store_root(workspace_root).join(layer.as_str());
        if !layer_root.exists() || !layer_root.is_dir() {
            return Ok(());
        }

        let mut candidates: Vec<(Uuid, ArtifactManifest, PathBuf)> = Vec::new();
        for entry in fs::read_dir(&layer_root)
            .map_err(|e| RetentionError::Database(format!("failed to read artifact store: {e}")))?
        {
            let entry = entry.map_err(|e| RetentionError::Database(e.to_string()))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Ok(artifact_id) = Uuid::parse_str(name) else {
                continue;
            };

            let manifest = read_artifact_manifest(workspace_root, layer, artifact_id)
                .map_err(|e| RetentionError::Database(e.to_string()))?;
            candidates.push((artifact_id, manifest, path));
        }

        candidates.sort_by(|a, b| a.0.to_string().cmp(&b.0.to_string()));

        for (artifact_id, manifest, artifact_root) in candidates {
            report.items_scanned = report.items_scanned.saturating_add(1);

            if manifest.pinned.unwrap_or(false) {
                report.items_spared_pinned = report.items_spared_pinned.saturating_add(1);
                continue;
            }

            let Some(ttl_days) = manifest.retention_ttl_days else {
                report.items_spared_window = report.items_spared_window.saturating_add(1);
                continue;
            };

            let expires_at = manifest.created_at + Duration::days(ttl_days as i64);
            if expires_at > now {
                report.items_spared_window = report.items_spared_window.saturating_add(1);
                continue;
            }

            let size_bytes = dir_size_bytes(&artifact_root).unwrap_or(0);
            report.items_pruned = report.items_pruned.saturating_add(1);
            report.total_bytes_freed = report.total_bytes_freed.saturating_add(size_bytes);

            deleted_artifacts.push(DeletedArtifactRecord {
                artifact_id,
                layer,
                reason: format!("expired_retention_ttl_days({ttl_days})"),
                size_bytes,
            });
        }

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

fn dir_size_bytes(root: &Path) -> std::io::Result<u64> {
    if root.is_file() {
        return Ok(fs::metadata(root)?.len());
    }

    let mut total = 0u64;
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if meta.is_dir() {
                stack.push(entry.path());
            } else if meta.is_file() {
                total = total.saturating_add(meta.len());
            }
        }
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight_recorder::{EventFilter, RecorderError};
    use crate::storage::sqlite::SqliteDatabase;
    use crate::storage::{AccessMode, JobKind, JobMetrics, NewAiJob, SafetyMode};
    use async_trait::async_trait;
    use chrono::Utc;
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
            tempfile::TempDir,
        ),
        Box<dyn Error>,
    > {
        let workspace_dir = tempdir()?;

        let db_dir = tempdir()?;
        let db_path = db_dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let db = SqliteDatabase::connect(&db_url, 1).await?;
        db.run_migrations().await?;

        // In-memory recorder for tests
        let recorder = MemoryRecorder::new(7);
        let events = recorder.events();

        Ok((
            db.into_arc(),
            Arc::new(recorder),
            events,
            db_dir,
            workspace_dir,
        ))
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
        let (db, flight_recorder, _events, _db_dir, workspace_dir) = setup_test_db().await?;

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
        let report = janitor.prune_in_workspace(workspace_dir.path()).await?;

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
        let (db, flight_recorder, _events, _db_dir, workspace_dir) = setup_test_db().await?;

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
        let report = janitor.prune_in_workspace(workspace_dir.path()).await?;

        // Old job should be pruned
        assert!(report.items_pruned >= 1, "Old item should be pruned");

        Ok(())
    }

    #[tokio::test]
    async fn test_dry_run_does_not_delete() -> Result<(), Box<dyn Error>> {
        let (db, flight_recorder, _events, _db_dir, workspace_dir) = setup_test_db().await?;

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
        let report = janitor.prune_in_workspace(workspace_dir.path()).await?;

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
        let (db, flight_recorder, _events, _db_dir, workspace_dir) = setup_test_db().await?;

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
        let report = janitor.prune_in_workspace(workspace_dir.path()).await?;

        // All 3 should be spared due to min_versions
        assert_eq!(
            report.items_pruned, 0,
            "No items should be pruned (min_versions)"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_flight_recorder_event_emitted() -> Result<(), Box<dyn Error>> {
        let (db, flight_recorder, events, _db_dir, workspace_dir) = setup_test_db().await?;

        let config = JanitorConfig::default();
        let janitor = Janitor::new(db, flight_recorder, config);

        janitor.prune_in_workspace(workspace_dir.path()).await?;

        // Check Flight Recorder for gc_summary event
        let count = events
            .lock()
            .map_err(|e| std::io::Error::other(format!("Failed to lock FR events: {e}")))?
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
