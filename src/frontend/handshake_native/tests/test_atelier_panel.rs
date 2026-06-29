//! MT-006 Atelier main-panel proofs.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use egui::accesskit;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::accessibility::{UiTreeNode, UiTreeSnapshot};
use handshake_native::atelier_panel::{
    ckc_field_suggestion_row_author_id, ckc_folder_row_author_id, ckc_media_album_row_author_id,
    ckc_media_row_author_id, ckc_moodboard_document_row_author_id, ckc_search_result_row_author_id,
    ckc_source_url_row_author_id, ckc_story_document_row_author_id, AtelierPanel,
    ATELIER_CKC_ALBUM_CREATE_AUTHOR_ID, ATELIER_CKC_ALBUM_CREATE_NAME_AUTHOR_ID,
    ATELIER_CKC_ALBUM_CREATE_NOTES_AUTHOR_ID, ATELIER_CKC_ALBUM_CREATE_TAGS_AUTHOR_ID,
    ATELIER_CKC_ALBUM_LINK_ASSET_IDS_AUTHOR_ID, ATELIER_CKC_ALBUM_LINK_AUTHOR_ID,
    ATELIER_CKC_ALBUM_LINK_SOURCE_PATH_AUTHOR_ID, ATELIER_CKC_ALBUM_LINK_SOURCE_URL_AUTHOR_ID,
    ATELIER_CKC_ALBUM_STATUS_AUTHOR_ID, ATELIER_CKC_BOOK_LAYOUT_AUTHOR_ID,
    ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID, ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID,
    ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID, ATELIER_CKC_CHARACTER_CREATE_AUTHOR_ID,
    ATELIER_CKC_CHARACTER_CREATE_NAME_AUTHOR_ID, ATELIER_CKC_CHARACTER_LIST_AUTHOR_ID,
    ATELIER_CKC_CHARACTER_NOTES_APPLY_AUTHOR_ID, ATELIER_CKC_CHARACTER_NOTES_EDITOR_AUTHOR_ID,
    ATELIER_CKC_CHARACTER_REF_AUTHOR_ID, ATELIER_CKC_EXPORT_JSON_AUTHOR_ID,
    ATELIER_CKC_EXPORT_PREVIEW_AUTHOR_ID, ATELIER_CKC_EXPORT_REF_AUTHOR_ID,
    ATELIER_CKC_EXPORT_SAFE_JSON_AUTHOR_ID, ATELIER_CKC_EXPORT_SAFE_TXT_AUTHOR_ID,
    ATELIER_CKC_EXPORT_STATUS_AUTHOR_ID, ATELIER_CKC_EXPORT_TXT_AUTHOR_ID,
    ATELIER_CKC_FIELD_SUGGESTIONS_LIST_AUTHOR_ID, ATELIER_CKC_FIELD_SUGGESTIONS_LOAD_AUTHOR_ID,
    ATELIER_CKC_FIELD_SUGGESTION_FIELD_AUTHOR_ID, ATELIER_CKC_IMPORT_AUTHOR_ID,
    ATELIER_CKC_IMPORT_EDITOR_AUTHOR_ID, ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID,
    ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID, ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID,
    ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID, ATELIER_CKC_MEDIA_VIEWER_AUTHOR_ID,
    ATELIER_CKC_MODE_MOODBOARD_AUTHOR_ID, ATELIER_CKC_MODE_NOTES_AUTHOR_ID,
    ATELIER_CKC_MODE_SHEET_AUTHOR_ID, ATELIER_CKC_MODE_STORY_AUTHOR_ID,
    ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID, ATELIER_CKC_MOODBOARD_DOC_REF_AUTHOR_ID,
    ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID, ATELIER_CKC_MOODBOARD_LATEST_REF_AUTHOR_ID,
    ATELIER_CKC_MOODBOARD_OPEN_AUTHOR_ID, ATELIER_CKC_MOODBOARD_SAVE_AUTHOR_ID,
    ATELIER_CKC_SAFE_SUBSET_LOAD_AUTHOR_ID, ATELIER_CKC_SEARCH_FILTER_CHARACTER_AUTHOR_ID,
    ATELIER_CKC_SEARCH_FILTER_COLLECTION_AUTHOR_ID, ATELIER_CKC_SEARCH_FILTER_MEDIA_AUTHOR_ID,
    ATELIER_CKC_SEARCH_FILTER_SIMILARITY_AUTHOR_ID, ATELIER_CKC_SEARCH_MODE_COMBINED_AUTHOR_ID,
    ATELIER_CKC_SEARCH_MODE_FUZZY_AUTHOR_ID, ATELIER_CKC_SEARCH_MODE_VECTOR_AUTHOR_ID,
    ATELIER_CKC_SEARCH_QUERY_AUTHOR_ID, ATELIER_CKC_SEARCH_RESULTS_AUTHOR_ID,
    ATELIER_CKC_SEARCH_RUN_AUTHOR_ID, ATELIER_CKC_SEARCH_STATUS_AUTHOR_ID,
    ATELIER_CKC_SEARCH_TAGS_AUTHOR_ID, ATELIER_CKC_SELECTED_CHARACTER_AUTHOR_ID,
    ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID, ATELIER_CKC_SHEET_SAVE_AUTHOR_ID,
    ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID, ATELIER_CKC_STORY_BEAT_EDITOR_AUTHOR_ID,
    ATELIER_CKC_STORY_BEAT_SAVE_AUTHOR_ID, ATELIER_CKC_STORY_CARD_BODY_AUTHOR_ID,
    ATELIER_CKC_STORY_CARD_LIST_AUTHOR_ID, ATELIER_CKC_STORY_CARD_SAVE_AUTHOR_ID,
    ATELIER_CKC_STORY_CARD_TITLE_AUTHOR_ID, ATELIER_CKC_STORY_DOC_REF_AUTHOR_ID,
    ATELIER_CKC_STORY_EDITOR_AUTHOR_ID, ATELIER_CKC_STORY_SAVE_AUTHOR_ID,
    ATELIER_CKC_TAG_NOTE_EDITOR_AUTHOR_ID, ATELIER_CKC_TAG_NOTE_SAVE_AUTHOR_ID,
    ATELIER_CKC_TAG_NOTE_SCOPE_AUTHOR_ID, ATELIER_CKC_TAG_NOTE_TAG_AUTHOR_ID,
    ATELIER_CKC_TEMPLATE_LOAD_AUTHOR_ID, ATELIER_CKC_TEMPLATE_STATUS_AUTHOR_ID,
    ATELIER_CKC_TYPED_REF_KIND_AUTHOR_ID, ATELIER_CONTENT_CKC_AUTHOR_ID,
    ATELIER_CONTENT_INGEST_AUTHOR_ID, ATELIER_CONTENT_POSEKIT_AUTHOR_ID,
    ATELIER_INGEST_BATCH_TAGS_AUTHOR_ID, ATELIER_INGEST_PASS_AUTHOR_ID,
    ATELIER_INGEST_REJECT_AUTHOR_ID, ATELIER_INGEST_UNSURE_AUTHOR_ID, ATELIER_PANEL_AUTHOR_ID,
    ATELIER_POSE_BODY_TOGGLE_AUTHOR_ID, ATELIER_POSE_FACE_TOGGLE_AUTHOR_ID,
    ATELIER_POSE_HANDS_TOGGLE_AUTHOR_ID, ATELIER_POSE_PITCH_SLIDER_AUTHOR_ID,
    ATELIER_POSE_RESET_AUTHOR_ID, ATELIER_POSE_YAW_MINUS_AUTHOR_ID,
    ATELIER_POSE_YAW_PLUS_AUTHOR_ID, ATELIER_POSE_YAW_SLIDER_AUTHOR_ID,
    ATELIER_POSE_ZOOM_SLIDER_AUTHOR_ID, ATELIER_TABLIST_AUTHOR_ID, ATELIER_TAB_CKC_AUTHOR_ID,
    ATELIER_TAB_INGEST_AUTHOR_ID, ATELIER_TAB_POSEKIT_AUTHOR_ID,
};
use handshake_native::atelier_side_panel::{item_author_id, AtelierSidePanel, PANEL_AUTHOR_ID};
use handshake_native::backend_client::{AtelierBatchRow, AtelierItemRow};
use handshake_native::graph::canvas_board::{CanvasEvent, LoomCanvasBoard};
use handshake_native::mcp::{
    dispatch_request, ActionChannel, McpRequest, ScreenshotError, SessionToken,
};
use handshake_native::theme::HsTheme;

const MIRA_DEMO_ROW_AUTHOR_ID: &str = "atelier-ckc-character-018f7848-1111-7000-9000-000000000001";
const ARIA_DEMO_ROW_AUTHOR_ID: &str = "atelier-ckc-character-018f7848-1111-7000-9000-000000000002";
const MIRA_DEMO_ALBUM_ID: &str = "018f7848-1111-7000-9000-00000000a001";
const MIRA_DEMO_MEDIA_ID: &str = "018f7848-1111-7000-9000-00000000b001";
const MIRA_DEMO_FOLDER_REF: &str = "atelier://folder/mira-reference-set";
const MIRA_DEMO_SOURCE_URL_REF: &str = "https://example.invalid/reference/mira-reference-set";
const MIRA_DEMO_SECOND_ALBUM_ID: &str = "018f7848-1111-7000-9000-00000000a003";
const MIRA_DEMO_SECOND_MEDIA_ID: &str = "018f7848-1111-7000-9000-00000000b003";
const MIRA_DEMO_SECOND_FOLDER_REF: &str = "atelier://folder/mira-expression-set";
const MIRA_DEMO_SECOND_SOURCE_URL_REF: &str =
    "https://example.invalid/reference/mira-expression-set";
const MIRA_DEMO_SECOND_STORY_DOC_ID: &str = "018f7848-1111-7000-9000-00000000c002";
const MIRA_DEMO_SECOND_MOODBOARD_DOC_ID: &str = "018f7848-1111-7000-9000-00000000d003";
const MIRA_DEMO_SECOND_MOODBOARD_SNAPSHOT_ID: &str = "018f7848-1111-7000-9000-00000000d103";
const PROBE_ACTIONS: &[accesskit::Action] = &[
    accesskit::Action::Click,
    accesskit::Action::Focus,
    accesskit::Action::SetValue,
    accesskit::Action::ReplaceSelectedText,
    accesskit::Action::ScrollIntoView,
];

fn seeded_side_panel() -> Arc<Mutex<AtelierSidePanel>> {
    Arc::new(Mutex::new(AtelierSidePanel::with_rows(
        vec![AtelierBatchRow {
            batch_id: "batch-1".to_owned(),
            source_label: "Sourcing Run A".to_owned(),
            status: "open".to_owned(),
        }],
        vec![],
        Some((
            "batch-1".to_owned(),
            vec![AtelierItemRow {
                item_id: "item-aaa".to_owned(),
                file_name: "sunset.png".to_owned(),
                source_path: "/intake/sunset.png".to_owned(),
                lane: "accept".to_owned(),
            }],
        )),
    )))
}

fn author_ids(harness: &Harness<'_, AtelierPanel>) -> std::collections::HashSet<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect()
}

fn author_id_counts(
    harness: &Harness<'_, AtelierPanel>,
) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    for node in harness.root().children_recursive() {
        if let Some(author_id) = node.accesskit_node().author_id() {
            *counts.entry(author_id.to_owned()).or_insert(0) += 1;
        }
    }
    counts
}

fn snapshot_author_id_prefix_count(snapshot: &UiTreeSnapshot, prefix: &str) -> usize {
    snapshot
        .root
        .children
        .iter()
        .filter_map(|node| node.author_id.as_deref())
        .filter(|author_id| author_id.starts_with(prefix))
        .count()
}

fn snapshot_harness(harness: &mut Harness<'_, AtelierPanel>) -> UiTreeSnapshot {
    let mut children = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        let author_id = ak.author_id().map(str::to_owned);
        let node_id = ak.id().0;
        let actions = PROBE_ACTIONS
            .iter()
            .filter(|action| ak.data().supports_action(**action))
            .map(|action| format!("{action:?}"))
            .collect();
        children.push(UiTreeNode {
            id: author_id
                .clone()
                .unwrap_or_else(|| format!("node:{node_id}")),
            author_id,
            node_id,
            role: format!("{:?}", ak.role()),
            label: ak.label().map(|value| value.to_string()),
            value: ak.value().map(|value| value.to_string()),
            disabled: ak.is_disabled(),
            actions,
            bounds: None,
            children: Vec::new(),
        });
    }
    let widget_count = children.len() + 1;
    UiTreeSnapshot {
        root: UiTreeNode {
            id: "node:atelier-proof-root".to_owned(),
            author_id: None,
            node_id: 0,
            role: "Window".to_owned(),
            label: None,
            value: None,
            disabled: false,
            actions: Vec::new(),
            bounds: None,
            children,
        },
        captured_at_utc: "0.000000000Z".to_owned(),
        widget_count,
    }
}

static WGPU_RENDER_LOCK: Mutex<()> = Mutex::new(());

fn external_artifact_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("..")
        .join("Handshake_Artifacts")
        .join("handshake-test")
        .join(name)
}

fn capture_atelier_harness(
    harness: &mut Harness<'_, AtelierPanel>,
) -> Result<handshake_native::mcp::ScreenshotResult, ScreenshotError> {
    use image::ImageEncoder;

    let image = harness.render().map_err(ScreenshotError)?;
    let (width, height) = (image.width(), image.height());
    let mut png_bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png_bytes)
        .write_image(
            image.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|err| ScreenshotError(format!("PNG encode failed: {err}")))?;
    Ok(handshake_native::mcp::screenshot::screenshot_from_png(
        &png_bytes, width, height,
    ))
}

fn save_visual_probe_to(harness: &mut Harness<'_, AtelierPanel>, artifact_dir: &str, name: &str) {
    let _guard = WGPU_RENDER_LOCK.lock().expect("wgpu render lock poisoned");
    let snapshot = snapshot_harness(harness);
    let mut channel = ActionChannel::new();
    let response = dispatch_request(
        &argus_req("argus.screenshot", serde_json::json!({})),
        &argus_token(),
        &snapshot,
        &mut channel,
        || capture_atelier_harness(harness),
    );
    let json = response.to_json();
    assert!(
        json.get("error").is_none(),
        "argus.screenshot must succeed for CKC visual proof: {json}"
    );
    assert_eq!(
        json["result"]["argus"]["method"], "argus.screenshot",
        "visual proof must flow through Argus screenshot"
    );
    assert_eq!(
        json["result"]["argus"]["headless"], true,
        "Argus screenshot proof must be headless"
    );
    assert_eq!(
        json["result"]["argus"]["non_intrusive"], true,
        "Argus screenshot proof must be non-intrusive"
    );
    let width = json["result"]["width"]
        .as_u64()
        .expect("argus screenshot width present") as u32;
    let height = json["result"]["height"]
        .as_u64()
        .expect("argus screenshot height present") as u32;
    assert!(width > 0 && height > 0, "Argus screenshot is non-empty");
    let png_base64 = json["result"]["png_base64"]
        .as_str()
        .expect("argus screenshot png_base64 present");
    let bytes = decode_base64(png_base64).expect("argus screenshot base64 decodes");
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "PNG magic bytes present");
    let decoded = image::load_from_memory(&bytes).expect("Argus screenshot bytes are a valid PNG");
    assert_eq!(decoded.width(), width);
    assert_eq!(decoded.height(), height);
    let has_visible_pixels = decoded.to_rgba8().pixels().any(|pixel| pixel.0[3] > 0);
    assert!(
        has_visible_pixels,
        "Argus screenshot must contain visible pixels"
    );
    let out_dir = external_artifact_dir(artifact_dir);
    std::fs::create_dir_all(&out_dir).expect("create external CKC visual proof artifact directory");
    let out_path = out_dir.join(format!("{name}.png"));
    std::fs::write(&out_path, &bytes).expect("save Argus CKC visual proof screenshot");
    let abs = std::fs::canonicalize(&out_path).unwrap_or(out_path);
    println!(
        "{artifact_dir} Argus CKC visual proof: {width}x{height} saved={}",
        abs.display()
    );
}

fn build_panel_harness_with_size(size: egui::Vec2) -> Harness<'static, AtelierPanel> {
    let panel = AtelierPanel::new(
        seeded_side_panel(),
        Arc::new(Mutex::new(LoomCanvasBoard::new("ws-test", "canvas-1"))),
        Arc::new(Mutex::new(Vec::<CanvasEvent>::new())),
    );
    Harness::builder().with_size(size).build_state(
        |ctx, panel: &mut AtelierPanel| {
            egui::CentralPanel::default().show(ctx, |ui| {
                panel.show(ui, &HsTheme::Dark.palette());
            });
        },
        panel,
    )
}

fn build_panel_harness() -> Harness<'static, AtelierPanel> {
    build_panel_harness_with_size(egui::vec2(1280.0, 760.0))
}

fn decode_base64(s: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let clean: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for chunk in clean.chunks(4) {
        if chunk.len() < 2 {
            return Err("truncated base64".to_owned());
        }
        let b0 = val(chunk[0]).ok_or("bad base64 char")?;
        let b1 = val(chunk[1]).ok_or("bad base64 char")?;
        out.push((b0 << 2) | (b1 >> 4));
        if chunk.len() >= 3 && chunk[2] != b'=' {
            let b2 = val(chunk[2]).ok_or("bad base64 char")?;
            out.push((b1 << 4) | (b2 >> 2));
            if chunk.len() == 4 && chunk[3] != b'=' {
                let b3 = val(chunk[3]).ok_or("bad base64 char")?;
                out.push((b2 << 6) | b3);
            }
        }
    }
    Ok(out)
}

fn assert_no_four_window_artifacts(snapshot: &UiTreeSnapshot) {
    const FORBIDDEN: &[&str] = &[
        "pane-a",
        "pane-b",
        "pane-c",
        "pane-d",
        "splitlayoutwidget",
        "four-window",
        "four pane",
        "4-pane",
        "popout",
    ];
    let mut hits = Vec::new();
    for node in snapshot.iter_nodes() {
        for text in [
            node.id.as_str(),
            node.author_id.as_deref().unwrap_or_default(),
            node.label.as_deref().unwrap_or_default(),
            node.value.as_deref().unwrap_or_default(),
        ] {
            let lower = text.to_ascii_lowercase();
            for forbidden in FORBIDDEN {
                if lower.contains(forbidden) {
                    hits.push(format!("{} contains {forbidden}", node.id));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "CKC Atelier view must not expose old four-window/split-layout artifacts: {hits:?}"
    );
}

fn argus_token() -> SessionToken {
    SessionToken::from_hex("mt009-argus-proof-secret")
}

fn argus_req(method: &str, params: serde_json::Value) -> McpRequest {
    McpRequest {
        id: serde_json::json!(1),
        method: method.to_owned(),
        params,
        session_token: "mt009-argus-proof-secret".to_owned(),
        agent_label: Some("mt009-argus-proof".to_owned()),
    }
}

#[test]
fn atelier_main_panel_exposes_ckc_posekit_ingest_tabs() {
    let mut harness = build_panel_harness();
    harness.run();
    harness.run();

    let ids = author_ids(&harness);
    let counts = author_id_counts(&harness);
    for expected in [
        ATELIER_PANEL_AUTHOR_ID,
        ATELIER_TABLIST_AUTHOR_ID,
        ATELIER_TAB_CKC_AUTHOR_ID,
        ATELIER_TAB_POSEKIT_AUTHOR_ID,
        ATELIER_TAB_INGEST_AUTHOR_ID,
        ATELIER_CONTENT_CKC_AUTHOR_ID,
        ATELIER_CKC_BOOK_LAYOUT_AUTHOR_ID,
        ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
        ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
        ATELIER_CKC_MEDIA_VIEWER_AUTHOR_ID,
        ATELIER_CKC_MODE_SHEET_AUTHOR_ID,
        ATELIER_CKC_MODE_STORY_AUTHOR_ID,
        ATELIER_CKC_MODE_NOTES_AUTHOR_ID,
        ATELIER_CKC_MODE_MOODBOARD_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_LIST_AUTHOR_ID,
        ATELIER_CKC_SELECTED_CHARACTER_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_REF_AUTHOR_ID,
        ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID,
        ATELIER_CKC_TYPED_REF_KIND_AUTHOR_ID,
        ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
        ATELIER_CKC_SHEET_SAVE_AUTHOR_ID,
        ATELIER_CKC_TEMPLATE_STATUS_AUTHOR_ID,
        ATELIER_CKC_TEMPLATE_LOAD_AUTHOR_ID,
        ATELIER_CKC_SAFE_SUBSET_LOAD_AUTHOR_ID,
        ATELIER_CKC_IMPORT_EDITOR_AUTHOR_ID,
        ATELIER_CKC_IMPORT_AUTHOR_ID,
        ATELIER_CKC_EXPORT_TXT_AUTHOR_ID,
        ATELIER_CKC_EXPORT_JSON_AUTHOR_ID,
        ATELIER_CKC_EXPORT_SAFE_TXT_AUTHOR_ID,
        ATELIER_CKC_EXPORT_SAFE_JSON_AUTHOR_ID,
        ATELIER_CKC_EXPORT_STATUS_AUTHOR_ID,
        ATELIER_CKC_FIELD_SUGGESTION_FIELD_AUTHOR_ID,
        ATELIER_CKC_FIELD_SUGGESTIONS_LOAD_AUTHOR_ID,
        ATELIER_CKC_FIELD_SUGGESTIONS_LIST_AUTHOR_ID,
        ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID,
        ATELIER_CKC_ALBUM_STATUS_AUTHOR_ID,
        ATELIER_CKC_ALBUM_CREATE_NAME_AUTHOR_ID,
        ATELIER_CKC_ALBUM_CREATE_NOTES_AUTHOR_ID,
        ATELIER_CKC_ALBUM_CREATE_TAGS_AUTHOR_ID,
        ATELIER_CKC_ALBUM_CREATE_AUTHOR_ID,
        ATELIER_CKC_ALBUM_LINK_ASSET_IDS_AUTHOR_ID,
        ATELIER_CKC_ALBUM_LINK_SOURCE_PATH_AUTHOR_ID,
        ATELIER_CKC_ALBUM_LINK_SOURCE_URL_AUTHOR_ID,
        ATELIER_CKC_ALBUM_LINK_AUTHOR_ID,
        ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID,
        ATELIER_CKC_SEARCH_QUERY_AUTHOR_ID,
        ATELIER_CKC_SEARCH_TAGS_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_CHARACTER_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_COLLECTION_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_MEDIA_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_SIMILARITY_AUTHOR_ID,
        ATELIER_CKC_SEARCH_MODE_FUZZY_AUTHOR_ID,
        ATELIER_CKC_SEARCH_MODE_VECTOR_AUTHOR_ID,
        ATELIER_CKC_SEARCH_MODE_COMBINED_AUTHOR_ID,
        ATELIER_CKC_SEARCH_RUN_AUTHOR_ID,
        ATELIER_CKC_SEARCH_STATUS_AUTHOR_ID,
        ATELIER_CKC_SEARCH_RESULTS_AUTHOR_ID,
        ATELIER_CKC_TAG_NOTE_TAG_AUTHOR_ID,
        ATELIER_CKC_TAG_NOTE_SCOPE_AUTHOR_ID,
        ATELIER_CKC_TAG_NOTE_EDITOR_AUTHOR_ID,
        ATELIER_CKC_TAG_NOTE_SAVE_AUTHOR_ID,
        PANEL_AUTHOR_ID,
        &item_author_id("item-aaa"),
    ] {
        assert!(
            ids.contains(expected),
            "expected stable author_id {expected}; got {ids:?}"
        );
    }
    for absent in [
        ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID,
        ATELIER_CKC_STORY_DOC_REF_AUTHOR_ID,
        ATELIER_CKC_STORY_EDITOR_AUTHOR_ID,
        ATELIER_CKC_STORY_SAVE_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_DOC_REF_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_LATEST_REF_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_SAVE_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_OPEN_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID,
    ] {
        assert_eq!(
            counts.get(absent).copied().unwrap_or_default(),
            0,
            "default CKC sheet mode must not expose middle-panel author_id {absent}"
        );
    }
}

#[test]
fn ckc_character_sheet_surface_is_model_addressable() {
    let mut harness = build_panel_harness();
    harness.run();
    harness.run();

    let ids = author_ids(&harness);
    for expected in [
        ATELIER_CKC_BOOK_LAYOUT_AUTHOR_ID,
        ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
        ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
        ATELIER_CKC_MEDIA_VIEWER_AUTHOR_ID,
        ATELIER_CKC_MODE_SHEET_AUTHOR_ID,
        ATELIER_CKC_MODE_STORY_AUTHOR_ID,
        ATELIER_CKC_MODE_NOTES_AUTHOR_ID,
        ATELIER_CKC_MODE_MOODBOARD_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_LIST_AUTHOR_ID,
        MIRA_DEMO_ROW_AUTHOR_ID,
        ARIA_DEMO_ROW_AUTHOR_ID,
        ATELIER_CKC_SELECTED_CHARACTER_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_CREATE_NAME_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_CREATE_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_REF_AUTHOR_ID,
        ATELIER_CKC_TYPED_REF_KIND_AUTHOR_ID,
        ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID,
        ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
        ATELIER_CKC_SHEET_SAVE_AUTHOR_ID,
        ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID,
        ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID,
        ATELIER_CKC_SEARCH_QUERY_AUTHOR_ID,
        ATELIER_CKC_SEARCH_TAGS_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_CHARACTER_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_COLLECTION_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_MEDIA_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_SIMILARITY_AUTHOR_ID,
        ATELIER_CKC_SEARCH_RUN_AUTHOR_ID,
        ATELIER_CKC_SEARCH_RESULTS_AUTHOR_ID,
        ATELIER_CKC_TAG_NOTE_EDITOR_AUTHOR_ID,
        ATELIER_CKC_TAG_NOTE_SAVE_AUTHOR_ID,
    ] {
        assert!(
            ids.contains(expected),
            "CKC character-sheet surface must expose stable author_id {expected}; got {ids:?}"
        );
    }
    for expected in [
        ckc_media_album_row_author_id(MIRA_DEMO_ALBUM_ID),
        ckc_media_row_author_id(MIRA_DEMO_ALBUM_ID, MIRA_DEMO_MEDIA_ID),
        ckc_folder_row_author_id(MIRA_DEMO_ALBUM_ID, MIRA_DEMO_MEDIA_ID, MIRA_DEMO_FOLDER_REF),
        ckc_source_url_row_author_id(
            MIRA_DEMO_ALBUM_ID,
            MIRA_DEMO_MEDIA_ID,
            MIRA_DEMO_SOURCE_URL_REF,
        ),
        ckc_media_album_row_author_id(MIRA_DEMO_SECOND_ALBUM_ID),
        ckc_media_row_author_id(MIRA_DEMO_SECOND_ALBUM_ID, MIRA_DEMO_SECOND_MEDIA_ID),
        ckc_folder_row_author_id(
            MIRA_DEMO_SECOND_ALBUM_ID,
            MIRA_DEMO_SECOND_MEDIA_ID,
            MIRA_DEMO_SECOND_FOLDER_REF,
        ),
        ckc_source_url_row_author_id(
            MIRA_DEMO_SECOND_ALBUM_ID,
            MIRA_DEMO_SECOND_MEDIA_ID,
            MIRA_DEMO_SECOND_SOURCE_URL_REF,
        ),
    ] {
        assert!(
            ids.contains(&expected),
            "CKC linked-media surface must expose stable author_id {expected}; got {ids:?}"
        );
    }
    for absent in [
        ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID,
        ATELIER_CKC_STORY_DOC_REF_AUTHOR_ID,
        ATELIER_CKC_STORY_EDITOR_AUTHOR_ID,
        ATELIER_CKC_STORY_SAVE_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_DOC_REF_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_LATEST_REF_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_SAVE_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_OPEN_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_NOTES_EDITOR_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_NOTES_APPLY_AUTHOR_ID,
    ] {
        assert!(
            !ids.contains(absent),
            "default CKC sheet mode must not expose middle-panel author_id {absent}"
        );
    }

    for expected in [
        ATELIER_CKC_CHARACTER_CREATE_NAME_AUTHOR_ID,
        ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
        ATELIER_CKC_IMPORT_EDITOR_AUTHOR_ID,
        ATELIER_CKC_FIELD_SUGGESTION_FIELD_AUTHOR_ID,
        ATELIER_CKC_ALBUM_CREATE_NAME_AUTHOR_ID,
        ATELIER_CKC_ALBUM_CREATE_NOTES_AUTHOR_ID,
        ATELIER_CKC_ALBUM_CREATE_TAGS_AUTHOR_ID,
        ATELIER_CKC_ALBUM_LINK_ASSET_IDS_AUTHOR_ID,
        ATELIER_CKC_ALBUM_LINK_SOURCE_PATH_AUTHOR_ID,
        ATELIER_CKC_ALBUM_LINK_SOURCE_URL_AUTHOR_ID,
        ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
        ATELIER_CKC_SEARCH_QUERY_AUTHOR_ID,
        ATELIER_CKC_SEARCH_TAGS_AUTHOR_ID,
        ATELIER_CKC_TAG_NOTE_TAG_AUTHOR_ID,
        ATELIER_CKC_TAG_NOTE_SCOPE_AUTHOR_ID,
        ATELIER_CKC_TAG_NOTE_EDITOR_AUTHOR_ID,
    ] {
        let node = harness.get_by(|node| node.author_id() == Some(expected));
        assert!(
            node.accesskit_node()
                .data()
                .supports_action(egui::accesskit::Action::Focus),
            "CKC text control {expected} must be steerable by Argus set_value"
        );
    }
    let save = harness.get_by(|node| node.author_id() == Some(ATELIER_CKC_SHEET_SAVE_AUTHOR_ID));
    assert!(
        save.accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::Click),
        "CKC append-version control must be steerable by Argus click"
    );
    for expected in [
        ATELIER_CKC_TEMPLATE_LOAD_AUTHOR_ID,
        ATELIER_CKC_SAFE_SUBSET_LOAD_AUTHOR_ID,
        ATELIER_CKC_IMPORT_AUTHOR_ID,
        ATELIER_CKC_EXPORT_TXT_AUTHOR_ID,
        ATELIER_CKC_EXPORT_JSON_AUTHOR_ID,
        ATELIER_CKC_EXPORT_SAFE_TXT_AUTHOR_ID,
        ATELIER_CKC_EXPORT_SAFE_JSON_AUTHOR_ID,
        ATELIER_CKC_FIELD_SUGGESTIONS_LOAD_AUTHOR_ID,
        ATELIER_CKC_ALBUM_CREATE_AUTHOR_ID,
        ATELIER_CKC_ALBUM_LINK_AUTHOR_ID,
    ] {
        let node = harness.get_by(|node| node.author_id() == Some(expected));
        assert!(
            node.accesskit_node()
                .data()
                .supports_action(egui::accesskit::Action::Click),
            "CKC template/import/export/suggestion control {expected} must be steerable by Argus click"
        );
    }
    let media_save =
        harness.get_by(|node| node.author_id() == Some(ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID));
    assert!(
        media_save
            .accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::Click),
        "CKC media-notes save control must be steerable by Argus click"
    );
    let search_run =
        harness.get_by(|node| node.author_id() == Some(ATELIER_CKC_SEARCH_RUN_AUTHOR_ID));
    assert!(
        search_run
            .accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::Click),
        "CKC search run control must be steerable by Argus click"
    );
    let tag_note_save =
        harness.get_by(|node| node.author_id() == Some(ATELIER_CKC_TAG_NOTE_SAVE_AUTHOR_ID));
    assert!(
        tag_note_save
            .accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::Click),
        "CKC tag-note save control must be steerable by Argus click"
    );
}

#[test]
fn ckc_book_layout_modes_gate_middle_panel_surfaces() {
    let mut harness = build_panel_harness();
    harness.run();
    harness.run();

    let default_snapshot = snapshot_harness(&mut harness);
    assert_no_four_window_artifacts(&default_snapshot);
    for expected in [
        ATELIER_CKC_BOOK_LAYOUT_AUTHOR_ID,
        ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
        ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
        ATELIER_CKC_MEDIA_VIEWER_AUTHOR_ID,
        ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
    ] {
        assert!(
            default_snapshot.find_by_author_id(expected).is_some(),
            "default CKC book layout must expose {expected}"
        );
    }
    for absent in [
        ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID,
        ATELIER_CKC_STORY_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_NOTES_EDITOR_AUTHOR_ID,
    ] {
        assert!(
            default_snapshot.find_by_author_id(absent).is_none(),
            "default CKC sheet mode must not expose {absent}"
        );
    }
    save_visual_probe_to(
        &mut harness,
        "wp-ckc-posekit-overhaul-mt-013",
        "ckc_book_default_desktop",
    );

    let mut channel = ActionChannel::new();
    let story_click = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({ "target": ATELIER_CKC_MODE_STORY_AUTHOR_ID }),
        ),
        &argus_token(),
        &default_snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(story_click.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let story_snapshot = snapshot_harness(&mut harness);
    assert_no_four_window_artifacts(&story_snapshot);
    for expected in [
        ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
        ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID,
        ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
        ATELIER_CKC_STORY_DOC_REF_AUTHOR_ID,
        ATELIER_CKC_STORY_EDITOR_AUTHOR_ID,
        ATELIER_CKC_STORY_SAVE_AUTHOR_ID,
        ATELIER_CKC_STORY_CARD_LIST_AUTHOR_ID,
        ATELIER_CKC_STORY_CARD_TITLE_AUTHOR_ID,
        ATELIER_CKC_STORY_CARD_BODY_AUTHOR_ID,
        ATELIER_CKC_STORY_CARD_SAVE_AUTHOR_ID,
        ATELIER_CKC_STORY_BEAT_EDITOR_AUTHOR_ID,
        ATELIER_CKC_STORY_BEAT_SAVE_AUTHOR_ID,
    ] {
        assert!(
            story_snapshot.find_by_author_id(expected).is_some(),
            "story mode must expose {expected}"
        );
    }
    for absent in [
        ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_NOTES_EDITOR_AUTHOR_ID,
    ] {
        assert!(
            story_snapshot.find_by_author_id(absent).is_none(),
            "story mode must not expose unrelated middle-panel control {absent}"
        );
    }

    let notes_click = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({ "target": ATELIER_CKC_MODE_NOTES_AUTHOR_ID }),
        ),
        &argus_token(),
        &story_snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(notes_click.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let notes_snapshot = snapshot_harness(&mut harness);
    assert_no_four_window_artifacts(&notes_snapshot);
    for expected in [
        ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
        ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID,
        ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_NOTES_EDITOR_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_NOTES_APPLY_AUTHOR_ID,
    ] {
        assert!(
            notes_snapshot.find_by_author_id(expected).is_some(),
            "notes mode must expose {expected}"
        );
    }
    for absent in [
        ATELIER_CKC_STORY_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID,
    ] {
        assert!(
            notes_snapshot.find_by_author_id(absent).is_none(),
            "notes mode must not expose unrelated middle-panel control {absent}"
        );
    }
    let media_note_before = notes_snapshot
        .find_by_author_id(ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("media notes visible before sheet-note edit");
    let set_notes = dispatch_request(
        &argus_req(
            "argus.set_value",
            serde_json::json!({
                "target": ATELIER_CKC_CHARACTER_NOTES_EDITOR_AUTHOR_ID,
                "value": "MT-013 character sheet note"
            }),
        ),
        &argus_token(),
        &notes_snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(set_notes.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let notes_after_set = snapshot_harness(&mut harness);
    let apply_notes = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({ "target": ATELIER_CKC_CHARACTER_NOTES_APPLY_AUTHOR_ID }),
        ),
        &argus_token(),
        &notes_after_set,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(apply_notes.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let notes_after_apply = snapshot_harness(&mut harness);
    let sheet_after_notes = notes_after_apply
        .find_by_author_id(ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("sheet visible after applying sheet notes");
    assert!(
        sheet_after_notes.contains("MT-013 character sheet note"),
        "sheet-note mode must write into the selected sheet editor"
    );
    let media_note_after = notes_after_apply
        .find_by_author_id(ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("media notes visible after sheet-note edit");
    assert_eq!(
        media_note_before, media_note_after,
        "character sheet notes must not mutate selected image notes"
    );

    let moodboard_click = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({ "target": ATELIER_CKC_MODE_MOODBOARD_AUTHOR_ID }),
        ),
        &argus_token(),
        &notes_after_apply,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(moodboard_click.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let moodboard_snapshot = snapshot_harness(&mut harness);
    assert_no_four_window_artifacts(&moodboard_snapshot);
    for expected in [
        ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
        ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID,
        ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_DOC_REF_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_LATEST_REF_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_SAVE_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_OPEN_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID,
    ] {
        assert!(
            moodboard_snapshot.find_by_author_id(expected).is_some(),
            "moodboard mode must expose {expected}"
        );
    }
    for absent in [
        ATELIER_CKC_STORY_EDITOR_AUTHOR_ID,
        ATELIER_CKC_CHARACTER_NOTES_EDITOR_AUTHOR_ID,
    ] {
        assert!(
            moodboard_snapshot.find_by_author_id(absent).is_none(),
            "moodboard mode must not expose unrelated middle-panel control {absent}"
        );
    }
    save_visual_probe_to(
        &mut harness,
        "wp-ckc-posekit-overhaul-mt-013",
        "ckc_book_moodboard_desktop",
    );

    let mut constrained = build_panel_harness_with_size(egui::vec2(920.0, 640.0));
    constrained.run();
    constrained.run();
    let constrained_snapshot = snapshot_harness(&mut constrained);
    assert_no_four_window_artifacts(&constrained_snapshot);
    for expected in [
        ATELIER_CKC_BOOK_LAYOUT_AUTHOR_ID,
        ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
        ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
        ATELIER_CKC_MEDIA_VIEWER_AUTHOR_ID,
        ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
    ] {
        assert!(
            constrained_snapshot.find_by_author_id(expected).is_some(),
            "constrained CKC book layout must expose {expected}"
        );
    }
    save_visual_probe_to(
        &mut constrained,
        "wp-ckc-posekit-overhaul-mt-013",
        "ckc_book_default_constrained",
    );
}

#[test]
fn argus_inspects_and_steers_ckc_character_sheet_surface() {
    let mut harness = build_panel_harness();
    harness.run();
    harness.run();
    let snapshot = snapshot_harness(&mut harness);
    for expected in [
        ATELIER_CKC_CHARACTER_LIST_AUTHOR_ID,
        ATELIER_CKC_SELECTED_CHARACTER_AUTHOR_ID,
        ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
        ATELIER_CKC_SHEET_SAVE_AUTHOR_ID,
        ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID,
        ATELIER_CKC_TEMPLATE_STATUS_AUTHOR_ID,
        ATELIER_CKC_IMPORT_EDITOR_AUTHOR_ID,
        ATELIER_CKC_EXPORT_STATUS_AUTHOR_ID,
        ATELIER_CKC_FIELD_SUGGESTIONS_LIST_AUTHOR_ID,
    ] {
        assert!(
            snapshot.find_by_author_id(expected).is_some(),
            "Argus inspect snapshot must include CKC sheet author_id {expected}"
        );
    }
    for expected in [
        ATELIER_CKC_SEARCH_FILTER_CHARACTER_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_COLLECTION_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_MEDIA_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_SIMILARITY_AUTHOR_ID,
    ] {
        let node = snapshot
            .find_by_author_id(expected)
            .expect("CKC search filter node exists");
        assert!(
            node.actions.iter().any(|action| action == "Click"),
            "Argus must be able to click CKC search filter {expected}; actions={:?}",
            node.actions
        );
    }

    let mut channel = ActionChannel::new();
    let inspect = dispatch_request(
        &argus_req("argus.inspect", serde_json::json!({})),
        &argus_token(),
        &snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    let inspect_json = inspect.to_json();
    assert_eq!(inspect_json["result"]["argus"]["method"], "argus.inspect");
    assert_eq!(inspect_json["result"]["argus"]["headless"], true);
    assert_eq!(inspect_json["result"]["argus"]["non_intrusive"], true);

    let load_suggestions = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({ "target": ATELIER_CKC_FIELD_SUGGESTIONS_LOAD_AUTHOR_ID }),
        ),
        &argus_token(),
        &snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(load_suggestions.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let suggestions_snapshot = snapshot_harness(&mut harness);
    let suggestion_row =
        ckc_field_suggestion_row_author_id("CHAR-ID-006", "reusable character/avatar");
    assert!(
        suggestions_snapshot
            .find_by_author_id(&suggestion_row)
            .is_some(),
        "Argus click must load stable CKC field suggestion row {suggestion_row}"
    );

    let set_value = dispatch_request(
        &argus_req(
            "argus.set_value",
            serde_json::json!({
                "target": ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
                "value": "CHAR-ID-001 — Character_ID: mira-demo\nCHAR-ID-002 — Name: Argus Proof\nCHAR-ID-006 — Primary_Role: CKC sheet edit"
            }),
        ),
        &argus_token(),
        &snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    let set_value_json = set_value.to_json();
    assert_eq!(set_value_json["result"]["queued"], true);
    assert_eq!(
        set_value_json["result"]["target"],
        ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID
    );

    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let after_set = snapshot_harness(&mut harness);
    let editor = after_set
        .find_by_author_id(ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID)
        .expect("editor visible after Argus set_value");
    assert!(
        editor
            .value
            .as_deref()
            .unwrap_or_default()
            .contains("Argus Proof"),
        "Argus set_value must change the live CKC sheet editor value; got {editor:?}"
    );

    let before_ref = snapshot_harness(&mut harness)
        .find_by_author_id(ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID)
        .and_then(|node| node.label.clone())
        .expect("sheet ref label before save");
    let click_snapshot = snapshot_harness(&mut harness);
    let click = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({ "target": ATELIER_CKC_SHEET_SAVE_AUTHOR_ID }),
        ),
        &argus_token(),
        &click_snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    let click_json = click.to_json();
    assert_eq!(click_json["result"]["queued"], true);
    assert_eq!(
        click_json["result"]["target"],
        ATELIER_CKC_SHEET_SAVE_AUTHOR_ID
    );
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let after_ref = snapshot_harness(&mut harness)
        .find_by_author_id(ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID)
        .and_then(|node| node.label.clone())
        .expect("sheet ref label after save");
    assert_ne!(
        before_ref, after_ref,
        "Argus click must rerender a new visible CKC sheet_version_ref"
    );

    let story_mode_click = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({ "target": ATELIER_CKC_MODE_STORY_AUTHOR_ID }),
        ),
        &argus_token(),
        &snapshot_harness(&mut harness),
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(story_mode_click.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let story_mode_snapshot = snapshot_harness(&mut harness);
    for expected in [
        ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
        ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID,
        ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
        ATELIER_CKC_STORY_EDITOR_AUTHOR_ID,
    ] {
        assert!(
            story_mode_snapshot.find_by_author_id(expected).is_some(),
            "Argus must see {expected} after switching to Story mode"
        );
    }
    let story_doc_click = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({
                "target": ckc_story_document_row_author_id(MIRA_DEMO_SECOND_STORY_DOC_ID)
            }),
        ),
        &argus_token(),
        &story_mode_snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(story_doc_click.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let selected_story = snapshot_harness(&mut harness);
    let active_story_ref = selected_story
        .find_by_author_id(ATELIER_CKC_STORY_DOC_REF_AUTHOR_ID)
        .and_then(|node| node.label.clone())
        .expect("active story ref visible after second story row click");
    assert!(
        active_story_ref.contains(MIRA_DEMO_SECOND_STORY_DOC_ID),
        "Argus document-row click must switch the active story document; got {active_story_ref}"
    );

    let story_set = dispatch_request(
        &argus_req(
            "argus.set_value",
            serde_json::json!({
                "target": ATELIER_CKC_STORY_EDITOR_AUTHOR_ID,
                "value": "Argus-steered story continuity note"
            }),
        ),
        &argus_token(),
        &selected_story,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(story_set.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let story_after_set = snapshot_harness(&mut harness);
    let story_editor = story_after_set
        .find_by_author_id(ATELIER_CKC_STORY_EDITOR_AUTHOR_ID)
        .expect("story editor visible after Argus set_value");
    assert!(
        story_editor
            .value
            .as_deref()
            .unwrap_or_default()
            .contains("Argus-steered story continuity note"),
        "Argus set_value must change the live CKC story editor; got {story_editor:?}"
    );

    let card_count_before =
        snapshot_author_id_prefix_count(&story_after_set, "atelier-ckc-story-card-");
    for (target, value) in [
        (ATELIER_CKC_STORY_CARD_TITLE_AUTHOR_ID, "Argus story card"),
        (
            ATELIER_CKC_STORY_CARD_BODY_AUTHOR_ID,
            "Argus-created reusable story card body",
        ),
        (
            ATELIER_CKC_STORY_BEAT_EDITOR_AUTHOR_ID,
            "Argus-created reusable beat",
        ),
    ] {
        let set = dispatch_request(
            &argus_req(
                "argus.set_value",
                serde_json::json!({ "target": target, "value": value }),
            ),
            &argus_token(),
            &snapshot_harness(&mut harness),
            &mut channel,
            || Err(ScreenshotError("not used".to_owned())),
        );
        assert_eq!(set.to_json()["result"]["queued"], true);
        for event in channel.drain_into_events() {
            harness.event(event);
        }
        harness.run();
        harness.run();
    }
    for target in [
        ATELIER_CKC_STORY_SAVE_AUTHOR_ID,
        ATELIER_CKC_STORY_CARD_SAVE_AUTHOR_ID,
        ATELIER_CKC_STORY_BEAT_SAVE_AUTHOR_ID,
    ] {
        let click = dispatch_request(
            &argus_req("argus.click", serde_json::json!({ "target": target })),
            &argus_token(),
            &snapshot_harness(&mut harness),
            &mut channel,
            || Err(ScreenshotError("not used".to_owned())),
        );
        assert_eq!(click.to_json()["result"]["queued"], true);
        for event in channel.drain_into_events() {
            harness.event(event);
        }
        harness.run();
        harness.run();
    }
    let story_after_clicks = snapshot_harness(&mut harness);
    let card_count_after =
        snapshot_author_id_prefix_count(&story_after_clicks, "atelier-ckc-story-card-");
    assert!(
        card_count_after > card_count_before,
        "Argus story-card save click must add an inspectable CKC story-card row"
    );
    save_visual_probe_to(
        &mut harness,
        "wp-ckc-posekit-overhaul-mt-013",
        "ckc_story_middle_panel_desktop",
    );

    let moodboard_mode_click = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({ "target": ATELIER_CKC_MODE_MOODBOARD_AUTHOR_ID }),
        ),
        &argus_token(),
        &story_after_clicks,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(moodboard_mode_click.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let moodboard_mode_snapshot = snapshot_harness(&mut harness);
    for expected in [
        ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
        ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID,
        ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID,
    ] {
        assert!(
            moodboard_mode_snapshot
                .find_by_author_id(expected)
                .is_some(),
            "Argus must see {expected} after switching to Moodboard mode"
        );
    }
    let moodboard_doc_click = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({
                "target": ckc_moodboard_document_row_author_id(MIRA_DEMO_SECOND_MOODBOARD_DOC_ID)
            }),
        ),
        &argus_token(),
        &moodboard_mode_snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(moodboard_doc_click.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let selected_moodboard = snapshot_harness(&mut harness);
    let active_moodboard_ref = selected_moodboard
        .find_by_author_id(ATELIER_CKC_MOODBOARD_DOC_REF_AUTHOR_ID)
        .and_then(|node| node.label.clone())
        .expect("active moodboard ref visible after second moodboard row click");
    assert!(
        active_moodboard_ref.contains(MIRA_DEMO_SECOND_MOODBOARD_DOC_ID),
        "Argus document-row click must switch the active moodboard document; got {active_moodboard_ref}"
    );
    let active_latest_ref = selected_moodboard
        .find_by_author_id(ATELIER_CKC_MOODBOARD_LATEST_REF_AUTHOR_ID)
        .and_then(|node| node.label.clone())
        .expect("active latest moodboard ref visible after second moodboard row click");
    assert!(
        active_latest_ref.contains(MIRA_DEMO_SECOND_MOODBOARD_SNAPSHOT_ID),
        "active latest moodboard ref must follow selected moodboard document; got {active_latest_ref}"
    );

    let moodboard_payload = serde_json::json!({
        "schema_id": "hsk.atelier.moodboard@1",
        "schema_version": 1,
        "moodboard_id": MIRA_DEMO_SECOND_MOODBOARD_DOC_ID,
        "name": "Argus moodboard proof",
        "description": "Argus moodboard continuity proof",
        "canvas": {
            "width": 1600.0,
            "height": 1000.0,
            "background_color": "#101418"
        },
        "layers": [{
            "layer_id": "argus-layer-1",
            "name": "Argus proof layer",
            "order": 1,
            "visible": true,
            "locked": false,
            "opacity": 1.0,
            "parent_layer_id": null
        }],
        "images": [],
        "text": [{
            "element_id": "argus-note-1",
            "layer_id": "argus-layer-1",
            "content": "Argus moodboard continuity note",
            "font": "Inter",
            "font_size": 18.0,
            "color": "#f4f7fb",
            "position": { "x": 120.0, "y": 140.0 },
            "rotation": 0.0,
            "flags": {}
        }],
        "shapes": [],
        "connectors": [],
        "folders": [],
        "guides": [],
        "flags": {
            "locked": false,
            "archived": false,
            "operator_reviewed": false
        },
        "style": {
            "dominant_colors": ["#101418", "#f4f7fb"],
            "mood_keywords": ["argus", "continuity"],
            "style_description": "Argus native moodboard proof",
            "suggested_presets": []
        },
        "history": [{
            "history_id": "argus-history-1",
            "at": "2026-06-29T00:00:00Z",
            "actor": "argus-test",
            "operation": "updated",
            "summary": "Argus moodboard save proof"
        }]
    })
    .to_string();
    let moodboard_set = dispatch_request(
        &argus_req(
            "argus.set_value",
            serde_json::json!({
                "target": ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID,
                "value": moodboard_payload
            }),
        ),
        &argus_token(),
        &selected_moodboard,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(moodboard_set.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let moodboard_after_set = snapshot_harness(&mut harness);
    let moodboard_editor = moodboard_after_set
        .find_by_author_id(ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID)
        .expect("moodboard editor visible after Argus set_value");
    assert_eq!(
        moodboard_editor.value.as_deref(),
        Some(moodboard_payload.as_str()),
        "Argus set_value must replace the live CKC moodboard editor value; got {moodboard_editor:?}"
    );

    for target in [
        ATELIER_CKC_MOODBOARD_SAVE_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_OPEN_AUTHOR_ID,
    ] {
        let click = dispatch_request(
            &argus_req("argus.click", serde_json::json!({ "target": target })),
            &argus_token(),
            &snapshot_harness(&mut harness),
            &mut channel,
            || Err(ScreenshotError("not used".to_owned())),
        );
        assert_eq!(click.to_json()["result"]["queued"], true);
        for event in channel.drain_into_events() {
            harness.event(event);
        }
        harness.run();
        harness.run();
    }
    let moodboard_after_clicks = snapshot_harness(&mut harness);
    for expected in [
        ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_SAVE_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_OPEN_AUTHOR_ID,
        ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID,
    ] {
        assert!(
            moodboard_after_clicks.find_by_author_id(expected).is_some(),
            "Argus inspect snapshot must include the CKC moodboard surface {expected}"
        );
    }
    save_visual_probe_to(
        &mut harness,
        "wp-ckc-posekit-overhaul-mt-013",
        "ckc_moodboard_middle_panel_desktop",
    );
    let moodboard_canvas_cards = snapshot_author_id_prefix_count(
        &moodboard_after_clicks,
        "canvas.placement.moodboard-text-",
    );
    assert!(
        moodboard_canvas_cards > 0,
        "Argus moodboard save/open must load the selected CKC moodboard snapshot into the native canvas"
    );
    let moodboard_canvas_label = moodboard_after_clicks
        .find_by_author_id("canvas.placement.moodboard-text-argus-note-1")
        .and_then(|node| node.label.clone());
    assert!(
        moodboard_canvas_label.is_some(),
        "Argus moodboard save must project the edited snapshot element into the native canvas"
    );

    let export_snapshot = snapshot_harness(&mut harness);
    let export_click = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({ "target": ATELIER_CKC_EXPORT_TXT_AUTHOR_ID }),
        ),
        &argus_token(),
        &export_snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(export_click.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let exported_snapshot = snapshot_harness(&mut harness);
    let export_ref = exported_snapshot
        .find_by_author_id(ATELIER_CKC_EXPORT_REF_AUTHOR_ID)
        .expect("Argus export click must reveal the CKC export ref/hash");
    assert!(
        export_ref
            .label
            .as_deref()
            .unwrap_or_default()
            .contains("atelier://sheet/"),
        "CKC export ref must include the typed sheet ref; got {export_ref:?}"
    );
    let preview = exported_snapshot
        .find_by_author_id(ATELIER_CKC_EXPORT_PREVIEW_AUTHOR_ID)
        .expect("Argus export click must reveal CKC export preview");
    assert!(
        preview
            .value
            .as_deref()
            .unwrap_or_default()
            .contains("Argus Proof"),
        "CKC export preview must carry the exported sheet content; got {preview:?}"
    );
}

#[test]
fn argus_inspects_and_steers_ckc_linked_media_without_touching_sheet_notes() {
    let mut harness = build_panel_harness();
    harness.run();
    harness.run();
    let snapshot = snapshot_harness(&mut harness);
    for expected in [
        ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID,
        ATELIER_CKC_ALBUM_STATUS_AUTHOR_ID,
        ATELIER_CKC_ALBUM_CREATE_NAME_AUTHOR_ID,
        ATELIER_CKC_ALBUM_CREATE_NOTES_AUTHOR_ID,
        ATELIER_CKC_ALBUM_CREATE_TAGS_AUTHOR_ID,
        ATELIER_CKC_ALBUM_CREATE_AUTHOR_ID,
        ATELIER_CKC_ALBUM_LINK_ASSET_IDS_AUTHOR_ID,
        ATELIER_CKC_ALBUM_LINK_SOURCE_PATH_AUTHOR_ID,
        ATELIER_CKC_ALBUM_LINK_SOURCE_URL_AUTHOR_ID,
        ATELIER_CKC_ALBUM_LINK_AUTHOR_ID,
        ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
        ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID,
    ] {
        assert!(
            snapshot.find_by_author_id(expected).is_some(),
            "Argus inspect snapshot must include CKC linked-media author_id {expected}"
        );
    }
    for expected in [
        ckc_media_album_row_author_id(MIRA_DEMO_ALBUM_ID),
        ckc_media_row_author_id(MIRA_DEMO_ALBUM_ID, MIRA_DEMO_MEDIA_ID),
        ckc_folder_row_author_id(MIRA_DEMO_ALBUM_ID, MIRA_DEMO_MEDIA_ID, MIRA_DEMO_FOLDER_REF),
        ckc_source_url_row_author_id(
            MIRA_DEMO_ALBUM_ID,
            MIRA_DEMO_MEDIA_ID,
            MIRA_DEMO_SOURCE_URL_REF,
        ),
        ckc_media_album_row_author_id(MIRA_DEMO_SECOND_ALBUM_ID),
        ckc_media_row_author_id(MIRA_DEMO_SECOND_ALBUM_ID, MIRA_DEMO_SECOND_MEDIA_ID),
        ckc_folder_row_author_id(
            MIRA_DEMO_SECOND_ALBUM_ID,
            MIRA_DEMO_SECOND_MEDIA_ID,
            MIRA_DEMO_SECOND_FOLDER_REF,
        ),
        ckc_source_url_row_author_id(
            MIRA_DEMO_SECOND_ALBUM_ID,
            MIRA_DEMO_SECOND_MEDIA_ID,
            MIRA_DEMO_SECOND_SOURCE_URL_REF,
        ),
    ] {
        assert!(
            snapshot.find_by_author_id(&expected).is_some(),
            "Argus inspect snapshot must include CKC linked-media row {expected}"
        );
    }
    for (author_id, expected_description) in [
        (
            ckc_media_album_row_author_id(MIRA_DEMO_ALBUM_ID),
            "draggable; atelier-ref media_album:atelier://collection/",
        ),
        (
            ckc_media_row_author_id(MIRA_DEMO_ALBUM_ID, MIRA_DEMO_MEDIA_ID),
            "draggable; atelier-ref media:atelier://media/",
        ),
        (
            ckc_folder_row_author_id(MIRA_DEMO_ALBUM_ID, MIRA_DEMO_MEDIA_ID, MIRA_DEMO_FOLDER_REF),
            "draggable; atelier-ref folder:atelier://folder/",
        ),
        (
            ckc_source_url_row_author_id(
                MIRA_DEMO_ALBUM_ID,
                MIRA_DEMO_MEDIA_ID,
                MIRA_DEMO_SOURCE_URL_REF,
            ),
            "draggable; atelier-ref source_url:https://example.invalid/reference/",
        ),
    ] {
        let node = harness.get_by(|node| node.author_id() == Some(author_id.as_str()));
        let description = node.accesskit_node().description().unwrap_or_default();
        assert!(
            description.contains(expected_description),
            "CKC linked-media row {author_id} must expose typed draggable ref metadata; got {description:?}"
        );
    }

    let sheet_before = snapshot
        .find_by_author_id(ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("sheet editor value before media note edit");
    let first_media_note_before = snapshot
        .find_by_author_id(ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("first media notes value before selecting second media");

    let mut channel = ActionChannel::new();
    let second_media_author_id =
        ckc_media_row_author_id(MIRA_DEMO_SECOND_ALBUM_ID, MIRA_DEMO_SECOND_MEDIA_ID);
    let select_second = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({ "target": second_media_author_id }),
        ),
        &argus_token(),
        &snapshot,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(select_second.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let second_selected = snapshot_harness(&mut harness);
    let selected_media_note = second_selected
        .find_by_author_id(ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("second media notes value after selecting second media");
    assert_ne!(
        first_media_note_before, selected_media_note,
        "Argus click on a second CKC media row must retarget the shared image-note editor"
    );

    let set_note = dispatch_request(
        &argus_req(
            "argus.set_value",
            serde_json::json!({
                "target": ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
                "value": "approved close-up for album, separate from character sheet notes"
            }),
        ),
        &argus_token(),
        &second_selected,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(set_note.to_json()["result"]["queued"], true);
    let set_tags = dispatch_request(
        &argus_req(
            "argus.set_value",
            serde_json::json!({
                "target": ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
                "value": "face, approved, training"
            }),
        ),
        &argus_token(),
        &second_selected,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(set_tags.to_json()["result"]["queued"], true);

    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let after = snapshot_harness(&mut harness);
    let media_note = after
        .find_by_author_id(ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("media notes value after Argus set_value");
    assert!(
        media_note.contains("approved close-up"),
        "Argus set_value must update the CKC image-note editor; got {media_note}"
    );
    let media_tags = after
        .find_by_author_id(ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("media tags value after Argus set_value");
    assert!(
        media_tags.contains("training"),
        "Argus set_value must update the CKC media tags editor; got {media_tags}"
    );
    let sheet_after = after
        .find_by_author_id(ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .expect("sheet editor value after media note edit");
    assert_eq!(
        sheet_before, sheet_after,
        "image notes/tags must stay separate from character sheet notes"
    );
}

#[test]
fn argus_inspects_and_steers_ckc_search_and_tag_notes() {
    let mut harness = build_panel_harness();
    harness.run();
    harness.run();
    let snapshot = snapshot_harness(&mut harness);
    for expected in [
        ATELIER_CKC_SEARCH_QUERY_AUTHOR_ID,
        ATELIER_CKC_SEARCH_TAGS_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_CHARACTER_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_COLLECTION_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_MEDIA_AUTHOR_ID,
        ATELIER_CKC_SEARCH_FILTER_SIMILARITY_AUTHOR_ID,
        ATELIER_CKC_SEARCH_MODE_FUZZY_AUTHOR_ID,
        ATELIER_CKC_SEARCH_MODE_VECTOR_AUTHOR_ID,
        ATELIER_CKC_SEARCH_MODE_COMBINED_AUTHOR_ID,
        ATELIER_CKC_SEARCH_RUN_AUTHOR_ID,
        ATELIER_CKC_SEARCH_STATUS_AUTHOR_ID,
        ATELIER_CKC_SEARCH_RESULTS_AUTHOR_ID,
        ATELIER_CKC_TAG_NOTE_TAG_AUTHOR_ID,
        ATELIER_CKC_TAG_NOTE_SCOPE_AUTHOR_ID,
        ATELIER_CKC_TAG_NOTE_EDITOR_AUTHOR_ID,
        ATELIER_CKC_TAG_NOTE_SAVE_AUTHOR_ID,
    ] {
        assert!(
            snapshot.find_by_author_id(expected).is_some(),
            "Argus inspect snapshot must include CKC search/tag-note author_id {expected}"
        );
    }

    let mut channel = ActionChannel::new();
    for (target, value) in [
        (ATELIER_CKC_SEARCH_QUERY_AUTHOR_ID, "expression"),
        (ATELIER_CKC_SEARCH_TAGS_AUTHOR_ID, "reference"),
        (
            ATELIER_CKC_TAG_NOTE_EDITOR_AUTHOR_ID,
            "training tag note saved through the CKC search surface",
        ),
    ] {
        let response = dispatch_request(
            &argus_req(
                "argus.set_value",
                serde_json::json!({ "target": target, "value": value }),
            ),
            &argus_token(),
            &snapshot,
            &mut channel,
            || Err(ScreenshotError("not used".to_owned())),
        );
        assert_eq!(
            response.to_json()["result"]["queued"],
            true,
            "Argus set_value queues for {target}"
        );
    }
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();

    let after_inputs = snapshot_harness(&mut harness);
    let query_value = after_inputs
        .find_by_author_id(ATELIER_CKC_SEARCH_QUERY_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .unwrap_or_else(|| "<missing query>".to_owned());
    let tag_value = after_inputs
        .find_by_author_id(ATELIER_CKC_SEARCH_TAGS_AUTHOR_ID)
        .and_then(|node| node.value.clone())
        .unwrap_or_else(|| "<missing tags>".to_owned());
    assert!(
        query_value.contains("expression") && tag_value.contains("reference"),
        "Argus set_value must update CKC search inputs; query={query_value:?}, tags={tag_value:?}"
    );
    for target in [
        ATELIER_CKC_SEARCH_MODE_COMBINED_AUTHOR_ID,
        ATELIER_CKC_SEARCH_RUN_AUTHOR_ID,
    ] {
        let response = dispatch_request(
            &argus_req("argus.click", serde_json::json!({ "target": target })),
            &argus_token(),
            &after_inputs,
            &mut channel,
            || Err(ScreenshotError("not used".to_owned())),
        );
        assert_eq!(
            response.to_json()["result"]["queued"],
            true,
            "Argus click queues for {target}"
        );
    }
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();

    let after_search = snapshot_harness(&mut harness);
    let second_album_ref = format!("atelier://collection/{MIRA_DEMO_SECOND_ALBUM_ID}");
    let second_album_result = ckc_search_result_row_author_id(&second_album_ref);
    let search_debug_ids: Vec<_> = after_search
        .root
        .children
        .iter()
        .filter_map(|node| node.author_id.as_deref())
        .filter(|id| id.starts_with("atelier-ckc-search"))
        .collect();
    let debug_status = after_search
        .find_by_author_id(ATELIER_CKC_SEARCH_STATUS_AUTHOR_ID)
        .and_then(|node| node.label.clone())
        .unwrap_or_else(|| "<missing status>".to_owned());
    assert!(
        after_search
            .find_by_author_id(&second_album_result)
            .is_some(),
        "Argus must see a stable CKC search result row for the matching album; expected {second_album_result}, status {debug_status:?}, saw {search_debug_ids:?}"
    );
    let status = after_search
        .find_by_author_id(ATELIER_CKC_SEARCH_STATUS_AUTHOR_ID)
        .and_then(|node| node.label.clone())
        .expect("search status visible");
    assert!(
        status.contains("Combined") && status.contains("result"),
        "combined search status must be visible after Argus click; got {status}"
    );

    let save_note = dispatch_request(
        &argus_req(
            "argus.click",
            serde_json::json!({ "target": ATELIER_CKC_TAG_NOTE_SAVE_AUTHOR_ID }),
        ),
        &argus_token(),
        &after_search,
        &mut channel,
        || Err(ScreenshotError("not used".to_owned())),
    );
    assert_eq!(save_note.to_json()["result"]["queued"], true);
    for event in channel.drain_into_events() {
        harness.event(event);
    }
    harness.run();
    harness.run();
    let after_note = snapshot_harness(&mut harness);
    let status = after_note
        .find_by_author_id(ATELIER_CKC_SEARCH_STATUS_AUTHOR_ID)
        .and_then(|node| node.label.clone())
        .expect("tag-note save status visible");
    assert!(
        status.contains("Saved local CKC tag note for training"),
        "tag-note save status must be visible after Argus click; got {status}"
    );
}

#[test]
fn atelier_internal_tabs_switch_visible_content_regions() {
    let mut harness = build_panel_harness();
    harness.run();

    harness
        .get_by(|node| node.author_id() == Some(ATELIER_TAB_POSEKIT_AUTHOR_ID))
        .click();
    harness.run();
    let ids = author_ids(&harness);
    assert!(
        ids.contains(ATELIER_CONTENT_POSEKIT_AUTHOR_ID),
        "Posekit content region should be visible after clicking the Posekit tab"
    );

    harness
        .get_by(|node| node.author_id() == Some(ATELIER_TAB_INGEST_AUTHOR_ID))
        .click();
    harness.run();
    let ids = author_ids(&harness);
    assert!(
        ids.contains(ATELIER_CONTENT_INGEST_AUTHOR_ID),
        "Ingest content region should be visible after clicking the Ingest tab"
    );
}

#[test]
fn posekit_and_ingest_controls_are_model_addressable() {
    let mut harness = build_panel_harness();
    harness.run();

    harness
        .get_by(|node| node.author_id() == Some(ATELIER_TAB_POSEKIT_AUTHOR_ID))
        .click();
    harness.run();
    for expected in [
        ATELIER_POSE_YAW_MINUS_AUTHOR_ID,
        ATELIER_POSE_YAW_PLUS_AUTHOR_ID,
        ATELIER_POSE_RESET_AUTHOR_ID,
        ATELIER_POSE_FACE_TOGGLE_AUTHOR_ID,
        ATELIER_POSE_BODY_TOGGLE_AUTHOR_ID,
        ATELIER_POSE_HANDS_TOGGLE_AUTHOR_ID,
        ATELIER_POSE_YAW_SLIDER_AUTHOR_ID,
        ATELIER_POSE_PITCH_SLIDER_AUTHOR_ID,
        ATELIER_POSE_ZOOM_SLIDER_AUTHOR_ID,
    ] {
        let node = harness.get_by(|node| node.author_id() == Some(expected));
        assert!(
            node.accesskit_node()
                .data()
                .supports_action(egui::accesskit::Action::Click)
                || node
                    .accesskit_node()
                    .data()
                    .supports_action(egui::accesskit::Action::Focus),
            "Posekit control {expected} must be steerable by Argus/MCP"
        );
    }

    harness
        .get_by(|node| node.author_id() == Some(ATELIER_TAB_INGEST_AUTHOR_ID))
        .click();
    harness.run();
    for expected in [
        ATELIER_INGEST_PASS_AUTHOR_ID,
        ATELIER_INGEST_REJECT_AUTHOR_ID,
        ATELIER_INGEST_UNSURE_AUTHOR_ID,
        ATELIER_INGEST_BATCH_TAGS_AUTHOR_ID,
    ] {
        let node = harness.get_by(|node| node.author_id() == Some(expected));
        assert!(
            node.accesskit_node()
                .data()
                .supports_action(egui::accesskit::Action::Click)
                || node
                    .accesskit_node()
                    .data()
                    .supports_action(egui::accesskit::Action::Focus),
            "Ingest control {expected} must be steerable by Argus/MCP"
        );
    }
}
