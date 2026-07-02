//! WP-KERNEL-012 MT-070 wave-2 — the LIVE code-editor-body context-menu call site.
//!
//! The MT-070 typed layer (`context_menu_surfaces::editor_body_context_items` /
//! `editor_body_action_for_id`) was previously proven only against a synthetic right-clickable
//! surface (`test_context_menu_surfaces.rs`) — the LIVE `CodeEditorPanel` still showed its old
//! 2-entry inline menu. Wave-2 wired the panel call site
//! (`code_editor/panel.rs::render_editor_context_menu`) to the typed layer; these tests prove the
//! PRODUCT surface end-to-end in kittest:
//!
//! - right-clicking the REAL editor body opens the typed menu with ALL FIVE MT-070 required entries
//!   (Rename Symbol / Quick Fix / Format Selection / Peek Definition / Create note from link) as
//!   live `Role::MenuItem` nodes carrying the stable `ctx-menu.{author_id}` ids, PLUS the MT-046
//!   'Copy as note reference' entry;
//! - activating 'Rename Symbol' from the open menu fires the REAL F2 path (the inline rename input
//!   opens — a live panel-state side effect, not a captured enum);
//! - activating 'Copy as note reference' stages the REAL `[[code:path#anchor]]` ref (MT-046);
//! - activating 'Create note from link' stages the REAL MT-057 create-note intent for the
//!   `[[title]]` under the caret.
//!
//! No artifacts are written; the proofs are AccessKit-tree + panel-state assertions.

use std::sync::Arc;

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::code_editor::panel::CODE_EDITOR_CONTEXT_SURFACE_AUTHOR_ID;
use handshake_native::code_editor::{
    CodeEditorPanel, Cursor, CODE_EDITOR_CTX_COPY_NOTE_REF_AUTHOR_ID,
};

/// A snippet with (a) a renameable identifier, (b) selectable text, and (c) a `[[wikilink]]` in a
/// comment — one live target per menu entry under test.
const SNIPPET: &str = "fn my_function() {\n    let value = 1;\n}\n// see [[Design Notes]]\n";

/// The five MT-070 required editor-body entry ids (the EXACT stable author_ids the owning MTs emit;
/// mirrored from `context_menu_surfaces::EDITOR_BODY_REQUIRED_IDS` — asserting the literals here
/// also pins the no-parallel-id-scheme contract at the product call site).
const REQUIRED_IDS: &[&str] = &[
    "code_editor_ctx_rename_symbol",
    "code_editor_ctx_quick_fix",
    "code_editor_ctx_format_selection",
    "code_editor_hover_gotodef",
    "ctxmenu-editor-create-note",
];

fn harness_for(panel: Arc<CodeEditorPanel>) -> Harness<'static> {
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(760.0, 320.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    harness.run();
    harness
}

/// Right-click the editor body's context surface (the live `ui.interact` response the panel opens
/// its context menu on) and settle two frames so the popup opens.
fn open_editor_body_menu(harness: &mut Harness<'_>) {
    harness
        .root()
        .children_recursive()
        .find(|n| {
            n.accesskit_node().author_id().as_deref() == Some(CODE_EDITOR_CONTEXT_SURFACE_AUTHOR_ID)
        })
        .expect("the editor-body context surface node is live")
        .click_secondary();
    harness.run();
    harness.run();
}

/// Every live author-id node: (author_id, role).
fn live_author_nodes(harness: &Harness<'_>) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((author_id.to_owned(), format!("{:?}", ak.role())));
        }
    }
    found
}

/// Click the OPEN menu's item node by its full `ctx-menu.{id}` author id (unambiguous — the
/// always-present hidden swarm nodes carry the UN-prefixed ids).
fn click_menu_item(harness: &mut Harness<'_>, ctx_menu_author_id: &str) {
    harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id().as_deref() == Some(ctx_menu_author_id))
        .unwrap_or_else(|| panic!("open menu item {ctx_menu_author_id} is live"))
        .click();
    harness.run();
}

// ── The live panel's right-click shows the 5 MT-070 entries + the MT-046 entry ────────────────────

#[test]
fn live_editor_body_right_click_shows_five_typed_entries_plus_copy_ref() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    // Caret inside `my_function` so Rename Symbol has a live target (entries with no target render
    // DISABLED — still present + addressable, which is what this test asserts).
    let caret = SNIPPET.find("my_function").expect("snippet ident") + 3;
    panel.set_single_cursor(caret);
    let mut harness = harness_for(Arc::clone(&panel));

    // Closed by default: no ctx-menu.* items before the right-click.
    let closed = live_author_nodes(&harness);
    assert!(
        !closed.iter().any(|(a, _)| a.starts_with("ctx-menu.")),
        "no context-menu items before the right-click: {closed:?}"
    );

    open_editor_body_menu(&mut harness);

    let nodes = live_author_nodes(&harness);
    for required in REQUIRED_IDS {
        let want = format!("ctx-menu.{required}");
        let found = nodes
            .iter()
            .find(|(a, _)| a == &want)
            .unwrap_or_else(|| panic!("editor-body menu entry {want} missing: {nodes:?}"));
        assert_eq!(found.1, "MenuItem", "{want} role is MenuItem");
    }
    // The MT-046 entry rides the same menu.
    let copy_want = format!("ctx-menu.{CODE_EDITOR_CTX_COPY_NOTE_REF_AUTHOR_ID}");
    assert!(
        nodes.iter().any(|(a, r)| a == &copy_want && r == "MenuItem"),
        "the MT-046 'Copy as note reference' entry is a live MenuItem: {nodes:?}"
    );
    println!(
        "PASS MT-070 wave-2: the LIVE editor body right-click shows all 5 typed entries \
         (+ MT-046 copy-as-note-reference) as Role::MenuItem nodes"
    );
}

// ── Activating 'Rename Symbol' from the LIVE menu fires the real F2 path ──────────────────────────

#[test]
fn live_editor_body_menu_rename_opens_the_inline_rename_input() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    let caret = SNIPPET.find("my_function").expect("snippet ident") + 3;
    panel.set_single_cursor(caret);
    let mut harness = harness_for(Arc::clone(&panel));

    assert!(
        !panel.is_rename_input_open(),
        "rename input starts closed"
    );
    open_editor_body_menu(&mut harness);
    click_menu_item(&mut harness, "ctx-menu.code_editor_ctx_rename_symbol");

    assert!(
        panel.is_rename_input_open(),
        "activating the typed menu's 'Rename Symbol' entry fired the REAL begin_rename path \
         (the inline rename input opened — a live handler side effect)"
    );
    println!("PASS MT-070 wave-2: menu 'Rename Symbol' fired the real F2 handler on the live panel");
}

// ── Activating 'Copy as note reference' stages the REAL MT-046 ref ────────────────────────────────

#[test]
fn live_editor_body_menu_copy_as_note_reference_stages_the_real_ref() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    panel.set_file_path("src/demo.rs");
    // Select `my_function` (the anchor the ref is built from).
    let start = SNIPPET.find("my_function").expect("snippet ident");
    let end = start + "my_function".len();
    panel.set_cursors(vec![Cursor::selection(start, end)]);
    let mut harness = harness_for(Arc::clone(&panel));

    assert_eq!(
        panel.note_reference_for_cursor().as_deref(),
        Some("[[code:src/demo.rs#my_function]]"),
        "the live selection resolves to the MT-034-shaped code ref"
    );

    open_editor_body_menu(&mut harness);
    click_menu_item(
        &mut harness,
        &format!("ctx-menu.{CODE_EDITOR_CTX_COPY_NOTE_REF_AUTHOR_ID}"),
    );

    assert_eq!(
        panel.take_pending_copy_note_reference().as_deref(),
        Some("[[code:src/demo.rs#my_function]]"),
        "activating 'Copy as note reference' staged the REAL `[[code:…]]` ref (MT-046) — the \
         factory render writes it to the shared InteractionBus clipboard"
    );
    println!("PASS MT-046: menu 'Copy as note reference' staged [[code:src/demo.rs#my_function]]");
}

// ── Activating 'Create note from link' stages the REAL MT-057 intent ──────────────────────────────

#[test]
fn live_editor_body_menu_create_note_stages_the_link_title() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    // Caret ON the `[[Design Notes]]` link (inside the comment on the last line).
    let caret = SNIPPET.find("Design").expect("snippet link") + 2;
    panel.set_single_cursor(caret);
    let mut harness = harness_for(Arc::clone(&panel));

    assert_eq!(
        panel.wikilink_under_cursor().as_deref(),
        Some("Design Notes"),
        "the caret sits on the [[Design Notes]] wikilink"
    );

    open_editor_body_menu(&mut harness);
    click_menu_item(&mut harness, "ctx-menu.ctxmenu-editor-create-note");

    assert_eq!(
        panel.take_pending_create_note_link().as_deref(),
        Some("Design Notes"),
        "activating 'Create note from link' staged the REAL MT-057 create-note intent for the \
         link under the caret"
    );
    println!("PASS MT-070/057: menu 'Create note from link' staged the typed create-note intent");
}
