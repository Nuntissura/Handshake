//! WP-CKC MT-022 model-operation API proof (SurrealDB port, MT-062).
//!
//! Drives the real CKC ops lane Axum router over loopback with embedded-SurrealDB-backed
//! model-operation leases, action receipts, and the model-operation guarded preference
//! mutations. This is the headless parallel-agent path the native Model Ops panel exposes.
//!
//! Every test runs against its own isolated embedded store; nothing skips.

mod atelier_surreal_support;

use std::sync::Arc;

use async_trait::async_trait;
use atelier_surreal_support::AtelierSurrealHarness;
use chrono::Utc;
use handshake_core::api::atelier_ckc_ops as ops_api;
use handshake_core::atelier::action_receipt::{ActionReceiptStatus, NewActionReceipt};
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use serde_json::{json, Value};
use uuid::Uuid;

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
            profile: ModelProfile::new("mt062-model-ops-test".to_string(), 4096),
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
        axum::serve(listener, ops_api::routes(state))
            .await
            .expect("CKC ops lane API server");
    });
    (format!("http://{addr}"), reqwest::Client::new(), server)
}

fn actor(request: reqwest::RequestBuilder, id: &str) -> reqwest::RequestBuilder {
    request.header("x-hsk-actor-id", id)
}

fn actor_kind(request: reqwest::RequestBuilder, kind: &str) -> reqwest::RequestBuilder {
    request.header("x-hsk-actor-kind", kind)
}

fn model_lease(
    request: reqwest::RequestBuilder,
    claim_id: &str,
    session_id: &str,
) -> reqwest::RequestBuilder {
    request
        .header("x-hsk-model-lease-id", claim_id)
        .header("x-hsk-session-id", session_id)
}

async fn claim_test_lease(
    client: &reqwest::Client,
    base: &str,
    thread_id: &str,
    actor_id: &str,
    session_id: &str,
) -> String {
    let lease: Value = actor(
        client.post(format!("{base}/atelier/model-ops/leases")),
        actor_id,
    )
    .json(&json!({
        "thread_id": thread_id,
        "executor_kind": "local_large_model",
        "session_id": session_id,
        "claim_mode": "exclusive_lease",
        "ttl_seconds": 900,
        "linked_work_packet_id": "WP-CKC-posekit-overhaul",
        "linked_micro_task_id": "MT-022"
    }))
    .send()
    .await
    .expect("claim model-operation test lease")
    .error_for_status()
    .expect("test lease status")
    .json()
    .await
    .expect("test lease json");
    lease["claim_id"].as_str().expect("claim_id").to_owned()
}

fn assert_model_ops_recovery_metadata(body: &Value, label: &str) {
    assert!(
        body["detail"]
            .as_str()
            .unwrap_or("")
            .contains("model-operation"),
        "{label} should include model-operation detail: {body}"
    );
    assert!(
        body["recovery_hint"]
            .as_str()
            .unwrap_or("")
            .contains("/atelier/model-ops/state"),
        "{label} should include Model Ops recovery hint: {body}"
    );
    assert_eq!(
        body["required_headers"],
        json!(["x-hsk-actor-id", "x-hsk-session-id", "x-hsk-model-lease-id"]),
        "{label} should list required mutation headers"
    );
    assert_eq!(
        body["state_route"],
        json!("/atelier/model-ops/state?thread_id={thread_id}"),
        "{label} should expose the state recovery route"
    );
}

#[tokio::test]
async fn model_tool_mutations_reject_actor_only_and_accept_explicit_paths() {
    let harness = AtelierSurrealHarness::create().await;
    let (base, client, server) = serve(app_state(&harness)).await;

    let actor_only = actor(
        client.put(format!("{base}/atelier/preferences")),
        "settings-agent-actor-only",
    )
    .json(&json!({
        "key": "ckc.book-mode",
        "value": "story",
        "value_type": "string"
    }))
    .send()
    .await
    .expect("send actor-only preference mutation");
    assert_eq!(
        actor_only.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "model-operation guarded mutations must not accept bare actor-only API calls"
    );
    let actor_only_body: Value = actor_only.json().await.expect("actor-only error json");
    assert_eq!(actor_only_body["error"], json!("bad_request"));
    assert_model_ops_recovery_metadata(&actor_only_body, "actor-only preference mutation");

    let missing_actor = client
        .put(format!("{base}/atelier/preferences"))
        .json(&json!({
            "key": "ckc.book-mode",
            "value": "story",
            "value_type": "string"
        }))
        .send()
        .await
        .expect("send actorless preference mutation");
    assert_eq!(missing_actor.status(), reqwest::StatusCode::BAD_REQUEST);
    let missing_actor_body: Value = missing_actor.json().await.expect("missing actor json");
    assert_eq!(missing_actor_body["error"], json!("missing_actor"));

    let spoofed_operator_path = actor_kind(
        actor(
            client.put(format!("{base}/atelier/preferences")),
            "settings-agent-operator",
        ),
        "operator",
    )
    .json(&json!({
        "key": "ckc.book-mode",
        "value": "notes",
        "value_type": "string"
    }))
    .send()
    .await
    .expect("send spoofed operator preference mutation");
    assert_eq!(
        spoofed_operator_path.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "model actors must not bypass leases by self-labeling as operator"
    );

    let operator_path = actor_kind(
        actor(
            client.put(format!("{base}/atelier/preferences")),
            "operator",
        ),
        "operator",
    )
    .json(&json!({
        "key": "ckc.book-mode",
        "value": "notes",
        "value_type": "string"
    }))
    .send()
    .await
    .expect("send canonical operator preference mutation");
    assert_eq!(operator_path.status(), reqwest::StatusCode::OK);
    let operator_path: Value = operator_path.json().await.expect("operator response json");
    assert_eq!(operator_path["key"], json!("ckc.book-mode"));
    assert_eq!(operator_path["value"], json!("notes"));
    assert_eq!(operator_path["source"], json!("operator"));

    // The list projection reflects the operator write.
    let listed: Vec<Value> = client
        .get(format!("{base}/atelier/preferences"))
        .send()
        .await
        .expect("list preferences")
        .error_for_status()
        .expect("list preferences status")
        .json()
        .await
        .expect("list preferences json");
    let listed_book_mode = listed
        .iter()
        .find(|row| row["key"] == json!("ckc.book-mode"))
        .expect("projection lists ckc.book-mode");
    assert_eq!(listed_book_mode["value"], json!("notes"));

    // Out-of-vocabulary enumerated value through the operator path is a typed 400.
    let bad_value = actor_kind(
        actor(
            client.put(format!("{base}/atelier/preferences")),
            "operator",
        ),
        "operator",
    )
    .json(&json!({
        "key": "ckc.book-mode",
        "value": "timeline",
        "value_type": "string"
    }))
    .send()
    .await
    .expect("send out-of-vocabulary preference mutation");
    assert_eq!(bad_value.status(), reqwest::StatusCode::BAD_REQUEST);

    let wrong_preference_thread = format!("atelier.preferences.unrelated.{}", Uuid::new_v4());
    let wrong_claim_id = claim_test_lease(
        &client,
        &base,
        &wrong_preference_thread,
        "settings-agent-model",
        "settings-agent-model-session",
    )
    .await;
    let wrong_model_path = model_lease(
        actor(
            client.put(format!("{base}/atelier/preferences")),
            "settings-agent-model",
        ),
        &wrong_claim_id,
        "settings-agent-model-session",
    )
    .json(&json!({
        "key": "ckc.book-mode",
        "value": "story",
        "value_type": "string"
    }))
    .send()
    .await
    .expect("send cross-thread model-lease preference mutation");
    assert_eq!(
        wrong_model_path.status(),
        reqwest::StatusCode::CONFLICT,
        "model-operation leases must be bound to the mutated preference key"
    );
    let wrong_model_body: Value = wrong_model_path.json().await.expect("wrong thread json");
    assert_model_ops_recovery_metadata(&wrong_model_body, "cross-thread preference mutation");

    let preference_thread = "atelier.preferences.ckc.book-mode";
    let claim_id = claim_test_lease(
        &client,
        &base,
        preference_thread,
        "settings-agent-model",
        "settings-agent-model-session",
    )
    .await;
    let model_path = model_lease(
        actor(
            client.put(format!("{base}/atelier/preferences")),
            "settings-agent-model",
        ),
        &claim_id,
        "settings-agent-model-session",
    )
    .json(&json!({
        "key": "ckc.book-mode",
        "value": "story",
        "value_type": "string"
    }))
    .send()
    .await
    .expect("send model-lease preference mutation");
    let model_path_status = model_path.status();
    let model_path_text = model_path.text().await.expect("model response text");
    assert_eq!(
        model_path_status,
        reqwest::StatusCode::OK,
        "model-lease preference mutation should succeed, got {model_path_status}: {model_path_text}"
    );
    let model_path: Value = serde_json::from_str(&model_path_text).expect("model response json");
    assert_eq!(model_path["key"], json!("ckc.book-mode"));
    assert_eq!(model_path["value"], json!("story"));

    // Reset under the same lease returns the registry default.
    let reset: Value = model_lease(
        actor(
            client.post(format!("{base}/atelier/preferences/reset")),
            "settings-agent-model",
        ),
        &claim_id,
        "settings-agent-model-session",
    )
    .json(&json!({ "key": "ckc.book-mode" }))
    .send()
    .await
    .expect("send model-lease preference reset")
    .error_for_status()
    .expect("preference reset status")
    .json()
    .await
    .expect("preference reset json");
    assert_eq!(reset["value"], json!("sheet"));
    assert_eq!(reset["source"], json!("default"));

    let release_model_path = actor(
        client.post(format!(
            "{base}/atelier/model-ops/leases/{claim_id}/release"
        )),
        "settings-agent-model",
    )
    .header("x-hsk-session-id", "settings-agent-model-session")
    .json(&json!({ "session_id": "settings-agent-model-session" }))
    .send()
    .await
    .expect("release preference test lease");
    assert_eq!(release_model_path.status(), reqwest::StatusCode::OK);

    // A released lease no longer authorises mutation.
    let released_lease_path = model_lease(
        actor(
            client.put(format!("{base}/atelier/preferences")),
            "settings-agent-model",
        ),
        &claim_id,
        "settings-agent-model-session",
    )
    .json(&json!({
        "key": "ckc.book-mode",
        "value": "moodboard",
        "value_type": "string"
    }))
    .send()
    .await
    .expect("send released-lease preference mutation");
    assert_eq!(
        released_lease_path.status(),
        reqwest::StatusCode::CONFLICT,
        "a released lease must not authorise further mutations"
    );

    server.abort();
    harness.shutdown().await;
}

#[tokio::test]
async fn model_ops_api_enforces_parallel_lease_conflict_and_receipt_readback() {
    let harness = AtelierSurrealHarness::create().await;
    let (base, client, server) = serve(app_state(&harness)).await;
    let thread_id = format!("atelier.posekit.mt022.{}", Uuid::new_v4());
    let store = harness.atelier.clone();

    let lease: Value = actor(
        client.post(format!("{base}/atelier/model-ops/leases")),
        "agent-a",
    )
    .json(&json!({
        "thread_id": thread_id,
        "executor_kind": "local_large_model",
        "session_id": "session-a",
        "claim_mode": "exclusive_lease",
        "ttl_seconds": 900,
        "linked_work_packet_id": "WP-CKC-posekit-overhaul",
        "linked_micro_task_id": "MT-022"
    }))
    .send()
    .await
    .expect("claim lease")
    .error_for_status()
    .expect("lease claim status")
    .json()
    .await
    .expect("lease json");
    let claim_id = lease["claim_id"].as_str().expect("claim_id");
    assert_eq!(
        lease["schema_id"],
        json!("hsk.atelier.model_operation_lease@1")
    );
    assert_eq!(lease["actor_id"], json!("agent-a"));
    assert_eq!(lease["session_id"], json!("session-a"));
    assert_eq!(lease["executor_kind"], json!("local_large_model"));
    assert_eq!(lease["claim_mode"], json!("exclusive_lease"));
    assert_eq!(lease["effective_state"], json!("active"));
    assert_eq!(lease["lease_expired"], json!(false));

    let fetched: Value = client
        .get(format!("{base}/atelier/model-ops/leases/{claim_id}"))
        .send()
        .await
        .expect("get lease")
        .error_for_status()
        .expect("get lease status")
        .json()
        .await
        .expect("get lease json");
    assert_eq!(fetched["claim_id"], json!(claim_id));

    let unknown_lease = client
        .get(format!(
            "{base}/atelier/model-ops/leases/{}",
            Uuid::now_v7()
        ))
        .send()
        .await
        .expect("get unknown lease");
    assert_eq!(unknown_lease.status(), reqwest::StatusCode::NOT_FOUND);

    let missing_actor_claim = client
        .post(format!("{base}/atelier/model-ops/leases"))
        .json(&json!({
            "thread_id": thread_id,
            "executor_kind": "local_large_model",
            "session_id": "session-x",
            "claim_mode": "exclusive_lease",
            "ttl_seconds": 900,
            "linked_work_packet_id": "WP-CKC-posekit-overhaul",
            "linked_micro_task_id": "MT-022"
        }))
        .send()
        .await
        .expect("claim without actor");
    assert_eq!(
        missing_actor_claim.status(),
        reqwest::StatusCode::BAD_REQUEST
    );
    let missing_actor_body: Value = missing_actor_claim
        .json()
        .await
        .expect("missing actor json");
    assert_eq!(missing_actor_body["error"], json!("missing_actor"));
    assert_model_ops_recovery_metadata(&missing_actor_body, "actorless lease claim");

    let bad_mode = actor(
        client.post(format!("{base}/atelier/model-ops/leases")),
        "agent-x",
    )
    .json(&json!({
        "thread_id": format!("atelier.other.{}", Uuid::new_v4()),
        "executor_kind": "local_large_model",
        "session_id": "session-x",
        "claim_mode": "not_a_mode",
        "ttl_seconds": 900,
        "linked_work_packet_id": "WP-CKC-posekit-overhaul",
        "linked_micro_task_id": "MT-022"
    }))
    .send()
    .await
    .expect("claim with bad mode");
    assert_eq!(bad_mode.status(), reqwest::StatusCode::BAD_REQUEST);
    let bad_mode_body: Value = bad_mode.json().await.expect("bad mode json");
    assert_model_ops_recovery_metadata(&bad_mode_body, "unknown claim_mode");

    let conflict = actor(
        client.post(format!("{base}/atelier/model-ops/leases")),
        "agent-b",
    )
    .json(&json!({
        "thread_id": thread_id,
        "executor_kind": "local_large_model",
        "session_id": "session-b",
        "claim_mode": "exclusive_lease",
        "ttl_seconds": 900,
        "linked_work_packet_id": "WP-CKC-posekit-overhaul",
        "linked_micro_task_id": "MT-022"
    }))
    .send()
    .await
    .expect("claim conflicting lease");
    assert_eq!(conflict.status(), reqwest::StatusCode::CONFLICT);
    let conflict_body: Value = conflict.json().await.expect("conflict json");
    assert_eq!(conflict_body["error"], json!("conflict"));

    // A shared observer on the same thread is allowed alongside the exclusive holder, but it
    // cannot be used to mutate.
    let observer: Value = actor(
        client.post(format!("{base}/atelier/model-ops/leases")),
        "agent-observer",
    )
    .json(&json!({
        "thread_id": thread_id,
        "executor_kind": "reviewer",
        "session_id": "session-observer",
        "claim_mode": "shared_observer",
        "ttl_seconds": 900,
        "linked_work_packet_id": "WP-CKC-posekit-overhaul",
        "linked_micro_task_id": "MT-022"
    }))
    .send()
    .await
    .expect("claim observer lease")
    .error_for_status()
    .expect("observer claim status")
    .json()
    .await
    .expect("observer lease json");
    let observer_claim_id = observer["claim_id"].as_str().expect("observer claim_id");

    let receipt_body = json!({
        "action_id": "kernel.action_catalog.view",
        "actor_kind": "agent",
        "session_id": "session-a",
        "params": {
            "thread_id": thread_id,
            "raw_secret": "do-not-persist-raw"
        },
        "started_at_utc": "2026-07-03T00:00:00Z",
        "completed_at_utc": "2026-07-03T00:00:01Z",
        "status": "succeeded",
        "target_refs": ["kernel://action-catalog/kernel002-action-catalog-v1"],
        "evidence_refs": ["src/backend/handshake_core/src/kernel/action_catalog.rs"],
        "result_refs": ["kernel://action-catalog/view-result"],
        "error_class": null,
        "recovery_hint": null
    });
    let missing_lease = actor(
        client.post(format!("{base}/atelier/model-ops/action-receipts")),
        "agent-a",
    )
    .json(&receipt_body)
    .send()
    .await
    .expect("record missing-lease receipt");
    assert_eq!(missing_lease.status(), reqwest::StatusCode::BAD_REQUEST);
    let missing_lease_body: Value = missing_lease.json().await.expect("missing lease json");
    assert_model_ops_recovery_metadata(&missing_lease_body, "missing-lease receipt");

    let observer_receipt = actor(
        model_lease(
            client.post(format!("{base}/atelier/model-ops/action-receipts")),
            observer_claim_id,
            "session-observer",
        ),
        "agent-observer",
    )
    .json(&json!({
        "action_id": "kernel.action_catalog.view",
        "session_id": "session-observer",
        "params": { "thread_id": thread_id },
        "started_at_utc": "2026-07-03T00:00:00Z",
        "completed_at_utc": "2026-07-03T00:00:01Z",
        "status": "succeeded",
        "target_refs": ["kernel://action-catalog/kernel002-action-catalog-v1"],
        "evidence_refs": ["src/backend/handshake_core/src/kernel/action_catalog.rs"],
        "result_refs": ["kernel://action-catalog/view-result"],
        "error_class": null,
        "recovery_hint": null
    }))
    .send()
    .await
    .expect("record receipt under observer lease");
    assert_eq!(
        observer_receipt.status(),
        reqwest::StatusCode::CONFLICT,
        "a shared_observer lease has no mutation authority"
    );

    let spoofed_body_session = actor(
        model_lease(
            client.post(format!("{base}/atelier/model-ops/action-receipts")),
            claim_id,
            "session-a",
        ),
        "agent-a",
    )
    .json(&json!({
        "action_id": "kernel.action_catalog.view",
        "actor_kind": "agent",
        "session_id": "session-b",
        "params": { "thread_id": thread_id },
        "started_at_utc": "2026-07-03T00:00:00Z",
        "completed_at_utc": "2026-07-03T00:00:01Z",
        "status": "succeeded",
        "target_refs": ["kernel://action-catalog/kernel002-action-catalog-v1"],
        "evidence_refs": ["src/backend/handshake_core/src/kernel/action_catalog.rs"],
        "result_refs": ["kernel://action-catalog/view-result"],
        "error_class": null,
        "recovery_hint": null
    }))
    .send()
    .await
    .expect("record spoofed-body-session receipt");
    assert_eq!(
        spoofed_body_session.status(),
        reqwest::StatusCode::BAD_REQUEST
    );

    let mut no_actor_kind_body = receipt_body.clone();
    no_actor_kind_body
        .as_object_mut()
        .expect("receipt body object")
        .remove("actor_kind");
    let no_actor_kind_receipt: Value = actor(
        model_lease(
            client.post(format!("{base}/atelier/model-ops/action-receipts")),
            claim_id,
            "session-a",
        ),
        "agent-a",
    )
    .json(&no_actor_kind_body)
    .send()
    .await
    .expect("record no-actor-kind receipt")
    .error_for_status()
    .expect("no-actor-kind receipt status")
    .json()
    .await
    .expect("no-actor-kind receipt json");
    assert_eq!(
        no_actor_kind_receipt["actor_kind"],
        json!("local_large_model"),
        "model-operation receipt actor_kind is derived from the validated lease executor"
    );
    assert_eq!(no_actor_kind_receipt["thread_id"], json!(thread_id));
    assert_eq!(no_actor_kind_receipt["lease_claim_id"], json!(claim_id));

    let receipt_response = actor(
        model_lease(
            client.post(format!("{base}/atelier/model-ops/action-receipts")),
            claim_id,
            "session-a",
        ),
        "agent-a",
    )
    .json(&receipt_body)
    .send()
    .await
    .expect("record receipt");
    assert_eq!(receipt_response.status(), reqwest::StatusCode::CREATED);
    let receipt: Value = receipt_response.json().await.expect("receipt json");
    assert_eq!(
        receipt["schema_id"],
        json!("hsk.atelier.model_operation_action_receipt@1")
    );
    assert_eq!(receipt["actor_id"], json!("agent-a"));
    assert_eq!(receipt["actor_kind"], json!("local_large_model"));
    assert_eq!(receipt["thread_id"], json!(thread_id));
    assert_eq!(receipt["lease_claim_id"], json!(claim_id));
    assert_eq!(receipt["status"], json!("succeeded"));
    assert!(receipt["params_sha256"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert!(!receipt.to_string().contains("do-not-persist-raw"));

    let receipt_id = receipt["receipt_id"].as_str().expect("receipt_id");
    let readback: Value = client
        .get(format!(
            "{base}/atelier/model-ops/action-receipts/{receipt_id}"
        ))
        .send()
        .await
        .expect("read receipt back")
        .error_for_status()
        .expect("receipt readback status")
        .json()
        .await
        .expect("receipt readback json");
    assert_eq!(
        readback, receipt,
        "receipt readback must equal the recorded receipt"
    );

    let wrong_thread_receipt = actor(
        model_lease(
            client.post(format!("{base}/atelier/model-ops/action-receipts")),
            claim_id,
            "session-a",
        ),
        "agent-a",
    )
    .json(&json!({
        "action_id": "kernel.action_catalog.view",
        "actor_kind": "agent",
        "session_id": "session-a",
        "params": { "thread_id": format!("atelier.action.wrong.{}", Uuid::new_v4()) },
        "started_at_utc": "2026-07-03T00:00:00Z",
        "completed_at_utc": "2026-07-03T00:00:01Z",
        "status": "succeeded",
        "target_refs": ["kernel://action-catalog/kernel002-action-catalog-v1"],
        "evidence_refs": ["src/backend/handshake_core/src/kernel/action_catalog.rs"],
        "result_refs": ["kernel://action-catalog/view-result"],
        "error_class": null,
        "recovery_hint": null
    }))
    .send()
    .await
    .expect("record wrong-thread receipt");
    assert_eq!(wrong_thread_receipt.status(), reqwest::StatusCode::CONFLICT);

    let legacy_receipt = store
        .record_action_receipt(&NewActionReceipt {
            action_id: "kernel.action_catalog.view".to_owned(),
            actor_kind: "operator".to_owned(),
            actor_id: "legacy-operator".to_owned(),
            session_id: "legacy-session".to_owned(),
            thread_id: String::new(),
            lease_claim_id: None,
            params: json!({ "legacy": true }),
            started_at_utc: Utc::now(),
            completed_at_utc: Utc::now(),
            status: ActionReceiptStatus::Succeeded,
            target_refs: vec!["kernel://action-catalog/kernel002-action-catalog-v1".to_owned()],
            evidence_refs: vec![
                "src/backend/handshake_core/src/kernel/action_catalog.rs".to_owned()
            ],
            result_refs: vec!["kernel://action-catalog/view-result".to_owned()],
            error_class: None,
            recovery_hint: None,
        })
        .await
        .expect("seed legacy non-model-operation receipt");
    let legacy_read = client
        .get(format!(
            "{base}/atelier/model-ops/action-receipts/{}",
            legacy_receipt.receipt_id
        ))
        .send()
        .await
        .expect("read legacy receipt through model-ops route");
    let legacy_status = legacy_read.status();
    let legacy_body: Value = legacy_read.json().await.expect("legacy receipt error json");
    assert_eq!(legacy_status, reqwest::StatusCode::BAD_REQUEST);
    assert_model_ops_recovery_metadata(&legacy_body, "legacy receipt readback");

    let unknown_receipt = client
        .get(format!(
            "{base}/atelier/model-ops/action-receipts/{}",
            Uuid::now_v7()
        ))
        .send()
        .await
        .expect("read unknown receipt");
    assert_eq!(unknown_receipt.status(), reqwest::StatusCode::NOT_FOUND);

    let wrong_session = actor(
        model_lease(
            client.post(format!("{base}/atelier/model-ops/action-receipts")),
            claim_id,
            "session-b",
        ),
        "agent-a",
    )
    .json(&receipt_body)
    .send()
    .await
    .expect("record wrong-session receipt");
    assert_eq!(wrong_session.status(), reqwest::StatusCode::CONFLICT);

    let state: Value = client
        .get(format!("{base}/atelier/model-ops/state"))
        .query(&[("thread_id", thread_id.as_str())])
        .send()
        .await
        .expect("state request")
        .error_for_status()
        .expect("state status")
        .json()
        .await
        .expect("state json");
    assert_eq!(
        state["schema_id"],
        json!("hsk.atelier.model_operation_state@1")
    );
    assert_eq!(state["thread_id"], json!(thread_id));
    let state_leases = state["leases"].as_array().expect("state leases");
    assert_eq!(state_leases.len(), 2, "exclusive holder + shared observer");
    assert!(state_leases
        .iter()
        .any(|lease| lease["claim_id"] == json!(claim_id)));
    assert!(state["required_headers_for_mutation"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "x-hsk-model-lease-id"));

    let listed: Vec<Value> = client
        .get(format!("{base}/atelier/model-ops/leases"))
        .query(&[("thread_id", thread_id.as_str())])
        .send()
        .await
        .expect("list leases")
        .error_for_status()
        .expect("list leases status")
        .json()
        .await
        .expect("list leases json");
    assert_eq!(listed.len(), 2);

    let wrong_renew = actor(
        client.post(format!("{base}/atelier/model-ops/leases/{claim_id}/renew")),
        "agent-a",
    )
    .header("x-hsk-session-id", "session-b")
    .json(&json!({
        "session_id": "session-b",
        "extend_seconds": 60
    }))
    .send()
    .await
    .expect("wrong-session renew");
    let wrong_renew_status = wrong_renew.status();
    let wrong_renew_body: Value = wrong_renew.json().await.expect("wrong renew json");
    assert_eq!(wrong_renew_status, reqwest::StatusCode::CONFLICT);
    assert_model_ops_recovery_metadata(&wrong_renew_body, "wrong-session renew");

    let no_session_renew = actor(
        client.post(format!("{base}/atelier/model-ops/leases/{claim_id}/renew")),
        "agent-a",
    )
    .json(&json!({
        "session_id": "session-a",
        "extend_seconds": 60
    }))
    .send()
    .await
    .expect("renew without session header");
    assert_eq!(no_session_renew.status(), reqwest::StatusCode::BAD_REQUEST);
    let no_session_body: Value = no_session_renew.json().await.expect("no session json");
    assert_eq!(no_session_body["error"], json!("missing_session"));

    let renewed: Value = actor(
        client.post(format!("{base}/atelier/model-ops/leases/{claim_id}/renew")),
        "agent-a",
    )
    .header("x-hsk-session-id", "session-a")
    .json(&json!({
        "session_id": "session-a",
        "extend_seconds": 60
    }))
    .send()
    .await
    .expect("renew lease")
    .error_for_status()
    .expect("renew status")
    .json()
    .await
    .expect("renew json");
    assert_eq!(renewed["effective_state"], json!("active"));
    assert!(
        renewed["lease_expires_at_utc"]
            .as_str()
            .expect("renewed expiry")
            > lease["lease_expires_at_utc"]
                .as_str()
                .expect("initial expiry"),
        "renewal must push the expiry forward"
    );

    let wrong_release = actor(
        client.post(format!(
            "{base}/atelier/model-ops/leases/{claim_id}/release"
        )),
        "agent-a",
    )
    .header("x-hsk-session-id", "session-b")
    .json(&json!({ "session_id": "session-b" }))
    .send()
    .await
    .expect("wrong-session release");
    let wrong_release_status = wrong_release.status();
    let wrong_release_body: Value = wrong_release.json().await.expect("wrong release json");
    assert_eq!(wrong_release_status, reqwest::StatusCode::CONFLICT);
    assert_model_ops_recovery_metadata(&wrong_release_body, "wrong-session release");

    let released: Value = actor(
        client.post(format!(
            "{base}/atelier/model-ops/leases/{claim_id}/release"
        )),
        "agent-a",
    )
    .header("x-hsk-session-id", "session-a")
    .json(&json!({ "session_id": "session-a" }))
    .send()
    .await
    .expect("release lease")
    .error_for_status()
    .expect("release status")
    .json()
    .await
    .expect("release json");
    assert_eq!(released["effective_state"], json!("released"));

    // Once released, the thread is claimable again by the former conflicter.
    let reclaimed = actor(
        client.post(format!("{base}/atelier/model-ops/leases")),
        "agent-b",
    )
    .json(&json!({
        "thread_id": thread_id,
        "executor_kind": "local_large_model",
        "session_id": "session-b",
        "claim_mode": "exclusive_lease",
        "ttl_seconds": 900,
        "linked_work_packet_id": "WP-CKC-posekit-overhaul",
        "linked_micro_task_id": "MT-022"
    }))
    .send()
    .await
    .expect("reclaim released thread");
    assert_eq!(reclaimed.status(), reqwest::StatusCode::CREATED);

    server.abort();
    harness.shutdown().await;
}

#[tokio::test]
async fn model_ops_api_serializes_concurrent_first_exclusive_claims() {
    let harness = AtelierSurrealHarness::create().await;
    let (base, client, server) = serve(app_state(&harness)).await;
    let thread_id = format!("atelier.posekit.mt022.race.{}", Uuid::new_v4());

    let mut tasks = Vec::new();
    for index in 0..8 {
        let client = client.clone();
        let base = base.clone();
        let thread_id = thread_id.clone();
        tasks.push(tokio::spawn(async move {
            actor(
                client.post(format!("{base}/atelier/model-ops/leases")),
                &format!("race-agent-{index}"),
            )
            .json(&json!({
                "thread_id": thread_id,
                "executor_kind": "local_large_model",
                "session_id": format!("race-session-{index}"),
                "claim_mode": "exclusive_lease",
                "ttl_seconds": 900,
                "linked_work_packet_id": "WP-CKC-posekit-overhaul",
                "linked_micro_task_id": "MT-022"
            }))
            .send()
            .await
            .expect("race claim request")
            .status()
        }));
    }

    let mut created = 0;
    let mut conflicts = 0;
    for task in tasks {
        match task.await.expect("race claim task") {
            reqwest::StatusCode::CREATED => created += 1,
            reqwest::StatusCode::CONFLICT => conflicts += 1,
            other => panic!("unexpected race-claim status {other}"),
        }
    }
    assert_eq!(created, 1, "exactly one first exclusive claim wins");
    assert_eq!(conflicts, 7, "all other concurrent claimers conflict");

    // The store agrees with the HTTP verdicts: one active exclusive holder on the thread.
    let leases: Vec<Value> = client
        .get(format!("{base}/atelier/model-ops/leases"))
        .query(&[("thread_id", thread_id.as_str())])
        .send()
        .await
        .expect("list race leases")
        .error_for_status()
        .expect("list race leases status")
        .json()
        .await
        .expect("list race leases json");
    let active = leases
        .iter()
        .filter(|lease| lease["effective_state"] == json!("active"))
        .count();
    assert_eq!(
        active, 1,
        "exactly one persisted active lease after the race"
    );
    assert_eq!(leases.len(), 1, "losing claimants must not persist rows");

    server.abort();
    harness.shutdown().await;
}
