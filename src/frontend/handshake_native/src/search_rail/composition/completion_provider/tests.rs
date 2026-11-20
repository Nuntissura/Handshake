use super::*;
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct TagServer {
    base: String,
    body: Arc<Mutex<serde_json::Value>>,
    task: tokio::task::JoinHandle<()>,
}

impl TagServer {
    async fn new(rows: &[(&str, &str)]) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let body = Arc::new(Mutex::new(tag_rows(rows)));
        let current = body.clone();
        let task = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    let read = stream.read(&mut buffer).await.unwrap();
                    assert!(read > 0 && request.len() + read <= 16_384);
                    request.extend_from_slice(&buffer[..read]);
                }
                assert!(String::from_utf8(request).unwrap()
                    .starts_with("GET /workspaces/workspace/loom/tags?"));
                let body = current.lock().unwrap().to_string();
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                stream.write_all(response.as_bytes()).await.unwrap();
                stream.shutdown().await.unwrap();
            }
        });
        Self { base, body, task }
    }

    fn replace(&self, rows: &[(&str, &str)]) { *self.body.lock().unwrap() = tag_rows(rows); }

    fn provider(&self) -> Provider {
        Provider { workspace: "workspace".into(),
            account: crate::local_account::mock_account_context(&self.base),
            base: self.base.clone(), runtime: tokio::runtime::Handle::current() }
    }
}

impl Drop for TagServer {
    fn drop(&mut self) { self.task.abort(); }
}

fn tag_rows(rows: &[(&str, &str)]) -> serde_json::Value {
    serde_json::Value::Array(rows.iter().map(|(id, title)| serde_json::json!({
        "block_id": id, "title": title, "workspace_id": "workspace",
        "content_type": "tag_hub", "pinned": false, "favorite": false,
        "created_at": "2026-10-29T00:00:00Z", "updated_at": "2026-10-29T00:00:00Z",
        "derived": {}
    })).collect())
}

fn completion(query: &str) -> (WritingSession, NativeCompletionTrigger) {
    let mut core = WritingSession::with_limits(ContextId::new("mock-session").unwrap(),
        SessionId::new("provider-test").unwrap(), writing_limits());
    core.bind(EditorId::new("editor").unwrap(), 0).unwrap();
    core.expand().unwrap();
    let snapshot = NativeCompletionSnapshot::new(core.begin_lookup().unwrap(),
        CompletionSnapshotId::new("provider-snapshot").unwrap(), query.into(),
        NativeCompletionSelection::Caret).unwrap();
    let trigger = core.native_completion_trigger(&snapshot, CompletionInvocation::Manual)
        .unwrap().unwrap();
    (core, trigger)
}

fn admit(core: &mut WritingSession, trigger: &NativeCompletionTrigger, page: &Page, selected: usize)
    -> NativeCompletionPlan {
    core.deliver(trigger.snapshot().fence(), Channel::Suggestions, page.status,
        page.rows.iter().map(|row| row.item.clone()).collect()).unwrap();
    core.select(trigger.snapshot().fence(), page.rows[selected].item.key.clone()).unwrap();
    core.plan_native_completion(trigger, trigger.snapshot(), page.rows[selected].choice.clone()).unwrap()
}

#[tokio::test]
async fn provider_filter_preserves_maximum_encoded_title_through_delivery_and_resolution() {
    let title = "\u{0001}".repeat(MAX_COMPLETION_TAG_NAME_BYTES);
    let server = TagServer::new(&[("hub", &title)]).await;
    let provider = server.provider();
    let (mut core, trigger) = completion("tag:");
    let page = provider.lookup(&trigger).await.unwrap();
    assert_eq!(page.rows.len(), 1);
    let expected = format!("tag:\"{}\" ", "\\u0001".repeat(MAX_COMPLETION_TAG_NAME_BYTES));
    assert_eq!(expected.len(), MAX_COMPLETION_FILTER_BYTES);
    let row = &page.rows[0];
    assert_eq!(row.item.label, expected);
    assert_eq!(row.item.key.result.as_str().len(), 64);
    assert!(row.item.actions[0].id.as_str().len() <= MAX_ID_BYTES);
    assert_eq!(row.item.label.len() + row.item.detail.as_ref().unwrap().len(),
        writing_limits().max_item_bytes);
    let plan = admit(&mut core, &trigger, &page, 0);
    assert!(matches!(&plan.choice().value, NativeCompletionValue::Text { text, .. } if text == &expected));
    let Material::Inline(Child::Text(text)) = provider.resolve(row, &plan).await.unwrap() else {
        panic!("Expected the original filter payload");
    };
    assert_eq!(text.text.to_string(), expected);
}

#[tokio::test]
async fn provider_direct_tag_keeps_full_unicode_label_and_bounded_action_identity() {
    let title = format!("## café東京\"\\\n{} ", "界".repeat(1300));
    assert!(title.len() < MAX_COMPLETION_TAG_NAME_BYTES && title.len() > 100);
    let id = "i".repeat(MAX_ID_BYTES);
    let server = TagServer::new(&[(&id, &title)]).await;
    let provider = server.provider();
    let (mut core, trigger) = completion("#");
    let page = provider.lookup(&trigger).await.unwrap();
    let row = &page.rows[0];
    assert_eq!(row.item.label, format!("#{title}"));
    assert_eq!(row.item.key.result.as_str(), id);
    assert!(row.item.actions[0].id.as_str().len() < MAX_ID_BYTES);
    let plan = admit(&mut core, &trigger, &page, 0);
    let Material::Inline(Child::HsLink(link)) = provider.resolve(row, &plan).await.unwrap() else {
        panic!("Expected the selected tag reference");
    };
    assert_eq!(link, HsLinkNode::tag_hub_id(&id, &title));
}

#[tokio::test]
async fn provider_direct_tag_admits_full_4096_byte_title_with_descriptor_detail() {
    let title = "界".repeat(1365) + "#";
    assert_eq!(title.len(), MAX_COMPLETION_TAG_NAME_BYTES);
    let server = TagServer::new(&[("hub", &title)]).await;
    let provider = server.provider();
    let (mut core, trigger) = completion("#");
    let page = provider.lookup(&trigger).await.unwrap();
    assert!(page.rows[0].item.label.len() + page.rows[0].item.detail.as_ref().unwrap().len() > 4096);
    let plan = admit(&mut core, &trigger, &page, 0);
    let Material::Inline(Child::HsLink(link)) = provider.resolve(&page.rows[0], &plan).await.unwrap() else {
        panic!("Expected the full title");
    };
    assert_eq!(link.label, title);
}

#[tokio::test]
async fn provider_filter_identity_is_stable_across_order_and_collapses_identical_actions() {
    let title = format!("## shared \"{}\"", "é".repeat(300));
    let other = format!("other {}", "界".repeat(200));
    let server = TagServer::new(&[("hub-a", &title), ("hub-b", &other), ("hub-c", &title)]).await;
    let provider = server.provider();
    let (mut core, trigger) = completion("tag:");
    let before = provider.lookup(&trigger).await.unwrap();
    assert_eq!(before.rows.len(), 2);
    server.replace(&[("hub-c", &title), ("hub-b", &other), ("hub-a", &title)]);
    let after = provider.lookup(&trigger).await.unwrap();
    assert_eq!(before.rows.iter().map(|row| &row.item).collect::<Vec<_>>(),
        after.rows.iter().map(|row| &row.item).collect::<Vec<_>>());
    assert_ne!(after.rows[0].item.key, after.rows[1].item.key);
    let plan = admit(&mut core, &trigger, &after, 0);
    assert!(provider.resolve(&after.rows[1], &plan).await.is_err());
    assert!(provider.resolve(&after.rows[0], &plan).await.is_ok());
}

#[tokio::test]
async fn provider_tag_revalidation_rejects_same_title_with_different_hub() {
    let title = format!("## {}", "界".repeat(200));
    let server = TagServer::new(&[("selected", &title)]).await;
    let provider = server.provider();
    let (mut core, trigger) = completion("#");
    let page = provider.lookup(&trigger).await.unwrap();
    let plan = admit(&mut core, &trigger, &page, 0);
    server.replace(&[("different", &title)]);
    assert!(provider.resolve(&page.rows[0], &plan).await.is_err());
}

#[tokio::test]
async fn provider_keeps_title_and_descriptor_bounds_and_rejects_stale_plans() {
    let title = "x".repeat(MAX_COMPLETION_TAG_NAME_BYTES + 1);
    let server = TagServer::new(&[("hub", &title)]).await;
    let provider = server.provider();
    let (_, trigger) = completion("#");
    assert!(provider.lookup(&trigger).await.is_err());
    server.replace(&[("hub", "#full title")]);
    let (mut core, trigger) = completion("tag:");
    let page = provider.lookup(&trigger).await.unwrap();
    let mut oversized = page.rows[0].item.clone();
    oversized.label = "x".repeat(writing_limits().max_item_bytes + 1);
    assert_eq!(core.deliver(trigger.snapshot().fence(), Channel::Suggestions,
        DeliveryStatus::Complete, vec![oversized]), Err(Error::PayloadLimit));
    let plan = admit(&mut core, &trigger, &page, 0);
    core.begin_lookup().unwrap();
    assert!(plan.validate(&core, trigger.snapshot()).is_err());
}

struct NoteResponses {
    document: serde_json::Value,
    transclusion: serde_json::Value,
    load_status: u16,
    loom_status: u16,
    requests: Vec<String>,
}

struct NoteServer {
    base: String,
    responses: Arc<Mutex<NoteResponses>>,
    task: tokio::task::JoinHandle<()>,
}

impl NoteServer {
    async fn new() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let content = serde_json::json!({"type":"doc", "content":[{
            "type":"paragraph", "content":[{"type":"text", "text":"Source body"}]
        }]});
        let responses = Arc::new(Mutex::new(NoteResponses {
            document: serde_json::json!({
                "document": {"rich_document_id":"source-note", "block_id":"loom-note-projection",
                    "workspace_id":"workspace", "doc_version":4, "title":"Road",
                    "content_json":content, "authority_label":"AUTHORITATIVE",
                    "created_at":"created", "updated_at":"revision-four"},
                "tree":{"schema_version":"hsk_richdoc_v1", "schema_matches":true,
                    "block_ids":[], "blocks":[]}, "code_nodes":[]
            }),
            transclusion: serde_json::json!({"block_id":"loom-note-projection",
                "workspace_id":"workspace", "source_document_id":"source-note",
                "source_doc_version":4, "content_json":content, "resolved":true}),
            load_status: 200, loom_status: 200, requests: Vec::new(),
        }));
        let current = responses.clone();
        let task = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    let read = stream.read(&mut buffer).await.unwrap();
                    assert!(read > 0 && request.len() + read <= 16_384);
                    request.extend_from_slice(&buffer[..read]);
                }
                let request = String::from_utf8(request).unwrap();
                let lower = request.to_ascii_lowercase();
                assert!(lower.contains(&format!("x-hsk-session-token: {}\r\n", "a".repeat(64))));
                assert!(lower.contains(&format!("x-hsk-channel-binding-token: {}\r\n", "b".repeat(64))));
                let route = request.lines().next().unwrap().to_owned();
                let (status, body) = {
                    let mut state = current.lock().unwrap();
                    state.requests.push(route.clone());
                    match route.as_str() {
                        "GET /knowledge/documents?workspace_id=workspace HTTP/1.1" => (200, serde_json::json!([
                            {"rich_document_id":"source-note", "title":"Road", "updated_at":"revision-four"}
                        ])),
                        "GET /knowledge/documents/source-note HTTP/1.1" => (state.load_status, state.document.clone()),
                        "GET /workspaces/workspace/loom/blocks/loom-note-projection/transclusion HTTP/1.1" =>
                            (state.loom_status, state.transclusion.clone()),
                        _ => panic!("Unexpected completion request: {route}"),
                    }
                };
                let body = body.to_string();
                let response = format!("HTTP/1.1 {status} response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                stream.write_all(response.as_bytes()).await.unwrap();
                stream.shutdown().await.unwrap();
            }
        });
        Self { base, responses, task }
    }

    fn provider(&self) -> Provider {
        Provider { workspace: "workspace".into(),
            account: crate::local_account::mock_account_context(&self.base),
            base: self.base.clone(), runtime: tokio::runtime::Handle::current() }
    }
}

impl Drop for NoteServer {
    fn drop(&mut self) { self.task.abort(); }
}

#[tokio::test]
async fn whole_note_embed_resolves_explicit_loom_identity_through_authenticated_clients() {
    let server = NoteServer::new().await;
    let provider = server.provider();
    let (mut core, trigger) = completion("![[Road|alias");
    let page = provider.lookup(&trigger).await.unwrap();
    assert_eq!(page.rows.len(), 1);
    assert_eq!(page.rows[0].item.detail.as_deref(), Some("Embed note"));
    let plan = admit(&mut core, &trigger, &page, 0);
    assert!(matches!(&plan.choice().value, NativeCompletionValue::Reference { label, embed: true, .. } if label == "alias"));
    let Material::NoteEmbed { note_id, block_id } = provider.resolve(&page.rows[0], &plan).await.unwrap() else {
        panic!("Expected a resolved whole-note embed");
    };
    assert_eq!(note_id, "source-note");
    assert_eq!(block_id, "loom-note-projection");
    assert_ne!(block_id, note_id);
    assert_eq!(server.responses.lock().unwrap().requests, vec![
        "GET /knowledge/documents?workspace_id=workspace HTTP/1.1",
        "GET /knowledge/documents/source-note HTTP/1.1",
        "GET /workspaces/workspace/loom/blocks/loom-note-projection/transclusion HTTP/1.1",
    ]);
}

#[tokio::test]
async fn whole_note_embed_never_infers_an_absent_or_invalid_loom_identity() {
    for identity in [None, Some(serde_json::Value::Null), Some(serde_json::json!("")),
        Some(serde_json::json!(" ")), Some(serde_json::json!(7))] {
        let server = NoteServer::new().await;
        {
            let mut state = server.responses.lock().unwrap();
            let document = state.document["document"].as_object_mut().unwrap();
            document.remove("block_id");
            if let Some(identity) = identity { document.insert("block_id".into(), identity); }
        }
        let provider = server.provider();
        let (mut core, trigger) = completion("![[Road");
        let page = provider.lookup(&trigger).await.unwrap();
        let plan = admit(&mut core, &trigger, &page, 0);
        assert!(provider.resolve(&page.rows[0], &plan).await.is_err());
        assert_eq!(server.responses.lock().unwrap().requests.len(), 2);
    }
}

#[tokio::test]
async fn whole_note_embed_rejects_wrong_source_workspace_identity_version_and_unresolved_content() {
    for field in ["source_document_id", "workspace_id", "block_id", "source_doc_version", "resolved", "content_json"] {
        let server = NoteServer::new().await;
        server.responses.lock().unwrap().transclusion[field] = match field {
            "source_doc_version" => serde_json::json!(5),
            "resolved" => serde_json::json!(false),
            "content_json" => serde_json::Value::Null,
            _ => serde_json::json!("different"),
        };
        let provider = server.provider();
        let (mut core, trigger) = completion("![[Road");
        let page = provider.lookup(&trigger).await.unwrap();
        let plan = admit(&mut core, &trigger, &page, 0);
        assert!(provider.resolve(&page.rows[0], &plan).await.is_err(), "accepted changed {field}");
    }
    for field in ["rich_document_id", "workspace_id", "updated_at"] {
        let server = NoteServer::new().await;
        server.responses.lock().unwrap().document["document"][field] = serde_json::json!("different");
        let provider = server.provider();
        let (mut core, trigger) = completion("![[Road");
        let page = provider.lookup(&trigger).await.unwrap();
        let plan = admit(&mut core, &trigger, &page, 0);
        assert!(provider.resolve(&page.rows[0], &plan).await.is_err());
        assert_eq!(server.responses.lock().unwrap().requests.len(), 2);
    }
}

#[tokio::test]
async fn whole_note_embed_preserves_missing_denied_error_and_revoked_authorization() {
    for loom in [false, true] {
        for status in [403, 404, 500] {
            let server = NoteServer::new().await;
            {
                let mut state = server.responses.lock().unwrap();
                if loom { state.loom_status = status; } else { state.load_status = status; }
            }
            let provider = server.provider();
            let (mut core, trigger) = completion("![[Road");
            let page = provider.lookup(&trigger).await.unwrap();
            let plan = admit(&mut core, &trigger, &page, 0);
            assert!(provider.resolve(&page.rows[0], &plan).await.is_err());
            assert_eq!(server.responses.lock().unwrap().requests.len(), if loom { 3 } else { 2 });
        }
    }
    let server = NoteServer::new().await;
    let provider = server.provider();
    let (mut core, trigger) = completion("![[Road");
    let page = provider.lookup(&trigger).await.unwrap();
    let plan = admit(&mut core, &trigger, &page, 0);
    provider.account.invalidate();
    assert!(provider.resolve(&page.rows[0], &plan).await.is_err());
    assert_eq!(server.responses.lock().unwrap().requests.len(), 1);
}

#[tokio::test]
async fn missing_loom_identity_does_not_change_note_link_or_template_resolution() {
    let server = NoteServer::new().await;
    server.responses.lock().unwrap().document["document"].as_object_mut().unwrap().remove("block_id");
    let provider = server.provider();
    for text in ["[[Road|alias", "/Road"] {
        let (mut core, trigger) = completion(text);
        let page = provider.lookup(&trigger).await.unwrap();
        let plan = admit(&mut core, &trigger, &page, 0);
        match provider.resolve(&page.rows[0], &plan).await.unwrap() {
            Material::Inline(Child::HsLink(link)) if text.starts_with("[[") =>
                assert_eq!(link, HsLinkNode::new("note", "source-note", "alias")),
            Material::Template(document) if text.starts_with('/') =>
                assert_eq!(crate::rich_editor::save::export::export_plain_text(&document), "Source body"),
            _ => panic!("Changed ordinary note link or template behavior"),
        }
    }
    assert!(server.responses.lock().unwrap().requests.iter().all(|request| !request.contains("/loom/")));
}

mod block_references;
