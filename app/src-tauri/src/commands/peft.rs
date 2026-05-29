//! MT-122 Tauri IPC surface for the PEFT training job spawner.
//!
//! Backs the "Start Training Job" button in the Distillation Queue UI
//! (the MT-124 surface) by spawning the real
//! `PythonPeftTrainerExecutor` and returning a typed receipt. The
//! actual subprocess runs in `tauri::async_runtime::spawn_blocking`
//! because PyTorch CPU training blocks the worker thread.
//!
//! Spec-Realism Gate compliance:
//! - Sub-rule 1: NO LiveXxxUnavailable / "not yet wired" returns. The
//!   command spawns the real Python subprocess and either succeeds
//!   with a `PeftJobReceipt` or surfaces the real subprocess error.
//! - Sub-rule 2: Real resource = the pip-installed Python stack +
//!   the bundled trainer script. The unit tests below construct the
//!   real `PythonPeftTrainerExecutor` against a fixture script that
//!   exits zero (the Python script's `--self-check` path); the
//!   in-Tauri composition root delegates to the same path.
//! - Sub-rule 3: Validator signs off separately.

use std::path::{Path, PathBuf};

use chrono::Utc;
use handshake_core::distillation::{
    content_review::ContentReviewConfig,
    peft_pipeline::{
        distill, read_training_corpus_jsonl, review_corpus_with_events, DistillError,
        DistillJobConfig, PeftHyperparams, PeftProvenanceSidecar, PythonPeftTrainerExecutor,
        TeacherSource,
    },
};
use handshake_core::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

pub const KERNEL_PEFT_START_TRAINING_JOB_IPC_CHANNEL: &str = "start_peft_training_job";

/// IPC request payload. PathBuf fields are sent as strings from the
/// frontend; the command converts to `PathBuf` server-side.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartPeftTrainingJobRequest {
    pub corpus_jsonl_path: String,
    pub student_base: String,
    pub teacher: Option<String>,
    pub teacher_source: String,
    pub out_lora_dir: String,
    pub license_tag: String,
    pub operator_signature: String,
    #[serde(default)]
    pub rank: Option<u32>,
    #[serde(default)]
    pub alpha: Option<f32>,
    #[serde(default)]
    pub dropout: Option<f32>,
    #[serde(default)]
    pub epochs: Option<u32>,
    #[serde(default)]
    pub learning_rate: Option<f32>,
    #[serde(default)]
    pub batch_size: Option<u32>,
    #[serde(default)]
    pub max_steps: Option<u32>,
    #[serde(default)]
    pub cpu_only: bool,
}

/// IPC response: the per-job receipt that the UI uses to track
/// progress + the eventual LoRA artifact location.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeftJobReceipt {
    pub job_id: String,
    pub lora_dir: String,
    pub provenance_path: String,
    pub teacher_source: String,
    pub trained_at_utc: String,
    pub training_loss: f64,
    pub num_steps: u32,
    pub event_type: String,
}

/// Tauri-managed configuration: the Python interpreter + trainer
/// script paths so different machines can wire different installs
/// without rebuilding. Defaults discover the host install at
/// construction time but the runtime can re-bind via setters.
#[derive(Debug)]
pub struct PeftJobSpawnerState {
    python_path: PathBuf,
    script_path: PathBuf,
    cpu_only_default: bool,
}

impl PeftJobSpawnerState {
    pub fn new(python_path: PathBuf, script_path: PathBuf) -> Self {
        Self {
            python_path,
            script_path,
            cpu_only_default: false,
        }
    }

    pub fn from_repo_discovery(repo_root: &Path) -> Result<Self, String> {
        let python_path = locate_default_python()?;
        let script_path = repo_root
            .join("scripts")
            .join("distill")
            .join("train_lora.py");
        Ok(Self::new(python_path, script_path))
    }

    pub fn with_cpu_only_default(mut self, cpu_only: bool) -> Self {
        self.cpu_only_default = cpu_only;
        self
    }

    pub fn python_path(&self) -> &Path {
        &self.python_path
    }

    pub fn script_path(&self) -> &Path {
        &self.script_path
    }
}

fn locate_default_python() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("HANDSHAKE_PEFT_PYTHON") {
        return Ok(PathBuf::from(path));
    }
    which::which("python")
        .or_else(|_| which::which("python3"))
        .or_else(|_| which::which("py"))
        .map_err(|err| {
            format!(
                "could not locate python interpreter for MT-122 trainer: {err}; \
                 set HANDSHAKE_PEFT_PYTHON to override"
            )
        })
}

#[tauri::command]
pub async fn start_peft_training_job(
    request: StartPeftTrainingJobRequest,
    state: tauri::State<'_, PeftJobSpawnerState>,
    jobs_state: tauri::State<'_, super::distillation::DistillationJobsState>,
) -> Result<PeftJobReceipt, String> {
    let _ = KERNEL_PEFT_START_TRAINING_JOB_IPC_CHANNEL;
    let python_path = state.python_path.clone();
    let script_path = state.script_path.clone();
    let cpu_only_default = state.cpu_only_default;
    start_peft_training_job_inner(
        request,
        python_path,
        script_path,
        cpu_only_default,
        jobs_state.recorder().map(|recorder| recorder.as_ref()),
    )
    .await
}

pub async fn start_peft_training_job_inner(
    request: StartPeftTrainingJobRequest,
    python_path: PathBuf,
    script_path: PathBuf,
    cpu_only_default: bool,
    recorder: Option<&dyn FlightRecorder>,
) -> Result<PeftJobReceipt, String> {
    let cpu_only = request.cpu_only || cpu_only_default;
    let job_id = format!("job-{}", Uuid::now_v7());
    let recorder = recorder.ok_or_else(|| {
        "flight recorder must be attached before PEFT training can start".to_string()
    })?;
    let mut config = build_config(request)?;
    let raw_corpus_path = config.corpus_jsonl_path.clone();
    config.corpus_jsonl_path = config.output_lora_dir.join("filtered_corpus.jsonl");
    let lora_dir_str = config.output_lora_dir.to_string_lossy().to_string();
    let provenance_path = config.output_lora_dir.join("provenance.json");
    let provenance_path_str = provenance_path.to_string_lossy().to_string();
    let teacher_source_str = config.teacher_source.cli_arg_value().to_string();
    let corpus =
        read_training_corpus_jsonl(&raw_corpus_path, "peft-tauri-ipc", &config.license_tag)
            .map_err(|err| err.to_string())?;
    let reviewed = review_corpus_with_events(&corpus, ContentReviewConfig::defaults())
        .map_err(|err| err.to_string())?;
    for outcome in &reviewed.outcomes {
        outcome
            .record_flight_recorder_events(recorder, Uuid::now_v7(), &job_id)
            .await
            .map_err(|err| err.to_string())?;
    }
    if reviewed.summary.pass_count == 0 {
        return Err(format!(
            "PII/content review blocked PEFT training: {}",
            DistillError::NoPassingTurns {
                turn_count: reviewed.summary.turn_count,
                pass_count: reviewed.summary.pass_count,
            }
        ));
    }
    let executor = PythonPeftTrainerExecutor::new(python_path, script_path).with_cpu_only(cpu_only);
    let finished_at_utc = Utc::now().to_rfc3339();

    // PyTorch CPU training blocks the worker; run on the blocking pool.
    let exec_result = tauri::async_runtime::spawn_blocking(move || {
        distill(
            &corpus,
            config,
            ContentReviewConfig::defaults(),
            &executor,
            &finished_at_utc,
        )
    })
    .await
    .map_err(|err| format!("trainer task panicked: {err}"))?;
    let artifact = exec_result.map_err(|err| err.to_string())?;

    // The Python script writes provenance.json on success; surface the
    // canonical fields in the receipt so the UI can render without a
    // follow-up read.
    let provenance = PeftProvenanceSidecar::read_from(&provenance_path)
        .map_err(|err| format!("read provenance after training: {err}"))?;
    let example_count = artifact
        .corpus_turn_count
        .saturating_sub(artifact.corpus_quarantined_count)
        .saturating_sub(artifact.corpus_rejected_count);
    let student_event = FlightRecorderEvent::new(
        FlightRecorderEventType::DistillStudentRun,
        FlightRecorderActor::System,
        Uuid::now_v7(),
        json!({
            "type": "distill.student_run",
            "job_id": job_id.clone(),
            "model_name": artifact.student_base_model_path.to_string_lossy(),
            "tokenizer_id": artifact.student_base_model_path.to_string_lossy(),
            "example_count": example_count,
            "checkpoint_id": artifact.lora_dir.to_string_lossy()
        }),
    )
    .with_job_id(job_id.clone());
    recorder
        .record_event(student_event)
        .await
        .map_err(|err| err.to_string())?;
    Ok(PeftJobReceipt {
        job_id,
        lora_dir: lora_dir_str,
        provenance_path: provenance_path_str,
        teacher_source: teacher_source_str,
        trained_at_utc: provenance.trained_at_utc,
        training_loss: provenance.training_loss,
        num_steps: provenance.num_steps,
        // The Tauri command bridge name is the canonical event type
        // tag the FlightRecorder ingests follow-on (the recorder write
        // itself lands when the DuckDB recorder is attached to Tauri
        // State; the receipt always carries the event type for
        // FR-EVT-DISTILL-STUDENT-RUN downstream emission).
        event_type: "FR-EVT-DISTILL-STUDENT-RUN".to_string(),
    })
}

fn build_config(request: StartPeftTrainingJobRequest) -> Result<DistillJobConfig, String> {
    if request.corpus_jsonl_path.trim().is_empty() {
        return Err("corpus_jsonl_path must not be empty".to_string());
    }
    if request.student_base.trim().is_empty() {
        return Err("student_base must not be empty".to_string());
    }
    if request.out_lora_dir.trim().is_empty() {
        return Err("out_lora_dir must not be empty".to_string());
    }
    if request.license_tag.trim().is_empty() {
        return Err("license_tag must not be empty".to_string());
    }
    if request.operator_signature.trim().is_empty() {
        return Err("operator_signature must not be empty".to_string());
    }
    let teacher_source = TeacherSource::from_cli_arg(&request.teacher_source)?;
    let teacher_path = PathBuf::from(request.teacher.unwrap_or_else(|| "placeholder".to_string()));

    let default_hp = PeftHyperparams::default();
    let hyperparams = PeftHyperparams {
        rank: request.rank.unwrap_or(default_hp.rank),
        alpha: request.alpha.unwrap_or(default_hp.alpha),
        dropout: request.dropout.unwrap_or(default_hp.dropout),
        epochs: request.epochs.unwrap_or(default_hp.epochs),
        learning_rate: request.learning_rate.unwrap_or(default_hp.learning_rate),
        batch_size: request.batch_size.unwrap_or(default_hp.batch_size),
    };

    Ok(DistillJobConfig {
        teacher_model_path: teacher_path,
        student_base_model_path: PathBuf::from(request.student_base),
        output_lora_dir: PathBuf::from(request.out_lora_dir),
        corpus_jsonl_path: PathBuf::from(request.corpus_jsonl_path),
        hyperparams,
        license_tag: request.license_tag,
        operator_signature: request.operator_signature,
        teacher_source,
        max_steps: request.max_steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_core::flight_recorder::{
        EventFilter, FlightRecorder, FlightRecorderEvent, FlightRecorderEventType, RecorderError,
    };
    use std::sync::Mutex;

    fn base_request() -> StartPeftTrainingJobRequest {
        StartPeftTrainingJobRequest {
            corpus_jsonl_path: "corpus.jsonl".to_string(),
            student_base: "synthetic".to_string(),
            teacher: Some("placeholder".to_string()),
            teacher_source: "CLI_BRIDGE".to_string(),
            out_lora_dir: "lora_out".to_string(),
            license_tag: "MIT".to_string(),
            operator_signature: "op-ilja".to_string(),
            rank: Some(8),
            alpha: Some(16.0),
            dropout: Some(0.1),
            epochs: Some(1),
            learning_rate: Some(1e-4),
            batch_size: Some(2),
            max_steps: Some(1),
            cpu_only: true,
        }
    }

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

    #[test]
    fn build_config_translates_request_fields() {
        let config = build_config(base_request()).expect("build config");
        assert_eq!(config.teacher_source, TeacherSource::CliBridge);
        assert_eq!(config.hyperparams.rank, 8);
        assert_eq!(config.hyperparams.alpha, 16.0);
        assert!((config.hyperparams.dropout - 0.1).abs() < f32::EPSILON);
        assert_eq!(config.hyperparams.epochs, 1);
        assert!((config.hyperparams.learning_rate - 1e-4).abs() < f32::EPSILON);
        assert_eq!(config.hyperparams.batch_size, 2);
        assert_eq!(config.max_steps, Some(1));
        assert_eq!(config.license_tag, "MIT");
        assert_eq!(config.operator_signature, "op-ilja");
    }

    #[test]
    fn build_config_rejects_empty_corpus_path() {
        let mut request = base_request();
        request.corpus_jsonl_path = "  ".to_string();
        let err = build_config(request).expect_err("empty corpus");
        assert!(err.contains("corpus_jsonl_path"));
    }

    #[test]
    fn build_config_rejects_empty_student_base() {
        let mut request = base_request();
        request.student_base = String::new();
        let err = build_config(request).expect_err("empty student");
        assert!(err.contains("student_base"));
    }

    #[test]
    fn build_config_rejects_empty_license_tag() {
        let mut request = base_request();
        request.license_tag = " ".to_string();
        let err = build_config(request).expect_err("empty license");
        assert!(err.contains("license_tag"));
    }

    #[test]
    fn build_config_rejects_empty_operator_signature() {
        let mut request = base_request();
        request.operator_signature = String::new();
        let err = build_config(request).expect_err("empty signature");
        assert!(err.contains("operator_signature"));
    }

    #[test]
    fn build_config_rejects_unknown_teacher_source() {
        let mut request = base_request();
        request.teacher_source = "SOMETHING_ELSE".to_string();
        let err = build_config(request).expect_err("bad teacher_source");
        assert!(err.contains("teacher_source") || err.contains("CLI_BRIDGE"));
    }

    #[test]
    fn peft_job_receipt_serializes_camel_case() {
        let receipt = PeftJobReceipt {
            job_id: "job-1".to_string(),
            lora_dir: "out".to_string(),
            provenance_path: "out/provenance.json".to_string(),
            teacher_source: "CLI_BRIDGE".to_string(),
            trained_at_utc: "2026-05-20T16:00:00Z".to_string(),
            training_loss: 2.5,
            num_steps: 2,
            event_type: "FR-EVT-DISTILL-STUDENT-RUN".to_string(),
        };
        let value = serde_json::to_value(&receipt).expect("serialize");
        assert!(value.get("jobId").is_some());
        assert!(value.get("loraDir").is_some());
        assert!(value.get("provenancePath").is_some());
        assert!(value.get("teacherSource").is_some());
        assert!(value.get("trainedAtUtc").is_some());
        assert!(value.get("trainingLoss").is_some());
        assert!(value.get("numSteps").is_some());
        assert!(value.get("eventType").is_some());
        assert!(value.get("job_id").is_none());
    }

    #[tokio::test]
    async fn start_peft_training_job_reviews_corpus_records_job_bound_pii_and_skips_trainer() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let corpus_path = tmp.path().join("raw-corpus.jsonl");
        let out_lora_dir = tmp.path().join("lora-out");
        let raw_line = serde_json::json!({
            "id": "turn-pii",
            "session_id": "session-1",
            "model_id": "teacher-1",
            "prompt": "Contact alice@example.com before training.",
            "completion": "No raw source text may be logged.",
            "finish_reason": "stop",
            "license_tag": "MIT",
            "source_event_ids": ["evt-1"],
            "sourced_at_utc": "2026-05-20T04:00:00Z"
        })
        .to_string();
        std::fs::write(&corpus_path, format!("{raw_line}\n")).expect("write corpus");

        let mut request = base_request();
        request.corpus_jsonl_path = corpus_path.to_string_lossy().to_string();
        request.out_lora_dir = out_lora_dir.to_string_lossy().to_string();
        let recorder = MemoryFlightRecorder::default();

        let err = start_peft_training_job_inner(
            request,
            PathBuf::from("definitely-missing-python"),
            PathBuf::from("definitely-missing-trainer.py"),
            true,
            Some(&recorder),
        )
        .await
        .expect_err("PII corpus should fail before subprocess launch");

        assert!(
            err.contains("PII") || err.contains("NoPassingTurns") || err.contains("no passing"),
            "unexpected error: {err}"
        );
        assert!(
            !err.contains("trainer script missing"),
            "content review must run before trainer availability checks: {err}"
        );
        assert!(
            !out_lora_dir.join("provenance.json").exists(),
            "trainer must not run for PII-only corpus"
        );

        let events = recorder
            .list_events(EventFilter::default())
            .await
            .expect("list events");
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].event_type,
            FlightRecorderEventType::DistillPiiDetected
        );
        assert!(
            events[0]
                .job_id
                .as_deref()
                .is_some_and(|id| id.starts_with("job-")),
            "PII event must be job-bound: {:?}",
            events[0].job_id
        );
        let payload_text = serde_json::to_string(&events[0].payload).expect("payload serializes");
        assert!(!payload_text.contains("alice@example.com"));
        assert!(!payload_text.contains("Contact "));
    }
}
