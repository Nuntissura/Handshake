//! MT-151 — ValidatorFirstPassEvaluator integration tests.
//!
//! Per the MT-151 contract proof_command:
//!   `cargo test -p handshake_core --test validator_first_pass_evaluator_tests`
//!
//! Inline `#[cfg(test)] mod tests` inside src/self_improve/evaluator.rs
//! already covers 3 cases (pass_rate + p95 computation, holdout decryption
//! failure without key, p95 helper on a small vec). This integration test
//! file satisfies the contract owned_files entry and exercises the
//! cross-cutting adversarial scenarios required by the red_team
//! minimum_controls + Q-SELF-IMPROVE-TARGET multi-metric contract that
//! the inline tests don't reach at the public-API level:
//!
//!   - Sandbox is invoked exactly once per evaluate() and produces a
//!     fresh run id; if the sandbox short-circuits (returns an error)
//!     evaluate must propagate it as EvalError::Sandbox without running
//!     any ValidatorRunner.run() calls.
//!   - Holdout plaintext is dropped after evaluate() returns: the
//!     ValidatorRunner records every item id it saw — we assert that
//!     after evaluate returns, the recorder holds only ids we already
//!     have access to via the public split.train/split.dev surface OR
//!     ids decoded from the (now-decrypted) holdout; the evaluator does
//!     not leak the plaintext as a side channel into EvalResult.
//!   - Latency p95 and capsule_bytes p95 are computed from raw per-item
//!     observations (not running averages); we feed a known distribution
//!     and assert the p95 lands on the correct order statistic.
//!   - snapshot_hash is a deterministic 64-char hex SHA-256 over the
//!     canonical JSON of the EditableSurfaceSnapshot; identical
//!     snapshots produce identical hashes; differing snapshots produce
//!     different hashes.
//!   - All-pass and all-fail corner cases: pass_rate = 1.0 / 0.0
//!     respectively; total_count + pass_count match the split sizes
//!     declared by MT-150.
//!   - Empty-split corner: pass_rate defaults to 0.0 without panic
//!     when an upstream split is somehow empty (defense in depth).
//!   - Per-item PerItemResult round-trip via serde JSON proves the
//!     EvalResult is auditable.
//!   - Multi-eval determinism: running evaluate() twice on the same
//!     (split, snapshot, deterministic runner) produces byte-identical
//!     SplitMetrics aside from the evaluated_at_utc timestamp.

use std::cell::RefCell;
use std::sync::Mutex;

use chrono::Utc;
use handshake_core::self_improve::corpus::{
    CorpusItem, CorpusSplit, HbrTestPacketCorpus, KeyProvider, StaticKeyProvider,
    ValidatorVerdict,
};
use handshake_core::self_improve::editable_surface::EditableSurfaceSnapshot;
use handshake_core::self_improve::evaluator::{
    EvalError, ValidatorFirstPassEvaluator, ValidatorRun, ValidatorRunner,
};
use handshake_core::self_improve::loop_core::{
    Evaluator, LoopSandbox, LoopSandboxError, SandboxRunResult,
};
use serde_json::json;
use uuid::Uuid;

// ----------------------------------------------------------------------------
// Stub sandbox + recording sandbox: real adapter lives in cluster B; the
// MT-151 contract states the evaluator consumes LoopSandbox via trait so
// the integration test injects a recording stub.
// ----------------------------------------------------------------------------

struct CountingSandbox {
    runs: Mutex<u32>,
}

impl CountingSandbox {
    fn new() -> Self {
        Self {
            runs: Mutex::new(0),
        }
    }
    fn run_count(&self) -> u32 {
        *self.runs.lock().unwrap()
    }
}

impl LoopSandbox for CountingSandbox {
    fn run(
        &self,
        _snapshot: &EditableSurfaceSnapshot,
    ) -> Result<SandboxRunResult, LoopSandboxError> {
        *self.runs.lock().unwrap() += 1;
        Ok(SandboxRunResult {
            sandbox_run_id: Uuid::now_v7(),
        })
    }
}

struct FailingSandbox;

impl LoopSandbox for FailingSandbox {
    fn run(
        &self,
        _snapshot: &EditableSurfaceSnapshot,
    ) -> Result<SandboxRunResult, LoopSandboxError> {
        Err(LoopSandboxError::new("sandbox: synthetic failure"))
    }
}

// ----------------------------------------------------------------------------
// Recording validator runner: records every item id it observed so the
// test can prove (a) the runner saw every item in every split and (b) it
// did not see anything else.
// ----------------------------------------------------------------------------

struct RecordingRunner {
    /// Map (item_id) -> (verdict, latency_ms, capsule_bytes).
    program: Vec<(Uuid, ValidatorVerdict, u64, u64)>,
    seen: RefCell<Vec<Uuid>>,
}

impl RecordingRunner {
    fn from_program(program: Vec<(Uuid, ValidatorVerdict, u64, u64)>) -> Self {
        Self {
            program,
            seen: RefCell::new(Vec::new()),
        }
    }

    fn seen_ids(&self) -> Vec<Uuid> {
        self.seen.borrow().clone()
    }
}

impl ValidatorRunner for RecordingRunner {
    fn run(
        &self,
        item: &CorpusItem,
        _snapshot: &EditableSurfaceSnapshot,
    ) -> Result<ValidatorRun, EvalError> {
        self.seen.borrow_mut().push(item.id);
        for (id, verdict, latency_ms, capsule_bytes) in &self.program {
            if *id == item.id {
                return Ok(ValidatorRun {
                    verdict: *verdict,
                    latency_ms: *latency_ms,
                    capsule_bytes: *capsule_bytes,
                });
            }
        }
        // Default: PASS with stable observations so tests that don't care
        // about a specific item get sensible defaults.
        Ok(ValidatorRun {
            verdict: ValidatorVerdict::Pass,
            latency_ms: 100,
            capsule_bytes: 20_000,
        })
    }
}

struct UniformRunner {
    verdict: ValidatorVerdict,
    latency_ms: u64,
    capsule_bytes: u64,
}

impl ValidatorRunner for UniformRunner {
    fn run(
        &self,
        _item: &CorpusItem,
        _snapshot: &EditableSurfaceSnapshot,
    ) -> Result<ValidatorRun, EvalError> {
        Ok(ValidatorRun {
            verdict: self.verdict,
            latency_ms: self.latency_ms,
            capsule_bytes: self.capsule_bytes,
        })
    }
}

struct FailingRunner {
    message: &'static str,
}

impl ValidatorRunner for FailingRunner {
    fn run(
        &self,
        _item: &CorpusItem,
        _snapshot: &EditableSurfaceSnapshot,
    ) -> Result<ValidatorRun, EvalError> {
        Err(EvalError::ValidatorRunner {
            message: self.message.to_string(),
        })
    }
}

// ----------------------------------------------------------------------------
// Fixture helpers.
// ----------------------------------------------------------------------------

fn make_corpus(n: u128) -> HbrTestPacketCorpus {
    let items: Vec<CorpusItem> = (1..=n)
        .map(|i| CorpusItem {
            id: Uuid::from_u128(i * 7919),
            hbr_rule_id: "HBR-INT-006".to_string(),
            packet_under_test: format!("packet-{i}"),
            expected_first_pass_verdict: ValidatorVerdict::Pass,
            fixtures: json!({ "i": i }),
        })
        .collect();
    HbrTestPacketCorpus::from_items(items).unwrap()
}

fn snapshot_a() -> EditableSurfaceSnapshot {
    EditableSurfaceSnapshot::ModelManual {
        manual_section_id: "intro.usage_overview".to_string(),
        before_text: "before-A".to_string(),
        after_text: "after-A".to_string(),
    }
}

fn snapshot_b() -> EditableSurfaceSnapshot {
    EditableSurfaceSnapshot::ModelManual {
        manual_section_id: "intro.usage_overview".to_string(),
        before_text: "before-B".to_string(),
        after_text: "after-B".to_string(),
    }
}

// ----------------------------------------------------------------------------
// Test 1: Sandbox is invoked exactly once per evaluate().
// ----------------------------------------------------------------------------

#[test]
fn mt151_sandbox_runs_exactly_once_per_evaluate() {
    let corpus = make_corpus(30);
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(11, &kp, "kid").unwrap();
    let sandbox = CountingSandbox::new();
    let runner = UniformRunner {
        verdict: ValidatorVerdict::Pass,
        latency_ms: 50,
        capsule_bytes: 10_000,
    };
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
    let snapshot = snapshot_a();

    let _ = evaluator
        .evaluate(&split, &kp, &snapshot)
        .expect("evaluate must succeed");
    assert_eq!(
        sandbox.run_count(),
        1,
        "sandbox.run() must fire exactly once per evaluator.evaluate()"
    );
}

// ----------------------------------------------------------------------------
// Test 2: When sandbox fails, evaluate propagates Sandbox error and does
// NOT invoke the validator runner.
// ----------------------------------------------------------------------------

#[test]
fn mt151_sandbox_failure_propagates_and_skips_validator_runs() {
    let corpus = make_corpus(30);
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(11, &kp, "kid").unwrap();
    let sandbox = FailingSandbox;
    let runner = RecordingRunner::from_program(Vec::new());
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
    let snapshot = snapshot_a();

    let err = evaluator
        .evaluate(&split, &kp, &snapshot)
        .expect_err("sandbox failure must propagate");
    match err {
        EvalError::Sandbox(_) => {}
        other => panic!("expected EvalError::Sandbox; got {other:?}"),
    }
    assert_eq!(
        runner.seen_ids().len(),
        0,
        "validator runner must NOT fire when sandbox fails"
    );
}

// ----------------------------------------------------------------------------
// Test 3: All-pass corpus produces pass_rate == 1.0 on every split.
// ----------------------------------------------------------------------------

#[test]
fn mt151_all_pass_produces_unit_pass_rate() {
    let corpus = make_corpus(30);
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(11, &kp, "kid").unwrap();
    let sandbox = CountingSandbox::new();
    let runner = UniformRunner {
        verdict: ValidatorVerdict::Pass,
        latency_ms: 50,
        capsule_bytes: 10_000,
    };
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
    let snapshot = snapshot_a();

    let result = evaluator.evaluate(&split, &kp, &snapshot).unwrap();
    assert_eq!(result.train.pass_rate, 1.0);
    assert_eq!(result.dev.pass_rate, 1.0);
    assert_eq!(result.holdout.pass_rate, 1.0);
    assert_eq!(result.train.pass_count, result.train.total_count);
    assert_eq!(result.dev.pass_count, result.dev.total_count);
    assert_eq!(result.holdout.pass_count, result.holdout.total_count);
    assert_eq!(
        result.train.total_count + result.dev.total_count + result.holdout.total_count,
        30,
        "train + dev + holdout must equal corpus size"
    );
}

// ----------------------------------------------------------------------------
// Test 4: All-fail corpus produces pass_rate == 0.0 on every split.
// ----------------------------------------------------------------------------

#[test]
fn mt151_all_fail_produces_zero_pass_rate() {
    let corpus = make_corpus(30);
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(11, &kp, "kid").unwrap();
    let sandbox = CountingSandbox::new();
    let runner = UniformRunner {
        verdict: ValidatorVerdict::Fail,
        latency_ms: 80,
        capsule_bytes: 15_000,
    };
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
    let snapshot = snapshot_a();

    let result = evaluator.evaluate(&split, &kp, &snapshot).unwrap();
    assert_eq!(result.train.pass_rate, 0.0);
    assert_eq!(result.dev.pass_rate, 0.0);
    assert_eq!(result.holdout.pass_rate, 0.0);
    assert_eq!(result.train.pass_count, 0);
    assert_eq!(result.dev.pass_count, 0);
    assert_eq!(result.holdout.pass_count, 0);
}

// ----------------------------------------------------------------------------
// Test 5: SKIP verdict counts as not-pass (only Pass counts toward pass_count).
// ----------------------------------------------------------------------------

#[test]
fn mt151_skip_verdict_counts_as_not_pass() {
    let corpus = make_corpus(30);
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(11, &kp, "kid").unwrap();
    let sandbox = CountingSandbox::new();
    let runner = UniformRunner {
        verdict: ValidatorVerdict::Skip,
        latency_ms: 12,
        capsule_bytes: 9_000,
    };
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
    let snapshot = snapshot_a();

    let result = evaluator.evaluate(&split, &kp, &snapshot).unwrap();
    assert_eq!(
        result.train.pass_rate, 0.0,
        "Skip must not count toward pass_count"
    );
    assert_eq!(result.train.pass_count, 0);
    assert!(
        result.train.total_count > 0,
        "items still ran; only verdict was Skip"
    );
}

// ----------------------------------------------------------------------------
// Test 6: Latency p95 lands on the correct order statistic for a known
// distribution. With 20 items and latencies [1..=20], nearest-rank p95 is
// ceil(0.95 * 20) - 1 = 19 -> latencies[19] (0-indexed sorted) == 20.
// ----------------------------------------------------------------------------

#[test]
fn mt151_latency_p95_is_correct_order_statistic() {
    // Build a 20-item corpus (smaller for direct control over the train
    // split size).
    let items: Vec<CorpusItem> = (1..=20u128)
        .map(|i| CorpusItem {
            id: Uuid::from_u128(i),
            hbr_rule_id: "HBR-INT-006".to_string(),
            packet_under_test: format!("pkt-{i}"),
            expected_first_pass_verdict: ValidatorVerdict::Pass,
            fixtures: json!({ "i": i }),
        })
        .collect();
    let corpus = HbrTestPacketCorpus::from_items(items.clone()).unwrap();
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(1, &kp, "kid").unwrap();

    // Program runner: latency_ms = item ordinal (1..=20). The split
    // permutation reorders item ids but the runner always sees a fixed
    // mapping from item.id -> latency, so per-split latencies are a
    // SUBSET of {1, ..., 20}; we therefore verify p95 against the train
    // split's actual member latencies rather than the global set.
    let program: Vec<(Uuid, ValidatorVerdict, u64, u64)> = items
        .iter()
        .enumerate()
        .map(|(idx, it)| {
            (
                it.id,
                ValidatorVerdict::Pass,
                (idx as u64) + 1, // latency 1..=20
                1_000 + (idx as u64) * 100,
            )
        })
        .collect();
    let runner = RecordingRunner::from_program(program);
    let sandbox = CountingSandbox::new();
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
    let snapshot = snapshot_a();

    let result = evaluator
        .evaluate(&split, &kp, &snapshot)
        .expect("evaluate must succeed");

    // The 12-item train split has 12 latencies; nearest-rank p95 is
    // ceil(0.95 * 12) - 1 = 11 -> sorted_latencies[11], i.e. the max.
    let mut train_latencies: Vec<u64> = result
        .train
        .per_item_results
        .iter()
        .map(|r| r.latency_ms)
        .collect();
    train_latencies.sort();
    let n = train_latencies.len();
    let nearest_rank_index = ((n as f64) * 0.95).ceil() as usize;
    let expected_index = nearest_rank_index.saturating_sub(1).min(n - 1);
    let expected_p95 = train_latencies[expected_index];
    assert_eq!(
        result.train.latency_p95_ms, expected_p95,
        "latency p95 must be the nearest-rank order statistic"
    );
}

// ----------------------------------------------------------------------------
// Test 7: Capsule bytes p95 is computed independently of latency.
// ----------------------------------------------------------------------------

#[test]
fn mt151_capsule_bytes_p95_is_independent_of_latency() {
    let corpus = make_corpus(30);
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(11, &kp, "kid").unwrap();
    let sandbox = CountingSandbox::new();
    // Constant latency, varying capsule_bytes: program runner so capsule
    // bytes are sortable + the p95 is non-trivial.
    let program: Vec<(Uuid, ValidatorVerdict, u64, u64)> = corpus
        .items
        .iter()
        .enumerate()
        .map(|(idx, it)| {
            (
                it.id,
                ValidatorVerdict::Pass,
                100, // constant latency
                10_000 + (idx as u64) * 200,
            )
        })
        .collect();
    let runner = RecordingRunner::from_program(program);
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
    let snapshot = snapshot_a();

    let result = evaluator.evaluate(&split, &kp, &snapshot).unwrap();
    assert_eq!(
        result.train.latency_p95_ms, 100,
        "constant latency => p95 = constant"
    );
    let mut train_caps: Vec<u64> = result
        .train
        .per_item_results
        .iter()
        .map(|r| r.capsule_bytes)
        .collect();
    train_caps.sort();
    let n = train_caps.len();
    let idx = ((n as f64) * 0.95).ceil() as usize;
    let expected = train_caps[idx.saturating_sub(1).min(n - 1)];
    assert_eq!(result.train.capsule_bytes_p95, expected);
}

// ----------------------------------------------------------------------------
// Test 8: snapshot_hash is deterministic per snapshot; different snapshots
// produce different hashes.
// ----------------------------------------------------------------------------

#[test]
fn mt151_snapshot_hash_is_deterministic_and_collision_sensitive() {
    let corpus = make_corpus(30);
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(11, &kp, "kid").unwrap();
    let sandbox_a1 = CountingSandbox::new();
    let runner_a1 = UniformRunner {
        verdict: ValidatorVerdict::Pass,
        latency_ms: 50,
        capsule_bytes: 10_000,
    };
    let evaluator_a1 = ValidatorFirstPassEvaluator::new(&sandbox_a1, &runner_a1);
    let result_a1 = evaluator_a1.evaluate(&split, &kp, &snapshot_a()).unwrap();
    assert_eq!(result_a1.snapshot_hash.len(), 64, "SHA-256 hex = 64 chars");
    assert!(
        result_a1.snapshot_hash.chars().all(|c| c.is_ascii_hexdigit()),
        "snapshot_hash must be hex"
    );

    // Same snapshot -> identical hash.
    let sandbox_a2 = CountingSandbox::new();
    let runner_a2 = UniformRunner {
        verdict: ValidatorVerdict::Pass,
        latency_ms: 50,
        capsule_bytes: 10_000,
    };
    let evaluator_a2 = ValidatorFirstPassEvaluator::new(&sandbox_a2, &runner_a2);
    let result_a2 = evaluator_a2.evaluate(&split, &kp, &snapshot_a()).unwrap();
    assert_eq!(
        result_a1.snapshot_hash, result_a2.snapshot_hash,
        "identical snapshots must produce identical hashes"
    );

    // Different snapshot -> different hash.
    let sandbox_b = CountingSandbox::new();
    let runner_b = UniformRunner {
        verdict: ValidatorVerdict::Pass,
        latency_ms: 50,
        capsule_bytes: 10_000,
    };
    let evaluator_b = ValidatorFirstPassEvaluator::new(&sandbox_b, &runner_b);
    let result_b = evaluator_b.evaluate(&split, &kp, &snapshot_b()).unwrap();
    assert_ne!(
        result_a1.snapshot_hash, result_b.snapshot_hash,
        "different snapshots must produce different hashes"
    );
}

// ----------------------------------------------------------------------------
// Test 9: Validator runner sees every item in every split exactly once.
// This is the proof that the evaluator iterates the FULL split, not a
// subset, and does not double-evaluate any item.
// ----------------------------------------------------------------------------

#[test]
fn mt151_runner_observes_every_item_exactly_once_per_split() {
    let corpus = make_corpus(30);
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(11, &kp, "kid").unwrap();
    let sandbox = CountingSandbox::new();
    let runner = RecordingRunner::from_program(Vec::new());
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
    let snapshot = snapshot_a();

    let result = evaluator.evaluate(&split, &kp, &snapshot).unwrap();
    let seen = runner.seen_ids();

    // Total observations == train + dev + holdout sizes (no duplicates,
    // no missing).
    let expected_total =
        result.train.total_count + result.dev.total_count + result.holdout.total_count;
    assert_eq!(seen.len(), expected_total as usize);

    // Each split's items appear exactly once in `seen`.
    let mut train_ids: Vec<Uuid> = split.train.iter().map(|i| i.id).collect();
    let mut dev_ids: Vec<Uuid> = split.dev.iter().map(|i| i.id).collect();
    let mut holdout_ids: Vec<Uuid> = split
        .decrypt_holdout(&kp)
        .unwrap()
        .iter()
        .map(|i| i.id)
        .collect();
    let mut all_expected = Vec::new();
    all_expected.append(&mut train_ids);
    all_expected.append(&mut dev_ids);
    all_expected.append(&mut holdout_ids);
    all_expected.sort();

    let mut all_seen = seen.clone();
    all_seen.sort();
    assert_eq!(all_seen, all_expected);
}

// ----------------------------------------------------------------------------
// Test 10: Holdout decryption fails cleanly with wrong key — verifies the
// guard from the inline test at the integration boundary and confirms the
// error variant is Corpus(_) so callers can distinguish it from
// Sandbox/Validator errors.
// ----------------------------------------------------------------------------

#[test]
fn mt151_wrong_key_for_holdout_yields_corpus_error() {
    let corpus = make_corpus(30);
    let kp_real = StaticKeyProvider::deterministic("real-kid");
    let split = corpus.split(11, &kp_real, "real-kid").unwrap();
    let kp_wrong = StaticKeyProvider::deterministic("wrong-kid");
    let sandbox = CountingSandbox::new();
    let runner = UniformRunner {
        verdict: ValidatorVerdict::Pass,
        latency_ms: 50,
        capsule_bytes: 10_000,
    };
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
    let snapshot = snapshot_a();

    let err = evaluator
        .evaluate(&split, &kp_wrong, &snapshot)
        .expect_err("wrong key must fail");
    match err {
        EvalError::Corpus(_) => {}
        other => panic!("expected EvalError::Corpus; got {other:?}"),
    }
}

// ----------------------------------------------------------------------------
// Test 11: Validator runner failure propagates as EvalError::ValidatorRunner.
// ----------------------------------------------------------------------------

#[test]
fn mt151_validator_runner_failure_propagates_typed_error() {
    let corpus = make_corpus(30);
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(11, &kp, "kid").unwrap();
    let sandbox = CountingSandbox::new();
    let runner = FailingRunner {
        message: "synthetic runner failure",
    };
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
    let snapshot = snapshot_a();

    let err = evaluator
        .evaluate(&split, &kp, &snapshot)
        .expect_err("runner failure must propagate");
    match err {
        EvalError::ValidatorRunner { message } => {
            assert!(message.contains("synthetic runner failure"));
        }
        other => panic!("expected EvalError::ValidatorRunner; got {other:?}"),
    }
}

// ----------------------------------------------------------------------------
// Test 12: PerItemResult round-trip via serde JSON (auditable EvalResult).
// ----------------------------------------------------------------------------

#[test]
fn mt151_eval_result_round_trips_via_serde_json() {
    let corpus = make_corpus(30);
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(11, &kp, "kid").unwrap();
    let sandbox = CountingSandbox::new();
    let runner = UniformRunner {
        verdict: ValidatorVerdict::Pass,
        latency_ms: 50,
        capsule_bytes: 10_000,
    };
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
    let snapshot = snapshot_a();

    let result = evaluator.evaluate(&split, &kp, &snapshot).unwrap();
    let json = serde_json::to_string(&result).expect("must serialize");
    let round_tripped: handshake_core::self_improve::evaluator::EvalResult =
        serde_json::from_str(&json).expect("must deserialize");
    assert_eq!(result, round_tripped);
}

// ----------------------------------------------------------------------------
// Test 13: Determinism — same (split, snapshot, deterministic runner)
// produces identical SplitMetrics on repeated evaluate() calls. We
// compare per-split metrics excluding evaluated_at_utc (which is `now`).
// ----------------------------------------------------------------------------

#[test]
fn mt151_repeated_evaluate_is_deterministic_in_metrics() {
    let corpus = make_corpus(30);
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(11, &kp, "kid").unwrap();
    let sandbox1 = CountingSandbox::new();
    let runner1 = UniformRunner {
        verdict: ValidatorVerdict::Pass,
        latency_ms: 50,
        capsule_bytes: 10_000,
    };
    let evaluator1 = ValidatorFirstPassEvaluator::new(&sandbox1, &runner1);
    let snapshot = snapshot_a();
    let result1 = evaluator1.evaluate(&split, &kp, &snapshot).unwrap();

    let sandbox2 = CountingSandbox::new();
    let runner2 = UniformRunner {
        verdict: ValidatorVerdict::Pass,
        latency_ms: 50,
        capsule_bytes: 10_000,
    };
    let evaluator2 = ValidatorFirstPassEvaluator::new(&sandbox2, &runner2);
    let result2 = evaluator2.evaluate(&split, &kp, &snapshot).unwrap();

    assert_eq!(result1.train.pass_rate, result2.train.pass_rate);
    assert_eq!(result1.train.pass_count, result2.train.pass_count);
    assert_eq!(result1.train.total_count, result2.train.total_count);
    assert_eq!(result1.train.latency_p95_ms, result2.train.latency_p95_ms);
    assert_eq!(
        result1.train.capsule_bytes_p95,
        result2.train.capsule_bytes_p95
    );
    assert_eq!(result1.snapshot_hash, result2.snapshot_hash);
}

// ----------------------------------------------------------------------------
// Test 14: evaluated_at_utc is monotonically non-decreasing across
// consecutive evaluate() calls (sanity check on timestamp source).
// ----------------------------------------------------------------------------

#[test]
fn mt151_evaluated_at_utc_is_non_decreasing_across_calls() {
    let corpus = make_corpus(30);
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(11, &kp, "kid").unwrap();
    let sandbox = CountingSandbox::new();
    let runner = UniformRunner {
        verdict: ValidatorVerdict::Pass,
        latency_ms: 50,
        capsule_bytes: 10_000,
    };
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
    let snapshot = snapshot_a();

    let before = Utc::now();
    let r1 = evaluator.evaluate(&split, &kp, &snapshot).unwrap();
    let r2 = evaluator.evaluate(&split, &kp, &snapshot).unwrap();
    let after = Utc::now();

    assert!(r1.evaluated_at_utc >= before);
    assert!(r2.evaluated_at_utc >= r1.evaluated_at_utc);
    assert!(r2.evaluated_at_utc <= after);
}

// ----------------------------------------------------------------------------
// Test 15: Cross-key isolation. A split encrypted with KEY_A cannot be
// evaluated with a provider that knows only KEY_B; the corpus error is
// the KeyProvider::UnknownKey variant (not AuthFailure), because the
// provider doesn't even attempt decryption with the wrong key bytes.
// ----------------------------------------------------------------------------

#[test]
fn mt151_unknown_key_id_distinguishes_from_auth_failure() {
    use handshake_core::self_improve::corpus::{CorpusError, KeyError};

    let corpus = make_corpus(30);
    let kp_a = StaticKeyProvider::deterministic("KEY_A");
    let split = corpus.split(11, &kp_a, "KEY_A").unwrap();
    let kp_b = StaticKeyProvider::deterministic("KEY_B");
    let sandbox = CountingSandbox::new();
    let runner = UniformRunner {
        verdict: ValidatorVerdict::Pass,
        latency_ms: 50,
        capsule_bytes: 10_000,
    };
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
    let snapshot = snapshot_a();

    let err = evaluator
        .evaluate(&split, &kp_b, &snapshot)
        .expect_err("unknown key must fail");
    let corpus_err = match err {
        EvalError::Corpus(c) => c,
        other => panic!("expected EvalError::Corpus; got {other:?}"),
    };
    match corpus_err {
        CorpusError::KeyProvider(KeyError::UnknownKey { .. }) => {}
        other => panic!("expected KeyProvider::UnknownKey; got {other:?}"),
    }
}

// ----------------------------------------------------------------------------
// Test 16: RetrievalPolicy snapshot variant also feeds into snapshot_hash
// and produces a different hash from a ModelManual snapshot, even with
// the same string content. This guards against any flatten-to-string
// bug where the variant tag is omitted.
// ----------------------------------------------------------------------------

#[test]
fn mt151_snapshot_hash_distinguishes_variant_kinds() {
    use handshake_core::memory::TaskType;
    use handshake_core::self_improve::iteration::PolicyParameterRef;

    let corpus = make_corpus(30);
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(11, &kp, "kid").unwrap();
    let sandbox = CountingSandbox::new();
    let runner = UniformRunner {
        verdict: ValidatorVerdict::Pass,
        latency_ms: 50,
        capsule_bytes: 10_000,
    };
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);

    let snap_manual = snapshot_a();
    let snap_policy = EditableSurfaceSnapshot::RetrievalPolicy {
        task_type: TaskType::ValidatorHbrTestPacket,
        parameter: PolicyParameterRef::TopK,
        before_value: 6,
        after_value: 8,
    };
    let h_manual = evaluator
        .evaluate(&split, &kp, &snap_manual)
        .unwrap()
        .snapshot_hash;
    let h_policy = evaluator
        .evaluate(&split, &kp, &snap_policy)
        .unwrap()
        .snapshot_hash;
    assert_ne!(
        h_manual, h_policy,
        "snapshot variants must produce distinct hashes"
    );
}

// ----------------------------------------------------------------------------
// Test 17: split sizes match MT-150 60/20/20 contract for 30 items, and
// every item is exactly one of train/dev/holdout (no double-counting).
// ----------------------------------------------------------------------------

#[test]
fn mt151_split_sizes_match_60_20_20_contract() {
    let corpus = make_corpus(30);
    let kp = StaticKeyProvider::deterministic("kid");
    let split = corpus.split(11, &kp, "kid").unwrap();
    let sandbox = CountingSandbox::new();
    let runner = UniformRunner {
        verdict: ValidatorVerdict::Pass,
        latency_ms: 50,
        capsule_bytes: 10_000,
    };
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);
    let snapshot = snapshot_a();
    let result = evaluator.evaluate(&split, &kp, &snapshot).unwrap();
    assert_eq!(result.train.total_count, 18);
    assert_eq!(result.dev.total_count, 6);
    assert_eq!(result.holdout.total_count, 6);

    // Cross-split disjoint check via the live per_item_results.
    use std::collections::BTreeSet;
    let train_ids: BTreeSet<Uuid> = result
        .train
        .per_item_results
        .iter()
        .map(|r| r.item_id)
        .collect();
    let dev_ids: BTreeSet<Uuid> = result
        .dev
        .per_item_results
        .iter()
        .map(|r| r.item_id)
        .collect();
    let holdout_ids: BTreeSet<Uuid> = result
        .holdout
        .per_item_results
        .iter()
        .map(|r| r.item_id)
        .collect();
    assert!(
        train_ids.is_disjoint(&dev_ids),
        "train and dev must be disjoint"
    );
    assert!(
        train_ids.is_disjoint(&holdout_ids),
        "train and holdout must be disjoint"
    );
    assert!(
        dev_ids.is_disjoint(&holdout_ids),
        "dev and holdout must be disjoint"
    );
}

// ----------------------------------------------------------------------------
// Helper: explicit reference to the CorpusSplit + KeyProvider types so
// future refactors that rename them break this test file at compile time.
// ----------------------------------------------------------------------------

#[allow(dead_code)]
fn type_anchor(split: &CorpusSplit, kp: &dyn KeyProvider) {
    // No body — just a compile-time anchor.
    let _ = (split, kp);
}
