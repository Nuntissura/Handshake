//! Native Atelier main panel.
//!
//! The shell-level Atelier module hosts sibling tool tabs inside one filling pane. CKC reuses the
//! existing Atelier intake/drag-source widget and canvas board; Posekit and Ingest expose stable,
//! nonblank native control surfaces so agents can address and inspect them before deeper parity work.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use egui::accesskit;
use sha2::{Digest, Sha256};

use crate::atelier_side_panel::AtelierSidePanel;
use crate::backend_client::{
    AtelierCharacterRow, AtelierCkcAppendCell, AtelierCkcCell, AtelierCkcCharacterDocumentCell,
    AtelierCkcCharacterDocumentRow, AtelierCkcCharacterSheetRow, AtelierCkcCreateCell,
    AtelierCkcExportCell, AtelierCkcFieldSuggestionsCell, AtelierCkcImportCell,
    AtelierCkcMediaAlbumCreateCell, AtelierCkcMediaAlbumItemsCell, AtelierCkcMediaAlbumRow,
    AtelierCkcMediaMemberRow, AtelierCkcMediaNotesCell, AtelierCkcMediaNotesTagsRow,
    AtelierCkcMoodboardSnapshotCell, AtelierCkcMoodboardSnapshotRow, AtelierCkcSafeSubsetCell,
    AtelierCkcSearchCell, AtelierCkcSearchResponse, AtelierCkcSearchResultRow,
    AtelierCkcSheetArtifactLinkRow, AtelierCkcSheetArtifactLinksCell, AtelierCkcStoryBeatCell,
    AtelierCkcStoryBeatRow, AtelierCkcStoryCardCell, AtelierCkcStoryCardRow, AtelierCkcTagNoteCell,
    AtelierCkcTagNoteRow, AtelierCkcTemplateCell, AtelierClient, AtelierIntakeClassificationCell,
    AtelierIntakeClassificationDecision, AtelierItemRow, AtelierPosekitExportCell,
    AtelierPosekitExportRow, AtelierSheetExportRow, AtelierSheetFieldSuggestionRow,
    AtelierSheetVersionRow,
};
use crate::editor_pane_factories::SharedPalette;
use crate::graph::canvas_board::{
    CanvasEvent, CanvasPlacementCard, LoomCanvasBoard, VisualEdge, DEFAULT_CARD_H, DEFAULT_CARD_W,
};
use crate::interop::{AtelierItemKind, AtelierRef, DragPayload};
use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::theme::HsPalette;
use uuid::Uuid;

const LOCAL_CKC_SAFE_SUBSET_V2_JSON: &str = include_str!(
    "../../../backend/handshake_core/src/atelier/templates/LLM_SAFE_SUBSET__v2.00.json"
);
const LOCAL_CKC_TEMPLATE_VERSION: &str = "v2.00";

pub const ATELIER_PANEL_AUTHOR_ID: &str = "atelier-main-panel";
pub const ATELIER_TABLIST_AUTHOR_ID: &str = "atelier-tab-list";
pub const ATELIER_TAB_CKC_AUTHOR_ID: &str = "atelier-tab-ckc";
pub const ATELIER_TAB_POSEKIT_AUTHOR_ID: &str = "atelier-tab-posekit";
pub const ATELIER_TAB_INGEST_AUTHOR_ID: &str = "atelier-tab-ingest";
pub const ATELIER_CONTENT_CKC_AUTHOR_ID: &str = "atelier-content-ckc";
pub const ATELIER_CONTENT_POSEKIT_AUTHOR_ID: &str = "atelier-content-posekit";
pub const ATELIER_CONTENT_INGEST_AUTHOR_ID: &str = "atelier-content-ingest";
pub const ATELIER_CKC_CHARACTER_LIST_AUTHOR_ID: &str = "atelier-ckc-character-list";
pub const ATELIER_CKC_SELECTED_CHARACTER_AUTHOR_ID: &str = "atelier-ckc-selected-character";
pub const ATELIER_CKC_CHARACTER_CREATE_NAME_AUTHOR_ID: &str = "atelier-ckc-character-create-name";
pub const ATELIER_CKC_CHARACTER_CREATE_AUTHOR_ID: &str = "atelier-ckc-character-create";
pub const ATELIER_CKC_CHARACTER_REF_AUTHOR_ID: &str = "atelier-ckc-character-ref";
pub const ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID: &str = "atelier-ckc-sheet-version-ref";
pub const ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID: &str = "atelier-ckc-sheet-editor";
pub const ATELIER_CKC_SHEET_SAVE_AUTHOR_ID: &str = "atelier-ckc-sheet-save-version";
pub const ATELIER_CKC_TYPED_REF_KIND_AUTHOR_ID: &str = "atelier-ckc-typed-ref-kind";
pub const ATELIER_CKC_TEMPLATE_STATUS_AUTHOR_ID: &str = "atelier-ckc-template-status";
pub const ATELIER_CKC_TEMPLATE_LOAD_AUTHOR_ID: &str = "atelier-ckc-template-load";
pub const ATELIER_CKC_SAFE_SUBSET_LOAD_AUTHOR_ID: &str = "atelier-ckc-safe-subset-load";
pub const ATELIER_CKC_IMPORT_EDITOR_AUTHOR_ID: &str = "atelier-ckc-import-editor";
pub const ATELIER_CKC_IMPORT_AUTHOR_ID: &str = "atelier-ckc-import-sheet-version";
pub const ATELIER_CKC_EXPORT_TXT_AUTHOR_ID: &str = "atelier-ckc-export-txt";
pub const ATELIER_CKC_EXPORT_JSON_AUTHOR_ID: &str = "atelier-ckc-export-json";
pub const ATELIER_CKC_EXPORT_SAFE_TXT_AUTHOR_ID: &str = "atelier-ckc-export-safe-txt";
pub const ATELIER_CKC_EXPORT_SAFE_JSON_AUTHOR_ID: &str = "atelier-ckc-export-safe-json";
pub const ATELIER_CKC_EXPORT_STATUS_AUTHOR_ID: &str = "atelier-ckc-export-status";
pub const ATELIER_CKC_EXPORT_PREVIEW_AUTHOR_ID: &str = "atelier-ckc-export-preview";
pub const ATELIER_CKC_EXPORT_REF_AUTHOR_ID: &str = "atelier-ckc-export-ref";
pub const ATELIER_CKC_FIELD_SUGGESTION_FIELD_AUTHOR_ID: &str = "atelier-ckc-field-suggestion-field";
pub const ATELIER_CKC_FIELD_SUGGESTIONS_LOAD_AUTHOR_ID: &str = "atelier-ckc-field-suggestions-load";
pub const ATELIER_CKC_FIELD_SUGGESTIONS_LIST_AUTHOR_ID: &str = "atelier-ckc-field-suggestions-list";
pub const ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID: &str = "atelier-ckc-linked-media-list";
pub const ATELIER_CKC_SHEET_ARTIFACT_LIST_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-list";
pub const ATELIER_CKC_SHEET_ARTIFACT_KIND_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-kind";
pub const ATELIER_CKC_SHEET_ARTIFACT_REF_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-ref";
pub const ATELIER_CKC_SHEET_ARTIFACT_MANIFEST_AUTHOR_ID: &str =
    "atelier-ckc-sheet-artifact-manifest";
pub const ATELIER_CKC_SHEET_ARTIFACT_LABEL_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-label";
pub const ATELIER_CKC_SHEET_ARTIFACT_ROLE_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-role";
pub const ATELIER_CKC_SHEET_ARTIFACT_ACTOR_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-actor";
pub const ATELIER_CKC_SHEET_ARTIFACT_ATTACH_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-attach";
pub const ATELIER_CKC_SHEET_ARTIFACT_ATTACH_POSE_AUTHOR_ID: &str =
    "atelier-ckc-sheet-artifact-attach-posekit";
pub const ATELIER_CKC_SHEET_ARTIFACT_DETACH_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-detach";
pub const ATELIER_CKC_SHEET_ARTIFACT_REUSE_REF_AUTHOR_ID: &str =
    "atelier-ckc-sheet-artifact-reuse-ref";
pub const ATELIER_CKC_SHEET_ARTIFACT_STATUS_AUTHOR_ID: &str = "atelier-ckc-sheet-artifact-status";
pub const ATELIER_CKC_ALBUM_STATUS_AUTHOR_ID: &str = "atelier-ckc-album-status";
pub const ATELIER_CKC_ALBUM_CREATE_NAME_AUTHOR_ID: &str = "atelier-ckc-album-create-name";
pub const ATELIER_CKC_ALBUM_CREATE_NOTES_AUTHOR_ID: &str = "atelier-ckc-album-create-notes";
pub const ATELIER_CKC_ALBUM_CREATE_TAGS_AUTHOR_ID: &str = "atelier-ckc-album-create-tags";
pub const ATELIER_CKC_ALBUM_CREATE_AUTHOR_ID: &str = "atelier-ckc-album-create";
pub const ATELIER_CKC_ALBUM_LINK_ASSET_IDS_AUTHOR_ID: &str = "atelier-ckc-album-link-asset-ids";
pub const ATELIER_CKC_ALBUM_LINK_SOURCE_PATH_AUTHOR_ID: &str = "atelier-ckc-album-link-source-path";
pub const ATELIER_CKC_ALBUM_LINK_SOURCE_URL_AUTHOR_ID: &str = "atelier-ckc-album-link-source-url";
pub const ATELIER_CKC_ALBUM_LINK_AUTHOR_ID: &str = "atelier-ckc-album-link-assets";
pub const ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID: &str = "atelier-ckc-media-notes-editor";
pub const ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID: &str = "atelier-ckc-media-tags-editor";
pub const ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID: &str = "atelier-ckc-media-save";
pub const ATELIER_CKC_STORY_DOC_REF_AUTHOR_ID: &str = "atelier-ckc-story-doc-ref";
pub const ATELIER_CKC_STORY_EDITOR_AUTHOR_ID: &str = "atelier-ckc-story-editor";
pub const ATELIER_CKC_STORY_SAVE_AUTHOR_ID: &str = "atelier-ckc-story-save";
pub const ATELIER_CKC_STORY_CARD_LIST_AUTHOR_ID: &str = "atelier-ckc-story-card-list";
pub const ATELIER_CKC_STORY_CARD_TITLE_AUTHOR_ID: &str = "atelier-ckc-story-card-title";
pub const ATELIER_CKC_STORY_CARD_BODY_AUTHOR_ID: &str = "atelier-ckc-story-card-body";
pub const ATELIER_CKC_STORY_CARD_SAVE_AUTHOR_ID: &str = "atelier-ckc-story-card-save";
pub const ATELIER_CKC_STORY_BEAT_EDITOR_AUTHOR_ID: &str = "atelier-ckc-story-beat-editor";
pub const ATELIER_CKC_STORY_BEAT_SAVE_AUTHOR_ID: &str = "atelier-ckc-story-beat-save";
pub const ATELIER_CKC_MOODBOARD_DOC_REF_AUTHOR_ID: &str = "atelier-ckc-moodboard-doc-ref";
pub const ATELIER_CKC_MOODBOARD_LATEST_REF_AUTHOR_ID: &str = "atelier-ckc-moodboard-latest-ref";
pub const ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID: &str = "atelier-ckc-moodboard-editor";
pub const ATELIER_CKC_MOODBOARD_SAVE_AUTHOR_ID: &str = "atelier-ckc-moodboard-save";
pub const ATELIER_CKC_MOODBOARD_OPEN_AUTHOR_ID: &str = "atelier-ckc-moodboard-open";
pub const ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID: &str = "atelier-ckc-moodboard-canvas";
pub const ATELIER_CKC_BOOK_LAYOUT_AUTHOR_ID: &str = "atelier-ckc-book-layout";
pub const ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID: &str = "atelier-ckc-book-left-media";
pub const ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID: &str = "atelier-ckc-book-middle";
pub const ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID: &str = "atelier-ckc-book-right-sheet";
pub const ATELIER_CKC_MEDIA_VIEWER_AUTHOR_ID: &str = "atelier-ckc-media-viewer";
pub const ATELIER_CKC_MODE_SHEET_AUTHOR_ID: &str = "atelier-ckc-mode-sheet";
pub const ATELIER_CKC_MODE_STORY_AUTHOR_ID: &str = "atelier-ckc-mode-story";
pub const ATELIER_CKC_MODE_NOTES_AUTHOR_ID: &str = "atelier-ckc-mode-notes";
pub const ATELIER_CKC_MODE_MOODBOARD_AUTHOR_ID: &str = "atelier-ckc-mode-moodboard";
pub const ATELIER_CKC_CHARACTER_NOTES_EDITOR_AUTHOR_ID: &str = "atelier-ckc-character-notes-editor";
pub const ATELIER_CKC_CHARACTER_NOTES_APPLY_AUTHOR_ID: &str = "atelier-ckc-character-notes-apply";
pub const ATELIER_CKC_SEARCH_QUERY_AUTHOR_ID: &str = "atelier-ckc-search-query";
pub const ATELIER_CKC_SEARCH_TAGS_AUTHOR_ID: &str = "atelier-ckc-search-tags";
pub const ATELIER_CKC_SEARCH_FILTER_CHARACTER_AUTHOR_ID: &str =
    "atelier-ckc-search-filter-character";
pub const ATELIER_CKC_SEARCH_FILTER_COLLECTION_AUTHOR_ID: &str =
    "atelier-ckc-search-filter-collection";
pub const ATELIER_CKC_SEARCH_FILTER_MEDIA_AUTHOR_ID: &str = "atelier-ckc-search-filter-media";
pub const ATELIER_CKC_SEARCH_FILTER_SIMILARITY_AUTHOR_ID: &str =
    "atelier-ckc-search-filter-similarity";
pub const ATELIER_CKC_SEARCH_MODE_FUZZY_AUTHOR_ID: &str = "atelier-ckc-search-mode-fuzzy";
pub const ATELIER_CKC_SEARCH_MODE_VECTOR_AUTHOR_ID: &str = "atelier-ckc-search-mode-vector";
pub const ATELIER_CKC_SEARCH_MODE_COMBINED_AUTHOR_ID: &str = "atelier-ckc-search-mode-combined";
pub const ATELIER_CKC_SEARCH_RUN_AUTHOR_ID: &str = "atelier-ckc-search-run";
pub const ATELIER_CKC_SEARCH_STATUS_AUTHOR_ID: &str = "atelier-ckc-search-status";
pub const ATELIER_CKC_SEARCH_RESULTS_AUTHOR_ID: &str = "atelier-ckc-search-results";
pub const ATELIER_CKC_TAG_NOTE_TAG_AUTHOR_ID: &str = "atelier-ckc-tag-note-tag";
pub const ATELIER_CKC_TAG_NOTE_SCOPE_AUTHOR_ID: &str = "atelier-ckc-tag-note-scope";
pub const ATELIER_CKC_TAG_NOTE_EDITOR_AUTHOR_ID: &str = "atelier-ckc-tag-note-editor";
pub const ATELIER_CKC_TAG_NOTE_SAVE_AUTHOR_ID: &str = "atelier-ckc-tag-note-save";
pub const ATELIER_POSE_YAW_MINUS_AUTHOR_ID: &str = "atelier-pose-yaw-minus";
pub const ATELIER_POSE_YAW_PLUS_AUTHOR_ID: &str = "atelier-pose-yaw-plus";
pub const ATELIER_POSE_RESET_AUTHOR_ID: &str = "atelier-pose-reset";
pub const ATELIER_POSE_FACE_TOGGLE_AUTHOR_ID: &str = "atelier-pose-face-toggle";
pub const ATELIER_POSE_BODY_TOGGLE_AUTHOR_ID: &str = "atelier-pose-body-toggle";
pub const ATELIER_POSE_HANDS_TOGGLE_AUTHOR_ID: &str = "atelier-pose-hands-toggle";
pub const ATELIER_POSE_YAW_SLIDER_AUTHOR_ID: &str = "atelier-pose-yaw-slider";
pub const ATELIER_POSE_PITCH_SLIDER_AUTHOR_ID: &str = "atelier-pose-pitch-slider";
pub const ATELIER_POSE_ZOOM_SLIDER_AUTHOR_ID: &str = "atelier-pose-zoom-slider";
pub const ATELIER_POSE_MARKER_FAMILY_AUTHOR_ID: &str = "atelier-pose-marker-family";
pub const ATELIER_POSE_MARKER_INDEX_AUTHOR_ID: &str = "atelier-pose-marker-index";
pub const ATELIER_POSE_MARKER_X_AUTHOR_ID: &str = "atelier-pose-marker-x";
pub const ATELIER_POSE_MARKER_Y_AUTHOR_ID: &str = "atelier-pose-marker-y";
pub const ATELIER_POSE_MARKER_CONFIDENCE_AUTHOR_ID: &str = "atelier-pose-marker-confidence";
pub const ATELIER_POSE_MARKER_APPLY_AUTHOR_ID: &str = "atelier-pose-marker-apply";
pub const ATELIER_POSE_MARKER_ADD_AUTHOR_ID: &str = "atelier-pose-marker-add";
pub const ATELIER_POSE_MARKER_REMOVE_AUTHOR_ID: &str = "atelier-pose-marker-remove";
pub const ATELIER_POSE_MARKER_RESET_AUTHOR_ID: &str = "atelier-pose-marker-reset";
pub const ATELIER_POSE_MARKER_NUDGE_LEFT_AUTHOR_ID: &str = "atelier-pose-marker-nudge-left";
pub const ATELIER_POSE_MARKER_NUDGE_RIGHT_AUTHOR_ID: &str = "atelier-pose-marker-nudge-right";
pub const ATELIER_POSE_MARKER_NUDGE_UP_AUTHOR_ID: &str = "atelier-pose-marker-nudge-up";
pub const ATELIER_POSE_MARKER_NUDGE_DOWN_AUTHOR_ID: &str = "atelier-pose-marker-nudge-down";
pub const ATELIER_POSE_MARKER_STATUS_AUTHOR_ID: &str = "atelier-pose-marker-status";
pub const ATELIER_POSE_FRAMING_PRESET_AUTHOR_ID: &str = "atelier-pose-framing-preset";
pub const ATELIER_POSE_FRAMING_LENS_AUTHOR_ID: &str = "atelier-pose-framing-lens";
pub const ATELIER_POSE_FRAMING_PADDING_TOP_AUTHOR_ID: &str = "atelier-pose-framing-padding-top";
pub const ATELIER_POSE_FRAMING_PADDING_RIGHT_AUTHOR_ID: &str = "atelier-pose-framing-padding-right";
pub const ATELIER_POSE_FRAMING_PADDING_BOTTOM_AUTHOR_ID: &str =
    "atelier-pose-framing-padding-bottom";
pub const ATELIER_POSE_FRAMING_PADDING_LEFT_AUTHOR_ID: &str = "atelier-pose-framing-padding-left";
pub const ATELIER_POSE_FRAMING_READOUT_AUTHOR_ID: &str = "atelier-pose-framing-readout";
pub const ATELIER_POSE_SOURCE_REF_AUTHOR_ID: &str = "atelier-pose-source-ref";
pub const ATELIER_POSE_RIG_ID_AUTHOR_ID: &str = "atelier-pose-rig-id";
pub const ATELIER_POSE_STATE_READOUT_AUTHOR_ID: &str = "atelier-pose-state-readout";
pub const ATELIER_POSE_SPLIT_VIEW_AUTHOR_ID: &str = "atelier-pose-split-view";
pub const ATELIER_POSE_3D_VIEWPORT_AUTHOR_ID: &str = "atelier-pose-3d-viewport";
pub const ATELIER_POSE_OPENPOSE_VIEWPORT_AUTHOR_ID: &str = "atelier-pose-openpose-viewport";
pub const ATELIER_POSE_EXPORT_AUTHOR_ID: &str = "atelier-pose-export-openpose";
pub const ATELIER_POSE_EXPORT_STATUS_AUTHOR_ID: &str = "atelier-pose-export-status";
pub const ATELIER_POSE_EXPORT_REF_AUTHOR_ID: &str = "atelier-pose-export-ref";
pub const ATELIER_POSE_EXPORT_PREVIEW_AUTHOR_ID: &str = "atelier-pose-export-preview";
pub const ATELIER_INGEST_PASS_AUTHOR_ID: &str = "atelier-ingest-pass";
pub const ATELIER_INGEST_REJECT_AUTHOR_ID: &str = "atelier-ingest-reject";
pub const ATELIER_INGEST_UNSURE_AUTHOR_ID: &str = "atelier-ingest-unsure";
pub const ATELIER_INGEST_BATCH_TAGS_AUTHOR_ID: &str = "atelier-ingest-batch-tags";
pub const ATELIER_INGEST_DATASET_REF_AUTHOR_ID: &str = "atelier-ingest-dataset-ref";
pub const ATELIER_INGEST_CHARACTER_REF_AUTHOR_ID: &str = "atelier-ingest-character-ref";
pub const ATELIER_INGEST_BATCH_NOTE_AUTHOR_ID: &str = "atelier-ingest-batch-note";
pub const ATELIER_INGEST_EVENT_AUTHOR_ID: &str = "atelier-ingest-event";
pub const ATELIER_INGEST_DATE_AUTHOR_ID: &str = "atelier-ingest-date";
pub const ATELIER_INGEST_LOCATION_AUTHOR_ID: &str = "atelier-ingest-location";
pub const ATELIER_INGEST_LINK_PASSED_AUTHOR_ID: &str = "atelier-ingest-link-passed";
pub const ATELIER_INGEST_APPLY_BATCH_AUTHOR_ID: &str = "atelier-ingest-apply-batch";
pub const ATELIER_INGEST_CONTACT_ROWS_AUTHOR_ID: &str = "atelier-ingest-contact-rows";
pub const ATELIER_INGEST_CONTACT_COLUMNS_AUTHOR_ID: &str = "atelier-ingest-contact-columns";
pub const ATELIER_INGEST_CONTACT_DPI_AUTHOR_ID: &str = "atelier-ingest-contact-dpi";
pub const ATELIER_INGEST_CONTACT_EXPORT_AUTHOR_ID: &str = "atelier-ingest-contact-export";
pub const ATELIER_INGEST_FACIAL_PROFILE_AUTHOR_ID: &str = "atelier-ingest-facial-profile";
pub const ATELIER_INGEST_QUEUE_READOUT_AUTHOR_ID: &str = "atelier-ingest-queue-readout";
pub const ATELIER_INGEST_STATUS_AUTHOR_ID: &str = "atelier-ingest-status";
pub const ATELIER_INGEST_LAST_RECEIPT_AUTHOR_ID: &str = "atelier-ingest-last-receipt";

pub fn ingest_item_row_author_id(item_id: &str) -> String {
    format!(
        "atelier-ingest-item-{}",
        crate::project_tree::stable_part(item_id)
    )
}

pub fn ingest_item_pass_author_id(item_id: &str) -> String {
    format!(
        "atelier-ingest-item-{}-pass",
        crate::project_tree::stable_part(item_id)
    )
}

pub fn ingest_item_reject_author_id(item_id: &str) -> String {
    format!(
        "atelier-ingest-item-{}-reject",
        crate::project_tree::stable_part(item_id)
    )
}

pub fn ingest_item_unsure_author_id(item_id: &str) -> String {
    format!(
        "atelier-ingest-item-{}-unsure",
        crate::project_tree::stable_part(item_id)
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtelierPanelTab {
    CastkitCodex,
    Posekit,
    Ingest,
}

impl AtelierPanelTab {
    pub const ALL: [Self; 3] = [Self::CastkitCodex, Self::Posekit, Self::Ingest];

    fn label(self) -> &'static str {
        match self {
            Self::CastkitCodex => "Castkit Codex",
            Self::Posekit => "Posekit",
            Self::Ingest => "Ingest",
        }
    }

    fn tab_author_id(self) -> &'static str {
        match self {
            Self::CastkitCodex => ATELIER_TAB_CKC_AUTHOR_ID,
            Self::Posekit => ATELIER_TAB_POSEKIT_AUTHOR_ID,
            Self::Ingest => ATELIER_TAB_INGEST_AUTHOR_ID,
        }
    }

    fn content_author_id(self) -> &'static str {
        match self {
            Self::CastkitCodex => ATELIER_CONTENT_CKC_AUTHOR_ID,
            Self::Posekit => ATELIER_CONTENT_POSEKIT_AUTHOR_ID,
            Self::Ingest => ATELIER_CONTENT_INGEST_AUTHOR_ID,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CkcBookMode {
    Sheet,
    Story,
    Notes,
    Moodboard,
}

impl CkcBookMode {
    const ALL: [Self; 4] = [Self::Sheet, Self::Story, Self::Notes, Self::Moodboard];

    fn label(self) -> &'static str {
        match self {
            Self::Sheet => "Sheet",
            Self::Story => "Story",
            Self::Notes => "Notes",
            Self::Moodboard => "Moodboard",
        }
    }

    fn author_id(self) -> &'static str {
        match self {
            Self::Sheet => ATELIER_CKC_MODE_SHEET_AUTHOR_ID,
            Self::Story => ATELIER_CKC_MODE_STORY_AUTHOR_ID,
            Self::Notes => ATELIER_CKC_MODE_NOTES_AUTHOR_ID,
            Self::Moodboard => ATELIER_CKC_MODE_MOODBOARD_AUTHOR_ID,
        }
    }

    fn middle_label(self) -> &'static str {
        match self {
            Self::Sheet => "No middle panel",
            Self::Story => "Story work surface",
            Self::Notes => "Character sheet notes",
            Self::Moodboard => "Moodboard work surface",
        }
    }

    fn has_middle_panel(self) -> bool {
        !matches!(self, Self::Sheet)
    }
}

const POSEKIT_EXPORT_WIDTH: i32 = 768;
const POSEKIT_EXPORT_HEIGHT: i32 = 768;
const POSEKIT_BODY_KEYPOINT_COUNT: usize = 18;
const POSEKIT_FACE_KEYPOINT_COUNT: usize = 70;
const POSEKIT_HAND_KEYPOINT_COUNT: usize = 21;

#[derive(Debug, Clone)]
struct PosekitExportSnapshot {
    source_ref: String,
    rig_id: Option<String>,
    yaw_deg: f32,
    pitch_deg: f32,
    zoom: f32,
    face: bool,
    body: bool,
    hands: bool,
    png_artifact_ref: String,
    png_manifest_ref: String,
    json_artifact_ref: String,
    json_manifest_ref: String,
    receipt_ref: String,
    content_hash: String,
    openpose_json: serde_json::Value,
    framing: serde_json::Value,
    applied_marker_edit_count: usize,
}

impl PosekitExportSnapshot {
    fn marker_layers(&self) -> String {
        marker_layer_summary(self.face, self.body, self.hands)
    }
}

#[derive(Debug, Clone)]
struct PosekitMarkerEditRecord {
    family: String,
    index: usize,
    action: String,
    x: Option<f32>,
    y: Option<f32>,
    confidence: Option<f32>,
}

#[derive(Debug, Clone)]
struct CkcCharacterRecord {
    public_id: String,
    display_name: String,
    character_internal_id: String,
    character_ref: String,
    sheet_version_id: Option<String>,
    parent_sheet_version_id: Option<String>,
    sheet_seq: i64,
    sheet_editor_text: String,
    sheet_version_ref: Option<String>,
    sheet_artifact_links: Vec<CkcSheetArtifactLinkRecord>,
    media_albums: Vec<CkcMediaAlbumRecord>,
    story_documents: Vec<CkcStoryDocumentRecord>,
    moodboard_documents: Vec<CkcMoodboardDocumentRecord>,
}

#[derive(Debug, Clone)]
struct CkcSheetArtifactLinkRecord {
    link_id: String,
    character_internal_id: String,
    character_ref: String,
    sheet_version_id: String,
    sheet_version_ref: String,
    typed_ref: String,
    artifact_kind: String,
    artifact_ref: String,
    manifest_ref: Option<String>,
    source_ref: Option<String>,
    label: Option<String>,
    reuse_role: Option<String>,
    linked_by: String,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
struct CkcMediaAlbumRecord {
    collection_id: String,
    collection_ref: String,
    name: String,
    description: String,
    tags: Vec<String>,
    member_count: usize,
    members_next_offset: Option<i64>,
    members: Vec<CkcMediaMemberRecord>,
}

#[derive(Debug, Clone)]
struct CkcMediaMemberRecord {
    asset_id: String,
    media_ref: String,
    display_label: String,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
    notes: String,
    review_status: Option<String>,
    tags_buffer: String,
}

#[derive(Debug, Clone)]
struct CkcStoryDocumentRecord {
    document_id: String,
    document_ref: String,
    current_version_id: Option<String>,
    current_version_seq: i64,
    title: String,
    body_raw_text: String,
    tags: Vec<String>,
    cards: Vec<CkcStoryCardRecord>,
    beats: Vec<CkcStoryBeatRecord>,
}

#[derive(Debug, Clone)]
struct CkcStoryCardRecord {
    card_id: String,
    card_ref: String,
    story_document_id: String,
    story_document_ref: String,
    title: String,
    body_raw_text: String,
    tags: Vec<String>,
}

#[derive(Debug, Clone)]
struct CkcStoryBeatRecord {
    beat_id: String,
    beat_ref: String,
    story_document_id: String,
    story_document_ref: String,
    card_id: Option<String>,
    beat_text: String,
}

#[derive(Debug, Clone)]
struct CkcMoodboardDocumentRecord {
    document_id: String,
    document_ref: String,
    current_version_id: Option<String>,
    current_version_seq: i64,
    title: String,
    body_raw_text: String,
    tags: Vec<String>,
    latest_snapshot_id: Option<String>,
    latest_snapshot_ref: Option<String>,
    moodboard_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CkcMoodboardCanvasProjection {
    pub placements: Vec<CanvasPlacementCard>,
    pub visual_edges: Vec<VisualEdge>,
    pub section_labels: BTreeMap<String, String>,
    pub pan: egui::Vec2,
    pub zoom: f32,
}

impl CkcMoodboardCanvasProjection {
    fn apply_to_board(self, board: &mut LoomCanvasBoard, snapshot_ref: &str) {
        board.set_section_labels(self.section_labels);
        board.set_board(self.placements, self.visual_edges, self.pan, self.zoom);
        board.status = format!("CKC moodboard snapshot loaded: {snapshot_ref}");
    }
}

fn apply_ckc_moodboard_snapshot_to_board(
    canvas_board: &Arc<Mutex<LoomCanvasBoard>>,
    raw_json_text: &str,
    snapshot_ref: &str,
) -> Result<(), String> {
    let projection = ckc_moodboard_snapshot_to_canvas_projection(raw_json_text)?;
    let mut board = canvas_board
        .lock()
        .map_err(|err| format!("CKC moodboard canvas lock failed: {err}"))?;
    projection.apply_to_board(&mut board, snapshot_ref);
    Ok(())
}

#[derive(Debug, Clone)]
struct CkcMediaSaveRequest {
    asset_id: String,
    notes: String,
    tags: Vec<String>,
    review_status: Option<String>,
}

#[derive(Debug, Clone)]
struct CkcAlbumCreateRequest {
    character_internal_id: String,
    name: String,
    notes: Option<String>,
    sheet_version_id: Option<String>,
    tags: Vec<String>,
}

#[derive(Debug, Clone)]
struct CkcAlbumLinkAssetsRequest {
    collection_id: String,
    asset_ids: Vec<String>,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
}

#[derive(Debug, Clone)]
struct CkcAlbumPageRequest {
    collection_id: String,
    offset: i64,
}

#[derive(Debug, Clone)]
struct CkcSheetArtifactAttachRequest {
    sheet_version_id: String,
    artifact_kind: String,
    artifact_ref: String,
    manifest_ref: Option<String>,
    source_ref: Option<String>,
    label: Option<String>,
    reuse_role: Option<String>,
    metadata: serde_json::Value,
    actor_id: String,
}

#[derive(Debug, Clone)]
struct CkcSheetArtifactDetachRequest {
    sheet_version_id: String,
    link_id: String,
    actor_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CkcSearchMode {
    Fuzzy,
    Vector,
    Combined,
}

impl CkcSearchMode {
    const ALL: [Self; 3] = [Self::Fuzzy, Self::Vector, Self::Combined];

    fn label(self) -> &'static str {
        match self {
            Self::Fuzzy => "Fuzzy",
            Self::Vector => "Vector",
            Self::Combined => "Combined",
        }
    }

    fn backend_value(self) -> &'static str {
        match self {
            Self::Fuzzy => "fuzzy",
            Self::Vector => "vector",
            Self::Combined => "combined",
        }
    }

    fn author_id(self) -> &'static str {
        match self {
            Self::Fuzzy => ATELIER_CKC_SEARCH_MODE_FUZZY_AUTHOR_ID,
            Self::Vector => ATELIER_CKC_SEARCH_MODE_VECTOR_AUTHOR_ID,
            Self::Combined => ATELIER_CKC_SEARCH_MODE_COMBINED_AUTHOR_ID,
        }
    }
}

#[derive(Debug, Clone)]
struct CkcSearchResultRecord {
    target_kind: String,
    target_ref: String,
    title: String,
    snippet: String,
    character_ref: Option<String>,
    sheet_version_ref: Option<String>,
    collection_ref: Option<String>,
    media_ref: Option<String>,
    tag_ref: Option<String>,
    tags: Vec<String>,
    tag_notes: Vec<CkcTagNoteRecord>,
    match_modes: Vec<String>,
}

#[derive(Debug, Clone)]
struct CkcTagNoteRecord {
    tag_ref: String,
    tag_text: String,
    scope_ref: Option<String>,
    note: String,
}

#[derive(Debug, Clone)]
struct CkcTagNoteSaveRequest {
    tag_text: String,
    scope_ref: Option<String>,
    note: String,
}

#[derive(Debug, Clone, Default)]
struct CkcSearchFilterRefs {
    character_internal_id: Option<String>,
    character_ref: Option<String>,
    collection_id: Option<String>,
    collection_ref: Option<String>,
    media_asset_id: Option<String>,
    media_ref: Option<String>,
}

fn selected_ckc_search_filter_refs(state: &AtelierPanelState) -> CkcSearchFilterRefs {
    let Some(character) = state.ckc_characters.get(
        state
            .ckc_selected_index
            .min(state.ckc_characters.len().saturating_sub(1)),
    ) else {
        return CkcSearchFilterRefs::default();
    };
    let mut refs = CkcSearchFilterRefs {
        character_internal_id: Some(character.character_internal_id.clone()),
        character_ref: Some(character.character_ref()),
        ..Default::default()
    };
    let media_location = state
        .ckc_selected_media_key
        .as_deref()
        .and_then(|media_key| character.media_location(media_key))
        .or_else(|| character.first_media_location());
    if let Some((album_idx, member_idx)) = media_location {
        let album = &character.media_albums[album_idx];
        let member = &album.members[member_idx];
        refs.collection_id = Some(album.collection_id.clone());
        refs.collection_ref = Some(album.collection_ref.clone());
        refs.media_asset_id = Some(member.asset_id.clone());
        refs.media_ref = Some(member.media_ref.clone());
    }
    refs
}

fn ckc_result_has_ref(result: &CkcSearchResultRecord, expected: Option<&str>) -> bool {
    let Some(expected) = expected else {
        return false;
    };
    [
        Some(result.target_ref.as_str()),
        result.character_ref.as_deref(),
        result.sheet_version_ref.as_deref(),
        result.collection_ref.as_deref(),
        result.media_ref.as_deref(),
        result.tag_ref.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|actual| actual == expected)
}

fn ckc_scope_matches_result_refs(
    scope_ref: Option<&str>,
    target_ref: &str,
    character_ref: Option<&str>,
    sheet_version_ref: Option<&str>,
    collection_ref: Option<&str>,
    media_ref: Option<&str>,
    tag_ref: Option<&str>,
) -> bool {
    let Some(scope_ref) = scope_ref else {
        return true;
    };
    [
        Some(target_ref),
        character_ref,
        sheet_version_ref,
        collection_ref,
        media_ref,
        tag_ref,
    ]
    .into_iter()
    .flatten()
    .any(|actual| actual == scope_ref)
}

fn ckc_search_result_matches_filters(
    result: &CkcSearchResultRecord,
    filters: &CkcSearchFilterRefs,
    use_character: bool,
    use_collection: bool,
    use_media: bool,
) -> bool {
    (!use_character || ckc_result_has_ref(result, filters.character_ref.as_deref()))
        && (!use_collection || ckc_result_has_ref(result, filters.collection_ref.as_deref()))
        && (!use_media || ckc_result_has_ref(result, filters.media_ref.as_deref()))
}

impl CkcCharacterRecord {
    fn character_ref(&self) -> String {
        if self.character_ref.is_empty() {
            format!("atelier://character/{}", self.character_internal_id)
        } else {
            self.character_ref.clone()
        }
    }

    fn sheet_version_ref(&self) -> Option<String> {
        self.sheet_version_ref.clone().or_else(|| {
            self.sheet_version_id.as_ref().map(|version_id| {
                format!(
                    "atelier://sheet/{}/{}",
                    self.character_internal_id, version_id
                )
            })
        })
    }

    fn sheet_atelier_ref(&self) -> Option<AtelierRef> {
        self.sheet_version_id.as_ref().map(|sheet_version_id| {
            AtelierRef::character_sheet_version(
                &self.character_internal_id,
                sheet_version_id,
                format!("{} sheet v{}", self.display_name, self.sheet_seq),
            )
        })
    }

    fn from_backend(row: AtelierCkcCharacterSheetRow) -> Self {
        let AtelierCkcCharacterSheetRow {
            character,
            latest_sheet,
            sheet_artifact_links,
            media_albums,
            story_documents,
            moodboard_documents,
            moodboard_snapshots,
        } = row;
        let (
            sheet_version_id,
            parent_sheet_version_id,
            sheet_seq,
            sheet_editor_text,
            sheet_version_ref,
        ) = latest_sheet
            .map(
                |AtelierSheetVersionRow {
                     version_id,
                     parent_version_id,
                     seq,
                     raw_text,
                     sheet_version_ref,
                     ..
                 }| {
                    (
                        Some(version_id),
                        parent_version_id,
                        seq,
                        raw_text,
                        Some(sheet_version_ref),
                    )
                },
            )
            .unwrap_or_else(|| (None, None, 0, String::new(), None));
        let story_documents = story_documents
            .into_iter()
            .map(CkcStoryDocumentRecord::from_backend)
            .collect();
        let moodboard_documents = moodboard_documents
            .into_iter()
            .map(|document| {
                let snapshot = moodboard_snapshots
                    .iter()
                    .find(|snapshot| snapshot.document_id == document.document_id);
                CkcMoodboardDocumentRecord::from_backend(document, snapshot)
            })
            .collect();
        Self {
            public_id: character.public_id,
            display_name: character.display_name,
            character_internal_id: character.internal_id,
            character_ref: character.character_ref,
            sheet_version_id,
            parent_sheet_version_id,
            sheet_seq,
            sheet_editor_text,
            sheet_version_ref,
            sheet_artifact_links: sheet_artifact_links
                .into_iter()
                .map(CkcSheetArtifactLinkRecord::from_backend)
                .collect(),
            media_albums: media_albums
                .into_iter()
                .map(CkcMediaAlbumRecord::from_backend)
                .collect(),
            story_documents,
            moodboard_documents,
        }
    }

    fn from_created_character(character: AtelierCharacterRow) -> Self {
        Self {
            public_id: character.public_id,
            display_name: character.display_name.clone(),
            character_internal_id: character.internal_id,
            character_ref: character.character_ref,
            sheet_version_id: None,
            parent_sheet_version_id: None,
            sheet_seq: 0,
            sheet_editor_text: format!(
                "name: {}\nrole: reusable character/avatar\npipelines: ComfyUI, Unreal, Blender\nnotes: ",
                character.display_name
            ),
            sheet_version_ref: None,
            sheet_artifact_links: Vec::new(),
            media_albums: Vec::new(),
            story_documents: Vec::new(),
            moodboard_documents: Vec::new(),
        }
    }

    fn apply_sheet_version(&mut self, sheet: AtelierSheetVersionRow) {
        self.character_internal_id = sheet.character_internal_id;
        self.character_ref = sheet.character_ref;
        self.parent_sheet_version_id = sheet.parent_version_id;
        self.sheet_version_id = Some(sheet.version_id);
        self.sheet_seq = sheet.seq;
        self.sheet_editor_text = sheet.raw_text;
        self.sheet_version_ref = Some(sheet.sheet_version_ref);
        self.sheet_artifact_links.clear();
    }

    fn first_media_location(&self) -> Option<(usize, usize)> {
        self.media_albums
            .iter()
            .enumerate()
            .find_map(|(album_idx, album)| {
                if album.members.is_empty() {
                    None
                } else {
                    Some((album_idx, 0))
                }
            })
    }

    fn media_location(&self, media_key: &str) -> Option<(usize, usize)> {
        self.media_albums
            .iter()
            .enumerate()
            .find_map(|(album_idx, album)| {
                album
                    .members
                    .iter()
                    .position(|member| {
                        ckc_media_occurrence_key(&album.collection_id, &member.asset_id)
                            == media_key
                    })
                    .map(|member_idx| (album_idx, member_idx))
            })
    }

    fn selected_or_first_media_location(
        &self,
        selected_media_key: Option<&str>,
    ) -> Option<(usize, usize)> {
        match selected_media_key {
            Some(media_key) => self.media_location(media_key),
            None => self.first_media_location(),
        }
    }
}

impl CkcSheetArtifactLinkRecord {
    fn from_backend(row: AtelierCkcSheetArtifactLinkRow) -> Self {
        Self {
            link_id: row.link_id,
            character_internal_id: row.character_internal_id,
            character_ref: row.character_ref,
            sheet_version_id: row.sheet_version_id,
            sheet_version_ref: row.sheet_version_ref,
            typed_ref: row.typed_ref,
            artifact_kind: row.artifact_kind,
            artifact_ref: row.artifact_ref,
            manifest_ref: row.manifest_ref,
            source_ref: row.source_ref,
            label: row.label,
            reuse_role: row.reuse_role,
            linked_by: row.linked_by,
            metadata: row.metadata,
        }
    }

    fn local(
        character: &CkcCharacterRecord,
        artifact_kind: String,
        artifact_ref: String,
        manifest_ref: Option<String>,
        source_ref: Option<String>,
        label: Option<String>,
        reuse_role: Option<String>,
        metadata: serde_json::Value,
        actor_id: String,
    ) -> Option<Self> {
        let sheet_version_id = character.sheet_version_id.clone()?;
        let sheet_version_ref = character.sheet_version_ref()?;
        let link_id = Uuid::new_v4().to_string();
        Some(Self {
            link_id: link_id.clone(),
            character_internal_id: character.character_internal_id.clone(),
            character_ref: character.character_ref(),
            sheet_version_id,
            sheet_version_ref,
            typed_ref: format!("atelier://sheet-artifact/{link_id}"),
            artifact_kind,
            artifact_ref,
            manifest_ref,
            source_ref,
            label,
            reuse_role,
            linked_by: actor_id,
            metadata,
        })
    }

    fn summary(&self) -> String {
        format!(
            "{} | {} | {} | {}",
            self.artifact_kind,
            self.reuse_role.as_deref().unwrap_or("reuse"),
            self.typed_ref,
            self.artifact_ref
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CkcSheetArtifactApplyOutcome {
    count: usize,
    current_selection_owns_target: bool,
    target_found: bool,
}

fn apply_ckc_sheet_artifact_link_rows_to_state(
    state: &mut AtelierPanelState,
    target_sheet_version_id: &str,
    rows: Vec<AtelierCkcSheetArtifactLinkRow>,
) -> CkcSheetArtifactApplyOutcome {
    let selected_index = state.ckc_selected_index;
    let current_selection_owns_target = state
        .ckc_characters
        .get(selected_index)
        .and_then(|character| character.sheet_version_id.as_deref())
        == Some(target_sheet_version_id);
    let selected_link_id = state.ckc_selected_sheet_artifact_link_id.clone();
    let records: Vec<CkcSheetArtifactLinkRecord> = rows
        .into_iter()
        .map(CkcSheetArtifactLinkRecord::from_backend)
        .collect();
    let count = records.len();
    let mut next_selected_link_id = None;
    let mut next_reuse_ref = String::new();
    let target_index = state.ckc_characters.iter().position(|character| {
        character.sheet_version_id.as_deref() == Some(target_sheet_version_id)
    });
    if let Some(target_index) = target_index {
        let character = &mut state.ckc_characters[target_index];
        character.sheet_artifact_links = records;
        if current_selection_owns_target {
            next_selected_link_id = if let Some(selected_link_id) = selected_link_id {
                if character
                    .sheet_artifact_links
                    .iter()
                    .any(|link| link.link_id == selected_link_id)
                {
                    Some(selected_link_id)
                } else {
                    character
                        .sheet_artifact_links
                        .first()
                        .map(|link| link.link_id.clone())
                }
            } else {
                character
                    .sheet_artifact_links
                    .first()
                    .map(|link| link.link_id.clone())
            };
            next_reuse_ref = next_selected_link_id
                .as_ref()
                .and_then(|link_id| {
                    character
                        .sheet_artifact_links
                        .iter()
                        .find(|link| &link.link_id == link_id)
                })
                .map(|link| link.typed_ref.clone())
                .unwrap_or_default();
        }
    }
    if current_selection_owns_target {
        state.ckc_selected_sheet_artifact_link_id = next_selected_link_id;
        state.ckc_sheet_artifact_reuse_ref = next_reuse_ref;
    }
    CkcSheetArtifactApplyOutcome {
        count,
        current_selection_owns_target,
        target_found: target_index.is_some(),
    }
}

impl CkcMediaAlbumRecord {
    fn from_backend(row: AtelierCkcMediaAlbumRow) -> Self {
        Self {
            collection_id: row.collection_id,
            collection_ref: row.collection_ref,
            name: row.name,
            description: row.description.unwrap_or_default(),
            tags: row.tags,
            member_count: row.member_count,
            members_next_offset: row.members_next_offset,
            members: row
                .members
                .into_iter()
                .map(CkcMediaMemberRecord::from_backend)
                .collect(),
        }
    }
}

impl CkcMediaMemberRecord {
    fn from_backend(row: AtelierCkcMediaMemberRow) -> Self {
        Self {
            asset_id: row.asset_id,
            media_ref: row.media_ref,
            display_label: row.file_name,
            source_path_ref: row.source_path_ref,
            source_url_ref: row.source_url_ref,
            notes: row.notes.unwrap_or_default(),
            review_status: row.review_status,
            tags_buffer: row.tags.join(", "),
        }
    }

    fn apply_notes_tags(&mut self, row: &AtelierCkcMediaNotesTagsRow) {
        self.media_ref = row.media_ref.clone();
        self.notes = row.notes.clone().unwrap_or_default();
        self.review_status = row.review_status.clone();
        self.tags_buffer = row.tags.join(", ");
    }
}

impl CkcStoryDocumentRecord {
    fn from_backend(row: AtelierCkcCharacterDocumentRow) -> Self {
        let body_raw_text = row
            .current_version
            .as_ref()
            .map(|version| version.body_raw_text.clone())
            .unwrap_or_default();
        let cards = row
            .story_cards
            .into_iter()
            .map(CkcStoryCardRecord::from_backend)
            .collect();
        let beats = row
            .story_beats
            .into_iter()
            .map(CkcStoryBeatRecord::from_backend)
            .collect();
        Self {
            document_id: row.document_id,
            document_ref: row.document_ref,
            current_version_id: Some(row.current_version_id),
            current_version_seq: row.current_version_seq,
            title: row.title,
            body_raw_text,
            tags: row.tags,
            cards,
            beats,
        }
    }
}

impl CkcStoryCardRecord {
    fn from_backend(row: AtelierCkcStoryCardRow) -> Self {
        Self {
            card_id: row.card_id,
            card_ref: row.card_ref,
            story_document_id: row.story_document_id,
            story_document_ref: row.story_document_ref,
            title: row.title,
            body_raw_text: row.body_raw_text,
            tags: row.tags,
        }
    }
}

impl CkcStoryBeatRecord {
    fn from_backend(row: AtelierCkcStoryBeatRow) -> Self {
        Self {
            beat_id: row.beat_id,
            beat_ref: row.beat_ref,
            story_document_id: row.story_document_id,
            story_document_ref: row.story_document_ref,
            card_id: row.card_id,
            beat_text: row.beat_text,
        }
    }
}

impl CkcMoodboardDocumentRecord {
    fn from_backend(
        row: AtelierCkcCharacterDocumentRow,
        snapshot: Option<&AtelierCkcMoodboardSnapshotRow>,
    ) -> Self {
        let body_raw_text = snapshot
            .map(|snapshot| snapshot.raw_json_text.clone())
            .or_else(|| {
                row.current_version
                    .as_ref()
                    .map(|version| version.body_raw_text.clone())
            })
            .unwrap_or_default();
        Self {
            document_id: row.document_id,
            document_ref: row.document_ref,
            current_version_id: Some(row.current_version_id),
            current_version_seq: row.current_version_seq,
            title: row.title,
            body_raw_text,
            tags: row.tags,
            latest_snapshot_id: snapshot.map(|snapshot| snapshot.snapshot_id.clone()),
            latest_snapshot_ref: snapshot.map(|snapshot| snapshot.moodboard_ref.clone()),
            moodboard_name: snapshot
                .map(|snapshot| snapshot.moodboard_name.clone())
                .unwrap_or_else(|| "Moodboard".to_owned()),
        }
    }
}

fn json_field_str<'a>(row: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    row.get(field)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
}

fn json_field_f32(row: &serde_json::Value, field: &str, fallback: f32) -> f32 {
    row.get(field)
        .and_then(|value| value.as_f64())
        .map(|value| value as f32)
        .filter(|value| value.is_finite())
        .unwrap_or(fallback)
}

fn moodboard_position(row: &serde_json::Value) -> (f32, f32) {
    row.get("position")
        .map(|position| {
            (
                json_field_f32(position, "x", 40.0),
                json_field_f32(position, "y", 40.0),
            )
        })
        .unwrap_or((40.0, 40.0))
}

fn moodboard_size(row: &serde_json::Value, fallback_w: f32, fallback_h: f32) -> (f32, f32) {
    row.get("size")
        .map(|size| {
            (
                json_field_f32(size, "width", fallback_w).max(48.0),
                json_field_f32(size, "height", fallback_h).max(32.0),
            )
        })
        .unwrap_or((fallback_w, fallback_h))
}

fn moodboard_layer_order(layer_orders: &BTreeMap<String, i32>, layer_id: Option<&str>) -> i32 {
    layer_id
        .and_then(|id| layer_orders.get(id).copied())
        .unwrap_or_default()
}

fn moodboard_element_placement_id(kind: &str, element_id: &str) -> String {
    format!("moodboard-{kind}-{element_id}")
}

fn moodboard_element_title(prefix: &str, value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        prefix.to_owned()
    } else {
        let first_line = trimmed.lines().next().unwrap_or(trimmed);
        let short: String = first_line.chars().take(72).collect();
        format!("{prefix}: {short}")
    }
}

pub fn ckc_moodboard_snapshot_to_canvas_projection(
    raw_json_text: &str,
) -> Result<CkcMoodboardCanvasProjection, String> {
    let value: serde_json::Value = serde_json::from_str(raw_json_text)
        .map_err(|err| format!("moodboard snapshot JSON parse failed: {err}"))?;
    ckc_moodboard_json_to_canvas_projection(&value)
}

pub fn ckc_moodboard_json_to_canvas_projection(
    value: &serde_json::Value,
) -> Result<CkcMoodboardCanvasProjection, String> {
    let schema_id = json_field_str(value, "schema_id").unwrap_or_default();
    if schema_id != "hsk.atelier.moodboard@1" {
        return Err(format!(
            "unsupported moodboard schema_id {schema_id:?}; expected hsk.atelier.moodboard@1"
        ));
    }

    let mut section_labels = BTreeMap::new();
    let mut layer_orders = BTreeMap::new();
    if let Some(layers) = value.get("layers").and_then(|layers| layers.as_array()) {
        for layer in layers {
            let Some(layer_id) = json_field_str(layer, "layer_id") else {
                continue;
            };
            let name = json_field_str(layer, "name").unwrap_or(layer_id);
            section_labels.insert(layer_id.to_owned(), name.to_owned());
            layer_orders.insert(
                layer_id.to_owned(),
                layer
                    .get("order")
                    .and_then(|order| order.as_i64())
                    .unwrap_or_default() as i32,
            );
        }
    }

    let mut placements = Vec::new();
    let mut element_to_placement = BTreeMap::new();

    if let Some(text_items) = value.get("text").and_then(|items| items.as_array()) {
        for (idx, item) in text_items.iter().enumerate() {
            let Some(element_id) = json_field_str(item, "element_id") else {
                continue;
            };
            let layer_id = json_field_str(item, "layer_id");
            let content = json_field_str(item, "content").unwrap_or("Moodboard text");
            let (x, y) = moodboard_position(item);
            let width = DEFAULT_CARD_W.max((content.chars().count() as f32 * 7.0).min(440.0));
            let height = DEFAULT_CARD_H;
            let placement_id = moodboard_element_placement_id("text", element_id);
            let mut card = CanvasPlacementCard::new(
                placement_id.clone(),
                format!("moodboard-text-{element_id}"),
                x,
                y,
                width,
                height,
            )
            .as_text_card(content.to_owned());
            card.live_title = Some(moodboard_element_title("Text", content));
            card.live_content_type = Some("moodboard_text".to_owned());
            card.group_id = layer_id.map(ToOwned::to_owned);
            card.z_index = moodboard_layer_order(&layer_orders, layer_id) * 1000 + idx as i32;
            element_to_placement.insert(element_id.to_owned(), placement_id);
            placements.push(card);
        }
    }

    if let Some(images) = value.get("images").and_then(|items| items.as_array()) {
        for (idx, item) in images.iter().enumerate() {
            let Some(element_id) = json_field_str(item, "element_id") else {
                continue;
            };
            let layer_id = json_field_str(item, "layer_id");
            let source = json_field_str(item, "source")
                .or_else(|| json_field_str(item, "url"))
                .unwrap_or("image");
            let block_id = json_field_str(item, "asset_id").unwrap_or(element_id);
            let (x, y) = moodboard_position(item);
            let (w, h) = moodboard_size(item, DEFAULT_CARD_W, DEFAULT_CARD_H);
            let placement_id = moodboard_element_placement_id("image", element_id);
            let mut card = CanvasPlacementCard::new(
                placement_id.clone(),
                format!("moodboard-image-{block_id}"),
                x,
                y,
                w,
                h,
            );
            card.live_title = Some(moodboard_element_title("Image", source));
            card.live_content_type = Some("moodboard_image".to_owned());
            card.group_id = layer_id.map(ToOwned::to_owned);
            card.z_index = moodboard_layer_order(&layer_orders, layer_id) * 1000 + idx as i32;
            element_to_placement.insert(element_id.to_owned(), placement_id);
            placements.push(card);
        }
    }

    if let Some(shapes) = value.get("shapes").and_then(|items| items.as_array()) {
        for (idx, item) in shapes.iter().enumerate() {
            let Some(element_id) = json_field_str(item, "element_id") else {
                continue;
            };
            let layer_id = json_field_str(item, "layer_id");
            let shape_type = json_field_str(item, "shape_type").unwrap_or("shape");
            let (x, y) = moodboard_position(item);
            let (w, h) = moodboard_size(item, DEFAULT_CARD_W, DEFAULT_CARD_H);
            let placement_id = moodboard_element_placement_id("shape", element_id);
            let mut card = CanvasPlacementCard::new(
                placement_id.clone(),
                format!("moodboard-shape-{element_id}"),
                x,
                y,
                w,
                h,
            );
            card.live_title = Some(moodboard_element_title("Shape", shape_type));
            card.live_content_type = Some("moodboard_shape".to_owned());
            card.group_id = layer_id.map(ToOwned::to_owned);
            card.z_index = moodboard_layer_order(&layer_orders, layer_id) * 1000 + idx as i32;
            element_to_placement.insert(element_id.to_owned(), placement_id);
            placements.push(card);
        }
    }

    let mut visual_edges = Vec::new();
    if let Some(connectors) = value.get("connectors").and_then(|items| items.as_array()) {
        for item in connectors {
            let Some(connector_id) = json_field_str(item, "connector_id") else {
                continue;
            };
            let Some(from_element_id) = json_field_str(item, "from_element_id") else {
                continue;
            };
            let Some(to_element_id) = json_field_str(item, "to_element_id") else {
                continue;
            };
            let Some(from_placement_id) = element_to_placement.get(from_element_id) else {
                continue;
            };
            let Some(to_placement_id) = element_to_placement.get(to_element_id) else {
                continue;
            };
            visual_edges.push(VisualEdge {
                visual_edge_id: format!("moodboard-connector-{connector_id}"),
                from_placement_id: from_placement_id.clone(),
                to_placement_id: to_placement_id.clone(),
            });
        }
    }

    Ok(CkcMoodboardCanvasProjection {
        placements,
        visual_edges,
        section_labels,
        pan: egui::Vec2::ZERO,
        zoom: 1.0,
    })
}

impl CkcSearchResultRecord {
    fn from_backend(row: AtelierCkcSearchResultRow) -> Self {
        let AtelierCkcSearchResultRow {
            target_kind,
            target_ref,
            title,
            snippet,
            character_ref,
            sheet_version_ref,
            collection_ref,
            media_ref,
            tag_ref,
            tags,
            tag_notes,
            match_modes,
            fuzzy_score: _,
            vector_score: _,
        } = row;
        let tag_notes = tag_notes
            .into_iter()
            .filter(|note| {
                ckc_scope_matches_result_refs(
                    note.scope_ref.as_deref(),
                    &target_ref,
                    character_ref.as_deref(),
                    sheet_version_ref.as_deref(),
                    collection_ref.as_deref(),
                    media_ref.as_deref(),
                    tag_ref.as_deref(),
                )
            })
            .map(CkcTagNoteRecord::from_backend)
            .collect();
        Self {
            target_kind,
            target_ref,
            title,
            snippet,
            character_ref,
            sheet_version_ref,
            collection_ref,
            media_ref,
            tag_ref,
            tags,
            tag_notes,
            match_modes,
        }
    }

    fn summary_label(&self) -> String {
        let modes = if self.match_modes.is_empty() {
            "match".to_owned()
        } else {
            self.match_modes.join("+")
        };
        format!(
            "{}: {} [{}] {}",
            self.target_kind, self.title, modes, self.target_ref
        )
    }
}

impl CkcTagNoteRecord {
    fn from_backend(row: AtelierCkcTagNoteRow) -> Self {
        Self {
            tag_ref: row.tag_ref,
            tag_text: row.tag_text,
            scope_ref: row.scope_ref,
            note: row.note,
        }
    }
}

fn seeded_ckc_search_results(characters: &[CkcCharacterRecord]) -> Vec<CkcSearchResultRecord> {
    local_ckc_search(
        characters,
        "mira reference",
        CkcSearchMode::Fuzzy,
        &["reference".to_owned()],
    )
}

fn local_ckc_search(
    characters: &[CkcCharacterRecord],
    query: &str,
    mode: CkcSearchMode,
    tags: &[String],
) -> Vec<CkcSearchResultRecord> {
    let query = query.trim().to_ascii_lowercase();
    let tags: Vec<String> = tags
        .iter()
        .map(|tag| tag.trim().to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect();
    let mut out = Vec::new();
    for character in characters {
        let character_tags = ["character".to_owned(), "sheet".to_owned()];
        let haystack = format!(
            "{}\n{}\n{}",
            character.display_name, character.public_id, character.sheet_editor_text
        )
        .to_ascii_lowercase();
        if local_match(&haystack, &query, mode)
            && tags.iter().all(|tag| {
                character_tags.iter().any(|candidate| candidate == tag)
                    || haystack.contains(tag.as_str())
            })
        {
            out.push(CkcSearchResultRecord {
                target_kind: "character".to_owned(),
                target_ref: character.character_ref(),
                title: character.display_name.clone(),
                snippet: character
                    .sheet_editor_text
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_owned(),
                character_ref: Some(character.character_ref()),
                sheet_version_ref: character.sheet_version_ref(),
                collection_ref: None,
                media_ref: None,
                tag_ref: None,
                tags: character_tags.to_vec(),
                tag_notes: Vec::new(),
                match_modes: vec![mode.backend_value().to_owned()],
            });
        }

        for album in &character.media_albums {
            let album_haystack = format!(
                "{}\n{}\n{}\n{}",
                album.name,
                album.description,
                album.tags.join(" "),
                character.display_name
            )
            .to_ascii_lowercase();
            if local_match(&album_haystack, &query, mode)
                && tags
                    .iter()
                    .all(|tag| album.tags.iter().any(|candidate| candidate == tag))
            {
                out.push(CkcSearchResultRecord {
                    target_kind: "album".to_owned(),
                    target_ref: album.collection_ref.clone(),
                    title: album.name.clone(),
                    snippet: album.description.clone(),
                    character_ref: Some(character.character_ref()),
                    sheet_version_ref: character.sheet_version_ref(),
                    collection_ref: Some(album.collection_ref.clone()),
                    media_ref: None,
                    tag_ref: None,
                    tags: album.tags.clone(),
                    tag_notes: seeded_local_tag_notes(album),
                    match_modes: vec![mode.backend_value().to_owned()],
                });
            }
            for member in &album.members {
                let member_tags = ckc_tags_from_buffer(&member.tags_buffer);
                let member_haystack = format!(
                    "{}\n{}\n{}\n{}\n{}",
                    member.display_label,
                    member.notes,
                    member_tags.join(" "),
                    album.name,
                    character.display_name
                )
                .to_ascii_lowercase();
                if local_match(&member_haystack, &query, mode)
                    && tags
                        .iter()
                        .all(|tag| member_tags.iter().any(|candidate| candidate == tag))
                {
                    out.push(CkcSearchResultRecord {
                        target_kind: "media".to_owned(),
                        target_ref: member.media_ref.clone(),
                        title: member.display_label.clone(),
                        snippet: member.notes.clone(),
                        character_ref: Some(character.character_ref()),
                        sheet_version_ref: character.sheet_version_ref(),
                        collection_ref: Some(album.collection_ref.clone()),
                        media_ref: Some(member.media_ref.clone()),
                        tag_ref: None,
                        tags: member_tags,
                        tag_notes: seeded_local_tag_notes(album),
                        match_modes: vec![mode.backend_value().to_owned()],
                    });
                }
            }
        }

        for story in &character.story_documents {
            let story_haystack = format!(
                "{}\n{}\n{}\n{}",
                story.title,
                story.body_raw_text,
                story.tags.join(" "),
                character.display_name
            )
            .to_ascii_lowercase();
            if local_match(&story_haystack, &query, mode)
                && tags
                    .iter()
                    .all(|tag| story.tags.iter().any(|candidate| candidate == tag))
            {
                out.push(CkcSearchResultRecord {
                    target_kind: "story".to_owned(),
                    target_ref: story.document_ref.clone(),
                    title: story.title.clone(),
                    snippet: story.body_raw_text.clone(),
                    character_ref: Some(character.character_ref()),
                    sheet_version_ref: character.sheet_version_ref(),
                    collection_ref: None,
                    media_ref: None,
                    tag_ref: None,
                    tags: story.tags.clone(),
                    tag_notes: Vec::new(),
                    match_modes: vec![mode.backend_value().to_owned()],
                });
            }
            for card in &story.cards {
                let card_haystack = format!(
                    "{}\n{}\n{}\n{}\n{}",
                    card.title,
                    card.body_raw_text,
                    card.tags.join(" "),
                    story.title,
                    character.display_name
                )
                .to_ascii_lowercase();
                if local_match(&card_haystack, &query, mode)
                    && tags
                        .iter()
                        .all(|tag| card.tags.iter().any(|candidate| candidate == tag))
                {
                    out.push(CkcSearchResultRecord {
                        target_kind: "story_card".to_owned(),
                        target_ref: card.card_ref.clone(),
                        title: card.title.clone(),
                        snippet: card.body_raw_text.clone(),
                        character_ref: Some(character.character_ref()),
                        sheet_version_ref: character.sheet_version_ref(),
                        collection_ref: None,
                        media_ref: None,
                        tag_ref: None,
                        tags: card.tags.clone(),
                        tag_notes: Vec::new(),
                        match_modes: vec![mode.backend_value().to_owned()],
                    });
                }
            }
        }

        for moodboard in &character.moodboard_documents {
            let moodboard_haystack = format!(
                "{}\n{}\n{}\n{}\n{}",
                moodboard.title,
                moodboard.moodboard_name,
                moodboard.body_raw_text,
                moodboard.tags.join(" "),
                character.display_name
            )
            .to_ascii_lowercase();
            if local_match(&moodboard_haystack, &query, mode)
                && tags
                    .iter()
                    .all(|tag| moodboard.tags.iter().any(|candidate| candidate == tag))
            {
                out.push(CkcSearchResultRecord {
                    target_kind: "moodboard".to_owned(),
                    target_ref: moodboard
                        .latest_snapshot_ref
                        .clone()
                        .unwrap_or_else(|| moodboard.document_ref.clone()),
                    title: moodboard.title.clone(),
                    snippet: moodboard.body_raw_text.clone(),
                    character_ref: Some(character.character_ref()),
                    sheet_version_ref: character.sheet_version_ref(),
                    collection_ref: None,
                    media_ref: None,
                    tag_ref: None,
                    tags: moodboard.tags.clone(),
                    tag_notes: Vec::new(),
                    match_modes: vec![mode.backend_value().to_owned()],
                });
            }
        }
    }
    out.truncate(8);
    out
}

fn local_match(haystack: &str, query: &str, mode: CkcSearchMode) -> bool {
    if query.is_empty() {
        return true;
    }
    if haystack.contains(query) {
        return true;
    }
    match mode {
        CkcSearchMode::Fuzzy | CkcSearchMode::Combined => query.split_whitespace().all(|needle| {
            haystack
                .split_whitespace()
                .any(|word| fuzzy_word_match(word, needle))
        }),
        CkcSearchMode::Vector => query.split_whitespace().any(|needle| {
            haystack
                .split_whitespace()
                .any(|word| fuzzy_word_match(word, needle))
        }),
    }
}

fn fuzzy_word_match(word: &str, needle: &str) -> bool {
    if needle.len() <= 2 {
        return word == needle;
    }
    if word.contains(needle) || needle.contains(word) {
        return true;
    }
    let common = needle.chars().filter(|ch| word.contains(*ch)).count();
    common + 1 >= needle.len()
}

fn seeded_local_tag_notes(album: &CkcMediaAlbumRecord) -> Vec<CkcTagNoteRecord> {
    album
        .tags
        .iter()
        .filter(|tag| tag.as_str() == "training" || tag.as_str() == "reference")
        .map(|tag| CkcTagNoteRecord {
            tag_ref: format!("atelier://tag/local-{tag}"),
            tag_text: tag.clone(),
            scope_ref: Some(album.collection_ref.clone()),
            note: format!("{tag} applies to reusable CKC media for this album."),
        })
        .collect()
}

fn local_import_ckc_sheet(state: &mut AtelierPanelState, selected_index: usize) {
    let import_text = match local_import_raw_text(&state.ckc_import_text) {
        Ok(raw_text) => raw_text,
        Err(err) => {
            state.ckc_export_status = err;
            return;
        }
    };
    let next_seq = {
        let Some(character) = state.ckc_characters.get_mut(selected_index) else {
            state.ckc_export_status = "No CKC character selected for import.".to_owned();
            return;
        };
        if let Err(err) = local_validate_ckc_sheet_owner(character, &import_text) {
            state.ckc_export_status = err;
            return;
        }
        character.parent_sheet_version_id = character.sheet_version_id.clone();
        let next_sheet_version_id = Uuid::new_v4().to_string();
        character.sheet_version_id = Some(next_sheet_version_id.clone());
        character.sheet_seq += 1;
        character.sheet_editor_text = import_text;
        character.sheet_version_ref = Some(format!(
            "atelier://sheet/{}/{}",
            character.character_internal_id, next_sheet_version_id
        ));
        character.sheet_seq
    };
    state.ckc_last_export = None;
    state.ckc_export_status = format!(
        "Imported CKC sheet locally as append-only version v{}",
        next_seq
    );
}

fn local_import_raw_text(import_text: &str) -> Result<String, String> {
    let trimmed = import_text.trim();
    if trimmed.is_empty() {
        return Err("CKC import text is empty.".to_owned());
    }
    if !trimmed.starts_with('{') {
        return Ok(import_text.to_owned());
    }
    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|err| format!("CKC sheet import JSON is invalid: {err}"))?;
    local_raw_text_from_export_json(&value)
        .ok_or_else(|| "CKC sheet import JSON must contain raw_text or content.".to_owned())
}

fn local_raw_text_from_export_json(value: &serde_json::Value) -> Option<String> {
    if let Some(raw_text) = value.get("raw_text").and_then(|value| value.as_str()) {
        return Some(raw_text.to_owned());
    }
    let content = value.get("content").and_then(|value| value.as_str())?;
    if content.trim_start().starts_with('{') {
        serde_json::from_str::<serde_json::Value>(content)
            .ok()
            .and_then(|nested| local_raw_text_from_export_json(&nested))
            .or_else(|| Some(content.to_owned()))
    } else {
        Some(content.to_owned())
    }
}

fn local_validate_ckc_sheet_owner(
    character: &CkcCharacterRecord,
    raw_text: &str,
) -> Result<(), String> {
    let character_ids = sheet_field_values(raw_text, "CHAR-ID-001");
    if character_ids.is_empty() {
        return Err(
            "CKC sheet import must include CHAR-ID-001 for character ownership.".to_owned(),
        );
    }
    if character_ids.len() > 1 {
        return Err(format!(
            "CKC sheet import must include exactly one CHAR-ID-001 for character ownership; found {}",
            character_ids.len()
        ));
    }
    let Some(character_id) = character_ids.into_iter().next() else {
        return Err(
            "CKC sheet import must include CHAR-ID-001 for character ownership.".to_owned(),
        );
    };
    if character_id != character.public_id {
        return Err(format!(
            "CKC sheet CHAR-ID-001={character_id} does not match character public_id={}",
            character.public_id
        ));
    }
    Ok(())
}

fn local_content_hash(content: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn local_export_ckc_sheet(character: &CkcCharacterRecord, format: &str) -> AtelierSheetExportRow {
    let version_id = character
        .sheet_version_id
        .clone()
        .unwrap_or_else(|| "local-unsaved-sheet".to_owned());
    let sheet_version_ref = character.sheet_version_ref.clone().unwrap_or_else(|| {
        format!(
            "atelier://sheet/{}/{}",
            character.character_internal_id, version_id
        )
    });
    let (format_label, file_ext, raw_text) = match format {
        "json" => ("json", "json", character.sheet_editor_text.clone()),
        "safe-txt" => (
            "safe-txt",
            "safe.txt",
            local_safe_subset_sheet_text(&character.sheet_editor_text),
        ),
        "safe-json" => (
            "safe-json",
            "safe.json",
            local_safe_subset_sheet_text(&character.sheet_editor_text),
        ),
        _ => ("txt", "txt", character.sheet_editor_text.clone()),
    };
    let content = if format_label.ends_with("json") {
        local_export_sheet_json(
            character,
            &version_id,
            &sheet_version_ref,
            &raw_text,
            format_label,
        )
    } else {
        raw_text
    };
    AtelierSheetExportRow {
        version_id: version_id.clone(),
        format: format_label.to_owned(),
        file_name: format!("ckc-sheet-{version_id}.{file_ext}"),
        content_hash: local_content_hash(&content),
        content,
        character_ref: character.character_ref.clone(),
        sheet_version_ref,
    }
}

fn local_export_sheet_json(
    character: &CkcCharacterRecord,
    version_id: &str,
    sheet_version_ref: &str,
    raw_text: &str,
    format_label: &str,
) -> String {
    let export_format = if format_label == "safe-json" {
        "ckc-sheet-safe-export.v1"
    } else {
        "ckc-sheet-export.v1"
    };
    serde_json::to_string_pretty(&serde_json::json!({
        "export_format": export_format,
        "template_version": LOCAL_CKC_TEMPLATE_VERSION,
        "version_id": version_id,
        "character_internal_id": &character.character_internal_id,
        "parent_version_id": &character.parent_sheet_version_id,
        "seq": character.sheet_seq,
        "author": "handshake-native-atelier-ckc-local",
        "tool": "handshake-native-atelier-local-export",
        "character_ref": &character.character_ref,
        "sheet_version_ref": sheet_version_ref,
        "raw_text": raw_text,
        "created_at_utc": "local-no-backend",
    }))
    .unwrap_or_else(|_| raw_text.to_owned())
}

fn local_safe_subset_sheet_text(raw_text: &str) -> String {
    let safe_ids = local_safe_subset_ids();
    let mut out = String::with_capacity(raw_text.len());
    for segment in raw_text.split_inclusive('\n') {
        let trimmed_line = segment.trim_end_matches(['\r', '\n']);
        match local_sheet_field_id_from_line(trimmed_line) {
            Some(field_id) if safe_ids.contains(&field_id) => out.push_str(segment),
            Some(_) => {}
            None if local_sheet_line_looks_like_field(trimmed_line) => {}
            None => out.push_str(segment),
        }
    }
    out
}

fn local_safe_subset_ids() -> std::collections::HashSet<String> {
    serde_json::from_str::<Vec<String>>(LOCAL_CKC_SAFE_SUBSET_V2_JSON)
        .unwrap_or_default()
        .into_iter()
        .map(|field_id| field_id.to_ascii_uppercase())
        .collect()
}

fn local_field_suggestions(
    characters: &[CkcCharacterRecord],
    field_id: &str,
) -> Vec<AtelierSheetFieldSuggestionRow> {
    let mut out = Vec::new();
    let field_id = field_id.trim().to_ascii_uppercase();
    for character in characters {
        if let Some(value) = sheet_field_value(&character.sheet_editor_text, &field_id) {
            if let Some(index) = out
                .iter()
                .position(|row: &AtelierSheetFieldSuggestionRow| row.value == value)
            {
                out[index].occurrences += 1;
            } else {
                out.push(AtelierSheetFieldSuggestionRow {
                    field_id: field_id.clone(),
                    value,
                    occurrences: 1,
                });
            }
        }
    }
    out.truncate(8);
    out
}

fn sheet_field_value(raw_text: &str, field_id: &str) -> Option<String> {
    sheet_field_values(raw_text, field_id).into_iter().next()
}

fn sheet_field_values(raw_text: &str, field_id: &str) -> Vec<String> {
    let field_id = field_id.trim();
    if field_id.is_empty() {
        return Vec::new();
    }
    let mut values = Vec::new();
    for line in raw_text.lines().map(str::trim) {
        let Some((parsed_field_id, value)) = local_split_field_line(line) else {
            continue;
        };
        if parsed_field_id.eq_ignore_ascii_case(field_id) {
            values.push(value);
        }
    }
    values
}

fn local_sheet_field_id_from_line(line: &str) -> Option<String> {
    let (field_id, _) = local_split_field_line(line.trim())?;
    Some(field_id)
}

fn local_sheet_line_looks_like_field(line: &str) -> bool {
    let Some(colon) = line.find(':') else {
        return false;
    };
    let before_colon = line[..colon].trim();
    let Some(id_end) = local_field_id_end(before_colon) else {
        return false;
    };
    before_colon[id_end..]
        .chars()
        .any(|ch| matches!(ch, '\u{2014}' | '\u{2013}' | '-'))
}

fn local_split_field_line(line: &str) -> Option<(String, String)> {
    let colon = line.find(':')?;
    let before_colon = line[..colon].trim();
    let descriptor = line[colon + 1..].trim();
    let id_end = local_field_id_end(before_colon)?;
    let id = before_colon[..id_end].trim();
    let after_id = before_colon[id_end..].trim_start();
    let separator = after_id.chars().next()?;
    if !matches!(separator, '\u{2014}' | '\u{2013}' | '-') {
        return None;
    }
    let label = after_id[separator.len_utf8()..].trim();
    if label.is_empty() {
        return None;
    }
    let value = local_normalize_field_value(descriptor)?;
    Some((id.to_ascii_uppercase(), value))
}

fn local_field_id_end(before_colon: &str) -> Option<usize> {
    let mut idx = 0usize;
    for segment_idx in 0..3 {
        let segment_start = idx;
        while let Some(ch) = before_colon[idx..].chars().next() {
            if ch.is_ascii_uppercase() || ch.is_ascii_digit() {
                idx += ch.len_utf8();
            } else {
                break;
            }
        }
        if idx == segment_start {
            return None;
        }
        if segment_idx < 2 {
            if before_colon[idx..].starts_with('-') {
                idx += 1;
            } else {
                return None;
            }
        }
    }
    Some(idx)
}

fn local_normalize_field_value(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 500 {
        return None;
    }
    if value.starts_with('<') && value.ends_with('>') {
        return None;
    }
    Some(value.to_owned())
}

fn short_hash(hash: &str) -> &str {
    hash.get(..hash.len().min(12)).unwrap_or(hash)
}

pub fn ckc_field_suggestion_row_author_id(field_id: &str, value: &str) -> String {
    format!(
        "atelier-ckc-field-suggestion-{}-{}",
        stable_author_id_suffix(field_id),
        stable_author_id_suffix(value)
    )
}

fn ckc_search_status_from_response(response: &AtelierCkcSearchResponse) -> String {
    let modes = if response.modes.is_empty() {
        "fuzzy".to_owned()
    } else {
        response.modes.join("+")
    };
    let vector = response
        .vector_source
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("no vector source");
    format!(
        "CKC search returned {} result(s) for '{}' via {modes}; semantic_available={} ({vector})",
        response.result_count, response.query, response.semantic_available
    )
}

fn attach_tag_note_to_visible_results(
    results: &mut [CkcSearchResultRecord],
    note: CkcTagNoteRecord,
) {
    for result in results {
        let tag_matches = result.tags.iter().any(|tag| tag == &note.tag_text)
            || result
                .tag_ref
                .as_deref()
                .is_some_and(|tag_ref| tag_ref == note.tag_ref);
        let scope_matches = note
            .scope_ref
            .as_deref()
            .map(|scope_ref| ckc_result_has_ref(result, Some(scope_ref)))
            .unwrap_or(true);
        if tag_matches && scope_matches {
            result.tag_notes.retain(|existing| {
                !(existing.tag_text == note.tag_text && existing.scope_ref == note.scope_ref)
            });
            result.tag_notes.push(note.clone());
        }
    }
}

fn apply_ckc_character_document_row(
    characters: &mut [CkcCharacterRecord],
    row: AtelierCkcCharacterDocumentRow,
) -> Option<String> {
    let character_internal_id = row.character_internal_id.clone();
    let document_id = row.document_id.clone();
    let doc_type = row.doc_type.clone();
    let target = characters.iter_mut().find(|character| {
        character.character_internal_id == character_internal_id
            || character
                .story_documents
                .iter()
                .any(|document| document.document_id == document_id)
            || character
                .moodboard_documents
                .iter()
                .any(|document| document.document_id == document_id)
    })?;
    match doc_type.as_str() {
        "story" => {
            let mut updated = CkcStoryDocumentRecord::from_backend(row);
            let existing_index = target
                .story_documents
                .iter()
                .position(|document| document.document_id == document_id)
                .or_else(|| {
                    target
                        .story_documents
                        .iter()
                        .position(|document| is_pending_ckc_document_id(&document.document_id))
                });
            if let Some(existing_index) = existing_index {
                let existing = &mut target.story_documents[existing_index];
                if updated.cards.is_empty() {
                    updated.cards = existing.cards.clone();
                }
                if updated.beats.is_empty() {
                    updated.beats = existing.beats.clone();
                }
                *existing = updated;
            } else {
                target.story_documents.push(updated);
            }
            Some(format!("Saved CKC story document {document_id}"))
        }
        "moodboard" => {
            let mut updated = CkcMoodboardDocumentRecord::from_backend(row, None);
            let existing_index = target
                .moodboard_documents
                .iter()
                .position(|document| document.document_id == document_id)
                .or_else(|| {
                    target
                        .moodboard_documents
                        .iter()
                        .position(|document| is_pending_ckc_document_id(&document.document_id))
                });
            if let Some(existing_index) = existing_index {
                let existing = &mut target.moodboard_documents[existing_index];
                if updated.latest_snapshot_id.is_none() {
                    updated.latest_snapshot_id = existing.latest_snapshot_id.clone();
                    updated.latest_snapshot_ref = existing.latest_snapshot_ref.clone();
                    updated.moodboard_name = existing.moodboard_name.clone();
                }
                *existing = updated;
            } else {
                target.moodboard_documents.push(updated);
            }
            Some(format!("Saved CKC moodboard document {document_id}"))
        }
        _ => None,
    }
}

fn apply_ckc_story_card_row(
    characters: &mut [CkcCharacterRecord],
    row: AtelierCkcStoryCardRow,
) -> Option<String> {
    let story_document_id = row.story_document_id.clone();
    let card_id = row.card_id.clone();
    for character in characters {
        if let Some(story) = character
            .story_documents
            .iter_mut()
            .find(|story| story.document_id == story_document_id)
        {
            let updated = CkcStoryCardRecord::from_backend(row);
            if let Some(existing) = story.cards.iter_mut().find(|card| card.card_id == card_id) {
                *existing = updated;
            } else {
                story.cards.push(updated);
            }
            return Some(format!("Added CKC story card {card_id}"));
        }
    }
    None
}

fn apply_ckc_story_beat_row(
    characters: &mut [CkcCharacterRecord],
    row: AtelierCkcStoryBeatRow,
) -> Option<String> {
    let story_document_id = row.story_document_id.clone();
    let beat_id = row.beat_id.clone();
    for character in characters {
        if let Some(story) = character
            .story_documents
            .iter_mut()
            .find(|story| story.document_id == story_document_id)
        {
            let updated = CkcStoryBeatRecord::from_backend(row);
            if let Some(existing) = story.beats.iter_mut().find(|beat| beat.beat_id == beat_id) {
                *existing = updated;
            } else {
                story.beats.push(updated);
            }
            return Some(format!("Added CKC story beat {beat_id}"));
        }
    }
    None
}

fn apply_ckc_moodboard_snapshot_row(
    characters: &mut [CkcCharacterRecord],
    row: AtelierCkcMoodboardSnapshotRow,
) -> Result<(Option<String>, CkcMoodboardCanvasProjection, String), String> {
    let projection = ckc_moodboard_snapshot_to_canvas_projection(&row.raw_json_text)?;
    let document_id = row.document_id.clone();
    let snapshot_id = row.snapshot_id.clone();
    let snapshot_ref = row.moodboard_ref.clone();
    for character in characters {
        if let Some(moodboard) = character
            .moodboard_documents
            .iter_mut()
            .find(|moodboard| moodboard.document_id == document_id)
        {
            moodboard.latest_snapshot_id = Some(row.snapshot_id);
            moodboard.latest_snapshot_ref = Some(row.moodboard_ref);
            moodboard.moodboard_name = row.moodboard_name;
            moodboard.body_raw_text = row.raw_json_text;
            return Ok((
                Some(format!("Opened CKC moodboard snapshot {snapshot_id}")),
                projection,
                snapshot_ref,
            ));
        }
    }
    Ok((None, projection, snapshot_ref))
}

#[derive(Debug)]
struct AtelierPanelState {
    active_tab: AtelierPanelTab,
    ckc_book_mode: CkcBookMode,
    ckc_characters: Vec<CkcCharacterRecord>,
    ckc_selected_index: usize,
    ckc_new_display_name: String,
    ckc_backend_loaded: bool,
    ckc_load_requested: bool,
    ckc_loading: bool,
    ckc_create_pending: bool,
    ckc_append_pending: bool,
    ckc_template_pending: bool,
    ckc_safe_subset_pending: bool,
    ckc_template_status: String,
    ckc_import_text: String,
    ckc_import_pending: bool,
    ckc_export_pending: bool,
    ckc_export_status: String,
    ckc_last_export: Option<AtelierSheetExportRow>,
    ckc_field_suggestion_id: String,
    ckc_field_suggestion_pending: bool,
    ckc_field_suggestion_status: String,
    ckc_field_suggestions: Vec<AtelierSheetFieldSuggestionRow>,
    ckc_sheet_artifact_pending: bool,
    ckc_sheet_artifact_status: String,
    ckc_sheet_artifact_kind: String,
    ckc_sheet_artifact_ref: String,
    ckc_sheet_artifact_manifest_ref: String,
    ckc_sheet_artifact_label: String,
    ckc_sheet_artifact_reuse_role: String,
    ckc_sheet_artifact_actor_id: String,
    ckc_selected_sheet_artifact_link_id: Option<String>,
    ckc_sheet_artifact_reuse_ref: String,
    ckc_media_save_pending: bool,
    ckc_selected_media_key: Option<String>,
    ckc_selected_album_collection_id: Option<String>,
    ckc_album_create_name: String,
    ckc_album_create_notes: String,
    ckc_album_create_tags: String,
    ckc_album_link_asset_ids: String,
    ckc_album_link_source_path_ref: String,
    ckc_album_link_source_url_ref: String,
    ckc_album_create_pending: bool,
    ckc_album_link_pending: bool,
    ckc_album_page_pending: bool,
    ckc_album_status: String,
    ckc_story_card_title: String,
    ckc_story_card_body: String,
    ckc_story_beat_text: String,
    ckc_story_status: String,
    ckc_moodboard_status: String,
    ckc_active_story_document_id: Option<String>,
    ckc_active_moodboard_document_id: Option<String>,
    ckc_character_notes_buffer: String,
    ckc_character_notes_source_key: Option<String>,
    ckc_character_notes_status: String,
    ckc_search_query: String,
    ckc_search_tags: String,
    ckc_search_filter_selected_character: bool,
    ckc_search_filter_selected_collection: bool,
    ckc_search_filter_selected_media: bool,
    ckc_search_use_selected_media_similarity: bool,
    ckc_search_mode: CkcSearchMode,
    ckc_search_pending: bool,
    ckc_search_status: String,
    ckc_search_results: Vec<CkcSearchResultRecord>,
    ckc_tag_note_tag: String,
    ckc_tag_note_scope_ref: String,
    ckc_tag_note_editor: String,
    ckc_tag_note_pending: bool,
    ckc_error: Option<String>,
    pose_yaw: f32,
    pose_pitch: f32,
    pose_zoom: f32,
    pose_face: bool,
    pose_body: bool,
    pose_hands: bool,
    pose_source_ref: String,
    pose_rig_id: String,
    pose_marker_family: String,
    pose_marker_index: i32,
    pose_marker_x: f32,
    pose_marker_y: f32,
    pose_marker_confidence: f32,
    pose_marker_status: String,
    pose_marker_edits: Vec<PosekitMarkerEditRecord>,
    pose_framing_preset: String,
    pose_framing_lens_mm: i32,
    pose_framing_padding_top_px: i32,
    pose_framing_padding_right_px: i32,
    pose_framing_padding_bottom_px: i32,
    pose_framing_padding_left_px: i32,
    pose_export_pending: bool,
    pose_export_request_seq: u64,
    pose_active_export_request: Option<u64>,
    pose_export_status: String,
    pose_last_export: Option<PosekitExportSnapshot>,
    ingest_decision: IngestDecision,
    ingest_dataset_ref: String,
    ingest_character_ref: String,
    ingest_tag_buffer: String,
    ingest_batch_note: String,
    ingest_event: String,
    ingest_date: String,
    ingest_location: String,
    ingest_link_passed: bool,
    ingest_contact_rows: String,
    ingest_contact_columns: String,
    ingest_contact_dpi: String,
    ingest_facial_profile: String,
    ingest_status: String,
    ingest_item_decisions: BTreeMap<String, IngestDecision>,
    ingest_persisted_item_ids: BTreeSet<String>,
    ingest_apply_pending: bool,
    ingest_apply_request_id: Option<String>,
    ingest_apply_batch_id: Option<String>,
    ingest_last_apply_receipt: String,
}

impl Default for AtelierPanelState {
    fn default() -> Self {
        let ckc_characters = seeded_ckc_characters();
        let ckc_search_results = seeded_ckc_search_results(&ckc_characters);
        Self {
            active_tab: AtelierPanelTab::CastkitCodex,
            ckc_book_mode: CkcBookMode::Sheet,
            ckc_characters,
            ckc_selected_index: 0,
            ckc_new_display_name: "New character".to_owned(),
            ckc_backend_loaded: false,
            ckc_load_requested: false,
            ckc_loading: false,
            ckc_create_pending: false,
            ckc_append_pending: false,
            ckc_template_pending: false,
            ckc_safe_subset_pending: false,
            ckc_template_status:
                "Built-in CKC template: not loaded; use Load template or Safe subset.".to_owned(),
            ckc_import_text: String::new(),
            ckc_import_pending: false,
            ckc_export_pending: false,
            ckc_export_status: "No CKC sheet export requested.".to_owned(),
            ckc_last_export: None,
            ckc_field_suggestion_id: "CHAR-ID-006".to_owned(),
            ckc_field_suggestion_pending: false,
            ckc_field_suggestion_status: "No CKC field suggestions loaded.".to_owned(),
            ckc_field_suggestions: Vec::new(),
            ckc_sheet_artifact_pending: false,
            ckc_sheet_artifact_status:
                "Sheet artifact links ready: attach Posekit/OpenPose or Comfy refs to the current sheet version."
                    .to_owned(),
            ckc_sheet_artifact_kind: "openpose_png".to_owned(),
            ckc_sheet_artifact_ref: "artifact://atelier/comfy/render/example.png".to_owned(),
            ckc_sheet_artifact_manifest_ref: "receipt://atelier/comfy/example".to_owned(),
            ckc_sheet_artifact_label: "reusable CUI artifact".to_owned(),
            ckc_sheet_artifact_reuse_role: "cui_identity_reference".to_owned(),
            ckc_sheet_artifact_actor_id: String::new(),
            ckc_selected_sheet_artifact_link_id: None,
            ckc_sheet_artifact_reuse_ref: String::new(),
            ckc_media_save_pending: false,
            ckc_selected_media_key: None,
            ckc_selected_album_collection_id: None,
            ckc_album_create_name: "Reference album".to_owned(),
            ckc_album_create_notes: String::new(),
            ckc_album_create_tags: "reference".to_owned(),
            ckc_album_link_asset_ids: String::new(),
            ckc_album_link_source_path_ref: String::new(),
            ckc_album_link_source_url_ref: String::new(),
            ckc_album_create_pending: false,
            ckc_album_link_pending: false,
            ckc_album_page_pending: false,
            ckc_album_status: "CKC album controls ready".to_owned(),
            ckc_story_card_title: "New story card".to_owned(),
            ckc_story_card_body: "Reusable scene, continuity, or production beat.".to_owned(),
            ckc_story_beat_text: "Reusable story beat for this character.".to_owned(),
            ckc_story_status:
                "CKC story documents are separate from sheet notes, image notes, and tag notes."
                    .to_owned(),
            ckc_moodboard_status:
                "CKC moodboards use native Handshake moodboard snapshots, not Excalidraw."
                    .to_owned(),
            ckc_active_story_document_id: None,
            ckc_active_moodboard_document_id: None,
            ckc_character_notes_buffer: String::new(),
            ckc_character_notes_source_key: None,
            ckc_character_notes_status:
                "Character sheet notes mirror the notes field inside the selected sheet.".to_owned(),
            ckc_search_query: String::new(),
            ckc_search_tags: String::new(),
            ckc_search_filter_selected_character: false,
            ckc_search_filter_selected_collection: false,
            ckc_search_filter_selected_media: false,
            ckc_search_use_selected_media_similarity: false,
            ckc_search_mode: CkcSearchMode::Fuzzy,
            ckc_search_pending: false,
            ckc_search_status: "Local CKC search ready".to_owned(),
            ckc_search_results,
            ckc_tag_note_tag: "training".to_owned(),
            ckc_tag_note_scope_ref: "atelier://collection/018f7848-1111-7000-9000-00000000a001"
                .to_owned(),
            ckc_tag_note_editor: "Use this tag for reusable CKC training/reference media."
                .to_owned(),
            ckc_tag_note_pending: false,
            ckc_error: None,
            pose_yaw: 0.0,
            pose_pitch: 0.0,
            pose_zoom: 1.0,
            pose_face: true,
            pose_body: true,
            pose_hands: false,
            pose_source_ref: "atelier://media/mira-demo/pose-source.png".to_owned(),
            pose_rig_id: String::new(),
            pose_marker_family: "face".to_owned(),
            pose_marker_index: 12,
            pose_marker_x: 321.0,
            pose_marker_y: 222.0,
            pose_marker_confidence: 0.87,
            pose_marker_status:
                "Posekit marker editor ready; staged edits apply to the next OpenPose export."
                    .to_owned(),
            pose_marker_edits: Vec::new(),
            pose_framing_preset: "standard".to_owned(),
            pose_framing_lens_mm: 50,
            pose_framing_padding_top_px: 0,
            pose_framing_padding_right_px: 0,
            pose_framing_padding_bottom_px: 0,
            pose_framing_padding_left_px: 0,
            pose_export_pending: false,
            pose_export_request_seq: 0,
            pose_active_export_request: None,
            pose_export_status: "No Posekit OpenPose export requested.".to_owned(),
            pose_last_export: None,
            ingest_decision: IngestDecision::Unsure,
            ingest_dataset_ref: "dataset://atelier/inbox".to_owned(),
            ingest_character_ref: "atelier://character/mira-demo".to_owned(),
            ingest_tag_buffer: "event, outfit, source".to_owned(),
            ingest_batch_note:
                "Batch note applied to selected/pass/reject/unsure image review rows.".to_owned(),
            ingest_event: "source mining".to_owned(),
            ingest_date: "2026-06-30".to_owned(),
            ingest_location: "atelier intake".to_owned(),
            ingest_link_passed: false,
            ingest_contact_rows: "3".to_owned(),
            ingest_contact_columns: "4".to_owned(),
            ingest_contact_dpi: "300".to_owned(),
            ingest_facial_profile: "quality+dedupe+identity".to_owned(),
            ingest_status:
                "Ingest ready: stage dataset metadata, triage loaded rows, contact sheet settings, and Facial profile hints."
                    .to_owned(),
            ingest_item_decisions: BTreeMap::new(),
            ingest_persisted_item_ids: BTreeSet::new(),
            ingest_apply_pending: false,
            ingest_apply_request_id: None,
            ingest_apply_batch_id: None,
            ingest_last_apply_receipt: "No backend apply receipt yet.".to_owned(),
        }
    }
}

fn posekit_state_readout(state: &AtelierPanelState) -> String {
    let rig_id = posekit_optional_rig_id(&state.pose_rig_id).unwrap_or_else(|| "<none>".to_owned());
    let framing = posekit_framing_readout(state);
    format!(
        "source_ref={} rig_id={} yaw_deg={:.0} pitch_deg={:.0} zoom={:.2} markers={} staged_marker_edits={} {}",
        state.pose_source_ref,
        rig_id,
        state.pose_yaw,
        state.pose_pitch,
        state.pose_zoom,
        marker_layer_summary(state.pose_face, state.pose_body, state.pose_hands),
        state.pose_marker_edits.len(),
        framing
    )
}

fn posekit_optional_rig_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn marker_layer_summary(face: bool, body: bool, hands: bool) -> String {
    format!(
        "face:{} body:{} hands:{}",
        if face { "on" } else { "off" },
        if body { "on" } else { "off" },
        if hands { "on" } else { "off" }
    )
}

fn posekit_framing_preset(value: &str) -> String {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "standard" => "standard".to_owned(),
        "full_body_with_feet" | "full_body" | "feet" => "full_body_with_feet".to_owned(),
        "portrait" => "portrait".to_owned(),
        "custom" => "custom".to_owned(),
        _ => "custom".to_owned(),
    }
}

fn posekit_framing_json_from_state(state: &AtelierPanelState) -> serde_json::Value {
    serde_json::json!({
        "preset": posekit_framing_preset(&state.pose_framing_preset),
        "lens_mm": state.pose_framing_lens_mm.clamp(18, 120),
        "padding_top_px": state.pose_framing_padding_top_px.clamp(0, 256),
        "padding_right_px": state.pose_framing_padding_right_px.clamp(0, 256),
        "padding_bottom_px": state.pose_framing_padding_bottom_px.clamp(0, 256),
        "padding_left_px": state.pose_framing_padding_left_px.clamp(0, 256),
    })
}

fn posekit_framing_readout(state: &AtelierPanelState) -> String {
    format!(
        "framing preset={} lens_mm={} top={} right={} bottom={} left={}",
        posekit_framing_preset(&state.pose_framing_preset),
        state.pose_framing_lens_mm.clamp(18, 120),
        state.pose_framing_padding_top_px.clamp(0, 256),
        state.pose_framing_padding_right_px.clamp(0, 256),
        state.pose_framing_padding_bottom_px.clamp(0, 256),
        state.pose_framing_padding_left_px.clamp(0, 256)
    )
}

fn posekit_marker_family(value: &str) -> Result<String, String> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "body" | "pose" => Ok("body".to_owned()),
        "face" | "facial" => Ok("face".to_owned()),
        "left_hand" | "lefthand" | "left" => Ok("left_hand".to_owned()),
        "right_hand" | "righthand" | "right" => Ok("right_hand".to_owned()),
        other => Err(format!(
            "unknown marker family '{other}'; use body, face, left_hand, or right_hand"
        )),
    }
}

fn posekit_marker_family_count(family: &str) -> usize {
    match family {
        "body" => POSEKIT_BODY_KEYPOINT_COUNT,
        "face" => POSEKIT_FACE_KEYPOINT_COUNT,
        "left_hand" | "right_hand" => POSEKIT_HAND_KEYPOINT_COUNT,
        _ => 0,
    }
}

fn posekit_marker_family_enabled(state: &AtelierPanelState, family: &str) -> bool {
    match family {
        "body" => state.pose_body,
        "face" => state.pose_face,
        "left_hand" | "right_hand" => state.pose_hands,
        _ => false,
    }
}

fn posekit_generated_marker_slot_is_zero(
    state: &AtelierPanelState,
    family: &str,
    index: usize,
) -> bool {
    let yaw = state.pose_yaw.clamp(-180.0, 180.0);
    let pitch = state.pose_pitch.clamp(-45.0, 45.0);
    let zoom = state.pose_zoom.clamp(0.4, 2.2);
    let keypoints = match family {
        "body" => posekit_body_keypoints(yaw, pitch, zoom, state.pose_body),
        "face" => posekit_face_keypoints(yaw, pitch, zoom, state.pose_face),
        "left_hand" => posekit_hand_keypoints(yaw, pitch, zoom, state.pose_hands, -1.0),
        "right_hand" => posekit_hand_keypoints(yaw, pitch, zoom, state.pose_hands, 1.0),
        _ => return false,
    };
    let offset = index.saturating_mul(3);
    if offset + 2 >= keypoints.len() {
        return false;
    }
    keypoints[offset] == 0.0 && keypoints[offset + 1] == 0.0 && keypoints[offset + 2] == 0.0
}

#[derive(Clone, Copy)]
struct PosekitMarkerSlotState {
    zero: bool,
    locally_mutated: bool,
}

fn posekit_staged_marker_slot_state(
    state: &AtelierPanelState,
    family: &str,
    index: usize,
) -> PosekitMarkerSlotState {
    let mut slot_state = PosekitMarkerSlotState {
        zero: posekit_generated_marker_slot_is_zero(state, family, index),
        locally_mutated: false,
    };
    for edit in &state.pose_marker_edits {
        if edit.family != family || edit.index != index {
            continue;
        }
        slot_state = match edit.action.as_str() {
            "remove" => PosekitMarkerSlotState {
                zero: true,
                locally_mutated: true,
            },
            "set" | "add" => PosekitMarkerSlotState {
                zero: false,
                locally_mutated: true,
            },
            _ => slot_state,
        };
    }
    slot_state
}

fn posekit_validate_marker_edit(
    state: &AtelierPanelState,
    action: &str,
) -> Result<PosekitMarkerEditRecord, String> {
    let family = posekit_marker_family(&state.pose_marker_family)?;
    if !posekit_marker_family_enabled(state, &family) {
        return Err(format!(
            "{family} marker layer is disabled; enable the layer before editing it"
        ));
    }
    let index = usize::try_from(state.pose_marker_index)
        .map_err(|_| "marker index must be zero or greater".to_owned())?;
    let limit = posekit_marker_family_count(&family);
    if index >= limit {
        return Err(format!(
            "{family}[{index}] is outside the supported 0..{} range",
            limit.saturating_sub(1)
        ));
    }
    let action = action.to_owned();
    if action == "remove" {
        return Ok(PosekitMarkerEditRecord {
            family,
            index,
            action,
            x: None,
            y: None,
            confidence: None,
        });
    }
    if !matches!(action.as_str(), "set" | "add") {
        return Err(format!("unsupported marker action '{action}'"));
    }
    let backend_rig_will_validate_source_slot =
        posekit_optional_rig_id(&state.pose_rig_id).is_some();
    let slot_state = posekit_staged_marker_slot_state(state, &family, index);
    if action == "add"
        && !slot_state.zero
        && !(backend_rig_will_validate_source_slot && !slot_state.locally_mutated)
    {
        return Err(format!(
            "add would overwrite existing {family}[{index}]; use apply for existing markers, remove it first, or bind a stored rig so the backend can validate the source slot"
        ));
    }
    for (label, value) in [("x", state.pose_marker_x), ("y", state.pose_marker_y)] {
        if !value.is_finite() || !(0.0..=POSEKIT_EXPORT_WIDTH as f32).contains(&value) {
            return Err(format!(
                "{label} must be a finite coordinate inside the 768px canvas"
            ));
        }
    }
    if !state.pose_marker_confidence.is_finite()
        || !(0.0..=1.0).contains(&state.pose_marker_confidence)
    {
        return Err("confidence must be finite and between 0.0 and 1.0".to_owned());
    }
    Ok(PosekitMarkerEditRecord {
        family,
        index,
        action,
        x: Some((state.pose_marker_x * 10.0).round() / 10.0),
        y: Some((state.pose_marker_y * 10.0).round() / 10.0),
        confidence: Some((state.pose_marker_confidence * 100.0).round() / 100.0),
    })
}

fn posekit_stage_marker_edit(state: &mut AtelierPanelState, action: &str) {
    match posekit_validate_marker_edit(state, action) {
        Ok(edit) => {
            let verb = match edit.action.as_str() {
                "add" => "Added marker edit",
                "remove" => "Removed marker",
                _ => "Applied marker edit",
            };
            state.pose_marker_status = format!("{verb} {}[{}]", edit.family, edit.index);
            state.pose_marker_edits.push(edit);
        }
        Err(err) => {
            state.pose_marker_status = format!("Marker edit rejected: {err}");
        }
    }
}

fn posekit_validate_staged_marker_edits_for_export(
    state: &AtelierPanelState,
    allow_backend_source_validation: bool,
) -> Result<(), String> {
    let backend_can_validate_source_slot =
        allow_backend_source_validation && posekit_optional_rig_id(&state.pose_rig_id).is_some();
    let mut marker_slot_state: std::collections::BTreeMap<(String, usize), PosekitMarkerSlotState> =
        std::collections::BTreeMap::new();

    for edit in &state.pose_marker_edits {
        let family = posekit_marker_family(&edit.family)?;
        if !posekit_marker_family_enabled(state, &family) {
            return Err(format!(
                "staged marker edit {}[{}] targets a disabled marker layer",
                family, edit.index
            ));
        }
        let limit = posekit_marker_family_count(&family);
        if edit.index >= limit {
            return Err(format!(
                "staged marker edit {}[{}] is outside the supported 0..{} range",
                family,
                edit.index,
                limit.saturating_sub(1)
            ));
        }
        let key = (family.clone(), edit.index);
        let slot_state =
            *marker_slot_state
                .entry(key.clone())
                .or_insert_with(|| PosekitMarkerSlotState {
                    zero: posekit_generated_marker_slot_is_zero(state, &family, edit.index),
                    locally_mutated: false,
                });

        match edit.action.as_str() {
            "remove" => {
                marker_slot_state.insert(
                    key,
                    PosekitMarkerSlotState {
                        zero: true,
                        locally_mutated: true,
                    },
                );
            }
            "set" => {
                posekit_validate_staged_marker_payload(edit)?;
                marker_slot_state.insert(
                    key,
                    PosekitMarkerSlotState {
                        zero: false,
                        locally_mutated: true,
                    },
                );
            }
            "add" => {
                posekit_validate_staged_marker_payload(edit)?;
                if !slot_state.zero
                    && !(backend_can_validate_source_slot && !slot_state.locally_mutated)
                {
                    return Err(format!(
                        "staged marker add {}[{}] needs an empty local slot or backend source validation",
                        family, edit.index
                    ));
                }
                marker_slot_state.insert(
                    key,
                    PosekitMarkerSlotState {
                        zero: false,
                        locally_mutated: true,
                    },
                );
            }
            other => {
                return Err(format!("unsupported staged marker action '{other}'"));
            }
        }
    }

    Ok(())
}

fn posekit_validate_staged_marker_payload(edit: &PosekitMarkerEditRecord) -> Result<(), String> {
    let Some(x) = edit.x else {
        return Err(format!(
            "staged marker edit {}[{}] is missing x",
            edit.family, edit.index
        ));
    };
    let Some(y) = edit.y else {
        return Err(format!(
            "staged marker edit {}[{}] is missing y",
            edit.family, edit.index
        ));
    };
    let Some(confidence) = edit.confidence else {
        return Err(format!(
            "staged marker edit {}[{}] is missing confidence",
            edit.family, edit.index
        ));
    };
    for (label, value, max) in [
        ("x", x, POSEKIT_EXPORT_WIDTH as f32),
        ("y", y, POSEKIT_EXPORT_HEIGHT as f32),
    ] {
        if !value.is_finite() || !(0.0..=max).contains(&value) {
            return Err(format!(
                "staged marker edit {}[{}] has invalid {label}; coordinates must stay inside the 768px canvas",
                edit.family, edit.index
            ));
        }
    }
    if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
        return Err(format!(
            "staged marker edit {}[{}] has invalid confidence",
            edit.family, edit.index
        ));
    }
    Ok(())
}

fn posekit_staged_edits_target_family(state: &AtelierPanelState, family: &str) -> bool {
    state
        .pose_marker_edits
        .iter()
        .any(|edit| edit.family == family)
}

fn posekit_warn_for_disabled_staged_marker_edits(state: &mut AtelierPanelState) {
    let mut disabled_families = Vec::new();
    if !state.pose_body && posekit_staged_edits_target_family(state, "body") {
        disabled_families.push("body");
    }
    if !state.pose_face && posekit_staged_edits_target_family(state, "face") {
        disabled_families.push("face");
    }
    if !state.pose_hands
        && (posekit_staged_edits_target_family(state, "left_hand")
            || posekit_staged_edits_target_family(state, "right_hand"))
    {
        disabled_families.push("hands");
    }
    if disabled_families.is_empty() {
        return;
    }
    state.pose_marker_status = format!(
        "Staged marker edits target disabled {} layer(s); re-enable the layer or clear edits before export.",
        disabled_families.join(", ")
    );
}

fn posekit_nudge_marker(state: &mut AtelierPanelState, dx: f32, dy: f32) {
    state.pose_marker_x = (state.pose_marker_x + dx).clamp(0.0, POSEKIT_EXPORT_WIDTH as f32);
    state.pose_marker_y = (state.pose_marker_y + dy).clamp(0.0, POSEKIT_EXPORT_HEIGHT as f32);
    state.pose_marker_status = format!(
        "Nudged marker candidate to x={:.1} y={:.1}; apply to stage it.",
        state.pose_marker_x, state.pose_marker_y
    );
}

fn posekit_marker_edits_json(edits: &[PosekitMarkerEditRecord]) -> Vec<serde_json::Value> {
    edits
        .iter()
        .map(|edit| {
            serde_json::json!({
                "family": edit.family.as_str(),
                "index": edit.index,
                "action": edit.action.as_str(),
                "x": edit.x,
                "y": edit.y,
                "confidence": edit.confidence,
            })
        })
        .collect()
}

fn posekit_export_snapshot(state: &AtelierPanelState) -> Result<PosekitExportSnapshot, String> {
    posekit_validate_staged_marker_edits_for_export(state, false)?;
    let yaw = state.pose_yaw.clamp(-180.0, 180.0);
    let pitch = state.pose_pitch.clamp(-45.0, 45.0);
    let zoom = state.pose_zoom.clamp(0.4, 2.2);
    let source_ref = if state.pose_source_ref.trim().is_empty() {
        "atelier://posekit/blank-source".to_owned()
    } else {
        state.pose_source_ref.trim().to_owned()
    };
    let rig_id = posekit_optional_rig_id(&state.pose_rig_id);
    let marker_edits = posekit_marker_edits_json(&state.pose_marker_edits);
    let framing = posekit_framing_json_from_state(state);
    let openpose_json = posekit_openpose_json(
        &source_ref,
        rig_id.as_deref(),
        yaw,
        pitch,
        zoom,
        state.pose_face,
        state.pose_body,
        state.pose_hands,
        &marker_edits,
        &framing,
    );
    posekit_validate_local_openpose_export(&openpose_json, state.pose_body)?;
    let hash_basis = format!(
        "{}|{}|{yaw:.0}|{pitch:.0}|{zoom:.2}|{}|{}|{}|{}|{}",
        source_ref,
        rig_id.as_deref().unwrap_or("<none>"),
        state.pose_face,
        state.pose_body,
        state.pose_hands,
        framing,
        openpose_json
    );
    let content_hash = stable_posekit_hash(&hash_basis);
    let png_artifact_ref = format!("preview://atelier/posekit/openpose/{content_hash}/png/payload");
    let png_manifest_ref =
        format!("preview://atelier/posekit/openpose/{content_hash}/png/manifest");
    let json_artifact_ref =
        format!("preview://atelier/posekit/openpose/{content_hash}/json/payload");
    let json_manifest_ref =
        format!("preview://atelier/posekit/openpose/{content_hash}/json/manifest");
    let receipt_ref = format!("preview://atelier/posekit/openpose/{content_hash}/receipt");
    Ok(PosekitExportSnapshot {
        source_ref,
        rig_id,
        yaw_deg: yaw,
        pitch_deg: pitch,
        zoom,
        face: state.pose_face,
        body: state.pose_body,
        hands: state.pose_hands,
        png_artifact_ref,
        png_manifest_ref,
        json_artifact_ref,
        json_manifest_ref,
        receipt_ref,
        content_hash,
        openpose_json,
        framing,
        applied_marker_edit_count: state.pose_marker_edits.len(),
    })
}

fn posekit_export_snapshot_from_backend(row: AtelierPosekitExportRow) -> PosekitExportSnapshot {
    PosekitExportSnapshot {
        source_ref: row.source_ref,
        rig_id: row.rig_id,
        yaw_deg: row.yaw_deg as f32,
        pitch_deg: row.pitch_deg as f32,
        zoom: row.zoom_percent as f32 / 100.0,
        face: row.marker_layers.face,
        body: row.marker_layers.body,
        hands: row.marker_layers.hands,
        png_artifact_ref: row.openpose_png_artifact.artifact_ref,
        png_manifest_ref: row.openpose_png_artifact.manifest_ref,
        json_artifact_ref: row.openpose_json_artifact.artifact_ref,
        json_manifest_ref: row.openpose_json_artifact.manifest_ref,
        receipt_ref: row.receipt_ref,
        content_hash: row.content_hash,
        openpose_json: row.openpose_json,
        framing: row.framing,
        applied_marker_edit_count: row.applied_marker_edit_count,
    }
}

fn posekit_openpose_json(
    source_ref: &str,
    rig_id: Option<&str>,
    yaw_deg: f32,
    pitch_deg: f32,
    zoom: f32,
    face: bool,
    body: bool,
    hands: bool,
    marker_edits: &[serde_json::Value],
    framing: &serde_json::Value,
) -> serde_json::Value {
    let mut openpose = serde_json::json!({
        "version": 1.3,
        "handshake_schema": "hsk.atelier.posekit.openpose_export@1",
        "preview_only": true,
        "source_ref": source_ref,
        "rig_id": rig_id,
        "canvas": {
            "width": POSEKIT_EXPORT_WIDTH,
            "height": POSEKIT_EXPORT_HEIGHT,
        },
        "pose_state": {
            "yaw_deg": yaw_deg.round(),
            "pitch_deg": pitch_deg.round(),
            "zoom": ((zoom * 100.0).round() / 100.0),
            "zoom_percent": (zoom * 100.0).round(),
            "marker_layers": {
                "face": face,
                "body": body,
                "hands": hands,
            },
            "marker_edits": marker_edits,
            "framing": framing,
        },
        "people": [{
            "pose_keypoints_2d": posekit_body_keypoints(yaw_deg, pitch_deg, zoom, body),
            "face_keypoints_2d": posekit_face_keypoints(yaw_deg, pitch_deg, zoom, face),
            "hand_left_keypoints_2d": posekit_hand_keypoints(yaw_deg, pitch_deg, zoom, hands, -1.0),
            "hand_right_keypoints_2d": posekit_hand_keypoints(yaw_deg, pitch_deg, zoom, hands, 1.0),
        }],
    });
    posekit_apply_framing_to_openpose(&mut openpose, framing);
    posekit_apply_marker_edits_to_openpose(&mut openpose, marker_edits);
    openpose
}

fn posekit_validate_local_openpose_export(
    openpose: &serde_json::Value,
    body_enabled: bool,
) -> Result<(), String> {
    let mut visible = 0usize;
    let body_visible = posekit_validate_local_openpose_field(
        openpose,
        "pose_keypoints_2d",
        POSEKIT_BODY_KEYPOINT_COUNT,
    )?;
    visible += body_visible;
    if body_enabled && body_visible == 0 {
        return Err(
            "Posekit body export cannot be all-zero after marker edits and framing".to_owned(),
        );
    }
    visible += posekit_validate_local_openpose_field(
        openpose,
        "face_keypoints_2d",
        POSEKIT_FACE_KEYPOINT_COUNT,
    )?;
    visible += posekit_validate_local_openpose_field(
        openpose,
        "hand_left_keypoints_2d",
        POSEKIT_HAND_KEYPOINT_COUNT,
    )?;
    visible += posekit_validate_local_openpose_field(
        openpose,
        "hand_right_keypoints_2d",
        POSEKIT_HAND_KEYPOINT_COUNT,
    )?;
    if visible == 0 {
        return Err(
            "Posekit OpenPose export would be blank after marker edits and framing".to_owned(),
        );
    }
    Ok(())
}

fn posekit_validate_local_openpose_field(
    openpose: &serde_json::Value,
    field: &str,
    expected_count: usize,
) -> Result<usize, String> {
    let Some(points) = openpose
        .get("people")
        .and_then(serde_json::Value::as_array)
        .and_then(|people| people.first())
        .and_then(|person| person.get(field))
        .and_then(serde_json::Value::as_array)
    else {
        return Err(format!("Posekit OpenPose field {field} is missing"));
    };
    if points.len() != expected_count.saturating_mul(3) {
        return Err(format!(
            "Posekit OpenPose field {field} has {} values but expected {}",
            points.len(),
            expected_count.saturating_mul(3)
        ));
    }
    let mut visible = 0usize;
    for triple in points.chunks_exact(3) {
        let x = posekit_json_number(&triple[0], field)?;
        let y = posekit_json_number(&triple[1], field)?;
        let confidence = posekit_json_number(&triple[2], field)?;
        if !(0.0..=1.0).contains(&confidence) {
            return Err(format!(
                "Posekit OpenPose field {field} confidence must be in 0..=1"
            ));
        }
        if confidence <= 0.0 {
            continue;
        }
        if x < 0.0 || y < 0.0 || x > POSEKIT_EXPORT_WIDTH as f64 || y > POSEKIT_EXPORT_HEIGHT as f64
        {
            return Err(format!(
                "Posekit OpenPose field {field} has a visible point outside the export canvas"
            ));
        }
        visible = visible.saturating_add(1);
    }
    Ok(visible)
}

fn posekit_json_number(value: &serde_json::Value, field: &str) -> Result<f64, String> {
    let Some(number) = value.as_f64() else {
        return Err(format!(
            "Posekit OpenPose field {field} contains a non-number"
        ));
    };
    if !number.is_finite() {
        return Err(format!(
            "Posekit OpenPose field {field} contains non-finite values"
        ));
    }
    Ok(number)
}

fn posekit_apply_framing_to_openpose(
    openpose: &mut serde_json::Value,
    framing: &serde_json::Value,
) {
    let lens_mm = framing
        .get("lens_mm")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(50.0)
        .clamp(18.0, 120.0) as f32;
    let padding_top = posekit_padding_from_framing(framing, "padding_top_px");
    let padding_right = posekit_padding_from_framing(framing, "padding_right_px");
    let padding_bottom = posekit_padding_from_framing(framing, "padding_bottom_px");
    let padding_left = posekit_padding_from_framing(framing, "padding_left_px");
    let content_width = (POSEKIT_EXPORT_WIDTH as f32 - padding_left - padding_right).max(128.0);
    let content_height = (POSEKIT_EXPORT_HEIGHT as f32 - padding_top - padding_bottom).max(128.0);
    let source_center_x = POSEKIT_EXPORT_WIDTH as f32 * 0.5;
    let source_center_y = POSEKIT_EXPORT_HEIGHT as f32 * 0.5;
    let content_center_x = padding_left + content_width * 0.5;
    let content_center_y = padding_top + content_height * 0.5;
    let lens_scale = lens_mm / 50.0;

    let Some(person) = openpose
        .get_mut("people")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|people| people.first_mut())
    else {
        return;
    };
    for field in [
        "pose_keypoints_2d",
        "face_keypoints_2d",
        "hand_left_keypoints_2d",
        "hand_right_keypoints_2d",
    ] {
        let Some(points) = person
            .get_mut(field)
            .and_then(serde_json::Value::as_array_mut)
        else {
            continue;
        };
        let mut offset = 0;
        while offset + 2 < points.len() {
            let x = points[offset].as_f64().unwrap_or(0.0) as f32;
            let y = points[offset + 1].as_f64().unwrap_or(0.0) as f32;
            let confidence = points[offset + 2].as_f64().unwrap_or(0.0) as f32;
            if confidence > 0.0 {
                let framed_x = content_center_x + (x - source_center_x) * lens_scale;
                let framed_y = content_center_y + (y - source_center_y) * lens_scale;
                points[offset] = posekit_json_f32(framed_x);
                points[offset + 1] = posekit_json_f32(framed_y);
            }
            offset += 3;
        }
    }
}

fn posekit_apply_marker_edits_to_openpose(
    openpose: &mut serde_json::Value,
    marker_edits: &[serde_json::Value],
) {
    let Some(person) = openpose
        .get_mut("people")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|people| people.first_mut())
    else {
        return;
    };
    for edit in marker_edits {
        let family = edit
            .get("family")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let action = edit
            .get("action")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("set");
        let Some(index) = edit
            .get("index")
            .and_then(serde_json::Value::as_u64)
            .map(|value| value as usize)
        else {
            continue;
        };
        let Some(points) = posekit_keypoint_array_mut(person, family) else {
            continue;
        };
        let offset = index.saturating_mul(3);
        if offset + 2 >= points.len() {
            continue;
        }
        if action == "remove" {
            points[offset] = serde_json::json!(0.0);
            points[offset + 1] = serde_json::json!(0.0);
            points[offset + 2] = serde_json::json!(0.0);
            continue;
        }
        if action == "add" && !posekit_json_marker_slot_is_zero(points, offset) {
            continue;
        }
        let Some(x) = edit.get("x").and_then(serde_json::Value::as_f64) else {
            continue;
        };
        let Some(y) = edit.get("y").and_then(serde_json::Value::as_f64) else {
            continue;
        };
        let Some(confidence) = edit.get("confidence").and_then(serde_json::Value::as_f64) else {
            continue;
        };
        points[offset] = posekit_json_f32(x as f32);
        points[offset + 1] = posekit_json_f32(y as f32);
        points[offset + 2] = posekit_json_confidence(confidence as f32);
    }
}

fn posekit_padding_from_framing(framing: &serde_json::Value, field: &str) -> f32 {
    framing
        .get(field)
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 256.0) as f32
}

fn posekit_keypoint_array_mut<'a>(
    person: &'a mut serde_json::Value,
    family: &str,
) -> Option<&'a mut Vec<serde_json::Value>> {
    let field = match family {
        "body" => "pose_keypoints_2d",
        "face" => "face_keypoints_2d",
        "left_hand" => "hand_left_keypoints_2d",
        "right_hand" => "hand_right_keypoints_2d",
        _ => return None,
    };
    person
        .get_mut(field)
        .and_then(serde_json::Value::as_array_mut)
}

fn posekit_json_marker_slot_is_zero(points: &[serde_json::Value], offset: usize) -> bool {
    points
        .get(offset)
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
        == 0.0
        && points
            .get(offset + 1)
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
            == 0.0
        && points
            .get(offset + 2)
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
            == 0.0
}

fn posekit_json_f32(value: f32) -> serde_json::Value {
    serde_json::json!((value * 10.0).round() / 10.0)
}

fn posekit_json_confidence(value: f32) -> serde_json::Value {
    serde_json::json!((value * 100.0).round() / 100.0)
}

fn posekit_body_keypoints(yaw_deg: f32, pitch_deg: f32, zoom: f32, visible: bool) -> Vec<f32> {
    if !visible {
        return zero_keypoints(POSEKIT_BODY_KEYPOINT_COUNT);
    }
    let center_x = POSEKIT_EXPORT_WIDTH as f32 * 0.5 + yaw_deg / 180.0 * 72.0;
    let center_y = POSEKIT_EXPORT_HEIGHT as f32 * 0.51 + pitch_deg / 45.0 * 42.0;
    let scale = zoom.clamp(0.4, 2.2);
    let yaw_bias = yaw_deg / 180.0;
    let shoulder = 86.0 * scale * (1.0 - yaw_bias.abs() * 0.28);
    let hip = 52.0 * scale * (1.0 - yaw_bias.abs() * 0.18);
    let points = [
        (center_x, center_y - 170.0 * scale, 0.95),
        (center_x, center_y - 102.0 * scale, 0.94),
        (center_x - shoulder, center_y - 92.0 * scale, 0.91),
        (
            center_x - shoulder - 54.0 * scale,
            center_y - 34.0 * scale,
            0.86,
        ),
        (
            center_x - shoulder - 70.0 * scale,
            center_y + 34.0 * scale,
            0.82,
        ),
        (center_x + shoulder, center_y - 92.0 * scale, 0.91),
        (
            center_x + shoulder + 54.0 * scale,
            center_y - 34.0 * scale,
            0.86,
        ),
        (
            center_x + shoulder + 70.0 * scale,
            center_y + 34.0 * scale,
            0.82,
        ),
        (center_x - hip, center_y + 46.0 * scale, 0.90),
        (
            center_x - hip - 22.0 * scale,
            center_y + 142.0 * scale,
            0.86,
        ),
        (
            center_x - hip - 18.0 * scale,
            center_y + 238.0 * scale,
            0.82,
        ),
        (center_x + hip, center_y + 46.0 * scale, 0.90),
        (
            center_x + hip + 22.0 * scale,
            center_y + 142.0 * scale,
            0.86,
        ),
        (
            center_x + hip + 18.0 * scale,
            center_y + 238.0 * scale,
            0.82,
        ),
        (
            center_x - 18.0 * scale - yaw_bias * 8.0,
            center_y - 180.0 * scale,
            0.80,
        ),
        (
            center_x + 18.0 * scale - yaw_bias * 8.0,
            center_y - 180.0 * scale,
            0.80,
        ),
        (
            center_x - 42.0 * scale - yaw_bias * 10.0,
            center_y - 164.0 * scale,
            0.76,
        ),
        (
            center_x + 42.0 * scale - yaw_bias * 10.0,
            center_y - 164.0 * scale,
            0.76,
        ),
    ];
    flatten_keypoints(&points)
}

fn posekit_face_keypoints(yaw_deg: f32, pitch_deg: f32, zoom: f32, visible: bool) -> Vec<f32> {
    if !visible {
        return zero_keypoints(POSEKIT_FACE_KEYPOINT_COUNT);
    }
    let center_x = POSEKIT_EXPORT_WIDTH as f32 * 0.5 + yaw_deg / 180.0 * 72.0;
    let center_y = POSEKIT_EXPORT_HEIGHT as f32 * 0.51 + pitch_deg / 45.0 * 42.0 - 170.0 * zoom;
    let scale = zoom.clamp(0.4, 2.2);
    let yaw_bias = yaw_deg / 180.0;
    let mut points = Vec::with_capacity(POSEKIT_FACE_KEYPOINT_COUNT);
    for index in 0..POSEKIT_FACE_KEYPOINT_COUNT {
        let theta = index as f32 / POSEKIT_FACE_KEYPOINT_COUNT as f32 * std::f32::consts::TAU;
        let x = center_x
            + theta.cos() * 34.0 * scale * (1.0 - yaw_bias.abs() * 0.32)
            + yaw_bias * 14.0 * scale;
        let y = center_y + theta.sin() * 45.0 * scale;
        points.push((x, y, 0.78));
    }
    flatten_keypoints(&points)
}

fn posekit_hand_keypoints(
    yaw_deg: f32,
    pitch_deg: f32,
    zoom: f32,
    visible: bool,
    side: f32,
) -> Vec<f32> {
    if !visible {
        return zero_keypoints(POSEKIT_HAND_KEYPOINT_COUNT);
    }
    let center_x = POSEKIT_EXPORT_WIDTH as f32 * 0.5
        + yaw_deg / 180.0 * 72.0
        + side * 158.0 * zoom.clamp(0.4, 2.2);
    let center_y = POSEKIT_EXPORT_HEIGHT as f32 * 0.51 + pitch_deg / 45.0 * 42.0 + 34.0 * zoom;
    let scale = zoom.clamp(0.4, 2.2);
    let mut points = Vec::with_capacity(POSEKIT_HAND_KEYPOINT_COUNT);
    for index in 0..POSEKIT_HAND_KEYPOINT_COUNT {
        let finger = (index / 4) as f32;
        let joint = (index % 4) as f32;
        points.push((
            center_x + side * (finger - 2.0) * 8.0 * scale,
            center_y - joint * 13.0 * scale - finger * 2.0 * scale,
            0.70,
        ));
    }
    flatten_keypoints(&points)
}

fn flatten_keypoints(points: &[(f32, f32, f32)]) -> Vec<f32> {
    let mut flattened = Vec::with_capacity(points.len() * 3);
    for (x, y, confidence) in points {
        flattened.push((x * 10.0).round() / 10.0);
        flattened.push((y * 10.0).round() / 10.0);
        flattened.push((confidence * 100.0).round() / 100.0);
    }
    flattened
}

fn zero_keypoints(count: usize) -> Vec<f32> {
    vec![0.0; count * 3]
}

fn stable_posekit_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

fn posekit_export_preview(snapshot: &PosekitExportSnapshot) -> String {
    let rig_id = snapshot.rig_id.as_deref().unwrap_or("<none>");
    format!(
        "schema=hsk.atelier.posekit.openpose_export@1\nsource_ref={}\nrig_id={}\nyaw_deg={:.0}\npitch_deg={:.0}\nzoom={:.2}\nmarkers={}\napplied_marker_edit_count={}\nframing={}\npng_artifact_ref={}\npng_manifest_ref={}\njson_artifact_ref={}\njson_manifest_ref={}\nreceipt_ref={}\ncontent_hash={}\npng_mime=image/png\njson_mime=application/json\nopenpose_json={}",
        snapshot.source_ref,
        rig_id,
        snapshot.yaw_deg,
        snapshot.pitch_deg,
        snapshot.zoom,
        snapshot.marker_layers(),
        snapshot.applied_marker_edit_count,
        snapshot.framing,
        snapshot.png_artifact_ref,
        snapshot.png_manifest_ref,
        snapshot.json_artifact_ref,
        snapshot.json_manifest_ref,
        snapshot.receipt_ref,
        snapshot.content_hash,
        snapshot.openpose_json
    )
}

fn seeded_ckc_characters() -> Vec<CkcCharacterRecord> {
    vec![
        CkcCharacterRecord {
            public_id: "mira-demo".to_owned(),
            display_name: "Mira Demo".to_owned(),
            character_internal_id: "018f7848-1111-7000-9000-000000000001".to_owned(),
            character_ref: "atelier://character/018f7848-1111-7000-9000-000000000001".to_owned(),
            sheet_version_id: Some("018f7848-1111-7000-9000-000000000101".to_owned()),
            parent_sheet_version_id: None,
            sheet_seq: 1,
            sheet_editor_text: "CHAR-ID-001 — Character_ID: mira-demo\nCHAR-ID-002 — Name: Mira Demo\nCHAR-ID-006 — Primary_Role: reusable character/avatar\nPIPELINES\npipelines: ComfyUI, Unreal, Blender\nnotes: seed CKC sheet for Argus and model workflow proof".to_owned(),
            sheet_version_ref: Some(
                "atelier://sheet/018f7848-1111-7000-9000-000000000001/018f7848-1111-7000-9000-000000000101".to_owned(),
            ),
            sheet_artifact_links: vec![seeded_ckc_sheet_artifact_link(
                "018f7848-1111-7000-9000-00000000e001",
                "018f7848-1111-7000-9000-000000000001",
                "018f7848-1111-7000-9000-000000000101",
                "openpose_png",
                "artifact://atelier/posekit/openpose/mira-demo-yaw45.png",
                "manifest://atelier/posekit/openpose/mira-demo-yaw45",
                "posekit://rig/mira-demo-yaw45",
                "Mira yaw +45 OpenPose",
                "cui_openpose_conditioning",
            )],
            media_albums: vec![
                seeded_ckc_media_album(
                    "018f7848-1111-7000-9000-00000000a001",
                    "Mira reference album",
                    "018f7848-1111-7000-9000-00000000b001",
                    "mira-closeup-001.png",
                    "atelier://folder/mira-reference-set",
                    "https://example.invalid/reference/mira-reference-set",
                ),
                seeded_ckc_media_album(
                    "018f7848-1111-7000-9000-00000000a003",
                    "Mira expression set",
                    "018f7848-1111-7000-9000-00000000b003",
                    "mira-expression-002.png",
                    "atelier://folder/mira-expression-set",
                    "https://example.invalid/reference/mira-expression-set",
                ),
            ],
            story_documents: vec![
                seeded_ckc_story_document(
                    "018f7848-1111-7000-9000-00000000c001",
                    "Mira story bible",
                    "018f7848-1111-7000-9000-00000000c101",
                ),
                seeded_ckc_story_document(
                    "018f7848-1111-7000-9000-00000000c002",
                    "Mira production scenes",
                    "018f7848-1111-7000-9000-00000000c102",
                ),
            ],
            moodboard_documents: vec![
                seeded_ckc_moodboard_document(
                    "018f7848-1111-7000-9000-00000000d001",
                    "018f7848-1111-7000-9000-00000000d101",
                    "Mira visual continuity board",
                ),
                seeded_ckc_moodboard_document(
                    "018f7848-1111-7000-9000-00000000d003",
                    "018f7848-1111-7000-9000-00000000d103",
                    "Mira production moodboard",
                ),
            ],
        },
        CkcCharacterRecord {
            public_id: "aria-demo".to_owned(),
            display_name: "Aria Demo".to_owned(),
            character_internal_id: "018f7848-1111-7000-9000-000000000002".to_owned(),
            character_ref: "atelier://character/018f7848-1111-7000-9000-000000000002".to_owned(),
            sheet_version_id: Some("018f7848-1111-7000-9000-000000000201".to_owned()),
            parent_sheet_version_id: None,
            sheet_seq: 1,
            sheet_editor_text: "CHAR-ID-001 — Character_ID: aria-demo\nCHAR-ID-002 — Name: Aria Demo\nCHAR-ID-006 — Primary_Role: production avatar reference\nPIPELINES\npipelines: CKC albums, Posekit, ComfyUI\nnotes: second selectable sheet proves CKC is a database surface".to_owned(),
            sheet_version_ref: Some(
                "atelier://sheet/018f7848-1111-7000-9000-000000000002/018f7848-1111-7000-9000-000000000201".to_owned(),
            ),
            sheet_artifact_links: Vec::new(),
            media_albums: vec![seeded_ckc_media_album(
                "018f7848-1111-7000-9000-00000000a002",
                "Aria pose references",
                "018f7848-1111-7000-9000-00000000b002",
                "aria-pose-001.png",
                "atelier://folder/aria-pose-set",
                "https://example.invalid/reference/aria-pose-set",
            )],
            story_documents: vec![seeded_ckc_story_document(
                "018f7848-1111-7000-9000-00000000c002",
                "Aria production story",
                "018f7848-1111-7000-9000-00000000c102",
            )],
            moodboard_documents: vec![seeded_ckc_moodboard_document(
                "018f7848-1111-7000-9000-00000000d002",
                "018f7848-1111-7000-9000-00000000d102",
                "Aria pose board",
            )],
        },
    ]
}

fn seeded_ckc_sheet_artifact_link(
    link_id: &str,
    character_internal_id: &str,
    sheet_version_id: &str,
    artifact_kind: &str,
    artifact_ref: &str,
    manifest_ref: &str,
    source_ref: &str,
    label: &str,
    reuse_role: &str,
) -> CkcSheetArtifactLinkRecord {
    CkcSheetArtifactLinkRecord {
        link_id: link_id.to_owned(),
        character_internal_id: character_internal_id.to_owned(),
        character_ref: format!("atelier://character/{character_internal_id}"),
        sheet_version_id: sheet_version_id.to_owned(),
        sheet_version_ref: format!("atelier://sheet/{character_internal_id}/{sheet_version_id}"),
        typed_ref: format!("atelier://sheet-artifact/{link_id}"),
        artifact_kind: artifact_kind.to_owned(),
        artifact_ref: artifact_ref.to_owned(),
        manifest_ref: Some(manifest_ref.to_owned()),
        source_ref: Some(source_ref.to_owned()),
        label: Some(label.to_owned()),
        reuse_role: Some(reuse_role.to_owned()),
        linked_by: "seed".to_owned(),
        metadata: serde_json::json!({
            "seed": true,
            "sheet_version_id": sheet_version_id,
        }),
    }
}

fn seeded_ckc_media_album(
    collection_id: &str,
    name: &str,
    asset_id: &str,
    media_label: &str,
    source_path_ref: &str,
    source_url_ref: &str,
) -> CkcMediaAlbumRecord {
    CkcMediaAlbumRecord {
        collection_id: collection_id.to_owned(),
        collection_ref: format!("atelier://collection/{collection_id}"),
        name: name.to_owned(),
        description: "Seeded CKC linked-media album for Argus inspection and model workflows"
            .to_owned(),
        tags: vec!["reference".to_owned(), "training".to_owned()],
        member_count: 1,
        members_next_offset: None,
        members: vec![CkcMediaMemberRecord {
            asset_id: asset_id.to_owned(),
            media_ref: format!("atelier://media/{asset_id}"),
            display_label: media_label.to_owned(),
            source_path_ref: Some(source_path_ref.to_owned()),
            source_url_ref: Some(source_url_ref.to_owned()),
            notes: format!(
                "{media_label} image note stays separate from the character sheet notes"
            ),
            review_status: Some("approved".to_owned()),
            tags_buffer: "face, reference, approved".to_owned(),
        }],
    }
}

fn seeded_ckc_story_document(
    document_id: &str,
    title: &str,
    card_id: &str,
) -> CkcStoryDocumentRecord {
    let document_ref = format!("atelier://document/{document_id}");
    CkcStoryDocumentRecord {
        document_id: document_id.to_owned(),
        document_ref: document_ref.clone(),
        current_version_id: Some(format!("{document_id}-v1")),
        current_version_seq: 1,
        title: title.to_owned(),
        body_raw_text:
            "Story bible content stays separate from the character sheet and image notes."
                .to_owned(),
        tags: vec!["story".to_owned(), "continuity".to_owned()],
        cards: vec![CkcStoryCardRecord {
            card_id: card_id.to_owned(),
            card_ref: format!("atelier://story-card/{card_id}"),
            story_document_id: document_id.to_owned(),
            story_document_ref: document_ref.clone(),
            title: "Continuity card".to_owned(),
            body_raw_text:
                "Reusable character continuity card for ComfyUI, Unreal, Blender, and story work."
                    .to_owned(),
            tags: vec!["continuity".to_owned(), "reuse".to_owned()],
        }],
        beats: vec![CkcStoryBeatRecord {
            beat_id: format!("{card_id}-beat-001"),
            beat_ref: format!("atelier://story-beat/{card_id}-beat-001"),
            story_document_id: document_id.to_owned(),
            story_document_ref: document_ref,
            card_id: Some(card_id.to_owned()),
            beat_text: "Keep character intent, scene role, and asset reuse linked but editable."
                .to_owned(),
        }],
    }
}

fn seeded_ckc_moodboard_document(
    document_id: &str,
    snapshot_id: &str,
    title: &str,
) -> CkcMoodboardDocumentRecord {
    CkcMoodboardDocumentRecord {
        document_id: document_id.to_owned(),
        document_ref: format!("atelier://document/{document_id}"),
        current_version_id: Some(format!("{document_id}-v1")),
        current_version_seq: 1,
        title: title.to_owned(),
        body_raw_text: local_ckc_moodboard_snapshot_json(
            document_id,
            title,
            "Native Handshake moodboard linked to this character sheet for visual continuity.",
        ),
        tags: vec!["moodboard".to_owned(), "visual-reference".to_owned()],
        latest_snapshot_id: Some(snapshot_id.to_owned()),
        latest_snapshot_ref: Some(format!("atelier://moodboard/{snapshot_id}")),
        moodboard_name: title.to_owned(),
    }
}

fn pending_ckc_story_document(
    character_internal_id: &str,
    display_name: &str,
) -> CkcStoryDocumentRecord {
    let document_id = format!("pending-story-document-{character_internal_id}");
    CkcStoryDocumentRecord {
        document_id: document_id.clone(),
        document_ref: format!("atelier://document/{document_id}"),
        current_version_id: None,
        current_version_seq: 0,
        title: format!("{display_name} story"),
        body_raw_text: String::new(),
        tags: vec!["story".to_owned()],
        cards: Vec::new(),
        beats: Vec::new(),
    }
}

fn pending_ckc_moodboard_document(
    character_internal_id: &str,
    display_name: &str,
) -> CkcMoodboardDocumentRecord {
    let document_id = format!("pending-moodboard-document-{character_internal_id}");
    CkcMoodboardDocumentRecord {
        document_id: document_id.clone(),
        document_ref: format!("atelier://document/{document_id}"),
        current_version_id: None,
        current_version_seq: 0,
        title: format!("{display_name} moodboard"),
        body_raw_text: String::new(),
        tags: vec!["moodboard".to_owned()],
        latest_snapshot_id: None,
        latest_snapshot_ref: None,
        moodboard_name: format!("{display_name} moodboard"),
    }
}

fn local_ckc_moodboard_snapshot_json(moodboard_id: &str, name: &str, description: &str) -> String {
    let layer_id = Uuid::new_v4().to_string();
    let text_id = Uuid::new_v4().to_string();
    let history_id = Uuid::new_v4().to_string();
    serde_json::json!({
        "schema_id": "hsk.atelier.moodboard@1",
        "schema_version": 1,
        "moodboard_id": moodboard_id,
        "name": name,
        "description": description,
        "canvas": {
            "width": 1600.0,
            "height": 1000.0,
            "background_color": "#101418"
        },
        "layers": [{
            "layer_id": layer_id,
            "name": "CKC moodboard",
            "order": 1,
            "visible": true,
            "locked": false,
            "opacity": 1.0,
            "parent_layer_id": null
        }],
        "images": [],
        "text": [{
            "element_id": text_id,
            "layer_id": layer_id,
            "content": description,
            "font": "Inter",
            "font_size": 18.0,
            "color": "#f4f7fb",
            "position": { "x": 80.0, "y": 80.0 },
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
            "mood_keywords": ["ckc", "continuity"],
            "style_description": "Native CKC moodboard projection",
            "suggested_presets": []
        },
        "history": [{
            "history_id": history_id,
            "at": "2026-06-29T00:00:00Z",
            "actor": "handshake-native-atelier",
            "operation": "created",
            "summary": "Materialized local CKC moodboard snapshot"
        }]
    })
    .to_string()
}

fn is_pending_ckc_document_id(document_id: &str) -> bool {
    document_id.starts_with("pending-story-document-")
        || document_id.starts_with("pending-moodboard-document-")
}

fn slugify_public_id(label: &str, fallback_index: usize) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in label.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        format!("ckc-character-{fallback_index}")
    } else {
        out
    }
}

fn ckc_character_row_author_id(character_internal_id: &str) -> String {
    format!("atelier-ckc-character-{character_internal_id}")
}

pub fn ckc_media_album_row_author_id(collection_id: &str) -> String {
    format!(
        "atelier-ckc-album-{}",
        stable_author_id_suffix(collection_id)
    )
}

pub fn ckc_media_album_load_more_author_id(collection_id: &str) -> String {
    format!(
        "atelier-ckc-album-load-more-{}",
        stable_author_id_suffix(collection_id)
    )
}

pub fn ckc_sheet_artifact_row_author_id(link_id: &str) -> String {
    format!(
        "atelier-ckc-sheet-artifact-{}",
        stable_author_id_suffix(link_id)
    )
}

fn ckc_media_occurrence_key(collection_id: &str, asset_id: &str) -> String {
    format!("{collection_id}::{asset_id}")
}

pub fn ckc_media_row_author_id(collection_id: &str, asset_id: &str) -> String {
    format!(
        "atelier-ckc-media-{}-{}",
        stable_author_id_suffix(collection_id),
        stable_author_id_suffix(asset_id)
    )
}

pub fn ckc_folder_row_author_id(collection_id: &str, asset_id: &str, folder_ref: &str) -> String {
    format!(
        "atelier-ckc-folder-{}-{}-{}",
        stable_author_id_suffix(collection_id),
        stable_author_id_suffix(asset_id),
        stable_author_id_suffix(folder_ref)
    )
}

pub fn ckc_source_url_row_author_id(
    collection_id: &str,
    asset_id: &str,
    source_url_ref: &str,
) -> String {
    format!(
        "atelier-ckc-source-url-{}-{}-{}",
        stable_author_id_suffix(collection_id),
        stable_author_id_suffix(asset_id),
        stable_author_id_suffix(source_url_ref)
    )
}

pub fn ckc_search_result_row_author_id(target_ref: &str) -> String {
    format!(
        "atelier-ckc-search-result-{}",
        stable_author_id_suffix(target_ref)
    )
}

pub fn ckc_story_card_row_author_id(document_id: &str, card_id: &str) -> String {
    format!(
        "atelier-ckc-story-card-{}-{}",
        stable_author_id_suffix(document_id),
        stable_author_id_suffix(card_id)
    )
}

pub fn ckc_story_beat_row_author_id(document_id: &str, beat_id: &str) -> String {
    format!(
        "atelier-ckc-story-beat-{}-{}",
        stable_author_id_suffix(document_id),
        stable_author_id_suffix(beat_id)
    )
}

pub fn ckc_story_document_row_author_id(document_id: &str) -> String {
    format!(
        "atelier-ckc-story-document-{}",
        stable_author_id_suffix(document_id)
    )
}

pub fn ckc_moodboard_document_row_author_id(document_id: &str) -> String {
    format!(
        "atelier-ckc-moodboard-document-{}",
        stable_author_id_suffix(document_id)
    )
}

pub fn ckc_moodboard_snapshot_row_author_id(document_id: &str, snapshot_id: &str) -> String {
    format!(
        "atelier-ckc-moodboard-snapshot-{}-{}",
        stable_author_id_suffix(document_id),
        stable_author_id_suffix(snapshot_id)
    )
}

fn stable_author_id_suffix(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "ref".to_owned()
    } else {
        out
    }
}

fn ckc_tags_from_buffer(buffer: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for tag in buffer
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
    {
        let normalized = tag.to_ascii_lowercase();
        if !tags.iter().any(|existing| existing == &normalized) {
            tags.push(normalized);
        }
    }
    tags
}

fn ckc_asset_ids_from_buffer(buffer: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for raw in buffer
        .split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace())
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let id = raw
            .strip_prefix("atelier://media/")
            .unwrap_or(raw)
            .trim()
            .trim_matches('/');
        if !id.is_empty() && !ids.iter().any(|existing| existing == id) {
            ids.push(id.to_owned());
        }
    }
    ids
}

fn non_empty_trimmed(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn actor_id_or_default(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "local-atelier-panel".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn ckc_sheet_notes_source_key(character: &CkcCharacterRecord) -> String {
    let version = character
        .sheet_version_id
        .as_deref()
        .unwrap_or("draft-sheet-version");
    format!("{}:{version}", character.character_internal_id)
}

fn extract_ckc_sheet_notes(sheet_text: &str) -> String {
    sheet_text
        .lines()
        .find_map(|line| {
            let trimmed = line.trim_start();
            trimmed
                .strip_prefix("notes:")
                .map(|notes| notes.trim().to_owned())
        })
        .unwrap_or_default()
}

fn upsert_ckc_sheet_notes(sheet_text: &mut String, notes: &str) {
    let normalized_notes = notes
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    let mut found = false;
    let mut out = Vec::new();
    for line in sheet_text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("notes:") {
            let prefix_len = line.len() - trimmed.len();
            let prefix = &line[..prefix_len];
            out.push(format!("{prefix}notes: {normalized_notes}"));
            found = true;
        } else {
            out.push(line.to_owned());
        }
    }
    if !found {
        out.push(format!("notes: {normalized_notes}"));
    }
    *sheet_text = out.join("\n");
}

fn local_ckc_media_member(
    asset_id: &str,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
) -> CkcMediaMemberRecord {
    CkcMediaMemberRecord {
        asset_id: asset_id.to_owned(),
        media_ref: format!("atelier://media/{asset_id}"),
        display_label: format!("linked-media-{asset_id}"),
        source_path_ref,
        source_url_ref,
        notes: String::new(),
        review_status: Some("unreviewed".to_owned()),
        tags_buffer: String::new(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IngestDecision {
    Pass,
    Reject,
    Unsure,
}

impl IngestDecision {
    fn label(self) -> &'static str {
        match self {
            Self::Pass => "Pass",
            Self::Reject => "Reject",
            Self::Unsure => "Unsure",
        }
    }

    fn machine_label(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Reject => "reject",
            Self::Unsure => "unsure",
        }
    }

    fn backend_lane(self) -> &'static str {
        match self {
            Self::Pass => "accepted",
            Self::Reject => "rejected",
            Self::Unsure => "deferred",
        }
    }

    fn from_lane(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "accepted" | "accept" | "pass" => Self::Pass,
            "rejected" | "reject" => Self::Reject,
            _ => Self::Unsure,
        }
    }
}

fn ingest_bounded_number(value: &str, fallback: usize, min: usize, max: usize) -> usize {
    value
        .trim()
        .parse::<usize>()
        .ok()
        .map(|parsed| parsed.clamp(min, max))
        .unwrap_or(fallback)
}

fn ingest_contact_sheet_shape(state: &AtelierPanelState) -> (usize, usize, usize, usize) {
    let rows = ingest_bounded_number(&state.ingest_contact_rows, 3, 1, 24);
    let columns = ingest_bounded_number(&state.ingest_contact_columns, 4, 1, 24);
    let dpi = ingest_bounded_number(&state.ingest_contact_dpi, 300, 72, 1200);
    (rows, columns, dpi, rows.saturating_mul(columns))
}

fn ingest_queue_readout(state: &AtelierPanelState) -> String {
    let (rows, columns, dpi, cells) = ingest_contact_sheet_shape(state);
    format!(
        "dataset_ref={} character_ref={} decision={} link_passed={} tags={} note={} event={} date={} location={} contact_sheet={}x{}@{}dpi cells={} facial_profile={}",
        state.ingest_dataset_ref.trim(),
        state.ingest_character_ref.trim(),
        state.ingest_decision.machine_label(),
        state.ingest_link_passed,
        state.ingest_tag_buffer.trim(),
        state.ingest_batch_note.trim(),
        state.ingest_event.trim(),
        state.ingest_date.trim(),
        state.ingest_location.trim(),
        rows,
        columns,
        dpi,
        cells,
        state.ingest_facial_profile.trim()
    )
}

fn ingest_tag_values(state: &AtelierPanelState) -> Vec<String> {
    state
        .ingest_tag_buffer
        .split([',', ';'])
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn ingest_optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn ingest_metadata_payload(
    state: &AtelierPanelState,
    request_id: &str,
    batch_id: Option<&str>,
    loaded_item_count: usize,
) -> serde_json::Value {
    let (rows, columns, dpi, cells) = ingest_contact_sheet_shape(state);
    serde_json::json!({
        "request_id": request_id,
        "batch_id": batch_id,
        "dataset_ref": ingest_optional_string(&state.ingest_dataset_ref),
        "character_ref": ingest_optional_string(&state.ingest_character_ref),
        "link_passed": state.ingest_link_passed,
        "tags": ingest_tag_values(state),
        "note": ingest_optional_string(&state.ingest_batch_note),
        "event": ingest_optional_string(&state.ingest_event),
        "date": ingest_optional_string(&state.ingest_date),
        "location": ingest_optional_string(&state.ingest_location),
        "facial_profile": ingest_optional_string(&state.ingest_facial_profile),
        "loaded_item_count": loaded_item_count,
        "contact_sheet": {
            "rows": rows,
            "columns": columns,
            "dpi": dpi,
            "cells": cells,
        },
    })
}

fn ingest_receipt_applied_item_ids(
    rows: &[crate::backend_client::AtelierIntakeClassificationRow],
) -> String {
    if rows.is_empty() {
        return "<none>".to_owned();
    }
    rows.iter()
        .map(|row| row.item.item_id.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

fn ingest_item_decision(state: &AtelierPanelState, item: &AtelierItemRow) -> IngestDecision {
    state
        .ingest_item_decisions
        .get(&item.item_id)
        .copied()
        .unwrap_or_else(|| IngestDecision::from_lane(&item.lane))
}

pub struct AtelierPanel {
    state: Mutex<AtelierPanelState>,
    side_panel: Arc<Mutex<AtelierSidePanel>>,
    canvas_board: Arc<Mutex<LoomCanvasBoard>>,
    canvas_events: Arc<Mutex<Vec<CanvasEvent>>>,
    ckc_client: Option<AtelierClient>,
    ckc_cell: AtelierCkcCell,
    ckc_create_cell: AtelierCkcCreateCell,
    ckc_append_cell: AtelierCkcAppendCell,
    ckc_template_cell: AtelierCkcTemplateCell,
    ckc_safe_subset_cell: AtelierCkcSafeSubsetCell,
    ckc_import_cell: AtelierCkcImportCell,
    ckc_export_cell: AtelierCkcExportCell,
    ckc_field_suggestions_cell: AtelierCkcFieldSuggestionsCell,
    ckc_sheet_artifact_links_cell: AtelierCkcSheetArtifactLinksCell,
    ckc_media_album_create_cell: AtelierCkcMediaAlbumCreateCell,
    ckc_media_album_items_cell: AtelierCkcMediaAlbumItemsCell,
    ckc_media_album_page_cell: AtelierCkcMediaAlbumItemsCell,
    ckc_media_notes_cell: AtelierCkcMediaNotesCell,
    ckc_character_document_cell: AtelierCkcCharacterDocumentCell,
    ckc_story_card_cell: AtelierCkcStoryCardCell,
    ckc_story_beat_cell: AtelierCkcStoryBeatCell,
    ckc_moodboard_latest_cell: AtelierCkcMoodboardSnapshotCell,
    ckc_search_cell: AtelierCkcSearchCell,
    ckc_tag_note_cell: AtelierCkcTagNoteCell,
    pose_export_cell: AtelierPosekitExportCell,
    ingest_classification_cell: AtelierIntakeClassificationCell,
}

impl AtelierPanel {
    pub fn new(
        side_panel: Arc<Mutex<AtelierSidePanel>>,
        canvas_board: Arc<Mutex<LoomCanvasBoard>>,
        canvas_events: Arc<Mutex<Vec<CanvasEvent>>>,
    ) -> Self {
        Self::with_client(side_panel, canvas_board, canvas_events, None)
    }

    pub fn with_client(
        side_panel: Arc<Mutex<AtelierSidePanel>>,
        canvas_board: Arc<Mutex<LoomCanvasBoard>>,
        canvas_events: Arc<Mutex<Vec<CanvasEvent>>>,
        ckc_client: Option<AtelierClient>,
    ) -> Self {
        let mut state = AtelierPanelState::default();
        if ckc_client.is_some() {
            state.ckc_characters.clear();
            state.ckc_search_results.clear();
            state.ckc_search_status = "Waiting for live CKC database load".to_owned();
            state.ckc_tag_note_scope_ref.clear();
        }
        Self {
            state: Mutex::new(state),
            side_panel,
            canvas_board,
            canvas_events,
            ckc_client,
            ckc_cell: Arc::new(Mutex::new(None)),
            ckc_create_cell: Arc::new(Mutex::new(None)),
            ckc_append_cell: Arc::new(Mutex::new(None)),
            ckc_template_cell: Arc::new(Mutex::new(None)),
            ckc_safe_subset_cell: Arc::new(Mutex::new(None)),
            ckc_import_cell: Arc::new(Mutex::new(None)),
            ckc_export_cell: Arc::new(Mutex::new(None)),
            ckc_field_suggestions_cell: Arc::new(Mutex::new(None)),
            ckc_sheet_artifact_links_cell: Arc::new(Mutex::new(None)),
            ckc_media_album_create_cell: Arc::new(Mutex::new(None)),
            ckc_media_album_items_cell: Arc::new(Mutex::new(None)),
            ckc_media_album_page_cell: Arc::new(Mutex::new(None)),
            ckc_media_notes_cell: Arc::new(Mutex::new(None)),
            ckc_character_document_cell: Arc::new(Mutex::new(None)),
            ckc_story_card_cell: Arc::new(Mutex::new(None)),
            ckc_story_beat_cell: Arc::new(Mutex::new(None)),
            ckc_moodboard_latest_cell: Arc::new(Mutex::new(None)),
            ckc_search_cell: Arc::new(Mutex::new(None)),
            ckc_tag_note_cell: Arc::new(Mutex::new(None)),
            pose_export_cell: Arc::new(Mutex::new(None)),
            ingest_classification_cell: Arc::new(Mutex::new(None)),
        }
    }

    pub fn active_tab(&self) -> AtelierPanelTab {
        self.state
            .lock()
            .map(|state| state.active_tab)
            .unwrap_or(AtelierPanelTab::CastkitCodex)
    }

    pub fn set_active_tab(&self, tab: AtelierPanelTab) {
        if let Ok(mut state) = self.state.lock() {
            state.active_tab = tab;
        }
    }

    pub fn show(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        let panel_id = egui::Id::new(ATELIER_PANEL_AUTHOR_ID);
        let response = ui
            .scope_builder(egui::UiBuilder::new().id_salt(panel_id), |ui| {
                self.show_inner(ui, palette);
            })
            .response;
        emit_node(
            ui.ctx(),
            response.id,
            accesskit::Role::Group,
            ATELIER_PANEL_AUTHOR_ID,
            "Atelier",
            false,
        );
    }

    fn show_inner(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("Atelier").color(palette.text));
                ui.add_space(8.0);
                ui.label(egui::RichText::new("CKC").color(palette.text_subtle));
            });
            ui.add_space(4.0);
            self.show_tab_strip(ui);
            ui.separator();

            let active = self.active_tab();
            self.show_content_region(ui, palette, active);
        });
    }

    fn show_tab_strip(&self, ui: &mut egui::Ui) {
        let response = ui
            .horizontal(|ui| {
                let mut active = self.active_tab();
                for tab in AtelierPanelTab::ALL {
                    let selected = active == tab;
                    let button = ui.add(egui::Button::selectable(selected, tab.label()));
                    button.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::Button,
                            ui.is_enabled(),
                            selected,
                            tab.label(),
                        )
                    });
                    emit_node(
                        ui.ctx(),
                        button.id,
                        accesskit::Role::Tab,
                        tab.tab_author_id(),
                        tab.label(),
                        selected,
                    );
                    if button.clicked() {
                        active = tab;
                    }
                }
                self.set_active_tab(active);
            })
            .response;
        emit_node(
            ui.ctx(),
            response.id,
            accesskit::Role::TabList,
            ATELIER_TABLIST_AUTHOR_ID,
            "Atelier tabs",
            false,
        );
    }

    fn show_content_region(&self, ui: &mut egui::Ui, palette: &HsPalette, tab: AtelierPanelTab) {
        let response = ui
            .scope_builder(
                egui::UiBuilder::new().id_salt(tab.content_author_id()),
                |ui| match tab {
                    AtelierPanelTab::CastkitCodex => self.show_ckc(ui, palette),
                    AtelierPanelTab::Posekit => self.show_posekit(ui, palette),
                    AtelierPanelTab::Ingest => self.show_ingest(ui, palette),
                },
            )
            .response;
        emit_node(
            ui.ctx(),
            response.id,
            accesskit::Role::Group,
            tab.content_author_id(),
            tab.label(),
            false,
        );
    }

    fn ensure_ckc_load_requested(&self) {
        let Some(client) = self.ckc_client.as_ref() else {
            return;
        };
        let should_request = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if state.ckc_load_requested {
                false
            } else {
                state.ckc_load_requested = true;
                state.ckc_loading = true;
                state.ckc_error = None;
                true
            }
        };
        if should_request {
            client.fetch_ckc(self.ckc_cell.clone());
        }
    }

    fn drain_ckc_backend(&self) {
        let load_result = self.ckc_cell.lock().ok().and_then(|mut slot| slot.take());
        if let Some(result) = load_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_loading = false;
                match result {
                    Ok(data) => {
                        let selected_id = state
                            .ckc_characters
                            .get(state.ckc_selected_index)
                            .map(|row| row.character_internal_id.clone());
                        state.ckc_characters = data
                            .characters
                            .into_iter()
                            .map(CkcCharacterRecord::from_backend)
                            .collect();
                        state.ckc_selected_index = selected_id
                            .and_then(|id| {
                                state
                                    .ckc_characters
                                    .iter()
                                    .position(|row| row.character_internal_id == id)
                            })
                            .unwrap_or(0);
                        state.ckc_backend_loaded = true;
                        state.ckc_error = None;
                        state.ckc_search_results.clear();
                        state.ckc_last_export = None;
                        let filters = selected_ckc_search_filter_refs(&state);
                        state.ckc_tag_note_scope_ref = filters
                            .collection_ref
                            .clone()
                            .or(filters.character_ref.clone())
                            .unwrap_or_default();
                        state.ckc_search_status = format!(
                            "CKC database loaded ({} character(s)); run search for live refs",
                            state.ckc_characters.len()
                        );
                    }
                    Err(err) => {
                        state.ckc_backend_loaded = false;
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let create_result = self
            .ckc_create_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        let mut refresh_after_create = false;
        if let Some(result) = create_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_create_pending = false;
                match result {
                    Ok(character) => {
                        let record = CkcCharacterRecord::from_created_character(character);
                        let selected_id = record.character_internal_id.clone();
                        if let Some(existing) = state
                            .ckc_characters
                            .iter_mut()
                            .find(|row| row.character_internal_id == selected_id)
                        {
                            *existing = record;
                        } else {
                            state.ckc_characters.push(record);
                        }
                        state.ckc_selected_index = state
                            .ckc_characters
                            .iter()
                            .position(|row| row.character_internal_id == selected_id)
                            .unwrap_or(0);
                        state.ckc_new_display_name = "New character".to_owned();
                        state.ckc_backend_loaded = true;
                        state.ckc_loading = self.ckc_client.is_some();
                        state.ckc_last_export = None;
                        state.ckc_error = None;
                        refresh_after_create = self.ckc_client.is_some();
                    }
                    Err(err) => {
                        state.ckc_error = Some(err);
                    }
                }
            }
        }
        if refresh_after_create {
            if let Some(client) = self.ckc_client.as_ref() {
                client.fetch_ckc(self.ckc_cell.clone());
            }
        }

        let append_result = self
            .ckc_append_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = append_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_append_pending = false;
                match result {
                    Ok(sheet) => {
                        let character_internal_id = sheet.character_internal_id.clone();
                        if let Some(row) = state
                            .ckc_characters
                            .iter_mut()
                            .find(|row| row.character_internal_id == character_internal_id)
                        {
                            row.apply_sheet_version(sheet);
                        }
                        state.ckc_last_export = None;
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let template_result = self
            .ckc_template_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = template_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_template_pending = false;
                match result {
                    Ok(template) => {
                        let hash = short_hash(&template.template_hash);
                        state.ckc_template_status = format!(
                            "{} {} loaded: {} fields, {} sections, hash {}",
                            template.file_name,
                            template.template_version,
                            template.field_count,
                            template.section_count,
                            hash
                        );
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_template_status = format!("Template load failed: {err}");
                    }
                }
            }
        }

        let safe_subset_result = self
            .ckc_safe_subset_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = safe_subset_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_safe_subset_pending = false;
                match result {
                    Ok(subset) => {
                        state.ckc_template_status = format!(
                            "{} {} loaded: {} Field IDs in short/SFW-safe subset",
                            subset.file_name,
                            subset.template_version,
                            subset.field_ids.len()
                        );
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_template_status = format!("Safe subset load failed: {err}");
                    }
                }
            }
        }

        let import_result = self
            .ckc_import_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = import_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_import_pending = false;
                match result {
                    Ok(sheet) => {
                        let character_internal_id = sheet.character_internal_id.clone();
                        let seq = sheet.seq;
                        if let Some(row) = state
                            .ckc_characters
                            .iter_mut()
                            .find(|row| row.character_internal_id == character_internal_id)
                        {
                            row.apply_sheet_version(sheet);
                        }
                        state.ckc_export_status =
                            format!("Imported CKC sheet as append-only version v{seq}");
                        state.ckc_last_export = None;
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_export_status = format!("CKC sheet import failed: {err}");
                    }
                }
            }
        }

        let export_result = self
            .ckc_export_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = export_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_export_pending = false;
                match result {
                    Ok(export) => {
                        state.ckc_export_status = format!(
                            "Exported {} as {} ({} bytes, hash {})",
                            export.file_name,
                            export.format,
                            export.content.len(),
                            short_hash(&export.content_hash)
                        );
                        state.ckc_last_export = Some(export);
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_export_status = format!("CKC sheet export failed: {err}");
                    }
                }
            }
        }

        let suggestions_result = self
            .ckc_field_suggestions_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = suggestions_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_field_suggestion_pending = false;
                match result {
                    Ok(rows) => {
                        let field_id = state.ckc_field_suggestion_id.clone();
                        let count = rows.len();
                        state.ckc_field_suggestions = rows;
                        state.ckc_field_suggestion_status =
                            format!("Loaded {count} prior value(s) for {field_id}");
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_field_suggestions.clear();
                        state.ckc_field_suggestion_status =
                            format!("CKC field suggestions failed: {err}");
                    }
                }
            }
        }

        let artifact_links_result = self
            .ckc_sheet_artifact_links_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some((target_sheet_version_id, result)) = artifact_links_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_sheet_artifact_pending = false;
                match result {
                    Ok(rows) => {
                        let outcome = apply_ckc_sheet_artifact_link_rows_to_state(
                            &mut state,
                            &target_sheet_version_id,
                            rows,
                        );
                        if outcome.current_selection_owns_target {
                            state.ckc_sheet_artifact_status =
                                format!("Loaded {} reusable sheet artifact link(s)", outcome.count);
                            state.ckc_error = None;
                        } else if !outcome.target_found {
                            state.ckc_sheet_artifact_status = format!(
                                "Ignored CKC sheet artifact result for stale sheet {target_sheet_version_id}"
                            );
                            state.ckc_error = Some(format!(
                                "No CKC character owns sheet_version_id={target_sheet_version_id}"
                            ));
                        }
                    }
                    Err(err) => {
                        let current_selection_owns_target = state
                            .ckc_characters
                            .get(state.ckc_selected_index)
                            .and_then(|character| character.sheet_version_id.as_deref())
                            == Some(target_sheet_version_id.as_str());
                        if current_selection_owns_target {
                            state.ckc_sheet_artifact_status =
                                format!("CKC sheet artifact link operation failed: {err}");
                            state.ckc_error = Some(err);
                        }
                    }
                }
            }
        }

        let album_create_result = self
            .ckc_media_album_create_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = album_create_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_album_create_pending = false;
                match result {
                    Ok(row) => {
                        let collection_id = row.collection_id.clone();
                        let character_internal_id = row.character_internal_id.clone();
                        let album = CkcMediaAlbumRecord::from_backend(row);
                        let mut applied = false;
                        for character in &mut state.ckc_characters {
                            if character.character_internal_id == character_internal_id {
                                if let Some(existing) = character
                                    .media_albums
                                    .iter_mut()
                                    .find(|existing| existing.collection_id == collection_id)
                                {
                                    *existing = album.clone();
                                } else {
                                    character.media_albums.push(album.clone());
                                }
                                applied = true;
                                break;
                            }
                        }
                        state.ckc_selected_album_collection_id = Some(collection_id.clone());
                        state.ckc_album_status = if applied {
                            format!("Created CKC album {collection_id}")
                        } else {
                            format!(
                                "Created CKC album {collection_id}, but its character is not visible"
                            )
                        };
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_album_status = format!("CKC album create failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let album_items_result = self
            .ckc_media_album_items_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = album_items_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_album_link_pending = false;
                match result {
                    Ok(row) => {
                        let collection_id = row.collection_id.clone();
                        let mut applied = false;
                        for character in &mut state.ckc_characters {
                            if let Some(album) = character
                                .media_albums
                                .iter_mut()
                                .find(|album| album.collection_id == collection_id)
                            {
                                album.collection_ref = row.collection_ref.clone();
                                album.member_count = row.member_count;
                                album.members_next_offset = row.members_next_offset;
                                album.members = row
                                    .members
                                    .into_iter()
                                    .map(CkcMediaMemberRecord::from_backend)
                                    .collect();
                                applied = true;
                                break;
                            }
                        }
                        state.ckc_selected_album_collection_id = Some(collection_id.clone());
                        state.ckc_album_status = if applied {
                            format!(
                                "Linked {} of {} requested media asset(s) into CKC album {collection_id}",
                                row.inserted, row.requested
                            )
                        } else {
                            format!(
                                "Linked media into CKC album {collection_id}, but it is not visible"
                            )
                        };
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_album_status = format!("CKC media link failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let album_page_result = self
            .ckc_media_album_page_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = album_page_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_album_page_pending = false;
                match result {
                    Ok(row) => {
                        let collection_id = row.collection_id.clone();
                        let mut appended = 0usize;
                        let mut applied = false;
                        for character in &mut state.ckc_characters {
                            if let Some(album) = character
                                .media_albums
                                .iter_mut()
                                .find(|album| album.collection_id == collection_id)
                            {
                                album.collection_ref = row.collection_ref.clone();
                                album.member_count = row.member_count;
                                album.members_next_offset = row.members_next_offset;
                                for member in row.members {
                                    if album
                                        .members
                                        .iter()
                                        .any(|existing| existing.asset_id == member.asset_id)
                                    {
                                        continue;
                                    }
                                    album
                                        .members
                                        .push(CkcMediaMemberRecord::from_backend(member));
                                    appended += 1;
                                }
                                applied = true;
                                break;
                            }
                        }
                        state.ckc_selected_album_collection_id = Some(collection_id.clone());
                        state.ckc_album_status = if applied {
                            format!(
                                "Loaded {appended} more CKC media item(s) for album {collection_id}"
                            )
                        } else {
                            format!(
                                "Loaded CKC media page for album {collection_id}, but it is not visible"
                            )
                        };
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_album_status = format!("CKC media page load failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let media_result = self
            .ckc_media_notes_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = media_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_media_save_pending = false;
                match result {
                    Ok(row) => {
                        let asset_id = row.asset_id.clone();
                        let mut applied_count = 0usize;
                        for character in &mut state.ckc_characters {
                            for album in &mut character.media_albums {
                                for member in
                                    album.members.iter_mut().filter(|m| m.asset_id == asset_id)
                                {
                                    member.apply_notes_tags(&row);
                                    applied_count += 1;
                                }
                            }
                        }
                        state.ckc_error = if applied_count > 0 {
                            None
                        } else {
                            Some(format!("saved media asset {asset_id} is no longer visible"))
                        };
                    }
                    Err(err) => {
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let document_result = self
            .ckc_character_document_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = document_result {
            if let Ok(mut state) = self.state.lock() {
                match result {
                    Ok(row) => {
                        let doc_type = row.doc_type.clone();
                        let document_id = row.document_id.clone();
                        let status =
                            apply_ckc_character_document_row(&mut state.ckc_characters, row);
                        match doc_type.as_str() {
                            "story" => {
                                state.ckc_active_story_document_id = Some(document_id);
                                state.ckc_story_status = status.unwrap_or_else(|| {
                                    "Saved CKC story document, but it is not visible".to_owned()
                                });
                            }
                            "moodboard" => {
                                state.ckc_active_moodboard_document_id = Some(document_id);
                                state.ckc_moodboard_status = status.unwrap_or_else(|| {
                                    "Saved CKC moodboard document, but it is not visible".to_owned()
                                });
                            }
                            _ => {}
                        }
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_story_status = format!("CKC story document save failed: {err}");
                        state.ckc_moodboard_status =
                            format!("CKC moodboard document save failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let story_card_result = self
            .ckc_story_card_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = story_card_result {
            if let Ok(mut state) = self.state.lock() {
                match result {
                    Ok(row) => {
                        state.ckc_story_status =
                            apply_ckc_story_card_row(&mut state.ckc_characters, row)
                                .unwrap_or_else(|| {
                                    "Added CKC story card, but its story is not visible".to_owned()
                                });
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_story_status = format!("CKC story card save failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let story_beat_result = self
            .ckc_story_beat_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = story_beat_result {
            if let Ok(mut state) = self.state.lock() {
                match result {
                    Ok(row) => {
                        state.ckc_story_status =
                            apply_ckc_story_beat_row(&mut state.ckc_characters, row)
                                .unwrap_or_else(|| {
                                    "Added CKC story beat, but its story is not visible".to_owned()
                                });
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_story_status = format!("CKC story beat save failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let moodboard_latest_result = self
            .ckc_moodboard_latest_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = moodboard_latest_result {
            if let Ok(mut state) = self.state.lock() {
                match result {
                    Ok(row) => {
                        match apply_ckc_moodboard_snapshot_row(&mut state.ckc_characters, row) {
                            Ok((status, projection, snapshot_ref)) => {
                                if status.is_some() {
                                    match self.canvas_board.lock() {
                                        Ok(mut board) => {
                                            projection.apply_to_board(&mut board, &snapshot_ref);
                                            state.ckc_moodboard_status = status.unwrap();
                                            state.ckc_error = None;
                                        }
                                        Err(err) => {
                                            state.ckc_moodboard_status =
                                                format!("CKC moodboard canvas lock failed: {err}");
                                            state.ckc_error =
                                                Some(state.ckc_moodboard_status.clone());
                                        }
                                    }
                                } else {
                                    state.ckc_moodboard_status =
                                        "Opened CKC moodboard snapshot, but its document is not visible"
                                            .to_owned();
                                    state.ckc_error = Some(state.ckc_moodboard_status.clone());
                                }
                            }
                            Err(err) => {
                                state.ckc_moodboard_status =
                                    format!("CKC moodboard snapshot projection failed: {err}");
                                state.ckc_error = Some(state.ckc_moodboard_status.clone());
                            }
                        }
                    }
                    Err(err) => {
                        state.ckc_moodboard_status = format!("CKC moodboard open failed: {err}");
                        state.ckc_error = Some(err);
                    }
                }
            }
        }

        let search_result = self
            .ckc_search_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = search_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_search_pending = false;
                match result {
                    Ok(response) => {
                        let status = ckc_search_status_from_response(&response);
                        state.ckc_search_results = response
                            .results
                            .into_iter()
                            .map(CkcSearchResultRecord::from_backend)
                            .collect();
                        state.ckc_search_status = status;
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_search_results.clear();
                        state.ckc_search_status = format!("CKC search failed: {err}");
                    }
                }
            }
        }

        let tag_note_result = self
            .ckc_tag_note_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(result) = tag_note_result {
            if let Ok(mut state) = self.state.lock() {
                state.ckc_tag_note_pending = false;
                match result {
                    Ok(row) => {
                        let tag_text = row.tag_text.clone();
                        state.ckc_search_status = format!("Saved CKC tag note for {tag_text}");
                        attach_tag_note_to_visible_results(
                            &mut state.ckc_search_results,
                            CkcTagNoteRecord::from_backend(row),
                        );
                        state.ckc_error = None;
                    }
                    Err(err) => {
                        state.ckc_search_status = format!("CKC tag note save failed: {err}");
                    }
                }
            }
        }
    }

    fn drain_posekit_export_backend(&self) {
        let export_result = self
            .pose_export_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some((request_id, result)) = export_result {
            if let Ok(mut state) = self.state.lock() {
                if state.pose_active_export_request != Some(request_id) {
                    return;
                }
                state.pose_export_pending = false;
                state.pose_active_export_request = None;
                match result {
                    Ok(row) => {
                        let snapshot = posekit_export_snapshot_from_backend(row);
                        state.pose_export_status = format!(
                            "Exported backend Posekit OpenPose: yaw_deg={:.0} png_artifact_ref={} json_artifact_ref={} receipt_ref={}",
                            snapshot.yaw_deg,
                            snapshot.png_artifact_ref,
                            snapshot.json_artifact_ref,
                            snapshot.receipt_ref
                        );
                        state.pose_last_export = Some(snapshot);
                    }
                    Err(err) => {
                        state.pose_export_status = format!("Posekit OpenPose export failed: {err}");
                    }
                }
            }
        }
    }

    fn drain_ingest_classification_backend(&self) {
        let outcome = self
            .ingest_classification_cell
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(outcome) = outcome {
            if let Ok(mut state) = self.state.lock() {
                if state.ingest_apply_request_id.as_deref() != Some(outcome.request_id.as_str())
                    || state.ingest_apply_batch_id.as_deref() != outcome.batch_id.as_deref()
                {
                    state.ingest_status = format!(
                        "Ignored stale ingest classification response request_id={} batch_id={:?}.",
                        outcome.request_id, outcome.batch_id
                    );
                    return;
                }

                state.ingest_apply_pending = false;
                state.ingest_apply_request_id = None;
                state.ingest_apply_batch_id = None;

                let request_id = outcome.request_id.clone();
                let batch_id = outcome
                    .batch_id
                    .clone()
                    .unwrap_or_else(|| "<none>".to_owned());
                let applied_count = outcome.applied.len();
                let applied_ids = ingest_receipt_applied_item_ids(&outcome.applied);
                for row in &outcome.applied {
                    let decision = IngestDecision::from_lane(&row.item.lane);
                    state
                        .ingest_item_decisions
                        .insert(row.item.item_id.clone(), decision);
                    state
                        .ingest_persisted_item_ids
                        .insert(row.item.item_id.clone());
                }

                if let Some(failed) = outcome.failed {
                    state.ingest_last_apply_receipt = format!(
                        "request_id={request_id} batch_id={batch_id} applied_count={applied_count} applied_item_ids={applied_ids} failed_item_id={} failed_row={} failed_error={}",
                        failed.item_id,
                        failed.index + 1,
                        failed.error
                    );
                    state.ingest_status = format!(
                        "Persisted {applied_count} loaded intake item classification(s); failed item {} at row {}: {}",
                        failed.item_id,
                        failed.index + 1,
                        failed.error
                    );
                } else {
                    state.ingest_last_apply_receipt = format!(
                        "request_id={request_id} batch_id={batch_id} applied_count={applied_count} applied_item_ids={applied_ids} failed_item_id=<none>"
                    );
                    state.ingest_status = format!(
                        "Persisted {applied_count} loaded intake item classification(s) through backend actor {}.",
                        self.ckc_client
                            .as_ref()
                            .map(|client| client.actor_id())
                            .unwrap_or("atelier")
                    );
                }
            }
        }
    }

    fn show_ckc(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        self.ensure_ckc_load_requested();
        self.drain_ckc_backend();
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if self.ckc_client.is_none() && state.ckc_characters.is_empty() {
            state.ckc_characters = seeded_ckc_characters();
            state.ckc_selected_index = 0;
        }
        self.show_ckc_mode_controls(ui, palette, &mut state.ckc_book_mode);
        let book_mode = state.ckc_book_mode;
        let selected_index = state
            .ckc_selected_index
            .min(state.ckc_characters.len().saturating_sub(1));
        state.ckc_selected_index = selected_index;
        let book_response = ui
            .scope_builder(
                egui::UiBuilder::new().id_salt(egui::Id::new(ATELIER_CKC_BOOK_LAYOUT_AUTHOR_ID)),
                |ui| {
                    let available_width = ui.available_width();
                    let has_middle = book_mode.has_middle_panel();
                    let left_w = if has_middle {
                        (available_width * 0.30).clamp(220.0, 340.0)
                    } else {
                        (available_width * 0.42).clamp(280.0, 460.0)
                    };
                    let middle_w = if has_middle {
                        (available_width * 0.30).clamp(260.0, 430.0)
                    } else {
                        0.0
                    };
                    ui.horizontal(|ui| {
                        let left_response = ui
                            .scope_builder(
                                egui::UiBuilder::new().id_salt(egui::Id::new(
                                    ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
                                )),
                                |ui| {
                                    ui.set_width(left_w);
                                    egui::ScrollArea::vertical()
                                        .id_salt("atelier-ckc-book-left-media-scroll")
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            ui.heading(
                                                egui::RichText::new("Character images")
                                                    .color(palette.text),
                                            );
                if state.ckc_loading {
                    ui.label(egui::RichText::new("Loading CKC database...").color(palette.text_subtle));
                }
                if let Some(error) = &state.ckc_error {
                    ui.label(egui::RichText::new(format!("CKC backend: {error}")).color(palette.error_text));
                }
                self.show_ckc_search(ui, palette, &mut state);
                ui.separator();
                let list_response = ui
                    .vertical(|ui| {
                        let mut pending_selection = None;
                        for (idx, character) in state.ckc_characters.iter().enumerate() {
                            let selected = state.ckc_selected_index == idx;
                            let row_label = if character.sheet_seq > 0 {
                                format!("{}  v{}", character.display_name, character.sheet_seq)
                            } else {
                                format!("{}  no sheet", character.display_name)
                            };
                            let row = ui
                                .push_id(
                                    ("ckc-character-row", character.character_internal_id.as_str()),
                                    |ui| ui.add(egui::Button::new(row_label).selected(selected)),
                                )
                                .inner;
                            emit_node(
                                ui.ctx(),
                                row.id,
                                accesskit::Role::Button,
                                &ckc_character_row_author_id(&character.character_internal_id),
                                &format!(
                                    "{} sheet version {}",
                                    character.display_name, character.sheet_seq
                                ),
                                selected,
                            );
                            if row.clicked() {
                                pending_selection = Some(idx);
                            }
                        }
                        if let Some(idx) = pending_selection {
                            if state.ckc_selected_index != idx {
                                state.ckc_selected_index = idx;
                                state.ckc_last_export = None;
                                state.ckc_selected_media_key = None;
                                state.ckc_selected_album_collection_id = None;
                                state.ckc_selected_sheet_artifact_link_id = None;
                                state.ckc_sheet_artifact_reuse_ref.clear();
                            }
                        }
                    })
                    .response;
                emit_node(
                    ui.ctx(),
                    list_response.id,
                    accesskit::Role::List,
                    ATELIER_CKC_CHARACTER_LIST_AUTHOR_ID,
                    "CKC character database",
                    false,
                );
                ui.separator();
                ui.horizontal(|ui| {
                    let create_name = ui.text_edit_singleline(&mut state.ckc_new_display_name);
                    emit_node(
                        ui.ctx(),
                        create_name.id,
                        accesskit::Role::TextInput,
                        ATELIER_CKC_CHARACTER_CREATE_NAME_AUTHOR_ID,
                        "New character display name",
                        false,
                    );
                    let create = ui.button("Create");
                    emit_node(
                        ui.ctx(),
                        create.id,
                        accesskit::Role::Button,
                        ATELIER_CKC_CHARACTER_CREATE_AUTHOR_ID,
                        "Create CKC character",
                        state.ckc_create_pending,
                    );
                    if create.clicked() {
                        let display_name = state.ckc_new_display_name.trim().to_owned();
                        if !display_name.is_empty() {
                            let next = state.ckc_characters.len() + 1;
                            let public_id = slugify_public_id(&display_name, next);
                            if let Some(client) = self.ckc_client.as_ref() {
                                if !state.ckc_create_pending {
                                    state.ckc_create_pending = true;
                                    state.ckc_error = None;
                                    client.create_ckc_character(
                                        &public_id,
                                        &display_name,
                                        client.actor_id(),
                                        self.ckc_create_cell.clone(),
                                    );
                                }
                            } else {
                                let character_internal_id = Uuid::new_v4().to_string();
                                state.ckc_characters.push(CkcCharacterRecord {
                                    public_id: public_id.clone(),
                                    display_name: display_name.clone(),
                                    character_internal_id: character_internal_id.clone(),
                                    character_ref: format!("atelier://character/{character_internal_id}"),
                                    sheet_version_id: None,
                                    parent_sheet_version_id: None,
                                    sheet_seq: 0,
                                    sheet_editor_text: format!(
                                        "CHAR-ID-001 \u{2014} Character_ID: {public_id}\nCHAR-ID-002 \u{2014} Name: {display_name}\nCHAR-ID-006 \u{2014} Primary_Role: reusable character/avatar\nPIPELINES\npipelines: ComfyUI, Unreal, Blender\nnotes: "
                                    ),
                                    sheet_version_ref: None,
                                    sheet_artifact_links: Vec::new(),
                                    media_albums: Vec::new(),
                                    story_documents: Vec::new(),
                                    moodboard_documents: Vec::new(),
                                });
                                state.ckc_selected_index = state.ckc_characters.len() - 1;
                                state.ckc_last_export = None;
                                state.ckc_new_display_name = "New character".to_owned();
                            }
                        }
                    }
                });
                ui.separator();
                let selected_index = state
                    .ckc_selected_index
                    .min(state.ckc_characters.len().saturating_sub(1));
                let media_save_pending = state.ckc_media_save_pending;
                let selected_media_key = state.ckc_selected_media_key.clone();
                let selected_album_collection_id = state.ckc_selected_album_collection_id.clone();
                let album_create_pending = state.ckc_album_create_pending;
                let album_link_pending = state.ckc_album_link_pending;
                let album_page_pending = state.ckc_album_page_pending;
                let album_status = state.ckc_album_status.clone();
                let mut album_create_name = std::mem::take(&mut state.ckc_album_create_name);
                let mut album_create_notes = std::mem::take(&mut state.ckc_album_create_notes);
                let mut album_create_tags = std::mem::take(&mut state.ckc_album_create_tags);
                let mut album_link_asset_ids =
                    std::mem::take(&mut state.ckc_album_link_asset_ids);
                let mut album_link_source_path_ref =
                    std::mem::take(&mut state.ckc_album_link_source_path_ref);
                let mut album_link_source_url_ref =
                    std::mem::take(&mut state.ckc_album_link_source_url_ref);
                let mut pending_media_save = None;
                let mut pending_media_selection = None;
                let mut pending_album_selection = None;
                let mut pending_album_create = None;
                let mut pending_album_link = None;
                let mut pending_album_page = None;
                if let Some(character) = state.ckc_characters.get_mut(selected_index) {
                    let (
                        save,
                        media_selection,
                        album_selection,
                        album_create,
                        album_link,
                        album_page,
                    ) = self
                        .show_ckc_linked_media(
                        ui,
                        palette,
                        character,
                        media_save_pending,
                        selected_media_key.as_deref(),
                        selected_album_collection_id.as_deref(),
                        album_create_pending,
                        album_link_pending,
                        album_page_pending,
                        &album_status,
                        &mut album_create_name,
                        &mut album_create_notes,
                        &mut album_create_tags,
                        &mut album_link_asset_ids,
                        &mut album_link_source_path_ref,
                        &mut album_link_source_url_ref,
                    );
                    pending_media_save = save;
                    pending_media_selection = media_selection;
                    pending_album_selection = album_selection;
                    pending_album_create = album_create;
                    pending_album_link = album_link;
                    pending_album_page = album_page;
                }
                state.ckc_album_create_name = album_create_name;
                state.ckc_album_create_notes = album_create_notes;
                state.ckc_album_create_tags = album_create_tags;
                state.ckc_album_link_asset_ids = album_link_asset_ids;
                state.ckc_album_link_source_path_ref = album_link_source_path_ref;
                state.ckc_album_link_source_url_ref = album_link_source_url_ref;
                if let Some(media_key) = pending_media_selection {
                    state.ckc_selected_media_key = Some(media_key);
                }
                if let Some(collection_id) = pending_album_selection {
                    state.ckc_selected_album_collection_id = Some(collection_id);
                }
                if let Some(request) = pending_album_create {
                    if let Some(client) = self.ckc_client.as_ref() {
                        state.ckc_album_create_pending = true;
                        state.ckc_album_status = format!("Creating CKC album {}", request.name);
                        state.ckc_error = None;
                        client.create_ckc_media_album(
                            &request.character_internal_id,
                            &request.name,
                            request.notes.as_deref(),
                            request.sheet_version_id.as_deref(),
                            &request.tags,
                            client.actor_id(),
                            self.ckc_media_album_create_cell.clone(),
                        );
                    }
                }
                if let Some(request) = pending_album_link {
                    if let Some(client) = self.ckc_client.as_ref() {
                        state.ckc_album_link_pending = true;
                        state.ckc_album_status = format!(
                            "Linking {} media asset(s) into CKC album {}",
                            request.asset_ids.len(),
                            request.collection_id
                        );
                        state.ckc_error = None;
                        client.add_ckc_media_album_items(
                            &request.collection_id,
                            &request.asset_ids,
                            request.source_path_ref.as_deref(),
                            request.source_url_ref.as_deref(),
                            client.actor_id(),
                            self.ckc_media_album_items_cell.clone(),
                        );
                    }
                }
                if let Some(request) = pending_album_page {
                    if let Some(client) = self.ckc_client.as_ref() {
                        state.ckc_album_page_pending = true;
                        state.ckc_album_status = format!(
                            "Loading CKC media album {} from offset {}",
                            request.collection_id, request.offset
                        );
                        state.ckc_error = None;
                        client.fetch_ckc_media_album_items(
                            &request.collection_id,
                            request.offset,
                            200,
                            self.ckc_media_album_page_cell.clone(),
                        );
                    }
                }
                if let Some(request) = pending_media_save {
                    if let Some(client) = self.ckc_client.as_ref() {
                        state.ckc_media_save_pending = true;
                        state.ckc_error = None;
                        client.save_ckc_media_notes_tags(
                            &request.asset_id,
                            Some(&request.notes),
                            Some(&request.tags),
                            request.review_status.as_deref(),
                            client.actor_id(),
                            self.ckc_media_notes_cell.clone(),
                        );
                    }
                }
                ui.separator();
                if let Ok(mut side_panel) = self.side_panel.lock() {
                    side_panel.show(ui, palette);
                }
                                        });
                                },
                            )
                            .response;
                        emit_node(
                            ui.ctx(),
                            left_response.id,
                            accesskit::Role::Group,
                            ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
                            "CKC left page: character images albums media notes and source refs",
                            false,
                        );
                        ui.separator();
                        let selected_index = state
                            .ckc_selected_index
                            .min(state.ckc_characters.len().saturating_sub(1));
                        state.ckc_selected_index = selected_index;
                        if has_middle {
                            let middle_response = ui
                                .scope_builder(
                                    egui::UiBuilder::new()
                                        .id_salt(egui::Id::new(ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID)),
                                    |ui| {
                                        ui.set_width(middle_w);
                                        egui::ScrollArea::vertical()
                                            .id_salt("atelier-ckc-book-middle-scroll")
                                            .auto_shrink([false, false])
                                            .show(ui, |ui| {
                                                self.show_ckc_middle_work_panel(
                                                    ui,
                                                    palette,
                                                    &mut state,
                                                    selected_index,
                                                );
                                            });
                                    },
                                )
                                .response;
                            emit_node(
                                ui.ctx(),
                                middle_response.id,
                                accesskit::Role::Group,
                                ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID,
                                book_mode.middle_label(),
                                false,
                            );
                            ui.separator();
                        }
                        let right_response = ui
                            .scope_builder(
                                egui::UiBuilder::new().id_salt(egui::Id::new(
                                    ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
                                )),
                                |ui| {
                                    egui::ScrollArea::vertical()
                                        .id_salt("atelier-ckc-book-right-sheet-scroll")
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            self.show_ckc_character_sheet_panel(
                                                ui,
                                                palette,
                                                &mut state,
                                                selected_index,
                                            );
                                            ui.separator();
                                            self.show_ckc_sheet_tools(
                                                ui,
                                                palette,
                                                &mut state,
                                                selected_index,
                                            );
                                            ui.separator();
                                            self.show_ckc_sheet_artifact_panel(
                                                ui,
                                                palette,
                                                &mut state,
                                                selected_index,
                                            );
                                        });
                                },
                            )
                            .response;
                        emit_node(
                            ui.ctx(),
                            right_response.id,
                            accesskit::Role::Group,
                            ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
                            "CKC right page: editable character sheet and sheet tools",
                            false,
                        );
                    });
                },
            )
            .response;
        emit_node(
            ui.ctx(),
            book_response.id,
            accesskit::Role::Group,
            ATELIER_CKC_BOOK_LAYOUT_AUTHOR_ID,
            if book_mode.has_middle_panel() {
                "CKC book layout with left media middle work surface and right sheet"
            } else {
                "CKC book layout with left media and right sheet"
            },
            false,
        );
    }

    fn show_ckc_mode_controls(
        &self,
        ui: &mut egui::Ui,
        _palette: &HsPalette,
        mode: &mut CkcBookMode,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.label("CKC book mode:");
            for next_mode in CkcBookMode::ALL {
                let selected = *mode == next_mode;
                let response = ui.add(egui::Button::selectable(selected, next_mode.label()));
                emit_node(
                    ui.ctx(),
                    response.id,
                    accesskit::Role::Button,
                    next_mode.author_id(),
                    next_mode.middle_label(),
                    selected,
                );
                if response.clicked() {
                    *mode = next_mode;
                }
            }
        });
    }

    fn show_ckc_character_sheet_panel(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        state: &mut AtelierPanelState,
        selected_index: usize,
    ) {
        let append_pending = state.ckc_append_pending;
        let mut pending_append_request: Option<(String, String, Option<String>)> = None;
        let mut clear_last_export = false;
        if let Some(character) = state.ckc_characters.get_mut(selected_index) {
            let selected_response = ui
                .vertical(|ui| {
                    ui.heading(egui::RichText::new(&character.display_name).color(palette.text));
                    ui.label(format!("public_id: {}", character.public_id));
                    if character.sheet_seq > 0 {
                        ui.label(format!("sheet seq: {}", character.sheet_seq));
                    } else {
                        ui.label("sheet seq: no sheet version yet");
                    }
                    if let Some(parent) = &character.parent_sheet_version_id {
                        ui.label(format!("parent_version_id: {parent}"));
                    }
                })
                .response;
            emit_node(
                ui.ctx(),
                selected_response.id,
                accesskit::Role::Group,
                ATELIER_CKC_SELECTED_CHARACTER_AUTHOR_ID,
                &format!(
                    "{} current sheet version {}",
                    character.display_name, character.sheet_seq
                ),
                true,
            );
            ui.add_space(4.0);
            let character_ref = character.character_ref();
            let character_ref_response = ui.label(format!("character_ref: {character_ref}"));
            emit_node(
                ui.ctx(),
                character_ref_response.id,
                accesskit::Role::Label,
                ATELIER_CKC_CHARACTER_REF_AUTHOR_ID,
                &character_ref,
                false,
            );
            let sheet_ref = character.sheet_atelier_ref();
            if let Some(sheet_ref) = &sheet_ref {
                debug_assert_eq!(sheet_ref.item_kind, AtelierItemKind::CharacterSheet);
            }
            let ref_kind = sheet_ref
                .as_ref()
                .map(|sheet_ref| sheet_ref.ref_kind())
                .unwrap_or("character_sheet");
            let ref_kind_response = ui.label(format!("hsLink refKind: {ref_kind}"));
            emit_node(
                ui.ctx(),
                ref_kind_response.id,
                accesskit::Role::Label,
                ATELIER_CKC_TYPED_REF_KIND_AUTHOR_ID,
                ref_kind,
                false,
            );
            let sheet_version_ref = character
                .sheet_version_ref()
                .unwrap_or_else(|| "pending-first-sheet-version".to_owned());
            let sheet_ref_response = ui.label(format!("sheet_version_ref: {sheet_version_ref}"));
            emit_node(
                ui.ctx(),
                sheet_ref_response.id,
                accesskit::Role::Label,
                ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID,
                &sheet_version_ref,
                false,
            );
            ui.add_space(8.0);
            let editor = ui.add(
                egui::TextEdit::multiline(&mut character.sheet_editor_text)
                    .desired_rows(15)
                    .lock_focus(true),
            );
            emit_node(
                ui.ctx(),
                editor.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
                "CKC character sheet editor",
                false,
            );
            let save = ui.button("Append sheet version");
            emit_node(
                ui.ctx(),
                save.id,
                accesskit::Role::Button,
                ATELIER_CKC_SHEET_SAVE_AUTHOR_ID,
                "Append CKC sheet version",
                append_pending,
            );
            if save.clicked() {
                if self.ckc_client.is_some() {
                    if !append_pending {
                        pending_append_request = Some((
                            character.character_internal_id.clone(),
                            character.sheet_editor_text.clone(),
                            character.sheet_version_id.clone(),
                        ));
                    }
                } else {
                    character.parent_sheet_version_id = character.sheet_version_id.clone();
                    let next_sheet_version_id = Uuid::new_v4().to_string();
                    character.sheet_version_id = Some(next_sheet_version_id.clone());
                    character.sheet_seq += 1;
                    character.sheet_version_ref = Some(format!(
                        "atelier://sheet/{}/{}",
                        character.character_internal_id, next_sheet_version_id
                    ));
                    clear_last_export = true;
                }
            }
        } else {
            ui.label(egui::RichText::new("No CKC characters yet").color(palette.text_subtle));
        }
        if clear_last_export {
            state.ckc_last_export = None;
        }
        if let Some((character_internal_id, raw_text, expected_parent_version_id)) =
            pending_append_request
        {
            if let Some(client) = self.ckc_client.as_ref() {
                state.ckc_append_pending = true;
                state.ckc_error = None;
                client.append_ckc_sheet_version(
                    &character_internal_id,
                    &raw_text,
                    expected_parent_version_id.as_deref(),
                    Some("handshake-native-atelier"),
                    client.actor_id(),
                    self.ckc_append_cell.clone(),
                );
            }
        }
    }

    fn show_ckc_character_notes_panel(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        character: &mut CkcCharacterRecord,
        notes_buffer: &mut String,
        notes_source_key: &mut Option<String>,
        notes_status: &mut String,
    ) {
        let source_key = ckc_sheet_notes_source_key(character);
        if notes_source_key.as_deref() != Some(source_key.as_str()) {
            *notes_buffer = extract_ckc_sheet_notes(&character.sheet_editor_text);
            *notes_source_key = Some(source_key);
        }
        ui.heading(egui::RichText::new("Character sheet notes").color(palette.text));
        ui.label(egui::RichText::new(notes_status.as_str()).color(palette.text_subtle));
        ui.label(
            egui::RichText::new(
                "Image notes stay in the left media panel; this edits sheet notes.",
            )
            .color(palette.text_subtle),
        );
        let notes = ui.add(
            egui::TextEdit::multiline(notes_buffer)
                .desired_rows(8)
                .lock_focus(true),
        );
        emit_node(
            ui.ctx(),
            notes.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_CHARACTER_NOTES_EDITOR_AUTHOR_ID,
            "CKC character sheet notes editor",
            false,
        );
        let apply = ui.button("Apply notes to sheet");
        emit_node(
            ui.ctx(),
            apply.id,
            accesskit::Role::Button,
            ATELIER_CKC_CHARACTER_NOTES_APPLY_AUTHOR_ID,
            "Apply CKC character notes to the selected sheet text",
            false,
        );
        if apply.clicked() {
            upsert_ckc_sheet_notes(&mut character.sheet_editor_text, notes_buffer);
            *notes_status = format!(
                "Applied character sheet notes to {}. Append the sheet version to persist.",
                character
                    .sheet_version_ref()
                    .unwrap_or_else(|| "pending sheet".to_owned())
            );
        }
    }

    fn show_ckc_moodboard_canvas(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        ui.separator();
        ui.heading(egui::RichText::new("Moodboard").color(palette.text));
        ui.add_space(4.0);
        let mut event = None;
        let canvas_response = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .id_salt(egui::Id::new(ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID)),
                |ui| {
                    if let Ok(mut board) = self.canvas_board.lock() {
                        event = board.show(ui, palette);
                        let drained = board.drain_knowledge_events();
                        if !drained.is_empty() {
                            if let Ok(mut q) = self.canvas_events.lock() {
                                q.extend(drained);
                            }
                        }
                    }
                },
            )
            .response;
        emit_node(
            ui.ctx(),
            canvas_response.id,
            accesskit::Role::Group,
            ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID,
            "Native CKC moodboard canvas surface",
            false,
        );
        if let Some(ev) = event {
            if let Ok(mut q) = self.canvas_events.lock() {
                q.push(ev);
            }
        }
    }

    fn show_ckc_middle_work_panel(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        state: &mut AtelierPanelState,
        selected_index: usize,
    ) {
        match state.ckc_book_mode {
            CkcBookMode::Sheet => {}
            CkcBookMode::Notes => {
                let mut notes_buffer = std::mem::take(&mut state.ckc_character_notes_buffer);
                let mut notes_source_key =
                    std::mem::take(&mut state.ckc_character_notes_source_key);
                let mut notes_status = std::mem::take(&mut state.ckc_character_notes_status);
                if let Some(character) = state.ckc_characters.get_mut(selected_index) {
                    self.show_ckc_character_notes_panel(
                        ui,
                        palette,
                        character,
                        &mut notes_buffer,
                        &mut notes_source_key,
                        &mut notes_status,
                    );
                } else {
                    ui.label(
                        egui::RichText::new("No CKC character selected for notes.")
                            .color(palette.text_subtle),
                    );
                }
                state.ckc_character_notes_buffer = notes_buffer;
                state.ckc_character_notes_source_key = notes_source_key;
                state.ckc_character_notes_status = notes_status;
            }
            CkcBookMode::Story | CkcBookMode::Moodboard => {
                let mode = state.ckc_book_mode;
                let mut story_card_title = std::mem::take(&mut state.ckc_story_card_title);
                let mut story_card_body = std::mem::take(&mut state.ckc_story_card_body);
                let mut story_beat_text = std::mem::take(&mut state.ckc_story_beat_text);
                let mut story_status = std::mem::take(&mut state.ckc_story_status);
                let mut moodboard_status = std::mem::take(&mut state.ckc_moodboard_status);
                let mut active_story_document_id =
                    std::mem::take(&mut state.ckc_active_story_document_id);
                let mut active_moodboard_document_id =
                    std::mem::take(&mut state.ckc_active_moodboard_document_id);
                if let Some(character) = state.ckc_characters.get_mut(selected_index) {
                    self.show_ckc_story_and_moodboard(
                        ui,
                        palette,
                        character,
                        &mut story_card_title,
                        &mut story_card_body,
                        &mut story_beat_text,
                        &mut story_status,
                        &mut moodboard_status,
                        &mut active_story_document_id,
                        &mut active_moodboard_document_id,
                        mode,
                    );
                    if mode == CkcBookMode::Moodboard {
                        self.show_ckc_moodboard_canvas(ui, palette);
                    }
                } else {
                    ui.label(
                        egui::RichText::new("No CKC character selected for this work surface.")
                            .color(palette.text_subtle),
                    );
                }
                state.ckc_story_card_title = story_card_title;
                state.ckc_story_card_body = story_card_body;
                state.ckc_story_beat_text = story_beat_text;
                state.ckc_story_status = story_status;
                state.ckc_moodboard_status = moodboard_status;
                state.ckc_active_story_document_id = active_story_document_id;
                state.ckc_active_moodboard_document_id = active_moodboard_document_id;
            }
        }
    }

    fn show_ckc_story_and_moodboard(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        character: &mut CkcCharacterRecord,
        story_card_title: &mut String,
        story_card_body: &mut String,
        story_beat_text: &mut String,
        story_status: &mut String,
        moodboard_status: &mut String,
        active_story_document_id: &mut Option<String>,
        active_moodboard_document_id: &mut Option<String>,
        mode: CkcBookMode,
    ) {
        if character.story_documents.is_empty() {
            character.story_documents.push(pending_ckc_story_document(
                &character.character_internal_id,
                &character.display_name,
            ));
        }
        if character.moodboard_documents.is_empty() {
            character
                .moodboard_documents
                .push(pending_ckc_moodboard_document(
                    &character.character_internal_id,
                    &character.display_name,
                ));
        }
        let mut active_story_idx = character
            .story_documents
            .iter()
            .position(|story| {
                active_story_document_id.as_deref() == Some(story.document_id.as_str())
            })
            .unwrap_or(0);
        *active_story_document_id = character
            .story_documents
            .get(active_story_idx)
            .map(|story| story.document_id.clone());
        let mut active_moodboard_idx = character
            .moodboard_documents
            .iter()
            .position(|moodboard| {
                active_moodboard_document_id.as_deref() == Some(moodboard.document_id.as_str())
            })
            .unwrap_or(0);
        *active_moodboard_document_id = character
            .moodboard_documents
            .get(active_moodboard_idx)
            .map(|moodboard| moodboard.document_id.clone());
        let character_internal_id = character.character_internal_id.clone();
        let display_name = character.display_name.clone();

        if mode == CkcBookMode::Story {
            ui.heading(egui::RichText::new("Story").color(palette.text));
            ui.label(egui::RichText::new(story_status.as_str()).color(palette.text_subtle));

            let _story_doc_list = ui
                .vertical(|ui| {
                    for (idx, story) in character.story_documents.iter().enumerate() {
                        let selected = idx == active_story_idx;
                        let marker = if selected { "active" } else { "linked" };
                        let row_label = format!(
                            "{marker} story document: {} [{}]",
                            story.title, story.document_ref
                        );
                        let row = ui.selectable_label(selected, &row_label);
                        if row.clicked() {
                            active_story_idx = idx;
                            *active_story_document_id = Some(story.document_id.clone());
                            *story_status =
                                format!("Selected CKC story document {}", story.document_ref);
                        }
                        let author_id = ckc_story_document_row_author_id(&story.document_id);
                        emit_node(
                            ui.ctx(),
                            row.id,
                            accesskit::Role::ListItem,
                            &author_id,
                            &row_label,
                            selected,
                        );
                    }
                })
                .response;

            {
                let story = character
                    .story_documents
                    .get_mut(active_story_idx)
                    .expect("story placeholder inserted before rendering");
                let story_ref_label = format!("story_document_ref: {}", story.document_ref);
                let story_ref_response = ui.label(&story_ref_label);
                emit_node(
                    ui.ctx(),
                    story_ref_response.id,
                    accesskit::Role::Label,
                    ATELIER_CKC_STORY_DOC_REF_AUTHOR_ID,
                    &story_ref_label,
                    false,
                );
                if !story.tags.is_empty() {
                    ui.label(
                        egui::RichText::new(format!("story tags: {}", story.tags.join(", ")))
                            .color(palette.text_subtle),
                    );
                }
                let story_editor = ui.add(
                    egui::TextEdit::multiline(&mut story.body_raw_text)
                        .desired_rows(4)
                        .lock_focus(true),
                );
                emit_node(
                    ui.ctx(),
                    story_editor.id,
                    accesskit::Role::TextInput,
                    ATELIER_CKC_STORY_EDITOR_AUTHOR_ID,
                    "CKC story document editor",
                    false,
                );
                let story_save = ui.button("Save story note");
                emit_node(
                    ui.ctx(),
                    story_save.id,
                    accesskit::Role::Button,
                    ATELIER_CKC_STORY_SAVE_AUTHOR_ID,
                    "Save CKC story document draft",
                    false,
                );
                if story_save.clicked() {
                    let tags = story.tags.clone();
                    if let Some(client) = self.ckc_client.as_ref() {
                        if is_pending_ckc_document_id(&story.document_id) {
                            *story_status =
                                format!("Creating CKC story document for {display_name}");
                            client.create_ckc_character_document(
                                &character_internal_id,
                                "story",
                                &story.title,
                                &story.body_raw_text,
                                &tags,
                                client.actor_id(),
                                self.ckc_character_document_cell.clone(),
                            );
                        } else {
                            *story_status =
                                format!("Appending CKC story document {}", story.document_ref);
                            let expected_parent_version_id = story.current_version_id.as_deref();
                            client.append_ckc_character_document_version(
                                &story.document_id,
                                &story.title,
                                &story.body_raw_text,
                                &tags,
                                expected_parent_version_id,
                                client.actor_id(),
                                self.ckc_character_document_cell.clone(),
                            );
                        }
                    } else {
                        if is_pending_ckc_document_id(&story.document_id) {
                            let next_document_id = Uuid::new_v4().to_string();
                            story.document_id = next_document_id.clone();
                            story.document_ref = format!("atelier://document/{next_document_id}");
                            story.current_version_id = Some(Uuid::new_v4().to_string());
                            story.current_version_seq = 1;
                        }
                        *story_status = format!(
                        "Saved local CKC story document draft {} separate from sheet/image/tag notes",
                        story.document_ref
                    );
                    }
                }

                let story_card_list = ui
                    .vertical(|ui| {
                        if story.cards.is_empty() {
                            ui.label(
                                egui::RichText::new("No story cards yet.")
                                    .color(palette.text_subtle),
                            );
                        }
                        for card in &story.cards {
                            let card_label = format!(
                                "{} [{}] document:{} ref:{}",
                                card.title,
                                card.card_ref,
                                card.story_document_id,
                                card.story_document_ref
                            );
                            let card_row = ui.label(&card_label);
                            let card_author_id =
                                ckc_story_card_row_author_id(&story.document_id, &card.card_id);
                            emit_node(
                                ui.ctx(),
                                card_row.id,
                                accesskit::Role::ListItem,
                                &card_author_id,
                                &format!(
                                    "atelier-ref story_card:{} story_document:{}",
                                    card.card_ref, card.story_document_ref
                                ),
                                false,
                            );
                            if !card.body_raw_text.is_empty() {
                                ui.label(
                                    egui::RichText::new(card.body_raw_text.clone())
                                        .color(palette.text_subtle),
                                );
                            }
                        }
                        for beat in &story.beats {
                            let card = beat.card_id.as_deref().unwrap_or("unscoped");
                            let beat_label = format!(
                                "beat {} {} document:{} ref:{} [{}]: {}",
                                beat.beat_id,
                                beat.beat_ref,
                                beat.story_document_id,
                                beat.story_document_ref,
                                card,
                                beat.beat_text
                            );
                            let beat_row = ui
                                .label(egui::RichText::new(&beat_label).color(palette.text_subtle));
                            let beat_author_id =
                                ckc_story_beat_row_author_id(&story.document_id, &beat.beat_id);
                            emit_node(
                                ui.ctx(),
                                beat_row.id,
                                accesskit::Role::ListItem,
                                &beat_author_id,
                                &format!(
                                    "atelier-ref story_beat:{} story_document:{}",
                                    beat.beat_ref, beat.story_document_ref
                                ),
                                false,
                            );
                        }
                    })
                    .response;
                emit_node(
                    ui.ctx(),
                    story_card_list.id,
                    accesskit::Role::List,
                    ATELIER_CKC_STORY_CARD_LIST_AUTHOR_ID,
                    "CKC reusable story cards and beats",
                    false,
                );

                ui.horizontal_wrapped(|ui| {
                    let title = ui.text_edit_singleline(story_card_title);
                    emit_node(
                        ui.ctx(),
                        title.id,
                        accesskit::Role::TextInput,
                        ATELIER_CKC_STORY_CARD_TITLE_AUTHOR_ID,
                        "CKC story card title",
                        false,
                    );
                    let save_card = ui.button("Add card");
                    emit_node(
                        ui.ctx(),
                        save_card.id,
                        accesskit::Role::Button,
                        ATELIER_CKC_STORY_CARD_SAVE_AUTHOR_ID,
                        "Add CKC story card",
                        false,
                    );
                    if save_card.clicked() {
                        let title = story_card_title.trim().to_owned();
                        if !title.is_empty() {
                            let body = story_card_body.trim().to_owned();
                            let tags = vec!["story".to_owned()];
                            if let Some(client) = self.ckc_client.as_ref() {
                                if is_pending_ckc_document_id(&story.document_id) {
                                    *story_status =
                                        "Save the CKC story document before adding cards."
                                            .to_owned();
                                } else {
                                    *story_status = format!(
                                        "Adding CKC story card {title} under {}",
                                        story.document_ref
                                    );
                                    client.add_ckc_story_card(
                                        &story.document_id,
                                        &title,
                                        &body,
                                        &tags,
                                        client.actor_id(),
                                        self.ckc_story_card_cell.clone(),
                                    );
                                }
                            } else {
                                if is_pending_ckc_document_id(&story.document_id) {
                                    let next_document_id = Uuid::new_v4().to_string();
                                    story.document_id = next_document_id.clone();
                                    story.document_ref =
                                        format!("atelier://document/{next_document_id}");
                                }
                                let card_id = Uuid::new_v4().to_string();
                                story.cards.push(CkcStoryCardRecord {
                                    card_id: card_id.clone(),
                                    card_ref: format!("atelier://story-card/{card_id}"),
                                    story_document_id: story.document_id.clone(),
                                    story_document_ref: story.document_ref.clone(),
                                    title: title.clone(),
                                    body_raw_text: body,
                                    tags,
                                });
                                *story_status = format!(
                                    "Added local CKC story card {title} under {}",
                                    story.document_ref
                                );
                            }
                        }
                    }
                });
                let card_body = ui.add(
                    egui::TextEdit::multiline(story_card_body)
                        .desired_rows(3)
                        .lock_focus(true),
                );
                emit_node(
                    ui.ctx(),
                    card_body.id,
                    accesskit::Role::TextInput,
                    ATELIER_CKC_STORY_CARD_BODY_AUTHOR_ID,
                    "CKC story card body",
                    false,
                );
                let beat_editor = ui.add(
                    egui::TextEdit::multiline(story_beat_text)
                        .desired_rows(2)
                        .lock_focus(true),
                );
                emit_node(
                    ui.ctx(),
                    beat_editor.id,
                    accesskit::Role::TextInput,
                    ATELIER_CKC_STORY_BEAT_EDITOR_AUTHOR_ID,
                    "CKC story beat editor",
                    false,
                );
                let save_beat = ui.button("Add beat");
                emit_node(
                    ui.ctx(),
                    save_beat.id,
                    accesskit::Role::Button,
                    ATELIER_CKC_STORY_BEAT_SAVE_AUTHOR_ID,
                    "Add CKC story beat",
                    false,
                );
                if save_beat.clicked() {
                    let beat_text = story_beat_text.trim().to_owned();
                    if !beat_text.is_empty() {
                        let card_id = story.cards.first().map(|card| card.card_id.clone());
                        if let Some(client) = self.ckc_client.as_ref() {
                            if is_pending_ckc_document_id(&story.document_id) {
                                *story_status =
                                    "Save the CKC story document before adding beats.".to_owned();
                            } else {
                                *story_status =
                                    format!("Adding CKC story beat under {}", story.document_ref);
                                client.add_ckc_story_beat(
                                    &story.document_id,
                                    card_id.as_deref(),
                                    &beat_text,
                                    client.actor_id(),
                                    self.ckc_story_beat_cell.clone(),
                                );
                            }
                        } else {
                            if is_pending_ckc_document_id(&story.document_id) {
                                let next_document_id = Uuid::new_v4().to_string();
                                story.document_id = next_document_id.clone();
                                story.document_ref =
                                    format!("atelier://document/{next_document_id}");
                            }
                            let beat_id = Uuid::new_v4().to_string();
                            story.beats.push(CkcStoryBeatRecord {
                                beat_id: beat_id.clone(),
                                beat_ref: format!("atelier://story-beat/{beat_id}"),
                                story_document_id: story.document_id.clone(),
                                story_document_ref: story.document_ref.clone(),
                                card_id,
                                beat_text,
                            });
                            *story_status =
                                format!("Added local CKC story beat under {}", story.document_ref);
                        }
                    }
                }
            }
        }

        if mode == CkcBookMode::Moodboard {
            ui.separator();
            ui.heading(egui::RichText::new("Moodboard links").color(palette.text));
            ui.label(egui::RichText::new(moodboard_status.as_str()).color(palette.text_subtle));

            let _moodboard_doc_list = ui
                .vertical(|ui| {
                    for (idx, moodboard) in character.moodboard_documents.iter().enumerate() {
                        let selected = idx == active_moodboard_idx;
                        let marker = if selected { "active" } else { "linked" };
                        let row_label = format!(
                            "{marker} moodboard document: {} [{}]",
                            moodboard.title, moodboard.document_ref
                        );
                        let row = ui.selectable_label(selected, &row_label);
                        if row.clicked() {
                            active_moodboard_idx = idx;
                            *active_moodboard_document_id = Some(moodboard.document_id.clone());
                            *moodboard_status = format!(
                                "Selected CKC moodboard document {}",
                                moodboard.document_ref
                            );
                        }
                        let author_id =
                            ckc_moodboard_document_row_author_id(&moodboard.document_id);
                        emit_node(
                            ui.ctx(),
                            row.id,
                            accesskit::Role::ListItem,
                            &author_id,
                            &row_label,
                            selected,
                        );
                        if let Some(snapshot_id) = &moodboard.latest_snapshot_id {
                            let latest_ref = moodboard
                                .latest_snapshot_ref
                                .clone()
                                .unwrap_or_else(|| format!("atelier://moodboard/{snapshot_id}"));
                            let snapshot_author_id = ckc_moodboard_snapshot_row_author_id(
                                &moodboard.document_id,
                                snapshot_id,
                            );
                            let snapshot_label = format!(
                                "{marker} moodboard snapshot: {} [{}] {}",
                                moodboard.moodboard_name, snapshot_id, latest_ref
                            );
                            let snapshot_row = ui.label(
                                egui::RichText::new(&snapshot_label).color(palette.text_subtle),
                            );
                            emit_node(
                                ui.ctx(),
                                snapshot_row.id,
                                accesskit::Role::ListItem,
                                &snapshot_author_id,
                                &format!(
                                    "atelier-ref moodboard:{latest_ref} document:{}",
                                    moodboard.document_ref
                                ),
                                selected,
                            );
                        }
                    }
                })
                .response;

            {
                let moodboard = character
                    .moodboard_documents
                    .get_mut(active_moodboard_idx)
                    .expect("moodboard placeholder inserted before rendering");
                let doc_ref_label = format!("moodboard_document_ref: {}", moodboard.document_ref);
                let doc_ref_response = ui.label(&doc_ref_label);
                emit_node(
                    ui.ctx(),
                    doc_ref_response.id,
                    accesskit::Role::Label,
                    ATELIER_CKC_MOODBOARD_DOC_REF_AUTHOR_ID,
                    &doc_ref_label,
                    false,
                );
                let latest_ref = moodboard
                    .latest_snapshot_ref
                    .clone()
                    .unwrap_or_else(|| "pending-native-moodboard-snapshot".to_owned());
                let latest_ref_label = format!("latest_moodboard_ref: {latest_ref}");
                let latest_ref_response = ui.label(&latest_ref_label);
                emit_node(
                    ui.ctx(),
                    latest_ref_response.id,
                    accesskit::Role::Label,
                    ATELIER_CKC_MOODBOARD_LATEST_REF_AUTHOR_ID,
                    &latest_ref_label,
                    false,
                );
                if !moodboard.tags.is_empty() {
                    ui.label(
                        egui::RichText::new(format!(
                            "moodboard tags: {}",
                            moodboard.tags.join(", ")
                        ))
                        .color(palette.text_subtle),
                    );
                }
                let moodboard_editor = ui.add(
                    egui::TextEdit::multiline(&mut moodboard.body_raw_text)
                        .desired_rows(5)
                        .lock_focus(true),
                );
                emit_node(
                    ui.ctx(),
                    moodboard_editor.id,
                    accesskit::Role::TextInput,
                    ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID,
                    "CKC moodboard native snapshot JSON editor",
                    false,
                );
                let save_moodboard = ui.button("Save moodboard");
                emit_node(
                    ui.ctx(),
                    save_moodboard.id,
                    accesskit::Role::Button,
                    ATELIER_CKC_MOODBOARD_SAVE_AUTHOR_ID,
                    "Save CKC native moodboard snapshot",
                    false,
                );
                if save_moodboard.clicked() {
                    if moodboard.body_raw_text.trim().is_empty() {
                        moodboard.body_raw_text = local_ckc_moodboard_snapshot_json(
                            &moodboard.document_id,
                            &moodboard.moodboard_name,
                            &format!(
                                "Native CKC moodboard for {} linked to {}",
                                display_name, moodboard.document_ref
                            ),
                        );
                    }
                    if let Err(err) =
                        ckc_moodboard_snapshot_to_canvas_projection(&moodboard.body_raw_text)
                    {
                        *moodboard_status = format!("CKC moodboard save blocked: {err}");
                    } else if let Some(client) = self.ckc_client.as_ref() {
                        if is_pending_ckc_document_id(&moodboard.document_id) {
                            *moodboard_status = format!(
                                "Creating CKC moodboard document and snapshot for {display_name}"
                            );
                            client.create_ckc_moodboard_document_snapshot(
                                &character_internal_id,
                                &moodboard.title,
                                &moodboard.body_raw_text,
                                &moodboard.tags,
                                client.actor_id(),
                                self.ckc_character_document_cell.clone(),
                                self.ckc_moodboard_latest_cell.clone(),
                            );
                        } else {
                            let expected_parent_version_id =
                                moodboard.current_version_id.as_deref();
                            *moodboard_status = format!(
                                "Saving CKC moodboard document and snapshot for {}",
                                moodboard.document_ref
                            );
                            client.save_ckc_moodboard_document_snapshot(
                                &moodboard.document_id,
                                &moodboard.title,
                                &moodboard.body_raw_text,
                                &moodboard.tags,
                                expected_parent_version_id,
                                client.actor_id(),
                                self.ckc_character_document_cell.clone(),
                                self.ckc_moodboard_latest_cell.clone(),
                            );
                        }
                    } else {
                        if is_pending_ckc_document_id(&moodboard.document_id) {
                            let next_document_id = Uuid::new_v4().to_string();
                            moodboard.document_id = next_document_id.clone();
                            moodboard.document_ref =
                                format!("atelier://document/{next_document_id}");
                        }
                        moodboard.current_version_id = Some(Uuid::new_v4().to_string());
                        moodboard.current_version_seq = (moodboard.current_version_seq + 1).max(1);
                        let snapshot_id = Uuid::new_v4().to_string();
                        moodboard.latest_snapshot_id = Some(snapshot_id.clone());
                        moodboard.latest_snapshot_ref =
                            Some(format!("atelier://moodboard/{snapshot_id}"));
                        let latest_ref = moodboard
                            .latest_snapshot_ref
                            .clone()
                            .unwrap_or_else(|| "pending-native-moodboard-snapshot".to_owned());
                        match apply_ckc_moodboard_snapshot_to_board(
                            &self.canvas_board,
                            &moodboard.body_raw_text,
                            &latest_ref,
                        ) {
                            Ok(()) => {
                                *moodboard_status =
                                    format!("Saved local CKC moodboard {latest_ref}");
                            }
                            Err(err) => {
                                *moodboard_status =
                                    format!("CKC local moodboard save projection failed: {err}");
                            }
                        }
                    }
                }
                let open = ui.button("Open moodboard");
                emit_node(
                    ui.ctx(),
                    open.id,
                    accesskit::Role::Button,
                    ATELIER_CKC_MOODBOARD_OPEN_AUTHOR_ID,
                    "Open CKC native moodboard snapshot",
                    false,
                );
                if open.clicked() {
                    if let Some(client) = self.ckc_client.as_ref() {
                        if is_pending_ckc_document_id(&moodboard.document_id) {
                            *moodboard_status = format!(
                                "Creating CKC moodboard document and snapshot for {display_name}"
                            );
                            client.create_ckc_moodboard_document_snapshot(
                                &character_internal_id,
                                &moodboard.title,
                                &moodboard.body_raw_text,
                                &moodboard.tags,
                                client.actor_id(),
                                self.ckc_character_document_cell.clone(),
                                self.ckc_moodboard_latest_cell.clone(),
                            );
                        } else {
                            *moodboard_status = format!(
                                "Opening CKC moodboard snapshot for {}",
                                moodboard.document_ref
                            );
                            client.fetch_ckc_latest_moodboard_snapshot(
                                &moodboard.document_id,
                                self.ckc_moodboard_latest_cell.clone(),
                            );
                        }
                    } else {
                        if is_pending_ckc_document_id(&moodboard.document_id) {
                            let next_document_id = Uuid::new_v4().to_string();
                            moodboard.document_id = next_document_id.clone();
                            moodboard.document_ref =
                                format!("atelier://document/{next_document_id}");
                            moodboard.current_version_id = Some(Uuid::new_v4().to_string());
                            moodboard.current_version_seq = 1;
                        }
                        if moodboard.latest_snapshot_id.is_none() {
                            let snapshot_id = Uuid::new_v4().to_string();
                            moodboard.latest_snapshot_id = Some(snapshot_id.clone());
                            moodboard.latest_snapshot_ref =
                                Some(format!("atelier://moodboard/{snapshot_id}"));
                        }
                        if ckc_moodboard_snapshot_to_canvas_projection(&moodboard.body_raw_text)
                            .is_err()
                        {
                            moodboard.body_raw_text = local_ckc_moodboard_snapshot_json(
                                &moodboard.document_id,
                                &moodboard.moodboard_name,
                                &format!(
                                    "Native CKC moodboard for {} linked to {}",
                                    display_name, moodboard.document_ref
                                ),
                            );
                        }
                        let latest_ref = moodboard
                            .latest_snapshot_ref
                            .clone()
                            .unwrap_or_else(|| "pending-native-moodboard-snapshot".to_owned());
                        match apply_ckc_moodboard_snapshot_to_board(
                            &self.canvas_board,
                            &moodboard.body_raw_text,
                            &latest_ref,
                        ) {
                            Ok(()) => {
                                *moodboard_status =
                                    format!("Opened local CKC moodboard {latest_ref}");
                            }
                            Err(err) => {
                                *moodboard_status =
                                    format!("CKC local moodboard projection failed: {err}");
                            }
                        }
                    }
                }
            }
        }
    }

    fn show_ckc_sheet_tools(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        state: &mut AtelierPanelState,
        selected_index: usize,
    ) {
        ui.heading(egui::RichText::new("Sheet tools").color(palette.text));
        let template_status = ui.label(&state.ckc_template_status);
        emit_node(
            ui.ctx(),
            template_status.id,
            accesskit::Role::Label,
            ATELIER_CKC_TEMPLATE_STATUS_AUTHOR_ID,
            &state.ckc_template_status,
            state.ckc_template_pending || state.ckc_safe_subset_pending,
        );
        ui.horizontal_wrapped(|ui| {
            let load_template = ui.add_enabled(
                !state.ckc_template_pending,
                egui::Button::new("Load template"),
            );
            emit_node(
                ui.ctx(),
                load_template.id,
                accesskit::Role::Button,
                ATELIER_CKC_TEMPLATE_LOAD_AUTHOR_ID,
                "Load bundled CKC CHARACTER_SHEET__v2.00.txt metadata",
                state.ckc_template_pending,
            );
            if load_template.clicked() {
                if let Some(client) = self.ckc_client.as_ref() {
                    state.ckc_template_pending = true;
                    state.ckc_template_status = "Loading CHARACTER_SHEET__v2.00.txt".to_owned();
                    client.fetch_ckc_template(self.ckc_template_cell.clone());
                } else {
                    state.ckc_template_status =
                        "CHARACTER_SHEET__v2.00.txt is bundled locally; live backend not connected."
                            .to_owned();
                }
            }

            let load_safe = ui.add_enabled(
                !state.ckc_safe_subset_pending,
                egui::Button::new("Safe subset"),
            );
            emit_node(
                ui.ctx(),
                load_safe.id,
                accesskit::Role::Button,
                ATELIER_CKC_SAFE_SUBSET_LOAD_AUTHOR_ID,
                "Load CKC LLM_SAFE_SUBSET__v2.00.json short/SFW-safe Field ID subset",
                state.ckc_safe_subset_pending,
            );
            if load_safe.clicked() {
                if let Some(client) = self.ckc_client.as_ref() {
                    state.ckc_safe_subset_pending = true;
                    state.ckc_template_status = "Loading LLM_SAFE_SUBSET__v2.00.json".to_owned();
                    client.fetch_ckc_safe_subset(self.ckc_safe_subset_cell.clone());
                } else {
                    state.ckc_template_status =
                        "LLM_SAFE_SUBSET__v2.00.json is a bundled Field ID whitelist for short/SFW-safe use."
                            .to_owned();
                }
            }
        });

        ui.label(egui::RichText::new("Import raw sheet text").color(palette.text_subtle));
        let import_editor = ui.add(
            egui::TextEdit::multiline(&mut state.ckc_import_text)
                .desired_rows(3)
                .lock_focus(true),
        );
        emit_node(
            ui.ctx(),
            import_editor.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_IMPORT_EDITOR_AUTHOR_ID,
            "CKC raw character sheet import text",
            false,
        );

        let selected_sheet = state.ckc_characters.get(selected_index).map(|character| {
            (
                character.character_internal_id.clone(),
                character.sheet_version_id.clone(),
            )
        });
        ui.horizontal_wrapped(|ui| {
            let import_enabled = !state.ckc_import_pending
                && selected_sheet.is_some()
                && !state.ckc_import_text.trim().is_empty();
            let import = ui.add_enabled(import_enabled, egui::Button::new("Import sheet"));
            emit_node(
                ui.ctx(),
                import.id,
                accesskit::Role::Button,
                ATELIER_CKC_IMPORT_AUTHOR_ID,
                "Import CKC raw sheet text as a guarded append-only version",
                state.ckc_import_pending || !import_enabled,
            );
            if import.clicked() {
                if let Some((character_internal_id, expected_parent_version_id)) =
                    selected_sheet.clone()
                {
                    if let Some(client) = self.ckc_client.as_ref() {
                        state.ckc_import_pending = true;
                        state.ckc_export_status =
                            "Importing CKC sheet as append-only version".to_owned();
                        client.import_ckc_sheet_version(
                            &character_internal_id,
                            &state.ckc_import_text,
                            expected_parent_version_id.as_deref(),
                            Some("handshake-native-atelier-import"),
                            client.actor_id(),
                            self.ckc_import_cell.clone(),
                        );
                    } else {
                        local_import_ckc_sheet(state, selected_index);
                    }
                }
            }

            let selected_version_id = selected_sheet
                .as_ref()
                .and_then(|(_, version_id)| version_id.clone());
            let export_enabled = !state.ckc_export_pending && selected_version_id.is_some();
            let export_txt = ui.add_enabled(export_enabled, egui::Button::new("Export txt"));
            emit_node(
                ui.ctx(),
                export_txt.id,
                accesskit::Role::Button,
                ATELIER_CKC_EXPORT_TXT_AUTHOR_ID,
                "Export CKC sheet version as deterministic txt content",
                state.ckc_export_pending || !export_enabled,
            );
            if export_txt.clicked() {
                self.request_ckc_sheet_export(state, selected_index, "txt");
            }
            let export_json = ui.add_enabled(export_enabled, egui::Button::new("Export json"));
            emit_node(
                ui.ctx(),
                export_json.id,
                accesskit::Role::Button,
                ATELIER_CKC_EXPORT_JSON_AUTHOR_ID,
                "Export CKC sheet version as deterministic json content",
                state.ckc_export_pending || !export_enabled,
            );
            if export_json.clicked() {
                self.request_ckc_sheet_export(state, selected_index, "json");
            }
            let export_safe_txt =
                ui.add_enabled(export_enabled, egui::Button::new("Export safe txt"));
            emit_node(
                ui.ctx(),
                export_safe_txt.id,
                accesskit::Role::Button,
                ATELIER_CKC_EXPORT_SAFE_TXT_AUTHOR_ID,
                "Export CKC sheet version as short/SFW-safe txt content",
                state.ckc_export_pending || !export_enabled,
            );
            if export_safe_txt.clicked() {
                self.request_ckc_sheet_export(state, selected_index, "safe-txt");
            }
            let export_safe_json =
                ui.add_enabled(export_enabled, egui::Button::new("Export safe json"));
            emit_node(
                ui.ctx(),
                export_safe_json.id,
                accesskit::Role::Button,
                ATELIER_CKC_EXPORT_SAFE_JSON_AUTHOR_ID,
                "Export CKC sheet version as short/SFW-safe json content",
                state.ckc_export_pending || !export_enabled,
            );
            if export_safe_json.clicked() {
                self.request_ckc_sheet_export(state, selected_index, "safe-json");
            }
        });
        let export_status = ui.label(&state.ckc_export_status);
        emit_node(
            ui.ctx(),
            export_status.id,
            accesskit::Role::Label,
            ATELIER_CKC_EXPORT_STATUS_AUTHOR_ID,
            &state.ckc_export_status,
            state.ckc_export_pending || state.ckc_import_pending,
        );
        if let Some(export) = state.ckc_last_export.as_ref() {
            let export_ref_label = format!(
                "{} {} {} {} {}",
                export.file_name,
                export.version_id,
                short_hash(&export.content_hash),
                export.character_ref,
                export.sheet_version_ref
            );
            let export_ref = ui.label(&export_ref_label);
            emit_node(
                ui.ctx(),
                export_ref.id,
                accesskit::Role::Label,
                ATELIER_CKC_EXPORT_REF_AUTHOR_ID,
                &export_ref_label,
                false,
            );
            let mut preview = export.content.clone();
            let preview_response = ui.add(
                egui::TextEdit::multiline(&mut preview)
                    .desired_rows(4)
                    .interactive(false),
            );
            emit_value_node(
                ui.ctx(),
                preview_response.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_EXPORT_PREVIEW_AUTHOR_ID,
                "CKC deterministic sheet export preview",
                &export.content,
            );
        }

        ui.horizontal_wrapped(|ui| {
            ui.label("Field ID");
            let field = ui.text_edit_singleline(&mut state.ckc_field_suggestion_id);
            emit_node(
                ui.ctx(),
                field.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_FIELD_SUGGESTION_FIELD_AUTHOR_ID,
                "CKC Field ID for prior-value suggestions",
                false,
            );
            let load = ui.add_enabled(
                !state.ckc_field_suggestion_pending
                    && !state.ckc_field_suggestion_id.trim().is_empty(),
                egui::Button::new("Load suggestions"),
            );
            emit_node(
                ui.ctx(),
                load.id,
                accesskit::Role::Button,
                ATELIER_CKC_FIELD_SUGGESTIONS_LOAD_AUTHOR_ID,
                "Load CKC prior values for the exact Field ID",
                state.ckc_field_suggestion_pending,
            );
            if load.clicked() {
                let field_id = state.ckc_field_suggestion_id.trim().to_owned();
                if let Some(client) = self.ckc_client.as_ref() {
                    state.ckc_field_suggestion_pending = true;
                    state.ckc_field_suggestion_status =
                        format!("Loading prior CKC values for {field_id}");
                    client.fetch_ckc_field_suggestions(
                        &field_id,
                        8,
                        self.ckc_field_suggestions_cell.clone(),
                    );
                } else {
                    state.ckc_field_suggestions =
                        local_field_suggestions(&state.ckc_characters, &field_id);
                    state.ckc_field_suggestion_status = format!(
                        "Loaded {} local prior value(s) for {field_id}",
                        state.ckc_field_suggestions.len()
                    );
                }
            }
        });
        let suggestion_response = ui
            .vertical(|ui| {
                ui.label(&state.ckc_field_suggestion_status);
                for suggestion in &state.ckc_field_suggestions {
                    let label = format!(
                        "{} = {} ({})",
                        suggestion.field_id, suggestion.value, suggestion.occurrences
                    );
                    let row = ui.label(&label);
                    let author_id =
                        ckc_field_suggestion_row_author_id(&suggestion.field_id, &suggestion.value);
                    emit_node(
                        ui.ctx(),
                        row.id,
                        accesskit::Role::ListItem,
                        &author_id,
                        &label,
                        false,
                    );
                }
            })
            .response;
        emit_node(
            ui.ctx(),
            suggestion_response.id,
            accesskit::Role::List,
            ATELIER_CKC_FIELD_SUGGESTIONS_LIST_AUTHOR_ID,
            &state.ckc_field_suggestion_status,
            state.ckc_field_suggestion_pending,
        );
    }

    fn request_ckc_sheet_export(
        &self,
        state: &mut AtelierPanelState,
        selected_index: usize,
        format: &'static str,
    ) {
        let Some(character) = state.ckc_characters.get(selected_index) else {
            state.ckc_export_status = "No CKC character selected for export.".to_owned();
            return;
        };
        let Some(version_id) = character.sheet_version_id.clone() else {
            state.ckc_export_status =
                "Selected CKC character has no sheet version to export.".to_owned();
            return;
        };
        if let Some(client) = self.ckc_client.as_ref() {
            state.ckc_export_pending = true;
            state.ckc_export_status = format!("Exporting CKC sheet as {format}");
            client.export_ckc_sheet_version(&version_id, format, self.ckc_export_cell.clone());
        } else {
            let export = local_export_ckc_sheet(character, format);
            state.ckc_export_status = format!(
                "Local export {} as {} ({} bytes, hash {})",
                export.file_name,
                export.format,
                export.content.len(),
                short_hash(&export.content_hash)
            );
            state.ckc_last_export = Some(export);
        }
    }

    fn show_ckc_search(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        state: &mut AtelierPanelState,
    ) {
        ui.heading(egui::RichText::new("Search").color(palette.text));
        let query = ui.text_edit_singleline(&mut state.ckc_search_query);
        emit_node(
            ui.ctx(),
            query.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_SEARCH_QUERY_AUTHOR_ID,
            "CKC fuzzy vector combined search query",
            false,
        );
        let tags = ui.text_edit_singleline(&mut state.ckc_search_tags);
        emit_node(
            ui.ctx(),
            tags.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_SEARCH_TAGS_AUTHOR_ID,
            "CKC rich tag filter",
            false,
        );
        ui.horizontal_wrapped(|ui| {
            let character =
                ui.checkbox(&mut state.ckc_search_filter_selected_character, "Character");
            emit_node(
                ui.ctx(),
                character.id,
                accesskit::Role::CheckBox,
                ATELIER_CKC_SEARCH_FILTER_CHARACTER_AUTHOR_ID,
                "Filter CKC search to the selected character",
                state.ckc_search_filter_selected_character,
            );
            let collection = ui.checkbox(&mut state.ckc_search_filter_selected_collection, "Album");
            emit_node(
                ui.ctx(),
                collection.id,
                accesskit::Role::CheckBox,
                ATELIER_CKC_SEARCH_FILTER_COLLECTION_AUTHOR_ID,
                "Filter CKC search to the selected album",
                state.ckc_search_filter_selected_collection,
            );
            let media = ui.checkbox(&mut state.ckc_search_filter_selected_media, "Media");
            emit_node(
                ui.ctx(),
                media.id,
                accesskit::Role::CheckBox,
                ATELIER_CKC_SEARCH_FILTER_MEDIA_AUTHOR_ID,
                "Filter CKC search to the selected media asset",
                state.ckc_search_filter_selected_media,
            );
            let similarity = ui.checkbox(
                &mut state.ckc_search_use_selected_media_similarity,
                "Similarity",
            );
            emit_node(
                ui.ctx(),
                similarity.id,
                accesskit::Role::CheckBox,
                ATELIER_CKC_SEARCH_FILTER_SIMILARITY_AUTHOR_ID,
                "Use selected media as CKC image-similarity source",
                state.ckc_search_use_selected_media_similarity,
            );
        });
        ui.horizontal(|ui| {
            for mode in CkcSearchMode::ALL {
                let selected = state.ckc_search_mode == mode;
                let button = ui.add(egui::Button::selectable(selected, mode.label()));
                emit_node(
                    ui.ctx(),
                    button.id,
                    accesskit::Role::Button,
                    mode.author_id(),
                    mode.label(),
                    selected,
                );
                if button.clicked() {
                    state.ckc_search_mode = mode;
                }
            }
        });
        let run = ui.add_enabled(!state.ckc_search_pending, egui::Button::new("Search CKC"));
        emit_node(
            ui.ctx(),
            run.id,
            accesskit::Role::Button,
            ATELIER_CKC_SEARCH_RUN_AUTHOR_ID,
            "Run CKC search",
            state.ckc_search_pending,
        );
        if run.clicked() {
            let tags = ckc_tags_from_buffer(&state.ckc_search_tags);
            let mode = state.ckc_search_mode;
            let filters = selected_ckc_search_filter_refs(state);
            let character_internal_id = if state.ckc_search_filter_selected_character {
                filters.character_internal_id.as_deref()
            } else {
                None
            };
            let collection_id = if state.ckc_search_filter_selected_collection {
                filters.collection_id.as_deref()
            } else {
                None
            };
            let media_asset_id = if state.ckc_search_filter_selected_media {
                filters.media_asset_id.as_deref()
            } else {
                None
            };
            let similar_to_asset_id = if state.ckc_search_use_selected_media_similarity {
                filters.media_asset_id.as_deref()
            } else {
                None
            };
            if let Some(client) = self.ckc_client.as_ref() {
                state.ckc_search_pending = true;
                state.ckc_search_status = format!("Searching CKC with {} mode", mode.label());
                let modes = vec![mode.backend_value().to_owned()];
                client.search_ckc(
                    &state.ckc_search_query,
                    &modes,
                    &tags,
                    character_internal_id,
                    collection_id,
                    media_asset_id,
                    similar_to_asset_id,
                    None,
                    12,
                    self.ckc_search_cell.clone(),
                );
            } else {
                let mut results =
                    local_ckc_search(&state.ckc_characters, &state.ckc_search_query, mode, &tags);
                results.retain(|result| {
                    ckc_search_result_matches_filters(
                        result,
                        &filters,
                        state.ckc_search_filter_selected_character,
                        state.ckc_search_filter_selected_collection,
                        state.ckc_search_filter_selected_media
                            || state.ckc_search_use_selected_media_similarity,
                    )
                });
                state.ckc_search_results = results;
                state.ckc_search_status = format!(
                    "Local CKC {} search returned {} result(s)",
                    mode.label(),
                    state.ckc_search_results.len()
                );
            }
        }
        let status =
            ui.label(egui::RichText::new(&state.ckc_search_status).color(palette.text_subtle));
        emit_node(
            ui.ctx(),
            status.id,
            accesskit::Role::Label,
            ATELIER_CKC_SEARCH_STATUS_AUTHOR_ID,
            &state.ckc_search_status,
            state.ckc_search_pending,
        );

        let results_response = ui
            .vertical(|ui| {
                if state.ckc_search_results.is_empty() {
                    ui.label(
                        egui::RichText::new("No CKC search results").color(palette.text_subtle),
                    );
                }
                for result in &state.ckc_search_results {
                    let row = ui.label(result.summary_label());
                    emit_node(
                        ui.ctx(),
                        row.id,
                        accesskit::Role::ListItem,
                        &ckc_search_result_row_author_id(&result.target_ref),
                        &result.summary_label(),
                        false,
                    );
                    if !result.snippet.is_empty() {
                        ui.label(
                            egui::RichText::new(result.snippet.clone()).color(palette.text_subtle),
                        );
                    }
                    let mut refs = Vec::new();
                    for value in [
                        result.character_ref.as_deref(),
                        result.sheet_version_ref.as_deref(),
                        result.collection_ref.as_deref(),
                        result.media_ref.as_deref(),
                        result.tag_ref.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    {
                        refs.push(value.to_owned());
                    }
                    if !refs.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("refs: {}", refs.join(" | ")))
                                .color(palette.text_subtle),
                        );
                    }
                    if !result.tags.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("tags: {}", result.tags.join(", ")))
                                .color(palette.text_subtle),
                        );
                    }
                    for note in &result.tag_notes {
                        let scope = note.scope_ref.as_deref().unwrap_or("global");
                        ui.label(
                            egui::RichText::new(format!(
                                "tag note {} [{}]: {}",
                                note.tag_text, scope, note.note
                            ))
                            .color(palette.text_subtle),
                        );
                    }
                }
            })
            .response;
        emit_node(
            ui.ctx(),
            results_response.id,
            accesskit::Role::List,
            ATELIER_CKC_SEARCH_RESULTS_AUTHOR_ID,
            "CKC search results",
            false,
        );

        ui.add_space(4.0);
        ui.label(egui::RichText::new("Tag note").color(palette.text));
        ui.horizontal(|ui| {
            let tag = ui.text_edit_singleline(&mut state.ckc_tag_note_tag);
            emit_node(
                ui.ctx(),
                tag.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_TAG_NOTE_TAG_AUTHOR_ID,
                "CKC tag note tag",
                false,
            );
            let scope = ui.text_edit_singleline(&mut state.ckc_tag_note_scope_ref);
            emit_node(
                ui.ctx(),
                scope.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_TAG_NOTE_SCOPE_AUTHOR_ID,
                "CKC tag note scope ref",
                false,
            );
        });
        let note = ui.add(
            egui::TextEdit::multiline(&mut state.ckc_tag_note_editor)
                .desired_rows(2)
                .lock_focus(true),
        );
        emit_node(
            ui.ctx(),
            note.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_TAG_NOTE_EDITOR_AUTHOR_ID,
            "CKC rich tag note editor",
            false,
        );
        let save = ui.add_enabled(
            !state.ckc_tag_note_pending,
            egui::Button::new("Save tag note"),
        );
        emit_node(
            ui.ctx(),
            save.id,
            accesskit::Role::Button,
            ATELIER_CKC_TAG_NOTE_SAVE_AUTHOR_ID,
            "Save CKC rich tag note",
            state.ckc_tag_note_pending,
        );
        if save.clicked() {
            let request = CkcTagNoteSaveRequest {
                tag_text: state.ckc_tag_note_tag.trim().to_ascii_lowercase(),
                scope_ref: if state.ckc_tag_note_scope_ref.trim().is_empty() {
                    None
                } else {
                    Some(state.ckc_tag_note_scope_ref.trim().to_owned())
                },
                note: state.ckc_tag_note_editor.clone(),
            };
            if !request.tag_text.is_empty() {
                if let Some(client) = self.ckc_client.as_ref() {
                    state.ckc_tag_note_pending = true;
                    client.save_ckc_tag_note(
                        &request.tag_text,
                        request.scope_ref.as_deref(),
                        &request.note,
                        client.actor_id(),
                        self.ckc_tag_note_cell.clone(),
                    );
                } else {
                    let note = CkcTagNoteRecord {
                        tag_ref: format!("atelier://tag/local-{}", request.tag_text),
                        tag_text: request.tag_text.clone(),
                        scope_ref: request.scope_ref,
                        note: request.note,
                    };
                    attach_tag_note_to_visible_results(&mut state.ckc_search_results, note);
                    state.ckc_search_status =
                        format!("Saved local CKC tag note for {}", request.tag_text);
                }
            }
        }
    }

    fn show_ckc_sheet_artifact_panel(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        state: &mut AtelierPanelState,
        selected_index: usize,
    ) {
        if let Some(selected_link_id) = state.ckc_selected_sheet_artifact_link_id.as_deref() {
            let selected_link_still_belongs_to_character = state
                .ckc_characters
                .get(selected_index)
                .is_some_and(|character| {
                    character
                        .sheet_artifact_links
                        .iter()
                        .any(|link| link.link_id == selected_link_id)
                });
            if !selected_link_still_belongs_to_character {
                state.ckc_selected_sheet_artifact_link_id = None;
                state.ckc_sheet_artifact_reuse_ref.clear();
            }
        }
        let pending = state.ckc_sheet_artifact_pending;
        let status = state.ckc_sheet_artifact_status.clone();
        let selected_link_id = state.ckc_selected_sheet_artifact_link_id.clone();
        let reuse_ref = state.ckc_sheet_artifact_reuse_ref.clone();
        let latest_pose_export = state.pose_last_export.clone();
        let backend_available = self.ckc_client.is_some();
        let mut artifact_kind = std::mem::take(&mut state.ckc_sheet_artifact_kind);
        let mut artifact_ref = std::mem::take(&mut state.ckc_sheet_artifact_ref);
        let mut manifest_ref = std::mem::take(&mut state.ckc_sheet_artifact_manifest_ref);
        let mut label = std::mem::take(&mut state.ckc_sheet_artifact_label);
        let mut reuse_role = std::mem::take(&mut state.ckc_sheet_artifact_reuse_role);
        if state.ckc_sheet_artifact_actor_id.trim().is_empty() {
            state.ckc_sheet_artifact_actor_id = self
                .ckc_client
                .as_ref()
                .map(|client| client.actor_id().to_owned())
                .unwrap_or_else(|| "local-atelier-panel".to_owned());
        }
        let mut actor_id = std::mem::take(&mut state.ckc_sheet_artifact_actor_id);
        let mut pending_attach = None;
        let mut pending_detach = None;
        let mut next_selection = None;
        let mut next_reuse_ref = None;
        let mut next_status = None;

        if let Some(character) = state.ckc_characters.get_mut(selected_index) {
            let (attach, detach, selection, reuse, status) = self.show_ckc_sheet_artifacts(
                ui,
                palette,
                character,
                pending,
                &status,
                &mut artifact_kind,
                &mut artifact_ref,
                &mut manifest_ref,
                &mut label,
                &mut reuse_role,
                &mut actor_id,
                selected_link_id.as_deref(),
                &reuse_ref,
                latest_pose_export.as_ref(),
                backend_available,
            );
            pending_attach = attach;
            pending_detach = detach;
            next_selection = selection;
            next_reuse_ref = reuse;
            next_status = status;
        }

        state.ckc_sheet_artifact_kind = artifact_kind;
        state.ckc_sheet_artifact_ref = artifact_ref;
        state.ckc_sheet_artifact_manifest_ref = manifest_ref;
        state.ckc_sheet_artifact_label = label;
        state.ckc_sheet_artifact_reuse_role = reuse_role;
        state.ckc_sheet_artifact_actor_id = actor_id;

        if let Some(selection) = next_selection {
            state.ckc_selected_sheet_artifact_link_id = Some(selection);
        }
        if let Some(reuse_ref) = next_reuse_ref {
            state.ckc_sheet_artifact_reuse_ref = reuse_ref;
        }
        if let Some(status) = next_status {
            state.ckc_sheet_artifact_status = status;
            state.ckc_error = None;
        }

        if let Some(request) = pending_attach {
            if let Some(client) = self.ckc_client.as_ref() {
                let CkcSheetArtifactAttachRequest {
                    sheet_version_id,
                    artifact_kind,
                    artifact_ref,
                    manifest_ref,
                    source_ref,
                    label,
                    reuse_role,
                    metadata,
                    actor_id,
                } = request;
                state.ckc_sheet_artifact_pending = true;
                state.ckc_sheet_artifact_status = format!(
                    "Attaching {artifact_kind} reusable artifact to sheet {sheet_version_id}"
                );
                state.ckc_error = None;
                client.attach_ckc_sheet_artifact_link(
                    &sheet_version_id,
                    &artifact_kind,
                    &artifact_ref,
                    manifest_ref.as_deref(),
                    source_ref.as_deref(),
                    label.as_deref(),
                    reuse_role.as_deref(),
                    metadata,
                    &actor_id,
                    self.ckc_sheet_artifact_links_cell.clone(),
                );
            }
        }

        if let Some(request) = pending_detach {
            if let Some(client) = self.ckc_client.as_ref() {
                state.ckc_sheet_artifact_pending = true;
                state.ckc_sheet_artifact_status = format!(
                    "Detaching reusable artifact {} from sheet {}",
                    request.link_id, request.sheet_version_id
                );
                state.ckc_error = None;
                client.detach_ckc_sheet_artifact_link(
                    &request.sheet_version_id,
                    &request.link_id,
                    &request.actor_id,
                    self.ckc_sheet_artifact_links_cell.clone(),
                );
            }
        }
    }

    fn show_ckc_sheet_artifacts(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        character: &mut CkcCharacterRecord,
        pending: bool,
        status: &str,
        artifact_kind: &mut String,
        artifact_ref: &mut String,
        manifest_ref: &mut String,
        label: &mut String,
        reuse_role: &mut String,
        actor_id: &mut String,
        selected_link_id: Option<&str>,
        reuse_ref: &str,
        latest_pose_export: Option<&PosekitExportSnapshot>,
        backend_available: bool,
    ) -> (
        Option<CkcSheetArtifactAttachRequest>,
        Option<CkcSheetArtifactDetachRequest>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) {
        ui.heading(egui::RichText::new("Reusable sheet artifacts").color(palette.text));
        let status_response = ui.label(egui::RichText::new(status).color(palette.text_subtle));
        emit_node(
            ui.ctx(),
            status_response.id,
            accesskit::Role::Label,
            ATELIER_CKC_SHEET_ARTIFACT_STATUS_AUTHOR_ID,
            status,
            pending,
        );

        let mut pending_attach = None;
        let mut pending_detach = None;
        let selected_link_id = selected_link_id
            .filter(|link_id| {
                character
                    .sheet_artifact_links
                    .iter()
                    .any(|link| &link.link_id == link_id)
            })
            .map(ToOwned::to_owned)
            .or_else(|| {
                character
                    .sheet_artifact_links
                    .first()
                    .map(|link| link.link_id.clone())
            });
        let mut pending_selection = selected_link_id.clone();
        let mut pending_reuse_ref = None;
        let mut pending_status = None;
        let selected_link_id = selected_link_id.as_deref();

        let list_response = ui
            .vertical(|ui| {
                if character.sheet_artifact_links.is_empty() {
                    ui.label(
                        egui::RichText::new("No reusable sheet artifacts linked yet.")
                            .color(palette.text_subtle),
                    );
                }
                for link in &character.sheet_artifact_links {
                    let selected = selected_link_id == Some(link.link_id.as_str());
                    let row_author_id = ckc_sheet_artifact_row_author_id(&link.link_id);
                    let row = ui.selectable_label(selected, link.summary());
                    emit_node(
                        ui.ctx(),
                        row.id,
                        accesskit::Role::ListItem,
                        &row_author_id,
                        &format!(
                            "{} {} {} {}",
                            link.sheet_version_ref, link.typed_ref, link.artifact_kind, link.artifact_ref
                        ),
                        selected,
                    );
                    if row.clicked() {
                        pending_selection = Some(link.link_id.clone());
                        pending_reuse_ref = Some(link.typed_ref.clone());
                    }
                    if selected {
                        ui.label(
                            egui::RichText::new(format!(
                                "sheet={} sheet_version_id={} character={} character_internal_id={} manifest={} source={} label={} linked_by={} metadata={}",
                                link.sheet_version_ref,
                                link.sheet_version_id,
                                link.character_ref,
                                link.character_internal_id,
                                link.manifest_ref.as_deref().unwrap_or("<none>"),
                                link.source_ref.as_deref().unwrap_or("<none>"),
                                link.label.as_deref().unwrap_or("<none>"),
                                link.linked_by,
                                link.metadata
                            ))
                            .color(palette.text_subtle),
                        );
                    }
                }
            })
            .response;
        emit_node(
            ui.ctx(),
            list_response.id,
            accesskit::Role::List,
            ATELIER_CKC_SHEET_ARTIFACT_LIST_AUTHOR_ID,
            "CKC reusable sheet artifact links",
            pending,
        );

        let selected_reuse_ref = pending_reuse_ref
            .as_deref()
            .or_else(|| {
                selected_link_id.and_then(|link_id| {
                    character
                        .sheet_artifact_links
                        .iter()
                        .find(|link| link.link_id == link_id)
                        .map(|link| link.typed_ref.as_str())
                })
            })
            .unwrap_or(reuse_ref);
        let reuse_response = ui.label(format!("reuse typed_ref: {selected_reuse_ref}"));
        emit_node(
            ui.ctx(),
            reuse_response.id,
            accesskit::Role::Label,
            ATELIER_CKC_SHEET_ARTIFACT_REUSE_REF_AUTHOR_ID,
            selected_reuse_ref,
            false,
        );

        ui.horizontal_wrapped(|ui| {
            let kind = ui.text_edit_singleline(artifact_kind);
            emit_node(
                ui.ctx(),
                kind.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_SHEET_ARTIFACT_KIND_AUTHOR_ID,
                "Sheet artifact kind",
                false,
            );
            let artifact = ui.text_edit_singleline(artifact_ref);
            emit_node(
                ui.ctx(),
                artifact.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_SHEET_ARTIFACT_REF_AUTHOR_ID,
                "Reusable artifact_ref to attach to the current CKC sheet version",
                false,
            );
        });
        ui.horizontal_wrapped(|ui| {
            let manifest = ui.text_edit_singleline(manifest_ref);
            emit_node(
                ui.ctx(),
                manifest.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_SHEET_ARTIFACT_MANIFEST_AUTHOR_ID,
                "Optional manifest_ref or Comfy receipt_ref",
                false,
            );
            let label_response = ui.text_edit_singleline(label);
            emit_node(
                ui.ctx(),
                label_response.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_SHEET_ARTIFACT_LABEL_AUTHOR_ID,
                "Human label for this reusable sheet artifact",
                false,
            );
            let role = ui.text_edit_singleline(reuse_role);
            emit_node(
                ui.ctx(),
                role.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_SHEET_ARTIFACT_ROLE_AUTHOR_ID,
                "Reuse role for downstream tools",
                false,
            );
            let actor = ui.text_edit_singleline(actor_id);
            emit_node(
                ui.ctx(),
                actor.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_SHEET_ARTIFACT_ACTOR_AUTHOR_ID,
                "Parallel agent actor_id for sheet artifact attach/detach writes",
                false,
            );
        });

        ui.horizontal_wrapped(|ui| {
            let attach = ui.add_enabled(!pending, egui::Button::new("Attach artifact"));
            emit_node(
                ui.ctx(),
                attach.id,
                accesskit::Role::Button,
                ATELIER_CKC_SHEET_ARTIFACT_ATTACH_AUTHOR_ID,
                "Attach reusable artifact ref to current CKC sheet version",
                pending,
            );
            if attach.clicked() {
                if let Some(sheet_version_id) = character.sheet_version_id.clone() {
                    let request = CkcSheetArtifactAttachRequest {
                        sheet_version_id,
                        artifact_kind: artifact_kind.trim().to_owned(),
                        artifact_ref: artifact_ref.trim().to_owned(),
                        manifest_ref: non_empty_trimmed(manifest_ref),
                        source_ref: None,
                        label: non_empty_trimmed(label),
                        reuse_role: non_empty_trimmed(reuse_role),
                        metadata: serde_json::json!({
                            "attached_from": "atelier_ckc_manual",
                        }),
                        actor_id: actor_id_or_default(actor_id),
                    };
                    if !request.artifact_kind.is_empty() && !request.artifact_ref.is_empty() {
                        if backend_available {
                            pending_attach = Some(request);
                        } else if let Some(link) = CkcSheetArtifactLinkRecord::local(
                            character,
                            request.artifact_kind,
                            request.artifact_ref,
                            request.manifest_ref,
                            request.source_ref,
                            request.label,
                            request.reuse_role,
                            request.metadata,
                            request.actor_id,
                        ) {
                            pending_selection = Some(link.link_id.clone());
                            pending_reuse_ref = Some(link.typed_ref.clone());
                            character.sheet_artifact_links.push(link);
                            pending_status =
                                Some("Attached local reusable sheet artifact".to_owned());
                        }
                    }
                }
            }

            let attach_pose = ui.add_enabled(
                !pending && latest_pose_export.is_some(),
                egui::Button::new("Attach Posekit export"),
            );
            emit_node(
                ui.ctx(),
                attach_pose.id,
                accesskit::Role::Button,
                ATELIER_CKC_SHEET_ARTIFACT_ATTACH_POSE_AUTHOR_ID,
                "Attach latest Posekit OpenPose PNG export to current CKC sheet version",
                pending || latest_pose_export.is_none(),
            );
            if attach_pose.clicked() {
                if let (Some(sheet_version_id), Some(snapshot)) =
                    (character.sheet_version_id.clone(), latest_pose_export)
                {
                    let source_ref = snapshot
                        .rig_id
                        .as_ref()
                        .map(|rig_id| format!("posekit://rig/{rig_id}"))
                        .unwrap_or_else(|| snapshot.source_ref.clone());
                    let request = CkcSheetArtifactAttachRequest {
                        sheet_version_id,
                        artifact_kind: "openpose_png".to_owned(),
                        artifact_ref: snapshot.png_artifact_ref.clone(),
                        manifest_ref: Some(snapshot.png_manifest_ref.clone()),
                        source_ref: Some(source_ref),
                        label: Some(format!("Posekit yaw {:.0} OpenPose", snapshot.yaw_deg)),
                        reuse_role: Some("cui_openpose_conditioning".to_owned()),
                        metadata: serde_json::json!({
                            "schema": "hsk.atelier.posekit.openpose_export@1",
                            "yaw_deg": snapshot.yaw_deg,
                            "pitch_deg": snapshot.pitch_deg,
                            "zoom": snapshot.zoom,
                            "receipt_ref": snapshot.receipt_ref,
                            "json_artifact_ref": snapshot.json_artifact_ref,
                            "json_manifest_ref": snapshot.json_manifest_ref,
                            "content_hash": snapshot.content_hash,
                        }),
                        actor_id: actor_id_or_default(actor_id),
                    };
                    if backend_available {
                        pending_attach = Some(request);
                    } else if let Some(link) = CkcSheetArtifactLinkRecord::local(
                        character,
                        request.artifact_kind,
                        request.artifact_ref,
                        request.manifest_ref,
                        request.source_ref,
                        request.label,
                        request.reuse_role,
                        request.metadata,
                        request.actor_id,
                    ) {
                        pending_selection = Some(link.link_id.clone());
                        pending_reuse_ref = Some(link.typed_ref.clone());
                        character.sheet_artifact_links.push(link);
                        pending_status = Some("Attached local Posekit OpenPose export".to_owned());
                    }
                }
            }

            let detach = ui.add_enabled(
                selected_link_id.is_some() && !pending,
                egui::Button::new("Detach"),
            );
            emit_node(
                ui.ctx(),
                detach.id,
                accesskit::Role::Button,
                ATELIER_CKC_SHEET_ARTIFACT_DETACH_AUTHOR_ID,
                "Soft-detach selected reusable sheet artifact",
                pending || selected_link_id.is_none(),
            );
            if detach.clicked() {
                if let (Some(sheet_version_id), Some(link_id)) =
                    (character.sheet_version_id.clone(), selected_link_id)
                {
                    if backend_available {
                        pending_detach = Some(CkcSheetArtifactDetachRequest {
                            sheet_version_id,
                            link_id: link_id.to_owned(),
                            actor_id: actor_id_or_default(actor_id),
                        });
                    } else {
                        character
                            .sheet_artifact_links
                            .retain(|link| link.link_id != link_id);
                        pending_selection = character
                            .sheet_artifact_links
                            .first()
                            .map(|link| link.link_id.clone());
                        pending_reuse_ref = character
                            .sheet_artifact_links
                            .first()
                            .map(|link| link.typed_ref.clone());
                        pending_status = Some("Detached local reusable sheet artifact".to_owned());
                    }
                }
            }
        });

        (
            pending_attach,
            pending_detach,
            pending_selection,
            pending_reuse_ref,
            pending_status,
        )
    }

    fn show_ckc_linked_media(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        character: &mut CkcCharacterRecord,
        media_save_pending: bool,
        selected_media_key: Option<&str>,
        selected_album_collection_id: Option<&str>,
        album_create_pending: bool,
        album_link_pending: bool,
        album_page_pending: bool,
        album_status: &str,
        album_create_name: &mut String,
        album_create_notes: &mut String,
        album_create_tags: &mut String,
        album_link_asset_ids: &mut String,
        album_link_source_path_ref: &mut String,
        album_link_source_url_ref: &mut String,
    ) -> (
        Option<CkcMediaSaveRequest>,
        Option<String>,
        Option<String>,
        Option<CkcAlbumCreateRequest>,
        Option<CkcAlbumLinkAssetsRequest>,
        Option<CkcAlbumPageRequest>,
    ) {
        ui.heading(egui::RichText::new("Linked media").color(palette.text));
        let mut pending_album_create = None;
        let mut pending_album_link = None;
        let mut pending_album_page = None;
        let mut pending_album_selection = None;
        let album_status_response = ui.label(album_status);
        emit_node(
            ui.ctx(),
            album_status_response.id,
            accesskit::Role::Label,
            ATELIER_CKC_ALBUM_STATUS_AUTHOR_ID,
            album_status,
            album_create_pending || album_link_pending,
        );
        ui.horizontal_wrapped(|ui| {
            let name = ui.text_edit_singleline(album_create_name);
            emit_node(
                ui.ctx(),
                name.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_ALBUM_CREATE_NAME_AUTHOR_ID,
                "CKC album name",
                false,
            );
            let tags = ui.text_edit_singleline(album_create_tags);
            emit_node(
                ui.ctx(),
                tags.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_ALBUM_CREATE_TAGS_AUTHOR_ID,
                "CKC album tags",
                false,
            );
            let create = ui.add_enabled(!album_create_pending, egui::Button::new("Create album"));
            emit_node(
                ui.ctx(),
                create.id,
                accesskit::Role::Button,
                ATELIER_CKC_ALBUM_CREATE_AUTHOR_ID,
                "Create CKC media album for selected character",
                album_create_pending,
            );
            if create.clicked() {
                let name = album_create_name.trim().to_owned();
                if !name.is_empty() {
                    let notes = non_empty_trimmed(album_create_notes);
                    let tags = ckc_tags_from_buffer(album_create_tags);
                    if self.ckc_client.is_some() {
                        pending_album_create = Some(CkcAlbumCreateRequest {
                            character_internal_id: character.character_internal_id.clone(),
                            name,
                            notes,
                            sheet_version_id: character.sheet_version_id.clone(),
                            tags,
                        });
                    } else {
                        let collection_id = Uuid::new_v4().to_string();
                        let collection_ref = format!("atelier://collection/{collection_id}");
                        character.media_albums.push(CkcMediaAlbumRecord {
                            collection_id: collection_id.clone(),
                            collection_ref,
                            name,
                            description: notes.unwrap_or_default(),
                            tags,
                            member_count: 0,
                            members_next_offset: None,
                            members: Vec::new(),
                        });
                        pending_album_selection = Some(collection_id);
                    }
                }
            }
        });
        let notes = ui.add(
            egui::TextEdit::multiline(album_create_notes)
                .desired_rows(2)
                .lock_focus(true),
        );
        emit_node(
            ui.ctx(),
            notes.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_ALBUM_CREATE_NOTES_AUTHOR_ID,
            "CKC album notes",
            false,
        );

        let resolved_selection = character
            .selected_or_first_media_location(selected_media_key)
            .map(|(album_idx, member_idx)| {
                let album = &character.media_albums[album_idx];
                let member = &album.members[member_idx];
                ckc_media_occurrence_key(&album.collection_id, &member.asset_id)
            });
        let viewer_response = ui
            .vertical(|ui| {
                ui.label(egui::RichText::new("Selected image preview").color(palette.text));
                if let Some((album_idx, member_idx)) =
                    character.selected_or_first_media_location(resolved_selection.as_deref())
                {
                    let album = &character.media_albums[album_idx];
                    let member = &album.members[member_idx];
                    let preview_label = format!(
                        "{}\n{}\n{}",
                        member.display_label,
                        member
                            .source_path_ref
                            .as_deref()
                            .unwrap_or("no source_path_ref"),
                        member
                            .source_url_ref
                            .as_deref()
                            .unwrap_or("no source_url_ref")
                    );
                    let desired_size = egui::vec2(ui.available_width().max(160.0), 116.0);
                    let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
                    ui.painter()
                        .rect_filled(rect, 4.0, palette.surface.linear_multiply(1.12));
                    ui.painter().rect_stroke(
                        rect,
                        4.0,
                        egui::Stroke::new(1.0, palette.border),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        preview_label,
                        egui::FontId::proportional(13.0),
                        palette.text,
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "album: {} | status: {}",
                            album.name,
                            member.review_status.as_deref().unwrap_or("unreviewed")
                        ))
                        .color(palette.text_subtle),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("No linked image selected for this character.")
                            .color(palette.text_subtle),
                    );
                }
            })
            .response;
        emit_node(
            ui.ctx(),
            viewer_response.id,
            accesskit::Role::Group,
            ATELIER_CKC_MEDIA_VIEWER_AUTHOR_ID,
            "CKC selected image preview and source refs",
            false,
        );
        let mut pending_selection = None;
        let list_response = ui
            .vertical(|ui| {
                if character.media_albums.is_empty() {
                    ui.label(egui::RichText::new("No linked albums").color(palette.text_subtle));
                }
                for album in &character.media_albums {
                    let album_ref = AtelierRef::media_album(&album.collection_ref, &album.name);
                    debug_assert_eq!(album_ref.item_kind, AtelierItemKind::MediaAlbum);
                    let album_selected =
                        selected_album_collection_id == Some(album.collection_id.as_str());
                    let album_author_id = ckc_media_album_row_author_id(&album.collection_id);
                    let album_payload = DragPayload::AtelierRef(album_ref.clone());
                    let album_drag = ui
                        .dnd_drag_source(egui::Id::new(&album_author_id), album_payload, |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} Album: {} ({} items)",
                                    if album_selected { "*" } else { "" },
                                    album.name,
                                    album.member_count
                                ))
                                .color(palette.text),
                            );
                        })
                        .response;
                    let album_row = ui.interact(
                        album_drag.rect,
                        egui::Id::new(&album_author_id),
                        egui::Sense::click_and_drag(),
                    );
                    emit_draggable_list_item_node(
                        ui.ctx(),
                        album_row.id,
                        &album_author_id,
                        &format!("{} {}", album_ref.ref_kind(), album.collection_ref),
                        &format!(
                            "draggable; atelier-ref {}:{}",
                            album_ref.ref_kind(),
                            album_ref.item_id
                        ),
                        album_selected,
                    );
                    if album_row.clicked() {
                        pending_album_selection = Some(album.collection_id.clone());
                    }
                    if !album.description.is_empty() {
                        ui.label(
                            egui::RichText::new(&album.description).color(palette.text_subtle),
                        );
                    }
                    if !album.tags.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("album tags: {}", album.tags.join(", ")))
                                .color(palette.text_subtle),
                        );
                    }
                    if let Some(next_offset) = album.members_next_offset {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "showing {} of {}; next offset {}",
                                    album.members.len(),
                                    album.member_count,
                                    next_offset
                                ))
                                .color(palette.text_subtle),
                            );
                            let load_more_author_id =
                                ckc_media_album_load_more_author_id(&album.collection_id);
                            let load_more =
                                ui.add_enabled(!album_page_pending, egui::Button::new("Load more"));
                            emit_node(
                                ui.ctx(),
                                load_more.id,
                                accesskit::Role::Button,
                                &load_more_author_id,
                                "Load the next CKC album media page",
                                album_page_pending,
                            );
                            if load_more.clicked() {
                                pending_album_page = Some(CkcAlbumPageRequest {
                                    collection_id: album.collection_id.clone(),
                                    offset: next_offset,
                                });
                            }
                        });
                    }
                    for member in &album.members {
                        let media_key =
                            ckc_media_occurrence_key(&album.collection_id, &member.asset_id);
                        let selected = resolved_selection.as_deref() == Some(media_key.as_str());
                        let media_author_id =
                            ckc_media_row_author_id(&album.collection_id, &member.asset_id);
                        let media_ref = AtelierRef::new(
                            member.media_ref.clone(),
                            AtelierItemKind::Media,
                            member.display_label.clone(),
                        );
                        let media_payload = DragPayload::AtelierRef(media_ref.clone());
                        let media_drag = ui
                            .dnd_drag_source(egui::Id::new(&media_author_id), media_payload, |ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}{} [{}]",
                                        if selected { "*" } else { "" },
                                        member.display_label,
                                        member.review_status.as_deref().unwrap_or("unreviewed")
                                    ))
                                    .color(palette.text),
                                );
                            })
                            .response;
                        let media_row = ui.interact(
                            media_drag.rect,
                            egui::Id::new(&media_author_id),
                            egui::Sense::click_and_drag(),
                        );
                        emit_draggable_list_item_node(
                            ui.ctx(),
                            media_row.id,
                            &media_author_id,
                            &member.media_ref,
                            &format!(
                                "draggable; atelier-ref {}:{}",
                                media_ref.ref_kind(),
                                media_ref.item_id
                            ),
                            selected,
                        );
                        if media_row.clicked() {
                            pending_selection = Some(media_key);
                        }
                        if let Some(folder_ref) = &member.source_path_ref {
                            let folder_author_id = ckc_folder_row_author_id(
                                &album.collection_id,
                                &member.asset_id,
                                folder_ref,
                            );
                            let folder_ref_value = AtelierRef::new(
                                folder_ref.clone(),
                                AtelierItemKind::Folder,
                                folder_ref.clone(),
                            );
                            let folder_payload = DragPayload::AtelierRef(folder_ref_value.clone());
                            let folder_drag = ui
                                .dnd_drag_source(
                                    egui::Id::new(&folder_author_id),
                                    folder_payload,
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "folder_ref: {folder_ref}"
                                            ))
                                            .color(palette.text_subtle),
                                        );
                                    },
                                )
                                .response;
                            emit_draggable_list_item_node(
                                ui.ctx(),
                                folder_drag.id,
                                &folder_author_id,
                                folder_ref,
                                &format!(
                                    "draggable; atelier-ref {}:{}",
                                    folder_ref_value.ref_kind(),
                                    folder_ref_value.item_id
                                ),
                                false,
                            );
                        }
                        if let Some(source_url_ref) = &member.source_url_ref {
                            let source_url_author_id = ckc_source_url_row_author_id(
                                &album.collection_id,
                                &member.asset_id,
                                source_url_ref,
                            );
                            let source_url_ref_value = AtelierRef::new(
                                source_url_ref.clone(),
                                AtelierItemKind::SourceUrl,
                                source_url_ref.clone(),
                            );
                            let source_url_payload =
                                DragPayload::AtelierRef(source_url_ref_value.clone());
                            let source_url_drag = ui
                                .dnd_drag_source(
                                    egui::Id::new(&source_url_author_id),
                                    source_url_payload,
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "source_url_ref: {source_url_ref}"
                                            ))
                                            .color(palette.text_subtle),
                                        );
                                    },
                                )
                                .response;
                            emit_draggable_list_item_node(
                                ui.ctx(),
                                source_url_drag.id,
                                &source_url_author_id,
                                source_url_ref,
                                &format!(
                                    "draggable; atelier-ref {}:{}",
                                    source_url_ref_value.ref_kind(),
                                    source_url_ref_value.item_id
                                ),
                                false,
                            );
                        }
                    }
                }
            })
            .response;
        emit_node(
            ui.ctx(),
            list_response.id,
            accesskit::Role::List,
            ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID,
            "CKC linked images folders and albums",
            false,
        );

        ui.horizontal_wrapped(|ui| {
            let asset_ids = ui.text_edit_singleline(album_link_asset_ids);
            emit_node(
                ui.ctx(),
                asset_ids.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_ALBUM_LINK_ASSET_IDS_AUTHOR_ID,
                "Existing media asset IDs to link into the selected CKC album",
                false,
            );
            let source_path = ui.text_edit_singleline(album_link_source_path_ref);
            emit_node(
                ui.ctx(),
                source_path.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_ALBUM_LINK_SOURCE_PATH_AUTHOR_ID,
                "Optional link-scoped CKC source path ref",
                false,
            );
            let source_url = ui.text_edit_singleline(album_link_source_url_ref);
            emit_node(
                ui.ctx(),
                source_url.id,
                accesskit::Role::TextInput,
                ATELIER_CKC_ALBUM_LINK_SOURCE_URL_AUTHOR_ID,
                "Optional link-scoped CKC source URL ref",
                false,
            );
            let link = ui.add_enabled(!album_link_pending, egui::Button::new("Link media IDs"));
            emit_node(
                ui.ctx(),
                link.id,
                accesskit::Role::Button,
                ATELIER_CKC_ALBUM_LINK_AUTHOR_ID,
                "Link existing media asset IDs into selected CKC album",
                album_link_pending,
            );
            if link.clicked() {
                let asset_ids = ckc_asset_ids_from_buffer(album_link_asset_ids);
                let source_path_ref = non_empty_trimmed(album_link_source_path_ref);
                let source_url_ref = non_empty_trimmed(album_link_source_url_ref);
                let selected_collection_id = selected_album_collection_id
                    .filter(|collection_id| {
                        character
                            .media_albums
                            .iter()
                            .any(|album| album.collection_id == *collection_id)
                    })
                    .map(ToOwned::to_owned)
                    .or_else(|| pending_album_selection.clone())
                    .or_else(|| {
                        character
                            .media_albums
                            .first()
                            .map(|album| album.collection_id.clone())
                    });
                if let (Some(collection_id), false) = (selected_collection_id, asset_ids.is_empty())
                {
                    if self.ckc_client.is_some() {
                        pending_album_link = Some(CkcAlbumLinkAssetsRequest {
                            collection_id,
                            asset_ids,
                            source_path_ref,
                            source_url_ref,
                        });
                    } else if let Some(album) = character
                        .media_albums
                        .iter_mut()
                        .find(|album| album.collection_id == collection_id)
                    {
                        for asset_id in asset_ids {
                            if album
                                .members
                                .iter()
                                .any(|member| member.asset_id == asset_id)
                            {
                                continue;
                            }
                            album.members.push(local_ckc_media_member(
                                &asset_id,
                                source_path_ref.clone(),
                                source_url_ref.clone(),
                            ));
                        }
                        album.member_count = album.members.len();
                        pending_album_selection = Some(album.collection_id.clone());
                    }
                }
            }
        });

        let selected_media_key = pending_selection
            .as_deref()
            .or(resolved_selection.as_deref());
        let Some((album_idx, member_idx)) =
            character.selected_or_first_media_location(selected_media_key)
        else {
            return (
                None,
                pending_selection,
                pending_album_selection,
                pending_album_create,
                pending_album_link,
                pending_album_page,
            );
        };
        let member = &mut character.media_albums[album_idx].members[member_idx];
        ui.add_space(4.0);
        let notes = ui.add(
            egui::TextEdit::multiline(&mut member.notes)
                .desired_rows(3)
                .lock_focus(true),
        );
        emit_node(
            ui.ctx(),
            notes.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
            "CKC image notes editor",
            false,
        );
        let tags = ui.text_edit_singleline(&mut member.tags_buffer);
        emit_node(
            ui.ctx(),
            tags.id,
            accesskit::Role::TextInput,
            ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
            "CKC image tags editor",
            false,
        );
        let save = ui.add_enabled(!media_save_pending, egui::Button::new("Save media notes"));
        emit_node(
            ui.ctx(),
            save.id,
            accesskit::Role::Button,
            ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID,
            "Save CKC image notes and tags",
            media_save_pending,
        );
        if save.clicked() {
            (
                Some(CkcMediaSaveRequest {
                    asset_id: member.asset_id.clone(),
                    notes: member.notes.clone(),
                    tags: ckc_tags_from_buffer(&member.tags_buffer),
                    review_status: member.review_status.clone(),
                }),
                pending_selection,
                pending_album_selection,
                pending_album_create,
                pending_album_link,
                pending_album_page,
            )
        } else {
            (
                None,
                pending_selection,
                pending_album_selection,
                pending_album_create,
                pending_album_link,
                pending_album_page,
            )
        }
    }

    fn show_posekit(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        self.drain_posekit_export_backend();
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Source image").color(palette.text));
            let source = ui.text_edit_singleline(&mut state.pose_source_ref);
            emit_value_node(
                ui.ctx(),
                source.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_SOURCE_REF_AUTHOR_ID,
                "Posekit source image ref",
                &state.pose_source_ref,
            );
            ui.label(egui::RichText::new("Rig id").color(palette.text));
            let rig = ui.text_edit_singleline(&mut state.pose_rig_id);
            emit_value_node(
                ui.ctx(),
                rig.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_RIG_ID_AUTHOR_ID,
                "Posekit stored rig id",
                &posekit_optional_rig_id(&state.pose_rig_id).unwrap_or_else(|| "<none>".to_owned()),
            );
        });
        let readout = posekit_state_readout(&state);
        let readout_response = ui.label(egui::RichText::new(&readout).color(palette.text_subtle));
        emit_value_node(
            ui.ctx(),
            readout_response.id,
            accesskit::Role::Label,
            ATELIER_POSE_STATE_READOUT_AUTHOR_ID,
            "Posekit current pose state",
            &readout,
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let yaw_minus = ui.button("Yaw -15");
            emit_node(
                ui.ctx(),
                yaw_minus.id,
                accesskit::Role::Button,
                ATELIER_POSE_YAW_MINUS_AUTHOR_ID,
                "Yaw -15",
                false,
            );
            if yaw_minus.clicked() {
                state.pose_yaw = (state.pose_yaw - 15.0).max(-180.0);
            }
            let yaw_plus = ui.button("Yaw +15");
            emit_node(
                ui.ctx(),
                yaw_plus.id,
                accesskit::Role::Button,
                ATELIER_POSE_YAW_PLUS_AUTHOR_ID,
                "Yaw +15",
                false,
            );
            if yaw_plus.clicked() {
                state.pose_yaw = (state.pose_yaw + 15.0).min(180.0);
            }
            let reset = ui.button("Reset");
            emit_node(
                ui.ctx(),
                reset.id,
                accesskit::Role::Button,
                ATELIER_POSE_RESET_AUTHOR_ID,
                "Reset pose",
                false,
            );
            if reset.clicked() {
                state.pose_yaw = 0.0;
                state.pose_pitch = 0.0;
                state.pose_zoom = 1.0;
                state.pose_face = true;
                state.pose_body = true;
                state.pose_hands = false;
                state.pose_export_pending = false;
                state.pose_active_export_request = None;
                state.pose_last_export = None;
                state.pose_marker_family = "face".to_owned();
                state.pose_marker_index = 12;
                state.pose_marker_x = 321.0;
                state.pose_marker_y = 222.0;
                state.pose_marker_confidence = 0.87;
                state.pose_marker_edits.clear();
                state.pose_marker_status =
                    "Pose reset; marker edits cleared and ready for the next export.".to_owned();
                state.pose_framing_preset = "standard".to_owned();
                state.pose_framing_lens_mm = 50;
                state.pose_framing_padding_top_px = 0;
                state.pose_framing_padding_right_px = 0;
                state.pose_framing_padding_bottom_px = 0;
                state.pose_framing_padding_left_px = 0;
                state.pose_export_status =
                    "Pose reset; export again to refresh OpenPose artifact metadata.".to_owned();
            }
            ui.separator();
            let face = ui.checkbox(&mut state.pose_face, "Face");
            emit_node(
                ui.ctx(),
                face.id,
                accesskit::Role::CheckBox,
                ATELIER_POSE_FACE_TOGGLE_AUTHOR_ID,
                "Face markers",
                state.pose_face,
            );
            let body = ui.checkbox(&mut state.pose_body, "Body");
            emit_node(
                ui.ctx(),
                body.id,
                accesskit::Role::CheckBox,
                ATELIER_POSE_BODY_TOGGLE_AUTHOR_ID,
                "Body markers",
                state.pose_body,
            );
            let hands = ui.checkbox(&mut state.pose_hands, "Hands");
            emit_node(
                ui.ctx(),
                hands.id,
                accesskit::Role::CheckBox,
                ATELIER_POSE_HANDS_TOGGLE_AUTHOR_ID,
                "Hand markers",
                state.pose_hands,
            );
            if face.changed() || body.changed() || hands.changed() {
                posekit_warn_for_disabled_staged_marker_edits(&mut state);
            }
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Yaw").color(palette.text));
            let mut yaw_text = format!("{:.0}", state.pose_yaw);
            let yaw = ui.text_edit_singleline(&mut yaw_text);
            if yaw.changed() {
                if let Ok(value) = yaw_text.trim().parse::<f32>() {
                    state.pose_yaw = value.clamp(-180.0, 180.0);
                }
            }
            emit_value_node(
                ui.ctx(),
                yaw.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_YAW_SLIDER_AUTHOR_ID,
                "Posekit yaw degrees",
                &format!("{:.0}", state.pose_yaw),
            );

            ui.label(egui::RichText::new("Pitch").color(palette.text));
            let mut pitch_text = format!("{:.0}", state.pose_pitch);
            let pitch = ui.text_edit_singleline(&mut pitch_text);
            if pitch.changed() {
                if let Ok(value) = pitch_text.trim().parse::<f32>() {
                    state.pose_pitch = value.clamp(-45.0, 45.0);
                }
            }
            emit_value_node(
                ui.ctx(),
                pitch.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_PITCH_SLIDER_AUTHOR_ID,
                "Posekit pitch degrees",
                &format!("{:.0}", state.pose_pitch),
            );

            ui.label(egui::RichText::new("Zoom").color(palette.text));
            let mut zoom_text = format!("{:.2}", state.pose_zoom);
            let zoom = ui.text_edit_singleline(&mut zoom_text);
            if zoom.changed() {
                if let Ok(value) = zoom_text.trim().parse::<f32>() {
                    state.pose_zoom = value.clamp(0.4, 2.2);
                }
            }
            emit_value_node(
                ui.ctx(),
                zoom.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_ZOOM_SLIDER_AUTHOR_ID,
                "Posekit zoom",
                &format!("{:.2}", state.pose_zoom),
            );
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Marker").color(palette.text));
            let family = ui.text_edit_singleline(&mut state.pose_marker_family);
            emit_value_node(
                ui.ctx(),
                family.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_MARKER_FAMILY_AUTHOR_ID,
                "Posekit marker family",
                &state.pose_marker_family,
            );

            ui.label(egui::RichText::new("Index").color(palette.text));
            let mut index_text = state.pose_marker_index.to_string();
            let index = ui.text_edit_singleline(&mut index_text);
            if index.changed() {
                if let Ok(value) = index_text.trim().parse::<i32>() {
                    state.pose_marker_index = value;
                }
            }
            emit_value_node(
                ui.ctx(),
                index.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_MARKER_INDEX_AUTHOR_ID,
                "Posekit marker index",
                &state.pose_marker_index.to_string(),
            );

            ui.label(egui::RichText::new("X").color(palette.text));
            let mut x_text = format!("{:.1}", state.pose_marker_x);
            let x = ui.text_edit_singleline(&mut x_text);
            if x.changed() {
                if let Ok(value) = x_text.trim().parse::<f32>() {
                    state.pose_marker_x = value;
                }
            }
            emit_value_node(
                ui.ctx(),
                x.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_MARKER_X_AUTHOR_ID,
                "Posekit marker x coordinate",
                &format!("{:.1}", state.pose_marker_x),
            );

            ui.label(egui::RichText::new("Y").color(palette.text));
            let mut y_text = format!("{:.1}", state.pose_marker_y);
            let y = ui.text_edit_singleline(&mut y_text);
            if y.changed() {
                if let Ok(value) = y_text.trim().parse::<f32>() {
                    state.pose_marker_y = value;
                }
            }
            emit_value_node(
                ui.ctx(),
                y.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_MARKER_Y_AUTHOR_ID,
                "Posekit marker y coordinate",
                &format!("{:.1}", state.pose_marker_y),
            );

            ui.label(egui::RichText::new("Conf").color(palette.text));
            let mut confidence_text = format!("{:.2}", state.pose_marker_confidence);
            let confidence = ui.text_edit_singleline(&mut confidence_text);
            if confidence.changed() {
                if let Ok(value) = confidence_text.trim().parse::<f32>() {
                    state.pose_marker_confidence = value;
                }
            }
            emit_value_node(
                ui.ctx(),
                confidence.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_MARKER_CONFIDENCE_AUTHOR_ID,
                "Posekit marker confidence",
                &format!("{:.2}", state.pose_marker_confidence),
            );
        });
        ui.horizontal_wrapped(|ui| {
            let nudge_left = ui.button("<");
            emit_node(
                ui.ctx(),
                nudge_left.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_NUDGE_LEFT_AUTHOR_ID,
                "Nudge marker left",
                false,
            );
            if nudge_left.clicked() {
                posekit_nudge_marker(&mut state, -1.0, 0.0);
            }
            let nudge_right = ui.button(">");
            emit_node(
                ui.ctx(),
                nudge_right.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_NUDGE_RIGHT_AUTHOR_ID,
                "Nudge marker right",
                false,
            );
            if nudge_right.clicked() {
                posekit_nudge_marker(&mut state, 1.0, 0.0);
            }
            let nudge_up = ui.button("^");
            emit_node(
                ui.ctx(),
                nudge_up.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_NUDGE_UP_AUTHOR_ID,
                "Nudge marker up",
                false,
            );
            if nudge_up.clicked() {
                posekit_nudge_marker(&mut state, 0.0, -1.0);
            }
            let nudge_down = ui.button("v");
            emit_node(
                ui.ctx(),
                nudge_down.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_NUDGE_DOWN_AUTHOR_ID,
                "Nudge marker down",
                false,
            );
            if nudge_down.clicked() {
                posekit_nudge_marker(&mut state, 0.0, 1.0);
            }

            let apply = ui.button("Apply");
            emit_node(
                ui.ctx(),
                apply.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_APPLY_AUTHOR_ID,
                "Apply marker edit",
                false,
            );
            if apply.clicked() {
                posekit_stage_marker_edit(&mut state, "set");
            }
            let add = ui.button("Add");
            emit_node(
                ui.ctx(),
                add.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_ADD_AUTHOR_ID,
                "Add marker into empty slot",
                false,
            );
            if add.clicked() {
                posekit_stage_marker_edit(&mut state, "add");
            }
            let remove = ui.button("Remove");
            emit_node(
                ui.ctx(),
                remove.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_REMOVE_AUTHOR_ID,
                "Remove marker",
                false,
            );
            if remove.clicked() {
                posekit_stage_marker_edit(&mut state, "remove");
            }
            let reset_marker = ui.button("Clear edits");
            emit_node(
                ui.ctx(),
                reset_marker.id,
                accesskit::Role::Button,
                ATELIER_POSE_MARKER_RESET_AUTHOR_ID,
                "Clear staged marker edits",
                false,
            );
            if reset_marker.clicked() {
                state.pose_marker_edits.clear();
                state.pose_marker_status =
                    "Posekit marker edits cleared; last export remains visible until refreshed."
                        .to_owned();
            }
        });
        let marker_status = state.pose_marker_status.clone();
        let marker_status_response =
            ui.label(egui::RichText::new(&marker_status).color(palette.text_subtle));
        emit_value_node(
            ui.ctx(),
            marker_status_response.id,
            accesskit::Role::Label,
            ATELIER_POSE_MARKER_STATUS_AUTHOR_ID,
            "Posekit marker edit status",
            &marker_status,
        );

        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Framing").color(palette.text));
            let preset = ui.text_edit_singleline(&mut state.pose_framing_preset);
            emit_value_node(
                ui.ctx(),
                preset.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_FRAMING_PRESET_AUTHOR_ID,
                "Posekit framing preset",
                &posekit_framing_preset(&state.pose_framing_preset),
            );

            ui.label(egui::RichText::new("Lens mm").color(palette.text));
            let mut lens_text = state.pose_framing_lens_mm.to_string();
            let lens = ui.text_edit_singleline(&mut lens_text);
            if lens.changed() {
                if let Ok(value) = lens_text.trim().parse::<i32>() {
                    state.pose_framing_lens_mm = value.clamp(18, 120);
                }
            }
            emit_value_node(
                ui.ctx(),
                lens.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_FRAMING_LENS_AUTHOR_ID,
                "Posekit framing lens millimeters",
                &state.pose_framing_lens_mm.clamp(18, 120).to_string(),
            );

            ui.label(egui::RichText::new("Top").color(palette.text));
            let mut top_text = state.pose_framing_padding_top_px.to_string();
            let top = ui.text_edit_singleline(&mut top_text);
            if top.changed() {
                if let Ok(value) = top_text.trim().parse::<i32>() {
                    state.pose_framing_padding_top_px = value.clamp(0, 256);
                }
            }
            emit_value_node(
                ui.ctx(),
                top.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_FRAMING_PADDING_TOP_AUTHOR_ID,
                "Posekit framing top padding pixels",
                &state.pose_framing_padding_top_px.clamp(0, 256).to_string(),
            );

            ui.label(egui::RichText::new("Right").color(palette.text));
            let mut right_text = state.pose_framing_padding_right_px.to_string();
            let right = ui.text_edit_singleline(&mut right_text);
            if right.changed() {
                if let Ok(value) = right_text.trim().parse::<i32>() {
                    state.pose_framing_padding_right_px = value.clamp(0, 256);
                }
            }
            emit_value_node(
                ui.ctx(),
                right.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_FRAMING_PADDING_RIGHT_AUTHOR_ID,
                "Posekit framing right padding pixels",
                &state
                    .pose_framing_padding_right_px
                    .clamp(0, 256)
                    .to_string(),
            );

            ui.label(egui::RichText::new("Bottom").color(palette.text));
            let mut bottom_text = state.pose_framing_padding_bottom_px.to_string();
            let bottom = ui.text_edit_singleline(&mut bottom_text);
            if bottom.changed() {
                if let Ok(value) = bottom_text.trim().parse::<i32>() {
                    state.pose_framing_padding_bottom_px = value.clamp(0, 256);
                }
            }
            emit_value_node(
                ui.ctx(),
                bottom.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_FRAMING_PADDING_BOTTOM_AUTHOR_ID,
                "Posekit framing bottom padding pixels",
                &state
                    .pose_framing_padding_bottom_px
                    .clamp(0, 256)
                    .to_string(),
            );

            ui.label(egui::RichText::new("Left").color(palette.text));
            let mut left_text = state.pose_framing_padding_left_px.to_string();
            let left = ui.text_edit_singleline(&mut left_text);
            if left.changed() {
                if let Ok(value) = left_text.trim().parse::<i32>() {
                    state.pose_framing_padding_left_px = value.clamp(0, 256);
                }
            }
            emit_value_node(
                ui.ctx(),
                left.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_FRAMING_PADDING_LEFT_AUTHOR_ID,
                "Posekit framing left padding pixels",
                &state.pose_framing_padding_left_px.clamp(0, 256).to_string(),
            );
        });
        let framing_readout = posekit_framing_readout(&state);
        let framing_response =
            ui.label(egui::RichText::new(&framing_readout).color(palette.text_subtle));
        emit_value_node(
            ui.ctx(),
            framing_response.id,
            accesskit::Role::Label,
            ATELIER_POSE_FRAMING_READOUT_AUTHOR_ID,
            "Posekit export framing readout",
            &framing_readout,
        );
        ui.separator();
        let split = ui
            .scope_builder(
                egui::UiBuilder::new().id_salt(ATELIER_POSE_SPLIT_VIEW_AUTHOR_ID),
                |ui| {
                    ui.columns(2, |cols| {
                        draw_pose_view(
                            &mut cols[0],
                            palette,
                            "3D rig/source preview",
                            ATELIER_POSE_3D_VIEWPORT_AUTHOR_ID,
                            state.pose_yaw,
                            state.pose_pitch,
                            state.pose_zoom,
                            state.pose_face,
                            state.pose_body,
                            state.pose_hands,
                            &state.pose_source_ref,
                            posekit_optional_rig_id(&state.pose_rig_id).as_deref(),
                            false,
                        );
                        draw_pose_view(
                            &mut cols[1],
                            palette,
                            "OpenPose preview",
                            ATELIER_POSE_OPENPOSE_VIEWPORT_AUTHOR_ID,
                            state.pose_yaw,
                            state.pose_pitch,
                            state.pose_zoom,
                            state.pose_face,
                            state.pose_body,
                            state.pose_hands,
                            &state.pose_source_ref,
                            posekit_optional_rig_id(&state.pose_rig_id).as_deref(),
                            true,
                        );
                    });
                },
            )
            .response;
        emit_node(
            ui.ctx(),
            split.id,
            accesskit::Role::Group,
            ATELIER_POSE_SPLIT_VIEW_AUTHOR_ID,
            "Posekit rig/OpenPose split view",
            false,
        );
        ui.separator();
        let export = ui.add_enabled(
            !state.pose_export_pending,
            egui::Button::new("Export OpenPose"),
        );
        emit_node(
            ui.ctx(),
            export.id,
            accesskit::Role::Button,
            ATELIER_POSE_EXPORT_AUTHOR_ID,
            "Export ComfyUI-ready OpenPose",
            state.pose_export_pending,
        );
        if export.clicked() {
            if !(state.pose_face || state.pose_body || state.pose_hands) {
                state.pose_export_status =
                    "Posekit OpenPose export failed: enable at least one marker layer.".to_owned();
            } else if state.pose_source_ref.trim().is_empty()
                || state.pose_source_ref.trim() != state.pose_source_ref
            {
                state.pose_export_status =
                    "Posekit OpenPose export failed: source_ref must be non-empty and unpadded."
                        .to_owned();
            } else if let Some(client) = self.ckc_client.as_ref() {
                match posekit_validate_staged_marker_edits_for_export(&state, true) {
                    Ok(()) => {
                        state.pose_export_request_seq =
                            state.pose_export_request_seq.saturating_add(1);
                        let request_id = state.pose_export_request_seq;
                        state.pose_active_export_request = Some(request_id);
                        state.pose_export_pending = true;
                        state.pose_export_status =
                            "Posekit backend OpenPose export pending; waiting for ArtifactStore refs."
                                .to_owned();
                        let rig_id = posekit_optional_rig_id(&state.pose_rig_id);
                        let marker_edits = posekit_marker_edits_json(&state.pose_marker_edits);
                        let framing = posekit_framing_json_from_state(&state);
                        client.export_posekit_openpose(
                            &state.pose_source_ref,
                            state.pose_yaw,
                            state.pose_pitch,
                            state.pose_zoom,
                            state.pose_face,
                            state.pose_body,
                            state.pose_hands,
                            rig_id.as_deref(),
                            marker_edits,
                            framing,
                            client.actor_id(),
                            request_id,
                            self.pose_export_cell.clone(),
                        );
                    }
                    Err(err) => {
                        state.pose_export_status = format!("Posekit OpenPose export failed: {err}");
                    }
                }
            } else {
                match posekit_export_snapshot(&state) {
                    Ok(snapshot) => {
                        state.pose_export_status = format!(
                            "Local Argus preview only: yaw_deg={:.0} png_artifact_ref={} json_artifact_ref={} receipt_ref={}",
                            snapshot.yaw_deg,
                            snapshot.png_artifact_ref,
                            snapshot.json_artifact_ref,
                            snapshot.receipt_ref
                        );
                        state.pose_last_export = Some(snapshot);
                    }
                    Err(err) => {
                        state.pose_export_status =
                            format!("Local Posekit OpenPose export failed: {err}");
                    }
                }
            }
        }
        let export_status = state.pose_export_status.clone();
        let status = ui.label(egui::RichText::new(&export_status).color(palette.text));
        emit_value_node(
            ui.ctx(),
            status.id,
            accesskit::Role::Label,
            ATELIER_POSE_EXPORT_STATUS_AUTHOR_ID,
            "Posekit export status",
            &export_status,
        );
        if let Some(snapshot) = state.pose_last_export.as_ref() {
            let export_ref_label = format!(
                "{} {} {} {} {} {}",
                snapshot.png_artifact_ref,
                snapshot.json_artifact_ref,
                snapshot.receipt_ref,
                snapshot.content_hash,
                snapshot.png_manifest_ref,
                snapshot.json_manifest_ref
            );
            let export_ref = ui.label(&export_ref_label);
            emit_value_node(
                ui.ctx(),
                export_ref.id,
                accesskit::Role::Label,
                ATELIER_POSE_EXPORT_REF_AUTHOR_ID,
                "Posekit OpenPose artifact and receipt refs",
                &export_ref_label,
            );
            let mut preview = posekit_export_preview(snapshot);
            let preview_response = ui.add(
                egui::TextEdit::multiline(&mut preview)
                    .desired_rows(8)
                    .interactive(false),
            );
            emit_value_node(
                ui.ctx(),
                preview_response.id,
                accesskit::Role::TextInput,
                ATELIER_POSE_EXPORT_PREVIEW_AUTHOR_ID,
                "Posekit ComfyUI-ready OpenPose export preview",
                &preview,
            );
        }
    }

    fn show_ingest(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        self.drain_ingest_classification_backend();
        ui.label(egui::RichText::new("Intake batch source").color(palette.text));
        if let Ok(mut side_panel) = self.side_panel.lock() {
            side_panel.show(ui, palette);
        }
        ui.separator();
        let expanded_items = self
            .side_panel
            .lock()
            .ok()
            .and_then(|panel| panel.expanded().cloned());
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Dataset").color(palette.text));
            let dataset = ui.text_edit_singleline(&mut state.ingest_dataset_ref);
            emit_value_node(
                ui.ctx(),
                dataset.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_DATASET_REF_AUTHOR_ID,
                "Ingest dataset or source folder ref",
                &state.ingest_dataset_ref,
            );
            ui.label(egui::RichText::new("Character").color(palette.text));
            let character = ui.text_edit_singleline(&mut state.ingest_character_ref);
            emit_value_node(
                ui.ctx(),
                character.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_CHARACTER_REF_AUTHOR_ID,
                "CKC character ref for passed image links",
                &state.ingest_character_ref,
            );
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for decision in [
                IngestDecision::Pass,
                IngestDecision::Reject,
                IngestDecision::Unsure,
            ] {
                let selected = state.ingest_decision == decision;
                let button = ui.add(egui::Button::selectable(selected, decision.label()));
                let author_id = match decision {
                    IngestDecision::Pass => ATELIER_INGEST_PASS_AUTHOR_ID,
                    IngestDecision::Reject => ATELIER_INGEST_REJECT_AUTHOR_ID,
                    IngestDecision::Unsure => ATELIER_INGEST_UNSURE_AUTHOR_ID,
                };
                emit_node(
                    ui.ctx(),
                    button.id,
                    accesskit::Role::Button,
                    author_id,
                    decision.label(),
                    selected,
                );
                if button.clicked() {
                    state.ingest_decision = decision;
                    let loaded_count = expanded_items.as_ref().map_or(0, |(_, items)| {
                        for item in items {
                            state
                                .ingest_item_decisions
                                .insert(item.item_id.clone(), decision);
                            state.ingest_persisted_item_ids.remove(&item.item_id);
                        }
                        items.len()
                    });
                    state.ingest_status = if loaded_count > 0 {
                        format!(
                            "Ingest {} staged for {} loaded rows",
                            decision.machine_label(),
                            loaded_count
                        )
                    } else {
                        format!(
                            "Ingest default decision staged: {}",
                            decision.machine_label()
                        )
                    };
                }
            }
            let link = ui.checkbox(&mut state.ingest_link_passed, "CKC link intent");
            emit_node(
                ui.ctx(),
                link.id,
                accesskit::Role::CheckBox,
                ATELIER_INGEST_LINK_PASSED_AUTHOR_ID,
                "Persist CKC link intent metadata for passed images",
                state.ingest_link_passed,
            );
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Batch tags").color(palette.text));
            let tags = ui.text_edit_singleline(&mut state.ingest_tag_buffer);
            emit_value_node(
                ui.ctx(),
                tags.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_BATCH_TAGS_AUTHOR_ID,
                "Batch tags",
                &state.ingest_tag_buffer,
            );
            ui.label(egui::RichText::new("Facial").color(palette.text));
            let facial = ui.text_edit_singleline(&mut state.ingest_facial_profile);
            emit_value_node(
                ui.ctx(),
                facial.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_FACIAL_PROFILE_AUTHOR_ID,
                "Facial quality, dedupe, and identity profile hint",
                &state.ingest_facial_profile,
            );
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Note").color(palette.text));
            let note = ui.text_edit_singleline(&mut state.ingest_batch_note);
            emit_value_node(
                ui.ctx(),
                note.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_BATCH_NOTE_AUTHOR_ID,
                "Batch note applied to reviewed images",
                &state.ingest_batch_note,
            );
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Event").color(palette.text));
            let event = ui.text_edit_singleline(&mut state.ingest_event);
            emit_value_node(
                ui.ctx(),
                event.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_EVENT_AUTHOR_ID,
                "Batch event metadata",
                &state.ingest_event,
            );
            ui.label(egui::RichText::new("Date").color(palette.text));
            let date = ui.text_edit_singleline(&mut state.ingest_date);
            emit_value_node(
                ui.ctx(),
                date.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_DATE_AUTHOR_ID,
                "Batch date metadata",
                &state.ingest_date,
            );
            ui.label(egui::RichText::new("Location").color(palette.text));
            let location = ui.text_edit_singleline(&mut state.ingest_location);
            emit_value_node(
                ui.ctx(),
                location.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_LOCATION_AUTHOR_ID,
                "Batch location metadata",
                &state.ingest_location,
            );
        });
        ui.horizontal(|ui| {
            let apply = ui.add_enabled(
                !state.ingest_apply_pending,
                egui::Button::new("Apply loaded rows"),
            );
            emit_node(
                ui.ctx(),
                apply.id,
                accesskit::Role::Button,
                ATELIER_INGEST_APPLY_BATCH_AUTHOR_ID,
                "Apply loaded intake row classifications with structured ingest metadata",
                state.ingest_apply_pending,
            );
            if apply.clicked() {
                let readout = ingest_queue_readout(&state);
                if let (Some(client), Some((batch_id, items))) =
                    (self.ckc_client.as_ref(), expanded_items.as_ref())
                {
                    let request_id = format!("atelier-ingest-{}", Uuid::new_v4());
                    let metadata = ingest_metadata_payload(
                        &state,
                        &request_id,
                        Some(batch_id.as_str()),
                        items.len(),
                    );
                    let decisions: Vec<AtelierIntakeClassificationDecision> = items
                        .iter()
                        .map(|item| {
                            let decision = ingest_item_decision(&state, item);
                            let reason = format!(
                                "dataset_ref={} character_ref={} link_passed={} tags={} note={} event={} date={} location={} facial_profile={}",
                                state.ingest_dataset_ref.trim(),
                                state.ingest_character_ref.trim(),
                                state.ingest_link_passed,
                                state.ingest_tag_buffer.trim(),
                                state.ingest_batch_note.trim(),
                                state.ingest_event.trim(),
                                state.ingest_date.trim(),
                                state.ingest_location.trim(),
                                state.ingest_facial_profile.trim()
                            );
                            AtelierIntakeClassificationDecision {
                                item_id: item.item_id.clone(),
                                lane: decision.backend_lane().to_owned(),
                                reason: Some(reason),
                                metadata: metadata.clone(),
                            }
                        })
                        .collect();
                    if decisions.is_empty() {
                        state.ingest_status =
                            format!("No loaded intake items to classify: {readout}");
                    } else {
                        let count = decisions.len();
                        state.ingest_apply_pending = true;
                        state.ingest_apply_request_id = Some(request_id.clone());
                        state.ingest_apply_batch_id = Some(batch_id.clone());
                        state.ingest_status = format!(
                            "Dispatching {count} loaded intake item classification(s) to backend with request_id={request_id}: {readout}"
                        );
                        client.apply_intake_classifications(
                            request_id,
                            Some(batch_id.clone()),
                            decisions,
                            client.actor_id(),
                            self.ingest_classification_cell.clone(),
                        );
                    }
                } else {
                    state.ingest_status = format!("Loaded-row metadata staged locally: {readout}");
                }
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Contact sheet").color(palette.text));
            let rows = ui.add(
                egui::TextEdit::singleline(&mut state.ingest_contact_rows).desired_width(42.0),
            );
            emit_value_node(
                ui.ctx(),
                rows.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_CONTACT_ROWS_AUTHOR_ID,
                "Contact sheet rows",
                &state.ingest_contact_rows,
            );
            ui.label("x");
            let columns = ui.add(
                egui::TextEdit::singleline(&mut state.ingest_contact_columns).desired_width(42.0),
            );
            emit_value_node(
                ui.ctx(),
                columns.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_CONTACT_COLUMNS_AUTHOR_ID,
                "Contact sheet columns",
                &state.ingest_contact_columns,
            );
            ui.label("@");
            let dpi = ui
                .add(egui::TextEdit::singleline(&mut state.ingest_contact_dpi).desired_width(58.0));
            emit_value_node(
                ui.ctx(),
                dpi.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_CONTACT_DPI_AUTHOR_ID,
                "Contact sheet DPI",
                &state.ingest_contact_dpi,
            );
            ui.label("dpi");
            let export = ui.button("Stage contact settings");
            emit_node(
                ui.ctx(),
                export.id,
                accesskit::Role::Button,
                ATELIER_INGEST_CONTACT_EXPORT_AUTHOR_ID,
                "Stage contact sheet settings",
                false,
            );
            if export.clicked() {
                let (rows, columns, dpi, cells) = ingest_contact_sheet_shape(&state);
                state.ingest_status = format!(
                    "Contact sheet staged: {rows}x{columns}@{dpi}dpi with {cells} cells for {}",
                    state.ingest_dataset_ref.trim()
                );
            }
        });
        ui.add_space(6.0);
        let queue_readout = ingest_queue_readout(&state);
        let queue = ui.label(egui::RichText::new(&queue_readout).color(palette.text_subtle));
        emit_value_node(
            ui.ctx(),
            queue.id,
            accesskit::Role::Label,
            ATELIER_INGEST_QUEUE_READOUT_AUTHOR_ID,
            "Ingest queue readout",
            &queue_readout,
        );
        let status = ui.label(egui::RichText::new(&state.ingest_status).color(palette.text));
        emit_value_node(
            ui.ctx(),
            status.id,
            accesskit::Role::Label,
            ATELIER_INGEST_STATUS_AUTHOR_ID,
            "Ingest status",
            &state.ingest_status,
        );
        let receipt = ui.label(
            egui::RichText::new(&state.ingest_last_apply_receipt).color(palette.text_subtle),
        );
        emit_value_node(
            ui.ctx(),
            receipt.id,
            accesskit::Role::Label,
            ATELIER_INGEST_LAST_RECEIPT_AUTHOR_ID,
            "Last Ingest backend apply receipt",
            &state.ingest_last_apply_receipt,
        );
        ui.separator();
        egui::Grid::new("atelier-ingest-grid")
            .striped(true)
            .min_col_width(110.0)
            .show(ui, |ui| {
                ui.strong("Asset");
                ui.strong("Source");
                ui.strong("Decision");
                ui.strong("Apply state");
                ui.strong("Set");
                ui.strong("Tags");
                ui.strong("CKC link");
                ui.end_row();
                let Some((batch_id, items)) = expanded_items.as_ref() else {
                    ui.label("No loaded intake batch.");
                    ui.label("-");
                    ui.label(state.ingest_decision.label());
                    ui.label("-");
                    ui.label("-");
                    ui.label(&state.ingest_tag_buffer);
                    ui.label("-");
                    ui.end_row();
                    return;
                };
                if items.is_empty() {
                    ui.label(format!("Batch {batch_id} has no loaded items."));
                    ui.label("-");
                    ui.label(state.ingest_decision.label());
                    ui.label("-");
                    ui.label("-");
                    ui.label(&state.ingest_tag_buffer);
                    ui.label("-");
                    ui.end_row();
                    return;
                }
                for item in items {
                    let item_decision = ingest_item_decision(&state, item);
                    let persisted_state = if state.ingest_persisted_item_ids.contains(&item.item_id)
                    {
                        "persisted"
                    } else {
                        "staged"
                    };
                    let item_ref = format!(
                        "item_id={} file_name={} source_path={} source_lane={} staged_decision={} apply_state={}",
                        item.item_id,
                        item.file_name,
                        item.source_path,
                        item.lane,
                        item_decision.machine_label(),
                        persisted_state
                    );
                    let item_label = ui.label(&item.file_name);
                    let row_author_id = ingest_item_row_author_id(&item.item_id);
                    emit_value_node(
                        ui.ctx(),
                        item_label.id,
                        accesskit::Role::ListItem,
                        &row_author_id,
                        &format!("Ingest item {}", item.file_name),
                        &item_ref,
                    );
                    ui.label(&item.source_path);
                    ui.label(item_decision.label());
                    ui.label(persisted_state);
                    ui.horizontal(|ui| {
                        for decision in [
                            IngestDecision::Pass,
                            IngestDecision::Reject,
                            IngestDecision::Unsure,
                        ] {
                            let selected = item_decision == decision;
                            let button = ui
                                .add(egui::Button::selectable(selected, decision.machine_label()));
                            let author_id = match decision {
                                IngestDecision::Pass => ingest_item_pass_author_id(&item.item_id),
                                IngestDecision::Reject => {
                                    ingest_item_reject_author_id(&item.item_id)
                                }
                                IngestDecision::Unsure => {
                                    ingest_item_unsure_author_id(&item.item_id)
                                }
                            };
                            emit_node(
                                ui.ctx(),
                                button.id,
                                accesskit::Role::Button,
                                &author_id,
                                &format!("Set {} to {}", item.file_name, decision.machine_label()),
                                selected,
                            );
                            if button.clicked() {
                                state
                                    .ingest_item_decisions
                                    .insert(item.item_id.clone(), decision);
                                state.ingest_persisted_item_ids.remove(&item.item_id);
                                state.ingest_status = format!(
                                    "Ingest item staged: {} -> {}",
                                    item.file_name,
                                    decision.machine_label()
                                );
                            }
                        }
                    });
                    ui.label(&state.ingest_tag_buffer);
                    ui.label(
                        if state.ingest_link_passed && item_decision == IngestDecision::Pass {
                            state.ingest_character_ref.as_str()
                        } else {
                            "-"
                        },
                    );
                    ui.end_row();
                }
            });
    }
}

pub struct AtelierPanelPaneMount {
    panel: AtelierPanel,
    palette: SharedPalette,
}

impl AtelierPanelPaneMount {
    pub fn new(
        side_panel: Arc<Mutex<AtelierSidePanel>>,
        canvas_board: Arc<Mutex<LoomCanvasBoard>>,
        palette: SharedPalette,
        canvas_events: Arc<Mutex<Vec<CanvasEvent>>>,
    ) -> Self {
        Self::with_optional_client(side_panel, canvas_board, palette, canvas_events, None)
    }

    pub fn with_client(
        side_panel: Arc<Mutex<AtelierSidePanel>>,
        canvas_board: Arc<Mutex<LoomCanvasBoard>>,
        palette: SharedPalette,
        canvas_events: Arc<Mutex<Vec<CanvasEvent>>>,
        ckc_client: AtelierClient,
    ) -> Self {
        Self::with_optional_client(
            side_panel,
            canvas_board,
            palette,
            canvas_events,
            Some(ckc_client),
        )
    }

    fn with_optional_client(
        side_panel: Arc<Mutex<AtelierSidePanel>>,
        canvas_board: Arc<Mutex<LoomCanvasBoard>>,
        palette: SharedPalette,
        canvas_events: Arc<Mutex<Vec<CanvasEvent>>>,
        ckc_client: Option<AtelierClient>,
    ) -> Self {
        Self {
            panel: AtelierPanel::with_client(side_panel, canvas_board, canvas_events, ckc_client),
            palette,
        }
    }
}

impl PaneFactory for AtelierPanelPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::AtelierEditor
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        self.panel.show(ui, &palette);
    }
}

fn palette_of(cell: &SharedPalette) -> HsPalette {
    cell.lock()
        .map(|p| p.clone())
        .unwrap_or_else(|p| p.into_inner().clone())
}

fn draw_pose_view(
    ui: &mut egui::Ui,
    palette: &HsPalette,
    label: &str,
    author_id: &str,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    face: bool,
    body: bool,
    hands: bool,
    source_ref: &str,
    rig_id: Option<&str>,
    openpose: bool,
) {
    ui.label(egui::RichText::new(label).strong().color(palette.text));
    let height = 260.0;
    let width = ui.available_width().max(180.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(
        rect,
        4.0,
        if openpose {
            egui::Color32::BLACK
        } else {
            palette.surface
        },
    );

    let center = rect.center() + egui::vec2(yaw / 180.0 * 24.0, pitch / 45.0 * 18.0);
    let scale = zoom.clamp(0.4, 2.2);
    let head_r = 22.0 * scale;
    let torso = 64.0 * scale;
    let color = if openpose {
        egui::Color32::from_rgb(70, 220, 255)
    } else {
        palette.accent
    };
    let muted = if openpose {
        egui::Color32::from_rgb(255, 190, 80)
    } else {
        palette.text_subtle
    };
    let faint = if openpose {
        egui::Color32::from_gray(70)
    } else {
        palette.border
    };
    let viewport_mode = if openpose {
        "openpose_conditioning_preview"
    } else {
        "native_3d_projection_preview"
    };
    let projection_mode = if rig_id.is_some() {
        "rig-linked-native-preview"
    } else {
        "procedural-posekit-preview"
    };
    let source_fingerprint =
        stable_posekit_hash(&format!("{}|{}", source_ref, rig_id.unwrap_or("<none>")));

    if !openpose {
        let tint_r = u8::from_str_radix(&source_fingerprint[0..2], 16).unwrap_or(96);
        let tint_g = u8::from_str_radix(&source_fingerprint[2..4], 16).unwrap_or(128);
        let tint_b = u8::from_str_radix(&source_fingerprint[4..6], 16).unwrap_or(160);
        let source_tile = egui::Rect::from_min_size(
            rect.left_top() + egui::vec2(12.0, 12.0),
            egui::vec2(74.0, 48.0),
        );
        painter.rect_filled(
            source_tile,
            3.0,
            egui::Color32::from_rgb(32 + tint_r / 4, 32 + tint_g / 4, 32 + tint_b / 4),
        );
        painter.rect_stroke(
            source_tile,
            3.0,
            egui::Stroke::new(1.0, muted),
            egui::StrokeKind::Outside,
        );
        let floor_y = rect.top() + height * 0.74;
        for step in 0..4 {
            let y = floor_y + step as f32 * 12.0 * scale;
            if y < rect.bottom() - 8.0 {
                painter.line_segment(
                    [
                        egui::pos2(rect.left() + 12.0, y),
                        egui::pos2(rect.right() - 12.0, y),
                    ],
                    egui::Stroke::new(1.0, faint),
                );
            }
        }
        let yaw_radians = yaw.to_radians();
        let pelvis = center + egui::vec2(0.0, torso * 0.45);
        let yaw_tip = pelvis
            + egui::vec2(
                yaw_radians.sin() * 54.0 * scale,
                -yaw_radians.cos() * 20.0 * scale,
            );
        painter.circle_stroke(pelvis, 58.0 * scale, egui::Stroke::new(1.0, faint));
        painter.line_segment([pelvis, yaw_tip], egui::Stroke::new(2.0, muted));
        painter.circle_filled(yaw_tip, 3.0, muted);

        let yaw_radians = yaw.to_radians();
        let pitch_radians = pitch.to_radians();
        let project = |x: f32, y: f32, z: f32| {
            let yaw_x = x * yaw_radians.cos() + z * yaw_radians.sin();
            let yaw_z = -x * yaw_radians.sin() + z * yaw_radians.cos();
            let pitch_y = y * pitch_radians.cos() - yaw_z * pitch_radians.sin();
            let pitch_z = y * pitch_radians.sin() + yaw_z * pitch_radians.cos();
            let perspective = 1.0 / (1.0 + (pitch_z + 220.0).max(16.0) / 760.0);
            center + egui::vec2(yaw_x * perspective * scale, pitch_y * perspective * scale)
        };
        let neck = project(0.0, -92.0, 28.0);
        let pelvis_3d = project(0.0, 28.0, -20.0);
        let left_shoulder = project(-76.0, -76.0, 8.0);
        let right_shoulder = project(76.0, -76.0, 8.0);
        let left_hand = project(-128.0, 8.0, -18.0);
        let right_hand = project(128.0, 8.0, -18.0);
        let left_foot = project(-44.0, 164.0, 12.0);
        let right_foot = project(44.0, 164.0, 12.0);
        let projection_stroke = egui::Stroke::new(3.0, color);
        for (from, to) in [
            (neck, pelvis_3d),
            (left_shoulder, right_shoulder),
            (left_shoulder, left_hand),
            (right_shoulder, right_hand),
            (pelvis_3d, left_foot),
            (pelvis_3d, right_foot),
        ] {
            painter.line_segment([from, to], projection_stroke);
        }
        for joint in [
            neck,
            pelvis_3d,
            left_shoulder,
            right_shoulder,
            left_hand,
            right_hand,
            left_foot,
            right_foot,
        ] {
            painter.circle_filled(joint, 4.0, muted);
        }
    }

    let stroke = egui::Stroke::new(2.0, if body { color } else { faint });
    if face || !openpose {
        painter.circle_stroke(center + egui::vec2(0.0, -58.0 * scale), head_r, stroke);
    }
    if body || !openpose {
        painter.line_segment(
            [
                center + egui::vec2(0.0, -36.0 * scale),
                center + egui::vec2(0.0, torso * 0.45),
            ],
            stroke,
        );
        painter.line_segment(
            [
                center + egui::vec2(-42.0 * scale, -8.0 * scale),
                center + egui::vec2(42.0 * scale, -8.0 * scale),
            ],
            stroke,
        );
        painter.line_segment(
            [
                center + egui::vec2(-28.0 * scale, torso * 0.95),
                center + egui::vec2(0.0, torso * 0.45),
            ],
            stroke,
        );
        painter.line_segment(
            [
                center + egui::vec2(28.0 * scale, torso * 0.95),
                center + egui::vec2(0.0, torso * 0.45),
            ],
            stroke,
        );
    }

    if openpose {
        if face {
            for point in [
                center + egui::vec2(0.0, -58.0 * scale),
                center + egui::vec2(-10.0 * scale, -62.0 * scale),
                center + egui::vec2(10.0 * scale, -62.0 * scale),
                center + egui::vec2(0.0, -50.0 * scale),
            ] {
                painter.circle_filled(point, 3.5, muted);
            }
        }
        if body {
            for point in [
                center + egui::vec2(-42.0 * scale, -8.0 * scale),
                center + egui::vec2(42.0 * scale, -8.0 * scale),
                center + egui::vec2(0.0, torso * 0.45),
                center + egui::vec2(-28.0 * scale, torso * 0.95),
                center + egui::vec2(28.0 * scale, torso * 0.95),
            ] {
                painter.circle_filled(point, 3.5, muted);
            }
        }
        if hands {
            for side in [-1.0_f32, 1.0_f32] {
                for joint in 0..5 {
                    let point = center
                        + egui::vec2(
                            side * (66.0 + joint as f32 * 5.0) * scale,
                            (18.0 - joint as f32 * 6.0) * scale,
                        );
                    painter.circle_filled(point, 2.8, egui::Color32::from_rgb(120, 255, 150));
                }
            }
        }
    }
    let node_label = format!(
        "{label} viewport_mode={} projection={} source_ref={} rig_id={} source_fingerprint={} yaw_deg={:.0} pitch_deg={:.0} zoom={:.2} markers={}",
        viewport_mode,
        projection_mode,
        source_ref,
        rig_id.unwrap_or("<none>"),
        &source_fingerprint[..12],
        yaw,
        pitch,
        zoom,
        marker_layer_summary(face, body, hands)
    );
    emit_value_node(
        ui.ctx(),
        response.id,
        accesskit::Role::Group,
        author_id,
        &node_label,
        &node_label,
    );
}

fn emit_node(
    ctx: &egui::Context,
    id: egui::Id,
    role: accesskit::Role,
    author_id: &str,
    label: &str,
    selected: bool,
) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ctx.accesskit_node_builder(id, move |node| {
        node.set_role(role);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        if selected {
            node.set_selected(true);
        }
        if matches!(
            role,
            accesskit::Role::Tab
                | accesskit::Role::Button
                | accesskit::Role::CheckBox
                | accesskit::Role::ListItem
        ) {
            node.add_action(accesskit::Action::Click);
        }
        if matches!(role, accesskit::Role::TextInput | accesskit::Role::Slider) {
            node.add_action(accesskit::Action::Focus);
        }
    });
}

fn emit_draggable_list_item_node(
    ctx: &egui::Context,
    id: egui::Id,
    author_id: &str,
    label: &str,
    description: &str,
    selected: bool,
) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    let description = description.to_owned();
    ctx.accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::ListItem);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_description(description.clone());
        if selected {
            node.set_selected(true);
        }
        node.add_action(accesskit::Action::Click);
    });
}

fn emit_value_node(
    ctx: &egui::Context,
    id: egui::Id,
    role: accesskit::Role,
    author_id: &str,
    label: &str,
    value: &str,
) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    let value = value.to_owned();
    ctx.accesskit_node_builder(id, move |node| {
        node.set_role(role);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_value(value.clone());
        if matches!(role, accesskit::Role::TextInput | accesskit::Role::Slider) {
            node.add_action(accesskit::Action::Focus);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend_client::{
        AtelierBatchRow, AtelierCkcDocumentVersionRow, AtelierCkcSheetArtifactLinkRow,
        AtelierItemRow,
    };

    fn empty_side_panel() -> Arc<Mutex<AtelierSidePanel>> {
        Arc::new(Mutex::new(AtelierSidePanel::with_rows(
            Vec::<AtelierBatchRow>::new(),
            Vec::new(),
            None::<(String, Vec<AtelierItemRow>)>,
        )))
    }

    #[test]
    fn backend_ckc_search_result_filters_nonmatching_scoped_tag_notes() {
        let result = CkcSearchResultRecord::from_backend(AtelierCkcSearchResultRow {
            target_kind: "media".to_owned(),
            target_ref: "atelier://media/asset-1".to_owned(),
            title: "hero".to_owned(),
            snippet: "training face".to_owned(),
            character_ref: Some("atelier://character/char-1".to_owned()),
            sheet_version_ref: Some("atelier://sheet/char-1/sheet-1".to_owned()),
            collection_ref: Some("atelier://collection/album-1".to_owned()),
            media_ref: Some("atelier://media/asset-1".to_owned()),
            tag_ref: None,
            tags: vec!["training".to_owned()],
            tag_notes: vec![
                AtelierCkcTagNoteRow {
                    tag_ref: "atelier://tag/tag-1".to_owned(),
                    tag_text: "training".to_owned(),
                    scope_ref: None,
                    note: "global training note".to_owned(),
                },
                AtelierCkcTagNoteRow {
                    tag_ref: "atelier://tag/tag-1".to_owned(),
                    tag_text: "training".to_owned(),
                    scope_ref: Some("atelier://collection/album-1".to_owned()),
                    note: "matching album note".to_owned(),
                },
                AtelierCkcTagNoteRow {
                    tag_ref: "atelier://tag/tag-1".to_owned(),
                    tag_text: "training".to_owned(),
                    scope_ref: Some("atelier://collection/other-album".to_owned()),
                    note: "wrong album note".to_owned(),
                },
            ],
            match_modes: vec!["combined".to_owned()],
            fuzzy_score: 0.8,
            vector_score: 1.0,
        });

        let notes: Vec<&str> = result
            .tag_notes
            .iter()
            .map(|note| note.note.as_str())
            .collect();
        assert_eq!(notes, vec!["global training note", "matching album note"]);
    }

    #[test]
    fn local_ckc_field_parser_rejects_prefix_owner_ids_and_placeholders() {
        let invalid_prefix = "CHAR-ID-001X \u{2014} Character_ID: mira-demo";
        assert_eq!(sheet_field_value(invalid_prefix, "CHAR-ID-001"), None);
        let invalid_digit_prefix = "CHAR-ID-0010 \u{2014} Character_ID: mira-demo";
        assert_eq!(sheet_field_value(invalid_digit_prefix, "CHAR-ID-001"), None);
        let lowercase = "char-id-001 \u{2014} Character_ID: mira-demo";
        assert_eq!(sheet_field_value(lowercase, "CHAR-ID-001"), None);
        let placeholder = "CHAR-ID-001 \u{2014} Character_ID: <string>";
        assert_eq!(sheet_field_value(placeholder, "CHAR-ID-001"), None);

        let valid = "CHAR-ID-001\u{2014}Character_ID: mira-demo";
        assert_eq!(
            sheet_field_value(valid, "CHAR-ID-001").as_deref(),
            Some("mira-demo")
        );
    }

    fn test_document_row(
        character_internal_id: &str,
        character_ref: &str,
        document_id: &str,
        doc_type: &str,
        title: &str,
        body_raw_text: &str,
    ) -> AtelierCkcCharacterDocumentRow {
        AtelierCkcCharacterDocumentRow {
            document_id: document_id.to_owned(),
            document_ref: format!("atelier://document/{document_id}"),
            character_internal_id: character_internal_id.to_owned(),
            character_ref: character_ref.to_owned(),
            doc_type: doc_type.to_owned(),
            title: title.to_owned(),
            tags: vec![doc_type.to_owned()],
            current_version_id: format!("{document_id}-v1"),
            current_version_seq: 1,
            current_version: Some(AtelierCkcDocumentVersionRow {
                version_id: format!("{document_id}-v1"),
                document_id: document_id.to_owned(),
                document_ref: format!("atelier://document/{document_id}"),
                version_seq: 1,
                title: title.to_owned(),
                body_raw_text: body_raw_text.to_owned(),
                tags: vec![doc_type.to_owned()],
                author: "mt012-test".to_owned(),
            }),
            story_cards: Vec::new(),
            story_beats: Vec::new(),
        }
    }

    fn test_sheet_artifact_link_row(
        link_id: &str,
        character_internal_id: &str,
        sheet_version_id: &str,
    ) -> AtelierCkcSheetArtifactLinkRow {
        AtelierCkcSheetArtifactLinkRow {
            link_id: link_id.to_owned(),
            character_internal_id: character_internal_id.to_owned(),
            character_ref: format!("atelier://character/{character_internal_id}"),
            sheet_version_id: sheet_version_id.to_owned(),
            sheet_version_ref: format!(
                "atelier://sheet/{character_internal_id}/{sheet_version_id}"
            ),
            typed_ref: format!("atelier://sheet-artifact/{link_id}"),
            artifact_kind: "comfy_render".to_owned(),
            artifact_ref: "artifact://atelier/comfy/render/mira-async.png".to_owned(),
            manifest_ref: Some("receipt://atelier/comfy/mira-async".to_owned()),
            source_ref: Some("comfy://workflow-run/mira-async".to_owned()),
            label: Some("Mira async render".to_owned()),
            reuse_role: Some("cui_identity_reference".to_owned()),
            linked_by: "ckc-artifact-agent".to_owned(),
            metadata: serde_json::json!({ "source": "unit-test" }),
        }
    }

    #[test]
    fn ckc_sheet_artifact_result_updates_origin_sheet_not_current_selection() {
        let mut state = AtelierPanelState::default();
        state.ckc_selected_index = 1;
        state.ckc_selected_sheet_artifact_link_id = None;
        state.ckc_sheet_artifact_reuse_ref.clear();
        let mira_character_id = state.ckc_characters[0].character_internal_id.clone();
        let mira_sheet_id = state.ckc_characters[0]
            .sheet_version_id
            .clone()
            .expect("seeded Mira sheet id");
        let aria_initial_link_count = state.ckc_characters[1].sheet_artifact_links.len();
        let link_id = "018f7848-1111-7000-9000-00000000e777";

        let hidden_outcome = apply_ckc_sheet_artifact_link_rows_to_state(
            &mut state,
            &mira_sheet_id,
            vec![test_sheet_artifact_link_row(
                link_id,
                &mira_character_id,
                &mira_sheet_id,
            )],
        );

        assert_eq!(hidden_outcome.count, 1);
        assert!(hidden_outcome.target_found);
        assert!(!hidden_outcome.current_selection_owns_target);
        assert_eq!(state.ckc_selected_index, 1);
        assert_eq!(
            state.ckc_characters[0].sheet_artifact_links[0].link_id,
            link_id
        );
        assert_eq!(
            state.ckc_characters[1].sheet_artifact_links.len(),
            aria_initial_link_count,
            "late Mira rows must not attach to the currently selected Aria sheet"
        );
        assert!(
            state.ckc_selected_sheet_artifact_link_id.is_none(),
            "hidden sheet result must not mutate selected visible artifact"
        );
        assert!(
            state.ckc_sheet_artifact_reuse_ref.is_empty(),
            "hidden sheet result must not expose a Mira reuse ref while Aria is selected"
        );

        state.ckc_selected_index = 0;
        let visible_outcome = apply_ckc_sheet_artifact_link_rows_to_state(
            &mut state,
            &mira_sheet_id,
            vec![test_sheet_artifact_link_row(
                link_id,
                &mira_character_id,
                &mira_sheet_id,
            )],
        );
        assert!(visible_outcome.current_selection_owns_target);
        assert_eq!(
            state.ckc_selected_sheet_artifact_link_id.as_deref(),
            Some(link_id)
        );
        assert!(
            state.ckc_sheet_artifact_reuse_ref.contains(link_id),
            "visible Mira result should expose the reusable typed ref"
        );
    }

    #[test]
    fn ckc_backend_document_create_replaces_pending_story_and_moodboard_docs() {
        let mut characters = seeded_ckc_characters();
        let character = characters.first_mut().expect("seeded character");
        let character_internal_id = character.character_internal_id.clone();
        let character_ref = character.character_ref.clone();
        let display_name = character.display_name.clone();
        character.story_documents = vec![pending_ckc_story_document(
            &character_internal_id,
            &display_name,
        )];
        character.moodboard_documents = vec![pending_ckc_moodboard_document(
            &character_internal_id,
            &display_name,
        )];

        let story_status = apply_ckc_character_document_row(
            &mut characters,
            test_document_row(
                &character_internal_id,
                &character_ref,
                "018f7848-1111-7000-9000-00000000c777",
                "story",
                "Real backend story",
                "persisted story text",
            ),
        )
        .expect("story document applied");
        let moodboard_status = apply_ckc_character_document_row(
            &mut characters,
            test_document_row(
                &character_internal_id,
                &character_ref,
                "018f7848-1111-7000-9000-00000000d777",
                "moodboard",
                "Real backend moodboard",
                "persisted moodboard note",
            ),
        )
        .expect("moodboard document applied");

        let character = characters.first().expect("seeded character");
        assert_eq!(character.story_documents.len(), 1);
        assert_eq!(
            character.story_documents[0].document_id,
            "018f7848-1111-7000-9000-00000000c777"
        );
        assert_eq!(character.story_documents[0].title, "Real backend story");
        assert_eq!(character.moodboard_documents.len(), 1);
        assert_eq!(
            character.moodboard_documents[0].document_id,
            "018f7848-1111-7000-9000-00000000d777"
        );
        assert_eq!(
            character.moodboard_documents[0].title,
            "Real backend moodboard"
        );
        assert!(story_status.contains("018f7848-1111-7000-9000-00000000c777"));
        assert!(moodboard_status.contains("018f7848-1111-7000-9000-00000000d777"));
    }

    #[test]
    fn ckc_moodboard_snapshot_projection_loads_native_canvas_cards() {
        let layer_id = Uuid::new_v4().to_string();
        let text_id = Uuid::new_v4().to_string();
        let image_id = Uuid::new_v4().to_string();
        let connector_id = Uuid::new_v4().to_string();
        let moodboard_id = Uuid::new_v4().to_string();
        let asset_id = Uuid::new_v4().to_string();
        let history_id = Uuid::new_v4().to_string();
        let raw = serde_json::json!({
            "schema_id": "hsk.atelier.moodboard@1",
            "schema_version": 1,
            "moodboard_id": moodboard_id,
            "name": "Projection proof board",
            "description": "projection proof",
            "canvas": {
                "width": 1600.0,
                "height": 1000.0,
                "background_color": "#101418"
            },
            "layers": [{
                "layer_id": layer_id,
                "name": "Reference layer",
                "order": 2,
                "visible": true,
                "locked": false,
                "opacity": 1.0,
                "parent_layer_id": null
            }],
            "images": [{
                "element_id": image_id,
                "layer_id": layer_id,
                "asset_id": asset_id,
                "source": "hero-reference.png",
                "url": null,
                "position": { "x": 320.0, "y": 140.0 },
                "size": { "width": 240.0, "height": 180.0 },
                "rotation": 0.0,
                "opacity": 1.0,
                "flags": {}
            }],
            "text": [{
                "element_id": text_id,
                "layer_id": layer_id,
                "content": "Character visual continuity note",
                "font": "Inter",
                "font_size": 18.0,
                "color": "#f4f7fb",
                "position": { "x": 80.0, "y": 80.0 },
                "rotation": 0.0,
                "flags": {}
            }],
            "shapes": [],
            "connectors": [{
                "connector_id": connector_id,
                "layer_id": layer_id,
                "from_element_id": text_id,
                "to_element_id": image_id,
                "points": [],
                "style": {}
            }],
            "folders": [],
            "guides": [],
            "flags": {
                "locked": false,
                "archived": false,
                "operator_reviewed": false
            },
            "style": {
                "dominant_colors": ["#101418"],
                "mood_keywords": ["continuity"],
                "style_description": "native moodboard projection",
                "suggested_presets": []
            },
            "history": [{
                "history_id": history_id,
                "at": "2026-06-29T00:00:00Z",
                "actor": "mt012-test",
                "operation": "created",
                "summary": "Projection proof"
            }]
        });
        let projection = ckc_moodboard_snapshot_to_canvas_projection(&raw.to_string())
            .expect("snapshot projects");
        assert_eq!(projection.placements.len(), 2);
        assert_eq!(projection.visual_edges.len(), 1);
        assert_eq!(
            projection.section_labels.get(&layer_id).map(String::as_str),
            Some("Reference layer")
        );

        let mut board = LoomCanvasBoard::new("ws-test", "canvas-1");
        projection.apply_to_board(&mut board, "atelier://moodboard/snap-1");
        assert_eq!(board.placements.len(), 2);
        assert_eq!(board.visual_edges.len(), 1);
        assert!(
            board.placements.iter().any(|placement| {
                placement.live_content_type.as_deref() == Some("moodboard_text")
                    && placement
                        .live_body
                        .as_deref()
                        .unwrap_or_default()
                        .contains("Character visual continuity note")
            }),
            "projection must load moodboard text as a native canvas text card"
        );
        assert!(board
            .status
            .contains("CKC moodboard snapshot loaded: atelier://moodboard/snap-1"));
    }

    #[test]
    fn local_ckc_export_matches_backend_format_shapes() {
        let mut character = seeded_ckc_characters()
            .into_iter()
            .next()
            .expect("seeded character");
        character.sheet_editor_text.push_str(
            "\nCHAR-SEX-001\u{2014}Sex_Model: private-field\nCHAR-ID-001X \u{2014} Character_ID: wrong",
        );

        let json = local_export_ckc_sheet(&character, "json");
        assert_eq!(json.format, "json");
        let json_value: serde_json::Value =
            serde_json::from_str(&json.content).expect("json export envelope");
        assert_eq!(
            json_value["raw_text"].as_str(),
            Some(character.sheet_editor_text.as_str())
        );
        assert_eq!(
            json_value["sheet_version_ref"].as_str(),
            character.sheet_version_ref.as_deref()
        );
        assert_eq!(
            json.sheet_version_ref,
            character.sheet_version_ref.clone().unwrap()
        );

        let safe_txt = local_export_ckc_sheet(&character, "safe-txt");
        assert_eq!(safe_txt.format, "safe-txt");
        assert!(safe_txt.content.contains("CHAR-ID-001"));
        assert!(!safe_txt.content.contains("CHAR-SEX-001"));
        assert!(!safe_txt.content.contains("CHAR-ID-001X"));

        let safe_json = local_export_ckc_sheet(&character, "safe-json");
        let safe_json_value: serde_json::Value =
            serde_json::from_str(&safe_json.content).expect("safe json export envelope");
        let safe_raw = safe_json_value["raw_text"]
            .as_str()
            .expect("safe json raw_text");
        assert!(safe_raw.contains("CHAR-ID-001"));
        assert!(!safe_raw.contains("CHAR-SEX-001"));
        assert!(!safe_raw.contains("CHAR-ID-001X"));
        assert_eq!(safe_json.character_ref, character.character_ref);
    }

    #[test]
    fn posekit_add_marker_validation_allows_source_backed_or_remove_then_add() {
        let mut state = AtelierPanelState::default();
        state.pose_marker_family = "face".to_owned();
        state.pose_marker_index = 12;
        state.pose_marker_x = 321.0;
        state.pose_marker_y = 222.0;
        state.pose_marker_confidence = 0.87;

        let blocked = posekit_validate_marker_edit(&state, "add")
            .expect_err("synthetic non-empty marker slot must be protected");
        assert!(blocked.contains("overwrite existing face[12]"));

        let remove = posekit_validate_marker_edit(&state, "remove").expect("remove stages");
        state.pose_marker_edits.push(remove);
        let add_after_remove =
            posekit_validate_marker_edit(&state, "add").expect("remove then add is a safe slot");
        assert_eq!(add_after_remove.action, "add");
        assert_eq!(add_after_remove.family, "face");
        assert_eq!(add_after_remove.index, 12);

        let mut source_backed = AtelierPanelState::default();
        source_backed.pose_rig_id = Uuid::new_v4().to_string();
        source_backed.pose_marker_family = "face".to_owned();
        source_backed.pose_marker_index = 12;
        source_backed.pose_marker_x = 321.0;
        source_backed.pose_marker_y = 222.0;
        source_backed.pose_marker_confidence = 0.87;
        assert!(
            posekit_validate_marker_edit(&source_backed, "add").is_ok(),
            "stored rigs let the backend validate the real source slot instead of synthetic preview data"
        );

        posekit_stage_marker_edit(&mut source_backed, "add");
        assert_eq!(source_backed.pose_marker_edits.len(), 1);
        posekit_stage_marker_edit(&mut source_backed, "add");
        assert_eq!(
            source_backed.pose_marker_edits.len(),
            1,
            "backend source validation only applies to the initial unknown source slot"
        );
        assert!(
            source_backed
                .pose_marker_status
                .contains("overwrite existing face[12]"),
            "unexpected marker status: {}",
            source_backed.pose_marker_status
        );

        let mut set_then_add = AtelierPanelState::default();
        set_then_add.pose_rig_id = Uuid::new_v4().to_string();
        set_then_add.pose_marker_family = "face".to_owned();
        set_then_add.pose_marker_index = 12;
        set_then_add.pose_marker_x = 321.0;
        set_then_add.pose_marker_y = 222.0;
        set_then_add.pose_marker_confidence = 0.87;
        posekit_stage_marker_edit(&mut set_then_add, "set");
        posekit_stage_marker_edit(&mut set_then_add, "add");
        assert_eq!(
            set_then_add.pose_marker_edits.len(),
            1,
            "a local set makes a later add provably invalid even when a backend rig exists"
        );

        let mut stale_recovery = AtelierPanelState::default();
        stale_recovery.pose_rig_id = Uuid::new_v4().to_string();
        stale_recovery
            .pose_marker_edits
            .push(PosekitMarkerEditRecord {
                family: "face".to_owned(),
                index: 12,
                action: "set".to_owned(),
                x: Some(321.0),
                y: Some(222.0),
                confidence: Some(0.87),
            });
        stale_recovery
            .pose_marker_edits
            .push(PosekitMarkerEditRecord {
                family: "face".to_owned(),
                index: 12,
                action: "add".to_owned(),
                x: Some(322.0),
                y: Some(223.0),
                confidence: Some(0.86),
            });
        let err = posekit_validate_staged_marker_edits_for_export(&stale_recovery, true)
            .expect_err("state recovery must reject locally impossible add sequences");
        assert!(
            err.contains("empty local slot"),
            "unexpected export validation error: {err}"
        );
    }

    #[test]
    fn posekit_local_export_rejects_staged_edit_for_disabled_layer() {
        let mut state = AtelierPanelState::default();
        state.pose_marker_family = "face".to_owned();
        state.pose_marker_index = 12;
        state.pose_marker_x = 321.0;
        state.pose_marker_y = 222.0;
        state.pose_marker_confidence = 0.87;
        posekit_stage_marker_edit(&mut state, "set");
        assert_eq!(state.pose_marker_edits.len(), 1);

        state.pose_face = false;

        let err = posekit_export_snapshot(&state)
            .expect_err("local export must reject edits for layers disabled after staging");
        assert!(
            err.contains("disabled marker layer"),
            "unexpected export error: {err}"
        );
    }

    #[test]
    fn posekit_local_export_rejects_backend_deferred_add_without_backend() {
        let mut state = AtelierPanelState::default();
        state.pose_rig_id = Uuid::new_v4().to_string();
        state.pose_marker_family = "face".to_owned();
        state.pose_marker_index = 12;
        state.pose_marker_x = 321.0;
        state.pose_marker_y = 222.0;
        state.pose_marker_confidence = 0.87;
        posekit_stage_marker_edit(&mut state, "add");
        assert_eq!(state.pose_marker_edits.len(), 1);
        assert!(
            posekit_validate_staged_marker_edits_for_export(&state, true).is_ok(),
            "backend export may defer add-slot validation to the stored rig source"
        );

        let err = posekit_export_snapshot(&state)
            .expect_err("local export has no backend source slot and must not fake the add");
        assert!(
            err.contains("backend source validation"),
            "unexpected export error: {err}"
        );
    }

    #[test]
    fn posekit_layer_toggle_warns_about_stale_staged_edits() {
        let mut state = AtelierPanelState::default();
        posekit_stage_marker_edit(&mut state, "set");
        assert_eq!(state.pose_marker_edits.len(), 1);

        state.pose_face = false;
        posekit_warn_for_disabled_staged_marker_edits(&mut state);

        assert!(
            state.pose_marker_status.contains("disabled face layer"),
            "unexpected marker status: {}",
            state.pose_marker_status
        );
    }

    #[test]
    fn posekit_backend_export_failure_preserves_previous_preview_snapshot() {
        let panel = AtelierPanel::new(
            empty_side_panel(),
            Arc::new(Mutex::new(LoomCanvasBoard::new("ws-test", "canvas-1"))),
            Arc::new(Mutex::new(Vec::<CanvasEvent>::new())),
        );
        let previous_hash = {
            let mut state = panel.state.lock().expect("panel state");
            let snapshot = posekit_export_snapshot(&state).expect("initial local preview snapshot");
            let previous_hash = snapshot.content_hash.clone();
            state.pose_last_export = Some(snapshot);
            state.pose_export_pending = true;
            state.pose_active_export_request = Some(77);
            previous_hash
        };
        *panel.pose_export_cell.lock().expect("pose export cell") =
            Some((77, Err("backend rejected marker add".to_owned())));

        panel.drain_posekit_export_backend();

        let state = panel.state.lock().expect("panel state");
        assert!(!state.pose_export_pending);
        assert_eq!(state.pose_active_export_request, None);
        assert!(state
            .pose_export_status
            .contains("backend rejected marker add"));
        assert_eq!(
            state
                .pose_last_export
                .as_ref()
                .map(|snapshot| snapshot.content_hash.as_str()),
            Some(previous_hash.as_str()),
            "backend export rejection must not clear the last known-good OpenPose preview"
        );
    }

    #[test]
    fn local_posekit_framing_rejects_backend_invalid_off_canvas_points() {
        let mut body_keypoints = zero_keypoints(POSEKIT_BODY_KEYPOINT_COUNT);
        body_keypoints[0] = 700.0;
        body_keypoints[1] = 384.0;
        body_keypoints[2] = 1.0;
        let mut openpose = serde_json::json!({
            "people": [{
                "pose_keypoints_2d": body_keypoints,
                "face_keypoints_2d": zero_keypoints(POSEKIT_FACE_KEYPOINT_COUNT),
                "hand_left_keypoints_2d": zero_keypoints(POSEKIT_HAND_KEYPOINT_COUNT),
                "hand_right_keypoints_2d": zero_keypoints(POSEKIT_HAND_KEYPOINT_COUNT),
            }],
            "pose_state": {},
        });
        let framing = serde_json::json!({
            "preset": "custom",
            "lens_mm": 120,
            "padding_top_px": 0,
            "padding_right_px": 0,
            "padding_bottom_px": 0,
            "padding_left_px": 0,
        });

        posekit_apply_framing_to_openpose(&mut openpose, &framing);

        let framed_x = openpose["people"][0]["pose_keypoints_2d"][0]
            .as_f64()
            .expect("framed x");
        assert!(
            framed_x > POSEKIT_EXPORT_WIDTH as f64,
            "local preview must not clamp framing points that backend validation would reject"
        );
        let err = posekit_validate_local_openpose_export(&openpose, true)
            .expect_err("local preview must reject the same off-canvas visible point as backend");
        assert!(
            err.contains("outside the export canvas"),
            "unexpected local validation error: {err}"
        );
    }

    #[test]
    fn local_posekit_snapshot_rejects_generated_points_backend_would_reject() {
        let mut state = AtelierPanelState::default();
        state.pose_zoom = 2.2;
        state.pose_framing_lens_mm = 120;

        let err = posekit_export_snapshot(&state)
            .expect_err("local snapshot must not store backend-invalid OpenPose geometry");
        assert!(
            err.contains("outside the export canvas"),
            "unexpected local snapshot validation error: {err}"
        );
    }

    #[test]
    fn live_ckc_panel_starts_without_seeded_demo_refs() {
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        let panel = AtelierPanel::with_client(
            empty_side_panel(),
            Arc::new(Mutex::new(LoomCanvasBoard::new("ws-test", "canvas-1"))),
            Arc::new(Mutex::new(Vec::<CanvasEvent>::new())),
            Some(AtelierClient::new(
                "http://127.0.0.1:65535",
                runtime.handle().clone(),
            )),
        );
        let state = panel.state.lock().expect("panel state");
        assert!(state.ckc_characters.is_empty());
        assert!(state.ckc_search_results.is_empty());
        assert_eq!(
            state.ckc_search_status,
            "Waiting for live CKC database load"
        );
        assert!(state.ckc_tag_note_scope_ref.is_empty());
    }
}
