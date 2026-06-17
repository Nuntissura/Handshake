//! WP-KERNEL-009 MT-260 UnifiedWorkSurface-260-AILoomJobs (GAP-LM-011).
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11 — "AI edit
//! proposals, graph mutation proposals, relationship extraction, auto-linking,
//! auto-tagging, and manual edits MUST leave actor, source span, state-vector,
//! validation, denial, or promotion receipts."
//!
//! AI Loom jobs run the operator's CONFIGURED model (`AppState.llm_client`,
//! the `LlmClient` trait) over LoomBlocks to produce auto-tag / auto-caption /
//! link suggestions. EVERY suggestion lands as a PENDING row in
//! `loom_ai_suggestions` (kernel-event-backed: AI_EDIT_PROPOSAL_RECORDED), with
//! full model attribution (model, version, token usage, trace_id) and
//! prompt/output hashes. Nothing becomes authority until an operator/validator
//! confirms — see [`promotion`].
//!
//! No-model path: when the configured client declines (`DisabledLlmClient` ->
//! typed `LlmError`), the job runner returns [`LoomAiJobError::NoModel`] and
//! writes ZERO rows. There is NO fabricated/canned suggestion fallback.

pub mod promotion;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::kernel::crdt::actor_site::KnowledgeActorIdV1;
use crate::kernel::{KernelEventType, NewKernelEvent};
use crate::llm::{CompletionRequest, LlmClient, LlmError};
use crate::storage::loom_ai::{
    insert_loom_ai_suggestion, new_job_id, new_suggestion_id, LoomAiJobKind, LoomAiSuggestionRow,
    NewLoomAiSuggestion,
};
use crate::storage::{Database, LoomBlock, StorageError};

pub const LOOM_AI_SUGGESTION_SCHEMA_ID: &str = "hsk.loom.ai_suggestion@1";

/// Typed errors for running an AI Loom job.
#[derive(Debug)]
pub enum LoomAiJobError {
    /// The configured model declined (no model configured / provider error).
    /// The job wrote ZERO rows. The HTTP layer maps this to
    /// `HSK-409-LOOM-AI-NO-MODEL`.
    NoModel { reason: String },
    /// A durable storage / EventLedger failure.
    Storage(StorageError),
    /// Internal (serialization, event build).
    Internal(String),
}

impl std::fmt::Display for LoomAiJobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoModel { reason } => write!(f, "no model configured: {reason}"),
            Self::Storage(err) => write!(f, "storage error: {err}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for LoomAiJobError {}

impl From<StorageError> for LoomAiJobError {
    fn from(err: StorageError) -> Self {
        Self::Storage(err)
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Stable idempotency hash over the suggestion's identifying tuple.
fn value_hash(kind: LoomAiJobKind, block_id: &str, target: Option<&str>, value: &Value) -> String {
    let canonical = json!({
        "kind": kind.as_str(),
        "block": block_id,
        "target": target,
        "value": value,
    });
    sha256_hex(
        serde_json::to_vec(&canonical)
            .unwrap_or_default()
            .as_slice(),
    )
}

/// Request to run an AI Loom job over a set of blocks.
#[derive(Debug, Clone)]
pub struct LoomAiJobRequest {
    pub workspace_id: String,
    pub kind: LoomAiJobKind,
    /// The blocks to process (job scope). For `link_suggest`, each block's
    /// suggested links target the OTHER blocks in this set.
    pub blocks: Vec<LoomBlock>,
    /// Optional candidate tag-hub blocks the model may pick from for
    /// `auto_tag` (their titles seed the prompt). Empty => the model proposes
    /// free-form tag names.
    pub tag_candidates: Vec<String>,
    /// Session id for receipts.
    pub session_id: String,
    /// Correlation id stamped on every receipt.
    pub correlation_id: String,
    /// The model actor proposing (must be a model actor; receipts attribute it).
    pub actor: KnowledgeActorIdV1,
}

/// Result of running a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoomAiJobResult {
    pub job_id: String,
    pub kind: String,
    pub suggestions: Vec<LoomAiSuggestionRow>,
}

/// The model output parsed into typed suggestion values.
struct ParsedSuggestion {
    block_id: String,
    target_block_id: Option<String>,
    value: Value,
}

/// Build the prompt for one block + kind.
fn build_prompt(kind: LoomAiJobKind, block: &LoomBlock, req: &LoomAiJobRequest) -> String {
    let title = block.title.as_deref().unwrap_or("(untitled)");
    match kind {
        LoomAiJobKind::AutoTag => {
            let candidates = if req.tag_candidates.is_empty() {
                "(no existing tags; propose concise lowercase tag names)".to_string()
            } else {
                req.tag_candidates.join(", ")
            };
            format!(
                "Suggest ONE tag for the note titled \"{title}\".\n\
                 Existing tags: {candidates}.\n\
                 Reply with ONLY the tag name (a short lowercase word)."
            )
        }
        LoomAiJobKind::AutoCaption => format!(
            "Write a ONE-sentence caption describing the item titled \"{title}\".\n\
             Reply with ONLY the caption text."
        ),
        LoomAiJobKind::LinkSuggest => {
            let others: Vec<&str> = req
                .blocks
                .iter()
                .filter(|b| b.block_id != block.block_id)
                .map(|b| b.title.as_deref().unwrap_or("(untitled)"))
                .collect();
            format!(
                "Given the note titled \"{title}\", which related note should it link to?\n\
                 Candidates: {}.\n\
                 Reply with ONLY the candidate title.",
                others.join(", ")
            )
        }
    }
}

/// Parse the model's raw text into a typed suggestion value for the block.
/// Returns `None` when the model gave nothing usable (e.g. blank, or a
/// link target that matches no candidate) — that block simply gets no
/// suggestion (never a fabricated one).
fn parse_output(kind: LoomAiJobKind, block: &LoomBlock, req: &LoomAiJobRequest, raw: &str) -> Option<ParsedSuggestion> {
    let cleaned = raw.trim().trim_matches('"').trim();
    if cleaned.is_empty() {
        return None;
    }
    match kind {
        LoomAiJobKind::AutoTag => {
            // Take the first token-ish word, lowercased.
            let tag = cleaned
                .split_whitespace()
                .next()
                .unwrap_or(cleaned)
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                .to_lowercase();
            if tag.is_empty() {
                return None;
            }
            Some(ParsedSuggestion {
                block_id: block.block_id.clone(),
                target_block_id: None,
                value: json!({ "tag": tag }),
            })
        }
        LoomAiJobKind::AutoCaption => Some(ParsedSuggestion {
            block_id: block.block_id.clone(),
            target_block_id: None,
            value: json!({ "caption": cleaned }),
        }),
        LoomAiJobKind::LinkSuggest => {
            // Resolve the chosen title to a real candidate block id.
            let lowered = cleaned.to_lowercase();
            let target = req.blocks.iter().find(|b| {
                b.block_id != block.block_id
                    && b.title
                        .as_deref()
                        .map(|t| t.to_lowercase() == lowered || lowered.contains(&t.to_lowercase()))
                        .unwrap_or(false)
            })?;
            Some(ParsedSuggestion {
                block_id: block.block_id.clone(),
                target_block_id: Some(target.block_id.clone()),
                value: json!({ "reason": cleaned }),
            })
        }
    }
}

/// Run an AI Loom job. Calls the real configured model per block, records each
/// resulting suggestion as a PENDING row + AI_EDIT_PROPOSAL_RECORDED event with
/// model attribution. Writes NOTHING to authority (no edges/derived fields).
///
/// No-model path: the FIRST `completion` call failing with an `LlmError`
/// short-circuits the whole job with [`LoomAiJobError::NoModel`] and ZERO rows
/// — the job declines loudly rather than fabricating output.
pub async fn run_loom_ai_job(
    db: &(dyn Database + '_),
    pool: &PgPool,
    llm: &dyn LlmClient,
    req: LoomAiJobRequest,
) -> Result<LoomAiJobResult, LoomAiJobError> {
    let job_id = new_job_id();
    let model_id = llm.profile().model_id.clone();

    // First pass: call the model for every block and collect parsed
    // suggestions. A model decline on ANY call fails the whole job with zero
    // rows (no partial fabricated authority).
    struct Pending {
        parsed: ParsedSuggestion,
        prompt_sha256: String,
        output_sha256: String,
        attribution: Value,
    }
    let mut pending: Vec<Pending> = Vec::new();

    for block in &req.blocks {
        let prompt = build_prompt(req.kind, block, &req);
        let trace_id = Uuid::now_v7();
        let completion = CompletionRequest::new(trace_id, prompt.clone(), model_id.clone());
        let response = match llm.completion(completion).await {
            Ok(resp) => resp,
            Err(LlmError::ProviderError(reason)) => {
                return Err(LoomAiJobError::NoModel { reason });
            }
            Err(other) => {
                return Err(LoomAiJobError::NoModel {
                    reason: other.to_string(),
                });
            }
        };

        let Some(parsed) = parse_output(req.kind, block, &req, &response.text) else {
            continue;
        };
        let attribution = json!({
            "model": model_id,
            "version": llm.profile().max_context_tokens,
            "prompt_tokens": response.usage.prompt_tokens,
            "completion_tokens": response.usage.completion_tokens,
            "total_tokens": response.usage.total_tokens,
            "trace_id": trace_id.to_string(),
            "latency_ms": response.latency_ms,
        });
        pending.push(Pending {
            parsed,
            prompt_sha256: sha256_hex(prompt.as_bytes()),
            output_sha256: sha256_hex(response.text.as_bytes()),
            attribution,
        });
    }

    // Second pass: persist each suggestion + its recorded event. Done after the
    // model pass so a late decline never leaves a half-written job.
    let mut suggestions = Vec::with_capacity(pending.len());
    for item in pending {
        let suggestion_id = new_suggestion_id();
        let vhash = value_hash(
            req.kind,
            &item.parsed.block_id,
            item.parsed.target_block_id.as_deref(),
            &item.parsed.value,
        );

        let event = NewKernelEvent::builder(
            format!("KTR-LOOM-AI-{job_id}"),
            req.session_id.clone(),
            KernelEventType::AiEditProposalRecorded,
            req.actor.to_kernel_actor(),
        )
        .aggregate("loom_ai_suggestion", suggestion_id.clone())
        .idempotency_key(format!("loom-ai:{suggestion_id}:recorded"))
        .correlation_id(req.correlation_id.clone())
        .source_component("loom_ai_job")
        .payload(json!({
            "schema_id": LOOM_AI_SUGGESTION_SCHEMA_ID,
            "suggestion_id": suggestion_id,
            "job_id": job_id,
            "kind": req.kind.as_str(),
            "block_id": item.parsed.block_id,
            "target_block_id": item.parsed.target_block_id,
            "suggested_value": item.parsed.value,
            "model_attribution": item.attribution,
            "prompt_sha256": item.prompt_sha256,
            "output_sha256": item.output_sha256,
            "actor_id": req.actor.canonical(),
        }))
        .build()
        .map_err(|err| LoomAiJobError::Internal(err.to_string()))?;
        let stored = db
            .append_kernel_event(event)
            .await
            .map_err(|err| LoomAiJobError::Internal(err.to_string()))?;

        let row = insert_loom_ai_suggestion(
            pool,
            NewLoomAiSuggestion {
                suggestion_id,
                job_id: job_id.clone(),
                workspace_id: req.workspace_id.clone(),
                kind: req.kind,
                block_id: item.parsed.block_id,
                target_block_id: item.parsed.target_block_id,
                suggested_value: item.parsed.value,
                model_attribution: item.attribution,
                prompt_sha256: item.prompt_sha256,
                output_sha256: item.output_sha256,
                value_hash: vhash,
                recorded_event_id: stored.event_id,
            },
        )
        .await?;
        suggestions.push(row);
    }

    Ok(LoomAiJobResult {
        job_id,
        kind: req.kind.as_str().to_string(),
        suggestions,
    })
}
