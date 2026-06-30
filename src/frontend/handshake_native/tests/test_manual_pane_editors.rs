//! WP-KERNEL-012 MT-073 (E12) — built-in User Manual editors content + agent-tool reference proofs.
//!
//! Proves, with REAL runtime evidence (no tautologies):
//! - AC-001 / PT-001: the manual pane loads the editors section and ALL eight GLOBAL-BUILD-MANUAL
//!   headings are present as individual topics.
//! - AC-002 / PT-002: the agent-tool reference lists every editor/knowledge/FEMS/interop action with a
//!   NON-EMPTY author_id + a NON-EMPTY MCP tool that is one of the four REAL mcp/tools.rs methods.
//! - AC-003 / PT-003: the WP-011-style manual SEARCH box (driven via egui_kittest) finds an editor topic
//!   by keyword — a live interaction, not an in-memory assertion.
//! - AC-004 / PT-004: NO documented author_id is missing from the LIVE AccessKit registry — the id-audit
//!   cross-checks every agent-tool-reference author_id against the live registries (catalogs +
//!   DECLARED_IDENTITIES + the fixed interop/FEMS/Stage/Calendar/Locus constants) and fails on any orphan.
//! - AC-005 / PT-002: the four interop edges (FEMS, Stage, Calendar, Locus) are each documented with an
//!   author_id + mcp_tool.
//! - MC-006: the manual content contains NO 'SQLite' token and no direct-DB-write language.
//!
//! ARTIFACT HYGIENE (CX-212E / the SCREENSHOT/TEST-ARTIFACT rule): the HBR-VIS screenshot is written ONLY
//! to the EXTERNAL Handshake_Artifacts/handshake-test/wp-kernel-012-mt-073/ root via
//! [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if any repo-local
//! `test_output/` or `tests/screenshots/` dir exists.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::accessibility::editor_action_registry::{
    rich_action_catalog, CODE_ACTION_CATALOG,
};
use handshake_native::accessibility::{
    CANVAS_CONTROL_CATALOG, COLLECTION_CONTROL_CATALOG, DECLARED_IDENTITIES, GRAPH_CONTROL_CATALOG,
    PALETTE_AUTHOR_IDS,
};
use handshake_native::manual_content_editors::{
    agent_tool_rows, editors_manual_section, INTEROP_EDGES, REQUIRED_HEADINGS,
};
use handshake_native::manual_pane::{
    ManualPane, ManualPaneState, ManualRegistry, ManualSurface, MANUAL_SEARCH_AUTHOR_ID,
};
use handshake_native::theme::HsPalette;

/// The crate-relative path to the external artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts` is a
/// sibling of the repo worktree. (The SCREENSHOT/TEST-ARTIFACT rule overrides any repo-local path.)
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (the artifact-hygiene guard the
/// SCREENSHOT/TEST-ARTIFACT rule mandates). Checks BOTH `test_output/` and `tests/screenshots/`.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "artifact hygiene: no repo-local '{local}' dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

/// Serialize the `.wgpu()` screenshot test (the documented Windows-wgpu concurrent-device hazard).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// The canonical Argus tool names plus their compatibility MCP primitive names.
const REAL_MCP_TOOLS: &[&str] = &[
    "argus.inspect",
    "argus.click",
    "argus.set_value",
    "argus.screenshot",
    "list_widgets",
    "click_widget",
    "set_value",
    "screenshot",
];

/// Build the LIVE author_id set — the union of every real registered/static author_id across the surfaces
/// the manual documents. This is the id-audit's source of truth; a documented author_id absent from this
/// set is an ORPHAN (AC-004).
///
/// Sources (all LIVE registry-owning resources, none hand-typed): every entry is read from a real
/// registry/catalog/const so a documented id that drifts from the live id is caught as an orphan — the
/// set is NEVER seeded with a literal copy of a documented id (that would make the audit tautological).
/// - shell chrome: [`DECLARED_IDENTITIES`] + [`PALETTE_AUTHOR_IDS`] (the dot-form command-palette
///   container ids the live shell emits; quick-switcher / settings container ids live in
///   DECLARED_IDENTITIES);
/// - code editor: `editor.code.<action>` for every [`CODE_ACTION_CATALOG`] entry;
/// - rich editor: `editor.rich.<action>` for every `rich_action_catalog()` entry;
/// - graph/canvas/collection: the three control catalogs;
/// - FEMS / Stage / Calendar / Locus / manual: the fixed `&'static str` constants from their modules.
fn live_author_id_set() -> HashSet<String> {
    let mut set: HashSet<String> = HashSet::new();

    // Shell chrome declared identities (this is where the command-palette + quick-switcher + settings
    // container ids actually live — the DOT-form ids the live shell emits).
    for ident in DECLARED_IDENTITIES {
        set.insert(ident.author_id.to_owned());
    }
    // The command-palette dialog/search/list container ids, sourced from the REAL registry const
    // (PALETTE_AUTHOR_IDS = command-palette.dialog/.search/.list) — NOT hand-typed literals. These are
    // already covered by DECLARED_IDENTITIES above; pulling them from the same const the registry exports
    // keeps the audit reading the live resource instead of an implementer-authored mirror, so any
    // documented palette id that drifts from the live id is correctly flagged as an orphan (AC-004/MC-001).
    for id in PALETTE_AUTHOR_IDS {
        set.insert((*id).to_owned());
    }

    // Code editor canonical action ids.
    for entry in CODE_ACTION_CATALOG {
        set.insert(format!("editor.code.{}", entry.action_id));
    }
    // Rich editor canonical action ids.
    for entry in rich_action_catalog() {
        set.insert(format!("editor.rich.{}", entry.action_id));
    }
    // Graph / canvas / collection control catalogs.
    for entry in GRAPH_CONTROL_CATALOG {
        set.insert(entry.author_id.to_owned());
    }
    for entry in CANVAS_CONTROL_CATALOG {
        set.insert(entry.author_id.to_owned());
    }
    for entry in COLLECTION_CONTROL_CATALOG {
        set.insert(entry.author_id.to_owned());
    }

    // FEMS fixed ids.
    set.insert(handshake_native::fems::RELEVANT_MEMORY_PANEL_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::RELEVANT_MEMORY_LIST_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::FEMS_PROPOSE_DIALOG_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::FEMS_PROPOSE_CONFIRM_AUTHOR_ID.to_owned());

    // Stage fixed ids.
    set.insert(handshake_native::stage_pane::STAGE_PANE_AUTHOR_ID.to_owned());
    set.insert(handshake_native::stage_pane::STAGE_ROUTED_CONTENT_AUTHOR_ID.to_owned());
    set.insert(handshake_native::stage_pane::STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID.to_owned());

    // Calendar (daily-journal) fixed ids.
    set.insert(
        handshake_native::graph::daily_journal_panel::DAILY_JOURNAL_PANEL_AUTHOR_ID.to_owned(),
    );
    set.insert(
        handshake_native::graph::daily_journal_panel::DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID
            .to_owned(),
    );
    set.insert(
        handshake_native::graph::daily_journal_panel::DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID
            .to_owned(),
    );
    set.insert(
        handshake_native::graph::daily_journal_panel::DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID
            .to_owned(),
    );

    // Locus (outgoing-links) fixed ids.
    set.insert(
        handshake_native::rich_editor::wikilinks::outgoing_links_panel::PANEL_AUTHOR_ID.to_owned(),
    );
    set.insert(
        handshake_native::rich_editor::wikilinks::outgoing_links_panel::RESOLVED_SECTION_AUTHOR_ID
            .to_owned(),
    );
    set.insert(
        handshake_native::rich_editor::wikilinks::outgoing_links_panel::UNRESOLVED_SECTION_AUTHOR_ID
            .to_owned(),
    );

    // Atelier fixed ids, sourced from the live panel constants.
    for id in [
        handshake_native::atelier_panel::ATELIER_PANEL_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_TABLIST_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_TAB_CKC_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_TAB_POSEKIT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_TAB_INGEST_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CONTENT_CKC_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CONTENT_POSEKIT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CONTENT_INGEST_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_BOOK_LAYOUT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MEDIA_VIEWER_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MODE_SHEET_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MODE_STORY_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MODE_NOTES_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MODE_MOODBOARD_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_CHARACTER_NOTES_EDITOR_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_CHARACTER_NOTES_APPLY_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_CHARACTER_LIST_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SELECTED_CHARACTER_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_CHARACTER_CREATE_NAME_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_CHARACTER_CREATE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_CHARACTER_REF_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_SAVE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_TYPED_REF_KIND_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_TEMPLATE_STATUS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_TEMPLATE_LOAD_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SAFE_SUBSET_LOAD_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_IMPORT_EDITOR_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_IMPORT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_EXPORT_TXT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_EXPORT_JSON_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_EXPORT_SAFE_TXT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_EXPORT_SAFE_JSON_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_EXPORT_STATUS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_EXPORT_REF_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_EXPORT_PREVIEW_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_FIELD_SUGGESTION_FIELD_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_FIELD_SUGGESTIONS_LOAD_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_FIELD_SUGGESTIONS_LIST_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_LIST_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_KIND_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_REF_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_MANIFEST_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_LABEL_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_ROLE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_ACTOR_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_ATTACH_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_ATTACH_POSE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_DETACH_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_REUSE_REF_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_STATUS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_ALBUM_STATUS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_ALBUM_CREATE_NAME_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_ALBUM_CREATE_NOTES_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_ALBUM_CREATE_TAGS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_ALBUM_CREATE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_ALBUM_LINK_ASSET_IDS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_ALBUM_LINK_SOURCE_PATH_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_ALBUM_LINK_SOURCE_URL_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_ALBUM_LINK_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SEARCH_QUERY_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SEARCH_TAGS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SEARCH_FILTER_CHARACTER_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SEARCH_FILTER_COLLECTION_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SEARCH_FILTER_MEDIA_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SEARCH_FILTER_SIMILARITY_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SEARCH_MODE_FUZZY_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SEARCH_MODE_VECTOR_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SEARCH_MODE_COMBINED_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SEARCH_RUN_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SEARCH_STATUS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_SEARCH_RESULTS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_TAG_NOTE_TAG_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_TAG_NOTE_SCOPE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_TAG_NOTE_EDITOR_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_TAG_NOTE_SAVE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_STORY_DOC_REF_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_STORY_EDITOR_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_STORY_SAVE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_STORY_CARD_LIST_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_STORY_CARD_TITLE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_STORY_CARD_BODY_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_STORY_CARD_SAVE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_STORY_BEAT_EDITOR_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_STORY_BEAT_SAVE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MOODBOARD_DOC_REF_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MOODBOARD_LATEST_REF_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MOODBOARD_SAVE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MOODBOARD_OPEN_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_SOURCE_REF_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_RIG_ID_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_STATE_READOUT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_SPLIT_VIEW_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_3D_VIEWPORT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_OPENPOSE_VIEWPORT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_YAW_MINUS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_YAW_PLUS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_RESET_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_FACE_TOGGLE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_BODY_TOGGLE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_HANDS_TOGGLE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_YAW_SLIDER_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_PITCH_SLIDER_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_ZOOM_SLIDER_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_MARKER_FAMILY_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_MARKER_INDEX_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_MARKER_X_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_MARKER_Y_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_MARKER_CONFIDENCE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_MARKER_APPLY_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_MARKER_ADD_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_MARKER_REMOVE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_MARKER_RESET_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_MARKER_NUDGE_LEFT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_MARKER_NUDGE_RIGHT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_MARKER_NUDGE_UP_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_MARKER_NUDGE_DOWN_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_MARKER_STATUS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_FRAMING_PRESET_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_FRAMING_LENS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_FRAMING_PADDING_TOP_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_FRAMING_PADDING_RIGHT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_FRAMING_PADDING_BOTTOM_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_FRAMING_PADDING_LEFT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_FRAMING_READOUT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_EXPORT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_EXPORT_STATUS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_EXPORT_REF_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_POSE_EXPORT_PREVIEW_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_PASS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_REJECT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_UNSURE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_BATCH_TAGS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_DATASET_REF_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_CHARACTER_REF_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_BATCH_NOTE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_EVENT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_DATE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_LOCATION_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_LINK_PASSED_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_APPLY_BATCH_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_CONTACT_ROWS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_CONTACT_COLUMNS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_CONTACT_DPI_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_CONTACT_EXPORT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_FACIAL_PROFILE_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_QUEUE_READOUT_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_STATUS_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_INGEST_LAST_RECEIPT_AUTHOR_ID,
    ] {
        set.insert(id.to_owned());
    }

    // Manual pane's own search box id (documented as a Knowledge surface row).
    set.insert(MANUAL_SEARCH_AUTHOR_ID.to_owned());

    set
}

fn rendered_ckc_panel_author_id_set() -> HashSet<String> {
    let side_panel = Arc::new(Mutex::new(
        handshake_native::atelier_side_panel::AtelierSidePanel::with_rows(
            vec![handshake_native::backend_client::AtelierBatchRow {
                batch_id: "batch-1".to_owned(),
                source_label: "Manual Audit Batch".to_owned(),
                status: "open".to_owned(),
            }],
            vec![],
            Some((
                "batch-1".to_owned(),
                vec![handshake_native::backend_client::AtelierItemRow {
                    item_id: "item-aaa".to_owned(),
                    file_name: "manual-audit.png".to_owned(),
                    source_path: "/intake/manual-audit.png".to_owned(),
                    lane: "accept".to_owned(),
                }],
            )),
        ),
    ));
    let panel = handshake_native::atelier_panel::AtelierPanel::new(
        side_panel,
        Arc::new(Mutex::new(
            handshake_native::graph::canvas_board::LoomCanvasBoard::new("ws-test", "canvas-1"),
        )),
        Arc::new(Mutex::new(Vec::<
            handshake_native::graph::canvas_board::CanvasEvent,
        >::new())),
    );
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 760.0))
        .build_state(
            |ctx, panel: &mut handshake_native::atelier_panel::AtelierPanel| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    panel.show(ui, &handshake_native::theme::HsTheme::Dark.palette());
                });
            },
            panel,
        );
    harness.run();
    harness.run();
    let mut ids: HashSet<String> = harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect();

    let export_txt_node_id = harness
        .root()
        .children_recursive()
        .find(|node| {
            node.accesskit_node().author_id()
                == Some(handshake_native::atelier_panel::ATELIER_CKC_EXPORT_TXT_AUTHOR_ID)
        })
        .expect("CKC export txt button present in rendered Atelier panel")
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: export_txt_node_id,
            data: None,
        },
    ));
    harness.run();
    harness.run();
    ids.extend(
        harness
            .root()
            .children_recursive()
            .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned)),
    );

    for mode_author_id in [
        handshake_native::atelier_panel::ATELIER_CKC_MODE_STORY_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MODE_NOTES_AUTHOR_ID,
        handshake_native::atelier_panel::ATELIER_CKC_MODE_MOODBOARD_AUTHOR_ID,
    ] {
        let node_id = harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(mode_author_id))
            .unwrap_or_else(|| panic!("CKC mode button {mode_author_id} present"))
            .accesskit_node()
            .id();
        harness.event(egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Click,
                target: node_id,
                data: None,
            },
        ));
        harness.run();
        harness.run();
        ids.extend(
            harness
                .root()
                .children_recursive()
                .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned)),
        );
    }
    ids
}

// ── AC-001 / PT-001: all eight GLOBAL-BUILD-MANUAL headings present as topics ─────────────────────────
#[test]
fn manual_loads_section_with_all_eight_required_headings() {
    let mut reg = ManualRegistry::new();
    reg.register_section(editors_manual_section());
    assert_eq!(reg.len(), 1, "the editors section registered into the pane");

    let section = reg
        .section("native-editors")
        .expect("editors section is registered");
    for heading in REQUIRED_HEADINGS {
        assert!(
            section.topic(heading).is_some(),
            "AC-001: GLOBAL-BUILD-MANUAL heading '{heading}' must be present as an individual topic"
        );
        // Each topic body must be a real no-context body (not an empty stub).
        let body = &section.topic(heading).unwrap().body;
        assert!(
            body.len() > 60,
            "AC-001: heading '{heading}' must carry a substantive no-context body (got {} chars)",
            body.len()
        );
    }
    assert_eq!(
        REQUIRED_HEADINGS.len(),
        8,
        "exactly the eight GLOBAL-BUILD-MANUAL headings"
    );
}

// ── MT-006 Argus: native manual names the non-intrusive visual inspection tool ───────────────────────
#[test]
fn manual_documents_argus_as_native_non_intrusive_visual_inspection() {
    let section = editors_manual_section();
    let topic = section
        .topic("Argus Visual Inspection")
        .expect("Argus topic exists in the native manual");

    for required in [
        "Argus",
        "Rust-native",
        "AccessKit",
        "list_widgets",
        "screenshot",
        "parallel agents",
        "must not bring Handshake to the foreground",
        "must not steal keyboard focus",
        "must not steal mouse input",
        "stable author_id",
        "not inspected",
    ] {
        assert!(
            topic.body.contains(required),
            "Argus native manual topic missing required guidance: {required}"
        );
    }
}

#[test]
fn manual_documents_atelier_tabs_and_argus_control_ids() {
    let section = editors_manual_section();
    let topic = section
        .topic("Atelier Tools")
        .expect("Atelier Tools topic exists in the native manual");

    for required in [
        "module-ckc",
        "atelier-main-panel",
        "atelier-tab-ckc",
        "atelier-tab-posekit",
        "atelier-tab-ingest",
        "book-style two-pane",
        "atelier-ckc-book-layout",
        "atelier-ckc-book-left-media",
        "atelier-ckc-media-viewer",
        "atelier-ckc-book-right-sheet",
        "atelier-ckc-book-middle",
        "atelier-ckc-mode-sheet",
        "atelier-ckc-mode-story",
        "atelier-ckc-mode-notes",
        "atelier-ckc-mode-moodboard",
        "desktop and constrained sizes",
        "four-window grid",
        "atelier-ckc-character-list",
        "atelier-ckc-sheet-editor",
        "atelier-ckc-sheet-save-version",
        "atelier-ckc-sheet-version-ref",
        "atelier-ckc-template-status",
        "atelier-ckc-template-load",
        "atelier-ckc-safe-subset-load",
        "atelier-ckc-import-editor",
        "atelier-ckc-import-sheet-version",
        "atelier-ckc-export-txt",
        "atelier-ckc-export-json",
        "atelier-ckc-export-safe-txt",
        "atelier-ckc-export-safe-json",
        "atelier-ckc-export-status",
        "atelier-ckc-export-ref",
        "atelier-ckc-export-preview",
        "atelier-ckc-field-suggestion-field",
        "atelier-ckc-field-suggestions-load",
        "atelier-ckc-field-suggestions-list",
        "GET/POST /atelier/sheet-versions/{version_id}/artifact-links",
        "GET/DELETE /atelier/sheet-artifact-links/{link_id}",
        "atelier-ckc-sheet-artifact-list",
        "atelier-ckc-sheet-artifact-kind",
        "atelier-ckc-sheet-artifact-ref",
        "atelier-ckc-sheet-artifact-manifest",
        "atelier-ckc-sheet-artifact-label",
        "atelier-ckc-sheet-artifact-role",
        "atelier-ckc-sheet-artifact-actor",
        "atelier-ckc-sheet-artifact-attach",
        "atelier-ckc-sheet-artifact-attach-posekit",
        "atelier-ckc-sheet-artifact-detach",
        "atelier-ckc-sheet-artifact-reuse-ref",
        "atelier-ckc-sheet-artifact-status",
        "atelier://sheet-artifact/{link_id}",
        "openpose_json",
        "openpose_png",
        "conditioning_png",
        "comfy_render",
        "comfy_receipt",
        "cui_openpose_conditioning",
        "parallel agents must keep distinct actor ids",
        "do not delete artifact files from CKC UI",
        "atelier-ckc-linked-media-list",
        "atelier-ckc-album-status",
        "atelier-ckc-album-create-name",
        "atelier-ckc-album-create-notes",
        "atelier-ckc-album-create-tags",
        "atelier-ckc-album-create",
        "atelier-ckc-album-link-asset-ids",
        "atelier-ckc-album-link-assets",
        "atelier-ckc-album-load-more-*",
        "GET /atelier/media-albums/{collection_id}/items?offset=...&limit=200",
        "atelier-ckc-media-notes-editor",
        "atelier-ckc-media-tags-editor",
        "atelier-ckc-media-save",
        "atelier-ckc-search-query",
        "atelier-ckc-search-tags",
        "atelier-ckc-search-filter-character",
        "atelier-ckc-search-filter-collection",
        "atelier-ckc-search-filter-media",
        "atelier-ckc-search-filter-similarity",
        "atelier-ckc-search-mode-fuzzy",
        "atelier-ckc-search-mode-vector",
        "atelier-ckc-search-mode-combined",
        "atelier-ckc-search-run",
        "atelier-ckc-search-status",
        "atelier-ckc-search-results",
        "atelier-ckc-search-result-*",
        "llm_embedding+pgvector_projection",
        "semantic_unavailable_no_embedding_model",
        "POST /atelier/ckc/search",
        "atelier-ckc-tag-note-tag",
        "atelier-ckc-tag-note-scope",
        "atelier-ckc-tag-note-editor",
        "atelier-ckc-tag-note-save",
        "POST /atelier/ckc/tag-notes",
        "Click atelier-ckc-mode-story before expecting story controls",
        "atelier-ckc-character-notes-editor",
        "atelier-ckc-character-notes-apply",
        "Character sheet notes are not image notes",
        "Click atelier-ckc-mode-moodboard before expecting moodboard controls",
        "/atelier/characters/{character_internal_id}/documents?doc_type=story",
        "atelier://document/{document_id}",
        "atelier-ckc-story-doc-ref",
        "atelier-ckc-story-editor",
        "atelier-ckc-story-save",
        "/atelier/character-documents/{document_id}/story-cards",
        "/atelier/character-documents/{document_id}/story-beats",
        "atelier-ckc-story-card-list",
        "atelier-ckc-story-card-title",
        "atelier-ckc-story-card-body",
        "atelier-ckc-story-card-save",
        "atelier-ckc-story-beat-editor",
        "atelier-ckc-story-beat-save",
        "/atelier/characters/{character_internal_id}/documents?doc_type=moodboard",
        "/atelier/character-documents/{document_id}/moodboard/snapshots",
        "/atelier/character-documents/{document_id}/moodboard/latest",
        "atelier://moodboard/{snapshot_id}",
        "atelier-ckc-moodboard-doc-ref",
        "atelier-ckc-moodboard-latest-ref",
        "atelier-ckc-moodboard-editor",
        "atelier-ckc-moodboard-save",
        "atelier-ckc-moodboard-open",
        "atelier-ckc-moodboard-canvas",
        "Story, sheet notes, image notes, tag notes, and moodboards stay distinct but cross-linked",
        "inspect/click/set_value the story and moodboard controls",
        "technical debt",
        "atelier-ckc-album-",
        "atelier-ckc-media-",
        "atelier-ckc-folder-",
        "atelier-ckc-source-url-",
        "media_album",
        "draggable atelier-ref",
        "refKind media",
        "refKind folder",
        "refKind source_url",
        "source_path_ref/source_url_ref",
        "atelier-ckc-album-link-source-path",
        "atelier-ckc-album-link-source-url",
        "character_sheet",
        "x-hsk-actor-id",
        "expected_parent_version_id",
        "stale_sheet_version",
        "current_head_sheet_version_ref",
        "re-inspect the panel",
        "/atelier/characters",
        "/atelier/sheet-templates/default",
        "CHARACTER_SHEET__v2.00.txt",
        "LLM_SAFE_SUBSET__v2.00.json",
        "/sheet-versions/import",
        "/export?format=txt|json|safe-txt|safe-json",
        "JSON export envelope",
        "CHAR-ID-001",
        "/atelier/sheet-field-suggestions",
        "/media-albums",
        "/notes-tags",
        "Field ID",
        "native OpenRepose-style split-view workflow",
        "atelier-pose-source-ref",
        "atelier-pose-rig-id",
        "atelier-pose-state-readout",
        "atelier-pose-split-view",
        "atelier-pose-3d-viewport",
        "atelier-pose-openpose-viewport",
        "atelier-pose-yaw-minus",
        "atelier-pose-yaw-plus",
        "atelier-pose-reset",
        "atelier-pose-yaw-slider",
        "atelier-pose-pitch-slider",
        "atelier-pose-zoom-slider",
        "atelier-pose-face-toggle",
        "atelier-pose-body-toggle",
        "atelier-pose-hands-toggle",
        "atelier-pose-marker-family",
        "atelier-pose-marker-index",
        "atelier-pose-marker-x",
        "atelier-pose-marker-y",
        "atelier-pose-marker-confidence",
        "atelier-pose-marker-apply",
        "atelier-pose-marker-remove",
        "atelier-pose-marker-add",
        "safe empty slot",
        "atelier-pose-marker-nudge-left",
        "atelier-pose-marker-nudge-right",
        "atelier-pose-marker-nudge-up",
        "atelier-pose-marker-nudge-down",
        "atelier-pose-marker-reset",
        "atelier-pose-marker-status",
        "rejected edit must leave the previous export preview intact",
        "atelier-pose-framing-preset",
        "atelier-pose-framing-lens",
        "atelier-pose-framing-padding-top",
        "atelier-pose-framing-padding-right",
        "atelier-pose-framing-padding-bottom",
        "atelier-pose-framing-padding-left",
        "atelier-pose-framing-readout",
        "full_body_with_feet",
        "ComfyUI full-body outputs",
        "atelier-pose-export-openpose",
        "atelier-pose-export-status",
        "atelier-pose-export-ref",
        "atelier-pose-export-preview",
        "hsk.atelier.posekit.openpose_export@1",
        "image/png",
        "body 18",
        "face 70",
        "hand 21",
        "marker_edits",
        "framing metadata",
        "source_ref provenance",
        "rig_id lineage",
        "content_hash",
        "artifact_ref",
        "backend ArtifactStore receipt JSON metadata",
        "preview://atelier/posekit/openpose",
        "Return to atelier-tab-ckc",
        "reusable ComfyUI conditioning artifact",
        "argus.screenshot{} for a full-frame visual proof",
        "screenshot target cropping is not supported yet",
        "headless/non-intrusive",
        "no foreground window",
        "no keyboard capture",
        "no mouse steal",
        "atelier-ingest-pass",
        "atelier-ingest-batch-tags",
        "atelier-ingest-dataset-ref",
        "atelier-ingest-character-ref",
        "atelier-ingest-actor",
        "atelier-ingest-batch-note",
        "atelier-ingest-event",
        "atelier-ingest-date",
        "atelier-ingest-location",
        "atelier-ingest-link-passed",
        "atelier-ingest-apply-batch",
        "atelier-ingest-contact-rows",
        "atelier-ingest-contact-columns",
        "atelier-ingest-contact-dpi",
        "atelier-ingest-contact-export",
        "atelier-ingest-facial-profile",
        "atelier-ingest-queue-readout",
        "atelier-ingest-batch-summary",
        "atelier-ingest-status",
        "atelier-ingest-last-receipt",
        "atelier-intake-batch-{stable_batch_id}",
        "Facial quality/dedupe/identity profile",
        "contact sheet",
        "link intent metadata",
        "full canonical backend batch",
        "requested_by",
        "applied_count",
        "applied_preview_count",
        "total_item_count",
        "canonical_counts_loaded=false",
        "truncated_count",
        "Argus",
    ] {
        assert!(
            topic.body.contains(required),
            "Atelier manual topic missing required control/navigation guidance: {required}"
        );
    }
}

#[test]
fn manual_agent_tool_rows_cover_ingest_dataset_contact_and_facial_controls() {
    let rows = agent_tool_rows();
    for (author_id, tool) in [
        (
            handshake_native::atelier_panel::ATELIER_INGEST_DATASET_REF_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_CHARACTER_REF_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_ACTOR_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_BATCH_NOTE_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_EVENT_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_DATE_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_LOCATION_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_LINK_PASSED_AUTHOR_ID,
            "argus.click",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_APPLY_BATCH_AUTHOR_ID,
            "argus.click",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_CONTACT_ROWS_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_CONTACT_COLUMNS_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_CONTACT_DPI_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_CONTACT_EXPORT_AUTHOR_ID,
            "argus.click",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_FACIAL_PROFILE_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_QUEUE_READOUT_AUTHOR_ID,
            "argus.inspect",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_BATCH_SUMMARY_AUTHOR_ID,
            "argus.inspect",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_STATUS_AUTHOR_ID,
            "argus.inspect",
        ),
        (
            handshake_native::atelier_panel::ATELIER_INGEST_LAST_RECEIPT_AUTHOR_ID,
            "argus.inspect",
        ),
    ] {
        let row = rows
            .iter()
            .find(|row| row.author_id == author_id)
            .unwrap_or_else(|| panic!("missing manual agent-tool row for {author_id}"));
        assert_eq!(
            row.mcp_tool, tool,
            "manual row for {author_id} must use {tool}"
        );
    }
}

#[test]
fn manual_agent_tool_rows_cover_ckc_sheet_artifact_controls() {
    let rows = agent_tool_rows();
    for (author_id, tool) in [
        (
            handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_LIST_AUTHOR_ID,
            "argus.inspect",
        ),
        (
            handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_KIND_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_REF_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_MANIFEST_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_LABEL_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_ROLE_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_ACTOR_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_ATTACH_AUTHOR_ID,
            "argus.click",
        ),
        (
            handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_ATTACH_POSE_AUTHOR_ID,
            "argus.click",
        ),
        (
            handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_DETACH_AUTHOR_ID,
            "argus.click",
        ),
        (
            handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_REUSE_REF_AUTHOR_ID,
            "argus.inspect",
        ),
        (
            handshake_native::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_STATUS_AUTHOR_ID,
            "argus.inspect",
        ),
    ] {
        let row = rows
            .iter()
            .find(|row| row.author_id == author_id)
            .unwrap_or_else(|| panic!("missing manual agent-tool row for {author_id}"));
        assert_eq!(
            row.mcp_tool, tool,
            "manual row for {author_id} must use {tool}"
        );
    }
}

#[test]
fn manual_agent_tool_rows_cover_posekit_marker_and_framing_controls() {
    let rows = agent_tool_rows();
    for (author_id, tool) in [
        (
            handshake_native::atelier_panel::ATELIER_POSE_MARKER_FAMILY_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_POSE_MARKER_INDEX_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_POSE_MARKER_X_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_POSE_MARKER_Y_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_POSE_MARKER_CONFIDENCE_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_POSE_MARKER_APPLY_AUTHOR_ID,
            "argus.click",
        ),
        (
            handshake_native::atelier_panel::ATELIER_POSE_MARKER_ADD_AUTHOR_ID,
            "argus.click",
        ),
        (
            handshake_native::atelier_panel::ATELIER_POSE_MARKER_REMOVE_AUTHOR_ID,
            "argus.click",
        ),
        (
            handshake_native::atelier_panel::ATELIER_POSE_MARKER_STATUS_AUTHOR_ID,
            "argus.inspect",
        ),
        (
            handshake_native::atelier_panel::ATELIER_POSE_FRAMING_PRESET_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_POSE_FRAMING_LENS_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_POSE_FRAMING_PADDING_BOTTOM_AUTHOR_ID,
            "argus.set_value",
        ),
        (
            handshake_native::atelier_panel::ATELIER_POSE_FRAMING_READOUT_AUTHOR_ID,
            "argus.inspect",
        ),
    ] {
        let row = rows
            .iter()
            .find(|row| row.author_id == author_id)
            .unwrap_or_else(|| panic!("missing Posekit manual agent-tool row {author_id}"));
        assert_eq!(
            row.mcp_tool, tool,
            "Posekit manual row {author_id} must use the right Argus tool"
        );
    }
}

// ── AC-002 / PT-002: every agent-tool row has a non-empty author_id + a REAL mcp_tool ─────────────────
#[test]
fn agent_tool_reference_rows_are_complete_and_use_real_tools() {
    let rows = agent_tool_rows();
    assert!(
        rows.len() >= 30,
        "the reference covers every editor/knowledge/FEMS/interop action (got {})",
        rows.len()
    );
    for row in &rows {
        assert!(
            !row.author_id.is_empty(),
            "AC-002: a row has an empty author_id"
        );
        assert!(
            !row.mcp_tool.is_empty(),
            "AC-002: row '{}' has an empty mcp_tool",
            row.author_id
        );
        assert!(
            REAL_MCP_TOOLS.contains(&row.mcp_tool),
            "AC-002/RISK-002: row '{}' uses non-real MCP tool '{}' (must be one of {:?})",
            row.author_id,
            row.mcp_tool,
            REAL_MCP_TOOLS
        );
    }
    // The reference must cover EACH editor + knowledge + FEMS + interop surface (no surface omitted).
    let surfaces: HashSet<ManualSurface> = rows.iter().map(|r| r.surface).collect();
    for required in [
        ManualSurface::Code,
        ManualSurface::RichText,
        ManualSurface::Graph,
        ManualSurface::Canvas,
        ManualSurface::Knowledge,
        ManualSurface::Fems,
        ManualSurface::Interop,
    ] {
        assert!(
            surfaces.contains(&required),
            "AC-002: surface {required:?} has no agent-tool rows"
        );
    }

    let row_tools: HashSet<&str> = rows.iter().map(|r| r.mcp_tool).collect();
    for canonical in [
        "argus.inspect",
        "argus.click",
        "argus.set_value",
        "argus.screenshot",
    ] {
        assert!(
            row_tools.contains(canonical),
            "structured manual rows expose canonical Argus tool {canonical}"
        );
    }
}

// ── AC-004 / PT-004: id-audit — no documented author_id is an orphan ──────────────────────────────────
#[test]
fn id_audit_no_documented_author_id_missing_from_live_registry() {
    let live = live_author_id_set();
    // Sanity: the live set is non-trivial (guards against a false-green empty-registry pass).
    assert!(
        live.len() > 40,
        "live author_id set is suspiciously small ({})",
        live.len()
    );

    let rows = agent_tool_rows();
    let mut orphans: Vec<&str> = Vec::new();
    for row in &rows {
        if !live.contains(row.author_id) {
            orphans.push(row.author_id);
        }
    }
    assert!(
        orphans.is_empty(),
        "AC-004/MC-001: documented author_id(s) absent from the live AccessKit registry (ORPHANS): {orphans:?}"
    );
}

#[test]
fn id_audit_documented_ckc_author_ids_are_rendered_in_live_atelier_panel() {
    let live = rendered_ckc_panel_author_id_set();
    assert!(
        live.contains(handshake_native::atelier_panel::ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID),
        "runtime Atelier panel must expose the CKC moodboard editor"
    );
    assert!(
        live.contains(handshake_native::atelier_panel::ATELIER_CKC_MOODBOARD_SAVE_AUTHOR_ID),
        "runtime Atelier panel must expose the CKC moodboard save control"
    );

    let rows = agent_tool_rows();
    let mut orphans: Vec<&str> = Vec::new();
    for row in &rows {
        if row.author_id.starts_with("atelier-ckc-") && !live.contains(row.author_id) {
            orphans.push(row.author_id);
        }
    }
    assert!(
        orphans.is_empty(),
        "AC-004/MC-001: documented CKC author_id(s) absent from the rendered Atelier AccessKit tree: {orphans:?}"
    );
}

// ── AC-005 / PT-002: the four interop edges are each documented with an author_id + mcp_tool ───────────
#[test]
fn interop_edges_all_documented_with_author_id_and_tool() {
    let section = editors_manual_section();
    let interop_topic = section
        .topic("Interop Edges")
        .expect("the interop topic exists");
    // Each of FEMS / Stage / Calendar / Locus is named in the interop topic body (AC-005).
    for edge in INTEROP_EDGES {
        assert!(
            interop_topic.body.contains(edge),
            "AC-005/MC-007: interop edge '{edge}' is not named in the interop topic"
        );
    }
    assert_eq!(INTEROP_EDGES.len(), 4, "exactly FEMS/Stage/Calendar/Locus");

    // Each edge has at least one agent-tool row carrying a non-empty author_id + mcp_tool.
    let rows = agent_tool_rows();
    let interop_rows: Vec<_> = rows
        .iter()
        .filter(|r| r.surface == ManualSurface::Interop)
        .collect();
    assert!(
        interop_rows.len() >= 4,
        "AC-005: at least one interop row per edge (got {})",
        interop_rows.len()
    );
    // FEMS rows are the dedicated Fems surface (the FEMS edge); assert it too.
    let fems_rows: Vec<_> = rows
        .iter()
        .filter(|r| r.surface == ManualSurface::Fems)
        .collect();
    assert!(
        !fems_rows.is_empty(),
        "AC-005: the FEMS edge has agent-tool rows"
    );

    // Concretely assert each edge's signature author_id appears among the rows (Stage/Calendar/Locus on
    // the Interop surface; FEMS on the Fems surface).
    let row_ids: HashSet<&str> = rows.iter().map(|r| r.author_id).collect();
    assert!(
        row_ids.contains("stage-pane"),
        "Stage edge author_id present"
    );
    assert!(
        row_ids.contains("daily-journal-panel"),
        "Calendar edge author_id present"
    );
    assert!(
        row_ids.contains("outgoing.panel"),
        "Locus edge author_id present"
    );
    assert!(
        row_ids.contains("relevant-memory-panel"),
        "FEMS edge author_id present"
    );
}

// ── MC-006: the manual content names NO SQLite and no direct-DB-write language ────────────────────────
#[test]
fn manual_content_has_no_sqlite_and_no_direct_db_writes() {
    let section = editors_manual_section();
    let all_text: String = section
        .topics
        .iter()
        .map(|t| format!("{}\n{}", t.heading, t.body))
        .collect::<Vec<_>>()
        .join("\n");
    let lower = all_text.to_lowercase();
    assert!(
        !lower.contains("sqlite"),
        "MC-006: the manual must not mention SQLite"
    );
    // Persistence must be described as PostgreSQL/EventLedger via handshake_core.
    assert!(
        lower.contains("postgresql") || lower.contains("eventledger"),
        "MC-006: persistence must be described as PostgreSQL/EventLedger"
    );
    assert!(
        lower.contains("handshake_core"),
        "MC-006: persistence routes through handshake_core"
    );
    // No "direct DB write" affirmation (the manual states persistence is NOT direct).
    assert!(
        !lower.contains("write directly to the database") && !lower.contains("direct db write"),
        "MC-006: the manual must not describe direct DB writes as a path"
    );
}

// ── AC-003 / PT-003: the manual SEARCH box (live egui_kittest) finds an editor topic by keyword ───────
#[test]
fn manual_search_box_finds_editor_topic_by_keyword() {
    // The standalone manual-pane widget driven headlessly via egui_kittest (AccessKit enabled). Typing a
    // keyword into the search box filters the nav list to the matching topic — a LIVE interaction.
    let mut reg = ManualRegistry::new();
    reg.register_section(editors_manual_section());
    let palette = HsPalette::dark();
    let mut state = ManualPaneState::default();

    // Drive the pane in a kittest harness. State (reg/palette) lives outside the closure via 'static
    // leaks so the harness app closure can borrow them for 'static (the test owns process lifetime).
    let reg: &'static ManualRegistry = Box::leak(Box::new(reg));
    let palette: &'static HsPalette = Box::leak(Box::new(palette));

    let mut harness = Harness::builder().build_ui(move |ui| {
        ManualPane::new(reg, &mut state, palette).show(ui);
    });
    harness.run();

    // The search box carries the stable accessible label "Search Manual" (and the author_id
    // 'manual-search'). Type a keyword that lives in the "Core Workflows" topic body ("command palette").
    let search = harness.get_by_label("Search Manual");
    search.focus();
    harness.run();
    harness
        .get_by_label("Search Manual")
        .type_text("command palette");
    harness.run();
    harness.run();

    // After filtering, the matching topic surfaces in the live tree (the nav list + body show only
    // matching topics). "Core Workflows" mentions the command palette. The heading appears as BOTH a nav
    // Button AND a body Label, so count matches with query_all (query_by_label panics on >1).
    let match_count = harness.query_all_by_label("Core Workflows").count();
    assert!(
        match_count > 0,
        "AC-003: typing 'command palette' into manual-search surfaces the matching editor topic"
    );

    // A non-matching keyword filters it OUT (proves the search actually filters, not always-passes).
    // Appending more text makes the query no longer a substring of the topic, so the row disappears.
    harness
        .get_by_label("Search Manual")
        .type_text(" zzznotarealtopiczzz");
    harness.run();
    harness.run();
    let after_count = harness.query_all_by_label("Core Workflows").count();
    assert_eq!(
        after_count, 0,
        "AC-003: a non-matching keyword removes the topic (the search really filters)"
    );
}

// ── HBR-VIS: render the manual pane + save a screenshot to the EXTERNAL artifact root ─────────────────
#[test]
fn manual_pane_renders_and_screenshots() {
    let _guard = wgpu_guard();
    assert_no_local_artifact_dir();

    let mut reg = ManualRegistry::new();
    reg.register_section(editors_manual_section());
    let reg: &'static ManualRegistry = Box::leak(Box::new(reg));
    let palette: &'static HsPalette = Box::leak(Box::new(HsPalette::dark()));
    // Pre-select the agent-tool reference so the screenshot shows the steering table.
    let mut state = ManualPaneState {
        selected: Some((
            "native-editors".to_owned(),
            "Agent Tool Reference".to_owned(),
        )),
        ..Default::default()
    };

    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 620.0))
        .wgpu()
        .build_ui(move |ui| {
            ManualPane::new(reg, &mut state, palette).show(ui);
        });
    harness.run();
    harness.run();

    // The container + search box render without panic/overlap.
    assert!(
        harness.query_by_label("Search Manual").is_some(),
        "HBR-VIS: the manual search box renders"
    );

    let out_dir = external_artifact_dir("wp-kernel-012-mt-073");
    let _ = std::fs::create_dir_all(&out_dir);
    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image is non-empty");
            let out_path = out_dir.join("manual_pane_editors.png");
            let saved = image.save(&out_path).is_ok();
            let abs = std::fs::canonicalize(&out_path).unwrap_or(out_path.clone());
            println!(
                "PT-005 manual-pane screenshot: {w}x{h}, saved={saved} ({})",
                abs.display()
            );
            assert!(
                saved,
                "HBR-VIS: the manual pane screenshot PNG saved to the external root"
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-073 manual-pane screenshot render unavailable (no wgpu \
                 adapter): {e}. The content + search + id-audit proofs stand; the PNG is a GPU-host item."
            );
        }
    }

    assert_no_local_artifact_dir();
}
