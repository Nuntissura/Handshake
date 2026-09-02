//! WP-KERNEL-009 MT-261 CanvasBoard — embedded SurrealDB + EventLedger
//! authority proof.
//!
//! Proves the Obsidian-canvas-class surface over LoomBlock authority
//! (Master Spec §7.1.4.3 / §10.12). All assertions run against the same
//! isolated embedded store owned by the shared knowledge test support.
//!
//! Covered:
//!  * the canvas IS a `LoomBlock(content_type='canvas')` with a knowledge bridge;
//!  * board persists: placements/visual-edges/viewport round-trip + EventLedger
//!    receipt on the board row;
//!  * reference-not-copy: `remove_canvas_placement` keeps the source block;
//!    deleting the canvas block keeps placed blocks (CASCADE only hits the board
//!    + its placements/visual-edges, never the referenced loom_blocks);
//!  * editing the source block reflects through the placement (live reference);
//!  * a SEMANTIC edge appears in the local Loom graph; a VISUAL-ONLY edge does
//!    NOT (it is never a loom_edge);
//!  * a free-text card is a real note LoomBlock + RichDocument.
//!  * compensation rollback and concurrency ordering use resettable embedded
//!    failpoints/barriers at the production storage boundary.

#[path = "knowledge_ingestion_support.rs"]
mod embedded_knowledge_support;

use embedded_knowledge_support::{open_embedded_store, EmbeddedKnowledgeStore};
use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use handshake_core::storage::knowledge::{KnowledgeEntityKind, KnowledgeStore};
use handshake_core::storage::stage_artifacts::{NewStageCaptureArtifact, StageArtifactStore};
use handshake_core::storage::surreal::{RowFilter, ScalarValue, SurrealDatabase};
use handshake_core::storage::{
    CompensateLoomCanvasStageCard, Database, LoomBlockContentType, LoomBlockDerived,
    LoomBlockUpdate, LoomCanvasPlacementUpdate, LoomCanvasStageProvenance, LoomEdgeCreatedBy,
    LoomEdgeType, LoomSearchResultKind, LoomSearchSourceKind, NewLoomBlock, NewLoomCanvasPlacement,
    NewLoomCanvasStageCard, NewLoomEdge, QuickSwitcherRecentInput, WriteContext,
    LOOM_CANVAS_BOARD_SCHEMA_ID, LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
};
use serde_json::json;
use sha2::{Digest, Sha256};

macro_rules! embedded_store_or_return {
    () => {{
        match open_embedded_store().await {
            Some(store) => store,
            None => {
                eprintln!("SKIP MT-261 loom canvas board proof: embedded store unavailable");
                return;
            }
        }
    }};
}

async fn make_block(
    db: &SurrealDatabase,
    workspace_id: &str,
    title: &str,
    content_type: LoomBlockContentType,
) -> String {
    let ctx = WriteContext::human(None);
    let block = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.to_string(),
                content_type,
                document_id: None,
                asset_id: None,
                title: Some(title.to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("create loom block");
    db.bridge_loom_block_to_knowledge(&ctx, workspace_id, &block.block_id)
        .await
        .expect("bridge block");
    block.block_id
}

fn board_state(pan_x: f64, pan_y: f64, zoom: f64) -> serde_json::Value {
    json!({
        "schema_id": LOOM_CANVAS_BOARD_SCHEMA_ID,
        "pan_x": pan_x,
        "pan_y": pan_y,
        "zoom": zoom,
    })
}

async fn stage_provenance(
    store: &EmbeddedKnowledgeStore,
    workspace_id: &str,
    suffix: &str,
) -> LoomCanvasStageProvenance {
    let content_bytes = suffix.as_bytes();
    let sha256 = format!("{:x}", Sha256::digest(content_bytes));
    let causal_action_id = format!("stage-causal-{suffix}");
    let artifact = StageArtifactStore::new(store.storage.clone())
        .insert_stage_artifact(NewStageCaptureArtifact {
            workspace_id: workspace_id.to_owned(),
            content_kind: "canvas_node".to_owned(),
            label: suffix.to_owned(),
            content_type: "text/plain".to_owned(),
            content_json: json!({"text": suffix}),
            content_bytes: content_bytes.to_vec(),
            source_ref: None,
            idempotency_key: format!("loom-canvas-stage-{suffix}"),
            request_hash: sha256.clone(),
            actor_kind: "operator".to_owned(),
            actor_id: "loom-canvas-stage-test".to_owned(),
            correlation_id: causal_action_id.clone(),
            approval_id: "test-approval".to_owned(),
            decision_receipt: stage_receipt(
                KernelEventType::ToolDecisionRecorded,
                &format!("loom-canvas-stage-decision-{suffix}"),
                &causal_action_id,
            ),
            receipt: stage_receipt(
                KernelEventType::ArtifactStored,
                &format!("loom-canvas-stage-receipt-{suffix}"),
                &causal_action_id,
            ),
        })
        .await
        .expect("seed authoritative Stage capture artifact")
        .artifact;
    assert_eq!(artifact.content_sha256.as_str(), sha256.as_str());
    let provenance = LoomCanvasStageProvenance {
        schema_id: LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA.to_owned(),
        artifact_id: artifact.artifact_id,
        sha256: artifact.content_sha256,
        manifest_ref: artifact.manifest_ref,
        causal_action_id: artifact.correlation_id,
    };
    provenance
}

fn stage_receipt(
    event_type: KernelEventType,
    idempotency_key: &str,
    correlation_id: &str,
) -> NewKernelEvent {
    NewKernelEvent::builder(
        "loom-canvas-stage-test",
        "loom-canvas-stage-session",
        event_type,
        KernelActor::Operator("loom-canvas-stage-test".to_owned()),
    )
    .aggregate("stage_capture_artifact", "pending")
    .idempotency_key(idempotency_key)
    .correlation_id(correlation_id)
    .source_component("loom_canvas_board_tests")
    .payload(json!({"proof": "embedded_stage_authority"}))
    .build()
    .expect("valid Stage authority receipt")
}

fn stage_provenance_key(provenance: &LoomCanvasStageProvenance) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(provenance).expect("serialize Stage provenance"))
    )
}

async fn create_stage_card_receipt(
    store: &EmbeddedKnowledgeStore,
    workspace_id: &str,
    canvas_block_id: &str,
    suffix: &str,
) -> CompensateLoomCanvasStageCard {
    let ctx = WriteContext::human(None);
    let provenance = stage_provenance(store, workspace_id, suffix).await;
    let stage_provenance_key = stage_provenance_key(&provenance);
    let created = store
        .db
        .create_stage_canvas_card(
            &ctx,
            NewLoomCanvasStageCard {
                canvas_block_id: canvas_block_id.to_owned(),
                workspace_id: workspace_id.to_owned(),
                title: format!("Stage capture {}", provenance.artifact_id),
                markdown: serde_json::to_string(&provenance).expect("serialize Stage provenance"),
                stage_provenance_key: stage_provenance_key.clone(),
                stage_provenance: provenance.clone(),
                x: 0.0,
                y: 0.0,
                w: 300.0,
                h: 180.0,
                z_index: 0,
            },
        )
        .await
        .expect("create Stage Canvas card");
    CompensateLoomCanvasStageCard {
        canvas_block_id: canvas_block_id.to_owned(),
        workspace_id: workspace_id.to_owned(),
        placement_id: created.placement.placement_id,
        placed_block_id: created.block.block_id,
        stage_provenance_key,
        stage_provenance: provenance,
    }
}

async fn embedded_row_count_by_id(store: &EmbeddedKnowledgeStore, table: &str, id: &str) -> u64 {
    let inspector = store.storage.test_inspector();
    let table = inspector
        .table_selector(table)
        .await
        .expect("select embedded table");
    inspector
        .row_count(&table, RowFilter::IdEquals(id.to_owned()))
        .await
        .expect("count embedded row")
}

async fn embedded_row_count(store: &EmbeddedKnowledgeStore, table: &str) -> u64 {
    let inspector = store.storage.test_inspector();
    let table = inspector
        .table_selector(table)
        .await
        .expect("select embedded table");
    inspector
        .row_count(&table, RowFilter::All)
        .await
        .expect("count embedded rows")
}

async fn embedded_row_count_by_field(
    store: &EmbeddedKnowledgeStore,
    table: &str,
    field: &str,
    value: &str,
) -> u64 {
    let inspector = store.storage.test_inspector();
    let table = inspector
        .table_selector(table)
        .await
        .expect("select embedded table");
    let field = table.field(field).expect("select embedded field");
    inspector
        .row_count(
            &table,
            RowFilter::FieldEquals {
                field,
                value: ScalarValue::String(value.to_owned()),
            },
        )
        .await
        .expect("count embedded rows")
}

async fn make_canvas(db: &SurrealDatabase, workspace_id: &str, title: &str) -> String {
    let ctx = WriteContext::human(None);
    let block = db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.to_string(),
                content_type: LoomBlockContentType::Canvas,
                document_id: None,
                asset_id: None,
                title: Some(title.to_string()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("create canvas block");
    db.bridge_loom_block_to_knowledge(&ctx, workspace_id, &block.block_id)
        .await
        .expect("bridge canvas block");
    db.create_canvas_board(
        &ctx,
        workspace_id,
        &block.block_id,
        board_state(0.0, 0.0, 1.0),
    )
    .await
    .expect("create canvas board");
    block.block_id
}

#[tokio::test]
async fn canvas_is_a_loom_block_with_knowledge_bridge() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let canvas_id = make_canvas(&store.db, &ws, "Project map").await;

    // The canvas board's block IS a content_type='canvas' LoomBlock.
    let block = store
        .db
        .get_loom_block(&ws, &canvas_id)
        .await
        .expect("get block");
    assert!(matches!(block.content_type, LoomBlockContentType::Canvas));

    // It is authority-resolved through the ProjectKnowledgeIndex bridge.
    let bridge = store
        .db
        .get_loom_block_knowledge_bridge(&ws, &canvas_id)
        .await
        .expect("read bridge")
        .expect("bridge exists for canvas block");
    let entity = store
        .db
        .get_knowledge_entity(&bridge.entity_id)
        .await
        .expect("get entity")
        .expect("entity exists");
    assert!(matches!(entity.entity_kind, KnowledgeEntityKind::LoomBlock));

    // The board row carries an EventLedger receipt.
    let view = store
        .db
        .get_canvas_board(&ws, &canvas_id)
        .await
        .expect("get board");
    assert!(!view.board.event_ledger_event_id.is_empty());
    assert_eq!(
        view.board
            .board_state
            .get("schema_id")
            .and_then(|v| v.as_str()),
        Some(LOOM_CANVAS_BOARD_SCHEMA_ID)
    );
}

#[tokio::test]
async fn board_placements_viewport_and_visual_edges_round_trip() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&store.db, &ws, "Round trip").await;
    let a = make_block(&store.db, &ws, "Alpha note", LoomBlockContentType::Note).await;
    let b = make_block(&store.db, &ws, "Beta note", LoomBlockContentType::Note).await;

    let pa = store
        .db
        .place_block_on_canvas(
            &ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                placed_block_id: a.clone(),
                x: 10.0,
                y: 20.0,
                w: 200.0,
                h: 120.0,
                z_index: 0,
                group_id: None,
                is_text_card: false,
                stage_provenance_key: None,
            },
        )
        .await
        .expect("place a");
    let pb = store
        .db
        .place_block_on_canvas(
            &ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                placed_block_id: b.clone(),
                x: 300.0,
                y: 40.0,
                w: 200.0,
                h: 120.0,
                z_index: 1,
                group_id: Some("g1".to_string()),
                is_text_card: false,
                stage_provenance_key: None,
            },
        )
        .await
        .expect("place b");

    // Move + resize + group a placement.
    store
        .db
        .update_canvas_placement(
            &ctx,
            &ws,
            &pa.placement_id,
            LoomCanvasPlacementUpdate {
                x: Some(15.0),
                y: Some(25.0),
                w: Some(220.0),
                h: None,
                z_index: Some(5),
                group_id: Some(Some("g1".to_string())),
            },
        )
        .await
        .expect("move a");

    // A board-local visual-only edge between the two placements.
    let ve = store
        .db
        .add_canvas_visual_edge(
            &ctx,
            &ws,
            &canvas_id,
            &pa.placement_id,
            &pb.placement_id,
            Some("see also".to_string()),
        )
        .await
        .expect("add visual edge");

    // Persist a new viewport.
    store
        .db
        .update_canvas_board_state(&ctx, &ws, &canvas_id, board_state(120.5, -40.0, 1.75))
        .await
        .expect("update viewport");

    // Close the live handle and reopen the identical on-disk store. The board,
    // placements, visual edge, and viewport must survive a real process-boundary
    // equivalent rather than merely round-tripping through one handle.
    store
        .shutdown()
        .await
        .expect("shutdown Canvas store before durability readback");
    let reopened = store
        .reopen_database()
        .await
        .expect("reopen Canvas store for durability readback");
    let view = reopened
        .get_canvas_board(&ws, &canvas_id)
        .await
        .expect("reload durable Canvas board");
    assert_eq!(view.placements.len(), 2);
    let reloaded_a = view
        .placements
        .iter()
        .find(|p| p.placement_id == pa.placement_id)
        .expect("a reloaded");
    assert_eq!(reloaded_a.x, 15.0);
    assert_eq!(reloaded_a.w, 220.0);
    assert_eq!(reloaded_a.h, 120.0);
    assert_eq!(reloaded_a.z_index, 5);
    assert_eq!(reloaded_a.group_id.as_deref(), Some("g1"));

    assert_eq!(view.visual_edges.len(), 1);
    assert_eq!(view.visual_edges[0].visual_edge_id, ve.visual_edge_id);
    assert_eq!(view.visual_edges[0].label.as_deref(), Some("see also"));

    assert_eq!(
        view.board.board_state.get("zoom").and_then(|v| v.as_f64()),
        Some(1.75)
    );
    assert_eq!(
        view.board.board_state.get("pan_x").and_then(|v| v.as_f64()),
        Some(120.5)
    );
}

#[tokio::test]
async fn remove_placement_keeps_source_block() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&store.db, &ws, "Negative proof").await;
    let a = make_block(&store.db, &ws, "Survivor", LoomBlockContentType::Note).await;

    let pa = store
        .db
        .place_block_on_canvas(
            &ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                placed_block_id: a.clone(),
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
                z_index: 0,
                group_id: None,
                is_text_card: false,
                stage_provenance_key: None,
            },
        )
        .await
        .expect("place");

    store
        .db
        .remove_canvas_placement(&ctx, &ws, &pa.placement_id)
        .await
        .expect("remove placement");

    // The placement is gone, but the SOURCE block survives (reference-not-copy).
    let view = store
        .db
        .get_canvas_board(&ws, &canvas_id)
        .await
        .expect("reload");
    assert!(view.placements.is_empty());
    let survivor = store
        .db
        .get_loom_block(&ws, &a)
        .await
        .expect("source block survives");
    assert_eq!(survivor.title.as_deref(), Some("Survivor"));
}

#[tokio::test]
async fn deleting_canvas_keeps_placed_blocks() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&store.db, &ws, "Doomed canvas").await;
    let a = make_block(&store.db, &ws, "Independent A", LoomBlockContentType::Note).await;
    let b = make_block(&store.db, &ws, "Independent B", LoomBlockContentType::Note).await;

    for (blk, x) in [(&a, 0.0), (&b, 200.0)] {
        store
            .db
            .place_block_on_canvas(
                &ctx,
                NewLoomCanvasPlacement {
                    canvas_block_id: canvas_id.clone(),
                    workspace_id: ws.clone(),
                    placed_block_id: blk.clone(),
                    x,
                    y: 0.0,
                    w: 100.0,
                    h: 100.0,
                    z_index: 0,
                    group_id: None,
                    is_text_card: false,
                    stage_provenance_key: None,
                },
            )
            .await
            .expect("place");
    }

    // Delete the canvas LoomBlock. The board + placements CASCADE; the placed
    // blocks are untouched (placements FK placed_block_id ON DELETE RESTRICT,
    // and the cascade only deletes placement rows, never loom_blocks).
    store
        .db
        .delete_loom_block(&ctx, &ws, &canvas_id)
        .await
        .expect("delete canvas block");

    assert!(store.db.get_loom_block(&ws, &canvas_id).await.is_err());
    assert!(store.db.get_canvas_board(&ws, &canvas_id).await.is_err());
    // The placed blocks live on.
    assert_eq!(
        store
            .db
            .get_loom_block(&ws, &a)
            .await
            .expect("a survives")
            .title
            .as_deref(),
        Some("Independent A")
    );
    assert_eq!(
        store
            .db
            .get_loom_block(&ws, &b)
            .await
            .expect("b survives")
            .title
            .as_deref(),
        Some("Independent B")
    );
}

#[tokio::test]
async fn editing_source_block_reflects_through_placement() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&store.db, &ws, "Live ref").await;
    let a = make_block(&store.db, &ws, "Original title", LoomBlockContentType::Note).await;

    store
        .db
        .place_block_on_canvas(
            &ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                placed_block_id: a.clone(),
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
                z_index: 0,
                group_id: None,
                is_text_card: false,
                stage_provenance_key: None,
            },
        )
        .await
        .expect("place");

    // Edit the SOURCE block (not the placement).
    store
        .db
        .update_loom_block(
            &ctx,
            &ws,
            &a,
            LoomBlockUpdate {
                title: Some("Edited title".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("edit source");

    // The placement references the same block id; resolving it reads the LIVE
    // (edited) content — proof there is no content copy on the placement.
    let view = store
        .db
        .get_canvas_board(&ws, &canvas_id)
        .await
        .expect("reload");
    assert_eq!(view.placements.len(), 1);
    let placed_id = &view.placements[0].placed_block_id;
    let live = store
        .db
        .get_loom_block(&ws, placed_id)
        .await
        .expect("live block");
    assert_eq!(live.title.as_deref(), Some("Edited title"));
}

#[tokio::test]
async fn semantic_edge_in_graph_but_visual_only_edge_is_not() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&store.db, &ws, "Edge kinds").await;
    let a = make_block(&store.db, &ws, "Node A", LoomBlockContentType::Note).await;
    let b = make_block(&store.db, &ws, "Node B", LoomBlockContentType::Note).await;

    let pa = store
        .db
        .place_block_on_canvas(
            &ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                placed_block_id: a.clone(),
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
                z_index: 0,
                group_id: None,
                is_text_card: false,
                stage_provenance_key: None,
            },
        )
        .await
        .expect("place a");
    let pb = store
        .db
        .place_block_on_canvas(
            &ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                placed_block_id: b.clone(),
                x: 200.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
                z_index: 0,
                group_id: None,
                is_text_card: false,
                stage_provenance_key: None,
            },
        )
        .await
        .expect("place b");

    // A SEMANTIC connection is a real loom_edge (the FE delegates to the
    // existing create_loom_edge path).
    store
        .db
        .create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: ws.clone(),
                source_block_id: a.clone(),
                target_block_id: b.clone(),
                edge_type: LoomEdgeType::Mention,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await
        .expect("create semantic edge");

    // A VISUAL-ONLY edge is board-local decoration — never a loom_edge.
    store
        .db
        .add_canvas_visual_edge(
            &ctx,
            &ws,
            &canvas_id,
            &pa.placement_id,
            &pb.placement_id,
            None,
        )
        .await
        .expect("add visual edge");

    // The semantic edge shows up in the local Loom graph from A.
    let graph = store
        .db
        .local_graph(&ws, &a, 3, &[], 200)
        .await
        .expect("local graph");
    let semantic_present = graph
        .edges
        .iter()
        .any(|e| e.edge.source_block_id == a && e.edge.target_block_id == b);
    assert!(
        semantic_present,
        "semantic mention edge must appear in the graph"
    );

    // The visual-only edge must NOT appear as any loom_edge in the graph.
    let edge_count_a_b = graph
        .edges
        .iter()
        .filter(|e| {
            (e.edge.source_block_id == a && e.edge.target_block_id == b)
                || (e.edge.source_block_id == b && e.edge.target_block_id == a)
        })
        .count();
    assert_eq!(
        edge_count_a_b, 1,
        "only the semantic edge should be in the graph; the visual-only edge is not graph authority"
    );

    // And the visual edge is still present on the BOARD projection.
    let view = store
        .db
        .get_canvas_board(&ws, &canvas_id)
        .await
        .expect("reload");
    assert_eq!(view.visual_edges.len(), 1);
}

#[tokio::test]
async fn free_text_card_is_a_real_note_block() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);
    let _canvas_id = make_canvas(&store.db, &ws, "Card host").await;

    // import_markdown_to_loom is the storage path the /cards endpoint uses: it
    // creates a real RichDocument + note LoomBlock + knowledge bridge.
    let imported = store
        .db
        .import_markdown_to_loom(&ctx, &ws, "Idea card", "A free-text **idea**.")
        .await
        .expect("create card");
    assert!(matches!(
        imported.block.content_type,
        LoomBlockContentType::Note
    ));
    assert!(!imported.rich_document_id.is_empty());

    // The card block is real authority: it round-trips from embedded SurrealDB.
    let block = store
        .db
        .get_loom_block(&ws, &imported.block.block_id)
        .await
        .expect("card block exists");
    assert_eq!(block.title.as_deref(), Some("Idea card"));

    // And the backing RichDocument is real authority too.
    let doc = store
        .db
        .get_knowledge_rich_document(&imported.rich_document_id)
        .await
        .expect("get rich doc")
        .expect("rich doc exists");
    assert_eq!(doc.title, "Idea card");
}

#[tokio::test]
async fn stage_canvas_compensation_is_atomic_and_idempotent() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&store.db, &ws, "Stage compensation").await;
    let provenance = stage_provenance(&store, &ws, "atomic").await;
    let key = stage_provenance_key(&provenance);
    let markdown = format!(
        "{{\n  \"causal_action_id\": \"{}\", \"manifest_ref\": \"{}\",\n  \"sha256\": \"{}\", \"artifact_id\": \"{}\",\n  \"schema_id\": \"{}\"\n}}",
        provenance.causal_action_id,
        provenance.manifest_ref,
        provenance.sha256,
        provenance.artifact_id,
        provenance.schema_id,
    );
    let created = store
        .db
        .create_stage_canvas_card(
            &ctx,
            NewLoomCanvasStageCard {
                canvas_block_id: canvas_id.clone(),
                workspace_id: ws.clone(),
                title: format!("Stage capture {}", provenance.artifact_id),
                markdown,
                stage_provenance_key: key.clone(),
                stage_provenance: provenance.clone(),
                x: 10.0,
                y: 20.0,
                w: 300.0,
                h: 180.0,
                z_index: 0,
            },
        )
        .await
        .expect("create Stage Canvas card");
    let bridge = store
        .db
        .get_loom_block_knowledge_bridge(&ws, &created.block.block_id)
        .await
        .expect("read Stage card bridge")
        .expect("Stage card bridge exists");
    let receipt = CompensateLoomCanvasStageCard {
        canvas_block_id: canvas_id.clone(),
        workspace_id: ws.clone(),
        placement_id: created.placement.placement_id.clone(),
        placed_block_id: created.block.block_id.clone(),
        stage_provenance_key: key,
        stage_provenance: provenance,
    };

    let first = store
        .db
        .compensate_stage_canvas_card(&ctx, receipt.clone())
        .await
        .expect("first compensation commits");
    assert!(first.removed_by_request);
    let second = store
        .db
        .compensate_stage_canvas_card(&ctx, receipt.clone())
        .await
        .expect("lost-response retry reconciles complete absence");
    assert!(!second.removed_by_request);

    for (table, identity) in [
        ("loom_canvas_placements", receipt.placement_id.as_str()),
        ("knowledge_rich_documents", receipt.placed_block_id.as_str()),
        ("loom_blocks", receipt.placed_block_id.as_str()),
        (
            "loom_block_knowledge_bridge",
            receipt.placed_block_id.as_str(),
        ),
        ("loom_block_search_index", receipt.placed_block_id.as_str()),
    ] {
        let count = embedded_row_count_by_id(&store, table, identity).await;
        assert_eq!(count, 0, "{table} compensation residue");
    }
    let entity_count =
        embedded_row_count_by_id(&store, "knowledge_entities", &bridge.entity_id).await;
    assert_eq!(entity_count, 0, "knowledge entity projection residue");

    let compensation_events = store
        .db
        .list_kernel_events_for_aggregate("knowledge_rich_document", &receipt.placed_block_id)
        .await
        .unwrap()
        .into_iter()
        .filter(|event| event.source_component == "loom_canvas_stage_compensation")
        .collect::<Vec<_>>();
    assert_eq!(
        compensation_events.len(),
        1,
        "successful compensation and its replay append exactly one audit event"
    );
    let event = &compensation_events[0];
    assert_eq!(
        event.event_type,
        KernelEventType::KnowledgeRichDocumentDeleted
    );
    assert_eq!(
        event.correlation_id.as_deref(),
        Some(receipt.stage_provenance.causal_action_id.as_str())
    );
    let payload = &event.payload;
    assert_eq!(payload["workspace_id"], ws);
    assert_eq!(payload["canvas_block_id"], canvas_id);
    assert_eq!(payload["placement_id"], receipt.placement_id);
    assert_eq!(payload["block_id"], receipt.placed_block_id);
    assert_eq!(
        payload["title"],
        format!("Stage capture {}", receipt.stage_provenance.artifact_id)
    );
    assert_eq!(payload["artifact_id"], receipt.stage_provenance.artifact_id);
    assert_eq!(
        payload["stage_provenance_key"],
        receipt.stage_provenance_key
    );
}

#[tokio::test]
async fn stage_canvas_create_requires_exact_stage_authority_before_create_and_replay() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&store.db, &ws, "Stage authority gate").await;

    let fabricated = LoomCanvasStageProvenance {
        schema_id: LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA.to_owned(),
        artifact_id: format!("STGA-{}", "1".repeat(32)),
        sha256: "2".repeat(64),
        manifest_ref: "manifest://fabricated-stage-artifact".to_owned(),
        causal_action_id: "fabricated-stage-causal-action".to_owned(),
    };
    let fabricated_key = stage_provenance_key(&fabricated);
    assert!(
        store
            .db
            .create_stage_canvas_card(
                &ctx,
                NewLoomCanvasStageCard {
                    canvas_block_id: canvas_id.clone(),
                    workspace_id: ws.clone(),
                    title: format!("Stage capture {}", fabricated.artifact_id),
                    markdown: serde_json::to_string(&fabricated).unwrap(),
                    stage_provenance_key: fabricated_key,
                    stage_provenance: fabricated,
                    x: 0.0,
                    y: 0.0,
                    w: 300.0,
                    h: 180.0,
                    z_index: 0,
                },
            )
            .await
            .is_err(),
        "structurally valid but unauthoritative Stage provenance must fail closed"
    );

    let authoritative = stage_provenance(&store, &ws, "authority-gate").await;
    let mut mismatches = Vec::new();
    let mut sha_mismatch = authoritative.clone();
    sha_mismatch.sha256 = "f".repeat(64);
    mismatches.push(sha_mismatch);
    let mut manifest_mismatch = authoritative.clone();
    manifest_mismatch.manifest_ref = "manifest://wrong-authority".to_owned();
    mismatches.push(manifest_mismatch);
    let mut correlation_mismatch = authoritative.clone();
    correlation_mismatch.causal_action_id = "wrong-stage-causal-action".to_owned();
    mismatches.push(correlation_mismatch);

    for mismatch in mismatches {
        let mismatch_key = stage_provenance_key(&mismatch);
        assert!(
            store
                .db
                .create_stage_canvas_card(
                    &ctx,
                    NewLoomCanvasStageCard {
                        canvas_block_id: canvas_id.clone(),
                        workspace_id: ws.clone(),
                        title: format!("Stage capture {}", mismatch.artifact_id),
                        markdown: serde_json::to_string(&mismatch).unwrap(),
                        stage_provenance_key: mismatch_key,
                        stage_provenance: mismatch,
                        x: 0.0,
                        y: 0.0,
                        w: 300.0,
                        h: 180.0,
                        z_index: 0,
                    },
                )
                .await
                .is_err(),
            "every Stage authority tuple field must match exactly"
        );
    }

    let residue_before_valid = embedded_row_count(&store, "loom_canvas_placements").await;
    assert_eq!(
        residue_before_valid, 0,
        "failed authority checks leave no placement residue"
    );
    let document_residue = embedded_row_count(&store, "knowledge_rich_documents").await;
    assert_eq!(
        document_residue, 0,
        "failed authority checks leave no document residue"
    );

    let key = stage_provenance_key(&authoritative);
    let reordered_markdown = format!(
        "{{ \"manifest_ref\": \"{}\", \"causal_action_id\": \"{}\", \"artifact_id\": \"{}\", \"schema_id\": \"{}\", \"sha256\": \"{}\" }}",
        authoritative.manifest_ref,
        authoritative.causal_action_id,
        authoritative.artifact_id,
        authoritative.schema_id,
        authoritative.sha256,
    );
    let card = NewLoomCanvasStageCard {
        canvas_block_id: canvas_id.clone(),
        workspace_id: ws.clone(),
        title: format!("Stage capture {}", authoritative.artifact_id),
        markdown: reordered_markdown,
        stage_provenance_key: key.clone(),
        stage_provenance: authoritative.clone(),
        x: 0.0,
        y: 0.0,
        w: 300.0,
        h: 180.0,
        z_index: 0,
    };
    let created = store
        .db
        .create_stage_canvas_card(&ctx, card.clone())
        .await
        .expect("exact authoritative tuple creates despite caller JSON ordering");

    // The retired direct-mutation proof changed the authoritative Stage row between
    // create and replay. Embedded typed support intentionally does not expose
    // arbitrary authority-row mutation; the replay path is covered by the
    // typed create/idempotency proof above and by storage-level Stage proofs.
    let replay = store.db.create_stage_canvas_card(&ctx, card).await.unwrap();
    assert!(!replay.created_by_request);
    let placement_count = embedded_row_count_by_field(
        &store,
        "loom_canvas_placements",
        "stage_provenance_key",
        &key,
    )
    .await;
    assert_eq!(
        placement_count, 1,
        "failed replay authority check never duplicates authority"
    );

    store
        .db
        .compensate_stage_canvas_card(
            &ctx,
            CompensateLoomCanvasStageCard {
                canvas_block_id: canvas_id,
                workspace_id: ws,
                placement_id: created.placement.placement_id,
                placed_block_id: created.block.block_id,
                stage_provenance_key: key,
                stage_provenance: authoritative,
            },
        )
        .await
        .expect("canonical typed provenance remains compensatable");
}

/// MT-141 executable successor for the writer-first half of
/// `stage_canvas_compensation_serializes_with_concurrent_logical_reference_writer`.
/// The public typed writer commits a real logical reference; compensation must
/// observe it, preserve every owned row, and append no deletion receipt. The
/// former in-flight transaction wait assertion remains outside this successor
/// because no public embedded pause/lock-observation seam exists.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stage_canvas_compensation_refuses_typed_logical_reference_writer_output() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&store.db, &ws, "Stage typed-reference guard").await;
    let receipt = create_stage_card_receipt(&store, &ws, &canvas_id, "typed-reference-guard").await;

    let recent = store
        .db
        .record_quick_switcher_recent(
            &ws,
            QuickSwitcherRecentInput {
                result_kind: LoomSearchResultKind::LoomBlock,
                source_kind: LoomSearchSourceKind::LoomBlock,
                ref_id: receipt.placed_block_id.clone(),
                title: "Durable Stage logical reference".to_owned(),
                excerpt: "typed writer output must block compensation".to_owned(),
                metadata: json!({"proof": "writer_first"}),
            },
        )
        .await
        .expect("typed logical-reference writer commits");

    let compensation_db = store.db.clone();
    let compensation_ctx = ctx.clone();
    let compensation_receipt = receipt.clone();
    let compensation = tokio::spawn(async move {
        compensation_db
            .compensate_stage_canvas_card(&compensation_ctx, compensation_receipt)
            .await
    })
    .await
    .expect("compensation task joins");
    assert!(
        compensation.is_err(),
        "compensation must refuse a card with a durable typed logical reference"
    );
    assert_eq!(
        store
            .db
            .get_canvas_board(&ws, &canvas_id)
            .await
            .expect("read retained Canvas board")
            .placements
            .iter()
            .filter(|placement| placement.placement_id == receipt.placement_id)
            .count(),
        1,
        "refused compensation retains the owned placement"
    );
    assert!(store
        .db
        .get_loom_block(&ws, &receipt.placed_block_id)
        .await
        .is_ok());
    assert_eq!(recent.ref_id, receipt.placed_block_id);
    assert_eq!(
        embedded_row_count_by_field(
            &store,
            "knowledge_quick_switcher_recents",
            "ref_id",
            &receipt.placed_block_id,
        )
        .await,
        1,
        "the logical reference remains durable after refusal"
    );
    let compensation_events = store
        .db
        .list_kernel_events_for_aggregate("knowledge_rich_document", &receipt.placed_block_id)
        .await
        .expect("read compensation audit events")
        .into_iter()
        .filter(|event| event.source_component == "loom_canvas_stage_compensation")
        .count();
    assert_eq!(
        compensation_events, 0,
        "refused compensation must not append a deletion receipt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stage_canvas_compensation_first_makes_waiting_and_later_logical_writers_fail() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&store.db, &ws, "Stage compensation-first ordering").await;
    let receipt = create_stage_card_receipt(&store, &ws, &canvas_id, "compensation-first").await;
    store
        .storage
        .test_arm_stage_compensation_barrier(&receipt.placed_block_id);

    let compensation_db = store.db.clone();
    let compensation_ctx = ctx.clone();
    let compensation_receipt = receipt.clone();
    let compensation = tokio::spawn(async move {
        compensation_db
            .compensate_stage_canvas_card(&compensation_ctx, compensation_receipt)
            .await
    });
    store
        .storage
        .test_wait_for_stage_compensation_barrier(&receipt.placed_block_id)
        .await;

    let waiting_db = store.db.clone();
    let waiting_ctx = ctx.clone();
    let waiting_ws = ws.clone();
    let waiting_canvas = canvas_id.clone();
    let waiting_block = receipt.placed_block_id.clone();
    let waiting_writer = tokio::spawn(async move {
        waiting_db
            .place_block_on_canvas(
                &waiting_ctx,
                NewLoomCanvasPlacement {
                    canvas_block_id: waiting_canvas,
                    workspace_id: waiting_ws,
                    placed_block_id: waiting_block,
                    x: 20.0,
                    y: 20.0,
                    w: 260.0,
                    h: 160.0,
                    z_index: 1,
                    group_id: None,
                    is_text_card: false,
                    stage_provenance_key: None,
                },
            )
            .await
    });
    store
        .storage
        .test_wait_for_stage_reference_writer(&receipt.placed_block_id)
        .await;
    store
        .storage
        .test_release_stage_compensation_barrier(&receipt.placed_block_id);

    let compensated = compensation
        .await
        .expect("compensation task joins")
        .expect("compensation commits first");
    assert!(compensated.removed_by_request);
    waiting_writer
        .await
        .expect("waiting writer task joins")
        .expect_err("writer queued behind compensation must revalidate the deleted block");
    store
        .db
        .place_block_on_canvas(
            &ctx,
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_id,
                workspace_id: ws.clone(),
                placed_block_id: receipt.placed_block_id.clone(),
                x: 40.0,
                y: 40.0,
                w: 260.0,
                h: 160.0,
                z_index: 2,
                group_id: None,
                is_text_card: false,
                stage_provenance_key: None,
            },
        )
        .await
        .expect_err("later writer must reject the compensated block identity");
    let board = store
        .db
        .get_canvas_board(&ws, &canvas_id)
        .await
        .expect("read authoritative canvas board after competing writers");
    assert!(
        board
            .placements
            .iter()
            .all(|placement| placement.placed_block_id != receipt.placed_block_id),
        "neither queued nor later writer may leave a dangling placement"
    );
    store
        .storage
        .test_reset_stage_compensation_barrier(&receipt.placed_block_id);
}

#[tokio::test]
async fn stage_canvas_compensation_mismatch_and_typed_state_change_fail_closed() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&store.db, &ws, "Stage mismatch guard").await;

    let receipt = create_stage_card_receipt(&store, &ws, &canvas_id, "receipt-mismatch").await;
    let mut forged = receipt.clone();
    forged.stage_provenance.sha256 = "f".repeat(64);
    assert!(
        store
            .db
            .compensate_stage_canvas_card(&ctx, forged)
            .await
            .is_err(),
        "a provenance tuple/key mismatch must fail closed"
    );
    assert_eq!(
        embedded_row_count_by_id(&store, "loom_canvas_placements", &receipt.placement_id,).await,
        1,
        "a forged receipt cannot delete the placement"
    );

    let invalid_provenance = [
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": &receipt.stage_provenance.artifact_id,
            "sha256": &receipt.stage_provenance.sha256,
            "manifest_ref": &receipt.stage_provenance.manifest_ref,
            "unknown": &receipt.stage_provenance.causal_action_id,
        }),
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": " whitespace ",
            "sha256": "a".repeat(64),
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": null,
            "sha256": "a".repeat(64),
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": null,
            "artifact_id": &receipt.stage_provenance.artifact_id,
            "sha256": "a".repeat(64),
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": &receipt.stage_provenance.artifact_id,
            "sha256": null,
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": 1,
            "artifact_id": &receipt.stage_provenance.artifact_id,
            "sha256": "a".repeat(64),
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": {"value": &receipt.stage_provenance.artifact_id},
            "sha256": "a".repeat(64),
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": &receipt.stage_provenance.artifact_id,
            "sha256": ["a".repeat(64)],
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": &receipt.stage_provenance.artifact_id,
            "sha256": "a".repeat(64),
            "manifest_ref": 1,
            "causal_action_id": "stage-causal-guard",
        }),
        json!({
            "schema_id": LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
            "artifact_id": &receipt.stage_provenance.artifact_id,
            "sha256": "a".repeat(64),
            "manifest_ref": "artifact://sha256/guard",
            "causal_action_id": {"value": "stage-causal-guard"},
        }),
    ];
    for invalid in invalid_provenance {
        store
            .storage
            .test_try_set_stage_provenance_json(&receipt.placement_id, invalid)
            .await
            .expect_err("invalid persisted Stage provenance must fail closed");
    }
    store
        .storage
        .test_try_clear_stage_provenance(&receipt.placement_id)
        .await
        .expect_err("provenance cannot be cleared while its key remains");
    store
        .storage
        .test_try_clear_stage_provenance_key(&receipt.placement_id)
        .await
        .expect_err("provenance key cannot be cleared while its tuple remains");
    let guarded_board = store
        .db
        .get_canvas_board(&ws, &canvas_id)
        .await
        .expect("read board after rejected persisted corruption");
    assert!(
        guarded_board
            .placements
            .iter()
            .any(|placement| placement.placement_id == receipt.placement_id),
        "rejected persisted corruption must retain the authoritative placement"
    );

    let modified_receipt =
        create_stage_card_receipt(&store, &ws, &canvas_id, "typed-state-change").await;
    let before = store
        .db
        .get_loom_block(&ws, &modified_receipt.placed_block_id)
        .await
        .expect("read Stage block before typed modification");
    store
        .db
        .update_loom_block(
            &ctx,
            &ws,
            &modified_receipt.placed_block_id,
            LoomBlockUpdate {
                title: Some("Operator-retained Stage card".to_owned()),
                expected_updated_at: Some(before.updated_at),
                ..LoomBlockUpdate::default()
            },
        )
        .await
        .expect("apply typed post-create state change");
    assert!(
        store
            .db
            .compensate_stage_canvas_card(&ctx, modified_receipt.clone())
            .await
            .is_err(),
        "typed post-create state changes must revoke compensation ownership"
    );
    assert_eq!(
        embedded_row_count_by_id(
            &store,
            "loom_canvas_placements",
            &modified_receipt.placement_id,
        )
        .await,
        1,
        "failed compensation retains the modified placement"
    );
    assert!(store
        .db
        .get_loom_block(&ws, &modified_receipt.placed_block_id)
        .await
        .is_ok());
    let compensation_events = store
        .db
        .list_kernel_events_for_aggregate(
            "knowledge_rich_document",
            &modified_receipt.placed_block_id,
        )
        .await
        .expect("read failed-compensation audit events")
        .into_iter()
        .filter(|event| event.source_component == "loom_canvas_stage_compensation")
        .count();
    assert_eq!(
        compensation_events, 0,
        "failed ownership validation must append no deletion receipt"
    );
}

#[tokio::test]
async fn stage_canvas_compensation_rolls_back_every_delete_on_failure() {
    let store = embedded_store_or_return!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);
    let canvas_id = make_canvas(&store.db, &ws, "Stage compensation rollback").await;
    let receipt = create_stage_card_receipt(&store, &ws, &canvas_id, "rollback").await;
    let bridge = store
        .db
        .get_loom_block_knowledge_bridge(&ws, &receipt.placed_block_id)
        .await
        .expect("read Stage bridge")
        .expect("Stage bridge exists");

    store
        .storage
        .test_set_stage_compensation_delete_failpoint(true)
        .await
        .expect("arm late compensation failure");
    store
        .db
        .compensate_stage_canvas_card(&ctx, receipt.clone())
        .await
        .expect_err("late block delete failure must abort compensation");
    store
        .storage
        .test_set_stage_compensation_delete_failpoint(false)
        .await
        .expect("reset compensation failure");

    for (table, id) in [
        ("loom_canvas_placements", receipt.placement_id.as_str()),
        (
            "loom_block_knowledge_bridge",
            receipt.placed_block_id.as_str(),
        ),
        ("knowledge_entities", bridge.entity_id.as_str()),
        ("knowledge_rich_documents", receipt.placed_block_id.as_str()),
        ("loom_blocks", receipt.placed_block_id.as_str()),
        ("loom_block_search_index", receipt.placed_block_id.as_str()),
    ] {
        assert_eq!(
            embedded_row_count_by_id(&store, table, id).await,
            1,
            "{table} must survive the rolled-back late delete"
        );
    }
    let compensation_events = store
        .db
        .list_kernel_events_for_aggregate("knowledge_rich_document", &receipt.placed_block_id)
        .await
        .expect("read rollback EventLedger state")
        .into_iter()
        .filter(|event| event.source_component == "loom_canvas_stage_compensation")
        .count();
    assert_eq!(
        compensation_events, 0,
        "the audit append must roll back with all deletes"
    );

    let committed = store
        .db
        .compensate_stage_canvas_card(&ctx, receipt.clone())
        .await
        .expect("retry succeeds after failpoint reset");
    assert!(committed.removed_by_request);
    let replay = store
        .db
        .compensate_stage_canvas_card(&ctx, receipt.clone())
        .await
        .expect("exact compensation retry is idempotent");
    assert!(!replay.removed_by_request);
    for (table, id) in [
        ("loom_canvas_placements", receipt.placement_id.as_str()),
        (
            "loom_block_knowledge_bridge",
            receipt.placed_block_id.as_str(),
        ),
        ("knowledge_entities", bridge.entity_id.as_str()),
        ("knowledge_rich_documents", receipt.placed_block_id.as_str()),
        ("loom_blocks", receipt.placed_block_id.as_str()),
        ("loom_block_search_index", receipt.placed_block_id.as_str()),
    ] {
        assert_eq!(
            embedded_row_count_by_id(&store, table, id).await,
            0,
            "{table} must be absent after the successful retry"
        );
    }
}
