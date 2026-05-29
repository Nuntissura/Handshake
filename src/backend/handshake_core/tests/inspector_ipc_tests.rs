use std::path::{Path, PathBuf};

use handshake_core::{
    inspector_read::{
        EventLedgerRow, InspectorReadSnapshot, InspectorReadV1, ModelLoadedRow, ProcessRow,
        SessionId, SessionStateRead, SessionSummary, WorkspaceId, WorkspaceStateRead,
    },
    model_manual::{model_manual, CommandStatus},
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::json;

const INSPECTOR_IPC_COMMANDS: &[(&str, &str)] = &[
    ("kernel_inspector_port", "kernel.inspector.port"),
    (
        "kernel_inspector_list_sessions",
        "kernel.inspector.list_sessions",
    ),
    (
        "kernel_inspector_session_state",
        "kernel.inspector.session_state",
    ),
    (
        "kernel_inspector_event_ledger_tail",
        "kernel.inspector.event_ledger_tail",
    ),
    (
        "kernel_inspector_process_ledger_active",
        "kernel.inspector.process_ledger_active",
    ),
    (
        "kernel_inspector_trace_projection",
        "kernel.inspector.trace_projection",
    ),
    (
        "kernel_inspector_loaded_models",
        "kernel.inspector.loaded_models",
    ),
];

#[test]
fn inspector_ipc_tests_tauri_bridge_file_registers_all_inspector_commands() {
    let repo = repo_root();
    let inspector_rs = read(repo.join("app/src-tauri/src/inspector.rs"));
    let lib_rs = read(repo.join("app/src-tauri/src/lib.rs"));
    let cargo_toml = read(repo.join("app/src-tauri/Cargo.toml"));

    assert!(cargo_toml.contains("handshake_core"));
    assert!(cargo_toml.contains("\"inspector\""));
    assert!(cargo_toml.contains("\"runtime-full\""));
    assert!(lib_rs.contains("mod inspector;"));
    assert!(lib_rs.contains("Arc<dyn handshake_core::inspector_read::InspectorReadV1>"));
    assert!(inspector_rs.contains("State<'_, Arc<dyn InspectorReadV1>>"));

    for (tauri_command, ipc_channel) in INSPECTOR_IPC_COMMANDS {
        assert!(
            inspector_rs.contains(&format!("pub fn {tauri_command}")),
            "missing Tauri command function {tauri_command}"
        );
        assert!(
            inspector_rs.contains(ipc_channel),
            "missing IPC channel constant {ipc_channel}"
        );
        assert!(
            lib_rs.contains(&format!("inspector::{tauri_command}")),
            "missing invoke_handler registration for {tauri_command}"
        );
    }
}

#[test]
fn inspector_ipc_tests_model_manual_lists_all_wired_ipc_commands() {
    let manual = model_manual();
    let hbr_group = manual
        .feature_groups
        .iter()
        .find(|group| group.id == "hbr_process_diagnostics")
        .expect("hbr/process diagnostics group");

    for (tauri_command, ipc_channel) in INSPECTOR_IPC_COMMANDS {
        assert!(
            hbr_group.commands.contains(tauri_command),
            "manual feature group missing {tauri_command}"
        );
        let command = manual
            .command_reference
            .iter()
            .find(|command| command.id == *tauri_command)
            .unwrap_or_else(|| panic!("manual command missing {tauri_command}"));
        assert_eq!(command.name, *tauri_command);
        assert_eq!(command.status, CommandStatus::Wired);
        assert_eq!(command.ipc_channel, Some(*ipc_channel));
        assert_eq!(command.tauri_command, Some(*tauri_command));
        assert!(
            command.expected_output.contains("InspectorReadV1"),
            "{tauri_command} must document the trait source of truth"
        );
    }
}

#[test]
fn inspector_ipc_tests_payload_shapes_round_trip_cleanly() {
    let snapshot = sample_reader();
    let session_id = SessionId::new("session-alpha");

    round_trip(snapshot.list_sessions());
    round_trip(snapshot.session_state(session_id.clone()));
    round_trip(snapshot.event_ledger_tail(10));
    round_trip(snapshot.process_ledger_active());
    round_trip(snapshot.workspace_state_read(WorkspaceId::new("workspace-alpha")));
    round_trip(snapshot.trace_projection(session_id));
    round_trip(snapshot.loaded_models());
}

fn sample_reader() -> InspectorReadSnapshot {
    let session_id = SessionId::new("session-alpha");
    let workspace_id = WorkspaceId::new("workspace-alpha");
    let mut snapshot = InspectorReadSnapshot::default();
    snapshot.sessions.push(SessionSummary {
        id: session_id.clone(),
        state: "running".to_string(),
        model_id: Some("local-model".to_string()),
        active_process_count: 1,
    });
    snapshot.session_states.insert(
        session_id.clone(),
        SessionStateRead {
            id: session_id.clone(),
            state: "running".to_string(),
            latest_event_id: Some("evt-004".to_string()),
            active_process_count: 1,
        },
    );
    snapshot.event_ledger_tail = vec![
        EventLedgerRow {
            event_id: "evt-001".to_string(),
            event_type: "TASK_OPEN".to_string(),
            event_sequence: 1,
            created_at_utc: "2026-05-18T13:00:00Z".to_string(),
            session_run_id: session_id.0.clone(),
            aggregate_id: session_id.0.clone(),
            payload: json!({
                "wp_id": "WP-KERNEL-004",
                "mt_id": "MT-032",
                "task_summary": "Bridge InspectorReadV1 to Tauri IPC."
            }),
            ..EventLedgerRow::default()
        },
        EventLedgerRow {
            event_id: "evt-002".to_string(),
            event_type: "MODEL_RESPONSE".to_string(),
            event_sequence: 2,
            created_at_utc: "2026-05-18T13:01:00Z".to_string(),
            session_run_id: session_id.0.clone(),
            aggregate_id: session_id.0.clone(),
            payload: json!({ "content": "ipc bridge payload" }),
            ..EventLedgerRow::default()
        },
    ];
    snapshot.processes.push(ProcessRow {
        process_uuid: "proc-alpha".to_string(),
        session_id: session_id.clone(),
        engine_kind: "tauri_ipc".to_string(),
        status: "running".to_string(),
    });
    snapshot.workspace_states.insert(
        workspace_id.clone(),
        WorkspaceStateRead {
            workspace_id,
            state_vector: "sv:alpha".to_string(),
            last_update_id: Some("update-alpha".to_string()),
            readable_refs: vec!["crdt://workspace-alpha/update-alpha".to_string()],
        },
    );
    snapshot.loaded_models.push(ModelLoadedRow {
        model_id: "local-model".to_string(),
        adapter_id: "test-adapter".to_string(),
        process_uuid: Some("proc-alpha".to_string()),
        loaded_at_utc: Some("2026-05-18T13:00:00Z".to_string()),
    });
    snapshot
}

fn round_trip<T>(value: T)
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let encoded = serde_json::to_value(&value).expect("serialize inspector IPC payload");
    let decoded: T = serde_json::from_value(encoded).expect("deserialize inspector IPC payload");
    assert_eq!(decoded, value);
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

fn read(path: PathBuf) -> String {
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    })
}
