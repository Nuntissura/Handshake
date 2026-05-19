//! MT-122: Teacher/student PEFT distillation pipeline.
//!
//! Per Master Spec §4.8 + refinement INF-1 (LoRA-as-distillation-output).
//! PEFT training is not available in Rust at production maturity as of
//! the WP-KERNEL-004 window; the practical path is to spawn a Python
//! subprocess that runs PEFT/Transformers training and writes a LoRA
//! artifact directory. This module is the Rust orchestrator: it gates
//! corpus turns through `ContentReview` (MT-120), writes the filtered
//! corpus to a JSONL file (governed artifact), spawns the configured
//! trainer subprocess, awaits completion, and assembles the
//! `DistilledLoraArtifact` provenance.
//!
//! Subprocess launch + sandbox + ProcessOwnershipLedger registration
//! are abstracted behind the [`PeftTrainerExecutor`] trait so the
//! concrete impl (which depends on cluster-B sandbox adapters +
//! MT-069 EngineKind=DistillationJob ledger wiring) can land in a
//! follow-on without touching the orchestrator semantics. The
//! `ContentReview` gate runs in-process and is fully unit-testable.
//!
//! Adult-production discipline: per GLOBAL-PRODUCTION-002..009 the
//! pipeline never moralises or rewords operator content. The gates
//! are (a) opt-in (MT-121), (b) license + PII review (MT-120), (c)
//! operator-driven hyperparams. No automatic content judgement
//! beyond those.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::content_review::{ContentReview, ContentReviewConfig, ReviewVerdict};
use super::corpus_extractor::TrainingCorpus;

/// Operator-tunable hyperparameters. Defaults track common PEFT/LoRA
/// recipes for ~7B instruct models; operators override per run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeftHyperparams {
    pub rank: u32,
    pub alpha: f32,
    pub dropout: f32,
    pub epochs: u32,
    pub learning_rate: f32,
    pub batch_size: u32,
}

impl Default for PeftHyperparams {
    fn default() -> Self {
        Self {
            rank: 16,
            alpha: 32.0,
            dropout: 0.05,
            epochs: 1,
            learning_rate: 2e-4,
            batch_size: 4,
        }
    }
}

/// Inputs to a distillation run.
#[derive(Clone, Debug, PartialEq)]
pub struct DistillJobConfig {
    pub teacher_model_path: PathBuf,
    pub student_base_model_path: PathBuf,
    pub output_lora_dir: PathBuf,
    pub corpus_jsonl_path: PathBuf,
    pub hyperparams: PeftHyperparams,
    pub license_tag: String,
    pub operator_signature: String,
}

/// On-disk LoRA artifact provenance written alongside the LoRA weights.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DistilledLoraArtifact {
    pub lora_dir: PathBuf,
    pub teacher_model_path: PathBuf,
    pub student_base_model_path: PathBuf,
    pub corpus_path: PathBuf,
    pub corpus_turn_count: u32,
    pub corpus_quarantined_count: u32,
    pub corpus_rejected_count: u32,
    pub hyperparams: PeftHyperparams,
    pub license_tag: String,
    pub operator_signature: String,
    pub finished_at_utc: String,
}

#[derive(Debug, Error)]
pub enum DistillError {
    #[error("distillation config invalid: {0}")]
    InvalidConfig(String),
    #[error("content review fully rejected the corpus: {pass_count} of {turn_count} turns passed")]
    NoPassingTurns { turn_count: usize, pass_count: usize },
    #[error("corpus jsonl write failed: {0}")]
    CorpusWrite(String),
    #[error("PEFT trainer subprocess failed: {0}")]
    TrainerExec(String),
    #[error(
        "PEFT trainer environment not attached: {0}; install PEFT/Transformers + wire the \
         subprocess executor (cluster-B sandbox + MT-069 process ledger) before running live \
         distillation"
    )]
    TrainerUnavailable(String),
}

/// Summary of the corpus pass through `ContentReview`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorpusReviewSummary {
    pub turn_count: usize,
    pub pass_count: usize,
    pub quarantine_count: usize,
    pub reject_count: usize,
}

/// Abstraction over the actual subprocess launch + sandbox + process
/// ledger registration. The mock impl backs unit tests; the
/// production impl spawns `python scripts/distill/train_lora.py ...`
/// under a SandboxAdapter and registers a ProcessOwnershipLedger row
/// with engine_kind=DistillationJob (MT-069).
pub trait PeftTrainerExecutor {
    fn run(&self, config: &DistillJobConfig) -> Result<(), DistillError>;
}

/// Reviews a corpus through [`ContentReview`] and returns the verdicts
/// per turn alongside aggregate counts. The verdict ordering matches
/// `corpus.turns` ordering so callers can zip if they need per-turn
/// drill-down.
pub fn review_corpus(
    corpus: &TrainingCorpus,
    review_config: ContentReviewConfig,
) -> Result<(Vec<ReviewVerdict>, CorpusReviewSummary), DistillError> {
    let mut reviewer = ContentReview::new(review_config);
    let mut verdicts = Vec::with_capacity(corpus.turns.len());
    let mut pass_count = 0_usize;
    let mut quarantine_count = 0_usize;
    let mut reject_count = 0_usize;
    for turn in &corpus.turns {
        let verdict = reviewer
            .review(turn)
            .map_err(|err| DistillError::InvalidConfig(format!("content review: {err}")))?;
        match &verdict {
            ReviewVerdict::Pass { .. } => pass_count += 1,
            ReviewVerdict::Quarantine { .. } => quarantine_count += 1,
            ReviewVerdict::Reject { .. } => reject_count += 1,
        }
        verdicts.push(verdict);
    }
    Ok((
        verdicts,
        CorpusReviewSummary {
            turn_count: corpus.turns.len(),
            pass_count,
            quarantine_count,
            reject_count,
        },
    ))
}

/// Write the passing turns to a JSONL file in the format the Python
/// trainer expects: one `{ "prompt": ..., "completion": ... }` object
/// per line.
pub fn write_filtered_corpus_jsonl(
    corpus: &TrainingCorpus,
    verdicts: &[ReviewVerdict],
    path: &std::path::Path,
) -> Result<usize, DistillError> {
    use std::io::Write;
    if corpus.turns.len() != verdicts.len() {
        return Err(DistillError::InvalidConfig(format!(
            "turn/verdict length mismatch: {} vs {}",
            corpus.turns.len(),
            verdicts.len()
        )));
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|err| DistillError::CorpusWrite(format!("mkdir {}: {err}", parent.display())))?;
        }
    }
    let mut file = std::fs::File::create(path)
        .map_err(|err| DistillError::CorpusWrite(format!("create {}: {err}", path.display())))?;
    let mut written = 0_usize;
    for (turn, verdict) in corpus.turns.iter().zip(verdicts.iter()) {
        if !matches!(verdict, ReviewVerdict::Pass { .. }) {
            continue;
        }
        let line = serde_json::json!({
            "id": turn.id,
            "prompt": turn.prompt,
            "completion": turn.completion,
            "license_tag": turn.license_tag,
            "model_id": turn.model_id,
            "session_id": turn.session_id,
        })
        .to_string();
        writeln!(file, "{line}").map_err(|err| {
            DistillError::CorpusWrite(format!("write {}: {err}", path.display()))
        })?;
        written += 1;
    }
    Ok(written)
}

/// Top-level orchestrator. The trait-injected executor performs the
/// actual training; this function is responsible for the gating + I/O
/// + provenance assembly.
pub fn distill(
    corpus: &TrainingCorpus,
    config: DistillJobConfig,
    review_config: ContentReviewConfig,
    executor: &dyn PeftTrainerExecutor,
    finished_at_utc: &str,
) -> Result<DistilledLoraArtifact, DistillError> {
    if config.operator_signature.trim().is_empty() {
        return Err(DistillError::InvalidConfig(
            "operator_signature must not be empty".to_string(),
        ));
    }
    if config.license_tag.trim().is_empty() {
        return Err(DistillError::InvalidConfig(
            "license_tag must not be empty".to_string(),
        ));
    }

    let (verdicts, summary) = review_corpus(corpus, review_config)?;
    if summary.pass_count == 0 {
        return Err(DistillError::NoPassingTurns {
            turn_count: summary.turn_count,
            pass_count: summary.pass_count,
        });
    }
    let written = write_filtered_corpus_jsonl(corpus, &verdicts, &config.corpus_jsonl_path)?;
    if written != summary.pass_count {
        return Err(DistillError::CorpusWrite(format!(
            "wrote {written} turns to {} but {} passed review",
            config.corpus_jsonl_path.display(),
            summary.pass_count
        )));
    }
    executor.run(&config)?;
    Ok(DistilledLoraArtifact {
        lora_dir: config.output_lora_dir.clone(),
        teacher_model_path: config.teacher_model_path.clone(),
        student_base_model_path: config.student_base_model_path.clone(),
        corpus_path: config.corpus_jsonl_path.clone(),
        corpus_turn_count: summary.turn_count as u32,
        corpus_quarantined_count: summary.quarantine_count as u32,
        corpus_rejected_count: summary.reject_count as u32,
        hyperparams: config.hyperparams,
        license_tag: config.license_tag,
        operator_signature: config.operator_signature,
        finished_at_utc: finished_at_utc.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distillation::corpus_extractor::TrainingTurn;

    fn turn(id: &str, prompt: &str, completion: &str, license: &str) -> TrainingTurn {
        TrainingTurn {
            id: id.to_string(),
            session_id: "session".to_string(),
            model_id: "model".to_string(),
            prompt: prompt.to_string(),
            completion: completion.to_string(),
            finish_reason: Some("stop".to_string()),
            license_tag: license.to_string(),
            source_event_ids: vec!["e1".to_string()],
            sourced_at_utc: "2026-05-20T03:00:00Z".to_string(),
        }
    }

    struct MockExecutor {
        called: std::cell::RefCell<Option<DistillJobConfig>>,
        should_fail: bool,
    }
    impl PeftTrainerExecutor for MockExecutor {
        fn run(&self, config: &DistillJobConfig) -> Result<(), DistillError> {
            *self.called.borrow_mut() = Some(config.clone());
            if self.should_fail {
                Err(DistillError::TrainerExec("mock failure".to_string()))
            } else {
                Ok(())
            }
        }
    }

    struct NeverCalledExecutor;
    impl PeftTrainerExecutor for NeverCalledExecutor {
        fn run(&self, _config: &DistillJobConfig) -> Result<(), DistillError> {
            panic!("executor must not be invoked when corpus review yields zero passing turns");
        }
    }

    #[test]
    fn default_hyperparams_match_baseline_recipe() {
        let h = PeftHyperparams::default();
        assert_eq!(h.rank, 16);
        assert_eq!(h.epochs, 1);
        assert!((h.learning_rate - 2e-4).abs() < f32::EPSILON);
    }

    #[test]
    fn review_corpus_counts_pass_quarantine_reject() {
        let corpus = TrainingCorpus {
            session_id: "s".to_string(),
            turns: vec![
                turn("t1", "What is 7*8?", "56", "MIT"),
                turn("t2", "use api key sk-abcdefghij1234567890", "ok", "MIT"),
                turn("t3", "email alice@example.com", "ok", "MIT"),
                turn("t4", "x", "y", "Proprietary"),
            ],
        };
        let (verdicts, summary) = review_corpus(&corpus, ContentReviewConfig::defaults()).unwrap();
        assert_eq!(verdicts.len(), 4);
        assert_eq!(summary.turn_count, 4);
        assert_eq!(summary.pass_count, 1);
        assert!(summary.quarantine_count >= 1);
        assert!(summary.reject_count >= 1);
    }

    #[test]
    fn distill_errors_when_no_turn_passes_review() {
        let corpus = TrainingCorpus {
            session_id: "s".to_string(),
            turns: vec![turn("t1", "x", "y", "Proprietary")],
        };
        let tmp = tempfile::tempdir().unwrap();
        let config = DistillJobConfig {
            teacher_model_path: tmp.path().join("teacher"),
            student_base_model_path: tmp.path().join("student"),
            output_lora_dir: tmp.path().join("lora"),
            corpus_jsonl_path: tmp.path().join("corpus.jsonl"),
            hyperparams: PeftHyperparams::default(),
            license_tag: "MIT".to_string(),
            operator_signature: "op".to_string(),
        };
        let err = distill(
            &corpus,
            config,
            ContentReviewConfig::defaults(),
            &NeverCalledExecutor,
            "2026-05-20T03:00:00Z",
        )
        .expect_err("no passing turns");
        assert!(matches!(err, DistillError::NoPassingTurns { .. }));
    }

    #[test]
    fn distill_writes_filtered_jsonl_and_returns_provenance() {
        let corpus = TrainingCorpus {
            session_id: "s".to_string(),
            turns: vec![
                turn("t1", "Q1?", "A1", "MIT"),
                turn("t2", "Q2?", "A2", "MIT"),
                turn("t3", "use api key sk-abcdefghij1234567890", "leak", "MIT"),
            ],
        };
        let tmp = tempfile::tempdir().unwrap();
        let corpus_path = tmp.path().join("corpus.jsonl");
        let config = DistillJobConfig {
            teacher_model_path: tmp.path().join("teacher"),
            student_base_model_path: tmp.path().join("student"),
            output_lora_dir: tmp.path().join("lora"),
            corpus_jsonl_path: corpus_path.clone(),
            hyperparams: PeftHyperparams::default(),
            license_tag: "MIT".to_string(),
            operator_signature: "op".to_string(),
        };
        let executor = MockExecutor {
            called: std::cell::RefCell::new(None),
            should_fail: false,
        };
        let artifact = distill(
            &corpus,
            config,
            ContentReviewConfig::defaults(),
            &executor,
            "2026-05-20T03:00:00Z",
        )
        .expect("distill");
        assert!(executor.called.borrow().is_some());
        assert_eq!(artifact.corpus_turn_count, 3);
        assert_eq!(artifact.corpus_rejected_count, 1);
        // 2 passing turns written to corpus.jsonl.
        let raw = std::fs::read_to_string(&corpus_path).unwrap();
        let lines: Vec<_> = raw.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn distill_rejects_empty_signature_and_license() {
        let corpus = TrainingCorpus {
            session_id: "s".to_string(),
            turns: vec![turn("t1", "Q?", "A", "MIT")],
        };
        let tmp = tempfile::tempdir().unwrap();
        let base = DistillJobConfig {
            teacher_model_path: tmp.path().join("t"),
            student_base_model_path: tmp.path().join("s"),
            output_lora_dir: tmp.path().join("l"),
            corpus_jsonl_path: tmp.path().join("c.jsonl"),
            hyperparams: PeftHyperparams::default(),
            license_tag: "MIT".to_string(),
            operator_signature: "op".to_string(),
        };

        let mut bad = base.clone();
        bad.operator_signature = "  ".to_string();
        let err = distill(
            &corpus,
            bad,
            ContentReviewConfig::defaults(),
            &NeverCalledExecutor,
            "2026-05-20T03:00:00Z",
        )
        .expect_err("empty signature");
        assert!(matches!(err, DistillError::InvalidConfig(_)));

        let mut bad = base.clone();
        bad.license_tag = "".to_string();
        let err = distill(
            &corpus,
            bad,
            ContentReviewConfig::defaults(),
            &NeverCalledExecutor,
            "2026-05-20T03:00:00Z",
        )
        .expect_err("empty license");
        assert!(matches!(err, DistillError::InvalidConfig(_)));
    }

    #[test]
    fn distill_propagates_executor_failure() {
        let corpus = TrainingCorpus {
            session_id: "s".to_string(),
            turns: vec![turn("t1", "Q?", "A", "MIT")],
        };
        let tmp = tempfile::tempdir().unwrap();
        let config = DistillJobConfig {
            teacher_model_path: tmp.path().join("t"),
            student_base_model_path: tmp.path().join("s"),
            output_lora_dir: tmp.path().join("l"),
            corpus_jsonl_path: tmp.path().join("c.jsonl"),
            hyperparams: PeftHyperparams::default(),
            license_tag: "MIT".to_string(),
            operator_signature: "op".to_string(),
        };
        let executor = MockExecutor {
            called: std::cell::RefCell::new(None),
            should_fail: true,
        };
        let err = distill(
            &corpus,
            config,
            ContentReviewConfig::defaults(),
            &executor,
            "2026-05-20T03:00:00Z",
        )
        .expect_err("trainer failure");
        assert!(matches!(err, DistillError::TrainerExec(_)));
    }

}
