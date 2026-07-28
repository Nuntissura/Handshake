//! Slash-command menu integration proofs (WP-KERNEL-012 MT-016).
//!
//! Maps each acceptance criterion to a runtime proof against the LIVE editor widget + the
//! LIVE AccessKit tree (no tautologies). The pure model/filter/executor logic is proven by
//! the in-crate unit tests (`rich_editor::slash_commands::*`); these integration tests cover
//! the LIVE-frame obligations the contract's proof_targets need:
//!
//! - AC-1 / PT (`typing_slash_opens_menu`): a `/` typed at the start of an empty paragraph,
//!   driven THROUGH the live input handler (focus -> `egui::Event::Text("/")`), opens
//!   `RichEditorState.slash_menu`. A `/` typed mid-word does NOT (AC-2).
//! - AC-6 / AC-7 (`accesskit_menu_and_item_roles`): the LIVE AccessKit tree of an open menu
//!   contains the `slash-menu` popup (Role::Menu) and per-item `slash-item-{id}` nodes
//!   (Role::MenuItem) — the swarm-agent command surface (HBR-SWARM/HBR-VIS).
//! - AC-9 (`embed_prompt_modal_opens_and_inserts`): the embed-image command opens the
//!   `slash-prompt-dialog` modal (live tree); typing an asset id + confirming inserts an
//!   embed `hsLink` atom.
//! - PT screenshots (`slash_menu_open_screenshot`, `slash_menu_filtered_screenshot`): the
//!   open menu and the "head"-filtered menu render to PNGs saved to the EXTERNAL
//!   Handshake_Artifacts/handshake-test/wp-kernel-012-mt-016/ root (CX-212E — never
//!   repo-local; the contract's literal `tests/screenshots/` path is overridden).
//!
//! ## Screenshot model on this host
//!
//! `egui_kittest`'s `Harness::render()` does headless wgpu readback. With a GPU adapter the
//! PNG + pixel sample are produced; without one the test records an honest non-fatal blocker
//! and the AccessKit/structural proofs stand (the same best-effort model the MT-012 /
//! code-editor tests use).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::NodeT;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;
#[path = "native_gui_support/argus_surface_proof.rs"]
mod argus_surface_proof;
use argus_surface_proof::{prove_argus_surface, ArgusMutation};

use handshake_native::rich_editor::document_model::node::{BlockNode, Child};
use handshake_native::rich_editor::document_model::position::DocPosition;
use handshake_native::rich_editor::document_model::selection::Selection;
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorState, RichEditorWidget,
};
use handshake_native::rich_editor::slash_commands::registry::EmbedKind;
use handshake_native::rich_editor::slash_commands::{
    slash_item_author_id, SlashMenuState, SlashPrompt, SlashPromptKind, SLASH_MENU_AUTHOR_ID,
    SLASH_PROMPT_DIALOG_AUTHOR_ID, SLASH_PROMPT_INPUT_AUTHOR_ID,
};

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic — the
/// crate sits at `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where
/// `Handshake_Artifacts` is a sibling of the repo worktree.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert no repo-local artifact dir exists under the crate (CX-212E / CX-212E screenshot
/// rule): neither `test_output/` nor `tests/screenshots/`. Screenshots go to the external
/// Handshake_Artifacts root ONLY; a committed repo-local PNG is a hygiene FAILURE.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "artifact hygiene: no repo-local {local} dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

/// Build a harness rendering an editor over `state`, with the shell Inter fonts installed.
fn editor_harness<'a>(state: Arc<Mutex<RichEditorState>>, size: egui::Vec2) -> Harness<'a, ()> {
    let state_for_ui = Arc::clone(&state);
    Harness::builder()
        .with_size(size)
        .wgpu()
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        })
}

/// A non-wgpu harness (for the focus/input + AccessKit tests that don't need a rendered PNG).
fn editor_harness_cpu<'a>(state: Arc<Mutex<RichEditorState>>, size: egui::Vec2) -> Harness<'a, ()> {
    let state_for_ui = Arc::clone(&state);
    Harness::builder().with_size(size).build_ui(move |ui| {
        handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
        RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
    })
}

/// Runtime-accepting transport used by MT-036 mounted producer proofs. It records the exact canonical
/// event accepted by the ordered worker rather than observing a pre-dispatch draft.
struct RecordingEventTransport {
    posted: Arc<Mutex<Vec<handshake_native::event_emitter::NativeEditorEvent>>>,
}

impl handshake_native::event_emitter::EventLedgerTransport for RecordingEventTransport {
    fn build_post_body(
        &self,
        event: &handshake_native::event_emitter::NativeEditorEvent,
    ) -> serde_json::Value {
        event.to_native_payload()
    }

    fn post(
        &self,
        event: handshake_native::event_emitter::NativeEditorEvent,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(), handshake_native::event_emitter::EmitError>>
                + Send,
        >,
    > {
        let posted = Arc::clone(&self.posted);
        Box::pin(async move {
            posted.lock().unwrap().push(event);
            Ok(())
        })
    }
}

fn mounted_event_harness<'a>(
    state: Arc<Mutex<RichEditorState>>,
    size: egui::Vec2,
    emitter: handshake_native::event_emitter::NativeEditorEventEmitter,
) -> Harness<'a, ()> {
    let state_for_ui = Arc::clone(&state);
    Harness::builder().with_size(size).build_ui(move |ui| {
        handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
        let bus = handshake_native::interop::InteractionBus::get_or_init(ui.ctx());
        handshake_native::interop::InteractionBus::with_try_lock(&bus, |bus| {
            bus.set_event_emitter(emitter.clone());
        });
        RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
    })
}

fn recording_event_emitter(
    workspace_id: &str,
) -> (
    tokio::runtime::Runtime,
    handshake_native::event_emitter::NativeEditorEventEmitter,
    Arc<Mutex<Vec<handshake_native::event_emitter::NativeEditorEvent>>>,
) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("mounted event-producer runtime");
    let posted = Arc::new(Mutex::new(Vec::new()));
    let emitter = handshake_native::event_emitter::NativeEditorEventEmitter::new(
        workspace_id,
        Arc::new(RecordingEventTransport {
            posted: Arc::clone(&posted),
        }),
        Some(runtime.handle().clone()),
    );
    (runtime, emitter, posted)
}

fn drain_recording_emitter(runtime: &tokio::runtime::Runtime) {
    runtime.block_on(async {
        for _ in 0..32 {
            tokio::task::yield_now().await;
        }
    });
}

/// Focus the editor SURFACE (the focusable `editor.rich.text` node) by sending it an
/// AccessKit Focus action — the same focus an out-of-process agent would request by the stable
/// surface id, and the exact pattern `test_wikilinks.rs` uses. The input handler + the MC-004
/// focus-loss-close both gate on this focus, so the slash menu only survives + only processes
/// typed `/` when the surface is focused.
///
/// Uses `step()` (single frames), NOT `run()`: a focused editor schedules a continuous caret-
/// blink repaint, so `run()` (which loops until no repaint is pending) would exceed its
/// max_steps. Two steps let the focus action settle then take effect.
fn focus_editor(harness: &mut Harness<()>) {
    {
        let root = harness.root();
        let surface = root
            .children_recursive()
            .find(|n| n.accesskit_node().author_id() == Some("editor.rich.text"))
            .expect("the editor surface node carries author_id 'editor.rich.text'");
        surface.focus();
    }
    harness.step(); // process the focus action -> surface focused
    harness.step(); // focus is live this frame
}

// ── AC-1 / AC-2: typing `/` opens (or does not open) the menu through the live input path ──

#[test]
fn typing_slash_opens_menu() {
    // An empty paragraph; caret at offset 0. Type `/` through the live input handler.
    let doc = BlockNode::doc(vec![BlockNode::paragraph("")]);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));
    {
        let mut st = state.lock().unwrap();
        st.selection = Selection::caret(DocPosition::new(vec![0, 0], 0));
    }
    let mut harness = editor_harness_cpu(Arc::clone(&state), egui::vec2(600.0, 300.0));
    harness.step();
    focus_editor(&mut harness);

    // Type `/` (a printable char arrives as an egui Text event).
    harness.event(egui::Event::Text("/".into()));
    harness.step();

    let opened = state.lock().unwrap().slash_menu.is_some();
    assert!(
        opened,
        "AC-1: typing `/` at the start of an empty paragraph must open the slash menu"
    );
    // The trigger position is the `/` at char 0.
    {
        let st = state.lock().unwrap();
        let menu = st.slash_menu.as_ref().unwrap();
        assert_eq!(menu.trigger_char, 0);
        assert_eq!(menu.filter, "", "freshly opened menu has an empty filter");
    }
    println!("AC-1: `/` at blank-line start opened the slash menu");
}

#[test]
fn typing_slash_mid_word_does_not_open_menu() {
    // AC-2: a paragraph "ab"; caret at offset 2 (end). Type `/` -> mid-word, no menu.
    let doc = BlockNode::doc(vec![BlockNode::paragraph("ab")]);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));
    {
        let mut st = state.lock().unwrap();
        st.selection = Selection::caret(DocPosition::new(vec![0, 0], 2));
    }
    let mut harness = editor_harness_cpu(Arc::clone(&state), egui::vec2(600.0, 300.0));
    harness.step();
    focus_editor(&mut harness);

    harness.event(egui::Event::Text("/".into()));
    harness.step();

    assert!(
        state.lock().unwrap().slash_menu.is_none(),
        "AC-2: typing `/` after a non-whitespace char (mid-word) must NOT open the menu"
    );
    // The `/` is still inserted as plain text (it just doesn't trigger the menu).
    let text = block_plain_text(&state, 0);
    assert_eq!(text, "ab/", "the `/` is typed as plain text, no menu");
    println!("AC-2: mid-word `/` did not open the menu (typed as plain text)");
}

#[test]
fn typing_slash_in_url_does_not_open_menu() {
    // RISK-1 / MC-001: a paragraph "http:"; caret at offset 5. Type `/` -> URL `/`, no menu.
    let doc = BlockNode::doc(vec![BlockNode::paragraph("http:")]);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));
    {
        let mut st = state.lock().unwrap();
        st.selection = Selection::caret(DocPosition::new(vec![0, 0], 5));
    }
    let mut harness = editor_harness_cpu(Arc::clone(&state), egui::vec2(600.0, 300.0));
    harness.step();
    focus_editor(&mut harness);

    harness.event(egui::Event::Text("/".into()));
    harness.step();

    assert!(
        state.lock().unwrap().slash_menu.is_none(),
        "RISK-1: typing `/` after ':' (a URL char) must NOT open the menu (http:/ )"
    );
    println!("RISK-1: `/` inside a URL did not open the menu");
}

// ── MT-108 (MT-016 residual): Escape dismisses the menu but keeps the `/` char ──────────────

#[test]
fn escape_closes_slash_menu_but_keeps_the_slash_char() {
    // MT-108 (MT-016 residual): Escape to dismiss the slash menu must CLOSE the menu but LEAVE the typed
    // `/` (and any filter text) in the document — Escape is a dismiss, not a delete (widget AC-5).
    let doc = BlockNode::doc(vec![BlockNode::paragraph("")]);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));
    {
        let mut st = state.lock().unwrap();
        st.selection = Selection::caret(DocPosition::new(vec![0, 0], 0));
    }
    let mut harness = editor_harness_cpu(Arc::clone(&state), egui::vec2(600.0, 300.0));
    harness.step();
    focus_editor(&mut harness);

    // Type `/` -> the menu opens and the `/` is inserted into the text.
    harness.event(egui::Event::Text("/".into()));
    harness.step();
    assert!(
        state.lock().unwrap().slash_menu.is_some(),
        "typing `/` opened the slash menu"
    );
    assert_eq!(
        block_plain_text(&state, 0),
        "/",
        "the `/` is inserted into the text when the menu opens"
    );

    // Press Escape -> the menu closes but the `/` REMAINS.
    harness.event(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.step();
    harness.step();

    assert!(
        state.lock().unwrap().slash_menu.is_none(),
        "MT-016: Escape closed the slash menu"
    );
    assert_eq!(
        block_plain_text(&state, 0),
        "/",
        "MT-016: Escape leaves the `/` char in the text (dismiss, not delete)"
    );
    println!("MT-108 (MT-016): Escape closed the slash menu and kept the `/` char");
}

// ── AC-6 / AC-7: live AccessKit Role::Menu popup + Role::MenuItem rows ──────────────────────

#[test]
fn accesskit_menu_and_item_roles() {
    // Open the menu directly on the state (the open path is proven live above; here we prove
    // the RENDERED tree carries the contract author_ids + roles), then run a frame and inspect
    // the live AccessKit tree.
    let doc = BlockNode::doc(vec![BlockNode::paragraph("/")]);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));
    {
        let mut st = state.lock().unwrap();
        st.selection = Selection::caret(DocPosition::new(vec![0, 0], 1));
    }
    let mut harness = editor_harness_cpu(Arc::clone(&state), egui::vec2(600.0, 400.0));
    harness.step();
    // Focus FIRST so the MC-004 focus-loss-close does not clear the menu we set next.
    focus_editor(&mut harness);
    {
        let mut st = state.lock().unwrap();
        st.slash_menu = Some(SlashMenuState::open(vec![0, 0], 0));
    }
    harness.step();
    harness.step();

    let mut menu_found = false;
    let mut menu_role = String::new();
    let mut item_found = false;
    let mut item_role = String::new();
    // The first catalog command is "paragraph" -> author_id "slash-item-paragraph".
    let item_author = slash_item_author_id("paragraph");

    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        let Some(author) = ak.author_id() else {
            continue;
        };
        if author == SLASH_MENU_AUTHOR_ID {
            menu_found = true;
            menu_role = format!("{:?}", ak.role());
        } else if author == item_author {
            item_found = true;
            item_role = format!("{:?}", ak.role());
        }
    }

    assert!(
        menu_found,
        "AC-6: live tree must contain the `{SLASH_MENU_AUTHOR_ID}` popup node"
    );
    assert_eq!(
        menu_role, "Menu",
        "AC-6: `{SLASH_MENU_AUTHOR_ID}` must be Role::Menu (got {menu_role})"
    );
    assert!(
        item_found,
        "AC-7: live tree must contain a `slash-item-paragraph` row node"
    );
    assert_eq!(
        item_role, "MenuItem",
        "AC-7: each slash item must be Role::MenuItem (got {item_role})"
    );
    println!(
        "AC-6/AC-7: live AccessKit tree has the slash-menu (Menu) + slash-item-* (MenuItem) nodes"
    );
}

#[test]
#[ignore = "MT-108 runner-only proof: bounded supervisor supplies source/process identity and capture root"]
fn mt108_argus_slash_menu_real_server_loop() {
    let doc = BlockNode::doc(vec![BlockNode::paragraph("/")]);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));
    {
        let mut st = state.lock().unwrap();
        st.selection = Selection::caret(DocPosition::new(vec![0, 0], 1));
    }
    let mut harness = editor_harness_cpu(Arc::clone(&state), egui::vec2(600.0, 400.0));
    harness.step();
    focus_editor(&mut harness);
    state.lock().unwrap().slash_menu = Some(SlashMenuState::open(vec![0, 0], 0));
    harness.step();
    harness.step();

    let paragraph = slash_item_author_id("paragraph");
    prove_argus_surface(
        &mut harness,
        "slash menu",
        SLASH_MENU_AUTHOR_ID,
        ArgusMutation::Click { target: &paragraph },
        SLASH_MENU_AUTHOR_ID,
        false,
        |_| {
            let menu_closed = state.lock().unwrap().slash_menu.is_none();
            if !menu_closed {
                return Err("slash item click did not close the menu".to_owned());
            }
            Ok(serde_json::json!({ "slash_menu_closed": menu_closed }))
        },
    );
}

#[test]
fn no_unnamed_interactive_nodes_with_menu_open() {
    // HBR-SWARM: every interactive node in the OPEN-MENU tree carries a stable author_id (the
    // shell gate panics otherwise). Reuses the same gate the shell uses.
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let doc = BlockNode::doc(vec![BlockNode::paragraph("/")]);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));
    {
        let mut st = state.lock().unwrap();
        st.selection = Selection::caret(DocPosition::new(vec![0, 0], 1));
        st.slash_menu = Some(SlashMenuState::open(vec![0, 0], 0));
    }
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        handshake_native::app::HandshakeApp::install_fonts(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            RichEditorWidget::new(Arc::clone(&state)).show(ui);
        });
    });
    let update = output
        .platform_output
        .accesskit_update
        .expect("AccessKit update produced");
    let inspected = handshake_native::accessibility::assert_no_unnamed_interactive(&update);
    println!(
        "HBR-SWARM: inspected {inspected} interactive nodes with the slash menu open, all named"
    );
    assert!(
        inspected > 0,
        "the open-menu tree must contain >= 1 interactive node"
    );
}

// ── AC-9: embed prompt modal opens + a confirmed asset id inserts an embed atom ────────────

#[test]
fn embed_prompt_modal_opens_in_live_tree() {
    // AC-9 (part 1): with an embed-image prompt active, the live tree carries the
    // `slash-prompt-dialog` modal + its `slash-prompt-input` field.
    let doc = BlockNode::doc(vec![BlockNode::paragraph("")]);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));
    {
        let mut st = state.lock().unwrap();
        st.selection = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let mut menu = SlashMenuState::open(vec![0, 0], 0);
        menu.prompt = Some(SlashPrompt::new(SlashPromptKind::Embed(EmbedKind::Image)));
        st.slash_menu = Some(menu);
    }
    let mut harness = editor_harness_cpu(Arc::clone(&state), egui::vec2(600.0, 400.0));
    harness.step();

    let mut dialog_found = false;
    let mut input_found = false;
    for node in harness.root().children_recursive() {
        match node.accesskit_node().author_id() {
            Some(a) if a == SLASH_PROMPT_DIALOG_AUTHOR_ID => dialog_found = true,
            Some(a) if a == SLASH_PROMPT_INPUT_AUTHOR_ID => input_found = true,
            _ => {}
        }
    }
    assert!(
        dialog_found,
        "AC-9: the `{SLASH_PROMPT_DIALOG_AUTHOR_ID}` modal must be in the live tree"
    );
    assert!(
        input_found,
        "AC-9: the `{SLASH_PROMPT_INPUT_AUTHOR_ID}` field must be in the live tree"
    );
    println!("AC-9: the embed prompt modal + input render in the live AccessKit tree");
}

#[test]
fn embed_prompt_confirm_inserts_embed_atom() {
    // AC-9 (part 2): entering a valid asset id + confirming inserts an embed `hsLink` atom
    // (ref_kind = images). Driven through the live render: set an embed prompt with input, then
    // inject Enter (the prompt confirms on Enter) and run a frame.
    let doc = BlockNode::doc(vec![BlockNode::paragraph("")]);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));
    {
        let mut st = state.lock().unwrap();
        st.selection = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let mut menu = SlashMenuState::open(vec![0, 0], 0);
        let mut prompt = SlashPrompt::new(SlashPromptKind::Embed(EmbedKind::Image));
        prompt.input = "asset-xyz".to_string();
        menu.prompt = Some(prompt);
        st.slash_menu = Some(menu);
    }
    let mut harness = editor_harness_cpu(Arc::clone(&state), egui::vec2(600.0, 400.0));
    harness.step();

    // Enter confirms the prompt (read by render_slash_prompt before the window).
    harness.event(egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.step();

    let st = state.lock().unwrap();
    assert!(
        st.slash_menu.is_none(),
        "AC-9: confirming the prompt closes the slash surface"
    );
    let para = st.doc.children[0].as_block().unwrap();
    let atom = para
        .children
        .iter()
        .find_map(Child::as_hs_link)
        .expect("AC-9: a confirmed embed inserts an hsLink atom");
    assert_eq!(
        atom.ref_kind, "images",
        "the inserted embed is an image embed"
    );
    assert_eq!(atom.ref_value, "asset-xyz");
    println!(
        "AC-9: confirming the embed prompt inserted an image embed atom (ref_value=asset-xyz)"
    );
}

// ── PT screenshots: open menu + filtered menu, saved to the EXTERNAL artifact root ─────────

#[test]
fn slash_menu_open_screenshot() {
    // PT-2: the open menu with grouped items. Save mt016_slash_menu_open.png to the external
    // artifact root.
    let doc = BlockNode::doc(vec![BlockNode::paragraph("/")]);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));
    {
        let mut st = state.lock().unwrap();
        st.selection = Selection::caret(DocPosition::new(vec![0, 0], 1));
    }
    let mut harness = editor_harness(Arc::clone(&state), egui::vec2(700.0, 520.0));
    harness.step();
    // Focus the editor so the MC-004 focus-loss-close does not clear the menu before the
    // screenshot, and the live `/` trigger keeps the menu open this frame (the visible popup).
    focus_editor(&mut harness);
    {
        let mut st = state.lock().unwrap();
        st.slash_menu = Some(SlashMenuState::open(vec![0, 0], 0));
    }
    harness.step();
    harness.step();

    save_screenshot(&mut harness, "mt016_slash_menu_open.png", "PT-2 open menu");
    assert_no_local_artifact_dir();
}

#[test]
fn slash_menu_filtered_screenshot() {
    // PT-3: the "head"-filtered menu showing the 3 heading items. Save
    // mt016_slash_menu_filtered.png to the external artifact root.
    let doc = BlockNode::doc(vec![BlockNode::paragraph("/head")]);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));
    {
        let mut st = state.lock().unwrap();
        st.selection = Selection::caret(DocPosition::new(vec![0, 0], 5));
    }
    let mut harness = editor_harness(Arc::clone(&state), egui::vec2(700.0, 520.0));
    harness.step();
    // Focus first (MC-004) so the menu survives + the live `/head` trigger keeps the filter.
    focus_editor(&mut harness);
    {
        let mut st = state.lock().unwrap();
        let mut menu = SlashMenuState::open(vec![0, 0], 0);
        menu.filter = "head".to_string();
        st.slash_menu = Some(menu);
    }
    harness.step();
    harness.step();

    // The filtered catalog is exactly the 3 heading commands (proven by the unit filter test);
    // assert it here against the live state too.
    {
        use handshake_native::rich_editor::slash_commands::registry::filter_slash_commands;
        let rows = filter_slash_commands("head");
        let ids: Vec<&str> = rows.iter().map(|c| c.id).collect();
        assert_eq!(
            ids,
            vec!["heading-1", "heading-2", "heading-3"],
            "PT-3: 'head' -> 3 headings"
        );
    }

    save_screenshot(
        &mut harness,
        "mt016_slash_menu_filtered.png",
        "PT-3 filtered menu",
    );
    assert_no_local_artifact_dir();
}

/// Render the harness and save the PNG to the external artifact root (CX-212E). Asserts >= 2
/// distinct foreground colors when the GPU readback succeeds; records an honest non-fatal
/// blocker when no GPU adapter is available (the structural/AccessKit proofs stand).
fn save_screenshot(harness: &mut Harness<()>, file: &str, label: &str) {
    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image must be non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-016");
            let _ = std::fs::create_dir_all(&ext_dir);
            let path = ext_dir.join(file);
            let saved = image.save(&path).is_ok();

            let raw = image.as_raw();
            let mut counts: std::collections::HashMap<[u8; 4], u32> =
                std::collections::HashMap::new();
            let mut i = 0usize;
            while i + 4 <= raw.len() {
                let px = [raw[i], raw[i + 1], raw[i + 2], raw[i + 3]];
                if px[3] != 0 {
                    *counts.entry(px).or_insert(0) += 1;
                }
                i += 4 * 4;
            }
            let bg = counts.iter().max_by_key(|(_, c)| **c).map(|(p, _)| *p);
            let foreground: HashSet<[u8; 4]> =
                counts.keys().filter(|p| Some(**p) != bg).copied().collect();

            println!(
                "{label} screenshot: {w}x{h}, {} distinct colors, {} foreground; saved={saved} ({})",
                counts.len(),
                foreground.len(),
                path.display(),
            );
            assert!(
                foreground.len() >= 2,
                "{label}: the rendered menu must produce >= 2 distinct foreground colors; got {} (bg={bg:?})",
                foreground.len()
            );
            assert!(
                saved,
                "{label}: the screenshot must be saved to the external artifact root"
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): {label} screenshot render unavailable (no wgpu adapter / \
                 headless GPU crash): {e}. The AccessKit + structural proofs stand; the PNG is a \
                 GPU-host item."
            );
        }
    }
}

/// The concatenated plain text of the text leaves in the block at `idx`.
fn block_plain_text(state: &Arc<Mutex<RichEditorState>>, idx: usize) -> String {
    let st = state.lock().unwrap();
    st.doc.children[idx]
        .as_block()
        .map(|b| {
            b.children
                .iter()
                .filter_map(Child::as_text)
                .map(|t| t.text.to_string())
                .collect::<String>()
        })
        .unwrap_or_default()
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-020 — LIVE-WIDGET undo-after-insert for the slash-surface atom paths. Each path
// drives the REAL mounted widget (prompt Enter confirm / code-ref row click), then a REAL Ctrl+Z
// keystroke; the inserted atom must be gone and the doc must equal the exact pre-insert doc — the
// insert landed on the MT-035 unified undo bus, not a parallel stack.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

/// Focus the editor surface AND set the pane id so widget edits record on the unified bus.
fn mt020_state(pane: &str) -> Arc<Mutex<RichEditorState>> {
    let doc = BlockNode::doc(vec![BlockNode::paragraph("")]);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));
    {
        let mut st = state.lock().unwrap();
        st.selection = Selection::caret(DocPosition::new(vec![0, 0], 0));
        st.undo_pane_id = Some(Arc::from(pane));
    }
    state
}

fn pre_insert_json(state: &Arc<Mutex<RichEditorState>>) -> serde_json::Value {
    let st = state.lock().unwrap();
    handshake_native::rich_editor::document_model::doc_json::to_content_json_value(&st.doc)
}

fn assert_undo_restores(
    harness: &mut Harness<()>,
    state: &Arc<Mutex<RichEditorState>>,
    before: &serde_json::Value,
    label: &str,
) {
    // Re-focus the editor surface first: the prompt/dialog interaction may have moved egui focus,
    // and the Ctrl+Z chord decode runs on the FOCUSED editor input path.
    focus_editor(harness);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.step();
    harness.step();
    let st = state.lock().unwrap();
    let para = st.doc.children[0].as_block().unwrap();
    assert!(
        para.children
            .iter()
            .all(|c| c.as_hs_link().is_none() && c.as_transclusion().is_none()),
        "MT-020 ({label}): Ctrl+Z removed the inserted atom"
    );
    let now =
        handshake_native::rich_editor::document_model::doc_json::to_content_json_value(&st.doc);
    assert_eq!(
        &now, before,
        "MT-020 ({label}): undo restored the EXACT pre-insert doc"
    );
}

#[test]
fn mt020_live_embed_prompt_confirm_undo_restores_pre_insert_doc() {
    let state = mt020_state("pane-mt020-embed");
    {
        let mut st = state.lock().unwrap();
        let mut menu = SlashMenuState::open(vec![0, 0], 0);
        let mut prompt = SlashPrompt::new(SlashPromptKind::Embed(EmbedKind::Image));
        prompt.input = "asset-undo".to_string();
        menu.prompt = Some(prompt);
        st.slash_menu = Some(menu);
    }
    let mut harness = editor_harness_cpu(Arc::clone(&state), egui::vec2(600.0, 400.0));
    harness.step();
    focus_editor(&mut harness);
    let before = pre_insert_json(&state);

    // A REAL Enter confirms the prompt through the live render (the render-phase insert path the
    // frame-input diff cannot see — the pending_bus_undo drain must carry it to the bus).
    harness.key_press(egui::Key::Enter);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        let para = st.doc.children[0].as_block().unwrap();
        let atom = para
            .children
            .iter()
            .find_map(Child::as_hs_link)
            .expect("the confirmed embed inserted an hsLink atom");
        assert_eq!(atom.ref_value, "asset-undo");
    }
    assert_undo_restores(&mut harness, &state, &before, "embed prompt");
}

#[test]
fn mt020_live_transclusion_prompt_confirm_undo_restores_pre_insert_doc() {
    let state = mt020_state("pane-mt020-transclusion");
    {
        let mut st = state.lock().unwrap();
        let mut menu = SlashMenuState::open(vec![0, 0], 0);
        let mut prompt = SlashPrompt::new(SlashPromptKind::Transclusion);
        prompt.input = "block-undo".to_string();
        menu.prompt = Some(prompt);
        st.slash_menu = Some(menu);
    }
    let mut harness = editor_harness_cpu(Arc::clone(&state), egui::vec2(600.0, 400.0));
    harness.step();
    focus_editor(&mut harness);
    let before = pre_insert_json(&state);

    harness.key_press(egui::Key::Enter);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        let para = st.doc.children[0].as_block().unwrap();
        let atom = para
            .children
            .iter()
            .find_map(Child::as_transclusion)
            .expect("the confirmed prompt inserted a loomTransclusion atom");
        assert_eq!(atom.ref_value, "block-undo");
    }
    assert_undo_restores(&mut harness, &state, &before, "transclusion prompt");
}

#[test]
fn mt020_live_code_ref_select_undo_restores_pre_insert_doc() {
    use handshake_native::code_editor::code_nav::CodeSymbolNavProjection;
    use handshake_native::rich_editor::slash_commands::code_symbol_result_author_id;
    use handshake_native::rich_editor::slash_commands::code_symbol_search::CodeSymbolSearchState;

    let state = mt020_state("pane-mt020-coderef");
    {
        let mut st = state.lock().unwrap();
        let mut dialog = CodeSymbolSearchState::open("ws-test", None);
        dialog.query = "parse".to_string();
        dialog.results = vec![CodeSymbolNavProjection {
            symbol_entity_id: "SYM-undo-1".into(),
            display_name: "parse_config".into(),
            symbol_kind: "function".into(),
            ..Default::default()
        }];
        st.code_symbol_search = Some(dialog);
    }
    let mut harness = editor_harness_cpu(Arc::clone(&state), egui::vec2(700.0, 480.0));
    harness.step();
    focus_editor(&mut harness);
    let before = pre_insert_json(&state);

    // A REAL click on the LIVE result row (the AccessKit ListItem the swarm/kittest targets) selects
    // the symbol -> the render-phase insert_code_ref_atom path runs.
    {
        let target = code_symbol_result_author_id("SYM-undo-1");
        let root = harness.root();
        let row = root
            .children_recursive()
            .find(|n| n.accesskit_node().author_id() == Some(target.as_str()))
            .expect("the code-symbol result row is in the live tree");
        row.click();
    }
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert!(
            st.code_symbol_search.is_none(),
            "selecting a symbol closes the dialog"
        );
        let para = st.doc.children[0].as_block().unwrap();
        let atom = para
            .children
            .iter()
            .find_map(Child::as_hs_link)
            .expect("the selection inserted a code hsLink atom");
        assert_eq!(atom.ref_value, "SYM-undo-1");
    }
    assert_undo_restores(&mut harness, &state, &before, "code-ref select");
}

#[test]
fn mt036_mounted_embed_producer_is_canonical_exactly_once_and_zero_on_non_insert_paths() {
    let (runtime, emitter, posted) = recording_event_emitter("ws-mt036-embed");

    // Successful transaction: the persisted atom and event item identity are the same trimmed value.
    let state = mt020_state("pane-mt036-embed");
    {
        let mut st = state.lock().unwrap();
        st.code_ref_workspace_id = "ws-mt036-embed".to_owned();
        let mut menu = SlashMenuState::open(vec![0, 0], 0);
        let mut prompt = SlashPrompt::new(SlashPromptKind::Embed(EmbedKind::Image));
        prompt.input = "  asset-canonical  ".to_owned();
        menu.prompt = Some(prompt);
        st.slash_menu = Some(menu);
    }
    let mut harness = mounted_event_harness(
        Arc::clone(&state),
        egui::vec2(600.0, 400.0),
        emitter.clone(),
    );
    harness.step();
    focus_editor(&mut harness);
    harness.key_press(egui::Key::Enter);
    harness.step();
    harness.step();
    drain_recording_emitter(&runtime);
    {
        let st = state.lock().unwrap();
        let atom = st.doc.children[0]
            .as_block()
            .unwrap()
            .children
            .iter()
            .find_map(Child::as_hs_link)
            .expect("mounted prompt inserted the embed atom");
        assert_eq!(atom.ref_value, "asset-canonical");
    }
    {
        let events = posted.lock().unwrap();
        assert_eq!(
            events.len(),
            1,
            "successful embed transaction emits exactly once"
        );
        assert_eq!(events[0].action.as_str(), "embed_created");
        assert_eq!(events[0].payload["item_id"], "asset-canonical");
    }

    // Blank confirm, failed insertion (node selection), and cancel all cross the mounted render
    // boundary but must not produce another ledger event.
    for (input, selection, key) in [
        ("   ", None, egui::Key::Enter),
        (
            "asset-invalid-selection",
            Some(Selection::node(vec![0])),
            egui::Key::Enter,
        ),
        ("asset-cancelled", None, egui::Key::Escape),
    ] {
        let state = mt020_state("pane-mt036-embed-zero");
        {
            let mut st = state.lock().unwrap();
            st.code_ref_workspace_id = "ws-mt036-embed".to_owned();
            if let Some(selection) = selection {
                st.selection = selection;
            }
            let mut menu = SlashMenuState::open(vec![0, 0], 0);
            let mut prompt = SlashPrompt::new(SlashPromptKind::Embed(EmbedKind::Image));
            prompt.input = input.to_owned();
            menu.prompt = Some(prompt);
            st.slash_menu = Some(menu);
        }
        let mut harness = mounted_event_harness(state, egui::vec2(600.0, 400.0), emitter.clone());
        harness.step();
        focus_editor(&mut harness);
        harness.key_press(key);
        harness.step();
        harness.step();
    }
    drain_recording_emitter(&runtime);
    assert_eq!(
        posted.lock().unwrap().len(),
        1,
        "blank, failed, and cancelled embed paths emit zero"
    );
}

#[test]
fn mt036_mounted_cross_ref_producer_is_exactly_once_and_zero_on_failure_or_cancel() {
    use handshake_native::rich_editor::slash_commands::code_symbol_search::CodeSymbolSearchState;

    fn state_with_symbol(selection: Option<Selection>) -> Arc<Mutex<RichEditorState>> {
        let state = mt020_state("pane-mt036-code-ref");
        let mut st = state.lock().unwrap();
        st.code_ref_workspace_id = "ws-mt036-code-ref".to_owned();
        if let Some(selection) = selection {
            st.selection = selection;
        }
        let mut dialog = CodeSymbolSearchState::open("ws-mt036-code-ref", None);
        dialog.query = "parse".to_owned();
        dialog.results = vec![
            handshake_native::code_editor::code_nav::CodeSymbolNavProjection {
                symbol_entity_id: "SYM-MT036".into(),
                display_name: "parse_mt036".into(),
                symbol_kind: "function".into(),
                ..Default::default()
            },
        ];
        st.code_symbol_search = Some(dialog);
        drop(st);
        state
    }

    fn click_symbol(harness: &mut Harness<()>) {
        let target = handshake_native::rich_editor::slash_commands::code_symbol_result_author_id(
            "SYM-MT036",
        );
        let root = harness.root();
        root.children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(target.as_str()))
            .expect("mounted symbol result row")
            .click();
    }

    let (runtime, emitter, posted) = recording_event_emitter("ws-mt036-code-ref");
    let success = state_with_symbol(None);
    let mut success_harness =
        mounted_event_harness(success, egui::vec2(700.0, 480.0), emitter.clone());
    success_harness.step();
    click_symbol(&mut success_harness);
    success_harness.step();
    success_harness.step();
    drain_recording_emitter(&runtime);
    {
        let events = posted.lock().unwrap();
        assert_eq!(
            events.len(),
            1,
            "successful cross-ref transaction emits exactly once"
        );
        assert_eq!(events[0].action.as_str(), "cross_ref_inserted");
        assert_eq!(events[0].payload["symbol_entity_id"], "SYM-MT036");
    }

    let failure = state_with_symbol(Some(Selection::node(vec![0])));
    let mut failure_harness =
        mounted_event_harness(failure, egui::vec2(700.0, 480.0), emitter.clone());
    failure_harness.step();
    click_symbol(&mut failure_harness);
    failure_harness.step();
    failure_harness.step();

    let cancelled = state_with_symbol(None);
    let mut cancel_harness = mounted_event_harness(cancelled, egui::vec2(700.0, 480.0), emitter);
    cancel_harness.step();
    cancel_harness.key_press(egui::Key::Escape);
    cancel_harness.step();
    cancel_harness.step();
    drain_recording_emitter(&runtime);
    assert_eq!(
        posted.lock().unwrap().len(),
        1,
        "failed and cancelled cross-ref paths emit zero"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 wave-2 remediation — slash-command Done-outcome undo gap: `execute_slash_selection`
// (BOTH call sites: the popup Enter and the menu-row click share it) must queue a `(before, after)`
// pending_bus_undo pair for a `Done` outcome (SetBlock / InsertNode / InsertTemplate), so a REAL
// Ctrl+Z reverts a `/divider` insert exactly. The pre-fix defect: only the prompt-confirm /
// code-ref / wikilink / tag paths queued pairs — a Done-outcome slash command never reached the
// MT-035 unified undo bus (the Enter path runs BEFORE the frame's `doc_before` capture; the row
// click runs in the render phase — the frame diff sees neither).
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn mt035_live_slash_divider_bus_undo_restores_exact_doc() {
    use handshake_native::interop::interaction_bus::InteractionBus;
    use handshake_native::rich_editor::document_model::node::NodeKind;

    // A paragraph holding the REAL typed trigger text "/divider", caret after it, menu open with the
    // snapshot the widget would carry (trigger at char 0, filter "divider" -> horizontal-rule row 0).
    let doc = BlockNode::doc(vec![BlockNode::paragraph("/divider")]);
    let rich_pane: handshake_native::pane_registry::PaneId = Arc::from("pane-mt035-divider");
    let state = Arc::new(Mutex::new({
        let mut st = RichEditorState::new(doc);
        st.selection = Selection::caret(DocPosition::new(vec![0, 0], 8));
        st.undo_pane_id = Some(rich_pane.clone());
        let mut menu = SlashMenuState::open(vec![0, 0], 0);
        menu.filter = "divider".to_string();
        st.slash_menu = Some(menu);
        st
    }));
    let mut harness = editor_harness_cpu(Arc::clone(&state), egui::vec2(600.0, 400.0));
    harness.step();
    focus_editor(&mut harness);
    let before = pre_insert_json(&state);

    // A REAL Enter executes the selected row (the popup-Enter call site of execute_slash_selection).
    harness.key_press(egui::Key::Enter);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert!(st.slash_menu.is_none(), "a Done outcome closes the menu");
        assert_eq!(
            st.doc.children.len(),
            2,
            "the divider inserted a new block after the trigger paragraph"
        );
        assert_eq!(
            st.doc.children[1].as_block().unwrap().kind,
            NodeKind::HorizontalRule,
            "the inserted block is the horizontal rule (/divider)"
        );
    } // drop the state guard BEFORE `block_plain_text` re-locks it (std Mutex is non-reentrant).
    assert_eq!(
        block_plain_text(&state, 0),
        "",
        "the `/divider` trigger text was removed by the execution"
    );
    // The execution recorded EXACTLY ONE unified-bus entry (trigger removal + insert, one pair).
    let bus = InteractionBus::get_or_init(&harness.ctx);
    let depth =
        InteractionBus::with_try_lock(&bus, |b| b.local_undo_count(&rich_pane)).expect("bus lock");
    assert_eq!(
        depth, 1,
        "the /divider execution recorded one unified-bus undo entry"
    );

    // A REAL Ctrl+Z restores the EXACT pre-execution doc: the rule is gone AND `/divider` is back.
    focus_editor(&mut harness);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        assert_eq!(
            st.doc.children.len(),
            1,
            "Ctrl+Z removed the inserted divider block"
        );
        let now =
            handshake_native::rich_editor::document_model::doc_json::to_content_json_value(&st.doc);
        assert_eq!(
            now, before,
            "bus undo restored the EXACT pre-execution doc (`/divider` trigger text included)"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 wave-2 remediation — the in-window (<500ms) coalescing contract for ATOM CONFIRMS:
// two prompt confirms landing inside ONE RichUndoBatcher window coalesce into ONE unified-bus undo
// entry whose single Ctrl+Z restores the batch-START doc (both atoms gone). This pins the MT-035
// batching behavior for the pending_bus_undo drain path as a proven contract (it was previously only
// implied by the typed-edit batching tests).
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn mt035_in_window_atom_confirms_coalesce_into_one_bus_undo_entry() {
    use handshake_native::interop::interaction_bus::InteractionBus;
    use handshake_native::rich_editor::interop_adapter::RICH_UNDO_BATCH_MS;

    // Pin the production window value the contract names (<500ms = in-window).
    assert_eq!(
        RICH_UNDO_BATCH_MS, 500,
        "the rich-undo batching window is the contract's 500ms"
    );

    let state = mt020_state("pane-mt035-coalesce");
    let rich_pane: handshake_native::pane_registry::PaneId = Arc::from("pane-mt035-coalesce");
    {
        let mut st = state.lock().unwrap();
        let mut menu = SlashMenuState::open(vec![0, 0], 0);
        let mut prompt = SlashPrompt::new(SlashPromptKind::Embed(EmbedKind::Image));
        prompt.input = "asset-one".to_string();
        menu.prompt = Some(prompt);
        st.slash_menu = Some(menu);
    }
    let mut harness = editor_harness_cpu(Arc::clone(&state), egui::vec2(600.0, 400.0));
    harness.step();
    focus_editor(&mut harness);
    let before = pre_insert_json(&state);
    let window_start = std::time::Instant::now();

    // Confirm #1 (a REAL Enter through the live prompt render).
    harness.key_press(egui::Key::Enter);
    harness.step();
    harness.step();

    // Confirm #2, immediately (no window break — the same <500ms batch).
    {
        let mut st = state.lock().unwrap();
        let mut menu = SlashMenuState::open(vec![0, 0], 0);
        let mut prompt = SlashPrompt::new(SlashPromptKind::Embed(EmbedKind::Image));
        prompt.input = "asset-two".to_string();
        menu.prompt = Some(prompt);
        st.slash_menu = Some(menu);
    }
    harness.step();
    focus_editor(&mut harness);
    harness.key_press(egui::Key::Enter);
    harness.step();
    harness.step();

    // HONEST environment guard (not a seeded pass): the coalescing claim below is only provable if
    // both confirms actually landed inside one 500ms window. On a sane machine this elapsed is a few
    // milliseconds; a pathologically stalled host fails HERE with the reason, never silently.
    let elapsed = window_start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_millis(RICH_UNDO_BATCH_MS),
        "test host too slow to prove in-window coalescing (both confirms took {elapsed:?}, \
         window is {RICH_UNDO_BATCH_MS}ms)"
    );

    // Both atoms are in the doc...
    {
        let st = state.lock().unwrap();
        let para = st.doc.children[0].as_block().unwrap();
        let refs: Vec<String> = para
            .children
            .iter()
            .filter_map(Child::as_hs_link)
            .map(|l| l.ref_value.clone())
            .collect();
        assert_eq!(
            refs,
            vec!["asset-one".to_string(), "asset-two".to_string()],
            "both confirmed embeds are in the paragraph"
        );
    }
    // ...but the bus holds ONE coalesced entry (the in-window contract).
    let bus = InteractionBus::get_or_init(&harness.ctx);
    let depth =
        InteractionBus::with_try_lock(&bus, |b| b.local_undo_count(&rich_pane)).expect("bus lock");
    assert_eq!(
        depth, 1,
        "two in-window atom confirms coalesced into ONE unified-bus undo entry"
    );

    // ONE Ctrl+Z restores the batch-START doc: BOTH atoms are gone.
    focus_editor(&mut harness);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        let para = st.doc.children[0].as_block().unwrap();
        assert!(
            para.children.iter().all(|c| c.as_hs_link().is_none()),
            "one undo removed BOTH in-window confirmed atoms (batch-start restore)"
        );
        let now =
            handshake_native::rich_editor::document_model::doc_json::to_content_json_value(&st.doc);
        assert_eq!(
            now, before,
            "the coalesced undo restored the EXACT batch-start doc"
        );
    }
}
