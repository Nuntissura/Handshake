use handshake_core::kernel::crdt::identity::{CrdtAuthorityLinksV1, CrdtWorkspaceIdentityV1};
use handshake_core::kernel::crdt::persistence::{
    build_crdt_replay_plan, kernel_crdt_postgres_update_log_contract, new_crdt_update_record,
    sha256_hex, validate_crdt_update_record, CrdtReplayMetadataV1, CrdtReplayPlanError,
    CrdtStorageAuthorityPosture, CrdtUpdateRecordInputV1,
};
use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use handshake_core::storage::{tests::postgres_backend_from_env, StorageError};
use serde_json::json;
use uuid::Uuid;

async fn postgres_or_environment_blocked() -> std::sync::Arc<dyn handshake_core::storage::Database>
{
    match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            panic!("ENVIRONMENT_BLOCKED: Kernel002 CRDT persistence tests require POSTGRES_TEST_URL; {msg}");
        }
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
}

#[test]
fn kernel_crdt_update_record_carries_postgres_order_hash_actor_session_and_replay_metadata() {
    let record = sample_record(1, "crdt-update-1", b"first-update", "sv-0", "sv-1");

    assert_eq!(record.schema_id, "hsk.kernel.crdt_update_record@1");
    assert_eq!(record.update_seq, 1);
    assert_eq!(record.update_sha256, sha256_hex(b"first-update"));
    assert_eq!(record.actor_id, "actor-kernel-builder");
    assert_eq!(record.actor_kind, "model");
    assert_eq!(record.session_id, "session-kernel-builder");
    assert_eq!(record.trace_id, "trace-crdt-update-1");
    assert_eq!(record.replay_metadata.encoding, "yjs-update-v1");
    assert_eq!(
        record.storage_authority,
        CrdtStorageAuthorityPosture::PostgresEventLedger
    );

    validate_crdt_update_record(&record).expect("persisted update record must validate");
}

#[test]
fn kernel_crdt_replay_plan_reconstructs_workspace_after_restart_from_persisted_updates() {
    let second = sample_record(2, "crdt-update-2", b"second-update", "sv-1", "sv-2");
    let first = sample_record(1, "crdt-update-1", b"first-update", "sv-0", "sv-1");

    let plan = build_crdt_replay_plan(&[second, first]).expect("persisted updates must replay");

    assert_eq!(plan.workspace_id, "workspace-kernel");
    assert_eq!(plan.document_id, "document-kernel");
    assert_eq!(plan.crdt_document_id, "crdt-document-kernel");
    assert_eq!(plan.final_state_vector, "sv-2");
    assert_eq!(plan.ordered_updates.len(), 2);
    assert_eq!(plan.ordered_updates[0].update_seq, 1);
    assert_eq!(plan.ordered_updates[1].update_seq, 2);
    assert!(plan
        .ordered_updates
        .iter()
        .all(|step| step.update_bytes_ref.starts_with("postgres://")));
}

#[test]
fn kernel_crdt_persistence_rejects_filesystem_authority_and_broken_replay_order() {
    let mut file_backed = sample_record(1, "crdt-update-1", b"first-update", "sv-0", "sv-1");
    file_backed.update_bytes_ref = "file://workspace/cache/update.bin".to_string();
    file_backed.storage_authority = CrdtStorageAuthorityPosture::FileSystemAuthority;

    let errors = validate_crdt_update_record(&file_backed)
        .expect_err("filesystem authority must not validate");
    assert!(errors
        .iter()
        .any(|error| error.field == "storage_authority"));
    assert!(errors.iter().any(|error| error.field == "update_bytes_ref"));

    let first = sample_record(1, "crdt-update-1", b"first-update", "sv-0", "sv-1");
    let third = sample_record(3, "crdt-update-3", b"third-update", "sv-2", "sv-3");
    let error = build_crdt_replay_plan(&[first, third])
        .expect_err("gap in persisted update order must fail replay");
    assert!(matches!(
        error,
        CrdtReplayPlanError::SequenceGap {
            expected: 2,
            found: 3
        }
    ));
}

#[test]
fn kernel_crdt_postgres_update_log_contract_declares_persistence_columns_and_constraints() {
    let contract = kernel_crdt_postgres_update_log_contract();

    assert_eq!(contract.table_name, "kernel_crdt_updates");
    assert_eq!(
        contract.storage_authority,
        CrdtStorageAuthorityPosture::PostgresEventLedger
    );
    for column in [
        "workspace_id",
        "document_id",
        "crdt_document_id",
        "update_seq",
        "update_sha256",
        "actor_id",
        "session_id",
        "replay_metadata_json",
        "event_ledger_event_id",
    ] {
        assert!(contract.required_columns.contains(&column));
    }
    assert!(contract
        .unique_constraints
        .contains(&"workspace_id,document_id,crdt_document_id,update_seq"));
    assert!(contract
        .denied_authority_refs
        .contains(&"filesystem_update_bytes"));
}

#[tokio::test]
async fn kernel_crdt_updates_persist_and_replay_after_postgres_reconnect() {
    let db = postgres_or_environment_blocked().await;
    let suffix = Uuid::now_v7().simple().to_string();
    let mut first =
        sample_record_for_workspace(&suffix, 1, "crdt-update-1", b"first-update", "sv-0", "sv-1");
    let mut second = sample_record_for_workspace(
        &suffix,
        2,
        "crdt-update-2",
        b"second-update",
        "sv-1",
        "sv-2",
    );
    first.event_ledger_event_id =
        append_kernel_crdt_event(db.as_ref(), &suffix, &first.update_id).await;
    second.event_ledger_event_id =
        append_kernel_crdt_event(db.as_ref(), &suffix, &second.update_id).await;

    db.append_kernel_crdt_update(first.clone(), b"first-update".to_vec())
        .await
        .expect("append first CRDT update to Postgres");
    db.append_kernel_crdt_update(second.clone(), b"second-update".to_vec())
        .await
        .expect("append second CRDT update to Postgres");

    let replayed = db
        .list_kernel_crdt_updates(
            &first.workspace_id,
            &first.document_id,
            &first.crdt_document_id,
        )
        .await
        .expect("list persisted CRDT updates");
    let plan = build_crdt_replay_plan(&replayed).expect("persisted Postgres updates replay");

    assert_eq!(replayed.len(), 2);
    assert_eq!(replayed[0].update_id, first.update_id);
    assert_eq!(replayed[1].update_id, second.update_id);
    assert_eq!(plan.final_state_vector, "sv-2");
    assert!(replayed.iter().all(|record| record
        .update_bytes_ref
        .starts_with("postgres://kernel_crdt_updates/")));
    assert_eq!(
        db.read_kernel_crdt_update_bytes(&first.update_bytes_ref)
            .await
            .expect("read first CRDT update bytes"),
        b"first-update".to_vec()
    );
    assert_eq!(
        db.read_kernel_crdt_update_bytes(&second.update_bytes_ref)
            .await
            .expect("read second CRDT update bytes"),
        b"second-update".to_vec()
    );
}

#[tokio::test]
async fn kernel_crdt_update_persistence_rejects_missing_eventledger_ref() {
    let db = postgres_or_environment_blocked().await;
    let suffix = Uuid::now_v7().simple().to_string();
    let missing_event = sample_record_for_workspace(
        &suffix,
        1,
        "crdt-update-missing-event",
        b"missing-event-update",
        "sv-0",
        "sv-1",
    );

    let error = db
        .append_kernel_crdt_update(missing_event, b"missing-event-update".to_vec())
        .await
        .expect_err("CRDT updates must cite an existing EventLedger event");
    assert!(matches!(
        error,
        StorageError::Validation(message)
            if message.contains("kernel CRDT EventLedger event ref is missing")
    ));
}

async fn append_kernel_crdt_event(
    db: &(dyn handshake_core::storage::Database + '_),
    suffix: &str,
    update_id: &str,
) -> String {
    let event = NewKernelEvent::builder(
        format!("KTR-CRDT-PERSISTENCE-{suffix}"),
        format!("SR-CRDT-PERSISTENCE-{suffix}"),
        KernelEventType::ArtifactStored,
        KernelActor::System("kernel-crdt-persistence-test".to_string()),
    )
    .aggregate("kernel_crdt_update", update_id.to_string())
    .idempotency_key(format!("kernel-crdt-update:{suffix}:{update_id}"))
    .source_component("kernel_crdt_persistence_test")
    .payload(json!({
        "suffix": suffix,
        "update_id": update_id
    }))
    .build()
    .expect("valid CRDT persistence event");

    db.append_kernel_event(event)
        .await
        .expect("append CRDT persistence EventLedger event")
        .event_id
}

fn sample_record(
    update_seq: u64,
    update_id: &str,
    update_bytes: &[u8],
    state_vector_before: &str,
    state_vector_after: &str,
) -> handshake_core::kernel::crdt::persistence::CrdtUpdateRecordV1 {
    let identity = sample_identity();
    new_crdt_update_record(CrdtUpdateRecordInputV1 {
        identity: &identity,
        update_id,
        update_seq,
        update_bytes,
        update_bytes_ref: &format!("postgres://kernel_crdt_updates/{update_id}/update_bytes"),
        session_id: "session-kernel-builder",
        trace_id: &format!("trace-{update_id}"),
        state_vector_before,
        state_vector_after,
        replay_metadata: CrdtReplayMetadataV1 {
            replay_order_key: format!("workspace-kernel/document-kernel/{update_seq:020}"),
            dependency_update_ids: Vec::new(),
            encoding: "yjs-update-v1".to_string(),
            schema_version: "kernel-crdt-update-v1".to_string(),
        },
        event_ledger_event_id: &format!("evt-{update_id}"),
    })
}

fn sample_record_for_workspace(
    suffix: &str,
    update_seq: u64,
    update_id: &str,
    update_bytes: &[u8],
    state_vector_before: &str,
    state_vector_after: &str,
) -> handshake_core::kernel::crdt::persistence::CrdtUpdateRecordV1 {
    let mut identity = sample_identity();
    identity.workspace_id = format!("workspace-kernel-{suffix}");
    identity.document_id = format!("document-kernel-{suffix}");
    identity.crdt_document_id = format!("crdt-document-kernel-{suffix}");
    identity.authority_links.event_ledger_stream_id = format!("event-ledger-stream-{suffix}");
    new_crdt_update_record(CrdtUpdateRecordInputV1 {
        identity: &identity,
        update_id,
        update_seq,
        update_bytes,
        update_bytes_ref: &format!(
            "postgres://kernel_crdt_updates/{}/{update_id}/update_bytes",
            identity.crdt_document_id
        ),
        session_id: "session-kernel-builder",
        trace_id: &format!("trace-{suffix}-{update_id}"),
        state_vector_before,
        state_vector_after,
        replay_metadata: CrdtReplayMetadataV1 {
            replay_order_key: format!(
                "{}/{}/{update_seq:020}",
                identity.workspace_id, identity.document_id
            ),
            dependency_update_ids: Vec::new(),
            encoding: "yjs-update-v1".to_string(),
            schema_version: "kernel-crdt-update-v1".to_string(),
        },
        event_ledger_event_id: &format!("evt-{suffix}-{update_id}"),
    })
}

fn sample_identity() -> CrdtWorkspaceIdentityV1 {
    CrdtWorkspaceIdentityV1 {
        schema_id: "hsk.kernel.crdt_workspace_identity@1".to_string(),
        workspace_id: "workspace-kernel".to_string(),
        document_id: "document-kernel".to_string(),
        crdt_document_id: "crdt-document-kernel".to_string(),
        actor_id: "actor-kernel-builder".to_string(),
        actor_kind: "model".to_string(),
        crdt_site_id: "site-kernel-builder".to_string(),
        crdt_client_id: "client-kernel-builder".to_string(),
        document_schema_id: "hsk.kernel.crdt_document@1".to_string(),
        authority_links: CrdtAuthorityLinksV1 {
            work_item_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-011"
                .to_string(),
            action_trace_id: "trace-crdt-workspace".to_string(),
            artifact_proposal_id: "artifact-proposal-crdt".to_string(),
            role_mailbox_thread_id: "role-mailbox-thread-crdt".to_string(),
            dcc_projection_id: "dcc-crdt-projection".to_string(),
            event_ledger_stream_id: "event-ledger-stream-crdt".to_string(),
        },
    }
}
