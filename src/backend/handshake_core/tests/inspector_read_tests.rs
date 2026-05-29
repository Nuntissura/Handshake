use std::{fs, path::Path, sync::Arc};

use handshake_core::inspector_read::{
    validate_inspector_read_source_tree, EventLedgerRow, InspectorReadIsolationRule,
    InspectorReadSnapshot, InspectorReadV1, ModelLoadedRow, ProcessRow, SessionId,
    SessionStateRead, SessionSummary, WorkspaceId, WorkspaceStateRead,
};

#[test]
fn inspector_read_tests_trait_surface_has_no_mutation_receivers() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let trait_source =
        fs::read_to_string(manifest_dir.join("src/inspector_read/trait_def.rs")).unwrap();

    assert!(trait_source.contains("pub trait InspectorReadV1"));
    assert!(trait_source.contains("fn list_sessions(&self) -> Vec<SessionSummary>"));
    assert!(trait_source.contains("fn loaded_models(&self) -> Vec<ModelLoadedRow>"));
    assert!(
        !trait_source.contains("&mut self"),
        "InspectorReadV1 must remain read-only at the trait receiver boundary"
    );
    assert!(
        !trait_source.contains("self: &mut"),
        "InspectorReadV1 must not hide mutable receivers behind explicit self syntax"
    );
}

#[test]
fn inspector_read_tests_trait_is_object_safe_and_returns_owned_clones() {
    let mut snapshot = InspectorReadSnapshot::default();
    let session_id = SessionId::new("session-alpha");
    let workspace_id = WorkspaceId::new("workspace-alpha");
    snapshot.sessions.push(SessionSummary {
        id: session_id.clone(),
        state: "running".to_string(),
        model_id: Some("local-llama".to_string()),
        active_process_count: 1,
    });
    snapshot.session_states.insert(
        session_id.clone(),
        SessionStateRead {
            id: session_id.clone(),
            state: "running".to_string(),
            latest_event_id: Some("evt-1".to_string()),
            active_process_count: 1,
        },
    );
    snapshot.event_ledger_tail.push(EventLedgerRow {
        event_id: "evt-1".to_string(),
        event_type: "session_started".to_string(),
        event_sequence: 1,
        created_at_utc: "2026-05-18T08:00:00Z".to_string(),
        ..EventLedgerRow::default()
    });
    snapshot.processes.push(ProcessRow {
        process_uuid: "proc-1".to_string(),
        session_id: session_id.clone(),
        engine_kind: "webview2_cdp".to_string(),
        status: "running".to_string(),
    });
    snapshot.workspace_states.insert(
        workspace_id.clone(),
        WorkspaceStateRead {
            workspace_id: workspace_id.clone(),
            state_vector: "sv:1".to_string(),
            last_update_id: Some("update-1".to_string()),
            readable_refs: vec!["crdt://workspace-alpha/update-1".to_string()],
        },
    );
    snapshot.loaded_models.push(ModelLoadedRow {
        model_id: "local-llama".to_string(),
        adapter_id: "llama-cpp-placeholder".to_string(),
        process_uuid: Some("proc-1".to_string()),
        loaded_at_utc: Some("2026-05-18T08:00:00Z".to_string()),
    });

    let inspector: Arc<dyn InspectorReadV1 + Send + Sync> = Arc::new(snapshot);

    let mut sessions = inspector.list_sessions();
    sessions[0].state = "locally-mutated-clone".to_string();

    assert_eq!(inspector.list_sessions()[0].state, "running");
    assert_eq!(
        inspector
            .session_state(session_id)
            .expect("session state")
            .latest_event_id
            .as_deref(),
        Some("evt-1")
    );
    assert_eq!(inspector.event_ledger_tail(10)[0].event_sequence, 1);
    assert_eq!(inspector.process_ledger_active()[0].status, "running");
    assert_eq!(
        inspector
            .workspace_state_read(workspace_id)
            .expect("workspace state")
            .readable_refs[0],
        "crdt://workspace-alpha/update-1"
    );
    assert!(inspector
        .trace_projection(SessionId::new("missing"))
        .is_none());
    assert_eq!(
        inspector.loaded_models()[0].adapter_id,
        "llama-cpp-placeholder"
    );
}

#[test]
fn inspector_read_tests_isolation_rule_rejects_synthetic_write_side_imports() {
    let rule = InspectorReadIsolationRule::default();
    let error = rule
        .validate_source("use crate::kernel::write_boxes::WriteBoxQueueRow;\n")
        .expect_err("write-side import must fail closed");

    assert!(error.to_string().contains("crate::kernel::write_boxes"));
}

#[test]
fn inspector_read_tests_source_tree_and_cargo_metadata_enforce_boundary() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml = fs::read_to_string(manifest_dir.join("Cargo.toml")).unwrap();

    assert!(cargo_toml.contains("[package.metadata.handshake.inspector_read_isolation]"));
    assert!(cargo_toml.contains("deny_mutable_self_methods = true"));
    assert!(cargo_toml.contains("\"crate::kernel::write_boxes\""));

    validate_inspector_read_source_tree(manifest_dir).expect("inspector_read source isolation");
}
