//! WP-KERNEL-005 MT-139 action receipt schema proof.
//!
//! Generic model-visible action receipts persist only parameter hashes plus
//! actor/session/timing/status/refs, then mirror the receipt through the
//! canonical Atelier EventLedger family.

mod atelier_pg_support;

use atelier_pg_support::database_url;
use chrono::Utc;
use handshake_core::atelier::AtelierStore;
use handshake_core::atelier::action_receipt::{
    ActionReceiptStatus, NewActionReceipt, action_receipt_event_family,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{Database, postgres::PostgresDatabase};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

async fn connected_store_with_ledger(url: &str) -> (AtelierStore, Arc<dyn Database>) {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("connect to PostgreSQL");
    let database = PostgresDatabase::new(pool.clone());
    database
        .run_migrations()
        .await
        .expect("run kernel migrations");
    let database = database.into_arc();
    let store = AtelierStore::with_event_ledger(pool, database.clone());
    store.ensure_schema().await.expect("ensure atelier schema");
    (store, database)
}

fn valid_receipt() -> NewActionReceipt {
    NewActionReceipt {
        action_id: "kernel.action_catalog.view".to_string(),
        actor_kind: "agent".to_string(),
        actor_id: format!("test-agent-{}", Uuid::new_v4()),
        session_id: format!("test-session-{}", Uuid::new_v4()),
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
async fn action_receipt_records_model_visible_operation_without_raw_params() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP action_receipt_records_model_visible_operation_without_raw_params: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

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
}

#[tokio::test]
async fn action_receipt_rejects_unknown_action_and_incomplete_failure_receipt() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP action_receipt_rejects_unknown_action_and_incomplete_failure_receipt: PostgreSQL unavailable"
        );
        return;
    };
    let (store, _) = connected_store_with_ledger(&url).await;

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
}
