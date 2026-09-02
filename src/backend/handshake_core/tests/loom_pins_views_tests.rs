//! WP-KERNEL-009 MT-183 PinsFavoritesAndUnlinked — real embedded proof.
//!
//! §10.12 §7.1 / §7.1.4.3 / [LM-VIEW-002][LM-VIEW-004]: the four Loom views
//! (All/Unlinked/Sorted/Pins) backend, focused on the MT-183 additions — the
//! REORDERABLE Pins grid (user-controlled pin_order) and the Unlinked triage
//! queue (blocks with zero mention/tag edges). Authority = loom_blocks +
//! loom_edges. No parallel store.

#[path = "knowledge_ingestion_support.rs"]
mod embedded_knowledge_support;

use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomBlockUpdate, LoomEdgeCreatedBy,
    LoomEdgeType, LoomSearchFilters, LoomViewFilters, LoomViewResponse, LoomViewType, NewLoomBlock,
    NewLoomEdge, WriteContext,
};
use knowledge_ingestion_support::open_embedded_store;

macro_rules! embedded_or_skip {
    () => {{
        match open_embedded_store().await {
            Some(store) => store,
            None => {
                eprintln!("SKIP MT-183 loom pins/views proof: embedded store unavailable");
                return;
            }
        }
    }};
}

async fn blk(
    db: &handshake_core::storage::surreal::SurrealDatabase,
    ws: &str,
    title: &str,
) -> String {
    let ctx = WriteContext::human(None);
    db.create_loom_block(
        &ctx,
        NewLoomBlock {
            block_id: None,
            workspace_id: ws.to_string(),
            content_type: LoomBlockContentType::Note,
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
    .expect("block")
    .block_id
}

async fn pin(db: &handshake_core::storage::surreal::SurrealDatabase, ws: &str, id: &str) {
    let ctx = WriteContext::human(None);
    db.update_loom_block(
        &ctx,
        ws,
        id,
        LoomBlockUpdate {
            pinned: Some(true),
            ..Default::default()
        },
    )
    .await
    .expect("pin");
}

fn pin_ids(resp: &LoomViewResponse) -> Vec<String> {
    match resp {
        LoomViewResponse::Pins { blocks } => blocks.iter().map(|b| b.block_id.clone()).collect(),
        _ => panic!("expected Pins response"),
    }
}

fn favorite_ids(resp: &LoomViewResponse) -> Vec<String> {
    match resp {
        LoomViewResponse::Favorites { blocks } => {
            blocks.iter().map(|b| b.block_id.clone()).collect()
        }
        _ => panic!("expected Favorites response"),
    }
}

#[tokio::test]
async fn favorites_view_is_independent_from_pins() {
    let store = embedded_or_skip!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);

    let favorite_only = blk(&store.db, &ws, "Favorite only").await;
    let pinned_only = blk(&store.db, &ws, "Pinned only").await;

    store
        .db
        .update_loom_block(
            &ctx,
            &ws,
            &favorite_only,
            LoomBlockUpdate {
                favorite: Some(true),
                ..Default::default()
            },
        )
        .await
        .expect("favorite block");
    pin(&store.db, &ws, &pinned_only).await;

    let favorites = store
        .db
        .query_loom_view(
            &ws,
            LoomViewType::Favorites,
            LoomViewFilters::default(),
            100,
            0,
        )
        .await
        .expect("favorites view");
    assert_eq!(favorite_ids(&favorites), vec![favorite_only.clone()]);
    match favorites {
        LoomViewResponse::Favorites { blocks } => {
            assert!(blocks[0].favorite, "favorite flag is returned on the block");
            assert!(
                !blocks[0].pinned,
                "favorite-only block is not implicitly pinned"
            );
        }
        _ => unreachable!("favorite_ids already checked the response type"),
    }

    let pins = store
        .db
        .query_loom_view(&ws, LoomViewType::Pins, LoomViewFilters::default(), 100, 0)
        .await
        .expect("pins view");
    assert_eq!(pin_ids(&pins), vec![pinned_only]);
}

#[tokio::test]
async fn favorite_flag_survives_search_results() {
    let store = embedded_or_skip!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);

    let favorite = blk(&store.db, &ws, "Favorite searchable propagation").await;
    store
        .db
        .update_loom_block(
            &ctx,
            &ws,
            &favorite,
            LoomBlockUpdate {
                favorite: Some(true),
                ..Default::default()
            },
        )
        .await
        .expect("favorite block");

    let results = store
        .db
        .search_loom_blocks(&ws, "propagation", LoomSearchFilters::default(), 10, 0)
        .await
        .expect("search");
    let found = results
        .iter()
        .find(|result| result.block.block_id == favorite)
        .expect("favorite block in search results");
    assert!(
        found.block.favorite,
        "search results preserve the favorite flag"
    );
}

#[tokio::test]
async fn pins_view_respects_user_pin_order() {
    let store = embedded_or_skip!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);

    let a = blk(&store.db, &ws, "Apple").await;
    let b = blk(&store.db, &ws, "Banana").await;
    let c = blk(&store.db, &ws, "Cherry").await;
    for id in [&a, &b, &c] {
        pin(&store.db, &ws, id).await;
    }

    // Assign pin_order: C=0, A=1, B=2 -> expected order C, A, B.
    store
        .db
        .set_loom_block_pin_order(&ctx, &ws, &c, Some(0))
        .await
        .expect("order c");
    store
        .db
        .set_loom_block_pin_order(&ctx, &ws, &a, Some(1))
        .await
        .expect("order a");
    store
        .db
        .set_loom_block_pin_order(&ctx, &ws, &b, Some(2))
        .await
        .expect("order b");

    let pins = store
        .db
        .query_loom_view(&ws, LoomViewType::Pins, LoomViewFilters::default(), 100, 0)
        .await
        .expect("pins view");
    assert_eq!(pin_ids(&pins), vec![c.clone(), a.clone(), b.clone()]);

    // Reorder: move B to the front (pin_order -1).
    store
        .db
        .set_loom_block_pin_order(&ctx, &ws, &b, Some(-1))
        .await
        .expect("reorder b");
    let pins2 = store
        .db
        .query_loom_view(&ws, LoomViewType::Pins, LoomViewFilters::default(), 100, 0)
        .await
        .expect("pins view 2");
    assert_eq!(pin_ids(&pins2), vec![b.clone(), c.clone(), a.clone()]);

    // The block carries its pin_order back on a single read.
    let b_block = store.db.get_loom_block(&ws, &b).await.expect("get b");
    assert_eq!(b_block.pin_order, Some(-1));
}

#[tokio::test]
async fn clearing_pin_order_sends_block_to_the_end() {
    let store = embedded_or_skip!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);

    let a = blk(&store.db, &ws, "Ordered").await;
    let b = blk(&store.db, &ws, "Cleared").await;
    pin(&store.db, &ws, &a).await;
    pin(&store.db, &ws, &b).await;
    store
        .db
        .set_loom_block_pin_order(&ctx, &ws, &a, Some(0))
        .await
        .expect("order a");
    store
        .db
        .set_loom_block_pin_order(&ctx, &ws, &b, Some(1))
        .await
        .expect("order b");

    // Clear B's order (NULL) -> NULLS LAST puts it after the ordered A.
    let cleared = store
        .db
        .set_loom_block_pin_order(&ctx, &ws, &b, None)
        .await
        .expect("clear b");
    assert_eq!(cleared.pin_order, None, "pin_order cleared to NULL");

    let pins = store
        .db
        .query_loom_view(&ws, LoomViewType::Pins, LoomViewFilters::default(), 100, 0)
        .await
        .expect("pins");
    assert_eq!(
        pin_ids(&pins),
        vec![a.clone(), b.clone()],
        "cleared pin trails ordered pin"
    );
}

#[tokio::test]
async fn mt258_bookmark_add_remove_persists_to_embedded_authority() {
    let store = embedded_or_skip!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);

    let bookmark = blk(&store.db, &ws, "MT-258 bookmark authority").await;
    let bridge = store
        .db
        .bridge_loom_block_to_knowledge(&ctx, &ws, &bookmark)
        .await
        .expect("bookmark block has ProjectKnowledgeIndex/EventLedger bridge");
    assert_eq!(bridge.block_id, bookmark);
    assert_eq!(bridge.workspace_id, ws);

    let receipts = store
        .db
        .list_kernel_events_for_aggregate("knowledge_loom_block", &bridge.entity_id)
        .await
        .expect("bridge EventLedger receipt exists");
    let receipt = receipts
        .iter()
        .find(|event| event.event_id == bridge.index_event_id)
        .expect("bridge receipt is present in the typed EventLedger read");
    assert_eq!(
        receipt.event_type.to_string(),
        "KNOWLEDGE_LOOM_BLOCK_INDEXED"
    );
    assert_eq!(receipt.aggregate_type, "knowledge_loom_block");
    assert_eq!(receipt.aggregate_id, bridge.entity_id);
    assert_eq!(
        receipt
            .payload
            .get("block_id")
            .and_then(|value| value.as_str()),
        Some(bookmark.as_str())
    );

    pin(&store.db, &ws, &bookmark).await;
    store
        .db
        .set_loom_block_pin_order(&ctx, &ws, &bookmark, Some(0))
        .await
        .expect("set bookmark order");

    let pins = store
        .db
        .query_loom_view(&ws, LoomViewType::Pins, LoomViewFilters::default(), 100, 0)
        .await
        .expect("pins after add");
    assert_eq!(pin_ids(&pins), vec![bookmark.clone()]);
    let LoomViewResponse::Pins { blocks } = pins else {
        panic!("expected Pins response");
    };
    assert!(blocks[0].pinned);
    assert_eq!(blocks[0].pin_order, Some(0));

    store
        .db
        .set_loom_block_pin_order(&ctx, &ws, &bookmark, None)
        .await
        .expect("clear bookmark order before remove");
    store
        .db
        .update_loom_block(
            &ctx,
            &ws,
            &bookmark,
            LoomBlockUpdate {
                pinned: Some(false),
                ..Default::default()
            },
        )
        .await
        .expect("remove bookmark pin");

    let stored = store
        .db
        .get_loom_block(&ws, &bookmark)
        .await
        .expect("get bookmark");
    assert!(!stored.pinned, "bookmark remove persists pinned=false");
    assert_eq!(
        stored.pin_order, None,
        "bookmark remove persists pin_order=NULL"
    );

    let pins_after_remove = store
        .db
        .query_loom_view(&ws, LoomViewType::Pins, LoomViewFilters::default(), 100, 0)
        .await
        .expect("pins after remove");
    assert!(
        pin_ids(&pins_after_remove).is_empty(),
        "removed bookmark disappears from canonical pins view"
    );
}

#[tokio::test]
async fn unlinked_view_excludes_linked_blocks() {
    let store = embedded_or_skip!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);

    let lonely = blk(&store.db, &ws, "Lonely").await; // no edges -> unlinked
    let src = blk(&store.db, &ws, "Source").await;
    let tgt = blk(&store.db, &ws, "Target").await;
    // src --mention--> tgt: both become linked.
    store
        .db
        .create_loom_edge(
            &ctx,
            NewLoomEdge {
                edge_id: None,
                workspace_id: ws.clone(),
                source_block_id: src.clone(),
                target_block_id: tgt.clone(),
                edge_type: LoomEdgeType::Mention,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await
        .expect("edge");

    let unlinked = store
        .db
        .query_loom_view(
            &ws,
            LoomViewType::Unlinked,
            LoomViewFilters::default(),
            100,
            0,
        )
        .await
        .expect("unlinked");
    let ids: Vec<String> = match unlinked {
        LoomViewResponse::Unlinked { blocks } => {
            blocks.iter().map(|b| b.block_id.clone()).collect()
        }
        _ => panic!("expected Unlinked"),
    };
    assert!(ids.contains(&lonely), "lonely block is unlinked");
    assert!(
        !ids.contains(&src) && !ids.contains(&tgt),
        "linked blocks are excluded"
    );
}

#[tokio::test]
async fn set_pin_order_fails_closed_on_missing_block() {
    let store = embedded_or_skip!();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);
    let err = store
        .db
        .set_loom_block_pin_order(&ctx, &ws, "loom-missing", Some(1))
        .await
        .expect_err("missing block");
    assert!(
        format!("{err}").contains("loom_block") || format!("{err}").contains("not"),
        "{err}"
    );
}
