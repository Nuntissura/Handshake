//! WP-KERNEL-005 atelier Core-Data: real PostgreSQL round-trip proofs for the
//! six folded-in submodules (intake / collections / search / exports /
//! annotation / settings). Run with a live DATABASE_URL, e.g.
//!   DATABASE_URL=postgres://postgres@127.0.0.1:5544/handshake \
//!     cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --test atelier_core_data_tests -- --nocapture
//!
//! No mocks: each test connects the actual `AtelierStore` to a real Postgres,
//! ensures the schema, exercises one submodule with REAL data, and asserts the
//! load-bearing invariants from the adversarial review. Tables persist between
//! runs, so all public ids / hashes / keys are made unique per run via
//! `Uuid::new_v4()` to avoid cross-run collisions. Only `handshake_core` +
//! `tokio` + `uuid` (+ std) are used; sqlx is never imported directly.

use std::collections::HashMap;

use handshake_core::atelier::annotation::{AnnotationKind, NewMediaAnnotation};
use handshake_core::atelier::collections::NewCollection;
use handshake_core::atelier::exports::{ExportFormat, ManifestItemKind, NewExportRequest};
use handshake_core::atelier::intake::{
    BatchStatus, IntakeLane, NewIntakeBatch, NewIntakeItem,
};
use handshake_core::atelier::search::{MatchType, NewTagRule, TagType};
use handshake_core::atelier::settings::{
    PreferenceScope, PreferenceType, SetPreference,
};
use handshake_core::atelier::{
    AtelierStore, NewCharacter, NewMediaAsset, NewSheetVersion,
};
use uuid::Uuid;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Connect + ensure schema, the shared preamble every test runs against a real
/// Postgres.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// Materialize a fresh, run-unique media asset and return its `asset_id`.
async fn fresh_asset(store: &AtelierStore) -> Uuid {
    let asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: format!("sha256-{}", Uuid::new_v4()),
            mime: "image/png".to_string(),
            byte_len: 4096,
            source_provenance: Some("core-data-test".to_string()),
            artifact_ref: format!("artifact://atelier/media/{}", Uuid::new_v4()),
        })
        .await
        .expect("materialize media asset");
    asset.asset_id
}

#[tokio::test]
async fn atelier_intake_lanes_idempotency_and_close_guard() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_intake_lanes_idempotency_and_close_guard: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    // --- open batch is idempotent on idempotency_key ---
    let key = format!("intake-key-{}", Uuid::new_v4());
    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: key.clone(),
            source_label: "operator inbox scan".to_string(),
            character_internal_id: None,
        })
        .await
        .expect("open intake batch");
    assert_eq!(batch.status, BatchStatus::Open);
    let batch_again = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: key.clone(),
            source_label: "operator inbox scan (rescan)".to_string(),
            character_internal_id: None,
        })
        .await
        .expect("re-open same intake batch");
    assert_eq!(
        batch.batch_id, batch_again.batch_id,
        "re-opening the same idempotency_key must return the existing batch"
    );

    // --- add item is idempotent on (batch, source_path) ---
    let source_path = format!("/inbox/{}/a.png", Uuid::new_v4());
    let item = store
        .add_intake_item(
            batch.batch_id,
            &NewIntakeItem {
                source_path: source_path.clone(),
                file_name: "a.png".to_string(),
                byte_len: 1234,
                content_hash: Some(format!("sha256-{}", Uuid::new_v4())),
            },
        )
        .await
        .expect("add intake item");
    assert_eq!(item.lane, IntakeLane::New, "new items enter the New lane");
    let item_again = store
        .add_intake_item(
            batch.batch_id,
            &NewIntakeItem {
                source_path: source_path.clone(),
                file_name: "a.png".to_string(),
                byte_len: 1234,
                content_hash: None,
            },
        )
        .await
        .expect("re-add same source path");
    assert_eq!(
        item.item_id, item_again.item_id,
        "re-adding the same (batch, source_path) must not duplicate the item"
    );

    // Add a second item so we can exercise lane spread + the close guard.
    let source_path_2 = format!("/inbox/{}/b.png", Uuid::new_v4());
    let item2 = store
        .add_intake_item(
            batch.batch_id,
            &NewIntakeItem {
                source_path: source_path_2.clone(),
                file_name: "b.png".to_string(),
                byte_len: 5678,
                content_hash: None,
            },
        )
        .await
        .expect("add second intake item");

    // Two items, both still in New.
    let counts0 = store
        .intake_lane_counts(batch.batch_id)
        .await
        .expect("lane counts pre-triage");
    assert_eq!(counts0.new, 2, "both items start in the New lane");
    assert_eq!(counts0.accepted, 0);
    assert_eq!(counts0.rejected, 0);
    assert_eq!(counts0.deferred, 0);

    // --- close REFUSES while items are still in New ---
    let close_err = store.close_intake_batch(batch.batch_id).await;
    assert!(
        close_err.is_err(),
        "closing with New-lane items must error, not silently drop them"
    );

    // --- classify moves lane and PRESERVES source_path (no delete) ---
    let classified = store
        .classify_intake_item(item.item_id, IntakeLane::Accepted, Some("looks good"))
        .await
        .expect("classify first item accepted");
    assert_eq!(classified.lane, IntakeLane::Accepted, "lane moved to Accepted");
    assert_eq!(
        classified.source_path, source_path,
        "classify must preserve the original source_path (never delete the source)"
    );

    let rejected = store
        .classify_intake_item(item2.item_id, IntakeLane::Rejected, Some("dup"))
        .await
        .expect("classify second item rejected");
    assert_eq!(rejected.lane, IntakeLane::Rejected);
    assert_eq!(
        rejected.source_path, source_path_2,
        "rejecting an item only moves its lane; source_path is retained"
    );

    // --- lane counts correct after triage ---
    let counts1 = store
        .intake_lane_counts(batch.batch_id)
        .await
        .expect("lane counts post-triage");
    assert_eq!(counts1.new, 0, "no items left in New after triage");
    assert_eq!(counts1.accepted, 1);
    assert_eq!(counts1.rejected, 1);
    assert_eq!(counts1.deferred, 0);

    // The rejected item's row is still listable (no silent delete path).
    let all_items = store
        .list_intake_items(batch.batch_id, None)
        .await
        .expect("list all items");
    assert_eq!(all_items.len(), 2, "both source rows are preserved");

    // --- close SUCCEEDS once all items are classified out of New ---
    let closed = store
        .close_intake_batch(batch.batch_id)
        .await
        .expect("close batch after triage complete");
    assert_eq!(closed.status, BatchStatus::Closed, "batch is now Closed");
}

#[tokio::test]
async fn atelier_collections_membership_dedup_order_and_contact_sheet() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_collections_membership_dedup_order_and_contact_sheet: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    // --- create a collection ---
    let collection = store
        .create_collection(&NewCollection {
            name: format!("collection-{}", Uuid::new_v4()),
            notes: "core-data test collection".to_string(),
            tags: vec!["test".to_string(), "  test  ".to_string(), "blonde".to_string()],
            character_internal_id: None,
            sheet_version_id: None,
        })
        .await
        .expect("create collection");
    assert_eq!(
        collection.tags,
        vec!["test".to_string(), "blonde".to_string()],
        "tags are trimmed and de-duplicated on create"
    );

    // --- materialize media assets first, then add to the collection in order ---
    let asset_a = fresh_asset(&store).await;
    let asset_b = fresh_asset(&store).await;
    let asset_c = fresh_asset(&store).await;

    let inserted = store
        .add_images_to_collection(collection.collection_id, &[asset_a, asset_b, asset_c])
        .await
        .expect("add three images");
    assert_eq!(inserted, 3, "three distinct assets inserted");

    // --- re-adding the same assets does not duplicate ---
    let inserted_again = store
        .add_images_to_collection(collection.collection_id, &[asset_a, asset_b])
        .await
        .expect("re-add existing images");
    assert_eq!(
        inserted_again, 0,
        "re-adding existing memberships inserts nothing (ON CONFLICT DO NOTHING)"
    );

    // --- list ordering follows insertion sort_order ---
    let members = store
        .list_collection_images(collection.collection_id)
        .await
        .expect("list collection images");
    assert_eq!(members.len(), 3, "membership is exactly three (no duplicates)");
    assert_eq!(members[0].asset_id, asset_a, "first inserted is first in order");
    assert_eq!(members[1].asset_id, asset_b);
    assert_eq!(members[2].asset_id, asset_c);
    assert_eq!(members[0].sort_order, 0, "sort_order starts at 0 and increments");
    assert_eq!(members[1].sort_order, 1);
    assert_eq!(members[2].sort_order, 2);

    // --- create a contact sheet manifest snapshotting the membership ---
    let sheet = store
        .create_contact_sheet(
            &format!("sheet-{}", Uuid::new_v4()),
            "manual",
            None,
            &[asset_a, asset_b, asset_c],
            &["proof".to_string()],
            None,
            None,
        )
        .await
        .expect("create contact sheet");
    assert_eq!(sheet.image_count, 3, "manifest captured all three images");
    assert_eq!(sheet.source_type, "manual");
    let items = sheet
        .manifest
        .get("items")
        .and_then(|v| v.as_array())
        .expect("manifest items array");
    assert_eq!(items.len(), 3, "manifest items snapshot the membership");
    // The snapshot records content hashes so the sheet stays reproducible.
    assert!(
        items[0].get("content_hash").and_then(|v| v.as_str()).is_some(),
        "each manifest item records its content_hash"
    );
}

#[tokio::test]
async fn atelier_search_tags_rules_and_similarity() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_search_tags_rules_and_similarity: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    // --- ensure_tag dedups identical (normalized) text ---
    let tag_text = format!("Blonde-{}", Uuid::new_v4());
    let tag1 = store.ensure_tag(&tag_text).await.expect("ensure tag");
    let tag2 = store
        .ensure_tag(&format!("  {}  ", tag_text.to_uppercase()))
        .await
        .expect("ensure tag again (different case/whitespace)");
    assert_eq!(
        tag1.tag_id, tag2.tag_id,
        "identical normalized text dedups to the same tag row"
    );
    assert_eq!(tag1.text, tag_text.to_ascii_lowercase(), "tag text is normalized");

    // --- tag_character then list_character_tags ---
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-search-{}", Uuid::new_v4()),
            display_name: "Search Subject".to_string(),
        })
        .await
        .expect("create character");
    store
        .tag_character(character.internal_id, &tag_text, TagType::Manual)
        .await
        .expect("tag character manually");
    let manual_tags = store
        .list_character_tags(character.internal_id)
        .await
        .expect("list character tags");
    assert!(
        manual_tags
            .iter()
            .any(|t| t.text == tag_text.to_ascii_lowercase() && t.tag_type == TagType::Manual),
        "the manual tag is attached to the character"
    );

    // --- create_tag_rule then recompute_derived_tags emits derived tags ---
    let field_id = format!("hair-{}", Uuid::new_v4());
    let emit = format!("derived-blonde-{}", Uuid::new_v4());
    store
        .create_tag_rule(&NewTagRule {
            source_field_id: field_id.clone(),
            match_type: MatchType::Contains,
            pattern: "blonde".to_string(),
            emit_tag: emit.clone(),
            enabled: true,
        })
        .await
        .expect("create tag rule");

    let mut values = HashMap::new();
    values.insert(field_id.clone(), "long blonde hair".to_string());
    let derived = store
        .recompute_derived_tags(character.internal_id, &values)
        .await
        .expect("recompute derived tags");
    let emit_norm = emit.to_ascii_lowercase();
    assert!(
        derived.contains(&emit_norm),
        "matching rule emits its derived tag: {derived:?} should contain {emit_norm}"
    );
    let after = store
        .list_character_tags(character.internal_id)
        .await
        .expect("list tags after recompute");
    assert!(
        after
            .iter()
            .any(|t| t.text == emit_norm && t.tag_type == TagType::Derived),
        "derived tag is persisted with Derived provenance"
    );

    // --- upsert_similarity_projection then find_similar_assets by dHash ---
    let asset_near = fresh_asset(&store).await;
    let asset_far = fresh_asset(&store).await;
    // Target hash and a near hash differing by a single bit (distance 1).
    let target_hash = "0000000000000000";
    let near_hash = "0000000000000001";
    let far_hash = "ffffffffffffffff";

    store
        .upsert_similarity_projection(
            asset_near,
            Some(near_hash),
            serde_json::json!({ "dominant": ["#000000"] }),
        )
        .await
        .expect("project near asset");
    store
        .upsert_similarity_projection(
            asset_far,
            Some(far_hash),
            serde_json::json!({ "dominant": ["#ffffff"] }),
        )
        .await
        .expect("project far asset");

    // Search within a tight threshold: the near asset (distance 1) is a hit,
    // the far asset (distance 64) is excluded.
    let hits = store
        .find_similar_assets(target_hash, 4, 50, None)
        .await
        .expect("find similar assets");
    assert!(
        hits.iter().any(|h| h.asset_internal_id == asset_near),
        "the near (1-bit) asset is returned within the threshold"
    );
    assert!(
        !hits.iter().any(|h| h.asset_internal_id == asset_far),
        "the far (64-bit) asset is excluded by the threshold"
    );
    let near_hit = hits
        .iter()
        .find(|h| h.asset_internal_id == asset_near)
        .expect("near hit present");
    assert_eq!(near_hit.distance, 1, "Hamming distance is exactly 1");
}

#[tokio::test]
async fn atelier_exports_request_result_idempotency_and_manifest() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_exports_request_result_idempotency_and_manifest: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    // --- create a character + an append-only sheet version (foundation) ---
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-export-{}", Uuid::new_v4()),
            display_name: "Export Subject".to_string(),
        })
        .await
        .expect("create character");
    let version = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: "name: Export Subject".to_string(),
            author: "operator".to_string(),
            tool: None,
        })
        .await
        .expect("append sheet version");

    // --- request_sheet_export pinned to that version ---
    let request = store
        .request_sheet_export(&NewExportRequest {
            character_internal_id: character.internal_id,
            sheet_version_id: version.version_id,
            format: ExportFormat::Markdown,
            label: Some("share pack".to_string()),
            requested_by: "operator".to_string(),
        })
        .await
        .expect("request sheet export");
    assert_eq!(request.format, ExportFormat::Markdown);

    // --- record_export_result, then re-record identical content_hash is idempotent ---
    let artifact_ref = format!("artifact://atelier/export/{}", Uuid::new_v4());
    let content_hash = format!("sha256-{}", Uuid::new_v4());
    let result1 = store
        .record_export_result(request.export_id, &artifact_ref, &content_hash, 2048)
        .await
        .expect("record export result");
    let result2 = store
        .record_export_result(request.export_id, &artifact_ref, &content_hash, 2048)
        .await
        .expect("re-record identical export result");
    assert_eq!(
        result1.result_id, result2.result_id,
        "re-recording identical (export_id, content_hash) returns the same result"
    );

    // --- add_manifest_entry seq increments ---
    let entry1 = store
        .add_manifest_entry(
            request.export_id,
            ManifestItemKind::Sheet,
            &artifact_ref,
            "sheet/character.md",
        )
        .await
        .expect("add sheet manifest entry");
    assert_eq!(entry1.seq, 1, "first manifest entry is seq 1");

    let media_ref = format!("artifact://atelier/media/{}", Uuid::new_v4());
    let entry2 = store
        .add_manifest_entry(
            request.export_id,
            ManifestItemKind::Media,
            &media_ref,
            "images/a.png",
        )
        .await
        .expect("add media manifest entry");
    assert_eq!(entry2.seq, 2, "second manifest entry is seq 2 (increments)");

    // --- export_manifest lists in seq order ---
    let manifest = store
        .export_manifest(request.export_id)
        .await
        .expect("read export manifest");
    assert_eq!(manifest.len(), 2, "two manifest entries recorded");
    assert_eq!(manifest[0].seq, 1, "manifest ordered ascending by seq");
    assert_eq!(manifest[0].kind, ManifestItemKind::Sheet);
    assert_eq!(manifest[1].seq, 2);
    assert_eq!(manifest[1].kind, ManifestItemKind::Media);
}

#[tokio::test]
async fn atelier_annotation_sequence_update_count_and_remove() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_annotation_sequence_update_count_and_remove: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    // --- materialize a media asset to annotate ---
    let asset_id = fresh_asset(&store).await;

    // --- add_media_annotation seq increments (1, 2) ---
    let ann1 = store
        .add_media_annotation(&NewMediaAnnotation {
            asset_id,
            kind: AnnotationKind::Point,
            label: Some("focus".to_string()),
            note: "left eye".to_string(),
            geometry: serde_json::json!({ "x": 0.25, "y": 0.40 }),
            author: "operator".to_string(),
        })
        .await
        .expect("add first annotation");
    assert_eq!(ann1.seq, 1, "first annotation is seq 1");

    let ann2 = store
        .add_media_annotation(&NewMediaAnnotation {
            asset_id,
            kind: AnnotationKind::Box,
            label: Some("wardrobe".to_string()),
            note: "jacket".to_string(),
            geometry: serde_json::json!({ "x": 0.1, "y": 0.1, "w": 0.3, "h": 0.4 }),
            author: "operator".to_string(),
        })
        .await
        .expect("add second annotation");
    assert_eq!(ann2.seq, 2, "second annotation is seq 2 (increments)");

    // --- list in paint/export order, get, update note ---
    let listed = store
        .list_media_annotations(asset_id)
        .await
        .expect("list annotations");
    assert_eq!(listed.len(), 2, "both annotations present in order");
    assert_eq!(listed[0].annotation_id, ann1.annotation_id);
    assert_eq!(listed[1].annotation_id, ann2.annotation_id);

    let fetched = store
        .get_media_annotation(ann1.annotation_id)
        .await
        .expect("get annotation");
    assert_eq!(fetched.note, "left eye");

    let updated = store
        .update_media_annotation_note(ann1.annotation_id, "right eye", Some("focus-2"))
        .await
        .expect("update annotation note");
    assert_eq!(updated.note, "right eye", "note is updated in place");
    assert_eq!(updated.label.as_deref(), Some("focus-2"), "label updated");
    assert_eq!(
        updated.geometry, fetched.geometry,
        "geometry is immutable on note update"
    );

    // --- count ---
    let count_before = store
        .count_media_annotations(asset_id)
        .await
        .expect("count annotations");
    assert_eq!(count_before, 2, "two annotations on the asset");

    // --- remove returns the asset id and decrements the count ---
    let removed_asset = store
        .remove_media_annotation(ann2.annotation_id)
        .await
        .expect("remove annotation");
    assert_eq!(removed_asset, asset_id, "remove returns the parent asset id");
    let count_after = store
        .count_media_annotations(asset_id)
        .await
        .expect("count annotations after remove");
    assert_eq!(count_after, 1, "removal decrements the annotation count");
}

#[tokio::test]
async fn atelier_settings_upsert_scope_redaction_and_delete() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_settings_upsert_scope_redaction_and_delete: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    // --- set a global preference, then re-set the SAME key: UPDATE in place ---
    let key = format!("libraryRoot-{}", Uuid::new_v4());
    let pref1 = store
        .set_preference(&SetPreference {
            scope: PreferenceScope::Global,
            key: key.clone(),
            value_type: PreferenceType::Path,
            value: "/data/library".to_string(),
            redacted: false,
        })
        .await
        .expect("set global preference");
    let pref2 = store
        .set_preference(&SetPreference {
            scope: PreferenceScope::Global,
            key: key.clone(),
            value_type: PreferenceType::Path,
            value: "/data/library-v2".to_string(),
            redacted: false,
        })
        .await
        .expect("re-set same global key");
    assert_eq!(
        pref1.preference_id, pref2.preference_id,
        "re-setting the same (global, key) UPDATES in place (UNIQUE NULLS NOT DISTINCT)"
    );
    assert_eq!(pref2.value, "/data/library-v2", "value updated in place");

    // Confirm exactly one row for this global key (no duplicate from the upsert).
    let globals = store
        .list_preferences(PreferenceScope::Global, false)
        .await
        .expect("list global preferences");
    let matches = globals.iter().filter(|p| p.key == key).count();
    assert_eq!(matches, 1, "global key has exactly one row after re-set");

    // --- set a character-scoped preference with the same key text ---
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("char-settings-{}", Uuid::new_v4()),
            display_name: "Settings Subject".to_string(),
        })
        .await
        .expect("create character");
    let char_scope = PreferenceScope::Character(character.internal_id);
    let char_pref = store
        .set_preference(&SetPreference {
            scope: char_scope,
            key: key.clone(),
            value_type: PreferenceType::String,
            value: "char-specific".to_string(),
            redacted: false,
        })
        .await
        .expect("set character-scoped preference");
    assert_ne!(
        char_pref.preference_id, pref2.preference_id,
        "the same key in a different scope is a distinct preference row"
    );

    // --- get_preference resolves the right scope ---
    let got_global = store
        .get_preference(PreferenceScope::Global, &key)
        .await
        .expect("get global preference")
        .expect("global preference present");
    assert_eq!(got_global.value, "/data/library-v2");
    let got_char = store
        .get_preference(char_scope, &key)
        .await
        .expect("get character preference")
        .expect("character preference present");
    assert_eq!(got_char.value, "char-specific");

    // --- redacted_projection hides secret values ---
    let secret_key = format!("apiToken-{}", Uuid::new_v4());
    let secret = store
        .set_preference(&SetPreference {
            scope: PreferenceScope::Global,
            key: secret_key.clone(),
            value_type: PreferenceType::String,
            value: "super-secret-token".to_string(),
            redacted: true,
        })
        .await
        .expect("set secret preference");
    let projected = secret.redacted_projection();
    assert_ne!(
        projected.value, "super-secret-token",
        "redacted projection must not expose the raw secret value"
    );
    assert_eq!(
        projected.value, "[REDACTED]",
        "secret value is replaced by the redaction placeholder"
    );

    // list with redact=true also masks the secret.
    let redacted_list = store
        .list_preferences(PreferenceScope::Global, true)
        .await
        .expect("list redacted globals");
    let listed_secret = redacted_list
        .iter()
        .find(|p| p.key == secret_key)
        .expect("secret present in redacted list");
    assert_eq!(
        listed_secret.value, "[REDACTED]",
        "list redaction masks secret values"
    );

    // --- delete returns true once, false on a second delete ---
    let deleted = store
        .delete_preference(PreferenceScope::Global, &key)
        .await
        .expect("delete global preference");
    assert!(deleted, "deleting an existing preference returns true");
    let deleted_again = store
        .delete_preference(PreferenceScope::Global, &key)
        .await
        .expect("delete missing preference");
    assert!(!deleted_again, "deleting a missing preference returns false");
    let after_delete = store
        .get_preference(PreferenceScope::Global, &key)
        .await
        .expect("get after delete");
    assert!(after_delete.is_none(), "deleted preference is gone");
}
