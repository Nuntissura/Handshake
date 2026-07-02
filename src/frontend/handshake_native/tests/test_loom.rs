//! WP-KERNEL-012 MT-038 (E6) — wire-level proofs for the consolidated Loom client
//! (`handshake_native::backend::loom`).
//!
//! ## Mock-server fixture provenance (SPEC-REALISM GATE — the load-bearing rule)
//!
//! This MT is fully provable WITHOUT a live PostgreSQL/handshake_core: every AC runs against an
//! in-process mock HTTP server (the PROVEN MT-020/021/037 `std::net::TcpListener` capture pattern — NO
//! new dependency). The mock JSON RESPONSES below are REAL backend response shapes — they are
//! field-verified against the handshake_core handler + storage bodies (read READ-ONLY for verification),
//! NOT self-serialized from the client's own Rust types (a fixture that feeds the client its own
//! serialization would be a TAUTOLOGY proving nothing — Spec-Realism Sub-rule 2). Each fixture cites the
//! exact backend source it mirrors:
//!   * `block_response_body`        <- `storage/loom.rs::LoomBlock` (serde shape; ~L300) returned by
//!     `api/loom.rs::create_loom_block`/`get_loom_block`/`open_daily_journal` as `Json<LoomBlock>`.
//!   * `edge_response_body`         <- `storage/loom.rs::LoomEdge` (~L429) returned by
//!     `api/loom.rs::create_loom_edge`/`delete_loom_edge` as `Json<LoomEdge>`.
//!   * `transclusion_unresolved_body` <- `api/loom.rs::LoomTransclusionResponse` (~L577) — the
//!     `resolved:false` 200 body a block with no source document returns (~L611).
//!   * `search_v2_response_body`    <- `storage/loom.rs::LoomSearchV2Response` (L687) whose `hits` are
//!     `storage/loom.rs::LoomSearchV2Hit` (L667): each hit is a NESTED full `block` (`LoomBlock`) plus
//!     `score`/`fts_rank`/`trgm_sim`/`vector_sim`/`edge_degree`/`highlight` (NO flat block id, NO
//!     per-modality `*_score` fields) — returned VERBATIM by `api/loom.rs::loom_search_v2` (L2683,
//!     `Json<LoomSearchV2Response>`, no `#[serde(flatten)]`).
//!   * `markdown_import_body`       <- `storage/loom.rs::LoomMarkdownImport` (~L1329) returned by
//!     `api/loom.rs::import_markdown_to_loom`.
//!   * `journal_date_400_body`      <- `api/loom.rs::parse_journal_date` `bad_request("HSK-400-LOOM-
//!     JOURNAL-DATE")` (~L494) -> `{"error":"HSK-400-LOOM-JOURNAL-DATE"}`.
//!
//! The mock asserts the externally-meaningful client behavior: block CRUD round-trips a typed
//! `LoomBlock`; edge create/delete round-trips a typed `LoomEdge`; an unresolved transclusion is a
//! FIRST-CLASS Ok (`resolved==false` + reason, NEVER an error); search-v2 parses a non-empty hit list;
//! the tag patch sends `add_tags`/`remove_tags` as tag-hub BLOCK ids (not edge ids) ON THE WIRE; a
//! markdown import returns the typed authority block; a malformed journal date is a typed `BadRequest`;
//! and NO `x-hsk-*` identity header is sent (loom requires none). The base URL is the mock server's
//! address (config-injected, never hardcoded — GLOBAL-PORTABILITY). The client is a stateless adapter
//! (workspace_id is always a param; no state held). NO GUI -> no screenshot / accesskit this MT.

use std::io::{Read, Write};
use std::net::TcpListener;

use handshake_native::backend::loom::{
    CreateLoomBlockRequest, CreateLoomEdgeRequest, ImportMarkdownRequest, LoomBlockContentType,
    LoomBlockPatchRequest, LoomClient, LoomEdgeCreatedBy, LoomEdgeType, LoomError,
    LoomSearchV2Request,
};
use serde_json::{json, Value};

// ── REAL backend response fixtures (verified against api/loom.rs + storage/loom.rs). ───────────────

/// `Json<LoomBlock>` shape — `storage::loom::LoomBlock` serde fields (the full row create/get/journal
/// return). `derived` is the real `LoomBlockDerived` object the backend embeds.
fn block_response_body() -> Value {
    json!({
        "block_id": "BLK-1",
        "workspace_id": "WS-1",
        "content_type": "note",
        "document_id": "KRD-1",
        "asset_id": null,
        "title": "Design Notes",
        "original_filename": null,
        "content_hash": null,
        "pinned": false,
        "favorite": false,
        "journal_date": null,
        "created_at": "2026-06-23T00:00:00Z",
        "updated_at": "2026-06-23T00:00:00Z",
        "imported_at": null,
        "derived": {
            "backlink_count": 2,
            "mention_count": 1,
            "tag_count": 3,
            "preview_status": "none"
        }
    })
}

/// A daily-journal `Json<LoomBlock>` (content_type=journal, journal_date set) — `open_daily_journal`.
fn journal_block_response_body() -> Value {
    json!({
        "block_id": "BLK-journal-20260623",
        "workspace_id": "WS-1",
        "content_type": "journal",
        "document_id": null,
        "asset_id": null,
        "title": "2026-06-23",
        "original_filename": null,
        "content_hash": null,
        "pinned": false,
        "favorite": false,
        "journal_date": "2026-06-23",
        "created_at": "2026-06-23T00:00:00Z",
        "updated_at": "2026-06-23T00:00:00Z",
        "imported_at": null,
        "derived": {
            "backlink_count": 0,
            "mention_count": 0,
            "tag_count": 0,
            "preview_status": "none"
        }
    })
}

/// `Json<LoomEdge>` shape — `storage::loom::LoomEdge` serde fields (create/delete edge return).
fn edge_response_body() -> Value {
    json!({
        "edge_id": "EDGE-1",
        "workspace_id": "WS-1",
        "source_block_id": "BLK-1",
        "target_block_id": "BLK-2",
        "edge_type": "mention",
        "created_by": "user",
        "created_at": "2026-06-23T00:00:00Z",
        "crdt_site_id": null,
        "source_anchor": null
    })
}

/// `api::loom::LoomTransclusionResponse` with `resolved:false` — the 200 body a block with no source
/// document returns (the FIRST-CLASS unresolved state; NEVER an error).
fn transclusion_unresolved_body() -> Value {
    json!({
        "block_id": "BLK-1",
        "workspace_id": "WS-1",
        "source_document_id": null,
        "source_doc_version": null,
        "content_json": null,
        "resolved": false,
        "unresolved_reason": "loom_block_has_no_source_document"
    })
}

/// `storage::loom::LoomSearchV2Response` (L687) — `hits` + `content_type_facets` + `semantic_available`
/// and `total`. Each hit is a `storage::loom::LoomSearchV2Hit` (L667): a NESTED full `block` (`LoomBlock`,
/// reusing `block_response_body()`) plus the per-modality blend fields `score`/`fts_rank`/`trgm_sim`/
/// `vector_sim`/`edge_degree`/`highlight`. There is NO flat block id and NO `keyword_score`/`trigram_score`
/// /`semantic_score`/`graph_score` field — those keys do not exist on the real wire (verified against
/// `storage/loom.rs:667-696`; handler returns `Json<LoomSearchV2Response>` verbatim at `api/loom.rs:2683`).
fn search_v2_response_body() -> Value {
    json!({
        "hits": [
            {
                "block": block_response_body(),
                "score": 0.91,
                "fts_rank": 0.8,
                "trgm_sim": 0.4,
                "vector_sim": 0.0,
                "edge_degree": 1,
                "highlight": "<mark>design</mark> notes"
            }
        ],
        "content_type_facets": { "note": 1 },
        "semantic_available": false,
        "total": 1
    })
}

/// `storage::loom::LoomMarkdownImport` — the new authority block + backing rich-doc id + warnings.
fn markdown_import_body() -> Value {
    json!({
        "block": block_response_body(),
        "rich_document_id": "KRD-1",
        "warnings": ["unsupported: footnotes"]
    })
}

/// `parse_journal_date` 400 body for a malformed date.
fn journal_date_400_body() -> Value {
    json!({ "error": "HSK-400-LOOM-JOURNAL-DATE" })
}

// ── In-process mock HTTP server (proven MT-020/037 TcpListener pattern; no new dependency). ─────────

/// A captured record of what the client sent on the wire.
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

/// The loom routes require NO identity headers; assert the client sent none of them.
fn assert_no_identity_headers(headers: &std::collections::HashMap<String, String>) {
    for forbidden in [
        "x-hsk-actor-id",
        "x-hsk-actor-kind",
        "x-hsk-kernel-task-run-id",
        "x-hsk-session-run-id",
    ] {
        assert!(
            !headers.contains_key(forbidden),
            "loom requires NO identity headers; '{forbidden}' must NOT be sent: {headers:?}"
        );
    }
}

// ── AC: get_loom_block deserializes a real LoomBlock without panicking, no identity headers. ───────

#[test]
fn ac_get_loom_block_deserializes_real_backend_shape() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", block_response_body());
    let client = LoomClient::with_base_url(base_url);

    let resp = rt().block_on(async { client.get_loom_block("WS-1", "BLK-1").await });
    let exchange = server.join().unwrap();

    let block =
        resp.expect("get_loom_block against a real-shape 200 body must deserialize, not err");
    assert_eq!(block.block_id, "BLK-1");
    assert_eq!(block.content_type, LoomBlockContentType::Note);
    assert_eq!(block.title.as_deref(), Some("Design Notes"));
    assert_eq!(block.document_id.as_deref(), Some("KRD-1"));
    // derived stays a Value (coupling-avoid) but is populated.
    assert_eq!(
        block.derived.get("backlink_count").and_then(Value::as_i64),
        Some(2)
    );

    assert!(
        exchange
            .captured_request_line
            .starts_with("GET /workspaces/WS-1/loom/blocks/BLK-1"),
        "get_loom_block hits GET /workspaces/:ws/loom/blocks/:id : {}",
        exchange.captured_request_line
    );
    assert_no_identity_headers(&exchange.captured_headers);
}

// ── AC: block CRUD roundtrip — the NEW create/delete. ──────────────────────────────────────────────

#[test]
fn ac_create_block_roundtrips_typed_loom_block() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", block_response_body());
    let client = LoomClient::with_base_url(base_url);
    let body = CreateLoomBlockRequest {
        block_id: None,
        content_type: LoomBlockContentType::Note,
        document_id: Some("KRD-1".into()),
        asset_id: None,
        title: Some("Design Notes".into()),
        pinned: None,
        journal_date: None,
    };

    let resp = rt().block_on(async { client.create_block("WS-1", &body).await });
    let exchange = server.join().unwrap();

    let block = resp.expect("create_block must return a typed LoomBlock");
    assert_eq!(block.block_id, "BLK-1");
    assert!(
        exchange
            .captured_request_line
            .starts_with("POST /workspaces/WS-1/loom/blocks"),
        "create hits POST /workspaces/:ws/loom/blocks : {}",
        exchange.captured_request_line
    );
    // The body carries content_type snake_case and omits absent optionals (no fabricated block_id).
    let sent: Value = serde_json::from_str(&exchange.captured_body).unwrap();
    assert_eq!(sent["content_type"], json!("note"));
    assert_eq!(sent["document_id"], json!("KRD-1"));
    assert!(
        sent.get("block_id").is_none(),
        "absent block_id omitted: {sent}"
    );
}

#[test]
fn ac_delete_block_roundtrips_status_ack() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", json!({ "status": "deleted" }));
    let client = LoomClient::with_base_url(base_url);

    let resp = rt().block_on(async { client.delete_loom_block("WS-1", "BLK-1").await });
    let exchange = server.join().unwrap();

    let ack = resp.expect("delete_loom_block must return the status ack");
    assert_eq!(ack["status"], json!("deleted"));
    assert!(
        exchange
            .captured_request_line
            .starts_with("DELETE /workspaces/WS-1/loom/blocks/BLK-1"),
        "delete hits DELETE /workspaces/:ws/loom/blocks/:id : {}",
        exchange.captured_request_line
    );
}

// ── AC: edge create/delete roundtrip a typed LoomEdge. ──────────────────────────────────────────────

#[test]
fn ac_create_edge_roundtrips_typed_loom_edge() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", edge_response_body());
    let client = LoomClient::with_base_url(base_url);
    let body = CreateLoomEdgeRequest {
        edge_id: None,
        source_block_id: "BLK-1".into(),
        target_block_id: "BLK-2".into(),
        edge_type: LoomEdgeType::Mention,
        created_by: LoomEdgeCreatedBy::User,
        crdt_site_id: None,
        source_anchor: None,
        target_title: Some("Other".into()),
    };

    let resp = rt().block_on(async { client.create_loom_edge("WS-1", &body).await });
    let exchange = server.join().unwrap();

    let edge = resp.expect("create_loom_edge must return a typed LoomEdge");
    assert_eq!(edge.edge_id, "EDGE-1");
    assert_eq!(edge.edge_type, LoomEdgeType::Mention);
    assert_eq!(edge.created_by, LoomEdgeCreatedBy::User);
    assert!(
        exchange
            .captured_request_line
            .starts_with("POST /workspaces/WS-1/loom/edges"),
        "create edge hits POST /workspaces/:ws/loom/edges : {}",
        exchange.captured_request_line
    );
    // created_by is REQUIRED on the wire (verified backend contract).
    let sent: Value = serde_json::from_str(&exchange.captured_body).unwrap();
    assert_eq!(
        sent["created_by"],
        json!("user"),
        "created_by required on the wire"
    );
    assert_eq!(sent["edge_type"], json!("mention"));
}

#[test]
fn ac_delete_edge_roundtrips_typed_loom_edge() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", edge_response_body());
    let client = LoomClient::with_base_url(base_url);

    let resp = rt().block_on(async { client.delete_loom_edge("WS-1", "EDGE-1").await });
    let exchange = server.join().unwrap();

    let edge = resp.expect("delete_loom_edge must return the deleted LoomEdge");
    assert_eq!(edge.edge_id, "EDGE-1");
    assert!(
        exchange
            .captured_request_line
            .starts_with("DELETE /workspaces/WS-1/loom/edges/EDGE-1"),
        "delete edge hits DELETE /workspaces/:ws/loom/edges/:edge_id : {}",
        exchange.captured_request_line
    );
}

// ── AC: tag patch — add_tags/remove_tags are tag-hub BLOCK ids on the wire, not edge ids (RISK-2). ──

#[test]
fn ac_patch_block_sends_tags_as_block_ids() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", block_response_body());
    let client = LoomClient::with_base_url(base_url);
    let patch = LoomBlockPatchRequest {
        add_tags: vec!["BLK-tag-hub-A".into()],
        remove_tags: vec!["BLK-tag-hub-B".into()],
        ..Default::default()
    };

    let resp = rt().block_on(async { client.patch_loom_block("WS-1", "BLK-1", &patch).await });
    let exchange = server.join().unwrap();

    resp.expect("patch_loom_block must return the updated LoomBlock");
    assert!(
        exchange
            .captured_request_line
            .starts_with("PATCH /workspaces/WS-1/loom/blocks/BLK-1"),
        "patch hits PATCH /workspaces/:ws/loom/blocks/:id : {}",
        exchange.captured_request_line
    );
    let sent: Value = serde_json::from_str(&exchange.captured_body).unwrap();
    assert_eq!(
        sent["add_tags"],
        json!(["BLK-tag-hub-A"]),
        "add_tags are tag-hub BLOCK ids (RISK-2), sent as a string array: {sent}"
    );
    assert_eq!(sent["remove_tags"], json!(["BLK-tag-hub-B"]));
}

// ── AC: transclusion resolved=false is a FIRST-CLASS Ok, not an error (RISK-3 / MC-2). ──────────────

#[test]
fn ac_transclusion_unresolved_is_first_class_ok() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", transclusion_unresolved_body());
    let client = LoomClient::with_base_url(base_url);

    let resp = rt().block_on(async { client.get_loom_block_transclusion("WS-1", "BLK-1").await });
    let exchange = server.join().unwrap();

    let tx =
        resp.expect("an unresolved transclusion MUST be Ok (the unresolved state), NEVER an Err");
    assert!(
        !tx.resolved,
        "resolved=false preserved as a typed unresolved state"
    );
    assert_eq!(
        tx.unresolved_reason.as_deref(),
        Some("loom_block_has_no_source_document"),
        "the typed unresolved reason drives a visible indicator, not a silent blank"
    );
    assert!(
        tx.content_json.is_none(),
        "no source content when unresolved"
    );
    assert!(
        exchange
            .captured_request_line
            .starts_with("GET /workspaces/WS-1/loom/blocks/BLK-1/transclusion"),
        "transclusion hits GET .../blocks/:id/transclusion : {}",
        exchange.captured_request_line
    );
}

// ── AC: search-v2 parses a non-empty hit list; body carries NO fabricated embedding (RISK-7). ───────

#[test]
fn ac_search_v2_parses_non_empty_results_and_sends_no_embedding() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", search_v2_response_body());
    let client = LoomClient::with_base_url(base_url);
    let body = LoomSearchV2Request {
        query: "design".into(),
        ..Default::default()
    };

    let resp = rt().block_on(async { client.loom_search_v2("WS-1", &body).await });
    let exchange = server.join().unwrap();

    let result = resp.expect("loom_search_v2 must parse the real search response");
    let hits = result["hits"].as_array().expect("hits is an array");
    assert!(!hits.is_empty(), "the search result list is non-empty");
    // REAL backend shape (storage/loom.rs::LoomSearchV2Hit, L667): block identity is NESTED under
    // hits[i]["block"] (a full LoomBlock), never a flat hits[i]["block_id"].
    assert_eq!(hits[0]["block"]["block_id"], json!("BLK-1"));
    // Pin the per-modality blend field NAMES to the real hit shape so a fabricated flat/*_score fixture
    // can never silently pass again: these keys exist, the invented ones do not.
    assert!(
        hits[0].get("score").is_some()
            && hits[0].get("fts_rank").is_some()
            && hits[0].get("trgm_sim").is_some()
            && hits[0].get("vector_sim").is_some()
            && hits[0].get("edge_degree").is_some(),
        "a LoomSearchV2Hit carries the real per-modality blend fields: {}",
        hits[0]
    );
    assert!(
        hits[0].get("keyword_score").is_none()
            && hits[0].get("trigram_score").is_none()
            && hits[0].get("semantic_score").is_none()
            && hits[0].get("graph_score").is_none()
            && hits[0].get("block_id").is_none(),
        "no fabricated flat block_id / *_score keys exist on the real hit: {}",
        hits[0]
    );
    assert_eq!(result["semantic_available"], json!(false));
    assert!(
        exchange
            .captured_request_line
            .starts_with("POST /workspaces/WS-1/loom/search-v2"),
        "search-v2 hits POST /workspaces/:ws/loom/search-v2 : {}",
        exchange.captured_request_line
    );
    // RISK-7: the body NEVER carries a fabricated embedding.
    let sent: Value = serde_json::from_str(&exchange.captured_body).unwrap();
    assert_eq!(sent["query"], json!("design"));
    assert!(
        sent.get("embedding").is_none() && sent.get("query_embedding").is_none(),
        "search-v2 body must NOT carry a fabricated embedding: {sent}"
    );
}

// ── AC: journal open roundtrips a typed journal LoomBlock; malformed date is a typed BadRequest. ────

#[test]
fn ac_open_daily_journal_roundtrips_journal_block() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", journal_block_response_body());
    let client = LoomClient::with_base_url(base_url);

    let resp = rt().block_on(async { client.open_daily_journal("WS-1", "2026-06-23").await });
    let exchange = server.join().unwrap();

    let block = resp.expect("open_daily_journal must return the journal LoomBlock");
    assert_eq!(block.content_type, LoomBlockContentType::Journal);
    assert_eq!(block.journal_date.as_deref(), Some("2026-06-23"));
    assert!(
        exchange
            .captured_request_line
            .starts_with("PUT /workspaces/WS-1/loom/journals/2026-06-23"),
        "journal open hits PUT /workspaces/:ws/loom/journals/:date : {}",
        exchange.captured_request_line
    );
}

#[test]
fn ac_open_daily_journal_malformed_date_is_typed_bad_request() {
    let (base_url, server) = spawn_mock("HTTP/1.1 400 Bad Request", journal_date_400_body());
    let client = LoomClient::with_base_url(base_url);

    let resp = rt().block_on(async { client.open_daily_journal("WS-1", "not-a-date").await });
    let _ = server.join().unwrap();

    match resp {
        Err(LoomError::BadRequest(detail)) => {
            assert!(
                detail.contains("HSK-400-LOOM-JOURNAL-DATE"),
                "the backend 400 detail is surfaced: {detail}"
            );
        }
        other => panic!("a malformed journal date MUST be a typed BadRequest, got {other:?}"),
    }
}

// ── AC: markdown import returns the typed authority block (vault-never-authority). ──────────────────

#[test]
fn ac_import_markdown_returns_typed_authority_block() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", markdown_import_body());
    let client = LoomClient::with_base_url(base_url);
    let body = ImportMarkdownRequest {
        title: "Imported".into(),
        markdown: "# Heading\n\nbody".into(),
    };

    let resp = rt().block_on(async { client.import_markdown_to_loom("WS-1", &body).await });
    let exchange = server.join().unwrap();

    let imported = resp.expect("import_markdown_to_loom must return the typed LoomMarkdownImport");
    assert_eq!(
        imported.block.block_id, "BLK-1",
        "the new authority LoomBlock is typed"
    );
    assert_eq!(
        imported.rich_document_id, "KRD-1",
        "the backing rich-doc id is surfaced"
    );
    assert_eq!(
        imported.warnings,
        vec!["unsupported: footnotes".to_string()]
    );
    assert!(
        exchange
            .captured_request_line
            .starts_with("POST /workspaces/WS-1/loom/import/markdown"),
        "import hits POST /workspaces/:ws/loom/import/markdown : {}",
        exchange.captured_request_line
    );
}

// ── AC: a 404 (missing block) is a typed NotFound, not a transport error. ───────────────────────────

#[test]
fn ac_missing_block_is_typed_not_found() {
    let (base_url, server) = spawn_mock("HTTP/1.1 404 Not Found", json!({ "error": "not_found" }));
    let client = LoomClient::with_base_url(base_url);

    let resp = rt().block_on(async { client.get_loom_block("WS-1", "BLK-missing").await });
    let _ = server.join().unwrap();

    assert!(
        matches!(resp, Err(LoomError::NotFound(_))),
        "a 404 must be a typed NotFound (not a transport/generic error): {resp:?}"
    );
}

// ── AC: the unified namespace re-exports the existing MT-021..032 clients (no fork). ────────────────

#[test]
fn ac_unified_namespace_reexports_existing_clients() {
    // These types resolve through `handshake_native::backend::loom::<X>` AND remain the SAME type as
    // `handshake_native::backend_client::<X>` (a re-export, not a fork). The functions below COMPILE
    // only if each `loom::<X>` is identically the same type as `backend_client::<X>` — a value of one is
    // returned where the other is expected without conversion. This is a pure compile-time type-identity
    // proof (no construction, no runtime handle needed). If MT-038 had re-implemented (forked) any of
    // these, the types would differ and these functions would fail to compile.
    use handshake_native::backend::loom as unified;
    fn _same_search(
        c: handshake_native::backend_client::LoomSearchV2Client,
    ) -> unified::LoomSearchV2Client {
        c
    }
    fn _same_block_view(
        c: handshake_native::backend_client::BlockViewClient,
    ) -> unified::BlockViewClient {
        c
    }
    fn _same_folder(
        c: handshake_native::backend_client::LoomFolderClient,
    ) -> unified::LoomFolderClient {
        c
    }
    fn _same_block(
        c: handshake_native::backend_client::LoomBlockClient,
    ) -> unified::LoomBlockClient {
        c
    }
    fn _same_view_def(
        d: handshake_native::graph::block_collection_view::BlockViewDefinition,
    ) -> unified::BlockViewDefinition {
        d
    }
    // Reference each fn so they are not dead-code-eliminated before type-checking matters.
    let _ = (
        _same_search,
        _same_block_view,
        _same_folder,
        _same_block,
        _same_view_def,
    );
}
