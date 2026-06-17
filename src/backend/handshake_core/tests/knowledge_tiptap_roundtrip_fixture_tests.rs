//! WP-KERNEL-009 MT-236 Tiptap/ProseMirror roundtrip fixture.
//!
//! This test proves every WP-009 custom editor node shape survives the real
//! RichDocument authority path: PostgreSQL create, optimistic save, load,
//! version history, and CRDT promotion metadata stamping. There is no SQLite,
//! mock, generated markdown, or frontend-only serializer in the proof path.

#![recursion_limit = "256"]

mod knowledge_pg_support;

use base64::Engine;
use handshake_core::kernel::crdt::actor_site::{
    knowledge_crdt_identity, KnowledgeActorIdV1, KnowledgeActorKind,
};
use handshake_core::kernel::crdt::persistence::sha256_hex;
use handshake_core::kernel::crdt::rich_document_snapshot::{
    build_rich_document_snapshot_record, restore_rich_document_snapshot,
    RichDocumentSnapshotPayloadV1, RICH_DOCUMENT_SCHEMA_ID,
    RICH_DOCUMENT_SNAPSHOT_PAYLOAD_SCHEMA_ID,
};
use handshake_core::kernel::crdt::state_vector::KnowledgeStateVectorV1;
use handshake_core::kernel::crdt::yjs_bridge::{
    push_yjs_update, YjsPushOutcomeV1, YjsUpdateEnvelopeV1, YJS_UPDATE_ENCODING_V1,
    YJS_UPDATE_ENVELOPE_SCHEMA_ID,
};
use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use handshake_core::knowledge_document::backlink::{DocumentLinkKind, DocumentLinkReferences};
use handshake_core::knowledge_document::block_tree::{
    BlockKind, BlockTree, DOCUMENT_SCHEMA_VERSION,
};
use handshake_core::storage::knowledge::{
    KnowledgeStore, NewKnowledgeRichDocument, UpsertKnowledgeDocumentBacklink,
};
use handshake_core::storage::{Database, NewDocument, WriteContext};
use knowledge_pg_support::knowledge_pg;
use serde_json::{json, Value};
use uuid::Uuid;

fn all_custom_nodes_doc(extra_text: &str) -> Value {
    json!({
        "type": "doc",
        "content": [
            { "type": "heading", "attrs": { "level": 1, "blockId": "hsk-heading" }, "content": [{ "type": "text", "text": "MT-236" }] },
            { "type": "paragraph", "attrs": { "blockId": "hsk-paragraph" }, "content": [
                { "type": "text", "text": "Inline nodes " },
                { "type": "hsLink", "attrs": { "refKind": "note", "refValue": "Runbook", "label": "Runbook", "resolved": true } },
                { "type": "text", "text": " " },
                { "type": "hsLink", "attrs": { "refKind": "file", "refValue": "src/lib/editor.ts", "label": "editor.ts", "resolved": true } },
                { "type": "text", "text": " " },
                { "type": "hsLink", "attrs": { "refKind": "folder", "refValue": "src/lib", "label": "src/lib", "resolved": true } },
                { "type": "text", "text": " " },
                { "type": "hsLink", "attrs": { "refKind": "project", "refValue": "Handshake", "label": "Handshake", "resolved": true } },
                { "type": "text", "text": " " },
                { "type": "hsLink", "attrs": { "refKind": "spec", "refValue": "7.1.1.8", "label": "Rich editor", "resolved": true } },
                { "type": "text", "text": " " },
                { "type": "hsLink", "attrs": { "refKind": "wp", "refValue": "WP-KERNEL-009", "label": "WP-KERNEL-009", "resolved": true } },
                { "type": "text", "text": " " },
                { "type": "hsLink", "attrs": { "refKind": "symbol", "refValue": "RichTextEditor", "label": "RichTextEditor", "resolved": true } },
                { "type": "text", "text": " " },
                { "type": "hsLink", "attrs": { "refKind": "images", "refValue": "KIMG-fixture", "label": "images", "resolved": true } },
                { "type": "text", "text": " " },
                { "type": "hsLink", "attrs": { "refKind": "video", "refValue": "KVID-fixture", "label": "video", "resolved": true } },
                { "type": "text", "text": " " },
                { "type": "hsLink", "attrs": { "refKind": "album", "refValue": "KALB-fixture", "label": "album", "resolved": true } },
                { "type": "text", "text": " " },
                { "type": "hsLink", "attrs": { "refKind": "slideshow", "refValue": "KSLD-fixture", "label": "slideshow", "resolved": true } },
                { "type": "text", "text": " " },
                { "type": "mention", "attrs": { "id": "operator-1", "label": "Operator One" } },
                { "type": "text", "text": " " },
                { "type": "tagMention", "attrs": { "id": "tag-fixture", "label": "fixture" } },
                { "type": "text", "text": extra_text }
            ] },
            { "type": "bulletList", "attrs": { "blockId": "hsk-bullets" }, "content": [
                { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "bullet alpha" }] }] },
                { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "bullet beta" }] }] }
            ] },
            { "type": "orderedList", "attrs": { "blockId": "hsk-ordered" }, "content": [
                { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "first" }] }] },
                { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "second" }] }] }
            ] },
            { "type": "codeBlock", "attrs": { "language": "rust", "blockId": "hsk-code" }, "content": [{ "type": "text", "text": "fn proof() -> &'static str { \"mt236\" }" }] },
            { "type": "monacoCodeBlock", "attrs": { "language": "rust", "code": "fn proof() { println!(\"mt236\"); }", "contentHash": "0cdc2888", "blockId": "hsk-monaco" } },
            { "type": "table", "attrs": { "blockId": "hsk-table" }, "content": [
                { "type": "tableRow", "content": [
                    { "type": "tableHeader", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Key" }] }] },
                    { "type": "tableHeader", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Value" }] }] }
                ] },
                { "type": "tableRow", "content": [
                    { "type": "tableCell", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "roundtrip" }] }] },
                    { "type": "tableCell", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "ok" }] }] }
                ] }
            ] },
            { "type": "taskList", "attrs": { "blockId": "hsk-tasks" }, "content": [
                { "type": "taskItem", "attrs": { "checked": true }, "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "custom task item" }] }] }
            ] },
            { "type": "blockquote", "attrs": { "blockId": "hsk-quote" }, "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "quoted note" }] }] },
            { "type": "image", "attrs": { "target": "KMED-image", "display": { "fold": false }, "blockId": "hsk-image" }, "content": [{ "type": "text", "text": "image caption" }] },
            { "type": "video", "attrs": { "target": "KMED-video", "blockId": "hsk-video" }, "content": [{ "type": "text", "text": "video caption" }] },
            { "type": "album", "attrs": { "target": "KALB-fixture", "blockId": "hsk-album" }, "content": [{ "type": "text", "text": "album caption" }] },
            { "type": "slideshow", "attrs": { "target": "KSLD-fixture", "blockId": "hsk-slideshow" }, "content": [{ "type": "text", "text": "slides caption" }] },
            { "type": "fileLink", "attrs": { "target": "src/lib/editor.ts", "blockId": "hsk-file" }, "content": [{ "type": "text", "text": "file" }] },
            { "type": "folderLink", "attrs": { "target": "src/lib", "blockId": "hsk-folder" }, "content": [{ "type": "text", "text": "folder" }] },
            { "type": "projectLink", "attrs": { "target": "Handshake", "blockId": "hsk-project" }, "content": [{ "type": "text", "text": "project" }] },
            { "type": "specLink", "attrs": { "target": "7.1.1.8", "blockId": "hsk-spec" }, "content": [{ "type": "text", "text": "spec" }] },
            { "type": "wpLink", "attrs": { "target": "WP-KERNEL-009", "blockId": "hsk-wp" }, "content": [{ "type": "text", "text": "wp" }] },
            { "type": "symbolLink", "attrs": { "target": "RichTextEditor", "blockId": "hsk-symbol" }, "content": [{ "type": "text", "text": "symbol" }] },
            { "type": "importedRaw", "attrs": { "source_format": "html", "repairable": true, "blockId": "hsk-imported" }, "content": [{ "type": "text", "text": "<aside>raw import</aside>" }] }
        ]
    })
}

fn new_doc(
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    snapshot_id: &str,
    event_id: &str,
    content: Value,
) -> NewKnowledgeRichDocument {
    NewKnowledgeRichDocument {
        workspace_id: workspace_id.to_string(),
        document_id: Some(document_id.to_string()),
        title: "MT-236 Tiptap Roundtrip".to_string(),
        schema_version: DOCUMENT_SCHEMA_VERSION.to_string(),
        content_json: content,
        crdt_document_id: Some(crdt_document_id.to_string()),
        crdt_snapshot_id: Some(snapshot_id.to_string()),
        promotion_receipt_event_id: Some(event_id.to_string()),
        project_ref: Some("PRJ-mt236".to_string()),
        folder_ref: Some("fixtures".to_string()),
        authority_label: Some("promoted".to_string()),
        owner_actor_kind: Some("operator".to_string()),
        owner_actor_id: Some("op-mt236".to_string()),
    }
}

fn envelope(
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    update_id: &str,
    update_bytes: &[u8],
    before: &KnowledgeStateVectorV1,
    after: &KnowledgeStateVectorV1,
    actor: &KnowledgeActorIdV1,
) -> YjsUpdateEnvelopeV1 {
    let site_id = handshake_core::kernel::crdt::actor_site::derive_knowledge_site_id(
        workspace_id,
        crdt_document_id,
        actor,
    )
    .site_id;
    YjsUpdateEnvelopeV1 {
        schema_id: YJS_UPDATE_ENVELOPE_SCHEMA_ID.to_string(),
        workspace_id: workspace_id.to_string(),
        document_id: document_id.to_string(),
        crdt_document_id: crdt_document_id.to_string(),
        update_id: update_id.to_string(),
        actor_id: actor.canonical(),
        site_id,
        session_id: "SR-MT236".to_string(),
        trace_id: format!("trace-{update_id}"),
        document_schema_id: RICH_DOCUMENT_SCHEMA_ID.to_string(),
        update_b64: base64::engine::general_purpose::STANDARD.encode(update_bytes),
        update_sha256: sha256_hex(update_bytes),
        state_vector_before: before.encode(),
        state_vector_after: after.encode(),
        encoding: YJS_UPDATE_ENCODING_V1.to_string(),
    }
}

async fn append_snapshot_and_receipt(
    db: &dyn handshake_core::storage::Database,
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    snapshot_id: &str,
    doc_json: Value,
    update_ids: &[String],
    covered_update_seq: u64,
    state_vector: &str,
    actor: &KnowledgeActorIdV1,
) -> String {
    let event = NewKernelEvent::builder(
        format!("KTR-MT236-{snapshot_id}"),
        "SR-MT236".to_string(),
        KernelEventType::KnowledgeCrdtSnapshotRecorded,
        KernelActor::Operator(actor.canonical()),
    )
    .aggregate("knowledge_crdt_document", crdt_document_id.to_string())
    .idempotency_key(format!("mt236:{snapshot_id}:snapshot"))
    .source_component("knowledge_tiptap_roundtrip_fixture_tests")
    .payload(json!({"snapshot_id": snapshot_id, "covered_update_seq": covered_update_seq}))
    .build()
    .expect("snapshot receipt builds");
    let stored_event = db.append_kernel_event(event).await.expect("append receipt");
    let identity = knowledge_crdt_identity(
        workspace_id,
        document_id,
        crdt_document_id,
        RICH_DOCUMENT_SCHEMA_ID,
        actor,
        &format!("trace-{snapshot_id}"),
    );
    let payload = RichDocumentSnapshotPayloadV1 {
        schema_id: RICH_DOCUMENT_SNAPSHOT_PAYLOAD_SCHEMA_ID.to_string(),
        document_schema_id: RICH_DOCUMENT_SCHEMA_ID.to_string(),
        prosemirror_schema_version: DOCUMENT_SCHEMA_VERSION.to_string(),
        doc_json,
        state_vector: state_vector.to_string(),
        covered_update_seq,
    };
    let update_refs: Vec<&str> = update_ids.iter().map(String::as_str).collect();
    let (record, bytes) = build_rich_document_snapshot_record(
        &identity,
        snapshot_id,
        &payload,
        &stored_event.event_id,
        &update_refs,
    )
    .expect("snapshot record builds");
    let restored = restore_rich_document_snapshot(&record, &bytes).expect("snapshot restores");
    assert_eq!(restored.doc_json, payload.doc_json);
    db.append_kernel_crdt_snapshot(record, bytes)
        .await
        .expect("append snapshot");
    stored_event.event_id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt236_custom_tiptap_nodes_survive_save_load_history_and_crdt_promotion() {
    let pg = knowledge_pg()
        .await
        .expect("MT-236 requires PostgreSQL proof; missing PostgreSQL must fail closed");
    let workspace_id = pg.create_workspace().await;
    let suffix = Uuid::now_v7().simple().to_string();
    let document = pg
        .db
        .create_document(
            &WriteContext::human(None),
            NewDocument {
                workspace_id: workspace_id.clone(),
                title: "MT-236 Tiptap Roundtrip Base".to_string(),
            },
        )
        .await
        .expect("create base document");
    let document_id = document.id;
    let crdt_document_id = format!("crdt-mt236-{suffix}");
    let actor = KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op-mt236").expect("actor");

    let mut state_before = KnowledgeStateVectorV1::new();
    let mut state_after_v1 = state_before.clone();
    let site_id = handshake_core::kernel::crdt::actor_site::derive_knowledge_site_id(
        &workspace_id,
        &crdt_document_id,
        &actor,
    )
    .site_id;
    state_after_v1.increment(&site_id);
    let update_v1 = envelope(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        "mt236-u1",
        b"mt236 custom node draft v1",
        &state_before,
        &state_after_v1,
        &actor,
    );
    assert!(matches!(
        push_yjs_update(&pg.db, &update_v1).await.expect("push v1"),
        YjsPushOutcomeV1::Stored { .. }
    ));

    let v1_json = all_custom_nodes_doc(" v1");
    let v1_event_id = append_snapshot_and_receipt(
        &pg.db,
        &workspace_id,
        &document_id,
        &crdt_document_id,
        "snap-mt236-v1",
        v1_json.clone(),
        &[update_v1.update_id.clone()],
        1,
        &state_after_v1.encode(),
        &actor,
    )
    .await;
    let created = pg
        .db
        .create_knowledge_rich_document(new_doc(
            &workspace_id,
            &document_id,
            &crdt_document_id,
            "snap-mt236-v1",
            &v1_event_id,
            v1_json.clone(),
        ))
        .await
        .expect("create rich document");

    let mut state_after_v2 = state_after_v1.clone();
    state_before = state_after_v1;
    state_after_v2.increment(&site_id);
    let update_v2 = envelope(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        "mt236-u2",
        b"mt236 custom node draft v2",
        &state_before,
        &state_after_v2,
        &actor,
    );
    assert!(matches!(
        push_yjs_update(&pg.db, &update_v2).await.expect("push v2"),
        YjsPushOutcomeV1::Stored { .. }
    ));

    let v2_json = all_custom_nodes_doc(" v2");
    let v2_event_id = append_snapshot_and_receipt(
        &pg.db,
        &workspace_id,
        &document_id,
        &crdt_document_id,
        "snap-mt236-v2",
        v2_json.clone(),
        &[update_v1.update_id.clone(), update_v2.update_id.clone()],
        2,
        &state_after_v2.encode(),
        &actor,
    )
    .await;
    let saved = pg
        .db
        .save_knowledge_rich_document_version(
            &created.rich_document_id,
            1,
            v2_json.clone(),
            Some(crdt_document_id.as_str()),
            Some("snap-mt236-v2"),
            Some(&v2_event_id),
        )
        .await
        .expect("save promoted v2");

    assert_eq!(saved.doc_version, 2);
    assert_eq!(saved.content_json, v2_json);
    assert_eq!(
        saved.crdt_document_id.as_deref(),
        Some(crdt_document_id.as_str())
    );
    assert_eq!(saved.crdt_snapshot_id.as_deref(), Some("snap-mt236-v2"));
    assert_eq!(
        saved.promotion_receipt_event_id.as_deref(),
        Some(v2_event_id.as_str())
    );

    let mut state_after_v3 = state_after_v2.clone();
    let state_before_v3 = state_after_v2;
    state_after_v3.increment(&site_id);
    let update_v3 = envelope(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        "mt236-u3",
        b"mt236 custom node draft v3 idempotent",
        &state_before_v3,
        &state_after_v3,
        &actor,
    );
    assert!(matches!(
        push_yjs_update(&pg.db, &update_v3).await.expect("push v3"),
        YjsPushOutcomeV1::Stored { .. }
    ));

    let v3_json = all_custom_nodes_doc(" v3");
    let v3_event_id = append_snapshot_and_receipt(
        &pg.db,
        &workspace_id,
        &document_id,
        &crdt_document_id,
        "snap-mt236-v3",
        v3_json.clone(),
        &[
            update_v1.update_id.clone(),
            update_v2.update_id.clone(),
            update_v3.update_id.clone(),
        ],
        3,
        &state_after_v3.encode(),
        &actor,
    )
    .await;
    let idempotency_key = format!("mt236-rich-save-{suffix}");
    let first_idempotent = pg
        .db
        .save_knowledge_rich_document_version_idempotent(
            &idempotency_key,
            &created.rich_document_id,
            2,
            v3_json.clone(),
            Some(crdt_document_id.as_str()),
            Some("snap-mt236-v3"),
            Some(&v3_event_id),
        )
        .await
        .expect("first idempotent save");
    assert!(!first_idempotent.replayed);
    assert_eq!(first_idempotent.value.doc_version, 3);
    assert_eq!(
        first_idempotent.value.crdt_snapshot_id.as_deref(),
        Some("snap-mt236-v3")
    );
    assert_eq!(
        first_idempotent.value.promotion_receipt_event_id.as_deref(),
        Some(v3_event_id.as_str())
    );

    let replayed_idempotent = pg
        .db
        .save_knowledge_rich_document_version_idempotent(
            &idempotency_key,
            &created.rich_document_id,
            2,
            v3_json.clone(),
            Some(crdt_document_id.as_str()),
            Some("snap-mt236-v3"),
            Some(&v3_event_id),
        )
        .await
        .expect("replayed idempotent save");
    assert!(replayed_idempotent.replayed);
    assert_eq!(replayed_idempotent.value.doc_version, 3);
    assert_eq!(
        replayed_idempotent.value.crdt_snapshot_id.as_deref(),
        Some("snap-mt236-v3")
    );

    let loaded = pg
        .db
        .get_knowledge_rich_document(&created.rich_document_id)
        .await
        .expect("load")
        .expect("document exists");
    assert_eq!(loaded.doc_version, 3);
    assert_eq!(loaded.content_json, v3_json);
    assert_eq!(loaded.crdt_snapshot_id.as_deref(), Some("snap-mt236-v3"));
    assert_eq!(
        loaded.promotion_receipt_event_id.as_deref(),
        Some(v3_event_id.as_str())
    );

    let tree = BlockTree::from_document_json(
        &loaded.rich_document_id,
        &loaded.schema_version,
        &loaded.content_json,
    )
    .expect("block tree parses every supported custom block");
    let kinds: Vec<BlockKind> = tree.blocks.iter().map(|block| block.kind).collect();
    assert_eq!(
        kinds,
        vec![
            BlockKind::Heading,
            BlockKind::Paragraph,
            BlockKind::BulletList,
            BlockKind::OrderedList,
            BlockKind::CodeBlock,
            BlockKind::CodeBlock,
            BlockKind::Table,
            BlockKind::TaskList,
            BlockKind::Blockquote,
            BlockKind::Image,
            BlockKind::Video,
            BlockKind::Album,
            BlockKind::Slideshow,
            BlockKind::FileLink,
            BlockKind::FolderLink,
            BlockKind::ProjectLink,
            BlockKind::SpecLink,
            BlockKind::WpLink,
            BlockKind::SymbolLink,
            BlockKind::ImportedRaw,
        ]
    );
    assert_eq!(tree.to_document_json(), loaded.content_json);
    assert!(
        loaded
            .content_json
            .to_string()
            .contains("\"type\":\"hsLink\""),
        "inline hsLink node must not be stripped by backend save/load"
    );
    assert!(
        loaded
            .content_json
            .to_string()
            .contains("\"type\":\"tagMention\""),
        "inline tagMention node must not be stripped by backend save/load"
    );

    let refs = DocumentLinkReferences::extract(&tree);
    let has_ref = |kind: DocumentLinkKind, target: &str| {
        refs.references
            .iter()
            .any(|r| r.kind == kind && r.target == target)
    };
    assert!(has_ref(DocumentLinkKind::File, "src/lib/editor.ts"));
    assert!(has_ref(DocumentLinkKind::Wp, "WP-KERNEL-009"));
    assert!(has_ref(DocumentLinkKind::Mention, "operator-1"));
    assert!(has_ref(DocumentLinkKind::Tag, "tag-fixture"));
    assert!(has_ref(DocumentLinkKind::Wikilink, "video:KVID-fixture"));

    let persisted_backlinks = pg
        .db
        .replace_knowledge_document_backlinks(
            &loaded.rich_document_id,
            refs.references
                .iter()
                .map(|r| UpsertKnowledgeDocumentBacklink {
                    workspace_id: loaded.workspace_id.clone(),
                    relationship_id: r.relationship_id.clone(),
                    source_document_id: loaded.rich_document_id.clone(),
                    link_kind: r.kind.as_str().to_string(),
                    target: r.target.clone(),
                    block_id: r.block_id.clone(),
                })
                .collect(),
        )
        .await
        .expect("persist inline Tiptap backlinks");
    let persisted_ref = |kind: &str, target: &str| {
        persisted_backlinks
            .iter()
            .any(|b| b.link_kind == kind && b.target == target)
    };
    assert!(
        persisted_ref("file", "src/lib/editor.ts"),
        "inline file hsLink node must persist as a backend backlink"
    );
    assert!(
        persisted_ref("wp", "WP-KERNEL-009"),
        "inline wp hsLink node must persist as a backend backlink"
    );
    assert!(
        persisted_ref("wikilink", "video:KVID-fixture"),
        "unsupported media hsLink refKind must persist as a namespaced wikilink fallback"
    );
    assert!(
        persisted_ref("mention", "operator-1"),
        "inline mention node must persist as a backend backlink"
    );
    assert!(
        persisted_ref("tag", "tag-fixture"),
        "inline tagMention node must persist as a backend backlink"
    );

    let versions = pg
        .db
        .list_knowledge_rich_document_versions(&created.rich_document_id)
        .await
        .expect("version history");
    assert_eq!(versions.len(), 3);
    assert_eq!(versions[0].content_json, v1_json);
    assert_eq!(
        versions[0].crdt_snapshot_id.as_deref(),
        Some("snap-mt236-v1")
    );
    assert_eq!(
        versions[0].promotion_receipt_event_id.as_deref(),
        Some(v1_event_id.as_str())
    );
    assert_eq!(versions[1].content_json, v2_json);
    assert_eq!(
        versions[1].crdt_snapshot_id.as_deref(),
        Some("snap-mt236-v2")
    );
    assert_eq!(
        versions[1].promotion_receipt_event_id.as_deref(),
        Some(v2_event_id.as_str())
    );
    assert_eq!(versions[2].content_json, v3_json);
    assert_eq!(
        versions[2].crdt_snapshot_id.as_deref(),
        Some("snap-mt236-v3")
    );
    assert_eq!(
        versions[2].promotion_receipt_event_id.as_deref(),
        Some(v3_event_id.as_str())
    );
}
