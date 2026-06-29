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
    AtelierCkcCreateCell, AtelierCkcMediaAlbumRow, AtelierCkcMediaMemberRow,
    AtelierCkcMediaNotesCell, AtelierCkcMediaNotesTagsRow, AtelierClient, AtelierSheetVersionRow,
    ATELIER_CKC_ACTOR_ID,
};
use crate::editor_pane_factories::SharedPalette;
use crate::graph::canvas_board::{CanvasEvent, LoomCanvasBoard};
use crate::interop::{AtelierItemKind, AtelierRef};
use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::theme::HsPalette;
use uuid::Uuid;

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
pub const ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID: &str = "atelier-ckc-linked-media-list";
pub const ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID: &str = "atelier-ckc-media-notes-editor";
pub const ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID: &str = "atelier-ckc-media-tags-editor";
pub const ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID: &str = "atelier-ckc-media-save";
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
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
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

    fn media_location(&self, asset_id: &str) -> Option<(usize, usize)> {
        self.media_albums
            .iter()
            .enumerate()
            .find_map(|(album_idx, album)| {
                album
                    .members
                    .iter()
                    .position(|member| member.asset_id == asset_id)
                    .map(|member_idx| (album_idx, member_idx))
            })
    }

    fn selected_or_first_media_location(
        &self,
        selected_asset_id: Option<&str>,
    ) -> Option<(usize, usize)> {
        selected_asset_id
            .and_then(|asset_id| self.media_location(asset_id))
            .or_else(|| self.first_media_location())
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
        self.source_path_ref = row.source_path_ref.clone();
        self.source_url_ref = row.source_url_ref.clone();
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
    ckc_media_save_pending: bool,
    ckc_selected_media_asset_id: Option<String>,
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
        Self {
            active_tab: AtelierPanelTab::CastkitCodex,
            ckc_characters: seeded_ckc_characters(),
            ckc_selected_index: 0,
            ckc_new_display_name: "New character".to_owned(),
            ckc_backend_loaded: false,
            ckc_load_requested: false,
            ckc_loading: false,
            ckc_create_pending: false,
            ckc_append_pending: false,
            ckc_media_save_pending: false,
            ckc_selected_media_asset_id: None,
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
            sheet_editor_text: "name: Mira Demo\nrole: reusable character/avatar\npipelines: ComfyUI, Unreal, Blender\nnotes: seed CKC sheet for Argus and model workflow proof".to_owned(),
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
                ),
                seeded_ckc_media_album(
                    "018f7848-1111-7000-9000-00000000a003",
                    "Mira expression set",
                    "018f7848-1111-7000-9000-00000000b003",
                    "mira-expression-002.png",
                    "atelier://folder/mira-expression-set",
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
            sheet_editor_text: "name: Aria Demo\nrole: production avatar reference\npipelines: CKC albums, Posekit, ComfyUI\nnotes: second selectable sheet proves CKC is a database surface".to_owned(),
            sheet_version_ref: Some(
                "atelier://sheet/018f7848-1111-7000-9000-000000000002/018f7848-1111-7000-9000-000000000201".to_owned(),
            ),
            media_albums: vec![seeded_ckc_media_album(
                "018f7848-1111-7000-9000-00000000a002",
                "Aria pose references",
                "018f7848-1111-7000-9000-00000000b002",
                "aria-pose-001.png",
                "atelier://folder/aria-pose-set",
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
            source_url_ref: None,
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

pub fn ckc_media_row_author_id(asset_id: &str) -> String {
    format!("atelier-ckc-media-{}", stable_author_id_suffix(asset_id))
}

pub fn ckc_folder_row_author_id(folder_ref: &str) -> String {
    format!("atelier-ckc-folder-{}", stable_author_id_suffix(folder_ref))
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
    ckc_media_notes_cell: AtelierCkcMediaNotesCell,
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
        Self {
            state: Mutex::new(AtelierPanelState::default()),
            side_panel,
            canvas_board,
            canvas_events,
            ckc_client,
            ckc_cell: Arc::new(Mutex::new(None)),
            ckc_create_cell: Arc::new(Mutex::new(None)),
            ckc_append_cell: Arc::new(Mutex::new(None)),
            ckc_media_notes_cell: Arc::new(Mutex::new(None)),
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
                        state.ckc_error = None;
                    }
                    Err(err) => {
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
                        let mut applied = false;
                        for character in &mut state.ckc_characters {
                            for album in &mut character.media_albums {
                                if let Some(member) =
                                    album.members.iter_mut().find(|m| m.asset_id == asset_id)
                                {
                                    member.apply_notes_tags(&row);
                                    applied = true;
                                    break;
                                }
                            }
                            if applied {
                                break;
                            }
                        }
                        state.ckc_error = if applied {
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
                if state.ckc_loading {
                    ui.label(egui::RichText::new("Loading CKC database...").color(palette.text_subtle));
                }
                if let Some(error) = &state.ckc_error {
                    ui.label(egui::RichText::new(format!("CKC backend: {error}")).color(palette.error_text));
                }
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
                            state.ckc_selected_index = idx;
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
                                        ATELIER_CKC_ACTOR_ID,
                                        self.ckc_create_cell.clone(),
                                    );
                                }
                            } else {
                                let character_internal_id = Uuid::new_v4().to_string();
                                state.ckc_characters.push(CkcCharacterRecord {
                                    public_id,
                                    display_name: display_name.clone(),
                                    character_internal_id: character_internal_id.clone(),
                                    character_ref: format!("atelier://character/{character_internal_id}"),
                                    sheet_version_id: None,
                                    parent_sheet_version_id: None,
                                    sheet_seq: 0,
                                    sheet_editor_text: format!(
                                        "name: {display_name}\nrole: reusable character/avatar\npipelines: ComfyUI, Unreal, Blender\nnotes: "
                                    ),
                                    sheet_version_ref: None,
                                    media_albums: Vec::new(),
                                });
                                state.ckc_selected_index = state.ckc_characters.len() - 1;
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
                let selected_media_asset_id = state.ckc_selected_media_asset_id.clone();
                let mut pending_media_save = None;
                let mut pending_media_selection = None;
                if let Some(character) = state.ckc_characters.get_mut(selected_index) {
                    let (save, selection) = self.show_ckc_linked_media(
                        ui,
                        palette,
                        character,
                        media_save_pending,
                        selected_media_asset_id.as_deref(),
                    );
                    pending_media_save = save;
                    pending_media_selection = selection;
                }
                if let Some(asset_id) = pending_media_selection {
                    state.ckc_selected_media_asset_id = Some(asset_id);
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
                            request.source_path_ref.as_deref(),
                            request.source_url_ref.as_deref(),
                            ATELIER_CKC_ACTOR_ID,
                            self.ckc_media_notes_cell.clone(),
                        );
                    }
                }
                ui.separator();
                if let Ok(mut side_panel) = self.side_panel.lock() {
                    side_panel.show(ui, palette);
                }
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
                            ATELIER_CKC_ACTOR_ID,
                            self.ckc_append_cell.clone(),
                        );
                    }
                }
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

    fn show_ckc_linked_media(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        character: &mut CkcCharacterRecord,
        media_save_pending: bool,
        selected_media_asset_id: Option<&str>,
    ) -> (Option<CkcMediaSaveRequest>, Option<String>) {
        ui.heading(egui::RichText::new("Linked media").color(palette.text));
        let resolved_selection = character
            .selected_or_first_media_location(selected_media_asset_id)
            .map(|(album_idx, member_idx)| {
                character.media_albums[album_idx].members[member_idx]
                    .asset_id
                    .clone()
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
                    let album_row = ui.label(format!(
                        "Album: {} ({} items)",
                        album.name, album.member_count
                    ));
                    emit_node(
                        ui.ctx(),
                        album_row.id,
                        accesskit::Role::ListItem,
                        &ckc_media_album_row_author_id(&album.collection_id),
                        &format!("{} {}", album_ref.ref_kind(), album.collection_ref),
                        false,
                    );
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
                        ui.label(
                            egui::RichText::new(format!(
                                "showing {} of {}; next offset {}",
                                album.members.len(),
                                album.member_count,
                                next_offset
                            ))
                            .color(palette.text_subtle),
                        );
                    }
                    for member in &album.members {
                        let selected =
                            resolved_selection.as_deref() == Some(member.asset_id.as_str());
                        let media_row = ui.add(egui::Button::selectable(
                            selected,
                            format!(
                                "{} [{}]",
                                member.display_label,
                                member.review_status.as_deref().unwrap_or("unreviewed")
                            ),
                        ));
                        emit_node(
                            ui.ctx(),
                            media_row.id,
                            accesskit::Role::ListItem,
                            &ckc_media_row_author_id(&member.asset_id),
                            &member.media_ref,
                            selected,
                        );
                        if media_row.clicked() {
                            pending_selection = Some(member.asset_id.clone());
                        }
                        if let Some(folder_ref) = &member.source_path_ref {
                            let folder = ui.label(format!("folder_ref: {folder_ref}"));
                            emit_node(
                                ui.ctx(),
                                folder.id,
                                accesskit::Role::ListItem,
                                &ckc_folder_row_author_id(folder_ref),
                                folder_ref,
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

        let selected_asset_id = pending_selection
            .as_deref()
            .or(resolved_selection.as_deref());
        let Some((album_idx, member_idx)) =
            character.selected_or_first_media_location(selected_asset_id)
        else {
            return (None, pending_selection);
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
                    source_path_ref: member.source_path_ref.clone(),
                    source_url_ref: member.source_url_ref.clone(),
                }),
                pending_selection,
            )
        } else {
            (None, pending_selection)
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
