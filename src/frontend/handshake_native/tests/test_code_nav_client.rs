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
//!    handshake_core code-nav API backed by Handshake-managed PostgreSQL. The existing
//!    `mt249_code_intelligence_fixture` seeds `add` plus its `caller` through the real CodeIndexEngine
//!    and prints the base URL, workspace id, and symbol entity id. The runner supplies those values as
//!    `HANDSHAKE_TEST_DB_URL`, `HANDSHAKE_TEST_WORKSPACE_ID`, and
//!    `HANDSHAKE_TEST_CODE_SYMBOL_ENTITY_ID`. Missing fixture values are a hard test failure when the
//!    integration feature is explicitly enabled; empty results and typed 404s are never accepted as
//!    populated-content proof.

use handshake_native::code_editor::code_nav::{
    code_symbol_staleness_label, markdown_for_symbol, preferred_symbol_for_identifier,
    preferred_symbol_for_identifier_in_file, staleness_marker_for, symbol_file_path, CodeNavCache,
    CodeStaleness, CodeSymbolDefinition, CodeSymbolLookupResponse, CodeSymbolNavProjection,
    CompletionItem, CompletionKind,
};
use handshake_native::code_editor::gutter::{DiagnosticSeverity, GutterMarkerKind};

fn spawn_code_nav_response(response: Vec<u8>) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind code-nav negative-path server");
    let base_url = format!(
        "http://{}",
        listener.local_addr().expect("server local address")
    );
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept code-nav request");
        let mut buffer = [0_u8; 2048];
        let _ = stream.read(&mut buffer);
        if !response.is_empty() {
            stream
                .write_all(&response)
                .expect("write code-nav response");
            stream.flush().expect("flush code-nav response");
        }
    });
    (base_url, server)
}

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
fn lookup_cache_respects_workspace_prefix_and_client_invalidation() {
    let mut cache = CodeNavCache::new();
    assert!(cache.get("ws-a", "ad").is_none());
    cache.put(
        "ws-a",
        "ad",
        vec![CodeSymbolNavProjection {
            display_name: "add".into(),
            ..Default::default()
        }],
    );
    assert_eq!(cache.get("ws-a", "ad").map(|m| m.len()), Some(1));
    assert!(
        cache.get("ws-b", "ad").is_none(),
        "the same prefix cannot reuse another workspace's symbols"
    );
    assert!(cache.get("ws-a", "xy").is_none(), "different prefix misses");
    cache.clear();
    assert!(
        cache.get("ws-a", "ad").is_none(),
        "client-change invalidation clears the old response"
    );
}

#[test]
fn exact_identifier_wins_over_earlier_prefix_sibling() {
    let selected = preferred_symbol_for_identifier(
        vec![
            CodeSymbolNavProjection {
                symbol_entity_id: "ent-address".into(),
                display_name: "address".into(),
                ..Default::default()
            },
            CodeSymbolNavProjection {
                symbol_entity_id: "ent-add".into(),
                display_name: "add".into(),
                ..Default::default()
            },
        ],
        "add",
    )
    .expect("prefix response contains the exact identifier");
    assert_eq!(selected.symbol_entity_id, "ent-add");
    assert_eq!(selected.display_name, "add");
}

#[test]
fn prefix_sibling_without_exact_identifier_is_not_a_semantic_match() {
    let selected = preferred_symbol_for_identifier(
        vec![CodeSymbolNavProjection {
            symbol_entity_id: "ent-address".into(),
            display_name: "address".into(),
            ..Default::default()
        }],
        "add",
    );
    assert!(
        selected.is_none(),
        "semantic actions must not bind `add` to the prefix sibling `address`"
    );
}

#[test]
fn duplicate_exact_identifiers_are_rejected_as_ambiguous_in_any_backend_order() {
    let in_math = CodeSymbolNavProjection {
        symbol_entity_id: "ent-math-add".into(),
        symbol_key: "function:src/math.rs#add".into(),
        display_name: "add".into(),
        definition: Some(CodeSymbolDefinition {
            source_id: Some("source-math".into()),
            line_start: Some(3),
            line_end: Some(3),
        }),
        ..Default::default()
    };
    let in_utils = CodeSymbolNavProjection {
        symbol_entity_id: "ent-utils-add".into(),
        symbol_key: "function:src/utils.rs#add".into(),
        display_name: "add".into(),
        definition: Some(CodeSymbolDefinition {
            source_id: Some("source-utils".into()),
            line_start: Some(17),
            line_end: Some(17),
        }),
        ..Default::default()
    };

    for symbols in [
        vec![in_math.clone(), in_utils.clone()],
        vec![in_utils, in_math],
    ] {
        assert!(
            preferred_symbol_for_identifier(symbols, "add").is_none(),
            "an exact duplicate name across source files must not resolve from backend order"
        );
    }
}

#[test]
fn duplicate_exact_identifier_prefers_the_active_document_source_in_any_backend_order() {
    let in_math = CodeSymbolNavProjection {
        symbol_entity_id: "ent-math-add".into(),
        symbol_key: "function:src/math.rs#add".into(),
        display_name: "add".into(),
        ..Default::default()
    };
    let in_utils = CodeSymbolNavProjection {
        symbol_entity_id: "ent-utils-add".into(),
        symbol_key: "function:src/utils.rs#add".into(),
        display_name: "add".into(),
        ..Default::default()
    };

    for symbols in [
        vec![in_math.clone(), in_utils.clone()],
        vec![in_utils.clone(), in_math],
    ] {
        let selected =
            preferred_symbol_for_identifier_in_file(symbols, "add", r"D:\workspace\src\utils.rs")
                .expect("the active document path disambiguates the exact symbol");
        assert_eq!(selected.symbol_entity_id, "ent-utils-add");
    }
}

#[test]
fn direct_declaration_beats_same_file_inherent_impl_in_any_backend_order() {
    let declaration = CodeSymbolNavProjection {
        symbol_entity_id: "ent-struct".into(),
        symbol_key: "rust:src/model.rs#Model".into(),
        display_name: "Model".into(),
        symbol_kind: "struct".into(),
        ..Default::default()
    };
    let inherent_impl = CodeSymbolNavProjection {
        symbol_entity_id: "ent-inherent-impl".into(),
        symbol_key: "rust:src/model.rs#impl Model~inherent".into(),
        display_name: "Model".into(),
        symbol_kind: "impl".into(),
        ..Default::default()
    };

    for symbols in [
        vec![declaration.clone(), inherent_impl.clone()],
        vec![inherent_impl, declaration],
    ] {
        let selected =
            preferred_symbol_for_identifier_in_file(symbols, "Model", r"D:\workspace\src\model.rs")
                .expect("the unique direct declaration disambiguates its inherent impl projection");
        assert_eq!(selected.symbol_entity_id, "ent-struct");
    }
}

#[test]
fn lone_impl_projection_remains_a_valid_fallback() {
    let selected = preferred_symbol_for_identifier_in_file(
        vec![CodeSymbolNavProjection {
            symbol_entity_id: "ent-inherent-impl".into(),
            symbol_key: "rust:src/model.rs#impl Model~inherent".into(),
            display_name: "Model".into(),
            symbol_kind: "impl".into(),
            ..Default::default()
        }],
        "Model",
        r"D:\workspace\src\model.rs",
    )
    .expect("an impl remains usable when no declaration projection exists");
    assert_eq!(selected.symbol_entity_id, "ent-inherent-impl");
}

#[test]
fn duplicate_same_file_declarations_remain_ambiguous() {
    let declarations = ["ent-struct-a", "ent-struct-b"].map(|entity_id| CodeSymbolNavProjection {
        symbol_entity_id: entity_id.into(),
        symbol_key: format!("rust:src/model.rs#Model-{entity_id}"),
        display_name: "Model".into(),
        symbol_kind: "struct".into(),
        ..Default::default()
    });
    assert!(
        preferred_symbol_for_identifier_in_file(
            declarations.into_iter().collect(),
            "Model",
            r"D:\workspace\src\model.rs",
        )
        .is_none(),
        "two same-file declarations must not resolve from backend order"
    );
}

#[tokio::test]
async fn code_nav_http_500_is_typed_error_not_empty_success() {
    use handshake_native::code_editor::code_nav::CodeNavClient;

    let (base_url, server) = spawn_code_nav_response(
        b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec(),
    );
    let error = CodeNavClient::new(base_url)
        .lookup_symbols("ws-negative", "add", 5)
        .await
        .expect_err("HTTP 500 must not become an empty successful lookup");
    server.join().expect("500 server exits");
    assert!(error.to_string().contains("non-success"));
}

#[tokio::test]
async fn code_nav_malformed_json_is_typed_error_not_empty_success() {
    use handshake_native::code_editor::code_nav::CodeNavClient;

    let body = b"{not-json";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        String::from_utf8_lossy(body)
    );
    let (base_url, server) = spawn_code_nav_response(response.into_bytes());
    let error = CodeNavClient::new(base_url)
        .lookup_symbols("ws-negative", "add", 5)
        .await
        .expect_err("malformed JSON must not become an empty successful lookup");
    server.join().expect("malformed server exits");
    assert!(error.to_string().to_ascii_lowercase().contains("parse"));
}

#[tokio::test]
async fn code_nav_dropped_connection_is_typed_error_not_empty_success() {
    use handshake_native::code_editor::code_nav::CodeNavClient;

    let (base_url, server) = spawn_code_nav_response(Vec::new());
    let error = CodeNavClient::new(base_url)
        .lookup_symbols("ws-negative", "add", 5)
        .await
        .expect_err("dropped connection must not become an empty successful lookup");
    server.join().expect("drop server exits");
    assert!(!error.to_string().trim().is_empty());
}

// ── LIVE-BACKEND (--features integration): the REAL handshake_core code-nav binding ────────────────
//
// These consume the ready values printed by the real managed-PostgreSQL fixture. The integration feature
// is the explicit resource gate; once enabled, every required value and every populated result is strict.

#[cfg(feature = "integration")]
mod live_backend {
    use handshake_native::code_editor::code_nav::CodeNavClient;

    struct Fixture {
        base_url: String,
        workspace_id: String,
        symbol_entity_id: String,
    }

    fn required_env(name: &str) -> String {
        std::env::var(name)
            .unwrap_or_else(|_| panic!("MT-008 live proof requires {name} from the ready fixture"))
    }

    fn fixture() -> Fixture {
        let base_url = required_env("HANDSHAKE_TEST_DB_URL");
        assert!(
            base_url.starts_with("http://") || base_url.starts_with("https://"),
            "HANDSHAKE_TEST_DB_URL must be the fixture HTTP base URL; got {base_url:?}"
        );
        Fixture {
            base_url,
            workspace_id: required_env("HANDSHAKE_TEST_WORKSPACE_ID"),
            symbol_entity_id: required_env("HANDSHAKE_TEST_CODE_SYMBOL_ENTITY_ID"),
        }
    }

    /// AC-001: the native client consumes the populated real-backend lookup result.
    #[tokio::test]
    async fn ac001_lookup_symbols_returns_populated_live_symbol() {
        let fixture = fixture();
        let client = CodeNavClient::new(&fixture.base_url);
        let matches = client
            .lookup_symbols(&fixture.workspace_id, "add", 5)
            .await
            .expect("AC-001 live lookup succeeds");
        let add = matches
            .iter()
            .find(|symbol| symbol.display_name == "add")
            .expect("AC-001 lookup returns the seeded add symbol");
        assert_eq!(add.symbol_entity_id, fixture.symbol_entity_id);
        assert!(
            !add.symbol_kind.trim().is_empty(),
            "symbol_kind is populated"
        );
        assert!(
            add.definition
                .as_ref()
                .and_then(|definition| definition.line_start)
                .unwrap_or_default()
                > 0,
            "definition.line_start is populated"
        );
        assert!(add.staleness.is_some(), "served staleness is populated");
        println!(
            "AC-001 populated live symbol: id={} display_name={} symbol_kind={} definition.line_start={:?}",
            add.symbol_entity_id,
            add.display_name,
            add.symbol_kind,
            add.definition.as_ref().and_then(|definition| definition.line_start)
        );
    }

    /// AC-002: detail returns the seeded symbol and its definition span.
    #[tokio::test]
    async fn ac002_get_symbol_returns_populated_live_definition() {
        let fixture = fixture();
        let client = CodeNavClient::new(&fixture.base_url);
        let response = client
            .get_symbol(&fixture.symbol_entity_id)
            .await
            .expect("AC-002 live symbol detail succeeds");
        assert_eq!(response.symbol.symbol_entity_id, fixture.symbol_entity_id);
        assert_eq!(response.symbol.display_name, "add");
        assert!(!response.symbol.symbol_kind.trim().is_empty());
        let line_start = response
            .symbol
            .definition
            .as_ref()
            .and_then(|definition| definition.line_start)
            .expect("AC-002 definition.line_start is populated");
        assert!(line_start > 0);
        println!(
            "AC-002 populated live hover/detail: display_name={} definition.line_start={line_start}",
            response.symbol.display_name
        );
    }

    /// AC-003: `add` has the real indexed `caller` incoming edge.
    #[tokio::test]
    async fn ac003_get_references_returns_populated_live_caller() {
        let fixture = fixture();
        let client = CodeNavClient::new(&fixture.base_url);
        let references = client
            .get_references(&fixture.symbol_entity_id)
            .await
            .expect("AC-003 live references succeeds");
        assert!(references.total() >= 1, "at least one caller or callee");
        assert!(
            references
                .callers
                .iter()
                .any(|caller| caller.display_name == "caller"),
            "the seeded caller appears in callers: {:?}",
            references
                .callers
                .iter()
                .map(|caller| caller.display_name.as_str())
                .collect::<Vec<_>>()
        );
        println!(
            "AC-003 populated live references: callers={} callees={} total={}",
            references.callers.len(),
            references.callees.len(),
            references.total()
        );
    }
}
