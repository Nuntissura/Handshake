//! WP-KERNEL-012 MT-046 — INTERCONNECTION EDGE 2: note <-> code cross-reference (IC-06..IC-09).
//!
//! The scenarios drive the mounted native surfaces. IC-06 binds the managed product backend so the
//! clicked rich-editor chip resolves a real indexed code symbol; IC-08 binds the real MT-029 global-search
//! pane to managed code/note data; IC-07 remains deterministic; IC-09 drives a real rendered diagnostic
//! note chip through the mounted shell. They
//! prove that the code editor and the rich-text note editor share ONE InteractionBus + ONE command surface,
//! so a note can open a code symbol, a code symbol can be referenced from a note, a find runs across BOTH
//! surfaces through the SAME bus, and a code diagnostic can navigate to a note — all over the single shared
//! substrate, NOT two independent backends that happen to return the same data (the anti-RISK-1 control).
//!
//! The LOAD-BEARING anti-mock-smuggling control (CTRL-1, RISK-1): IC-06/IC-08/IC-09 use the real crate
//! `InteractionBus` (not a mock bus). IC-08 dispatches the product-registered Find-in-Files command,
//! enters the query in MT-029, and asserts exact producer ids from the global code + note results while
//! the mounted rich-document find executes that same query.
//!
//! AccessKit note (CTRL-3): `Harness::run_steps()` flushes the mounted shell, and the real rich-editor
//! interactive surface author_id is `editor.rich.text`. IC-09 asserts that actual destination surface.
//!
//! Artifact hygiene (CX-212E): no artifact under `src/`.

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
#[path = "interconnect_support/mod.rs"]
mod interconnect_support;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use base64::Engine as _;
use canonical_argus_driver::{json_has_author_id, json_node_by_author_id, CanonicalArgusDriver};
use egui_kittest::kittest::{NodeT, Queryable};
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::gutter::DiagnosticSeverity;
use handshake_native::code_editor::panel::{
    CodeEditorPanel, CODE_EDITOR_CONTEXT_SURFACE_AUTHOR_ID, CODE_EDITOR_TEXT_AUTHOR_ID,
};
use handshake_native::code_editor::CODE_EDITOR_CTX_COPY_NOTE_REF_AUTHOR_ID;
use handshake_native::command_registry::CMD_VIEW_RICH_NOTE;
use handshake_native::find_in_files::{result_author_id, QUERY_AUTHOR_ID, SEARCH_AUTHOR_ID};
use handshake_native::interop::{ClipboardPayload, InteractionBus};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::rich_editor::document_model::node::{
    BlockNode, Child, HsLinkNode, NodeKind, TextLeaf,
};
use handshake_native::rich_editor::document_model::{DocPosition, Selection};
use handshake_native::rich_editor::wikilinks::inline_view::code_ref_chip_author_id;

use interconnect_support::{
    assert_no_local_artifact_dir, author_ids, event_ledger_payload, external_artifact_dir,
    require_live_backend, save_rich_document_via_production_manager, ScenarioAttempt,
};

#[test]
fn supplemental_ic06_code_panel_focus_boundary_probe() {
    let panel = Arc::new(CodeEditorPanel::with_instance(
        "fn focus_probe() {}\n",
        "rs",
        "ic06-focus-probe",
    ));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 500.0))
        .build_state(
            |ctx, panel: &mut Arc<CodeEditorPanel>| {
                egui::CentralPanel::default().show(ctx, |ui| panel.show(ui));
            },
            panel,
        );
    harness.run_steps(2);
    harness.state().request_text_focus();
    harness.step();
    harness.step();

    let panel = harness.state();
    let focused = harness
        .root()
        .children_recursive()
        .filter(|node| node.accesskit_node().is_focused())
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect::<Vec<_>>();
    assert!(
        panel.live_text_has_focus(&harness.ctx),
        "IC-06 boundary probe: direct code panel must retain egui focus; focused={focused:?}; egui={:?}; pending={}",
        harness.ctx.memory(|memory| memory.focused()),
        panel.text_focus_request_pending_for_test(),
    );
    assert_eq!(focused, vec![panel.text_author_id()]);
}

fn editor_shell() -> (HandshakeApp, tokio::runtime::Runtime) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("IC-06/09: build runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    {
        let registry = app.pane_registry();
        let mut registry = registry.lock().expect("IC-06/09: pane registry");
        registry.insert(PaneRecord::new(
            PaneId::from("pane-a"),
            PaneType::CodeSymbol,
            DEFAULT_PROJECT_ID,
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
        registry.insert(PaneRecord::new(
            PaneId::from("pane-b"),
            PaneType::LoomWikiPage,
            DEFAULT_PROJECT_ID,
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
    }
    (app, runtime)
}

struct SourceFixture {
    root: PathBuf,
    file: PathBuf,
    content: String,
}

impl Drop for SourceFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn indexed_source_fixture() -> SourceFixture {
    let root =
        external_artifact_dir("fixtures").join(format!("ic06-{}", uuid::Uuid::new_v4().simple()));
    let source_dir = root.join("ic06_fixture_src");
    std::fs::create_dir_all(&source_dir).expect("IC-06: create external source fixture");
    let content = (0..40)
        .map(|line| {
            if line == 6 {
                "pub fn my_function() -> u32 { 7 }\n".to_owned()
            } else {
                format!("pub fn filler_{line}() -> usize {{ {line} }}\n")
            }
        })
        .collect::<String>();
    let file = source_dir.join("lib.rs");
    std::fs::write(&file, &content).expect("IC-06: write external source fixture");
    std::fs::write(root.join("README.md"), "# IC-06 source root\n")
        .expect("IC-06: write source-root anchor");
    SourceFixture {
        root,
        file,
        content,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-06 — Open code block from note (LIVE-PG PASS): a real rendered `[[code:symbol]]` chip is clicked by
// stable AccessKit id. The mounted shell resolves it against the managed code-nav index, opens the exact
// source file in the real code pane, focuses that pane, and places the caret on the definition line.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic06_open_code_block_from_note() {
    let attempt = ScenarioAttempt::begin("IC-06");
    let mut be = require_live_backend();
    let backend_binding = be.owned_backend_binding_receipt();
    let fixture = indexed_source_fixture();
    let index = be.post_json(
        &format!("/workspaces/{}/code-nav/index", be.workspace_id),
        &serde_json::json!({"root_path": fixture.root.to_string_lossy()}),
    );
    assert!(
        index["symbol_count"].as_u64().unwrap_or(0) >= 1,
        "IC-06: managed code-nav index must contain the fixture symbol: {index}"
    );
    let literal_ref = "ic06_fixture_src/lib.rs#my_function".to_owned();
    let chip_id = code_ref_chip_author_id(&literal_ref);

    let (mut app, runtime) = editor_shell();
    app.set_backend_base_url_for_test(&be.base, runtime.handle().clone());
    app.bind_active_project_for_integration_test(be.workspace_id.clone());
    app.mounted_code_panel()
        .set_file_path(fixture.root.join("README.md").to_string_lossy());
    app.set_active_pane_for_test(Some(PaneId::from("pane-b")));
    let rich_state = app.mounted_rich_state();
    let mut paragraph = BlockNode::new(NodeKind::Paragraph);
    paragraph.children.push(Child::Text(TextLeaf::new("open ")));
    paragraph.children.push(Child::HsLink(HsLinkNode::new(
        "code",
        literal_ref.clone(),
        "my_function",
    )));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    let ctx = harness.ctx.clone();
    assert!(
        harness
            .state_mut()
            .dispatch_palette_action_for_test_with_ctx(&ctx, CMD_VIEW_RICH_NOTE),
        "IC-06: operator-facing rich-note command mounts the note containing the code chip"
    );
    rich_state.lock().expect("IC-06: rich state").doc = BlockNode::doc(vec![paragraph]);
    harness.run_steps(3);
    assert!(
        author_ids(&harness).contains(&chip_id),
        "IC-06: mounted rich editor renders the code chip at stable AccessKit id {chip_id}"
    );
    harness
        .get_by(|node| node.author_id() == Some(chip_id.as_str()))
        .click_accesskit();
    harness.step();

    let expected_file = fixture
        .file
        .canonicalize()
        .expect("IC-06: canonical fixture");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        let panel = harness.state().active_mounted_code_panel();
        if std::path::Path::new(&panel.file_path())
            .canonicalize()
            .ok()
            .as_ref()
            == Some(&expected_file)
            && panel.buffer().to_string() == fixture.content
            && panel.cursors().primary().head
                == panel
                    .buffer()
                    .line_to_byte(6)
                    .expect("IC-06: definition line")
        {
            break;
        }
        harness.step();
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    // The async load completion opens the target tab after the pane render in that frame. Give the
    // newly mounted destination one frame to emit its live text node and one to apply its focus request.
    harness.run_steps(2);

    let app = harness.state();
    let panel = app.active_mounted_code_panel();
    assert_eq!(
        std::path::Path::new(&panel.file_path())
            .canonicalize()
            .expect("IC-06: active canonical code file"),
        expected_file,
        "IC-06: AccessKit chip activation opens the indexed source file; navigation status={:?}",
        app.quick_switcher_nav_status()
    );
    let expected_byte = panel.buffer().line_to_byte(6).expect("IC-06: line 7");
    assert_eq!(
        panel.cursors().primary().head,
        expected_byte,
        "IC-06: mounted code editor lands on my_function's real indexed definition"
    );
    let active_pane = app
        .active_pane()
        .expect("IC-06: navigation focuses code pane");
    assert_eq!(
        active_pane.as_ref(),
        "pane-a",
        "IC-06: code pane owns focus"
    );
    let is_code_text_surface = |author_id: &str| {
        author_id == CODE_EDITOR_TEXT_AUTHOR_ID
            || author_id.starts_with(&format!("{CODE_EDITOR_TEXT_AUTHOR_ID}#"))
            || author_id.starts_with(&format!("{CODE_EDITOR_TEXT_AUTHOR_ID}--view-"))
    };
    assert!(
        author_ids(&harness)
            .iter()
            .any(|author_id| is_code_text_surface(author_id)),
        "IC-06: focused destination exposes the real code editor AccessKit surface"
    );
    let focused_author_ids = harness
        .root()
        .children_recursive()
        .filter(|node| node.accesskit_node().is_focused())
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect::<Vec<_>>();
    let egui_focused = harness.ctx.memory(|memory| memory.focused());
    let live_code_text_has_focus = panel.live_text_has_focus(&harness.ctx);
    let focus_request_pending = panel.text_focus_request_pending_for_test();
    let active_panel_text_author = panel.text_author_id();
    let rendered_code_text_authors = author_ids(&harness)
        .into_iter()
        .filter(|author_id| is_code_text_surface(author_id))
        .collect::<Vec<_>>();
    assert!(
        focused_author_ids
            .iter()
            .any(|author_id| is_code_text_surface(author_id)),
        "IC-06: chip navigation must transfer AccessKit focus to the code editor text surface; focused nodes={focused_author_ids:?}; egui focused={egui_focused:?}; active panel live-text focus={live_code_text_has_focus}; active panel focus pending={focus_request_pending}; active panel text author={active_panel_text_author}; rendered code text authors={rendered_code_text_authors:?}"
    );

    let runtime_diagnostics = be
        .assert_cleanup_and_publish_runtime_diagnostics("IC-06")
        .expect("IC-06: publish fixture-owned backend runtime diagnostics");
    attempt.pass(serde_json::json!({
        "backend_binding": backend_binding,
        "runtime_diagnostics": runtime_diagnostics,
        "accesskit_chip": chip_id,
        "destination_file": expected_file,
        "caret_byte": expected_byte,
        "focused_pane": active_pane.as_ref(),
    }));
    assert_no_local_artifact_dir();
    println!(
        "IC-06 LIVE-PG PASS: AccessKit code chip opened the indexed source and focused my_function"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-07 — Reference code symbol from note (SUBSTRATE PASS, wave-2: drives the REAL MT-046 command):
// 'Copy as note reference' is activated from the mounted code editor's real context menu. The mounted
// pane factory drains the staged `[[code:...]]` ref into the app-owned InteractionBus clipboard. The test
// then switches to the mounted rich pane, focuses its AccessKit text surface, and sends Ctrl+V through the
// production key route; Paste materializes the canonical code wikilink as an hsLink which round-trips PG.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic07_reference_code_symbol_from_note() {
    let attempt = ScenarioAttempt::begin("IC-07");
    use handshake_native::code_editor::Cursor;
    let mut be = require_live_backend();
    let workspace_id = be.workspace_id.clone();
    let backend_binding = be.owned_backend_binding_receipt();

    // (1) Mount both product editors in one app, then activate the REAL code-body context-menu command.
    let src = "fn my_function() {\n    let x = 1;\n}\n";
    let (mut app, _runtime) = editor_shell();
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
    let code_panel = app.mounted_code_panel();
    code_panel.set_text(src);
    let sel_start = src.find("my_function").expect("symbol in the snippet");
    code_panel.set_cursors(vec![Cursor::selection(
        sel_start,
        sel_start + "my_function".len(),
    )]);
    let rich_state = app.mounted_rich_state();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);
    assert!(
        author_ids(&harness).contains(CODE_EDITOR_CONTEXT_SURFACE_AUTHOR_ID),
        "IC-07: active mounted code editor context surface is live"
    );
    assert_eq!(
        code_panel.note_reference_for_cursor(),
        None,
        "IC-07: the selected symbol cannot produce a canonical reference before the buffer has a path"
    );
    harness
        .get_by(|node| node.author_id() == Some("code_editor_ctx_rename_symbol"))
        .click_accesskit();
    harness.run_steps(2);
    let copy_menu_author_id = format!("ctx-menu.{CODE_EDITOR_CTX_COPY_NOTE_REF_AUTHOR_ID}");
    let unsaved_copy_item = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(copy_menu_author_id.as_str()))
        .expect("IC-07: unsaved-buffer Copy as note reference item remains mounted");
    assert!(
        unsaved_copy_item.accesskit_node().is_disabled(),
        "IC-07: unsaved-buffer Copy as note reference is disabled on the operator-facing menu"
    );
    harness.key_press(egui::Key::Escape);
    harness.run_steps(2);

    code_panel.set_file_path("src/lib.rs");
    harness.run_steps(2);
    assert_eq!(
        code_panel.note_reference_for_cursor().as_deref(),
        Some("[[code:src/lib.rs#my_function]]"),
        "IC-07: saving/assigning the path enables the canonical path#symbol producer"
    );
    // Activate the mounted panel's canonical always-addressable context-menu opener. It opens the REAL
    // typed editor-body popup; the next action still targets the actual Copy-as-note-reference MenuItem,
    // so this cannot bypass or directly dispatch the command under proof.
    harness
        .get_by(|node| node.author_id() == Some("code_editor_ctx_rename_symbol"))
        .click_accesskit();
    harness.run_steps(2);
    let _copy_menu_item = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(copy_menu_author_id.as_str()))
        .unwrap_or_else(|| {
            panic!(
                "IC-07: Copy as note reference is a live mounted MenuItem; live author ids: {:?}",
                author_ids(&harness)
            )
        });
    // The shared menu cursor starts on the first live entry. ArrowUp wraps to the last live entry,
    // which is Copy as note reference, then Enter confirms that exact typed MenuItem.
    harness.key_press(egui::Key::ArrowUp);
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();

    let note_ref = InteractionBus::get_or_init(&harness.ctx)
        .lock()
        .unwrap()
        .clipboard_read_text()
        .expect("IC-07: mounted factory wrote the generated ref to the app-owned shared clipboard");
    assert_eq!(
        note_ref, "[[code:src/lib.rs#my_function]]",
        "IC-07: mounted Copy as note reference built path#symbol from the live code selection"
    );

    // (2) Switch through the real command dispatcher, focus the mounted rich AccessKit surface, and send
    // Ctrl+V. The production rich clipboard adapter parses this complete canonical token to an hsLink.
    harness
        .state_mut()
        .set_active_pane_for_test(Some(PaneId::from("pane-b")));
    let ctx = harness.ctx.clone();
    assert!(
        harness
            .state_mut()
            .dispatch_palette_action_for_test_with_ctx(&ctx, CMD_VIEW_RICH_NOTE),
        "IC-07: the real view command activates the mounted rich pane"
    );
    harness.run_steps(3);
    {
        // Mounting an unbound rich-note tab intentionally supplies the product demo document. Install
        // the proof document after that mount so Ctrl+V targets the same active state the pane renders.
        let mut rich = rich_state.lock().unwrap();
        rich.doc = BlockNode::doc(vec![
            BlockNode::paragraph("see REPLACE_"),
            BlockNode::paragraph("ME later"),
        ]);
        rich.selection = Selection::text(
            DocPosition::new(vec![0, 0], 4),
            DocPosition::new(vec![1, 0], 2),
        );
    }
    harness.step();
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("editor.rich.text"))
        .expect("IC-07: mounted rich editor AccessKit text surface is live")
        .focus();
    harness.run_steps(2);
    let before_paste = rich_state.lock().unwrap().doc.clone();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
    harness.run_steps(3);

    {
        let rich = rich_state.lock().unwrap();
        assert_eq!(
            rich.doc.children.len(),
            1,
            "IC-07: covered sibling blocks merge; selection={:?}, blocks={:?}",
            rich.selection,
            rich.doc.children
        );
        let paragraph = rich.doc.children[0]
            .as_block()
            .expect("IC-07: mounted rich paragraph");
        assert!(
            matches!(
                paragraph.children.as_slice(),
                [Child::Text(prefix), Child::HsLink(link), Child::Text(suffix)]
                    if prefix.text.to_string() == "see "
                        && link.ref_kind == "code"
                        && link.ref_value == "src/lib.rs#my_function"
                        && suffix.text.to_string() == " later"
            ),
            "IC-07: Ctrl+V replaces only the active rich range with one hsLink: {:?}",
            paragraph.children
        );
    }
    let rich_pane = PaneId::from("pane-b");
    assert_eq!(
        InteractionBus::get_or_init(&harness.ctx)
            .lock()
            .unwrap()
            .local_undo_count(&rich_pane),
        1,
        "IC-07: range replacement plus hsLink insertion is one mounted undo transaction"
    );
    assert!(
        harness.root().children_recursive().any(|node| {
            node.accesskit_node().author_id() == Some("editor.rich.text")
                && node.accesskit_node().is_focused()
        }),
        "IC-07: rich editor retains focus after mounted paste"
    );

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run_steps(3);
    let undo_count_after_shortcut = InteractionBus::get_or_init(&harness.ctx)
        .lock()
        .unwrap()
        .local_undo_count(&rich_pane);
    assert_eq!(
        rich_state.lock().unwrap().doc,
        before_paste,
        "IC-07: mounted Ctrl+Z restores the exact pre-paste selected text and document shape; local undo count after shortcut={undo_count_after_shortcut}"
    );
    assert_eq!(
        InteractionBus::get_or_init(&harness.ctx)
            .lock()
            .unwrap()
            .local_undo_count(&rich_pane),
        0,
        "IC-07: one Ctrl+Z consumes the paste's single local undo entry"
    );

    // Reverse-direction cross-block selection must produce the same one-transaction mounted result.
    {
        let mut rich = rich_state.lock().unwrap();
        rich.selection = Selection::text(
            DocPosition::new(vec![1, 0], 2),
            DocPosition::new(vec![0, 0], 4),
        );
    }
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
    harness.run_steps(3);
    {
        let rich = rich_state.lock().unwrap();
        let paragraph = rich.doc.children[0].as_block().unwrap();
        assert!(matches!(
            paragraph.children.as_slice(),
            [Child::Text(prefix), Child::HsLink(link), Child::Text(suffix)]
                if prefix.text.to_string() == "see "
                    && link.ref_value == "src/lib.rs#my_function"
                    && suffix.text.to_string() == " later"
        ));
    }
    assert_eq!(
        InteractionBus::get_or_init(&harness.ctx)
            .lock()
            .unwrap()
            .local_undo_count(&rich_pane),
        1,
        "IC-07: reverse cross-block paste creates exactly one shared undo entry"
    );
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run_steps(3);
    assert_eq!(rich_state.lock().unwrap().doc, before_paste);

    // Empty and malformed code targets must never become typed atoms. Drive both through the same
    // mounted Ctrl+V route and prove their exact bytes survive as ordinary text, then undo each probe.
    for (probe_index, malformed) in [
        "[[code:   ]]",
        "[[code:src/lib.rs#]]",
        "[[code: src/lib.rs#my_function ]]",
        "[[code:src/lib.rs#bad]symbol]]",
        "[[code:src/lib.rs#foo bar]]",
        "see [[code:src/lib.rs#my_function]] now",
    ]
    .into_iter()
    .enumerate()
    {
        {
            let mut rich = rich_state.lock().unwrap();
            rich.selection = if probe_index % 2 == 0 {
                Selection::text(
                    DocPosition::new(vec![0, 0], 4),
                    DocPosition::new(vec![1, 0], 2),
                )
            } else {
                Selection::text(
                    DocPosition::new(vec![1, 0], 2),
                    DocPosition::new(vec![0, 0], 4),
                )
            };
        }
        InteractionBus::get_or_init(&harness.ctx)
            .lock()
            .unwrap()
            .cache_clipboard(ClipboardPayload::PlainText(malformed.to_owned()));
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
        harness.run_steps(3);
        {
            let rich = rich_state.lock().unwrap();
            let expected_text = format!("see {malformed} later");
            assert_eq!(
                rich.block_plain_text(0).as_deref(),
                Some(expected_text.as_str()),
                "IC-07: malformed code target remains byte-for-byte plain text"
            );
            let paragraph = rich.doc.children[0].as_block().unwrap();
            assert!(
                paragraph
                    .children
                    .iter()
                    .all(|child| !matches!(child, Child::HsLink(_))),
                "IC-07: malformed code target does not materialize any hsLink"
            );
        }
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        harness.run_steps(3);
        assert_eq!(
            rich_state.lock().unwrap().doc,
            before_paste,
            "IC-07: malformed/mixed cross-block plain-text probe is independently undoable"
        );
    }

    // Restore the producer's canonical payload and selected range, then paste again for the durable PG
    // round-trip below. The proof therefore persists the same product outcome that passed Ctrl+Z.
    InteractionBus::get_or_init(&harness.ctx)
        .lock()
        .unwrap()
        .cache_clipboard(ClipboardPayload::PlainText(note_ref.clone()));
    {
        let mut rich = rich_state.lock().unwrap();
        rich.selection = Selection::text(
            DocPosition::new(vec![0, 0], 4),
            DocPosition::new(vec![1, 0], 2),
        );
    }
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
    harness.run_steps(3);

    let symbol_key = note_ref
        .strip_prefix("[[code:")
        .and_then(|s| s.strip_suffix("]]"))
        .expect("IC-07: the real ref has the [[code:…]] shape");
    let doc = rich_state.lock().unwrap().doc.clone();

    // (3) Structural preflight through the same DocJson shape sent by the native SaveManager.
    use handshake_native::rich_editor::document_model::doc_json::{
        from_json_string, to_content_json_value, to_json_string,
    };
    let json = to_json_string(&doc).expect("serialize");
    let reloaded = from_json_string(&json).expect("reload");
    assert_eq!(
        doc, reloaded,
        "IC-07: the code-ref note round-trips DocJson unchanged"
    );
    let v = to_content_json_value(&doc);
    let json_str = serde_json::to_string(&v).unwrap();
    assert!(
        json_str.contains("\"hsLink\""),
        "IC-07: the inserted ref is an hsLink node"
    );
    assert!(
        json_str.contains("\"refKind\":\"code\""),
        "IC-07: the node kind is code"
    );
    assert!(
        json_str.contains(symbol_key),
        "IC-07: mounted Paste materialized the symbol key as the hsLink refValue"
    );

    // (4) Real managed-PostgreSQL KRD save + GET reload through the production native SaveManager.
    // Create link-free content first so only this save can introduce the code reference.
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({
            "workspace_id": workspace_id,
            "title": "IC-07 code reference note",
            "content_json": to_content_json_value(&BlockNode::doc(vec![BlockNode::paragraph("reference pending")]))
        }),
    );
    let document_id = created
        .pointer("/document/rich_document_id")
        .or_else(|| created.get("rich_document_id"))
        .and_then(serde_json::Value::as_str)
        .expect("IC-07: production create returns rich_document_id")
        .to_owned();
    let created_version = created
        .pointer("/document/doc_version")
        .or_else(|| created.get("doc_version"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1);
    let save =
        save_rich_document_via_production_manager(&be, &document_id, created_version, v.clone());
    assert!(
        save.doc_version > created_version,
        "IC-07: production save advances the KRD document version"
    );
    let loaded = be.get_json(&format!("/knowledge/documents/{document_id}"));
    let persisted = loaded
        .pointer("/document/content_json")
        .or_else(|| loaded.get("content_json"))
        .expect("IC-07: production GET returns persisted content_json");
    let persisted_text = serde_json::to_string(persisted).expect("IC-07 persisted content JSON");
    assert!(
        persisted_text.contains("\"hsLink\"")
            && persisted_text.contains("\"refKind\":\"code\"")
            && persisted_text.contains(symbol_key),
        "IC-07: managed-PG GET preserves hsLink(code,{symbol_key}); got {persisted_text}"
    );
    let receipt_payload = event_ledger_payload(&save.save_receipt_event_id);
    assert_eq!(
        receipt_payload["workspace_id"].as_str(),
        Some(workspace_id.as_str()),
        "IC-07: exact save receipt is scoped to the owned workspace"
    );
    let receipt_reference_target = format!("code:{symbol_key}");
    assert!(
        receipt_payload["reference_targets"]
            .as_array()
            .is_some_and(|targets| targets
                .iter()
                .any(|target| target.as_str() == Some(receipt_reference_target.as_str()))),
        "IC-07: exact save receipt records the persisted code reference target"
    );
    let _ = be.delete(&format!("/knowledge/documents/{document_id}"));
    let runtime_diagnostics = be
        .assert_cleanup_and_publish_runtime_diagnostics("IC-07")
        .expect("IC-07: publish fixture-owned backend runtime diagnostics");

    attempt.pass(serde_json::json!({
        "backend_binding": backend_binding,
        "runtime_diagnostics": runtime_diagnostics,
        "workspace_id": workspace_id,
        "document_id": document_id,
        "note_reference": note_ref,
        "ref_kind": "code",
        "ref_value": symbol_key,
        "receipt_reference_target": receipt_reference_target,
        "saved_doc_version": save.doc_version,
        "save_receipt_event_id": save.save_receipt_event_id,
        "managed_pg_get_round_trip": true,
    }));
    assert_no_local_artifact_dir();
    println!("IC-07 LIVE-PG PASS: 'Copy as note reference' -> clipboard {note_ref} -> hsLink(code) persisted through production SaveManager + GET");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-08 — Shared global find across code and note (LIVE-PG PASS, the load-bearing CTRL-1 proof): the
// operator-facing Find in Files command opens the real MT-029 pane. Its rendered query + real graph-search
// rows and the mounted rich-document find converge through ONE InteractionBus; no hand-built result/count
// can satisfy the assertions.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic08_shared_find_replace() {
    let attempt = ScenarioAttempt::begin("IC-08");
    let mut be = require_live_backend();
    let workspace_id = be.workspace_id.clone();
    let backend_binding = be.owned_backend_binding_receipt();
    let probe = format!("SHARED_FIND_PROBE_{}", uuid::Uuid::new_v4().simple());
    let code_source = format!("// {probe}\nfn ic08_search_fixture() {{}}\n");
    let code_created = be.post_json(
        &format!("/workspaces/{workspace_id}/loom/import"),
        &serde_json::json!({
            "bytes_b64": base64::engine::general_purpose::STANDARD.encode(code_source),
            "original_filename": format!("ic08_{probe}.rs"),
            "mime": "text/x-rust"
        }),
    );
    let code_block_id = code_created["block_id"]
        .as_str()
        .or_else(|| code_created["id"].as_str())
        .expect("IC-08: managed code-file block id")
        .to_owned();
    let note_created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({
            "workspace_id": workspace_id,
            "title": "IC-08 managed rich note",
            "content_json": {
                "type": "doc",
                "content": [{
                    "type": "paragraph",
                    "content": [{"type": "text", "text": format!("managed note {probe}")}]
                }]
            }
        }),
    );
    let note_document_id = note_created["document"]["rich_document_id"]
        .as_str()
        .expect("IC-08: managed rich-document id")
        .to_owned();
    let seeded_search = be.get_json(&format!(
        "/workspaces/{workspace_id}/loom/graph-search?q={probe}&limit=500&offset=0"
    ));
    let seeded_hits = seeded_search
        .as_array()
        .expect("IC-08: graph-search seed precondition returns an array");
    assert!(
        seeded_hits.iter().any(|hit| {
            hit["ref_id"].as_str() == Some(code_block_id.as_str())
                && hit["source_kind"].as_str() == Some("file")
        }),
        "IC-08: canonical Loom import must be immediately searchable as the exact file row: {seeded_search}"
    );
    assert!(
        seeded_hits
            .iter()
            .any(|hit| hit["ref_id"].as_str() == Some(note_document_id.as_str())),
        "IC-08: canonical rich-document save must be immediately searchable as the exact note row: {seeded_search}"
    );

    // Use the exact mounted HandshakeApp + MT-029 client, bound to the managed workspace/backend.
    let (mut app, runtime) = editor_shell();
    app.set_backend_base_url_for_test(&be.base, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.clone());
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
    let rich_state = app.mounted_rich_state();
    rich_state.lock().expect("IC-08: rich state").doc = BlockNode::doc(vec![BlockNode::paragraph(
        &format!("the mounted note mentions {probe} once"),
    )]);

    // Capture the Context owned by this rendered shell so command dispatch, MT-029 UI input, and bus
    // inspection all address the same real InteractionBus instance.
    let ctx_capture = Arc::new(Mutex::new(None::<egui::Context>));
    let render_ctx_capture = Arc::clone(&ctx_capture);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 700.0))
        .build_state(
            move |ctx, app: &mut HandshakeApp| {
                *render_ctx_capture.lock().expect("IC-08: capture ctx") = Some(ctx.clone());
                app.ui(ctx);
            },
            app,
        );
    harness.run_steps(2);
    let ctx = ctx_capture
        .lock()
        .expect("IC-08: captured ctx lock")
        .clone()
        .expect("IC-08: rendered egui context");
    assert!(
        harness
            .state_mut()
            .dispatch_palette_action_for_test_with_ctx(
                &ctx,
                handshake_native::command_registry::CMD_EDITOR_FIND_IN_FILES,
            ),
        "IC-08: operator-facing Find in Files dispatched through HandshakeApp"
    );
    harness.run_steps(2);
    let ids = author_ids(&harness);
    assert!(
        ids.contains(QUERY_AUTHOR_ID),
        "IC-08: real MT-029 query input rendered"
    );
    assert!(
        ids.contains(SEARCH_AUTHOR_ID),
        "IC-08: real MT-029 Search control rendered"
    );
    harness
        .get_by(|node| node.author_id() == Some(QUERY_AUTHOR_ID))
        .focus();
    harness.step();
    harness
        .get_by(|node| node.author_id() == Some(QUERY_AUTHOR_ID))
        .type_text(&probe);
    harness.run_steps(1);
    harness
        .get_by(|node| node.author_id() == Some(SEARCH_AUTHOR_ID))
        .click_accesskit();

    // The rendered pane owns the HTTP request and stamped delivery. Wait for the exact persisted ids to
    // appear in BOTH typed bus lanes; counts or local buffers cannot satisfy this gate.
    let bus = InteractionBus::get_or_init(&ctx);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        harness.run_steps(1);
        let found =
            InteractionBus::with_try_lock(&bus, |bus| {
                let results = bus.shared_find_results();
                results
                    .code
                    .entries
                    .iter()
                    .any(|entry| entry.block_id.as_deref() == Some(code_block_id.as_str()))
                    && results.note.entries.iter().any(|entry| {
                        entry.document_id.as_deref() == Some(note_document_id.as_str())
                    })
            })
            .unwrap_or(false);
        if found {
            break;
        }
        if std::time::Instant::now() >= deadline {
            let snapshot =
                InteractionBus::with_try_lock(&bus, |bus| bus.shared_find_results().clone());
            let panel_diagnostics = harness.state().find_in_files_diagnostics_for_test();
            panic!(
                "IC-08: real MT-029 backend results did not reach both typed bus lanes within 30s; shared results={snapshot:?}; panel={panel_diagnostics:?}"
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    let (dispatches, results, command_count) = InteractionBus::with_try_lock(&bus, |bus| {
        (
            bus.shared_find_dispatch_generation(),
            bus.shared_find_results().clone(),
            bus.commands().len(),
        )
    })
    .expect("IC-08: shared bus available");
    assert_eq!(
        dispatches, 1,
        "IC-08 CTRL-1: exactly one real global Find dispatch"
    );
    assert_eq!(results.query.pattern, probe, "IC-08: typed shared query");
    assert_eq!(
        results
            .code
            .entries
            .iter()
            .filter(|entry| entry.block_id.as_deref() == Some(code_block_id.as_str()))
            .count(),
        1,
        "IC-08: exact managed code-file row appears once"
    );
    assert_eq!(
        results
            .note
            .entries
            .iter()
            .filter(|entry| entry.document_id.as_deref() == Some(note_document_id.as_str()))
            .count(),
        1,
        "IC-08: exact managed rich-document row appears once"
    );
    assert_eq!(
        rich_state
            .lock()
            .expect("IC-08: rich state")
            .find_replace
            .as_ref()
            .expect("IC-08: native note find surface opened")
            .query
            .pattern,
        probe,
        "IC-08: mounted rich-document find executes the MT-029 query"
    );
    assert!(
        command_count >= 6,
        "IC-08: mounted app retains at least the six canonical editor commands"
    );
    assert_eq!(
        results.note.mounted_match_count, 1,
        "IC-08: mounted rich-note scan remains distinct from backend entry count"
    );
    drop(harness);
    let runtime_diagnostics = be
        .assert_cleanup_and_publish_runtime_diagnostics("IC-08")
        .expect("IC-08: publish fixture-owned backend runtime diagnostics");
    let evidence = serde_json::json!({
        "backend_binding": backend_binding,
        "runtime_diagnostics": runtime_diagnostics,
        "workspace_id": workspace_id,
        "query": probe,
        "code_block_id": code_block_id,
        "note_document_id": note_document_id,
        "code_backend_entries": results.code.entries.len(),
        "note_backend_entries": results.note.entries.len(),
        "mounted_note_matches": results.note.mounted_match_count,
        "dispatches": dispatches,
    });
    attempt.pass(evidence);
    assert_no_local_artifact_dir();
    println!(
        "IC-08 LIVE-PG PASS: real MT-029 global Find returned the managed code file + rich note and \
         drove the mounted rich-document find through one InteractionBus"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-09 — Code diagnostics surface note reference (SUBSTRATE PASS): the mounted code gutter renders a real
// related-note button with a stable AccessKit id. Clicking it routes through the shared product bus, opens
// the exact note destination, focuses the rich pane, and renders `editor.rich.text` in the live tree.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic09_diagnostic_note_reference() {
    let attempt = ScenarioAttempt::begin("IC-09");
    let mut be = require_live_backend();
    let backend_binding = be.owned_backend_binding_receipt();
    let note_created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({
            "workspace_id": be.workspace_id.clone(),
            "title": "IC-09 diagnostic destination",
            "content_json": {
                "type": "doc",
                "content": [{
                    "type": "paragraph",
                    "content": [{"type": "text", "text": "Diagnostic explanation loaded from PostgreSQL"}]
                }]
            }
        }),
    );
    let note_doc_id = note_created["document"]["rich_document_id"]
        .as_str()
        .expect("IC-09: managed rich-document id")
        .to_owned();
    let (mut app, runtime) = editor_shell();
    app.set_backend_base_url_for_test(&be.base, runtime.handle().clone());
    app.bind_active_project_for_integration_test(be.workspace_id.clone());
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
    let code_panel = app.mounted_code_panel();
    code_panel.set_text("fn main() { missing(); }\n");
    code_panel.push_diagnostic_note_reference(
        0,
        DiagnosticSeverity::Error,
        "missing() is explained in the linked note",
        &note_doc_id,
    );
    let chip_id = code_panel.diagnostic_note_reference_author_id(0);

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);
    assert!(
        author_ids(&harness).contains(&chip_id),
        "IC-09: the real code gutter renders the related-note chip at {chip_id}"
    );
    harness
        .get_by(|node| node.author_id() == Some(chip_id.as_str()))
        .click_accesskit();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        harness.run_steps(1);
        let target_is_active = harness.state().active_pane().is_some_and(|pane_id| {
            harness
                .state()
                .tab_bar_states()
                .get(pane_id)
                .and_then(|bar| bar.active())
                .is_some_and(|tab| {
                    tab.pane_type == PaneType::LoomWikiPage
                        && tab.content_id.as_deref() == Some(note_doc_id.as_str())
                })
        });
        if target_is_active && author_ids(&harness).contains("editor.rich.text") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    let app = harness.state();
    let active_pane = app
        .active_pane()
        .expect("IC-09: diagnostic note navigation focuses a pane");
    let active_tab = app
        .tab_bar_states()
        .get(active_pane)
        .and_then(|bar| bar.active())
        .expect("IC-09: focused pane has active tab");
    assert_eq!(
        active_tab.pane_type,
        PaneType::LoomWikiPage,
        "IC-09: diagnostic chip opens the real rich-editor pane"
    );
    assert_eq!(
        active_tab.content_id.as_deref(),
        Some(note_doc_id.as_str()),
        "IC-09: diagnostic chip preserves the exact destination document id"
    );
    let ids = author_ids(&harness);
    assert!(
        ids.contains("editor.rich.text"),
        "IC-09: focused destination renders the real rich-editor AccessKit surface; got {ids:?}"
    );
    assert!(
        harness.root().children_recursive().any(|node| {
            node.accesskit_node().author_id() == Some("editor.rich.text")
                && node.accesskit_node().is_focused()
        }),
        "IC-09: diagnostic-chip navigation must transfer AccessKit focus to the rich editor"
    );

    let _ = be.delete(&format!("/knowledge/documents/{note_doc_id}"));
    let runtime_diagnostics = be
        .assert_cleanup_and_publish_runtime_diagnostics("IC-09")
        .expect("IC-09: publish fixture-owned backend runtime diagnostics");
    attempt.pass(serde_json::json!({
        "backend_binding": backend_binding,
        "runtime_diagnostics": runtime_diagnostics,
        "diagnostic_chip_author_id": chip_id,
        "document_id": note_doc_id,
        "destination_surface": "editor.rich.text",
        "focused_pane": active_pane.as_ref(),
    }));
    assert_no_local_artifact_dir();
    println!(
        "IC-09 PASS: AccessKit diagnostic note chip opened {note_doc_id} in the focused rich editor"
    );
}

fn finish_supplemental_mt046_argus(
    scenario_id: &str,
    argus: CanonicalArgusDriver,
    harness: &mut Harness<'_, HandshakeApp>,
    initial: serde_json::Value,
    terminal: serde_json::Value,
    scenario_evidence: serde_json::Value,
) {
    let proof_dir = supplemental_mt046_tree_dir(scenario_id);
    let workspace_ids = scenario_evidence
        .get("workspace_ids")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let envelope = serde_json::json!({
        "schema_id": "hsk.mt046.scenario-evidence@1",
        "run_id": required_mt046_env("HANDSHAKE_ARGUS_MATRIX_RUN_ID"),
        "scenario_id": scenario_id,
        "source_sha": required_mt046_env("HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA"),
        "process_correlation_id": required_mt046_env("HANDSHAKE_PROOF_PROCESS_CORRELATION_ID"),
        "workspace_ids": workspace_ids,
        "evidence": scenario_evidence,
    });
    let _ = harness.render_proof_frame(&format!("{scenario_id} mounted terminal frame"));
    assert!(harness.last_screenshot_outcome().is_some());
    argus.finish_require_no_indeterminate();
    write_immutable_json(&proof_dir.join("initial-tree.json"), &initial);
    write_immutable_json(&proof_dir.join("terminal-tree.json"), &terminal);
    write_immutable_json(&proof_dir.join("scenario-evidence.json"), &envelope);
    assert_no_local_artifact_dir();
}

fn required_mt046_env(name: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| panic!("MT-046 supplemental Argus proof requires {name}"))
}

fn supplemental_mt046_tree_dir(scenario_id: &str) -> PathBuf {
    PathBuf::from(required_mt046_env("HANDSHAKE_PROOF_ARTIFACT_DIR"))
        .join(required_mt046_env("HANDSHAKE_ARGUS_MATRIX_RUN_ID"))
        .join("trees")
        .join(scenario_id)
}

fn write_immutable_json(path: &std::path::Path, value: &serde_json::Value) {
    use sha2::{Digest as _, Sha256};
    use std::io::Write as _;

    let parent = path.parent().expect("MT-046 evidence path has a parent");
    std::fs::create_dir_all(parent).expect("create MT-046 unified tree directory");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap_or_else(|error| {
            panic!(
                "create immutable MT-046 evidence {}: {error}",
                path.display()
            )
        });
    let mut bytes = serde_json::to_vec_pretty(value).expect("serialize MT-046 evidence");
    bytes.push(b'\n');
    file.write_all(&bytes).unwrap_or_else(|error| {
        panic!(
            "write immutable MT-046 evidence {}: {error}",
            path.display()
        )
    });
    file.sync_all().unwrap_or_else(|error| {
        panic!("sync immutable MT-046 evidence {}: {error}", path.display())
    });
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let digest_path = path.with_extension("json.sha256");
    let mut digest_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&digest_path)
        .unwrap_or_else(|error| {
            panic!(
                "create immutable MT-046 evidence digest {}: {error}",
                digest_path.display()
            )
        });
    digest_file
        .write_all(
            format!(
                "{digest}  {}\n",
                path.file_name().unwrap().to_string_lossy()
            )
            .as_bytes(),
        )
        .unwrap_or_else(|error| {
            panic!(
                "write immutable MT-046 evidence digest {}: {error}",
                digest_path.display()
            )
        });
    digest_file.sync_all().unwrap_or_else(|error| {
        panic!(
            "sync immutable MT-046 evidence digest {}: {error}",
            digest_path.display()
        )
    });
}

fn write_supplemental_mt046_workspace_receipt(
    scenario_id: &str,
    workspace_id: &str,
    backend_binding: &serde_json::Value,
    runtime_diagnostics: &serde_json::Value,
) {
    let run_id = required_mt046_env("HANDSHAKE_ARGUS_MATRIX_RUN_ID");
    let receipt = serde_json::json!({
        "schema_id": "hsk.mt046.workspace-binding@1",
        "run_id": run_id.clone(),
        "scenario_id": scenario_id,
        "source_sha": required_mt046_env("HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA"),
        "process_id": std::process::id(),
        "process_correlation_id": required_mt046_env("HANDSHAKE_PROOF_PROCESS_CORRELATION_ID"),
        "workspace_id": workspace_id,
        "backend_binding": backend_binding,
        "runtime_diagnostics": runtime_diagnostics,
    });
    let path = external_artifact_dir("canonical-argus")
        .join(run_id)
        .join(scenario_id)
        .join("workspace.json");
    write_immutable_json(&path, &receipt);
    write_immutable_json(
        &supplemental_mt046_tree_dir(scenario_id).join("backend-runtime-diagnostics.json"),
        &serde_json::json!({
            "schema_id": "hsk.mt046.backend-runtime-diagnostics@1",
            "run_id": required_mt046_env("HANDSHAKE_ARGUS_MATRIX_RUN_ID"),
            "scenario_id": scenario_id,
            "source_sha": required_mt046_env("HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA"),
            "process_id": std::process::id(),
            "process_correlation_id": required_mt046_env("HANDSHAKE_PROOF_PROCESS_CORRELATION_ID"),
            "workspace_id": workspace_id,
            "backend_binding": backend_binding,
            "runtime_diagnostics": runtime_diagnostics,
        }),
    );
}

fn json_has_author_id_prefix(value: &serde_json::Value, prefix: &str) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            object
                .get("author_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|author_id| author_id.starts_with(prefix))
                || object
                    .values()
                    .any(|child| json_has_author_id_prefix(child, prefix))
        }
        serde_json::Value::Array(children) => children
            .iter()
            .any(|child| json_has_author_id_prefix(child, prefix)),
        _ => false,
    }
}

#[test]
#[ignore = "run only by MT-046 canonical supervisor with per-process matrix metadata"]
fn supplemental_mt046_argus_ic06_note_to_code() {
    let mut be = require_live_backend();
    let workspace_id = be.workspace_id.clone();
    let backend_binding = be.owned_backend_binding_receipt();
    let fixture = indexed_source_fixture();
    let index = be.post_json(
        &format!("/workspaces/{}/code-nav/index", be.workspace_id),
        &serde_json::json!({"root_path": fixture.root.to_string_lossy()}),
    );
    assert!(index["symbol_count"].as_u64().unwrap_or(0) >= 1);
    let literal_ref = "ic06_fixture_src/lib.rs#my_function".to_owned();
    let chip_id = code_ref_chip_author_id(&literal_ref);
    let (mut app, runtime) = editor_shell();
    app.set_backend_base_url_for_test(&be.base, runtime.handle().clone());
    app.bind_active_project_for_integration_test(be.workspace_id.clone());
    // The backend returns the canonical repo-relative symbol path. Anchor the mounted code panel in
    // this exact indexed fixture root so the product resolver can load, paint, and acknowledge the
    // real definition instead of searching from the test process working directory.
    app.mounted_code_panel()
        .set_file_path(fixture.root.join("README.md").to_string_lossy());
    app.set_active_pane_for_test(Some(PaneId::from("pane-b")));
    let mut paragraph = BlockNode::new(NodeKind::Paragraph);
    paragraph.children.push(Child::HsLink(HsLinkNode::new(
        "code",
        literal_ref,
        "my_function",
    )));
    app.mounted_rich_state().lock().unwrap().doc = BlockNode::doc(vec![paragraph]);
    let mut harness = Harness::builder()
        .proof_mt_id("MT-046")
        .with_size(egui::vec2(1100.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);
    let mut argus = CanonicalArgusDriver::bind(harness.state(), "mt046-ic06");
    let initial = argus.inspect(&mut harness);
    assert!(json_has_author_id(&initial, &chip_id));
    argus.click_expect_applied_and_reinspect(&mut harness, &chip_id);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline
        && !author_ids(&harness)
            .iter()
            .any(|id| id.starts_with(CODE_EDITOR_TEXT_AUTHOR_ID))
    {
        harness.step();
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let terminal =
        argus.assert_latest_terminal_predicate(&mut harness, "code-editor-mounted", |tree| {
            tree.to_string().contains(CODE_EDITOR_TEXT_AUTHOR_ID)
        });
    finish_supplemental_mt046_argus(
        "IC-06",
        argus,
        &mut harness,
        initial,
        terminal,
        serde_json::json!({
            "workspace_ids": [workspace_id.clone()],
            "backend_binding": backend_binding.clone(),
            "target": chip_id,
            "terminal_surface": CODE_EDITOR_TEXT_AUTHOR_ID,
        }),
    );
    let runtime_diagnostics = be
        .assert_cleanup_and_publish_runtime_diagnostics("IC-06")
        .expect("IC-06 supplemental: publish fixture-owned backend runtime diagnostics");
    write_supplemental_mt046_workspace_receipt(
        "IC-06",
        &workspace_id,
        &backend_binding,
        &runtime_diagnostics,
    );
}

#[test]
#[ignore = "run only by MT-046 canonical supervisor with per-process matrix metadata"]
fn supplemental_mt046_argus_ic07_copy_note_reference() {
    use handshake_native::code_editor::Cursor;
    let (mut app, _runtime) = editor_shell();
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
    let panel = app.mounted_code_panel();
    panel.set_text("fn my_function() {}\n");
    panel.set_cursors(vec![Cursor::selection(3, 14)]);
    let mut harness = Harness::builder()
        .proof_mt_id("MT-046")
        .with_size(egui::vec2(1100.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);
    let mut argus = CanonicalArgusDriver::bind(harness.state(), "mt046-ic07");
    let initial = argus.inspect(&mut harness);
    argus.click_expect_applied_and_reinspect(&mut harness, "code_editor_ctx_rename_symbol");
    let copy_id = format!("ctx-menu.{CODE_EDITOR_CTX_COPY_NOTE_REF_AUTHOR_ID}");
    argus.assert_latest_terminal_predicate(&mut harness, "disabled-copy-visible", |tree| {
        json_has_author_id(tree, &copy_id)
    });
    argus.click_expect_typed_rejected_and_reinspect(&mut harness, &copy_id, "disabled");
    argus.assert_latest_terminal_predicate(&mut harness, "unsaved-copy-rejected", |tree| {
        tree["action_receipts"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row["status"] == "rejected"))
    });
    harness.key_press(egui::Key::Escape);
    harness.run_steps(2);
    panel.set_file_path("src/lib.rs");
    harness.run_steps(2);
    argus.click_expect_applied_and_reinspect(&mut harness, "code_editor_ctx_rename_symbol");
    let stale_copy_snapshot = argus.assert_latest_terminal_predicate(
        &mut harness,
        "copy-item-enabled-before-stale-hide",
        |tree| json_has_author_id(tree, &copy_id),
    );
    let clipboard_before_stale = InteractionBus::get_or_init(&harness.ctx)
        .lock()
        .unwrap()
        .clipboard_read_text();
    harness.key_press(egui::Key::Escape);
    harness.run_steps(2);
    let never_started_raw = argus.click_from_snapshot_expect_rpc_rejected(
        &mut harness,
        &copy_id,
        &stale_copy_snapshot,
        "no live widget",
    );
    let clipboard_after_stale = InteractionBus::get_or_init(&harness.ctx)
        .lock()
        .unwrap()
        .clipboard_read_text();
    assert_eq!(
        clipboard_after_stale, clipboard_before_stale,
        "stale-hidden copy action must be rejected before clipboard mutation"
    );
    let process_correlation_id = required_mt046_env("HANDSHAKE_PROOF_PROCESS_CORRELATION_ID");
    let never_started = serde_json::json!({
        "schema_id": "hsk.mt046.argus-never-started@1",
        "run_id": required_mt046_env("HANDSHAKE_ARGUS_MATRIX_RUN_ID"),
        "scenario_id": "IC-07",
        "source_sha": required_mt046_env("HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA"),
        "process_id": std::process::id(),
        "process_correlation_id": process_correlation_id.clone(),
        "correlation_id": format!("{process_correlation_id}:never-started:{copy_id}"),
        "target": copy_id.clone(),
        "error": never_started_raw["response"]["error"].clone(),
        "response": never_started_raw["response"].clone(),
        "before": never_started_raw["before"].clone(),
        "after": never_started_raw["after"].clone(),
        "state_unchanged": {
            "clipboard_before": clipboard_before_stale.clone(),
            "clipboard_after": clipboard_after_stale.clone(),
            "equal": true,
            "file_path": panel.file_path(),
            "selection": "my_function",
        },
        "canonical_stale_snapshot": never_started_raw,
    });
    argus.click_expect_applied_and_reinspect(&mut harness, "code_editor_ctx_rename_symbol");
    argus.assert_latest_terminal_predicate(&mut harness, "copy-item-enabled", |tree| {
        json_has_author_id(tree, &copy_id)
    });
    let copy_observation = argus.click_expect_applied_and_reinspect(&mut harness, &copy_id);
    harness.run_steps(1);
    let expected_clipboard = "[[code:src/lib.rs#my_function]]";
    let clipboard_after_copy = InteractionBus::get_or_init(&harness.ctx)
        .lock()
        .unwrap()
        .clipboard_read_text();
    assert_eq!(clipboard_after_copy.as_deref(), Some(expected_clipboard));
    let copy_receipt_id = copy_observation.receipt_id;
    let terminal = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "exact-copy-receipt-clipboard-menu-closed",
        serde_json::json!({
            "receipt_id": copy_receipt_id,
            "clipboard": expected_clipboard,
            "context_menu_prefix": "ctx-menu.",
            "context_menu_closed": true,
        }),
        |tree| {
            tree["action_receipts"].as_array().is_some_and(|rows| {
                rows.iter().any(|row| {
                    row["receipt_id"].as_u64() == Some(copy_receipt_id)
                        && row["status"] == "applied"
                })
            }) && clipboard_after_copy.as_deref() == Some(expected_clipboard)
                && !json_has_author_id_prefix(tree, "ctx-menu.")
        },
    );
    finish_supplemental_mt046_argus(
        "IC-07",
        argus,
        &mut harness,
        initial,
        terminal,
        serde_json::json!({
            "workspace_ids": [],
            "successful_target": copy_id,
            "successful_receipt_id": copy_receipt_id,
            "clipboard": expected_clipboard,
            "never_started_artifact": "never-started.json",
        }),
    );
    write_immutable_json(
        &supplemental_mt046_tree_dir("IC-07").join("never-started.json"),
        &never_started,
    );
    assert_no_local_artifact_dir();
}

#[test]
#[ignore = "run only by MT-046 canonical supervisor with per-process matrix metadata"]
fn supplemental_mt046_argus_ic08_shared_find() {
    let mut be = require_live_backend();
    let workspace_id = be.workspace_id.clone();
    let backend_binding = be.owned_backend_binding_receipt();
    let probe = format!("MT046_ARGUS_SHARED_FIND_{}", uuid::Uuid::new_v4().simple());
    let code_source = format!("// {probe}\nfn mt046_argus_shared_find() {{}}\n");
    let code_created = be.post_json(
        &format!("/workspaces/{workspace_id}/loom/import"),
        &serde_json::json!({
            "bytes_b64": base64::engine::general_purpose::STANDARD.encode(code_source),
            "original_filename": format!("mt046_{probe}.rs"),
            "mime": "text/x-rust",
        }),
    );
    let code_block_id = code_created["block_id"]
        .as_str()
        .or_else(|| code_created["id"].as_str())
        .expect("IC-08 Argus: managed code Loom block id")
        .to_owned();
    let note_created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({
            "workspace_id": workspace_id,
            "title": "IC-08 Argus managed rich note",
            "content_json": {
                "type": "doc",
                "content": [{
                    "type": "paragraph",
                    "content": [{"type": "text", "text": format!("managed note {probe}")}]
                }]
            }
        }),
    );
    let note_document_id = note_created["document"]["rich_document_id"]
        .as_str()
        .expect("IC-08 Argus: managed rich-document id")
        .to_owned();
    let seeded_search = be.get_json(&format!(
        "/workspaces/{workspace_id}/loom/graph-search?q={probe}&limit=500&offset=0"
    ));
    let seeded_hits = seeded_search
        .as_array()
        .expect("IC-08 Argus: graph-search seed precondition returns an array");
    let code_source_kind = seeded_hits
        .iter()
        .find(|hit| hit["ref_id"].as_str() == Some(code_block_id.as_str()))
        .and_then(|hit| hit["source_kind"].as_str())
        .expect("IC-08 Argus: exact code Loom hit source kind")
        .to_owned();
    let note_source_kind = seeded_hits
        .iter()
        .find(|hit| hit["ref_id"].as_str() == Some(note_document_id.as_str()))
        .and_then(|hit| hit["source_kind"].as_str())
        .expect("IC-08 Argus: exact rich-note hit source kind")
        .to_owned();
    let code_result_id = result_author_id(&code_source_kind, &code_block_id);
    let note_result_id = result_author_id(&note_source_kind, &note_document_id);

    let (mut app, runtime) = editor_shell();
    app.set_backend_base_url_for_test(&be.base, runtime.handle().clone());
    app.bind_active_project_for_integration_test(workspace_id.clone());
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
    app.mounted_rich_state().lock().unwrap().doc = BlockNode::doc(vec![BlockNode::paragraph(
        &format!("mounted note contains {probe}"),
    )]);
    let mut harness = Harness::builder()
        .proof_mt_id("MT-046")
        .with_size(egui::vec2(1100.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    let ctx = harness.ctx.clone();
    assert!(harness
        .state_mut()
        .dispatch_palette_action_for_test_with_ctx(
            &ctx,
            handshake_native::command_registry::CMD_EDITOR_FIND_IN_FILES,
        ));
    harness.run_steps(2);
    let mut argus = CanonicalArgusDriver::bind(harness.state(), "mt046-ic08");
    let initial = argus.inspect(&mut harness);
    assert!(json_has_author_id(&initial, QUERY_AUTHOR_ID));
    assert!(json_has_author_id(&initial, SEARCH_AUTHOR_ID));
    let set_query = argus.set_value_and_reinspect(&mut harness, QUERY_AUTHOR_ID, &probe);
    assert_eq!(
        set_query.receipt_status, "applied",
        "IC-08 Argus query SetValue must be decisively Applied"
    );
    let set_query_receipt_id = set_query.receipt_id;
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "exact-shared-find-query-applied",
        serde_json::json!({"query":probe,"receipt_id":set_query_receipt_id}),
        |tree| {
            json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(probe.as_str())
                && tree["action_receipts"].as_array().is_some_and(|receipts| {
                    receipts.iter().any(|receipt| {
                        receipt["receipt_id"].as_u64() == Some(set_query_receipt_id)
                            && receipt["status"] == "applied"
                    })
                })
        },
    );
    let search = argus.click_expect_applied_and_reinspect(&mut harness, SEARCH_AUTHOR_ID);
    let search_receipt_id = search.receipt_id;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        harness.run_steps(1);
        let fresh = argus.inspect(&mut harness);
        let exact_results_present = json_has_author_id(&fresh, &code_result_id)
            && json_has_author_id(&fresh, &note_result_id)
            && json_node_by_author_id(&fresh, QUERY_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(probe.as_str());
        if exact_results_present {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "IC-08 Argus: exact code/rich result nodes did not appear; code={code_result_id}, note={note_result_id}, fresh={fresh}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let terminal = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "exact-code-and-rich-results-visible",
        serde_json::json!({
            "query": probe,
            "search_receipt_id": search_receipt_id,
            "code": {"source_kind":code_source_kind,"ref_id":code_block_id,"author_id":code_result_id},
            "note": {"source_kind":note_source_kind,"ref_id":note_document_id,"author_id":note_result_id},
        }),
        |tree| {
            json_has_author_id(tree, &code_result_id)
                && json_has_author_id(tree, &note_result_id)
                && json_node_by_author_id(tree, QUERY_AUTHOR_ID)
                    .and_then(|node| node.get("value"))
                    .and_then(serde_json::Value::as_str)
                    == Some(probe.as_str())
                && tree["action_receipts"]
                    .as_array()
                    .is_some_and(|receipts| {
                        receipts.iter().any(|receipt| {
                            receipt["receipt_id"].as_u64() == Some(search_receipt_id)
                                && receipt["status"] == "applied"
                        })
                    })
        },
    );
    finish_supplemental_mt046_argus(
        "IC-08",
        argus,
        &mut harness,
        initial,
        terminal,
        serde_json::json!({
            "workspace_ids": [workspace_id],
            "backend_binding": backend_binding.clone(),
            "query": probe,
            "set_value_receipt_id": set_query_receipt_id,
            "search_receipt_id": search_receipt_id,
            "code_result_author_id": code_result_id,
            "note_result_author_id": note_result_id,
            "code_block_id": code_block_id,
            "note_document_id": note_document_id,
        }),
    );
    let runtime_diagnostics = be
        .assert_cleanup_and_publish_runtime_diagnostics("IC-08")
        .expect("IC-08 supplemental: publish fixture-owned backend runtime diagnostics");
    write_supplemental_mt046_workspace_receipt(
        "IC-08",
        &workspace_id,
        &backend_binding,
        &runtime_diagnostics,
    );
}

#[test]
#[ignore = "run only by MT-046 canonical supervisor with per-process matrix metadata"]
fn supplemental_mt046_argus_ic09_diagnostic_to_note() {
    let mut be = require_live_backend();
    let workspace_id = be.workspace_id.clone();
    let backend_binding = be.owned_backend_binding_receipt();
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({
            "workspace_id": be.workspace_id.clone(),
            "title": "IC-09 Argus destination",
            "content_json": {"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Argus diagnostic destination"}]}]}
        }),
    );
    let doc_id = created["document"]["rich_document_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (mut app, runtime) = editor_shell();
    app.set_backend_base_url_for_test(&be.base, runtime.handle().clone());
    app.bind_active_project_for_integration_test(be.workspace_id.clone());
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
    let panel = app.mounted_code_panel();
    panel.set_text("fn main() { missing(); }\n");
    panel.push_diagnostic_note_reference(0, DiagnosticSeverity::Error, "related note", &doc_id);
    let chip_id = panel.diagnostic_note_reference_author_id(0);
    let mut harness = Harness::builder()
        .proof_mt_id("MT-046")
        .with_size(egui::vec2(1100.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);
    let mut argus = CanonicalArgusDriver::bind(harness.state(), "mt046-ic09");
    let initial = argus.inspect(&mut harness);
    assert!(json_has_author_id(&initial, &chip_id));
    let open_note = argus.click_expect_applied_and_reinspect(&mut harness, &chip_id);
    let open_receipt_id = open_note.receipt_id;
    let rich_document_author_id = format!("rich-editor.document.{doc_id}");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        harness.run_steps(1);
        let active_tab_matches = harness.state().active_pane().is_some_and(|pane_id| {
            harness
                .state()
                .tab_bar_states()
                .get(pane_id)
                .and_then(|bar| bar.active())
                .is_some_and(|tab| {
                    tab.pane_type == PaneType::LoomWikiPage
                        && tab.content_id.as_deref() == Some(doc_id.as_str())
                })
        });
        let rich_text_focused = harness.root().children_recursive().any(|node| {
            node.accesskit_node().author_id() == Some("editor.rich.text")
                && node.accesskit_node().is_focused()
        });
        let ids = author_ids(&harness);
        if active_tab_matches
            && rich_text_focused
            && ids.contains("editor.rich.text")
            && ids.contains(rich_document_author_id.as_str())
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "IC-09 Argus: exact document/tab/focus state did not mount; doc={doc_id}, ids={ids:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let active_pane_id = harness
        .state()
        .active_pane()
        .expect("IC-09 Argus: exact destination pane is active")
        .to_string();
    let active_tab_content_id = harness
        .state()
        .tab_bar_states()
        .get(harness.state().active_pane().unwrap())
        .and_then(|bar| bar.active())
        .and_then(|tab| tab.content_id.clone())
        .expect("IC-09 Argus: exact destination tab carries content_id");
    let rich_text_focused = harness.root().children_recursive().any(|node| {
        node.accesskit_node().author_id() == Some("editor.rich.text")
            && node.accesskit_node().is_focused()
    });
    let terminal = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "exact-scoped-rich-document-active-tab-focused",
        serde_json::json!({
            "receipt_id": open_receipt_id,
            "document_author_id": rich_document_author_id,
            "document_id": doc_id,
            "active_pane_id": active_pane_id,
            "active_tab_content_id": active_tab_content_id,
            "rich_text_author_id": "editor.rich.text",
            "rich_text_focused": rich_text_focused,
        }),
        |tree| {
            json_node_by_author_id(tree, &rich_document_author_id)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some(doc_id.as_str())
                && json_has_author_id(tree, "editor.rich.text")
                && active_tab_content_id == doc_id
                && rich_text_focused
                && tree["action_receipts"].as_array().is_some_and(|receipts| {
                    receipts.iter().any(|receipt| {
                        receipt["receipt_id"].as_u64() == Some(open_receipt_id)
                            && receipt["status"] == "applied"
                    })
                })
        },
    );
    finish_supplemental_mt046_argus(
        "IC-09",
        argus,
        &mut harness,
        initial,
        terminal,
        serde_json::json!({
            "workspace_ids": [workspace_id.clone()],
            "backend_binding": backend_binding.clone(),
            "document_id": doc_id,
            "document_author_id": rich_document_author_id,
            "active_pane_id": active_pane_id,
            "active_tab_content_id": active_tab_content_id,
            "rich_text_focused": rich_text_focused,
            "receipt_id": open_receipt_id,
        }),
    );
    let _ = be.delete(&format!("/knowledge/documents/{doc_id}"));
    let runtime_diagnostics = be
        .assert_cleanup_and_publish_runtime_diagnostics("IC-09")
        .expect("IC-09 supplemental: publish fixture-owned backend runtime diagnostics");
    write_supplemental_mt046_workspace_receipt(
        "IC-09",
        &workspace_id,
        &backend_binding,
        &runtime_diagnostics,
    );
}

// ── Hygiene guard (runs in the default suite). ────────────────────────────────────────────────────────

#[test]
fn no_local_artifact_dir_edge2() {
    assert_no_local_artifact_dir();
    println!("CX-212E: no repo-local artifact dir under the crate (edge 2)");
}
