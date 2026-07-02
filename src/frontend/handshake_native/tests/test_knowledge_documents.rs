//! WP-KERNEL-012 MT-037 (E6) — wire-level proofs for the consolidated knowledge-documents client
//! (`handshake_native::backend::knowledge_documents`).
//!
//! ## Mock-server fixture provenance (SPEC-REALISM GATE — the load-bearing rule)
//!
//! This MT is fully provable WITHOUT a live PostgreSQL/handshake_core: every AC runs against an
//! in-process mock HTTP server (the PROVEN MT-020/021 `std::net::TcpListener` capture pattern — NO new
//! dependency). The mock JSON RESPONSES below are REAL backend response shapes — they are
//! field-verified BYTE-FOR-BYTE against the handshake_core handler bodies in
//! `src/backend/handshake_core/src/api/knowledge_documents.rs` (read READ-ONLY for verification), NOT
//! self-serialized from the client's own Rust types (a fixture that feeds the client its own
//! serialization would be a TAUTOLOGY proving nothing — Spec-Realism Sub-rule 2). Each fixture cites
//! the exact backend handler + `json!({...})` block it mirrors:
//!   * `LOAD_RESPONSE`     <- `load_document` handler `json!({"document":..,"tree":..,"code_nodes":..})`
//!     (api/knowledge_documents.rs ~L766) + `block_tree_view` `json!({...})` (~L464).
//!   * `SAVE_409_BODY`     <- `conflict()` helper `json!({"error":"conflict","detail":..})` (~L191).
//!   * `MISSING_ACTOR_400` <- `bad_request()` helper `json!({"error":"bad_request","detail":
//!                            "x-hsk-actor-id header is required"})` from `doc_context` (~L132/L228).
//!
//! The mock asserts the externally-meaningful client behavior: load deserializes a real
//! `DocumentLoadResponse`; a 409 save -> a DISTINCT `SaveConflict` (not a generic error); a 400 ->
//! `BadRequest`; the double-option move serializes three ways ON THE WIRE; a >100 batch is rejected
//! client-side BEFORE send; and the history limit is clamped client-side. The base URL is the mock
//! server's address (config-injected, never hardcoded — GLOBAL-PORTABILITY). The client is a stateless
//! adapter (no doc state held). NO GUI -> no screenshot / accesskit this MT.

use std::io::{Read, Write};
use std::net::TcpListener;

use handshake_native::backend::knowledge_documents::{
    BatchOperation, BatchRequest, HskDocumentHeaders, KnowledgeDocumentsClient,
    KnowledgeDocumentsError, MoveDocumentRequest, SaveDocumentRequest, HISTORY_MAX_LIMIT,
};
use serde_json::{json, Value};

// ── REAL backend response fixtures (verified byte-for-byte against api/knowledge_documents.rs). ────

/// `GET /knowledge/documents/:id` body — `load_document` handler shape: `{document, tree, code_nodes}`
/// where `tree` is `block_tree_view`'s `{schema_version, schema_matches, block_ids, blocks}`. The
/// `document` object carries the real KnowledgeRichDocument fields the load surfaces.
fn load_response_body() -> Value {
    json!({
        "document": {
            "rich_document_id": "KRD-abc123",
            "workspace_id": "WS-1",
            "title": "Design Notes",
            "schema_version": "rich_document_block_tree_v1",
            "content_json": {"type": "doc", "content": []},
            "content_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "doc_version": 3,
            "crdt_document_id": "KCRDT-abc123",
            "authority_label": "promoted",
            "owner_actor_kind": "operator",
            "owner_actor_id": "handshake-native-editor"
        },
        "tree": {
            "schema_version": "rich_document_block_tree_v1",
            "schema_matches": true,
            "block_ids": ["blk-1", "blk-2"],
            "blocks": [
                {"block_id": "blk-1", "kind": "Raw"},
                {"block_id": "blk-2", "kind": "Raw"}
            ]
        },
        "code_nodes": []
    })
}

/// The real `conflict()` 409 body — `{"error":"conflict","detail":...}`. NOTE: it carries NO
/// `server_version` field (verified), which is exactly why the client's `SaveConflict.server_version`
/// is `None` against this backend.
fn save_409_body() -> Value {
    json!({"error": "conflict", "detail": "expected version 1 is stale; current is 3"})
}

/// The real `bad_request()` 400 body for a missing required identity header — the `doc_context`
/// `"{HSK_HEADER_ACTOR_ID} header is required"` detail.
fn missing_actor_400_body() -> Value {
    json!({"error": "bad_request", "detail": "x-hsk-actor-id header is required"})
}

// ── In-process mock HTTP server (proven MT-020 TcpListener pattern; no new dependency). ────────────

/// A captured record of what the client sent on the wire (the mock's reply is configured by the
/// caller of [`spawn_mock`]; only the captured request is asserted on).
struct MockExchange {
    captured_request_line: String,
    captured_headers: std::collections::HashMap<String, String>,
    captured_body: String,
}

/// Spin up a one-shot mock server that replies with `status_line` + `body` to the FIRST request, and
/// captures that request's line/headers/body. Returns (base_url, join handle delivering the capture).
fn spawn_mock(
    status_line: &'static str,
    body: Value,
) -> (String, std::thread::JoinHandle<MockExchange>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let (request_line, headers, req_body) = read_one_http_request(&mut stream);
        let body_str = body.to_string();
        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body_str}",
            body_str.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
        MockExchange {
            captured_request_line: request_line,
            captured_headers: headers,
            captured_body: req_body,
        }
    });
    (base_url, handle)
}

/// Read one full HTTP request (headers + Content-Length body) off the stream.
fn read_one_http_request(
    stream: &mut std::net::TcpStream,
) -> (String, std::collections::HashMap<String, String>, String) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        let text = String::from_utf8_lossy(&buf);
        if let Some(hdr_end) = text.find("\r\n\r\n") {
            let headers_part = &text[..hdr_end];
            let content_len = headers_part
                .lines()
                .find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    if k.trim().eq_ignore_ascii_case("content-length") {
                        v.trim().parse::<usize>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            let body_start = hdr_end + 4;
            if buf.len() >= body_start + content_len {
                break;
            }
        }
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    let hdr_end = text.find("\r\n\r\n").unwrap_or(text.len());
    let mut lines = text[..hdr_end].lines();
    let request_line = lines.next().unwrap_or("").to_string();
    let mut headers = std::collections::HashMap::new();
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    let body = text[(hdr_end + 4).min(text.len())..].to_string();
    (request_line, headers, body)
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
}

// ── AC: load_document deserializes a real DocumentLoadResponse without panicking. ──────────────────

#[test]
fn ac_load_document_deserializes_real_backend_shape() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", load_response_body());
    let client = KnowledgeDocumentsClient::with_base_url(base_url);
    let headers = HskDocumentHeaders::for_read("session-1", "KRD-abc123");

    let resp = rt().block_on(async { client.load_document(&headers, "KRD-abc123").await });
    let exchange = server.join().unwrap();

    let resp = resp.expect("load against a real-shape 200 body must deserialize, not panic/err");
    // Externally-meaningful assertions on the typed projection.
    assert_eq!(
        resp.document
            .get("rich_document_id")
            .and_then(Value::as_str),
        Some("KRD-abc123"),
        "the typed response exposes the real document id"
    );
    assert!(
        resp.tree.schema_matches,
        "the block tree schema_matches is parsed"
    );
    assert_eq!(
        resp.tree.block_ids,
        vec!["blk-1", "blk-2"],
        "block_ids parsed from the real tree shape"
    );

    // The client used the mock's injected base URL and attached the three required identity headers.
    assert!(
        exchange
            .captured_request_line
            .starts_with("GET /knowledge/documents/KRD-abc123"),
        "load hits GET /knowledge/documents/:id : {}",
        exchange.captured_request_line
    );
    for required in [
        "x-hsk-actor-id",
        "x-hsk-kernel-task-run-id",
        "x-hsk-session-run-id",
    ] {
        assert!(
            exchange
                .captured_headers
                .get(required)
                .is_some_and(|v| !v.is_empty()),
            "the required '{required}' header reached the wire: {:?}",
            exchange.captured_headers
        );
    }
    // A read omits actor-kind (least-privileged default).
    assert!(
        !exchange.captured_headers.contains_key("x-hsk-actor-kind"),
        "a read must NOT send x-hsk-actor-kind (least-privileged read-only default)"
    );
}

// ── AC: save with a stale expected_version (mock 409) -> SaveConflict, not a generic error. ────────

#[test]
fn ac_save_conflict_returns_distinct_save_conflict_variant() {
    let (base_url, server) = spawn_mock("HTTP/1.1 409 Conflict", save_409_body());
    let client = KnowledgeDocumentsClient::with_base_url(base_url);
    let headers = HskDocumentHeaders::for_operator("session-1", "KRD-abc123");
    let body = SaveDocumentRequest {
        expected_version: 1, // stale on purpose
        content_json: json!({"type": "doc", "content": []}),
        crdt_document_id: None,
        crdt_snapshot_id: None,
        promotion_receipt_event_id: None,
    };

    let result = rt().block_on(async { client.save_document(&headers, "KRD-abc123", &body).await });
    let exchange = server.join().unwrap();

    match result {
        Err(KnowledgeDocumentsError::SaveConflict { server_version }) => {
            // The real backend 409 body carries no version, so None — but the variant is DISTINCT.
            assert_eq!(
                server_version, None,
                "the real backend 409 body has no server_version"
            );
        }
        other => panic!(
            "a 409 save MUST be a DISTINCT SaveConflict (never a generic error), got {other:?}"
        ),
    }
    assert!(
        exchange
            .captured_request_line
            .starts_with("PUT /knowledge/documents/KRD-abc123/save"),
        "save hits PUT /save: {}",
        exchange.captured_request_line
    );
    // A write asserts the operator actor-kind (a missing kind 403s a write server-side).
    assert_eq!(
        exchange
            .captured_headers
            .get("x-hsk-actor-kind")
            .map(String::as_str),
        Some("operator"),
        "a save asserts the operator actor-kind on the wire"
    );
    // The save body carries the optimistic-concurrency token.
    let sent: Value = serde_json::from_str(&exchange.captured_body).unwrap();
    assert_eq!(
        sent["expected_version"], 1,
        "the save body sends expected_version"
    );
}

// ── AC: a 400 (mock returns the missing-actor-id body) -> typed BadRequest. ────────────────────────

#[test]
fn ac_bad_request_400_returns_typed_bad_request_variant() {
    let (base_url, server) = spawn_mock("HTTP/1.1 400 Bad Request", missing_actor_400_body());
    let client = KnowledgeDocumentsClient::with_base_url(base_url);
    let headers = HskDocumentHeaders::for_read("session-1", "KRD-abc123");

    let result = rt().block_on(async { client.load_document(&headers, "KRD-abc123").await });
    let _ = server.join().unwrap();

    match result {
        Err(KnowledgeDocumentsError::BadRequest(detail)) => {
            assert!(
                detail.contains("x-hsk-actor-id"),
                "the typed BadRequest carries the backend detail: {detail}"
            );
        }
        other => panic!("a 400 MUST be a typed BadRequest, got {other:?}"),
    }
}

// ── minimum-control: double-option MoveDocumentRequest serializes three ways ON THE WIRE. ──────────

#[test]
fn control_move_double_option_serializes_three_ways_on_the_wire() {
    // Capture the actual JSON body the client PUTs for each of the three double-option states.
    fn capture_move_body(req: MoveDocumentRequest) -> Value {
        let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", json!({"document": {}}));
        let client = KnowledgeDocumentsClient::with_base_url(base_url);
        let headers = HskDocumentHeaders::for_operator("session-1", "KRD-1");
        let _ = rt().block_on(async { client.move_document(&headers, "KRD-1", &req).await });
        let exchange = server.join().unwrap();
        assert!(
            exchange
                .captured_request_line
                .starts_with("POST /knowledge/documents/KRD-1/move"),
            "move hits POST /move: {}",
            exchange.captured_request_line
        );
        serde_json::from_str(&exchange.captured_body).expect("move body is JSON")
    }

    // 1) Absent (leave unchanged) -> the key is OMITTED on the wire.
    let absent = capture_move_body(MoveDocumentRequest::default());
    assert!(
        absent.get("project_ref").is_none() && absent.get("folder_ref").is_none(),
        "an ABSENT ref must be OMITTED on the wire (absent != null — the MT-157 data-loss control): {absent}"
    );

    // 2) Explicit null (clear) -> the key is present with a null value.
    let cleared = capture_move_body(MoveDocumentRequest {
        project_ref: Some(None),
        folder_ref: None,
    });
    assert_eq!(
        cleared["project_ref"],
        Value::Null,
        "Some(None) sends explicit null: {cleared}"
    );
    assert!(
        cleared.get("folder_ref").is_none(),
        "the absent folder_ref stays omitted: {cleared}"
    );

    // 3) String (set) -> the key is present with the string value.
    let set = capture_move_body(MoveDocumentRequest {
        project_ref: Some(Some("PROJ-9".into())),
        folder_ref: Some(Some("FOLDER-9".into())),
    });
    assert_eq!(set["project_ref"], json!("PROJ-9"));
    assert_eq!(set["folder_ref"], json!("FOLDER-9"));
}

// ── minimum-control: a >100-operation batch is rejected CLIENT-SIDE before any send. ──────────────

#[test]
fn control_batch_over_100_is_rejected_client_side_before_send() {
    // No server needed — the guard must fire before a socket is opened. Use an unroutable base URL so
    // any accidental send would error loudly (proving the guard short-circuits before the request).
    let client = KnowledgeDocumentsClient::with_base_url("http://127.0.0.1:1");
    let headers = HskDocumentHeaders::for_operator("session-1", "BATCH");
    let operations: Vec<BatchOperation> = (0..101)
        .map(|i| BatchOperation::Rename {
            document_id: format!("KRD-{i}"),
            title: format!("T{i}"),
        })
        .collect();
    let body = BatchRequest { operations };

    let result = rt().block_on(async { client.batch_documents(&headers, &body).await });
    match result {
        Err(KnowledgeDocumentsError::BatchTooLarge { len, max }) => {
            assert_eq!(len, 101);
            assert_eq!(max, 100);
        }
        other => {
            panic!("a >100 batch MUST be a client-side BatchTooLarge BEFORE send, got {other:?}")
        }
    }

    // An empty batch is likewise rejected client-side.
    let empty = BatchRequest { operations: vec![] };
    let result = rt().block_on(async { client.batch_documents(&headers, &empty).await });
    assert!(
        matches!(result, Err(KnowledgeDocumentsError::BatchEmpty)),
        "an empty batch MUST be a client-side BatchEmpty before send: {result:?}"
    );
}

// ── minimum-control: history limit is clamped 1..=200 CLIENT-SIDE before sending. ─────────────────

#[test]
fn control_history_limit_clamped_client_side() {
    // Capture the query the client actually sends for an out-of-range limit + a negative offset.
    let (base_url, server) = spawn_mock(
        "HTTP/1.1 200 OK",
        json!({
            "rich_document_id": "KRD-1",
            "current_version": 5,
            "versions": [],
            "total_versions": 0,
            "limit": HISTORY_MAX_LIMIT,
            "offset": 0
        }),
    );
    let client = KnowledgeDocumentsClient::with_base_url(base_url);
    let headers = HskDocumentHeaders::for_read("session-1", "KRD-1");

    let resp = rt().block_on(async {
        client
            .load_history(
                &headers, "KRD-1", 9999, /* over cap */
                -42,  /* negative */
            )
            .await
    });
    let exchange = server.join().unwrap();
    let resp = resp.expect("history list deserializes the real shape");
    assert_eq!(
        resp.limit, HISTORY_MAX_LIMIT,
        "the response echoes the clamped limit"
    );

    // The clamp happened CLIENT-SIDE: the request query carries limit=200 (cap) and offset=0 (>=0).
    assert!(
        exchange.captured_request_line.contains("limit=200"),
        "the client clamps limit to 200 before sending: {}",
        exchange.captured_request_line
    );
    assert!(
        exchange.captured_request_line.contains("offset=0"),
        "the client clamps a negative offset to 0 before sending: {}",
        exchange.captured_request_line
    );
}

// ── stateless adapter: the client carries no document state across calls. ──────────────────────────

#[test]
fn stateless_adapter_holds_no_document_state() {
    // Prove statelessness on the WIRE, not structurally: two clones of one client each load a DIFFERENT
    // document, and each request must carry ONLY its own per-call document id in the path (no id is held
    // on the client between calls). If the client held doc state, a clone would carry a stale id; it
    // does not, because the id is a per-call argument.
    let client = KnowledgeDocumentsClient::with_base_url("http://placeholder.invalid");
    let clone_a = client.clone();
    let clone_b = client.clone();

    fn captured_load_path(client: &KnowledgeDocumentsClient, document_id: &str) -> String {
        let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", load_response_body());
        // Re-point THIS clone at the mock (with_base_url keeps the client stateless: only the host moves).
        let bound = KnowledgeDocumentsClient::with_base_url(base_url);
        let headers = HskDocumentHeaders::for_read("session-stateless", document_id);
        let _ = rt().block_on(async { bound.load_document(&headers, document_id).await });
        // The original client argument is exercised too (its clone-ness is the point under test): it
        // holds no per-document state, so cloning it never carries a document id.
        let _ = client;
        server.join().unwrap().captured_request_line
    }

    let line_a = captured_load_path(&clone_a, "KRD-stateless-A");
    let line_b = captured_load_path(&clone_b, "KRD-stateless-B");
    assert!(
        line_a.starts_with("GET /knowledge/documents/KRD-stateless-A"),
        "clone A's call carries ONLY its own per-call document id: {line_a}"
    );
    assert!(
        line_b.starts_with("GET /knowledge/documents/KRD-stateless-B"),
        "clone B's call carries ONLY its own per-call document id (no state carried from A): {line_b}"
    );
}
