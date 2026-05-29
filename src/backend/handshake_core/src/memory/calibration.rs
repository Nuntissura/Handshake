//! MT-162: FEMS calibration dashboard data feed.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use super::bitemporal::{AsOfQuery, PostgresBitemporalMemoryIndex};

pub const MEMORY_CALIBRATION_SNAPSHOT_COMMAND: &str = "kernel.memory_calibration.snapshot";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalStatus {
    Healthy,
    Watch,
    Alert,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationSignal {
    pub value: f64,
    pub status: SignalStatus,
    pub updated_at_utc: DateTime<Utc>,
    pub threshold_watch: f64,
    pub threshold_alert: f64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub details: BTreeMap<String, f64>,
}

impl CalibrationSignal {
    pub fn from_value(value: f64, threshold_watch: f64, threshold_alert: f64) -> Self {
        Self::from_value_with_details(value, threshold_watch, threshold_alert, BTreeMap::new())
    }

    pub fn from_value_with_details(
        value: f64,
        threshold_watch: f64,
        threshold_alert: f64,
        details: BTreeMap<String, f64>,
    ) -> Self {
        let status = if value >= threshold_alert {
            SignalStatus::Alert
        } else if value >= threshold_watch {
            SignalStatus::Watch
        } else {
            SignalStatus::Healthy
        };
        Self {
            value,
            status,
            updated_at_utc: Utc::now(),
            threshold_watch,
            threshold_alert,
            details,
        }
    }
}

pub type BloatSignal = CalibrationSignal;
pub type StaleDominanceSignal = CalibrationSignal;
pub type TrustDriftSignal = CalibrationSignal;
pub type EmbeddingGapSignal = CalibrationSignal;
pub type DegradationRateSignal = CalibrationSignal;
pub type HygieneLagSignal = CalibrationSignal;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationSignals {
    pub bloat: BloatSignal,
    pub stale_dominance: StaleDominanceSignal,
    pub trust_drift: TrustDriftSignal,
    pub embedding_gap: EmbeddingGapSignal,
    pub degradation_rate: DegradationRateSignal,
    pub hygiene_lag: HygieneLagSignal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationSnapshot {
    pub signals: CalibrationSignals,
    pub generated_at_utc: DateTime<Utc>,
    pub source_observed_at_utc: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signal_errors: Vec<CalibrationSignalError>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationMetrics {
    pub total_items: u64,
    pub total_bytes: u64,
    pub bytes_growth_rate: f64,
    pub items_older_than_30d: u64,
    pub average_trust: f64,
    pub items_without_embedding: u64,
    pub recent_retrievals_total: u64,
    pub recent_retrievals_degraded: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trust_histogram_current: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trust_histogram_baseline: Vec<u64>,
    pub last_hygiene_run_at: Option<DateTime<Utc>>,
    pub observed_at_utc: DateTime<Utc>,
}

impl Default for CalibrationMetrics {
    fn default() -> Self {
        Self {
            total_items: 0,
            total_bytes: 0,
            bytes_growth_rate: 0.0,
            items_older_than_30d: 0,
            average_trust: 1.0,
            items_without_embedding: 0,
            recent_retrievals_total: 0,
            recent_retrievals_degraded: 0,
            trust_histogram_current: Vec::new(),
            trust_histogram_baseline: Vec::new(),
            last_hygiene_run_at: None,
            observed_at_utc: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationSignalKind {
    Bloat,
    StaleDominance,
    TrustDrift,
    EmbeddingGap,
    DegradationRate,
    HygieneLag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationSignalError {
    pub signal: CalibrationSignalKind,
    pub message: String,
}

/// Trait the collector consumes. Production can wire this to a Postgres-backed
/// FEMS projection; tests inject a fixed metrics snapshot.
pub trait FemsCalibrationSource: Send + Sync {
    fn calibration_metrics(&self) -> Result<CalibrationMetrics, CalibrationError>;

    fn calibration_metrics_for(
        &self,
        _signal: CalibrationSignalKind,
    ) -> Result<CalibrationMetrics, CalibrationError> {
        self.calibration_metrics()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CalibrationThresholds {
    pub bloat_watch_count: f64,
    pub bloat_alert_count: f64,
    pub stale_dominance_watch: f64,
    pub stale_dominance_alert: f64,
    pub trust_drift_watch: f64,
    pub trust_drift_alert: f64,
    pub embedding_gap_watch: f64,
    pub embedding_gap_alert: f64,
    pub degradation_rate_watch: f64,
    pub degradation_rate_alert: f64,
    pub hygiene_lag_hours_watch: f64,
    pub hygiene_lag_hours_alert: f64,
}

impl Default for CalibrationThresholds {
    fn default() -> Self {
        Self {
            bloat_watch_count: 10_000.0,
            bloat_alert_count: 100_000.0,
            stale_dominance_watch: 0.40,
            stale_dominance_alert: 0.70,
            trust_drift_watch: 0.30,
            trust_drift_alert: 0.50,
            embedding_gap_watch: 0.05,
            embedding_gap_alert: 0.20,
            degradation_rate_watch: 0.10,
            degradation_rate_alert: 0.30,
            hygiene_lag_hours_watch: 24.0,
            hygiene_lag_hours_alert: 72.0,
        }
    }
}

impl CalibrationThresholds {
    pub fn validate(&self) -> Result<(), CalibrationError> {
        validate_threshold_pair("bloat", self.bloat_watch_count, self.bloat_alert_count)?;
        validate_threshold_pair(
            "stale_dominance",
            self.stale_dominance_watch,
            self.stale_dominance_alert,
        )?;
        validate_threshold_pair(
            "trust_drift",
            self.trust_drift_watch,
            self.trust_drift_alert,
        )?;
        validate_threshold_pair(
            "embedding_gap",
            self.embedding_gap_watch,
            self.embedding_gap_alert,
        )?;
        validate_threshold_pair(
            "degradation_rate",
            self.degradation_rate_watch,
            self.degradation_rate_alert,
        )?;
        validate_threshold_pair(
            "hygiene_lag",
            self.hygiene_lag_hours_watch,
            self.hygiene_lag_hours_alert,
        )
    }
}

pub struct CalibrationCollector;

impl CalibrationCollector {
    pub fn collect_snapshot(
        src: &dyn FemsCalibrationSource,
        thresholds: CalibrationThresholds,
    ) -> Result<CalibrationSnapshot, CalibrationError> {
        thresholds.validate()?;
        let mut signal_errors = Vec::new();
        let mut observed_at = Utc::now();

        let bloat = collect_signal(
            src,
            CalibrationSignalKind::Bloat,
            thresholds.bloat_watch_count,
            thresholds.bloat_alert_count,
            &mut observed_at,
            &mut signal_errors,
            |metrics| {
                let total = metrics.total_items as f64;
                CalibrationSignal::from_value_with_details(
                    total,
                    thresholds.bloat_watch_count,
                    thresholds.bloat_alert_count,
                    BTreeMap::from([
                        ("total_items".to_string(), total),
                        ("total_bytes".to_string(), metrics.total_bytes as f64),
                        ("bytes_growth_rate".to_string(), metrics.bytes_growth_rate),
                    ]),
                )
            },
        );

        let stale_dominance = collect_signal(
            src,
            CalibrationSignalKind::StaleDominance,
            thresholds.stale_dominance_watch,
            thresholds.stale_dominance_alert,
            &mut observed_at,
            &mut signal_errors,
            |metrics| {
                let total = metrics.total_items as f64;
                let stale_value = ratio(metrics.items_older_than_30d, metrics.total_items);
                CalibrationSignal::from_value_with_details(
                    stale_value,
                    thresholds.stale_dominance_watch,
                    thresholds.stale_dominance_alert,
                    BTreeMap::from([
                        (
                            "items_older_than_30d".to_string(),
                            metrics.items_older_than_30d as f64,
                        ),
                        ("total_items".to_string(), total),
                    ]),
                )
            },
        );

        let trust_drift = collect_signal(
            src,
            CalibrationSignalKind::TrustDrift,
            thresholds.trust_drift_watch,
            thresholds.trust_drift_alert,
            &mut observed_at,
            &mut signal_errors,
            |metrics| {
                let (trust_value, histogram_details) = trust_histogram_drift(&metrics);
                let mut details =
                    BTreeMap::from([("average_trust".to_string(), metrics.average_trust)]);
                details.extend(histogram_details);
                CalibrationSignal::from_value_with_details(
                    trust_value,
                    thresholds.trust_drift_watch,
                    thresholds.trust_drift_alert,
                    details,
                )
            },
        );

        let embedding_gap = collect_signal(
            src,
            CalibrationSignalKind::EmbeddingGap,
            thresholds.embedding_gap_watch,
            thresholds.embedding_gap_alert,
            &mut observed_at,
            &mut signal_errors,
            |metrics| {
                let total = metrics.total_items as f64;
                let embedding_value = ratio(metrics.items_without_embedding, metrics.total_items);
                CalibrationSignal::from_value_with_details(
                    embedding_value,
                    thresholds.embedding_gap_watch,
                    thresholds.embedding_gap_alert,
                    BTreeMap::from([
                        (
                            "items_without_embedding".to_string(),
                            metrics.items_without_embedding as f64,
                        ),
                        ("total_items".to_string(), total),
                    ]),
                )
            },
        );

        let degradation_rate = collect_signal(
            src,
            CalibrationSignalKind::DegradationRate,
            thresholds.degradation_rate_watch,
            thresholds.degradation_rate_alert,
            &mut observed_at,
            &mut signal_errors,
            |metrics| {
                let degraded = ratio(
                    metrics.recent_retrievals_degraded,
                    metrics.recent_retrievals_total,
                );
                CalibrationSignal::from_value_with_details(
                    degraded,
                    thresholds.degradation_rate_watch,
                    thresholds.degradation_rate_alert,
                    BTreeMap::from([
                        (
                            "recent_retrievals_total".to_string(),
                            metrics.recent_retrievals_total as f64,
                        ),
                        (
                            "recent_retrievals_degraded".to_string(),
                            metrics.recent_retrievals_degraded as f64,
                        ),
                    ]),
                )
            },
        );

        let hygiene_lag = collect_signal(
            src,
            CalibrationSignalKind::HygieneLag,
            thresholds.hygiene_lag_hours_watch,
            thresholds.hygiene_lag_hours_alert,
            &mut observed_at,
            &mut signal_errors,
            |metrics| {
                let hygiene_lag_value = match metrics.last_hygiene_run_at {
                    Some(t) => {
                        let diff = Utc::now() - t;
                        let hours = diff.num_seconds() as f64 / 3600.0;
                        hours.max(0.0)
                    }
                    None => thresholds.hygiene_lag_hours_alert + 1.0,
                };
                CalibrationSignal::from_value(
                    hygiene_lag_value,
                    thresholds.hygiene_lag_hours_watch,
                    thresholds.hygiene_lag_hours_alert,
                )
            },
        );

        Ok(CalibrationSnapshot {
            signals: CalibrationSignals {
                bloat,
                stale_dominance,
                trust_drift,
                embedding_gap,
                degradation_rate,
                hygiene_lag,
            },
            generated_at_utc: Utc::now(),
            source_observed_at_utc: observed_at,
            signal_errors,
        })
    }
}

impl FemsCalibrationSource for PostgresBitemporalMemoryIndex {
    fn calibration_metrics(&self) -> Result<CalibrationMetrics, CalibrationError> {
        let now = Utc::now();
        let cutoff = now - Duration::days(30);
        let items =
            block_on_calibration(self.items_visible_at(&AsOfQuery::now())).map_err(|error| {
                CalibrationError::Io {
                    message: error.to_string(),
                }
            })?;

        let mut metrics = CalibrationMetrics {
            observed_at_utc: now,
            total_items: items.len() as u64,
            trust_histogram_current: vec![0; 5],
            ..CalibrationMetrics::default()
        };
        let mut trust_sum = 0.0;

        for item in items {
            metrics.total_bytes = metrics
                .total_bytes
                .saturating_add(item.payload.to_string().len() as u64);
            if item.stamps.recorded_at < cutoff {
                metrics.items_older_than_30d = metrics.items_older_than_30d.saturating_add(1);
            }
            let trust = json_f64(&item.payload, &["trust", "trust_score", "score"]).unwrap_or(1.0);
            trust_sum += trust.clamp(0.0, 1.0);
            let bin = trust_bin(trust);
            if let Some(count) = metrics.trust_histogram_current.get_mut(bin) {
                *count = count.saturating_add(1);
            }
            if let Some(baseline) = json_u64_array(&item.payload, &["trust_histogram_baseline"]) {
                merge_histogram(&mut metrics.trust_histogram_baseline, &baseline);
            }
            if json_array_len(&item.payload, &["embedding"])
                .map(|len| len == 0)
                .unwrap_or(true)
            {
                metrics.items_without_embedding = metrics.items_without_embedding.saturating_add(1);
            }
            metrics.bytes_growth_rate += json_f64(&item.payload, &["bytes_growth_rate"])
                .unwrap_or(0.0)
                .max(0.0);
            metrics.recent_retrievals_total = metrics
                .recent_retrievals_total
                .saturating_add(json_u64(&item.payload, &["recent_retrievals_total"]).unwrap_or(0));
            metrics.recent_retrievals_degraded = metrics.recent_retrievals_degraded.saturating_add(
                json_u64(&item.payload, &["recent_retrievals_degraded"]).unwrap_or(0),
            );
            if let Some(timestamp) = json_datetime(&item.payload, &["last_hygiene_run_at"]) {
                metrics.last_hygiene_run_at = Some(
                    metrics
                        .last_hygiene_run_at
                        .map(|current| current.max(timestamp))
                        .unwrap_or(timestamp),
                );
            }
        }

        if metrics.total_items > 0 {
            metrics.average_trust = trust_sum / metrics.total_items as f64;
        }

        Ok(metrics)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CalibrationError {
    #[error("calibration source IO failed: {message}")]
    Io { message: String },
    #[error("invalid calibration threshold {field}: {message}")]
    InvalidThreshold { field: String, message: String },
    #[error("invalid calibration metric {field}: {message}")]
    InvalidMetric { field: String, message: String },
}

fn validate_threshold_pair(field: &str, watch: f64, alert: f64) -> Result<(), CalibrationError> {
    if !watch.is_finite() || !alert.is_finite() {
        return Err(CalibrationError::InvalidThreshold {
            field: field.to_string(),
            message: "watch and alert thresholds must be finite".to_string(),
        });
    }
    if watch < 0.0 || alert < 0.0 {
        return Err(CalibrationError::InvalidThreshold {
            field: field.to_string(),
            message: "watch and alert thresholds must be non-negative".to_string(),
        });
    }
    if watch > alert {
        return Err(CalibrationError::InvalidThreshold {
            field: field.to_string(),
            message: "watch threshold must be <= alert threshold".to_string(),
        });
    }
    Ok(())
}

fn collect_signal(
    src: &dyn FemsCalibrationSource,
    signal: CalibrationSignalKind,
    threshold_watch: f64,
    threshold_alert: f64,
    observed_at: &mut DateTime<Utc>,
    errors: &mut Vec<CalibrationSignalError>,
    build: impl FnOnce(CalibrationMetrics) -> CalibrationSignal,
) -> CalibrationSignal {
    match src.calibration_metrics_for(signal) {
        Ok(metrics) => {
            *observed_at = (*observed_at).max(metrics.observed_at_utc);
            match validate_metrics(&metrics) {
                Ok(()) => build(metrics),
                Err(error) => {
                    failed_signal(signal, threshold_watch, threshold_alert, error, errors)
                }
            }
        }
        Err(error) => failed_signal(signal, threshold_watch, threshold_alert, error, errors),
    }
}

fn failed_signal(
    signal: CalibrationSignalKind,
    threshold_watch: f64,
    threshold_alert: f64,
    error: CalibrationError,
    errors: &mut Vec<CalibrationSignalError>,
) -> CalibrationSignal {
    errors.push(CalibrationSignalError {
        signal,
        message: error.to_string(),
    });
    CalibrationSignal::from_value_with_details(
        threshold_alert,
        threshold_watch,
        threshold_alert,
        BTreeMap::from([("source_error".to_string(), 1.0)]),
    )
}

fn validate_metrics(metrics: &CalibrationMetrics) -> Result<(), CalibrationError> {
    validate_finite_non_negative("bytes_growth_rate", metrics.bytes_growth_rate)?;
    validate_range("average_trust", metrics.average_trust, 0.0, 1.0)
}

fn validate_finite_non_negative(field: &str, value: f64) -> Result<(), CalibrationError> {
    if !value.is_finite() {
        return Err(CalibrationError::InvalidMetric {
            field: field.to_string(),
            message: "metric must be finite".to_string(),
        });
    }
    if value < 0.0 {
        return Err(CalibrationError::InvalidMetric {
            field: field.to_string(),
            message: "metric must be non-negative".to_string(),
        });
    }
    Ok(())
}

fn validate_range(field: &str, value: f64, min: f64, max: f64) -> Result<(), CalibrationError> {
    if !value.is_finite() || value < min || value > max {
        return Err(CalibrationError::InvalidMetric {
            field: field.to_string(),
            message: format!("metric must be finite and within [{min}, {max}]"),
        });
    }
    Ok(())
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        ((numerator as f64) / (denominator as f64)).clamp(0.0, 1.0)
    }
}

fn trust_histogram_drift(metrics: &CalibrationMetrics) -> (f64, BTreeMap<String, f64>) {
    let mut details = BTreeMap::new();
    details.insert(
        "trust_histogram_current_bins".to_string(),
        metrics.trust_histogram_current.len() as f64,
    );
    details.insert(
        "trust_histogram_baseline_bins".to_string(),
        metrics.trust_histogram_baseline.len() as f64,
    );

    if metrics.trust_histogram_current.is_empty()
        || metrics.trust_histogram_baseline.is_empty()
        || metrics.trust_histogram_current.len() != metrics.trust_histogram_baseline.len()
    {
        details.insert("trust_histogram_fallback".to_string(), 1.0);
        return ((1.0 - metrics.average_trust).clamp(0.0, 1.0), details);
    }

    let current_total = metrics.trust_histogram_current.iter().sum::<u64>() as f64;
    let baseline_total = metrics.trust_histogram_baseline.iter().sum::<u64>() as f64;
    if current_total <= 0.0 || baseline_total <= 0.0 {
        details.insert("trust_histogram_fallback".to_string(), 1.0);
        return ((1.0 - metrics.average_trust).clamp(0.0, 1.0), details);
    }

    let distance = metrics
        .trust_histogram_current
        .iter()
        .zip(metrics.trust_histogram_baseline.iter())
        .enumerate()
        .map(|(index, (current, baseline))| {
            details.insert(
                format!("trust_histogram_current_bin_{index}"),
                *current as f64,
            );
            details.insert(
                format!("trust_histogram_baseline_bin_{index}"),
                *baseline as f64,
            );
            ((*current as f64 / current_total) - (*baseline as f64 / baseline_total)).abs()
        })
        .sum::<f64>()
        / 2.0;
    (distance.clamp(0.0, 1.0), details)
}

fn block_on_calibration<F: std::future::Future>(future: F) -> F::Output {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(future))
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("calibration Postgres adapter runtime")
            .block_on(future)
    }
}

fn json_f64(payload: &Value, path: &[&str]) -> Option<f64> {
    let value = json_path(payload, path)?;
    value.as_f64().or_else(|| value.as_i64().map(|v| v as f64))
}

fn json_u64(payload: &Value, path: &[&str]) -> Option<u64> {
    json_path(payload, path)?.as_u64()
}

fn json_u64_array(payload: &Value, path: &[&str]) -> Option<Vec<u64>> {
    Some(
        json_path(payload, path)?
            .as_array()?
            .iter()
            .map(Value::as_u64)
            .collect::<Option<Vec<_>>>()?,
    )
}

fn json_array_len(payload: &Value, path: &[&str]) -> Option<usize> {
    json_path(payload, path)?.as_array().map(Vec::len)
}

fn json_datetime(payload: &Value, path: &[&str]) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(json_path(payload, path)?.as_str()?)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn json_path<'a>(payload: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(payload, |current, key| current.get(key))
}

fn trust_bin(trust: f64) -> usize {
    let trust = trust.clamp(0.0, 1.0);
    ((trust * 5.0).floor() as usize).min(4)
}

fn merge_histogram(target: &mut Vec<u64>, values: &[u64]) {
    if target.len() < values.len() {
        target.resize(values.len(), 0);
    }
    for (index, value) in values.iter().enumerate() {
        target[index] = target[index].saturating_add(*value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubSource {
        metrics: CalibrationMetrics,
    }

    impl FemsCalibrationSource for StubSource {
        fn calibration_metrics(&self) -> Result<CalibrationMetrics, CalibrationError> {
            Ok(self.metrics.clone())
        }
    }

    fn metrics() -> CalibrationMetrics {
        CalibrationMetrics {
            total_items: 100,
            total_bytes: 4096,
            bytes_growth_rate: 0.0,
            items_older_than_30d: 0,
            average_trust: 0.95,
            items_without_embedding: 0,
            recent_retrievals_total: 10,
            recent_retrievals_degraded: 0,
            trust_histogram_current: vec![0, 0, 0, 0, 10],
            trust_histogram_baseline: vec![0, 0, 0, 0, 10],
            last_hygiene_run_at: Some(Utc::now() - chrono::Duration::seconds(60)),
            observed_at_utc: Utc::now(),
        }
    }

    #[test]
    fn healthy_signals_for_pristine_state() {
        let src = StubSource { metrics: metrics() };
        let snap =
            CalibrationCollector::collect_snapshot(&src, CalibrationThresholds::default()).unwrap();
        assert_eq!(snap.signals.stale_dominance.status, SignalStatus::Healthy);
        assert_eq!(snap.signals.trust_drift.status, SignalStatus::Healthy);
        assert_eq!(snap.signals.embedding_gap.status, SignalStatus::Healthy);
        assert_eq!(snap.signals.degradation_rate.status, SignalStatus::Healthy);
        assert_eq!(snap.signals.hygiene_lag.status, SignalStatus::Healthy);
    }

    #[test]
    fn alert_when_stale_dominance_exceeds_threshold() {
        let mut metrics = metrics();
        metrics.items_older_than_30d = 80;
        let src = StubSource { metrics };
        let snap =
            CalibrationCollector::collect_snapshot(&src, CalibrationThresholds::default()).unwrap();
        assert_eq!(snap.signals.stale_dominance.status, SignalStatus::Alert);
    }

    #[test]
    fn watch_when_hygiene_lag_above_24h() {
        let mut metrics = metrics();
        metrics.last_hygiene_run_at = Some(Utc::now() - chrono::Duration::hours(30));
        let src = StubSource { metrics };
        let snap =
            CalibrationCollector::collect_snapshot(&src, CalibrationThresholds::default()).unwrap();
        assert_eq!(snap.signals.hygiene_lag.status, SignalStatus::Watch);
    }

    #[test]
    fn alert_when_hygiene_never_run() {
        let mut metrics = metrics();
        metrics.last_hygiene_run_at = None;
        let src = StubSource { metrics };
        let snap =
            CalibrationCollector::collect_snapshot(&src, CalibrationThresholds::default()).unwrap();
        assert_eq!(snap.signals.hygiene_lag.status, SignalStatus::Alert);
    }

    #[test]
    fn snapshot_carries_timestamps() {
        let src = StubSource { metrics: metrics() };
        let snap =
            CalibrationCollector::collect_snapshot(&src, CalibrationThresholds::default()).unwrap();
        let drift = (Utc::now() - snap.generated_at_utc).num_seconds().abs();
        assert!(drift < 2);
    }
}
