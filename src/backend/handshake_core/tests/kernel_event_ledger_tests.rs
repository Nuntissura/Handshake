use handshake_core::kernel::{
    assert_kernel_authority_storage_mode, KernelActor, KernelEventType, NewKernelEvent, SessionRun,
    SessionRunState,
};
use handshake_core::storage::{
    sqlite::SqliteDatabase, ControlPlaneStorageMode, Database, StorageError,
};
use serde_json::json;
use std::{fs, path::Path};

#[test]
fn kernel_event_taxonomy_covers_first_slice_families() {
    let event_names: Vec<&'static str> = KernelEventType::required_first_slice_events()
        .iter()
        .map(KernelEventType::as_str)
        .collect();

    for required in [
        "TASK_INTENT_RECORDED",
        "SESSION_QUEUED",
        "SESSION_CLAIMED",
        "CONTEXT_BUNDLE_RECORDED",
        "MODEL_RESPONSE_RECORDED",
        "TOOL_REQUEST_RECORDED",
        "TOOL_DECISION_RECORDED",
        "ARTIFACT_PROPOSED",
        "ARTIFACT_STORED",
        "VALIDATION_RECORDED",
        "PROMOTION_DECIDED",
        "SESSION_CANCELLED",
        "SESSION_BACKPRESSURE_DELAYED",
        "SESSION_DEAD_LETTERED",
        "TRACE_REPLAYED",
    ] {
        assert!(
            event_names.contains(&required),
            "missing required Kernel V1 event family {required}"
        );
    }
}

#[test]
fn kernel_events_preserve_run_causation_and_payload() {
    let event = NewKernelEvent::builder(
        "KTR-EXAMPLE",
        "SR-EXAMPLE",
        KernelEventType::ToolDecisionRecorded,
        KernelActor::System("toolgate".to_string()),
    )
    .causation_id("evt-tool-request")
    .correlation_id("corr-kernel-proof")
    .payload(json!({
        "tool_request_id": "tool-1",
        "decision": "allow",
        "reason": "deterministic proof tool"
    }))
    .build()
    .expect("valid kernel event");

    assert_eq!(event.kernel_task_run_id, "KTR-EXAMPLE");
    assert_eq!(event.session_run_id, "SR-EXAMPLE");
    assert_eq!(event.event_type, KernelEventType::ToolDecisionRecorded);
    assert_eq!(event.event_type.as_str(), "TOOL_DECISION_RECORDED");
    assert_eq!(event.aggregate_type, "session_run");
    assert_eq!(event.aggregate_id, "SR-EXAMPLE");
    assert_eq!(event.causation_id.as_deref(), Some("evt-tool-request"));
    assert_eq!(event.correlation_id.as_deref(), Some("corr-kernel-proof"));
    assert_eq!(event.source_component, "system");
    assert_eq!(event.payload_hash.len(), 64);
    assert_eq!(event.payload["decision"], "allow");
}

#[test]
fn no_sqlite_kernel_authority_rejects_sqlite_and_test_modes() {
    for mode in [
        ControlPlaneStorageMode::SqliteCache,
        ControlPlaneStorageMode::SqliteOffline,
        ControlPlaneStorageMode::Test,
    ] {
        let err = assert_kernel_authority_storage_mode(mode)
            .expect_err("Kernel V1 authority must reject non-Postgres modes");
        assert!(
            err.to_string().contains("Postgres"),
            "error should tell the caller to use Postgres authority: {err}"
        );
    }

    assert_kernel_authority_storage_mode(ControlPlaneStorageMode::PostgresPrimary)
        .expect("PostgresPrimary is the only Kernel V1 authority mode");
}

#[tokio::test]
async fn no_sqlite_kernel_authority_sqlite_kernel_event_ledger_fails_closed() {
    let sqlite = SqliteDatabase::connect("sqlite::memory:", 5)
        .await
        .expect("sqlite backend");
    sqlite.run_migrations().await.expect("sqlite migrations");

    let event = NewKernelEvent::builder(
        "KTR-SQLITE-BLOCKED",
        "SR-SQLITE-BLOCKED",
        KernelEventType::SessionQueued,
        KernelActor::SessionBroker("broker-test".to_string()),
    )
    .payload(json!({"reason": "sqlite must not store kernel events"}))
    .build()
    .expect("valid kernel event");

    let append_err = sqlite
        .append_kernel_event(event)
        .await
        .expect_err("SQLite must reject Kernel V1 authority writes");
    assert_kernel_v1_requires_postgres(append_err);

    let list_err = sqlite
        .list_kernel_events_for_session("SR-SQLITE-BLOCKED")
        .await
        .expect_err("SQLite must reject Kernel V1 authority reads");
    assert_kernel_v1_requires_postgres(list_err);

    let enqueue_err = sqlite
        .enqueue_kernel_session_run(SessionRun::queued("KTR-SQLITE-BLOCKED", "dummy-echo"))
        .await
        .expect_err("SQLite must reject Kernel V1 session queue writes");
    assert_kernel_v1_requires_postgres(enqueue_err);

    let atomic_enqueue_err = sqlite
        .enqueue_kernel_session_run_and_record_event(
            SessionRun::queued("KTR-SQLITE-BLOCKED", "dummy-echo"),
            Some("KE-cause".to_string()),
            "corr-sqlite-blocked".to_string(),
        )
        .await
        .expect_err("SQLite must reject atomic Kernel V1 session queue authority writes");
    assert_kernel_v1_requires_postgres(atomic_enqueue_err);

    let claim_err = sqlite
        .claim_kernel_session_run("SR-SQLITE-BLOCKED", "worker", 30)
        .await
        .expect_err("SQLite must reject Kernel V1 claim leases");
    assert_kernel_v1_requires_postgres(claim_err);

    let atomic_claim_err = sqlite
        .claim_kernel_session_run_and_record_event(
            "SR-SQLITE-BLOCKED",
            "worker",
            30,
            Some("KE-cause".to_string()),
            "corr-sqlite-blocked".to_string(),
        )
        .await
        .expect_err("SQLite must reject atomic Kernel V1 claim authority writes");
    assert_kernel_v1_requires_postgres(atomic_claim_err);

    let state_err = sqlite
        .update_kernel_session_run_state("SR-SQLITE-BLOCKED", SessionRunState::Running)
        .await
        .expect_err("SQLite must reject Kernel V1 session state writes");
    assert_kernel_v1_requires_postgres(state_err);

    let atomic_state_err = sqlite
        .update_kernel_session_run_state_and_record_event(
            "SR-SQLITE-BLOCKED",
            SessionRunState::Running,
            Some("KE-cause".to_string()),
            "corr-sqlite-blocked".to_string(),
        )
        .await
        .expect_err("SQLite must reject atomic Kernel V1 state authority writes");
    assert_kernel_v1_requires_postgres(atomic_state_err);
}

fn assert_kernel_v1_requires_postgres(err: StorageError) {
    let message = err.to_string();
    assert!(
        message.contains("Kernel V1 EventLedger")
            && message.contains("SessionBroker")
            && message.contains("trace replay")
            && message.contains("validation")
            && message.contains("promotion authority")
            && message.contains("Postgres"),
        "error should tell callers to use Postgres for Kernel V1 authority surfaces: {message}"
    );
}

#[test]
fn product_sqlite_leakage_tripwire() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let kernel_dir = manifest_dir.join("src").join("kernel");
    let mut checked_files = 0;

    for entry in fs::read_dir(&kernel_dir).expect("kernel source dir") {
        let path = entry.expect("kernel dir entry").path();
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read kernel source file");
        let normalized = source.to_ascii_lowercase();
        assert!(
            !normalized.contains("sqlite") && !normalized.contains("locus_sqlite"),
            "Kernel V1 authority code must not depend on SQLite or locus_sqlite: {}",
            path.display()
        );
        checked_files += 1;
    }

    assert!(checked_files > 0, "expected to check kernel source files");
}

#[test]
fn flight_recorder_kernel_mirror_exposes_required_debug_fields() {
    let stored = handshake_core::kernel::KernelEvent::from_new(
        NewKernelEvent::builder(
            "KTR-MIRROR",
            "SR-MIRROR",
            KernelEventType::ToolDecisionRecorded,
            KernelActor::ToolGate("kernel-toolgate".to_string()),
        )
        .idempotency_key("idem-mirror-debug-fields")
        .causation_id("KE-cause")
        .correlation_id("corr-mirror")
        .payload(json!({"decision": "allow"}))
        .build()
        .expect("valid event"),
    );

    let mirror = handshake_core::kernel::flight_recorder_mirror_event(&stored);

    assert_eq!(
        mirror.payload["causation_id"].as_str(),
        Some("KE-cause"),
        "mirror diagnostics must expose causation for no-context debugging"
    );
    assert_eq!(
        mirror.payload["correlation_id"].as_str(),
        Some("corr-mirror"),
        "mirror diagnostics must expose correlation for no-context debugging"
    );
    assert_eq!(
        mirror.payload["idempotency_key"].as_str(),
        Some("idem-mirror-debug-fields"),
        "mirror diagnostics must expose idempotency for duplicate/conflict triage"
    );
}
