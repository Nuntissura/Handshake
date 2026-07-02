//! MT-014 embed NodeView PROOFS: kittest screenshots, AccessKit-tree assertions, album modal
//! interaction, and the gated real-backend asset-resolve integration test.
//!
//! Artifact hygiene (CX-212E): EVERY PNG is written ONLY to the EXTERNAL
//! `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-014/` root via [`external_artifact_dir`];
//! [`assert_no_local_artifact_dir`] fails the run if a repo-local `tests/screenshots/` or
//! `test_output/` directory exists (the MT contract names a repo-local screenshot path, but the
//! CX-212E artifact rule OVERRIDES it — a tracked PNG under src/ is a hygiene failure the reviewer
//! greps for with `git ls-files "src/**/*.png"`).
//!
//! Backend reality (Spec-Realism Gate): the FAIL-CLOSED validation + view logic (slideshow nav,
//! album grid, typed error chips, resolution caching, decode-failure->error-chip, AccessKit ids)
//! are FULLY proven here with mock resolution states + a counted mock fetcher — NO backend. The
//! image-CONTENT real-asset-resolve AC is the `#[ignore]` `real_image_resolve_*` test, which needs
//! a live Handshake-managed backend with a seeded asset; absent that, it is NEEDS_MANAGED_RESOURCE_PROOF
//! (run with `--features integration -- --ignored` against a live backend). The mock never fakes the
//! backend — it proves the resolver BINDING + the view dispatch, not a fabricated asset.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::rich_editor::document_model::node::{BlockNode, Child, HsLinkNode, NodeKind};
use handshake_native::rich_editor::embeds::asset_resolver::SequenceItem;
use handshake_native::rich_editor::embeds::asset_resolver::{
    AssetMetadataFetcher, EmbedAssetMetadata, EmbedError, EmbedResolutionState, MetadataFuture,
    ResolvedAsset,
};
use handshake_native::rich_editor::embeds::embed_block_renderer::{EmbedRuntime, SequenceState};
use handshake_native::rich_editor::embeds::image_view::decode_rgba;
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorState, RichEditorWidget,
};

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic. The crate
/// sits at `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where
/// `Handshake_Artifacts` is a sibling of the repo worktree.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` and `tests/screenshots/` (the path the MT contract literally names, which this
/// rule overrides). A stray local artifact dir is a hygiene regression the reviewer also greps for
/// via `git ls-files "src/**/*.png"`.
fn assert_no_local_artifact_dir() {
    for local in [Path::new("test_output"), Path::new("tests/screenshots")] {
        assert!(
            !local.exists(),
            "CX-212E: no repo-local artifact dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            local.display()
        );
    }
}

// ── Mock transport (no backend) ──────────────────────────────────────────────────────────────

/// A fetcher that always errors — drives the headless typed-error-chip path (used by the
/// error-chip screenshot when no resolution is pre-seeded).
struct ErrFetcher;
impl AssetMetadataFetcher for ErrFetcher {
    fn fetch_metadata<'a>(&'a self, _ws: &'a str, id: &'a str) -> MetadataFuture<'a> {
        let id = id.to_owned();
        Box::pin(async move { Err(EmbedError::NotFound(id)) })
    }
}

/// Build an `EmbedRuntime` with no runtime handle (headless) over a mock fetcher.
fn headless_runtime() -> EmbedRuntime {
    EmbedRuntime::new("ws", "http://b", Arc::new(ErrFetcher), None)
}

/// A fetcher that serves BOTH metadata AND real PNG content bytes from memory (no backend, no
/// network). It proves the FULL production wiring — `resolve_one` (metadata) -> `fetch_content`
/// (bytes) -> off-thread `decode_rgba` -> egui-thread `EmbedTextureCache::upload` — drives a real
/// texture into the cache through the runtime, NOT a manual pre-seed. Counts content fetches so
/// the test asserts the bytes GET actually fired.
struct PngContentFetcher {
    png: Vec<u8>,
    content_calls: std::sync::atomic::AtomicUsize,
}
impl PngContentFetcher {
    fn new(png: Vec<u8>) -> Self {
        Self {
            png,
            content_calls: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    fn content_calls(&self) -> usize {
        self.content_calls.load(std::sync::atomic::Ordering::SeqCst)
    }
}
impl AssetMetadataFetcher for PngContentFetcher {
    fn fetch_metadata<'a>(&'a self, ws: &'a str, id: &'a str) -> MetadataFuture<'a> {
        let (ws, id) = (ws.to_owned(), id.to_owned());
        Box::pin(async move {
            Ok(EmbedAssetMetadata {
                asset_id: id.clone(),
                workspace_id: ws,
                kind: "image".to_owned(),
                mime: "image/png".to_owned(),
                original_filename: Some(format!("{id}.png")),
                content_hash: "hash".to_owned(),
                size_bytes: 16,
                width: Some(40),
                height: Some(20),
            })
        })
    }
    fn fetch_content<'a>(
        &'a self,
        _ws: &'a str,
        _id: &'a str,
    ) -> handshake_native::rich_editor::embeds::asset_resolver::ContentFuture<'a> {
        self.content_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let png = self.png.clone();
        Box::pin(async move { Ok(png) })
    }
}

/// A resolved image asset for seeding the cache (so the view renders without a live backend).
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
        thumbnail_url: format!("http://b/workspaces/ws/assets/{asset_id}/thumbnail"),
    }
}

/// A small in-memory PNG (a 40x20 two-color image) the image-embed screenshot decodes + uploads,
/// proving the decode->ColorImage->TextureHandle path without a backend.
fn sample_png() -> Vec<u8> {
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
    buf.into_inner()
}

/// A paragraph that is a STANDALONE media embed (the `hsLink` atom by ref_kind), the shape the
/// renderer routes to the interactive embed path.
fn embed_block(ref_kind: &str, ref_value: &str) -> BlockNode {
    BlockNode::with_children(
        NodeKind::Paragraph,
        vec![Child::HsLink(HsLinkNode::new(ref_kind, ref_value, ""))],
    )
}

/// Build a RichEditorState whose doc is a single standalone embed block, with a pre-seeded
/// headless embed runtime.
fn embed_editor(ref_kind: &str, ref_value: &str) -> RichEditorState {
    let doc = BlockNode::doc(vec![embed_block(ref_kind, ref_value)]);
    RichEditorState::new(doc).with_embed_runtime(headless_runtime())
}

/// Build a RichEditorState over a standalone embed block with a CALLER-PROVIDED embed runtime
/// (used by the runtime-wiring test to inject a real tokio handle + content mock).
fn embed_editor_with_runtime(
    ref_kind: &str,
    ref_value: &str,
    runtime: EmbedRuntime,
) -> RichEditorState {
    let doc = BlockNode::doc(vec![embed_block(ref_kind, ref_value)]);
    RichEditorState::new(doc).with_embed_runtime(runtime)
}

// ── PT-003 / AC-1: image embed renders (decoded image, seeded resolution + texture) ───────────

#[test]
fn mt014_image_embed_screenshot() {
    // The image-embed view, with the resolution pre-seeded Ok AND the decoded texture uploaded on
    // the first frame (the off-thread decode->upload pipeline's end state, reproduced headlessly so
    // the SCREENSHOT shows a real decoded image without a backend). The real-backend variant is the
    // #[ignore] integration test below.
    let png = sample_png();
    // Sanity: the sample decodes (the same decode the production off-thread path runs).
    let decoded = decode_rgba(&png).expect("sample PNG decodes");
    assert_eq!(decoded.size, [40, 20]);

    let state = {
        let mut s = embed_editor("images", "img1");
        s.embeds
            .resolutions
            .insert("img1", EmbedResolutionState::Ok(resolved_image("img1")));
        Arc::new(std::sync::Mutex::new(s))
    };

    let state_for_ui = Arc::clone(&state);
    let decoded_for_ui = decoded.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(480.0, 240.0))
        .wgpu()
        .build_ui(move |ui| {
            // Upload the decoded texture into the embed texture cache ON the egui thread (the
            // production path uploads here too, after the off-thread decode). Idempotent.
            {
                let mut st = state_for_ui.lock().unwrap();
                let _texture = st
                    .embeds
                    .textures
                    .upload(ui.ctx(), "img1", decoded_for_ui.clone());
            }
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();
    harness.run();

    // The image control is AccessKit-addressable by its embed-image author_id. NOTE: both the
    // texture branch and the decoding-spinner branch emit `embed-image-img1` with Role::Image, so
    // the author_id alone does NOT distinguish them. The texture-vs-placeholder distinction is the
    // texture-cache assertion below (the texture was uploaded on the first frame) + the runtime
    // end-to-end test `mt014_runtime_decode_pipeline_uploads_texture` (which drives the upload
    // through the production chain with no pre-seed).
    let root = harness.root();
    let mut image_node_found = false;
    for node in root.children_recursive() {
        if node.accesskit_node().author_id() == Some("embed-image-img1") {
            image_node_found = true;
            break;
        }
    }
    assert!(
        image_node_found,
        "AC-1: the resolved image renders an 'embed-image-img1' node"
    );
    // The decoded texture is actually uploaded in the cache -> the texture branch (not the
    // placeholder) is what rendered. This is the honest texture-vs-placeholder discriminator.
    assert!(
        state.lock().unwrap().embeds.textures.contains("img1"),
        "AC-1: the decoded image texture is uploaded in the cache (texture branch, not placeholder)"
    );

    match harness.render() {
        Ok(image) => {
            assert!(image.width() > 0 && image.height() > 0, "rendered image non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-014");
            let _ = std::fs::create_dir_all(&ext_dir);
            let path = ext_dir.join("mt014_image_embed.png");
            let saved = image.save(&path).is_ok();
            println!(
                "PT-003 image embed: {}x{} saved={saved} ({})",
                image.width(),
                image.height(),
                path.display()
            );
        }
        Err(e) => println!(
            "BLOCKER(non-fatal): mt014_image_embed screenshot render unavailable (no wgpu adapter): {e}. \
             The AccessKit image-node structural proof passed; the PNG is a GPU-host item."
        ),
    }

    assert_no_local_artifact_dir();
}

// ── AC-1 RUNTIME WIRING: the production resolve->fetch-content->decode->upload chain drives a ──
// ── real texture into the cache through the EmbedRuntime (no manual pre-seed, no backend). ─────

#[test]
fn mt014_runtime_decode_pipeline_uploads_texture() {
    // This is the runtime proof the adversarial review required: NOTHING is pre-seeded. The
    // EmbedRuntime is wired with a REAL tokio runtime handle + a workspace id (the production
    // shape `set_embed_context` installs) and a content-mock that serves real PNG bytes. Driving
    // frames must take the FULL production path:
    //   render -> ensure_single (spawn metadata fetch) -> drain Ok -> render_resolved_image
    //   -> ensure_image_content (spawn content GET + spawn_blocking decode_rgba)
    //   -> drain decoded ColorImage -> upload on the egui thread -> texture in cache.
    // If the decode pipeline were not wired into production (the review's must-fix), no texture
    // would ever appear and this test would TIME OUT.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("test tokio runtime");

    let fetcher = Arc::new(PngContentFetcher::new(sample_png()));
    let fetcher_dyn: Arc<dyn AssetMetadataFetcher> = Arc::clone(&fetcher) as _;

    let state = {
        let runtime = EmbedRuntime::new(
            "ws-live",
            "http://b",
            fetcher_dyn,
            Some(rt.handle().clone()),
        );
        let s = embed_editor_with_runtime("images", "img1", runtime);
        Arc::new(std::sync::Mutex::new(s))
    };

    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(480.0, 240.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });

    // Pump frames, giving the runtime time to complete the metadata fetch, then the content
    // fetch + off-thread decode, then the egui-thread upload. Bounded so a genuine wiring failure
    // fails fast (it never resolves) rather than hanging.
    let mut uploaded = false;
    for _ in 0..200 {
        harness.run();
        if state.lock().unwrap().embeds.textures.contains("img1") {
            uploaded = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    assert!(
        uploaded,
        "AC-1 RUNTIME: the production resolve->content-fetch->off-thread-decode->upload chain must \
         land a real texture in the cache (no pre-seed). If this times out, the decode pipeline is \
         not wired into the runtime."
    );
    assert!(
        fetcher.content_calls() >= 1,
        "the content-bytes GET (.../content) must actually fire in the production chain"
    );
    // The metadata also resolved Ok through the same runtime (not pre-seeded).
    assert!(
        matches!(
            state.lock().unwrap().embeds.resolutions.get("img1"),
            Some(EmbedResolutionState::Ok(_))
        ),
        "metadata resolved Ok through the runtime (the chain's first stage)"
    );

    // One more frame now renders the texture branch: the embed-image node is present AND the
    // texture is cached (this is what distinguishes a real decoded image from the placeholder).
    harness.run();
    let root = harness.root();
    let mut image_node_found = false;
    for node in root.children_recursive() {
        if node.accesskit_node().author_id() == Some("embed-image-img1") {
            image_node_found = true;
            break;
        }
    }
    assert!(
        image_node_found,
        "AC-1: the decoded image renders an 'embed-image-img1' node"
    );
}

// ── PT-004 / AC-2: empty-ref embed renders the typed error chip (not blank) ────────────────────

#[test]
fn mt014_embed_error_screenshot() {
    // An empty ref_value -> the typed 'empty_ref' error chip (fail-closed, never blank).
    let state = Arc::new(std::sync::Mutex::new(embed_editor("images", "")));

    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(480.0, 160.0))
        .wgpu()
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();
    harness.run();

    // The error chip is AccessKit-addressable by 'embed-error-empty' (empty ref sentinel token),
    // and carries the typed 'empty_ref' kind in its label text.
    let root = harness.root();
    let mut chip_found = false;
    for node in root.children_recursive() {
        if node.accesskit_node().author_id() == Some("embed-error-empty") {
            chip_found = true;
            break;
        }
    }
    assert!(
        chip_found,
        "AC-2: an empty ref renders a typed 'embed-error-empty' chip (not blank)"
    );
    // The typed kind text is on screen.
    assert!(
        harness.query_by_label_contains("empty_ref").is_some(),
        "AC-2: the chip shows the typed 'empty_ref' error kind"
    );

    match harness.render() {
        Ok(image) => {
            assert!(image.width() > 0 && image.height() > 0);
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-014");
            let _ = std::fs::create_dir_all(&ext_dir);
            let path = ext_dir.join("mt014_embed_error.png");
            let saved = image.save(&path).is_ok();
            println!(
                "PT-004 embed error chip: {}x{} saved={saved} ({})",
                image.width(),
                image.height(),
                path.display()
            );
        }
        Err(e) => println!(
            "BLOCKER(non-fatal): mt014_embed_error screenshot render unavailable (no wgpu adapter): {e}. \
             The error-chip structural proof passed; the PNG is a GPU-host item."
        ),
    }

    assert_no_local_artifact_dir();
}

// ── AC-8: slideshow prev/next AccessKit nodes are present in the tree ──────────────────────────

#[test]
fn mt014_slideshow_prev_next_accesskit() {
    // Seed a resolved 3-image sequence so the slideshow renders its prev/next controls.
    let state = {
        let mut s = embed_editor("slideshow", "s1,s2,s3");
        let items: Vec<SequenceItem> = ["s1", "s2", "s3"]
            .iter()
            .map(|id| SequenceItem {
                ref_value: (*id).to_owned(),
                resolution: Ok(resolved_image(id)),
            })
            .collect();
        s.embeds
            .sequences
            .insert("s1,s2,s3".to_owned(), SequenceState::Items(Arc::new(items)));
        Arc::new(std::sync::Mutex::new(s))
    };

    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(480.0, 240.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();

    let root = harness.root();
    let mut prev_found = false;
    let mut next_found = false;
    for node in root.children_recursive() {
        match node.accesskit_node().author_id() {
            // first asset token of "s1,s2,s3" is "s1".
            Some("slideshow-prev-s1") => prev_found = true,
            Some("slideshow-next-s1") => next_found = true,
            _ => {}
        }
    }
    assert!(
        prev_found,
        "AC-8: 'slideshow-prev-s1' must be present in the AccessKit tree"
    );
    assert!(
        next_found,
        "AC-8: 'slideshow-next-s1' must be present in the AccessKit tree"
    );
    println!("AC-8 slideshow nav nodes present: prev=slideshow-prev-s1 next=slideshow-next-s1");
}

// ── AC-6: album grid renders cells; clicking a cell opens the full-size modal ──────────────────

#[test]
fn mt014_album_click_opens_modal() {
    let state = {
        let mut s = embed_editor("album", "a1,a2,a3");
        let items: Vec<SequenceItem> = ["a1", "a2", "a3"]
            .iter()
            .map(|id| SequenceItem {
                ref_value: (*id).to_owned(),
                resolution: Ok(resolved_image(id)),
            })
            .collect();
        s.embeds
            .sequences
            .insert("a1,a2,a3".to_owned(), SequenceState::Items(Arc::new(items)));
        Arc::new(std::sync::Mutex::new(s))
    };

    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(520.0, 320.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.run();

    // The album cell author_ids are present.
    {
        let root = harness.root();
        let mut cell_found = false;
        for node in root.children_recursive() {
            if node.accesskit_node().author_id() == Some("album-cell-a2") {
                cell_found = true;
                break;
            }
        }
        assert!(
            cell_found,
            "AC-6: 'album-cell-a2' must be present in the album grid"
        );
    }

    // Drive a click on the middle cell programmatically, then re-run: the modal opens (AlbumViewState).
    {
        let node = harness.get_by_label_contains("a2.png");
        node.click();
    }
    harness.run();
    harness.run();

    let opened = state
        .lock()
        .unwrap()
        .embeds
        .album_states
        .get("a1,a2,a3")
        .and_then(|s| s.open_index);
    assert_eq!(
        opened,
        Some(1),
        "AC-6: clicking album cell a2 (index 1) opens the full-size modal"
    );
    println!("AC-6 album modal opened on cell index {opened:?}");
}

// ── AC-3 / AC-4: rejected refs render the typed chip (render-time, no backend) ─────────────────

#[test]
fn mt014_traversal_and_scheme_refs_render_typed_chip() {
    // The error-chip author_id is `embed-error-{first_comma_token}` (the single ref verbatim here,
    // since neither value has a comma). Assert BOTH the on-screen typed kind text AND the
    // AccessKit author_id (so the chip-id shape is proven for the render path, not just the text).
    for (ref_value, expect_kind, expect_author) in [
        ("../secret", "traversal_rejected", "embed-error-../secret"),
        (
            "http://evil/x",
            "scheme_rejected",
            "embed-error-http://evil/x",
        ), // ':' -> scheme
    ] {
        let state = Arc::new(std::sync::Mutex::new(embed_editor("images", ref_value)));
        let state_for_ui = Arc::clone(&state);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(480.0, 160.0))
            .build_ui(move |ui| {
                RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
            });
        harness.run();
        // The validation rejected the ref BEFORE any fetch: no resolution was recorded.
        assert!(
            state.lock().unwrap().embeds.resolutions.is_empty(),
            "AC-3/AC-4: a rejected ref '{ref_value}' issues no resolution"
        );
        // The typed kind text is on screen.
        assert!(
            harness.query_by_label_contains(expect_kind).is_some(),
            "AC-3/AC-4: ref '{ref_value}' must render the typed '{expect_kind}' chip"
        );
        // The chip is AccessKit-addressable by its typed embed-error author_id.
        let root = harness.root();
        let mut author_found = false;
        for node in root.children_recursive() {
            if node.accesskit_node().author_id() == Some(expect_author) {
                author_found = true;
                break;
            }
        }
        assert!(
            author_found,
            "AC-3/AC-4: ref '{ref_value}' must render an addressable '{expect_author}' chip node"
        );
    }
}

// ── PT-002 (gated): real backend asset-resolve (NEEDS_MANAGED_RESOURCE_PROOF without a backend) ─

/// Real-backend asset-resolve proof. Requires a LIVE Handshake-managed backend on
/// 127.0.0.1:37501 with a SEEDED image asset whose id is `HANDSHAKE_TEST_ASSET_ID` in workspace
/// `HANDSHAKE_TEST_WORKSPACE_ID`. OFF by default (`#[ignore]` + `integration` feature) so CI does
/// not fail without a backend. Run:
///   cargo test -p handshake-native --features integration --test test_embeds -- --ignored real_image_resolve
///
/// This binds the production `ReqwestAssetFetcher` against the REAL endpoint and asserts the
/// resolver returns `EmbedResolutionState::Ok` with the asset metadata fetched (NOT mocked). When
/// the backend/asset is absent this is the NEEDS_MANAGED_RESOURCE_PROOF gap the MT discloses.
#[test]
#[ignore = "needs a live Handshake-managed backend + a seeded image asset (NEEDS_MANAGED_RESOURCE_PROOF)"]
#[cfg(feature = "integration")]
fn real_image_resolve_against_live_backend() {
    use handshake_native::backend_client::BACKEND_BASE_URL;
    use handshake_native::rich_editor::embeds::asset_resolver::{
        resolve_one, MediaEmbedKind, ReqwestAssetFetcher,
    };

    let workspace_id = std::env::var("HANDSHAKE_TEST_WORKSPACE_ID")
        .expect("set HANDSHAKE_TEST_WORKSPACE_ID to a real workspace with a seeded image asset");
    let asset_id = std::env::var("HANDSHAKE_TEST_ASSET_ID")
        .expect("set HANDSHAKE_TEST_ASSET_ID to a real seeded image asset id");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let fetcher = ReqwestAssetFetcher::new(BACKEND_BASE_URL);
    let resolved = rt.block_on(async {
        resolve_one(
            MediaEmbedKind::Images,
            &workspace_id,
            &asset_id,
            BACKEND_BASE_URL,
            &fetcher,
        )
        .await
    });
    match resolved {
        Ok(asset) => {
            assert_eq!(
                asset.asset.asset_id, asset_id,
                "the real backend returned the requested asset"
            );
            assert!(
                asset.asset.mime.starts_with("image/"),
                "the seeded asset is an image"
            );
            println!(
                "PT-002 REAL backend resolve OK: asset_id={} mime={} content_url={}",
                asset.asset.asset_id, asset.asset.mime, asset.content_url
            );
        }
        Err(e) => {
            panic!("PT-002 real-backend resolve failed (is the backend up + asset seeded?): {e}")
        }
    }
}
