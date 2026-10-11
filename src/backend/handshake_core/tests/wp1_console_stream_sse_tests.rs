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
use handshake_core::api::account_scope::ProductLocalResourceScope;
use handshake_core::console_stream::{
    ConsoleBroadcast, ConsoleCategory, ConsoleEntry, ConsoleEntryDraft, ConsoleSeverity,
    ConsoleSwarmSink,
};
use handshake_core::model_runtime::ModelId;
use handshake_core::swarm_orchestration::events::{SwarmEvent, SwarmEventSink};
use handshake_core::swarm_orchestration::ids::ModelInstanceId;
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, WorkspaceScopeRef,
};
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
            Ok(Ok(None)) => break, // stream ended
            Ok(Err(err)) => panic!("SSE read error: {err}"),
            Err(_) => continue, // per-read timeout; re-check deadline
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

/// Collect the SSE `id:` and decoded payload together so privacy tests prove
/// both externally visible sequence surfaces are account-local.
async fn collect_marked_frames(
    mut response: reqwest::Response,
    marker: &str,
    want: usize,
    overall_timeout: Duration,
) -> Vec<(String, ConsoleEntry)> {
    let mut buf = String::new();
    let mut collected = Vec::new();
    let deadline = tokio::time::Instant::now() + overall_timeout;

    while collected.len() < want && tokio::time::Instant::now() < deadline {
        let chunk = match tokio::time::timeout(Duration::from_millis(250), response.chunk()).await {
            Ok(Ok(Some(bytes))) => bytes,
            Ok(Ok(None)) => break,
            Ok(Err(error)) => panic!("SSE read error: {error}"),
            Err(_) => continue,
        };
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(index) = buf.find("\n\n") {
            let raw = buf[..index].to_owned();
            buf.drain(..index + 2);
            let id = raw
                .lines()
                .find_map(|line| line.strip_prefix("id:").map(str::trim))
                .map(str::to_owned);
            let entry = raw.lines().find_map(|line| {
                line.strip_prefix("data:")
                    .and_then(|data| serde_json::from_str::<ConsoleEntry>(data.trim()).ok())
            });
            if let (Some(id), Some(entry)) = (id, entry) {
                if entry.subject.contains(marker) {
                    collected.push((id, entry));
                }
            }
        }
    }
    collected
}

fn exact_scope(owner: OwnerAccountId, workspace: &str) -> ExactResourceScopeAttribution {
    ExactResourceScopeAttribution {
        owner_account_id: owner,
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new(workspace).expect("valid test workspace"),
    }
}

async fn collect_stream_text(mut response: reqwest::Response, duration: Duration) -> String {
    let deadline = tokio::time::Instant::now() + duration;
    let mut body = String::new();
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(250), response.chunk()).await {
            Ok(Ok(Some(bytes))) => body.push_str(&String::from_utf8_lossy(&bytes)),
            Ok(Ok(None)) => break,
            Ok(Err(error)) => panic!("SSE read error: {error}"),
            Err(_) => {}
        }
    }
    body
}

fn scoped_console_draft(
    subject: impl Into<String>,
    scope: ExactResourceScopeAttribution,
) -> ConsoleEntryDraft {
    ConsoleEntryDraft::new(
        ConsoleSeverity::Info,
        ConsoleCategory::System,
        subject,
        "scope privacy regression probe",
        None,
    )
    .with_resource_scope(scope)
}

async fn connect_isolated_console(
    hub: ConsoleBroadcast,
    scope: ExactResourceScopeAttribution,
) -> reqwest::Response {
    let owner = scope.owner_account_id;
    let workspace = scope.workspace_id.as_str().to_owned();
    let server_scope =
        ProductLocalResourceScope::from_exact(scope).expect("valid server-owned console scope");
    let (base, _server) =
        start_server(api::console_stream::routes(hub).layer(axum::Extension(server_scope))).await;
    let response = reqwest::Client::new()
        .get(format!("{base}/wp1/diagnostics/console/stream"))
        .header("accept", "text/event-stream")
        .header("x-handshake-owner-account", owner.to_string())
        .header("x-handshake-workspace", &workspace)
        .send()
        .await
        .expect("connect isolated console SSE reader");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    response
}

#[tokio::test]
async fn wp1_console_stream_uses_server_scope_without_optional_assertion_headers() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "wp1_console_stream_missing_scope"
    );
    let state = app_state_for(&kpg.schema_url).await;
    let server_scope = ProductLocalResourceScope::from_exact(exact_scope(
        OwnerAccountId::mint(),
        "console-missing-header",
    ))
    .expect("valid server scope");
    let (base, _server) =
        start_server(api::routes_with_product_local_scope(state, server_scope)).await;

    let response = reqwest::Client::new()
        .get(format!("{base}/wp1/diagnostics/console/stream"))
        .header("accept", "text/event-stream")
        .send()
        .await
        .expect("request console SSE endpoint without a resource scope");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "server-owned exact scope authorizes the stream without caller-selected headers"
    );
}

#[tokio::test]
async fn wp1_console_stream_streams_swarm_events_in_order_over_the_real_router() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "wp1_console_stream_sse"
    );
    let state = app_state_for(&kpg.schema_url).await;
    let owner = OwnerAccountId::mint();
    let workspace = format!("console-owner-{}", uuid::Uuid::now_v7());
    let projection_scope = exact_scope(owner, &workspace);
    let server_scope = ProductLocalResourceScope::from_exact(projection_scope.clone())
        .expect("valid server scope");
    let (base, _server) =
        start_server(api::routes_with_product_local_scope(state, server_scope)).await;
    let http = reqwest::Client::new();

    // Connect. `send()` resolves once the response headers arrive; the handler
    // subscribes to the shared console hub BEFORE returning the SSE response, so
    // the subscription is live by the time we publish below.
    let resp = http
        .get(format!("{base}/wp1/diagnostics/console/stream"))
        .header("accept", "text/event-stream")
        .header("x-handshake-owner-account", owner.to_string())
        .header("x-handshake-workspace", &workspace)
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
    let sink = ConsoleSwarmSink::new_scoped(ConsoleBroadcast::shared(), projection_scope.clone());
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
    sink.emit(SwarmEvent::SessionCompleted {
        instance_id: iid,
        event_id: None,
    })
    .expect("console tee never errors");

    let collected = collect_marked_entries(resp, &marker, 5, Duration::from_secs(20)).await;

    assert!(
        collected.len() >= 5,
        "expected at least 5 teed console entries to stream through, got {}: {collected:?}",
        collected.len()
    );

    // Categories arrive in the exact emission order.
    let categories: Vec<ConsoleCategory> = collected
        .iter()
        .take(5)
        .map(|entry| entry.category)
        .collect();
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
    assert!(
        collected
            .iter()
            .all(|entry| entry.resource_scope.as_ref() == Some(&projection_scope)),
        "every identifier-bearing console entry carries the exact five-field durable scope"
    );
}

#[tokio::test]
async fn wp1_console_stream_hides_cross_account_and_wrong_workspace_identifiers() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "wp1_console_stream_scope_isolation"
    );
    let state = app_state_for(&kpg.schema_url).await;
    let owner = OwnerAccountId::mint();
    let other_owner = OwnerAccountId::mint();
    let workspace = format!("console-private-{}", uuid::Uuid::now_v7());
    let projection_scope = exact_scope(owner, &workspace);
    let server_scope = ProductLocalResourceScope::from_exact(projection_scope.clone())
        .expect("valid server scope");
    let (base, _server) =
        start_server(api::routes_with_product_local_scope(state, server_scope)).await;
    let http = reqwest::Client::new();

    let cross_account = http
        .get(format!("{base}/wp1/diagnostics/console/stream"))
        .header("accept", "text/event-stream")
        .header("x-handshake-owner-account", other_owner.to_string())
        .header("x-handshake-workspace", &workspace)
        .send()
        .await
        .expect("connect cross-account SSE reader");
    let wrong_workspace = http
        .get(format!("{base}/wp1/diagnostics/console/stream"))
        .header("accept", "text/event-stream")
        .header("x-handshake-owner-account", owner.to_string())
        .header("x-handshake-workspace", format!("{workspace}-other"))
        .send()
        .await
        .expect("connect wrong-workspace SSE reader");
    assert_eq!(cross_account.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(wrong_workspace.status(), reqwest::StatusCode::FORBIDDEN);

    let model_id = ModelId::new_v7();
    let marker = model_id.to_string();
    let process_uuid = uuid::Uuid::now_v7();
    let worktree_id = format!("private-worktree-{}", uuid::Uuid::now_v7());
    ConsoleSwarmSink::new_scoped(ConsoleBroadcast::shared(), projection_scope)
        .emit(SwarmEvent::SessionSpawned {
            instance_id: ModelInstanceId::new(model_id, 0),
            parent_session_id: "private-parent-session".to_owned(),
            process_uuid,
            swarm_id: Some("private-swarm".to_owned()),
            worktree_id: Some(worktree_id.clone()),
        })
        .expect("console tee never errors");

    let (cross_body, workspace_body) = tokio::join!(
        collect_stream_text(cross_account, Duration::from_secs(2)),
        collect_stream_text(wrong_workspace, Duration::from_secs(2)),
    );
    for denied_body in [&cross_body, &workspace_body] {
        assert!(!denied_body.contains(&marker), "model identifier leaked");
        assert!(
            !denied_body.contains(&process_uuid.to_string()),
            "process identifier leaked"
        );
        assert!(
            !denied_body.contains(&worktree_id),
            "worktree identifier leaked"
        );
    }
}

#[tokio::test]
async fn wp1_console_replay_hides_foreign_and_unattributed_entries_published_before_connect() {
    let hub = ConsoleBroadcast::new(8, 8);
    let owner = OwnerAccountId::mint();
    let foreign_owner = OwnerAccountId::mint();
    let workspace = format!("console-replay-private-{}", uuid::Uuid::now_v7());
    let owner_scope = exact_scope(owner, &workspace);
    let foreign_scope = exact_scope(foreign_owner, &workspace);
    let owner_marker = format!("owner-replay-{}", uuid::Uuid::now_v7());
    let foreign_marker = format!("foreign-replay-{}", uuid::Uuid::now_v7());
    let unattributed_marker = format!("unattributed-replay-{}", uuid::Uuid::now_v7());

    hub.publish(scoped_console_draft(&owner_marker, owner_scope.clone()));
    hub.publish(scoped_console_draft(&foreign_marker, foreign_scope));
    hub.publish(ConsoleEntryDraft::new(
        ConsoleSeverity::Info,
        ConsoleCategory::System,
        &unattributed_marker,
        "must remain system-only",
        None,
    ));

    let response = connect_isolated_console(hub, owner_scope).await;
    let body = collect_stream_text(response, Duration::from_secs(1)).await;
    assert!(body.contains(&owner_marker), "owner replay row was lost");
    assert!(
        !body.contains(&foreign_marker),
        "foreign replay identifier leaked"
    );
    assert!(
        !body.contains(&unattributed_marker),
        "unattributed replay identifier leaked"
    );
}

#[tokio::test]
async fn wp1_console_live_rejects_unattributed_entries_and_recovers_to_owner_entry() {
    let hub = ConsoleBroadcast::new(8, 8);
    let owner = OwnerAccountId::mint();
    let workspace = format!("console-live-private-{}", uuid::Uuid::now_v7());
    let owner_scope = exact_scope(owner, &workspace);
    let unattributed_marker = format!("unattributed-live-{}", uuid::Uuid::now_v7());
    let owner_marker = format!("owner-live-{}", uuid::Uuid::now_v7());
    let response = connect_isolated_console(hub.clone(), owner_scope.clone()).await;

    hub.publish(ConsoleEntryDraft::new(
        ConsoleSeverity::Info,
        ConsoleCategory::System,
        &unattributed_marker,
        "must remain system-only",
        None,
    ));
    hub.publish(scoped_console_draft(&owner_marker, owner_scope));

    let body = collect_stream_text(response, Duration::from_secs(1)).await;
    assert!(body.contains(&owner_marker), "owner live row was lost");
    assert!(
        !body.contains(&unattributed_marker),
        "unattributed live identifier leaked"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn wp1_console_tiny_buffer_suppresses_foreign_lag_metadata_then_recovers_owner() {
    let hub = ConsoleBroadcast::new(1, 1);
    let owner = OwnerAccountId::mint();
    let foreign_owner = OwnerAccountId::mint();
    let workspace = format!("console-lag-private-{}", uuid::Uuid::now_v7());
    let owner_scope = exact_scope(owner, &workspace);
    let foreign_scope = exact_scope(foreign_owner, &workspace);
    let foreign_marker = format!("foreign-lag-{}", uuid::Uuid::now_v7());
    let owner_marker = format!("owner-after-lag-{}", uuid::Uuid::now_v7());
    let response = connect_isolated_console(hub.clone(), owner_scope.clone()).await;

    // A current-thread runtime and a capacity-one ring make the subscribed
    // receiver fall behind deterministically while this synchronous burst owns
    // the executor. The latest owner row must still be delivered after the
    // Lagged notice is consumed internally.
    for index in 0..64 {
        hub.publish(scoped_console_draft(
            format!("{foreign_marker}-{index}"),
            foreign_scope.clone(),
        ));
    }
    hub.publish(scoped_console_draft(&owner_marker, owner_scope));

    let body = collect_stream_text(response, Duration::from_secs(1)).await;
    assert!(
        body.contains(&owner_marker),
        "the owner stream did not recover after a foreign-only lag burst"
    );
    assert!(
        !body.contains(&foreign_marker),
        "foreign identifiers leaked through the lag path"
    );
    assert!(
        !body.contains("console_lagged") && !body.contains("skipped"),
        "foreign traffic volume leaked through lag metadata"
    );
}

#[tokio::test]
async fn wp1_console_replay_live_boundary_is_strictly_ordered_without_duplicates() {
    let hub = ConsoleBroadcast::new(8, 8);
    let owner = OwnerAccountId::mint();
    let workspace = format!("console-boundary-{}", uuid::Uuid::now_v7());
    let owner_scope = exact_scope(owner, &workspace);
    let marker = format!("boundary-{}", uuid::Uuid::now_v7());

    hub.publish(scoped_console_draft(
        format!("{marker}-replay-1"),
        owner_scope.clone(),
    ));
    hub.publish(scoped_console_draft(
        format!("{marker}-replay-2"),
        owner_scope.clone(),
    ));
    let response = connect_isolated_console(hub.clone(), owner_scope.clone()).await;
    hub.publish(scoped_console_draft(
        format!("{marker}-live-1"),
        owner_scope.clone(),
    ));
    hub.publish(scoped_console_draft(
        format!("{marker}-live-2"),
        owner_scope,
    ));

    let entries = collect_marked_entries(response, &marker, 4, Duration::from_secs(3)).await;
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.subject.as_str())
            .collect::<Vec<_>>(),
        vec![
            format!("{marker}-replay-1"),
            format!("{marker}-replay-2"),
            format!("{marker}-live-1"),
            format!("{marker}-live-2"),
        ]
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>(),
        "replay must finish before the live tail and the boundary must not duplicate rows"
    );
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.seq)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4,
        "every replay/live row must appear exactly once"
    );
    assert!(
        entries.windows(2).all(|pair| pair[0].seq < pair[1].seq),
        "replay/live sequence ids must remain strictly increasing: {entries:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wp1_console_forced_subscribe_replay_overlap_is_emitted_once() {
    let hub = ConsoleBroadcast::new(8, 8);
    let owner = OwnerAccountId::mint();
    let workspace = format!("console-forced-overlap-{}", uuid::Uuid::now_v7());
    let owner_scope = exact_scope(owner, &workspace);
    let marker = format!("forced-overlap-{}", uuid::Uuid::now_v7());
    let gate = hub.arm_recent_snapshot_gate_for_tests();

    let connecting_hub = hub.clone();
    let connecting_scope = owner_scope.clone();
    let connect =
        tokio::spawn(
            async move { connect_isolated_console(connecting_hub, connecting_scope).await },
        );
    let waiting_gate = gate.clone();
    tokio::task::spawn_blocking(move || waiting_gate.wait_until_blocked())
        .await
        .expect("wait for subscribe/replay boundary");

    let publishing_hub = hub.clone();
    let published_marker = marker.clone();
    let publisher = tokio::task::spawn_blocking(move || {
        publishing_hub.publish(scoped_console_draft(&published_marker, owner_scope))
    });
    gate.release();
    let response = connect.await.expect("console connection task");
    publisher.await.expect("boundary publisher");
    let entries = collect_marked_entries(response, &marker, 2, Duration::from_secs(1)).await;
    assert_eq!(
        entries.len(),
        1,
        "an entry present in both the replay snapshot and subscribed live ring must be emitted once: {entries:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wp1_console_atomic_snapshot_preserves_owner_order_across_foreign_eviction_burst() {
    let hub = ConsoleBroadcast::new(64, 2);
    let owner = OwnerAccountId::mint();
    let foreign = OwnerAccountId::mint();
    let workspace = format!("console-eviction-window-{}", uuid::Uuid::now_v7());
    let owner_scope = exact_scope(owner, &workspace);
    let foreign_scope = exact_scope(foreign, &workspace);
    let owner_marker = format!("ordered-owner-{}", uuid::Uuid::now_v7());
    let owner_first = format!("{owner_marker}-first");
    let owner_second = format!("{owner_marker}-second");
    let foreign_marker = format!("evicting-foreign-{}", uuid::Uuid::now_v7());
    let gate = hub.arm_recent_snapshot_gate_for_tests();

    let connecting_hub = hub.clone();
    let connecting_scope = owner_scope.clone();
    let connect =
        tokio::spawn(
            async move { connect_isolated_console(connecting_hub, connecting_scope).await },
        );
    let waiting_gate = gate.clone();
    tokio::task::spawn_blocking(move || waiting_gate.wait_until_blocked())
        .await
        .expect("wait for subscribe/replay boundary");

    // Queue the exact historical failure interleaving while the route holds the
    // publication mutex: owner E1, a foreign eviction burst, then owner E2.
    let publishing_hub = hub.clone();
    let published_foreign_marker = foreign_marker.clone();
    let publisher = tokio::task::spawn_blocking(move || {
        publishing_hub.publish(scoped_console_draft(&owner_first, owner_scope.clone()));
        for index in 0..8 {
            publishing_hub.publish(scoped_console_draft(
                format!("{published_foreign_marker}-{index}"),
                foreign_scope.clone(),
            ));
        }
        publishing_hub.publish(scoped_console_draft(&owner_second, owner_scope));
    });
    gate.release();

    let response = connect.await.expect("console connection task");
    publisher.await.expect("ordered boundary publisher");
    let body = collect_stream_text(response, Duration::from_secs(1)).await;
    let owner_frames = body
        .split("\n\n")
        .filter_map(|frame| {
            let id = frame
                .lines()
                .find_map(|line| line.strip_prefix("id:").map(str::trim))?;
            let entry = frame.lines().find_map(|line| {
                line.strip_prefix("data:")
                    .and_then(|data| serde_json::from_str::<ConsoleEntry>(data.trim()).ok())
            })?;
            entry
                .subject
                .contains(&owner_marker)
                .then(|| (id.to_owned(), entry))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        owner_frames.len(),
        2,
        "both owner events must arrive: {body}"
    );
    assert_eq!(
        owner_frames
            .iter()
            .map(|(_, entry)| entry.subject.as_str())
            .collect::<Vec<_>>(),
        vec![
            format!("{owner_marker}-first"),
            format!("{owner_marker}-second")
        ]
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>(),
        "atomic snapshot/live handoff must preserve publication order"
    );
    assert_eq!(
        owner_frames
            .iter()
            .map(|(id, entry)| (id.as_str(), entry.seq))
            .collect::<Vec<_>>(),
        vec![("0", 0), ("1", 1)]
    );
    assert!(
        !body.contains(&foreign_marker),
        "foreign replay/live rows must not leak: {body}"
    );
    assert!(
        !body.contains("console_lagged") && !body.contains("skipped"),
        "foreign traffic must not expose lag metadata: {body}"
    );
}

#[tokio::test]
async fn wp1_console_visible_ids_are_contiguous_across_foreign_bursts() {
    let hub = ConsoleBroadcast::new(64, 64);
    let owner = OwnerAccountId::mint();
    let foreign = OwnerAccountId::mint();
    let workspace = format!("console-contiguous-{}", uuid::Uuid::now_v7());
    let owner_scope = exact_scope(owner, &workspace);
    let foreign_scope = exact_scope(foreign, &workspace);
    let marker = format!("visible-owner-{}", uuid::Uuid::now_v7());
    let response = connect_isolated_console(hub.clone(), owner_scope.clone()).await;

    hub.publish(scoped_console_draft(
        format!("{marker}-first"),
        owner_scope.clone(),
    ));
    for index in 0..32 {
        hub.publish(scoped_console_draft(
            format!("hidden-foreign-{marker}-{index}"),
            foreign_scope.clone(),
        ));
    }
    hub.publish(scoped_console_draft(
        format!("{marker}-second"),
        owner_scope,
    ));

    let frames = collect_marked_frames(response, &marker, 2, Duration::from_secs(3)).await;
    assert_eq!(frames.len(), 2, "both authorized entries must arrive");
    assert_eq!(
        frames.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
        vec!["0", "1"],
        "SSE ids must not reveal the count of filtered foreign entries"
    );
    assert_eq!(
        frames
            .iter()
            .map(|(_, entry)| entry.seq)
            .collect::<Vec<_>>(),
        vec![0, 1],
        "payload sequence ids must be contiguous inside the authorized stream"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn wp1_console_concurrent_publishers_keep_history_and_live_order_identical() {
    const PUBLISHERS: usize = 256;
    let hub = ConsoleBroadcast::new(PUBLISHERS * 2, PUBLISHERS * 2);
    let mut live = hub.subscribe();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(PUBLISHERS));
    let mut tasks = Vec::with_capacity(PUBLISHERS);
    for index in 0..PUBLISHERS {
        let publisher = hub.clone();
        let start = barrier.clone();
        tasks.push(tokio::spawn(async move {
            start.wait().await;
            publisher.publish_parts(
                ConsoleSeverity::Info,
                ConsoleCategory::System,
                format!("concurrent-{index}"),
                "concurrent ordering proof",
                None,
            )
        }));
    }
    let mut returned = Vec::with_capacity(PUBLISHERS);
    for task in tasks {
        returned.push(task.await.expect("concurrent publisher task"));
    }
    let history = hub.recent(PUBLISHERS);
    let mut received = Vec::with_capacity(PUBLISHERS);
    for _ in 0..PUBLISHERS {
        received.push(live.recv().await.expect("concurrent live entry"));
    }

    assert_eq!(
        received, history,
        "history and live subscribers must observe the same serialized publish order"
    );
    assert_eq!(
        history.iter().map(|entry| entry.seq).collect::<Vec<_>>(),
        (0..PUBLISHERS as u64).collect::<Vec<_>>(),
        "serialized publication must preserve contiguous sequence order"
    );
    let returned_by_seq = returned
        .into_iter()
        .map(|entry| (entry.seq, entry.subject))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        returned_by_seq.len(),
        PUBLISHERS,
        "each concurrent publisher receives one unique sequence"
    );
}
