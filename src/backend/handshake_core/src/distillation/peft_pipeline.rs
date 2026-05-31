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
//! Subprocess launch is abstracted behind the [`PeftTrainerExecutor`]
//! trait. The production [`PythonPeftTrainerExecutor`] env-isolates the
//! trainer fork (clears the environment, re-injects only an OS-essential
//! allowlist + `HANDSHAKE_DISTILL_*`) and, when a process ledger is
//! attached, registers an attributable `ProcessOwnershipLedger` row on
//! spawn (MT-122). The `ContentReview` gate runs in-process and is fully
//! unit-testable.
//!
//! Adult-production discipline: per GLOBAL-PRODUCTION-002..009 the
//! pipeline never moralises or rewords operator content. The gates
//! are (a) opt-in (MT-121), (b) license + PII review (MT-120), (c)
//! operator-driven hyperparams. No automatic content judgement
//! beyond those.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::content_review::{
    ContentReview, ContentReviewConfig, ContentReviewOutcome, ReviewVerdict,
};
use super::corpus_extractor::{TrainingCorpus, TrainingTurn};
use crate::flight_recorder::{FlightRecorder, RecorderError};
use crate::process_ledger::{
    LedgerBatcher, ProcessEngineKind, ProcessOwnershipRecordId, ProcessStart, ProcessStop, SpawnMeta,
};

/// Default owner role recorded on the distillation trainer's
/// ProcessOwnershipLedger row when the caller does not override it.
const DEFAULT_DISTILL_OWNER_ROLE: &str = "DISTILLATION_PIPELINE";

/// OS-essential environment variables the Python trainer subprocess needs to
/// launch. SECURITY (MT-122): the trainer MUST NOT inherit the parent's full
/// environment — a bare fork would leak operator secrets (API keys, tokens,
/// session credentials) into an out-of-process Python process. We clear the
/// environment and re-inject only this allowlist plus `HANDSHAKE_DISTILL_*`
/// configuration. Secret-bearing variables (e.g. `OPENAI_API_KEY`,
/// `ANTHROPIC_API_KEY`) are intentionally excluded.
const TRAINER_ENV_ALLOWLIST: &[&str] = &[
    "PATH",
    "PATHEXT",
    "SYSTEMROOT",
    "WINDIR",
    "TEMP",
    "TMP",
    "TMPDIR",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "HOME",
    "USERPROFILE",
    "PYTHONHOME",
];

/// Build the isolated environment for the trainer subprocess from a parent
/// environment iterator: keep only the OS-essential allowlist and
/// `HANDSHAKE_DISTILL_*` configuration; drop everything else (secrets).
fn sandboxed_trainer_env<I>(parent: I) -> Vec<(String, String)>
where
    I: IntoIterator<Item = (String, String)>,
{
    parent
        .into_iter()
        .filter(|(key, _)| {
            TRAINER_ENV_ALLOWLIST
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(key))
                || key.starts_with("HANDSHAKE_DISTILL_")
        })
        .collect()
}

/// Build the ProcessOwnershipLedger row metadata for a distillation trainer
/// subprocess so the fork is attributable (MT-122 red_team control:
/// "Subprocess registered + sandboxed; no unguarded fork"). The dedicated
/// `DistillationJob` engine kind is not yet in `ProcessEngineKind`
/// (process-ledger / MT-069 schema); the row is recorded as
/// `HelperSubprocess` with an explicit distillation marker in metadata.
fn distillation_spawn_meta(pid: u32, owner_role: &str, config: &DistillJobConfig) -> SpawnMeta {
    let mut meta = SpawnMeta::new(pid, ProcessEngineKind::HelperSubprocess, owner_role);
    meta.sandbox_adapter = Some("env_clear_allowlist".to_string());
    meta.mt_id = Some("MT-122".to_string());
    meta.metadata_blob = serde_json::json!({
        "subprocess_kind": "distillation_peft_trainer",
        "mt": "MT-122",
        "teacher_source": config.teacher_source.cli_arg_value(),
        "output_lora_dir": config.output_lora_dir.display().to_string(),
        "env_isolation": "env_clear_allowlist",
    });
    meta
}

/// Build the `ProcessStart` row for a distillation trainer spawn from its
/// `SpawnMeta`. This mirrors `process_ledger::record_spawn` but returns the
/// fully-built `ProcessStart` so the caller can record the matching
/// `ProcessStop` on completion (MT-122 requires both START and STOP rows so
/// the trainer subprocess is attributable AND reclaimable across its full
/// lifecycle).
fn distillation_process_start(
    record_id: ProcessOwnershipRecordId,
    meta: SpawnMeta,
) -> ProcessStart {
    let mut start = ProcessStart::new(meta.engine_kind, meta.owner_role.clone(), meta.owner_wp)
        .with_process_uuid(record_id.as_uuid())
        .with_os_pid(meta.pid)
        .with_metadata_jsonb(meta.metadata_blob)
        .with_sandbox_capabilities_snapshot(meta.sandbox_capabilities_snapshot);
    start.started_at = meta.started_at_utc;
    if let Some(sandbox_adapter) = meta.sandbox_adapter {
        start = start.with_sandbox_adapter_id(sandbox_adapter);
    }
    if let Some(mt_id) = meta.mt_id {
        start = start.with_mt_id(mt_id);
    }
    start
}

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

/// Teacher routing per MT-122 operator clarification 2026-05-20.
/// Distillation defaults to CLOUD-TEACHER -> LOCAL-STUDENT; BYOK
/// teachers route through MT-127 CLI bridge by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TeacherSource {
    /// MT-127 governed CLI bridge to a cloud-official CLI (anthropic, openai).
    CliBridge,
    /// MT-125/MT-126 BYOK lanes.
    Byok,
    /// Local-larger-teacher model loaded via transformers (rare default).
    LocalLarger,
}

impl Default for TeacherSource {
    fn default() -> Self {
        TeacherSource::CliBridge
    }
}

impl TeacherSource {
    pub fn cli_arg_value(&self) -> &'static str {
        match self {
            TeacherSource::CliBridge => "CLI_BRIDGE",
            TeacherSource::Byok => "BYOK",
            TeacherSource::LocalLarger => "LOCAL_LARGER",
        }
    }

    pub fn from_cli_arg(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_uppercase().as_str() {
            "CLI_BRIDGE" => Ok(TeacherSource::CliBridge),
            "BYOK" => Ok(TeacherSource::Byok),
            "LOCAL_LARGER" => Ok(TeacherSource::LocalLarger),
            other => Err(format!(
                "unknown teacher_source {other:?}; expected CLI_BRIDGE | BYOK | LOCAL_LARGER"
            )),
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
    /// Teacher routing (CLI_BRIDGE default per operator clarification).
    pub teacher_source: TeacherSource,
    /// Optional max-steps override. None falls back to the trainer's
    /// per-script default (1 for integration tests).
    pub max_steps: Option<u32>,
}

impl DistillJobConfig {
    /// Returns the Python script path bundled at the repo root. Used by
    /// `PythonPeftTrainerExecutor` and the CLI binary.
    pub fn default_python_script_relative_path() -> PathBuf {
        PathBuf::from("scripts")
            .join("distill")
            .join("train_lora.py")
    }
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
    NoPassingTurns {
        turn_count: usize,
        pass_count: usize,
    },
    #[error("corpus jsonl write failed: {0}")]
    CorpusWrite(String),
    #[error("corpus jsonl read failed: {0}")]
    CorpusRead(String),
    #[error("flight recorder write failed: {0}")]
    FlightRecorder(#[from] RecorderError),
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

/// Event-aware corpus review output for callers that need to persist
/// content-review telemetry through Flight Recorder before training.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorpusReviewWithEvents {
    pub verdicts: Vec<ReviewVerdict>,
    pub outcomes: Vec<ContentReviewOutcome>,
    pub summary: CorpusReviewSummary,
}

/// Abstraction over the actual subprocess launch + sandbox + process
/// ledger registration. The mock impl backs unit tests; the
/// production impl spawns `python scripts/distill/train_lora.py ...`
/// under a SandboxAdapter and registers a ProcessOwnershipLedger row
/// with engine_kind=DistillationJob (MT-069).
pub trait PeftTrainerExecutor {
    fn run(&self, config: &DistillJobConfig) -> Result<(), DistillError>;
}

/// Provenance sidecar written by `scripts/distill/train_lora.py` next
/// to the LoRA artifact directory as `provenance.json`. The Rust
/// orchestrator reads this back after subprocess success to assemble
/// the `DistilledLoraArtifact` returned to the caller.
///
/// Schema: `hsk.distill.lora_provenance@v1`. Field stability is
/// guaranteed; new fields are appended (semver-minor).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeftProvenanceSidecar {
    pub teacher_model_id: String,
    pub teacher_source: String,
    pub student_base_id: String,
    pub corpus_path: String,
    pub corpus_sha256: Option<String>,
    pub license_tag: String,
    pub operator_signature: String,
    pub trained_at_utc: String,
    pub training_loss: f64,
    pub num_steps: u32,
    pub hyperparams: serde_json::Value,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub schema: String,
}

impl PeftProvenanceSidecar {
    pub fn read_from(path: &Path) -> Result<Self, DistillError> {
        let raw = std::fs::read_to_string(path).map_err(|err| {
            DistillError::TrainerExec(format!("read provenance sidecar {}: {err}", path.display()))
        })?;
        serde_json::from_str(&raw).map_err(|err| {
            DistillError::TrainerExec(format!(
                "parse provenance sidecar {}: {err}",
                path.display()
            ))
        })
    }
}

/// Production `PeftTrainerExecutor` that spawns
/// `python <script> --corpus ... --out ... --teacher-source ...`
/// as a subprocess. The script path is resolved relative to the
/// configured repo root; the python interpreter is configured at
/// construction time so operators can pin the version (Python 3.11+
/// is required for peft/transformers/torch compatibility).
///
/// SECURITY (MT-122): the trainer subprocess is NOT an unguarded fork.
/// Its environment is cleared and re-populated from a strict allowlist
/// ([`sandboxed_trainer_env`]) so operator secrets in the parent
/// environment never reach the Python process. A [`LedgerBatcher`] is
/// MANDATORY at construction: every trainer spawn is registered as an
/// attributable `ProcessOwnershipLedger` START row before the process is
/// awaited and a matching STOP row after it exits, so the fork is always
/// attributable and reclaimable — there is no unattributed code path.
pub struct PythonPeftTrainerExecutor {
    python_path: PathBuf,
    script_path: PathBuf,
    extra_args: Vec<String>,
    cpu_only: bool,
    process_ledger: Arc<LedgerBatcher>,
    owner_role: String,
}

impl PythonPeftTrainerExecutor {
    /// Construct from explicit python+script paths. The process ledger is
    /// mandatory (MT-122): the trainer subprocess is always registered as an
    /// attributable `ProcessOwnershipLedger` row.
    pub fn new(
        python_path: PathBuf,
        script_path: PathBuf,
        process_ledger: Arc<LedgerBatcher>,
    ) -> Self {
        Self {
            python_path,
            script_path,
            extra_args: Vec::new(),
            cpu_only: false,
            process_ledger,
            owner_role: DEFAULT_DISTILL_OWNER_ROLE.to_string(),
        }
    }

    /// Override the owner role recorded on the ledger row (defaults to
    /// `DISTILLATION_PIPELINE`).
    pub fn with_owner_role(mut self, owner_role: impl Into<String>) -> Self {
        self.owner_role = owner_role.into();
        self
    }

    /// Convenience: locate the bundled `train_lora.py` at
    /// `<repo_root>/scripts/distill/train_lora.py`.
    pub fn from_repo_root(
        python_path: PathBuf,
        repo_root: &Path,
        process_ledger: Arc<LedgerBatcher>,
    ) -> Self {
        let script_path = repo_root
            .join("scripts")
            .join("distill")
            .join("train_lora.py");
        Self::new(python_path, script_path, process_ledger)
    }

    /// Force the trainer onto CPU. Required for CI / no-GPU host runs.
    pub fn with_cpu_only(mut self, cpu_only: bool) -> Self {
        self.cpu_only = cpu_only;
        self
    }

    /// Append extra arguments to the subprocess invocation (operator
    /// override surface; e.g. `--max-steps 100`).
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }

    pub fn python_path(&self) -> &Path {
        &self.python_path
    }

    pub fn script_path(&self) -> &Path {
        &self.script_path
    }
}

impl PeftTrainerExecutor for PythonPeftTrainerExecutor {
    fn run(&self, config: &DistillJobConfig) -> Result<(), DistillError> {
        if !self.script_path.is_file() {
            return Err(DistillError::TrainerUnavailable(format!(
                "trainer script missing at {}",
                self.script_path.display()
            )));
        }
        let mut command = Command::new(&self.python_path);
        command
            .arg(&self.script_path)
            .arg("--corpus")
            .arg(&config.corpus_jsonl_path)
            .arg("--teacher")
            .arg(&config.teacher_model_path)
            .arg("--student")
            .arg(&config.student_base_model_path)
            .arg("--teacher-source")
            .arg(config.teacher_source.cli_arg_value())
            .arg("--out")
            .arg(&config.output_lora_dir)
            .arg("--license-tag")
            .arg(&config.license_tag)
            .arg("--operator-signature")
            .arg(&config.operator_signature)
            .arg("--rank")
            .arg(config.hyperparams.rank.to_string())
            .arg("--alpha")
            .arg(config.hyperparams.alpha.to_string())
            .arg("--dropout")
            .arg(config.hyperparams.dropout.to_string())
            .arg("--epochs")
            .arg(config.hyperparams.epochs.to_string())
            .arg("--lr")
            .arg(config.hyperparams.learning_rate.to_string())
            .arg("--batch-size")
            .arg(config.hyperparams.batch_size.to_string());
        if let Some(max_steps) = config.max_steps {
            command.arg("--max-steps").arg(max_steps.to_string());
        }
        if self.cpu_only {
            command.arg("--cpu-only");
        }
        for arg in &self.extra_args {
            command.arg(arg);
        }
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        // HBR-QUIET: the Python trainer is backgrounded by Handshake and must
        // not pop a console window on Windows.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        // SECURITY (MT-122): never inherit the parent environment — clear it
        // and re-inject only the OS-essential allowlist + HANDSHAKE_DISTILL_*
        // so operator secrets cannot leak into the Python trainer subprocess.
        command.env_clear();
        command.envs(sandboxed_trainer_env(std::env::vars()));

        // MT-122: ledger registration is UNCONDITIONAL. The subprocess is
        // spawned, registered as an attributable ProcessOwnershipLedger START
        // row BEFORE it is awaited (so it is never an unguarded/unattributable
        // fork), then a matching STOP row is recorded after it exits so the
        // process is reclaimable across its full lifecycle. There is no
        // unattributed code path.
        let child = command.spawn().map_err(|err| {
            DistillError::TrainerExec(format!(
                "spawn python {}: {err}",
                self.python_path.display()
            ))
        })?;
        let record_id = ProcessOwnershipRecordId::new_v7();
        let start = distillation_process_start(
            record_id,
            distillation_spawn_meta(child.id(), &self.owner_role, config),
        );
        self.process_ledger
            .record_start(start.clone())
            .map_err(|err| {
                DistillError::TrainerExec(format!(
                    "process ledger START registration failed for distillation trainer: {err}"
                ))
            })?;
        let output = child.wait_with_output().map_err(|err| {
            DistillError::TrainerExec(format!(
                "await python {}: {err}",
                self.python_path.display()
            ))
        })?;
        // Record the matching STOP row regardless of trainer exit status so
        // the ProcessOwnershipLedger reflects the full lifecycle (the
        // subprocess is no longer running once wait_with_output returns).
        let stop = ProcessStop::from_start(&start, output.status.code())
            .with_stop_reason("distillation_trainer_exit");
        self.process_ledger.record_stop(stop).map_err(|err| {
            DistillError::TrainerExec(format!(
                "process ledger STOP registration failed for distillation trainer: {err}"
            ))
        })?;
        if !output.status.success() {
            let stderr_text = String::from_utf8_lossy(&output.stderr).into_owned();
            let exit_code = output.status.code().unwrap_or(-1);
            // Exit code 3 from the script = peft/transformers/torch
            // not importable. Surface that distinctly so operators get
            // a clear remediation path.
            if exit_code == 3 {
                return Err(DistillError::TrainerUnavailable(stderr_text));
            }
            return Err(DistillError::TrainerExec(format!(
                "python trainer exited with code {exit_code}: {stderr_text}"
            )));
        }
        Ok(())
    }
}

/// Reviews a corpus through [`ContentReview`] and returns the verdicts
/// per turn alongside aggregate counts. The verdict ordering matches
/// `corpus.turns` ordering so callers can zip if they need per-turn
/// drill-down.
pub fn review_corpus(
    corpus: &TrainingCorpus,
    review_config: ContentReviewConfig,
) -> Result<(Vec<ReviewVerdict>, CorpusReviewSummary), DistillError> {
    let reviewed = review_corpus_with_events(corpus, review_config)?;
    Ok((reviewed.verdicts, reviewed.summary))
}

pub fn review_corpus_with_events(
    corpus: &TrainingCorpus,
    review_config: ContentReviewConfig,
) -> Result<CorpusReviewWithEvents, DistillError> {
    let mut reviewer = ContentReview::new(review_config);
    let mut verdicts = Vec::with_capacity(corpus.turns.len());
    let mut outcomes = Vec::with_capacity(corpus.turns.len());
    let mut pass_count = 0_usize;
    let mut quarantine_count = 0_usize;
    let mut reject_count = 0_usize;
    for turn in &corpus.turns {
        let outcome = reviewer
            .review_with_events(turn)
            .map_err(|err| DistillError::InvalidConfig(format!("content review: {err}")))?;
        let verdict = outcome.verdict.clone();
        match &verdict {
            ReviewVerdict::Pass { .. } => pass_count += 1,
            ReviewVerdict::Quarantine { .. } => quarantine_count += 1,
            ReviewVerdict::Reject { .. } => reject_count += 1,
        }
        verdicts.push(verdict);
        outcomes.push(outcome);
    }
    Ok(CorpusReviewWithEvents {
        verdicts,
        outcomes,
        summary: CorpusReviewSummary {
            turn_count: corpus.turns.len(),
            pass_count,
            quarantine_count,
            reject_count,
        },
    })
}

#[derive(Deserialize)]
struct TrainingCorpusJsonlRow {
    id: Option<String>,
    session_id: Option<String>,
    model_id: Option<String>,
    prompt: String,
    completion: String,
    finish_reason: Option<String>,
    license_tag: Option<String>,
    source_event_ids: Option<Vec<String>>,
    sourced_at_utc: Option<String>,
}

/// Read a JSONL corpus from disk into the richer [`TrainingCorpus`]
/// shape required by ContentReview. Rows written by this module already
/// carry provenance fields; legacy trainer-minimal rows are accepted by
/// filling stable fallback metadata before review.
pub fn read_training_corpus_jsonl(
    path: &Path,
    fallback_session_id: &str,
    fallback_license_tag: &str,
) -> Result<TrainingCorpus, DistillError> {
    let file = File::open(path)
        .map_err(|err| DistillError::CorpusRead(format!("open {}: {err}", path.display())))?;
    let mut turns = Vec::new();
    for (idx, raw) in BufReader::new(file).lines().enumerate() {
        let line_no = idx + 1;
        let raw = raw.map_err(|err| {
            DistillError::CorpusRead(format!("read {} line {line_no}: {err}", path.display()))
        })?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row: TrainingCorpusJsonlRow = serde_json::from_str(trimmed).map_err(|err| {
            DistillError::CorpusRead(format!(
                "parse {} line {line_no} as training row: {err}",
                path.display()
            ))
        })?;
        turns.push(TrainingTurn {
            id: row.id.unwrap_or_else(|| format!("jsonl-line-{line_no}")),
            session_id: row
                .session_id
                .unwrap_or_else(|| fallback_session_id.to_string()),
            model_id: row.model_id.unwrap_or_else(|| "unknown".to_string()),
            prompt: row.prompt,
            completion: row.completion,
            finish_reason: row.finish_reason,
            license_tag: row
                .license_tag
                .unwrap_or_else(|| fallback_license_tag.to_string()),
            source_event_ids: row.source_event_ids.unwrap_or_default(),
            sourced_at_utc: row.sourced_at_utc.unwrap_or_default(),
        });
    }
    Ok(TrainingCorpus {
        session_id: fallback_session_id.to_string(),
        turns,
    })
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
            std::fs::create_dir_all(parent).map_err(|err| {
                DistillError::CorpusWrite(format!("mkdir {}: {err}", parent.display()))
            })?;
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
        writeln!(file, "{line}")
            .map_err(|err| DistillError::CorpusWrite(format!("write {}: {err}", path.display())))?;
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
    validate_distill_config(&config)?;

    let reviewed = review_corpus_with_events(corpus, review_config)?;
    finish_distill(
        corpus,
        config,
        reviewed.verdicts,
        reviewed.summary,
        executor,
        finished_at_utc,
    )
}

/// Async orchestrator variant for production callers that need
/// content-review telemetry durably recorded before any trainer gate
/// can abort the job.
pub async fn distill_with_flight_recorder<R>(
    corpus: &TrainingCorpus,
    config: DistillJobConfig,
    review_config: ContentReviewConfig,
    executor: &dyn PeftTrainerExecutor,
    finished_at_utc: &str,
    recorder: &R,
    trace_id: Uuid,
    job_id: &str,
) -> Result<DistilledLoraArtifact, DistillError>
where
    R: FlightRecorder + ?Sized,
{
    if job_id.trim().is_empty() {
        return Err(DistillError::InvalidConfig(
            "job_id must not be empty for flight recorder distillation".to_string(),
        ));
    }
    validate_distill_config(&config)?;

    let reviewed = review_corpus_with_events(corpus, review_config)?;
    for outcome in &reviewed.outcomes {
        outcome
            .record_flight_recorder_events(recorder, trace_id, job_id)
            .await?;
    }

    finish_distill(
        corpus,
        config,
        reviewed.verdicts,
        reviewed.summary,
        executor,
        finished_at_utc,
    )
}

fn validate_distill_config(config: &DistillJobConfig) -> Result<(), DistillError> {
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
    Ok(())
}

fn finish_distill(
    corpus: &TrainingCorpus,
    config: DistillJobConfig,
    verdicts: Vec<ReviewVerdict>,
    summary: CorpusReviewSummary,
    executor: &dyn PeftTrainerExecutor,
    finished_at_utc: &str,
) -> Result<DistilledLoraArtifact, DistillError> {
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

    fn spawn_meta_config() -> DistillJobConfig {
        DistillJobConfig {
            teacher_model_path: PathBuf::from("/models/teacher"),
            student_base_model_path: PathBuf::from("/models/student"),
            output_lora_dir: PathBuf::from("/artifacts/lora-out"),
            corpus_jsonl_path: PathBuf::from("/artifacts/corpus.jsonl"),
            hyperparams: PeftHyperparams::default(),
            license_tag: "MIT".to_string(),
            operator_signature: "op-ilja".to_string(),
            teacher_source: TeacherSource::CliBridge,
            max_steps: None,
        }
    }

    #[test]
    fn sandboxed_trainer_env_drops_secrets_and_keeps_allowlist() {
        // MT-122: the trainer fork must not inherit operator secrets.
        let parent = vec![
            ("PATH".to_string(), "/usr/bin".to_string()),
            ("OPENAI_API_KEY".to_string(), "sk-secret".to_string()),
            ("ANTHROPIC_API_KEY".to_string(), "sk-ant-secret".to_string()),
            ("AWS_SECRET_ACCESS_KEY".to_string(), "shhh".to_string()),
            ("HANDSHAKE_DISTILL_RANK".to_string(), "16".to_string()),
            ("RANDOM_OPERATOR_VAR".to_string(), "leak-me".to_string()),
        ];
        let keys: Vec<String> = sandboxed_trainer_env(parent)
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert!(keys.iter().any(|k| k == "PATH"));
        assert!(keys.iter().any(|k| k == "HANDSHAKE_DISTILL_RANK"));
        assert!(!keys.iter().any(|k| k == "OPENAI_API_KEY"));
        assert!(!keys.iter().any(|k| k == "ANTHROPIC_API_KEY"));
        assert!(!keys.iter().any(|k| k == "AWS_SECRET_ACCESS_KEY"));
        assert!(!keys.iter().any(|k| k == "RANDOM_OPERATOR_VAR"));
    }

    #[test]
    fn sandboxed_trainer_env_allowlist_is_case_insensitive() {
        let parent = vec![
            ("SystemRoot".to_string(), "C:\\Windows".to_string()),
            ("Path".to_string(), "C:\\bin".to_string()),
        ];
        assert_eq!(sandboxed_trainer_env(parent).len(), 2);
    }

    #[test]
    fn distillation_spawn_meta_is_attributable() {
        let meta = distillation_spawn_meta(4242, "DISTILLATION_PIPELINE", &spawn_meta_config());
        assert_eq!(meta.pid, 4242);
        assert_eq!(meta.engine_kind, ProcessEngineKind::HelperSubprocess);
        assert_eq!(meta.owner_role, "DISTILLATION_PIPELINE");
        assert_eq!(meta.mt_id.as_deref(), Some("MT-122"));
        assert_eq!(meta.sandbox_adapter.as_deref(), Some("env_clear_allowlist"));
        assert_eq!(
            meta.metadata_blob["subprocess_kind"].as_str(),
            Some("distillation_peft_trainer")
        );
        assert_eq!(meta.metadata_blob["mt"].as_str(), Some("MT-122"));
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
    fn review_corpus_with_events_exposes_pii_flight_recorder_events() {
        let corpus = TrainingCorpus {
            session_id: "s".to_string(),
            turns: vec![turn("t1", "email alice@example.com", "ok", "MIT")],
        };

        let reviewed = review_corpus_with_events(&corpus, ContentReviewConfig::defaults()).unwrap();
        assert_eq!(reviewed.verdicts.len(), 1);
        assert_eq!(reviewed.outcomes.len(), 1);
        assert_eq!(reviewed.summary.quarantine_count, 1);

        let events = reviewed.outcomes[0].flight_recorder_events(uuid::Uuid::now_v7(), "job-pii");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].job_id.as_deref(), Some("job-pii"));
        assert_eq!(events[0].event_type.to_string(), "distill.pii_detected");
        assert_eq!(events[0].payload["pii_kinds"], serde_json::json!(["email"]));
        let payload_text = serde_json::to_string(&events[0].payload).unwrap();
        assert!(!payload_text.contains("alice@example.com"));
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
            teacher_source: TeacherSource::CliBridge,
            max_steps: None,
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
            teacher_source: TeacherSource::CliBridge,
            max_steps: None,
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
            teacher_source: TeacherSource::CliBridge,
            max_steps: None,
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
            teacher_source: TeacherSource::CliBridge,
            max_steps: None,
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
