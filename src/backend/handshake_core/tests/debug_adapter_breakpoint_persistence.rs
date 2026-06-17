//! WP-KERNEL-009 / MT-254 DebugAdapterCore — durable breakpoint persistence
//! against REAL Handshake-managed PostgreSQL + EventLedger.
//!
//! Proves the per-document breakpoint round-trip: set a breakpoint set on a real
//! RichDocument, read it back, replace it (PUT semantics), and confirm each write
//! left a real kernel_event_ledger receipt. No mock store.

mod knowledge_pg_support;

use handshake_core::storage::knowledge::{KnowledgeStore, NewKnowledgeRichDocument};
use handshake_core::storage::{Database, DebugBreakpointInput};
use knowledge_pg_support::knowledge_pg;
use serde_json::json;

fn rich_doc(workspace_id: &str) -> NewKnowledgeRichDocument {
    NewKnowledgeRichDocument {
        workspace_id: workspace_id.to_string(),
        document_id: None,
        title: "Debug Doc".to_string(),
        schema_version: "hsk_richdoc_v1".to_string(),
        content_json: json!({
            "type": "doc",
            "content": [{"type": "paragraph", "content": [{"type": "text", "text": "x"}]}]
        }),
        crdt_document_id: None,
        crdt_snapshot_id: None,
        promotion_receipt_event_id: None,
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn breakpoints_round_trip_with_real_event_ledger_receipt() {
    // Real-PG proof is contract-mandated (DEC-007): do NOT soft-pass when PG is
    // absent. If the Handshake-managed PostgreSQL is not reachable the test fails
    // loudly with ENVIRONMENT_BLOCKED rather than counting green on a silent skip.
    let pg = knowledge_pg().await.expect(
        "ENVIRONMENT_BLOCKED: MT-254 breakpoint persistence requires Handshake-managed PostgreSQL \
         (set DATABASE_URL or ensure managed PG is running)",
    );
    let workspace_id = pg.create_workspace().await;
    let doc = pg
        .db
        .create_knowledge_rich_document(rich_doc(&workspace_id))
        .await
        .expect("create rich document");
    let doc_id = doc.rich_document_id;

    // Initially empty.
    let empty = pg
        .db
        .list_debug_breakpoints(&doc_id)
        .await
        .expect("list empty");
    assert!(empty.is_empty(), "no breakpoints before any are set");

    // Set two breakpoints; the write must return a real EventLedger receipt id.
    let stored = pg
        .db
        .set_debug_breakpoints(
            &doc_id,
            &workspace_id,
            vec![
                DebugBreakpointInput {
                    source_url: "file:///x.js".to_string(),
                    line: 2,
                    condition: None,
                    verified: true,
                },
                DebugBreakpointInput {
                    source_url: "file:///x.js".to_string(),
                    line: 7,
                    condition: Some("a > 1".to_string()),
                    verified: false,
                },
            ],
        )
        .await
        .expect("set breakpoints");
    assert_eq!(stored.len(), 2, "both breakpoints persisted");
    assert!(
        stored[0].event_ledger_event_id.starts_with("KE-"),
        "real EventLedger receipt, got {}",
        stored[0].event_ledger_event_id
    );
    let receipt_id = stored[0].event_ledger_event_id.clone();

    // Read back: durable across a fresh query.
    let read = pg
        .db
        .list_debug_breakpoints(&doc_id)
        .await
        .expect("list after set");
    assert_eq!(read.len(), 2);
    assert_eq!(read[0].line, 2);
    assert!(read[0].verified);
    assert_eq!(read[1].line, 7);
    assert_eq!(read[1].condition.as_deref(), Some("a > 1"));

    // The receipt is a real kernel event in the ledger, found by aggregate.
    let events = pg
        .db
        .list_kernel_events_for_aggregate("debug_breakpoints", &doc_id)
        .await
        .expect("query ledger by aggregate");
    let receipt_event = events
        .iter()
        .find(|e| e.event_id == receipt_id)
        .expect("receipt event present in EventLedger");
    assert_eq!(
        receipt_event.payload.get("type").and_then(|v| v.as_str()),
        Some("knowledge_debug_breakpoints_recorded")
    );

    // PUT semantics: replacing the set removes line 7 and keeps only line 2.
    let replaced = pg
        .db
        .set_debug_breakpoints(
            &doc_id,
            &workspace_id,
            vec![DebugBreakpointInput {
                source_url: "file:///x.js".to_string(),
                line: 2,
                condition: None,
                verified: true,
            }],
        )
        .await
        .expect("replace breakpoints");
    assert_eq!(replaced.len(), 1, "PUT replaced the whole set");
    assert_eq!(replaced[0].line, 2);

    let after_replace = pg
        .db
        .list_debug_breakpoints(&doc_id)
        .await
        .expect("list after replace");
    assert_eq!(after_replace.len(), 1);
    assert_ne!(
        replaced[0].event_ledger_event_id, receipt_id,
        "the replace write left its OWN new receipt"
    );

    // Clearing breakpoints (empty set) leaves the doc with none.
    let cleared = pg
        .db
        .set_debug_breakpoints(&doc_id, &workspace_id, vec![])
        .await
        .expect("clear breakpoints");
    assert!(cleared.is_empty());
    assert!(pg
        .db
        .list_debug_breakpoints(&doc_id)
        .await
        .expect("list after clear")
        .is_empty());
}
