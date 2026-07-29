//! WP-1 live orchestration debug console — HEADLESS SSE proof.
//!
//! Connects to `GET /wp1/diagnostics/console/stream` over the REAL full product
//! router (`api::routes`) on a loopback listener (quiet; no foreground window),
//! then triggers real WP-1 orchestration `SwarmEvent`s through a
//! `ConsoleSwarmSink` bound to the SAME process-wide hub the route reads
//! (`ConsoleBroadcast::shared()`). It asserts the structured console entries
//! stream through IN ORDER with the right categories/severities — proving the
//! SwarmEvent -> ConsoleEntry mapping, the broadcast tee, and the SSE
//! serialization end to end, headlessly, with managed PostgreSQL.

mod knowledge_pg_support;
#[allow(dead_code)]
mod user_manual_support;

use std::time::Duration;

use handshake_core::api;
use handshake_core::console_stream::{
    ConsoleBroadcast, ConsoleCategory, ConsoleEntry, ConsoleSeverity, ConsoleSwarmSink,
};
use handshake_core::model_runtime::ModelId;
use handshake_core::swarm_orchestration::events::{SwarmEvent, SwarmEventSink};
use handshake_core::swarm_orchestration::ids::ModelInstanceId;
use handshake_core::swarm_orchestration::state::ModelSessionState;
use user_manual_support::{app_state_for, start_server};

/// Read SSE `data:` lines from the response, deserialize each into a
/// [`ConsoleEntry`], keep those whose subject carries `marker`, and return once
/// `want` matching entries are collected or the deadline elapses.
async fn collect_marked_entries(
    resp: reqwest::Response,
    marker: &str,
    want: usize,
    overall_timeout: Duration,
) -> Vec<ConsoleEntry> {
    let mut resp = resp;
    let mut buf = String::new();
    let mut collected: Vec<ConsoleEntry> = Vec::new();
    let deadline = tokio::time::Instant::now() + overall_timeout;

    while collected.len() < want && tokio::time::Instant::now() < deadline {
        let chunk = match tokio::time::timeout(Duration::from_secs(3), resp.chunk()).await {
            Ok(Ok(Some(bytes))) => bytes,
            Ok(Ok(None)) => break,               // stream ended
            Ok(Err(err)) => panic!("SSE read error: {err}"),
            Err(_) => continue,                  // per-read timeout; re-check deadline
        };
        buf.push_str(&String::from_utf8_lossy(&chunk));

        // SSE events are separated by a blank line.
        while let Some(idx) = buf.find("\n\n") {
            let raw: String = buf[..idx].to_string();
            buf.drain(..idx + 2);
            for line in raw.lines() {
                let Some(data) = line.strip_prefix("data:") else {
                    continue; // event:/id:/comment/keep-alive lines
                };
                let data = data.trim();
                if let Ok(entry) = serde_json::from_str::<ConsoleEntry>(data) {
                    if entry.subject.contains(marker) {
                        collected.push(entry);
                    }
                }
            }
        }
    }
    collected
}

#[tokio::test]
async fn wp1_console_stream_streams_swarm_events_in_order_over_the_real_router() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "wp1_console_stream_sse"
    );
    let state = app_state_for(&kpg.schema_url).await;
    let (base, _server) = start_server(api::routes(state)).await;
    let http = reqwest::Client::new();

    // Connect. `send()` resolves once the response headers arrive; the handler
    // subscribes to the shared console hub BEFORE returning the SSE response, so
    // the subscription is live by the time we publish below.
    let resp = http
        .get(format!("{base}/wp1/diagnostics/console/stream"))
        .header("accept", "text/event-stream")
        .send()
        .await
        .expect("connect to console SSE endpoint");
    assert_eq!(resp.status(), reqwest::StatusCode::OK, "SSE returns 200");
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(
        content_type.starts_with("text/event-stream"),
        "SSE content-type, got {content_type}"
    );

    // Let the streaming body task start polling the subscriber before publishing.
    tokio::time::sleep(Duration::from_millis(250)).await;

    // Trigger REAL WP-1 orchestration events through a ConsoleSwarmSink bound to
    // the SAME process-wide hub the route reads. Every event uses one instance id
    // whose Display carries a per-run-unique model UUID; that UUID is the marker
    // isolating this test's entries from any replayed/concurrent hub traffic.
    let sink = ConsoleSwarmSink::new(ConsoleBroadcast::shared());
    let model_id = ModelId::new_v7();
    let marker = model_id.to_string();
    let iid = ModelInstanceId::new(model_id, 0);

    // Ordered batch spanning the teed WP-1 categories.
    sink.emit(SwarmEvent::SessionSpawned {
        instance_id: iid,
        parent_session_id: "owner-session".to_string(),
        process_uuid: uuid::Uuid::now_v7(),
        swarm_id: Some("swarm-alpha".to_string()),
        worktree_id: None,
    })
    .expect("console tee never errors");
    sink.emit(SwarmEvent::SessionStateChanged {
        instance_id: iid,
        from: ModelSessionState::Loading,
        to: ModelSessionState::Ready,
    })
    .expect("console tee never errors");
    sink.emit(SwarmEvent::ModelInvocationStarted {
        instance_id: iid,
        trace_id: uuid::Uuid::now_v7(),
        run_id: "run-1".to_string(),
        session_id: "session-1".to_string(),
        max_tokens: 256,
    })
    .expect("console tee never errors");
    sink.emit(SwarmEvent::ModelInvocationFinished {
        instance_id: iid,
        trace_id: uuid::Uuid::now_v7(),
        run_id: "run-1".to_string(),
        session_id: "session-1".to_string(),
        outcome: "failed".to_string(),
        generated_tokens: 3,
        error: Some("provider failed".to_string()),
    })
    .expect("console tee never errors");
    sink.emit(SwarmEvent::SessionCompleted { instance_id: iid })
        .expect("console tee never errors");

    let collected =
        collect_marked_entries(resp, &marker, 5, Duration::from_secs(20)).await;

    assert!(
        collected.len() >= 5,
        "expected at least 5 teed console entries to stream through, got {}: {collected:?}",
        collected.len()
    );

    // Categories arrive in the exact emission order.
    let categories: Vec<ConsoleCategory> =
        collected.iter().take(5).map(|entry| entry.category).collect();
    assert_eq!(
        categories,
        vec![
            ConsoleCategory::ModelLaneLaunch,
            ConsoleCategory::ModelLaneStatus,
            ConsoleCategory::ModelInvocation,
            ConsoleCategory::ModelInvocation,
            ConsoleCategory::ModelLaneStatus,
        ],
        "streamed console entries preserve emission order + category mapping"
    );

    // The monotonic seq is strictly increasing across the ordered tail.
    for pair in collected.windows(2) {
        assert!(
            pair[1].seq > pair[0].seq,
            "console seq must be strictly increasing: {} then {}",
            pair[0].seq,
            pair[1].seq
        );
    }

    // The failed invocation is surfaced at error severity with its detail.
    let failed = &collected[3];
    assert_eq!(failed.severity, ConsoleSeverity::Error);
    assert!(
        failed.detail.contains("provider failed"),
        "failed invocation detail carries the error: {}",
        failed.detail
    );

    // The invocation entries carry the trace id (correlation for headless triage).
    assert!(
        collected[2].trace_id.is_some(),
        "model invocation entry carries a trace id"
    );
}
