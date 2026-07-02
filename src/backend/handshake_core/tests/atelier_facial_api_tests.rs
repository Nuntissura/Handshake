//! MT-029 Facial native command API proof.
//!
//! Drives the actual Atelier Axum router over a loopback listener and a real
//! PostgreSQL-backed intake batch. The test stays quiet: no GUI launch, no
//! foreground window, and all artifacts are written through the test workspace.

mod atelier_pg_support;
mod user_manual_support;

use chrono::Utc;
use handshake_core::api::atelier as atelier_api;
use handshake_core::atelier::intake::{
    IntakeBatchMode, IntakeProfileMode, NewIntakeBatch, NewIntakeItem,
};
use handshake_core::atelier::AtelierStore;
use handshake_core::storage::artifacts::{
    artifact_root_rel, validate_artifact_content_hash, write_file_artifact, ArtifactClassification,
    ArtifactLayer, ArtifactManifest, ArtifactPayloadKind,
};
use handshake_core::storage::EntityRef;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect atelier store to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
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

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn write_non_facial_json_artifact() -> String {
    let workspace_root = atelier_pg_support::test_artifact_workspace_root();
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
    write_file_artifact(&workspace_root, &manifest, payload)
        .expect("write non-Facial JSON artifact fixture");
    validate_artifact_content_hash(&workspace_root, ArtifactLayer::L1, artifact_id)
        .expect("validate non-Facial JSON fixture");
    format!(
        "artifact://{}/payload",
        artifact_root_rel(ArtifactLayer::L1, artifact_id)
    )
}

#[tokio::test]
async fn atelier_facial_command_routes_round_trip_artifact_backed_review_flow() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP atelier_facial_command_routes_round_trip_artifact_backed_review_flow: PostgreSQL unavailable"
        );
        return;
    };
    let _workspace_root = atelier_pg_support::test_artifact_workspace_root();
    let store = connected_store(&url).await;
    let batch_id = seed_intake_batch(&store).await;

    let state = user_manual_support::app_state_for(&url).await;
    let (base, server) = user_manual_support::start_server(atelier_api::routes(state)).await;
    let client = reqwest::Client::new();

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

    let session_response: Value = actor(
        client.post(format!(
            "{base}/atelier/intake/batches/{batch_id}/facial/review/session"
        )),
        "mt-029-agent-a",
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

    let claim_response: Value = actor(
        client.post(format!("{base}/atelier/facial/review/claims")),
        "mt-029-agent-a",
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

    let decision_response: Value = actor(
        client.post(format!("{base}/atelier/facial/review/decisions")),
        "mt-029-agent-a",
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

    let status_response: Value = actor(
        client.post(format!("{base}/atelier/facial/review/status")),
        "mt-029-agent-b",
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
    let recovered_status_response: Value = actor(
        client.post(format!("{base}/atelier/facial/review/status")),
        "mt-029-agent-b",
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

    let montage_response: Value = actor(
        client.post(format!("{base}/atelier/facial/review/montage")),
        "mt-029-agent-b",
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

    let export_response: Value = actor(
        client.post(format!("{base}/atelier/facial/review/export")),
        "mt-029-agent-c",
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
}

/// MT-031: the Facial export command must return DURABLE, model-usable command
/// *outcome* envelopes (blocked/degraded/error/succeeded) at HTTP 200 with persisted
/// receipt artifacts, while pre-context failures stay bare HTTP 4xx transport errors.
#[tokio::test]
async fn facial_command_failure_envelopes_are_durable() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP facial_command_failure_envelopes_are_durable: PostgreSQL unavailable");
        return;
    };
    let _workspace_root = atelier_pg_support::test_artifact_workspace_root();
    let store = connected_store(&url).await;
    let batch_id = seed_intake_batch(&store).await;

    let state = user_manual_support::app_state_for(&url).await;
    let (base, server) = user_manual_support::start_server(atelier_api::routes(state)).await;
    let client = reqwest::Client::new();

    // One shard so a single claim exposes all 3 seeded items (deterministic decide set).
    let session_response: Value = actor(
        client.post(format!(
            "{base}/atelier/intake/batches/{batch_id}/facial/review/session"
        )),
        "mt-031-agent",
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

    let claim_response: Value = actor(
        client.post(format!("{base}/atelier/facial/review/claims")),
        "mt-031-agent",
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
    let decision_response: Value = actor(
        client.post(format!("{base}/atelier/facial/review/decisions")),
        "mt-031-agent",
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

    // ── BLOCKED: allow_partial=false while undecided items remain -> HTTP-200 envelope.
    let blocked_http = actor(
        client.post(format!("{base}/atelier/facial/review/export")),
        "mt-031-agent",
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
    // Read the receipt back through the existing Facial artifact reader -> durable + attributable.
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

    // ── DEGRADED: allow_partial=true with undecided items -> HTTP-200 degraded + both artifacts.
    let degraded_http = actor(
        client.post(format!("{base}/atelier/facial/review/export")),
        "mt-031-agent",
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

    // ── ERROR: allow_partial=true but a bad dataset_name -> HTTP-200 error envelope, STABLE code only.
    let error_http = actor(
        client.post(format!("{base}/atelier/facial/review/export")),
        "mt-031-agent",
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
    assert!(!error_code.trim().is_empty(), "error code must be non-empty");
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

    // ── SUCCEEDED guard: decide the remaining 2 items -> clean success, NO degraded_reasons.
    for item_id in item_ids.iter().skip(1) {
        let _decided: Value = actor(
            client.post(format!("{base}/atelier/facial/review/decisions")),
            "mt-031-agent",
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
    let succeeded_http = actor(
        client.post(format!("{base}/atelier/facial/review/export")),
        "mt-031-agent",
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

    // ── TRANSPORT DOC: pre-context failures stay bare HTTP 4xx, NOT command envelopes.
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
}
