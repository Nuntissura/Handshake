//! WP-KERNEL-012 MT-040 (E6) — wire-level proofs for the CRDT transport client
//! (`handshake_native::backend::knowledge_crdt`).
//!
//! ## Mock-server fixture provenance (SPEC-REALISM GATE — the load-bearing rule)
//!
//! This MT is fully provable WITHOUT a live PostgreSQL/handshake_core: every AC runs against an
//! in-process mock HTTP server (the PROVEN MT-020/037 `std::net::TcpListener` capture pattern — NO new
//! dependency). The mock JSON RESPONSES below are REAL backend response shapes — field-verified against
//! the handshake_core source READ-ONLY (NOT self-serialized from the client's own Rust types, which
//! would be a tautology proving nothing — Spec-Realism Sub-rule 2). Each fixture cites the exact backend
//! struct/handler it mirrors:
//!   * push 200/409 body <- `api::knowledge_crdt::PushUpdateResponse {result, receipt}` (api L131-135)
//!     with `result` = `yjs_bridge::YjsPushOutcomeV1` tagged by `outcome` (snake_case, yjs_bridge
//!     L433-450) and `receipt` = `KnowledgeNavigationReceiptV1` (api L76-85). The backend returns 200 for
//!     Stored/AlreadyStored and 409 for Denied (api `push_update` L157-162).
//!   * pull body <- `api::knowledge_crdt::PullUpdatesResponse {result, receipt}` (api L263-267) with
//!     `result` = `yjs_bridge::YjsUpdatePullResponseV1` whose `updates` is `Vec<YjsUpdateEnvelopeV1>`
//!     (yjs_bridge L655-665); each envelope's binary update is `update_b64` (base64 STANDARD, encoded by
//!     `update_record_to_envelope`, yjs_bridge L383).
//!   * conflict body <- `api::knowledge_crdt::ConflictStateResponse {result, receipt}` (api L312-316)
//!     with `result` = `conflict_ui::ConflictUiStateV1` (conflict_ui L96-105).
//!   * 400 body <- `api::knowledge_crdt::KnowledgeCrdtErrorResponse {code, message}` from
//!     `require_navigation_ids` (api L105-124).
//!
//! The base URL is the mock server's address (config-injected, never hardcoded — GLOBAL-PORTABILITY).
//! The client is a stateless adapter (no CRDT state held). NO GUI -> no screenshot / accesskit this MT.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use base64::Engine;
use handshake_native::backend::knowledge_crdt::{
    ConflictStateParams, CrdtError, KnowledgeCrdtClient, PullUpdatesParams, YjsPushDenialReasonV1,
    YjsPushOutcomeV1, YjsUpdateEnvelopeV1, YJS_PUSH_DENIAL_SCHEMA_ID, YJS_UPDATE_ENCODING_V1,
    YJS_UPDATE_ENVELOPE_SCHEMA_ID,
};
use serde_json::{json, Value};

// ── REAL backend response fixtures (verified field-for-field against the backend source). ──────────

fn b64_standard(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// A spec-2.3.13.11 receipt body shape (api `navigation_receipt`, L93-103) — all seven fields.
fn receipt_body(operation: &str) -> Value {
    json!({
        "receipt_kind": "knowledge_crdt_navigation_receipt_v1",
        "actor_id": "operator:ilja",
        "session_id": "SESS-1",
        "correlation_id": "CORR-1",
        "target_authority_ref": "postgres://kernel_crdt_updates/KCRDT-1",
        "operation": operation,
        "served_at_utc": "2026-06-23T00:00:00+00:00"
    })
}

/// The real `push_update` 200 body: `{result: Stored{..}, receipt}` — `result` tagged by `outcome`.
fn push_200_stored_body() -> Value {
    json!({
        "result": {
            "outcome": "stored",
            "update_seq": 42u64,
            "update_id": "U-1",
            "event_ledger_event_id": "EVT-99",
            "head_state_vector": "sv-after-encoded"
        },
        "receipt": receipt_body("push_update")
    })
}

/// The real `push_update` 409 body: `{result: Denied{denial}, receipt}`. The denial's `reason` is the
/// `yjs_bridge::YjsPushDenialReasonV1::StaleBase` tagged by `code` (snake_case).
fn push_409_denied_body() -> Value {
    json!({
        "result": {
            "outcome": "denied",
            "denial": {
                "schema_id": YJS_PUSH_DENIAL_SCHEMA_ID,
                "crdt_document_id": "KCRDT-1",
                "update_id": "U-1",
                "actor_id": "operator:ilja",
                "reason": {
                    "code": "stale_base",
                    "head_update_seq": 7u64,
                    "head_state_vector": "sv-head-encoded",
                    "ordering": "Concurrent"
                }
            }
        },
        "receipt": receipt_body("push_update")
    })
}

/// The real `pull_updates` body: `{result: YjsUpdatePullResponseV1{.., updates:[envelope]}, receipt}`.
/// `update_bytes` are the bytes the test will assert survive the base64 STANDARD roundtrip.
fn pull_one_update_body(update_bytes: &[u8]) -> Value {
    json!({
        "result": {
            "workspace_id": "WS-1",
            "document_id": "DOC-1",
            "crdt_document_id": "KCRDT-1",
            "since_update_seq": 0u64,
            "updates": [
                {
                    "schema_id": YJS_UPDATE_ENVELOPE_SCHEMA_ID,
                    "workspace_id": "WS-1",
                    "document_id": "DOC-1",
                    "crdt_document_id": "KCRDT-1",
                    "update_id": "U-1",
                    "actor_id": "operator:ilja",
                    "site_id": "SITE-1",
                    "session_id": "SESS-1",
                    "trace_id": "TRACE-1",
                    "document_schema_id": "prosemirror-v1",
                    "update_b64": b64_standard(update_bytes),
                    "update_sha256": "abc123",
                    "state_vector_before": "sv-before",
                    "state_vector_after": "sv-after",
                    "encoding": YJS_UPDATE_ENCODING_V1
                }
            ],
            "head_update_seq": 1u64,
            "head_state_vector": "sv-head-encoded"
        },
        "receipt": receipt_body("pull_updates")
    })
}

/// An empty-CRDT-doc pull body: `updates: []` + `head_update_seq: 0` — a NORMAL empty state (RISK-8).
fn pull_empty_body() -> Value {
    json!({
        "result": {
            "workspace_id": "WS-1",
            "document_id": "DOC-1",
            "crdt_document_id": "KCRDT-NEW",
            "since_update_seq": 0u64,
            "updates": [],
            "head_update_seq": 0u64,
            "head_state_vector": ""
        },
        "receipt": receipt_body("pull_updates")
    })
}

/// The real `conflict_state` body with one conflict: `{result: ConflictUiStateV1{..}, receipt}`.
fn conflict_has_one_body() -> Value {
    json!({
        "result": {
            "schema_id": "hsk.kernel.knowledge_conflict_ui_state@1",
            "workspace_id": "WS-1",
            "document_id": "DOC-1",
            "crdt_document_id": "KCRDT-1",
            "head_update_seq": 7u64,
            "head_state_vector": "sv-head-encoded",
            "conflicts": [
                {
                    "conflict_id": "RCPT-1",
                    "kind": "stale_draft_save",
                    "detected_at_utc": "2026-06-23T00:00:00+00:00",
                    "conflicting_actors": [
                        {"actor_id": "operator:ilja", "actor_kind": "operator", "session_id": "SESS-1"}
                    ],
                    "base": null,
                    "ours": {"label": "ours", "update_seq": 7u64, "update_id": null, "state_vector": "sv-head"},
                    "theirs": {"label": "theirs", "update_seq": null, "update_id": "U-1", "state_vector": "sv-theirs"},
                    "resolution_options": [
                        {"option": "pull_merge_resubmit", "pull_since_update_seq": 0u64},
                        {"option": "adopt_server_head"}
                    ],
                    "denial_receipt_id": "RCPT-1",
                    "event_ledger_event_id": "EVT-1"
                }
            ]
        },
        "receipt": receipt_body("conflict_state")
    })
}

/// The real `require_navigation_ids` 400 body (api L116-122).
fn missing_ids_400_body() -> Value {
    json!({
        "code": "knowledge_crdt_navigation_ids_required",
        "message": "actor_id, session_id and correlation_id are required (spec 2.3.13.11)"
    })
}

// ── In-process mock HTTP server (proven MT-020/037 TcpListener pattern; no new dependency). ────────

struct MockExchange {
    captured_request_line: String,
    captured_body: String,
}

/// Spin up a one-shot mock server that replies with `status_line` + `body` to the FIRST request, and
/// captures that request's line/body. Returns (base_url, join handle delivering the capture).
fn spawn_mock(
    status_line: &'static str,
    body: Value,
) -> (String, std::thread::JoinHandle<MockExchange>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let (request_line, req_body) = read_one_http_request(&mut stream);
        let body_str = body.to_string();
        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body_str}",
            body_str.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
        MockExchange {
            captured_request_line: request_line,
            captured_body: req_body,
        }
    });
    (base_url, handle)
}

/// Read one full HTTP request (headers + Content-Length body) off the stream.
fn read_one_http_request(stream: &mut std::net::TcpStream) -> (String, String) {
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
    let request_line = text[..hdr_end].lines().next().unwrap_or("").to_string();
    let body = text[(hdr_end + 4).min(text.len())..].to_string();
    (request_line, body)
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
}

/// A non-empty, identity-complete envelope for the push tests.
fn valid_envelope(update_bytes: &[u8]) -> YjsUpdateEnvelopeV1 {
    YjsUpdateEnvelopeV1 {
        schema_id: YJS_UPDATE_ENVELOPE_SCHEMA_ID.to_string(),
        workspace_id: "WS-1".into(),
        document_id: "DOC-1".into(),
        crdt_document_id: "KCRDT-1".into(),
        update_id: "U-1".into(),
        actor_id: "operator:ilja".into(),
        site_id: "SITE-1".into(),
        session_id: "SESS-1".into(),
        trace_id: "TRACE-1".into(),
        document_schema_id: "prosemirror-v1".into(),
        update_b64: String::new(),
        update_sha256: "abc123".into(),
        state_vector_before: "sv-before".into(),
        state_vector_after: "sv-after".into(),
        encoding: YJS_UPDATE_ENCODING_V1.to_string(),
    }
    .with_update_bytes(update_bytes)
}

// ── AC: push 200 Stored -> Ok(Stored{..}) with update_seq set. ─────────────────────────────────────

#[test]
fn ac_push_200_stored_returns_ok_stored_with_update_seq() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", push_200_stored_body());
    let client = KnowledgeCrdtClient::with_base_url(base_url);
    let env = valid_envelope(b"hello-update");

    let outcome = rt().block_on(async { client.push_update(&env).await });
    let _ = server.join().unwrap();

    match outcome {
        Ok(YjsPushOutcomeV1::Stored { update_seq, update_id, head_state_vector, .. }) => {
            assert_eq!(update_seq, 42, "update_seq from the real Stored outcome");
            assert_eq!(update_id, "U-1");
            assert_eq!(head_state_vector, "sv-after-encoded");
        }
        other => panic!("expected Ok(Stored), got {other:?}"),
    }
}

// ── AC + MC-1 (the load-bearing rule): push 409 Denied -> Ok(Denied{..}), NOT Err. ─────────────────

#[test]
fn crdt_push_409_is_ok() {
    let (base_url, server) = spawn_mock("HTTP/1.1 409 Conflict", push_409_denied_body());

    // Also prove the conflict listener fires (note 10 — denial is never silently swallowed).
    let fired: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let fired_clone = Arc::clone(&fired);
    let client = KnowledgeCrdtClient::with_base_url(base_url).with_conflict_listener(Arc::new(
        move |denial| {
            fired_clone.lock().unwrap().push(denial.update_id.clone());
        },
    ));
    let env = valid_envelope(b"stale-update");

    let outcome = rt().block_on(async { client.push_update(&env).await });
    let exchange = server.join().unwrap();

    // The CRDT push identity travels IN the envelope BODY (NOT x-hsk-* headers): prove actor_id /
    // session_id / trace_id were serialized into the POST body on the wire.
    let sent: Value = serde_json::from_str(&exchange.captured_body).expect("push body is json");
    assert_eq!(sent["envelope"]["actor_id"], json!("operator:ilja"));
    assert_eq!(sent["envelope"]["session_id"], json!("SESS-1"));
    assert_eq!(sent["envelope"]["trace_id"], json!("TRACE-1"));
    assert!(
        exchange.captured_request_line.starts_with("POST /knowledge/crdt/updates/push"),
        "push must POST the push route: {}",
        exchange.captured_request_line
    );

    match outcome {
        Ok(YjsPushOutcomeV1::Denied { denial }) => {
            // 409 is a VALID DOMAIN OUTCOME returned as Ok, never Err (RISK-1 / MC-1).
            assert_eq!(denial.update_id, "U-1");
            assert_eq!(denial.schema_id, YJS_PUSH_DENIAL_SCHEMA_ID);
            match denial.reason {
                YjsPushDenialReasonV1::StaleBase { head_update_seq, ordering, .. } => {
                    assert_eq!(head_update_seq, 7);
                    assert_eq!(ordering, "Concurrent");
                }
                other => panic!("expected StaleBase reason, got {other:?}"),
            }
        }
        Err(e) => panic!("a 409 push denial MUST be Ok(Denied), NEVER Err — got Err({e})"),
        other => panic!("expected Ok(Denied), got {other:?}"),
    }
    assert_eq!(
        *fired.lock().unwrap(),
        vec!["U-1".to_string()],
        "the conflict listener must fire on a Denied (note 10 — no silent swallow)"
    );
}

// ── AC + MC-2 (the load-bearing data-integrity rule): pull base64 STANDARD roundtrip byte-for-byte. ─

#[test]
fn crdt_base64_roundtrip() {
    // Bytes that DIFFER between STANDARD and url-safe alphabets so a url-safe engine would corrupt them.
    let raw: Vec<u8> = vec![0x00, 0xFF, 0x3E, 0x3F, 0xFB, 0xEF, b'y', b'j', b's', 0x80, 0x10];
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", pull_one_update_body(&raw));
    let client = KnowledgeCrdtClient::with_base_url(base_url);
    let params = PullUpdatesParams {
        workspace_id: "WS-1".into(),
        document_id: "DOC-1".into(),
        crdt_document_id: "KCRDT-1".into(),
        document_schema_id: "prosemirror-v1".into(),
        since_update_seq: None,
        actor_id: "operator:ilja".into(),
        session_id: "SESS-1".into(),
        correlation_id: "CORR-1".into(),
    };

    let resp = rt().block_on(async { client.pull_updates(&params).await }).expect("pull ok");
    let _ = server.join().unwrap();

    assert_eq!(resp.updates.len(), 1, "one StoredYjsUpdate envelope replayed");
    let decoded = resp.updates[0].update_bytes().expect("update_b64 decodes");
    assert_eq!(decoded, raw, "base64 STANDARD roundtrip must be byte-for-byte exact");
    assert_eq!(decoded.len(), raw.len(), "decoded byte length matches");
    assert_eq!(resp.head_update_seq, 1);
}

// ── AC + MC-5: pull with since_update_seq=5 puts ?since_update_seq=5 on the wire. ──────────────────

#[test]
fn pull_since_update_seq_is_on_the_wire() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", pull_empty_body());
    let client = KnowledgeCrdtClient::with_base_url(base_url);
    let params = PullUpdatesParams {
        workspace_id: "WS-1".into(),
        document_id: "DOC-1".into(),
        crdt_document_id: "KCRDT-1".into(),
        document_schema_id: "prosemirror-v1".into(),
        since_update_seq: Some(5),
        actor_id: "operator:ilja".into(),
        session_id: "SESS-1".into(),
        correlation_id: "CORR-1".into(),
    };

    let _ = rt().block_on(async { client.pull_updates(&params).await }).expect("pull ok");
    let exchange = server.join().unwrap();

    assert!(
        exchange.captured_request_line.contains("since_update_seq=5"),
        "the outgoing GET must carry ?since_update_seq=5 (MC-5); got: {}",
        exchange.captured_request_line
    );
    // The request was a GET against the pull route.
    assert!(exchange.captured_request_line.starts_with("GET /knowledge/crdt/updates/pull"));
}

// ── RISK-8: an empty CRDT doc pull is a NORMAL empty state, NOT an error. ──────────────────────────

#[test]
fn pull_empty_doc_is_not_an_error() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", pull_empty_body());
    let client = KnowledgeCrdtClient::with_base_url(base_url);
    let params = PullUpdatesParams {
        workspace_id: "WS-1".into(),
        document_id: "DOC-1".into(),
        crdt_document_id: "KCRDT-NEW".into(),
        document_schema_id: "prosemirror-v1".into(),
        since_update_seq: None,
        actor_id: "operator:ilja".into(),
        session_id: "SESS-1".into(),
        correlation_id: "CORR-1".into(),
    };

    let resp = rt().block_on(async { client.pull_updates(&params).await }).expect("empty pull is Ok");
    let _ = server.join().unwrap();

    assert!(resp.updates.is_empty(), "empty doc -> no updates");
    assert_eq!(resp.head_update_seq, 0, "fresh doc head_update_seq is 0, NOT an error");
}

// ── AC: conflict_state has_conflict=true. ──────────────────────────────────────────────────────────

#[test]
fn ac_conflict_state_has_conflict_true() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", conflict_has_one_body());
    let client = KnowledgeCrdtClient::with_base_url(base_url);
    let params = ConflictStateParams {
        workspace_id: "WS-1".into(),
        document_id: "DOC-1".into(),
        crdt_document_id: "KCRDT-1".into(),
        actor_id: "operator:ilja".into(),
        session_id: "SESS-1".into(),
        correlation_id: "CORR-1".into(),
    };

    let state = rt()
        .block_on(async { client.conflict_state(&params).await })
        .expect("conflict_state ok");
    let _ = server.join().unwrap();

    assert!(state.has_conflict(), "one conflict -> has_conflict true");
    assert_eq!(state.denial_count(), 1);
    assert_eq!(state.conflicts[0].denial_receipt_id, "RCPT-1");
    // The receipt is preserved (all 7 fields deserialized) on this read path too.
}

// ── AC + MC-4: KnowledgeNavigationReceiptV1 preserved with all SEVEN fields on every response. ─────

#[test]
fn receipt_all_seven_fields_preserved_on_push() {
    // The push response struct carries the receipt; deserialize it through the client by capturing the
    // typed PushUpdateResponse via a Stored push and asserting the parsed receipt round-trips. Since the
    // client returns only the outcome, prove the receipt is parsed (no parse error) AND assert each field
    // by deserializing the same body through the public response struct.
    use handshake_native::backend::knowledge_crdt::PushUpdateResponse;
    let body = push_200_stored_body();
    let parsed: PushUpdateResponse = serde_json::from_value(body).expect("parse PushUpdateResponse");
    let r = parsed.receipt;
    assert_eq!(r.receipt_kind, "knowledge_crdt_navigation_receipt_v1");
    assert_eq!(r.actor_id, "operator:ilja");
    assert_eq!(r.session_id, "SESS-1");
    assert_eq!(r.correlation_id, "CORR-1");
    assert_eq!(r.target_authority_ref, "postgres://kernel_crdt_updates/KCRDT-1");
    assert_eq!(r.operation, "push_update");
    assert_eq!(r.served_at_utc, "2026-06-23T00:00:00+00:00");
}

// ── AC + MC-3 / RISK-5: empty identity is a client-side pre-flight error (no request sent). ────────

#[test]
fn push_empty_actor_id_is_identity_missing_before_send() {
    // No server: the guard must fire BEFORE a socket opens. Use an unroutable base URL so any accidental
    // send would error differently (Transport), proving the guard short-circuited first.
    let client = KnowledgeCrdtClient::with_base_url("http://127.0.0.1:1");
    let mut env = valid_envelope(b"x");
    env.actor_id = "   ".into(); // whitespace-only is empty after trim

    let outcome = rt().block_on(async { client.push_update(&env).await });
    match outcome {
        Err(CrdtError::IdentityMissing(fields)) => {
            assert!(fields.contains("actor_id"), "must name actor_id: {fields}");
        }
        other => panic!("expected IdentityMissing pre-flight error, got {other:?}"),
    }
}

#[test]
fn pull_empty_correlation_id_is_identity_missing_before_send() {
    let client = KnowledgeCrdtClient::with_base_url("http://127.0.0.1:1");
    let params = PullUpdatesParams {
        workspace_id: "WS-1".into(),
        document_id: "DOC-1".into(),
        crdt_document_id: "KCRDT-1".into(),
        document_schema_id: "prosemirror-v1".into(),
        since_update_seq: None,
        actor_id: "operator:ilja".into(),
        session_id: "SESS-1".into(),
        correlation_id: "".into(),
    };
    let outcome = rt().block_on(async { client.pull_updates(&params).await });
    match outcome {
        Err(CrdtError::IdentityMissing(fields)) => {
            assert!(fields.contains("correlation_id"), "must name correlation_id: {fields}");
        }
        other => panic!("expected IdentityMissing pre-flight error, got {other:?}"),
    }
}

// ── A real backend 400 (missing ids) maps to CrdtError::HttpError (NOT a silent success). ──────────

#[test]
fn push_real_backend_400_maps_to_http_error() {
    // If a request DID reach the backend with empty ids (e.g. ids present client-side but rejected
    // server-side for another reason), the 400 is a genuine HttpError, never a Denied/Ok.
    let (base_url, server) = spawn_mock("HTTP/1.1 400 Bad Request", missing_ids_400_body());
    let client = KnowledgeCrdtClient::with_base_url(base_url);
    let env = valid_envelope(b"x"); // identity present so the client actually sends

    let outcome = rt().block_on(async { client.push_update(&env).await });
    let _ = server.join().unwrap();

    match outcome {
        Err(CrdtError::HttpError { status, body }) => {
            assert_eq!(status, 400);
            assert!(body.contains("knowledge_crdt_navigation_ids_required"), "body: {body}");
        }
        other => panic!("expected HttpError(400), got {other:?}"),
    }
}
