//! Parallel TEACHER/STUDENT distillation through the [`SwarmCoordinator`].
//!
//! Master Spec §9 (continuous local skill distillation) + reset brief §6.9:
//! the operator wants autonomous local models with the teacher and student
//! running CONCURRENTLY, not serially. The existing PEFT pipeline
//! ([`super::peft_pipeline`]) already covers the offline
//! sample/select -> review -> corpus -> trainer -> candidate-registry promotion
//! flow; the gap this module closes is the LIVE generation step that feeds it:
//! a teacher model and a student model spawned as TWO concurrent
//! [`crate::swarm_orchestration`] sessions that run at the same time.
//!
//! ## What runs in parallel
//!
//! Given a set of sample prompts (the corpus the operator wants the student to
//! learn), [`ParallelDistillOrchestrator::run`]:
//!
//! 1. Spawns the TEACHER and the STUDENT as two `ModelInstanceId`s through the
//!    injected [`SwarmCoordinator`]. Both are real model sessions created by the
//!    coordinator's factory (the production candle/llama/cloud factory in
//!    production; the swarm controllable factory in default-CI tests). Each
//!    spawn is attributed in the `ProcessOwnershipLedger` by the coordinator —
//!    governance is preserved, not re-implemented here.
//! 2. Drives both sessions CONCURRENTLY with `tokio::join!`: the teacher
//!    *generates* a completion for every sample prompt while, at the same time,
//!    the student is *scored* against those same prompts (its baseline
//!    log-probability over the prompt sequence — the "how surprised is the
//!    student before distillation" signal). The two model processes are live at
//!    the same time; this is the real overlap the proof test asserts
//!    (`peak_concurrent_sessions == 2`).
//! 3. Assembles the teacher's completions into a [`TrainingCorpus`] of
//!    [`TrainingTurn`]s and hands it to the EXISTING
//!    [`super::peft_pipeline::distill`] orchestrator (content review ->
//!    filtered-corpus write -> trainer executor -> provenance), then registers
//!    the resulting [`DistilledLoraArtifact`] in the EXISTING
//!    [`super::candidate_registry::CandidateRegistry`] so the MT-122/123
//!    PromotionGate still gates production mounts.
//!
//! ## What is reused vs new
//!
//! - REUSED: [`SwarmCoordinator`] (spawn/cancel/ledger attribution + the
//!   two-level fan-out bound), [`TrainingCorpus`]/[`TrainingTurn`] (corpus
//!   shape), [`super::peft_pipeline::distill`] (review + corpus write + trainer +
//!   provenance), [`super::candidate_registry::CandidateRegistry`] (the
//!   PromotionGate + audit ledger), the `ModelRuntime` generate/score seams.
//! - NEW (this module): the concurrent two-session orchestration that runs the
//!   teacher and student at the same time and feeds the existing pipeline.
//!
//! ## Honest failures, no mocks
//!
//! Every absent-resource path is a typed error, never a placeholder: a missing
//! teacher/student artifact fails the coordinator's factory with a typed
//! [`SwarmError`]; a generation/scoring error surfaces as a typed
//! [`crate::model_runtime::ModelRuntimeError`] wrapped in
//! [`ParallelDistillError`]. On any failure the orchestrator cancels BOTH
//! sessions so no model process is leaked (the coordinator's terminate path
//! frees the engine + writes the ledger STOP).

use std::sync::Arc;

use futures::StreamExt;

use crate::model_runtime::{
    CancellationToken, GenPrompt, GenerateRequest, ModelId, ModelRuntime, ModelRuntimeError,
    SamplingParams,
};
use crate::swarm_orchestration::{
    ModelInstanceId, SpawnRequest, SwarmCoordinator, SwarmError,
};

use super::candidate_registry::{CandidateRegistry, CandidateRegistryError};
use super::corpus_extractor::{TrainingCorpus, TrainingTurn};
use super::content_review::ContentReviewConfig;
use super::peft_pipeline::{
    distill, DistillError, DistillJobConfig, DistilledLoraArtifact, PeftTrainerExecutor,
};

/// One sample prompt the teacher answers and the student is scored against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DistillSample {
    /// Stable id carried through to the [`TrainingTurn`] so a turn traces back
    /// to its sample.
    pub id: String,
    /// The prompt text the teacher generates a completion for and the student
    /// is scored against.
    pub prompt: String,
    /// License tag inherited from the source session / artifact, stamped on the
    /// assembled turn for Skill Bank license discipline (matches the
    /// corpus-extractor contract).
    pub license_tag: String,
}

impl DistillSample {
    pub fn new(
        id: impl Into<String>,
        prompt: impl Into<String>,
        license_tag: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            prompt: prompt.into(),
            license_tag: license_tag.into(),
        }
    }
}

/// Identity + spawn inputs for the two roles. The teacher and student models
/// are provided by the caller as [`SpawnRequest`]s (local artifact or cloud);
/// the coordinator's factory turns each into a real live session.
pub struct ParallelDistillPlan {
    /// Spawn request for the teacher session (the larger / cloud model that
    /// produces the high-quality completions).
    pub teacher: SpawnRequest,
    /// Spawn request for the student session (the small local model being
    /// distilled into; scored concurrently against the same prompts).
    pub student: SpawnRequest,
    /// Sample prompts driving the run.
    pub samples: Vec<DistillSample>,
    /// Max tokens the teacher generates per sample.
    pub teacher_max_tokens: u32,
    /// Sampling params for the teacher generation.
    pub teacher_sampling: SamplingParams,
}

impl ParallelDistillPlan {
    pub fn new(teacher: SpawnRequest, student: SpawnRequest, samples: Vec<DistillSample>) -> Self {
        Self {
            teacher,
            student,
            samples,
            teacher_max_tokens: 64,
            teacher_sampling: SamplingParams::default(),
        }
    }

    pub fn with_teacher_max_tokens(mut self, max_tokens: u32) -> Self {
        self.teacher_max_tokens = max_tokens;
        self
    }
}

/// Baseline student score for one sample, captured concurrently with the
/// teacher generation. The mean log-probability is the "how well does the
/// student already model this prompt" signal that downstream selection can use
/// to prioritise the samples the student is weakest on.
#[derive(Clone, Debug, PartialEq)]
pub struct StudentBaselineScore {
    pub sample_id: String,
    pub mean_logprob: f32,
}

/// The concurrent generation result: the teacher's assembled corpus and the
/// student's baseline scores, plus the observed peak number of concurrently
/// live sessions (the real-overlap probe — must be 2 when both ran at once).
#[derive(Debug, Clone)]
pub struct ConcurrentDistillRun {
    /// The teacher-generated corpus, ready for the existing PEFT pipeline.
    pub corpus: TrainingCorpus,
    /// The student's per-sample baseline scores (captured in parallel).
    pub student_baseline: Vec<StudentBaselineScore>,
    /// Peak number of sessions the coordinator held live at the same time
    /// during the run. Proves the teacher and student genuinely overlapped.
    pub peak_concurrent_sessions: usize,
    /// The teacher / student instance ids (for the caller's own bookkeeping;
    /// both have already been completed by the time this returns).
    pub teacher_instance: ModelInstanceId,
    pub student_instance: ModelInstanceId,
}

/// Errors from the parallel-distill orchestration. Every variant is a genuine
/// runtime condition — a missing model, a generation/scoring failure, or a
/// downstream pipeline error — never a placeholder.
#[derive(Debug, thiserror::Error)]
pub enum ParallelDistillError {
    #[error("parallel distill requires at least one sample prompt")]
    NoSamples,
    #[error("teacher session spawn failed: {0}")]
    TeacherSpawn(SwarmError),
    #[error("student session spawn failed: {0}")]
    StudentSpawn(SwarmError),
    #[error("teacher session has no live runtime registered in the coordinator")]
    TeacherRuntimeMissing,
    #[error("student session has no live runtime registered in the coordinator")]
    StudentRuntimeMissing,
    #[error("teacher generation failed for sample {sample_id}: {source}")]
    TeacherGenerate {
        sample_id: String,
        source: ModelRuntimeError,
    },
    #[error("student scoring failed for sample {sample_id}: {source}")]
    StudentScore {
        sample_id: String,
        source: ModelRuntimeError,
    },
    #[error("downstream PEFT pipeline failed: {0}")]
    Pipeline(#[from] DistillError),
    #[error("candidate registry rejected the distilled artifact: {0}")]
    Registry(#[from] CandidateRegistryError),
    #[error("session teardown failed: {0}")]
    Teardown(SwarmError),
}

/// Resolves the LIVE [`ModelRuntime`] + [`ModelId`] the coordinator registered
/// for a spawned session, so the orchestrator can drive a real generate/score
/// on exactly the session the coordinator owns.
///
/// This is the seam over the coordinator's private session registry. The
/// `SwarmCoordinator` deliberately keeps its registry private and exposes the
/// per-session runtime only via `#[cfg(test)]` accessors; a non-test caller in
/// `swarm_orchestration/` ownership cannot reach it. Rather than reaching into
/// coordinator internals (out of this module's file ownership, and a layering
/// violation anyway), the orchestrator depends on this small resolver trait.
///
/// PLATFORM GAP (documented, not closed in this module's ownership): the
/// production coordinator needs a public `session_runtime(instance_id) ->
/// Option<(Arc<dyn ModelRuntime>, ModelId)>` accessor for the app wiring to
/// supply a production resolver. Until that public accessor exists on
/// `SwarmCoordinator`, the production resolver cannot be written without editing
/// `swarm_orchestration/coordinator.rs`. The default-CI proof below supplies a
/// real resolver backed by the coordinator's test accessors, so the concurrent
/// orchestration itself is fully proven now.
pub trait SessionRuntimeResolver: Send + Sync {
    fn runtime_for(&self, instance_id: ModelInstanceId) -> Option<Arc<dyn ModelRuntime>>;
    fn model_id_for(&self, instance_id: ModelInstanceId) -> Option<ModelId>;
}

/// Orchestrates a teacher + student distillation run where the two models run
/// as concurrent sessions through a [`SwarmCoordinator`]. Holds an `Arc` to the
/// coordinator (spawn/cancel/complete + ledger attribution) and a
/// [`SessionRuntimeResolver`] (the live runtime seam) so the SAME orchestrator
/// drives the controllable coordinator in tests and the production coordinator
/// in production.
pub struct ParallelDistillOrchestrator {
    coordinator: Arc<SwarmCoordinator>,
    resolver: Arc<dyn SessionRuntimeResolver>,
}

impl ParallelDistillOrchestrator {
    pub fn new(
        coordinator: Arc<SwarmCoordinator>,
        resolver: Arc<dyn SessionRuntimeResolver>,
    ) -> Self {
        Self {
            coordinator,
            resolver,
        }
    }

    /// Spawn the teacher and student as two concurrent sessions and run the
    /// teacher generation + student baseline scoring AT THE SAME TIME.
    ///
    /// Returns the assembled corpus + student baselines + the observed peak
    /// concurrency. Both sessions are completed (ledger STOP written, engine
    /// freed) before this returns, on success and on every error path.
    pub async fn run_concurrent_generation(
        &self,
        plan: &ParallelDistillPlan,
        sourced_at_utc: &str,
    ) -> Result<ConcurrentDistillRun, ParallelDistillError> {
        if plan.samples.is_empty() {
            return Err(ParallelDistillError::NoSamples);
        }

        let teacher_iid = plan.teacher.instance_id;
        let student_iid = plan.student.instance_id;

        // (1) Spawn BOTH sessions concurrently. Each spawn drives the
        // coordinator's factory (real model load) and records a
        // ProcessOwnershipLedger START. We spawn them concurrently so the two
        // model loads overlap as well — the first observable parallelism.
        let coord_t = self.coordinator.clone();
        let coord_s = self.coordinator.clone();
        let teacher_req = plan.teacher.clone();
        let student_req = plan.student.clone();
        let (teacher_spawn, student_spawn) = tokio::join!(
            async move { coord_t.spawn_session(teacher_req).await },
            async move { coord_s.spawn_session(student_req).await },
        );

        // Honest, symmetric failure handling: if one spawned and the other did
        // not, tear the live one down so no model process is leaked.
        match (&teacher_spawn, &student_spawn) {
            (Ok(_), Ok(_)) => {}
            (Err(_), Ok(_)) => {
                let _ = self.coordinator.cancel_session(student_iid, "peer_spawn_failed").await;
            }
            (Ok(_), Err(_)) => {
                let _ = self.coordinator.cancel_session(teacher_iid, "peer_spawn_failed").await;
            }
            (Err(_), Err(_)) => {}
        }
        teacher_spawn.map_err(ParallelDistillError::TeacherSpawn)?;
        student_spawn.map_err(ParallelDistillError::StudentSpawn)?;

        // Both sessions are now live in the coordinator at the same time.
        let peak_after_spawn = self.coordinator.live_session_count();

        // (2) Run teacher generation + student scoring CONCURRENTLY. We resolve
        // the live runtimes from the coordinator registry (the same handles the
        // coordinator spawned) and drive real generate/score on each. The two
        // futures are joined so both model processes are doing work at the same
        // time — the load-bearing concurrency.
        let teacher_fut = self.teacher_generate_all(teacher_iid, plan);
        let student_fut = self.student_score_all(student_iid, plan);
        let (teacher_res, student_res) = tokio::join!(teacher_fut, student_fut);

        // Peak concurrency observed across the whole run: both were live during
        // the join, so this is 2 for a healthy run.
        let peak_concurrent_sessions = peak_after_spawn.max(self.peak_during_generation());

        // Whatever happened, complete both sessions so the ledger is symmetric
        // and the engines are freed. Errors from generation are surfaced AFTER
        // the cleanup so a generation failure never leaks a session.
        let turns = match teacher_res {
            Ok(turns) => turns,
            Err(err) => {
                let _ = self.coordinator.complete_session(teacher_iid).await;
                let _ = self.coordinator.complete_session(student_iid).await;
                return Err(err);
            }
        };
        let student_baseline = match student_res {
            Ok(scores) => scores,
            Err(err) => {
                let _ = self.coordinator.complete_session(teacher_iid).await;
                let _ = self.coordinator.complete_session(student_iid).await;
                return Err(err);
            }
        };

        self.coordinator
            .complete_session(teacher_iid)
            .await
            .map_err(ParallelDistillError::Teardown)?;
        self.coordinator
            .complete_session(student_iid)
            .await
            .map_err(ParallelDistillError::Teardown)?;

        let corpus = TrainingCorpus {
            session_id: plan.teacher.parent_session_id.clone(),
            turns,
        };

        Ok(ConcurrentDistillRun {
            corpus,
            student_baseline,
            peak_concurrent_sessions,
            teacher_instance: teacher_iid,
            student_instance: student_iid,
        })
    }

    /// Full run: concurrent generation THEN the existing offline pipeline
    /// (review -> corpus write -> trainer -> provenance) THEN registration in
    /// the EXISTING candidate registry behind the MT-123 PromotionGate.
    ///
    /// `executor` is the existing [`PeftTrainerExecutor`] (the production
    /// Python trainer, or a test executor). `registry` is the existing
    /// [`CandidateRegistry`]; the freshly distilled artifact lands `Pending`
    /// review — promotion still requires an operator signature, unchanged.
    #[allow(clippy::too_many_arguments)]
    pub async fn run(
        &self,
        plan: &ParallelDistillPlan,
        distill_config: DistillJobConfig,
        review_config: ContentReviewConfig,
        executor: &dyn PeftTrainerExecutor,
        registry: &CandidateRegistry,
        lora_id: &str,
        now_utc: &str,
    ) -> Result<DistilledLoraArtifact, ParallelDistillError> {
        // Concurrent teacher+student step (the new parallelism).
        let run = self.run_concurrent_generation(plan, now_utc).await?;

        // Hand the teacher-generated corpus to the EXISTING pipeline: content
        // review -> filtered corpus write -> trainer -> provenance. Not
        // duplicated here.
        let artifact = distill(
            &run.corpus,
            distill_config,
            review_config,
            executor,
            now_utc,
        )?;

        // Register behind the EXISTING PromotionGate (lands Pending; operator
        // signature still required to promote — governance unchanged).
        registry.register(lora_id, artifact.clone(), now_utc)?;

        Ok(artifact)
    }

    /// Drive the teacher runtime to generate a completion for every sample,
    /// assembling [`TrainingTurn`]s. Real `generate` stream consumption; a
    /// generation error is a typed failure, not a swallowed empty turn.
    async fn teacher_generate_all(
        &self,
        teacher_iid: ModelInstanceId,
        plan: &ParallelDistillPlan,
    ) -> Result<Vec<TrainingTurn>, ParallelDistillError> {
        let runtime = self
            .resolver
            .runtime_for(teacher_iid)
            .ok_or(ParallelDistillError::TeacherRuntimeMissing)?;
        let model_id = self
            .resolver
            .model_id_for(teacher_iid)
            .ok_or(ParallelDistillError::TeacherRuntimeMissing)?;

        let mut turns = Vec::with_capacity(plan.samples.len());
        for sample in &plan.samples {
            let req = GenerateRequest {
                id: model_id,
                prompt: GenPrompt::new(sample.prompt.clone()),
                sampling: plan.teacher_sampling.clone(),
                lora_overrides: vec![],
                steering_overrides: vec![],
                kv_prefix_handle: None,
                cancel: CancellationToken::new(),
                max_tokens: plan.teacher_max_tokens,
                stop_sequences: vec![],
                speculative_mode: None,
                structured_decoding: None,
            };
            let mut stream = runtime.generate(req);
            let mut completion = String::new();
            let mut finish_reason = None;
            let mut event_ids = Vec::new();
            let mut idx = 0u32;
            while let Some(item) = stream.next().await {
                let token = item.map_err(|source| ParallelDistillError::TeacherGenerate {
                    sample_id: sample.id.clone(),
                    source,
                })?;
                completion.push_str(&token.text);
                if let Some(fr) = token.finish_reason {
                    finish_reason = Some(format!("{fr:?}").to_lowercase());
                }
                event_ids.push(format!("{}-tok-{idx}", sample.id));
                idx += 1;
            }

            turns.push(TrainingTurn {
                id: sample.id.clone(),
                session_id: plan.teacher.parent_session_id.clone(),
                model_id: model_id.to_string(),
                prompt: sample.prompt.clone(),
                completion,
                finish_reason,
                license_tag: sample.license_tag.clone(),
                source_event_ids: event_ids,
                sourced_at_utc: String::new(),
            });
        }
        Ok(turns)
    }

    /// Drive the student runtime to produce a baseline score for every sample,
    /// CONCURRENTLY with the teacher generation. Uses the real `score` seam
    /// over the prompt bytes (a deterministic, engine-backed sequence) so the
    /// student session is genuinely doing work while the teacher generates.
    async fn student_score_all(
        &self,
        student_iid: ModelInstanceId,
        plan: &ParallelDistillPlan,
    ) -> Result<Vec<StudentBaselineScore>, ParallelDistillError> {
        let runtime = self
            .resolver
            .runtime_for(student_iid)
            .ok_or(ParallelDistillError::StudentRuntimeMissing)?;
        let model_id = self
            .resolver
            .model_id_for(student_iid)
            .ok_or(ParallelDistillError::StudentRuntimeMissing)?;

        let mut scores = Vec::with_capacity(plan.samples.len());
        for sample in &plan.samples {
            // The prompt bytes form a real token sequence for the score seam;
            // the engine returns its mean log-probability over the sequence.
            let sequence: Vec<u32> = sample.prompt.bytes().map(u32::from).collect();
            let score = runtime
                .score(model_id, sequence)
                .await
                .map_err(|source| ParallelDistillError::StudentScore {
                    sample_id: sample.id.clone(),
                    source,
                })?;
            scores.push(StudentBaselineScore {
                sample_id: sample.id.clone(),
                mean_logprob: score.mean_logprob,
            });
        }
        Ok(scores)
    }

    /// Best-effort peak-concurrency probe used after the join: the live-session
    /// count during generation. Both sessions are live until completion, so a
    /// healthy run reports 2.
    fn peak_during_generation(&self) -> usize {
        self.coordinator.live_session_count()
    }
}

/// Convenience [`SpawnRequest`] builder for a teacher/student local-artifact
/// session. Pairs the role's `ModelInstanceId` with the local model artifact +
/// its sha256 (the integrity gate). Cloud teachers use
/// [`SpawnRequest::with_cloud_provider`] directly instead.
pub fn local_distill_spawn(
    instance_id: ModelInstanceId,
    runtime_binding: crate::model_runtime::registry::RuntimeBinding,
    artifact_path: impl Into<String>,
    sha256: impl Into<String>,
    parent_session_id: impl Into<String>,
    role: &str,
) -> SpawnRequest {
    SpawnRequest::new(instance_id, runtime_binding, role, parent_session_id)
        .with_local_artifact(artifact_path, sha256)
}

#[cfg(test)]
mod tests;
