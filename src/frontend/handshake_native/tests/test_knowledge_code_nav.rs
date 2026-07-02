//! WP-KERNEL-012 MT-039 (E6 — backend reuse wiring): wire-level proofs for the typed `/knowledge/code/*`
//! client `handshake_native::backend::knowledge_code_nav`.
//!
//! These tests round-trip the client against an IN-PROCESS mock HTTP server using the PROVEN MT-020/037
//! `std::net::TcpListener` capture pattern (NO new dependency, no wiremock/httpmock). The mock JSON
//! RESPONSES below are REAL backend response shapes — every field was VERIFIED READ-ONLY against the
//! running backend source `src/backend/handshake_core/src/api/knowledge_code_nav.rs` handler `json!`
//! blocks (and `knowledge_code_index/{monaco_bridge,staleness}.rs` for the file_lens payload), NOT
//! self-serialized from the client structs (which would be a tautology) and NOT taken from the MT-039
//! contract prose (whose `FileLensEntry` listed fields the real `CodeLensEntry` does not have).
//!
//! What is proven (each maps to an MT-039 acceptance_criterion / proof_target):
//!   * AC: lookup_symbols(name="Foo") against a real-shape 200 -> typed SymbolLookupResponse with
//!     `matches` populated, the required identity headers on the wire, and the receipt ids preserved.
//!   * AC: lookup_symbols with NO name/prefix/path -> EmptyLookup BEFORE any HTTP request (the mock is
//!     never contacted).
//!   * AC: file_lens with a path containing '/' -> the OUTGOING request line contains %2F, not a bare /.
//!   * AC + RISK-1: file_lens with an absolute / '..' path -> UnsafePath BEFORE any HTTP request.
//!   * AC: symbol_references against a real-shape 200 with callers AND callees -> both lists populated.
//!   * AC + RISK-3: StalenessState deserializes the fresh shape, the unindexed shape, AND an unknown
//!     future state without error (covered in the lib unit tests; re-asserted here on a real nav body).
//!   * AC: nav_receipt_event_id + quiet_background_work_receipt_id preserved through deserialization on
//!     every response struct.

use std::io::{Read, Write};
use std::net::TcpListener;

use handshake_native::backend::knowledge_code_nav::{
    code_nav_headers, CodeNavError, EditorSessionContext, KnowledgeCodeNavClient, LookupRequest,
};
#[allow(unused_imports)]
use handshake_native::backend::knowledge_code_nav::{SymbolSpan, SymbolSpansResponse};
use serde_json::{json, Value};

// ── In-process mock HTTP server (proven MT-020/037 TcpListener pattern; no new dependency). ─────────

/// A captured record of what the client sent on the wire.
struct MockExchange {
    captured_request_line: String,
    captured_headers: std::collections::HashMap<String, String>,
}

/// Spin up a one-shot mock server that replies with `status_line` + `body` to the FIRST request, and
/// captures that request's line + headers. Returns (base_url, join handle delivering the capture).
fn spawn_mock(
    status_line: &'static str,
    body: Value,
) -> (String, std::thread::JoinHandle<MockExchange>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let (request_line, headers) = read_one_http_request(&mut stream);
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
        }
    });
    (base_url, handle)
}

/// Read one full HTTP request (header block; these are all GETs with no body) off the stream.
fn read_one_http_request(
    stream: &mut std::net::TcpStream,
) -> (String, std::collections::HashMap<String, String>) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if String::from_utf8_lossy(&buf).contains("\r\n\r\n") {
            break;
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
    (request_line, headers)
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
}

// ── REAL backend response-shape fixtures (verified against api/knowledge_code_nav.rs json! blocks). ─

/// The `lookup_symbols` 200 body (`lookup_symbols:488-493` wrapping `symbol_to_json:387-397`). The
/// `staleness` object is the `served_staleness` "fresh" shape (`:339-342`).
fn lookup_response_body() -> Value {
    json!({
        "workspace_id": "ws-1",
        "matches": [
            {
                "symbol_entity_id": "KE-11111111-2222-3333-4444-555555555555",
                "symbol_key": "rust:src/lib.rs#Foo",
                "display_name": "Foo",
                "symbol_kind": "struct",
                "owning_wp": "WP-KERNEL-009",
                "primary_source_id": "file:src/lib.rs",
                "lifecycle_state": "active",
                "definition": {
                    "span_id": "KSP-aaa",
                    "source_id": "file:src/lib.rs",
                    "line_start": 10,
                    "line_end": 20,
                    "range_start": 100,
                    "range_end": 200,
                    "section_path": null
                },
                "staleness": {
                    "state": "fresh", "fresh": true,
                    "indexed_content_hash": "sha256:abc",
                    "indexed_parser_version": "rust@0.25"
                }
            }
        ],
        "nav_receipt_event_id": "KEV-nav-001",
        "quiet_background_work_receipt_id": "QBW-quiet-001"
    })
}

/// The `symbol_references` 200 body with BOTH a caller and a callee (`symbol_references:577-584`,
/// caller `:540-547`, callee `:553-560`, evidence span `:782-786`). The callee carries the "unindexed"
/// staleness shape (`served_staleness:346-347`) to prove the tolerant deserialization on a real nav body.
fn references_response_body() -> Value {
    json!({
        "symbol_entity_id": "KE-target",
        "staleness": {"state": "fresh", "fresh": true,
            "indexed_content_hash": "sha256:t", "indexed_parser_version": "rust@0.25"},
        "callers": [
            {
                "symbol_entity_id": "KE-caller",
                "symbol_key": "rust:src/a.rs#call_target",
                "display_name": "call_target",
                "confidence": 0.92,
                "evidence_spans": [{"span_id": "KSP-c1", "line_start": 5, "line_end": 5}],
                "staleness": {"state": "fresh", "fresh": true,
                    "indexed_content_hash": "sha256:a", "indexed_parser_version": "rust@0.25"}
            }
        ],
        "callees": [
            {
                "symbol_entity_id": "KE-callee",
                "symbol_key": "rust:src/b.rs#helper",
                "display_name": "helper",
                "confidence": 0.5,
                "evidence_spans": [],
                "staleness": {"state": "unindexed", "fresh": false,
                    "detail": "no code-file index state for source"}
            }
        ],
        "nav_receipt_event_id": "KEV-nav-refs",
        "quiet_background_work_receipt_id": "QBW-quiet-refs"
    })
}

/// The `file_lens` 200 body: the serialized `MonacoCodeLensPayload` (`monaco_bridge.rs:59-69`) with the
/// two receipt ids inserted (`file_lens:743-749`). `staleness` is the backend `StalenessVerdict` tagged
/// "fresh" shape (`staleness.rs:#[serde(tag="state")]` -> `{"state":"fresh"}`, NO bool field). The single
/// entry is the REAL `CodeLensEntry` (`monaco_bridge.rs:37-55`: definition LineRange + references + doc +
/// caller_count — NO callee_count/test_count/per-entry staleness).
fn file_lens_response_body() -> Value {
    json!({
        "workspace_id": "ws-1",
        "relative_path": "src/lib.rs",
        "staleness": {"state": "fresh"},
        "truncated": false,
        "entries": [
            {
                "symbol_entity_id": "KEN-lens-1",
                "symbol_key": "rust:src/lib.rs#Foo",
                "display_name": "Foo",
                "symbol_kind": "struct",
                "definition": {"start_line": 10, "end_line": 20},
                "references": [{"start_line": 30, "end_line": 30}, {"start_line": 41, "end_line": 42}],
                "doc": "A Foo.",
                "caller_count": 3
            }
        ],
        "nav_receipt_event_id": "KEV-nav-lens",
        "quiet_background_work_receipt_id": "QBW-quiet-lens"
    })
}

fn ctx() -> EditorSessionContext {
    EditorSessionContext::for_native_editor("session-mt039")
}

// ── AC: lookup_symbols(name) deserializes a real SymbolLookupResponse with matches populated. ───────

#[test]
fn ac_lookup_symbols_by_name_returns_typed_response_with_matches() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", lookup_response_body());
    let client = KnowledgeCodeNavClient::with_base_url(base_url);
    let headers = code_nav_headers(&ctx());

    let resp = rt().block_on(async {
        client
            .lookup_symbols(&headers, &LookupRequest::by_name("ws-1", "Foo"))
            .await
    });
    let exchange = server.join().unwrap();

    let resp = resp.expect("lookup against a real-shape 200 must deserialize, not panic/err");
    assert_eq!(resp.workspace_id, "ws-1");
    assert_eq!(
        resp.matches.len(),
        1,
        "matches populated from the real lookup body"
    );
    let m = &resp.matches[0];
    assert_eq!(
        m.symbol_entity_id,
        "KE-11111111-2222-3333-4444-555555555555"
    );
    assert_eq!(m.symbol_key, "rust:src/lib.rs#Foo");
    assert_eq!(m.display_name, "Foo");
    assert!(
        m.staleness.is_fresh(),
        "the fresh staleness shape parses as fresh"
    );
    let def = m.definition.as_ref().expect("definition span parsed");
    assert_eq!(def.line_start, Some(10));
    assert_eq!(def.line_end, Some(20));
    // AC: receipt ids preserved through deserialization (RISK-4/MC-4).
    assert_eq!(resp.nav_receipt_event_id, "KEV-nav-001");
    assert_eq!(resp.quiet_background_work_receipt_id, "QBW-quiet-001");

    // The request hit the right route with name + workspace_id, and the required identity headers.
    assert!(
        exchange
            .captured_request_line
            .starts_with("GET /knowledge/code/symbols?"),
        "lookup hits GET /knowledge/code/symbols : {}",
        exchange.captured_request_line
    );
    assert!(
        exchange.captured_request_line.contains("name=Foo")
            && exchange.captured_request_line.contains("workspace_id=ws-1"),
        "lookup query carries workspace_id + name : {}",
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
    // A nav asserts the `system` actor-kind (the verified-valid nav kind).
    assert_eq!(
        exchange
            .captured_headers
            .get("x-hsk-actor-kind")
            .map(String::as_str),
        Some("system"),
        "a nav asserts the system actor-kind on the wire"
    );
}

// ── AC: lookup_symbols with NO name/prefix/path -> EmptyLookup BEFORE any HTTP request. ─────────────

#[test]
fn ac_empty_lookup_errors_before_sending_any_request() {
    // Bind a listener but NEVER accept — if the client sent a request this test would hang on join; it
    // does not, because the guard fires first and the mock thread is simply dropped.
    let client = KnowledgeCodeNavClient::with_base_url("http://127.0.0.1:1");
    let headers = code_nav_headers(&ctx());
    let empty = LookupRequest {
        workspace_id: "ws-1".into(),
        ..Default::default()
    };

    let result = rt().block_on(async { client.lookup_symbols(&headers, &empty).await });
    assert_eq!(
        result.err(),
        Some(CodeNavError::EmptyLookup),
        "a no-filter lookup is a client-side EmptyLookup, not a backend-400 round-trip (RISK-2/MC-2)"
    );
}

// ── AC: file_lens with a '/' in the path -> the OUTGOING request line contains %2F (not bare /). ────

#[test]
fn ac_file_lens_url_encodes_path_slash_as_pct_2f() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", file_lens_response_body());
    let client = KnowledgeCodeNavClient::with_base_url(base_url);
    let headers = code_nav_headers(&ctx());

    let resp = rt().block_on(async {
        client
            .file_lens(&headers, "src/lib.rs", "ws-1", "sha256:live", "rust@0.25")
            .await
    });
    let exchange = server.join().unwrap();

    let resp = resp.expect("file_lens against a real-shape 200 must deserialize");
    assert_eq!(resp.relative_path, "src/lib.rs");
    assert_eq!(resp.entries.len(), 1, "the real CodeLensEntry parses");
    let e = &resp.entries[0];
    assert_eq!(e.symbol_key, "rust:src/lib.rs#Foo");
    assert_eq!(e.definition.start_line, 10);
    assert_eq!(e.references.len(), 2, "in-file reference ranges parse");
    assert_eq!(e.caller_count, 3);
    assert!(
        resp.staleness.is_fresh(),
        "the StalenessVerdict 'fresh' (no bool) parses as fresh"
    );
    assert!(!resp.truncated);
    assert_eq!(resp.nav_receipt_event_id, "KEV-nav-lens");
    assert_eq!(resp.quiet_background_work_receipt_id, "QBW-quiet-lens");

    // THE load-bearing RISK-1/MC-1 proof: the path segment is %2F-encoded on the WIRE (a bare '/' would
    // be mis-parsed by axum's path extractor into multiple segments -> 404).
    let line = &exchange.captured_request_line;
    assert!(
        line.contains("/knowledge/code/files/src%2Flib.rs/lens"),
        "the outgoing path segment %2F-encodes the embedded '/': {line}"
    );
    assert!(
        !line.contains("/knowledge/code/files/src/lib.rs/lens"),
        "the outgoing path segment must NOT contain a bare '/' inside the file path: {line}"
    );
    // The lens query carries workspace_id + content_hash + parser_version.
    assert!(
        line.contains("workspace_id=ws-1") && line.contains("parser_version=rust"),
        "lens query carries workspace_id + parser_version : {line}"
    );
}

// ── AC + RISK-1: file_lens with an unsafe path -> UnsafePath BEFORE any HTTP request. ───────────────

#[test]
fn ac_file_lens_rejects_unsafe_path_before_sending() {
    let client = KnowledgeCodeNavClient::with_base_url("http://127.0.0.1:1");
    let headers = code_nav_headers(&ctx());

    for bad in [
        "/etc/passwd",
        "../../secret.rs",
        "a/../b.rs",
        "win\\path.rs",
    ] {
        let result =
            rt().block_on(async { client.file_lens(&headers, bad, "ws-1", "h", "v").await });
        match result {
            Err(CodeNavError::UnsafePath(p)) => assert_eq!(p, bad),
            other => panic!("unsafe path '{bad}' must be a client-side UnsafePath, got {other:?}"),
        }
    }
}

// ── AC: symbol_references returns both callers AND callees populated. ───────────────────────────────

#[test]
fn ac_symbol_references_populates_callers_and_callees() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", references_response_body());
    let client = KnowledgeCodeNavClient::with_base_url(base_url);
    let headers = code_nav_headers(&ctx());

    let resp = rt().block_on(async { client.symbol_references(&headers, "KE-target").await });
    let exchange = server.join().unwrap();

    let resp = resp.expect("references against a real-shape 200 must deserialize");
    assert_eq!(resp.symbol_entity_id, "KE-target");
    assert_eq!(resp.callers.len(), 1, "callers populated");
    assert_eq!(resp.callees.len(), 1, "callees populated");
    assert_eq!(resp.callers[0].symbol_key, "rust:src/a.rs#call_target");
    assert_eq!(resp.callers[0].evidence_spans.len(), 1);
    assert_eq!(resp.callers[0].evidence_spans[0].line_start, Some(5));
    assert!(
        (resp.callers[0].confidence - 0.92).abs() < 1e-9,
        "edge confidence parses as f64"
    );
    assert_eq!(resp.callees[0].symbol_key, "rust:src/b.rs#helper");
    // RISK-3: the callee's "unindexed" staleness deserializes tolerantly and reads as STALE.
    assert!(
        !resp.callees[0].staleness.is_fresh(),
        "an unindexed callee is correctly surfaced as not-fresh (never authoritative)"
    );
    assert_eq!(resp.nav_receipt_event_id, "KEV-nav-refs");
    assert_eq!(resp.quiet_background_work_receipt_id, "QBW-quiet-refs");

    assert!(
        exchange
            .captured_request_line
            .starts_with("GET /knowledge/code/symbols/KE-target/references"),
        "references hits the right route : {}",
        exchange.captured_request_line
    );
}

// ── AC + RISK-3: an UNKNOWN future staleness state on a real nav body deserializes without error. ───

#[test]
fn ac_unknown_future_staleness_state_does_not_break_a_real_nav_body() {
    // A get_symbol body whose staleness is a state the client has never seen. The whole response MUST
    // still deserialize (RISK-3/MC-3/MC-6) and the symbol MUST read as STALE (fail-closed).
    let body = json!({
        "symbol": {
            "symbol_entity_id": "KE-future",
            "symbol_key": "rust:src/x.rs#Bar",
            "display_name": "Bar",
            "symbol_kind": "fn",
            "owning_wp": null,
            "primary_source_id": "file:src/x.rs",
            "lifecycle_state": "active",
            "definition": null,
            "staleness": {"state": "custom_future_state", "fresh": false, "future_field": 7}
        },
        "nav_receipt_event_id": "KEV-nav-future",
        "quiet_background_work_receipt_id": "QBW-quiet-future"
    });
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", body);
    let client = KnowledgeCodeNavClient::with_base_url(base_url);
    let headers = code_nav_headers(&ctx());

    let resp = rt().block_on(async { client.get_symbol(&headers, "KE-future").await });
    let _ = server.join().unwrap();

    let resp =
        resp.expect("an unknown future staleness state must NOT break response deserialization");
    assert_eq!(resp.symbol.staleness.state, "custom_future_state");
    assert!(
        !resp.symbol.staleness.is_fresh(),
        "an unknown staleness state is treated as STALE (fail-closed) — never rendered as authoritative"
    );
    // The unknown extra field is captured, not dropped.
    assert_eq!(
        resp.symbol
            .staleness
            .extra
            .get("future_field")
            .and_then(Value::as_i64),
        Some(7)
    );
    // The definition was JSON null -> None (not a parse error).
    assert!(resp.symbol.definition.is_none());
    assert_eq!(resp.nav_receipt_event_id, "KEV-nav-future");
}

// ── BACKEND-SHAPE: a span whose line_start/line_end are JSON null deserializes to None, not Parse. ──

#[test]
fn ac_symbol_spans_with_null_line_numbers_deserializes_to_none_not_parse_error() {
    // The backend `KnowledgeSpan.line_start` / `.line_end` columns are `Option<i32>`
    // (`storage/knowledge.rs:763-764`) and `symbol_spans` projects them directly
    // (`api/knowledge_code_nav.rs:667-668`), so a span with a NULL line serializes to JSON `null`.
    // This is a REAL backend shape: the same `symbol_spans` 200 envelope (`:689-695`) with one span
    // whose lines are null. Before the hardening fix the client bound `line_start: i64` (non-optional)
    // and this body would have failed with `CodeNavError::Parse`, silently dropping the whole nav
    // response. The fix binds `Option<i64>` + `#[serde(default)]`; a null line must read as `None`.
    let body = json!({
        "symbol_entity_id": "KE-null-lines",
        "staleness": {"state": "fresh", "fresh": true,
            "indexed_content_hash": "sha256:n", "indexed_parser_version": "rust@0.25"},
        "spans": [
            {
                "span_id": "KSP-null",
                "source_id": "file:src/lib.rs",
                "span_kind": "reference",
                "line_start": null,
                "line_end": null,
                "range_start": 100,
                "range_end": 200,
                "section_path": null,
                "content_sha256": "sha256:c",
                "parser_version": "rust@0.25"
            }
        ],
        "nav_receipt_event_id": "KEV-nav-spans-null",
        "quiet_background_work_receipt_id": "QBW-quiet-spans-null"
    });
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", body);
    let client = KnowledgeCodeNavClient::with_base_url(base_url);
    let headers = code_nav_headers(&ctx());

    let resp = rt().block_on(async { client.symbol_spans(&headers, "KE-null-lines").await });
    let exchange = server.join().unwrap();

    let resp = resp
        .expect("a span with null line_start/line_end is a valid backend shape and MUST NOT be a Parse error");
    assert_eq!(resp.symbol_entity_id, "KE-null-lines");
    assert_eq!(resp.spans.len(), 1, "the span with null lines is preserved");
    let span = &resp.spans[0];
    assert_eq!(span.span_id, "KSP-null");
    assert_eq!(
        span.line_start, None,
        "a JSON null line_start deserializes to None (tolerant, MC-3)"
    );
    assert_eq!(
        span.line_end, None,
        "a JSON null line_end deserializes to None (tolerant, MC-3)"
    );
    // The non-null sibling fields still parse.
    assert_eq!(span.range_start, Some(100));
    assert_eq!(span.range_end, Some(200));
    assert_eq!(span.content_sha256.as_deref(), Some("sha256:c"));
    // Receipt ids preserved through deserialization (RISK-4/MC-4).
    assert_eq!(resp.nav_receipt_event_id, "KEV-nav-spans-null");
    assert_eq!(
        resp.quiet_background_work_receipt_id,
        "QBW-quiet-spans-null"
    );
    // The request hit the right route (also closes the symbol_spans integration coverage gap).
    assert!(
        exchange
            .captured_request_line
            .starts_with("GET /knowledge/code/symbols/KE-null-lines/spans"),
        "symbol_spans hits the right route : {}",
        exchange.captured_request_line
    );
}

// ── HYGIENE: this MT writes ZERO test artifacts; guard that no repo-local artifact dir exists. ──────

/// MT-039 is a pure-client MT with NO screenshots / PNGs / snapshots. This guard enforces the CX-212E
/// hygiene rule reflexively: a code-nav test must never create a repo-local `test_output/` or
/// `tests/screenshots/` directory. If a future edit adds a screenshot test it MUST route to the external
/// artifact root, never here.
#[test]
fn no_local_artifact_dir_is_created_by_this_mt() {
    assert_no_local_artifact_dir();
}

/// Fail if a repo-local artifact directory exists under the crate (`test_output/` or `tests/screenshots/`).
fn assert_no_local_artifact_dir() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    for rel in ["test_output", "tests/screenshots"] {
        let path = std::path::Path::new(crate_dir).join(rel);
        assert!(
            !path.exists(),
            "repo-local artifact dir '{}' must not exist (CX-212E: artifacts go to the external root)",
            path.display()
        );
    }
}
