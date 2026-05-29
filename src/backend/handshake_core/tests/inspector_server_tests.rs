#![cfg(feature = "inspector")]

use std::{net::SocketAddr, path::Path, sync::Arc};

use futures_util::StreamExt;
use serde_json::Value;

use handshake_core::inspector_read::{
    EventLedgerRow, InspectorReadSnapshot, InspectorServer, ModelLoadedRow, ProcessRow, SessionId,
    SessionStateRead, SessionSummary, WorkspaceId, WorkspaceStateRead,
};

#[tokio::test]
async fn inspector_server_tests_binds_random_localhost_and_serves_read_only_routes() {
    let handle = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");

    assert_eq!(handle.addr().ip().to_string(), "127.0.0.1");
    assert_ne!(handle.port(), 0);
    assert_eq!(handle.port_command_ref(), "kernel.inspector.port");

    let base = format!("http://{}", handle.addr());
    let client = reqwest::Client::new();

    let sessions: Vec<SessionSummary> = client
        .get(format!("{base}/inspector/v1/sessions"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(sessions[0].id, SessionId::new("session-alpha"));

    let session: SessionStateRead = client
        .get(format!("{base}/inspector/v1/sessions/session-alpha"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(session.latest_event_id.as_deref(), Some("evt-1"));

    let events: Vec<EventLedgerRow> = client
        .get(format!("{base}/inspector/v1/event-ledger/tail?n=1"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, "evt-2");

    let processes: Vec<ProcessRow> = client
        .get(format!("{base}/inspector/v1/process-ledger/active"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(processes[0].process_uuid, "proc-1");

    let workspace: WorkspaceStateRead = client
        .get(format!("{base}/inspector/v1/workspace/workspace-alpha"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(workspace.state_vector, "sv:1");

    let models: Vec<ModelLoadedRow> = client
        .get(format!("{base}/inspector/v1/models"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(models[0].model_id, "local-llama");

    let post_status = client
        .post(format!("{base}/inspector/v1/sessions"))
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(post_status, reqwest::StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn inspector_server_tests_rejects_non_loopback_bind_addresses() {
    let error = InspectorServer::bind_reader(
        SocketAddr::from(([0, 0, 0, 0], 0)),
        Arc::new(sample_reader()),
    )
    .await
    .expect_err("0.0.0.0 bind must fail");

    assert!(error.to_string().contains("127.0.0.1"));
}

#[tokio::test]
async fn inspector_server_tests_event_stream_websocket_emits_tail_snapshot() {
    let handle = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");
    let url = format!("ws://{}/inspector/v1/event-stream", handle.addr());
    let (mut socket, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    let message = socket
        .next()
        .await
        .expect("websocket message")
        .expect("websocket ok");
    let text = message.into_text().expect("text message");
    let payload: Value = serde_json::from_str(&text).expect("json payload");

    assert_eq!(payload["schema_id"], "hsk.inspector.event_stream@1");
    assert_eq!(payload["events"][0]["event_id"], "evt-1");
    assert_eq!(payload["events"][1]["event_id"], "evt-2");
}

#[test]
fn inspector_server_tests_source_is_feature_gated_and_release_off_by_default() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml = std::fs::read_to_string(manifest_dir.join("Cargo.toml")).unwrap();
    let mod_rs = std::fs::read_to_string(manifest_dir.join("src/inspector_read/mod.rs")).unwrap();
    let server_rs =
        std::fs::read_to_string(manifest_dir.join("src/inspector_read/server.rs")).unwrap();

    assert!(cargo_toml.contains("inspector ="));
    assert!(mod_rs.contains("#[cfg(feature = \"inspector\")]"));
    assert!(server_rs.contains("#![cfg(feature = \"inspector\")]"));
    assert!(server_rs.contains("127.0.0.1:0"));
    assert!(!server_rs.contains("0.0.0.0:0"));
}

fn sample_reader() -> InspectorReadSnapshot {
    let session_id = SessionId::new("session-alpha");
    let workspace_id = WorkspaceId::new("workspace-alpha");
    let mut snapshot = InspectorReadSnapshot::default();
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
        created_at_utc: "2026-05-18T09:00:00Z".to_string(),
        ..EventLedgerRow::default()
    });
    snapshot.event_ledger_tail.push(EventLedgerRow {
        event_id: "evt-2".to_string(),
        event_type: "session_completed".to_string(),
        event_sequence: 2,
        created_at_utc: "2026-05-18T09:01:00Z".to_string(),
        ..EventLedgerRow::default()
    });
    snapshot.processes.push(ProcessRow {
        process_uuid: "proc-1".to_string(),
        session_id,
        engine_kind: "webview2_cdp".to_string(),
        status: "running".to_string(),
    });
    snapshot.workspace_states.insert(
        workspace_id.clone(),
        WorkspaceStateRead {
            workspace_id,
            state_vector: "sv:1".to_string(),
            last_update_id: Some("update-1".to_string()),
            readable_refs: vec!["crdt://workspace-alpha/update-1".to_string()],
        },
    );
    snapshot.loaded_models.push(ModelLoadedRow {
        model_id: "local-llama".to_string(),
        adapter_id: "llama-cpp-placeholder".to_string(),
        process_uuid: Some("proc-1".to_string()),
        loaded_at_utc: Some("2026-05-18T09:00:00Z".to_string()),
    });
    snapshot
}
