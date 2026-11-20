use super::*;
use serde_json::json;

fn source(server: &NoteServer, content: serde_json::Value) {
    server.responses.lock().unwrap().document["document"]["content_json"] =
        json!({"type":"doc","content":content});
}
fn blocks() -> serde_json::Value {
    json!([
        {"type":"paragraph","content":[{"type":"text","text":"body without persisted identity"}]},
        {"type":"paragraph","attrs":{"block_id":"paragraph-id"},"content":[{"type":"text","text":"body"}]},
        {"type":"heading","attrs":{"level":2,"block_id":"heading-id"},"content":[{"type":"text","text":"Heading"}]}
    ])
}

#[tokio::test]
async fn heading_reference_preserves_owner_persisted_identity_and_alias_without_loom() {
    let server = NoteServer::new().await;
    source(&server, blocks());
    let provider = server.provider();
    let (mut core, trigger) = completion("[[Road#heading|Exact Alias");
    let page = provider.lookup(&trigger).await.unwrap();
    assert_eq!(page.rows.len(), 1);
    let plan = admit(&mut core, &trigger, &page, 0);
    let Material::Inline(Child::HsLink(link)) = provider.resolve(&page.rows[0], &plan).await.unwrap() else { panic!("heading reference"); };
    assert_eq!(link.ref_value, "source-note");
    assert_eq!(link.target_block_id.as_deref(), Some("heading-id"));
    assert_eq!(link.target_kind, Some(crate::rich_editor::document_model::ReferenceTargetKind::Heading));
    assert_eq!(link.label, "Exact Alias");
    let requests = &server.responses.lock().unwrap().requests;
    assert_eq!(requests.iter().filter(|r| r.contains("?workspace_id=")).count(), 1);
    assert!(!requests.iter().any(|r| r.contains("/loom/")));
}

#[tokio::test]
async fn block_template_matches_one_block_but_returns_the_entire_owning_note() {
    let server = NoteServer::new().await;
    source(&server, blocks());
    let provider = server.provider();
    let (mut core, trigger) = completion("/Road^body");
    let page = provider.lookup(&trigger).await.unwrap();
    assert_eq!(page.rows.len(), 1);
    assert_eq!(page.rows[0].item.detail.as_deref(), Some("Insert owning note: Road"));
    let plan = admit(&mut core, &trigger, &page, 0);
    let Material::Template(document) = provider.resolve(&page.rows[0], &plan).await.unwrap() else { panic!("owning note template"); };
    assert_eq!(document.children.len(), 3);
    assert_eq!(document.children[2].as_block().unwrap().attrs["block_id"], json!("heading-id"));
}

#[tokio::test]
async fn block_resolve_rejects_removed_retyped_wrong_owner_version_denied_and_revoked_sources() {
    for change in ["deleted", "kind", "version", "owner", "workspace", "updated", "denied", "revoked"] {
        let server = NoteServer::new().await;
        source(&server, blocks());
        let provider = server.provider();
        let (mut core, trigger) = completion("[[Road#heading");
        let page = provider.lookup(&trigger).await.unwrap();
        let plan = admit(&mut core, &trigger, &page, 0);
        {
            let mut state = server.responses.lock().unwrap();
            match change {
                "deleted" => state.document["document"]["content_json"]["content"][2]["attrs"]["block_id"] = json!("replacement"),
                "kind" => state.document["document"]["content_json"]["content"][2]["type"] = json!("paragraph"),
                "version" => state.document["document"]["doc_version"] = json!(5),
                "owner" => state.document["document"]["rich_document_id"] = json!("other"),
                "workspace" => state.document["document"]["workspace_id"] = json!("other"),
                "updated" => state.document["document"]["updated_at"] = json!("changed"),
                "denied" => state.load_status = 403,
                "revoked" => provider.account.invalidate(),
                _ => unreachable!(),
            }
        }
        assert!(provider.resolve(&page.rows[0], &plan).await.is_err(), "accepted {change}");
    }
}

#[tokio::test]
async fn partial_page_does_not_bound_direct_resolution_of_a_later_persisted_block() {
    let server = NoteServer::new().await;
    source(&server, serde_json::Value::Array((0..45).map(|n| json!({"type":"paragraph",
        "attrs":{"block_id":format!("persisted-{n}")},"content":[{"type":"text","text":format!("body {n}")}]})).collect()));
    let provider = server.provider();
    let (_, all) = completion("((body");
    let page = provider.lookup(&all).await.unwrap();
    assert_eq!(page.status, DeliveryStatus::Partial);
    assert_eq!(page.rows.len(), PAGE_SIZE);
    let (mut core, exact) = completion("((body 44");
    let page = provider.lookup(&exact).await.unwrap();
    assert_eq!(page.status, DeliveryStatus::Complete);
    assert_eq!(page.rows.len(), 1);
    let plan = admit(&mut core, &exact, &page, 0);
    server.responses.lock().unwrap().requests.clear();
    let Material::Inline(Child::HsLink(link)) = provider.resolve(&page.rows[0], &plan).await.unwrap() else { panic!("block reference"); };
    assert_eq!(link.target_block_id.as_deref(), Some("persisted-44"));
    assert_eq!(server.responses.lock().unwrap().requests, vec!["GET /knowledge/documents/source-note HTTP/1.1"]);
}

#[tokio::test]
async fn block_embeds_resolve_persisted_note_and_target_without_loom_or_whole_note_material() {
    for (query, id, heading) in [("![[Road#heading|Alias", "heading-id", true),
        ("!((body|Alias", "paragraph-id", false)] {
        let server = NoteServer::new().await;
        source(&server, blocks());
        server.responses.lock().unwrap().document["document"].as_object_mut().unwrap().remove("block_id");
        let provider = server.provider();
        let (mut core, trigger) = completion(query);
        let page = provider.lookup(&trigger).await.unwrap();
        assert_eq!(page.rows.len(), 1);
        let plan = admit(&mut core, &trigger, &page, 0);
        let Material::BlockEmbed { note_id, target, label } = provider.resolve(&page.rows[0], &plan).await.unwrap()
            else { panic!("Expected a block embed") };
        assert_eq!(note_id, "source-note");
        assert_eq!(target.block_id, id);
        assert_eq!(target.kind == crate::rich_editor::document_model::ReferenceTargetKind::Heading, heading);
        assert_eq!(label, "Alias");
        assert!(server.responses.lock().unwrap().requests.iter().all(|request| !request.contains("/loom/")));
    }
}

#[tokio::test]
async fn block_embed_resolution_fails_closed_after_source_changes_or_denial() {
    for change in ["missing", "forbidden", "version", "owner", "malformed"] {
        let server = NoteServer::new().await;
        source(&server, blocks());
        let provider = server.provider();
        let (mut core, trigger) = completion("![[Road#heading");
        let page = provider.lookup(&trigger).await.unwrap();
        let plan = admit(&mut core, &trigger, &page, 0);
        {
            let mut state = server.responses.lock().unwrap();
            match change {
                "missing" => state.load_status = 404,
                "forbidden" => state.load_status = 403,
                "version" => state.document["document"]["doc_version"] = json!(5),
                "owner" => state.document["document"]["rich_document_id"] = json!("other"),
                "malformed" => state.document["document"]["content_json"] = json!({"type":"unknown"}),
                _ => unreachable!(),
            }
        }
        assert!(provider.resolve(&page.rows[0], &plan).await.is_err(), "accepted {change}");
        assert!(server.responses.lock().unwrap().requests.iter().all(|request| !request.contains("/loom/")));
    }
}
