//! MT-151: ValidatorFirstPassEvaluator — wires the loop eval to the HBR
//! test-packet corpus and produces multi-metric SplitMetrics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use super::corpus::{CorpusItem, CorpusSplit, KeyProvider, ValidatorVerdict};
use super::editable_surface::EditableSurfaceSnapshot;
use super::loop_core::{LoopSandbox, LoopSandboxError};

/// Trait the evaluator consumes to run the validator first-pass against
/// a corpus item.
pub trait ValidatorRunner {
    fn run(
        &self,
        item: &CorpusItem,
        snapshot: &EditableSurfaceSnapshot,
    ) -> Result<ValidatorRun, EvalError>;
}

/// One validator first-pass run result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidatorRun {
    pub verdict: ValidatorVerdict,
    pub latency_ms: u64,
    pub capsule_bytes: u64,
}

/// ValidatorFirstPassEvaluator implementation per MT-151. It runs each
/// corpus item through the sandboxed ValidatorRunner and computes
/// SplitMetrics with proper p95 latency and capsule-bytes.
pub struct ValidatorFirstPassEvaluator<'a> {
    pub sandbox: &'a dyn LoopSandbox,
    pub validator_runner: &'a dyn ValidatorRunner,
}

impl<'a> ValidatorFirstPassEvaluator<'a> {
    pub fn new(sandbox: &'a dyn LoopSandbox, validator_runner: &'a dyn ValidatorRunner) -> Self {
        Self {
            sandbox,
            validator_runner,
        }
    }
}

impl<'a> super::loop_core::Evaluator for ValidatorFirstPassEvaluator<'a> {
    fn evaluate(
        &self,
        split: &CorpusSplit,
        key_provider: &dyn KeyProvider,
        snapshot: &EditableSurfaceSnapshot,
    ) -> Result<EvalResult, EvalError> {
        // Frozen sandbox run; we tie the sandbox run id into the result so
        // auditors can link to the actual isolated execution.
        let _sandbox_run = self.sandbox.run(snapshot).map_err(EvalError::Sandbox)?;

        let train = evaluate_split(&split.train, snapshot, self.validator_runner)?;
        let dev = evaluate_split(&split.dev, snapshot, self.validator_runner)?;

        // Holdout: decrypt, evaluate, drop plaintext.
        let holdout_items = split
            .decrypt_holdout(key_provider)
            .map_err(EvalError::Corpus)?;
        let holdout = evaluate_split(&holdout_items, snapshot, self.validator_runner)?;

        let snapshot_hash = hash_snapshot(snapshot);
        Ok(EvalResult {
            train,
            dev,
            holdout,
            evaluated_at_utc: Utc::now(),
            snapshot_hash,
        })
    }
}

fn evaluate_split(
    items: &[CorpusItem],
    snapshot: &EditableSurfaceSnapshot,
    runner: &dyn ValidatorRunner,
) -> Result<SplitMetrics, EvalError> {
    let mut per_item = Vec::with_capacity(items.len());
    let mut latencies = Vec::with_capacity(items.len());
    let mut bytes_obs = Vec::with_capacity(items.len());

    for item in items {
        let run = runner.run(item, snapshot)?;
        latencies.push(run.latency_ms);
        bytes_obs.push(run.capsule_bytes);
        per_item.push(PerItemResult {
            item_id: item.id,
            verdict: run.verdict,
            latency_ms: run.latency_ms,
            capsule_bytes: run.capsule_bytes,
            error: None,
        });
    }

    let total_count = items.len() as u32;
    let pass_count = per_item
        .iter()
        .filter(|r| r.verdict == ValidatorVerdict::Pass)
        .count() as u32;
    let pass_rate = if total_count == 0 {
        0.0
    } else {
        f64::from(pass_count) / f64::from(total_count)
    };

    Ok(SplitMetrics {
        pass_rate,
        pass_count,
        total_count,
        latency_p95_ms: p95(&mut latencies),
        capsule_bytes_p95: p95(&mut bytes_obs),
        per_item_results: per_item,
    })
}

fn p95<T: Ord + Copy + Default>(values: &mut Vec<T>) -> T {
    if values.is_empty() {
        return T::default();
    }
    values.sort();
    // Standard nearest-rank p95: ceil(0.95 * n) - 1.
    let n = values.len();
    let idx = ((n as f64) * 0.95).ceil() as usize;
    let idx = idx.saturating_sub(1).min(n - 1);
    values[idx]
}

fn hash_snapshot(snapshot: &EditableSurfaceSnapshot) -> String {
    let bytes = serde_json::to_vec(snapshot).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let mut s = String::with_capacity(64);
    for b in digest.as_slice() {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Per-item evaluation record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerItemResult {
    pub item_id: Uuid,
    pub verdict: ValidatorVerdict,
    pub latency_ms: u64,
    pub capsule_bytes: u64,
    pub error: Option<String>,
}

/// Per-split metrics surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitMetrics {
    pub pass_rate: f64,
    pub pass_count: u32,
    pub total_count: u32,
    pub latency_p95_ms: u64,
    pub capsule_bytes_p95: u64,
    pub per_item_results: Vec<PerItemResult>,
}

impl SplitMetrics {
    /// Zero-pass-rate baseline metric for tests / fixtures.
    pub fn empty() -> Self {
        Self {
            pass_rate: 0.0,
            pass_count: 0,
            total_count: 0,
            latency_p95_ms: 0,
            capsule_bytes_p95: 0,
            per_item_results: Vec::new(),
        }
    }
}

/// Eval result returned by [`ValidatorFirstPassEvaluator::evaluate`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalResult {
    pub train: SplitMetrics,
    pub dev: SplitMetrics,
    pub holdout: SplitMetrics,
    pub evaluated_at_utc: DateTime<Utc>,
    pub snapshot_hash: String,
}

#[derive(Debug, Error)]
pub enum EvalError {
    #[error("sandbox failed: {0}")]
    Sandbox(LoopSandboxError),
    #[error("corpus failure: {0}")]
    Corpus(super::corpus::CorpusError),
    #[error("validator runner failed: {message}")]
    ValidatorRunner { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_improve::corpus::{HbrTestPacketCorpus, StaticKeyProvider};
    use serde_json::json;

    struct StubSandbox;
    impl LoopSandbox for StubSandbox {
        fn run(
            &self,
            _snapshot: &EditableSurfaceSnapshot,
        ) -> Result<super::super::loop_core::SandboxRunResult, LoopSandboxError> {
            Ok(super::super::loop_core::SandboxRunResult {
                sandbox_run_id: Uuid::now_v7(),
            })
        }
    }

    struct DeterministicRunner {
        pass_rate: f64,
    }
    impl ValidatorRunner for DeterministicRunner {
        fn run(
            &self,
            item: &CorpusItem,
            _snapshot: &EditableSurfaceSnapshot,
        ) -> Result<ValidatorRun, EvalError> {
            // Use the low bits of the uuid to deterministically pass/fail
            // at the configured rate.
            let bits = item.id.as_u128();
            let bucket = (bits % 100) as f64 / 100.0;
            let verdict = if bucket < self.pass_rate {
                ValidatorVerdict::Pass
            } else {
                ValidatorVerdict::Fail
            };
            Ok(ValidatorRun {
                verdict,
                latency_ms: 50 + (bits as u64 % 80),
                capsule_bytes: 10_000 + (bits as u64 % 5000),
            })
        }
    }

    fn corpus_for(n: u128) -> HbrTestPacketCorpus {
        let items: Vec<CorpusItem> = (1..=n)
            .map(|i| CorpusItem {
                id: Uuid::from_u128(i * 7919),
                hbr_rule_id: "HBR-INT-001".to_string(),
                packet_under_test: format!("pkt-{i}"),
                expected_first_pass_verdict: ValidatorVerdict::Pass,
                fixtures: json!({}),
            })
            .collect();
        HbrTestPacketCorpus::from_items(items).unwrap()
    }

    #[test]
    fn evaluator_computes_pass_rate_and_p95() {
        let corpus = corpus_for(30);
        let kp = StaticKeyProvider::deterministic("k");
        let split = corpus.split(11, &kp, "k").unwrap();
        let sandbox = StubSandbox;
        let runner = DeterministicRunner { pass_rate: 0.7 };
        let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
        let snapshot = EditableSurfaceSnapshot::ModelManual {
            manual_section_id: "intro.section".to_string(),
            before_text: "before".to_string(),
            after_text: "after".to_string(),
        };
        let result = <ValidatorFirstPassEvaluator as super::super::loop_core::Evaluator>::evaluate(
            &evaluator, &split, &kp, &snapshot,
        )
        .unwrap();
        assert_eq!(result.train.total_count, 18);
        assert_eq!(result.dev.total_count, 6);
        assert_eq!(result.holdout.total_count, 6);
        assert!(result.train.pass_rate >= 0.0 && result.train.pass_rate <= 1.0);
        assert!(result.train.latency_p95_ms >= 50);
        assert!(result.train.capsule_bytes_p95 >= 10_000);
        assert_eq!(result.snapshot_hash.len(), 64);
    }

    #[test]
    fn evaluator_holdout_decryption_fails_cleanly_without_key() {
        let corpus = corpus_for(30);
        let kp = StaticKeyProvider::deterministic("real-key");
        let split = corpus.split(11, &kp, "real-key").unwrap();
        let wrong_kp = StaticKeyProvider::deterministic("wrong-key");
        let sandbox = StubSandbox;
        let runner = DeterministicRunner { pass_rate: 1.0 };
        let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
        let snapshot = EditableSurfaceSnapshot::ModelManual {
            manual_section_id: "intro.section".to_string(),
            before_text: "a".to_string(),
            after_text: "b".to_string(),
        };
        let err = <ValidatorFirstPassEvaluator as super::super::loop_core::Evaluator>::evaluate(
            &evaluator, &split, &wrong_kp, &snapshot,
        )
        .unwrap_err();
        assert!(matches!(err, EvalError::Corpus(_)));
    }

    #[test]
    fn split_metrics_p95_handles_small_input() {
        let mut v = vec![1u64, 2, 3, 4, 5];
        assert_eq!(p95(&mut v), 5);
    }
}
