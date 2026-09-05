//! WP-CKC-posekit-overhaul (SurrealDB port) MT-061: integration proof for the native Atelier
//! contact-sheet export store/API path (reference MT-018 remediation).
//!
//! Drives the REAL `export_contact_sheet` HTTP handler (`POST /atelier/contact-sheets/export`) over
//! the lane Axum router against an isolated embedded SurrealDB `AtelierStore`, seeding a real intake
//! batch, then reads the resulting artifacts back off the ArtifactStore to assert content-hash
//! integrity and item-ID lineage. No mock store, no canned refs. The reference ran against managed
//! PostgreSQL and skipped without `DATABASE_URL`; this port always runs.

mod atelier_surreal_support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use atelier_surreal_support::AtelierSurrealHarness;
use handshake_core::api::atelier_ckc_intake_facial as lane_api;
use handshake_core::atelier::contact_sheet::CONTACT_SHEET_EXPORT_SCHEMA_ID;
use handshake_core::atelier::intake::{
    IntakeBatchMode, IntakeProfileMode, NewIntakeBatch, NewIntakeItem,
};
use handshake_core::atelier::AtelierStore;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::artifacts::{
    artifact_root_rel, read_file_artifact, sha256_hex, validate_artifact_content_hash,
    ArtifactLayer,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use uuid::Uuid;

/// One workspace root for the whole test binary (`HANDSHAKE_WORKSPACE_ROOT` is process-global).
fn shared_workspace_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let root = tempfile::tempdir()
            .expect("create isolated contact-sheet workspace root")
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
            profile: ModelProfile::new("mt061-contact-sheet-test".to_string(), 4096),
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

/// Extract the L1 artifact UUID from a native `artifact://.../payload` handle, asserting the shape
/// the store contract guarantees.
fn artifact_id_from_payload_ref(artifact_ref: &str) -> Uuid {
    let rest = artifact_ref
        .strip_prefix("artifact://.handshake/artifacts/L1/")
        .expect("ArtifactStore payload ref must use the native L1 prefix");
    let artifact_id = rest
        .strip_suffix("/payload")
        .expect("ArtifactStore payload ref must end with /payload");
    Uuid::parse_str(artifact_id).expect("ArtifactStore payload ref carries a UUID artifact id")
}

fn payload_path(workspace_root: &Path, artifact_id: Uuid) -> PathBuf {
    workspace_root
        .join(artifact_root_rel(ArtifactLayer::L1, artifact_id))
        .join("payload")
}

/// A run-unique intake item as returned by the store (item_id is store-assigned).
struct SeededItem {
    item_id: Uuid,
    source_path: String,
    label: String,
}

/// Seed a real intake batch with `item_count` real items and return the batch id plus the
/// store-assigned items. Every item_id is minted by the store, not by the test, so downstream
/// lineage assertions are grounded in real persisted state.
async fn seed_intake_batch(store: &AtelierStore, item_count: usize) -> (Uuid, Vec<SeededItem>) {
    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: format!("mt-018-contact-sheet-{}", Uuid::new_v4()),
            source_label: "mt-018-contact-sheet".to_string(),
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
        .expect("open intake batch");

    let mut seeded = Vec::with_capacity(item_count);
    for index in 0..item_count {
        let source_path = format!(
            "source://operator-inbox/{}/frame-{index}.png",
            Uuid::new_v4()
        );
        let file_name = format!("frame-{index}.png");
        let item = store
            .add_intake_item(
                batch.batch_id,
                &NewIntakeItem {
                    source_path: source_path.clone(),
                    file_name: file_name.clone(),
                    byte_len: 2048 + index as i64,
                    content_hash: Some(format!("sha256-{}", Uuid::new_v4())),
                },
            )
            .await
            .expect("add intake item");
        // Ground the request on the STORE-RETURNED item so `source_ref` matches the canonical
        // `source_path` exactly, even if the store normalizes it.
        seeded.push(SeededItem {
            item_id: item.item_id,
            source_path: item.source_path.clone(),
            label: item.file_name.clone(),
        });
    }
    (batch.batch_id, seeded)
}

fn contact_sheet_item_json(item: &SeededItem) -> serde_json::Value {
    serde_json::json!({
        "item_id": item.item_id.to_string(),
        "label": item.label,
        "source_ref": item.source_path,
    })
}

/// Primary proof. Drives the real `export_contact_sheet` route and asserts:
///   (a) the SVG artifact is persisted as a real `artifact://` L1 handle whose stored bytes hash to
///       exactly the API-declared `svg_sha256`;
///   (b) the JSON receipt artifact is persisted and its `source_items` link back to the exact set
///       of store-canonical intake item IDs + source paths;
///   (c1) referencing a non-existent intake item is rejected (400) before any receipt is persisted;
///   (c2) a tampered persisted SVG payload is rejected by content-hash validation.
#[tokio::test]
async fn atelier_contact_sheet_export_route_persists_svg_and_lineage_receipt() {
    // Resolve (and pin) the process-local ArtifactStore workspace BEFORE the server runs so the
    // handler's `resolve_workspace_root()` and our read-back point at the same root.
    let workspace_root = shared_workspace_root().clone();
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let (batch_id, seeded) = seed_intake_batch(store, 3).await;

    // Ground the lineage in persisted state, not the request echo.
    let canonical = store
        .list_intake_items(batch_id, None)
        .await
        .expect("list seeded intake items");
    assert_eq!(
        canonical.len(),
        3,
        "seeded batch must expose exactly 3 canonical intake items"
    );
    let canonical_lineage: BTreeSet<(String, String)> = canonical
        .iter()
        .map(|item| (item.item_id.to_string(), item.source_path.clone()))
        .collect();

    let request_items: Vec<serde_json::Value> =
        seeded.iter().map(contact_sheet_item_json).collect();
    let request_body = serde_json::json!({
        "source_kind": "ingest_batch",
        "source_ref": batch_id.to_string(),
        "rows": 2,
        "columns": 2,
        "dpi": 150,
        "include_labels": true,
        "thumbnail_fit": "contain",
        "items": request_items,
    });

    let (base, client, server) = serve(app_state(&harness)).await;

    let response = client
        .post(format!("{base}/atelier/contact-sheets/export"))
        .header("x-hsk-actor-id", "operator")
        .header("x-hsk-actor-kind", "operator")
        .json(&request_body)
        .send()
        .await
        .expect("send contact sheet export request");
    let status = response.status();
    let text = response
        .text()
        .await
        .expect("read contact sheet export response");

    // (c1) A receipt cannot reference a non-existent item.
    let mut bogus_items = request_items.clone();
    bogus_items.push(serde_json::json!({
        "item_id": Uuid::new_v4().to_string(),
        "label": "ghost.png",
        "source_ref": "source://operator-inbox/ghost/ghost.png",
    }));
    let bogus_body = serde_json::json!({
        "source_kind": "ingest_batch",
        "source_ref": batch_id.to_string(),
        "rows": 2,
        "columns": 2,
        "dpi": 150,
        "include_labels": true,
        "items": bogus_items,
    });
    let bogus_response = client
        .post(format!("{base}/atelier/contact-sheets/export"))
        .header("x-hsk-actor-id", "operator")
        .header("x-hsk-actor-kind", "operator")
        .json(&bogus_body)
        .send()
        .await
        .expect("send contact sheet export request with a non-existent item");
    let bogus_status = bogus_response.status();
    let bogus_text = bogus_response
        .text()
        .await
        .expect("read bogus response body");

    // An agent caller without a model-operation lease is a pre-context transport 400.
    let leaseless_agent = client
        .post(format!("{base}/atelier/contact-sheets/export"))
        .header("x-hsk-actor-id", "mt-061-agent")
        .json(&request_body)
        .send()
        .await
        .expect("send leaseless agent contact sheet export request");
    let leaseless_status = leaseless_agent.status();

    server.abort();

    assert!(
        status.is_success(),
        "contact sheet export route must return success, got {status}: {text}"
    );
    let body: serde_json::Value = serde_json::from_str(&text).expect("export response JSON");
    assert_eq!(
        body["schema_id"],
        serde_json::json!(CONTACT_SHEET_EXPORT_SCHEMA_ID)
    );
    assert_eq!(body["source_ref"], serde_json::json!(batch_id.to_string()));
    assert_eq!(body["item_count"], serde_json::json!(3));
    assert_eq!(body["rendered_item_count"], serde_json::json!(3));

    // (a) SVG artifact: real handle + stored bytes hash to svg_sha256.
    let svg_ref = body["svg_artifact"]["artifact_ref"]
        .as_str()
        .expect("svg_artifact.artifact_ref string");
    assert!(
        svg_ref.starts_with("artifact://.handshake/artifacts/L1/"),
        "svg artifact_ref must be a native ArtifactStore payload handle: {svg_ref}"
    );
    assert!(
        !svg_ref.starts_with("preview://"),
        "svg artifact_ref must not be a preview handle: {svg_ref}"
    );
    let response_svg_sha256 = body["svg_sha256"].as_str().expect("svg_sha256 string");
    assert_eq!(
        body["svg_artifact"]["content_hash"].as_str(),
        Some(response_svg_sha256),
        "svg_artifact.content_hash must equal the response svg_sha256"
    );
    let svg_id = artifact_id_from_payload_ref(svg_ref);
    let stored_svg = read_file_artifact(&workspace_root, ArtifactLayer::L1, svg_id)
        .expect("read persisted contact sheet SVG artifact");
    assert_eq!(
        sha256_hex(&stored_svg),
        response_svg_sha256,
        "sha256 of the persisted SVG bytes must equal the API-declared svg_sha256"
    );
    validate_artifact_content_hash(&workspace_root, ArtifactLayer::L1, svg_id)
        .expect("persisted SVG artifact self-validates its content hash");
    let stored_svg_text = String::from_utf8(stored_svg.clone()).expect("persisted SVG is UTF-8");
    assert!(
        stored_svg_text.starts_with("<svg"),
        "persisted SVG payload must be a real SVG document"
    );
    for item in &seeded {
        assert!(
            stored_svg_text.contains(&format!("data-item-id=\"{}\"", item.item_id)),
            "persisted SVG must render seeded item {} as a lineage-tagged cell",
            item.item_id
        );
    }

    // (b) JSON receipt artifact: persisted + lineage back to intake IDs.
    let receipt_ref = body["receipt_ref"].as_str().expect("receipt_ref string");
    assert_eq!(
        body["receipt_artifact"]["artifact_ref"].as_str(),
        Some(receipt_ref),
        "receipt_ref must equal receipt_artifact.artifact_ref"
    );
    assert!(
        receipt_ref.starts_with("artifact://.handshake/artifacts/L1/"),
        "receipt artifact_ref must be a native ArtifactStore payload handle: {receipt_ref}"
    );
    let receipt_id = artifact_id_from_payload_ref(receipt_ref);
    let stored_receipt = read_file_artifact(&workspace_root, ArtifactLayer::L1, receipt_id)
        .expect("read persisted contact sheet receipt artifact");
    validate_artifact_content_hash(&workspace_root, ArtifactLayer::L1, receipt_id)
        .expect("persisted receipt artifact self-validates its content hash");
    assert_eq!(
        body["receipt_sha256"].as_str(),
        Some(sha256_hex(&stored_receipt).as_str()),
        "receipt_sha256 must equal the hash of the persisted receipt bytes"
    );
    let receipt: serde_json::Value =
        serde_json::from_slice(&stored_receipt).expect("persisted receipt payload is JSON");
    assert_eq!(
        receipt["schema_id"],
        serde_json::json!("hsk.atelier.contact_sheet_export_receipt@1")
    );
    assert_eq!(
        receipt["source_ref"],
        serde_json::json!(batch_id.to_string()),
        "receipt must record the ingest batch it was generated from"
    );
    assert_eq!(
        receipt["svg_artifact_ref"].as_str(),
        Some(svg_ref),
        "receipt must link back to the persisted SVG artifact it describes"
    );
    assert_eq!(
        receipt["svg_sha256"].as_str(),
        Some(response_svg_sha256),
        "receipt must carry the SVG content hash"
    );
    let source_items = receipt["source_items"]
        .as_array()
        .expect("receipt source_items array");
    let receipt_lineage: BTreeSet<(String, String)> = source_items
        .iter()
        .map(|item| {
            (
                item["item_id"]
                    .as_str()
                    .expect("receipt item_id string")
                    .to_string(),
                item["source_ref"]
                    .as_str()
                    .expect("receipt source_ref string")
                    .to_string(),
            )
        })
        .collect();
    assert_eq!(
        receipt_lineage, canonical_lineage,
        "receipt lineage must equal the store-canonical intake item IDs and source paths"
    );

    // (c1) non-existent item rejected before any persistence.
    assert_eq!(
        bogus_status.as_u16(),
        400,
        "non-existent intake item must be rejected with 400, got {bogus_status}: {bogus_text}"
    );
    assert_eq!(
        leaseless_status.as_u16(),
        400,
        "an agent without a model-operation lease must be rejected with 400"
    );

    // (c2) tampered persisted SVG payload is rejected by content-hash check.
    let svg_payload_path = payload_path(&workspace_root, svg_id);
    let mut tampered = stored_svg.clone();
    tampered.extend_from_slice(b"<!-- tamper -->");
    std::fs::write(&svg_payload_path, &tampered)
        .expect("overwrite persisted SVG payload for the tamper proof");
    assert!(
        read_file_artifact(&workspace_root, ArtifactLayer::L1, svg_id).is_err(),
        "a tampered SVG payload must be rejected by ArtifactStore content-hash validation"
    );
    assert!(
        validate_artifact_content_hash(&workspace_root, ArtifactLayer::L1, svg_id).is_err(),
        "content-hash validation must fail after the persisted SVG payload is tampered"
    );
    harness.shutdown().await;
}

/// Lineage-integrity negative: an item_id that DOES belong to the batch but is paired with a
/// forged source_ref must be rejected. This proves the route cross-checks `item.source_ref`
/// against the stored intake `source_path`, not just item_id membership.
#[tokio::test]
async fn atelier_contact_sheet_export_route_rejects_source_ref_that_does_not_match_intake_lineage()
{
    let _workspace_root = shared_workspace_root();
    let harness = AtelierSurrealHarness::create().await;
    let (batch_id, seeded) = seed_intake_batch(&harness.atelier, 1).await;

    let forged_items = serde_json::json!([{
        "item_id": seeded[0].item_id.to_string(),
        "label": "forged.png",
        "source_ref": "source://operator-inbox/forged/forged.png",
    }]);
    let forged_body = serde_json::json!({
        "source_kind": "ingest_batch",
        "source_ref": batch_id.to_string(),
        "rows": 1,
        "columns": 1,
        "dpi": 120,
        "include_labels": true,
        "items": forged_items,
    });

    let (base, client, server) = serve(app_state(&harness)).await;
    let response = client
        .post(format!("{base}/atelier/contact-sheets/export"))
        .header("x-hsk-actor-id", "operator")
        .header("x-hsk-actor-kind", "operator")
        .json(&forged_body)
        .send()
        .await
        .expect("send contact sheet export request with a forged source_ref");
    let status = response.status();
    let text = response.text().await.expect("read forged response body");

    // Unknown batch UUID and an empty batch are both 400 (validation), never a fabricated sheet.
    let unknown_batch = client
        .post(format!("{base}/atelier/contact-sheets/export"))
        .header("x-hsk-actor-id", "operator")
        .header("x-hsk-actor-kind", "operator")
        .json(&serde_json::json!({
            "source_kind": "ingest_batch",
            "source_ref": Uuid::now_v7().to_string(),
            "rows": 1,
            "columns": 1,
            "dpi": 120,
            "items": [],
        }))
        .send()
        .await
        .expect("send contact sheet export for an unknown batch");
    let unknown_status = unknown_batch.status();
    server.abort();

    assert_eq!(
        status.as_u16(),
        400,
        "forged source_ref must be rejected with 400, got {status}: {text}"
    );
    assert_eq!(
        unknown_status.as_u16(),
        400,
        "a batch with no canonical items must be rejected with 400"
    );
    harness.shutdown().await;
}
