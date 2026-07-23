//! Engine-agnostic runtime perf telemetry surfaced through the object-safe
//! `ModelRuntime` contract for the Section 10.13 operator control panel.
//!
//! MT-014b telemetry data plane: the panel's live perf stats (tokens/sec, VRAM
//! resident, time-since-last-call) are produced from real recorded generation
//! activity, not hardcoded placeholders. Adapters record each completed call
//! into a [`RuntimePerfRecorder`] and expose the derived [`RuntimePerfSnapshot`]
//! through `ModelRuntime::perf_snapshot`. Metrics an engine genuinely cannot
//! expose are returned as a typed, engine-specific unavailable
//! ([`RuntimeVramResidency::NotApplicable`]) rather than an invented zero.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// EMA smoothing factor for the tokens/sec estimate. Matches the llama.cpp
/// perf-stats recorder so both adapters (and the object-safe snapshot) report a
/// consistent, field-tested throughput smoothing.
pub const RUNTIME_PERF_EMA_ALPHA: f64 = 0.25;

/// Truthful live perf snapshot for one loaded model, exposed through the
/// object-safe `ModelRuntime` boundary. `None`/`NotApplicable` variants are
/// honest "no data yet" / "engine cannot expose this" signals, never zeroes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuntimePerfSnapshot {
    /// Total completed generation calls recorded for this model this boot.
    pub total_calls: u64,
    /// Total decode tokens generated across every recorded call.
    pub total_tokens_generated: u64,
    /// EMA decode throughput. `None` until a call with measurable wall time and
    /// at least one generated token has completed.
    pub tokens_per_second: Option<f64>,
    /// Completion timestamp of the most recent recorded call. `None` until one
    /// call has completed.
    pub last_call_at_utc: Option<DateTime<Utc>>,
    /// Device-resident memory for this model, or a typed engine-specific reason
    /// when the active device/build genuinely exposes no VRAM residency.
    pub vram_resident_bytes: RuntimeVramResidency,
}

/// VRAM residency envelope. Device-reported when the engine offloads weights to
/// a GPU that exposes resident bytes; otherwise a typed, engine-specific reason.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RuntimeVramResidency {
    DeviceReported { bytes: u64 },
    NotApplicable { reason: String },
}

impl RuntimeVramResidency {
    /// Report device VRAM when the engine measured a non-zero resident size;
    /// otherwise fall back to the typed engine-specific reason. This keeps the
    /// field honest: a CPU-resident or non-offloaded build reports its real
    /// reason instead of pretending device memory is `0`.
    pub fn from_measured(bytes: u64, unavailable_reason: impl Into<String>) -> Self {
        if bytes > 0 {
            Self::DeviceReported { bytes }
        } else {
            Self::NotApplicable {
                reason: unavailable_reason.into(),
            }
        }
    }
}

/// A single completed call's measured perf inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimePerfCall {
    pub tokens_generated: u64,
    pub gen_eval_ms: u64,
    /// Engine-measured device VRAM residency, or `0` when the engine/device
    /// exposes none (the snapshot then reports a typed reason).
    pub vram_resident_bytes: u64,
    pub completed_at_utc: DateTime<Utc>,
}

/// Engine-agnostic recorder that turns real completed generations into the
/// object-safe [`RuntimePerfSnapshot`]. Reused by the llama.cpp and Candle
/// adapters (and test doubles) so the panel telemetry math has one field-tested
/// implementation.
#[derive(Clone, Debug, Default)]
pub struct RuntimePerfRecorder {
    total_calls: u64,
    total_tokens_generated: u64,
    tokens_per_sec_ema: f64,
    last_call_at_utc: Option<DateTime<Utc>>,
    vram_resident_bytes: u64,
}

impl RuntimePerfRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one completed generation into the running telemetry. Zero-token or
    /// zero-duration calls still advance `total_calls`/`last_call_at_utc` but do
    /// not corrupt the throughput EMA.
    pub fn record_call(&mut self, call: RuntimePerfCall) {
        self.total_calls = self.total_calls.saturating_add(1);
        self.total_tokens_generated = self
            .total_tokens_generated
            .saturating_add(call.tokens_generated);
        self.vram_resident_bytes = call.vram_resident_bytes;
        self.last_call_at_utc = Some(call.completed_at_utc);

        if let Some(sample) = tokens_per_second(call.tokens_generated, call.gen_eval_ms) {
            self.tokens_per_sec_ema = if self.tokens_per_sec_ema == 0.0 {
                sample
            } else {
                (self.tokens_per_sec_ema * (1.0 - RUNTIME_PERF_EMA_ALPHA))
                    + (sample * RUNTIME_PERF_EMA_ALPHA)
            };
        }
    }

    /// Project the recorded activity into the object-safe snapshot. The caller
    /// supplies the engine-specific reason used when no device VRAM residency
    /// has been measured.
    pub fn snapshot(&self, vram_unavailable_reason: impl Into<String>) -> RuntimePerfSnapshot {
        RuntimePerfSnapshot {
            total_calls: self.total_calls,
            total_tokens_generated: self.total_tokens_generated,
            tokens_per_second: (self.total_calls > 0 && self.tokens_per_sec_ema > 0.0)
                .then_some(self.tokens_per_sec_ema),
            last_call_at_utc: self.last_call_at_utc,
            vram_resident_bytes: RuntimeVramResidency::from_measured(
                self.vram_resident_bytes,
                vram_unavailable_reason,
            ),
        }
    }
}

fn tokens_per_second(tokens_generated: u64, gen_eval_ms: u64) -> Option<f64> {
    if tokens_generated == 0 || gen_eval_ms == 0 {
        return None;
    }
    let sample = (tokens_generated as f64) / (gen_eval_ms as f64 / 1_000.0);
    sample.is_finite().then_some(sample)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(secs, 0).single().expect("valid utc")
    }

    #[test]
    fn empty_recorder_reports_no_generation_and_typed_vram_reason() {
        let snapshot = RuntimePerfRecorder::new().snapshot("candle CPU device holds no VRAM");
        assert_eq!(snapshot.total_calls, 0);
        assert_eq!(snapshot.tokens_per_second, None);
        assert_eq!(snapshot.last_call_at_utc, None);
        assert_eq!(
            snapshot.vram_resident_bytes,
            RuntimeVramResidency::NotApplicable {
                reason: "candle CPU device holds no VRAM".to_owned()
            }
        );
    }

    #[test]
    fn recorded_generation_surfaces_real_throughput_and_last_call() {
        let mut recorder = RuntimePerfRecorder::new();
        // 40 tokens over 100 ms == 400 tokens/sec.
        recorder.record_call(RuntimePerfCall {
            tokens_generated: 40,
            gen_eval_ms: 100,
            vram_resident_bytes: 0,
            completed_at_utc: at(1_000),
        });
        let snapshot = recorder.snapshot("no device VRAM");
        assert_eq!(snapshot.total_calls, 1);
        assert_eq!(snapshot.total_tokens_generated, 40);
        assert_eq!(snapshot.tokens_per_second, Some(400.0));
        assert_eq!(snapshot.last_call_at_utc, Some(at(1_000)));
    }

    #[test]
    fn device_reported_vram_is_surfaced_when_measured() {
        let mut recorder = RuntimePerfRecorder::new();
        recorder.record_call(RuntimePerfCall {
            tokens_generated: 10,
            gen_eval_ms: 50,
            vram_resident_bytes: 2_147_483_648,
            completed_at_utc: at(2_000),
        });
        let snapshot = recorder.snapshot("unused reason");
        assert_eq!(
            snapshot.vram_resident_bytes,
            RuntimeVramResidency::DeviceReported {
                bytes: 2_147_483_648
            }
        );
    }

    #[test]
    fn zero_duration_call_advances_last_call_without_polluting_throughput() {
        let mut recorder = RuntimePerfRecorder::new();
        recorder.record_call(RuntimePerfCall {
            tokens_generated: 5,
            gen_eval_ms: 0,
            vram_resident_bytes: 0,
            completed_at_utc: at(3_000),
        });
        let snapshot = recorder.snapshot("no device VRAM");
        assert_eq!(snapshot.total_calls, 1);
        assert_eq!(snapshot.last_call_at_utc, Some(at(3_000)));
        // No measurable duration means no honest throughput sample.
        assert_eq!(snapshot.tokens_per_second, None);
    }
}
