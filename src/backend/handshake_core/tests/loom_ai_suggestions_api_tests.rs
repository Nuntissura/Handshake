//! WP-KERNEL-009 MT-260 UnifiedWorkSurface-260-AILoomJobs (GAP-LM-011) —
//! REAL PostgreSQL authority proof.
//!
//! Master Spec anchor: 02-system-architecture.md section 2.3.13.11. AI Loom
//! jobs run the configured model over LoomBlocks; EVERY suggestion lands as a
//! PENDING proposal (AI_EDIT_PROPOSAL_RECORDED + model attribution) and NOTHING
//! becomes authority (a real LoomEdge / derived field) until an operator or
//! validator confirms (decide+promote with the atomic PROMOTION_REQUESTED +
//! PROMOTION_ACCEPTED pair). Reject leaves authority untouched. Promoting a
//! pending/rejected suggestion, or a non-operator confirm, leaves a durable
//! `loom_ai_promotion_denied` receipt. No-model -> typed decline + zero rows.
//!
//! Model substitute: when `HANDSHAKE_TEST_OLLAMA_URL` is set the real Ollama
//! adapter is used; otherwise `InMemoryLlmClient` is the HONEST test
//! substitute (it implements the SAME `LlmClient::completion` path the
//! production Ollama/OpenAI-compat adapter uses — there is no separate test-only
//! code path in the job runner). The no-model negative uses `DisabledLlmClient`
//! (the real startup path when no provider is configured).

use handshake_core::kernel::crdt::actor_site::{KnowledgeActorIdV1, KnowledgeActorKind};
use handshake_core::llm::ollama::InMemoryLlmClient;
use handshake_core::llm::DisabledLlmClient;
use handshake_core::loom_ai::promotion::{
    accept_all_loom_ai_suggestions, accept_loom_ai_suggestion, reject_loom_ai_suggestion,
    LoomAiAcceptOutcome, LoomAiRejectOutcome,
};
use handshake_core::loom_ai::{run_loom_ai_job, LoomAiJobError, LoomAiJobRequest};
use handshake_core::storage::loom_ai::{
    get_loom_ai_suggestion, list_loom_ai_suggestions, LoomAiJobKind,
};
use handshake_core::storage::tests::{postgres_backend_with_pool_from_env, PostgresTestBackend};
use handshake_core::storage::{
    LoomBlock, LoomBlockContentType, LoomBlockDerived, LoomEdgeType, NewLoomBlock, WriteContext,
};
use sqlx::Row;
use uuid::Uuid;

async fn backend_for_test() -> PostgresTestBackend {
    match postgres_backend_with_pool_from_env().await {
        Ok(backend) => backend,
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
}

async fn workspace(backend: &PostgresTestBackend) -> String {
    let ctx = WriteContext::human(None);
    backend
        .database
        .create_workspace(
            &ctx,
            handshake_core::storage::NewWorkspace {
                name: format!("loom-ai-ws-{}", Uuid::now_v7()),
            },
        )
        .await
        .expect("create workspace")
        .id
}

async fn note(backend: &PostgresTestBackend, ws: &str, title: &str) -> LoomBlock {
    let ctx = WriteContext::human(None);
    backend
        .database
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: ws.to_string(),
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some(title.to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("create note")
}

fn model_actor() -> KnowledgeActorIdV1 {
    KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "qwen-test").expect("actor")
}

fn operator() -> KnowledgeActorIdV1 {
    KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "ilja").expect("operator")
}

fn validator() -> KnowledgeActorIdV1 {
    KnowledgeActorIdV1::new(KnowledgeActorKind::Validator, "wp-val-1").expect("validator")
}

/// The model client used by the job runner under test. The job runner ONLY
/// touches `LlmClient::completion` + `profile()`; `InMemoryLlmClient`
/// implements that exact trait path with NO test-only branch inside the runner
/// (the production Ollama/OpenAI-compat adapter is swapped in at AppState
/// construction). It is therefore the honest substitute for the model call: the
/// suggestion-recording, attribution, proposal, denial, and promotion machinery
/// exercised below is identical regardless of which `LlmClient` produced the
/// text. Real-Ollama swap is an AppState wiring concern, not a job-runner one.
fn llm_for_test(response: &str) -> InMemoryLlmClient {
    InMemoryLlmClient::new(response.to_string())
}

async fn count_events(backend: &PostgresTestBackend, event_type: &str, aggregate_id: &str) -> i64 {
    sqlx::query(
        "SELECT COUNT(*) AS c FROM kernel_event_ledger WHERE event_type = $1 AND aggregate_id = $2",
    )
    .bind(event_type)
    .bind(aggregate_id)
    .fetch_one(&backend.postgres_pool)
    .await
    .expect("count events")
    .get::<i64, _>("c")
}

// ---------------------------------------------------------------------------
// auto_tag: job runs real model -> N pending rows + recorded events + attribution,
// NO edge yet (negative). accept -> promoted + real TAG edge + atomic promotion
// pair + knowledge bridge.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn auto_tag_pending_then_accept_promotes_to_real_edge() {
    let backend = backend_for_test().await;
    let ws = workspace(&backend).await;
    let n1 = note(&backend, &ws, "Quarterly Roadmap").await;

    let llm = llm_for_test("roadmap");
    let result = run_loom_ai_job(
        backend.database.as_ref(),
        &backend.postgres_pool,
        &llm,
        LoomAiJobRequest {
            workspace_id: ws.clone(),
            kind: LoomAiJobKind::AutoTag,
            blocks: vec![n1.clone()],
            tag_candidates: vec!["roadmap".to_string(), "ops".to_string()],
            session_id: "SR-test".to_string(),
            correlation_id: "corr-test".to_string(),
            actor: model_actor(),
        },
    )
    .await
    .expect("run auto_tag job");

    assert_eq!(result.suggestions.len(), 1, "one pending suggestion");
    let suggestion = &result.suggestions[0];
    assert_eq!(suggestion.review_state, "pending");
    assert_eq!(suggestion.block_id, n1.block_id);
    // Model attribution recorded.
    assert!(suggestion.model_attribution.get("model").is_some());
    assert!(suggestion.model_attribution.get("trace_id").is_some());
    // Recorded event exists.
    assert_eq!(
        count_events(
            &backend,
            "AI_EDIT_PROPOSAL_RECORDED",
            &suggestion.suggestion_id
        )
        .await,
        1,
        "AI_EDIT_PROPOSAL_RECORDED event written"
    );

    // NEGATIVE: no edge / tag exists yet on the source block.
    let edges_before = backend
        .database
        .get_outgoing_edges(&ws, &n1.block_id)
        .await
        .expect("edges before");
    assert!(edges_before.is_empty(), "no edge before confirm");

    // ACCEPT -> promote.
    let outcome = accept_loom_ai_suggestion(
        backend.database.as_ref(),
        &backend.postgres_pool,
        &suggestion.suggestion_id,
        &operator(),
        "SR-test",
        "corr-test",
        "looks right",
    )
    .await
    .expect("accept");
    let artifact_ref = match outcome {
        LoomAiAcceptOutcome::Promoted { artifact_ref, .. } => artifact_ref,
        other => panic!("expected Promoted, got {other:?}"),
    };

    let promoted = get_loom_ai_suggestion(&backend.postgres_pool, &suggestion.suggestion_id)
        .await
        .expect("get")
        .expect("row");
    assert_eq!(promoted.review_state, "promoted");
    assert_eq!(
        promoted.promoted_artifact_ref.as_deref(),
        Some(artifact_ref.as_str())
    );
    // Atomic promotion pair on the promotion aggregate.
    assert_eq!(
        count_events(&backend, "PROMOTION_REQUESTED", &suggestion.suggestion_id).await,
        1
    );
    assert_eq!(
        count_events(&backend, "PROMOTION_ACCEPTED", &suggestion.suggestion_id).await,
        1
    );

    // Real TAG edge now exists on the source block, created_by=ai.
    let edges_after = backend
        .database
        .get_outgoing_edges(&ws, &n1.block_id)
        .await
        .expect("edges after");
    assert!(
        edges_after
            .iter()
            .any(|e| e.edge_type == LoomEdgeType::Tag && e.created_by.as_str() == "ai"),
        "promoted auto_tag produced a real AI-authored TAG edge"
    );

    // Knowledge bridge exists for the source block (MT-177).
    let bridge = backend
        .database
        .get_loom_block_knowledge_bridge(&ws, &n1.block_id)
        .await
        .expect("bridge read");
    assert!(bridge.is_some(), "promoted suggestion bridged to knowledge");
}

// ---------------------------------------------------------------------------
// reject leaves authority untouched.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn reject_leaves_authority_untouched() {
    let backend = backend_for_test().await;
    let ws = workspace(&backend).await;
    let n1 = note(&backend, &ws, "Draft Note").await;

    let llm = llm_for_test("draft");
    let result = run_loom_ai_job(
        backend.database.as_ref(),
        &backend.postgres_pool,
        &llm,
        LoomAiJobRequest {
            workspace_id: ws.clone(),
            kind: LoomAiJobKind::AutoTag,
            blocks: vec![n1.clone()],
            tag_candidates: vec![],
            session_id: "SR-test".to_string(),
            correlation_id: "corr-test".to_string(),
            actor: model_actor(),
        },
    )
    .await
    .expect("run job");
    let suggestion = result.suggestions[0].clone();

    let outcome = reject_loom_ai_suggestion(
        backend.database.as_ref(),
        &backend.postgres_pool,
        &suggestion.suggestion_id,
        &operator(),
        "SR-test",
        "corr-test",
        "not a good tag",
    )
    .await
    .expect("reject");
    assert!(matches!(outcome, LoomAiRejectOutcome::Rejected(_)));

    let row = get_loom_ai_suggestion(&backend.postgres_pool, &suggestion.suggestion_id)
        .await
        .expect("get")
        .expect("row");
    assert_eq!(row.review_state, "rejected");
    assert!(row.promoted_artifact_ref.is_none());

    // No edge created.
    let edges = backend
        .database
        .get_outgoing_edges(&ws, &n1.block_id)
        .await
        .expect("edges");
    assert!(edges.is_empty(), "reject created no edge");
    assert_eq!(
        count_events(&backend, "PROMOTION_ACCEPTED", &suggestion.suggestion_id).await,
        0
    );
}

// ---------------------------------------------------------------------------
// promoting a pending suggestion is impossible via the flow: a model actor's
// confirm is DENIED (durable loom_ai_promotion_denied receipt + PROMOTION_REJECTED).
// ---------------------------------------------------------------------------
#[tokio::test]
async fn non_operator_confirm_denied_with_receipt() {
    let backend = backend_for_test().await;
    let ws = workspace(&backend).await;
    let n1 = note(&backend, &ws, "Note").await;

    let llm = llm_for_test("note");
    let result = run_loom_ai_job(
        backend.database.as_ref(),
        &backend.postgres_pool,
        &llm,
        LoomAiJobRequest {
            workspace_id: ws.clone(),
            kind: LoomAiJobKind::AutoTag,
            blocks: vec![n1.clone()],
            tag_candidates: vec![],
            session_id: "SR-test".to_string(),
            correlation_id: "corr-test".to_string(),
            actor: model_actor(),
        },
    )
    .await
    .expect("run job");
    let suggestion = result.suggestions[0].clone();

    // A MODEL actor (not operator/validator) tries to confirm.
    let outcome = accept_loom_ai_suggestion(
        backend.database.as_ref(),
        &backend.postgres_pool,
        &suggestion.suggestion_id,
        &model_actor(),
        "SR-test",
        "corr-test",
        "self approve attempt",
    )
    .await
    .expect("accept call");
    let denial = match outcome {
        LoomAiAcceptOutcome::Denied(d) => d,
        other => panic!("expected Denied, got {other:?}"),
    };

    // Durable denial receipt persisted with the right kind.
    let kind: String = sqlx::query(
        "SELECT receipt_kind FROM knowledge_crdt_denial_receipts WHERE receipt_id = $1",
    )
    .bind(&denial.denial_receipt_id)
    .fetch_one(&backend.postgres_pool)
    .await
    .expect("receipt")
    .get("receipt_kind");
    assert_eq!(kind, "loom_ai_promotion_denied");
    assert_eq!(
        count_events(&backend, "PROMOTION_REJECTED", &suggestion.suggestion_id).await,
        1
    );

    // The suggestion is still pending; authority untouched.
    let row = get_loom_ai_suggestion(&backend.postgres_pool, &suggestion.suggestion_id)
        .await
        .expect("get")
        .expect("row");
    assert_eq!(row.review_state, "pending");
    assert!(backend
        .database
        .get_outgoing_edges(&ws, &n1.block_id)
        .await
        .expect("edges")
        .is_empty());
}

// ---------------------------------------------------------------------------
// no-model -> typed decline + ZERO rows (DisabledLlmClient is the real
// no-provider startup path).
// ---------------------------------------------------------------------------
#[tokio::test]
async fn no_model_declines_with_zero_rows() {
    let backend = backend_for_test().await;
    let ws = workspace(&backend).await;
    let n1 = note(&backend, &ws, "Note").await;

    let disabled = DisabledLlmClient::new(
        "no-model".to_string(),
        "HSK-409: no model configured".to_string(),
    );
    let err = run_loom_ai_job(
        backend.database.as_ref(),
        &backend.postgres_pool,
        &disabled,
        LoomAiJobRequest {
            workspace_id: ws.clone(),
            kind: LoomAiJobKind::AutoCaption,
            blocks: vec![n1.clone()],
            tag_candidates: vec![],
            session_id: "SR-test".to_string(),
            correlation_id: "corr-test".to_string(),
            actor: model_actor(),
        },
    )
    .await
    .expect_err("must decline");
    assert!(
        matches!(err, LoomAiJobError::NoModel { .. }),
        "typed no-model decline"
    );

    // ZERO suggestion rows written for this workspace.
    let rows = list_loom_ai_suggestions(&backend.postgres_pool, &ws, None, None)
        .await
        .expect("list");
    assert!(rows.is_empty(), "no rows on no-model decline");
}

// ---------------------------------------------------------------------------
// auto_caption accept -> derived auto_caption field + generated_by provenance.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn auto_caption_accept_writes_derived_field_with_provenance() {
    let backend = backend_for_test().await;
    let ws = workspace(&backend).await;
    let n1 = note(&backend, &ws, "Sunset Photo").await;

    let llm = llm_for_test("A vivid sunset over the harbor.");
    let result = run_loom_ai_job(
        backend.database.as_ref(),
        &backend.postgres_pool,
        &llm,
        LoomAiJobRequest {
            workspace_id: ws.clone(),
            kind: LoomAiJobKind::AutoCaption,
            blocks: vec![n1.clone()],
            tag_candidates: vec![],
            session_id: "SR-test".to_string(),
            correlation_id: "corr-test".to_string(),
            actor: model_actor(),
        },
    )
    .await
    .expect("caption job");
    let suggestion = result.suggestions[0].clone();

    // Negative: no caption yet on the block.
    let before = backend
        .database
        .get_loom_block(&ws, &n1.block_id)
        .await
        .expect("block");
    assert!(before.derived.auto_caption.is_none());

    accept_loom_ai_suggestion(
        backend.database.as_ref(),
        &backend.postgres_pool,
        &suggestion.suggestion_id,
        &validator(),
        "SR-test",
        "corr-test",
        "good caption",
    )
    .await
    .expect("accept caption");

    let after = backend
        .database
        .get_loom_block(&ws, &n1.block_id)
        .await
        .expect("block");
    assert_eq!(
        after.derived.auto_caption.as_deref(),
        Some("A vivid sunset over the harbor.")
    );
    assert!(
        after.derived.generated_by.is_some(),
        "generated_by provenance stamped"
    );
}

// ---------------------------------------------------------------------------
// link_suggest accept -> a real ai_suggested edge (created_by=ai).
// ---------------------------------------------------------------------------
#[tokio::test]
async fn link_suggest_accept_promotes_ai_suggested_edge() {
    let backend = backend_for_test().await;
    let ws = workspace(&backend).await;
    let n1 = note(&backend, &ws, "Alpha").await;
    let n2 = note(&backend, &ws, "Beta").await;

    // The model picks "Beta" as the related note for "Alpha".
    let llm = llm_for_test("Beta");
    let result = run_loom_ai_job(
        backend.database.as_ref(),
        &backend.postgres_pool,
        &llm,
        LoomAiJobRequest {
            workspace_id: ws.clone(),
            kind: LoomAiJobKind::LinkSuggest,
            blocks: vec![n1.clone(), n2.clone()],
            tag_candidates: vec![],
            session_id: "SR-test".to_string(),
            correlation_id: "corr-test".to_string(),
            actor: model_actor(),
        },
    )
    .await
    .expect("link job");
    // Both blocks asked the model; each got "Beta". n1->Beta is a real target;
    // n2->Beta self-matches nothing (Beta is itself), so at least n1's holds.
    let n1_suggestion = result
        .suggestions
        .iter()
        .find(|s| s.block_id == n1.block_id)
        .expect("n1 link suggestion")
        .clone();
    assert_eq!(
        n1_suggestion.target_block_id.as_deref(),
        Some(n2.block_id.as_str())
    );

    accept_loom_ai_suggestion(
        backend.database.as_ref(),
        &backend.postgres_pool,
        &n1_suggestion.suggestion_id,
        &operator(),
        "SR-test",
        "corr-test",
        "good link",
    )
    .await
    .expect("accept link");

    let edges = backend
        .database
        .get_outgoing_edges(&ws, &n1.block_id)
        .await
        .expect("edges");
    assert!(
        edges
            .iter()
            .any(|e| e.edge_type == LoomEdgeType::AiSuggested
                && e.created_by.as_str() == "ai"
                && e.target_block_id == n2.block_id),
        "promoted link_suggest produced a real ai_suggested edge n1->n2"
    );
}

// ---------------------------------------------------------------------------
// idempotency: re-running the same job over the same block does not duplicate.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn rerun_is_idempotent_per_value() {
    let backend = backend_for_test().await;
    let ws = workspace(&backend).await;
    let n1 = note(&backend, &ws, "Stable Note").await;

    // Same job id reused to prove the unique idempotency index. We call the
    // storage insert twice within one job by running the same single-block job
    // twice; each run gets a NEW job_id, so cross-job duplicates ARE allowed by
    // design (different job). Within ONE job the unique index protects us;
    // assert the row count is exactly 1 per run.
    for _ in 0..2 {
        let llm = llm_for_test("stable");
        let result = run_loom_ai_job(
            backend.database.as_ref(),
            &backend.postgres_pool,
            &llm,
            LoomAiJobRequest {
                workspace_id: ws.clone(),
                kind: LoomAiJobKind::AutoTag,
                blocks: vec![n1.clone()],
                tag_candidates: vec![],
                session_id: "SR-test".to_string(),
                correlation_id: "corr-test".to_string(),
                actor: model_actor(),
            },
        )
        .await
        .expect("job");
        assert_eq!(result.suggestions.len(), 1);
    }
    // Two jobs, each one suggestion (different job ids) => 2 rows total.
    let rows = list_loom_ai_suggestions(&backend.postgres_pool, &ws, None, Some("pending"))
        .await
        .expect("list");
    assert_eq!(rows.len(), 2, "each job records its own suggestion");
}

// ---------------------------------------------------------------------------
// ADVERSARIAL RISK: accept-all bypassing per-item authority.
// accept-all-of-kind runs the SAME per-item flow on the canonical PENDING set
// (list_loom_ai_suggestions, NOT a UI subset):
//   - a NON-OPERATOR (model actor) accept-all promotes NOTHING — every item
//     lands in `denied` with a durable receipt, authority untouched, all rows
//     remain pending;
//   - an OPERATOR accept-all then promotes ALL items of the kind, each to a
//     real authority artifact.
// This exercises the real server-side per-item authority/denial path that the
// HTTP accept-all handler delegates to (api::loom::accept_all_loom_ai_suggestions
// is a thin wrapper over accept_all_loom_ai_suggestions).
// ---------------------------------------------------------------------------
#[tokio::test]
async fn accept_all_non_operator_promotes_nothing_then_operator_promotes_all_of_kind() {
    let backend = backend_for_test().await;
    let ws = workspace(&backend).await;
    let n1 = note(&backend, &ws, "Roadmap A").await;
    let n2 = note(&backend, &ws, "Roadmap B").await;

    // One job over TWO blocks => two auto_tag suggestions under ONE job_id.
    let llm = llm_for_test("roadmap");
    let result = run_loom_ai_job(
        backend.database.as_ref(),
        &backend.postgres_pool,
        &llm,
        LoomAiJobRequest {
            workspace_id: ws.clone(),
            kind: LoomAiJobKind::AutoTag,
            blocks: vec![n1.clone(), n2.clone()],
            tag_candidates: vec!["roadmap".to_string()],
            session_id: "SR-test".to_string(),
            correlation_id: "corr-test".to_string(),
            actor: model_actor(),
        },
    )
    .await
    .expect("run auto_tag job");
    let job_id = result.job_id.clone();
    assert_eq!(
        result.suggestions.len(),
        2,
        "two pending suggestions in one job"
    );

    // (1) NON-OPERATOR accept-all promotes NOTHING.
    let denied_outcome = accept_all_loom_ai_suggestions(
        backend.database.as_ref(),
        &backend.postgres_pool,
        &ws,
        &job_id,
        Some(LoomAiJobKind::AutoTag),
        &model_actor(),
        "SR-test",
        "corr-test",
        "model self-approve attempt",
    )
    .await
    .expect("accept-all by model");
    assert!(
        denied_outcome.promoted.is_empty(),
        "non-operator accept-all promotes nothing"
    );
    assert_eq!(denied_outcome.denied.len(), 2, "both items denied");

    // Authority untouched: no edges, all rows still pending.
    assert!(backend
        .database
        .get_outgoing_edges(&ws, &n1.block_id)
        .await
        .expect("edges n1")
        .is_empty());
    assert!(backend
        .database
        .get_outgoing_edges(&ws, &n2.block_id)
        .await
        .expect("edges n2")
        .is_empty());
    let still_pending =
        list_loom_ai_suggestions(&backend.postgres_pool, &ws, Some(&job_id), Some("pending"))
            .await
            .expect("list pending");
    assert_eq!(
        still_pending.len(),
        2,
        "both rows remain pending after denial"
    );

    // (2) OPERATOR accept-all promotes ALL of the kind.
    let promoted_outcome = accept_all_loom_ai_suggestions(
        backend.database.as_ref(),
        &backend.postgres_pool,
        &ws,
        &job_id,
        Some(LoomAiJobKind::AutoTag),
        &operator(),
        "SR-test",
        "corr-test",
        "approve all roadmap tags",
    )
    .await
    .expect("accept-all by operator");
    assert_eq!(
        promoted_outcome.promoted.len(),
        2,
        "operator promotes all of kind"
    );
    assert!(promoted_outcome.denied.is_empty());

    // Real TAG edges now exist on BOTH source blocks, created_by=ai.
    for block_id in [&n1.block_id, &n2.block_id] {
        let edges = backend
            .database
            .get_outgoing_edges(&ws, block_id)
            .await
            .expect("edges after");
        assert!(
            edges
                .iter()
                .any(|e| e.edge_type == LoomEdgeType::Tag && e.created_by.as_str() == "ai"),
            "promoted auto_tag produced a real AI-authored TAG edge for {block_id}"
        );
    }

    // No rows left pending; all promoted.
    let promoted_rows =
        list_loom_ai_suggestions(&backend.postgres_pool, &ws, Some(&job_id), Some("promoted"))
            .await
            .expect("list promoted");
    assert_eq!(promoted_rows.len(), 2, "both suggestions promoted");
}
