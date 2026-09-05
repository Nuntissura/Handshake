//! WP-CKC-posekit-overhaul (SurrealDB port) MT-061: Facial native command API + intake
//! classification API proof over the real `api::atelier_ckc_intake_facial` Axum router.
//!
//! Ported from the reference `atelier_facial_api_tests.rs` (feat/WP-CKC-posekit-overhaul, MT-029 /
//! MT-031 / MT-055). The reference drove a PostgreSQL-backed intake batch and claimed model-operation
//! leases through the ops-lane HTTP routes; here the intake batch lives in an isolated embedded
//! SurrealDB store and leases are claimed through `AtelierStore::claim_model_lease` directly, so
//! this binary depends only on its own lane router. Every artifact is written through the test
//! workspace ArtifactStore; the test stays quiet (no GUI, no foreground window).

mod atelier_surreal_support;

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use atelier_surreal_support::AtelierSurrealHarness;
use chrono::Utc;
use handshake_core::api::atelier_ckc_intake_facial as lane_api;
use handshake_core::atelier::intake::{
    IntakeBatchMode, IntakeLane, IntakeProfileMode, NewIntakeBatch, NewIntakeItem,
};
use handshake_core::atelier::model_lease::NewModelLeaseClaim;
use handshake_core::atelier::AtelierStore;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::kernel::role_mailbox_claim_lease::{
    RoleMailboxClaimMode, RoleMailboxExecutorKind,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::artifacts::{
    artifact_root_rel, sha256_hex, validate_artifact_content_hash, write_file_artifact,
    ArtifactClassification, ArtifactLayer, ArtifactManifest, ArtifactPayloadKind,
};
use handshake_core::storage::EntityRef;
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use serde_json::{json, Value};
use uuid::Uuid;

/// One workspace root for the whole test binary. `HANDSHAKE_WORKSPACE_ROOT` is process-global and
/// tests run on parallel threads, so every test writes distinct-UUID artifacts into this one root.
fn shared_workspace_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let root = tempfile::tempdir()
            .expect("create isolated facial-api workspace root")
            .into_path();
        std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", &root);
        root
    })
}

#[derive(Default)]
struct NoopRecorder;

#[async_trait]
impl FlightRecorder for NoopRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl DiagnosticsStore for NoopRecorder {
    async fn record_diagnostic(
        &self,
        _diag: Diagnostic,
    ) -> Result<(), handshake_core::storage::StorageError> {
        Ok(())
    }

    async fn list_problems(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<ProblemGroup>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }

    async fn get_diagnostic(
        &self,
        _id: Uuid,
    ) -> Result<Diagnostic, handshake_core::storage::StorageError> {
        Err(handshake_core::storage::StorageError::NotFound(
            "diagnostic",
        ))
    }

    async fn list_diagnostics(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<Diagnostic>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }
}

struct NoopLlmClient {
    profile: ModelProfile,
}

#[async_trait]
impl LlmClient for NoopLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: String::new(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            latency_ms: 0,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

fn app_state(harness: &AtelierSurrealHarness) -> AppState {
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage: harness.database.clone(),
        surreal: harness.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("mt061-facial-api-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

async fn serve(state: AppState) -> (String, reqwest::Client, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener address");
    let server = tokio::spawn(async move {
        axum::serve(listener, lane_api::routes(state))
            .await
            .expect("intake/facial lane API server");
    });
    (format!("http://{addr}"), reqwest::Client::new(), server)
}

async fn seed_intake_batch(store: &AtelierStore) -> Uuid {
    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: format!("mt-029-facial-api-{}", Uuid::new_v4()),
            source_label: "mt-029-facial-api".to_owned(),
            source_ref: None,
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::LooseProfile,
            character_internal_id: None,
            target_character_id: None,
            target_sheet_version_id: None,
            target_collection_id: None,
            resume_cursor: None,
        })
        .await
        .expect("open MT-029 facial intake batch");

    for index in 0..3 {
        store
            .add_intake_item(
                batch.batch_id,
                &NewIntakeItem {
                    source_path: format!("artifact://atelier/intake/mt-029/{index}"),
                    file_name: format!("facial-candidate-{index}.png"),
                    byte_len: 1024 + index,
                    content_hash: Some(format!("sha256:{:064x}", index + 1)),
                },
            )
            .await
            .expect("add MT-029 facial intake item");
    }

    batch.batch_id
}

fn actor(request: reqwest::RequestBuilder, id: &str) -> reqwest::RequestBuilder {
    request.header("x-hsk-actor-id", id)
}

fn operator_actor(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    actor(request, "operator").header("x-hsk-actor-kind", "operator")
}

/// Attach the model-operation lease headers to a request. Facial review mutations are
/// model-operation guarded (MT-022): an agent actor must present a valid
/// `x-hsk-model-lease-id` + `x-hsk-session-id` pair bound to the mutated thread, or the
/// request is rejected before the command runs. Operator-kind callers bypass the lease.
fn model_lease(
    request: reqwest::RequestBuilder,
    claim_id: &str,
    session_id: &str,
) -> reqwest::RequestBuilder {
    request
        .header("x-hsk-model-lease-id", claim_id)
        .header("x-hsk-session-id", session_id)
}

/// Model-operation thread the facial review *session create* command is bound to
/// (mirrors `intake_batch_model_operation_thread_id` in the lane router).
fn intake_batch_model_operation_thread_id(batch_id: Uuid) -> String {
    format!("atelier.intake.batch.{batch_id}")
}

/// Model-operation thread every facial review *session-scoped* command
/// (claim/decision/status/montage/export) is bound to (mirrors
/// `facial_review_session_model_operation_thread_id` in the lane router).
fn facial_review_session_model_operation_thread_id(session_id: &str) -> String {
    format!("atelier.facial.review.session.{session_id}")
}

/// Claim an exclusive model-operation lease on `thread_id` for `actor_id`/`session_id` and return
/// its `claim_id`. The reference went through `POST /atelier/model-ops/leases` (ops lane); this
/// binary claims through the store so it does not depend on another lane's router.
async fn claim_facial_review_lease(
    store: &AtelierStore,
    thread_id: &str,
    actor_id: &str,
    session_id: &str,
) -> String {
    store
        .claim_model_lease(&NewModelLeaseClaim {
            thread_id: thread_id.to_owned(),
            executor_kind: RoleMailboxExecutorKind::LocalLargeModel,
            actor_id: actor_id.to_owned(),
            session_id: session_id.to_owned(),
            claim_mode: RoleMailboxClaimMode::ExclusiveLease,
            ttl_seconds: 900,
            linked_work_packet_id: "WP-CKC-posekit-overhaul".to_owned(),
            linked_micro_task_id: "MT-061".to_owned(),
        })
        .await
        .expect("claim facial-review model-operation lease")
        .claim_id
        .to_string()
}

/// Release an exclusive model-operation lease so a different agent can claim the same thread.
async fn release_facial_review_lease(store: &AtelierStore, claim_id: &str, actor_id: &str) {
    store
        .release_model_lease(
            Uuid::parse_str(claim_id).expect("lease claim id is a UUID"),
            actor_id,
        )
        .await
        .expect("release facial-review model-operation lease");
}

fn assert_artifact_ref(value: &str) {
    assert!(
        value.starts_with("artifact://.handshake/artifacts/L1/") && value.ends_with("/payload"),
        "response must expose a native ArtifactStore payload ref, got {value}"
    );
    assert!(
        !value.contains(":\\") && !value.contains(".GOV") && !value.starts_with("file:"),
        "artifact ref must not leak filesystem or .GOV paths, got {value}"
    );
}

async fn read_facial_receipt_payload(
    client: &reqwest::Client,
    base: &str,
    receipt_ref: &str,
) -> Value {
    let receipt: Value = client
        .get(format!("{base}/atelier/facial/artifacts/read"))
        .query(&[("artifact_ref", receipt_ref)])
        .send()
        .await
        .expect("send Facial command receipt read")
        .json()
        .await
        .expect("parse Facial command receipt read");
    receipt["payload"].clone()
}

async fn assert_non_success_outcome(
    client: &reqwest::Client,
    base: &str,
    envelope: Value,
    command: &str,
    status: &str,
    error_code: &str,
    actor_id: &str,
) -> String {
    assert_eq!(
        envelope["schema_id"],
        json!("hsk.atelier.facial_api.command_response@1")
    );
    assert_eq!(envelope["command"], json!(command));
    assert_eq!(envelope["status"], json!(status));
    assert_eq!(envelope["actor"], json!(actor_id));
    assert_eq!(envelope["error"], json!(error_code));
    assert!(
        !error_code.contains(' ') && !error_code.contains('/') && !error_code.contains(':'),
        "error must be a stable code, got {error_code}"
    );
    assert!(
        envelope["recovery_hint"]
            .as_str()
            .map(|hint| !hint.trim().is_empty())
            .unwrap_or(false),
        "{command} {status} envelope must carry a non-empty recovery_hint"
    );
    assert!(
        envelope["result"].is_null(),
        "{command} {status} envelope must not fabricate a result payload"
    );
    assert!(
        envelope["result_artifact"].is_null(),
        "{command} {status} envelope must not promise a result artifact"
    );
    let receipt_ref = envelope["receipt_ref"]
        .as_str()
        .expect("non-success envelope receipt_ref")
        .to_owned();
    assert_artifact_ref(&receipt_ref);
    assert_artifact_ref(
        envelope["receipt_artifact"]["artifact_ref"]
            .as_str()
            .expect("non-success envelope receipt artifact ref"),
    );
    let receipt = read_facial_receipt_payload(client, base, &receipt_ref).await;
    assert_eq!(receipt["command"], json!(command));
    assert_eq!(receipt["status"], json!(status));
    assert_eq!(receipt["actor"], json!(actor_id));
    assert_eq!(receipt["error"], json!(error_code));
    assert!(
        receipt["recovery_hint"]
            .as_str()
            .map(|hint| !hint.trim().is_empty())
            .unwrap_or(false),
        "{command} receipt must persist the recovery_hint"
    );
    receipt_ref
}

fn write_non_facial_json_artifact() -> String {
    let workspace_root = shared_workspace_root();
    let payload = br#"{"schema_id":"hsk.unrelated.json@1","value":true}"#;
    let artifact_id = Uuid::now_v7();
    let manifest = ArtifactManifest {
        artifact_id,
        layer: ArtifactLayer::L1,
        kind: ArtifactPayloadKind::File,
        mime: "application/json".to_owned(),
        filename_hint: Some("unrelated.json".to_owned()),
        created_at: Utc::now(),
        created_by_job_id: None,
        source_entity_refs: vec![EntityRef {
            entity_kind: "unrelated_test_fixture".to_owned(),
            entity_id: "mt-029".to_owned(),
        }],
        source_artifact_refs: Vec::new(),
        content_hash: sha256_hex(payload),
        size_bytes: payload.len() as u64,
        classification: ArtifactClassification::Low,
        exportable: true,
        retention_ttl_days: None,
        pinned: Some(true),
        hash_basis: Some("mt-029-non-facial-json-fixture".to_owned()),
        hash_exclude_paths: Vec::new(),
    };
    write_file_artifact(workspace_root, &manifest, payload)
        .expect("write non-Facial JSON artifact fixture");
    validate_artifact_content_hash(workspace_root, ArtifactLayer::L1, artifact_id)
        .expect("validate non-Facial JSON fixture");
    format!(
        "artifact://{}/payload",
        artifact_root_rel(ArtifactLayer::L1, artifact_id)
    )
}

#[tokio::test]
async fn atelier_facial_command_routes_round_trip_artifact_backed_review_flow() {
    let _workspace_root = shared_workspace_root();
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let batch_id = seed_intake_batch(store).await;
    let (base, client, server) = serve(app_state(&harness)).await;

    let features: Value = client
        .get(format!("{base}/atelier/facial/features"))
        .send()
        .await
        .expect("send Facial features request")
        .json()
        .await
        .expect("parse Facial features response");
    assert_eq!(
        features["schema_id"],
        json!("hsk.atelier.facial.features@1")
    );
    let route_commands = features["command_routes"]
        .as_array()
        .expect("feature route list")
        .iter()
        .filter_map(|route| route["command"].as_str())
        .collect::<Vec<_>>();
    for command in [
        "atelier.facial.review.session.create",
        "atelier.facial.review.claim",
        "atelier.facial.review.decision",
        "atelier.facial.review.status",
        "atelier.facial.review.montage",
        "atelier.facial.review.export",
    ] {
        assert!(
            route_commands.contains(&command),
            "Facial features route must advertise {command}"
        );
    }
    let session_route = features["command_routes"]
        .as_array()
        .expect("feature route list")
        .iter()
        .find(|route| route["command"] == "atelier.facial.review.session.create")
        .expect("session route metadata");
    assert_eq!(
        session_route["response_schema_id"],
        json!("hsk.atelier.facial_api.command_response@1")
    );
    assert_eq!(
        session_route["result_schema_id"],
        json!("hsk.atelier.facial_review.session@1")
    );
    assert_eq!(
        session_route["output_schema_id"],
        json!("hsk.atelier.facial_api.command_response@1")
    );

    // Facial review commands are model-operation guarded (MT-022): an agent actor must hold an
    // active exclusive lease bound to the mutated thread, or the request is a pre-context
    // transport error (operator-kind bypasses the lease). MT-029 drives real multi-agent
    // attribution (agent-a/b/c), so each agent acquires its own lease on the thread it mutates.
    let batch_thread = intake_batch_model_operation_thread_id(batch_id);
    let agent_a_batch_lease = claim_facial_review_lease(
        store,
        &batch_thread,
        "mt-029-agent-a",
        "mt-029-agent-a-session",
    )
    .await;
    let session_response: Value = model_lease(
        actor(
            client.post(format!(
                "{base}/atelier/intake/batches/{batch_id}/facial/review/session"
            )),
            "mt-029-agent-a",
        ),
        &agent_a_batch_lease,
        "mt-029-agent-a-session",
    )
    .json(&json!({
        "profile": "quality+dedupe+identity+review",
        "shard_count": 2,
        "claim_ttl_seconds": 600
    }))
    .send()
    .await
    .expect("send Facial review session request")
    .json()
    .await
    .expect("parse Facial review session response");
    assert_eq!(
        session_response["schema_id"],
        json!("hsk.atelier.facial_api.command_response@1")
    );
    assert_eq!(
        session_response["result"]["session"]["schema_id"],
        json!("hsk.atelier.facial_review.session@1")
    );
    let session_artifact_ref = session_response["result_artifact"]["artifact_ref"]
        .as_str()
        .expect("session result artifact ref");
    assert_artifact_ref(session_artifact_ref);
    assert_artifact_ref(
        session_response["receipt_artifact"]["artifact_ref"]
            .as_str()
            .expect("session command receipt artifact ref"),
    );
    let session_id = session_response["result"]["session"]["session_id"]
        .as_str()
        .expect("session id")
        .to_owned();
    let session_thread = facial_review_session_model_operation_thread_id(&session_id);
    let agent_a_session_lease = claim_facial_review_lease(
        store,
        &session_thread,
        "mt-029-agent-a",
        "mt-029-agent-a-session",
    )
    .await;

    let read_session: Value = client
        .get(format!("{base}/atelier/facial/artifacts/read"))
        .query(&[("artifact_ref", session_artifact_ref)])
        .send()
        .await
        .expect("send Facial artifact read request")
        .json()
        .await
        .expect("parse Facial artifact read response");
    assert_eq!(
        read_session["payload_schema_id"],
        json!("hsk.atelier.facial_review.session@1")
    );
    assert_eq!(read_session["payload"]["item_count"], json!(3));

    let invalid_read = client
        .get(format!("{base}/atelier/facial/artifacts/read"))
        .query(&[("artifact_ref", "file:///tmp/unsafe.json")])
        .send()
        .await
        .expect("send invalid Facial artifact read request");
    assert!(
        invalid_read.status().is_client_error(),
        "invalid artifact refs must be rejected, got {}",
        invalid_read.status()
    );
    let missing_read = client
        .get(format!("{base}/atelier/facial/artifacts/read"))
        .query(&[(
            "artifact_ref",
            format!(
                "artifact://.handshake/artifacts/L1/{}/payload",
                Uuid::now_v7()
            ),
        )])
        .send()
        .await
        .expect("send missing Facial artifact read request");
    assert_eq!(
        missing_read.status().as_u16(),
        404,
        "an artifact that is not on disk must be 404, never a fabricated body"
    );
    let non_facial_artifact_ref = write_non_facial_json_artifact();
    let non_facial_read = client
        .get(format!("{base}/atelier/facial/artifacts/read"))
        .query(&[("artifact_ref", non_facial_artifact_ref.as_str())])
        .send()
        .await
        .expect("send non-Facial JSON artifact read request");
    assert!(
        non_facial_read.status().is_client_error(),
        "Facial artifact reader must reject non-Facial JSON artifacts, got {}",
        non_facial_read.status()
    );

    let claim_response: Value = model_lease(
        actor(
            client.post(format!("{base}/atelier/facial/review/claims")),
            "mt-029-agent-a",
        ),
        &agent_a_session_lease,
        "mt-029-agent-a-session",
    )
    .json(&json!({
        "session_artifact_ref": session_artifact_ref,
        "existing_claim_artifact_refs": [],
        "decision_artifact_refs": [],
        "shard": 0
    }))
    .send()
    .await
    .expect("send Facial claim request")
    .json()
    .await
    .expect("parse Facial claim response");
    assert_eq!(
        claim_response["result"]["schema_id"],
        json!("hsk.atelier.facial_review.claim@1")
    );
    let claim_artifact_ref = claim_response["result_artifact"]["artifact_ref"]
        .as_str()
        .expect("claim artifact ref");
    assert_artifact_ref(claim_artifact_ref);
    let item_id = claim_response["result"]["work_items"][0]["item_id"]
        .as_str()
        .expect("claimed work item id");
    // agent-b never holds the session-thread lease agent-a is mutating under, so this duplicate
    // shard-0 claim is rejected by the model-operation guard before it reaches the domain.
    let duplicate_claim = actor(
        client.post(format!("{base}/atelier/facial/review/claims")),
        "mt-029-agent-b",
    )
    .json(&json!({
        "session_artifact_ref": session_artifact_ref,
        "existing_claim_artifact_refs": [],
        "decision_artifact_refs": [],
        "shard": 0
    }))
    .send()
    .await
    .expect("send duplicate Facial claim request");
    assert!(
        duplicate_claim.status().is_client_error(),
        "server-authoritative recovery must reject duplicate shard claims even when caller omits existing claim refs, got {}",
        duplicate_claim.status()
    );

    let decision_response: Value = model_lease(
        actor(
            client.post(format!("{base}/atelier/facial/review/decisions")),
            "mt-029-agent-a",
        ),
        &agent_a_session_lease,
        "mt-029-agent-a-session",
    )
    .json(&json!({
        "session_artifact_ref": session_artifact_ref,
        "claim_artifact_ref": claim_artifact_ref,
        "item_id": item_id,
        "decision": "pass",
        "reason": "clean identity and usable quality",
        "tags": ["keeper", "face"],
        "notes": "MT-029 route proof"
    }))
    .send()
    .await
    .expect("send Facial decision request")
    .json()
    .await
    .expect("parse Facial decision response");
    assert_eq!(
        decision_response["result"]["schema_id"],
        json!("hsk.atelier.facial_review.decision@1")
    );
    assert_eq!(
        decision_response["result"]["canonical_decision"],
        json!("accept")
    );
    let decision_artifact_ref = decision_response["result_artifact"]["artifact_ref"]
        .as_str()
        .expect("decision artifact ref");
    assert_artifact_ref(decision_artifact_ref);

    // agent-a is done mutating the session thread; hand the exclusive lease off so agent-b
    // (status/montage) can acquire the same thread.
    release_facial_review_lease(store, &agent_a_session_lease, "mt-029-agent-a").await;
    let agent_b_session_lease = claim_facial_review_lease(
        store,
        &session_thread,
        "mt-029-agent-b",
        "mt-029-agent-b-session",
    )
    .await;

    let status_response: Value = model_lease(
        actor(
            client.post(format!("{base}/atelier/facial/review/status")),
            "mt-029-agent-b",
        ),
        &agent_b_session_lease,
        "mt-029-agent-b-session",
    )
    .json(&json!({
        "session_artifact_ref": session_artifact_ref,
        "claim_artifact_refs": [claim_artifact_ref],
        "decision_artifact_refs": [decision_artifact_ref]
    }))
    .send()
    .await
    .expect("send Facial status request")
    .json()
    .await
    .expect("parse Facial status response");
    assert_eq!(
        status_response["result"]["schema_id"],
        json!("hsk.atelier.facial_review.status@1")
    );
    assert_eq!(status_response["result"]["item_count"], json!(3));
    assert_eq!(status_response["result"]["accepted_count"], json!(1));
    let recovered_status_response: Value = model_lease(
        actor(
            client.post(format!("{base}/atelier/facial/review/status")),
            "mt-029-agent-b",
        ),
        &agent_b_session_lease,
        "mt-029-agent-b-session",
    )
    .json(&json!({
        "session_artifact_ref": session_artifact_ref,
        "claim_artifact_refs": [],
        "decision_artifact_refs": []
    }))
    .send()
    .await
    .expect("send recovered Facial status request")
    .json()
    .await
    .expect("parse recovered Facial status response");
    assert_eq!(
        recovered_status_response["result"]["accepted_count"],
        json!(1),
        "status must recover persisted decisions even when caller supplies no refs"
    );

    let montage_response: Value = model_lease(
        actor(
            client.post(format!("{base}/atelier/facial/review/montage")),
            "mt-029-agent-b",
        ),
        &agent_b_session_lease,
        "mt-029-agent-b-session",
    )
    .json(&json!({
        "session_artifact_ref": session_artifact_ref,
        "decision_artifact_refs": [decision_artifact_ref],
        "page": 0,
        "columns": 2,
        "rows": 2
    }))
    .send()
    .await
    .expect("send Facial montage request")
    .json()
    .await
    .expect("parse Facial montage response");
    assert_eq!(
        montage_response["result"]["schema_id"],
        json!("hsk.atelier.facial_review.montage@1")
    );
    assert!(
        montage_response["result"]["tiles"]
            .as_array()
            .expect("montage tiles")
            .iter()
            .all(|tile| tile["argus_selector"]
                .as_str()
                .unwrap_or("")
                .starts_with("argus://")),
        "montage must expose Argus-addressable tile selectors"
    );

    // Hand the session-thread lease from agent-b to agent-c for the export command.
    release_facial_review_lease(store, &agent_b_session_lease, "mt-029-agent-b").await;
    let agent_c_session_lease = claim_facial_review_lease(
        store,
        &session_thread,
        "mt-029-agent-c",
        "mt-029-agent-c-session",
    )
    .await;

    let export_response: Value = model_lease(
        actor(
            client.post(format!("{base}/atelier/facial/review/export")),
            "mt-029-agent-c",
        ),
        &agent_c_session_lease,
        "mt-029-agent-c-session",
    )
    .json(&json!({
        "session_artifact_ref": session_artifact_ref,
        "decision_artifact_refs": [decision_artifact_ref],
        "dataset_name": "mt-029-facial-route-proof",
        "repeats": 12,
        "allow_partial": true,
        "output_root_ref": "artifact://atelier/exports/mt-029"
    }))
    .send()
    .await
    .expect("send Facial export request")
    .json()
    .await
    .expect("parse Facial export response");
    assert_eq!(
        export_response["result"]["schema_id"],
        json!("hsk.atelier.facial_review.export@1")
    );
    assert_eq!(export_response["result"]["source_mutation"], json!(false));
    assert_eq!(
        export_response["result"]["copy_mode"],
        json!("manifest_only_no_source_mutation")
    );

    server.abort();
    harness.shutdown().await;
}

/// MT-031: the Facial export command must return DURABLE, model-usable command *outcome*
/// envelopes (blocked/degraded/error/succeeded) at HTTP 200 with persisted receipt artifacts,
/// while pre-context failures stay bare HTTP 4xx transport errors.
#[tokio::test]
async fn facial_command_failure_envelopes_are_durable() {
    let _workspace_root = shared_workspace_root();
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let batch_id = seed_intake_batch(store).await;
    let (base, client, server) = serve(app_state(&harness)).await;

    // One agent identity (mt-031-agent) holds one exclusive lease on the intake batch thread
    // (session creation) and one on the review session thread (claim/decision/export). One shard
    // so a single claim exposes all 3 seeded items (deterministic set).
    let batch_thread = intake_batch_model_operation_thread_id(batch_id);
    let agent_batch_lease =
        claim_facial_review_lease(store, &batch_thread, "mt-031-agent", "mt-031-agent-session")
            .await;
    let session_response: Value = model_lease(
        actor(
            client.post(format!(
                "{base}/atelier/intake/batches/{batch_id}/facial/review/session"
            )),
            "mt-031-agent",
        ),
        &agent_batch_lease,
        "mt-031-agent-session",
    )
    .json(&json!({
        "profile": "quality+dedupe+identity+review",
        "shard_count": 1,
        "claim_ttl_seconds": 600
    }))
    .send()
    .await
    .expect("send session request")
    .json()
    .await
    .expect("parse session response");
    let session_artifact_ref = session_response["result_artifact"]["artifact_ref"]
        .as_str()
        .expect("session artifact ref")
        .to_owned();
    assert_artifact_ref(&session_artifact_ref);
    let session_id = session_response["result"]["session"]["session_id"]
        .as_str()
        .expect("session id")
        .to_owned();
    let session_thread = facial_review_session_model_operation_thread_id(&session_id);
    let agent_session_lease = claim_facial_review_lease(
        store,
        &session_thread,
        "mt-031-agent",
        "mt-031-agent-session",
    )
    .await;

    let claim_response: Value = model_lease(
        actor(
            client.post(format!("{base}/atelier/facial/review/claims")),
            "mt-031-agent",
        ),
        &agent_session_lease,
        "mt-031-agent-session",
    )
    .json(&json!({
        "session_artifact_ref": session_artifact_ref,
        "existing_claim_artifact_refs": [],
        "decision_artifact_refs": [],
        "shard": 0
    }))
    .send()
    .await
    .expect("send claim request")
    .json()
    .await
    .expect("parse claim response");
    let claim_artifact_ref = claim_response["result_artifact"]["artifact_ref"]
        .as_str()
        .expect("claim artifact ref")
        .to_owned();
    let item_ids: Vec<String> = claim_response["result"]["work_items"]
        .as_array()
        .expect("claim work items")
        .iter()
        .map(|item| {
            item["item_id"]
                .as_str()
                .expect("claimed item id")
                .to_owned()
        })
        .collect();
    assert_eq!(
        item_ids.len(),
        3,
        "single-shard claim must expose all 3 seeded items"
    );

    // Record exactly ONE decision -> 1 decided, 2 undecided across the session.
    let decision_response: Value = model_lease(
        actor(
            client.post(format!("{base}/atelier/facial/review/decisions")),
            "mt-031-agent",
        ),
        &agent_session_lease,
        "mt-031-agent-session",
    )
    .json(&json!({
        "session_artifact_ref": session_artifact_ref,
        "claim_artifact_ref": claim_artifact_ref,
        "item_id": item_ids[0],
        "decision": "pass",
        "reason": "clean identity and usable quality",
        "tags": ["keeper"]
    }))
    .send()
    .await
    .expect("send decision request")
    .json()
    .await
    .expect("parse decision response");
    let first_decision_ref = decision_response["result_artifact"]["artifact_ref"]
        .as_str()
        .expect("decision artifact ref")
        .to_owned();

    // BLOCKED: allow_partial=false while undecided items remain -> HTTP-200 envelope.
    let blocked_http = model_lease(
        actor(
            client.post(format!("{base}/atelier/facial/review/export")),
            "mt-031-agent",
        ),
        &agent_session_lease,
        "mt-031-agent-session",
    )
    .json(&json!({
        "session_artifact_ref": session_artifact_ref,
        "decision_artifact_refs": [first_decision_ref],
        "dataset_name": "mt-031-blocked",
        "repeats": 10,
        "allow_partial": false,
        "output_root_ref": "artifact://atelier/exports/mt-031"
    }))
    .send()
    .await
    .expect("send blocked export request");
    assert_eq!(
        blocked_http.status().as_u16(),
        200,
        "a blocked export is an HTTP-200 command envelope, not a transport error"
    );
    let blocked: Value = blocked_http.json().await.expect("parse blocked envelope");
    assert_eq!(
        blocked["schema_id"],
        json!("hsk.atelier.facial_api.command_response@1")
    );
    assert_eq!(blocked["status"], json!("blocked"));
    assert_eq!(blocked["error"], json!("undecided_items_block_export"));
    assert_eq!(blocked["actor"], json!("mt-031-agent"));
    assert!(
        blocked["recovery_hint"]
            .as_str()
            .map(|hint| !hint.trim().is_empty())
            .unwrap_or(false),
        "blocked envelope must carry a non-empty recovery_hint"
    );
    assert!(
        blocked["result_artifact"].is_null(),
        "blocked envelope must NOT promise a result artifact"
    );
    let blocked_receipt_ref = blocked["receipt_ref"]
        .as_str()
        .expect("blocked receipt_ref");
    assert_artifact_ref(blocked_receipt_ref);
    assert_artifact_ref(
        blocked["receipt_artifact"]["artifact_ref"]
            .as_str()
            .expect("blocked receipt artifact ref"),
    );
    let blocked_receipt: Value = client
        .get(format!("{base}/atelier/facial/artifacts/read"))
        .query(&[("artifact_ref", blocked_receipt_ref)])
        .send()
        .await
        .expect("send blocked receipt read")
        .json()
        .await
        .expect("parse blocked receipt read");
    assert_eq!(blocked_receipt["payload"]["status"], json!("blocked"));
    assert_eq!(blocked_receipt["payload"]["actor"], json!("mt-031-agent"));
    assert_eq!(
        blocked_receipt["payload"]["error"],
        json!("undecided_items_block_export")
    );

    // DEGRADED: allow_partial=true with undecided items -> HTTP-200 degraded + both artifacts.
    let degraded_http = model_lease(
        actor(
            client.post(format!("{base}/atelier/facial/review/export")),
            "mt-031-agent",
        ),
        &agent_session_lease,
        "mt-031-agent-session",
    )
    .json(&json!({
        "session_artifact_ref": session_artifact_ref,
        "decision_artifact_refs": [first_decision_ref],
        "dataset_name": "mt-031-degraded",
        "repeats": 10,
        "allow_partial": true,
        "output_root_ref": "artifact://atelier/exports/mt-031"
    }))
    .send()
    .await
    .expect("send degraded export request");
    assert_eq!(
        degraded_http.status().as_u16(),
        200,
        "a degraded export is an HTTP-200 command envelope"
    );
    let degraded: Value = degraded_http.json().await.expect("parse degraded envelope");
    assert_eq!(degraded["status"], json!("degraded"));
    let degraded_reasons = degraded["degraded_reasons"]
        .as_array()
        .expect("degraded_reasons array");
    assert!(
        !degraded_reasons.is_empty(),
        "degraded envelope must carry non-empty degraded_reasons"
    );
    assert!(
        degraded_reasons.iter().any(|reason| reason
            .as_str()
            .map(|value| value.starts_with("undecided_items_skipped:"))
            .unwrap_or(false)),
        "degraded_reasons must name the skipped undecided items"
    );
    assert_eq!(
        degraded["result"]["schema_id"],
        json!("hsk.atelier.facial_review.export@1")
    );
    assert_artifact_ref(
        degraded["result_artifact"]["artifact_ref"]
            .as_str()
            .expect("degraded result artifact ref"),
    );
    assert_artifact_ref(
        degraded["receipt_artifact"]["artifact_ref"]
            .as_str()
            .expect("degraded receipt artifact ref"),
    );

    // ERROR: allow_partial=true but a bad dataset_name -> HTTP-200 error envelope, STABLE code.
    let error_http = model_lease(
        actor(
            client.post(format!("{base}/atelier/facial/review/export")),
            "mt-031-agent",
        ),
        &agent_session_lease,
        "mt-031-agent-session",
    )
    .json(&json!({
        "session_artifact_ref": session_artifact_ref,
        "decision_artifact_refs": [first_decision_ref],
        "dataset_name": "bad/name",
        "repeats": 10,
        "allow_partial": true,
        "output_root_ref": "artifact://atelier/exports/mt-031"
    }))
    .send()
    .await
    .expect("send error export request");
    assert_eq!(
        error_http.status().as_u16(),
        200,
        "a command-level error is an HTTP-200 envelope, not a transport error"
    );
    let error_env: Value = error_http.json().await.expect("parse error envelope");
    assert_eq!(error_env["status"], json!("error"));
    let error_code = error_env["error"]
        .as_str()
        .expect("error envelope must carry a stable error code");
    assert!(
        !error_code.trim().is_empty(),
        "error code must be non-empty"
    );
    assert!(
        !error_code.contains('/') && !error_code.contains(' '),
        "error code must be a stable code, never the raw domain string, got {error_code}"
    );
    assert!(
        error_env["result_artifact"].is_null(),
        "error envelope must NOT promise a result artifact"
    );
    assert_artifact_ref(
        error_env["receipt_artifact"]["artifact_ref"]
            .as_str()
            .expect("error receipt artifact ref"),
    );

    // SUCCEEDED guard: decide the remaining 2 items -> clean success, NO degraded_reasons.
    for item_id in item_ids.iter().skip(1) {
        let _decided: Value = model_lease(
            actor(
                client.post(format!("{base}/atelier/facial/review/decisions")),
                "mt-031-agent",
            ),
            &agent_session_lease,
            "mt-031-agent-session",
        )
        .json(&json!({
            "session_artifact_ref": session_artifact_ref,
            "claim_artifact_ref": claim_artifact_ref,
            "item_id": item_id,
            "decision": "pass",
            "reason": "clean identity and usable quality",
            "tags": ["keeper"]
        }))
        .send()
        .await
        .expect("send remaining decision request")
        .json()
        .await
        .expect("parse remaining decision response");
    }
    let succeeded_http = model_lease(
        actor(
            client.post(format!("{base}/atelier/facial/review/export")),
            "mt-031-agent",
        ),
        &agent_session_lease,
        "mt-031-agent-session",
    )
    .json(&json!({
        "session_artifact_ref": session_artifact_ref,
        "decision_artifact_refs": [],
        "dataset_name": "mt-031-succeeded",
        "repeats": 10,
        "allow_partial": false,
        "output_root_ref": "artifact://atelier/exports/mt-031"
    }))
    .send()
    .await
    .expect("send succeeded export request");
    assert_eq!(succeeded_http.status().as_u16(), 200);
    let succeeded: Value = succeeded_http
        .json()
        .await
        .expect("parse succeeded envelope");
    assert_eq!(
        succeeded["status"],
        json!("succeeded"),
        "an all-decided export must be a clean success"
    );
    assert!(
        succeeded
            .get("degraded_reasons")
            .and_then(Value::as_array)
            .map(|reasons| reasons.is_empty())
            .unwrap_or(true),
        "a succeeded envelope must not carry degraded_reasons"
    );
    assert!(
        succeeded.get("error").map(Value::is_null).unwrap_or(true),
        "a succeeded envelope must not carry an error"
    );
    assert_eq!(
        succeeded["result"]["schema_id"],
        json!("hsk.atelier.facial_review.export@1")
    );
    assert_artifact_ref(
        succeeded["result_artifact"]["artifact_ref"]
            .as_str()
            .expect("succeeded result artifact ref"),
    );

    // TRANSPORT DOC: pre-context failures stay bare HTTP 4xx, NOT command envelopes.
    let no_actor = client
        .post(format!("{base}/atelier/facial/review/export"))
        .json(&json!({
            "session_artifact_ref": session_artifact_ref,
            "decision_artifact_refs": [],
            "dataset_name": "mt-031-no-actor",
            "repeats": 10,
            "allow_partial": true,
            "output_root_ref": "artifact://atelier/exports/mt-031"
        }))
        .send()
        .await
        .expect("send export without actor header");
    assert!(
        no_actor.status().is_client_error(),
        "missing actor header must stay a transport 4xx, got {}",
        no_actor.status()
    );
    let bad_ref = actor(
        client.post(format!("{base}/atelier/facial/review/export")),
        "mt-031-agent",
    )
    .json(&json!({
        "session_artifact_ref": "file:///tmp/unsafe.json",
        "decision_artifact_refs": [],
        "dataset_name": "mt-031-bad-ref",
        "repeats": 10,
        "allow_partial": true,
        "output_root_ref": "artifact://atelier/exports/mt-031"
    }))
    .send()
    .await
    .expect("send export with unresolvable session ref");
    assert!(
        bad_ref.status().is_client_error(),
        "unresolvable session artifact ref must stay a transport 4xx, got {}",
        bad_ref.status()
    );

    server.abort();
    harness.shutdown().await;
}

/// MT-055: non-export Facial review commands must use the same durable command-outcome envelope
/// contract as export after request context is established. Post-context command refusals /
/// errors stay HTTP 200 with a stable code, recovery hint, and read-backable receipt;
/// pre-context failures remain transport 4xx.
#[tokio::test]
async fn facial_review_non_export_commands_return_durable_outcome_envelopes() {
    let _workspace_root = shared_workspace_root();
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let empty_batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: format!("mt-055-empty-facial-api-{}", Uuid::new_v4()),
            source_label: "mt-055-empty-facial-api".to_owned(),
            source_ref: None,
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::LooseProfile,
            character_internal_id: None,
            target_character_id: None,
            target_sheet_version_id: None,
            target_collection_id: None,
            resume_cursor: None,
        })
        .await
        .expect("open empty MT-055 Facial intake batch");
    let batch_id = seed_intake_batch(store).await;
    let (base, client, server) = serve(app_state(&harness)).await;

    let unknown_batch_session = operator_actor(client.post(format!(
        "{base}/atelier/intake/batches/{}/facial/review/session",
        Uuid::new_v4()
    )))
    .json(&json!({
        "profile": "quality+dedupe+identity+review",
        "shard_count": 1,
        "claim_ttl_seconds": 600
    }))
    .send()
    .await
    .expect("send unknown-batch Facial review session request");
    assert!(
        unknown_batch_session.status().is_client_error(),
        "unknown session batch_id must stay a pre-context transport 4xx, got {}",
        unknown_batch_session.status()
    );

    let session_error_http = operator_actor(client.post(format!(
        "{base}/atelier/intake/batches/{}/facial/review/session",
        empty_batch.batch_id
    )))
    .json(&json!({
        "profile": "quality+dedupe+identity+review",
        "shard_count": 1,
        "claim_ttl_seconds": 600
    }))
    .send()
    .await
    .expect("send empty-batch Facial review session request");
    let session_error_status = session_error_http.status();
    let session_error_body = session_error_http
        .text()
        .await
        .expect("read session error response body");
    assert_eq!(
        session_error_status.as_u16(),
        200,
        "post-context session command errors must be HTTP-200 durable envelopes; body={session_error_body}"
    );
    let session_error: Value =
        serde_json::from_str(&session_error_body).expect("parse session error envelope");
    assert_non_success_outcome(
        &client,
        &base,
        session_error,
        "atelier.facial.review.session.create",
        "error",
        "session_command_failed",
        "operator",
    )
    .await;

    let session_response: Value = operator_actor(client.post(format!(
        "{base}/atelier/intake/batches/{batch_id}/facial/review/session"
    )))
    .json(&json!({
        "profile": "quality+dedupe+identity+review",
        "shard_count": 1,
        "claim_ttl_seconds": 600
    }))
    .send()
    .await
    .expect("send seeded Facial review session request")
    .json()
    .await
    .expect("parse seeded session response");
    assert_eq!(session_response["status"], json!("succeeded"));
    assert_eq!(
        session_response["command"],
        json!("atelier.facial.review.session.create")
    );
    assert_artifact_ref(
        session_response["result_artifact"]["artifact_ref"]
            .as_str()
            .expect("session result artifact ref"),
    );
    let session_receipt_ref = session_response["receipt_ref"]
        .as_str()
        .expect("session receipt ref");
    let session_receipt = read_facial_receipt_payload(&client, &base, session_receipt_ref).await;
    assert_eq!(session_receipt["status"], json!("succeeded"));
    assert_eq!(
        session_receipt["command"],
        json!("atelier.facial.review.session.create")
    );
    let session_artifact_ref = session_response["result_artifact"]["artifact_ref"]
        .as_str()
        .expect("session artifact ref")
        .to_owned();

    let claim_response: Value =
        operator_actor(client.post(format!("{base}/atelier/facial/review/claims")))
            .json(&json!({
                "session_artifact_ref": session_artifact_ref,
                "existing_claim_artifact_refs": [],
                "decision_artifact_refs": [],
                "shard": 0
            }))
            .send()
            .await
            .expect("send MT-055 claim request")
            .json()
            .await
            .expect("parse MT-055 claim response");
    assert_eq!(claim_response["status"], json!("succeeded"));
    assert_eq!(
        claim_response["command"],
        json!("atelier.facial.review.claim")
    );
    let claim_artifact_ref = claim_response["result_artifact"]["artifact_ref"]
        .as_str()
        .expect("claim artifact ref")
        .to_owned();

    let duplicate_claim_http =
        operator_actor(client.post(format!("{base}/atelier/facial/review/claims")))
            .json(&json!({
                "session_artifact_ref": session_artifact_ref,
                "existing_claim_artifact_refs": [],
                "decision_artifact_refs": [],
                "shard": 0
            }))
            .send()
            .await
            .expect("send duplicate MT-055 claim request");
    assert_eq!(
        duplicate_claim_http.status().as_u16(),
        200,
        "duplicate claim must be a durable blocked envelope, not a transport error"
    );
    let duplicate_claim: Value = duplicate_claim_http
        .json()
        .await
        .expect("parse duplicate claim envelope");
    assert_non_success_outcome(
        &client,
        &base,
        duplicate_claim,
        "atelier.facial.review.claim",
        "blocked",
        "claim_shard_already_active",
        "operator",
    )
    .await;

    let decision_error_http =
        operator_actor(client.post(format!("{base}/atelier/facial/review/decisions")))
            .json(&json!({
                "session_artifact_ref": session_artifact_ref,
                "claim_artifact_ref": claim_artifact_ref,
                "item_id": "not-a-session-item",
                "decision": "pass",
                "reason": "intentional MT-055 invalid item",
                "tags": ["mt-055"]
            }))
            .send()
            .await
            .expect("send MT-055 decision error request");
    assert_eq!(
        decision_error_http.status().as_u16(),
        200,
        "post-context decision command errors must be durable HTTP-200 envelopes"
    );
    let decision_error: Value = decision_error_http
        .json()
        .await
        .expect("parse decision error envelope");
    assert_non_success_outcome(
        &client,
        &base,
        decision_error,
        "atelier.facial.review.decision",
        "error",
        "decision_unknown_item",
        "operator",
    )
    .await;

    let status_error_http =
        operator_actor(client.post(format!("{base}/atelier/facial/review/status")))
            .json(&json!({
                "session_artifact_ref": session_artifact_ref,
                "claim_artifact_refs": [claim_artifact_ref],
                "decision_artifact_refs": [],
                "now_utc": "not-a-date"
            }))
            .send()
            .await
            .expect("send MT-055 status error request");
    assert_eq!(
        status_error_http.status().as_u16(),
        200,
        "post-context status command errors must be durable HTTP-200 envelopes"
    );
    let status_error: Value = status_error_http
        .json()
        .await
        .expect("parse status error envelope");
    assert_non_success_outcome(
        &client,
        &base,
        status_error,
        "atelier.facial.review.status",
        "error",
        "status_command_failed",
        "operator",
    )
    .await;

    let montage_error_http =
        operator_actor(client.post(format!("{base}/atelier/facial/review/montage")))
            .json(&json!({
                "session_artifact_ref": session_artifact_ref,
                "decision_artifact_refs": [],
                "page": 0,
                "columns": 0,
                "rows": 4
            }))
            .send()
            .await
            .expect("send MT-055 montage error request");
    assert_eq!(
        montage_error_http.status().as_u16(),
        200,
        "post-context montage command errors must be durable HTTP-200 envelopes"
    );
    let montage_error: Value = montage_error_http
        .json()
        .await
        .expect("parse montage error envelope");
    assert_non_success_outcome(
        &client,
        &base,
        montage_error,
        "atelier.facial.review.montage",
        "error",
        "montage_invalid_grid",
        "operator",
    )
    .await;

    let no_actor = client
        .post(format!("{base}/atelier/facial/review/montage"))
        .json(&json!({
            "session_artifact_ref": session_artifact_ref,
            "decision_artifact_refs": [],
            "page": 0,
            "columns": 1,
            "rows": 1
        }))
        .send()
        .await
        .expect("send MT-055 montage without actor");
    assert!(
        no_actor.status().is_client_error(),
        "missing actor remains a pre-context transport 4xx, got {}",
        no_actor.status()
    );

    server.abort();
    harness.shutdown().await;
}

/// MT-019: the native Facial analysis route writes the analysis JSON and its receipt into the
/// ArtifactStore and echoes the artifact refs on the native_run summary.
#[tokio::test]
async fn facial_analyze_route_writes_analysis_and_receipt_artifacts() {
    let _workspace_root = shared_workspace_root();
    let harness = AtelierSurrealHarness::create().await;
    let batch_id = seed_intake_batch(&harness.atelier).await;
    let (base, client, server) = serve(app_state(&harness)).await;

    let response = operator_actor(client.post(format!(
        "{base}/atelier/intake/batches/{batch_id}/facial/analyze"
    )))
    .json(&json!({ "profile": "quality+dedupe+identity" }))
    .send()
    .await
    .expect("send Facial analyze request");
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .expect("parse Facial analyze response");
    assert!(
        status.is_success(),
        "analyze must succeed, got {status}: {body}"
    );
    assert_eq!(
        body["schema_id"],
        json!("hsk.atelier.facial_ingest_analysis@1")
    );
    assert_eq!(body["batch_id"], json!(batch_id.to_string()));
    assert_eq!(body["item_count"], json!(3));
    assert_eq!(body["profile"], json!("quality+dedupe+identity"));
    let analysis_ref = body["analysis_artifact"]["artifact_ref"]
        .as_str()
        .expect("analysis artifact ref");
    let receipt_ref = body["receipt_ref"].as_str().expect("receipt ref");
    assert_artifact_ref(analysis_ref);
    assert_artifact_ref(receipt_ref);
    assert_eq!(
        body["summary"]["native_run"]["artifact_refs"],
        json!([analysis_ref, receipt_ref])
    );

    let analysis_read: Value = client
        .get(format!("{base}/atelier/facial/artifacts/read"))
        .query(&[("artifact_ref", analysis_ref)])
        .send()
        .await
        .expect("send analysis read")
        .json()
        .await
        .expect("parse analysis read");
    assert_eq!(
        analysis_read["payload_schema_id"],
        json!("hsk.atelier.facial_ingest_analysis@1")
    );
    assert_eq!(analysis_read["content_hash"], body["analysis_sha256"]);
    let receipt_read: Value = client
        .get(format!("{base}/atelier/facial/artifacts/read"))
        .query(&[("artifact_ref", receipt_ref)])
        .send()
        .await
        .expect("send receipt read")
        .json()
        .await
        .expect("parse receipt read");
    assert_eq!(
        receipt_read["payload"]["analysis_artifact_ref"],
        json!(analysis_ref)
    );
    assert_eq!(receipt_read["content_hash"], body["receipt_sha256"]);

    server.abort();
    harness.shutdown().await;
}

/// MT-017 / MT-031 over HTTP: the batch classification route applies the plan to the canonical
/// item set (default lane + per-item override), persists the dataset-mining metadata row for every
/// item with the canonical `loaded_item_count`, and the single-item route is idempotent on
/// `metadata.request_id` while rejecting a conflicting replay.
#[tokio::test]
async fn intake_classification_routes_persist_metadata_and_lane_decisions() {
    let _workspace_root = shared_workspace_root();
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let batch_id = seed_intake_batch(store).await;
    let items = store
        .list_intake_items(batch_id, None)
        .await
        .expect("list seeded items");
    assert_eq!(items.len(), 3);
    let overridden = items[2].item_id;
    let (base, client, server) = serve(app_state(&harness)).await;

    let no_actor = client
        .post(format!(
            "{base}/atelier/intake/batches/{batch_id}/classifications"
        ))
        .json(&json!({ "default_lane": "deferred", "default_reason": "no actor" }))
        .send()
        .await
        .expect("send classification without actor");
    assert_eq!(
        no_actor.status().as_u16(),
        400,
        "missing actor header is a transport 400"
    );

    let missing_metadata = operator_actor(client.post(format!(
        "{base}/atelier/intake/batches/{batch_id}/classifications"
    )))
    .json(&json!({ "default_lane": "deferred", "default_reason": "no metadata" }))
    .send()
    .await
    .expect("send classification without metadata");
    assert_eq!(
        missing_metadata.status().as_u16(),
        400,
        "batch apply requires metadata.request_id before any write"
    );
    for item in &items {
        assert!(
            store
                .get_intake_item_metadata(item.item_id)
                .await
                .expect("read metadata")
                .is_none(),
            "a rejected batch apply must not persist metadata"
        );
    }

    let request_id = format!("mt-061-http-batch-{}", Uuid::new_v4());
    let response = operator_actor(client.post(format!(
        "{base}/atelier/intake/batches/{batch_id}/classifications"
    )))
    .json(&json!({
        "default_lane": "deferred",
        "default_reason": "await operator review",
        "metadata": {
            "request_id": request_id,
            "batch_id": batch_id.to_string(),
            "dataset_ref": "dataset://mt-061/http-batch",
            "link_passed": true,
            "tags": ["event", "location"],
            "note": "batch apply over HTTP",
            "event": "mt-061",
            "date": "2026-09-05",
            "location": "test studio",
            "facial_profile": "quality+dedupe+identity",
            "loaded_item_count": 1,
            "contact_sheet": { "rows": 3, "columns": 4, "dpi": 300, "cells": 12 }
        },
        "overrides": [
            { "item_id": overridden, "lane": "rejected", "reason": "visible override reject" }
        ]
    }))
    .send()
    .await
    .expect("send batch classification");
    let status = response.status();
    let body: Value = response.json().await.expect("parse batch classification");
    assert!(
        status.is_success(),
        "batch apply must succeed, got {status}: {body}"
    );
    assert_eq!(body["batch_id"], json!(batch_id.to_string()));
    assert_eq!(body["total_item_count"], json!(3));
    assert_eq!(body["applied_count"], json!(3));
    assert_eq!(body["requested_by"], json!("operator"));
    assert!(body["failed"].is_null());
    let applied = body["applied"].as_array().expect("applied rows");
    assert_eq!(applied.len(), 3);
    for row in applied {
        let item_id = row["item"]["item_id"].as_str().expect("applied item id");
        let expected_lane = if item_id == overridden.to_string() {
            "rejected"
        } else {
            "deferred"
        };
        assert_eq!(row["item"]["lane"], json!(expected_lane));
        assert_eq!(row["requested_by"], json!("operator"));
    }

    let persisted = store
        .list_intake_items(batch_id, None)
        .await
        .expect("reload items");
    for item in &persisted {
        let expected = if item.item_id == overridden {
            IntakeLane::Rejected
        } else {
            IntakeLane::Deferred
        };
        assert_eq!(item.lane, expected, "lane persisted for {}", item.item_id);
        let metadata = store
            .get_intake_item_metadata(item.item_id)
            .await
            .expect("read durable metadata")
            .expect("durable metadata row exists for every canonical item");
        assert_eq!(metadata.request_id, request_id);
        assert_eq!(metadata.batch_id, batch_id);
        assert_eq!(metadata.requested_by, "operator");
        assert_eq!(
            metadata.tags,
            vec!["event".to_owned(), "location".to_owned()]
        );
        assert_eq!(metadata.event_label.as_deref(), Some("mt-061"));
        assert_eq!(metadata.location.as_deref(), Some("test studio"));
        assert_eq!(
            metadata.loaded_item_count,
            Some(3),
            "metadata must record the canonical batch total, not the loaded preview count"
        );
        assert_eq!(
            metadata
                .contact_sheet
                .as_ref()
                .and_then(|sheet| sheet.cells),
            Some(12)
        );
    }

    // Single-item route: same request_id + same lane/reason is idempotent; a conflicting replay
    // is rejected before any write.
    let item_id = items[0].item_id;
    let replay = operator_actor(client.post(format!(
        "{base}/atelier/intake/items/{item_id}/classification"
    )))
    .json(&json!({
        "lane": "deferred",
        "reason": "await operator review",
        "metadata": { "request_id": request_id }
    }))
    .send()
    .await
    .expect("send idempotent replay");
    let replay_status = replay.status();
    let replay_body: Value = replay.json().await.expect("parse replay");
    assert!(
        replay_status.is_success(),
        "same request_id replay is idempotent, got {replay_status}: {replay_body}"
    );
    assert_eq!(replay_body["item"]["lane"], json!("deferred"));
    assert_eq!(replay_body["item"]["item_id"], json!(item_id.to_string()));

    let conflicting = operator_actor(client.post(format!(
        "{base}/atelier/intake/items/{item_id}/classification"
    )))
    .json(&json!({
        "lane": "skipped",
        "reason": "conflicting replay",
        "metadata": { "request_id": request_id }
    }))
    .send()
    .await
    .expect("send conflicting replay");
    assert_eq!(
        conflicting.status().as_u16(),
        400,
        "same request_id with a different lane/reason must be rejected"
    );
    let unchanged = store
        .get_intake_item_by_id(item_id)
        .await
        .expect("reload item")
        .expect("item exists");
    assert_eq!(unchanged.lane, IntakeLane::Deferred);

    let unknown_item = operator_actor(client.post(format!(
        "{base}/atelier/intake/items/{}/classification",
        Uuid::now_v7()
    )))
    .json(&json!({ "lane": "deferred", "reason": "ghost" }))
    .send()
    .await
    .expect("send classification for unknown item");
    assert_eq!(unknown_item.status().as_u16(), 404);

    server.abort();
    harness.shutdown().await;
}
