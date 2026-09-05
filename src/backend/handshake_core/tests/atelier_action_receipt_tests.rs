//! WP-KERNEL-005 MT-139 action receipt schema proof.
//!
//! Generic model-visible action receipts persist only parameter hashes plus
//! actor/session/timing/status/refs, then mirror the receipt through the
//! canonical Atelier EventLedger family.
//!
//! WP-CKC-posekit-overhaul MT-022/MT-062: receipts recorded under a
//! model-operation lease carry `thread_id` + `lease_claim_id` lineage, and that
//! lineage must agree with the referenced lease (actor, session, thread).

mod atelier_surreal_support;

use atelier_surreal_support::AtelierSurrealHarness;
use chrono::Utc;
use handshake_core::atelier::action_receipt::{
    action_receipt_event_family, ActionReceiptStatus, NewActionReceipt,
};
use handshake_core::atelier::model_lease::NewModelLeaseClaim;
use handshake_core::kernel::role_mailbox_claim_lease::{
    RoleMailboxClaimMode, RoleMailboxExecutorKind,
};
use handshake_core::kernel::KernelEventType;
use serde_json::json;
use uuid::Uuid;

fn valid_receipt() -> NewActionReceipt {
    NewActionReceipt {
        action_id: "kernel.action_catalog.view".to_string(),
        actor_kind: "agent".to_string(),
        actor_id: format!("test-agent-{}", Uuid::new_v4()),
        session_id: format!("test-session-{}", Uuid::new_v4()),
        thread_id: String::new(),
        lease_claim_id: None,
        params: json!({
            "query": "super-secret-raw-param",
            "limit": 25,
        }),
        started_at_utc: Utc::now(),
        completed_at_utc: Utc::now(),
        status: ActionReceiptStatus::Succeeded,
        target_refs: vec!["kernel://action-catalog/kernel002-action-catalog-v1".to_string()],
        evidence_refs: vec!["src/backend/handshake_core/src/kernel/action_catalog.rs".to_string()],
        result_refs: vec!["kernel://action-catalog/view-result".to_string()],
        error_class: None,
        recovery_hint: None,
    }
}

#[tokio::test]
async fn action_receipt_lease_lineage_must_match_referenced_claim() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let unique = Uuid::new_v4();
    let thread_id = format!("atelier.test.receipt-lineage.{unique}");
    let actor_id = format!("receipt-agent-{unique}");
    let session_id = format!("receipt-session-{unique}");
    let lease = store
        .claim_model_lease(&NewModelLeaseClaim {
            thread_id: thread_id.clone(),
            executor_kind: RoleMailboxExecutorKind::LocalLargeModel,
            actor_id: actor_id.clone(),
            session_id: session_id.clone(),
            claim_mode: RoleMailboxClaimMode::ExclusiveLease,
            ttl_seconds: 900,
            linked_work_packet_id: "WP-CKC-posekit-overhaul".to_owned(),
            linked_micro_task_id: "MT-022".to_owned(),
        })
        .await
        .expect("claim receipt lineage lease");

    let mut receipt = valid_receipt();
    receipt.actor_id = actor_id;
    receipt.session_id = session_id;
    receipt.thread_id = thread_id;
    receipt.lease_claim_id = Some(lease.claim_id);
    let persisted = store
        .record_action_receipt(&receipt)
        .await
        .expect("matching receipt lineage persists");
    assert_eq!(persisted.lease_claim_id, Some(lease.claim_id));
    assert_eq!(persisted.thread_id, lease.thread_id);

    let reloaded = store
        .get_action_receipt(persisted.receipt_id)
        .await
        .expect("reload lineage receipt");
    assert_eq!(reloaded, persisted, "lineage fields must round-trip");

    let mut mismatched = receipt.clone();
    mismatched.session_id = format!("wrong-session-{unique}");
    let err = store
        .record_action_receipt(&mismatched)
        .await
        .expect_err("mismatched receipt session must be rejected");
    assert!(
        err.to_string().contains("lease lineage mismatch"),
        "lineage mismatch rejection must name lease lineage: {err}"
    );

    let mut unknown_lease = receipt.clone();
    unknown_lease.lease_claim_id = Some(Uuid::now_v7());
    let err = store
        .record_action_receipt(&unknown_lease)
        .await
        .expect_err("a receipt citing a non-existent lease must be rejected");
    assert!(
        matches!(err, handshake_core::atelier::AtelierError::NotFound(_)),
        "unknown lease must surface as NotFound, got {err:?}"
    );

    let mut padded_thread = valid_receipt();
    padded_thread.thread_id = " legacy ".to_owned();
    let err = store
        .record_action_receipt(&padded_thread)
        .await
        .expect_err("padded thread_id on a legacy receipt must be rejected");
    assert!(
        err.to_string().contains("thread_id"),
        "padded thread_id rejection must name the field: {err}"
    );

    harness.shutdown().await;
}

#[tokio::test]
async fn action_receipt_records_model_visible_operation_without_raw_params() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let database = &harness.database;

    let receipt = store
        .record_action_receipt(&valid_receipt())
        .await
        .expect("record generic action receipt");

    assert_eq!(receipt.action_id, "kernel.action_catalog.view");
    assert_eq!(receipt.status, ActionReceiptStatus::Succeeded);
    assert!(receipt.params_sha256.starts_with("sha256:"));
    assert!(!receipt.params_sha256.contains("super-secret-raw-param"));
    assert!(!receipt.actor_id.is_empty());
    assert!(!receipt.session_id.is_empty());
    assert!(receipt.completed_at_utc >= receipt.started_at_utc);
    assert!(!receipt.target_refs.is_empty());
    assert!(!receipt.evidence_refs.is_empty());
    assert!(!receipt.result_refs.is_empty());

    let reloaded = store
        .get_action_receipt(receipt.receipt_id)
        .await
        .expect("reload action receipt");
    assert_eq!(reloaded, receipt);

    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_action_receipt", &receipt.receipt_id.to_string())
        .await
        .expect("list action receipt EventLedger rows");
    let event = kernel_events
        .iter()
        .find(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == action_receipt_event_family::ACTION_RECEIPT_RECORDED
        })
        .expect("action receipt must emit canonical EventLedger event");
    assert_eq!(
        event.payload["atelier_payload"]["action_id"],
        serde_json::json!("kernel.action_catalog.view")
    );
    assert_eq!(
        event.payload["atelier_payload"]["params_sha256"],
        serde_json::json!(receipt.params_sha256)
    );
    assert!(
        !event.payload.to_string().contains("super-secret-raw-param"),
        "EventLedger payload must not leak raw action params"
    );

    harness.shutdown().await;
}

#[tokio::test]
async fn action_receipt_rejects_unknown_action_and_incomplete_failure_receipt() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;

    let mut unknown_action = valid_receipt();
    unknown_action.action_id = "kernel.not_in_catalog".to_string();
    let err = store
        .record_action_receipt(&unknown_action)
        .await
        .expect_err("unknown model-visible action must be rejected");
    assert!(
        err.to_string().contains("kernel.not_in_catalog"),
        "unknown-action rejection must name the action id: {err}"
    );

    let mut incomplete_failure = valid_receipt();
    incomplete_failure.status = ActionReceiptStatus::Failed;
    let err = store
        .record_action_receipt(&incomplete_failure)
        .await
        .expect_err("failed receipt must carry recovery details");
    assert!(
        err.to_string().contains("recovery_hint"),
        "failed receipt rejection must name recovery_hint: {err}"
    );

    harness.shutdown().await;
}
