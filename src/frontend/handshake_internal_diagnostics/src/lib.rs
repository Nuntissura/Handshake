pub mod operation_watchdog;

use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticMechanism {
    Heartbeat,
    FrameTime,
    ResourceSampler,
    BackendRoute,
    GuiAction,
    MechanicalJob,
    Panic,
    RingBuffer,
    Palmistry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticEventState {
    Started,
    Healthy,
    Degraded,
    Failed,
    Recovered,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    BackendUnavailable,
    BackendRecovered,
    FrameSlow,
    ResourceSampleUnavailable,
    PanicObserved,
    RingPublishFailed,
    WatcherUnavailable,
    WatcherRecoveredRecord,
}

/// The open diagnostic-event API accepts only these mechanical fields. There
/// is deliberately no free-form payload, project id, path, prompt, or content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalDiagnosticEvent {
    pub event_id: Uuid,
    pub observed_at_unix_ms: u64,
    pub mechanism: DiagnosticMechanism,
    pub state: DiagnosticEventState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<DiagnosticCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_micros: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counter: Option<u64>,
}

impl InternalDiagnosticEvent {
    pub fn mechanical(
        mechanism: DiagnosticMechanism,
        state: DiagnosticEventState,
        code: Option<DiagnosticCode>,
    ) -> Self {
        Self {
            event_id: Uuid::now_v7(),
            observed_at_unix_ms: unix_ms(),
            mechanism,
            state,
            code,
            duration_micros: None,
            counter: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameTimeStats {
    pub sample_count: u64,
    pub last_micros: u64,
    pub min_micros: u64,
    pub max_micros: u64,
    pub p50_micros: u64,
    pub p95_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceCounters {
    pub sampled_at_unix_ms: u64,
    pub cpu_percent: Option<f32>,
    pub rss_bytes: Option<u64>,
    pub gpu_percent: Option<f32>,
    pub gpu_status: ResourceMetricStatus,
}

impl Default for ResourceCounters {
    fn default() -> Self {
        Self {
            sampled_at_unix_ms: 0,
            cpu_percent: None,
            rss_bytes: None,
            gpu_percent: None,
            gpu_status: ResourceMetricStatus::Unavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceMetricStatus {
    Sampled,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalDiagnosticsSnapshot {
    pub schema_id: String,
    pub ring_version: u32,
    pub session_id: Uuid,
    pub launch_nonce: Uuid,
    pub process_id: u32,
    pub build_id: String,
    pub heartbeat_counter: u64,
    pub heartbeat_unix_ms: u64,
    pub heartbeat_monotonic_ms: u64,
    pub frame_time: FrameTimeStats,
    pub resources: ResourceCounters,
    pub events: Vec<InternalDiagnosticEvent>,
    pub behavior_observations: Vec<BehaviorObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorObservation {
    pub mechanism: String,
    pub heartbeat_counter: u64,
    pub observed_at_unix_ms: u64,
    pub correlation_hmac: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanicPayloadClass {
    String,
    StaticStr,
    Opaque,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableCrashRecord {
    pub schema_id: String,
    pub crash_id: Uuid,
    pub session_id: Uuid,
    pub process_id: u32,
    pub observed_at_unix_ms: u64,
    pub build_id: String,
    pub payload_class: PanicPayloadClass,
    pub location_file_sha256: Option<String>,
    pub location_line: Option<u32>,
    pub location_column: Option<u32>,
    pub redacted_backtrace: String,
    pub last_events: Vec<InternalDiagnosticEvent>,
}

pub fn frame_stats(samples: &VecDeque<u64>) -> FrameTimeStats {
    if samples.is_empty() {
        return FrameTimeStats::default();
    }
    let mut ordered = samples.iter().copied().collect::<Vec<_>>();
    ordered.sort_unstable();
    let percentile = |percent: usize| {
        let index = ((ordered.len() - 1) * percent) / 100;
        ordered[index]
    };
    FrameTimeStats {
        sample_count: samples.len() as u64,
        last_micros: *samples.back().unwrap_or(&0),
        min_micros: ordered[0],
        max_micros: *ordered.last().unwrap_or(&0),
        p50_micros: percentile(50),
        p95_micros: percentile(95),
    }
}

pub fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_stats_are_bounded_and_percentile_based() {
        let samples = VecDeque::from([1, 100, 3, 4, 5]);
        let stats = frame_stats(&samples);
        assert_eq!(stats.sample_count, 5);
        assert_eq!(stats.last_micros, 5);
        assert_eq!(stats.min_micros, 1);
        assert_eq!(stats.max_micros, 100);
        assert_eq!(stats.p50_micros, 4);
        assert_eq!(stats.p95_micros, 5);
    }
}
