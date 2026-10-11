//! WP-1 console SSE exact-scope/privacy proof with no relational fixture path.

use std::time::Duration;

use handshake_core::api;
use handshake_core::api::account_scope::ProductLocalResourceScope;
use handshake_core::console_stream::{
    ConsoleBroadcast, ConsoleCategory, ConsoleEntry, ConsoleEntryDraft, ConsoleSeverity,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, WorkspaceScopeRef,
};

#[tokio::test]
async fn console_replay_hides_every_foreign_scope_dimension_and_unattributed_rows() {
    let hub = ConsoleBroadcast::new(32, 32);
    let owner = exact_scope("owner");
    let owner_marker = "owner-console-secret";
    hub.publish(scoped_draft(owner_marker, owner.clone()));
    let mut foreign_markers = Vec::new();
    for (index, foreign) in one_field_mismatches(&owner).into_iter().enumerate() {
        let marker = format!("foreign-console-secret-{index}");
        hub.publish(scoped_draft(&marker, foreign));
        foreign_markers.push(marker);
    }
    let unattributed = "unattributed-console-secret";
    hub.publish(ConsoleEntryDraft::new(
        ConsoleSeverity::Info,
        ConsoleCategory::System,
        unattributed,
        "must remain system-only",
        None,
    ));

    let response = connect(hub, owner).await;
    let body = collect_stream_text(response, Duration::from_secs(1)).await;
    assert!(body.contains(owner_marker));
    assert!(!body.contains(unattributed));
    for marker in foreign_markers {
        assert!(!body.contains(&marker), "foreign identifier leaked: {body}");
    }
}

#[tokio::test]
async fn console_visible_sse_ids_are_contiguous_across_filtered_foreign_bursts() {
    let hub = ConsoleBroadcast::new(64, 64);
    let owner = exact_scope("contiguous-owner");
    let foreign = one_field_mismatches(&owner)
        .into_iter()
        .next()
        .expect("foreign scope");
    let marker = "visible-owner-console";
    let response = connect(hub.clone(), owner.clone()).await;
    hub.publish(scoped_draft(format!("{marker}-first"), owner.clone()));
    for index in 0..32 {
        hub.publish(scoped_draft(
            format!("hidden-foreign-{marker}-{index}"),
            foreign.clone(),
        ));
    }
    hub.publish(scoped_draft(format!("{marker}-second"), owner));

    let frames = collect_marked_frames(response, marker, 2, Duration::from_secs(3)).await;
    assert_eq!(frames.len(), 2);
    assert_eq!(
        frames.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
        vec!["0", "1"]
    );
    assert_eq!(
        frames
            .iter()
            .map(|(_, entry)| entry.seq)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
}

async fn connect(hub: ConsoleBroadcast, scope: ExactResourceScopeAttribution) -> reqwest::Response {
    let authority = ProductLocalResourceScope::from_exact(scope).expect("exact console scope");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind quiet loopback listener");
    let address = listener.local_addr().expect("loopback address");
    let app = api::console_stream::routes(hub).layer(axum::Extension(authority));
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve console SSE");
    });
    let response = reqwest::Client::new()
        .get(format!("http://{address}/wp1/diagnostics/console/stream"))
        .header("accept", "text/event-stream")
        .send()
        .await
        .expect("connect console SSE");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    response
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

async fn collect_marked_frames(
    mut response: reqwest::Response,
    marker: &str,
    want: usize,
    timeout: Duration,
) -> Vec<(String, ConsoleEntry)> {
    let deadline = tokio::time::Instant::now() + timeout;
    let mut buffer = String::new();
    let mut frames = Vec::new();
    while frames.len() < want && tokio::time::Instant::now() < deadline {
        let chunk = match tokio::time::timeout(Duration::from_millis(250), response.chunk()).await {
            Ok(Ok(Some(bytes))) => bytes,
            Ok(Ok(None)) => break,
            Ok(Err(error)) => panic!("SSE read error: {error}"),
            Err(_) => continue,
        };
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(index) = buffer.find("\n\n") {
            let raw = buffer[..index].to_owned();
            buffer.drain(..index + 2);
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
                    frames.push((id, entry));
                }
            }
        }
    }
    frames
}

fn scoped_draft(
    subject: impl Into<String>,
    scope: ExactResourceScopeAttribution,
) -> ConsoleEntryDraft {
    ConsoleEntryDraft::new(
        ConsoleSeverity::Info,
        ConsoleCategory::System,
        subject,
        "scope privacy proof",
        None,
    )
    .with_resource_scope(scope)
}

fn exact_scope(label: &str) -> ExactResourceScopeAttribution {
    ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new(format!("workspace-console-{label}"))
            .expect("workspace"),
    }
}

fn one_field_mismatches(
    exact: &ExactResourceScopeAttribution,
) -> Vec<ExactResourceScopeAttribution> {
    let mut owner = exact.clone();
    owner.owner_account_id = OwnerAccountId::mint();
    let mut actor = exact.clone();
    actor.actor_principal_id = ActorPrincipalId::mint();
    let mut session = exact.clone();
    session.authenticated_session_id = AuthenticatedSessionRef::mint();
    let mut access = exact.clone();
    access.access_space_id = AccessSpaceRef::mint();
    let mut workspace = exact.clone();
    workspace.workspace_id =
        WorkspaceScopeRef::new("workspace-console-foreign").expect("workspace");
    vec![owner, actor, session, access, workspace]
}
