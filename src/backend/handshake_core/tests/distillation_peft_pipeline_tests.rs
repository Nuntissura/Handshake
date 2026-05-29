//! MT-122 cross-crate integration smoke for the PEFT pipeline. The
//! inline tests in `distillation::peft_pipeline::tests` cover the
//! orchestrator decision tree exhaustively; this file pins the
//! public API surface + the existence of the Python script entrypoint.

use std::path::Path;

use handshake_core::distillation::{
    content_review::ContentReviewConfig,
    corpus_extractor::{TrainingCorpus, TrainingTurn},
    peft_pipeline::{
        distill, distill_with_flight_recorder, review_corpus_with_events, DistillError,
        DistillJobConfig, PeftHyperparams, PeftTrainerExecutor,
    },
};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, FlightRecorderEventType, RecorderError,
};
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Default)]
struct MemoryFlightRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

#[async_trait::async_trait]
impl FlightRecorder for MemoryFlightRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        self.events.lock().expect("recorder lock").push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self.events.lock().expect("recorder lock").clone())
    }
}

struct StubExecutor;
impl PeftTrainerExecutor for StubExecutor {
    fn run(&self, _config: &DistillJobConfig) -> Result<(), DistillError> {
        Ok(())
    }
}

struct NeverCalledExecutor;
impl PeftTrainerExecutor for NeverCalledExecutor {
    fn run(&self, _config: &DistillJobConfig) -> Result<(), DistillError> {
        panic!("executor must not run when content review yields zero passing turns");
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

fn distill_config(tmp: &tempfile::TempDir) -> DistillJobConfig {
    DistillJobConfig {
        teacher_model_path: tmp.path().join("teacher"),
        student_base_model_path: tmp.path().join("student"),
        output_lora_dir: tmp.path().join("lora"),
        corpus_jsonl_path: tmp.path().join("corpus.jsonl"),
        hyperparams: PeftHyperparams::default(),
        license_tag: "MIT".to_string(),
        operator_signature: "op-ilja".to_string(),
        teacher_source: handshake_core::distillation::peft_pipeline::TeacherSource::CliBridge,
        max_steps: None,
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
        teacher_source: handshake_core::distillation::peft_pipeline::TeacherSource::CliBridge,
        max_steps: None,
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
fn peft_pipeline_review_corpus_with_events_exposes_sanitized_pii_fr_event() {
    let corpus = TrainingCorpus {
        session_id: "s".to_string(),
        turns: vec![turn("t1", "email alice@example.com", "ok", "MIT")],
    };

    let reviewed = review_corpus_with_events(&corpus, ContentReviewConfig::defaults())
        .expect("review corpus with events");
    assert_eq!(reviewed.verdicts.len(), 1);
    assert_eq!(reviewed.outcomes.len(), 1);
    assert_eq!(reviewed.summary.quarantine_count, 1);

    let events =
        reviewed.outcomes[0].flight_recorder_events(uuid::Uuid::now_v7(), "job-review-corpus-pii");
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].event_type,
        FlightRecorderEventType::DistillPiiDetected
    );
    assert_eq!(events[0].event_type.to_string(), "distill.pii_detected");
    assert_eq!(events[0].job_id.as_deref(), Some("job-review-corpus-pii"));
    assert_eq!(events[0].payload["pii_kinds"], serde_json::json!(["email"]));
    let payload_text = serde_json::to_string(&events[0].payload).unwrap();
    assert!(!payload_text.contains("alice@example.com"));
}

#[tokio::test]
async fn peft_pipeline_distill_with_flight_recorder_records_sanitized_pii_before_zero_pass_abort() {
    let corpus = TrainingCorpus {
        session_id: "s".to_string(),
        turns: vec![turn(
            "t1",
            "email alice@example.com about the training run",
            "No raw source text may be logged.",
            "MIT",
        )],
    };
    let tmp = tempfile::tempdir().unwrap();
    let recorder = MemoryFlightRecorder::default();
    let job_id = "job-distill-pii-zero-pass";

    let err = distill_with_flight_recorder(
        &corpus,
        distill_config(&tmp),
        ContentReviewConfig::defaults(),
        &NeverCalledExecutor,
        "2026-05-20T04:00:00Z",
        &recorder,
        Uuid::now_v7(),
        job_id,
    )
    .await
    .expect_err("all-quarantined corpus should still abort");

    assert!(
        matches!(err, DistillError::NoPassingTurns { .. }),
        "unexpected error: {err:?}"
    );

    let events = recorder
        .list_events(EventFilter::default())
        .await
        .expect("list recorded events");
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].event_type,
        FlightRecorderEventType::DistillPiiDetected
    );
    assert_eq!(events[0].job_id.as_deref(), Some(job_id));
    assert_eq!(events[0].payload["pii_kinds"], serde_json::json!(["email"]));
    let payload_text = serde_json::to_string(&events[0].payload).unwrap();
    assert!(!payload_text.contains("alice@example.com"));
    assert!(!payload_text.contains("raw_prompt"));
    assert!(!payload_text.contains("raw_completion"));
    assert!(!payload_text.contains("email alice"));
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
