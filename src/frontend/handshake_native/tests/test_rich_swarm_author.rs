//! WP-KERNEL-012 MT-110 (E7 swarm-authoring) — the rich-editor swarm-edit AccessKit surface, the
//! MT-080 mirror for the rich pane.
//!
//! MT-080 gave the CODE editor's `code_editor_text` `Role::TextInput` node `Action::SetValue` +
//! `Action::ReplaceSelectedText` and a per-frame `consume_swarm_text_actions` that applies a dispatched
//! swarm request to the buffer (proven by `test_app_host_mount_secondary`'s `code_text_*` tests). MT-110
//! gives the RICH editor the SAME surface so an out-of-process swarm agent can author rich documents by
//! id (this unblocks MT-043 STEP 3 — a swarm agent adding a backlink via the rich editor purely via
//! AccessKit).
//!
//! These tests mirror the code_text proofs against the LIVE `RichEditorWidget` (kittest-mounted, the same
//! seam MT-041/043 use), driving RAW `egui::Event::AccessKitActionRequest` dispatch by id (NOT
//! key-simulation, NOT a direct app-code mutation):
//! - `rich_root_node_advertises_swarm_edit_actions`: the live `rich-editor-root` `Role::TextInput` node
//!   advertises `Action::SetValue` + `Action::ReplaceSelectedText`.
//! - `rich_root_setvalue_dispatch_authors_doc_through_undo_bus`: a dispatched `SetValue` at the root node
//!   replaces the DocJson body with the AGENT content AND records an undo entry on the MT-035 unified undo
//!   bus (a real `bus.undo(pane)` pops it and reverts the doc — the swarm edit is undoable, no `set_text`
//!   bypass).
//! - `rich_root_replace_selected_text_inserts_at_caret`: a dispatched `ReplaceSelectedText` inserts the
//!   agent text at the caret via the SAME `input_handler` insert the keyboard path uses.
//! - `rich_wikilink_setvalue_by_id_sets_target_ref_value_headless`: activating an `hsLink` chip by id (a
//!   `SetValue` dispatch) sets its target `ref_value` WITHOUT a live backend search — the headless
//!   wikilink-target-by-id pick MT-043 STEP 3 needs.

use std::sync::{Arc, Mutex};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::interop::interaction_bus::InteractionBus;
use handshake_native::pane_registry::PaneId;
use handshake_native::rich_editor::document_model::node::{
    BlockNode, Child, HsLinkNode, NodeKind, TextLeaf,
};
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorState, RichEditorWidget,
};
use handshake_native::rich_editor::renderer::RICH_EDITOR_ROOT_AUTHOR_ID;

/// Build a kittest harness rendering the LIVE editable `RichEditorWidget` over `state` each frame (fonts
/// installed so wikilink-chip galley layout is real). The widget is NOT focused (so `harness.run()`
/// converges — the caret blink is focus-gated), which is fine: the swarm dispatch reaches the node
/// through AccessKit, not the keyboard focus path.
fn mount(state: Arc<Mutex<RichEditorState>>) -> Harness<'static, ()> {
    let state_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 420.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            RichEditorWidget::new(Arc::clone(&state_ui)).show(ui);
        });
    // Two warm-up frames so the AccessKit tree + the live node ids populate.
    harness.run();
    harness.run();
    harness
}

/// The live AccessKit node id (accesskit `NodeId`) of the first node whose author_id matches `author_id`.
fn node_id_by_author(harness: &mut Harness<'_, ()>, author_id: &str) -> egui::accesskit::NodeId {
    harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("node with author_id={author_id} present in the live tree"))
        .accesskit_node()
        .id()
}

// ── requirement (1): the rich root node advertises the swarm edit actions ─────────────────────────────

#[test]
fn rich_root_node_advertises_swarm_edit_actions() {
    let state = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(vec![
        BlockNode::paragraph("note body"),
    ]))));
    let harness = mount(state);

    let root = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(RICH_EDITOR_ROOT_AUTHOR_ID))
        .expect("the rich-editor-root node is in the live tree");
    let node = root.accesskit_node();
    // Probe the RAW NodeData action set (single-arg `supports_action`, the same probe the swarm proof
    // uses) so the assertion reads the node's OWN declared actions.
    assert!(
        node.data()
            .supports_action(egui::accesskit::Action::SetValue),
        "MT-110: the rich-editor-root node advertises Action::SetValue (swarm author-whole-doc)"
    );
    assert!(
        node.data()
            .supports_action(egui::accesskit::Action::ReplaceSelectedText),
        "MT-110: the rich-editor-root node advertises Action::ReplaceSelectedText (swarm edit-selection)"
    );
}

// ── requirement (2): SetValue dispatch mutates the DocJson buffer, routed through the undo bus ─────────

#[test]
fn rich_root_setvalue_dispatch_authors_doc_through_undo_bus() {
    // A mounted rich pane carrying a pane id (the production wiring the factory sets on mount), so its
    // swarm edit records + routes on the SHARED unified undo bus under that pane's ring.
    let state = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(vec![
        BlockNode::paragraph("original body"),
    ]))));
    let rich_pane: PaneId = PaneId::from("pane-rich-swarm");
    state.lock().unwrap().undo_pane_id = Some(rich_pane.clone());

    let mut harness = mount(Arc::clone(&state));

    let before_text = state.lock().unwrap().block_plain_text(0).unwrap_or_default();
    assert_eq!(before_text, "original body", "the doc starts as 'original body'");

    // Dispatch a RAW AccessKit SetValue request carrying the AGENT content (the exact shape a swarm
    // agent's `egui::Event::AccessKitActionRequest` carries — NOT key-simulation, NOT a direct st.doc
    // mutation). The agent content is provably agent-authored: the doc STARTS with different content and
    // ONLY this dispatch replaces it.
    const AGENT_TEXT: &str = "swarm-authored text";
    let node_id = node_id_by_author(&mut harness, RICH_EDITOR_ROOT_AUTHOR_ID);
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::SetValue,
            target: node_id,
            data: Some(egui::accesskit::ActionData::Value(AGENT_TEXT.into())),
        },
    ));
    // `consume_swarm_text_actions` drains + applies the request within a frame (MT-110). Bounded explicit
    // frames (an unfocused editor never spins).
    let mut applied = false;
    for _ in 0..8 {
        harness.run();
        if state.lock().unwrap().block_plain_text(0).as_deref() == Some(AGENT_TEXT) {
            applied = true;
            break;
        }
    }
    assert!(
        applied,
        "MT-110: the swarm SetValue dispatch was consumed within the frame budget"
    );
    assert_eq!(
        state.lock().unwrap().block_plain_text(0).unwrap_or_default(),
        AGENT_TEXT,
        "MT-110: a swarm Action::SetValue at the rich text node authored the AGENT content into the DocJson"
    );

    // PROOF the edit routed through the MT-035 UNIFIED undo bus (not a set_text bypass): the pane's ring
    // has exactly ONE entry, and the observability seam advanced.
    let bus = InteractionBus::get_or_init(&harness.ctx);
    let depth =
        InteractionBus::with_try_lock(&bus, |b| b.local_undo_count(&rich_pane)).expect("bus lock");
    assert_eq!(
        depth, 1,
        "MT-110: the swarm SetValue recorded ONE entry on the unified undo bus (got {depth})"
    );
    assert!(
        state.lock().unwrap().swarm_undo_fired_count >= 1,
        "MT-110: the swarm-edit undo_fired observability seam advanced"
    );

    // A real undo POPS the entry and REVERTS the doc through the SAME shared scope a typed edit uses.
    let result = InteractionBus::with_try_lock(&bus, |b| b.undo(&rich_pane))
        .expect("bus lock")
        .expect("an entry to undo on the rich pane");
    assert!(result.ok, "the swarm-edit undo applied: {result:?}");
    assert_eq!(
        state.lock().unwrap().block_plain_text(0).unwrap_or_default(),
        "original body",
        "MT-110: undo reverted the swarm-authored doc to its pre-edit content"
    );
    let depth_after =
        InteractionBus::with_try_lock(&bus, |b| b.local_undo_count(&rich_pane)).expect("bus lock");
    assert_eq!(depth_after, 0, "MT-110: the unified ring drained after the undo");
}

// ── requirement (1b): ReplaceSelectedText inserts the agent text at the caret ─────────────────────────

#[test]
fn rich_root_replace_selected_text_inserts_at_caret() {
    // Caret at the doc start (the default after `new`), so the inserted text lands at the front.
    let state = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(vec![
        BlockNode::paragraph("tail"),
    ]))));
    let mut harness = mount(Arc::clone(&state));

    const AGENT_TEXT: &str = "head-";
    let node_id = node_id_by_author(&mut harness, RICH_EDITOR_ROOT_AUTHOR_ID);
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::ReplaceSelectedText,
            target: node_id,
            data: Some(egui::accesskit::ActionData::Value(AGENT_TEXT.into())),
        },
    ));
    let mut applied = false;
    for _ in 0..8 {
        harness.run();
        if state
            .lock()
            .unwrap()
            .block_plain_text(0)
            .as_deref()
            .map(|t| t.starts_with(AGENT_TEXT))
            .unwrap_or(false)
        {
            applied = true;
            break;
        }
    }
    assert!(
        applied,
        "MT-110: the swarm ReplaceSelectedText dispatch was consumed within the frame budget"
    );
    assert_eq!(
        state.lock().unwrap().block_plain_text(0).unwrap_or_default(),
        "head-tail",
        "MT-110: a swarm Action::ReplaceSelectedText inserted the AGENT text at the caret"
    );
}

// ── requirement (3): headless wikilink-target-by-id — activate an hsLink chip by id, set its refValue ──

#[test]
fn rich_wikilink_setvalue_by_id_sets_target_ref_value_headless() {
    // A doc with one paragraph holding a text run + a PLACEHOLDER hsLink atom (resolved, so no create
    // affordance — a plain wikilink chip that advertises the swarm SetValue action). A swarm agent picks
    // its TARGET by id with NO live backend search (the headless wikilink-target pick MT-043 STEP 3 needs).
    let placeholder = HsLinkNode::new("note", "placeholder-target", "");
    let doc = BlockNode::doc(vec![BlockNode::with_children(
        NodeKind::Paragraph,
        vec![
            Child::Text(TextLeaf::new("see ")),
            Child::HsLink(placeholder),
        ],
    )]);
    let state = Arc::new(Mutex::new(RichEditorState::new(doc)));
    let mut harness = mount(Arc::clone(&state));

    // Find the wikilink chip node — the single author-addressable node whose author_id starts with
    // `wikilink-chip-` AND advertises the swarm SetValue action (MT-110). No live backend is wired; the
    // chip is addressable purely from the DocJson atom.
    let chip = harness
        .root()
        .children_recursive()
        .find(|n| {
            let ak = n.accesskit_node();
            ak.author_id()
                .is_some_and(|a| a.starts_with("wikilink-chip-"))
                && ak.data().supports_action(egui::accesskit::Action::SetValue)
        })
        .expect("MT-110: the wikilink chip advertises the swarm SetValue action");
    let chip_id = chip.accesskit_node().id();

    const TARGET_REF: &str = "SwarmProofTarget-block";
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::SetValue,
            target: chip_id,
            data: Some(egui::accesskit::ActionData::Value(TARGET_REF.into())),
        },
    ));

    let mut applied = false;
    for _ in 0..8 {
        harness.run();
        let s = state.lock().unwrap();
        let set = s
            .doc
            .children
            .first()
            .and_then(Child::as_block)
            .and_then(|b| b.children.get(1))
            .and_then(Child::as_hs_link)
            .map(|l| l.ref_value == TARGET_REF)
            .unwrap_or(false);
        drop(s);
        if set {
            applied = true;
            break;
        }
    }
    assert!(
        applied,
        "MT-110: the swarm wikilink-target SetValue dispatch was consumed within the frame budget"
    );
    let s = state.lock().unwrap();
    let link = s
        .doc
        .children
        .first()
        .and_then(Child::as_block)
        .and_then(|b| b.children.get(1))
        .and_then(Child::as_hs_link)
        .expect("the hsLink atom is still at [0,1]");
    assert_eq!(
        link.ref_value, TARGET_REF,
        "MT-110: activating the hsLink chip by id set its target ref_value (headless, no backend search)"
    );
    assert!(
        link.resolved,
        "MT-110: the wikilink-target pick marked the link resolved"
    );
}
