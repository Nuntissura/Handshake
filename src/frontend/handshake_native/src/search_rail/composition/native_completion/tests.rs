use super::*;
use crate::rich_editor::document_model::{BlockNode, DocPosition, Selection};

fn editor(text: &str) -> Arc<Mutex<RichEditorState>> {
    let mut state = RichEditorState::new(BlockNode::doc(vec![BlockNode::paragraph(text)]));
    state.selection = Selection::caret(DocPosition::new(vec![0, 0], text.chars().count()));
    state.end_handpick_template();
    Arc::new(Mutex::new(state))
}

fn captured(editor: &Arc<Mutex<RichEditorState>>)
    -> (handpick::WritingSession, NotesCompletionSnapshot) {
    let state = editor.lock().unwrap();
    let mut core = handpick::WritingSession::with_limits(handpick::ContextId::new("context").unwrap(),
        handpick::SessionId::new("template-session").unwrap(), super::super::completion_provider::writing_limits());
    core.bind(handpick::EditorId::new("editor").unwrap(),
        state.document_owner().current_context().unwrap().edit_revision).unwrap();
    let snapshot = state.capture_handpick_completion(core.fence().unwrap(),
        handpick::CompletionSnapshotId::new("template-snapshot").unwrap()).unwrap();
    (core, snapshot)
}

#[test]
fn closing_template_popup_restores_the_existing_slash_catalog() {
    let editor = editor("/daily");
    let (core, snapshot) = captured(&editor);
    let before = editor.lock().unwrap().document_snapshot().unwrap();
    let mut completion = NativeCompletion::default();
    completion.lease = Some(TemplatePopupLease::acquire(&editor, &snapshot, core.fence().unwrap()).unwrap());
    assert!(editor.lock().unwrap().slash_menu.is_none());
    completion.close();
    let state = editor.lock().unwrap();
    assert!(!state.handpick_template_open);
    let menu = state.slash_menu.as_ref().unwrap();
    assert_eq!(menu.filter, "daily");
    assert!(crate::rich_editor::slash_commands::registry::filter_slash_commands(&menu.filter)
        .iter().any(|command| command.id == "template-daily-note"));
    assert_eq!(state.document_snapshot().unwrap(), before);
    assert!(state.history_snapshot().unwrap().is_empty());
}

#[test]
fn rejected_template_delivery_returns_keys_to_legacy_slash_commands() {
    let editor = editor("/meeting");
    let (core, snapshot) = captured(&editor);
    let mut notes = MountedNotes::default();
    notes.completion.lease = Some(TemplatePopupLease::acquire(&editor, &snapshot, core.fence().unwrap()).unwrap());
    notes.fail_completion("The selected note changed; invoke completion again".into());
    assert_eq!(notes.error.as_deref(), Some("The selected note changed; invoke completion again"));
    assert!(!notes.completion.is_open());
    let mut state = editor.lock().unwrap();
    assert!(!state.handpick_template_open);
    let menu = state.slash_menu.as_mut().unwrap();
    assert_eq!(menu.filter, "meeting");
    let ctx = egui::Context::default();
    let mut outcome = None;
    let _ = ctx.run(egui::RawInput { events: vec![egui::Event::Key {
        key: egui::Key::Enter, physical_key: None, pressed: true, repeat: false,
        modifiers: egui::Modifiers::NONE }], ..Default::default() }, |ctx| {
        outcome = Some(crate::rich_editor::slash_commands::menu::handle_menu_keys(ctx, menu));
    });
    assert!(matches!(outcome, Some(crate::rich_editor::slash_commands::menu::SlashMenuOutcome::Execute(_))));
    assert!(state.history_snapshot().unwrap().is_empty());
}

#[test]
fn stale_template_context_closes_without_restoring_the_old_document_menu() {
    let editor = editor("/daily");
    let (core, snapshot) = captured(&editor);
    let mut notes = MountedNotes::default();
    notes.completion.lease = Some(TemplatePopupLease::acquire(&editor, &snapshot, core.fence().unwrap()).unwrap());
    notes.completion.snapshot = Some(snapshot);
    {
        let mut state = editor.lock().unwrap();
        state.open_document_owner(BlockNode::doc(vec![BlockNode::paragraph("/meeting")]));
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 8));
    }
    let stale = notes.completion.snapshot.as_ref().unwrap()
        .current(&editor.lock().unwrap(), core.fence().unwrap()).unwrap_err();
    notes.fail_completion(stale);
    let state = editor.lock().unwrap();
    assert!(!state.handpick_template_open);
    assert_eq!(state.slash_menu.as_ref().unwrap().filter, "meeting");
    assert!(state.history_snapshot().unwrap().is_empty());
    assert!(state.pending_bus_undo.is_empty());
    assert!(!notes.completion.is_open());
}

#[test]
fn explicit_slash_menu_and_ime_cannot_be_taken_over_by_template_completion() {
    let editor = editor("/daily");
    let (core, snapshot) = captured(&editor);
    editor.lock().unwrap().slash_menu_opened_explicitly = true;
    assert!(TemplatePopupLease::acquire(&editor, &snapshot, core.fence().unwrap()).is_err());
    {
        let mut state = editor.lock().unwrap();
        assert_eq!(state.slash_menu.as_ref().unwrap().filter, "daily");
        state.slash_menu_opened_explicitly = false;
        state.preedit.anchor = Some(DocPosition::new(vec![0, 0], 6));
    }
    assert!(TemplatePopupLease::acquire(&editor, &snapshot, core.fence().unwrap()).is_err());
    assert!(!editor.lock().unwrap().handpick_template_open);
}

#[test]
fn revoked_owner_template_popup_close_releases_input_without_a_slash_menu() {
    let editor = editor("/daily");
    let endpoint = "http://127.0.0.1:9911";
    let authentication = crate::local_account::mock_account_context(endpoint);
    editor.lock().unwrap().document_owner().bind_context("workspace", "note", endpoint,
        None, Some(authentication.clone())).unwrap();
    let (core, snapshot) = captured(&editor);
    let mut completion = NativeCompletion::default();
    completion.lease = Some(TemplatePopupLease::acquire(&editor, &snapshot, core.fence().unwrap()).unwrap());
    assert!(editor.lock().unwrap().handpick_template_open);
    authentication.invalidate();
    assert!(editor.lock().unwrap().document_snapshot().is_err());
    completion.close();
    let state = editor.lock().unwrap();
    assert!(!state.handpick_template_open);
    assert!(state.slash_menu.is_none());
    assert!(state.pending_bus_undo.is_empty());
    assert!(!completion.is_open());
}

fn completion_ready_to_apply(runtime: &tokio::runtime::Runtime) -> MountedNotes {
    use crate::rich_editor::save::save_manager::{ReqwestSaveBackend, SaveManager};

    let editor = editor("/Road");
    let base = "http://127.0.0.1:9911";
    let account = crate::local_account::mock_account_context(base);
    {
        let mut state = editor.lock().unwrap();
        state.document_owner().bind_context("workspace", "note", base, None,
            Some(account.clone())).unwrap();
        state.save = Some(SaveManager::new(Arc::new(ReqwestSaveBackend::new(base)),
            None, "note", 0));
    }
    let (mut core, snapshot) = captured(&editor);
    let trigger = core.native_completion_trigger(snapshot.native(), CompletionInvocation::Manual)
        .unwrap().unwrap();
    let key = handpick::ResultKey { provider: handpick::ProviderId::new("native.notes").unwrap(),
        result: handpick::ResultId::new("source-note").unwrap() };
    core.deliver(&core.fence().unwrap(), handpick::Channel::Suggestions,
        handpick::DeliveryStatus::Complete, vec![handpick::SearchItem { key: key.clone(),
            label: "Road".into(), detail: None, actions: Vec::new() }]).unwrap();
    core.select(&core.fence().unwrap(), key.clone()).unwrap();
    let plan = core.plan_native_completion(&trigger, snapshot.native(), handpick::NativeCompletionChoice {
        key, value: handpick::NativeCompletionValue::Template,
    }).unwrap();
    let lease = TemplatePopupLease::acquire(&editor, &snapshot, core.fence().unwrap()).unwrap();
    let mut notes = MountedNotes::default();
    notes.editors.insert(core.editor().unwrap().0.clone(), NoteHandle {
        document_id: "note".into(), view_id: "view".into(), editor: editor.clone(), authority: editor,
    });
    notes.core = Some(core);
    notes.completion.lease = Some(lease);
    notes.completion.snapshot = Some(snapshot);
    notes.completion.trigger = Some(trigger);
    notes.completion.provider = Some(Provider { workspace: "workspace".into(), account,
        base: base.into(), runtime: runtime.handle().clone() });
    notes.completion.plan = Some(plan);
    *notes.completion.cell.lock().unwrap() = Some(Delivery::Resolved(Ok(Material::Template(BlockNode::doc(vec![
        BlockNode::paragraph("inserted template"),
    ])))));
    notes.observations = true;
    notes
}

fn assert_applied_template_diagnostic(revoke_after_apply: bool) {
    use crate::diagnostics::{self, gui_events::{GuiDiagnosticOperation, GuiDiagnosticPhase}};
    use handshake_diag_ring::{DiagEventCode, DiagPhase, DiagRingReader, DiagRingWriter,
        DiagSeverity, DEFAULT_CAPACITY};

    // Each exact selector needs a fresh process, as the recorder cannot be replaced after install.
    let root = std::path::PathBuf::from(std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT")
        .expect("HANDSHAKE_ARTIFACTS_ROOT must name the admitted artifact root"));
    assert!(root.is_absolute() && root.is_dir());
    let directory = root.join("WP-KERNEL-012/MT-212/template-diagnostics-tests")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&directory).unwrap();
    let ring = directory.join("completion.ring");
    assert!(diagnostics::install(DiagRingWriter::create(&ring, DEFAULT_CAPACITY).unwrap()),
        "run this exact selector in a fresh test process");
    let reader = DiagRingReader::open(&ring).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let mut notes = completion_ready_to_apply(&runtime);
    let editor = notes.active().unwrap().editor;
    let owner = editor.lock().unwrap().document_owner();
    let before_context = owner.current_context().unwrap();
    let before_document = owner.document_snapshot().unwrap();
    if revoke_after_apply {
        notes.completion.after_apply = Some(|notes| notes.completion.provider.as_ref().unwrap().account.invalidate());
    }

    notes.drain_completion(&egui::Context::default());

    if revoke_after_apply {
        assert!(owner.current_context().is_err());
        let expected = crate::rich_editor::document_owner::DocumentOwnerError::Unavailable.to_string();
        assert_eq!(notes.error.as_deref(), Some(expected.as_str()));
        let base = "http://127.0.0.1:9911";
        owner.bind_context("workspace", "note", base, None,
            Some(crate::local_account::mock_account_context(base))).unwrap();
    }
    let applied = owner.document_snapshot().unwrap();
    assert_ne!(applied, before_document);
    assert!(crate::rich_editor::save::export::export_plain_text(&applied).contains("inserted template"));
    assert_eq!(owner.current_context().unwrap().edit_revision, before_context.edit_revision + 1);
    assert!(owner.with_save_gate(|gate| gate.is_dirty()));
    {
        let state = editor.lock().unwrap();
        assert_eq!(state.history_snapshot().unwrap().len(), 1);
        assert_eq!(state.pending_bus_undo.len(), 1);
        assert!(!state.handpick_template_open);
    }
    assert!(!notes.completion.is_open());
    let expected = if revoke_after_apply {
        let expected = crate::rich_editor::document_owner::DocumentOwnerError::Unavailable.to_string();
        assert_eq!(notes.error.as_deref(), Some(expected.as_str()));
        GuiDiagnosticPhase::Failed
    } else {
        assert!(notes.error.is_none());
        assert_eq!(notes.core.as_ref().unwrap().fence().unwrap().revision,
            before_context.edit_revision + 1);
        GuiDiagnosticPhase::Completed
    };
    let events: Vec<_> = reader.read_last_n(DEFAULT_CAPACITY).into_iter()
        .filter(|event| event.event_code == DiagEventCode::Other.as_u16()
            && event.counter_a == GuiDiagnosticOperation::HandpickWriting as u64).collect();
    assert_eq!(events.len(), 1, "the actual drain path emits exactly one terminal observation");
    assert_eq!(events[0].counter_b, expected as u64);
    assert_eq!(events[0].phase_marker, (if revoke_after_apply { DiagPhase::Degraded } else { DiagPhase::End }).as_u8());
    assert_eq!(events[0].severity, (if revoke_after_apply { DiagSeverity::Warn } else { DiagSeverity::Info }).as_u8());
    let buffered: Vec<_> = diagnostics::snapshot_last_n(diagnostics::BUFFER_CAP).into_iter()
        .filter(|event| event.event_code == DiagEventCode::Other.as_u16()
            && event.counter_a == GuiDiagnosticOperation::HandpickWriting as u64).collect();
    assert_eq!(buffered, events);

    notes.drain_completion(&egui::Context::default());
    assert_eq!(owner.current_context().unwrap().edit_revision, before_context.edit_revision + 1);
    assert_eq!(reader.read_last_n(DEFAULT_CAPACITY).into_iter()
        .filter(|event| event.event_code == DiagEventCode::Other.as_u16()
            && event.counter_a == GuiDiagnosticOperation::HandpickWriting as u64).count(), 1);
}

#[test]
fn applied_template_sync_failure_preserves_one_edit_and_emits_only_failed() {
    assert_applied_template_diagnostic(true);
}

#[test]
fn applied_template_sync_success_preserves_one_edit_and_emits_completed() {
    assert_applied_template_diagnostic(false);
}


fn input(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput { focused: true, screen_rect: Some(egui::Rect::from_min_size(
        egui::Pos2::ZERO, egui::vec2(600.0, 500.0))), events, ..Default::default() }
}

fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

fn live_completion(text: &str, ctx: &egui::Context) -> (MountedNotes, Arc<crate::local_account::AuthenticatedContext>) {
    use crate::rich_editor::save::save_manager::{ReqwestSaveBackend, SaveManager};
    let editor = editor(text);
    let base = "http://127.0.0.1:9911";
    let account = crate::local_account::mock_account_context(base);
    {
        let mut state = editor.lock().unwrap();
        state.document_owner().bind_context("workspace", "note", base, None, Some(account.clone())).unwrap();
        state.save = Some(SaveManager::new(Arc::new(ReqwestSaveBackend::new(base)), None, "note", 0));
        state.request_editor_focus();
    }
    let (mut core, _) = captured(&editor);
    core.expand().unwrap();
    let editor_id = core.editor().unwrap().0.clone();
    let mut notes = MountedNotes::default();
    notes.core = Some(core);
    notes.editors.insert(editor_id, NoteHandle { document_id: "note".into(), view_id: "view".into(),
        editor: editor.clone(), authority: editor });
    notes.bus = Some(crate::interop::InteractionBus::get_or_init(ctx));
    let _ = ctx.run(input(Vec::new()), |ctx| draw_editor(&mut notes, ctx));
    assert!(notes.focused_binding().is_some());
    (notes, account)
}

fn draw_editor(notes: &mut MountedNotes, ctx: &egui::Context) {
    let handle = notes.active().unwrap();
    let factory = crate::rich_editor::renderer::rich_editor_widget::RichEditorPaneFactory::new(handle.editor);
    egui::CentralPanel::default().show(ctx, |ui| {
        let response = factory.render_for_view(ui, Arc::from("view"));
        notes.surface = Some(MountedSurface { editor: notes.core.as_ref().unwrap().editor().unwrap().0.clone(),
            view_id: "view".into(), document_id: "note".into(), response });
        notes.render_completion(ui);
    });
}

fn completion_frame(notes: &mut MountedNotes, ctx: &egui::Context,
    account: &Arc<crate::local_account::AuthenticatedContext>, runtime: &tokio::runtime::Runtime,
    events: Vec<egui::Event>) {
    let _ = ctx.run(input(events), |ctx| {
        notes.completion_before_input(ctx, "workspace", Some(account), "http://127.0.0.1:9911",
            Some(runtime.handle()), false);
        draw_editor(notes, ctx);
        notes.synchronize().unwrap();
        notes.completion_after_input(ctx, "workspace", Some(account), "http://127.0.0.1:9911",
            Some(runtime.handle()), false);
    });
}

#[test]
fn ctrl_space_reaches_supported_provider_modes_without_global_takeover() {
    use handpick::NativeCompletionLookup::*;
    for (text, expected) in [
        ("type:no", QueryFilter { prefix: "type:no".into() }),
        ("tag:ru", QueryFilter { prefix: "tag:ru".into() }),
        ("#rust", Tags { name: "rust".into() }),
        ("[[Road", Notes { title: "road".into() }),
        ("![[Road", Notes { title: "road".into() }),
        ("[[Road#heading", Blocks { note_title: Some("road".into()), text: "heading".into(), headings_only: true }),
        ("((block", Blocks { note_title: None, text: "block".into(), headings_only: false }),
        ("/Road^block", Templates { source: handpick::NativeTemplateSource::Blocks { note_title: "road".into(), text: "block".into(), headings_only: false } }),
        ("Road", Notes { title: "road".into() }),
        ("/Road", Templates { source: handpick::NativeTemplateSource::Note { title: "road".into() } }),
    ] {
        let ctx = egui::Context::default();
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let (mut notes, account) = live_completion(text, &ctx);
        let editor = notes.active().unwrap().editor;
        let before = editor.lock().unwrap().document_snapshot().unwrap();
        assert!(!editor.lock().unwrap().handpick_inline_open);
        completion_frame(&mut notes, &ctx, &account, &runtime, vec![key(egui::Key::Space, egui::Modifiers::CTRL)]);
        assert_eq!(notes.completion.trigger.as_ref().unwrap().lookup(), expected);
        let template = matches!(expected, Templates { .. });
        assert_eq!(notes.completion.lease.is_some(), template);
        assert_eq!(notes.completion.inline_lease.is_some(), !template);
        assert_eq!(editor.lock().unwrap().document_snapshot().unwrap(), before);
        assert!(editor.lock().unwrap().history_snapshot().unwrap().is_empty());
        completion_frame(&mut notes, &ctx, &account, &runtime, vec![key(egui::Key::Escape, egui::Modifiers::NONE)]);
        assert!(!notes.completion.is_open());
        let state = editor.lock().unwrap();
        assert!(!state.handpick_inline_open && !state.handpick_template_open);
        assert_eq!(state.document_snapshot().unwrap(), before, "native Escape must not reach legacy trigger deletion");
        if text.starts_with("[[") { assert!(state.wikilink_autocomplete.is_some()); }
        if text.starts_with('#') { assert!(state.tag_autocomplete.is_some()); }
        if text.starts_with('/') { assert!(state.slash_menu.is_some()); }
    }
}

#[test]
fn manual_selected_text_reaches_notes_and_refresh_retains_inline_lease() {
    let ctx = egui::Context::default();
    let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let (mut notes, account) = live_completion("Road", &ctx);
    let editor = notes.active().unwrap().editor;
    editor.lock().unwrap().selection = Selection::text(DocPosition::new(vec![0, 0], 0), DocPosition::new(vec![0, 0], 4));
    completion_frame(&mut notes, &ctx, &account, &runtime, vec![key(egui::Key::Space, egui::Modifiers::CTRL)]);
    assert_eq!(notes.completion.trigger.as_ref().unwrap().replacement_range(), &handpick::NativeCompletionRange::Selection);
    completion_frame(&mut notes, &ctx, &account, &runtime, vec![egui::Event::Text("New".into())]);
    assert_eq!(notes.completion.trigger.as_ref().unwrap().lookup(), handpick::NativeCompletionLookup::Notes { title: "new".into() });
    assert!(notes.completion.inline_lease.is_some());
    assert!(editor.lock().unwrap().handpick_inline_open);
}

#[test]
fn block_embed_ctrl_space_admits_lookup_and_escape_preserves_the_source() {
    for text in ["!((block", "![[Road#heading"] {
        let ctx = egui::Context::default();
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let (mut notes, account) = live_completion(text, &ctx);
        let editor = notes.active().unwrap().editor;
        let before = editor.lock().unwrap().document_snapshot().unwrap();
        completion_frame(&mut notes, &ctx, &account, &runtime, vec![key(egui::Key::Space, egui::Modifiers::CTRL)]);
        assert!(notes.error.is_none());
        let trigger = notes.completion.trigger.as_ref().unwrap();
        assert!(trigger.embed());
        assert!(matches!(trigger.lookup(), handpick::NativeCompletionLookup::Blocks { .. }));
        assert!(notes.completion.task.is_some());
        assert!(notes.completion.inline_lease.is_some());
        assert!(notes.completion.lease.is_none());
        assert_eq!(editor.lock().unwrap().document_snapshot().unwrap(), before);
        completion_frame(&mut notes, &ctx, &account, &runtime, vec![key(egui::Key::Escape, egui::Modifiers::NONE)]);
        assert!(!notes.completion.is_open());
        assert!(notes.completion.task.is_none());
        let state = editor.lock().unwrap();
        assert!(!state.handpick_inline_open && !state.handpick_template_open);
        assert_eq!(state.document_snapshot().unwrap(), before);
        assert!(state.history_snapshot().unwrap().is_empty());
        assert!(state.pending_bus_undo.is_empty());
    }
}

#[test]
fn block_embed_driver_reaches_lookup_selection_resolution_and_one_undoable_commit() {
    use crate::rich_editor::block_identity::references::Target as BlockTarget;
    use crate::rich_editor::document_model::{NodeKind, ReferenceTargetKind};
    for (text, kind) in [("!((block|Alias", ReferenceTargetKind::Block),
        ("![[Road#heading|Alias", ReferenceTargetKind::Heading)] {
        let ctx = egui::Context::default();
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let (mut notes, account) = live_completion(text, &ctx);
        let editor = notes.active().unwrap().editor;
        let before = editor.lock().unwrap().document_snapshot().unwrap();
        completion_frame(&mut notes, &ctx, &account, &runtime, vec![key(egui::Key::Space, egui::Modifiers::CTRL)]);
        assert!(notes.error.is_none());
        notes.completion.task.take().unwrap().abort();
        let target = BlockTarget { kind, block_id: "persisted-target".into() };
        let note = handpick::NoteId::new("owning-note").unwrap();
        let block = handpick::BlockId::new(&target.block_id).unwrap();
        let address = if kind == ReferenceTargetKind::Heading {
            handpick::NoteAddress::Heading { note, block }
        } else { handpick::NoteAddress::Block { note, block } };
        let result_key = handpick::ResultKey { provider: handpick::ProviderId::new("native.notes").unwrap(),
            result: handpick::ResultId::new("exact-target").unwrap() };
        let row = Row {
            item: handpick::SearchItem { key: result_key.clone(), label: "Target".into(), detail: None, actions: Vec::new() },
            choice: handpick::NativeCompletionChoice { key: result_key, value: handpick::NativeCompletionValue::Reference {
                target: address, label: "Target".into(), embed: true,
            } },
            target: super::super::completion_provider::Target::Block {
                note_id: "owning-note".into(), updated_at: "version-time".into(), doc_version: 7,
                target: target.clone(), template: false,
            },
        };
        *notes.completion.cell.lock().unwrap() = Some(Delivery::Lookup(Ok(Page {
            rows: vec![row], status: handpick::DeliveryStatus::Complete,
        })));
        completion_frame(&mut notes, &ctx, &account, &runtime, Vec::new());
        assert_eq!(notes.completion.rows.len(), 1);
        completion_frame(&mut notes, &ctx, &account, &runtime, vec![key(egui::Key::Enter, egui::Modifiers::NONE)]);
        assert!(notes.completion.plan.is_some());
        notes.completion.task.take().unwrap().abort();
        *notes.completion.cell.lock().unwrap() = Some(Delivery::Resolved(Ok(Material::BlockEmbed {
            note_id: "owning-note".into(), target, label: "Alias".into(),
        })));
        notes.drain_completion(&ctx);
        notes.drain_completion(&ctx);
        assert!(notes.error.is_none());
        assert!(!notes.completion.is_open());
        let mut state = editor.lock().unwrap();
        let after = state.document_snapshot().unwrap();
        let embed = after.children.iter().filter_map(|child| child.as_block())
            .find(|block| block.kind == NodeKind::HsEmbed).unwrap();
        assert_eq!(embed.attrs["refValue"], serde_json::json!("owning-note"));
        assert_eq!(embed.attrs["target_kind"], serde_json::json!(kind.as_str()));
        assert_eq!(embed.attrs["target_block_id"], serde_json::json!("persisted-target"));
        assert_ne!(embed.attrs["block_id"], embed.attrs["target_block_id"]);
        assert_eq!(embed.attrs["label"], serde_json::json!("Alias"));
        assert!(!state.handpick_inline_open && !state.handpick_template_open);
        assert_eq!(state.history_snapshot().unwrap().len(), 1);
        assert_eq!(state.pending_bus_undo.len(), 1);
        let wire = crate::rich_editor::document_model::to_content_json_value(&after);
        assert_eq!(crate::rich_editor::document_model::from_json_value(&wire).unwrap(), after);
        state.edit_document(|document, history| history.undo(document)).unwrap();
        assert_eq!(state.document_snapshot().unwrap(), before);
    }
}

#[test]
fn stale_tag_resolution_preserves_the_newer_notes_draft() {
    use super::super::completion_provider::Target;
    use crate::rich_editor::document_model::node::{Child, HsLinkNode};

    let ctx = egui::Context::default();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (mut notes, account) = live_completion("#orbit", &ctx);
    let editor = notes.active().unwrap().editor;

    completion_frame(
        &mut notes,
        &ctx,
        &account,
        &runtime,
        vec![key(egui::Key::Space, egui::Modifiers::CTRL)],
    );
    notes.completion.task.take().unwrap().abort();
    let hub_id = "orbit-hub-id";
    let title = "Orbitgarden Café / 模型";
    let result = handpick::ResultKey {
        provider: handpick::ProviderId::new("native.loom.tags").unwrap(),
        result: handpick::ResultId::new(hub_id).unwrap(),
    };
    let action = handpick::Action {
        id: handpick::ActionId::new("apply").unwrap(),
        kind: handpick::ActionKind::ApplyCompletion,
    };
    *notes.completion.cell.lock().unwrap() = Some(Delivery::Lookup(Ok(Page {
        rows: vec![Row {
            item: handpick::SearchItem {
                key: result.clone(),
                label: format!("#{title}"),
                detail: None,
                actions: vec![action.clone()],
            },
            choice: handpick::NativeCompletionChoice {
                key: result.clone(),
                value: handpick::NativeCompletionValue::Tag {
                    action,
                    label: title.into(),
                },
            },
            target: Target::Tag {
                id: hub_id.into(),
                name: title.into(),
            },
        }],
        status: handpick::DeliveryStatus::Complete,
    })));
    completion_frame(&mut notes, &ctx, &account, &runtime, Vec::new());
    completion_frame(
        &mut notes,
        &ctx,
        &account,
        &runtime,
        vec![key(egui::Key::Tab, egui::Modifiers::NONE)],
    );
    assert_eq!(notes.completion.plan.as_ref().unwrap().choice().key, result);
    notes.completion.task.take().unwrap().abort();

    let (draft, revision, undo_len) = {
        let mut state = editor.lock().unwrap();
        let previous_revision = state
            .document_owner()
            .current_context()
            .unwrap()
            .edit_revision;
        assert!(state.insert_plain_text_for_host(" revised"));
        let revision = state
            .document_owner()
            .current_context()
            .unwrap()
            .edit_revision;
        assert!(revision > previous_revision);
        assert_eq!(state.block_plain_text(0).as_deref(), Some("#orbit revised"));
        (
            state.document_snapshot().unwrap(),
            revision,
            state.history_snapshot().unwrap().len(),
        )
    };
    *notes.completion.cell.lock().unwrap() = Some(Delivery::Resolved(Ok(Material::Inline(
        Child::HsLink(HsLinkNode::tag_hub_id(hub_id, title)),
    ))));
    notes.drain_completion(&ctx);

    assert!(notes.error.is_none());
    assert!(!notes.completion.is_open());
    let state = editor.lock().unwrap();
    assert!(!state.handpick_inline_open);
    assert_eq!(state.document_snapshot().unwrap(), draft);
    assert_eq!(
        state
            .document_owner()
            .current_context()
            .unwrap()
            .edit_revision,
        revision
    );
    assert_eq!(state.history_snapshot().unwrap().len(), undo_len);
}

#[test]
fn inline_driver_applies_only_resolved_material_once_with_one_undo() {
    use crate::rich_editor::document_model::node::{Child, HsLinkNode, TextLeaf};
    for text in ["type:no", "#rust", "[[Road|alias"] {
        let ctx = egui::Context::default();
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let (mut notes, account) = live_completion(text, &ctx);
        let editor = notes.active().unwrap().editor;
        let before = editor.lock().unwrap().document_snapshot().unwrap();
        completion_frame(&mut notes, &ctx, &account, &runtime, vec![key(egui::Key::Space, egui::Modifiers::CTRL)]);
        if let Some(task) = notes.completion.task.take() { task.abort(); }
        let (provider, result) = if text.starts_with("type:") { ("native.query", "filter") }
            else if text.starts_with('#') { ("native.loom.tags", "real-tag-hub") }
            else { ("native.notes", "source-note") };
        let key = handpick::ResultKey { provider: handpick::ProviderId::new(provider).unwrap(), result: handpick::ResultId::new(result).unwrap() };
        let action = handpick::Action { id: handpick::ActionId::new("apply").unwrap(), kind: handpick::ActionKind::ApplyCompletion };
        let (value, material, actions) = if text.starts_with("type:") {
            (handpick::NativeCompletionValue::Text { action: action.clone(), text: "type:note ".into() },
                Child::Text(TextLeaf::new("type:note ")), vec![action])
        } else if text.starts_with('#') {
            (handpick::NativeCompletionValue::Tag { action: action.clone(), label: "rust".into() },
                Child::HsLink(HsLinkNode::tag_hub_id(result, "rust")), vec![action])
        } else {
            (handpick::NativeCompletionValue::Reference { target: handpick::NoteAddress::Note { note: handpick::NoteId::new(result).unwrap() },
                label: "Road".into(), embed: false }, Child::HsLink(HsLinkNode::new("note", result, "alias")), Vec::new())
        };
        let core = notes.core.as_mut().unwrap();
        core.deliver(&core.fence().unwrap(), handpick::Channel::Suggestions, handpick::DeliveryStatus::Complete,
            vec![handpick::SearchItem { key: key.clone(), label: "result".into(), detail: None, actions }]).unwrap();
        core.select(&core.fence().unwrap(), key.clone()).unwrap();
        let plan = core.plan_native_completion(notes.completion.trigger.as_ref().unwrap(),
            notes.completion.snapshot.as_ref().unwrap().native(), handpick::NativeCompletionChoice { key, value }).unwrap();
        notes.completion.plan = Some(plan);
        *notes.completion.cell.lock().unwrap() = Some(Delivery::Resolved(Ok(Material::Inline(material.clone()))));
        notes.drain_completion(&ctx);
        notes.drain_completion(&ctx);
        assert!(notes.error.is_none());
        assert!(!notes.completion.is_open());
        let mut state = editor.lock().unwrap();
        let edited = state.document_snapshot().unwrap();
        assert!(edited.children[0].as_block().unwrap().children.contains(&material));
        assert_eq!(state.history_snapshot().unwrap().len(), 1);
        assert_eq!(state.pending_bus_undo.len(), 1);
        assert!(!state.handpick_inline_open);
        state.edit_document(|document, history| history.undo(document)).unwrap();
        assert_eq!(state.document_snapshot().unwrap(), before);
    }
}

#[test]
fn revoked_inline_lease_clears_parked_popups_without_mutating_text() {
    let ctx = egui::Context::default();
    let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let (mut notes, account) = live_completion("[[Road", &ctx);
    let editor = notes.active().unwrap().editor;
    completion_frame(&mut notes, &ctx, &account, &runtime, vec![key(egui::Key::Space, egui::Modifiers::CTRL)]);
    account.invalidate();
    notes.completion.close();
    let state = editor.lock().unwrap();
    assert!(!state.handpick_inline_open);
    assert!(state.wikilink_autocomplete.is_none() && state.tag_autocomplete.is_none() && state.slash_menu.is_none());
    assert!(state.pending_bus_undo.is_empty());
}


fn filter_rows() -> Vec<Row> {
    ["type:note ", "type:block "].into_iter().enumerate().map(|(index, text)| {
        let key = handpick::ResultKey { provider: handpick::ProviderId::new("native.query").unwrap(),
            result: handpick::ResultId::new(format!("filter-{index}")).unwrap() };
        let action = handpick::Action { id: handpick::ActionId::new(format!("apply-{index}")).unwrap(),
            kind: handpick::ActionKind::ApplyCompletion };
        Row { item: handpick::SearchItem { key: key.clone(), label: text.into(), detail: None, actions: vec![action.clone()] },
            choice: handpick::NativeCompletionChoice { key, value: handpick::NativeCompletionValue::Text { action, text: text.into() } },
            target: super::super::completion_provider::Target::Filter(text.into()) }
    }).collect()
}

#[test]
fn popup_arrows_and_tab_keep_editor_focus_with_actual_result_and_close_controls() {
    let ctx = egui::Context::default();
    let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let (mut notes, account) = live_completion("type:", &ctx);
    completion_frame(&mut notes, &ctx, &account, &runtime, vec![key(egui::Key::Space, egui::Modifiers::CTRL)]);
    if let Some(task) = notes.completion.task.take() { task.abort(); }
    *notes.completion.cell.lock().unwrap() = Some(Delivery::Lookup(Ok(Page {
        rows: filter_rows(), status: handpick::DeliveryStatus::Complete,
    })));
    completion_frame(&mut notes, &ctx, &account, &runtime, Vec::new());
    let editor = notes.active().unwrap().editor;
    let before = editor.lock().unwrap().document_snapshot().unwrap();
    let selection = editor.lock().unwrap().selection.clone();
    let editor_widget = notes.surface.as_ref().unwrap().response.id;
    for (pressed, selected) in [(egui::Key::ArrowDown, 1), (egui::Key::ArrowUp, 0)] {
        completion_frame(&mut notes, &ctx, &account, &runtime, vec![key(pressed, egui::Modifiers::NONE)]);
        assert_eq!(notes.completion.selected, selected);
        assert!(ctx.memory(|memory| memory.has_focus(editor_widget)));
        assert_eq!(editor.lock().unwrap().selection, selection);
        assert_eq!(editor.lock().unwrap().document_snapshot().unwrap(), before);
    }
    completion_frame(&mut notes, &ctx, &account, &runtime, vec![key(egui::Key::Tab, egui::Modifiers::NONE)]);
    assert!(notes.completion.plan.is_some());
    assert!(ctx.memory(|memory| memory.has_focus(editor_widget)), "Tab selects the result instead of moving focus into popup controls");
    assert_eq!(editor.lock().unwrap().selection, selection);
    assert_eq!(editor.lock().unwrap().document_snapshot().unwrap(), before);
    completion_frame(&mut notes, &ctx, &account, &runtime, vec![key(egui::Key::Escape, egui::Modifiers::NONE)]);
    assert!(!notes.completion.is_open());
    assert!(!editor.lock().unwrap().handpick_inline_open);
    assert!(ctx.memory(|memory| memory.has_focus(editor_widget)), "Escape returns keyboard focus to the editor");
    assert_eq!(editor.lock().unwrap().document_snapshot().unwrap(), before);
}

#[test]
fn ctrl_space_without_editor_focus_cannot_start_a_native_popup() {
    let ctx = egui::Context::default();
    let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let (mut notes, account) = live_completion("[[Road", &ctx);
    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("other-control")));
    let _ = ctx.run(input(vec![key(egui::Key::Space, egui::Modifiers::CTRL)]), |ctx| {
        notes.completion_before_input(ctx, "workspace", Some(&account), "http://127.0.0.1:9911",
            Some(runtime.handle()), false);
    });
    assert!(!notes.completion.is_open());
    assert!(!notes.active().unwrap().editor.lock().unwrap().handpick_inline_open);
}

fn resolved_embed_ready_to_apply(ctx: &egui::Context, runtime: &tokio::runtime::Runtime)
    -> (MountedNotes, Arc<crate::local_account::AuthenticatedContext>) {
    let (mut notes, account) = live_completion("![[Road", ctx);
    completion_frame(&mut notes, ctx, &account, runtime, vec![key(egui::Key::Space, egui::Modifiers::CTRL)]);
    assert!(notes.completion.trigger.as_ref().unwrap().embed());
    if let Some(task) = notes.completion.task.take() { task.abort(); }
    let key = handpick::ResultKey { provider: handpick::ProviderId::new("native.notes").unwrap(),
        result: handpick::ResultId::new("source-note").unwrap() };
    let core = notes.core.as_mut().unwrap();
    core.deliver(&core.fence().unwrap(), handpick::Channel::Suggestions, handpick::DeliveryStatus::Complete,
        vec![handpick::SearchItem { key: key.clone(), label: "Road".into(), detail: None, actions: Vec::new() }]).unwrap();
    core.select(&core.fence().unwrap(), key.clone()).unwrap();
    notes.completion.plan = Some(core.plan_native_completion(notes.completion.trigger.as_ref().unwrap(),
        notes.completion.snapshot.as_ref().unwrap().native(), handpick::NativeCompletionChoice {
            key, value: handpick::NativeCompletionValue::Reference {
                target: handpick::NoteAddress::Note { note: handpick::NoteId::new("source-note").unwrap() },
                label: "Road".into(), embed: true,
            },
        }).unwrap());
    *notes.completion.cell.lock().unwrap() = Some(Delivery::Resolved(Ok(Material::NoteEmbed {
        note_id: "source-note".into(), block_id: "loom-note-projection".into(),
    })));
    (notes, account)
}

#[test]
fn whole_note_embed_driver_applies_resolved_identity_once_and_releases_popup() {
    use crate::rich_editor::document_model::{doc_json, node::{Child, TransclusionNode}};
    let ctx = egui::Context::default();
    let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let (mut notes, _) = resolved_embed_ready_to_apply(&ctx, &runtime);
    let editor = notes.active().unwrap().editor;
    let before = editor.lock().unwrap().document_snapshot().unwrap();
    notes.drain_completion(&ctx);
    notes.drain_completion(&ctx);
    assert!(notes.error.is_none());
    assert!(!notes.completion.is_open());
    let mut state = editor.lock().unwrap();
    let after = state.document_snapshot().unwrap();
    assert!(after.children[0].as_block().unwrap().children.contains(
        &Child::Transclusion(TransclusionNode::new("loom-note-projection"))));
    assert_eq!(doc_json::from_json_value(&doc_json::to_content_json_value(&after)).unwrap(), after);
    assert!(!state.handpick_inline_open && !state.handpick_template_open);
    assert_eq!(state.history_snapshot().unwrap().len(), 1);
    assert_eq!(state.pending_bus_undo.len(), 1);
    state.edit_document(|document, history| history.undo(document)).unwrap();
    assert_eq!(state.document_snapshot().unwrap(), before);
}

#[test]
fn whole_note_embed_delivery_rejects_stale_workspace_lease_selection_and_account() {
    for stale in ["workspace", "lease", "selection", "account"] {
        let ctx = egui::Context::default();
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let (mut notes, account) = resolved_embed_ready_to_apply(&ctx, &runtime);
        let editor = notes.active().unwrap().editor;
        let owner = editor.lock().unwrap().document_owner();
        let before = editor.lock().unwrap().document_snapshot().unwrap();
        match stale {
            "workspace" => notes.completion.provider.as_mut().unwrap().workspace = "other-workspace".into(),
            "lease" => { owner.advance_context().unwrap(); }
            "selection" => editor.lock().unwrap().selection = Selection::caret(DocPosition::new(vec![0, 0], 0)),
            "account" => account.invalidate(),
            _ => unreachable!(),
        }
        notes.drain_completion(&ctx);
        assert!(!notes.completion.is_open());
        if stale == "account" {
            owner.bind_context("workspace", "note", "http://127.0.0.1:9911", None,
                Some(crate::local_account::mock_account_context("http://127.0.0.1:9911"))).unwrap();
        }
        let state = editor.lock().unwrap();
        assert_eq!(state.document_snapshot().unwrap(), before);
        assert!(!state.handpick_inline_open);
        assert!(state.pending_bus_undo.is_empty());
        assert!(state.history_snapshot().unwrap().is_empty());
    }
}

#[test]
fn whole_note_embed_resolver_failure_keeps_text_and_restores_editor_input() {
    let ctx = egui::Context::default();
    let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let (mut notes, _) = resolved_embed_ready_to_apply(&ctx, &runtime);
    let editor = notes.active().unwrap().editor;
    let before = editor.lock().unwrap().document_snapshot().unwrap();
    *notes.completion.cell.lock().unwrap() = Some(Delivery::Resolved(Err("Note embed denied".into())));
    notes.drain_completion(&ctx);
    assert_eq!(notes.error.as_deref(), Some("Note embed denied"));
    assert!(!notes.completion.is_open());
    let state = editor.lock().unwrap();
    assert_eq!(state.document_snapshot().unwrap(), before);
    assert!(!state.handpick_inline_open);
    assert!(state.pending_bus_undo.is_empty());
    assert!(state.wikilink_autocomplete.is_some());
}

#[test]
fn pending_completion_reduces_motion_without_moving_controls_or_scheduling_idle_frames() {
    fn frame(ctx: &egui::Context, width: f32, time: f64, resolving: bool)
        -> (egui::FullOutput, (egui::Rect, egui::Rect, egui::Rect)) {
        let mut rects = None;
        let output = ctx.run(egui::RawInput { time: Some(time),
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 320.0))),
            ..Default::default() }, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let close = ui.small_button("Close").rect;
                let pending = completion_pending(ui, resolving).rect;
                let result = ui.button("Available result").rect;
                rects = Some((close, pending, result));
            });
        });
        (output, rects.unwrap())
    }
    for width in [120.0, 480.0] {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut normal = frame(&ctx, width, 0.0, false);
        for time in [1.0, 2.0, 3.0] { normal = frame(&ctx, width, time, false); }
        assert_eq!(normal.0.viewport_output[&egui::ViewportId::ROOT].repaint_delay, std::time::Duration::ZERO);
        motion_preference::apply_motion_preference(&ctx, true);
        let mut reduced = frame(&ctx, width, 4.0, false);
        for time in [5.0, 6.0, 7.0] { reduced = frame(&ctx, width, time, false); }
        assert_eq!(normal.1, reduced.1);
        assert_eq!(reduced.0.viewport_output[&egui::ViewportId::ROOT].repaint_delay, std::time::Duration::MAX);
        let next = frame(&ctx, width, 8.0, false);
        assert_eq!(reduced.0.shapes, next.0.shapes);
        let preparing = frame(&ctx, width, 9.0, true);
        assert_eq!(reduced.1, preparing.1);
        let node = preparing.0.platform_output.accesskit_update.as_ref().unwrap().nodes.iter()
            .find(|(_, node)| node.author_id() == Some("handpick.completion.pending")).unwrap();
        assert_eq!(node.1.role(), egui::accesskit::Role::ProgressIndicator);
        assert_eq!(node.1.label().as_deref(), Some("Preparing completion…"));
        motion_preference::apply_motion_preference(&ctx, false);
        let restored = frame(&ctx, width, 10.0, true);
        assert_eq!(normal.1, restored.1);
        assert_eq!(restored.0.viewport_output[&egui::ViewportId::ROOT].repaint_delay, std::time::Duration::ZERO);
    }
}

#[test]
fn reduced_motion_lookup_and_resolution_wake_and_deliver_without_spinner_polling() {
    use std::sync::atomic::{AtomicBool, Ordering};
    fn settle(ctx: &egui::Context) {
        for _ in 0..4 { let _ = ctx.run(input(Vec::new()), |_| {}); }
        assert!(!ctx.has_requested_repaint());
    }
    let ctx = egui::Context::default();
    motion_preference::apply_motion_preference(&ctx, true);
    let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let (mut notes, account) = live_completion("type:no", &ctx);
    completion_frame(&mut notes, &ctx, &account, &runtime, vec![key(egui::Key::Space, egui::Modifiers::CTRL)]);
    assert!(notes.completion.task.is_some());
    settle(&ctx);
    let woke = Arc::new(AtomicBool::new(false));
    let observed = woke.clone();
    ctx.set_request_repaint_callback(move |_| { observed.store(true, Ordering::SeqCst); });
    runtime.block_on(notes.completion.task.as_mut().unwrap()).unwrap();
    assert!(woke.load(Ordering::SeqCst), "actual lookup delivery wakes the viewport");
    notes.drain_completion(&ctx);
    assert!(notes.completion.task.is_none());
    assert!(!notes.completion.rows.is_empty());
    assert!(notes.error.is_none());
    notes.accept_completion(&ctx);
    assert!(notes.completion.task.is_some());
    settle(&ctx);
    woke.store(false, Ordering::SeqCst);
    runtime.block_on(notes.completion.task.as_mut().unwrap()).unwrap();
    assert!(woke.load(Ordering::SeqCst), "actual resolution delivery wakes the viewport");
    notes.drain_completion(&ctx);
    assert!(notes.error.is_none());
    assert!(!notes.completion.is_open());
    assert!(notes.completion.task.is_none());
    let editor = notes.active().unwrap().editor;
    let state = editor.lock().unwrap();
    assert_eq!(state.history_snapshot().unwrap().len(), 1);
    assert_eq!(state.pending_bus_undo.len(), 1);
    drop(state);
    settle(&ctx);
}
