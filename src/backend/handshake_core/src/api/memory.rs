//! WP-KERNEL-012 MT-109 FEMS memory routes: the backend surfaces the native editors
//! read (relevant-memory pack) and write (review-gated proposal) against.
//!
//! * `GET  /workspaces/{workspace_id}/memory/pack` (AC-109-2) — returns the REAL
//!   [`crate::ace::MemoryPack`] shape (`items[].memory_id` / `memory_class` /
//!   `source_refs`). The native `MemoryClient` (frontend MT-063) reads this; the
//!   response is a pinned contract so the client model can be aligned.
//! * `POST /workspaces/{workspace_id}/memory/proposals` (AC-109-3) — stores a
//!   review-gated memory-write PROPOSAL (`document_id` + selection-range + content-hash
//!   provenance REQUIRED, fail-closed on missing provenance). The proposal lands as
//!   `status='pending_review'`; there is NO path from this route to a committed memory
//!   item — a proposal can NEVER mutate memory directly.
//!
//! All durable writes go through PostgreSQL (`fems_memory_*` tables) plus a durable
//! kernel EventLedger receipt (`ARTIFACT_PROPOSED`). No SQLite anywhere.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::ace::{MemoryPack, MemoryPackBudgets, MemoryPackDeterminismMode, MemoryPolicy};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::{fems_memory, StorageError};
use crate::AppState;

const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
const HSK_HEADER_KERNEL_TASK_RUN_ID: &str = "x-hsk-kernel-task-run-id";
const HSK_HEADER_SESSION_RUN_ID: &str = "x-hsk-session-run-id";

const PROPOSAL_STATUS_PENDING_REVIEW: &str = "pending_review";
const PACK_SCHEMA_VERSION: &str = "fems.memory_pack@0.1";

type ApiError = (StatusCode, Json<Value>);

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/workspaces/:workspace_id/memory/pack",
            get(get_memory_pack),
        )
        .route(
            "/workspaces/:workspace_id/memory/proposals",
            post(create_memory_proposal),
        )
        .with_state(state)
}

fn bad_request(detail: impl Into<String>) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": "bad_request", "detail": detail.into()})),
    )
}

fn storage_error(err: StorageError) -> ApiError {
    match err {
        StorageError::NotFound(what) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "not_found", "detail": what})),
        ),
        StorageError::Validation(detail) => bad_request(detail),
        other => {
            tracing::error!(
                target: "handshake_core::memory_api",
                error = %other,
                "fems_memory_api_internal_error"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal_error"})),
            )
        }
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

// ---------------------------------------------------------------------------
// GET /workspaces/{workspace_id}/memory/pack  (AC-109-2)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
struct PackQuery {
    /// The retrieval context key the native client keys the capsule on (echoed by the
    /// frontend `MemoryClient`). Used as the scope key when present.
    #[serde(default)]
    context: Option<String>,
    /// Explicit scope key override (takes precedence over `context`).
    #[serde(default)]
    scope_key: Option<String>,
}

/// Return the REAL `ace::MemoryPack` for a workspace. When no pack has been stored yet,
/// return a well-formed EMPTY pack (200) rather than a 404 so the native client never
/// mistakes an empty capsule for a missing route.
async fn get_memory_pack(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<PackQuery>,
) -> Result<Json<MemoryPack>, ApiError> {
    let scope = query
        .scope_key
        .as_deref()
        .or(query.context.as_deref());

    let pack = fems_memory::get_latest_memory_pack(&state.postgres_pool, &workspace_id, scope)
        .await
        .map_err(storage_error)?;

    Ok(Json(pack.unwrap_or_else(empty_memory_pack)))
}

/// A well-formed empty `ace::MemoryPack` (no items) for the neutral "no relevant memory"
/// state. Carries the real shape so the client decode path is identical to a full pack.
fn empty_memory_pack() -> MemoryPack {
    let mut pack = MemoryPack {
        schema_version: PACK_SCHEMA_VERSION.to_string(),
        pack_id: format!("PACK-{}", Uuid::now_v7()),
        generated_at: chrono::Utc::now().to_rfc3339(),
        determinism_mode: MemoryPackDeterminismMode::Strict,
        memory_policy: MemoryPolicy::WorkspaceScoped,
        scope_refs: Vec::new(),
        budgets: MemoryPackBudgets {
            max_tokens: 500,
            max_items: 24,
            max_items_per_type: std::collections::BTreeMap::new(),
        },
        items: Vec::new(),
        token_estimate: 0,
        memory_pack_hash: String::new(),
        warnings: Vec::new(),
    };
    if let Ok(hash) = pack.compute_hash() {
        pack.memory_pack_hash = hash;
    }
    pack
}

// ---------------------------------------------------------------------------
// POST /workspaces/{workspace_id}/memory/proposals  (AC-109-3)
// ---------------------------------------------------------------------------

/// The Pillar 12 memory class a proposal targets (bounded vocabulary — unknown classes
/// are rejected at decode time). Mirrors the native `MemoryClass` wire strings.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ProposalClass {
    Episodic,
    Semantic,
    Procedural,
}

impl ProposalClass {
    fn wire(self) -> &'static str {
        match self {
            ProposalClass::Episodic => "episodic",
            ProposalClass::Semantic => "semantic",
            ProposalClass::Procedural => "procedural",
        }
    }
}

/// The source provenance a proposal MUST carry. Mirrors the native
/// `MemorySourceProvenance` exactly (closed field set — `deny_unknown_fields`).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProposalSource {
    document_id: String,
    selection_start: u64,
    selection_end: u64,
    content_hash: String,
    #[serde(default)]
    pane_id: Option<String>,
    #[serde(default)]
    workspace_id: Option<String>,
}

/// The review-gated proposal write body. Closed field set (`deny_unknown_fields`) matching
/// the native `submit_proposal` body so unknown/free-text smuggling is rejected.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProposalRequest {
    class: ProposalClass,
    content: String,
    source: ProposalSource,
    #[serde(default)]
    review_gated: Option<bool>,
    #[serde(default)]
    actor_id: Option<String>,
}

/// The server acknowledgement of a stored proposal (matches the native `ProposalAck`).
#[derive(Debug, Clone, Serialize)]
struct ProposalAck {
    proposal_id: String,
    status: String,
}

/// Store a review-gated proposal. Fail-closed on missing provenance (`document_id` +
/// selection range + 64-hex `content_hash`). The proposal is stored as `pending_review`
/// and a durable `ARTIFACT_PROPOSED` EventLedger receipt is appended. There is NO path
/// from here to a committed memory item.
async fn create_memory_proposal(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<ProposalRequest>,
) -> Result<Json<ProposalAck>, ApiError> {
    // Fail-closed provenance gate (AC-109-3).
    let document_id = request.source.document_id.trim();
    if document_id.is_empty() {
        return Err(bad_request(
            "proposal provenance requires a non-empty document_id",
        ));
    }
    if request.source.selection_end < request.source.selection_start {
        return Err(bad_request(
            "proposal provenance selection range is invalid (selection_end < selection_start)",
        ));
    }
    let content_hash = request.source.content_hash.trim();
    if !is_content_hash(content_hash) {
        return Err(bad_request(
            "proposal provenance requires a 64-char lowercase hex content_hash",
        ));
    }

    let proposal_id = format!("PROP-{}", Uuid::now_v7());
    // The commit is downstream + review-gated: the server ALWAYS records the proposal as
    // pending review and never trusts a client flag to bypass the gate.
    let review_gated = true;

    let proposal_payload = json!({
        "proposal_id": proposal_id,
        "workspace_id": workspace_id,
        "class": request.class.wire(),
        "content": request.content,
        "source": request.source,
        "review_gated": review_gated,
        "status": PROPOSAL_STATUS_PENDING_REVIEW,
        "actor_id": request.actor_id,
    });

    let stored = fems_memory::StoredMemoryProposal {
        proposal_id: proposal_id.clone(),
        workspace_id: workspace_id.clone(),
        document_id: document_id.to_string(),
        selection_start: request.source.selection_start as i64,
        selection_end: request.source.selection_end as i64,
        content_hash: content_hash.to_string(),
        memory_class: request.class.wire().to_string(),
        status: PROPOSAL_STATUS_PENDING_REVIEW.to_string(),
        review_gated,
        proposal: proposal_payload.clone(),
    };

    fems_memory::insert_memory_proposal(&state.postgres_pool, &stored)
        .await
        .map_err(storage_error)?;

    // Durable EventLedger receipt (PostgreSQL/EventLedger authority path). A review-gated
    // proposal is an ARTIFACT_PROPOSED event — it is explicitly NOT a commit.
    let actor_id = header_str(&headers, HSK_HEADER_ACTOR_ID)
        .map(ToOwned::to_owned)
        .or_else(|| request.actor_id.clone())
        .unwrap_or_else(|| "native_editor".to_string());
    let actor = match header_str(&headers, HSK_HEADER_ACTOR_KIND).unwrap_or("human") {
        "operator" | "human" => KernelActor::Operator(actor_id.clone()),
        _ => KernelActor::System(actor_id.clone()),
    };
    let kernel_task_run_id = header_str(&headers, HSK_HEADER_KERNEL_TASK_RUN_ID)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("native-editor-fems-propose-{workspace_id}"));
    let session_run_id = header_str(&headers, HSK_HEADER_SESSION_RUN_ID)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "native-editor-session".to_string());

    let receipt = NewKernelEvent::builder(
        kernel_task_run_id,
        session_run_id,
        KernelEventType::ArtifactProposed,
        actor,
    )
    .aggregate("fems_memory_proposal", proposal_id.clone())
    .idempotency_key(format!("fems-memory-proposal:{proposal_id}"))
    .source_component("fems_memory_proposal_intake")
    .payload(json!({
        "receipt_kind": "fems_memory_write_proposal",
        "proposal_id": proposal_id,
        "workspace_id": workspace_id,
        "document_id": document_id,
        "content_hash": content_hash,
        "memory_class": request.class.wire(),
        "review_gated": review_gated,
        "status": PROPOSAL_STATUS_PENDING_REVIEW,
        "never_editor_direct": true,
    }))
    .build()
    .map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "receipt_build_failed", "detail": err.to_string()})),
        )
    })?;

    state
        .storage
        .append_kernel_event(receipt)
        .await
        .map_err(storage_error)?;

    Ok(Json(ProposalAck {
        proposal_id,
        status: PROPOSAL_STATUS_PENDING_REVIEW.to_string(),
    }))
}

/// A content hash is provenance-valid iff it is exactly 64 lowercase hex chars (the loom
/// canonical SHA-256 primitive the native editor produces).
fn is_content_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

#[cfg(all(test, feature = "duckdb-flight-recorder"))]
mod tests {
    use super::*;
    use crate::ace::{
        FemsSourceRef, FemsSourceRefKind, MemoryPackItem,
    };
    use crate::capabilities::CapabilityRegistry;
    use crate::flight_recorder::duckdb::DuckDbFlightRecorder;
    use crate::llm::{
        CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
    };
    use crate::storage::tests::optional_postgres_backend_with_pool_from_env;
    use crate::workflows::{SessionRegistry, SessionSchedulerConfig};
    use std::sync::Arc;

    struct TestLlmClient {
        profile: ModelProfile,
    }

    impl TestLlmClient {
        fn new() -> Self {
            Self {
                profile: ModelProfile::new("fems-memory-api-test".to_string(), 4096),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmClient for TestLlmClient {
        async fn completion(
            &self,
            _req: CompletionRequest,
        ) -> Result<CompletionResponse, LlmError> {
            Ok(CompletionResponse {
                text: "ok".to_string(),
                usage: TokenUsage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                },
                latency_ms: 0,
            })
        }

        fn profile(&self) -> &ModelProfile {
            &self.profile
        }
    }

    async fn setup_state() -> Result<AppState, Box<dyn std::error::Error>> {
        let backend = optional_postgres_backend_with_pool_from_env()
            .await?
            .expect("managed postgres backend");
        let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(32)?);
        Ok(AppState {
            storage: backend.database,
            postgres_pool: backend.postgres_pool,
            flight_recorder: recorder.clone(),
            diagnostics: recorder,
            llm_client: Arc::new(TestLlmClient::new()),
            capability_registry: Arc::new(CapabilityRegistry::new()),
            session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        })
    }

    fn seeded_pack(workspace_id: &str) -> MemoryPack {
        let item = MemoryPackItem {
            memory_id: "MEM-EP-001".to_string(),
            memory_class: "episodic".to_string(),
            item_type: "note".to_string(),
            summary: "Aria refuses the summons".to_string(),
            content: "The protagonist Aria refuses the royal summons.".to_string(),
            structured: None,
            trust_level: "medium".to_string(),
            confidence: 0.9,
            scope_refs: Vec::new(),
            source_refs: vec![FemsSourceRef {
                kind: FemsSourceRefKind::DocBlock,
                id: "doc-block-42".to_string(),
                hash: Some("a".repeat(64)),
                selector: Some("0:47".to_string()),
                created_at: None,
                classification: None,
            }],
            pinned: false,
            last_verified_at: None,
        };
        let mut pack = MemoryPack {
            schema_version: PACK_SCHEMA_VERSION.to_string(),
            pack_id: format!("PACK-{workspace_id}"),
            generated_at: chrono::Utc::now().to_rfc3339(),
            determinism_mode: MemoryPackDeterminismMode::Strict,
            memory_policy: MemoryPolicy::WorkspaceScoped,
            scope_refs: Vec::new(),
            budgets: MemoryPackBudgets {
                max_tokens: 500,
                max_items: 24,
                max_items_per_type: std::collections::BTreeMap::new(),
            },
            items: vec![item],
            token_estimate: 12,
            memory_pack_hash: String::new(),
            warnings: Vec::new(),
        };
        pack.memory_pack_hash = pack.compute_hash().expect("hash");
        pack
    }

    fn valid_source(document_id: &str, content_hash: &str) -> ProposalSource {
        ProposalSource {
            document_id: document_id.to_string(),
            selection_start: 3,
            selection_end: 9,
            content_hash: content_hash.to_string(),
            pane_id: Some("pane-rich".to_string()),
            workspace_id: Some("WS-PROP".to_string()),
        }
    }

    /// AC-109-2: GET returns the REAL ace::MemoryPack shape; asserted field-by-field so
    /// the native client alignment has a pinned contract.
    #[tokio::test]
    async fn get_memory_pack_returns_real_ace_shape() -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;
        let workspace_id = format!("WS-PACK-{}", Uuid::now_v7());
        let pack = seeded_pack(&workspace_id);
        fems_memory::upsert_memory_pack(&state.postgres_pool, &workspace_id, "", &pack).await?;

        let Json(got) = get_memory_pack(
            State(state.clone()),
            Path(workspace_id.clone()),
            Query(PackQuery::default()),
        )
        .await
        .map_err(|(code, body)| format!("get pack failed: {code} {body:?}"))?;

        // Pack-level shape.
        assert_eq!(got.pack_id, format!("PACK-{workspace_id}"));
        assert_eq!(got.schema_version, PACK_SCHEMA_VERSION);
        assert_eq!(got.items.len(), 1);

        // Item-level shape: the exact fields the native client aligns to.
        let item = &got.items[0];
        assert_eq!(item.memory_id, "MEM-EP-001");
        assert_eq!(item.memory_class, "episodic");
        assert_eq!(item.summary, "Aria refuses the summons");
        assert_eq!(item.source_refs.len(), 1);
        assert_eq!(item.source_refs[0].kind, FemsSourceRefKind::DocBlock);
        assert_eq!(item.source_refs[0].id, "doc-block-42");
        assert_eq!(item.source_refs[0].hash.as_deref(), Some("a".repeat(64).as_str()));
        Ok(())
    }

    /// GET on a workspace with no stored pack returns a well-formed empty pack (200), not
    /// a 404 that the native client would mistake for a missing route.
    #[tokio::test]
    async fn get_memory_pack_empty_when_none() -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;
        let workspace_id = format!("WS-EMPTY-{}", Uuid::now_v7());
        let Json(got) = get_memory_pack(
            State(state.clone()),
            Path(workspace_id),
            Query(PackQuery::default()),
        )
        .await
        .map_err(|(code, body)| format!("get pack failed: {code} {body:?}"))?;
        assert!(got.items.is_empty());
        assert_eq!(got.schema_version, PACK_SCHEMA_VERSION);
        Ok(())
    }

    /// AC-109-3: a valid proposal is stored as pending_review + leaves a durable
    /// ARTIFACT_PROPOSED EventLedger receipt.
    #[tokio::test]
    async fn create_proposal_stores_pending_review_and_receipt(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;
        let workspace_id = format!("WS-PROP-{}", Uuid::now_v7());
        let content_hash = "b".repeat(64);
        let request = ProposalRequest {
            class: ProposalClass::Semantic,
            content: "durable fact".to_string(),
            source: valid_source("doc-1", &content_hash),
            review_gated: Some(true),
            actor_id: Some("actor-7".to_string()),
        };

        let Json(ack) = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(request),
        )
        .await
        .map_err(|(code, body)| format!("proposal failed: {code} {body:?}"))?;

        assert_eq!(ack.status, PROPOSAL_STATUS_PENDING_REVIEW);
        assert!(ack.proposal_id.starts_with("PROP-"));

        // The proposal is durably stored as pending_review with its provenance.
        let stored = fems_memory::get_memory_proposal(&state.postgres_pool, &ack.proposal_id)
            .await?
            .expect("proposal stored");
        assert_eq!(stored.status, PROPOSAL_STATUS_PENDING_REVIEW);
        assert_eq!(stored.document_id, "doc-1");
        assert_eq!(stored.content_hash, content_hash);
        assert_eq!(stored.memory_class, "semantic");
        assert!(stored.review_gated);

        // A durable ARTIFACT_PROPOSED EventLedger receipt was appended.
        let receipt_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_event_ledger WHERE idempotency_key = $1 AND event_type = $2",
        )
        .bind(format!("fems-memory-proposal:{}", ack.proposal_id))
        .bind(KernelEventType::ArtifactProposed.as_str())
        .fetch_one(&state.postgres_pool)
        .await?;
        assert_eq!(receipt_count, 1, "exactly one durable proposal receipt");
        Ok(())
    }

    /// AC-109-3 NEGATIVE: submitting a proposal can NOT mutate committed memory. A
    /// pre-seeded committed item is byte-unchanged and the committed-item count does not
    /// grow after the proposal is submitted.
    #[tokio::test]
    async fn proposal_cannot_mutate_committed_memory(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;
        let workspace_id = format!("WS-NEG-{}", Uuid::now_v7());
        let memory_id = "MEM-COMMITTED-1";
        let committed = json!({
            "memory_id": memory_id,
            "memory_class": "semantic",
            "content": "committed truth",
        });
        fems_memory::upsert_memory_item(&state.postgres_pool, &workspace_id, memory_id, &committed)
            .await?;
        let before = fems_memory::count_memory_items(&state.postgres_pool, &workspace_id).await?;
        assert_eq!(before, 1);

        // Submit a proposal that names the committed memory in its content.
        let request = ProposalRequest {
            class: ProposalClass::Procedural,
            content: format!("overwrite {memory_id} with attacker text"),
            source: valid_source(memory_id, &"c".repeat(64)),
            review_gated: Some(true),
            actor_id: Some("attacker".to_string()),
        };
        let Json(ack) = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(request),
        )
        .await
        .map_err(|(code, body)| format!("proposal failed: {code} {body:?}"))?;

        // The committed item is byte-for-byte unchanged.
        let after_item = fems_memory::get_memory_item(&state.postgres_pool, memory_id)
            .await?
            .expect("committed item still present");
        assert_eq!(after_item, committed, "proposal must not mutate committed memory");

        // No new committed item was created; the count is unchanged.
        let after = fems_memory::count_memory_items(&state.postgres_pool, &workspace_id).await?;
        assert_eq!(after, 1, "proposal must not create a committed memory item");

        // The proposal itself is only pending_review.
        let stored = fems_memory::get_memory_proposal(&state.postgres_pool, &ack.proposal_id)
            .await?
            .expect("proposal stored");
        assert_eq!(stored.status, PROPOSAL_STATUS_PENDING_REVIEW);
        Ok(())
    }

    /// AC-109-3 fail-closed: missing/invalid provenance is rejected with 400 and nothing
    /// is stored.
    #[tokio::test]
    async fn proposal_missing_provenance_fails_closed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = setup_state().await?;
        let workspace_id = format!("WS-FAIL-{}", Uuid::now_v7());

        // Empty content_hash.
        let bad_hash = ProposalRequest {
            class: ProposalClass::Episodic,
            content: "x".to_string(),
            source: valid_source("doc-1", ""),
            review_gated: Some(true),
            actor_id: None,
        };
        let err = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(bad_hash),
        )
        .await
        .expect_err("empty content_hash must be rejected");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);

        // Empty document_id.
        let bad_doc = ProposalRequest {
            class: ProposalClass::Episodic,
            content: "x".to_string(),
            source: valid_source("", &"d".repeat(64)),
            review_gated: Some(true),
            actor_id: None,
        };
        let err = create_memory_proposal(
            State(state.clone()),
            Path(workspace_id.clone()),
            HeaderMap::new(),
            Json(bad_doc),
        )
        .await
        .expect_err("empty document_id must be rejected");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);

        // Nothing was stored for this workspace. (Ensure the table exists first: the
        // fail-closed path returns before any insert, so nothing created it yet.)
        fems_memory::ensure_fems_memory_schema(&state.postgres_pool).await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM fems_memory_proposals WHERE workspace_id = $1",
        )
        .bind(&workspace_id)
        .fetch_one(&state.postgres_pool)
        .await?;
        assert_eq!(count, 0, "no proposal stored on fail-closed provenance");
        Ok(())
    }

    /// Typed rejection: an unknown top-level field or unknown class is rejected at decode
    /// (deny_unknown_fields + closed class vocabulary).
    #[test]
    fn proposal_body_rejects_unknown_field_and_unknown_class() {
        // Unknown top-level field.
        let unknown_field = json!({
            "class": "episodic",
            "content": "x",
            "source": {
                "document_id": "doc-1",
                "selection_start": 0,
                "selection_end": 1,
                "content_hash": "a".repeat(64),
                "pane_id": "p",
                "workspace_id": "w"
            },
            "review_gated": true,
            "actor_id": "a",
            "smuggled": "free text"
        });
        assert!(serde_json::from_value::<ProposalRequest>(unknown_field).is_err());

        // Unknown memory class.
        let unknown_class = json!({
            "class": "working",
            "content": "x",
            "source": {
                "document_id": "doc-1",
                "selection_start": 0,
                "selection_end": 1,
                "content_hash": "a".repeat(64),
                "pane_id": "p",
                "workspace_id": "w"
            }
        });
        assert!(serde_json::from_value::<ProposalRequest>(unknown_class).is_err());
    }
}
