//! MT-014 remediation (FAIL_V2): canonical Argus inspect / safe-steer / re-observe proof for the
//! MOUNTED media-embed states.
//!
//! `validation_v2` failed MT-014 because "there is no canonical Argus inspect/steer/re-observe
//! evidence for the mounted embed states". The isolated `test_embeds.rs` kittest coverage drives the
//! embed WIDGET, but never the mounted `HandshakeApp` through the real localhost `SwarmMcpServer`
//! transport the way an out-of-process swarm agent does. This test closes that exact gap:
//!
//!   1. mounts the production `HandshakeApp` shell with a rich document whose blocks are the four
//!      media-embed states — loaded image, missing asset, corrupt asset, and still-loading — seeded
//!      into the mounted rich editor's embed runtime,
//!   2. binds the CANONICAL Argus driver (real localhost JSON-RPC, the same `argus.inspect` /
//!      `argus.click` the swarm path uses) to the mounted app,
//!   3. `argus.inspect` proves ALL FOUR states are addressable by stable author_id in the live tree
//!      (`embed-image-{id}`, `embed-error-{id}` for missing + corrupt, `embed-loading-{id}`),
//!   4. drives ONE safe, reversible action (open the single-image full-size modal) through Argus,
//!   5. FRESH `argus.inspect` re-observes the post-action state (the modal node appears), and
//!   6. writes the before/after tree evidence externally + a screenshot marker (headless DEFERRED is
//!      an acceptable typed outcome).
//!
//! Artifact hygiene (CX-212E): every artifact is written ONLY under the EXTERNAL
//! `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-014/` root.

use std::path::{Path, PathBuf};

use egui_kittest::kittest::NodeT;

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, json_node_by_author_id, CanonicalArgusDriver};

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::rich_editor::document_model::node::{BlockNode, Child, HsLinkNode, NodeKind};
use handshake_native::rich_editor::embeds::asset_resolver::{
    EmbedAssetMetadata, EmbedError, EmbedResolutionState, ResolvedAsset,
};
use handshake_native::rich_editor::embeds::image_view::decode_rgba;

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

fn assert_no_local_artifact_dir() {
    for local in [Path::new("test_output"), Path::new("tests/screenshots")] {
        assert!(
            !local.exists(),
            "CX-212E: no repo-local artifact dir may exist (found {})",
            local.display()
        );
    }
}

fn json_has_enabled_click_target(value: &serde_json::Value, author_id: &str) -> bool {
    json_node_by_author_id(value, author_id).is_some_and(|node| {
        node.get("disabled").and_then(serde_json::Value::as_bool) == Some(false)
            && node
                .get("actions")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|actions| {
                    actions
                        .iter()
                        .any(|action| action.as_str() == Some("Click"))
                })
    })
}

fn json_retains_non_rejected_click_receipt(
    value: &serde_json::Value,
    receipt_id: u64,
    target: &str,
) -> bool {
    value
        .get("action_receipts")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|receipts| {
            receipts.iter().any(|receipt| {
                receipt
                    .get("receipt_id")
                    .and_then(serde_json::Value::as_u64)
                    == Some(receipt_id)
                    && receipt.get("target").and_then(serde_json::Value::as_str) == Some(target)
                    && receipt
                        .get("expected_action")
                        .and_then(serde_json::Value::as_str)
                        == Some("Click")
                    && receipt.get("status").and_then(serde_json::Value::as_str) == Some("applied")
                    && receipt
                        .get("rejection")
                        .map_or(true, serde_json::Value::is_null)
            })
        })
}

fn json_click_completion_token(value: &serde_json::Value, author_id: &str) -> serde_json::Value {
    let raw = json_node_by_author_id(value, author_id)
        .and_then(|node| node.get("value"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("{author_id} carries a click-completion value"));
    serde_json::from_str(raw).unwrap_or_else(|error| {
        panic!("{author_id} click-completion value is strict JSON: {error}")
    })
}

fn json_has_static_enabled_image(value: &serde_json::Value, author_id: &str) -> bool {
    json_node_by_author_id(value, author_id).is_some_and(|node| {
        node.get("role").and_then(serde_json::Value::as_str) == Some("Image")
            && node.get("disabled").and_then(serde_json::Value::as_bool) == Some(false)
            && node
                .get("actions")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|actions| {
                    !actions
                        .iter()
                        .any(|action| matches!(action.as_str(), Some("Click" | "Focus")))
                })
    })
}

fn json_bounds(value: &serde_json::Value, author_id: &str) -> Option<(f64, f64, f64, f64)> {
    let bounds = json_node_by_author_id(value, author_id)?.get("bounds")?;
    let x = bounds.get("x")?.as_f64()?;
    let y = bounds.get("y")?.as_f64()?;
    let width = bounds.get("w")?.as_f64()?;
    let height = bounds.get("h")?.as_f64()?;
    (x.is_finite()
        && y.is_finite()
        && width.is_finite()
        && height.is_finite()
        && width > 0.0
        && height > 0.0)
        .then_some((x, y, width, height))
}

fn json_dialog_contains(value: &serde_json::Value, dialog_id: &str, child_id: &str) -> bool {
    let Some((dialog_x, dialog_y, dialog_w, dialog_h)) = json_bounds(value, dialog_id) else {
        return false;
    };
    let Some((child_x, child_y, child_w, child_h)) = json_bounds(value, child_id) else {
        return false;
    };
    child_x >= dialog_x
        && child_y >= dialog_y
        && child_x + child_w <= dialog_x + dialog_w
        && child_y + child_h <= dialog_y + dialog_h
}

/// A live, RUNTIME-INJECTED shell whose top-right pane is RE-TYPED to the Notes/rich editor
/// (`PaneType::LoomWikiPage`) so the real `RichEditorPaneMount` factory renders the seeded document
/// under `HandshakeApp::ui` — the SAME host-mount sequence the MT-079 proofs use (test_app_host_mount).
/// A fresh `with_health` app has NO active rich pane, so its `active_rich_state()` is never laid out;
/// without this re-type the mounted embed nodes never enter the AccessKit tree. The runtime is returned
/// alongside the app so it OUTLIVES the harness (a dropped runtime would unbind the mounted editor).
fn live_shell() -> (HandshakeApp, tokio::runtime::Runtime) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime for the mounted embed shell");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    {
        let registry = app.pane_registry();
        let mut guard = registry.lock().expect("pane registry");
        guard.insert(PaneRecord::new(
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

/// A standalone media embed paragraph (the `hsLink` atom by ref_kind) — the shape the renderer
/// routes to the interactive embed path.
fn embed_block(ref_kind: &str, ref_value: &str) -> BlockNode {
    BlockNode::with_children(
        NodeKind::Paragraph,
        vec![Child::HsLink(HsLinkNode::new(ref_kind, ref_value, ""))],
    )
}

fn resolved_image(asset_id: &str) -> ResolvedAsset {
    ResolvedAsset {
        asset: EmbedAssetMetadata {
            asset_id: asset_id.to_owned(),
            workspace_id: "ws".to_owned(),
            kind: "image".to_owned(),
            mime: "image/png".to_owned(),
            original_filename: Some(format!("{asset_id}.png")),
            content_hash: "hash".to_owned(),
            size_bytes: 16,
            width: Some(40),
            height: Some(20),
        },
        content_url: format!("http://b/workspaces/ws/assets/{asset_id}/content"),
        thumbnail_url: format!("http://b/workspaces/ws/assets/{asset_id}/content?tier=thumb"),
        preview_url: format!("http://b/workspaces/ws/assets/{asset_id}/content?tier=preview"),
        poster_url: format!("http://b/workspaces/ws/assets/{asset_id}/content?tier=poster"),
    }
}

/// A small in-memory 40x20 two-colour PNG, decoded so the mounted render can upload a real texture
/// for the loaded-image state (making it a clickable image the safe Argus action targets).
fn sample_color_image() -> egui::ColorImage {
    let mut img = image::RgbaImage::new(40, 20);
    for (x, _y, px) in img.enumerate_pixels_mut() {
        *px = if x < 20 {
            image::Rgba([220, 40, 40, 255])
        } else {
            image::Rgba([40, 120, 220, 255])
        };
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut buf, image::ImageFormat::Png)
        .unwrap();
    decode_rgba(&buf.into_inner()).expect("sample PNG decodes")
}

#[test]
fn mt014_mounted_embed_states_canonical_argus_inspect_steer_reobserve() {
    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-014/canonical-argus");
    let tree_path = artifact_dir.join("mt014-mounted-embed-states-argus.json");
    let screenshot_path = artifact_dir.join("mt014-mounted-embed-states.png");
    // Fail closed against stale V3/V4 proof before any runtime, backend, harness, or Argus setup.
    // Remove only this test's two exact final artifacts; they are republished below only after the
    // canonical driver proves terminal state and shuts down cleanly.
    for stale_path in [&tree_path, &screenshot_path] {
        match std::fs::remove_file(stale_path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!(
                "remove stale MT-014 proof artifact {}: {error}",
                stale_path.display()
            ),
        }
    }

    // `_runtime` must outlive the harness: the mounted rich editor unbinds if its runtime is dropped.
    let (app, _runtime) = live_shell();

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    // MOUNT FIRST: the rich pane factory creates and owns its editor state on the first live frames.
    // Seeding before this writes into a pre-mount state the pane subsequently replaces — the mounted
    // pane then renders its own default document and no embed node ever enters the AccessKit tree.
    harness.run_steps(3);

    // Seed the four embed states into the MOUNTED rich editor's embed runtime. `decoded_images` is
    // pre-populated for the loaded image so the next mounted frame uploads a real texture on the egui
    // thread (the production upload path), making `embed-image-loaded1` a clickable image.
    {
        let rich = harness.state().mounted_rich_state();
        let mut state = rich.lock().unwrap();
        state.doc = BlockNode::doc(vec![
            embed_block("images", "loaded1"),
            embed_block("images", "missing1"),
            embed_block("images", "corrupt1"),
            embed_block("images", "loading1"),
        ]);
        state.embeds.resolutions.insert(
            "images:loaded1",
            EmbedResolutionState::Ok(resolved_image("loaded1")),
        );
        let loaded_image = sample_color_image();
        state
            .embeds
            .decoded_images
            .insert("images:thumb:loaded1".to_owned(), loaded_image.clone());
        state
            .embeds
            .decoded_images
            .insert("images:full:loaded1".to_owned(), loaded_image);
        state.embeds.resolutions.insert(
            "images:missing1",
            EmbedResolutionState::Err(EmbedError::NotFound("missing1".to_owned())),
        );
        state.embeds.resolutions.insert(
            "images:corrupt1",
            EmbedResolutionState::Err(EmbedError::MediaLoadFailed(
                "could not decode image".to_owned(),
            )),
        );
        state
            .embeds
            .resolutions
            .insert("images:loading1", EmbedResolutionState::Resolving);
    }

    // Frames: upload the seeded texture on the egui thread + settle the AccessKit tree.
    harness.run_steps(4);

    let mut argus =
        CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-014-embed-states");

    // (1) Canonical inspect: all FOUR mounted embed states are addressable by stable author_id.
    let before = argus.inspect(&mut harness);
    for author in [
        "embed-image-loaded1",
        "embed-error-missing1",
        "embed-error-corrupt1",
        "embed-loading-loading1",
    ] {
        assert!(
            json_has_author_id(&before, author),
            "canonical argus.inspect must see the mounted embed state '{author}' in the live tree"
        );
    }
    // The single-image modal is NOT open before the action.
    assert!(
        !json_has_author_id(&before, "embed-image-modal-loaded1"),
        "the loaded-image modal is closed before the safe action"
    );
    let ready_token = json_click_completion_token(&before, "embed-image-loaded1");
    assert_eq!(ready_token["schema"], "handshake.click-completion/v1");
    assert_eq!(ready_token["mode"], "same_target");
    assert_eq!(ready_token["effect"], "image-modal-open");
    assert_eq!(ready_token["state"], "ready");
    let ready_generation = ready_token["generation"]
        .as_u64()
        .expect("ready image token carries a generation");
    let ready_context = ready_token["context"]
        .as_str()
        .expect("ready image token carries stable context")
        .to_owned();

    // (2) Safe, reversible steer: click the loaded image (opens its full-size modal). This changes no
    // durable/backend/external state — it toggles an in-editor overlay.
    let observation = argus.click_and_reinspect(&mut harness, "embed-image-loaded1");
    assert_eq!(
        observation.receipt_status, "applied",
        "the exact image-modal token transition must produce a raw Applied receipt"
    );
    assert!(
        observation
            .agent_id
            .contains(":client:wp-kernel-012-mt-014-embed-states-agent"),
        "the canonical receipt retains the external caller attribution: {}",
        observation.agent_id
    );

    // (3) Bind this exact attributed click to an authoritative, freshly captured terminal tree.
    // The modal transition is terminal only when the click's exact receipt is retained, every
    // pre-existing edge state remains visible, and the modal exposes a usable close control. This
    // predicate is intentionally recomputed from the fresh tree; `evidence` is correlation data,
    // never a replacement pass flag.
    let receipt_id = observation.receipt_id;
    let receipt_status = observation.receipt_status.clone();
    let agent_id = observation.agent_id.clone();
    let terminal_after = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "mt014.loaded-image-click.opens-steerable-modal-and-retains-edge-states",
        serde_json::json!({
            "receipt_id": receipt_id,
            "receipt_status": receipt_status,
            "agent_id": agent_id,
            "target": "embed-image-loaded1",
            "required_modal": "embed-image-modal-loaded1",
            "required_close": "embed-image-modal-close-loaded1",
            "required_full_image": "embed-image-full-loaded1",
            "ready_click_token": ready_token.clone(),
            "required_edge_states": [
                "embed-error-missing1",
                "embed-error-corrupt1",
                "embed-loading-loading1",
            ],
        }),
        |after| {
            let applied_token = json_click_completion_token(after, "embed-image-loaded1");
            json_has_author_id(after, "embed-image-modal-loaded1")
                && json_has_enabled_click_target(after, "embed-image-modal-close-loaded1")
                && json_has_static_enabled_image(after, "embed-image-full-loaded1")
                && json_has_author_id(after, "embed-image-loaded1")
                && json_has_author_id(after, "embed-error-missing1")
                && json_has_author_id(after, "embed-error-corrupt1")
                && json_has_author_id(after, "embed-loading-loading1")
                && json_retains_non_rejected_click_receipt(after, receipt_id, "embed-image-loaded1")
                && applied_token["schema"] == "handshake.click-completion/v1"
                && applied_token["mode"] == "same_target"
                && applied_token["effect"] == "image-modal-open"
                && applied_token["context"] == ready_context
                && applied_token["generation"].as_u64() == ready_generation.checked_add(1)
                && applied_token["state"] == "applied"
                && json_dialog_contains(
                    after,
                    "embed-image-modal-loaded1",
                    "embed-image-modal-close-loaded1",
                )
                && json_dialog_contains(
                    after,
                    "embed-image-modal-loaded1",
                    "embed-image-full-loaded1",
                )
        },
    );
    let applied_token = json_click_completion_token(&terminal_after, "embed-image-loaded1");

    // Paint one ordinary mounted-shell frame after the isolated canonical snapshot. This proves the
    // modal remains open in the operator's live viewport (and gives the GPU capture the exact
    // post-action frame) instead of publishing pixels from the pre-action painter cache.
    harness.run_steps(1);
    for author in [
        "embed-image-modal-loaded1",
        "embed-image-modal-close-loaded1",
        "embed-image-full-loaded1",
    ] {
        assert!(
            harness
                .root()
                .children_recursive()
                .any(|node| node.accesskit_node().author_id() == Some(author)),
            "mounted post-action frame retains {author}"
        );
    }
    let screenshot = harness.render();

    // (4) Prepare evidence in memory, then publish final artifacts only after `finish` has proven
    // the canonical action log, terminal predicate, caller attribution, lease reclamation, and
    // server shutdown. A failed proof therefore cannot publish current-run success artifacts.
    let tree_evidence = serde_json::to_vec_pretty(&serde_json::json!({
        "terminal_predicate": {
            "predicate_id": "mt014.loaded-image-click.opens-steerable-modal-and-retains-edge-states",
            "passed": true,
            "expected_modal_id": "embed-image-modal-loaded1",
            "expected_close_id": "embed-image-modal-close-loaded1",
            "expected_full_image_id": "embed-image-full-loaded1",
            "expected_raw_receipt_status": "applied",
            "ready_click_token": ready_token,
            "applied_click_token": applied_token,
            "expected_edge_ids": [
                "embed-error-missing1",
                "embed-error-corrupt1",
                "embed-loading-loading1",
            ],
            "receipt_id": receipt_id,
        },
        "before": before,
        "after": terminal_after,
        "receipt_id": receipt_id,
        "receipt_status": receipt_status,
        "agent_id": agent_id,
    }))
    .expect("serialize canonical MT-014 embed-state tree evidence");

    argus.finish();
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-014 Argus artifact dir");
    std::fs::write(&tree_path, tree_evidence)
        .expect("write canonical MT-014 embed-state tree evidence externally");
    assert!(tree_path.is_file());

    let screenshot_marker = match screenshot {
        Ok(image) => {
            image
                .save(&screenshot_path)
                .expect("save mounted embed-states screenshot");
            format!("CAPTURED {}", screenshot_path.display())
        }
        Err(deferred) => format!("DEFERRED (headless): {deferred}"),
    };
    println!(
        "MT-014 canonical Argus mounted embed states: inspect(4 states) -> click(embed-image-loaded1) \
         -> reinspect(modal open); receipt={} agent={} screenshot={} tree={}",
        receipt_status,
        agent_id,
        screenshot_marker,
        tree_path.display()
    );
    assert_no_local_artifact_dir();
}
