#![cfg(feature = "inspector")]

use std::sync::Arc;

use handshake_core::{
    inspector_read::{
        expected_write_box_v1_signature, EventLedgerRow, InspectorReadSnapshot, InspectorServer,
        PerRunSecret, ReplayDriveResponse, PER_RUN_SECRET_HEADER, WRITE_BOX_V1_ENVELOPE_SCHEMA_ID,
    },
    kernel::{
        action_envelope::AuthorityEffect,
        write_boxes::{
            WriteBoxCommon, WriteBoxKind, WriteBoxLifecycleState, WriteBoxOwnerRef,
            WriteBoxPayloadRef, WriteBoxReplayMetadataV1, WriteBoxTargetRef,
            WriteBoxValidationState, WriteBoxValidationStatus,
        },
        KernelEventType,
    },
};
use reqwest::StatusCode;
use serde_json::{json, Value};

#[tokio::test]
async fn replay_drive_tests_accepts_valid_envelope_and_emits_event_receipt() {
    let handle = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");
    let client = reqwest::Client::new();
    let secret = handle.per_run_secret();
    let body = valid_body("kernel.write_box.promote", secret);

    let response: ReplayDriveResponse = client
        .post(format!(
            "http://{}/inspector/v1/replay-drive",
            handle.addr()
        ))
        .header(PER_RUN_SECRET_HEADER, secret.to_hex())
        .json(&body)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(response.schema_id, "hsk.inspector.replay_drive.response@1");
    assert_eq!(response.action_id, "kernel.write_box.promote");
    assert_eq!(response.status, "dispatched");
    assert_eq!(response.write_box_id, "WB-REPLAY-1");
    assert_eq!(response.event.event_type, "INSPECTOR_REPLAY_DRIVE");
    assert_eq!(response.event.envelope_signer, "KERNEL_BUILDER");
    assert_eq!(
        response.result["dispatched_through"],
        "KernelActionCatalogV1"
    );
    assert_eq!(
        KernelEventType::try_from("INSPECTOR_REPLAY_DRIVE").unwrap(),
        KernelEventType::InspectorReplayDrive
    );
}

#[tokio::test]
async fn replay_drive_tests_rejects_invalid_signature_with_403() {
    let handle = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");
    let client = reqwest::Client::new();
    let secret = handle.per_run_secret();
    let mut body = valid_body("kernel.write_box.promote", secret);
    body["envelope"]["signature"] = json!("hmac-sha256:bad-signature");

    let response = client
        .post(format!(
            "http://{}/inspector/v1/replay-drive",
            handle.addr()
        ))
        .header(PER_RUN_SECRET_HEADER, secret.to_hex())
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn replay_drive_tests_unknown_catalog_action_returns_404() {
    let handle = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");
    let client = reqwest::Client::new();
    let secret = handle.per_run_secret();

    let response = client
        .post(format!(
            "http://{}/inspector/v1/replay-drive",
            handle.addr()
        ))
        .header(PER_RUN_SECRET_HEADER, secret.to_hex())
        .json(&valid_body("kernel.missing.action", secret))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn replay_drive_tests_forbidden_shapes_return_403_not_400() {
    let handle = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");
    let client = reqwest::Client::new();
    let secret = handle.per_run_secret();
    let base = format!("http://{}/inspector/v1/replay-drive", handle.addr());

    let mut extra = valid_body("kernel.write_box.promote", secret);
    extra["payload"] = json!({"parallel": "mutation"});

    for body in [
        extra,
        json!({"catalog_action": "kernel.write_box.promote", "payload": {}}),
        json!({"action_id": "kernel.write_box.promote"}),
    ] {
        let response = client
            .post(&base)
            .header(PER_RUN_SECRET_HEADER, secret.to_hex())
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}

#[tokio::test]
async fn replay_drive_tests_malformed_json_returns_400() {
    let handle = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");
    let client = reqwest::Client::new();
    let secret = handle.per_run_secret();

    let response = client
        .post(format!(
            "http://{}/inspector/v1/replay-drive",
            handle.addr()
        ))
        .header(PER_RUN_SECRET_HEADER, secret.to_hex())
        .header("content-type", "application/json")
        .body("{not-json")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// CRIT-4 / MT-029 regression: an envelope signed under a different
// per-launch secret (i.e. the attacker does not know the live secret)
// must be rejected. This is the agent-corrupts-agent path the keyless
// SHA-256 verifier left open.
#[tokio::test]
async fn replay_drive_tests_rejects_envelope_forged_under_other_secret() {
    let handle = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");
    let client = reqwest::Client::new();
    let live_secret = handle.per_run_secret();
    let other_secret = PerRunSecret::from_bytes([0xAB; 16]);
    assert_ne!(other_secret.to_hex(), live_secret.to_hex());

    // Sign with the wrong secret; present the correct header so the only
    // remaining mismatch is the HMAC verifier.
    let body = valid_body("kernel.write_box.promote", &other_secret);

    let response = client
        .post(format!(
            "http://{}/inspector/v1/replay-drive",
            handle.addr()
        ))
        .header(PER_RUN_SECRET_HEADER, live_secret.to_hex())
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// MT-029 regression: missing the per-run secret header must reject 401
// before envelope verification even runs.
#[tokio::test]
async fn replay_drive_tests_rejects_missing_per_run_secret_header() {
    let handle = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");
    let client = reqwest::Client::new();
    let secret = handle.per_run_secret();
    let body = valid_body("kernel.write_box.promote", secret);

    let response = client
        .post(format!(
            "http://{}/inspector/v1/replay-drive",
            handle.addr()
        ))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// MT-029 regression: wrong-value per-run secret header rejected 401.
#[tokio::test]
async fn replay_drive_tests_rejects_wrong_per_run_secret_header() {
    let handle = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");
    let client = reqwest::Client::new();
    let secret = handle.per_run_secret();
    let body = valid_body("kernel.write_box.promote", secret);

    let response = client
        .post(format!(
            "http://{}/inspector/v1/replay-drive",
            handle.addr()
        ))
        .header(
            PER_RUN_SECRET_HEADER,
            PerRunSecret::from_bytes([0x11; 16]).to_hex(),
        )
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

fn valid_body(action_id: &str, secret: &PerRunSecret) -> Value {
    let write_box = sample_write_box();
    let signature = expected_write_box_v1_signature(secret, "KERNEL_BUILDER", &write_box);
    json!({
        "action_id": action_id,
        "envelope": {
            "schema_id": WRITE_BOX_V1_ENVELOPE_SCHEMA_ID,
            "signer": "KERNEL_BUILDER",
            "signature": signature,
            "write_box": write_box,
        }
    })
}

fn sample_reader() -> InspectorReadSnapshot {
    let mut snapshot = InspectorReadSnapshot::default();
    snapshot.event_ledger_tail.push(EventLedgerRow {
        event_id: "evt-1".to_string(),
        event_type: "session_started".to_string(),
        event_sequence: 1,
        created_at_utc: "2026-05-18T11:00:00Z".to_string(),
        ..EventLedgerRow::default()
    });
    snapshot
}

fn sample_write_box() -> WriteBoxCommon {
    WriteBoxCommon {
        write_box_id: "WB-REPLAY-1".to_string(),
        kind: WriteBoxKind::Promotion,
        schema_version: "hsk.write_box.promotion@1".to_string(),
        workspace_id: "workspace-alpha".to_string(),
        owner: WriteBoxOwnerRef {
            actor_id: "actor-kernel-builder".to_string(),
            actor_kind: "role".to_string(),
            role_id: "KERNEL_BUILDER".to_string(),
        },
        crdt_site_id: "site-alpha".to_string(),
        target_refs: vec![WriteBoxTargetRef {
            target_id: "promotion-target".to_string(),
            target_kind: "write_box".to_string(),
            authority_class: "event_ledger".to_string(),
        }],
        base_snapshot_refs: vec!["snapshot://workspace-alpha/base".to_string()],
        intent_summary: "Replay-drive a validated promotion box through the catalog.".to_string(),
        operation_payload_refs: vec![WriteBoxPayloadRef {
            payload_id: "payload-1".to_string(),
            payload_kind: "promotion_request".to_string(),
            payload_ref: "artifact://payload-1".to_string(),
            payload_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
        }],
        lifecycle_state: WriteBoxLifecycleState::Validated,
        allowed_transitions: vec![
            WriteBoxLifecycleState::Validated,
            WriteBoxLifecycleState::PromotionQueued,
            WriteBoxLifecycleState::Promoted,
        ],
        authority_effect: AuthorityEffect::EventLedgerAuthorityWrite,
        evidence_refs: vec!["event://evidence-1".to_string()],
        receipt_refs: vec!["receipt://validation-1".to_string()],
        denial_receipt_refs: Vec::new(),
        promotion_receipt_refs: vec!["receipt://promotion-1".to_string()],
        validation_status: WriteBoxValidationStatus {
            state: WriteBoxValidationState::Valid,
            check_ids: vec!["schema_validity".to_string()],
        },
        projection_rules: vec!["dcc.write_box.queue".to_string()],
        replay_metadata: WriteBoxReplayMetadataV1 {
            replay_plan_ref: "KTR-REPLAY-1".to_string(),
            replay_order_key: "001".to_string(),
            idempotency_key: "IK-REPLAY-1".to_string(),
            source_event_refs: vec!["event://source-1".to_string()],
        },
    }
}
