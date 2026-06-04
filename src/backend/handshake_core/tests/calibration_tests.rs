use chrono::{Duration, Utc};
use handshake_core::memory::calibration::{
    CalibrationCollector, CalibrationError, CalibrationMetrics, CalibrationThresholds,
    FemsCalibrationSource, SignalStatus, MEMORY_CALIBRATION_SNAPSHOT_COMMAND,
};

#[derive(Clone)]
struct StubCalibrationSource {
    metrics: Result<CalibrationMetrics, CalibrationError>,
}

impl FemsCalibrationSource for StubCalibrationSource {
    fn calibration_metrics(&self) -> Result<CalibrationMetrics, CalibrationError> {
        self.metrics.clone()
    }
}

fn base_metrics() -> CalibrationMetrics {
    CalibrationMetrics {
        total_items: 100,
        total_bytes: 16_384,
        bytes_growth_rate: 128.0,
        items_older_than_30d: 0,
        average_trust: 0.95,
        items_without_embedding: 0,
        recent_retrievals_total: 20,
        recent_retrievals_degraded: 0,
        trust_histogram_current: vec![0, 0, 0, 0, 20],
        trust_histogram_baseline: vec![0, 0, 0, 0, 20],
        last_hygiene_run_at: Some(Utc::now() - Duration::hours(1)),
        observed_at_utc: Utc::now(),
    }
}

fn source(metrics: CalibrationMetrics) -> StubCalibrationSource {
    StubCalibrationSource {
        metrics: Ok(metrics),
    }
}

#[test]
fn snapshot_command_constant_is_stable_read_only_ipc_name() {
    assert_eq!(
        MEMORY_CALIBRATION_SNAPSHOT_COMMAND,
        "kernel.memory_calibration.snapshot"
    );
}

#[test]
fn thresholds_reject_non_finite_and_inverted_values() {
    let mut thresholds = CalibrationThresholds::default();
    thresholds.embedding_gap_watch = 0.30;
    thresholds.embedding_gap_alert = 0.20;

    let err = CalibrationCollector::collect_snapshot(&source(base_metrics()), thresholds)
        .expect_err("inverted threshold pair must fail closed");
    assert!(matches!(
        err,
        CalibrationError::InvalidThreshold { field, .. } if field == "embedding_gap"
    ));

    let mut thresholds = CalibrationThresholds::default();
    thresholds.degradation_rate_watch = f64::NAN;
    let err = CalibrationCollector::collect_snapshot(&source(base_metrics()), thresholds)
        .expect_err("non-finite threshold must fail closed");
    assert!(matches!(
        err,
        CalibrationError::InvalidThreshold { field, .. } if field == "degradation_rate"
    ));
}

#[test]
fn degradation_rate_uses_recent_retrieval_counts_not_precomputed_fraction() {
    let mut metrics = base_metrics();
    metrics.recent_retrievals_total = 10;
    metrics.recent_retrievals_degraded = 4;

    let snapshot =
        CalibrationCollector::collect_snapshot(&source(metrics), CalibrationThresholds::default())
            .unwrap();

    assert_eq!(
        snapshot.signals.degradation_rate.status,
        SignalStatus::Alert
    );
    assert!((snapshot.signals.degradation_rate.value - 0.4).abs() < f64::EPSILON);
    assert_eq!(
        snapshot.signals.degradation_rate.details["recent_retrievals_total"],
        10.0
    );
}

#[test]
fn bloat_signal_carries_item_count_and_bytes_growth_details() {
    let mut metrics = base_metrics();
    metrics.total_items = 100_001;
    metrics.total_bytes = 8_192_000;
    metrics.bytes_growth_rate = 65_536.0;

    let snapshot =
        CalibrationCollector::collect_snapshot(&source(metrics), CalibrationThresholds::default())
            .unwrap();

    assert_eq!(snapshot.signals.bloat.status, SignalStatus::Alert);
    assert_eq!(snapshot.signals.bloat.value, 100_001.0);
    assert_eq!(snapshot.signals.bloat.details["total_bytes"], 8_192_000.0);
    assert_eq!(
        snapshot.signals.bloat.details["bytes_growth_rate"],
        65_536.0
    );
}

#[test]
fn zero_item_snapshot_keeps_ratio_signals_healthy_and_finite() {
    let mut metrics = base_metrics();
    metrics.total_items = 0;
    metrics.items_older_than_30d = 5;
    metrics.items_without_embedding = 5;
    metrics.recent_retrievals_total = 0;
    metrics.recent_retrievals_degraded = 10;

    let snapshot =
        CalibrationCollector::collect_snapshot(&source(metrics), CalibrationThresholds::default())
            .unwrap();

    assert_eq!(
        snapshot.signals.stale_dominance.status,
        SignalStatus::Healthy
    );
    assert_eq!(snapshot.signals.embedding_gap.status, SignalStatus::Healthy);
    assert_eq!(
        snapshot.signals.degradation_rate.status,
        SignalStatus::Healthy
    );
    assert!(snapshot.signals.stale_dominance.value.is_finite());
    assert!(snapshot.signals.embedding_gap.value.is_finite());
    assert!(snapshot.signals.degradation_rate.value.is_finite());
}

#[test]
fn trust_drift_uses_histogram_distance_when_baseline_is_available() {
    let mut metrics = base_metrics();
    metrics.average_trust = 0.95;
    metrics.trust_histogram_current = vec![20, 0, 0, 0, 0];
    metrics.trust_histogram_baseline = vec![0, 0, 0, 0, 20];

    let snapshot =
        CalibrationCollector::collect_snapshot(&source(metrics), CalibrationThresholds::default())
            .unwrap();

    assert_eq!(snapshot.signals.trust_drift.status, SignalStatus::Alert);
    assert!((snapshot.signals.trust_drift.value - 1.0).abs() < f64::EPSILON);
    assert_eq!(
        snapshot.signals.trust_drift.details["trust_histogram_current_bin_0"],
        20.0
    );
}

#[test]
fn one_signal_source_error_does_not_poison_the_whole_snapshot() {
    struct PartialFailureSource {
        metrics: CalibrationMetrics,
    }

    impl FemsCalibrationSource for PartialFailureSource {
        fn calibration_metrics(&self) -> Result<CalibrationMetrics, CalibrationError> {
            Ok(self.metrics.clone())
        }

        fn calibration_metrics_for(
            &self,
            signal: handshake_core::memory::calibration::CalibrationSignalKind,
        ) -> Result<CalibrationMetrics, CalibrationError> {
            if signal == handshake_core::memory::calibration::CalibrationSignalKind::EmbeddingGap {
                Err(CalibrationError::Io {
                    message: "embedding projection unavailable".to_string(),
                })
            } else {
                Ok(self.metrics.clone())
            }
        }
    }

    let snapshot = CalibrationCollector::collect_snapshot(
        &PartialFailureSource {
            metrics: base_metrics(),
        },
        CalibrationThresholds::default(),
    )
    .unwrap();

    assert_eq!(snapshot.signals.bloat.status, SignalStatus::Healthy);
    assert_eq!(snapshot.signals.embedding_gap.status, SignalStatus::Alert);
    assert_eq!(snapshot.signal_errors.len(), 1);
    assert!(snapshot.signal_errors[0]
        .message
        .contains("embedding projection unavailable"));
}
