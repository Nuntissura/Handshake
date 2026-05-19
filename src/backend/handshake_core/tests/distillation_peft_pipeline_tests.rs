//! MT-122 cross-crate integration smoke for the PEFT pipeline. The
//! inline tests in `distillation::peft_pipeline::tests` cover the
//! orchestrator decision tree exhaustively; this file pins the
//! public API surface + the existence of the Python script entrypoint.

use std::path::Path;

use handshake_core::distillation::{
    content_review::ContentReviewConfig,
    corpus_extractor::{TrainingCorpus, TrainingTurn},
    peft_pipeline::{
        distill, DistillError, DistillJobConfig, PeftHyperparams, PeftTrainerExecutor,
    },
};

struct StubExecutor;
impl PeftTrainerExecutor for StubExecutor {
    fn run(&self, _config: &DistillJobConfig) -> Result<(), DistillError> {
        Ok(())
    }
}

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
        sourced_at_utc: "2026-05-20T04:00:00Z".to_string(),
    }
}

#[test]
fn peft_pipeline_default_hyperparams_baseline_recipe() {
    let h = PeftHyperparams::default();
    assert_eq!(h.rank, 16);
    assert_eq!(h.alpha, 32.0);
    assert_eq!(h.epochs, 1);
    assert!((h.learning_rate - 2e-4).abs() < f32::EPSILON);
    assert_eq!(h.batch_size, 4);
}

#[test]
fn peft_pipeline_distill_writes_filtered_jsonl_and_invokes_executor() {
    let corpus = TrainingCorpus {
        session_id: "s".to_string(),
        turns: vec![
            turn("t1", "Q1?", "A1", "MIT"),
            turn("t2", "Q2?", "A2", "MIT"),
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
        operator_signature: "op-ilja".to_string(),
    };
    let artifact = distill(
        &corpus,
        config,
        ContentReviewConfig::defaults(),
        &StubExecutor,
        "2026-05-20T04:00:00Z",
    )
    .expect("distill ok");

    assert_eq!(artifact.corpus_turn_count, 2);
    assert_eq!(artifact.corpus_rejected_count, 0);
    assert_eq!(artifact.license_tag, "MIT");
    assert_eq!(artifact.operator_signature, "op-ilja");

    let raw = std::fs::read_to_string(&corpus_path).unwrap();
    assert_eq!(raw.lines().count(), 2);
}

#[test]
fn python_trainer_script_is_committed_at_known_path() {
    // The Rust executor invokes scripts/distill/train_lora.py; pin
    // its presence at the worktree-relative path so future repo
    // moves surface a regression here rather than at distill time.
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("handshake_core lives under src/backend/handshake_core");
    let script_path = repo_root.join("scripts/distill/train_lora.py");
    assert!(
        script_path.is_file(),
        "MT-122 trainer script must exist at {}",
        script_path.display()
    );
    let requirements_path = repo_root.join("scripts/distill/requirements.txt");
    assert!(
        requirements_path.is_file(),
        "MT-122 requirements must exist at {}",
        requirements_path.display()
    );
    // Spot-check critical dependency pins.
    let requirements = std::fs::read_to_string(&requirements_path).unwrap();
    assert!(requirements.contains("torch"));
    assert!(requirements.contains("transformers"));
    assert!(requirements.contains("peft"));
}
