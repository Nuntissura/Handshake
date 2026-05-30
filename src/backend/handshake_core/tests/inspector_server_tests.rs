#![cfg(feature = "inspector")]

use std::{net::SocketAddr, path::Path, sync::Arc};

use futures_util::StreamExt;
use serde_json::Value;

use handshake_core::inspector_read::{
    EventLedgerRow, InspectorReadSnapshot, InspectorServer, ModelLoadedRow, PerRunSecret,
    ProcessRow, SessionId, SessionStateRead, SessionSummary, WorkspaceId, WorkspaceStateRead,
    PER_RUN_SECRET_HEADER, PER_RUN_SECRET_LEN,
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
    let secret_hex = handle.per_run_secret().to_hex();
    let client = reqwest::Client::new();

    // MT-029 §6.5.5: every read endpoint now requires the per-run secret
    // header; supply it on every request.
    let sessions: Vec<SessionSummary> = client
        .get(format!("{base}/inspector/v1/sessions"))
        .header(PER_RUN_SECRET_HEADER, &secret_hex)
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
        .header(PER_RUN_SECRET_HEADER, &secret_hex)
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
        .header(PER_RUN_SECRET_HEADER, &secret_hex)
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
        .header(PER_RUN_SECRET_HEADER, &secret_hex)
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
        .header(PER_RUN_SECRET_HEADER, &secret_hex)
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
        .header(PER_RUN_SECRET_HEADER, &secret_hex)
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
        .header(PER_RUN_SECRET_HEADER, &secret_hex)
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(post_status, reqwest::StatusCode::METHOD_NOT_ALLOWED);
}

// MT-029 §6.5.5 proof: a read endpoint with NO per-run secret header is
// rejected 401 (was fully unauthenticated before this remediation).
#[tokio::test]
async fn inspector_server_tests_read_endpoint_without_secret_header_returns_401() {
    let handle = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");
    let base = format!("http://{}", handle.addr());
    let client = reqwest::Client::new();

    for route in [
        "/inspector/v1/sessions",
        "/inspector/v1/sessions/session-alpha",
        "/inspector/v1/event-ledger/tail?n=1",
        "/inspector/v1/process-ledger/active",
        "/inspector/v1/workspace/workspace-alpha",
        "/inspector/v1/trace/session-alpha",
        "/inspector/v1/models",
    ] {
        let status = client
            .get(format!("{base}{route}"))
            .send()
            .await
            .unwrap()
            .status();
        assert_eq!(
            status,
            reqwest::StatusCode::UNAUTHORIZED,
            "read route {route} must reject a request with no per-run secret header"
        );
    }
}

// MT-029 §6.5.5 proof: a read endpoint WITH the correct header returns 200.
#[tokio::test]
async fn inspector_server_tests_read_endpoint_with_correct_secret_header_returns_200() {
    let handle = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");
    let base = format!("http://{}", handle.addr());
    let secret_hex = handle.per_run_secret().to_hex();
    let client = reqwest::Client::new();

    let status = client
        .get(format!("{base}/inspector/v1/sessions"))
        .header(PER_RUN_SECRET_HEADER, &secret_hex)
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(status, reqwest::StatusCode::OK);
}

// MT-029 §6.5.5 proof: a read endpoint with the WRONG header is rejected 401.
#[tokio::test]
async fn inspector_server_tests_read_endpoint_with_wrong_secret_header_returns_401() {
    let handle = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");
    let base = format!("http://{}", handle.addr());
    let client = reqwest::Client::new();

    let status = client
        .get(format!("{base}/inspector/v1/sessions"))
        .header(PER_RUN_SECRET_HEADER, PerRunSecret::from_bytes([0u8; 32]).to_hex())
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
}

// Approved-concern proof: the per-run secret is drawn from the OS CSPRNG,
// is at least 128 bits, and is not derivable from launch time. Two
// back-to-back launches must not collide and must not share a leading
// timestamp prefix (the Uuid::now_v7 weakness this replaced). 256 random
// bits make accidental collision/prefix-sharing astronomically unlikely.
#[tokio::test]
async fn inspector_server_tests_per_run_secret_is_csprng_and_not_time_derivable() {
    let first = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");
    let second = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");

    let a = first.per_run_secret();
    let b = second.per_run_secret();

    // CSPRNG-sized: 32 bytes = 256 bits, well above the 128-bit minimum.
    assert_eq!(a.len(), PER_RUN_SECRET_LEN);
    assert!(a.len() * 8 >= 128, "per-run secret must carry >= 128 bits");
    assert_eq!(a.to_hex().len(), PER_RUN_SECRET_LEN * 2);

    // Distinct per launch.
    assert_ne!(a.to_hex(), b.to_hex());

    // Not time-ordered: with Uuid::now_v7 the first 48 bits were a
    // millisecond timestamp, so two near-simultaneous launches shared a
    // long leading hex prefix. A CSPRNG draw shares essentially no prefix.
    let shared_prefix = a
        .as_bytes()
        .iter()
        .zip(b.as_bytes().iter())
        .take_while(|(x, y)| x == y)
        .count();
    assert!(
        shared_prefix <= 2,
        "per-run secrets share an implausibly long {shared_prefix}-byte prefix; \
         secret may still be time-ordered"
    );
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
    // MT-029 §6.5.5: the event-stream WS upgrade is an inspector request and
    // now requires the per-run secret header too. Build a client request and
    // attach the header before connecting.
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut request = url.into_client_request().unwrap();
    request.headers_mut().insert(
        PER_RUN_SECRET_HEADER,
        handle.per_run_secret().to_hex().parse().unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
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

// MT-029 §6.5.5 reject-log proof: a rejected read request audit-logs the
// (timestamp, route, peer_addr, reason) tuple. We install a capturing
// tracing layer, fire an unauthenticated read, and assert a reject record
// was emitted carrying a non-empty `peer_addr`, the requested `route`, and a
// `reason`. The `tracing` framework stamps each event with the timestamp.
#[tokio::test]
async fn inspector_server_tests_reject_audit_log_captures_peer_addr_route_reason() {
    let records = reject_capture::install();

    let handle = InspectorServer::start(Arc::new(sample_reader()))
        .await
        .expect("server starts");
    let base = format!("http://{}", handle.addr());
    let client = reqwest::Client::new();

    // Unauthenticated read -> 401 -> audit reject record.
    let status = client
        .get(format!("{base}/inspector/v1/sessions"))
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);

    // Give the server task a moment to emit the warning, then inspect the
    // captured records.
    let captured = reject_capture::wait_for_record(&records).await;
    assert!(
        captured.iter().any(|r| {
            r.route == "/inspector/v1/sessions"
                && !r.peer_addr.is_empty()
                && r.peer_addr.starts_with("127.0.0.1:")
                && r.reason == "missing_per_run_secret_header"
        }),
        "expected a reject audit record with route, non-empty 127.0.0.1 peer_addr, \
         and reason; captured: {captured:?}"
    );
}

/// Test-local tracing capture for the inspector reject audit log. Installs
/// a process-global subscriber exactly once (subsequent tests reuse it) and
/// records every event on the inspector target that carries `route`,
/// `peer_addr`, and `reason` fields.
mod reject_capture {
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;

    use tracing::field::{Field, Visit};
    use tracing::Subscriber;
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::registry::LookupSpan;

    #[derive(Debug, Clone, Default)]
    pub struct RejectRecord {
        pub route: String,
        pub peer_addr: String,
        pub reason: String,
    }

    static RECORDS: OnceLock<Mutex<Vec<RejectRecord>>> = OnceLock::new();

    fn store() -> &'static Mutex<Vec<RejectRecord>> {
        RECORDS.get_or_init(|| Mutex::new(Vec::new()))
    }

    struct RejectVisitor {
        record: RejectRecord,
    }

    impl Visit for RejectVisitor {
        fn record_str(&mut self, field: &Field, value: &str) {
            match field.name() {
                "route" => self.record.route = value.to_string(),
                "peer_addr" => self.record.peer_addr = value.to_string(),
                "reason" => self.record.reason = value.to_string(),
                _ => {}
            }
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            // `%route` / `%peer_addr` are recorded via Display -> record_str;
            // `reason = reason` (a &str literal) also arrives as record_str.
            // This catches any Debug-formatted fallback so the fields are not
            // silently dropped.
            let rendered = format!("{value:?}");
            let trimmed = rendered.trim_matches('"').to_string();
            match field.name() {
                "route" if self.record.route.is_empty() => self.record.route = trimmed,
                "peer_addr" if self.record.peer_addr.is_empty() => self.record.peer_addr = trimmed,
                "reason" if self.record.reason.is_empty() => self.record.reason = trimmed,
                _ => {}
            }
        }
    }

    struct CaptureLayer;

    impl<S> Layer<S> for CaptureLayer
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            if event.metadata().target() != "handshake_core::inspector_read" {
                return;
            }
            let mut visitor = RejectVisitor {
                record: RejectRecord::default(),
            };
            event.record(&mut visitor);
            // Only retain events that look like a reject record (they carry
            // a reason field).
            if !visitor.record.reason.is_empty() || !visitor.record.peer_addr.is_empty() {
                store().lock().unwrap().push(visitor.record);
            }
        }
    }

    pub fn install() -> &'static Mutex<Vec<RejectRecord>> {
        static INSTALLED: OnceLock<()> = OnceLock::new();
        INSTALLED.get_or_init(|| {
            let subscriber = tracing_subscriber::registry().with(CaptureLayer);
            // Ignore the error if another test in the binary already set a
            // global subscriber; our capture store is shared regardless.
            let _ = tracing::subscriber::set_global_default(subscriber);
        });
        store().lock().unwrap().clear();
        store()
    }

    pub async fn wait_for_record(
        records: &'static Mutex<Vec<RejectRecord>>,
    ) -> Vec<RejectRecord> {
        for _ in 0..50 {
            {
                let guard = records.lock().unwrap();
                if !guard.is_empty() {
                    return guard.clone();
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        records.lock().unwrap().clone()
    }
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
