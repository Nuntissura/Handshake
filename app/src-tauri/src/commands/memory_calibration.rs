//! MT-162 FEMS calibration snapshot IPC.
//!
//! The command is read-only. Production binds to Postgres FEMS metrics when
//! storage initializes; otherwise the command returns a typed unavailable
//! error rather than a placeholder healthy snapshot.

use std::sync::Arc;

use handshake_core::{
    memory::{
        bitemporal::PostgresBitemporalMemoryIndex,
        calibration::{
            CalibrationCollector, CalibrationError, CalibrationMetrics, CalibrationSnapshot,
            CalibrationThresholds, FemsCalibrationSource, MEMORY_CALIBRATION_SNAPSHOT_COMMAND,
        },
    },
    storage::Database,
};
use tauri::State;

enum MemoryCalibrationBackend {
    Unavailable {
        reason: String,
    },
    Source {
        source: Arc<dyn FemsCalibrationSource>,
        thresholds: CalibrationThresholds,
    },
}

pub struct MemoryCalibrationIpcState {
    backend: MemoryCalibrationBackend,
}

impl Default for MemoryCalibrationIpcState {
    fn default() -> Self {
        Self {
            backend: MemoryCalibrationBackend::Unavailable {
                reason: "Postgres memory calibration state has not been initialized".to_string(),
            },
        }
    }
}

impl MemoryCalibrationIpcState {
    pub fn with_postgres(db: Arc<dyn Database>) -> Self {
        Self::with_source(
            Arc::new(PostgresBitemporalMemoryIndex::with_db(db)),
            CalibrationThresholds::default(),
        )
    }

    pub fn with_metrics(metrics: CalibrationMetrics) -> Self {
        Self::with_source(
            Arc::new(StaticCalibrationSource { metrics }),
            CalibrationThresholds::default(),
        )
    }

    pub fn with_source(
        source: Arc<dyn FemsCalibrationSource>,
        thresholds: CalibrationThresholds,
    ) -> Self {
        Self {
            backend: MemoryCalibrationBackend::Source { source, thresholds },
        }
    }

    pub fn from_env_or_unavailable() -> Self {
        match tauri::async_runtime::block_on(handshake_core::storage::init_storage()) {
            Ok(db) => Self::with_postgres(db),
            Err(error) => Self {
                backend: MemoryCalibrationBackend::Unavailable {
                    reason: format!("Postgres memory calibration state unavailable: {error}"),
                },
            },
        }
    }

    pub fn snapshot(&self) -> Result<CalibrationSnapshot, CalibrationError> {
        match &self.backend {
            MemoryCalibrationBackend::Unavailable { reason } => Err(CalibrationError::Io {
                message: reason.clone(),
            }),
            MemoryCalibrationBackend::Source { source, thresholds } => {
                CalibrationCollector::collect_snapshot(source.as_ref(), *thresholds)
            }
        }
    }
}

struct StaticCalibrationSource {
    metrics: CalibrationMetrics,
}

impl FemsCalibrationSource for StaticCalibrationSource {
    fn calibration_metrics(&self) -> Result<CalibrationMetrics, CalibrationError> {
        Ok(self.metrics.clone())
    }
}

#[tauri::command]
pub async fn kernel_memory_calibration_snapshot(
    state: State<'_, MemoryCalibrationIpcState>,
) -> Result<CalibrationSnapshot, String> {
    let _ = MEMORY_CALIBRATION_SNAPSHOT_COMMAND;
    state.snapshot().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use handshake_core::memory::calibration::{
        CalibrationMetrics, SignalStatus, MEMORY_CALIBRATION_SNAPSHOT_COMMAND,
    };

    use super::*;

    fn metrics() -> CalibrationMetrics {
        CalibrationMetrics {
            total_items: 200_000,
            total_bytes: 32_000_000,
            bytes_growth_rate: 1024.0,
            items_older_than_30d: 10,
            average_trust: 0.9,
            items_without_embedding: 0,
            recent_retrievals_total: 8,
            recent_retrievals_degraded: 0,
            trust_histogram_current: vec![0, 0, 0, 0, 8],
            trust_histogram_baseline: vec![0, 0, 0, 0, 8],
            last_hygiene_run_at: Some(Utc::now() - Duration::hours(2)),
            observed_at_utc: Utc::now(),
        }
    }

    #[test]
    fn memory_calibration_state_snapshot_is_read_only_and_stable() {
        let state = MemoryCalibrationIpcState::with_metrics(metrics());

        let first = state.snapshot().expect("snapshot should collect");
        let second = state.snapshot().expect("snapshot should collect again");

        assert_eq!(
            MEMORY_CALIBRATION_SNAPSHOT_COMMAND,
            "kernel.memory_calibration.snapshot"
        );
        assert_eq!(first.signals.bloat.status, SignalStatus::Alert);
        assert_eq!(second.signals.bloat.status, SignalStatus::Alert);
        assert_eq!(
            first.signals.bloat.details["total_bytes"],
            second.signals.bloat.details["total_bytes"]
        );
    }
}
