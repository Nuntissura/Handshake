//! WP-KERNEL-012 MT-111 — the frontend Flight Recorder client against the MT-109 authorization
//! boundary, proven end-to-end on a real `handshake_core` + real SurrealDB.
//!
//! MT-109 put fail-closed capability middleware over the WHOLE flight-recorder route group, removed the
//! unauthenticated unscoped ingestion routes, and made `actor_id` / `actor_kind` / `workspace_id`
//! server-derived rather than client-supplied. `src/frontend/**` was a forbidden path for MT-109, so its
//! callers were left broken. This suite proves the repaired client, and proves it WITHOUT weakening,
//! bypassing, feature-gating, or stubbing any part of that boundary — every request here presents the
//! same real on-disk native-MCP binding credential a real client presents.
//!
//! | Acceptance criterion | Proof |
//! |---|---|
//! | AC-111-1 (scoped ingest path) | `mt111_flight_recorder_authorization_boundary_real_surrealdb` step 6 posts through the production emitter to `POST /api/workspaces/{id}/flight_recorder/native_editor_event` and the row lands. |
//! | AC-111-2 (genuine credential + typed absence) | step 1 (401 without the header), step 7 (`EmitError::MissingSessionBinding` when the binding is gone — surfaced in the operator-visible error ring, never a silent drop). |
//! | AC-111-3 (no client-authored identity) | step 2 (`403 HSK-403-FR-ACTOR-SPOOF` and NO durable row), step 3 (`403 HSK-403-FR-WORKSPACE` and NO durable row), step 6 (the persisted row carries SERVER-derived attribution). |
//! | AC-111-4 (scoped read) | step 4 (unscoped read with a valid credential is `403 HSK-403-FR-CAPABILITY`), step 5 (scoped read without a credential is `401`). |
//! | AC-111-5 (live runtime proof) | the whole test: real backend, real SurrealDB, real workspace, production emitter. |
//! | AC-111-7 (honest harness) | every read here uses [`live_binding_session_token`], read from the REAL published binding. |
//!
//! ## Running it
//!
//! This is a live proof. It requires an explicitly built product backend. It no longer requires a
//! database to be running: Handshake's store is embedded in the backend, so the harness scopes the
//! run with a data directory and the backend opens its own store inside it.
//!
//! ```text
//! CARGO_TARGET_DIR=<scoped>            # per-owner scoped cargo target (CX-984)
//! HSK_TEST_BACKEND_BIN=<scoped>/debug/handshake_core.exe
//! HANDSHAKE_DATA_DIR=<scoped>/backend-runtime   # the run's isolated embedded store root
//! HANDSHAKE_TEST_STAGE_BINDING_ROOT=<short absolute isolated root>   # forces an OWNED backend child
//! cargo test --test test_flight_recorder_authz -- --nocapture
//! ```
//!
//! `HANDSHAKE_TEST_STAGE_BINDING_ROOT` is mandatory: it both forces `backend_proof_support` to own its
//! backend child and gives this proof a private app-data root. The child inherits the redirected
//! `%LOCALAPPDATA%`, so BOTH processes resolve the SAME `swarm_mcp_binding.json` — which is what makes
//! the credential real rather than mocked.

#[path = "backend_proof_support/mod.rs"]
mod backend_proof_support;

use handshake_native::event_emitter::{
    EmitError, NativeEditorEvent, NativeEditorEventEmitter, UndoScope, HSK_HEADER_SESSION_TOKEN,
    NATIVE_EDITOR_SCHEMA_VERSION,
};

/// AC-111-7: read the token from the REAL published binding, exactly as the mounted native client
/// does. This never weakens the gate - a missing, forged, or stale binding still fails closed.
use backend_proof_support::{live_flight_recorder_session_token as live_binding_session_token, RealNativeMcpBinding, NATIVE_BINDING_APP_DATA_ENV as APP_DATA_ENV};

fn native_editor_body(event_id: &str, extra: serde_json::Value) -> serde_json::Value {
    let mut body = serde_json::json!({
        "schema_version": NATIVE_EDITOR_SCHEMA_VERSION,
        "event_id": event_id,
        "ts_utc": chrono::Utc::now().to_rfc3339(),
        "kind": "undo_fired",
        "pane_id": "pane-mt111",
        "surface": "pane-mt111",
        "session_id": uuid::Uuid::new_v4().to_string(),
        "work_packet_id": "WP-KERNEL-012",
        "payload": {"scope": "local"},
    });
    if let (Some(object), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            object.insert(key.clone(), value.clone());
        }
    }
    body
}

/// One live test so the shared managed-backend fixture lock is taken exactly once. Every step is a
/// distinct MT-111 acceptance obligation and is labelled as such.
#[test]
fn mt111_flight_recorder_authorization_boundary_real_surrealdb() {
    // The app-data redirect must be installed BEFORE the backend child is spawned so the child
    // inherits it and both processes resolve the same binding file.
    let binding = RealNativeMcpBinding::publish();
    let mut backend = backend_proof_support::require_reachable_backend();
    let base = backend.base.clone();
    let workspace = backend.create_workspace(&format!("mt111-fr-authz-{}", uuid::Uuid::new_v4()));
    let workspace_id = workspace
        .get("id")
        .or_else(|| workspace.pointer("/workspace/id"))
        .and_then(serde_json::Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| panic!("POST /workspaces response lacks id: {workspace}"))
        .to_owned();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("MT-111 proof runtime");
    let http = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("bounded MT-111 HTTP client");
    let ingest_url =
        format!("{base}/api/workspaces/{workspace_id}/flight_recorder/native_editor_event");


    // ── Step 1 (AC-111-2 negative): no credential -> 401 HSK-401-FR-SESSION, no durable write ──────
    let unauthenticated_event_id = uuid::Uuid::new_v4().to_string();
    let (status, body) = runtime.block_on(async {
        let response = http
            .post(&ingest_url)
            .json(&native_editor_body(&unauthenticated_event_id, json_null()))
            .send()
            .await
            .expect("unauthenticated ingest POST reaches the backend");
        let status = response.status().as_u16();
        let body: serde_json::Value = response.json().await.unwrap_or(serde_json::Value::Null);
        (status, body)
    });
    assert_eq!(status, 401, "AC-111-2: no session token is fail-closed");
    assert_eq!(
        body["error"], "HSK-401-FR-SESSION",
        "AC-111-2: the exact MT-109 typed error contract"
    );

    // ── Step 2 (AC-111-3 negative): a deliberately spoofed actor_id -> 403 before any durable write ─
    let spoofed_event_id = uuid::Uuid::new_v4().to_string();
    let (status, body) = runtime.block_on(async {
        let response = http
            .post(&ingest_url)
            .header(HSK_HEADER_SESSION_TOKEN, binding.token())
            .json(&native_editor_body(
                &spoofed_event_id,
                serde_json::json!({"actor_id": "hsk:native_editor:human"}),
            ))
            .send()
            .await
            .expect("actor-spoof ingest POST reaches the backend");
        let status = response.status().as_u16();
        let body: serde_json::Value = response.json().await.unwrap_or(serde_json::Value::Null);
        (status, body)
    });
    assert_eq!(
        status, 403,
        "AC-111-3: a client-authored actor_id that disagrees with the authenticated context is denied"
    );
    assert_eq!(body["error"], "HSK-403-FR-ACTOR-SPOOF");

    // ── Step 3 (AC-111-3 negative): a body workspace_id that disagrees with the path -> 403 ────────
    let cross_workspace_event_id = uuid::Uuid::new_v4().to_string();
    let (status, body) = runtime.block_on(async {
        let response = http
            .post(&ingest_url)
            .header(HSK_HEADER_SESSION_TOKEN, binding.token())
            .json(&native_editor_body(
                &cross_workspace_event_id,
                serde_json::json!({"workspace_id": "some-other-workspace"}),
            ))
            .send()
            .await
            .expect("cross-workspace ingest POST reaches the backend");
        let status = response.status().as_u16();
        let body: serde_json::Value = response.json().await.unwrap_or(serde_json::Value::Null);
        (status, body)
    });
    assert_eq!(
        status, 403,
        "AC-111-3: the path segment is the workspace authority; a body field can never widen it"
    );
    assert_eq!(body["error"], "HSK-403-FR-WORKSPACE");

    // ── Step 4 (AC-111-4): an UNSCOPED read, even with a valid credential, requires fr.read.global
    //    which is granted to NO profile -> always 403. This is exactly why the shell must scope. ────
    let (status, body) = runtime.block_on(async {
        let response = http
            .get(format!("{base}/api/flight_recorder"))
            .header(HSK_HEADER_SESSION_TOKEN, binding.token())
            .send()
            .await
            .expect("unscoped recorder read reaches the backend");
        let status = response.status().as_u16();
        let body: serde_json::Value = response.json().await.unwrap_or(serde_json::Value::Null);
        (status, body)
    });
    assert_eq!(
        status, 403,
        "AC-111-4: an unscoped read escalates to fr.read.global, which no profile holds"
    );
    assert_eq!(body["error"], "HSK-403-FR-CAPABILITY");

    // ── Step 5 (AC-111-2 negative, read side): a scoped read without a credential -> 401 ───────────
    let status = runtime.block_on(async {
        http.get(format!("{base}/api/flight_recorder"))
            .query(&[("wsid", workspace_id.as_str())])
            .send()
            .await
            .expect("unauthenticated recorder read reaches the backend")
            .status()
            .as_u16()
    });
    assert_eq!(
        status, 401,
        "AC-111-2: the recorder read is authenticated, not anonymous"
    );

    // ── Step 6 (AC-111-1/2/3/5 positive): the PRODUCTION emitter lands a durable row with
    //    SERVER-derived attribution, read back through the scoped path. ───────────────────────────
    let emitter =
        NativeEditorEventEmitter::production(workspace_id.clone(), base.clone(), runtime.handle().clone());
    let persisted = runtime.block_on(async {
        emitter
            .emit_persisted(
                NativeEditorEvent::undo_fired(
                    UndoScope::Local,
                    "pane-mt111",
                    "caller-supplied-actor-that-is-not-authority",
                    workspace_id.clone(),
                ),
                std::time::Duration::from_secs(20),
            )
            .await
    });
    let persisted = persisted.unwrap_or_else(|error| {
        panic!(
            "AC-111-2: the production transport must authenticate with the live binding, but the \
             emit failed: {error}"
        )
    });

    let row = poll_scoped_recorder_row(&runtime, &http, &base, &workspace_id, &persisted.event_id);
    let server_actor_id = row["actor_id"]
        .as_str()
        .expect("persisted recorder row carries an actor_id")
        .to_owned();
    assert!(
        server_actor_id.starts_with("handshake-native:"),
        "AC-111-3: attribution must be SERVER-derived from the authenticated binding, got \
         {server_actor_id}"
    );
    assert_ne!(
        server_actor_id, "caller-supplied-actor-that-is-not-authority",
        "AC-111-3: the caller's actor must never become the durable attribution"
    );
    assert_ne!(
        server_actor_id,
        handshake_native::event_emitter::DEFAULT_ACTOR_ID,
        "AC-111-3: even the emitter's own default actor is not authority any more"
    );
    assert_eq!(
        row["payload"]["workspace_id"].as_str(),
        Some(workspace_id.as_str()),
        "AC-111-1: the durable row is bound to the workspace named by the route path"
    );
    assert_eq!(row["payload"]["action"], "undo_fired");

    // The denied attempts in steps 1-3 must have produced NO durable rows.
    for denied in [
        &unauthenticated_event_id,
        &spoofed_event_id,
        &cross_workspace_event_id,
    ] {
        let rows = runtime.block_on(async {
            let response = http
                .get(format!("{base}/api/flight_recorder"))
                .header(HSK_HEADER_SESSION_TOKEN, binding.token())
                .query(&[("wsid", workspace_id.as_str())])
                .send()
                .await
                .expect("scoped recorder read reaches the backend");
            assert!(response.status().is_success());
            response
                .json::<Vec<serde_json::Value>>()
                .await
                .expect("scoped recorder read returns a JSON array")
        });
        assert!(
            !rows
                .iter()
                .any(|row| row["payload"]["client_event_id"].as_str() == Some(denied.as_str())),
            "a denied request must not have written anything durable ({denied})"
        );
    }

    // ── Step 7 (AC-111-2): with the binding GONE the emit is a TYPED, operator-visible failure that
    //    names the exact credential file — never a silent drop, and never an unauthenticated retry. ─
    let missing_root = std::env::temp_dir().join(format!("hsk-mt111-nobinding-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&missing_root).expect("create empty app-data root");
    let previous_app_data = std::env::var_os(APP_DATA_ENV);
    std::env::set_var(APP_DATA_ENV, &missing_root);
    let unbound_emitter =
        NativeEditorEventEmitter::production(workspace_id.clone(), base.clone(), runtime.handle().clone());
    let unbound = runtime.block_on(async {
        unbound_emitter
            .emit_persisted(
                NativeEditorEvent::undo_fired(
                    UndoScope::Local,
                    "pane-mt111",
                    "actor",
                    workspace_id.clone(),
                ),
                std::time::Duration::from_secs(20),
            )
            .await
    });
    match previous_app_data {
        Some(previous) => std::env::set_var(APP_DATA_ENV, previous),
        None => std::env::remove_var(APP_DATA_ENV),
    }
    let _ = std::fs::remove_dir_all(&missing_root);
    match unbound {
        Err(EmitError::MissingSessionBinding { path, reason }) => {
            assert!(
                path.contains("swarm_mcp_binding.json"),
                "the typed failure names the exact credential file, got {path}"
            );
            assert!(!reason.trim().is_empty(), "the typed failure carries a reason");
        }
        other => panic!(
            "AC-111-2: a missing native-MCP binding must surface as a typed MissingSessionBinding \
             error, got {other:?}"
        ),
    }
    assert!(
        unbound_emitter
            .error_ring()
            .entries()
            .iter()
            .any(|entry| matches!(entry.error, EmitError::MissingSessionBinding { .. })),
        "AC-111-2: the failure must be visible to the operator in the shared error ring the \
         FlightRecorderPane renders"
    );

    // Product workspace deletion owns scoped projection cleanup. The fixture then reaps its isolated
    // embedded SurrealDB root, so no out-of-band EventLedger mutation is needed or permitted.
    let delete_status = backend.delete_workspace(&workspace_id);
    assert!(
        (200..300).contains(&delete_status) || delete_status == 404,
        "the proof deletes its own workspace, got {delete_status}"
    );
    backend.assert_cleanup();
    drop(binding);
}

fn json_null() -> serde_json::Value {
    serde_json::Value::Null
}

/// Poll the SCOPED read path (with the real credential) until the durable row for `event_id` lands.
fn poll_scoped_recorder_row(
    runtime: &tokio::runtime::Runtime,
    http: &reqwest::Client,
    base: &str,
    workspace_id: &str,
    event_id: &str,
) -> serde_json::Value {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    loop {
        let rows = runtime.block_on(async {
            let response = http
                .get(format!("{base}/api/flight_recorder"))
                .header(HSK_HEADER_SESSION_TOKEN, live_binding_session_token())
                .query(&[("wsid", workspace_id)])
                .send()
                .await
                .expect("scoped recorder read reaches the backend");
            assert!(
                response.status().is_success(),
                "AC-111-4: the scoped read with a live credential must succeed, got {}",
                response.status()
            );
            response
                .json::<Vec<serde_json::Value>>()
                .await
                .expect("scoped recorder read returns a JSON array")
        });
        if let Some(row) = rows
            .iter()
            .find(|row| row["payload"]["client_event_id"].as_str() == Some(event_id))
        {
            return row.clone();
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the production-emitted native-editor row for {event_id} did not arrive within 20s"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
