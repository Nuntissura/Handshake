//! Native Atelier main panel.
//!
//! The shell-level Atelier module hosts sibling tool tabs inside one filling pane. CKC reuses the
//! existing Atelier intake/drag-source widget and canvas board; Posekit and Ingest expose stable,
//! nonblank native control surfaces so agents can address and inspect them before deeper parity work.

use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::atelier_side_panel::AtelierSidePanel;
use crate::backend_client::{
    AtelierCharacterRow, AtelierCkcAppendCell, AtelierCkcCell, AtelierCkcCharacterSheetRow,
    AtelierCkcCreateCell, AtelierCkcExportCell, AtelierCkcFieldSuggestionsCell,
    AtelierCkcImportCell, AtelierCkcMediaAlbumCreateCell, AtelierCkcMediaAlbumItemsCell,
    AtelierCkcMediaAlbumRow, AtelierCkcMediaMemberRow, AtelierCkcMediaNotesCell,
    AtelierCkcMediaNotesTagsRow, AtelierCkcSafeSubsetCell, AtelierCkcSearchCell,
    AtelierCkcSearchResponse, AtelierCkcSearchResultRow, AtelierCkcTagNoteCell,
    AtelierCkcTagNoteRow, AtelierCkcTemplateCell, AtelierClient, AtelierSheetExportRow,
    AtelierSheetFieldSuggestionRow, AtelierSheetVersionRow,
};
use crate::editor_pane_factories::SharedPalette;
use crate::graph::canvas_board::{CanvasEvent, LoomCanvasBoard};
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
pub const ATELIER_INGEST_PASS_AUTHOR_ID: &str = "atelier-ingest-pass";
pub const ATELIER_INGEST_REJECT_AUTHOR_ID: &str = "atelier-ingest-reject";
pub const ATELIER_INGEST_UNSURE_AUTHOR_ID: &str = "atelier-ingest-unsure";
pub const ATELIER_INGEST_BATCH_TAGS_AUTHOR_ID: &str = "atelier-ingest-batch-tags";

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
    media_albums: Vec<CkcMediaAlbumRecord>,
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
            media_albums,
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
            media_albums: media_albums
                .into_iter()
                .map(CkcMediaAlbumRecord::from_backend)
                .collect(),
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
            media_albums: Vec::new(),
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

#[derive(Debug)]
struct AtelierPanelState {
    active_tab: AtelierPanelTab,
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
    ingest_decision: IngestDecision,
    ingest_tag_buffer: String,
}

impl Default for AtelierPanelState {
    fn default() -> Self {
        let ckc_characters = seeded_ckc_characters();
        let ckc_search_results = seeded_ckc_search_results(&ckc_characters);
        Self {
            active_tab: AtelierPanelTab::CastkitCodex,
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
            ingest_decision: IngestDecision::Unsure,
            ingest_tag_buffer: "event, outfit, source".to_owned(),
        }
    }
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
            media_albums: vec![seeded_ckc_media_album(
                "018f7848-1111-7000-9000-00000000a002",
                "Aria pose references",
                "018f7848-1111-7000-9000-00000000b002",
                "aria-pose-001.png",
                "atelier://folder/aria-pose-set",
                "https://example.invalid/reference/aria-pose-set",
            )],
        },
    ]
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
    ckc_media_album_create_cell: AtelierCkcMediaAlbumCreateCell,
    ckc_media_album_items_cell: AtelierCkcMediaAlbumItemsCell,
    ckc_media_album_page_cell: AtelierCkcMediaAlbumItemsCell,
    ckc_media_notes_cell: AtelierCkcMediaNotesCell,
    ckc_search_cell: AtelierCkcSearchCell,
    ckc_tag_note_cell: AtelierCkcTagNoteCell,
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
            ckc_media_album_create_cell: Arc::new(Mutex::new(None)),
            ckc_media_album_items_cell: Arc::new(Mutex::new(None)),
            ckc_media_album_page_cell: Arc::new(Mutex::new(None)),
            ckc_media_notes_cell: Arc::new(Mutex::new(None)),
            ckc_search_cell: Arc::new(Mutex::new(None)),
            ckc_tag_note_cell: Arc::new(Mutex::new(None)),
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
        ui.horizontal(|ui| {
            let left_w = (ui.available_width() * 0.34).clamp(240.0, 380.0);
            ui.vertical(|ui| {
                ui.set_width(left_w);
                ui.heading(egui::RichText::new("Characters").color(palette.text));
                egui::ScrollArea::vertical()
                    .id_salt("atelier-ckc-left-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
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
                            let row = ui.add(egui::Button::selectable(
                                selected,
                                if character.sheet_seq > 0 {
                                    format!("{}  v{}", character.display_name, character.sheet_seq)
                                } else {
                                    format!("{}  no sheet", character.display_name)
                                },
                            ));
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
                                    media_albums: Vec::new(),
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
            });
            ui.separator();
            ui.vertical(|ui| {
                let append_pending = state.ckc_append_pending;
                let mut pending_append_request: Option<(String, String, Option<String>)> = None;
                let selected_index = state
                    .ckc_selected_index
                    .min(state.ckc_characters.len().saturating_sub(1));
                state.ckc_selected_index = selected_index;
                if let Some(character) = state.ckc_characters.get_mut(selected_index) {
                    let selected_response = ui
                        .vertical(|ui| {
                            ui.heading(
                                egui::RichText::new(&character.display_name).color(palette.text),
                            );
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
                            .desired_rows(11)
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
                            state.ckc_last_export = None;
                        }
                    }
                } else {
                    ui.label(egui::RichText::new("No CKC characters yet").color(palette.text_subtle));
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
                ui.separator();
                self.show_ckc_sheet_tools(ui, palette, &mut state, selected_index);
                ui.separator();
                ui.heading(egui::RichText::new("Moodboard").color(palette.text));
                ui.add_space(4.0);
                let mut event = None;
                if let Ok(mut board) = self.canvas_board.lock() {
                    event = board.show(ui, palette);
                    let drained = board.drain_knowledge_events();
                    if !drained.is_empty() {
                        if let Ok(mut q) = self.canvas_events.lock() {
                            q.extend(drained);
                        }
                    }
                }
                if let Some(ev) = event {
                    if let Ok(mut q) = self.canvas_events.lock() {
                        q.push(ev);
                    }
                }
            });
        });
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
        let Ok(mut state) = self.state.lock() else {
            return;
        };
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
        });
        let yaw_slider = ui.add(egui::Slider::new(&mut state.pose_yaw, -180.0..=180.0).text("Yaw"));
        emit_node(
            ui.ctx(),
            yaw_slider.id,
            accesskit::Role::Slider,
            ATELIER_POSE_YAW_SLIDER_AUTHOR_ID,
            "Yaw",
            false,
        );
        let pitch_slider =
            ui.add(egui::Slider::new(&mut state.pose_pitch, -45.0..=45.0).text("Pitch"));
        emit_node(
            ui.ctx(),
            pitch_slider.id,
            accesskit::Role::Slider,
            ATELIER_POSE_PITCH_SLIDER_AUTHOR_ID,
            "Pitch",
            false,
        );
        let zoom_slider = ui.add(egui::Slider::new(&mut state.pose_zoom, 0.4..=2.2).text("Zoom"));
        emit_node(
            ui.ctx(),
            zoom_slider.id,
            accesskit::Role::Slider,
            ATELIER_POSE_ZOOM_SLIDER_AUTHOR_ID,
            "Zoom",
            false,
        );
        ui.separator();
        ui.columns(2, |cols| {
            draw_pose_view(
                &mut cols[0],
                palette,
                "3D rig",
                state.pose_yaw,
                state.pose_pitch,
                state.pose_zoom,
                false,
            );
            draw_pose_view(
                &mut cols[1],
                palette,
                "OpenPose",
                state.pose_yaw,
                state.pose_pitch,
                state.pose_zoom,
                true,
            );
        });
    }

    fn show_ingest(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
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
                }
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Batch tags").color(palette.text));
            let tags = ui.text_edit_singleline(&mut state.ingest_tag_buffer);
            emit_node(
                ui.ctx(),
                tags.id,
                accesskit::Role::TextInput,
                ATELIER_INGEST_BATCH_TAGS_AUTHOR_ID,
                "Batch tags",
                false,
            );
        });
        ui.separator();
        egui::Grid::new("atelier-ingest-grid")
            .striped(true)
            .min_col_width(110.0)
            .show(ui, |ui| {
                ui.strong("Asset");
                ui.strong("Decision");
                ui.strong("Tags");
                ui.end_row();
                for name in ["frame_0001.png", "frame_0002.png", "contact_sheet_a.jpg"] {
                    ui.label(name);
                    ui.label(state.ingest_decision.label());
                    ui.label(&state.ingest_tag_buffer);
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
    yaw: f32,
    pitch: f32,
    zoom: f32,
    openpose: bool,
) {
    ui.label(egui::RichText::new(label).strong().color(palette.text));
    let height = 260.0;
    let width = ui.available_width().max(180.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
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

    painter.circle_stroke(
        center + egui::vec2(0.0, -58.0 * scale),
        head_r,
        egui::Stroke::new(2.0, color),
    );
    painter.line_segment(
        [
            center + egui::vec2(0.0, -36.0 * scale),
            center + egui::vec2(0.0, torso * 0.45),
        ],
        egui::Stroke::new(2.0, color),
    );
    painter.line_segment(
        [
            center + egui::vec2(-42.0 * scale, -8.0 * scale),
            center + egui::vec2(42.0 * scale, -8.0 * scale),
        ],
        egui::Stroke::new(2.0, color),
    );
    painter.line_segment(
        [
            center + egui::vec2(-28.0 * scale, torso * 0.95),
            center + egui::vec2(0.0, torso * 0.45),
        ],
        egui::Stroke::new(2.0, color),
    );
    painter.line_segment(
        [
            center + egui::vec2(28.0 * scale, torso * 0.95),
            center + egui::vec2(0.0, torso * 0.45),
        ],
        egui::Stroke::new(2.0, color),
    );

    if openpose {
        for point in [
            center + egui::vec2(0.0, -58.0 * scale),
            center + egui::vec2(-10.0 * scale, -62.0 * scale),
            center + egui::vec2(10.0 * scale, -62.0 * scale),
            center + egui::vec2(0.0, -50.0 * scale),
            center + egui::vec2(-42.0 * scale, -8.0 * scale),
            center + egui::vec2(42.0 * scale, -8.0 * scale),
            center + egui::vec2(0.0, torso * 0.45),
        ] {
            painter.circle_filled(point, 3.5, muted);
        }
    }
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
    use crate::backend_client::{AtelierBatchRow, AtelierItemRow};

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
