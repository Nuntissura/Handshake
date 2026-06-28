//! Native Atelier main panel.
//!
//! The shell-level Atelier module hosts sibling tool tabs inside one filling pane. CKC reuses the
//! existing Atelier intake/drag-source widget and canvas board; Posekit and Ingest expose stable,
//! nonblank native control surfaces so agents can address and inspect them before deeper parity work.

use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::atelier_side_panel::AtelierSidePanel;
use crate::editor_pane_factories::SharedPalette;
use crate::graph::canvas_board::{CanvasEvent, LoomCanvasBoard};
use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::theme::HsPalette;

pub const ATELIER_PANEL_AUTHOR_ID: &str = "atelier-main-panel";
pub const ATELIER_TABLIST_AUTHOR_ID: &str = "atelier-tab-list";
pub const ATELIER_TAB_CKC_AUTHOR_ID: &str = "atelier-tab-ckc";
pub const ATELIER_TAB_POSEKIT_AUTHOR_ID: &str = "atelier-tab-posekit";
pub const ATELIER_TAB_INGEST_AUTHOR_ID: &str = "atelier-tab-ingest";
pub const ATELIER_CONTENT_CKC_AUTHOR_ID: &str = "atelier-content-ckc";
pub const ATELIER_CONTENT_POSEKIT_AUTHOR_ID: &str = "atelier-content-posekit";
pub const ATELIER_CONTENT_INGEST_AUTHOR_ID: &str = "atelier-content-ingest";
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

#[derive(Debug)]
struct AtelierPanelState {
    active_tab: AtelierPanelTab,
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
}

impl AtelierPanel {
    pub fn new(
        side_panel: Arc<Mutex<AtelierSidePanel>>,
        canvas_board: Arc<Mutex<LoomCanvasBoard>>,
        canvas_events: Arc<Mutex<Vec<CanvasEvent>>>,
    ) -> Self {
        Self {
            state: Mutex::new(AtelierPanelState::default()),
            side_panel,
            canvas_board,
            canvas_events,
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

    fn show_ckc(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        ui.horizontal(|ui| {
            let left_w = (ui.available_width() * 0.36).clamp(220.0, 360.0);
            ui.vertical(|ui| {
                ui.set_width(left_w);
                if let Ok(mut side_panel) = self.side_panel.lock() {
                    side_panel.show(ui, palette);
                }
            });
            ui.separator();
            ui.vertical(|ui| {
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
        Self {
            panel: AtelierPanel::new(side_panel, canvas_board, canvas_events),
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
            accesskit::Role::Tab | accesskit::Role::Button | accesskit::Role::CheckBox
        ) {
            node.add_action(accesskit::Action::Click);
        }
        if matches!(role, accesskit::Role::TextInput | accesskit::Role::Slider) {
            node.add_action(accesskit::Action::Focus);
        }
    });
}
