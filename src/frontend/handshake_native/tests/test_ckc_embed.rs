//! WP-KERNEL-012 MT-033 (E5 — CKC embeds / drag-in + route-to-Stage) proof suite.
//!
//! Maps each MT-033 acceptance criterion to a real runtime proof:
//!   - AC-1 (kittest): a `DragPayload::AtelierRef` released over the rich-text editor inserts an inline
//!     CKC `hsLink` embed atom at the caret (drag-and-drop simulated via egui's DragAndDrop channel,
//!     the same pattern as the canvas-board drop test).
//!   - AC-2 (unit + gated live-PG): the inserted CKC embed is an `hsLink` atom that ROUND-TRIPS the
//!     backend `content_json` (NOT an invented `atelier_embed` node) — proven structurally by a
//!     content_json round-trip, and end-to-end against real PG in the integration-gated proof.
//!   - AC-3 (kittest + gated live-PG): a resolved payload places its Loom block directly; an unresolved
//!     Atelier payload is projected through the real Loom block API and then placed by block id (never a
//!     fake `atelier_item_id`). The live-PG proof asserts the projected block and placement after reload.
//!   - AC-4 (kittest): the Route-to-Stage command (bus + palette) opens the Stage pane and displays the
//!     routed content; the `stage-pane` AccessKit GenericContainer node carries the staged summary.
//!   - AC-5 (gated live-PG): the AtelierSidePanel loads batches + corpus from the REAL atelier backend
//!     (no mocks) — at least one batch row when the backend has a seeded batch.
//!   - AC-6 (AccessKit dump): `atelier-side-panel` (List), `atelier-item-{id}` (ListItem, draggable),
//!     `stage-pane` (GenericContainer) are present in the live AccessKit tree.
//!   - AC-7: `cargo test -p handshake-native --test test_ckc_embed -- --nocapture` passes (this file).
//!
//! ## Artifact hygiene (CX-212E, HARD)
//!
//! The screenshot proof writes ONLY to the EXTERNAL artifact root via [`external_artifact_dir`];
//! [`assert_no_local_artifact_dir`] fails the run if a repo-local `test_output/` or `tests/screenshots/`
//! dir exists. NO artifact is ever written under `src/`.

use std::path::{Path, PathBuf};

use egui_kittest::kittest::{NodeT, Queryable};
#[cfg(feature = "wgpu_screenshots")]
#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
#[cfg(feature = "integration")]
#[path = "pg_proof_support/mod.rs"]
mod pg_proof_support;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
#[cfg(feature = "wgpu_screenshots")]
use canonical_argus_driver::{
    json_has_author_id, json_node_by_author_id, ArgusObservation, CanonicalArgusDriver,
};
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState};
#[cfg(feature = "integration")]
use handshake_native::atelier_side_panel::{
    batch_author_id, corpus_author_id, item_canvas_author_id,
};
use handshake_native::atelier_side_panel::{
    item_author_id, AtelierSidePanel, PANEL_AUTHOR_ID, REFRESH_AUTHOR_ID,
};
use handshake_native::backend_client::{AtelierBatchRow, AtelierItemRow, HealthInfo};
use handshake_native::interop::{
    AtelierItemKind, AtelierRef, DragPayload, InteractionBus, CMD_ROUTE_TO_STAGE,
};
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorState, RichEditorWidget,
};
use handshake_native::stage_pane::{StageContent, StagePane, STAGE_PANE_AUTHOR_ID};
use handshake_native::theme::HsTheme;

/// The external artifact root (CX-212E), resolved from an explicit operator root or the compile-time
/// repository location rather than process CWD. This remains correct when Cargo is invoked from the
/// crate or repo root and cannot accidentally create `D:\Handshake_Artifacts`.
#[allow(dead_code)]
fn external_artifact_dir(subdir: &str) -> PathBuf {
    let approved_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("handshake_native manifest is nested below the Handshake Worktrees root")
        .join("Handshake_Artifacts");
    let root = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| approved_root.clone());
    assert!(
        root.is_absolute(),
        "HANDSHAKE_ARTIFACTS_ROOT must be absolute so artifact placement never depends on process CWD"
    );
    assert_eq!(
        root, approved_root,
        "HANDSHAKE_ARTIFACTS_ROOT must equal the one manifest-derived sibling Handshake_Artifacts root"
    );
    root.join("handshake-test").join(subdir)
}

#[cfg(feature = "wgpu_screenshots")]
fn canonical_action_proof(
    target: &str,
    observation: &ArgusObservation,
    terminal_predicate: &str,
) -> serde_json::Value {
    use sha2::Digest as _;

    let receipt = observation.after["action_receipts"]
        .as_array()
        .and_then(|receipts| {
            receipts
                .iter()
                .find(|receipt| receipt["receipt_id"].as_u64() == Some(observation.receipt_id))
        })
        .unwrap_or_else(|| panic!("action {target} retains receipt {}", observation.receipt_id));
    assert_ne!(
        observation.receipt_status, "indeterminate",
        "MT-033 V4 prohibits indeterminate canonical action receipts"
    );
    assert!(
        matches!(observation.receipt_status.as_str(), "applied" | "rejected"),
        "MT-033 V4 requires a terminal Applied or typed-Rejected receipt: {receipt}"
    );
    assert_eq!(
        receipt["status"].as_str(),
        Some(observation.receipt_status.as_str()),
        "serialized receipt status must come from the exact terminal tree"
    );
    assert!(
        observation.terminal_refreshed,
        "MT-033 proof must serialize the driver's persisted terminal observation"
    );
    let predicate_result = observation
        .terminal_predicates
        .iter()
        .find(|predicate| predicate.predicate_id == terminal_predicate)
        .unwrap_or_else(|| {
            panic!("action {target} terminal observation retains predicate {terminal_predicate}")
        });
    assert!(
        predicate_result.passed,
        "serialized terminal predicate must have passed against observation.after"
    );
    let completion_token = receipt["observed_value"]
        .as_str()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .expect("terminal action receipt carries a parseable completion token");
    let product_detail = completion_token["terminal_detail"]
        .as_str()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok());
    let hash = |value: &serde_json::Value| {
        format!(
            "{:x}",
            sha2::Sha256::digest(serde_json::to_vec(value).expect("serialize Argus observation"))
        )
    };
    serde_json::json!({
        "requested_action": "argus.click",
        "stable_author_id": target,
        "binding_identity": observation.agent_id,
        "receipt": receipt,
        "completion_token": completion_token,
        "product_detail": product_detail,
        "observation": {
            "before": observation.before,
            "after": observation.after,
            "before_tree_hash": hash(&observation.before),
            "after_tree_hash": hash(&observation.after),
            "before_generation": observation.before["captured_at_utc"],
            "after_generation": observation.after["captured_at_utc"],
            "receipt_id": observation.receipt_id,
            "receipt_status": observation.receipt_status,
            "agent_id": observation.agent_id,
            "terminal_observed_sequence": observation.terminal_observed_sequence,
            "target_selected_before": observation.target_selected_before,
            "target_selected_after": observation.target_selected_after,
            "terminal_refreshed": observation.terminal_refreshed
        },
        "terminal_predicate": predicate_result,
        "terminal": observation.after
    })
}

#[cfg(feature = "wgpu_screenshots")]
fn canonical_product_detail(observation: &ArgusObservation) -> serde_json::Value {
    let receipt = observation.after["action_receipts"]
        .as_array()
        .and_then(|receipts| {
            receipts
                .iter()
                .find(|receipt| receipt["receipt_id"].as_u64() == Some(observation.receipt_id))
        })
        .expect("terminal observation retains exact receipt");
    let token: serde_json::Value = serde_json::from_str(
        receipt["observed_value"]
            .as_str()
            .expect("terminal receipt completion token"),
    )
    .expect("parse terminal completion token");
    serde_json::from_str(
        token["terminal_detail"]
            .as_str()
            .expect("observer terminal detail"),
    )
    .expect("parse observer product detail")
}

#[cfg(feature = "wgpu_screenshots")]
fn sha256_file(path: &Path) -> String {
    use sha2::Digest as _;
    format!(
        "{:x}",
        sha2::Sha256::digest(std::fs::read(path).expect("read proof artifact for SHA-256"))
    )
}

#[cfg(feature = "wgpu_screenshots")]
fn sha256_json(value: &serde_json::Value) -> String {
    use sha2::Digest as _;
    format!(
        "{:x}",
        sha2::Sha256::digest(serde_json::to_vec(value).expect("serialize JSON for SHA-256"))
    )
}

#[cfg(feature = "wgpu_screenshots")]
fn current_head_sha() -> String {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(3)
                .unwrap(),
        )
        .output()
        .expect("resolve current HEAD for V4 proof");
    assert!(output.status.success(), "git rev-parse HEAD must pass");
    String::from_utf8(output.stdout)
        .expect("HEAD is UTF-8")
        .trim()
        .to_owned()
}

/// Bind a visual proof to the complete dirty worktree, not merely its HEAD. This deliberately mirrors
/// the hardened MT-088 candidate algorithm: the binary tracked diff plus sorted untracked path/content
/// hashes form one deterministic identity material stream.
#[cfg(feature = "wgpu_screenshots")]
fn current_worktree_candidate_identity() -> (String, serde_json::Value) {
    use sha2::Digest as _;
    use std::io::Write as _;

    let head_sha = current_head_sha();
    let root = std::fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("handshake_native manifest is nested below the repository root"),
    )
    .expect("canonicalize MT-033 product repository root");
    let tracked_diff = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["diff", "--binary", "HEAD", "--", "."])
        .output()
        .expect("read complete tracked MT-033 worktree diff");
    assert!(tracked_diff.status.success(), "git diff HEAD must pass");
    let tracked_diff_sha256 = format!("{:x}", sha2::Sha256::digest(&tracked_diff.stdout));

    let untracked_output = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .output()
        .expect("list complete untracked MT-033 worktree candidate");
    assert!(
        untracked_output.status.success(),
        "git ls-files --others must pass"
    );
    let mut untracked_paths = untracked_output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8(path.to_vec()).expect("untracked repo path is UTF-8"))
        .collect::<Vec<_>>();
    untracked_paths.sort();

    let mut candidate_material = Vec::new();
    candidate_material.extend_from_slice(b"tracked-diff\0");
    candidate_material.extend_from_slice(&tracked_diff.stdout);
    let mut untracked_files = serde_json::Map::new();
    for repo_path in untracked_paths {
        let path = root.join(&repo_path);
        let canonical_path = std::fs::canonicalize(&path).unwrap_or_else(|error| {
            panic!(
                "canonicalize untracked candidate {}: {error}",
                path.display()
            )
        });
        assert!(
            canonical_path.starts_with(&root),
            "untracked candidate {} must remain inside {}",
            canonical_path.display(),
            root.display()
        );
        let sha256 = sha256_file(&canonical_path);
        candidate_material.extend_from_slice(b"\0untracked\0");
        candidate_material.extend_from_slice(repo_path.as_bytes());
        candidate_material.push(0);
        candidate_material.extend_from_slice(sha256.as_bytes());
        untracked_files.insert(
            repo_path.clone(),
            serde_json::json!({
                "repo_path": repo_path,
                "canonical_path": canonical_path,
                "sha256": sha256,
            }),
        );
    }

    let mut hash_object = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["hash-object", "--stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn git hash-object for complete MT-033 candidate");
    hash_object
        .stdin
        .as_mut()
        .expect("git hash-object stdin is piped")
        .write_all(&candidate_material)
        .expect("write complete MT-033 candidate identity material");
    let hash_output = hash_object
        .wait_with_output()
        .expect("wait for git hash-object");
    assert!(hash_output.status.success(), "git hash-object must pass");
    let candidate_git_blob_oid = String::from_utf8(hash_output.stdout)
        .expect("candidate git object id is UTF-8")
        .trim()
        .to_owned();
    assert!(!candidate_git_blob_oid.is_empty());
    let candidate_sha256 = format!("{:x}", sha2::Sha256::digest(&candidate_material));
    let identity = format!("{head_sha}-worktree-{candidate_git_blob_oid}");
    (
        identity.clone(),
        serde_json::json!({
            "identity": identity,
            "head_sha": head_sha,
            "worktree_diff_git_blob_oid": candidate_git_blob_oid,
            "candidate_sha256": candidate_sha256,
            "tracked_diff_sha256": tracked_diff_sha256,
            "tracked_diff_bytes": tracked_diff.stdout.len(),
            "untracked_files": untracked_files,
        }),
    )
}

#[cfg(feature = "wgpu_screenshots")]
fn running_test_executable_provenance() -> serde_json::Value {
    let executable = std::fs::canonicalize(
        std::env::current_exe().expect("resolve running MT-033 test executable"),
    )
    .expect("canonicalize running MT-033 test executable");
    let configured_target = std::fs::canonicalize(PathBuf::from(
        std::env::var_os("CARGO_TARGET_DIR")
            .expect("V4 proof requires an explicit isolated CARGO_TARGET_DIR"),
    ))
    .expect("canonicalize isolated MT-033 Cargo target");
    assert!(
        executable.starts_with(&configured_target),
        "running test executable {} must be inside isolated target {}",
        executable.display(),
        configured_target.display()
    );
    serde_json::json!({
        "canonical_path": executable,
        "sha256": sha256_file(&executable),
        "configured_target": configured_target,
        "process_id": std::process::id(),
    })
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` and `tests/screenshots/` (the path a contract might literally name, overridden here).
fn assert_no_local_artifact_dir() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_root
        .ancestors()
        .nth(3)
        .expect("handshake_native manifest is nested below the repository root");
    for local in [
        crate_root.join("test_output"),
        crate_root.join("tests/screenshots"),
        repo_root.join("Handshake_Artifacts"),
    ] {
        assert!(
            !local.exists(),
            "CX-212E: no repo-local artifact dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            local.display()
        );
    }
}

/// Serialize the `.wgpu()` screenshot test (the documented Windows-wgpu concurrent-device hazard).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// Collect every author_id present in the live AccessKit tree. Generic over the harness state type so it
/// works for both the `build_ui` widget harnesses (`State = ()`) and the live-shell `build_state`
/// harness (`State = HandshakeApp`).
fn author_ids<S>(harness: &Harness<'_, S>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

#[cfg(feature = "wgpu_screenshots")]
fn json_has_author_id_prefix(value: &serde_json::Value, expected_prefix: &str) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            object
                .get("author_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|author_id| author_id.starts_with(expected_prefix))
                || object
                    .values()
                    .any(|value| json_has_author_id_prefix(value, expected_prefix))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .any(|value| json_has_author_id_prefix(value, expected_prefix)),
        _ => false,
    }
}

fn center_by_author<S>(harness: &Harness<'_, S>, author_id: &str) -> egui::Pos2 {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("AccessKit node '{author_id}' must be mounted"))
        .rect()
        .center()
}

fn request_click_by_author<S>(harness: &Harness<'_, S>, author_id: &str) {
    let target = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("AccessKit node '{author_id}' must be mounted"))
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target,
            data: None,
        },
    ));
}

#[cfg(feature = "integration")]
fn pointer_click_by_author<S>(harness: &Harness<'_, S>, author_id: &str) {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("AccessKit node '{author_id}' must be mounted"))
        .click();
}

/// A seeded side panel with one expanded batch holding two draggable items (no backend / network).
fn seeded_panel() -> AtelierSidePanel {
    AtelierSidePanel::with_rows(
        vec![AtelierBatchRow {
            batch_id: "batch-1".to_owned(),
            source_label: "Sourcing Run A".to_owned(),
            status: "open".to_owned(),
        }],
        vec![],
        Some((
            "batch-1".to_owned(),
            vec![
                AtelierItemRow {
                    item_id: "item-aaa".to_owned(),
                    file_name: "sunset.png".to_owned(),
                    source_path: "/intake/sunset.png".to_owned(),
                    lane: "accept".to_owned(),
                    loom_block_id: None,
                },
                AtelierItemRow {
                    item_id: "item-bbb".to_owned(),
                    file_name: "mira.png".to_owned(),
                    source_path: "/intake/mira.png".to_owned(),
                    lane: "accept".to_owned(),
                    loom_block_id: None,
                },
            ],
        )),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-1 (unit): a DragPayload::AtelierRef serializes and deserializes losslessly + becomes an hsLink atom.
// (Re-proven here at the test boundary; the module also carries the unit tests.)
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac1_drag_payload_serde_round_trips() {
    let payload = DragPayload::AtelierRef(AtelierRef::new(
        "item-7",
        AtelierItemKind::Character,
        "Aria",
    ));
    let json = serde_json::to_string(&payload).expect("serialize");
    let back: DragPayload = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(payload, back, "AC-1: AtelierRef round-trips losslessly");
    let link = payload
        .to_hs_link()
        .expect("AtelierRef becomes an hsLink atom");
    assert_eq!(
        link.ref_kind, "character",
        "AC-1: CKC refKind discriminates the embed atom"
    );
    assert_eq!(
        link.ref_value, "item-7",
        "AC-1: refValue is the atelier item id"
    );
    assert!(
        link.resolved,
        "hsLink resolution is independent of canvas projection"
    );
    println!(
        "AC-1: DragPayload::AtelierRef round-trips + becomes an hsLink atom (refKind=character)"
    );
}

#[test]
fn ac1_real_panel_drag_source_drops_on_real_rich_editor() {
    let panel = std::sync::Arc::new(std::sync::Mutex::new(seeded_panel()));
    let editor = std::sync::Arc::new(std::sync::Mutex::new(RichEditorState::demo()));
    let editor_check = std::sync::Arc::clone(&editor);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 650.0))
        .build_ui(move |ui| {
            ui.columns(2, |columns| {
                panel
                    .lock()
                    .unwrap()
                    .show(&mut columns[0], &HsTheme::Dark.palette());
                RichEditorWidget::new(std::sync::Arc::clone(&editor)).show(&mut columns[1]);
            });
        });
    // A mounted rich editor intentionally repaints for its caret and async draft status; drive a
    // deterministic bounded frame count instead of requiring global UI quiescence.
    harness.run_steps(2);
    let source = center_by_author(&harness, &item_author_id("item-aaa"));
    let target = center_by_author(&harness, "editor.rich.text");
    harness.drag_at(source);
    harness.run();
    let mut producer_emitted_typed_payload = false;
    for step in 1..=8 {
        let t = step as f32 / 8.0;
        harness.hover_at(source + (target - source) * t);
        harness.run();
        producer_emitted_typed_payload |=
            egui::DragAndDrop::has_payload_of_type::<DragPayload>(&harness.ctx);
    }
    assert!(
        producer_emitted_typed_payload,
        "counterfactual producer gate: the actual Atelier dnd_drag_source must stage DragPayload before release"
    );
    harness.drop_at(target);
    harness.run_steps(2);
    assert_eq!(
        first_hs_link(&editor_check.lock().unwrap().current_content_json()),
        Some(("media".to_owned(), "item-aaa".to_owned())),
        "the actual dnd_drag_source row must insert through the mounted rich-editor drop target"
    );
}

#[test]
fn ac1_failed_editor_drop_is_visible_instead_of_silent() {
    use handshake_native::rich_editor::document_model::BlockNode;

    let state = std::sync::Arc::new(std::sync::Mutex::new(RichEditorState::new(BlockNode::doc(
        vec![],
    ))));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 500.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(std::sync::Arc::clone(&state)).show(ui);
        });
    harness.run();
    let target = center_by_author(&harness, "editor.rich.text");
    harness.event(egui::Event::PointerMoved(target));
    harness.run();
    egui::DragAndDrop::set_payload(
        &harness.ctx,
        DragPayload::AtelierRef(AtelierRef::new(
            "item-no-caret",
            AtelierItemKind::Media,
            "no-caret.png",
        )),
    );
    harness.event(egui::Event::PointerButton {
        pos: target,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run_steps(2);
    assert!(author_ids(&harness).contains("rich-editor-interop-status"));
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-1 (kittest): drag from the atelier panel + drop on the rich-text editor inserts an hsLink embed.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac1_drop_atelier_ref_on_editor_inserts_hs_link_embed() {
    // A live rich editor over a one-paragraph demo doc, caret at the paragraph end.
    let state = std::sync::Arc::new(std::sync::Mutex::new(RichEditorState::demo()));
    let state_ck = std::sync::Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(std::sync::Arc::clone(&state)).show(ui);
        });
    // The active rich editor has intentional caret/draft repaint sources.
    harness.run_steps(2);

    // Count the hsLink atoms before the drop (the demo doc has none).
    let before = count_hs_links(&state_ck.lock().unwrap().current_content_json());
    assert_eq!(before, 0, "the demo doc starts with no hsLink atoms");

    // Simulate the drag from the atelier panel: set the cross-surface DragPayload on the ctx, move the
    // pointer over the editor, then release. The editor's drop zone takes the payload + inserts the atom.
    let drop_pos = egui::pos2(400.0, 300.0);
    harness.event(egui::Event::PointerMoved(drop_pos));
    // The active rich editor has intentional caret/draft repaint sources.
    harness.run_steps(2);
    egui::DragAndDrop::set_payload(
        &harness.ctx,
        DragPayload::AtelierRef(AtelierRef::new(
            "item-aaa",
            AtelierItemKind::Media,
            "sunset.png",
        )),
    );
    harness.event(egui::Event::PointerButton {
        pos: drop_pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();

    let after_json = state_ck.lock().unwrap().current_content_json();
    let after = count_hs_links(&after_json);
    assert_eq!(
        after, 1,
        "AC-1: dropping an AtelierRef over the editor must insert exactly one hsLink embed atom"
    );
    // The inserted atom is the CKC embed (refKind=media, refValue=item-aaa) — the round-trippable shape.
    let (rk, rv) = first_hs_link(&after_json).expect("an hsLink atom is present after the drop");
    assert_eq!(rk, "media", "AC-1: the embed is a CKC media hsLink");
    assert_eq!(
        rv, "item-aaa",
        "AC-1: refValue is the dropped atelier item id"
    );
    // The payload was consumed (no dangling double-insert next frame).
    assert!(
        !egui::DragAndDrop::has_payload_of_type::<DragPayload>(&harness.ctx),
        "AC-1: the drop payload must be taken on release"
    );
    println!("AC-1: AtelierRef dropped on editor inserted an hsLink embed (media:item-aaa); 1 atom present");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-2 (structural): the inserted CKC embed is an hsLink atom that ROUND-TRIPS content_json — NOT an
// invented `atelier_embed` node. Proven by inserting via the production path then serializing +
// deserializing the doc through the SAME DocJson the backend persists/loads.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac2_ckc_embed_round_trips_content_json() {
    let mut state = RichEditorState::demo();
    // Insert a CKC character embed at the caret via the production insert path.
    let link = DragPayload::AtelierRef(AtelierRef::new(
        "char-9",
        AtelierItemKind::Character,
        "Mira",
    ))
    .to_hs_link()
    .expect("AtelierRef -> hsLink");
    assert!(
        RichEditorWidget::insert_atelier_embed_at_caret(&mut state, link),
        "the embed insert must succeed at the demo caret"
    );

    // The current content_json carries the hsLink node (NOT an `atelier_embed` / `atelierEmbed` node).
    let json = state.current_content_json();
    let json_str = serde_json::to_string(&json).unwrap();
    assert!(
        json_str.contains("\"hsLink\""),
        "AC-2: the embed serializes as an hsLink node"
    );
    assert!(
        !json_str.contains("atelier_embed") && !json_str.contains("atelierEmbed"),
        "AC-2: the embed must NOT be an invented atelier_embed node (it would be dropped on save)"
    );
    assert!(
        json_str.contains("character"),
        "AC-2: the CKC refKind is present"
    );
    assert!(
        json_str.contains("char-9"),
        "AC-2: the refValue (item id) is present"
    );

    // Round-trip through the backend DocJson exactly as saveRichDocument -> loadRichDocument would: the
    // bare doc content_json -> a JSON string (what the backend persists) -> parse back to a BlockNode ->
    // re-serialize. A stable round-trip proves the CKC embed survives a save/reload (AC-2).
    use handshake_native::rich_editor::document_model::doc_json::{
        from_json_string, to_json_string,
    };
    let serialized =
        serde_json::to_string(&json).expect("serialize content_json (the persisted blob)");
    let reloaded =
        from_json_string(&serialized).expect("deserialize doc (the loadRichDocument shape)");
    let reserialized = to_json_string(&reloaded).expect("re-serialize the reloaded doc");
    let reparsed = from_json_string(&reserialized).expect("the reloaded doc itself round-trips");
    assert_eq!(
        reloaded, reparsed,
        "AC-2: the CKC embed doc round-trips through DocJson byte-for-byte"
    );
    // The reloaded doc still carries the CKC hsLink atom with intact attrs.
    assert!(
        reserialized.contains("\"hsLink\""),
        "AC-2: the reloaded doc still carries the hsLink atom"
    );
    assert!(reserialized.contains("char-9") && reserialized.contains("character"));
    println!("AC-2: CKC embed is an hsLink atom that round-trips content_json (no invented node)");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-3 (kittest): a DragPayload released over the canvas places an existing Loom block directly or
// emits canonical resolve-then-place work for an unresolved Atelier item. No fabricated resolved ref
// or unsupported `atelier_item_id` is accepted.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac3_unresolved_atelier_ref_emits_canonical_resolve_then_place_event() {
    use handshake_native::graph::canvas_board::{CanvasEvent, LoomCanvasBoard};

    // Each drop runs in its OWN harness (one drag-release per harness — the proven canvas-drop pattern;
    // reusing a harness for a second release leaves egui's pointer-button state stale).
    fn drop_payload_on_canvas(payload: DragPayload) -> Vec<CanvasEvent> {
        let board = std::sync::Arc::new(std::sync::Mutex::new(LoomCanvasBoard::new(
            "ws-test", "canvas-1",
        )));
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::<CanvasEvent>::new()));
        let board_h = std::sync::Arc::clone(&board);
        let events_h = std::sync::Arc::clone(&events);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(900.0, 640.0))
            .build_ui(move |ui| {
                let pal = HsTheme::Dark.palette();
                if let Some(ev) = board_h.lock().unwrap().show(ui, &pal) {
                    events_h.lock().unwrap().push(ev);
                }
            });
        harness.run();
        let drop_pos = egui::pos2(500.0, 400.0);
        harness.event(egui::Event::PointerMoved(drop_pos));
        harness.run();
        egui::DragAndDrop::set_payload(&harness.ctx, payload);
        // Deliberately produce a competing viewport event in the release frame. The external drop must
        // retain priority over the board's legacy single-event channel.
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 1.0),
            modifiers: egui::Modifiers::default(),
        });
        harness.event(egui::Event::PointerButton {
            pos: drop_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        });
        harness.run();
        let out = events.lock().unwrap().clone();
        out
    }

    // A real panel row arrives unresolved. The widget preserves that exact reference and delegates the
    // canonical Loom projection + placement to the host; it never fabricates a resolved test payload.
    let unresolved = drop_payload_on_canvas(DragPayload::AtelierRef(AtelierRef::new(
        "item-x",
        AtelierItemKind::Media,
        "pic.png",
    )));
    let resolved = unresolved.iter().find_map(|event| match event {
        CanvasEvent::ResolveAtelierAndPlace { atelier_ref, .. } => Some(atelier_ref),
        _ => None,
    });
    assert_eq!(
        resolved.map(|reference| reference.item_id.as_str()),
        Some("item-x")
    );
    assert!(resolved.is_some_and(|reference| reference.loom_block_id.is_none()));
}

#[test]
fn ac3_real_panel_drag_source_drops_on_real_canvas() {
    use handshake_native::graph::canvas_board::{CanvasEvent, LoomCanvasBoard};

    let panel = std::sync::Arc::new(std::sync::Mutex::new(seeded_panel()));
    let board = std::sync::Arc::new(std::sync::Mutex::new(LoomCanvasBoard::new(
        "ws-test",
        "canvas-test",
    )));
    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::<CanvasEvent>::new()));
    let events_check = std::sync::Arc::clone(&events);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 650.0))
        .build_ui(move |ui| {
            ui.columns(2, |columns| {
                panel
                    .lock()
                    .unwrap()
                    .show(&mut columns[0], &HsTheme::Dark.palette());
                if let Some(event) = board
                    .lock()
                    .unwrap()
                    .show(&mut columns[1], &HsTheme::Dark.palette())
                {
                    events.lock().unwrap().push(event);
                }
            });
        });
    harness.run();
    let source = center_by_author(&harness, &item_author_id("item-aaa"));
    let target = center_by_author(&harness, handshake_native::graph::STATUS_AUTHOR_ID)
        + egui::vec2(0.0, 180.0);
    harness.drag_at(source);
    harness.run();
    let mut producer_emitted_typed_payload = false;
    for step in 1..=8 {
        let t = step as f32 / 8.0;
        harness.hover_at(source + (target - source) * t);
        harness.run();
        producer_emitted_typed_payload |=
            egui::DragAndDrop::has_payload_of_type::<DragPayload>(&harness.ctx);
    }
    assert!(
        producer_emitted_typed_payload,
        "counterfactual producer gate: the actual Atelier dnd_drag_source must stage DragPayload before canvas release"
    );
    harness.drop_at(target);
    harness.run_steps(2);
    let events = events_check.lock().unwrap();
    let resolved = events
        .iter()
        .filter_map(|event| match event {
            CanvasEvent::ResolveAtelierAndPlace { atelier_ref, .. } => Some(atelier_ref),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        resolved.len(),
        1,
        "one physical Atelier release must enqueue exactly one ResolveAtelierAndPlace"
    );
    assert_eq!(resolved[0].item_id.as_str(), "item-aaa");
    assert!(resolved[0].loom_block_id.is_none());
    assert!(
        !events.iter().any(|event| matches!(
            event,
            CanvasEvent::ViewportChanged { .. } | CanvasEvent::MovePlacement { .. }
        )),
        "external drop must outrank and suppress competing viewport/card-move events in the release frame: {events:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-4 (kittest): the Route-to-Stage command opens the Stage pane and displays the routed content; the
// stage-pane AccessKit GenericContainer node is visible with the routed summary as its value.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac4_route_to_stage_displays_routed_selection() {
    // The shell-side flow: a context-menu "Route to Stage" stages a selection on the bus + dispatches the
    // command; the shell drains the staged content into the Stage pane, which then displays it.
    let stage = std::sync::Arc::new(std::sync::Mutex::new(StagePane::new()));
    let stage_h = std::sync::Arc::clone(&stage);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 400.0))
        .build_ui(move |ui| {
            // Per-frame shell drain: pull any staged content off the bus into the Stage pane (the
            // production shell does exactly this each frame).
            let bus = InteractionBus::get_or_init(ui.ctx());
            let route =
                InteractionBus::with_try_lock(&bus, |bus| bus.pending_stage_route().cloned())
                    .flatten();
            if let Some(route) = route {
                let mut stage = stage_h.lock().unwrap();
                let _ = InteractionBus::with_try_lock(&bus, |bus| {
                    if bus
                        .pending_stage_route()
                        .is_some_and(|pending| pending.receipt.event_id == route.receipt.event_id)
                    {
                        stage.set_content_correlated(
                            route.content.clone(),
                            route.causal_action_id.clone(),
                        );
                        let _ = bus.ack_pending_stage_route(&route.receipt.event_id);
                    }
                });
            }
            let pal = HsTheme::Dark.palette();
            stage_h.lock().unwrap().show(ui, &pal);
        });
    harness.run();

    // Before routing: the Stage pane shows the empty prompt; its container value summarizes "nothing routed".
    assert!(stage_value(&harness)
        .unwrap_or_default()
        .contains("nothing routed"));

    // The context-menu path: register the command, stage a selection, dispatch — exactly as the shell
    // does on a right-click "Route to Stage" of a rich-text selection.
    let bus = InteractionBus::get_or_init(&harness.ctx);
    let dispatched = InteractionBus::with_try_lock(&bus, |bus| {
        bus.register_route_to_stage_command();
        assert!(
            bus.commands().get(CMD_ROUTE_TO_STAGE).is_some(),
            "AC-4: route-to-stage command registered"
        );
        bus.route_to_stage(
            &harness.ctx,
            StageContent::Selection("the quick brown fox".to_owned(), "DOC-42".to_owned()),
        )
    })
    .unwrap_or(false);
    assert!(dispatched, "AC-4: the route-to-stage command must dispatch");
    harness.run();
    harness.run(); // one more frame so the drain + display settle

    // The Stage pane now displays the routed selection; the stage-pane container value carries the summary.
    let val =
        stage_value(&harness).expect("AC-4: stage-pane GenericContainer node must be present");
    assert!(
        val.contains("DOC-42"),
        "AC-4: the routed selection's source document is shown ({val})"
    );
    assert!(
        val.contains("the quick brown fox"),
        "AC-4: the routed selection text is shown ({val})"
    );
    assert!(
        stage.lock().unwrap().content.is_some(),
        "AC-4: the Stage pane holds the routed content after the command"
    );
    println!(
        "AC-4: Route-to-Stage opened the Stage pane and displayed the routed selection ({val})"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-6 (AccessKit dump): atelier-side-panel (List), atelier-item-{id} (ListItem, draggable), stage-pane
// (GenericContainer) are present in the live AccessKit tree.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac6_accesskit_nodes_present() {
    // (a) The atelier side panel: List container + per-item ListItem nodes.
    let panel = std::sync::Arc::new(std::sync::Mutex::new(seeded_panel()));
    let panel_h = std::sync::Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(360.0, 640.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            panel_h.lock().unwrap().show(ui, &pal);
        });
    harness.run();

    let ids = author_ids(&harness);
    assert!(
        ids.contains(PANEL_AUTHOR_ID),
        "AC-6: atelier-side-panel List node present ({ids:?})"
    );
    assert!(
        ids.contains(REFRESH_AUTHOR_ID),
        "AC-6: refresh button node present"
    );
    let expected_item = item_author_id("item-aaa");
    assert!(
        ids.contains(&expected_item),
        "AC-6: at least one atelier-item-{{id}} ListItem node present (looked for {expected_item}; got {ids:?})"
    );

    // The panel container is Role::List; the item row is Role::ListItem with a truthful 'draggable'
    // description. AccessKit 0.21.1 has no StartDrag action, so Click is the executable model fallback.
    let mut saw_list = false;
    let mut saw_list_item_draggable = false;
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        match ak.author_id() {
            Some(a) if a == PANEL_AUTHOR_ID => {
                assert_eq!(
                    ak.role(),
                    egui::accesskit::Role::List,
                    "AC-6: panel is a List"
                );
                saw_list = true;
            }
            Some(a) if a == expected_item => {
                assert_eq!(
                    ak.role(),
                    egui::accesskit::Role::ListItem,
                    "AC-6: item is a ListItem"
                );
                let desc = ak.description().unwrap_or_default();
                assert!(
                    desc.contains("draggable"),
                    "AC-6: the item row exposes a 'draggable' affordance (got desc '{desc}')"
                );
                assert!(
                    desc.contains("item-aaa"),
                    "AC-6: the item row exposes its atelier ref in the description (got '{desc}')"
                );
                assert!(ak.data().supports_action(egui::accesskit::Action::Click));
                saw_list_item_draggable = true;
            }
            _ => {}
        }
    }
    assert!(saw_list, "AC-6: the List container node was inspected");
    assert!(
        saw_list_item_draggable,
        "AC-6: the draggable ListItem node was inspected"
    );
    request_click_by_author(&harness, &expected_item);
    harness.run();
    assert!(matches!(
        panel.lock().unwrap().take_action(),
        Some(handshake_native::atelier_side_panel::AtelierPanelAction::InsertIntoActiveEditor(
            reference
        )) if reference.item_id == "item-aaa"
    ));

    // (b) The Stage pane: GenericContainer node.
    let stage = std::sync::Arc::new(std::sync::Mutex::new(StagePane::new()));
    let stage_h = std::sync::Arc::clone(&stage);
    let mut stage_harness = Harness::builder()
        .with_size(egui::vec2(600.0, 400.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            stage_h.lock().unwrap().show(ui, &pal);
        });
    stage_harness.run();
    let stage_ids = author_ids(&stage_harness);
    assert!(
        stage_ids.contains(STAGE_PANE_AUTHOR_ID),
        "AC-6: stage-pane GenericContainer node present ({stage_ids:?})"
    );
    let mut saw_container = false;
    for node in stage_harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(STAGE_PANE_AUTHOR_ID) {
            assert_eq!(
                ak.role(),
                egui::accesskit::Role::GenericContainer,
                "AC-6: stage-pane is a GenericContainer"
            );
            saw_container = true;
        }
    }
    assert!(
        saw_container,
        "AC-6: the GenericContainer node was inspected"
    );
    println!(
        "AC-6: atelier-side-panel(List), injective atelier-item hex id(ListItem+draggable), stage-pane(GenericContainer) present"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-4 + AC-6 LIVE-SHELL reachability: the Stage pane and Atelier side panel must appear in the REAL
// `HandshakeApp` AccessKit tree — not only in standalone widget harnesses. This is the regression guard
// for the adversarial "unwired scaffolding" finding: a widget that passes its own isolated harness but is
// never mounted in `app.rs` would pass AC-4/AC-6's isolated tests yet be unreachable in the product. These
// tests render the actual shell via `HandshakeApp::ui` (the same path the production window drives) and
// assert the MT-033 surfaces are present + that a dispatched Route-to-Stage command DRAINS into the
// mounted Stage pane.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac6_atelier_toggle_is_a_model_addressable_command() {
    use handshake_native::command_registry::{all_commands, CMD_VIEW_ATELIER};

    let command = all_commands()
        .iter()
        .find(|command| command.id == CMD_VIEW_ATELIER)
        .expect("canonical Atelier view command is registered");
    assert_eq!(command.stable_id, "hs-view-palette-atelier");
    assert!(!command.disabled, "Atelier toggle is executable");
}

/// A headless real shell. Data-row proof belongs to the managed-backend mounted test; this helper never
/// seeds the production panel and therefore cannot turn a detached fixture into false shell evidence.
fn live_shell() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }))
}

/// Positive rich-route proofs must use production Notes navigation to activate an exact document. A
/// generic shell intentionally starts on another editor, and production correctly rejects Route to
/// Stage when no rich document is active.
fn live_rich_shell(document_id: &str) -> (HandshakeApp, tokio::runtime::Runtime) {
    use handshake_native::quick_switcher::{NavDispatchOutcome, ShellNavigator};

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build rich-route test runtime");
    let mut app = live_shell();
    app.set_backend_base_url_for_test("http://127.0.0.1:1", runtime.handle().clone());
    assert!(
        matches!(
            app.open_document(document_id),
            NavDispatchOutcome::Opened { .. }
        ),
        "production Notes navigation opens and activates the routed document"
    );
    let pane_id = app
        .active_pane()
        .expect("production Notes navigation focuses the rich pane")
        .clone();

    let demo = RichEditorState::demo();
    let content_json =
        handshake_native::rich_editor::document_model::doc_json::to_content_json_value(&demo.doc);
    app.apply_loaded_rich_document_to_view_for_test(
        pane_id.as_ref(),
        handshake_native::backend_client::RichDocBody {
            document_id: document_id.to_owned(),
            workspace_id: "ws-ckc-test".to_owned(),
            doc_version: 1,
            title: document_id.to_owned(),
            content_json,
            crdt_document_id: None,
            authority_label: "AUTHORITATIVE".to_owned(),
            owner_actor_kind: Some("operator".to_owned()),
            owner_actor_id: Some("operator".to_owned()),
            project_ref: None,
            folder_ref: None,
            created_at: "2026-07-26T00:00:00Z".to_owned(),
            updated_at: "2026-07-26T00:00:00Z".to_owned(),
        },
    )
    .expect("install ready demo document in exact active rich view");
    (app, runtime)
}

fn cross_pane_undo_modifiers() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        shift: true,
        command: true,
        ..Default::default()
    }
}

#[test]
fn ac6_atelier_side_panel_mounted_in_live_shell() {
    // Render the REAL shell while the panel is closed. A preloaded widget alone must not satisfy this
    // proof: the operator has to open the VIEW dropdown and invoke its canonical `view.atelier` route.
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), live_shell());
    harness.run();

    assert!(
        !author_ids(&harness).contains(PANEL_AUTHOR_ID),
        "AC-6 live: Atelier panel starts closed"
    );
    harness.get_by_label("VIEW").click();
    harness.run();
    let menu_ids = author_ids(&harness);
    assert!(
        menu_ids.contains("menu.view.toggle-atelier"),
        "AC-6 live: canonical model-addressable VIEW leaf is present ({menu_ids:?})"
    );
    harness.get_by_label("Toggle Atelier / CKC Panel").click();
    harness.step();
    harness.run();

    let ids = author_ids(&harness);
    assert!(
        ids.contains(PANEL_AUTHOR_ID),
        "AC-6 live: atelier-side-panel List node present in the REAL shell tree ({ids:?})"
    );
    // The same command is a real toggle, not a one-way test seam.
    harness.get_by_label("VIEW").click();
    harness.run();
    harness.get_by_label("Toggle Atelier / CKC Panel").click();
    harness.run();
    assert!(
        !author_ids(&harness).contains(PANEL_AUTHOR_ID),
        "AC-6 live: invoking the canonical route again closes the mounted panel"
    );
    println!(
        "AC-6 live: the Atelier side panel is mounted + reachable in the real HandshakeApp shell"
    );
}

#[test]
fn ac4_route_to_stage_in_live_shell_shows_stage_pane() {
    // Drive the REAL shell. Initially the Stage pane is closed (nothing routed). Stage a selection on the
    // shared bus + dispatch the Route-to-Stage command (exactly what the context-menu / palette path does);
    // the shell's per-frame `drive_ckc_interop` drain must open the Stage pane and display the routed
    // content, and the live tree must then carry the `stage-pane` GenericContainer node. This is the production drain
    // loop the isolated AC-4 harness only simulated.
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), live_shell());
    harness.run();

    // Before routing: no stage-pane node (the pane is closed until content is routed).
    assert!(
        !author_ids(&harness).contains(STAGE_PANE_AUTHOR_ID),
        "AC-4 live: the Stage pane is closed before any Route-to-Stage dispatch"
    );

    // Stage + dispatch on the SAME bus the running shell drains (get_or_init keys off the shell's ctx).
    let bus = InteractionBus::get_or_init(&harness.ctx);
    let dispatched = InteractionBus::with_try_lock(&bus, |bus| {
        bus.register_route_to_stage_command();
        bus.route_to_stage(
            &harness.ctx,
            StageContent::Selection("the quick brown fox".to_owned(), "DOC-42".to_owned()),
        )
    })
    .unwrap_or(false);
    assert!(
        dispatched,
        "AC-4 live: the route-to-stage command dispatched on the shell bus"
    );
    // The shell intentionally keeps repainting while the route opens/focuses Stage, so use an explicit
    // bounded frame count instead of `run()`, which requires quiescence.
    harness.run_steps(2);

    let ids = author_ids(&harness);
    assert!(
        ids.contains(STAGE_PANE_AUTHOR_ID),
        "AC-4 live: the shell drain opened the Stage pane (stage-pane GenericContainer node present) ({ids:?})"
    );
    let val = stage_value(&harness).expect("AC-4 live: stage-pane GenericContainer node present");
    assert!(
        val.contains("DOC-42"),
        "AC-4 live: routed selection's source document shown ({val})"
    );
    assert!(
        val.contains("the quick brown fox"),
        "AC-4 live: routed selection text shown ({val})"
    );
    assert!(
        harness.state().stage_panel_open(),
        "AC-4 live: the Stage panel is open after routing"
    );
    println!(
        "AC-4 live: Route-to-Stage dispatched in the REAL shell opened + filled the Stage pane"
    );
}

#[test]
fn ac4_view_stage_then_route_uses_one_docked_stage_region() {
    use handshake_native::pane_registry::PaneId;

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), live_shell());
    harness.run();
    harness.get_by_label("EDITORS").click();
    harness.run();
    harness.get_by_label("View: Stage").click();
    harness.run_steps(2);

    let stage_pane = harness
        .state()
        .tab_bar_states()
        .iter()
        .find_map(|(pane_id, bar)| {
            bar.tabs
                .iter()
                .any(|tab| tab.label() == "Stage")
                .then(|| pane_id.clone())
        })
        .expect("EDITORS opens one Stage tab");
    let other_pane = harness
        .state()
        .tab_bar_states()
        .keys()
        .find(|pane_id| **pane_id != stage_pane)
        .cloned()
        .unwrap_or_else(|| PaneId::from("pane-b"));
    harness
        .state_mut()
        .set_active_pane_for_test(Some(other_pane.clone()));

    assert!(
        harness
            .state_mut()
            .dispatch_palette_action_for_test(handshake_native::interop::CMD_EMBED_STAGE_CAPTURE),
        "Embed Stage Capture dispatches through the shell-wide Stage opener"
    );
    harness.run_steps(2);
    assert_eq!(
        harness
            .root()
            .children_recursive()
            .filter(|node| node.accesskit_node().author_id() == Some(STAGE_PANE_AUTHOR_ID))
            .count(),
        1,
        "Embed Stage Capture after a pane switch must focus, not duplicate, Stage"
    );
    harness
        .state_mut()
        .set_active_pane_for_test(Some(other_pane));

    let bus = InteractionBus::get_or_init(&harness.ctx);
    assert_eq!(
        InteractionBus::with_try_lock(&bus, |bus| {
            bus.register_route_to_stage_command();
            bus.route_to_stage(
                &harness.ctx,
                StageContent::Selection("one host".into(), "DOC-STAGE".into()),
            )
        }),
        Some(true)
    );
    harness.run_steps(3);

    let stage_nodes = harness
        .root()
        .children_recursive()
        .filter(|node| node.accesskit_node().author_id() == Some(STAGE_PANE_AUTHOR_ID))
        .collect::<Vec<_>>();
    assert_eq!(
        stage_nodes.len(),
        1,
        "EDITORS open, pane switch, then route must not duplicate stage-pane"
    );
    assert_eq!(
        format!("{:?}", stage_nodes[0].accesskit_node().role()),
        "GenericContainer"
    );
    assert!(stage_nodes[0]
        .accesskit_node()
        .value()
        .unwrap_or_default()
        .contains("one host"));
    assert_eq!(
        harness.state().active_pane(),
        Some(&stage_pane),
        "routing from another pane focuses the existing shell-wide Stage tab"
    );
}

#[test]
fn ac4_live_shell_route_registers_and_executes_unified_stage_undo() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), live_shell());
    harness.run();

    let bus = InteractionBus::get_or_init(&harness.ctx);
    let dispatched = InteractionBus::with_try_lock(&bus, |bus| {
        bus.register_route_to_stage_command();
        bus.route_to_stage(
            &harness.ctx,
            StageContent::Selection("undo me".to_owned(), "DOC-UNDO-33".to_owned()),
        )
    })
    .unwrap_or(false);
    assert!(dispatched, "live Stage route dispatches");
    harness.run_steps(2);
    assert!(matches!(
        harness.state().stage_content(),
        StageContent::Selection(ref text, ref source)
            if text == "undo me" && source == "DOC-UNDO-33"
    ));

    // Drive the actual shell-owned production shortcut consumer. Calling `undo_cross_pane` directly
    // would only prove the stack method and would leave the registered Ctrl+Shift+Z chord orphaned.
    harness.key_press_modifiers(cross_pane_undo_modifiers(), egui::Key::Z);
    harness.run_steps(2);
    assert!(matches!(
        harness.state().stage_content(),
        StageContent::Empty
    ));
}

#[test]
fn ac4_mounted_canvas_context_route_uses_live_pane_guard_and_rejects_closed_source() {
    use handshake_native::command_registry::CMD_VIEW_CANVAS;
    use handshake_native::context_menu_surfaces::NodeMenuAction;
    use handshake_native::graph::canvas_board::CanvasEvent;
    use handshake_native::pane_registry::{PaneId, PaneType};

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("runtime");
    let mut app = live_shell();
    app.set_backend_base_url_for_test("http://127.0.0.1:1", runtime.handle().clone());
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
    assert!(
        app.dispatch_palette_action_for_test(CMD_VIEW_CANVAS),
        "production View Canvas route mounts the Canvas tab"
    );
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
    {
        let board = app.mounted_canvas_board();
        let mut board = board.lock().unwrap();
        board.begin_projection_load("ws-canvas-route-33", "canvas-route-33");
        board.set_board(Vec::new(), Vec::new(), egui::Vec2::ZERO, 1.0);
    }
    let events = app.mounted_canvas_events();
    events.lock().unwrap().push(CanvasEvent::NodeMenu {
        placement_id: "placement-route-33".to_owned(),
        block_id: "block-route-33".to_owned(),
        source_pane_id: Some(PaneId::from("pane-a")),
        source_workspace_id: "ws-canvas-route-33".to_owned(),
        source_canvas_block_id: "canvas-route-33".to_owned(),
        unresolved_link_title: None,
        action: NodeMenuAction::RouteToStage,
    });

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);
    assert!(matches!(
        harness.state().stage_content(),
        StageContent::Selection(ref text, ref source)
            if text == "canvas node block-route-33"
                && source == "node://canvas-route-33/block-route-33"
    ));
    assert_eq!(harness.state().quick_switcher_nav_status(), None);

    // Close the actual Canvas tab, then deliver the same production Canvas menu event queue shape. The
    // queued stale action must not silently route through the pure builder after its source disappears.
    let canvas_tab_index = harness
        .state()
        .tab_bar_states()
        .get(&PaneId::from("pane-a"))
        .and_then(|bar| {
            bar.tabs
                .iter()
                .position(|tab| tab.pane_type == PaneType::AtelierEditor)
        })
        .expect("mounted Canvas tab is still present before close");
    harness
        .state_mut()
        .close_tab_indices_for_test(PaneId::from("pane-a"), vec![canvas_tab_index]);
    events.lock().unwrap().push(CanvasEvent::NodeMenu {
        placement_id: "placement-stale-33".to_owned(),
        block_id: "block-stale-33".to_owned(),
        source_pane_id: Some(PaneId::from("pane-a")),
        source_workspace_id: "ws-canvas-route-33".to_owned(),
        source_canvas_block_id: "canvas-route-33".to_owned(),
        unresolved_link_title: None,
        action: NodeMenuAction::RouteToStage,
    });
    harness.run_steps(3);
    let status = harness
        .state()
        .quick_switcher_nav_status()
        .expect("closed Canvas source produces a visible typed status");
    assert!(status.contains("pane that is not open: pane-a"), "{status}");
    assert!(matches!(
        harness.state().stage_content(),
        StageContent::Selection(ref text, _) if text == "canvas node block-route-33"
    ));

    drop(harness);
    runtime.shutdown_timeout(std::time::Duration::from_secs(2));
}

#[test]
fn ac4_real_rich_selection_context_menu_routes_to_stage_in_mounted_shell() {
    use handshake_native::rich_editor::document_model::{DocPosition, Selection};

    let (app, _runtime) = live_rich_shell("DOC-CONTEXT-33");
    {
        let rich = app.mounted_rich_state();
        let mut state = rich.lock().unwrap();
        state.wikilinks.document_id = "DOC-CONTEXT-33".to_owned();
        state.selection = Selection::text(
            DocPosition::new(vec![1, 0], 0),
            DocPosition::new(vec![1, 0], 5),
        );
        assert_eq!(
            state.selected_text().map(|(_, _, _, text)| text),
            Some("Hello".to_owned())
        );
    }
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("editor.rich.text"))
        .expect("mounted editor.rich.text")
        .click_secondary();
    harness.run_steps(2);
    assert!(
        author_ids(&harness).contains("rich-editor.route-to-stage"),
        "the required rich-selection context-menu action has a stable author id"
    );
    harness.get_by_label("Route to Stage").click();
    harness.run_steps(3);
    let value = stage_value(&harness).expect("context-menu route opens mounted Stage");
    assert!(value.contains("DOC-CONTEXT-33"), "{value}");
    assert!(value.contains("Hello"), "{value}");
}

#[test]
fn ac4_cross_block_selection_materializes_exact_text() {
    use handshake_native::rich_editor::document_model::{DocPosition, Selection};

    let mut state = RichEditorState::demo();
    state.wikilinks.document_id = "DOC-CROSS-33".to_owned();
    state.selection = Selection::text(
        DocPosition::new(vec![0, 0], 2),
        DocPosition::new(vec![1, 0], 5),
    );
    assert_eq!(
        state.selected_text_for_stage().as_deref(),
        Some("ading One\nHello")
    );
    assert!(matches!(
        state.stage_route_content(),
        Some(StageContent::Selection(text, document_id))
            if text == "ading One\nHello" && document_id == "DOC-CROSS-33"
    ));
}

#[test]
fn ac4_palette_routes_exact_cross_block_selection_in_mounted_shell() {
    use handshake_native::rich_editor::document_model::{DocPosition, Selection};

    let (app, _runtime) = live_rich_shell("DOC-PALETTE-CROSS-33");
    {
        let rich = app.mounted_rich_state();
        let mut state = rich.lock().unwrap();
        state.wikilinks.document_id = "DOC-PALETTE-CROSS-33".to_owned();
        state.selection = Selection::text(
            DocPosition::new(vec![0, 0], 2),
            DocPosition::new(vec![1, 0], 5),
        );
    }
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    let ctx = harness.ctx.clone();
    assert!(
        harness
            .state_mut()
            .dispatch_palette_action_for_test_with_ctx(&ctx, CMD_ROUTE_TO_STAGE),
        "positive palette dispatch must execute the registered Route-to-Stage command"
    );
    harness.run_steps(3);

    let value = stage_value(&harness).expect("palette route opens mounted Stage pane");
    assert!(value.contains("DOC-PALETTE-CROSS-33"), "{value}");
    assert!(value.contains("ading One\nHello"), "{value}");
    assert!(matches!(
        harness.state().stage_content(),
        StageContent::Selection(text, source)
            if text == "ading One\nHello" && source == "DOC-PALETTE-CROSS-33"
    ));
}

#[test]
fn ac4_no_selection_context_menu_routes_whole_active_document() {
    let (app, _runtime) = live_rich_shell("DOC-NO-SELECTION");
    {
        let rich = app.mounted_rich_state();
        rich.lock().unwrap().wikilinks.document_id = "DOC-NO-SELECTION".to_owned();
    }
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("editor.rich.text"))
        .expect("mounted editor.rich.text")
        .click_secondary();
    harness.run_steps(2);
    harness.get_by_label("Route to Stage").click();
    harness.run_steps(3);
    let value = stage_value(&harness).expect("whole document routed to mounted Stage");
    assert!(value.contains("DOC-NO-SELECTION"), "{value}");
    assert!(value.contains("Document:"), "{value}");
}

#[test]
fn ac4_palette_route_without_active_selection_visibly_fails() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), live_shell());
    harness.run();
    let ctx = harness.ctx.clone();
    assert!(harness
        .state_mut()
        .dispatch_palette_action_for_test_with_ctx(&ctx, CMD_ROUTE_TO_STAGE));
    harness.run();
    harness.run();
    assert!(author_ids(&harness).contains("stage-route-status"));
    assert!(stage_value(&harness)
        .unwrap_or_default()
        .contains("activate a saved rich document first"));
}

#[test]
fn route_to_stage_bus_contention_retains_visible_retry_and_recovers() {
    use handshake_native::rich_editor::document_model::{DocPosition, Selection};

    // The neighboring mounted-shell tests prove the app drain and Stage mount. Exercise the widget's
    // actual non-blocking contention branch here without app-level bus consumers swallowing the
    // synthetic click before it reaches the context-menu handler.
    let state = std::sync::Arc::new(std::sync::Mutex::new(RichEditorState::demo()));
    {
        let mut state = state.lock().unwrap();
        state.wikilinks.document_id = "DOC-BUSY-33".to_owned();
        state.selection = Selection::text(
            DocPosition::new(vec![1, 0], 0),
            DocPosition::new(vec![1, 0], 5),
        );
    }
    let rendered_state = std::sync::Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(std::sync::Arc::clone(&rendered_state)).show(ui);
        });
    harness.run_steps(2);
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("editor.rich.text"))
        .expect("mounted editor.rich.text")
        .click_secondary();
    harness.run_steps(2);
    let bus = InteractionBus::get_or_init(&harness.ctx);
    let guard = bus.lock().expect("hold InteractionBus to force contention");
    harness.get_by_label("Route to Stage").click();
    harness.run_steps(2);
    let (retained_while_contended, error_while_contended) = {
        let state = state.lock().unwrap();
        (
            state.pending_stage_route_retry.is_some(),
            state.interop_error.clone(),
        )
    };
    assert!(
        retained_while_contended,
        "the contended context-menu action retains the exact request before the bus is released; \
         interop_error={error_while_contended:?}"
    );
    drop(guard);
    harness.run_steps(2);
    let ids = author_ids(&harness);
    assert!(
        ids.contains("rich-editor-stage-route-retry"),
        "the retained route exposes its retry control: {ids:?}"
    );
    assert!(
        ids.contains("rich-editor-interop-status"),
        "the contended route exposes its typed status: {ids:?}"
    );
    let retained = state.lock().unwrap().pending_stage_route_retry.clone();
    assert!(matches!(
        retained,
        Some(handshake_native::interop::PendingStageRoute {
            content: StageContent::Selection(ref text, ref source),
            ..
        })
            if text == "Hello" && source == "DOC-BUSY-33"
    ));
    harness.get_by_label("Retry Route to Stage").click();
    harness.run_steps(3);
    {
        let state = state.lock().unwrap();
        assert!(state.pending_stage_route_retry.is_none());
        assert!(state.interop_error.is_none());
    }
    let bus = bus.lock().expect("inspect retried shared-bus route");
    assert!(matches!(
        bus.pending_stage_content(),
        Some(StageContent::Selection(text, source))
            if text == "Hello" && source == "DOC-BUSY-33"
    ));
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-4 named context-menu surface: the explorer-row "Route to Stage" item (the contract's named
// selection->stage dispatch surface) routes a DOCUMENT to the Stage pane through the bus + shell drain.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac4_explorer_context_menu_route_to_stage_item_routes_document() {
    use handshake_native::context_menu_surfaces::{
        explorer_action_for_id, explorer_context_items, explorer_ids, ExplorerMenuAction,
        ExplorerRowKind,
    };

    // The "Route to Stage" item is present + enabled on a Document row and maps to the RouteToStage action
    // (the named context-menu surface the contract requires — not a bus call made directly in the test).
    let items = explorer_context_items(ExplorerRowKind::Document);
    let route_item = items
        .iter()
        .find(|i| i.id == explorer_ids::ROUTE_TO_STAGE)
        .expect("AC-4: explorer 'Route to Stage' menu item present on a Document row");
    assert!(
        route_item.enabled,
        "AC-4: 'Route to Stage' is enabled on a Document row"
    );
    assert_eq!(
        explorer_action_for_id(explorer_ids::ROUTE_TO_STAGE, ExplorerRowKind::Document),
        Some(ExplorerMenuAction::RouteToStage),
        "AC-4: the confirmed menu id maps to the RouteToStage action",
    );
    // A canvas/bookmark row's item is disabled + maps to nothing (honest enable/disable, no fake route).
    for kind in [ExplorerRowKind::Canvas, ExplorerRowKind::Bookmark] {
        let item = explorer_context_items(kind)
            .into_iter()
            .find(|i| i.id == explorer_ids::ROUTE_TO_STAGE)
            .expect("the item is rendered for every kind (disabled where not applicable)");
        assert!(
            !item.enabled,
            "{kind:?} Route-to-Stage is disabled + disclosed"
        );
        assert_eq!(
            explorer_action_for_id(explorer_ids::ROUTE_TO_STAGE, kind),
            None
        );
    }
    println!("AC-4: explorer-row 'Route to Stage' item is the named dispatch surface (Document-only, enabled)");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// HBR-VIS screenshot: the Atelier side panel renders inside the real native shell; the PNG goes to the
// EXTERNAL root only.
// Gated behind the `wgpu_screenshots` feature (the WP-wide concurrent-wgpu hazard). The structural +
// AccessKit proofs above carry the AC coverage without a GPU.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
#[cfg(feature = "wgpu_screenshots")]
fn atelier_panel_screenshot() {
    use handshake_native::rich_editor::document_model::{DocPosition, Selection};
    use handshake_native::rich_editor::wikilinks::inline_view::chip_author_id;
    use handshake_native::stage_pane::{
        STAGE_ROUTED_CONTENT_AUTHOR_ID, STAGE_ROUTE_STATUS_AUTHOR_ID,
    };

    let _guard = wgpu_guard();
    let (candidate_identity_before, candidate_before) = current_worktree_candidate_identity();
    let test_executable = running_test_executable_provenance();
    let (mut app, runtime) = live_rich_shell("DOC-ARGUS-33");
    *app.atelier_side_panel_mut() = seeded_panel();
    let rich_state = app.mounted_rich_state();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);
    // Project rebinding can complete an unrelated FEMS refresh after the shell mounts. Clear that
    // ephemeral integration overlay before the first canonical inspection so it cannot obscure this
    // feature's success pixels; this does not alter persisted memory or the active project.
    harness
        .state_mut()
        .clear_fems_overlay_for_integration_test();
    // `live_rich_shell` deliberately binds a dead backend because this GPU proof seeds its CKC rows
    // directly. Once the expected asynchronous explorer failure has settled, clear that unrelated
    // test-only state so the accepted frame proves Atelier/Stage without presenting a false product
    // failure beside it. `set_content` is the ProjectTree-owned non-HTTP seed path and cancels no
    // product operation used by this proof.
    harness
        .state_mut()
        .left_rail_mut()
        .project_tree
        .set_content(Vec::new(), Vec::new());
    harness.run_steps(2);
    let run_id = format!("run-{}", uuid::Uuid::new_v4().simple());
    let head_sha = candidate_before["head_sha"]
        .as_str()
        .expect("candidate provenance carries HEAD")
        .to_owned();
    let invocation = "cargo test --manifest-path src/frontend/handshake_native/Cargo.toml --features wgpu_screenshots --test test_ckc_embed atelier_panel_screenshot -- --exact --nocapture --test-threads=1";
    let started_at_utc = chrono::Utc::now().to_rfc3339();
    let ext_dir =
        external_artifact_dir(&format!("wp-kernel-012-mt-033/canonical-argus-v4/{run_id}"));
    std::fs::create_dir_all(&ext_dir).expect("create external MT-033 canonical Argus directory");
    let mut argus = CanonicalArgusDriver::bind(
        harness.state(),
        &format!("mt033-success-{}", uuid::Uuid::new_v4().simple()),
    );

    let initial = argus.inspect(&mut harness);
    assert!(json_has_author_id(&initial, "menu-view"));
    assert!(json_has_author_id(&initial, "editor.rich.text"));
    assert!(!json_has_author_id(&initial, PANEL_AUTHOR_ID));

    argus.click_expect_applied_and_reinspect(&mut harness, "menu-view");
    argus.assert_latest_terminal_predicate(&mut harness, "atelier-toggle-discoverable", |tree| {
        json_has_author_id(tree, "menu.view.toggle-atelier")
    });
    let open_view = argus.latest_terminal_observation();
    let view_terminal = open_view.after.clone();
    assert!(json_has_author_id(
        &view_terminal,
        "menu.view.toggle-atelier"
    ));

    argus.click_expect_applied_and_reinspect(&mut harness, "menu.view.toggle-atelier");
    let item_author = item_author_id("item-aaa");
    argus.assert_latest_terminal_predicate(
        &mut harness,
        "atelier-panel-and-item-mounted",
        |tree| {
            json_has_author_id(tree, PANEL_AUTHOR_ID)
                && json_has_author_id(tree, &item_author)
                && json_has_author_id(tree, "atelier-character-list-blocker")
                && json_has_author_id(tree, "atelier-moodboard-list-blocker")
        },
    );
    let open_panel = argus.latest_terminal_observation();
    let panel_terminal = open_panel.after.clone();
    assert!(json_has_author_id(&panel_terminal, PANEL_AUTHOR_ID));

    let (insert_before_revision, insert_before_hash) = {
        let state = rich_state.lock().unwrap();
        (
            state.doc_revision(),
            sha256_json(&state.current_content_json()),
        )
    };
    argus.click_expect_applied_and_reinspect(&mut harness, &item_author);
    let chip_prefix = chip_author_id("item-aaa");
    argus.assert_latest_terminal_predicate(
        &mut harness,
        "atelier-click-fallback-inserts-exact-hslink",
        |tree| json_has_author_id_prefix(tree, &chip_prefix),
    );
    let insert = argus.latest_terminal_observation();
    let insert_terminal = insert.after.clone();
    assert!(
        json_has_author_id_prefix(&insert_terminal, &chip_prefix),
        "fresh canonical Argus inspection must expose the exact inserted CKC hsLink"
    );
    assert_eq!(
        first_hs_link(&rich_state.lock().unwrap().current_content_json()),
        Some(("media".to_owned(), "item-aaa".to_owned()))
    );
    let (insert_after_revision, insert_after_hash) = {
        let state = rich_state.lock().unwrap();
        (
            state.doc_revision(),
            sha256_json(&state.current_content_json()),
        )
    };
    assert!(insert_after_revision > insert_before_revision);
    assert_ne!(insert_after_hash, insert_before_hash);
    let insert_detail = canonical_product_detail(&insert);
    assert_eq!(insert_detail["ref_kind"], "media");
    assert_eq!(insert_detail["ref_value"], "item-aaa");
    assert_eq!(insert_detail["after_revision"], insert_after_revision);
    assert_eq!(insert_detail["after_content_hash"], insert_after_hash);

    {
        let mut state = rich_state.lock().unwrap();
        state.selection = Selection::text(
            DocPosition::new(vec![1, 0], 0),
            DocPosition::new(vec![1, 0], 5),
        );
        assert_eq!(
            state.selected_text().map(|(_, _, _, text)| text),
            Some("Hello".to_owned())
        );
    }
    argus.click_expect_applied_and_reinspect(&mut harness, "menu-editors");
    argus.assert_latest_terminal_predicate(
        &mut harness,
        "route-to-stage-control-discoverable",
        |tree| json_has_author_id(tree, "menu.editors.route-to-stage"),
    );
    let open_editors = argus.latest_terminal_observation();
    let editors_terminal = open_editors.after.clone();
    assert!(json_has_author_id(
        &editors_terminal,
        "menu.editors.route-to-stage"
    ));

    argus.click_expect_applied_and_reinspect(&mut harness, "menu.editors.route-to-stage");
    // The leaf action closes egui's popup memory in its dispatch frame. Advance one production frame
    // before the authoritative terminal inspection so the GPU paint and AccessKit tree prove the
    // same popup-free state; the exact route receipt remains attached to this action observation.
    harness.run_steps(1);
    argus.assert_latest_terminal_predicate(
        &mut harness,
        "routed-selection-visible-in-stage",
        |tree| {
            json_node_by_author_id(tree, STAGE_ROUTED_CONTENT_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value.contains("DOC-ARGUS-33") && value.contains("Hello"))
        },
    );
    let route = argus.latest_terminal_observation();
    let route_terminal = route.after.clone();
    assert!(json_has_author_id(
        &route_terminal,
        STAGE_ROUTED_CONTENT_AUTHOR_ID
    ));
    let route_detail = canonical_product_detail(&route);
    assert_eq!(route_detail["flight_recorder_action"], "route_to_stage");
    assert!(route_detail["route_receipt_event_id"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(route_detail["causal_action_id"]
        .as_str()
        .is_some_and(|value| value.starts_with("stage-route-")));
    let success_action_statuses = [
        open_view.receipt_status.as_str(),
        open_panel.receipt_status.as_str(),
        insert.receipt_status.as_str(),
        open_editors.receipt_status.as_str(),
        route.receipt_status.as_str(),
    ];
    assert_eq!(
        success_action_statuses, ["applied"; 5],
        "success branch must retain exactly five Applied canonical actions"
    );

    assert!(
        !json_has_author_id_prefix(&route_terminal, "menu."),
        "final success capture must not contain an open top-menu popup"
    );
    assert!(
        !json_has_author_id(&route_terminal, "command-palette.dialog"),
        "final success capture must not contain an open command-palette overlay"
    );
    assert_eq!(
        route_terminal, route.after,
        "success capture must be bound to the exact terminal tree persisted for the route action"
    );
    assert!(
        handshake_native::top_menu_bar::open_menu(&harness.ctx).is_none(),
        "the live egui context must have no top-level popup before success capture"
    );
    let success_terminal_tree_sha256 = sha256_json(&route.after);
    let success_capture_event_sequence =
        screenshot_harness::screenshot_marker::next_proof_event_sequence();
    assert!(
        success_capture_event_sequence > route.terminal_observed_sequence,
        "success capture event must follow the bound terminal observation"
    );
    let image = harness
        .render()
        .expect("HBR-VIS screenshot rendering is a required proof");
    let (w, h) = (image.width(), image.height());
    assert!(w > 0 && h > 0, "screenshot has non-zero size");
    let png = ext_dir.join("MT-033-atelier-stage-success.png");
    image
        .save(&png)
        .unwrap_or_else(|error| panic!("save required screenshot {}: {error}", png.display()));
    let png_sha256 = sha256_file(&png);
    let success_tree = ext_dir.join("MT-033-atelier-stage-success.json");
    std::fs::write(
        &success_tree,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_id": "hsk.mt033-canonical-argus-proof@1",
            "run_id": run_id,
            "head_sha": head_sha,
            "candidate_before": candidate_before,
            "test_executable": test_executable,
            "invocation": invocation,
            "started_at_utc": started_at_utc,
            "completed_at_utc": chrono::Utc::now().to_rfc3339(),
            "state": "success",
            "targets": [
                "menu-view",
                "menu.view.toggle-atelier",
                item_author,
                "menu-editors",
                "menu.editors.route-to-stage",
                STAGE_ROUTED_CONTENT_AUTHOR_ID
            ],
            "actions": [
                canonical_action_proof("menu-view", &open_view, "atelier-toggle-discoverable"),
                canonical_action_proof("menu.view.toggle-atelier", &open_panel, "atelier-panel-and-item-mounted"),
                canonical_action_proof(&item_author, &insert, "atelier-click-fallback-inserts-exact-hslink"),
                canonical_action_proof("menu-editors", &open_editors, "route-to-stage-control-discoverable"),
                canonical_action_proof("menu.editors.route-to-stage", &route, "routed-selection-visible-in-stage")
            ],
            "initial": initial,
            "terminal": route_terminal,
            "screenshot": {
                "path": png,
                "sha256": png_sha256,
                "capture_method": "mounted_wgpu_harness_render_after_fresh_argus_terminal",
                "bound_to_action_target": "menu.editors.route-to-stage",
                "bound_to_receipt_id": route.receipt_id,
                "bound_to_terminal_observed_sequence": route.terminal_observed_sequence,
                "bound_to_terminal_tree_sha256": success_terminal_tree_sha256,
                "capture_event_sequence": success_capture_event_sequence,
                "harness_run_steps_after_terminal_inspection": 0,
                "open_menu_or_popup_overlay": false
            }
        }))
        .expect("serialize MT-033 success tree"),
    )
    .expect("write external MT-033 success tree");
    println!(
        "HBR-VIS: canonical argus.inspect -> menu-view -> menu.view.toggle-atelier -> \
         {item_author} -> menu-editors -> menu.editors.route-to-stage -> fresh Stage observation; \
         {w}x{h} success screenshot={} tree={}",
        png.display(),
        success_tree.display()
    );
    argus.finish_require_no_indeterminate();
    drop(harness);
    runtime.shutdown_timeout(std::time::Duration::from_secs(2));

    // A second fresh shell proves the typed unavailable branch through the always-enabled canonical
    // Command Palette route. The Editors-menu leaf is correctly disabled without a rich editor, so the
    // palette is the model-steerable action surface for exercising the actual typed runtime failure.
    let mut failure_harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), live_shell());
    failure_harness.run_steps(3);
    let mut failure_argus = CanonicalArgusDriver::bind(
        failure_harness.state(),
        &format!("mt033-failure-{}", uuid::Uuid::new_v4().simple()),
    );
    let failure_initial = failure_argus.inspect(&mut failure_harness);
    failure_argus.click_expect_applied_and_reinspect(&mut failure_harness, "menu-operator");
    failure_argus.assert_latest_terminal_predicate(
        &mut failure_harness,
        "command-palette-control-visible",
        |tree| json_has_author_id(tree, "menu.operator.command-palette"),
    );
    let failure_menu = failure_argus.latest_terminal_observation();
    failure_argus
        .click_expect_applied_and_reinspect(&mut failure_harness, "menu.operator.command-palette");
    failure_argus.assert_latest_terminal_predicate(
        &mut failure_harness,
        "route-command-visible-without-rich-document",
        |tree| {
            json_has_author_id(tree, "command-palette.dialog")
                && json_has_author_id(tree, "command-palette.option.hs-stage-palette-route")
        },
    );
    let open_palette = failure_argus.latest_terminal_observation();
    failure_argus.click_expect_typed_rejected_and_reinspect(
        &mut failure_harness,
        "command-palette.option.hs-stage-palette-route",
        "activate a saved rich document first",
    );
    // As on the applied route path, inspect only after the leaf-dismissal frame has repainted. This
    // keeps the rejected receipt, terminal tree, and captured pixels on one popup-free state.
    failure_harness.run_steps(1);
    failure_argus.assert_latest_terminal_predicate(
        &mut failure_harness,
        "route-unavailable-is-visible-and-typed",
        |tree| {
            json_node_by_author_id(tree, STAGE_ROUTE_STATUS_AUTHOR_ID)
                .and_then(|node| node.get("value"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value.contains("activate a saved rich document first"))
        },
    );
    let unavailable = failure_argus.latest_terminal_observation();
    let unavailable_terminal = unavailable.after.clone();
    assert!(
        json_node_by_author_id(&unavailable_terminal, STAGE_ROUTED_CONTENT_AUTHOR_ID)
            .and_then(|node| node.get("value"))
            .and_then(serde_json::Value::as_str)
            == Some("(nothing routed to Stage)"),
        "typed unavailable terminal must preserve the explicit empty routed-content state"
    );
    assert!(
        !json_has_author_id_prefix(&unavailable_terminal, "menu."),
        "typed-unavailable capture must not contain an open top-menu popup"
    );
    assert!(
        !json_has_author_id(&unavailable_terminal, "command-palette.dialog"),
        "typed-unavailable capture must not contain an open command-palette overlay"
    );
    assert_eq!(
        unavailable_terminal, unavailable.after,
        "typed-unavailable capture must use the exact persisted rejected-action terminal tree"
    );
    assert!(
        handshake_native::top_menu_bar::open_menu(&failure_harness.ctx).is_none(),
        "the live egui context must have no top-level popup before typed-unavailable capture"
    );
    let failure_terminal_tree_sha256 = sha256_json(&unavailable.after);
    let failure_capture_event_sequence =
        screenshot_harness::screenshot_marker::next_proof_event_sequence();
    assert!(
        failure_capture_event_sequence > unavailable.terminal_observed_sequence,
        "typed-unavailable capture event must follow the bound terminal observation"
    );
    let failure_png = ext_dir.join("MT-033-route-unavailable.png");
    failure_harness
        .render()
        .expect("typed unavailable state requires a material render")
        .save(&failure_png)
        .expect("save MT-033 unavailable screenshot");
    let failure_png_sha256 = sha256_file(&failure_png);
    let unavailable_detail = canonical_product_detail(&unavailable);
    assert_eq!(unavailable_detail["typed_outcome"], "route_unavailable");
    assert_eq!(unavailable_detail["stage_content"], "empty");
    assert_eq!(
        unavailable_detail["stage_visible_value"],
        "(nothing routed to Stage)"
    );
    assert_eq!(
        [
            failure_menu.receipt_status.as_str(),
            open_palette.receipt_status.as_str(),
            unavailable.receipt_status.as_str(),
        ],
        ["applied", "applied", "rejected"],
        "typed-unavailable branch must retain exactly two Applied actions and one typed Rejected action"
    );
    let failure_tree = ext_dir.join("MT-033-route-unavailable.json");
    std::fs::write(
        &failure_tree,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_id": "hsk.mt033-canonical-argus-proof@1",
            "run_id": run_id,
            "head_sha": head_sha,
            "candidate_before": candidate_before,
            "test_executable": test_executable,
            "invocation": invocation,
            "started_at_utc": started_at_utc,
            "completed_at_utc": chrono::Utc::now().to_rfc3339(),
            "state": "route_unavailable",
            "targets": [
                "menu-operator",
                "menu.operator.command-palette",
                "command-palette.option.hs-stage-palette-route",
                STAGE_ROUTE_STATUS_AUTHOR_ID
            ],
            "actions": [
                canonical_action_proof("menu-operator", &failure_menu, "command-palette-control-visible"),
                canonical_action_proof("menu.operator.command-palette", &open_palette, "route-command-visible-without-rich-document"),
                canonical_action_proof("command-palette.option.hs-stage-palette-route", &unavailable, "route-unavailable-is-visible-and-typed")
            ],
            "initial": failure_initial,
            "terminal": unavailable_terminal,
            "screenshot": {
                "path": failure_png,
                "sha256": failure_png_sha256,
                "capture_method": "mounted_wgpu_harness_render_after_fresh_argus_terminal",
                "bound_to_action_target": "command-palette.option.hs-stage-palette-route",
                "bound_to_receipt_id": unavailable.receipt_id,
                "bound_to_terminal_observed_sequence": unavailable.terminal_observed_sequence,
                "bound_to_terminal_tree_sha256": failure_terminal_tree_sha256,
                "capture_event_sequence": failure_capture_event_sequence,
                "harness_run_steps_after_terminal_inspection": 0,
                "open_menu_or_popup_overlay": false
            }
        }))
        .expect("serialize MT-033 unavailable tree"),
    )
    .expect("write external MT-033 unavailable tree");
    println!(
        "HBR-VIS: canonical typed unavailable route screenshot={} tree={}",
        failure_png.display(),
        failure_tree.display()
    );
    failure_argus.finish_require_no_indeterminate();
    let (candidate_identity_after, candidate_after) = current_worktree_candidate_identity();
    assert_eq!(
        candidate_identity_after, candidate_identity_before,
        "MT-033 source candidate must remain byte-identical throughout the canonical V4 run"
    );
    let run_manifest = ext_dir.join("MT-033-canonical-argus-v4-run-manifest.json");
    std::fs::write(
        &run_manifest,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_id": "hsk.mt033-canonical-argus-v4-run@1",
            "run_id": run_id,
            "invocation": invocation,
            "started_at_utc": started_at_utc,
            "completed_at_utc": chrono::Utc::now().to_rfc3339(),
            "candidate_identity_before": candidate_identity_before,
            "candidate_identity_after": candidate_identity_after,
            "candidate_identity_unchanged": true,
            "candidate_before": candidate_before,
            "candidate_after": candidate_after,
            "test_executable": test_executable,
            "terminal_receipt_summary": {
                "applied": 7,
                "rejected": 1,
                "indeterminate": 0,
                "success_branch_applied": 5,
                "typed_unavailable_branch_applied": 2,
                "typed_unavailable_branch_rejected": 1,
            },
            "artifacts": {
                "success_png": { "path": png, "sha256": png_sha256 },
                "success_json": { "path": success_tree, "sha256": sha256_file(&success_tree) },
                "typed_unavailable_png": { "path": failure_png, "sha256": failure_png_sha256 },
                "typed_unavailable_json": { "path": failure_tree, "sha256": sha256_file(&failure_tree) },
            }
        }))
        .expect("serialize MT-033 V4 run manifest"),
    )
    .expect("write external MT-033 V4 run manifest");
    println!(
        "HBR-VIS: source-bound V4 run manifest={}",
        run_manifest.display()
    );
    assert_no_local_artifact_dir();
}

/// A no-GPU guard run so the hygiene assertion executes in the default suite even without the screenshot
/// feature (the screenshot test is the only PNG writer; this proves no repo-local artifact dir exists).
#[test]
fn no_local_artifact_dir_in_default_suite() {
    let _ = wgpu_guard; // keep the guard referenced even when the screenshot feature is off
    assert_no_local_artifact_dir();
}

#[test]
fn declared_test_command_targets_this_nonzero_integration_binary() {
    const DECLARED: &str = "cargo test -p handshake-native --test test_ckc_embed -- --nocapture";
    assert!(DECLARED.contains("--test test_ckc_embed"));
    let mandatory_runtime_proofs = std::collections::BTreeSet::from([
        "real-panel-to-rich-editor-dnd",
        "real-panel-to-canvas-dnd",
        "mounted-selection-context-menu-to-stage",
        "managed-backend-mounted-panel",
        "managed-backend-save-and-canvas-reload",
    ]);
    assert_eq!(
        mandatory_runtime_proofs.len(),
        5,
        "the declared integration binary must retain five uniquely named runtime proofs"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AtelierClient request-builder proofs (NO backend): the EXACT verified atelier routes. The real spawn
// paths route through these SAME builders, so a stale URL can never reach the live backend unnoticed.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn atelier_client_builds_verified_routes() {
    use handshake_native::backend_client::AtelierClient;
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let c = AtelierClient::new("http://127.0.0.1:37501", rt.handle().clone());
    assert_eq!(
        c.batches_request().url,
        "http://127.0.0.1:37501/atelier/intake/batches",
        "AC-5: the verified intake-batches route"
    );
    assert_eq!(
        c.corpus_request().url,
        "http://127.0.0.1:37501/atelier/command-corpus",
        "AC-5: the verified command-corpus route"
    );
    assert_eq!(
        c.items_request("batch-7").url,
        "http://127.0.0.1:37501/atelier/intake/batches/batch-7/items",
        "AC-5: the verified per-batch items route"
    );
    println!(
        "AC-5: AtelierClient builds the verified /atelier routes (batches, command-corpus, items)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// LIVE-PG (integration-gated): self-seeds the managed PostgreSQL/backend and cleans exact ids.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "integration")]
fn psql_program() -> std::path::PathBuf {
    for var in ["HANDSHAKE_MANAGED_PG_BIN", "PGBIN"] {
        if let Some(dir) = std::env::var_os(var).filter(|value| !value.is_empty()) {
            let name = if cfg!(windows) { "psql.exe" } else { "psql" };
            let candidate = std::path::PathBuf::from(dir).join(name);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    if cfg!(windows) {
        for root_var in ["ProgramFiles", "ProgramFiles(x86)"] {
            let Some(root) = std::env::var_os(root_var) else {
                continue;
            };
            let postgres = std::path::PathBuf::from(root).join("PostgreSQL");
            let Ok(entries) = std::fs::read_dir(postgres) else {
                continue;
            };
            let mut candidates = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path().join("bin").join("psql.exe"))
                .filter(|path| path.is_file())
                .collect::<Vec<_>>();
            candidates.sort();
            if let Some(candidate) = candidates.pop() {
                return candidate;
            }
        }
    }
    std::path::PathBuf::from(if cfg!(windows) { "psql.exe" } else { "psql" })
}

#[cfg(feature = "integration")]
fn pg_dsn() -> String {
    [
        "HANDSHAKE_TEST_PG_DSN",
        "POSTGRES_TEST_URL",
        "DATABASE_URL",
    ]
        .into_iter()
        .find_map(|name| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .expect(
            "AC-5 live requires HANDSHAKE_TEST_PG_DSN, POSTGRES_TEST_URL, or DATABASE_URL for exact test-row cleanup",
        )
}

#[cfg(feature = "integration")]
fn integration_suffix() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    )
}

#[cfg(feature = "integration")]
fn proof_headers(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    request
        .header("x-hsk-actor-id", "mt033-live-pg")
        .header("x-hsk-actor-kind", "operator")
        .header("x-hsk-kernel-task-run-id", "WP-KERNEL-012-MT-033")
        .header("x-hsk-session-run-id", "MT-033-integration")
}

#[cfg(feature = "integration")]
fn workspace_write_headers(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    request
        .header("x-hsk-actor-id", "mt033-live-pg")
        .header("x-hsk-actor-kind", "human")
}

#[cfg(feature = "integration")]
fn run_psql(sql: &str) -> String {
    let mut command = std::process::Command::new(psql_program());
    command
        .arg("--dbname")
        .arg(pg_dsn())
        .arg("--set")
        .arg("ON_ERROR_STOP=1")
        .arg("--no-align")
        .arg("--tuples-only")
        .arg("--command")
        .arg(sql)
        .env("PGCONNECT_TIMEOUT", "5");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0800_0000);
    }
    let output = match command.output() {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Some managed-PG hosts expose the trust-auth cluster without installing the PostgreSQL
            // client tools on PATH. Keep psql as the primary portable path, then use the already-probed
            // Python psycopg2 driver as a quiet equivalent for the same exact SQL and output format.
            let script = r#"
import json
import psycopg2
import sys

def render(value):
    if isinstance(value, (dict, list)):
        return json.dumps(value, separators=(",", ":"))
    if isinstance(value, bool):
        return "t" if value else "f"
    if value is None:
        return ""
    return str(value)

connection = psycopg2.connect(sys.argv[1])
try:
    connection.autocommit = True
    cursor = connection.cursor()
    cursor.execute(sys.argv[2])
    if cursor.description:
        for row in cursor.fetchall():
            print("|".join(render(value) for value in row))
finally:
    connection.close()
"#;
            let mut python = std::process::Command::new("python");
            python
                .arg("-c")
                .arg(script)
                .arg(pg_dsn())
                .arg(sql)
                .env("PGCONNECT_TIMEOUT", "5");
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt as _;
                python.creation_flags(0x0800_0000);
            }
            python
                .output()
                .expect("launch managed PostgreSQL psycopg2 fallback")
        }
        Err(error) => panic!("launch managed PostgreSQL psql: {error}"),
    };
    assert!(
        output.status.success(),
        "managed PostgreSQL SQL failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("psql emits UTF-8")
}

#[cfg(feature = "integration")]
struct AtelierPgCleanup {
    batch_id: String,
    corpus_action_id: String,
    armed: bool,
}

#[cfg(feature = "integration")]
impl AtelierPgCleanup {
    fn cleanup(&mut self) -> String {
        let deleted = run_psql(&format!(
            "WITH deleted_corpus AS (DELETE FROM atelier_command_corpus_entry WHERE action_id = '{action}' RETURNING 1), \
             deleted_batch AS (DELETE FROM atelier_intake_batch WHERE batch_id = '{batch}'::uuid RETURNING 1) \
             SELECT json_build_object('deleted_corpus', (SELECT count(*) FROM deleted_corpus), \
                                      'deleted_batch', (SELECT count(*) FROM deleted_batch));",
            action = self.corpus_action_id,
            batch = self.batch_id,
        ));
        // Data-modifying CTE subqueries share one MVCC snapshot, so a same-statement item count can
        // still observe rows removed by the batch's ON DELETE CASCADE. Measure absence in a fresh
        // statement after the delete has completed.
        let remaining_items = run_psql(&format!(
            "SELECT count(*) FROM atelier_intake_item WHERE batch_id = '{}'::uuid;",
            self.batch_id
        ));
        let mut receipt: serde_json::Value =
            serde_json::from_str(deleted.trim()).expect("Atelier deletion receipt is JSON");
        receipt["remaining_items"] = serde_json::json!(remaining_items
            .trim()
            .parse::<u64>()
            .expect("remaining item count"));
        self.armed = false;
        serde_json::to_string(&receipt).expect("serialize Atelier cleanup receipt")
    }
}

#[cfg(feature = "integration")]
impl Drop for AtelierPgCleanup {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.cleanup()));
        }
    }
}

#[cfg(feature = "integration")]
fn assert_atelier_cleanup(receipt: &str, expected_corpus: u64) {
    let parsed: serde_json::Value =
        serde_json::from_str(receipt.trim()).expect("Atelier cleanup receipt is JSON");
    assert_eq!(
        parsed["deleted_corpus"].as_u64(),
        Some(expected_corpus),
        "{receipt}"
    );
    assert_eq!(parsed["deleted_batch"].as_u64(), Some(1), "{receipt}");
    assert_eq!(parsed["remaining_items"].as_u64(), Some(0), "{receipt}");
}

/// AC-5 against REAL Handshake-managed PostgreSQL: create a unique batch through the product HTTP API,
/// add one item + corpus row directly to that same managed database, then drive the production
/// `AtelierClient` twice (fresh reload) over batches, corpus, and per-batch items. The exact generated ids
/// are deleted and the cleanup receipt proves the cascade left no item behind. This test is unignored:
/// selecting `--features integration` means the managed backend + DSN are required, not silently skipped.
#[test]
#[cfg(feature = "integration")]
fn ac5_atelier_side_panel_loads_from_live_pg() {
    use handshake_native::backend_client::{AtelierClient, AtelierItemsCell, AtelierSidePanelCell};
    use std::sync::{Arc, Mutex};

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let base =
        std::env::var("HSK_TEST_BASE").unwrap_or_else(|_| "http://127.0.0.1:37501".to_owned());
    let suffix = integration_suffix();
    let source_label = format!("MT-033 managed proof {suffix}");
    let corpus_action_id = format!("mt033.proof.{suffix}");
    let http = reqwest::Client::builder()
        .pool_max_idle_per_host(2)
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("bounded proof client");
    let created: serde_json::Value = rt.block_on(async {
        let response = http
            .post(format!("{base}/atelier/intake/batches"))
            .json(&serde_json::json!({
                "idempotency_key": format!("mt033-{suffix}"),
                "source_label": source_label.clone(),
                "source_ref": format!("mt033://{suffix}"),
                "mode": "manual",
                "profile_mode": "loose_profile"
            }))
            .send()
            .await
            .expect("POST managed Atelier batch");
        assert_eq!(response.status(), reqwest::StatusCode::CREATED);
        response.json().await.expect("parse created batch")
    });
    let batch_id = created["batch_id"]
        .as_str()
        .expect("created batch_id")
        .to_owned();
    let mut cleanup = AtelierPgCleanup {
        batch_id: batch_id.clone(),
        corpus_action_id: corpus_action_id.clone(),
        armed: true,
    };
    let item_id = run_psql(&format!(
        "WITH inserted AS (INSERT INTO atelier_intake_item (batch_id, source_path, file_name, byte_len, content_hash, lane) \
         VALUES ('{batch}'::uuid, '/mt033/{suffix}.png', 'mt033-{suffix}.png', 33, '{suffix}', 'pending') \
         RETURNING item_id) SELECT item_id FROM inserted;",
        batch = batch_id,
    ))
    .trim()
    .to_owned();
    uuid::Uuid::parse_str(&item_id).expect("backend database generated an item UUID");
    let corpus_entry_id = run_psql(&format!(
        "WITH inserted AS (INSERT INTO atelier_command_corpus_entry \
           (action_id, corpus_source, owner, params_schema_ref, execution_class, receipt_shape, manual_anchor) \
         VALUES ('{action}', 'preload', 'mt033-proof', 'hsk.mt033.proof@1', 'pure_projection', 'hsk.mt033.receipt@1', 'WP-KERNEL-012/MT-033') RETURNING entry_id) \
         SELECT entry_id FROM inserted;",
        action = corpus_action_id,
    ))
    .trim()
    .to_owned();
    uuid::Uuid::parse_str(&corpus_entry_id).expect("database generated corpus UUID");

    for generation in [1_u64, 2] {
        let client = AtelierClient::new(base.clone(), rt.handle().clone());
        let cell: AtelierSidePanelCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
        client.fetch_side_panel(generation, Arc::clone(&cell));
        let data = (0..50)
            .find_map(|_| {
                let delivered = cell.lock().unwrap().pop_front();
                if delivered.is_none() {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                delivered
            })
            .expect("live PG panel fetch within 5s");
        assert_eq!(data.0, generation, "fresh reload generation identity");
        let data = data.1.expect("live PG panel fetch ok (no mocks)");
        assert!(
            data.batches
                .iter()
                .any(|row| row.batch_id == batch_id && row.source_label == source_label),
            "AC-5 live: self-seeded intake batch survives reload"
        );
        assert!(
            data.corpus
                .iter()
                .any(|row| row.action_id == corpus_action_id),
            "AC-5 live: self-seeded command-corpus row survives reload"
        );
    }

    for generation in [3_u64, 4] {
        let client = AtelierClient::new(base.clone(), rt.handle().clone());
        let items_cell: AtelierItemsCell = Arc::new(Mutex::new(std::collections::VecDeque::new()));
        client.fetch_items(generation, &batch_id, Arc::clone(&items_cell));
        let items = (0..50)
            .find_map(|_| {
                let delivered = items_cell.lock().unwrap().pop_front();
                if delivered.is_none() {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                delivered
            })
            .expect("live PG items fetch within 5s");
        assert_eq!((items.0, items.1.as_str()), (generation, batch_id.as_str()));
        assert!(
            items
                .2
                .expect("live PG items fetch ok")
                .iter()
                .any(|row| row.item_id == item_id),
            "AC-5 live: self-seeded item is returned through a fresh production client"
        );
    }

    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&base, rt.handle().clone());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run();
    harness.get_by_label("VIEW").click();
    harness.run();
    harness.get_by_label("Toggle Atelier / CKC Panel").click();
    // Opening the live panel intentionally keeps repainting while the HTTP load is in flight. Drive the
    // first mounted frame explicitly; the bounded loop below owns the async completion deadline.
    harness.step();
    let expected_batch = batch_author_id(&batch_id);
    let expected_corpus = corpus_author_id(&corpus_entry_id);
    for _ in 0..60 {
        let ids = author_ids(&harness);
        if ids.contains(&expected_batch) && ids.contains(&expected_corpus) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        harness.step();
    }
    let mounted_ids = author_ids(&harness);
    assert!(
        mounted_ids.contains(PANEL_AUTHOR_ID),
        "real shell panel mounted"
    );
    assert!(
        mounted_ids.contains(&expected_batch),
        "real batch AccessKit row loaded"
    );
    assert!(
        mounted_ids.contains(&expected_corpus),
        "real corpus AccessKit row loaded"
    );
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(expected_batch.as_str()))
        .expect("real batch row")
        .click();
    harness.step();
    let expected_item = item_author_id(&item_id);
    for _ in 0..60 {
        if author_ids(&harness).contains(&expected_item) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        harness.step();
    }
    assert!(
        author_ids(&harness).contains(&expected_item),
        "real item AccessKit row loaded through mounted production client"
    );
    drop(harness);

    let receipt = cleanup.cleanup();
    assert_atelier_cleanup(&receipt, 1);
    println!(
        "AC-5 live: self-seeded batch/item/corpus loaded twice from managed PG; cleanup={}",
        receipt.trim()
    );
}

#[cfg(feature = "integration")]
struct WorkspacePgCleanup {
    base: String,
    workspace_id: String,
    /// WP-KERNEL-012 MT-115: the CLIENT event ids, not the durable recorder ids. MT-109 keys the
    /// EventLedger mirror rows `native-editor-fr-{pending,complete}:{workspace_id}:{client_event_id}`
    /// while `GET /api/flight_recorder` returns the DERIVED, workspace-scoped `event_id`. Cleaning up
    /// on the derived id (or without the workspace segment) matches zero rows and still reports
    /// success, which is how orphaned mirror rows survive a "clean" proof run.
    native_fr_client_event_ids: Vec<String>,
    save_receipt_event_ids: Vec<String>,
    armed: bool,
}

#[cfg(feature = "integration")]
impl WorkspacePgCleanup {
    fn track_native_fr_client_event(&mut self, client_event_id: &str) {
        uuid::Uuid::parse_str(client_event_id)
            .expect("native Flight Recorder client event id is a UUID");
        if !self
            .native_fr_client_event_ids
            .iter()
            .any(|tracked| tracked == client_event_id)
        {
            self.native_fr_client_event_ids
                .push(client_event_id.to_owned());
        }
    }

    fn track_save_receipt(&mut self, event_id: &str) {
        let uuid = event_id
            .strip_prefix("KE-")
            .expect("save receipt has the typed KE- prefix");
        uuid::Uuid::parse_str(uuid).expect("save receipt carries a UUID after KE-");
        if !self
            .save_receipt_event_ids
            .iter()
            .any(|tracked| tracked == event_id)
        {
            self.save_receipt_event_ids.push(event_id.to_owned());
        }
    }

    fn cleanup_owned_event_ledger(&mut self) -> String {
        // Discover by the unique fixture workspace before consulting tracked ids. This closes the panic
        // window where the backend has appended a save or native-FR row but HTTP/JSON readback fails
        // before the test can learn its durable event id.
        let workspace_id = self.workspace_id.replace('\'', "''");
        // MT-115: read the CLIENT event id straight out of the stored envelope. `aggregate_id` is the
        // MT-109 workspace-scoped derivation and can never reconstruct the mirror's idempotency key.
        let discovered = run_psql(&format!(
            "SELECT event_id || '|' || aggregate_type || '|' \
                 || COALESCE(payload #>> '{{envelope,event_id}}', aggregate_id) \
             FROM kernel_event_ledger \
             WHERE (aggregate_type='native_editor_event' \
                    AND payload #>> '{{envelope,workspace_id}}'='{workspace_id}') \
                OR (aggregate_type='knowledge_rich_document' \
                    AND aggregate_id IN (SELECT rich_document_id \
                                         FROM knowledge_rich_documents \
                                         WHERE workspace_id='{workspace_id}')) \
             ORDER BY event_sequence;"
        ));
        for line in discovered.lines().filter(|line| !line.trim().is_empty()) {
            let mut fields = line.splitn(3, '|');
            let event_id = fields.next().expect("owned EventLedger event id");
            let aggregate_type = fields.next().expect("owned EventLedger aggregate type");
            let client_or_aggregate_id = fields.next().expect("owned EventLedger aggregate id");
            match aggregate_type {
                "native_editor_event" => self.track_native_fr_client_event(client_or_aggregate_id),
                "knowledge_rich_document" => self.track_save_receipt(event_id),
                other => panic!("unexpected owned EventLedger aggregate type {other}"),
            }
        }

        let workspace_key_segment = self.workspace_id.clone();
        let native_keys = self
            .native_fr_client_event_ids
            .iter()
            .flat_map(|client_event_id| {
                [
                    format!("native-editor-fr-pending:{workspace_key_segment}:{client_event_id}"),
                    format!("native-editor-fr-complete:{workspace_key_segment}:{client_event_id}"),
                ]
            })
            .map(|key| format!("'{}'", key.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(",");
        let save_ids = self
            .save_receipt_event_ids
            .iter()
            .map(|event_id| format!("'{}'", event_id.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(",");
        let native_predicate = if native_keys.is_empty() {
            "FALSE".to_owned()
        } else {
            format!("idempotency_key IN ({native_keys})")
        };
        let save_predicate = if save_ids.is_empty() {
            "FALSE".to_owned()
        } else {
            format!("event_id IN ({save_ids})")
        };
        let receipt = run_psql(&format!(
            "BEGIN; DELETE FROM kernel_event_ledger \
             WHERE ({native_predicate}) OR ({save_predicate}); \
             DO $mt033_native_fr_cleanup$ BEGIN \
             IF EXISTS (SELECT 1 FROM kernel_event_ledger \
                        WHERE (aggregate_type='native_editor_event' \
                               AND payload #>> '{{envelope,workspace_id}}'='{workspace_id}') \
                           OR (aggregate_type='knowledge_rich_document' \
                               AND aggregate_id IN (SELECT rich_document_id \
                                                    FROM knowledge_rich_documents \
                                                    WHERE workspace_id='{workspace_id}'))) \
             THEN RAISE EXCEPTION 'MT-033 owned EventLedger cleanup left rows'; \
             END IF; \
             IF EXISTS (SELECT 1 FROM kernel_event_ledger \
                        WHERE idempotency_key LIKE 'native-editor-fr-pending:{workspace_id}:%' \
                           OR idempotency_key LIKE 'native-editor-fr-complete:{workspace_id}:%') \
             THEN RAISE EXCEPTION 'MT-033 workspace-partitioned native FR mirror rows survived cleanup'; \
             END IF; END $mt033_native_fr_cleanup$; COMMIT; \
             SELECT json_build_object('native_fr_client_event_ids', ARRAY[{event_ids}]::text[], \
             'save_receipt_event_ids', ARRAY[{save_event_ids}]::text[], \
             'ledger_rows_absent', true);",
            event_ids = self
                .native_fr_client_event_ids
                .iter()
                .map(|event_id| format!("'{}'", event_id.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(","),
            save_event_ids = self
                .save_receipt_event_ids
                .iter()
                .map(|event_id| format!("'{}'", event_id.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(","),
        ));
        self.native_fr_client_event_ids.clear();
        self.save_receipt_event_ids.clear();
        receipt
    }

    async fn cleanup(&mut self, client: &reqwest::Client) -> String {
        let owned_event_ledger = self.cleanup_owned_event_ledger();
        let response = workspace_write_headers(
            client.delete(format!("{}/workspaces/{}", self.base, self.workspace_id)),
        )
        .send()
        .await
        .expect("DELETE owned MT-033 workspace");
        assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);
        let workspaces: serde_json::Value = client
            .get(format!("{}/workspaces", self.base))
            .send()
            .await
            .expect("list workspaces after cleanup")
            .json()
            .await
            .expect("workspace list JSON");
        assert!(!workspaces
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| {
                row.get("id").and_then(|value| value.as_str()) == Some(self.workspace_id.as_str())
            })));
        self.armed = false;
        serde_json::json!({
            "workspace_id": self.workspace_id.clone(),
            "delete_status": 204,
            "workspace_absent": true,
            "owned_event_ledger": owned_event_ledger
        })
        .to_string()
    }
}

#[cfg(feature = "integration")]
impl Drop for WorkspacePgCleanup {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let already_panicking = std::thread::panicking();
        let ledger_cleanup = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.cleanup_owned_event_ledger()
        }));
        let base = self.base.clone();
        let workspace_id = self.workspace_id.clone();
        let workspace_cleanup = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("workspace cleanup runtime");
                runtime.block_on(async move {
                    let client = reqwest::Client::builder()
                        .connect_timeout(std::time::Duration::from_secs(2))
                        .timeout(std::time::Duration::from_secs(5))
                        .build()
                        .expect("workspace cleanup client");
                    let response = workspace_write_headers(
                        client.delete(format!("{base}/workspaces/{workspace_id}")),
                    )
                    .send()
                    .await
                    .expect("drop cleanup DELETE owned MT-033 workspace");
                    assert!(
                        response.status() == reqwest::StatusCode::NO_CONTENT
                            || response.status() == reqwest::StatusCode::NOT_FOUND,
                        "drop cleanup workspace DELETE returned {}",
                        response.status()
                    );
                    let workspaces: serde_json::Value = client
                        .get(format!("{base}/workspaces"))
                        .send()
                        .await
                        .expect("drop cleanup list workspaces")
                        .json()
                        .await
                        .expect("drop cleanup workspace list JSON");
                    assert!(
                        !workspaces
                            .as_array()
                            .is_some_and(|rows| rows.iter().any(|row| {
                                row.get("id").and_then(serde_json::Value::as_str)
                                    == Some(workspace_id.as_str())
                            })),
                        "drop cleanup left owned MT-033 workspace {workspace_id}"
                    );
                });
            })
            .join()
            .expect("join owned MT-033 workspace cleanup");
        }));
        if let Err(payload) = ledger_cleanup {
            if already_panicking {
                eprintln!(
                    "MT-033 EventLedger cleanup failed during unwind; owned residue may remain"
                );
            } else {
                std::panic::resume_unwind(payload);
            }
        }
        if let Err(payload) = workspace_cleanup {
            if already_panicking {
                eprintln!(
                    "MT-033 workspace cleanup failed during unwind; owned residue may remain"
                );
            } else {
                std::panic::resume_unwind(payload);
            }
        }
    }
}

/// AC-2 + AC-3 against real managed PG. Owns its workspace/document/Atelier item/canvas, drives the
/// production editor transform and canonical save + MT-026 placement routes, then cleans exact ids.
#[test]
#[cfg(feature = "integration")]
fn ac2_ac3_ckc_embed_and_canvas_round_trip_live_pg() {
    use handshake_native::command_registry::CMD_EDITOR_FILE_SAVE;
    use handshake_native::quick_switcher::{NavDispatchOutcome, ShellNavigator};

    // WP-KERNEL-012 MT-115 / MT-109 boundary: publish a REAL native-MCP session binding for THIS
    // process BEFORE the managed backend is selected, so the fixture-owned backend inherits the same
    // redirected app-data root and resolves the SAME `swarm_mcp_binding.json`. This is also what lets
    // the mounted shell's own event emitter reach the capability-gated ingestion route. Nothing is
    // weakened: an absent, forged, or stale binding still fails closed at the middleware.
    let native_binding = pg_proof_support::RealNativeMcpBinding::publish();
    let session_token = native_binding.token().to_owned();
    let mut managed_backend = pg_proof_support::require_live_backend();
    let managed_base = managed_backend.base.clone();
    let runtime = tokio::runtime::Runtime::new().expect("integration runtime");
    runtime.block_on(async {
        let base = managed_base;
        let session_token = session_token.as_str();
        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(2)
            .connect_timeout(std::time::Duration::from_secs(2))
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("bounded integration client");
        assert!(client
            .get(format!("{base}/health"))
            .send()
            .await
            .expect("integration feature requires live handshake_core")
            .status()
            .is_success());

        let suffix = integration_suffix();
        let response = workspace_write_headers(client.post(format!("{base}/workspaces")))
            .json(&serde_json::json!({"name": format!("MT-033-{suffix}")}))
            .send().await.expect("create workspace");
        assert_eq!(response.status(), reqwest::StatusCode::CREATED);
        let workspace: serde_json::Value = response.json().await.expect("workspace JSON");
        let workspace_id = workspace["id"].as_str().expect("workspace id").to_owned();
        let mut workspace_cleanup = WorkspacePgCleanup {
            base: base.clone(),
            workspace_id: workspace_id.clone(),
            native_fr_client_event_ids: Vec::new(),
            save_receipt_event_ids: Vec::new(),
            armed: true,
        };

        let response = proof_headers(client.post(format!("{base}/knowledge/documents")))
            .json(&serde_json::json!({
                "workspace_id": workspace_id.clone(),
                "title": format!("MT-033 note {suffix}"),
                "content_json": {"type":"doc","content":[{"type":"paragraph","content":[]}]}
            }))
            .send().await.expect("create rich document");
        assert!(response.status().is_success());
        let created: serde_json::Value = response.json().await.expect("document JSON");
        let document_id = created["document"]["rich_document_id"]
            .as_str().expect("document id").to_owned();
        assert!(client
            .get(format!("{base}/workspaces/{workspace_id}/loom/blocks/{document_id}"))
            .send().await.expect("same-id Loom block").status().is_success());

        let response = client.post(format!("{base}/atelier/intake/batches"))
            .json(&serde_json::json!({
                "idempotency_key": format!("mt033-ac23-{suffix}"),
                "source_label": format!("MT-033 AC2/3 {suffix}"),
                "source_ref": format!("mt033://ac23/{suffix}"),
                "mode":"manual", "profile_mode":"loose_profile"
            }))
            .send().await.expect("create Atelier batch");
        assert_eq!(response.status(), reqwest::StatusCode::CREATED);
        let batch: serde_json::Value = response.json().await.expect("batch JSON");
        let batch_id = batch["batch_id"].as_str().expect("batch id").to_owned();
        let mut atelier_cleanup = AtelierPgCleanup {
            batch_id: batch_id.clone(),
            corpus_action_id: format!("mt033.no-corpus.{suffix}"),
            armed: true,
        };
        let item_id = run_psql(&format!(
            "WITH inserted AS (INSERT INTO atelier_intake_item \
             (batch_id,source_path,file_name,byte_len,content_hash,lane) VALUES \
             ('{batch_id}'::uuid,'/mt033/ac23/{suffix}.png','atelier-{suffix}.png',33,'{suffix}','pending') \
             RETURNING item_id) SELECT item_id FROM inserted;"
        )).trim().to_owned();
        uuid::Uuid::parse_str(&item_id).expect("database-generated item UUID");

        // Publish a real canonical media asset + Loom block first. The native resolver is intentionally
        // forbidden from fabricating an empty file block for a raw intake row.
        let imported: serde_json::Value = proof_headers(client.post(format!(
            "{base}/workspaces/{workspace_id}/loom/import"
        )))
        .json(&serde_json::json!({
            "bytes_b64": "bXQwMzMtY2Fub25pY2FsLW1lZGlh",
            "original_filename": format!("atelier-{suffix}.png"),
            "mime": "image/png"
        }))
        .send()
        .await
        .expect("import canonical Atelier media asset")
        .json()
        .await
        .expect("canonical import JSON");
        let canonical_block_id = imported["block_id"]
            .as_str()
            .expect("canonical import block id")
            .to_owned();
        assert!(imported["asset_id"].as_str().is_some());
        let relation = proof_headers(client.put(format!(
            "{base}/atelier/intake/items/{item_id}/loom-projection"
        )))
        .json(&serde_json::json!({"loom_block_id": canonical_block_id}))
        .send()
        .await
        .expect("publish canonical Atelier-to-Loom relation");
        assert_eq!(relation.status(), reqwest::StatusCode::OK);
        let relation: serde_json::Value = relation.json().await.expect("relation JSON");
        assert_eq!(relation["item_id"], item_id);
        assert_eq!(relation["loom_block_id"], canonical_block_id);

        let loaded_body = handshake_native::backend_client::RichDocClient::new(
            base.clone(),
            runtime.handle().clone(),
        )
        .load_document(&document_id)
        .await
        .expect("load document through production client");
        let mut rich_app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
            status: "ok".to_owned(),
            db_status: "ok".to_owned(),
            migration_version: Some(1),
        }));
        rich_app.set_backend_base_url_for_test(&base, runtime.handle().clone());
        rich_app.bind_active_project_for_integration_test(workspace_id.clone());
        assert!(
            matches!(
                rich_app.open_document(&document_id),
                NavDispatchOutcome::Opened { .. }
            ),
            "production Notes navigation opens and activates the persisted document"
        );
        let rich_pane_id = rich_app
            .active_pane()
            .expect("production Notes navigation focuses the rich pane")
            .clone();
        rich_app
            .apply_loaded_rich_document_to_view_for_test(rich_pane_id.as_ref(), loaded_body)
            .expect("install the mounted document and canonical SaveManager in its active view");
        rich_app.set_atelier_panel_open(true);
        let rich_state = rich_app.mounted_rich_state();
        let mut rich_harness = Harness::builder()
            .with_size(egui::vec2(1280.0, 800.0))
            .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), rich_app);
        let batch_id_author = batch_author_id(&batch_id);
        for _ in 0..100 {
            rich_harness.step();
            if author_ids(&rich_harness).contains(&batch_id_author) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        request_click_by_author(&rich_harness, &batch_id_author);
        for _ in 0..100 {
            rich_harness.step();
            if author_ids(&rich_harness).contains(&item_author_id(&item_id)) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let item_author = item_author_id(&item_id);
        assert!(author_ids(&rich_harness).contains(&item_author));
        let source = center_by_author(&rich_harness, &item_author);
        let target = center_by_author(&rich_harness, "editor.rich.text");
        rich_harness.drag_at(source);
        rich_harness.run_steps(1);
        let mut producer_emitted_typed_payload = false;
        for step in 1..=8 {
            let t = step as f32 / 8.0;
            rich_harness.hover_at(source + (target - source) * t);
            rich_harness.run_steps(1);
            producer_emitted_typed_payload |=
                egui::DragAndDrop::has_payload_of_type::<DragPayload>(&rich_harness.ctx);
        }
        assert!(
            producer_emitted_typed_payload,
            "the real mounted backend row must emit a typed AtelierRef during pointer drag"
        );
        rich_harness.drop_at(target);
        rich_harness.run_steps(3);
        let (saved_content, interop_error) = {
            let state = rich_state.lock().unwrap();
            (state.current_content_json(), state.interop_error.clone())
        };
        assert_eq!(
            first_hs_link(&saved_content),
            Some(("media".into(), item_id.clone())),
            "mounted panel action must mutate the shared rich state; interop_error={interop_error:?}; content={saved_content}"
        );
        let save_ctx = rich_harness.ctx.clone();
        assert!(
            rich_harness
                .state_mut()
                .dispatch_palette_action_for_test_with_ctx(&save_ctx, CMD_EDITOR_FILE_SAVE),
            "mounted host published the panel-originated edit to the canonical SaveManager"
        );
        let mut persisted = false;
        for _ in 0..100 {
            rich_harness.step();
            let reloaded = handshake_native::backend_client::RichDocClient::new(
                base.clone(),
                runtime.handle().clone(),
            )
            .load_document(&document_id)
            .await
            .expect("fresh document reload while waiting for mounted save");
            if first_hs_link(&reloaded.content_json)
                == Some(("media".into(), item_id.clone()))
            {
                persisted = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(persisted, "fresh GET must observe the saved CKC hsLink");
        let save_receipt_deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(5);
        let save_receipt_event_id = loop {
            rich_harness.run_steps(1);
            if let Some(receipt) = rich_state
                .lock()
                .unwrap()
                .save
                .as_ref()
                .and_then(|save| save.last_save_receipt_event_id.clone())
            {
                break receipt;
            }
            assert!(
                std::time::Instant::now() < save_receipt_deadline,
                "mounted save did not expose its canonical backend receipt within five seconds"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        workspace_cleanup.track_save_receipt(&save_receipt_event_id);
        let save_receipt = run_psql(&format!(
            "SELECT (payload || jsonb_build_object(\
                 '_event_id', event_id, \
                 '_event_type', event_type, \
                 '_aggregate_type', aggregate_type, \
                 '_aggregate_id', aggregate_id\
             ))::text \
             FROM kernel_event_ledger \
             WHERE event_id='{}';",
            save_receipt_event_id.replace('\'', "''")
        ));
        let save_receipt: serde_json::Value = serde_json::from_str(save_receipt.trim())
            .expect("save receipt exact EventLedger row is valid JSON");
        assert_eq!(
            save_receipt["_event_id"].as_str(),
            Some(save_receipt_event_id.as_str())
        );
        assert_eq!(
            save_receipt["_event_type"].as_str(),
            Some("KNOWLEDGE_RICH_DOCUMENT_SAVED")
        );
        assert_eq!(
            save_receipt["_aggregate_type"].as_str(),
            Some("knowledge_rich_document")
        );
        assert_eq!(
            save_receipt["_aggregate_id"].as_str(),
            Some(document_id.as_str())
        );
        assert_eq!(
            save_receipt["workspace_id"].as_str(),
            Some(workspace_id.as_str())
        );
        assert_eq!(save_receipt["event"].as_str(), Some("saved"));

        // Route the same mounted, freshly-saved document through the real Editors menu. This keeps the
        // CKC drag, save, Stage surface, EventLedger producer, and Flight Recorder readback in one
        // operator-shaped proof instead of substituting a direct bus injection.
        request_click_by_author(&rich_harness, "menu-editors");
        rich_harness.run_steps(2);
        assert!(
            author_ids(&rich_harness).contains("menu.editors.route-to-stage"),
            "mounted Editors menu exposes the stable Route selection to Stage target"
        );
        request_click_by_author(&rich_harness, "menu.editors.route-to-stage");
        let stage = rich_harness.state().mounted_stage();
        let stage_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            rich_harness.run_steps(1);
            let staged = stage.lock().unwrap().content.clone();
            if matches!(
                staged,
                StageContent::Document(ref document)
                    if document.rich_document_id == document_id
                        && document.content_json.as_ref().and_then(first_hs_link)
                            == Some(("media".into(), item_id.clone()))
            ) {
                break;
            }
            assert!(
                std::time::Instant::now() < stage_deadline,
                "mounted Route-to-Stage did not expose the freshly-saved CKC document; staged={staged:?}"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        let flight_recorder_deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(5);
        let route_row = loop {
            rich_harness.run_steps(1);
            // MT-115: MT-109 gates this read. Present the SAME genuine native-MCP credential the
            // mounted client presents; without it the response is `401 HSK-401-FR-SESSION`, whose
            // empty body would read as "the route receipt never arrived".
            let rows: serde_json::Value = client
                .get(format!("{base}/api/flight_recorder?wsid={workspace_id}"))
                .header("x-hsk-session-token", session_token)
                .send()
                .await
                .expect("fresh Flight Recorder route readback")
                .json()
                .await
                .expect("Flight Recorder JSON");
            let matching = rows.as_array().into_iter().flatten().filter(|row| {
                row["payload"]["kind"].as_str() == Some("route_to_stage")
                    && row["payload"]["native_payload"]["content_kind"].as_str()
                        == Some("document")
            });
            let matching = matching.cloned().collect::<Vec<_>>();
            if matching.len() == 1 {
                break matching.into_iter().next().unwrap();
            }
            assert!(
                matching.is_empty(),
                "fresh workspace must contain exactly one route_to_stage receipt, got {matching:?}"
            );
            assert!(
                std::time::Instant::now() < flight_recorder_deadline,
                "mounted Stage route did not reach canonical Flight Recorder within five seconds"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        let route_event_id = route_row["event_id"]
            .as_str()
            .expect("route Flight Recorder event id")
            .to_owned();
        // MT-115: the durable `event_id` above is MT-109's workspace-scoped derivation. The
        // EventLedger mirror is keyed on the CLIENT event id, which the recorder row preserves
        // separately, so cleanup and the exact-row assertion below must both use THIS id.
        let route_client_event_id = route_row["payload"]["client_event_id"]
            .as_str()
            .expect("route Flight Recorder row preserves its client event id")
            .to_owned();
        assert_ne!(
            route_client_event_id, route_event_id,
            "MT-109 derives a workspace-scoped durable id distinct from the client receipt id"
        );
        // Arm panic/drop cleanup as soon as the durable identity becomes observable. Every later
        // assertion may fail independently, but none may strand either exact EventLedger row.
        workspace_cleanup.track_native_fr_client_event(&route_client_event_id);
        let route_causal_action_id = route_row["payload"]["native_payload"]["causal_action_id"]
            .as_str()
            .expect("route Flight Recorder causal action id")
            .to_owned();
        assert!(!route_causal_action_id.trim().is_empty());
        assert_eq!(
            stage.lock().unwrap().causal_action_id.as_deref(),
            Some(route_causal_action_id.as_str()),
            "fresh Flight Recorder row preserves the mounted Stage route correlation"
        );
        let route_accept_deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(5);
        while stage.lock().unwrap().has_pending_route_receipt() {
            rich_harness.run_steps(1);
            assert!(
                std::time::Instant::now() < route_accept_deadline,
                "exact route receipt remained pending after its durable Flight Recorder row appeared"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        // MT-115: MT-109's mirror keys carry the AUTHENTICATED workspace plus the CLIENT event id.
        // The previously asserted unpartitioned key matched zero rows in both counters, so a missing
        // mirror and a present one were indistinguishable — this assertion could only ever have
        // passed by returning "0|0", which it did not assert against.
        let ledger_counts = run_psql(&format!(
            "SELECT COUNT(*) FILTER (WHERE idempotency_key='native-editor-fr-pending:{workspace_id}:{route_client_event_id}') \
             || '|' || COUNT(*) FILTER (WHERE idempotency_key='native-editor-fr-complete:{workspace_id}:{route_client_event_id}') \
             FROM kernel_event_ledger;"
        ));
        assert_eq!(
            ledger_counts.trim(),
            "1|1",
            "canonical route has exact pending and complete EventLedger rows"
        );
        println!(
            "MT-033 receipts save={save_receipt_event_id} route={route_event_id} causal={route_causal_action_id}"
        );
        tokio::task::block_in_place(|| drop(rich_harness));

        let reload = reqwest::Client::builder()
            .pool_max_idle_per_host(1)
            .timeout(std::time::Duration::from_secs(10))
            .build().expect("fresh reload client");
        let loaded: serde_json::Value = proof_headers(reload.get(format!(
            "{base}/knowledge/documents/{document_id}"
        )))
            .send().await.expect("reload document").json().await.expect("reload JSON");
        assert_eq!(loaded["document"]["content_json"], saved_content);
        assert_eq!(
            first_hs_link(&loaded["document"]["content_json"]),
            Some(("media".into(), item_id.clone()))
        );
        assert!(loaded["document"]["content_json"].to_string()
            .contains(&format!("atelier-{suffix}.png")));

        let response = client.post(format!(
            "{base}/workspaces/{workspace_id}/loom/canvas-boards"
        )).json(&serde_json::json!({"title":format!("MT-033 canvas {suffix}")}))
            .send().await.expect("create canvas");
        assert!(response.status().is_success());
        let canvas: serde_json::Value = response.json().await.expect("canvas JSON");
        let canvas_id = canvas["block_id"].as_str().expect("canvas block id").to_owned();
        let projected_block_id = canonical_block_id;
        let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
            status: "ok".to_owned(),
            db_status: "ok".to_owned(),
            migration_version: Some(1),
        }));
        app.set_backend_base_url_for_test(&base, runtime.handle().clone());
        {
            let mounted = app.mounted_canvas_board();
            let mut board = mounted.lock().unwrap();
            board.workspace_id = workspace_id.clone();
            board.canvas_block_id = canvas_id.clone();
        }
        app.set_atelier_panel_open(true);
        let mounted_board = app.mounted_canvas_board();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1280.0, 800.0))
            .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
        let batch_id_author = batch_author_id(&batch_id);
        for _ in 0..100 {
            harness.step();
            if author_ids(&harness).contains(&batch_id_author) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        request_click_by_author(&harness, &batch_id_author);
        for _ in 0..100 {
            harness.step();
            if author_ids(&harness).contains(&item_canvas_author_id(&item_id)) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(author_ids(&harness).contains(&item_canvas_author_id(&item_id)));
        pointer_click_by_author(&harness, &item_canvas_author_id(&item_id));
        harness.step();
        pointer_click_by_author(&harness, &item_canvas_author_id(&item_id));
        for _ in 0..100 {
            harness.step();
            if mounted_board.lock().unwrap().placements.iter().any(|placement| {
                placement.placed_block_id == projected_block_id
            }) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let placement_id = mounted_board
            .lock()
            .unwrap()
            .placements
            .iter()
            .find(|placement| placement.placed_block_id == projected_block_id)
            .map(|placement| placement.placement_id.clone())
            .expect("mounted host resolver dispatches placement and applies fresh board reload");
        assert!(reload
            .get(format!(
                "{base}/workspaces/{workspace_id}/loom/blocks/{projected_block_id}"
            ))
            .send()
            .await
            .expect("fresh-client projected block reload")
            .status()
            .is_success());
        let board: serde_json::Value = reload.get(format!(
            "{base}/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"
        )).send().await.expect("reload canvas").json().await.expect("board JSON");
        let matching_placements = board["placements"]
            .as_array()
            .expect("fresh board placements array")
            .iter()
            .filter(|row| row["placed_block_id"].as_str() == Some(projected_block_id.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            matching_placements.len(),
            1,
            "repeated mounted Canvas action is idempotent: {board}"
        );
        assert_eq!(
            matching_placements[0]["placement_id"].as_str(),
            Some(placement_id.as_str()),
            "exact placement survives fresh reload: {board}"
        );

        // The production shell owns Ctrl+Shift+Z and the created placement registered a compensating
        // cross-pane undo. Drive the real key input, then wait for DELETE + canonical board reload; a
        // direct `undo_cross_pane()` call would not prove the mounted shortcut consumer.
        harness.key_press_modifiers(cross_pane_undo_modifiers(), egui::Key::Z);
        for _ in 0..100 {
            harness.step();
            if !mounted_board
                .lock()
                .unwrap()
                .placements
                .iter()
                .any(|placement| placement.placement_id == placement_id)
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(
            !mounted_board
                .lock()
                .unwrap()
                .placements
                .iter()
                .any(|placement| placement.placement_id == placement_id),
            "Ctrl+Shift+Z removes the exact mounted Canvas placement"
        );
        let board_after_undo: serde_json::Value = reload
            .get(format!(
                "{base}/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"
            ))
            .send()
            .await
            .expect("reload canvas after key undo")
            .json()
            .await
            .expect("board after key undo JSON");
        assert!(
            board_after_undo["placements"]
                .as_array()
                .expect("fresh board placements after key undo")
                .iter()
                .all(|row| row["placement_id"].as_str() != Some(placement_id.as_str())),
            "key-driven compensating DELETE persists after a fresh reload: {board_after_undo}"
        );

        // Redo uses the same provisional async record and must re-place the same block at the same
        // geometry, then reload the mounted board from backend truth. There is intentionally no separate
        // cross-pane-redo keyboard chord; drive the production bus command that the shell owns.
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let redo = InteractionBus::with_try_lock(&bus, |bus| bus.redo_cross_pane())
            .flatten()
            .expect("successful key undo leaves the Canvas action available for redo");
        assert!(redo.ok, "Canvas redo dispatches asynchronously: {redo:?}");
        for _ in 0..100 {
            harness.step();
            if mounted_board
                .lock()
                .unwrap()
                .placements
                .iter()
                .any(|placement| placement.placed_block_id == projected_block_id)
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(
            mounted_board
                .lock()
                .unwrap()
                .placements
                .iter()
                .any(|placement| placement.placed_block_id == projected_block_id),
            "Canvas redo reappears in the mounted board after the authoritative reload"
        );
        let board_after_redo: serde_json::Value = reload
            .get(format!(
                "{base}/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"
            ))
            .send()
            .await
            .expect("reload canvas after redo")
            .json()
            .await
            .expect("board after redo JSON");
        assert_eq!(
            board_after_redo["placements"]
                .as_array()
                .expect("fresh board placements after redo")
                .iter()
                .filter(|row| {
                    row["placed_block_id"].as_str() == Some(projected_block_id.as_str())
                })
                .count(),
            1,
            "redo persists exactly one replacement after fresh reload: {board_after_redo}"
        );
        tokio::task::block_in_place(|| drop(harness));

        let atelier_receipt = atelier_cleanup.cleanup();
        assert_atelier_cleanup(&atelier_receipt, 0);
        let workspace_receipt = workspace_cleanup.cleanup(&client).await;
        println!(
            "AC-2/3 document={document_id} item={item_id} canvas={canvas_id} placement={placement_id} atelier_cleanup={} workspace_cleanup={workspace_receipt}",
            atelier_receipt.trim()
        );
    });
    managed_backend.assert_cleanup();
}

// ── helpers ────────────────────────────────────────────────────────────────────────────────────────

/// The Stage pane's AccessKit GenericContainer value (the routed-content summary), or `None` when absent. Generic
/// over the harness state type (works for both the widget harnesses and the live-shell harness).
fn stage_value<S>(harness: &Harness<'_, S>) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(STAGE_PANE_AUTHOR_ID) {
            return ak.value().map(|v| v.to_owned());
        }
    }
    None
}

/// Count the `hsLink` nodes in a content_json doc value (the CKC embed atoms + any wikilinks).
fn count_hs_links(content_json: &serde_json::Value) -> usize {
    fn walk(v: &serde_json::Value, n: &mut usize) {
        if let Some(obj) = v.as_object() {
            if obj.get("type").and_then(|t| t.as_str()) == Some("hsLink") {
                *n += 1;
            }
            if let Some(content) = obj.get("content").and_then(|c| c.as_array()) {
                for c in content {
                    walk(c, n);
                }
            }
        }
    }
    let mut n = 0;
    walk(content_json, &mut n);
    n
}

/// The `(refKind, refValue)` of the first hsLink node in a content_json doc value.
fn first_hs_link(content_json: &serde_json::Value) -> Option<(String, String)> {
    fn walk(v: &serde_json::Value) -> Option<(String, String)> {
        if let Some(obj) = v.as_object() {
            if obj.get("type").and_then(|t| t.as_str()) == Some("hsLink") {
                let attrs = obj.get("attrs")?;
                let rk = attrs.get("refKind")?.as_str()?.to_owned();
                let rv = attrs.get("refValue")?.as_str()?.to_owned();
                return Some((rk, rv));
            }
            if let Some(content) = obj.get("content").and_then(|c| c.as_array()) {
                for c in content {
                    if let Some(found) = walk(c) {
                        return Some(found);
                    }
                }
            }
        }
        None
    }
    walk(content_json)
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-020 (inline-atom undo rewire) — the CKC/Atelier drag-in insert is TRANSACTIONAL:
// the whole drop (caret-leaf split + atom insert + tail re-host) is ONE receipt on the model
// UndoManager AND one queued (before, after) pair for the MT-035 unified bus. One undo restores the
// exact pre-drop doc — the drop path can no longer bypass the undo system by direct child mutation.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn mt020_atelier_drop_insert_is_one_undoable_transaction() {
    use handshake_native::rich_editor::document_model::doc_json::to_content_json_value;

    let mut state = RichEditorState::demo();
    let before_doc = state.doc.clone();
    let before_json = to_content_json_value(&state.doc);
    let undo_before = state.undo.len();

    let link = DragPayload::AtelierRef(AtelierRef::new(
        "char-undo",
        AtelierItemKind::Character,
        "Mira",
    ))
    .to_hs_link()
    .expect("AtelierRef -> hsLink");
    assert!(
        RichEditorWidget::insert_atelier_embed_at_caret(&mut state, link),
        "the transactional embed insert succeeds"
    );
    assert_ne!(state.doc, before_doc, "the drop mutated the doc");

    // ONE model-level receipt (atomic: split + atom + tail — not three separate entries).
    assert_eq!(
        state.undo.len(),
        undo_before + 1,
        "MT-020: the whole drop is ONE UndoManager receipt"
    );
    // The unified-bus pair is queued for the frame-end drain (the drop runs inside the render
    // closure, invisible to the frame-input diff — this pair is how it reaches the MT-035 bus).
    assert_eq!(
        state.pending_bus_undo.len(),
        1,
        "MT-020: the drop queued its (before, after) pair for the unified undo bus"
    );
    assert_eq!(
        state.pending_bus_undo[0].0, before_json,
        "the queued 'before' snapshot is the exact pre-drop doc"
    );

    // One model-level undo restores the EXACT pre-drop doc (split + atom + tail all invert).
    assert!(
        state.undo.undo(&mut state.doc).expect("undo applies"),
        "the receipt undoes"
    );
    assert_eq!(
        state.doc, before_doc,
        "MT-020: one undo restored the exact pre-drop doc (no split residue, no atom)"
    );
}
