//! Embedded-Surreal runtime-owner identity persistence and scope guards.

mod process_ledger_surreal_support;

use handshake_core::process_ledger::{
    LedgerEvent, ProcessEngineKind, ProcessLedgerStore, ProcessRuntimeOwner, ProcessStart,
    ProcessStop,
};
use process_ledger_surreal_support::ProcessLedgerSurrealHarness;
use uuid::Uuid;

fn runtime_owner(runtime_instance_id: Uuid) -> ProcessRuntimeOwner {
    ProcessRuntimeOwner {
        runtime_instance_id,
        host_scope_id: "surreal-runtime-owner-proof".to_string(),
        lease_schema_id: "hsk.embedded_runtime.instance@2".to_string(),
        lease_protocol: "tcp-loopback-connect-v1".to_string(),
        lease_address: "127.0.0.1".to_string(),
        lease_port: 32123,
    }
}

#[tokio::test]
async fn exact_scope_start_stop_persists_one_complete_typed_runtime_owner() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let owner = runtime_owner(Uuid::now_v7());
    let start = ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        "surreal-runtime-owner-proof",
        Some("WP-1".to_string()),
    )
    .with_parent_session_id("surreal-runtime-owner-proof")
    .with_runtime_owner(owner.clone())
    .with_metadata_jsonb(harness.metadata());
    let stop = ProcessStop::from_start(&start, Some(0)).with_stop_reason("proof-completed");
    harness
        .write_batch(vec![
            LedgerEvent::Start(start.clone()),
            LedgerEvent::Stop(stop),
        ])
        .await
        .expect("atomic exact-scope START/STOP");

    let row = harness
        .lifecycle(start.process_uuid)
        .await
        .expect("exact-scope lifecycle row");
    assert!(row.stopped_at.is_some());
    assert_eq!(row.exit_code, Some(0));
    assert_eq!(row.stop_reason.as_deref(), Some("proof-completed"));
    assert_eq!(
        row.owner_runtime_instance_id,
        Some(owner.runtime_instance_id)
    );
    assert_eq!(
        row.owner_host_scope_id.as_deref(),
        Some(owner.host_scope_id.as_str())
    );
    assert_eq!(
        row.owner_lease_schema_id.as_deref(),
        Some(owner.lease_schema_id.as_str())
    );
    assert_eq!(
        row.owner_lease_protocol.as_deref(),
        Some(owner.lease_protocol.as_str())
    );
    assert_eq!(
        row.owner_lease_address.as_deref(),
        Some(owner.lease_address.as_str())
    );
    assert_eq!(row.owner_lease_port, Some(i64::from(owner.lease_port)));
    assert_eq!(harness.process_event_count().await, 2);
    harness.close().await;
}

#[tokio::test]
async fn missing_all_five_scope_fields_rejects_new_runtime_owner_row() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let start = ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        "surreal-runtime-owner-missing-scope",
        Some("WP-1".to_string()),
    )
    .with_runtime_owner(runtime_owner(Uuid::now_v7()));
    let error = harness
        .store()
        .write_batch(vec![LedgerEvent::Start(start.clone())])
        .await
        .expect_err("unattributed runtime owner must fail closed");
    assert!(error.to_string().contains("five ResourceScope fields"));
    assert!(harness.lifecycle(start.process_uuid).await.is_none());
    assert_eq!(harness.process_event_count().await, 0);
    harness.close().await;
}
