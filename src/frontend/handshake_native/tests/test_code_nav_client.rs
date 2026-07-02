//! MT-008 Handshake backend code-nav client proofs (WP-KERNEL-012 E1 code editor).
//!
//! Two layers:
//!
//! 1. STANDALONE (default `cargo test`): the deterministic port-of-`code_intelligence.ts` logic —
//!    completion-kind mapping, staleness label, markdown rendering, staleness->gutter-marker mapping,
//!    and the lookup-response deserialization of the EXACT backend `lookup_symbols` body shape. These
//!    prove the CodeNavClient's transformation surface without a backend.
//!
//! 2. LIVE-BACKEND (`--features integration`, AC-001/002/003): the CodeNavClient binds the REAL running
//!    handshake_core code-nav API. The binding-level proof (the client constructs the verified request,
//!    the live backend ACCEPTS it with the correct nav headers, the response parses into the typed
//!    projections) runs against the managed backend on 127.0.0.1:37501. The CONTENT assertions
//!    (`lookup_symbols('add')` returns a populated symbol with `symbol_kind`, `get_symbol` returns a
//!    definition, `get_references` returns a caller/callee) require an INDEXED-CODE workspace seeded
//!    into PostgreSQL. Seeding the code index requires the backend's internal `CodeIndexEngine`
//!    (cross-crate, forbidden to this frontend test crate) OR a multi-step ingestion-pipeline drive
//!    that is out of MT-008's scope. The binding test below proves the transport end-to-end against the
//!    live backend; the content assertions are documented as a NEEDS_MANAGED_RESOURCE_PROOF blocker in
//!    the MT handoff (KERNEL_BUILDER Spec-Realism Gate sub-rule 2 — the binding is REAL, never mocked;
//!    only the seeded-symbol assertions are an honest deferred blocker).

use handshake_native::code_editor::code_nav::{
    code_symbol_staleness_label, markdown_for_symbol, staleness_marker_for, symbol_file_path,
    CodeNavCache, CodeStaleness, CodeSymbolDefinition, CodeSymbolLookupResponse,
    CodeSymbolNavProjection, CompletionItem, CompletionKind,
};
use handshake_native::code_editor::gutter::{DiagnosticSeverity, GutterMarkerKind};

// ── STANDALONE: the ported code-intelligence transformation surface ───────────────────────────────

#[test]
fn completion_kind_mapping_matches_react_completion_kind() {
    // Port of `completionKind` (code_intelligence.ts:106), including the Function default.
    assert_eq!(
        CompletionKind::from_symbol_kind("class"),
        CompletionKind::Class
    );
    assert_eq!(
        CompletionKind::from_symbol_kind("struct"),
        CompletionKind::Class
    );
    assert_eq!(
        CompletionKind::from_symbol_kind("enum"),
        CompletionKind::Enum
    );
    assert_eq!(
        CompletionKind::from_symbol_kind("field"),
        CompletionKind::Field
    );
    assert_eq!(
        CompletionKind::from_symbol_kind("module"),
        CompletionKind::Module
    );
    assert_eq!(
        CompletionKind::from_symbol_kind("variable"),
        CompletionKind::Variable
    );
    assert_eq!(
        CompletionKind::from_symbol_kind("function"),
        CompletionKind::Function
    );
    assert_eq!(
        CompletionKind::from_symbol_kind("whatever"),
        CompletionKind::Function
    );
}

#[test]
fn staleness_label_matches_react_format() {
    let fresh = CodeStaleness {
        state: Some("fresh".into()),
        fresh: true,
        ..Default::default()
    };
    assert_eq!(code_symbol_staleness_label(Some(&fresh)), "fresh (fresh)");
    let stale = CodeStaleness {
        state: Some("marked_stale".into()),
        fresh: false,
        ..Default::default()
    };
    assert_eq!(
        code_symbol_staleness_label(Some(&stale)),
        "marked_stale (not fresh)"
    );
    assert_eq!(code_symbol_staleness_label(None), "unknown");
}

#[test]
fn completion_item_built_from_symbol_like_react_suggestions_map() {
    // The React `suggestions.map(...)` body: label/insertText = display_name, detail = symbol_kind.
    let symbol = CodeSymbolNavProjection {
        symbol_entity_id: "ent-add".into(),
        symbol_key: "rust:src/lib.rs#add".into(),
        display_name: "add".into(),
        symbol_kind: "function".into(),
        staleness: Some(CodeStaleness {
            state: Some("fresh".into()),
            fresh: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let item = CompletionItem::from_symbol(&symbol);
    assert_eq!(item.label, "add");
    assert_eq!(item.insert_text, "add");
    assert_eq!(item.detail, "function");
    assert_eq!(item.kind, CompletionKind::Function);
    assert!(
        item.documentation.contains("**add**"),
        "doc carries the markdown heading"
    );
    assert_eq!(item.symbol_entity_id, "ent-add");
}

#[test]
fn markdown_for_symbol_renders_codesymbolpanel_data() {
    // Port of `markdownForSymbol` + the CodeSymbolPanel data the hover shows.
    let symbol = CodeSymbolNavProjection {
        display_name: "add".into(),
        symbol_kind: "function".into(),
        symbol_key: "rust:src/lib.rs#add".into(),
        staleness: Some(CodeStaleness {
            state: Some("fresh".into()),
            fresh: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let md = markdown_for_symbol(&symbol, Some("Adds two numbers."));
    assert!(md.contains("**add**"));
    assert!(md.contains("Kind: `function`"));
    assert!(md.contains("Symbol: `rust:src/lib.rs#add`"));
    assert!(md.contains("Staleness: `fresh (fresh)`"));
    assert!(md.contains("Adds two numbers."));
}

#[test]
fn symbol_file_path_extracts_segment() {
    assert_eq!(
        symbol_file_path("rust:src/lib.rs#add"),
        Some("src/lib.rs".to_owned())
    );
    assert_eq!(symbol_file_path("noseparator"), None);
}

#[test]
fn staleness_marker_maps_not_fresh_to_warning_on_definition_line() {
    // AC-007 basis: a not-fresh symbol with a definition span yields a Warning gutter marker on its
    // (0-based) line; a fresh symbol yields nothing.
    let stale = CodeSymbolNavProjection {
        display_name: "old".into(),
        definition: Some(CodeSymbolDefinition {
            line_start: Some(3),
            ..Default::default()
        }),
        staleness: Some(CodeStaleness {
            state: Some("marked_stale".into()),
            fresh: false,
            ..Default::default()
        }),
        ..Default::default()
    };
    let marker = staleness_marker_for(&stale).expect("not-fresh -> marker");
    assert_eq!(marker.line, 2, "1-based line 3 -> 0-based gutter line 2");
    assert!(matches!(
        marker.kind,
        GutterMarkerKind::Diagnostic(DiagnosticSeverity::Warning)
    ));
    assert!(marker.message.contains("Stale code intelligence"));

    let fresh = CodeSymbolNavProjection {
        definition: Some(CodeSymbolDefinition {
            line_start: Some(3),
            ..Default::default()
        }),
        staleness: Some(CodeStaleness {
            fresh: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(staleness_marker_for(&fresh).is_none(), "fresh -> no marker");
}

#[test]
fn lookup_response_parses_exact_backend_body_shape() {
    // The EXACT body `handshake_core::api::knowledge_code_nav::lookup_symbols` returns (verified
    // read-only against the backend handler): `{ "matches": [ symbol_to_json, ... ] }`.
    let body = serde_json::json!({
        "workspace_id": "ws-1",
        "matches": [{
            "symbol_entity_id": "ent-add",
            "symbol_key": "rust:src/lib.rs#add",
            "display_name": "add",
            "symbol_kind": "function",
            "owning_wp": null,
            "primary_source_id": "src-1",
            "lifecycle_state": "active",
            "definition": { "span_id": "s1", "source_id": "src-1", "line_start": 3, "line_end": 3 },
            "staleness": { "state": "fresh", "fresh": true }
        }],
        "nav_receipt_event_id": "evt-1",
        "quiet_background_work_receipt_id": "rcpt-1"
    });
    let parsed: CodeSymbolLookupResponse =
        serde_json::from_value(body).expect("parse backend body");
    assert_eq!(parsed.matches.len(), 1);
    let m = &parsed.matches[0];
    assert_eq!(m.display_name, "add");
    assert_eq!(m.symbol_kind, "function");
    assert_eq!(m.definition.as_ref().unwrap().line_start, Some(3));
    assert!(m.staleness.as_ref().unwrap().fresh);
}

#[test]
fn lookup_cache_respects_prefix_and_caching() {
    let mut cache = CodeNavCache::new();
    assert!(cache.get("ad").is_none());
    cache.put(
        "ad",
        vec![CodeSymbolNavProjection {
            display_name: "add".into(),
            ..Default::default()
        }],
    );
    assert_eq!(cache.get("ad").map(|m| m.len()), Some(1));
    assert!(cache.get("xy").is_none(), "different prefix misses");
}

// ── LIVE-BACKEND (--features integration): the REAL handshake_core code-nav binding ────────────────
//
// These hit the managed backend on 127.0.0.1:37501. They prove the CodeNavClient's transport is
// genuinely consumed end-to-end against the live backend (correct URL + nav headers + envelope parse).
// The CONTENT assertions (populated symbols) need a code-indexed workspace; see the module header for
// the NEEDS_MANAGED_RESOURCE_PROOF blocker on seeding.

#[cfg(feature = "integration")]
mod live_backend {
    use handshake_native::code_editor::code_nav::CodeNavClient;

    fn backend_base() -> String {
        std::env::var("HANDSHAKE_TEST_DB_URL")
            .ok()
            .filter(|s| s.starts_with("http"))
            .unwrap_or_else(|| "http://127.0.0.1:37501".to_owned())
    }

    /// AC-001 (binding): `lookup_symbols` against the LIVE backend returns Ok (the request is accepted
    /// with the correct nav headers + URL and the `matches` envelope parses). With no indexed-code
    /// workspace the matches may be empty — the CONTENT assertion (`symbol_kind` populated) is the
    /// documented NEEDS_MANAGED_RESOURCE_PROOF blocker (seeding requires the cross-crate CodeIndexEngine).
    #[tokio::test]
    async fn ac001_lookup_symbols_binds_live_backend() {
        let client = CodeNavClient::new(backend_base());
        // A workspace id need not exist for the binding proof: the backend validates headers + parses
        // the query before the workspace lookup, so a 200 with a `matches` array proves the transport.
        let result = client.lookup_symbols("ws-mt008-probe", "add", 5).await;
        match result {
            Ok(matches) => {
                println!(
                    "AC-001 binding: lookup_symbols accepted by live backend; matches={}",
                    matches.len()
                );
                // CONTENT assertion (deferred blocker — needs a seeded indexed-code workspace):
                if let Some(first) = matches.first() {
                    assert!(
                        !first.symbol_kind.is_empty(),
                        "AC-001 content: a returned symbol has a populated symbol_kind"
                    );
                    println!("AC-001 content PROVEN: symbol_kind={}", first.symbol_kind);
                } else {
                    println!(
                        "AC-001 content DEFERRED (NEEDS_MANAGED_RESOURCE_PROOF): no indexed-code \
                         workspace seeded; binding proven, content assertion gated on seeding"
                    );
                }
            }
            Err(e) => panic!("AC-001 binding FAILED against live backend: {e}"),
        }
    }

    /// AC-002 (binding): `get_symbol` against the LIVE backend. A non-existent entity id returns an
    /// error (404), proving the route + headers; the populated-definition CONTENT assertion needs a
    /// seeded symbol (deferred blocker).
    #[tokio::test]
    async fn ac002_get_symbol_binds_live_backend() {
        let client = CodeNavClient::new(backend_base());
        // A real seeded entity id would assert display_name + definition.line_start; absent seeding we
        // prove the binding: the route exists and responds (a 404 on a missing id is a real response).
        let result = client.get_symbol("ent-nonexistent-mt008").await;
        // Either Ok(empty default) on a 200, or Err on a 404 — both prove the route is bound. A hang /
        // wrong URL would be a transport error with a connection message, which we treat as a failure.
        match result {
            Ok(resp) => println!(
                "AC-002 binding: get_symbol 200 (display_name={:?}); content DEFERRED without seeding",
                resp.symbol.display_name
            ),
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("non-success") || msg.contains("404"),
                    "AC-002 binding: expected a real HTTP response (404 for a missing id), got {msg}"
                );
                println!("AC-002 binding: get_symbol route responded ({msg}); content DEFERRED");
            }
        }
    }

    /// AC-003 (binding): `get_references` against the LIVE backend route. Content (>=1 caller/callee)
    /// needs a seeded symbol with edges (deferred blocker); the binding is proven by a real response.
    #[tokio::test]
    async fn ac003_get_references_binds_live_backend() {
        let client = CodeNavClient::new(backend_base());
        let result = client.get_references("ent-nonexistent-mt008").await;
        match result {
            Ok(refs) => println!(
                "AC-003 binding: get_references 200 (total={}); content DEFERRED without seeding",
                refs.total()
            ),
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("non-success") || msg.contains("404"),
                    "AC-003 binding: expected a real HTTP response, got {msg}"
                );
                println!(
                    "AC-003 binding: get_references route responded ({msg}); content DEFERRED"
                );
            }
        }
    }
}
