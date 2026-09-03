use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde_json::{json, Value};
use surrealdb::types::{RecordId, SurrealValue};
use uuid::Uuid;

use handshake_core::{
    kernel::KernelEventType,
    process_ledger::{
        is_degraded, LedgerEvent, LedgerEventKind, LedgerOverflowEvent, ProcessEngineKind,
        ProcessLedgerError, ProcessLedgerOverflowSink, ProcessLedgerStore, ProcessLedgerWriter,
        ProcessRuntimeOwner, ProcessStart, ProcessStop, ReclaimKillOperationCandidate,
        ReclaimKillOperationStatus, ReclaimProcessStore, ReclaimResourceScope,
        SurrealProcessLedgerStore, WriterConfig, EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID,
        EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL, PROCESS_LEDGER_DEFAULT_CHANNEL_CAPACITY,
        PROCESS_LEDGER_RING_CAPACITY, PROCESS_LEDGER_TABLE_NAME,
        RECLAIM_IN_PROGRESS_RECOVERY_LIMIT,
    },
    storage::surreal::{bootstrap_schema, SurrealStorage},
};

mod surreal_test_store_support;

use surreal_test_store_support::EmbeddedSurrealTestScope;

const PROCESS_LEDGER_SOURCE_FILES: &[&str] = &[
    "src/process_ledger/mod.rs",
    "src/process_ledger/mt_executor.rs",
    "src/process_ledger/reclaim.rs",
    "src/process_ledger/restart_resume.rs",
    "src/process_ledger/table.rs",
    "src/process_ledger/writer.rs",
    "src/storage/surreal/process_ledger.rs",
    "tests/process_ledger_surreal_tests.rs",
];

#[test]
fn surreal_schema_declares_process_lifecycle_and_exact_scope() {
    assert_eq!(PROCESS_LEDGER_TABLE_NAME, "kernel_process_lifecycle");
    assert_eq!(
        PROCESS_LEDGER_DEFAULT_CHANNEL_CAPACITY,
        PROCESS_LEDGER_RING_CAPACITY
    );
    assert_eq!(PROCESS_LEDGER_DEFAULT_CHANNEL_CAPACITY, 10_000);

    let base_schema = include_str!("../src/storage/surreal/schema.surql");
    for definition in [
        "DEFINE TABLE OVERWRITE kernel_process_lifecycle SCHEMAFULL",
        "DEFINE FIELD OVERWRITE process_uuid ON TABLE kernel_process_lifecycle TYPE uuid",
        "DEFINE FIELD OVERWRITE os_pid ON TABLE kernel_process_lifecycle TYPE option<int>",
        "DEFINE FIELD OVERWRITE sandbox_adapter_id ON TABLE kernel_process_lifecycle",
        "DEFINE FIELD OVERWRITE sandbox_internal_id ON TABLE kernel_process_lifecycle",
        "DEFINE FIELD OVERWRITE started_at ON TABLE kernel_process_lifecycle TYPE datetime",
        "DEFINE FIELD OVERWRITE stopped_at ON TABLE kernel_process_lifecycle TYPE option<datetime>",
        "DEFINE FIELD OVERWRITE stop_reason ON TABLE kernel_process_lifecycle TYPE option<string>",
        "DEFINE FIELD OVERWRITE sandbox_capabilities_snapshot ON TABLE kernel_process_lifecycle",
        "DEFINE FIELD OVERWRITE metadata ON TABLE kernel_process_lifecycle",
        "pk_kernel_process_lifecycle",
        "idx_kernel_process_lifecycle_parent_session_started",
        "idx_kernel_process_lifecycle_engine_started",
        "idx_kernel_process_lifecycle_os_pid",
        "idx_kernel_process_lifecycle_adapter_spawned",
        "idx_kernel_process_lifecycle_wp_spawned",
    ] {
        assert!(
            base_schema.contains(definition),
            "missing ProcessLedger schema definition {definition}"
        );
    }
    for engine in [
        "llamacpp",
        "candle",
        "abliteration_tool",
        "sandbox_container",
        "mechanical_job",
        "asr_worker",
        "comfyui_worker",
        "plugin_process",
        "helper_subprocess",
        "external_compat",
        "webview2_cdp",
        "official_cli_bridge",
    ] {
        assert!(
            base_schema.contains(engine),
            "missing ProcessLedger engine kind {engine}"
        );
    }

    let schema = include_str!("../src/storage/surreal/process_ledger_schema.surql");
    for field in [
        "owner_account_id",
        "actor_principal_id",
        "authenticated_session_id",
        "access_space_id",
        "workspace_id",
    ] {
        assert!(
            schema.contains(&format!(
                "DEFINE FIELD IF NOT EXISTS {field} ON kernel_process_lifecycle"
            )),
            "missing exact scope field {field}"
        );
    }
    assert!(schema.contains("idx_kernel_process_lifecycle_exact_scope"));
    assert!(schema.contains("event_ledger_event_id"));
    assert!(schema.contains("record<kernel_event_ledger>"));
}

#[test]
fn overflow_event_is_registered_as_event_ledger_type() {
    assert_eq!(
        KernelEventType::FrEvtLedgerOverflow.as_str(),
        "FR_EVT_LEDGER_OVERFLOW"
    );
    assert_eq!(
        KernelEventType::try_from("FR_EVT_LEDGER_OVERFLOW").unwrap(),
        KernelEventType::FrEvtLedgerOverflow
    );
}

#[test]
fn process_uuid_uses_uuid_v7_and_stop_preserves_immutable_identity() {
    let scope = exact_scope();
    let start = start_event(&scope, "owner", "WP-KERNEL-004");
    assert_eq!(start.process_uuid.get_version_num(), 7);

    let stop = ProcessStop::from_start(&start, Some(0));
    assert_eq!(stop.process_uuid, start.process_uuid);
    assert_eq!(stop.os_pid, start.os_pid);
    assert_eq!(stop.engine_kind, start.engine_kind);
    assert_eq!(stop.owner_role, start.owner_role);
    assert_eq!(stop.metadata_jsonb, start.metadata_jsonb);
}

#[tokio::test]
async fn surreal_store_proves_atomicity_idempotency_exact_scope_and_writer_drain() {
    let fixture = SurrealFixture::open().await;
    let overflow = InMemoryOverflowSink::default();

    let atomic_start = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004");
    let atomic_stop = ProcessStop::from_start(&atomic_start, Some(0));
    fixture
        .store
        .write_batch(vec![
            LedgerEvent::Start(atomic_start.clone()),
            LedgerEvent::Stop(atomic_stop.clone()),
        ])
        .await
        .expect("persist atomic START/STOP batch");

    let final_row = fixture
        .lifecycle(atomic_start.process_uuid, &fixture.scope)
        .await
        .expect("exact-scope lifecycle exists");
    assert_eq!(final_row.process_uuid, atomic_start.process_uuid);
    assert_eq!(
        final_row.engine_kind,
        ProcessEngineKind::HelperSubprocess.as_str()
    );
    assert_eq!(final_row.stopped_at, Some(atomic_stop.stopped_at));
    assert_eq!(final_row.exit_code, Some(0));
    assert_eq!(
        final_row.event_ledger_event_id,
        Some(RecordId::new(
            "kernel_event_ledger",
            format!("process-lifecycle-{}-stop", atomic_start.process_uuid),
        ))
    );
    let start_ledger_event = fixture
        .event(
            atomic_start.process_uuid,
            LedgerEventKind::Start,
            &fixture.scope,
        )
        .await
        .expect("exact-scope START ledger event");
    let stop_ledger_event = fixture
        .event(
            atomic_start.process_uuid,
            LedgerEventKind::Stop,
            &fixture.scope,
        )
        .await
        .expect("exact-scope STOP ledger event");
    assert_eq!(start_ledger_event.event_type, "PROCESS_START");
    assert_eq!(stop_ledger_event.event_type, "PROCESS_STOP");
    assert_eq!(
        start_ledger_event.exact_scope(),
        [
            fixture.scope.account_uuid.to_string(),
            fixture.scope.actor_uuid.to_string(),
            fixture.scope.session_uuid.to_string(),
            fixture.scope.access_space_uuid.to_string(),
            fixture.scope.workspace_id.clone(),
        ]
    );
    assert_eq!(
        stop_ledger_event.exact_scope(),
        start_ledger_event.exact_scope()
    );
    assert!(start_ledger_event.created_at <= stop_ledger_event.created_at);

    let idempotent_start = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004");
    fixture
        .store
        .write_batch(vec![LedgerEvent::Start(idempotent_start.clone())])
        .await
        .expect("persist idempotency START");
    fixture
        .store
        .write_batch(vec![LedgerEvent::Start(idempotent_start.clone())])
        .await
        .expect("identical START replay is idempotent");
    let idempotent_stop = ProcessStop::from_start(&idempotent_start, Some(0));
    fixture
        .store
        .write_batch(vec![LedgerEvent::Stop(idempotent_stop.clone())])
        .await
        .expect("persist idempotency STOP");
    fixture
        .store
        .write_batch(vec![LedgerEvent::Stop(idempotent_stop)])
        .await
        .expect("identical STOP replay is idempotent");

    let verification_start = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004");
    fixture
        .store
        .write_batch(vec![LedgerEvent::Start(verification_start.clone())])
        .await
        .expect("seed final-row verification lifecycle");
    assert_eq!(
        fixture
            .tamper_metadata(verification_start.process_uuid, &fixture.scope)
            .await,
        1,
        "exact-scope fixture tamper must affect one row"
    );
    let error = fixture
        .store
        .write_batch(vec![LedgerEvent::Start(verification_start.clone())])
        .await
        .expect_err("final-row mismatch must fail closed before commit");
    assert!(error
        .to_string()
        .contains("PROCESS_LEDGER_VERIFICATION_MISMATCH:0"));
    assert_eq!(
        fixture
            .lifecycle(verification_start.process_uuid, &fixture.scope)
            .await
            .expect("tampered lifecycle remains authoritative after rollback")
            .metadata["verification_tamper"],
        true
    );

    let conflicting_uuid = Uuid::now_v7();
    let first = start_event_with_uuid(&fixture.scope, conflicting_uuid, 41_001);
    let second = start_event_with_uuid(&fixture.scope, conflicting_uuid, 41_002);
    let error = fixture
        .store
        .write_batch(vec![LedgerEvent::Start(first), LedgerEvent::Start(second)])
        .await
        .expect_err("second-event conflict rejects the whole batch");
    assert!(matches!(
        error,
        ProcessLedgerError::StartIdentityConflict { process_uuid, .. }
            if process_uuid == conflicting_uuid
    ));
    assert!(fixture
        .lifecycle(conflicting_uuid, &fixture.scope)
        .await
        .is_none());
    assert!(fixture
        .event(conflicting_uuid, LedgerEventKind::Start, &fixture.scope)
        .await
        .is_none());

    let mut mismatched_scope = fixture.scope.clone();
    mismatched_scope.access_space_uuid = Uuid::now_v7();
    assert!(fixture
        .lifecycle(atomic_start.process_uuid, &mismatched_scope)
        .await
        .is_none());

    let drain_start = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004");
    let drain_stop = ProcessStop::from_start(&drain_start, Some(0));
    let (writer, join) = ProcessLedgerWriter::spawn(
        fixture.store.clone(),
        Arc::new(overflow.clone()),
        WriterConfig {
            capacity: 8,
            batch_size: 2,
            flush_interval: Duration::from_secs(30),
        },
    );
    writer
        .append_start(drain_start.clone())
        .expect("queue START");
    writer.append_stop(drain_stop).expect("queue STOP");
    drop(writer);
    join.await
        .expect("ProcessLedger writer task joins")
        .expect("ProcessLedger writer drains before shutdown");
    assert!(fixture
        .lifecycle(drain_start.process_uuid, &fixture.scope)
        .await
        .expect("drained lifecycle exists")
        .stopped_at
        .is_some());
    assert!(overflow.events().is_empty());

    let unattributed = ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        "kernel_builder",
        Some("WP-KERNEL-004".to_owned()),
    );
    let error = fixture
        .store
        .write_batch(vec![LedgerEvent::Start(unattributed)])
        .await
        .expect_err("missing all five scope fields must fail closed");
    assert!(error
        .to_string()
        .contains("all five non-empty ResourceScope fields are required"));

    let mut malformed_runtime_owner = test_runtime_owner();
    malformed_runtime_owner.host_scope_id.clear();
    let malformed_owner_start = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_runtime_owner(malformed_runtime_owner);
    let error = fixture
        .store
        .write_batch(vec![LedgerEvent::Start(malformed_owner_start)])
        .await
        .expect_err("malformed runtime-owner identity must fail before persistence");
    assert!(error
        .to_string()
        .contains("runtime-owner host_scope_id is malformed"));

    fixture.close().await;
}

#[tokio::test]
async fn ownership_inspection_is_exact_scope_and_includes_stopped_lifecycle_evidence() {
    let fixture = SurrealFixture::open().await;
    let artifact = "a".repeat(64);
    let runtime_owner = test_runtime_owner();
    let start = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_os_pid(42_019)
        .with_model_artifact_sha256(artifact.clone())
        .with_sandbox_adapter_id("sandbox-adapter-primary")
        .with_runtime_owner(runtime_owner.clone());
    fixture
        .store
        .write_batch(vec![LedgerEvent::Start(start.clone())])
        .await
        .expect("persist inspectable START");

    let open = fixture
        .store
        .inspect_ownership_by_process_uuid(&fixture.scope, start.process_uuid)
        .await
        .expect("inspect exact-scope START")
        .expect("START ownership exists");
    assert_eq!(open.process_uuid, start.process_uuid);
    assert_eq!(open.os_pid, start.os_pid);
    assert_eq!(
        open.model_artifact_sha256.as_deref(),
        Some(artifact.as_str())
    );
    assert_eq!(open.started_at, start.started_at);
    assert_eq!(open.engine_kind, ProcessEngineKind::HelperSubprocess);
    assert_eq!(open.owner_role, "kernel_builder");
    assert_eq!(open.owner_wp.as_deref(), Some("WP-KERNEL-004"));
    assert_eq!(
        open.sandbox_adapter_id.as_deref(),
        Some("sandbox-adapter-primary")
    );
    assert_eq!(open.lifecycle_state, LedgerEventKind::Start);
    assert_eq!(open.stopped_at, None);
    assert_eq!(open.runtime_owner, Some(runtime_owner.clone()));
    assert_eq!(open.resource_scope, fixture.scope);
    assert_eq!(
        open.event_ledger_event_id,
        RecordId::new(
            "kernel_event_ledger",
            format!("process-lifecycle-{}-start", start.process_uuid),
        )
    );

    for mismatched_scope in five_mismatched_scopes(&fixture.scope) {
        assert!(fixture
            .store
            .inspect_ownership_by_process_uuid(&mismatched_scope, start.process_uuid)
            .await
            .expect("foreign process inspection fails without leakage")
            .is_none());
        assert!(fixture
            .store
            .inspect_latest_ownership_by_artifact(&mismatched_scope, &artifact)
            .await
            .expect("foreign artifact inspection fails without leakage")
            .is_none());
    }

    let mut stop = ProcessStop::from_start(&start, Some(17));
    stop.stopped_at = start.started_at + ChronoDuration::seconds(1);
    stop.stop_reason = Some("verified_unload".to_owned());
    fixture
        .store
        .write_batch(vec![LedgerEvent::Stop(stop.clone())])
        .await
        .expect("persist inspectable STOP");

    let stopped = fixture
        .store
        .inspect_ownership_by_process_uuid(&fixture.scope, start.process_uuid)
        .await
        .expect("inspect exact-scope STOP")
        .expect("stopped ownership remains inspectable");
    assert_eq!(stopped.stopped_at, Some(stop.stopped_at));
    assert_eq!(stopped.lifecycle_state, LedgerEventKind::Stop);
    assert_eq!(stopped.exit_code, Some(17));
    assert_eq!(stopped.stop_reason.as_deref(), Some("verified_unload"));
    assert_eq!(stopped.engine_kind, ProcessEngineKind::HelperSubprocess);
    assert_eq!(stopped.owner_role, "kernel_builder");
    assert_eq!(stopped.owner_wp.as_deref(), Some("WP-KERNEL-004"));
    assert_eq!(
        stopped.sandbox_adapter_id.as_deref(),
        Some("sandbox-adapter-primary")
    );
    assert_eq!(stopped.runtime_owner, Some(runtime_owner));
    assert_eq!(
        stopped.event_ledger_event_id,
        RecordId::new(
            "kernel_event_ledger",
            format!("process-lifecycle-{}-stop", start.process_uuid),
        )
    );
    let by_artifact = fixture
        .store
        .inspect_latest_ownership_by_artifact(&fixture.scope, &artifact)
        .await
        .expect("inspect latest exact-scope artifact ownership")
        .expect("stopped artifact ownership remains inspectable");
    assert_eq!(by_artifact, stopped);

    fixture.close().await;
}

#[tokio::test]
async fn artifact_inspection_orders_deterministically_and_rejects_ambiguity_or_duplicates() {
    let fixture = SurrealFixture::open().await;
    let artifact = "b".repeat(64);
    let now = Utc::now();
    let mut older = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256(artifact.clone());
    older.started_at = now - ChronoDuration::seconds(20);
    let mut newer = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256(artifact.clone());
    newer.started_at = now - ChronoDuration::seconds(10);
    fixture
        .store
        .write_batch(vec![
            LedgerEvent::Start(older.clone()),
            LedgerEvent::Start(newer.clone()),
        ])
        .await
        .expect("persist ordered artifact owners");
    let mut newer_stop = ProcessStop::from_start(&newer, Some(0));
    newer_stop.stopped_at = now;
    fixture
        .store
        .write_batch(vec![LedgerEvent::Stop(newer_stop)])
        .await
        .expect("stop latest artifact owner");
    let latest = fixture
        .store
        .inspect_latest_ownership_by_artifact(&fixture.scope, &artifact)
        .await
        .expect("inspect ordered artifact ownership")
        .expect("artifact ownership exists");
    assert_eq!(latest.process_uuid, newer.process_uuid);
    assert!(latest.stopped_at.is_some());

    let tied_artifact = "c".repeat(64);
    let tied_at = now + ChronoDuration::seconds(10);
    let mut tied_a = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256(tied_artifact.clone());
    tied_a.started_at = tied_at;
    let mut tied_b = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256(tied_artifact.clone());
    tied_b.started_at = tied_at;
    fixture
        .store
        .write_batch(vec![LedgerEvent::Start(tied_a), LedgerEvent::Start(tied_b)])
        .await
        .expect("persist tied artifact owners");
    let error = fixture
        .store
        .inspect_latest_ownership_by_artifact(&fixture.scope, &tied_artifact)
        .await
        .expect_err("equal latest timestamps are ambiguous");
    assert!(error
        .to_string()
        .contains("ambiguous latest artifact ownership"));

    let duplicate_artifact = "d".repeat(64);
    let duplicate = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256(duplicate_artifact.clone());
    fixture
        .store
        .write_batch(vec![LedgerEvent::Start(duplicate.clone())])
        .await
        .expect("persist identity to duplicate adversarially");
    fixture
        .store
        .test_duplicate_inspection_identity(&fixture.scope, duplicate.process_uuid)
        .await
        .expect("create isolated duplicate identity fixture");
    let before = fixture
        .lifecycle_count(duplicate.process_uuid, &fixture.scope)
        .await;
    assert_eq!(before, 2);
    let process_error = fixture
        .store
        .inspect_ownership_by_process_uuid(&fixture.scope, duplicate.process_uuid)
        .await
        .expect_err("duplicate process identity fails closed");
    assert!(process_error
        .to_string()
        .contains("duplicate canonical process identity"));
    let artifact_error = fixture
        .store
        .inspect_latest_ownership_by_artifact(&fixture.scope, &duplicate_artifact)
        .await
        .expect_err("duplicate artifact process identity fails closed");
    assert!(artifact_error
        .to_string()
        .contains("duplicate artifact process identity"));
    assert_eq!(
        fixture
            .lifecycle_count(duplicate.process_uuid, &fixture.scope)
            .await,
        before,
        "read-only inspection must not mutate duplicate evidence"
    );

    fixture.close().await;
}

#[tokio::test]
async fn ownership_inspection_rejects_missing_foreign_or_incomplete_evidence_without_mutation() {
    let fixture = SurrealFixture::open().await;

    let missing_link = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256("e".repeat(64));
    fixture
        .store
        .write_batch(vec![LedgerEvent::Start(missing_link.clone())])
        .await
        .expect("persist missing-link fixture");
    fixture
        .store
        .test_set_inspection_event_link(&fixture.scope, missing_link.process_uuid, None)
        .await
        .expect("clear canonical receipt linkage in isolated fixture");
    let before_missing = fixture
        .lifecycle(missing_link.process_uuid, &fixture.scope)
        .await
        .expect("lifecycle remains after linkage tamper");
    let error = fixture
        .store
        .inspect_ownership_by_process_uuid(&fixture.scope, missing_link.process_uuid)
        .await
        .expect_err("missing receipt linkage fails closed");
    assert!(error
        .to_string()
        .contains("missing canonical EventLedger receipt"));
    assert_eq!(
        fixture
            .lifecycle(missing_link.process_uuid, &fixture.scope)
            .await
            .expect("inspection preserved lifecycle")
            .event_ledger_event_id,
        before_missing.event_ledger_event_id
    );

    let foreign_receipt = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256("f".repeat(64));
    fixture
        .store
        .write_batch(vec![LedgerEvent::Start(foreign_receipt.clone())])
        .await
        .expect("persist foreign-receipt fixture");
    let foreign_scope = exact_scope();
    fixture
        .store
        .test_move_inspection_receipt_to_scope(
            &fixture.scope,
            foreign_receipt.process_uuid,
            LedgerEventKind::Start,
            &foreign_scope,
        )
        .await
        .expect("move receipt to foreign exact scope");
    let error = fixture
        .store
        .inspect_ownership_by_process_uuid(&fixture.scope, foreign_receipt.process_uuid)
        .await
        .expect_err("foreign receipt fails closed");
    assert!(error
        .to_string()
        .contains("missing or foreign canonical EventLedger receipt"));
    assert!(fixture
        .store
        .inspect_ownership_by_process_uuid(&foreign_scope, foreign_receipt.process_uuid)
        .await
        .expect("foreign caller receives no lifecycle leakage")
        .is_none());

    let mismatched_link = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256("1".repeat(64));
    let other = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256("2".repeat(64));
    fixture
        .store
        .write_batch(vec![
            LedgerEvent::Start(mismatched_link.clone()),
            LedgerEvent::Start(other.clone()),
        ])
        .await
        .expect("persist mismatched-link fixtures");
    fixture
        .store
        .test_set_inspection_event_link(
            &fixture.scope,
            mismatched_link.process_uuid,
            Some(RecordId::new(
                "kernel_event_ledger",
                format!("process-lifecycle-{}-start", other.process_uuid),
            )),
        )
        .await
        .expect("tamper lifecycle link to another process receipt");
    let error = fixture
        .store
        .inspect_ownership_by_process_uuid(&fixture.scope, mismatched_link.process_uuid)
        .await
        .expect_err("mismatched EventLedger linkage fails closed");
    assert!(error
        .to_string()
        .contains("canonical EventLedger linkage mismatch"));

    let incomplete = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256("3".repeat(64));
    fixture
        .store
        .write_batch(vec![LedgerEvent::Start(incomplete.clone())])
        .await
        .expect("persist incomplete-scope fixture");
    fixture
        .store
        .test_clear_inspection_scope_field(&fixture.scope, incomplete.process_uuid, "workspace_id")
        .await
        .expect("clear one scope field in isolated fixture");
    let error = fixture
        .store
        .inspect_ownership_by_process_uuid(&fixture.scope, incomplete.process_uuid)
        .await
        .expect_err("incomplete lifecycle scope fails closed");
    assert!(error
        .to_string()
        .contains("stored ResourceScope is incomplete"));

    for (index, field, value, expected_error) in [
        (
            0_u8,
            "engine_kind",
            Some("candle"),
            "EventLedger lifecycle identity mismatch",
        ),
        (1, "owner_role", Some(""), "stored owner role is malformed"),
        (
            2,
            "owner_wp",
            None,
            "EventLedger lifecycle identity mismatch",
        ),
        (
            3,
            "sandbox_adapter_id",
            None,
            "EventLedger lifecycle identity mismatch",
        ),
        (4, "owner_wp", Some(""), "stored owner WP is malformed"),
        (
            5,
            "sandbox_adapter_id",
            Some(""),
            "stored sandbox adapter id is malformed",
        ),
    ] {
        let artifact = format!("{:x}", index + 8).repeat(64);
        let identity = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
            .with_model_artifact_sha256(artifact.clone())
            .with_sandbox_adapter_id("sandbox-adapter-primary");
        fixture
            .store
            .write_batch(vec![LedgerEvent::Start(identity.clone())])
            .await
            .expect("persist identity-tamper fixture");
        fixture
            .store
            .test_set_inspection_identity_field(&fixture.scope, identity.process_uuid, field, value)
            .await
            .expect("tamper one canonical identity field");
        let error = if field == "sandbox_adapter_id" {
            fixture
                .store
                .inspect_latest_ownership_by_artifact(&fixture.scope, &artifact)
                .await
                .expect_err("artifact inspector rejects missing identity evidence")
        } else {
            fixture
                .store
                .inspect_ownership_by_process_uuid(&fixture.scope, identity.process_uuid)
                .await
                .expect_err("process inspector rejects malformed or tampered identity")
        };
        assert!(
            error.to_string().contains(expected_error),
            "{field} tamper returned unexpected error: {error}"
        );
        assert_eq!(
            fixture
                .lifecycle_count(identity.process_uuid, &fixture.scope)
                .await,
            1,
            "read-only inspection must not mutate {field} tamper evidence"
        );
    }

    let runtime_owner_tamper = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256("6".repeat(64))
        .with_runtime_owner(test_runtime_owner());
    fixture
        .store
        .write_batch(vec![LedgerEvent::Start(runtime_owner_tamper.clone())])
        .await
        .expect("persist complete runtime-owner tamper fixture");
    fixture
        .store
        .test_replace_inspection_runtime_owner(
            &fixture.scope,
            runtime_owner_tamper.process_uuid,
            &test_runtime_owner(),
        )
        .await
        .expect("replace the complete runtime-owner projection");
    let error = fixture
        .store
        .inspect_ownership_by_process_uuid(&fixture.scope, runtime_owner_tamper.process_uuid)
        .await
        .expect_err("complete runtime-owner replacement must disagree with its receipt");
    assert!(
        error
            .to_string()
            .contains("EventLedger lifecycle identity mismatch"),
        "complete runtime-owner tamper returned unexpected error: {error}"
    );

    let partial_runtime_owner = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256("7".repeat(64))
        .with_runtime_owner(test_runtime_owner());
    fixture
        .store
        .write_batch(vec![LedgerEvent::Start(partial_runtime_owner.clone())])
        .await
        .expect("persist partial runtime-owner tamper fixture");
    fixture
        .store
        .test_clear_inspection_runtime_owner_host_scope(
            &fixture.scope,
            partial_runtime_owner.process_uuid,
        )
        .await
        .expect("clear one runtime-owner field");
    let error = fixture
        .store
        .inspect_latest_ownership_by_artifact(&fixture.scope, &"7".repeat(64))
        .await
        .expect_err("partial runtime-owner identity must fail closed");
    assert!(
        error
            .to_string()
            .contains("runtime-owner identity is incomplete"),
        "partial runtime-owner tamper returned unexpected error: {error}"
    );

    let orphan_receipt = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256("4".repeat(64));
    fixture
        .store
        .write_batch(vec![LedgerEvent::Start(orphan_receipt.clone())])
        .await
        .expect("persist orphan-receipt fixture");
    fixture
        .store
        .test_delete_inspection_lifecycle_projection(&fixture.scope, orphan_receipt.process_uuid)
        .await
        .expect("delete lifecycle while preserving receipt");
    assert!(fixture
        .event(
            orphan_receipt.process_uuid,
            LedgerEventKind::Start,
            &fixture.scope,
        )
        .await
        .is_some());
    let retry_error = fixture
        .store
        .write_batch(vec![LedgerEvent::Start(orphan_receipt.clone())])
        .await
        .expect_err("receipt without lifecycle rejects identical retry");
    assert!(retry_error
        .to_string()
        .contains("PROCESS_LEDGER_VERIFICATION_MISMATCH:0"));
    assert!(fixture
        .lifecycle(orphan_receipt.process_uuid, &fixture.scope)
        .await
        .is_none());
    assert!(fixture
        .event(
            orphan_receipt.process_uuid,
            LedgerEventKind::Start,
            &fixture.scope,
        )
        .await
        .is_some());

    fixture.close().await;
}

#[tokio::test]
async fn ownership_inspection_survives_same_namespace_shutdown_and_reopen() {
    let mut allocator = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate durable embedded Surreal scope");
    let namespace = allocator.namespace().to_owned();
    let database = allocator.database().to_owned();
    let storage = allocator
        .activate_storage()
        .await
        .expect("activate initial injected SurrealStorage");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap shared Surreal schema");
    let store = SurrealProcessLedgerStore::open(storage.clone())
        .await
        .expect("open ProcessLedger before restart");
    let scope = exact_scope();
    let artifact = "5".repeat(64);
    let start = start_event(&scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256(artifact.clone())
        .with_sandbox_adapter_id("sandbox-adapter-restart")
        .with_runtime_owner(test_runtime_owner());
    let mut stop = ProcessStop::from_start(&start, Some(0));
    stop.stopped_at = start.started_at + ChronoDuration::seconds(1);
    store
        .write_batch(vec![
            LedgerEvent::Start(start.clone()),
            LedgerEvent::Stop(stop),
        ])
        .await
        .expect("persist lifecycle before restart");

    drop(store);
    drop(storage);
    allocator
        .close_for_reopen()
        .await
        .expect("close exact embedded namespace for reopen");
    allocator
        .reopen()
        .await
        .expect("reopen exact embedded namespace");
    assert_eq!(allocator.namespace(), namespace);
    assert_eq!(allocator.database(), database);
    let reopened_storage = allocator
        .activate_storage()
        .await
        .expect("reactivate cloned SurrealStorage on same namespace");
    let reopened_store = SurrealProcessLedgerStore::open(reopened_storage.clone())
        .await
        .expect("open ProcessLedger after restart");
    let by_process = reopened_store
        .inspect_ownership_by_process_uuid(&scope, start.process_uuid)
        .await
        .expect("inspect ownership after restart")
        .expect("reopened ownership exists");
    assert_eq!(by_process.process_uuid, start.process_uuid);
    assert!(by_process.stopped_at.is_some());
    assert_eq!(by_process.engine_kind, ProcessEngineKind::HelperSubprocess);
    assert_eq!(by_process.owner_role, "kernel_builder");
    assert_eq!(by_process.owner_wp.as_deref(), Some("WP-KERNEL-004"));
    assert_eq!(
        by_process.sandbox_adapter_id.as_deref(),
        Some("sandbox-adapter-restart")
    );
    let by_artifact = reopened_store
        .inspect_latest_ownership_by_artifact(&scope, &artifact)
        .await
        .expect("inspect artifact ownership after restart")
        .expect("reopened artifact ownership exists");
    assert_eq!(by_artifact, by_process);

    drop(reopened_store);
    drop(reopened_storage);
    allocator
        .shutdown_storage_for_reopen()
        .await
        .expect("shutdown reopened injected SurrealStorage");
    let diagnostics = allocator
        .cleanup()
        .await
        .expect("clean durable embedded Surreal scope");
    assert!(diagnostics.database_absent);
    assert!(diagnostics.namespace_absent_after_reopen);
    assert!(diagnostics.error.is_none());
}

/// MT-019 restart continuity: a crash-left reclaim claim (`kill_in_progress`)
/// must survive a real embedded-store shutdown and reopen of the SAME
/// namespace/database with its fence intact, remain recoverable through the
/// bounded in-progress sweep, and stay exact-scope gated after the restart.
#[tokio::test]
async fn reclaim_claim_state_and_fences_survive_same_namespace_shutdown_and_reopen() {
    let mut allocator = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate durable embedded Surreal scope");
    let namespace = allocator.namespace().to_owned();
    let database = allocator.database().to_owned();
    let storage = allocator
        .activate_storage()
        .await
        .expect("activate initial injected SurrealStorage");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap shared Surreal schema");
    let store = SurrealProcessLedgerStore::open(storage.clone())
        .await
        .expect("open ProcessLedger before restart");
    let scope = exact_scope();
    let dead_owner = test_runtime_owner();
    let session_id = "SR-PROCESS-LEDGER-TEST";

    let mut target = start_event(&scope, "kernel_builder", "WP-KERNEL-004")
        .with_sandbox_adapter_id("sandbox-adapter-reopen")
        .with_runtime_owner(dead_owner.clone());
    let target_internal = target.process_uuid.to_string();
    target = target.with_sandbox_internal_id(target_internal);
    let mut sibling = start_event(&scope, "kernel_builder", "WP-KERNEL-004")
        .with_sandbox_adapter_id("sandbox-adapter-reopen")
        .with_runtime_owner(dead_owner.clone());
    let sibling_internal = sibling.process_uuid.to_string();
    sibling = sibling.with_sandbox_internal_id(sibling_internal);
    assert_eq!(target.parent_session_id.as_deref(), Some(session_id));
    store
        .write_batch(vec![
            LedgerEvent::Start(target.clone()),
            LedgerEvent::Start(sibling.clone()),
        ])
        .await
        .expect("persist claimable session rows");

    // Claim exactly the target through the production single-row claim path and
    // advance it to the crash-left `kill_in_progress` phase.
    let claimed = store
        .active_process_for_session(&scope, session_id, target.process_uuid)
        .await
        .expect("single-row claim")
        .expect("target row is claimable");
    assert_eq!(claimed.process_uuid, target.process_uuid);
    let claim = claimed.reclaim_claim.clone();
    assert_eq!(claim.generation, 1);
    assert_eq!(claim.resource_scope, scope);
    store
        .mark_reclaim_kill_started(target.process_uuid, &claim)
        .await
        .expect("mark kill started");
    let before = reclaim_probe(&storage, &scope, target.process_uuid)
        .await
        .expect("claimed row exists before restart");
    assert_eq!(
        before.stop_reason.as_deref(),
        Some("reclaim_kill_in_progress")
    );
    assert_eq!(before.reclaim_state.as_deref(), Some("kill_in_progress"));
    assert_eq!(before.reclaim_claimant_uuid, Some(claim.claimant_uuid));
    assert_eq!(
        before.reclaim_kill_operation_uuid,
        Some(claim.kill_operation_uuid)
    );
    assert_eq!(before.reclaim_generation, Some(1));
    assert!(before.stopped_at.is_none());
    let sibling_before = reclaim_probe(&storage, &scope, sibling.process_uuid)
        .await
        .expect("sibling row exists before restart");
    assert!(sibling_before.stop_reason.is_none());
    assert!(sibling_before.reclaim_claimant_uuid.is_none());

    // Real storage shutdown, then reopen the SAME namespace/database.
    drop(store);
    drop(storage);
    allocator
        .close_for_reopen()
        .await
        .expect("close exact embedded namespace for reopen");
    allocator
        .reopen()
        .await
        .expect("reopen exact embedded namespace");
    assert_eq!(allocator.namespace(), namespace);
    assert_eq!(allocator.database(), database);
    let reopened_storage = allocator
        .activate_storage()
        .await
        .expect("reactivate cloned SurrealStorage on same namespace");
    let reopened_store = SurrealProcessLedgerStore::open(reopened_storage.clone())
        .await
        .expect("open ProcessLedger after restart");

    // Ledger rows and reclaim state are byte-identical after the restart.
    let after = reclaim_probe(&reopened_storage, &scope, target.process_uuid)
        .await
        .expect("claimed row survives restart");
    assert_eq!(after, before, "reclaim claim state must survive shutdown/reopen unchanged");
    let sibling_after = reclaim_probe(&reopened_storage, &scope, sibling.process_uuid)
        .await
        .expect("sibling row survives restart");
    assert_eq!(sibling_after, sibling_before);

    // Ownership fences survive: a stale claimant, a stale generation, and a
    // one-field-foreign scope can neither release nor renew the durable claim.
    let mut stale_claimant = claim.clone();
    stale_claimant.claimant_uuid = Uuid::now_v7();
    let error = reopened_store
        .release_reclaim_claim(target.process_uuid, &stale_claimant)
        .await
        .expect_err("stale claimant cannot release after reopen");
    assert!(error
        .to_string()
        .contains("failed to release open reclaim claim"));
    let mut stale_generation = claim.clone();
    stale_generation.generation = 2;
    let error = reopened_store
        .renew_reclaim_claim(target.process_uuid, &stale_generation)
        .await
        .expect_err("stale generation cannot renew after reopen");
    assert!(error
        .to_string()
        .contains("reclaim claim ownership lost while renewing"));
    for mismatched in five_mismatched_scopes(&scope) {
        let mut foreign = claim.clone();
        foreign.resource_scope = mismatched;
        reopened_store
            .release_reclaim_claim(target.process_uuid, &foreign)
            .await
            .expect_err("one-field scope mismatch cannot release after reopen");
    }
    assert_eq!(
        reopened_store
            .in_progress_kill_operations_for_session(
                &scope,
                session_id,
                Uuid::now_v7(),
                &[target.process_uuid],
                RECLAIM_IN_PROGRESS_RECOVERY_LIMIT,
            )
            .await
            .expect("drifted authorized set recovers nothing")
            .len(),
        0,
        "a process set that omits the open sibling must not recover the claim"
    );
    assert_eq!(
        reclaim_probe(&reopened_storage, &scope, target.process_uuid)
            .await
            .expect("row still present"),
        before,
        "fenced-out callers must not mutate the durable claim"
    );

    // The original claimant still owns the claim and can renew it after the
    // restart; while that lease is live, `kill_in_progress` is never lease-taken
    // over by a competing claimant.
    let renewed = reopened_store
        .renew_reclaim_claim(target.process_uuid, &claim)
        .await
        .expect("original claimant renews after reopen");
    assert_eq!(renewed.claimant_uuid, claim.claimant_uuid);
    assert_eq!(renewed.kill_operation_uuid, claim.kill_operation_uuid);
    assert_eq!(renewed.generation, 1);
    assert!(
        renewed.lease_expires_at_unix_ms >= before.reclaim_lease_expires_at_unix_ms.unwrap_or(0)
    );
    assert!(reopened_store
        .active_process_for_session(&scope, session_id, target.process_uuid)
        .await
        .expect("competing claim query")
        .is_none());

    // The crash-left kill operation is recoverable through the bounded sweep on
    // the reopened store, exact-scope gated, and only for the full open set.
    let self_instance = Uuid::now_v7();
    let mut authorized = vec![target.process_uuid, sibling.process_uuid];
    authorized.sort_unstable();
    let recoverable = reopened_store
        .in_progress_kill_operations_for_session(
            &scope,
            session_id,
            self_instance,
            &authorized,
            RECLAIM_IN_PROGRESS_RECOVERY_LIMIT,
        )
        .await
        .expect("recovery sweep after reopen");
    assert_eq!(recoverable.len(), 1, "exactly one crash-left operation: {recoverable:?}");
    match &recoverable[0] {
        ReclaimKillOperationCandidate::Operation { operation } => {
            assert_eq!(operation.process_uuid, target.process_uuid);
            assert_eq!(operation.kill_operation_uuid, claim.kill_operation_uuid);
            assert_eq!(operation.resource_scope, scope);
        }
        other => panic!("recovered candidate must be well formed: {other:?}"),
    }
    for mismatched in five_mismatched_scopes(&scope) {
        assert!(reopened_store
            .in_progress_kill_operations_for_session(
                &mismatched,
                session_id,
                self_instance,
                &authorized,
                RECLAIM_IN_PROGRESS_RECOVERY_LIMIT,
            )
            .await
            .expect("mismatched-scope recovery query")
            .is_empty());
    }

    // Read-only ownership inspection still verifies the OPEN claimed row against
    // its canonical START receipt after the restart.
    let inspection = reopened_store
        .inspect_ownership_by_process_uuid(&scope, target.process_uuid)
        .await
        .expect("inspect claimed row after restart")
        .expect("claimed row is inspectable");
    assert!(inspection.stopped_at.is_none());
    assert_eq!(inspection.lifecycle_state, LedgerEventKind::Start);
    assert_eq!(inspection.runtime_owner.as_ref(), Some(&dead_owner));
    assert_eq!(inspection.resource_scope, scope);

    // Resolving the crash-left operation as `not_started` releases the claim
    // without a STOP; the next claim advances the durable generation to 2.
    reopened_store
        .resolve_reclaim_kill_operation(
            &scope,
            target.process_uuid,
            claim.kill_operation_uuid,
            ReclaimKillOperationStatus::NotStarted,
        )
        .await
        .expect("resolve crash-left operation after reopen");
    let released = reclaim_probe(&reopened_storage, &scope, target.process_uuid)
        .await
        .expect("released row exists");
    assert!(released.stop_reason.is_none());
    assert!(released.reclaim_state.is_none());
    assert!(released.reclaim_claimant_uuid.is_none());
    assert!(released.stopped_at.is_none(), "release never fabricates a STOP");
    let reclaimed_again = reopened_store
        .active_process_for_session(&scope, session_id, target.process_uuid)
        .await
        .expect("claim after release")
        .expect("released row is claimable again");
    assert_eq!(reclaimed_again.reclaim_claim.generation, 2);
    assert_ne!(reclaimed_again.reclaim_claim.claimant_uuid, claim.claimant_uuid);
    reopened_store
        .release_reclaim_claim(target.process_uuid, &reclaimed_again.reclaim_claim)
        .await
        .expect("release generation-2 claim");

    drop(reopened_store);
    drop(reopened_storage);
    allocator
        .shutdown_storage_for_reopen()
        .await
        .expect("shutdown reopened injected SurrealStorage");
    let diagnostics = allocator
        .cleanup()
        .await
        .expect("clean durable embedded Surreal scope");
    assert!(diagnostics.database_absent);
    assert!(diagnostics.namespace_absent_after_reopen);
    assert!(diagnostics.error.is_none());
}

/// MT-019 orphan-receipt counterfactual: a canonical kernel EventLedger receipt
/// that exists WITHOUT its lifecycle projection must make the identical START
/// retry, the matching STOP, and any batch containing the retry fail closed
/// atomically, while the writer records the denial observably.
#[tokio::test]
async fn orphan_receipt_without_lifecycle_rejects_identical_retries_atomically_and_records_denial(
) {
    let fixture = SurrealFixture::open().await;
    let overflow = InMemoryOverflowSink::default();

    let orphan = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256("a".repeat(64));
    fixture
        .store
        .write_batch(vec![LedgerEvent::Start(orphan.clone())])
        .await
        .expect("persist orphan-receipt fixture");
    fixture
        .store
        .test_delete_inspection_lifecycle_projection(&fixture.scope, orphan.process_uuid)
        .await
        .expect("delete lifecycle while preserving the canonical receipt");
    assert!(fixture
        .lifecycle(orphan.process_uuid, &fixture.scope)
        .await
        .is_none());
    let receipt_before = fixture
        .event(orphan.process_uuid, LedgerEventKind::Start, &fixture.scope)
        .await
        .expect("canonical START receipt survives without its lifecycle");
    let events_before = fixture.process_event_count(&fixture.scope).await;
    assert_eq!(events_before, 1);

    // Identical START retry: denied before commit, typed marker names the row.
    let error = fixture
        .store
        .write_batch(vec![LedgerEvent::Start(orphan.clone())])
        .await
        .expect_err("receipt without lifecycle rejects identical START retry");
    assert!(matches!(error, ProcessLedgerError::Store(_)));
    assert!(error.to_string().contains(&format!(
        "PROCESS_LEDGER_VERIFICATION_MISMATCH:0:{}:START",
        orphan.process_uuid
    )));
    assert!(fixture
        .lifecycle(orphan.process_uuid, &fixture.scope)
        .await
        .is_none());
    let receipt_after = fixture
        .event(orphan.process_uuid, LedgerEventKind::Start, &fixture.scope)
        .await
        .expect("receipt is neither duplicated nor removed");
    assert_eq!(receipt_after.created_at, receipt_before.created_at);
    assert_eq!(fixture.process_event_count(&fixture.scope).await, events_before);

    // STOP against the orphan receipt: identity conflict, no STOP receipt.
    let stop = ProcessStop::from_start(&orphan, Some(0));
    let error = fixture
        .store
        .write_batch(vec![LedgerEvent::Stop(stop)])
        .await
        .expect_err("STOP without a lifecycle fails closed");
    assert!(matches!(
        error,
        ProcessLedgerError::StopIdentityConflict { process_uuid, .. }
            if process_uuid == orphan.process_uuid
    ));
    assert!(fixture
        .event(orphan.process_uuid, LedgerEventKind::Stop, &fixture.scope)
        .await
        .is_none());
    assert_eq!(fixture.process_event_count(&fixture.scope).await, events_before);

    // The denial is atomic across a batch: a healthy sibling in the same batch
    // is rolled back together with the rejected retry.
    let sibling = start_event(&fixture.scope, "kernel_builder", "WP-KERNEL-004")
        .with_model_artifact_sha256("b".repeat(64));
    let error = fixture
        .store
        .write_batch(vec![
            LedgerEvent::Start(sibling.clone()),
            LedgerEvent::Start(orphan.clone()),
        ])
        .await
        .expect_err("batch containing the orphan retry fails closed as a whole");
    assert!(error.to_string().contains(&format!(
        "PROCESS_LEDGER_VERIFICATION_MISMATCH:1:{}:START",
        orphan.process_uuid
    )));
    assert!(fixture
        .lifecycle(sibling.process_uuid, &fixture.scope)
        .await
        .is_none());
    assert!(fixture
        .event(sibling.process_uuid, LedgerEventKind::Start, &fixture.scope)
        .await
        .is_none());
    assert_eq!(fixture.process_event_count(&fixture.scope).await, events_before);

    // The production writer records the denial observably instead of dropping
    // it: the flush error propagates, failed rows are counted, the writer is
    // marked degraded, and nothing is misreported as an overflow drop.
    let (writer, drain) = ProcessLedgerWriter::new_manual(8, Arc::new(overflow.clone()))
        .expect("manual writer over the injected Surreal store");
    writer
        .append_start(orphan.clone())
        .expect("queue orphan retry");
    let error = drain
        .drain_available_to(fixture.store.clone())
        .await
        .expect_err("manual drain propagates the denial");
    assert!(error
        .to_string()
        .contains("PROCESS_LEDGER_VERIFICATION_MISMATCH:0:"));
    assert_eq!(drain.flush_failed_rows(), 1);
    assert!(writer.is_degraded());
    assert!(overflow.events().is_empty(), "a denial is not an overflow drop");
    assert_eq!(fixture.process_event_count(&fixture.scope).await, events_before);

    // Read-only inspection returns no partial data for the orphan receipt.
    assert!(fixture
        .store
        .inspect_ownership_by_process_uuid(&fixture.scope, orphan.process_uuid)
        .await
        .expect("inspection of an orphan receipt does not error")
        .is_none());

    drop(writer);
    fixture.close().await;
}

#[tokio::test]
async fn saturation_emits_overflow_receipts_and_clears_degraded_after_drain() {
    let store = InMemoryProcessLedgerStore::default();
    let overflow = InMemoryOverflowSink::default();
    let (writer, drain) =
        ProcessLedgerWriter::new_manual(128, Arc::new(overflow.clone())).expect("manual writer");

    let mut worst_append = Duration::ZERO;
    for index in 0..10_000 {
        let start = ProcessStart::new(
            ProcessEngineKind::MechanicalJob,
            format!("owner-{index}"),
            Some("WP-KERNEL-004".to_string()),
        );
        let started = Instant::now();
        writer
            .append_start(start)
            .expect("append never blocks on store");
        worst_append = worst_append.max(started.elapsed());
    }

    assert!(
        worst_append < Duration::from_millis(10),
        "append path waited too long: {worst_append:?}"
    );
    assert!(writer.is_degraded());
    assert!(is_degraded());

    let overflow_events = overflow.events();
    assert_eq!(overflow_events.len(), 10_000 - 128);
    assert_eq!(
        overflow_events.last().unwrap().overflow_count,
        (10_000 - 128) as u64
    );
    assert_eq!(
        overflow_events.last().unwrap().dropped_event_kind,
        LedgerEventKind::Start
    );

    drain
        .drain_available_to(Arc::new(store.clone()))
        .await
        .unwrap();
    assert_eq!(store.events().len(), 128);
    assert!(!writer.is_degraded());
    assert!(!is_degraded());
}

#[test]
fn overflow_payload_converts_to_typed_kernel_event() {
    let start = start_event(&exact_scope(), "kernel_builder", "WP-KERNEL-004");
    let overflow = LedgerOverflowEvent::new(1, 128, LedgerEvent::Start(start.clone()));
    let event = overflow
        .to_kernel_event()
        .expect("overflow event should convert to EventLedger row");

    assert_eq!(event.event_type, KernelEventType::FrEvtLedgerOverflow);
    assert_eq!(event.payload["event_type"], "FR_EVT_LEDGER_OVERFLOW");
    assert_eq!(event.payload["overflow_count"], 1);
    assert_eq!(event.payload["capacity"], 128);
    assert_eq!(event.payload["dropped_event_kind"], "START");
    assert_eq!(
        event.payload["sampled_event_payload"]["process_uuid"],
        start.process_uuid.to_string()
    );
}

#[test]
fn active_process_ledger_sources_are_embedded_surreal_only() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let forbidden = [
        ["post", "gres"].concat(),
        ["sql", "ite"].concat(),
        ["sql", "x"].concat(),
        ["pg", "pool"].concat(),
    ];
    for relative_path in PROCESS_LEDGER_SOURCE_FILES {
        let path = std::path::Path::new(manifest_dir).join(relative_path);
        let source = std::fs::read_to_string(&path).expect("read process ledger source");
        let lower = source.to_ascii_lowercase();
        for token in &forbidden {
            assert!(
                !lower.contains(token),
                "{} contains retired storage token {token}",
                path.display()
            );
        }
    }
}

fn start_event(scope: &ReclaimResourceScope, owner_role: &str, owner_wp: &str) -> ProcessStart {
    ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        owner_role.to_string(),
        Some(owner_wp.to_string()),
    )
    .with_parent_session_id("SR-PROCESS-LEDGER-TEST")
    .with_work_profile_id("work-profile-test")
    .with_metadata_jsonb(scope_metadata(scope))
}

fn start_event_with_uuid(
    scope: &ReclaimResourceScope,
    process_uuid: Uuid,
    os_pid: u32,
) -> ProcessStart {
    start_event(scope, "kernel_builder", "WP-KERNEL-004")
        .with_process_uuid(process_uuid)
        .with_os_pid(os_pid)
}

fn exact_scope() -> ReclaimResourceScope {
    ReclaimResourceScope {
        account_uuid: Uuid::now_v7(),
        actor_uuid: Uuid::now_v7(),
        session_uuid: Uuid::now_v7(),
        workspace_id: format!("workspace-{}", Uuid::now_v7()),
        access_space_uuid: Uuid::now_v7(),
    }
}

fn scope_metadata(scope: &ReclaimResourceScope) -> Value {
    json!({
        "owner_account_id": scope.account_uuid.to_string(),
        "actor_principal_id": scope.actor_uuid.to_string(),
        "authenticated_session_id": scope.session_uuid.to_string(),
        "access_space_id": scope.access_space_uuid.to_string(),
        "workspace_id": scope.workspace_id.clone(),
    })
}

fn test_runtime_owner() -> ProcessRuntimeOwner {
    ProcessRuntimeOwner {
        runtime_instance_id: Uuid::now_v7(),
        host_scope_id: format!("host-{}", Uuid::now_v7()),
        lease_schema_id: EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID.to_owned(),
        lease_protocol: EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL.to_owned(),
        lease_address: "127.0.0.1".to_owned(),
        lease_port: 49_019,
    }
}

fn five_mismatched_scopes(scope: &ReclaimResourceScope) -> [ReclaimResourceScope; 5] {
    let mut account = scope.clone();
    account.account_uuid = Uuid::now_v7();
    let mut actor = scope.clone();
    actor.actor_uuid = Uuid::now_v7();
    let mut session = scope.clone();
    session.session_uuid = Uuid::now_v7();
    let mut access_space = scope.clone();
    access_space.access_space_uuid = Uuid::now_v7();
    let mut workspace = scope.clone();
    workspace.workspace_id = format!("workspace-{}", Uuid::now_v7());
    [account, actor, session, access_space, workspace]
}

struct SurrealFixture {
    allocator: EmbeddedSurrealTestScope,
    storage: SurrealStorage,
    store: Arc<SurrealProcessLedgerStore>,
    scope: ReclaimResourceScope,
}

impl SurrealFixture {
    async fn open() -> Self {
        let mut allocator = EmbeddedSurrealTestScope::create()
            .await
            .expect("allocate isolated embedded Surreal scope");
        let storage = allocator
            .activate_storage()
            .await
            .expect("activate injected SurrealStorage");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap shared Surreal schema");
        let store = Arc::new(
            SurrealProcessLedgerStore::open(storage.clone())
                .await
                .expect("open ProcessLedger on injected SurrealStorage"),
        );
        Self {
            allocator,
            storage,
            store,
            scope: exact_scope(),
        }
    }

    async fn lifecycle(
        &self,
        process_uuid: Uuid,
        scope: &ReclaimResourceScope,
    ) -> Option<LifecycleProbe> {
        let bindings = ExactRecordBindings::new(
            scope,
            RecordId::new(PROCESS_LEDGER_TABLE_NAME, process_uuid.to_string()),
        );
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<LifecycleProbe, _>(READ_EXACT_LIFECYCLE, bindings)
                        .await
                })
            })
            .await
            .expect("read exact-scope lifecycle")
    }

    async fn event(
        &self,
        process_uuid: Uuid,
        kind: LedgerEventKind,
        scope: &ReclaimResourceScope,
    ) -> Option<EventProbe> {
        let bindings = ExactRecordBindings::new(
            scope,
            RecordId::new(
                "kernel_event_ledger",
                format!(
                    "process-lifecycle-{process_uuid}-{}",
                    kind.as_str().to_ascii_lowercase()
                ),
            ),
        );
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<EventProbe, _>(READ_EXACT_EVENT, bindings)
                        .await
                })
            })
            .await
            .expect("read exact-scope EventLedger row")
    }

    async fn tamper_metadata(&self, process_uuid: Uuid, scope: &ReclaimResourceScope) -> i64 {
        let bindings = ExactRecordBindings::new(
            scope,
            RecordId::new(PROCESS_LEDGER_TABLE_NAME, process_uuid.to_string()),
        );
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .execute_returning(TAMPER_EXACT_LIFECYCLE, bindings)
                        .await
                })
            })
            .await
            .expect("tamper exact-scope lifecycle fixture") as i64
    }

    async fn lifecycle_count(&self, process_uuid: Uuid, scope: &ReclaimResourceScope) -> i64 {
        let bindings = ProcessCountBindings::new(scope, process_uuid);
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<i64, _>(COUNT_EXACT_PROCESS_IDENTITIES, bindings)
                        .await
                })
            })
            .await
            .expect("count exact-scope process identities")
            .expect("count query returns one value")
    }

    /// Number of canonical `process_ledger` EventLedger receipts in one exact scope.
    async fn process_event_count(&self, scope: &ReclaimResourceScope) -> i64 {
        let bindings = ScopeOnlyBindings::new(scope);
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<i64, _>(COUNT_EXACT_PROCESS_EVENTS, bindings)
                        .await
                })
            })
            .await
            .expect("count exact-scope process receipts")
            .expect("receipt count query returns one value")
    }

    async fn close(mut self) {
        drop(self.store);
        drop(self.storage);
        self.allocator
            .shutdown_storage_for_reopen()
            .await
            .expect("shutdown injected SurrealStorage");
        let diagnostics = self
            .allocator
            .cleanup()
            .await
            .expect("clean isolated embedded Surreal scope");
        assert!(diagnostics.database_absent);
        assert!(diagnostics.namespace_absent_after_reopen);
        assert!(diagnostics.error.is_none());
    }
}

#[derive(Debug, SurrealValue)]
struct ExactRecordBindings {
    record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl ExactRecordBindings {
    fn new(scope: &ReclaimResourceScope, record: RecordId) -> Self {
        Self {
            record,
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[derive(Debug, SurrealValue)]
struct ProcessCountBindings {
    process_uuid: Uuid,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl ProcessCountBindings {
    fn new(scope: &ReclaimResourceScope, process_uuid: Uuid) -> Self {
        Self {
            process_uuid,
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

#[derive(Debug, SurrealValue)]
struct LifecycleProbe {
    process_uuid: Uuid,
    engine_kind: String,
    stopped_at: Option<DateTime<Utc>>,
    exit_code: Option<i64>,
    event_ledger_event_id: Option<RecordId>,
    metadata: Value,
}

#[derive(Debug, SurrealValue)]
struct EventProbe {
    event_type: String,
    created_at: DateTime<Utc>,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl EventProbe {
    fn exact_scope(&self) -> [String; 5] {
        [
            self.owner_account_id.clone(),
            self.actor_principal_id.clone(),
            self.authenticated_session_id.clone(),
            self.access_space_id.clone(),
            self.workspace_id.clone(),
        ]
    }
}

#[derive(Debug, SurrealValue)]
struct ScopeOnlyBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl ScopeOnlyBindings {
    fn new(scope: &ReclaimResourceScope) -> Self {
        Self {
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }
}

/// Durable reclaim-fence projection of one exact-scope lifecycle row.
#[derive(Debug, Clone, PartialEq, SurrealValue)]
struct ReclaimProbe {
    stopped_at: Option<DateTime<Utc>>,
    stop_reason: Option<String>,
    reclaim_state: Option<String>,
    reclaim_claimant_uuid: Option<Uuid>,
    reclaim_kill_operation_uuid: Option<Uuid>,
    reclaim_generation: Option<i64>,
    reclaim_claimed_at_unix_ms: Option<i64>,
    reclaim_lease_expires_at_unix_ms: Option<i64>,
    metadata: Value,
}

async fn reclaim_probe(
    storage: &SurrealStorage,
    scope: &ReclaimResourceScope,
    process_uuid: Uuid,
) -> Option<ReclaimProbe> {
    let bindings = ExactRecordBindings::new(
        scope,
        RecordId::new(PROCESS_LEDGER_TABLE_NAME, process_uuid.to_string()),
    );
    storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_first::<ReclaimProbe, _>(READ_EXACT_RECLAIM_STATE, bindings)
                    .await
            })
        })
        .await
        .expect("read exact-scope reclaim state")
}

const READ_EXACT_RECLAIM_STATE: &str = r#"
SELECT stopped_at, stop_reason, reclaim_state, reclaim_claimant_uuid,
    reclaim_kill_operation_uuid, reclaim_generation, reclaim_claimed_at_unix_ms,
    reclaim_lease_expires_at_unix_ms, metadata
FROM ONLY $record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

const COUNT_EXACT_PROCESS_EVENTS: &str = r#"
RETURN array::len(SELECT VALUE id FROM kernel_event_ledger
WHERE source_component = 'process_ledger'
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND payload.metadata_jsonb.owner_account_id = $owner_account_id
    AND payload.metadata_jsonb.actor_principal_id = $actor_principal_id
    AND payload.metadata_jsonb.authenticated_session_id = $authenticated_session_id
    AND payload.metadata_jsonb.access_space_id = $access_space_id
    AND payload.metadata_jsonb.workspace_id = $workspace_id);
"#;

const READ_EXACT_LIFECYCLE: &str = r#"
SELECT process_uuid, engine_kind, stopped_at, exit_code, event_ledger_event_id, metadata
FROM ONLY $record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

const COUNT_EXACT_PROCESS_IDENTITIES: &str = r#"
SELECT VALUE count() FROM kernel_process_lifecycle
WHERE process_uuid = $process_uuid
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
GROUP ALL;
"#;

const TAMPER_EXACT_LIFECYCLE: &str = r#"
UPDATE $record SET metadata.verification_tamper = true
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
RETURN AFTER;
"#;

const READ_EXACT_EVENT: &str = r#"
SELECT event_type, created_at, owner_account_id, actor_principal_id,
    authenticated_session_id, access_space_id, workspace_id
FROM ONLY $record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
    AND payload.metadata_jsonb.owner_account_id = $owner_account_id
    AND payload.metadata_jsonb.actor_principal_id = $actor_principal_id
    AND payload.metadata_jsonb.authenticated_session_id = $authenticated_session_id
    AND payload.metadata_jsonb.access_space_id = $access_space_id
    AND payload.metadata_jsonb.workspace_id = $workspace_id;
"#;

#[derive(Clone, Default)]
struct InMemoryProcessLedgerStore {
    events: Arc<Mutex<Vec<LedgerEvent>>>,
}

impl InMemoryProcessLedgerStore {
    fn events(&self) -> Vec<LedgerEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl ProcessLedgerStore for InMemoryProcessLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.events.lock().unwrap().extend(events);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct InMemoryOverflowSink {
    events: Arc<Mutex<Vec<LedgerOverflowEvent>>>,
}

impl InMemoryOverflowSink {
    fn events(&self) -> Vec<LedgerOverflowEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl ProcessLedgerOverflowSink for InMemoryOverflowSink {
    fn emit_overflow(&self, event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}
