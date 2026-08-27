//! WP-KERNEL-012 MT-046 — INTERCONNECTION EDGE 4: shared undo + event-ledger across surfaces (IC-15..IC-18).
//!
//! IC-15/IC-16/IC-18 are MELT-TOGETHER SUBSTRATE proofs that are PROVABLE NOW in-process (no SurrealDB): they prove
//! the code editor and the rich-text editor share ONE undo stack — the SAME `Arc<Mutex<InteractionBus>>`
//! undo scope instance — so an undo reverts an edit recorded by either surface, and the per-pane scope policy
//! (POLICY-1 local-first) holds. The LOAD-BEARING anti-RISK-1/anti-RISK-4 control (CTRL-1/CTRL-4): each
//! cross-surface test uses ONE shared bus instance for BOTH surfaces (NOT two independent undo stacks), and
//! IC-18 edits A then B and asserts ONE undo reverts ONLY B (the most-recently-edited surface) leaving A
//! unchanged — inspecting BOTH surfaces after the single undo.
//!
//! IC-17 runs by default through the managed product-backend fixture. It reads the kernel EventLedger by
//! the document's exact aggregate identity, correlates both save receipt ids, and verifies that the second
//! receipt carries the real linked block id. This deliberately does not substitute the separate Flight
//! Recorder projection for kernel EventLedger authority.
//!
//! Artifact hygiene (CX-212E): no artifact under `src/`.

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
#[path = "interconnect_support/mod.rs"]
mod interconnect_support;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;

use std::sync::Arc;

use canonical_argus_driver::{json_has_author_id, json_node_by_author_id, CanonicalArgusDriver};
use egui_kittest::kittest::NodeT;
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::panel::CODE_EDITOR_TEXT_AUTHOR_ID;
use handshake_native::interop::{undo_count_author_id, InteractionBus};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::rich_editor::document_model::doc_json::{
    from_json_value, to_content_json_value,
};
use handshake_native::rich_editor::document_model::node::BlockNode;
use handshake_native::rich_editor::interop_adapter::{push_rich_edit_undo, RichSnapshotApplier};
use handshake_native::rich_editor::renderer::rich_editor_widget::RichEditorState;

use interconnect_support::{
    assert_no_local_artifact_dir, require_live_backend, settle_incidental_fems_for_capture,
    ScenarioAttempt,
};

fn pane(id: &str) -> PaneId {
    Arc::from(id)
}

fn editor_shell() -> (HandshakeApp, tokio::runtime::Runtime) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("IC-15/18: build runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    {
        let registry = app.pane_registry();
        let mut registry = registry.lock().expect("IC-15/18: pane registry");
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

fn focus_accesskit(harness: &mut Harness<'_, HandshakeApp>, author_id: &str) {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("mounted AccessKit node {author_id} must be present"))
        .focus();
    // The code pane publishes focus ownership before `CodeEditorPanel::show`, while the AccessKit
    // focus request becomes visible to that panel during `show`. Settle the following frame as well so
    // the mounted pane publishes that observed focus to the app-owned InteractionBus.
    harness.run_steps(2);
}

/// The same native content_json -> DocModel restore operation the mounted rich editor's unified-undo
/// bridge uses. A malformed snapshot fails closed; it never becomes an empty stand-in document.
fn rich_restore() -> RichSnapshotApplier<RichEditorState> {
    Arc::new(|state: &mut RichEditorState, snapshot| {
        state.doc = from_json_value(snapshot)
            .expect("shared rich undo snapshot must parse through the native DocModel");
    })
}

fn rich_content(state: &RichEditorState) -> String {
    to_content_json_value(&state.doc).to_string()
}

fn run_supplemental_mt046_argus_undo(scenario_id: &str, edit_rich: bool, edit_code: bool) {
    let rich_pane = pane("pane-b");
    let code_pane = pane("pane-a");
    let (mut app, _runtime) = editor_shell();
    app.set_active_pane_for_test(Some(if edit_code {
        code_pane.clone()
    } else {
        rich_pane.clone()
    }));
    let rich_doc = app.mounted_rich_state();
    let rich_before = BlockNode::doc(vec![BlockNode::paragraph("argus-note-base")]);
    let rich_after = BlockNode::doc(vec![BlockNode::paragraph("argus-note-EDITED")]);
    rich_doc.lock().unwrap().doc = rich_before.clone();
    let code_panel = app.mounted_code_panel();
    code_panel.set_text("let argus_code = 0;\n");
    let code_before = code_panel.buffer();
    let mut harness = Harness::builder()
        .proof_mt_id("MT-046")
        .with_size(egui::vec2(1100.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    if edit_rich {
        let before_json = to_content_json_value(&rich_before);
        let after_json = to_content_json_value(&rich_after);
        let bus = InteractionBus::get_or_init(&harness.ctx);
        push_rich_edit_undo(
            &mut bus.lock().unwrap(),
            rich_pane.clone(),
            &rich_doc,
            before_json,
            after_json,
            rich_restore(),
            "argus rich edit",
        );
        rich_doc.lock().unwrap().doc = rich_after.clone();
    }
    if edit_code {
        code_panel.set_text("let argus_code = 999;\n");
        let code_after = code_panel.buffer();
        handshake_native::code_editor::interop_adapter::push_code_edit_undo(
            &mut InteractionBus::get_or_init(&harness.ctx).lock().unwrap(),
            code_pane.clone(),
            &code_panel,
            code_before.clone(),
            code_after,
            "argus code edit",
        );
    }

    let focused_author = if edit_code {
        CODE_EDITOR_TEXT_AUTHOR_ID
    } else {
        "editor.rich.text"
    };
    focus_accesskit(&mut harness, focused_author);
    let mut argus = CanonicalArgusDriver::bind(harness.state(), &format!("mt046-{scenario_id}"));
    let initial = argus.inspect(&mut harness);
    assert!(json_has_author_id(&initial, "menu-edit"));
    let rich_undo_author_id = undo_count_author_id(rich_pane.as_ref());
    let code_undo_author_id = undo_count_author_id(code_pane.as_ref());
    let initial_rich_value = json_node_by_author_id(&initial, &rich_undo_author_id)
        .and_then(|node| node.get("value"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let initial_code_value = json_node_by_author_id(&initial, &code_undo_author_id)
        .and_then(|node| node.get("value"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    if edit_rich {
        assert_eq!(initial_rich_value.as_deref(), Some("Undo (1)"));
        assert_eq!(
            rich_content(&rich_doc.lock().unwrap()),
            to_content_json_value(&rich_after).to_string()
        );
    }
    if edit_code {
        assert_eq!(initial_code_value.as_deref(), Some("Undo (1)"));
        assert_eq!(code_panel.buffer().to_string(), "let argus_code = 999;\n");
    }
    argus.click_expect_applied_and_reinspect(&mut harness, "menu-edit");
    argus.assert_latest_terminal_predicate(&mut harness, "undo-menu-item-enabled", |tree| {
        json_has_author_id(tree, "menu.edit.undo")
    });
    let undo = argus.click_expect_applied_and_reinspect(&mut harness, "menu.edit.undo");
    let undo_receipt_id = undo.receipt_id;
    let expected_focused_undo_author_id = if edit_code {
        code_undo_author_id.as_str()
    } else {
        rich_undo_author_id.as_str()
    };
    let code_before_text = code_before.to_string();
    let rich_before_text = to_content_json_value(&rich_before).to_string();
    let rich_after_text = to_content_json_value(&rich_after).to_string();
    let terminal = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "exact-focused-undo-restored-state",
        serde_json::json!({
            "receipt_id": undo_receipt_id,
            "focused_author_id": focused_author,
            "focused_undo_count_author_id": expected_focused_undo_author_id,
            "focused_undo_count_initial_value": "Undo (1)",
            "focused_undo_count_terminal_value": "Undo (0)",
            "initial_code": if edit_code { Some("let argus_code = 999;\n") } else { None },
            "terminal_code": if edit_code { Some(code_before_text.as_str()) } else { None },
            "initial_rich": if edit_rich { Some(rich_after_text.as_str()) } else { None },
            "terminal_rich": if edit_rich && !edit_code { Some(rich_before_text.as_str()) } else if edit_rich { Some(rich_after_text.as_str()) } else { None },
            "ic18_retained_rich_undo_count_value": if edit_rich && edit_code { Some("Undo (1)") } else { None },
        }),
        |tree| {
            let focused_undo_restored = json_node_by_author_id(tree, expected_focused_undo_author_id)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                == Some("Undo (0)");
            let exact_receipt_applied = tree["action_receipts"]
                .as_array()
                .is_some_and(|receipts| {
                    receipts.iter().any(|receipt| {
                        receipt["receipt_id"].as_u64() == Some(undo_receipt_id)
                            && receipt["status"] == "applied"
                    })
                });
            let exact_editor_state = if edit_code {
                code_panel.buffer().to_string() == code_before_text
            } else {
                rich_content(&rich_doc.lock().unwrap()) == rich_before_text
            };
            let ic18_retained_rich = !edit_rich
                || !edit_code
                || (rich_content(&rich_doc.lock().unwrap()) == rich_after_text
                    && json_node_by_author_id(tree, &rich_undo_author_id)
                        .and_then(|node| node.get("value"))
                        .and_then(serde_json::Value::as_str)
                        == Some("Undo (1)"));
            json_has_author_id(tree, focused_author)
                && focused_undo_restored
                && exact_receipt_applied
                && exact_editor_state
                && ic18_retained_rich
        },
    );

    if edit_code {
        assert_eq!(code_panel.buffer().to_string(), code_before.to_string());
        assert_eq!(
            InteractionBus::get_or_init(&harness.ctx)
                .lock()
                .unwrap()
                .local_undo_count(&code_pane),
            0
        );
    } else {
        assert_eq!(rich_doc.lock().unwrap().doc, rich_before);
    }
    if edit_rich && edit_code {
        assert!(rich_content(&rich_doc.lock().unwrap()).contains("argus-note-EDITED"));
        assert_eq!(
            InteractionBus::get_or_init(&harness.ctx)
                .lock()
                .unwrap()
                .local_undo_count(&rich_pane),
            1
        );
    }

    let scenario_evidence = serde_json::json!({
        "schema_id": "hsk.mt046.scenario-evidence@1",
        "run_id": required_mt046_env("HANDSHAKE_ARGUS_MATRIX_RUN_ID"),
        "scenario_id": scenario_id,
        "source_sha": required_mt046_env("HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA"),
        "process_correlation_id": required_mt046_env("HANDSHAKE_PROOF_PROCESS_CORRELATION_ID"),
        "workspace_ids": [],
        "target": "menu.edit.undo",
        "receipt_id": undo_receipt_id,
        "focused_author_id": focused_author,
        "initial": {
            "code_undo_count_value": initial_code_value,
            "rich_undo_count_value": initial_rich_value,
            "code": if edit_code { Some("let argus_code = 999;\n") } else { None },
            "rich": if edit_rich { Some(rich_after_text.as_str()) } else { None },
        },
        "terminal": {
            "focused_undo_count_value": "Undo (0)",
            "code": code_panel.buffer().to_string(),
            "rich": rich_content(&rich_doc.lock().unwrap()),
            "ic18_rich_undo_count_value": if edit_rich && edit_code { Some("Undo (1)") } else { None },
        },
    });
    settle_incidental_fems_for_capture(&mut harness, scenario_id);
    let _ = harness.render_settled_proof_frame(&format!("{scenario_id} mounted undo terminal"));
    assert!(harness.last_screenshot_outcome().is_some());
    argus.finish_require_no_indeterminate();
    let proof_dir = supplemental_mt046_tree_dir(scenario_id);
    write_immutable_json(&proof_dir.join("initial-tree.json"), &initial);
    write_immutable_json(&proof_dir.join("terminal-tree.json"), &terminal);
    write_immutable_json(
        &proof_dir.join("scenario-evidence.json"),
        &scenario_evidence,
    );
    assert_no_local_artifact_dir();
}

fn required_mt046_env(name: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| panic!("MT-046 supplemental Argus proof requires {name}"))
}

fn supplemental_mt046_tree_dir(scenario_id: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(required_mt046_env("HANDSHAKE_PROOF_ARTIFACT_DIR"))
        .join(required_mt046_env("HANDSHAKE_ARGUS_MATRIX_RUN_ID"))
        .join("trees")
        .join(scenario_id)
}

fn write_immutable_json(path: &std::path::Path, value: &serde_json::Value) {
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
    file.write_all(&serde_json::to_vec_pretty(value).expect("serialize MT-046 evidence"))
        .unwrap_or_else(|error| {
            panic!(
                "write immutable MT-046 evidence {}: {error}",
                path.display()
            )
        });
    file.sync_all().unwrap_or_else(|error| {
        panic!("sync immutable MT-046 evidence {}: {error}", path.display())
    });
}

#[test]
#[ignore = "run only by MT-046 canonical supervisor with per-process matrix metadata"]
fn supplemental_mt046_argus_ic15_rich_undo() {
    run_supplemental_mt046_argus_undo("IC-15", true, false);
}

#[test]
#[ignore = "run only by MT-046 canonical supervisor with per-process matrix metadata"]
fn supplemental_mt046_argus_ic16_code_undo() {
    run_supplemental_mt046_argus_undo("IC-16", false, true);
}

#[test]
#[ignore = "run only by MT-046 canonical supervisor with per-process matrix metadata"]
fn supplemental_mt046_argus_ic18_scoped_undo() {
    run_supplemental_mt046_argus_undo("IC-18", true, true);
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-15 — Undo crosses rich editor edit (LIVE-SURREALDB PASS): persist EDIT_A, route the Ctrl+Z operation through
// the ONE shared bus undo scope, persist the restored snapshot, and GET backend authority to prove EDIT_A
// is absent. The SAME `Arc<Mutex<InteractionBus>>` instance holds the scope (CTRL-1).
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic15_undo_rich_editor() {
    let attempt = ScenarioAttempt::begin("IC-15");
    let mut be = require_live_backend();
    let backend_binding = be.owned_backend_binding_receipt();
    let rich_pane = pane("pane-b");

    let before_doc = BlockNode::doc(vec![BlockNode::paragraph("base")]);
    let after_doc = BlockNode::doc(vec![BlockNode::paragraph("base EDIT_A")]);
    let before = to_content_json_value(&before_doc);
    let after = to_content_json_value(&after_doc);
    let (app, _runtime) = editor_shell();
    let rich_doc = app.mounted_rich_state();
    rich_doc.lock().unwrap().doc = before_doc;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({
            "workspace_id": be.workspace_id,
            "title": "IC-15 durable undo",
            "content_json": before.clone(),
        }),
    );
    let doc_id = created
        .pointer("/document/rich_document_id")
        .and_then(serde_json::Value::as_str)
        .expect("IC-15: create returns rich_document_id")
        .to_owned();
    let base_version = created
        .pointer("/document/doc_version")
        .and_then(serde_json::Value::as_i64)
        .expect("IC-15: create returns doc_version");

    // Record the rich edit on the SHARED bus's unified undo scope via the REAL adapter, then apply the edit.
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let mut b = bus.lock().unwrap();
        push_rich_edit_undo(
            &mut b,
            rich_pane.clone(),
            &rich_doc,
            before.clone(),
            after.clone(),
            rich_restore(),
            "rich: insert EDIT_A",
        );
    }
    rich_doc.lock().unwrap().doc = after_doc;
    let saved_edit = be.put_json(
        &format!("/knowledge/documents/{doc_id}/save"),
        &serde_json::json!({"expected_version": base_version, "content_json": after.clone()}),
    );
    let edit_version = saved_edit
        .pointer("/document/doc_version")
        .and_then(serde_json::Value::as_i64)
        .expect("IC-15: EDIT_A save returns advanced doc_version");
    assert!(
        edit_version > base_version,
        "IC-15: real EDIT_A save advances version"
    );
    assert!(
        rich_content(&rich_doc.lock().unwrap()).contains("EDIT_A"),
        "IC-15: the edit applied (EDIT_A present)"
    );
    assert_eq!(
        InteractionBus::get_or_init(&harness.ctx)
            .lock()
            .unwrap()
            .local_undo_count(&rich_pane),
        1,
        "IC-15: one entry on the shared scope"
    );

    // Focus the REAL mounted rich editor through its AccessKit surface, then send the canonical Ctrl+Z
    // chord. The shell decodes the key command and selects the focused pane's ring; the test never calls
    // `InteractionBus::undo` directly.
    focus_accesskit(&mut harness, "editor.rich.text");
    assert_eq!(
        InteractionBus::get_or_init(&harness.ctx)
            .lock()
            .unwrap()
            .focus_owner(),
        Some(&rich_pane),
        "IC-15: AccessKit focus selects the rich pane undo scope"
    );
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run_steps(3);
    assert!(
        !rich_content(&rich_doc.lock().unwrap()).contains("EDIT_A"),
        "IC-15: after Ctrl+Z the rich content no longer contains EDIT_A (got {:?})",
        rich_content(&rich_doc.lock().unwrap())
    );
    assert_eq!(
        to_content_json_value(&rich_doc.lock().unwrap().doc),
        before,
        "IC-15: native rich undo restores the exact pre-edit DocModel snapshot"
    );
    assert_eq!(
        InteractionBus::get_or_init(&harness.ctx)
            .lock()
            .unwrap()
            .local_undo_count(&rich_pane),
        0,
        "IC-15: canonical Ctrl+Z consumed the focused rich pane entry"
    );

    let restored_content = to_content_json_value(&rich_doc.lock().unwrap().doc);
    let saved_undo = be.put_json(
        &format!("/knowledge/documents/{doc_id}/save"),
        &serde_json::json!({"expected_version": edit_version, "content_json": restored_content}),
    );
    let undo_version = saved_undo
        .pointer("/document/doc_version")
        .and_then(serde_json::Value::as_i64)
        .expect("IC-15: Ctrl+Z save returns advanced doc_version");
    assert!(
        undo_version > edit_version,
        "IC-15: durable undo save advances version"
    );
    let reloaded = be.get_json(&format!("/knowledge/documents/{doc_id}"));
    let reloaded_content = reloaded
        .pointer("/document/content_json")
        .cloned()
        .expect("IC-15: backend GET returns content_json");
    assert!(
        !reloaded_content.to_string().contains("EDIT_A"),
        "IC-15: backend authority after Ctrl+Z/save must not contain EDIT_A: {reloaded_content}"
    );

    let delete_status = be.delete(&format!("/knowledge/documents/{doc_id}"));
    assert!(
        (200..300).contains(&delete_status) || delete_status == 404,
        "IC-15: explicit document cleanup returned {delete_status}"
    );
    let runtime_diagnostics = be
        .assert_cleanup_and_publish_runtime_diagnostics("IC-15")
        .expect("IC-15: publish fixture-owned backend runtime diagnostics");
    attempt.pass(serde_json::json!({
        "backend_binding": backend_binding,
        "runtime_diagnostics": runtime_diagnostics,
        "edit": "EDIT_A",
        "ctrl_z_via_shared_bus": true,
        "backend_get_absent_after_undo": true,
        "versions": [base_version, edit_version, undo_version],
    }));
    assert_no_local_artifact_dir();
    println!("IC-15 LIVE-SURREALDB PASS: EDIT_A saved, Ctrl+Z restored, and backend GET confirms absence");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-16 — Undo crosses code editor edit (SUBSTRATE PASS): type CODE_EDIT through the mounted code
// editor's real input route, then drive Ctrl+Z through the focused AccessKit surface and shell key router.
// The app-owned InteractionBus records and drains the pane-local entry; a second Ctrl+Z proves the empty
// stack fails closed without mutating the mounted file buffer.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic16_undo_code_editor() {
    let attempt = ScenarioAttempt::begin("IC-16");
    let code_pane = pane("pane-a");
    let (app, _runtime) = editor_shell();
    let code_panel = app.mounted_code_panel();
    code_panel.set_text("fn main() {}\n");
    code_panel.set_single_cursor(0);
    let before = code_panel.buffer().to_string();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    focus_accesskit(&mut harness, CODE_EDITOR_TEXT_AUTHOR_ID);
    assert_eq!(
        InteractionBus::get_or_init(&harness.ctx)
            .lock()
            .unwrap()
            .focus_owner(),
        Some(&code_pane),
        "IC-16: AccessKit focus selects the mounted code pane"
    );

    // Apply the edit through the production text-input loop. The mounted pane factory snapshots it and
    // records the resulting restore action on the app-owned shared bus; the test never seeds undo state.
    harness.event(egui::Event::Text("CODE_EDIT\n".to_owned()));
    harness.run_steps(2);
    assert!(
        code_panel.buffer().to_string().contains("CODE_EDIT"),
        "IC-16: the mounted code text input applied CODE_EDIT"
    );
    assert_eq!(
        InteractionBus::get_or_init(&harness.ctx)
            .lock()
            .unwrap()
            .local_undo_count(&code_pane),
        1,
        "IC-16: the live edit produced one entry on the app-owned shared scope"
    );

    // Drive the canonical mounted key route; no direct InteractionBus::undo call is allowed here.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run_steps(3);
    assert!(
        !code_panel.buffer().to_string().contains("CODE_EDIT"),
        "IC-16: after Ctrl+Z the code buffer no longer contains CODE_EDIT (got {:?})",
        code_panel.buffer().to_string()
    );
    assert_eq!(
        code_panel.buffer().to_string(),
        before,
        "IC-16: the buffer is restored to its pre-edit state"
    );
    assert_eq!(
        InteractionBus::get_or_init(&harness.ctx)
            .lock()
            .unwrap()
            .local_undo_count(&code_pane),
        0,
        "IC-16: the mounted Ctrl+Z drained the code pane ring"
    );

    // Empty-stack negative path: a repeated Ctrl+Z is a visible no-op and cannot corrupt file state.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run_steps(2);
    assert_eq!(
        code_panel.buffer().to_string(),
        before,
        "IC-16: Ctrl+Z on an empty app-owned ring leaves the mounted file unchanged"
    );
    assert_eq!(
        InteractionBus::get_or_init(&harness.ctx)
            .lock()
            .unwrap()
            .local_undo_count(&code_pane),
        0,
        "IC-16: repeated empty-stack undo does not fabricate an entry"
    );

    attempt.pass(serde_json::json!({
        "edit": "CODE_EDIT",
        "mounted_accesskit_input": true,
        "ctrl_z_via_app_key_route": true,
        "absent_after_undo": true,
        "empty_stack_noop": true,
    }));
    assert_no_local_artifact_dir();
    println!(
        "IC-16 SUBSTRATE PASS: mounted CODE_EDIT reverted through AccessKit/Ctrl+Z; empty-stack retry was a no-op"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-18 — Shared undo stack policy: scoped per-pane vs. global (SUBSTRATE PASS, the load-bearing CTRL-4
// proof). Open two panes (rich note A, code file B). Edit BOTH. Trigger undo ONCE. Assert the undo reverts
// the MOST RECENTLY EDITED surface (B), NOT both: if the code edit was last, the code buffer reverts and the
// note is unchanged. This proves the SAME shared undo stack respects the per-pane scope policy (POLICY-1
// local-first). The test uses ONE `Arc<Mutex<InteractionBus>>` shared by both surfaces and inspects BOTH
// surfaces after one undo (CTRL-4) — NOT two independent undo stacks.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic18_undo_scope_policy() {
    let attempt = ScenarioAttempt::begin("IC-18");
    let rich_pane = pane("pane-b");
    let code_pane = pane("pane-a");
    let note_before = BlockNode::doc(vec![BlockNode::paragraph("noteA-base")]);
    let note_after = BlockNode::doc(vec![BlockNode::paragraph("noteA-EDITED")]);
    let note_before_json = to_content_json_value(&note_before);
    let note_after_json = to_content_json_value(&note_after);

    // Mount BOTH real editor surfaces in the native shell. Their adapters and key-command router share
    // the shell-owned InteractionBus; the test does not create a detached substitute bus.
    let (app, _runtime) = editor_shell();
    let rich_doc = app.mounted_rich_state();
    rich_doc.lock().unwrap().doc = note_before;
    let code_panel = app.mounted_code_panel();
    code_panel.set_text("let b = 0;\n");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 700.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    // Surface A = rich note. Edit it FIRST.
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let mut b = bus.lock().unwrap();
        push_rich_edit_undo(
            &mut b,
            rich_pane.clone(),
            &rich_doc,
            note_before_json,
            note_after_json.clone(),
            rich_restore(),
            "rich: edit A",
        );
    }
    rich_doc.lock().unwrap().doc = note_after;

    // Surface B = code file. Edit it SECOND (the MOST RECENTLY edited surface).
    let code_before = code_panel.buffer();
    code_panel.set_text("let b = 999;\n");
    let code_after = code_panel.buffer();
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let mut b = bus.lock().unwrap();
        handshake_native::code_editor::interop_adapter::push_code_edit_undo(
            &mut b,
            code_pane.clone(),
            &code_panel,
            code_before.clone(),
            code_after.clone(),
            "code: edit B",
        );
    }

    // Snapshots BEFORE the single undo (to prove A is untouched after).
    let note_before_undo = to_content_json_value(&rich_doc.lock().unwrap().doc);
    let code_before_undo = code_panel.buffer().to_string();
    assert_eq!(
        note_before_undo, note_after_json,
        "IC-18: note A was edited"
    );
    assert!(
        code_before_undo.contains("999"),
        "IC-18: code B was edited (most recent)"
    );

    // Focus the REAL mounted code editor through AccessKit, then dispatch ONE canonical Ctrl+Z chord.
    // The native shell selects pane B's local-first ring from focus ownership.
    focus_accesskit(&mut harness, CODE_EDITOR_TEXT_AUTHOR_ID);
    assert_eq!(
        InteractionBus::get_or_init(&harness.ctx)
            .lock()
            .unwrap()
            .focus_owner(),
        Some(&code_pane),
        "IC-18: AccessKit focus selects the code pane undo scope"
    );
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run_steps(3);

    // INSPECT BOTH surfaces after the one undo (CTRL-4):
    //  - B (the most recently edited) is reverted.
    let code_after_undo = code_panel.buffer().to_string();
    assert_eq!(
        code_after_undo,
        code_before.to_string(),
        "IC-18: ONE undo reverted the MOST RECENTLY edited surface (code B): got {code_after_undo:?}"
    );
    //  - A (the note) is UNCHANGED (POLICY-1 per-pane scope — the undo did not touch A's ring).
    assert_eq!(
        to_content_json_value(&rich_doc.lock().unwrap().doc),
        note_after_json,
        "IC-18: the OTHER surface (note A) is UNCHANGED after the single undo (per-pane scope policy)"
    );
    // The code ring drained; the note ring still has its entry (proving they are distinct per-pane rings on
    // the ONE shared scope, not a single global stack).
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let b = bus.lock().unwrap();
        assert_eq!(
            b.local_undo_count(&code_pane),
            0,
            "IC-18: the code pane's ring drained"
        );
        assert_eq!(
            b.local_undo_count(&rich_pane),
            1,
            "IC-18: the note pane's ring is untouched"
        );
    }

    attempt.pass(serde_json::json!({"policy": "local_first", "one_undo_reverted": "code_pane_b"}));
    assert_no_local_artifact_dir();
    println!(
        "IC-18 SUBSTRATE PASS: scope correct — ONE undo on the shared stack reverted ONLY the most recently \
         edited surface (code B); note A unchanged (per-pane scope policy)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// IC-17 — Event-ledger records cross-surface actions. A sequence (insert text, save, insert wikilink,
// save again) produces >=2
// KNOWLEDGE_RICH_DOCUMENT_SAVED events in the ledger, in order; the second event's payload carries the
// wikilink block reference.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn interconnect_ic17_event_ledger_records() {
    let attempt = ScenarioAttempt::begin("IC-17");
    use handshake_native::rich_editor::document_model::doc_json::to_content_json_value;
    use handshake_native::rich_editor::document_model::node::{
        BlockNode, Child, HsLinkNode, NodeKind, TextLeaf,
    };

    let mut be = require_live_backend();
    let ws = be.workspace_id.clone();
    let backend_binding = be.owned_backend_binding_receipt();

    // (1) create a note, (2) save it (insert text), (3) add a wikilink, (4) save again.
    let plain = BlockNode::doc(vec![BlockNode::paragraph("event ledger note")]);
    // Knowledge docs are merged BARE (no /workspaces prefix): POST /knowledge/documents (workspace_id in
    // body), PUT /knowledge/documents/{id}/save ({expected_version,content_json}); the create response wraps
    // the doc as { "document": { rich_document_id, doc_version, .. }, .. } (verified knowledge_documents.rs).
    let created = be.post_json(
        "/knowledge/documents",
        &serde_json::json!({ "workspace_id": ws, "title": "IC-17 note",
            "content_json": to_content_json_value(&plain) }),
    );
    let doc_id = created
        .get("document")
        .and_then(|d| d.get("rich_document_id"))
        .and_then(|v| v.as_str())
        .or_else(|| created.get("rich_document_id").and_then(|v| v.as_str()))
        .or_else(|| created.get("id").and_then(|v| v.as_str()))
        .expect(
            "requires_surrealdb: created document returns a rich_document_id (document.rich_document_id)",
        )
        .to_owned();
    let mut version = created
        .get("document")
        .and_then(|d| d.get("doc_version"))
        .and_then(|v| v.as_i64())
        .or_else(|| created.get("doc_version").and_then(|v| v.as_i64()))
        .unwrap_or(1);
    let note_block_id = created
        .get("document")
        .and_then(|d| d.get("block_id").or_else(|| d.get("loom_block_id")))
        .and_then(|v| v.as_str())
        .unwrap_or(&doc_id)
        .to_owned();
    let linked = be.post_json(
        &format!("/workspaces/{ws}/loom/blocks"),
        &serde_json::json!({
            "content_type": "file",
            "title": "IC-17 linked code block"
        }),
    );
    let linked_block_id = linked["block_id"]
        .as_str()
        .expect("IC-17: real linked block id")
        .to_owned();

    // Save #1 (plain) via the REAL /save route.
    let save1 = be.put_json(
        &format!("/knowledge/documents/{doc_id}/save"),
        &serde_json::json!({ "expected_version": version, "content_json": to_content_json_value(&plain) }),
    );
    // Advance the optimistic-concurrency version from the save response so save #2 is accepted.
    version = save1
        .get("document")
        .and_then(|d| d.get("doc_version"))
        .and_then(|v| v.as_i64())
        .or_else(|| save1.get("doc_version").and_then(|v| v.as_i64()))
        .unwrap_or(version + 1);
    let save1_event_id = save1["save_receipt_event_id"]
        .as_str()
        .expect("IC-17: first save returns its EventLedger receipt id")
        .to_owned();
    // Save #2 (with a wikilink ref in the body).
    let mut para = BlockNode::new(NodeKind::Paragraph);
    para.children.push(Child::Text(TextLeaf::new("now links ")));
    para.children.push(Child::HsLink(HsLinkNode::new(
        "file",
        &linked_block_id,
        "linked",
    )));
    let with_link = BlockNode::doc(vec![para]);
    let save2 = be.put_json(
        &format!("/knowledge/documents/{doc_id}/save"),
        &serde_json::json!({ "expected_version": version, "content_json": to_content_json_value(&with_link) }),
    );
    let save2_event_id = save2["save_receipt_event_id"]
        .as_str()
        .expect("IC-17: second save returns its EventLedger receipt id")
        .to_owned();

    // Read kernel EventLedger authority for the exact document aggregate. Flight Recorder is a separate
    // projection and is intentionally not used as proof of document-save receipts.
    let events = be.get_json(&format!(
        "/kernel/events/aggregates/knowledge_rich_document/{doc_id}"
    ));
    let arr = events.as_array().cloned().unwrap_or_default();
    let saved: Vec<&serde_json::Value> = arr
        .iter()
        .filter(|e| {
            let kind = e["event_type"].as_str().unwrap_or("");
            kind.to_uppercase()
                .contains("KNOWLEDGE_RICH_DOCUMENT_SAVED")
                && e["aggregate_id"].as_str() == Some(doc_id.as_str())
        })
        .collect();
    assert!(
        saved.len() >= 2,
        "IC-17: aggregate EventLedger returns >= 2 KNOWLEDGE_RICH_DOCUMENT_SAVED events for the note (got {})",
        saved.len()
    );
    // AC: the SECOND save event's payload carries the wikilink block reference (linked-block-1). The two
    // saves were issued in order, so the later matching event is the wikilink save. Order the matches by
    // timestamp when present so the assertion does not depend on the server's return order.
    let first = saved
        .iter()
        .find(|event| event["event_id"].as_str() == Some(save1_event_id.as_str()))
        .expect("IC-17: first response receipt is readable from EventLedger");
    let second = saved
        .iter()
        .find(|event| event["event_id"].as_str() == Some(save2_event_id.as_str()))
        .expect("IC-17: second response receipt is readable from EventLedger");
    let first_sequence = first["event_sequence"]
        .as_i64()
        .expect("IC-17: first receipt exposes event_sequence");
    let second_sequence = second["event_sequence"]
        .as_i64()
        .expect("IC-17: second receipt exposes event_sequence");
    assert!(
        first_sequence < second_sequence,
        "IC-17: first save receipt precedes the second in EventLedger order"
    );
    assert!(
        second["payload"]["reference_targets"]
            .as_array()
            .is_some_and(|targets| {
                targets
                    .iter()
                    .any(|target| target.as_str() == Some(linked_block_id.as_str()))
            }),
        "IC-17: the second save receipt carries the exact wikilink block reference (got {second})"
    );
    let negative =
        be.get_json("/kernel/events/aggregates/knowledge_rich_document/KRD-ic17-missing");
    assert!(
        negative.as_array().is_some_and(Vec::is_empty),
        "IC-17: missing aggregate yields no fabricated EventLedger rows"
    );

    let _ = be.delete(&format!("/knowledge/documents/{doc_id}"));
    let _ = be.delete(&format!("/workspaces/{ws}/loom/blocks/{linked_block_id}"));
    let runtime_diagnostics = be
        .assert_cleanup_and_publish_runtime_diagnostics("IC-17")
        .expect("IC-17: publish fixture-owned backend runtime diagnostics");
    attempt.pass(serde_json::json!({
        "backend_binding": backend_binding,
        "runtime_diagnostics": runtime_diagnostics,
        "workspace_id": ws,
        "document_id": doc_id,
        "note_block_id": note_block_id,
        "linked_block_id": linked_block_id,
        "event_ledger_event_ids": [save1_event_id, save2_event_id],
        "negative_missing_aggregate_count": 0,
    }));
    println!(
        "IC-17 LIVE-SURREALDB PASS: {} KNOWLEDGE_RICH_DOCUMENT_SAVED events recorded for the note",
        saved.len()
    );
}

// ── Hygiene guard (runs in the default suite). ────────────────────────────────────────────────────────

#[test]
fn no_local_artifact_dir_edge4() {
    assert_no_local_artifact_dir();
    println!("CX-212E: no repo-local artifact dir under the crate (edge 4)");
}
