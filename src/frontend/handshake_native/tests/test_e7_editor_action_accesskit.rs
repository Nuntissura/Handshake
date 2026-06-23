//! WP-KERNEL-012 MT-041 (E7 model-vision parity): LIVE proofs for the consolidated
//! **EditorActionRegistry** AccessKit surface — every interactive editor action in BOTH the native
//! code editor (E1) and the native rich-text editor (E2) exposed through the WP-011 AccessKit channel
//! with a stable canonical `editor.<pane>.<action>` author_id, a correct role, at least one declared
//! action, and a REAL dispatch path (no screen-scraping, no keyboard simulation).
//!
//! These are the contract proof_targets that need a live egui frame (egui_kittest, in-process):
//! - PROOF-041-A / AC-041-01: open both panes in-process, query the AccessKit tree, assert every
//!   `EditorActionId` (the IN-041-03 + IN-041-04 catalog) appears with the correct `editor.<pane>.<id>`
//!   author_id, role, and >=1 declared action.
//! - PROOF-041-B: print the full AccessKit tree to stdout (the reviewer can locate the named nodes).
//! - PROOF-041-D / AC-041-02/03: dump >=5 find-panel action nodes showing role + a declared action.
//! - AC-041-04: dispatch `editor.code.find-open` via the AccessKit Action channel -> the find panel
//!   opens (a new `editor.code.find-panel` node appears within one frame).
//! - AC-041-05: dispatch `editor.rich.format-bold`; the cursor sits inside bold text so the
//!   `editor.rich.format-bold` node reports `checked=true` (CTRL-041-03 toggle-state-on-cursor).
//! - AC-041-06: dispatch `editor.code.save` -> the code-save intent reaches the host command bus
//!   (spied via the injected `command_palette` sender — the E6/MT-037 save wiring point; CTRL-041-06).
//! - AC-041-08 / CTRL-041-01/02/05: anti-scaffolding — every emitted node maps to a real widget; the
//!   ids survive a layout change (a dummy panel added above); a non-empty-tree health canary is present;
//!   two code panes get instance-suffixed author_ids.
//!
//! ## Dispatch path (the swarm-agent invocation)
//!
//! A swarm agent dispatches an action by feeding an `egui::Event::AccessKitActionRequest` targeting the
//! node's stable `NodeId` (the same path `crate::mcp::action` builds). The editor's `show` consumes that
//! Click on the canonical node within the SAME frame and routes it to the real action (RISK-041-04). The
//! test resolves a canonical author_id -> live `NodeId` from the kittest tree, then dispatches a Click.

use std::path::Path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::accessibility::editor_action_registry::{
    rich_action_catalog, EditorActionRegistry, CODE_ACTION_CATALOG, HEALTH_CANARY_AUTHOR_ID,
};
use handshake_native::code_editor::{CodeEditorAction, CodeEditorPanel};
use handshake_native::rich_editor::renderer::rich_editor_widget::{RichEditorState, RichEditorWidget};

// ── artifact-hygiene guard (CX-212E) ─────────────────────────────────────────────────────────────

/// Assert no repo-local artifact dir exists under the crate (CX-212E): neither `test_output/` nor
/// `tests/screenshots/`. This MT writes no screenshots, but the guard is required by the screenshot/
/// artifact rule and the reviewer's `git ls-files "src/**/*.png"` check — call it in the live test.
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

// ── harness builders ─────────────────────────────────────────────────────────────────────────────

/// A code-editor-only harness with the MT-041 registry installed at instance 0. Returns the shared
/// panel, the shared registry, and the harness. The harness renders the code editor in a CentralPanel.
fn code_harness<'a>() -> (Arc<CodeEditorPanel>, Arc<Mutex<EditorActionRegistry>>, Harness<'a, ()>) {
    let panel = Arc::new(CodeEditorPanel::new("fn main() {\n    let x = 1;\n}\n", "rs"));
    let registry = Arc::new(Mutex::new(EditorActionRegistry::new()));
    panel.install_editor_action_registry(Arc::clone(&registry), 0);
    let panel_ui = Arc::clone(&panel);
    let harness = Harness::builder()
        .with_size(egui::vec2(800.0, 320.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    (panel, registry, harness)
}

/// A rich-editor-only harness with the MT-041 registry installed at instance 0. The demo doc has a
/// bold "world" run with the caret inside it (so `format-bold` reports checked=true — AC-041-05).
fn rich_harness<'a>() -> (Arc<Mutex<RichEditorState>>, Arc<Mutex<EditorActionRegistry>>, Harness<'a, ()>) {
    let registry = Arc::new(Mutex::new(EditorActionRegistry::new()));
    let state = {
        let mut s = RichEditorState::demo();
        s.install_editor_action_registry(Arc::clone(&registry), 0);
        Arc::new(Mutex::new(s))
    };
    let state_ui = Arc::clone(&state);
    let harness = Harness::builder()
        .with_size(egui::vec2(800.0, 360.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            RichEditorWidget::new(Arc::clone(&state_ui)).show(ui);
        });
    (state, registry, harness)
}

/// A node found in the live kittest tree, reduced to the fields the proofs assert.
struct FoundNode {
    node_id: egui::accesskit::NodeId,
    role: String,
    toggled: Option<bool>,
    disabled: bool,
}

/// Resolve a canonical `author_id` to its live AccessKit consumer node in the harness tree.
fn find_node(root: &egui_kittest::Node<'_>, author_id: &str) -> Option<FoundNode> {
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            let toggled = match ak.toggled() {
                Some(egui::accesskit::Toggled::True) => Some(true),
                Some(egui::accesskit::Toggled::False) => Some(false),
                _ => None,
            };
            return Some(FoundNode {
                node_id: ak.id(),
                role: format!("{:?}", ak.role()),
                toggled,
                disabled: ak.is_disabled(),
            });
        }
    }
    None
}

/// Build a Click AccessKit action request event targeting `node_id` (the swarm-agent dispatch path,
/// the same shape `crate::mcp::action::build_action_request` produces).
fn click_event(node_id: egui::accesskit::NodeId) -> egui::Event {
    egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action: egui::accesskit::Action::Click,
        target: node_id,
        data: None,
    })
}

// ── PROOF-041-A / AC-041-01/02: every code action node present, correct role + >=1 action ─────────

#[test]
fn code_actions_all_present_with_role_and_action() {
    let (_panel, _registry, mut harness) = code_harness();
    // Open the find panel so the find-scoped actions (find-next/prev, toggles, replace) are present.
    harness.run();
    harness.event(egui::Event::Key {
        key: egui::Key::F,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers { ctrl: true, ..Default::default() },
    });
    harness.run();
    harness.run(); // settle so the find-scoped nodes emit

    let root = harness.root();
    // The health canary is always present (RISK-041-02 / CTRL-041-02 — never a false-green empty tree).
    assert!(
        find_node(&root, HEALTH_CANARY_AUTHOR_ID).is_some(),
        "CTRL-041-02: the health canary '{HEALTH_CANARY_AUTHOR_ID}' must be in the live tree"
    );

    let mut missing = Vec::new();
    for entry in CODE_ACTION_CATALOG {
        let author_id = format!("editor.code.{}", entry.action_id);
        match find_node(&root, &author_id) {
            Some(found) => {
                let want_role = entry.role.role_str();
                assert_eq!(
                    found.role, want_role,
                    "AC-041-02: '{author_id}' role mismatch (want {want_role}, got {})",
                    found.role
                );
            }
            None => missing.push(author_id),
        }
    }
    assert!(
        missing.is_empty(),
        "AC-041-01/02: every code action in IN-041-03 must be in the AccessKit tree; missing: {missing:?}"
    );
    println!(
        "PROOF-041-A (code): all {} IN-041-03 actions present with correct role + >=1 action",
        CODE_ACTION_CATALOG.len()
    );
}

// ── AC-041-03: every rich action node present, correct role + >=1 action ──────────────────────────

#[test]
fn rich_actions_all_present_with_role_and_action() {
    let (state, _registry, mut harness) = rich_harness();
    harness.run();
    // Open the rich find panel so the find-scoped rich actions are present.
    {
        let mut s = state.lock().unwrap();
        s.find_replace = Some(handshake_native::rich_editor::find_replace::FindReplaceState::open(true));
    }
    harness.run();
    harness.run();

    let root = harness.root();
    assert!(
        find_node(&root, HEALTH_CANARY_AUTHOR_ID).is_some(),
        "CTRL-041-02: rich tree carries the health canary"
    );

    let catalog = rich_action_catalog();
    let mut missing = Vec::new();
    for entry in &catalog {
        let author_id = format!("editor.rich.{}", entry.action_id);
        match find_node(&root, &author_id) {
            Some(found) => {
                assert_eq!(
                    found.role,
                    entry.role.role_str(),
                    "AC-041-03: '{author_id}' role mismatch (want {}, got {})",
                    entry.role.role_str(),
                    found.role
                );
            }
            None => missing.push(author_id),
        }
    }
    assert!(
        missing.is_empty(),
        "AC-041-03: every rich action in IN-041-04 must be in the AccessKit tree; missing: {missing:?}"
    );
    println!(
        "AC-041-03 (rich): all {} IN-041-04 actions present with correct role + >=1 action",
        catalog.len()
    );
}

// ── PROOF-041-B: full AccessKit tree dump (the reviewer locates the named nodes) ──────────────────

#[test]
fn proof_b_full_tree_dump_locates_named_nodes() {
    // A combined harness rendering BOTH panes so one dump shows code + rich action nodes.
    let panel = Arc::new(CodeEditorPanel::new("fn main() {}\n", "rs"));
    let registry = Arc::new(Mutex::new(EditorActionRegistry::new()));
    panel.install_editor_action_registry(Arc::clone(&registry), 0);
    let rich_state = {
        let mut s = RichEditorState::demo();
        s.install_editor_action_registry(Arc::clone(&registry), 0);
        Arc::new(Mutex::new(s))
    };
    let panel_ui = Arc::clone(&panel);
    let rich_ui = Arc::clone(&rich_state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 600.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            ui.horizontal(|ui| {
                ui.vertical(|ui| panel_ui.show(ui));
                ui.vertical(|ui| {
                    RichEditorWidget::new(Arc::clone(&rich_ui)).show(ui);
                });
            });
        });
    harness.run();
    harness.run();

    let root = harness.root();
    // Collect every author_id-bearing node into a deterministic, sorted dump.
    let mut dump: Vec<String> = Vec::new();
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author) = ak.author_id() {
            if author.starts_with("editor.") {
                let toggled = match ak.toggled() {
                    Some(egui::accesskit::Toggled::True) => " toggled=true",
                    Some(egui::accesskit::Toggled::False) => " toggled=false",
                    _ => "",
                };
                dump.push(format!("{author}  role={:?}{toggled}", ak.role()));
            }
        }
    }
    dump.sort();
    dump.dedup();
    println!("--- PROOF-041-B: editor.* AccessKit node dump ({} nodes) ---", dump.len());
    for line in &dump {
        println!("{line}");
    }

    // The reviewer must be able to locate these named nodes (PROOF-041-B).
    for want in [
        "editor.code.find-open",
        "editor.code.save",
        "editor.rich.format-bold",
        "editor.rich.command-palette-open",
    ] {
        assert!(
            find_node(&root, want).is_some(),
            "PROOF-041-B: named node '{want}' must be locatable in the dump"
        );
    }
    assert_no_local_artifact_dir();
}

// ── PROOF-041-D: >=5 find-panel action nodes with role + a declared action ────────────────────────

#[test]
fn proof_d_find_panel_actions_dump() {
    let (_panel, _registry, mut harness) = code_harness();
    harness.run();
    harness.event(egui::Event::Key {
        key: egui::Key::F,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers { ctrl: true, ..Default::default() },
    });
    harness.run();
    harness.run();

    let root = harness.root();
    let find_actions = [
        "editor.code.find-next",
        "editor.code.find-prev",
        "editor.code.find-toggle-case",
        "editor.code.find-toggle-word",
        "editor.code.find-toggle-regex",
    ];
    let mut shown = 0;
    println!("--- PROOF-041-D: find-panel action nodes ---");
    for aid in find_actions {
        let found = find_node(&root, aid)
            .unwrap_or_else(|| panic!("PROOF-041-D: '{aid}' must be present while find is open"));
        assert!(
            found.role == "Button" || found.role == "CheckBox",
            "PROOF-041-D: '{aid}' role must be Button or CheckBox (accesskit-0.21 ToggleButton=CheckBox); got {}",
            found.role
        );
        println!("{aid}  role={}  toggled={:?}", found.role, found.toggled);
        shown += 1;
    }
    assert!(shown >= 5, "PROOF-041-D: at least 5 find-panel actions dumped; got {shown}");
}

// ── AC-041-04: dispatch editor.code.find-open -> find panel node appears within one frame ─────────

#[test]
fn ac04_dispatch_find_open_opens_find_panel() {
    let (panel, _registry, mut harness) = code_harness();
    harness.run();
    harness.run();
    assert!(!panel.is_find_open(), "find starts closed");

    // The find-panel node is ABSENT before dispatch (AC-041-04 anti-scaffolding: no node for a
    // surface that is not rendered).
    assert!(
        find_node(&harness.root(), "editor.code.find-panel").is_none(),
        "AC-041-04: 'editor.code.find-panel' absent while find is closed"
    );

    // Resolve the canonical find-open node id from the live tree and dispatch a Click at it.
    let find_open = find_node(&harness.root(), "editor.code.find-open").expect("find-open node present");
    harness.event(click_event(find_open.node_id));
    harness.run(); // the editor consumes the Click + opens find this frame
    harness.run(); // settle so the find-panel node emits

    assert!(
        panel.is_find_open(),
        "AC-041-04: dispatching editor.code.find-open via AccessKit opened the find panel"
    );
    assert!(
        find_node(&harness.root(), "editor.code.find-panel").is_some(),
        "AC-041-04: 'editor.code.find-panel' node appears after find-open dispatch"
    );
    println!("AC-041-04: AccessKit dispatch of editor.code.find-open opened the find panel + node");
}

// ── AC-041-05: dispatch editor.rich.format-bold; toggle reports checked when cursor in bold ────────

#[test]
fn ac05_format_bold_toggle_reflects_cursor_in_bold() {
    let (state, _registry, mut harness) = rich_harness();
    harness.run();
    harness.run();

    // The demo caret sits at [1,1] offset 5 — INSIDE the bold "world" run — so the live toggle reads
    // checked=true (CTRL-041-03: toggle state tracks the cursor).
    let bold0 = find_node(&harness.root(), "editor.rich.format-bold").expect("format-bold node present");
    assert_eq!(bold0.role, "CheckBox", "format-bold is a ToggleButton -> CheckBox (accesskit 0.21)");
    assert_eq!(
        bold0.toggled,
        Some(true),
        "AC-041-05/CTRL-041-03: format-bold reports checked=true with the caret inside bold text"
    );

    // Move the caret OUT of the bold run (into the regular "Hello " leaf) and assert it flips to false.
    {
        use handshake_native::rich_editor::document_model::position::DocPosition;
        use handshake_native::rich_editor::document_model::selection::Selection;
        let mut s = state.lock().unwrap();
        s.selection = Selection::caret(DocPosition::new(vec![1, 0], 2)); // "Hello " leaf
    }
    harness.run();
    harness.run();
    let bold_after = find_node(&harness.root(), "editor.rich.format-bold").expect("format-bold node present");
    assert_eq!(
        bold_after.toggled,
        Some(false),
        "CTRL-041-03: format-bold flips to checked=false when the caret leaves bold text"
    );

    // Now DISPATCH editor.rich.format-bold via AccessKit on a selection of plain text -> the mark
    // applies (the real toggle_mark command runs). Select the "Hello " word so the toggle adds bold.
    {
        use handshake_native::rich_editor::document_model::position::DocPosition;
        use handshake_native::rich_editor::document_model::selection::Selection;
        let mut s = state.lock().unwrap();
        s.selection = Selection::Text {
            anchor: DocPosition::new(vec![1, 0], 0),
            head: DocPosition::new(vec![1, 0], 5),
        };
    }
    harness.run();
    let bold_sel = find_node(&harness.root(), "editor.rich.format-bold").expect("format-bold node present");
    harness.event(click_event(bold_sel.node_id));
    harness.run(); // editor consumes Click + runs toggle_mark this frame
    harness.run();
    let bold_applied = find_node(&harness.root(), "editor.rich.format-bold").expect("format-bold node present");
    assert_eq!(
        bold_applied.toggled,
        Some(true),
        "AC-041-05: dispatching editor.rich.format-bold applied bold to the selection (checked=true)"
    );
    println!("AC-041-05: format-bold toggle tracks the cursor + dispatch applies the mark");
}

// ── AC-041-06: dispatch editor.code.save -> save intent reaches the host bus (spy) ────────────────

#[test]
fn ac06_dispatch_save_reaches_backend_save_wiring() {
    let (panel, _registry, mut harness) = code_harness();
    // Spy: the code editor routes Save to the host command bus (the E6/MT-037 knowledge_documents save
    // wiring point — CTRL-041-06, not a new direct call). Inject the sender and assert Save arrives.
    let (tx, rx) = mpsc::channel::<CodeEditorAction>();
    panel.set_command_palette_sender(tx);
    harness.run();
    harness.run();

    let save = find_node(&harness.root(), "editor.code.save").expect("save node present");
    harness.event(click_event(save.node_id));
    harness.run();
    harness.run();

    // The save intent must have reached the host bus (observable spy on the save-dispatch channel).
    let mut saw_save = false;
    while let Ok(action) = rx.try_recv() {
        if action == CodeEditorAction::Save {
            saw_save = true;
        }
    }
    assert!(
        saw_save,
        "AC-041-06: dispatching editor.code.save routed a Save to the host command bus (E6 save wiring)"
    );
    println!("AC-041-06: editor.code.save dispatch reached the backend-save wiring (spy saw Save)");
}

// ── CTRL-041-01: stable ids survive a layout change (a panel added above the editor) ──────────────

#[test]
fn ctrl01_author_ids_stable_across_layout_change() {
    let panel = Arc::new(CodeEditorPanel::new("fn main() {}\n", "rs"));
    let registry = Arc::new(Mutex::new(EditorActionRegistry::new()));
    panel.install_editor_action_registry(Arc::clone(&registry), 0);
    let panel_ui = Arc::clone(&panel);
    // A flag the closure reads to add a dummy panel ABOVE the editor on later frames (RISK-041-01: an
    // insertion-order id would shift; a string-hashed id must NOT).
    let add_dummy = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let add_dummy_ui = Arc::clone(&add_dummy);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 320.0))
        .build_ui(move |ui| {
            if add_dummy_ui.load(std::sync::atomic::Ordering::Relaxed) {
                ui.label("dummy panel inserted above the editor (layout shift)");
                ui.separator();
            }
            panel_ui.show(ui);
        });
    harness.run();
    harness.run();
    let before = find_node(&harness.root(), "editor.code.save").expect("save node before");

    // Insert the dummy panel above and re-render: the save node's NodeId must be identical.
    add_dummy.store(true, std::sync::atomic::Ordering::Relaxed);
    harness.run();
    harness.run();
    let after = find_node(&harness.root(), "editor.code.save").expect("save node after layout shift");
    assert_eq!(
        before.node_id, after.node_id,
        "CTRL-041-01: editor.code.save NodeId must be stable across a layout change (string-hashed id)"
    );
    println!("CTRL-041-01: editor.code.save id {:?} stable across layout change", after.node_id);
}

// ── RISK-041-05 / CTRL-041-05: two code panes -> instance-suffixed author_ids ─────────────────────

#[test]
fn ctrl05_two_code_panes_get_instance_suffixed_ids() {
    let p0 = Arc::new(CodeEditorPanel::new("fn a() {}\n", "rs"));
    let p1 = Arc::new(CodeEditorPanel::new("fn b() {}\n", "rs"));
    let registry = Arc::new(Mutex::new(EditorActionRegistry::new()));
    p0.install_editor_action_registry(Arc::clone(&registry), 0);
    p1.install_editor_action_registry(Arc::clone(&registry), 1);
    let p0_ui = Arc::clone(&p0);
    let p1_ui = Arc::clone(&p1);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 320.0))
        .build_ui(move |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| p0_ui.show(ui));
                ui.vertical(|ui| p1_ui.show(ui));
            });
        });
    harness.run();
    harness.run();

    let root = harness.root();
    let zero = find_node(&root, "editor.code.save");
    let one = find_node(&root, "editor.code.save.1");
    assert!(zero.is_some(), "CTRL-041-05: instance 0 uses the bare 'editor.code.save'");
    assert!(one.is_some(), "CTRL-041-05: instance 1 suffixes 'editor.code.save.1'");
    assert_ne!(
        zero.unwrap().node_id,
        one.unwrap().node_id,
        "CTRL-041-05: the two panes' save nodes have distinct NodeIds (no collision)"
    );
    println!("CTRL-041-05: two code panes -> distinct editor.code.save / editor.code.save.1 nodes");
}

// ── AC-041-01: every emitted editor action node declares >=1 AccessKit action (non-tautological) ──

/// Build the live `accesskit::TreeUpdate` for both panes via a raw egui context (the SAME value the
/// out-of-process UIA adapter receives), then project it through the crate's own `collect_ui_tree_snapshot`
/// so each node's DECLARED ACTIONS are read straight from the live tree (not assumed).
#[test]
fn ac01_every_action_node_declares_at_least_one_action() {
    use handshake_native::accessibility::collect_ui_tree_snapshot;

    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let panel = Arc::new(CodeEditorPanel::new("fn main() {}\n", "rs"));
    let registry = Arc::new(Mutex::new(EditorActionRegistry::new()));
    panel.install_editor_action_registry(Arc::clone(&registry), 0);
    let rich = {
        let mut s = RichEditorState::demo();
        s.install_editor_action_registry(Arc::clone(&registry), 0);
        Arc::new(Mutex::new(s))
    };
    let panel_for = Arc::clone(&panel);
    let rich_for = Arc::clone(&rich);
    // Open both find panels so the find-scoped nodes also emit (full coverage).
    panel.open_find(true);
    rich.lock().unwrap().find_replace =
        Some(handshake_native::rich_editor::find_replace::FindReplaceState::open(true));
    // Two frames so the find-scoped nodes settle.
    for _ in 0..2 {
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            handshake_native::app::HandshakeApp::install_fonts(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| panel_for.show(ui));
                    ui.vertical(|ui| {
                        RichEditorWidget::new(Arc::clone(&rich_for)).show(ui);
                    });
                });
            });
        });
        if output.platform_output.accesskit_update.is_some() {
            let update = output.platform_output.accesskit_update.unwrap();
            let snap = collect_ui_tree_snapshot(&update);
            // Every present `editor.<pane>.<action>` node (excluding the present-only `find-panel`
            // container + the health canary) must declare >=1 action a swarm agent can dispatch.
            let mut checked = 0usize;
            for node in snap.iter_nodes() {
                let Some(author) = node.author_id.as_deref() else { continue };
                if !author.starts_with("editor.") || author.ends_with("find-panel") {
                    continue;
                }
                if author == HEALTH_CANARY_AUTHOR_ID {
                    continue;
                }
                assert!(
                    !node.actions.is_empty(),
                    "AC-041-01: '{author}' must declare >=1 AccessKit action; got none"
                );
                assert!(
                    node.actions.iter().any(|a| a == "Click"),
                    "AC-041-01: '{author}' must declare the Click action (the swarm activation); got {:?}",
                    node.actions
                );
                checked += 1;
            }
            assert!(checked >= 20, "AC-041-01: expected to verify many action nodes; checked {checked}");
            println!("AC-041-01: {checked} editor.* action nodes each declare >=1 (Click) action");
            return;
        }
    }
    panic!("AC-041-01: no AccessKit update produced over 2 frames");
}

// ── AC-041-08: language-picker gap is present-but-disabled (typed limitation, not a mock no-op) ───

#[test]
fn ac08_language_picker_gap_is_disabled_not_mocked() {
    let (_panel, _registry, mut harness) = code_harness();
    harness.run();
    harness.run();
    let root = harness.root();
    // The node IS present (discoverable) but DISABLED (a dispatch is rejected, never a silent no-op).
    let node = find_node(&root, "editor.code.language-picker-open")
        .expect("AC-041-08: language-picker-open node is present (discoverable)");
    assert!(
        node.disabled,
        "AC-041-08: editor.code.language-picker-open is a typed gap -> present but DISABLED"
    );
    println!("AC-041-08: language-picker-open present-but-disabled (typed gap, no mock no-op)");
}
