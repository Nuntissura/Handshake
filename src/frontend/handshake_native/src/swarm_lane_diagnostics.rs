//! Native Swarm lane diagnostics pane (WP-1 MT-008).
//!
//! The pane renders Dexterity model-lane runs, lanes, messages, payload refs,
//! promotion state, recovery state, and diagnostic tier posture. Production
//! data is fetched from the `handshake_core` PostgreSQL-backed diagnostics
//! route; tests may inject a projection into the same renderer.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use egui::accesskit;
use serde::{Deserialize, Serialize};

use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};

pub const SURFACE_CONTRACT_ID: &str = "native_swarm_lane_diagnostics";
pub const PROJECTION_SCHEMA_ID: &str = "hsk.model_lane_diagnostics_projection@1";
pub const SURFACE_AUTHOR_ID: &str = "swarm-lane-diagnostics.surface";
pub const RUN_FILTER_AUTHOR_ID: &str = "swarm-lane-diagnostics.filter.run";
pub const LANE_FILTER_AUTHOR_ID: &str = "swarm-lane-diagnostics.filter.lane";
pub const MESSAGE_FILTER_AUTHOR_ID: &str = "swarm-lane-diagnostics.filter.message";
pub const REFRESH_AUTHOR_ID: &str = "swarm-lane-diagnostics.action.refresh";
pub const ERROR_AUTHOR_ID: &str = "swarm-lane-diagnostics.error";

pub type SwarmLaneDiagnosticsCell =
    Arc<Mutex<Option<Result<SwarmLaneDiagnosticsProjection, String>>>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneDiagnosticsProjection {
    pub schema_id: String,
    pub surface_contract_id: String,
    pub run: SwarmLaneDiagnosticsRun,
    pub lanes: Vec<SwarmLaneDiagnosticsLane>,
    pub messages: Vec<SwarmLaneDiagnosticsMessage>,
    pub diagnostic_tiers: Vec<SwarmLaneDiagnosticsTier>,
    pub mt_runtime_statuses: Vec<SwarmLaneDiagnosticsMtStatus>,
    pub active_lease_count: usize,
    pub reclaimable_lease_ids: Vec<String>,
    pub orphan_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneDiagnosticsRun {
    pub run_id: String,
    pub trace_id: String,
    pub run_span_id: String,
    #[serde(default)]
    pub coordinator_session_id: String,
    #[serde(default)]
    pub routing_policy: String,
    #[serde(default)]
    pub artifact_namespace: String,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    #[serde(default)]
    pub owner_session: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub flight_recorder_correlation_id: String,
    pub context_bundle_id: String,
    pub memory_pack_ref: String,
    pub memory_pack_hash: String,
    pub locus_ref: Option<String>,
    pub loom_ref: Option<String>,
    pub fems_ref: Option<String>,
    pub status: String,
    pub recovery_hint_ref: Option<String>,
    pub selected_model_id: Option<String>,
    #[serde(default)]
    pub candidate_model_ids: Vec<String>,
    #[serde(default)]
    pub budget_summary_ref: String,
    #[serde(default)]
    pub determinism_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneDiagnosticsLane {
    pub lane_id: String,
    pub kind: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub backend: String,
    pub status: String,
    pub recovery_state: String,
    pub model_id: Option<String>,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub model_session_id: String,
    #[serde(default)]
    pub adapter_id: String,
    pub provider_kind: String,
    pub runtime_binding: String,
    #[serde(default)]
    pub launch_authority: String,
    #[serde(default)]
    pub capability_token_ids: Vec<String>,
    pub effective_capability_snapshot_ref: Option<String>,
    pub capability_negotiation_ref: Option<String>,
    pub provider_feature_profile_ref: Option<String>,
    pub requested_execution_policy_ref: Option<String>,
    pub effective_execution_policy_ref: Option<String>,
    pub projection_plan_ref: Option<String>,
    pub consent_receipt_ref: Option<String>,
    #[serde(default)]
    pub tool_gate_decision_refs: Vec<String>,
    pub trace_id: String,
    pub lane_span_id: String,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub flight_recorder_correlation_id: String,
    pub last_activity_utc: Option<String>,
    pub message_count: usize,
    pub payload_error_count: usize,
    pub orphan_state: String,
    pub cancellation_ref: Option<String>,
    pub reclaim_policy_ref: Option<String>,
    pub terminal_status_mapping_ref: Option<String>,
    pub process_ownership_ref: Option<String>,
    pub no_os_process_reason_ref: Option<String>,
    pub last_runtime_status_ref: Option<String>,
    pub last_recovery_event_ref: Option<String>,
    pub failstate_code: Option<String>,
    pub startup_failure_ref: Option<String>,
    pub reason_ref: Option<String>,
    pub recovery_hint_ref: Option<String>,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    #[serde(default)]
    pub owner_session: String,
    pub locus_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneDiagnosticsMessage {
    pub message_id: String,
    pub from_lane_id: String,
    #[serde(default)]
    pub to_lane: String,
    pub routing_target_role: Option<String>,
    pub routing_target_session: Option<String>,
    pub routing_correlation_id: Option<String>,
    #[serde(default)]
    pub routing_requires_ack: bool,
    pub routing_ack_for: Option<String>,
    pub kind: String,
    pub authority: String,
    pub promotion_state: String,
    pub payload_ref: String,
    pub payload_sha256: String,
    pub artifact_ref: Option<String>,
    pub promotion_decision_id: Option<String>,
    pub promotion_gate_ref: Option<String>,
    pub promotion_receipt_ref: Option<String>,
    pub validator_verdict_ref: Option<String>,
    pub operator_decision_ref: Option<String>,
    pub promoted_artifact_sha256: Option<String>,
    pub promoted_artifact_version: Option<String>,
    #[serde(default)]
    pub tool_gate_decision_refs: Vec<String>,
    #[serde(default)]
    pub coordinator_session_id: String,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub task_board_id: Option<String>,
    #[serde(default)]
    pub owner_session: String,
    pub trace_id: String,
    pub message_span_id: String,
    pub parent_span_id: Option<String>,
    pub linked_span_contexts: Vec<String>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
    pub flight_recorder_correlation_id: String,
    pub locus_ref: Option<String>,
    pub loom_ref: Option<String>,
    pub fems_ref: Option<String>,
    pub proposal_ref: Option<String>,
    pub crdt_update_ref: Option<String>,
    pub crdt_base_snapshot_ref: Option<String>,
    pub crdt_state_vector: Option<String>,
    pub crdt_proposal_ref: Option<String>,
    pub crdt_stale_base_ref: Option<String>,
    pub payload_error: Option<String>,
    pub reason_ref: Option<String>,
    pub recovery_hint_ref: Option<String>,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneDiagnosticsTier {
    pub tier: String,
    pub state: String,
    pub reason: String,
    pub evidence_ref: String,
    pub follow_up_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmLaneDiagnosticsMtStatus {
    pub micro_task_id: String,
    pub status: String,
    pub proof_status_ref: Option<String>,
    pub hbr_status_ref: Option<String>,
    pub event_ledger_event_id: String,
    pub event_ledger_seq: i64,
}

pub trait SwarmLaneDiagnosticsTransport: Send + Sync {
    fn fetch_latest(&self, cell: SwarmLaneDiagnosticsCell);
    fn fetch_run(&self, run_id: &str, cell: SwarmLaneDiagnosticsCell);
}

#[derive(Default)]
struct DiagnosticsUiState {
    projection: Option<SwarmLaneDiagnosticsProjection>,
    run_filter: String,
    lane_filter: String,
    message_filter: String,
    selected_message_id: Option<String>,
    pending_fetch: bool,
    error: Option<String>,
}

pub struct SwarmLaneDiagnosticsPaneFactory {
    state: Arc<Mutex<DiagnosticsUiState>>,
    transport: Option<Arc<dyn SwarmLaneDiagnosticsTransport>>,
    delivery: SwarmLaneDiagnosticsCell,
}

impl SwarmLaneDiagnosticsPaneFactory {
    pub fn offline() -> Self {
        Self {
            state: Arc::new(Mutex::new(DiagnosticsUiState::default())),
            transport: None,
            delivery: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_projection(projection: SwarmLaneDiagnosticsProjection) -> Self {
        Self {
            state: Arc::new(Mutex::new(DiagnosticsUiState {
                projection: Some(projection),
                ..Default::default()
            })),
            transport: None,
            delivery: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_transport(transport: Arc<dyn SwarmLaneDiagnosticsTransport>) -> Self {
        Self {
            state: Arc::new(Mutex::new(DiagnosticsUiState::default())),
            transport: Some(transport),
            delivery: Arc::new(Mutex::new(None)),
        }
    }

    fn start_fetch(&self, run_id: Option<&str>) {
        let Some(transport) = &self.transport else {
            return;
        };
        if let Ok(mut state) = self.state.lock() {
            if state.pending_fetch {
                return;
            }
            state.pending_fetch = true;
            state.error = None;
        }
        match run_id {
            Some(id) if !id.trim().is_empty() => {
                transport.fetch_run(id.trim(), self.delivery.clone())
            }
            _ => transport.fetch_latest(self.delivery.clone()),
        }
    }

    fn drain_delivery(&self) {
        let delivered = self.delivery.lock().ok().and_then(|mut slot| slot.take());
        if let Some(result) = delivered {
            if let Ok(mut state) = self.state.lock() {
                state.pending_fetch = false;
                match result {
                    Ok(projection) => {
                        if let Err(error) = validate_projection_for_native_surface(&projection) {
                            state.error = Some(error);
                        } else {
                            state.run_filter = projection.run.run_id.clone();
                            state.projection = Some(projection);
                            state.error = None;
                        }
                    }
                    Err(error) => state.error = Some(error),
                }
            }
        }
    }
}

impl PaneFactory for SwarmLaneDiagnosticsPaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::SwarmLaneDiagnostics
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        self.drain_delivery();
        if self.transport.is_some()
            && self
                .state
                .lock()
                .ok()
                .is_some_and(|state| state.projection.is_none() && !state.pending_fetch)
        {
            self.start_fetch(ctx.record.content_id.as_deref());
        }

        let Ok(mut state) = self.state.lock() else {
            ui.label("Diagnostics state unavailable");
            return;
        };

        let surface_id = ctx.egui_id.with("swarm-lane-diagnostics-surface");
        ui.ctx().accesskit_node_builder(surface_id, |node| {
            node.set_role(accesskit::Role::Group);
            node.set_author_id(SURFACE_AUTHOR_ID.to_owned());
            node.set_label("Swarm Lane Diagnostics".to_owned());
        });

        let mut fetch_after_render: Option<String> = None;
        ui.vertical(|ui| {
            ui.heading("Swarm Lane Diagnostics");
            ui.horizontal(|ui| {
                ui.label("Run");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.run_filter)
                        .id_source(ctx.egui_id.with("swarm-lane-diagnostics-run-filter"))
                        .desired_width(260.0),
                );
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_author_id(RUN_FILTER_AUTHOR_ID.to_owned());
                    node.set_label("Run filter".to_owned());
                });
                let refresh = ui.button("Refresh");
                refresh.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Refresh")
                });
                ui.ctx().accesskit_node_builder(refresh.id, |node| {
                    node.set_role(accesskit::Role::Button);
                    node.add_action(accesskit::Action::Click);
                    node.set_author_id(REFRESH_AUTHOR_ID.to_owned());
                    node.set_label("Refresh".to_owned());
                });
                if refresh.clicked() {
                    fetch_after_render = Some(state.run_filter.clone());
                }
            });
            ui.horizontal(|ui| {
                ui.label("Lane");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.lane_filter)
                        .id_source(ctx.egui_id.with("swarm-lane-diagnostics-lane-filter"))
                        .desired_width(220.0),
                );
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_author_id(LANE_FILTER_AUTHOR_ID.to_owned());
                    node.set_label("Lane filter".to_owned());
                });
                ui.label("Message");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.message_filter)
                        .id_source(ctx.egui_id.with("swarm-lane-diagnostics-message-filter"))
                        .desired_width(220.0),
                );
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_author_id(MESSAGE_FILTER_AUTHOR_ID.to_owned());
                    node.set_label("Message filter".to_owned());
                });
            });

            if state.pending_fetch {
                ui.label("Loading Dexterity model-lane diagnostics...");
            }
            if let Some(error) = state.error.as_deref() {
                let resp = ui.colored_label(ui.visuals().error_fg_color, error);
                ui.ctx().accesskit_node_builder(resp.id, |node| {
                    node.set_author_id(ERROR_AUTHOR_ID.to_owned());
                    node.set_label(error.to_owned());
                });
            }

            let Some(projection) = state.projection.clone() else {
                ui.label("No Dexterity model-lane run loaded.");
                return;
            };

            render_projection(ui, ctx, &projection, &mut state);
        });
        drop(state);
        if let Some(run_id) = fetch_after_render {
            self.start_fetch(Some(&run_id));
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Pane
    }
}

fn render_projection(
    ui: &mut egui::Ui,
    ctx: &PaneRenderContext,
    projection: &SwarmLaneDiagnosticsProjection,
    state: &mut DiagnosticsUiState,
) {
    ui.separator();
    tagged_label(
        ui,
        ctx.egui_id.with("run-row"),
        &run_author_id(&projection.run.run_id),
        &format!(
            "run {} | status {} | routing {} | coordinator {} | lanes {} | messages {} | event {}#{} | trace {} | FR {}",
            projection.run.run_id,
            projection.run.status,
            projection.run.routing_policy,
            projection.run.coordinator_session_id,
            projection.lanes.len(),
            projection.messages.len(),
            projection.run.event_ledger_event_id,
            projection.run.event_ledger_seq,
            projection.run.trace_id,
            projection.run.flight_recorder_correlation_id
        ),
    );
    tagged_label(
        ui,
        ctx.egui_id.with("context-row"),
        "swarm-lane-diagnostics.context",
        &format!(
            "context {} | namespace {} | memory {} {} | Locus {} | Loom {} | FEMS {} | WP {} | MT {} | owner {}",
            projection.run.context_bundle_id,
            projection.run.artifact_namespace,
            projection.run.memory_pack_ref,
            projection.run.memory_pack_hash,
            projection.run.locus_ref.as_deref().unwrap_or("missing"),
            projection.run.loom_ref.as_deref().unwrap_or("missing"),
            projection.run.fems_ref.as_deref().unwrap_or("missing"),
            projection.run.work_packet_id.as_deref().unwrap_or("missing"),
            projection.run.micro_task_id.as_deref().unwrap_or("missing"),
            projection.run.owner_session
        ),
    );
    tagged_label(
        ui,
        ctx.egui_id.with("recovery-row"),
        "swarm-lane-diagnostics.recovery",
        &format!(
            "active leases {} | reclaimable {:?} | orphan state {}",
            projection.active_lease_count,
            projection.reclaimable_lease_ids,
            projection.orphan_state
        ),
    );

    ui.separator();
    ui.label("Lanes");
    let visible_lane_ids = visible_lane_ids_for_filter(projection, &state.lane_filter);
    for lane in projection
        .lanes
        .iter()
        .filter(|lane| visible_lane_ids.contains(&lane.lane_id))
    {
        tagged_label(
            ui,
            ctx.egui_id.with(("lane", &lane.lane_id)),
            &lane_author_id(&lane.lane_id),
            &format!(
                "{} | {} role {} | session {} model-session {} | {} | recovery {} | runtime {} via {} | messages {} | payload errors {} | event {}#{} | span {} | FR {} | Locus {} | runtime-status {} | recovery-hint {}",
                lane.lane_id,
                lane.kind,
                lane.role,
                lane.session_id,
                lane.model_session_id,
                lane.status,
                lane.recovery_state,
                lane.runtime_binding,
                lane.launch_authority,
                lane.message_count,
                lane.payload_error_count,
                lane.event_ledger_event_id,
                lane.event_ledger_seq,
                lane.lane_span_id,
                lane.flight_recorder_correlation_id,
                lane.locus_ref.as_deref().unwrap_or("missing"),
                lane.last_runtime_status_ref.as_deref().unwrap_or("missing"),
                lane.recovery_hint_ref.as_deref().unwrap_or("missing")
            ),
        );
    }

    ui.separator();
    ui.label("Messages");
    let visible_message_ids = projection
        .messages
        .iter()
        .filter(|message| {
            message_is_visible_for_filters(
                message,
                &visible_lane_ids,
                state.message_filter.as_str(),
            )
        })
        .map(|message| message.message_id.clone())
        .collect::<BTreeSet<_>>();
    if state
        .selected_message_id
        .as_ref()
        .is_some_and(|message_id| !visible_message_ids.contains(message_id))
    {
        state.selected_message_id = None;
    }
    for message in projection
        .messages
        .iter()
        .filter(|message| visible_message_ids.contains(&message.message_id))
    {
        ui.horizontal_wrapped(|ui| {
            let row = ui.selectable_label(
                state.selected_message_id.as_deref() == Some(message.message_id.as_str()),
                format!(
                    "{} | {} | {} -> {} | route {} | promotion {} | payload {} | CRDT {}",
                    message.message_id,
                    message.kind,
                    message.authority,
                    message.to_lane,
                    message.routing_correlation_id.as_deref().unwrap_or("none"),
                    message.promotion_state,
                    message.payload_ref,
                    message.crdt_update_ref.as_deref().unwrap_or("none")
                ),
            );
            ui.ctx().accesskit_node_builder(row.id, |node| {
                node.set_author_id(message_author_id(&message.message_id));
                node.set_label(format!("Message {}", message.message_id));
            });
            if row.clicked() {
                state.selected_message_id = Some(message.message_id.clone());
            }

            let payload = diagnostics_button(
                ui,
                ctx.egui_id.with(("message-payload", &message.message_id)),
                message_payload_author_id(&message.message_id),
                "Payload",
                format!("Payload {}", message.payload_ref),
            );
            if payload.clicked() {
                state.selected_message_id = Some(message.message_id.clone());
            }

            let promotion = diagnostics_button(
                ui,
                ctx.egui_id.with(("message-promotion", &message.message_id)),
                message_promotion_author_id(&message.message_id),
                "Promotion",
                format!("Promotion {}", message.promotion_state),
            );
            if promotion.clicked() {
                state.selected_message_id = Some(message.message_id.clone());
            }
        });
    }

    if let Some(selected) = state
        .selected_message_id
        .as_deref()
        .filter(|id| visible_message_ids.contains(*id))
        .and_then(|id| projection.messages.iter().find(|m| m.message_id == id))
    {
        ui.separator();
        tagged_label(
            ui,
            ctx.egui_id.with("selected-message"),
            &selected_message_author_id(&selected.message_id),
            &format!(
                "selected {} | payload {} sha {} | artifact {} | promotion {} gate {} receipt {} validator {} operator {} | proposal {} CRDT update {} proposal {} base {} vector {} stale {} | trace {} span {} parent {} links {:?} | WP {} MT {} owner {} | recovery {}",
                selected.message_id,
                selected.payload_ref,
                selected.payload_sha256,
                selected.artifact_ref.as_deref().unwrap_or("none"),
                selected.promotion_decision_id.as_deref().unwrap_or("none"),
                selected.promotion_gate_ref.as_deref().unwrap_or("none"),
                selected.promotion_receipt_ref.as_deref().unwrap_or("none"),
                selected.validator_verdict_ref.as_deref().unwrap_or("none"),
                selected.operator_decision_ref.as_deref().unwrap_or("none"),
                selected.proposal_ref.as_deref().unwrap_or("none"),
                selected.crdt_update_ref.as_deref().unwrap_or("none"),
                selected.crdt_proposal_ref.as_deref().unwrap_or("none"),
                selected.crdt_base_snapshot_ref.as_deref().unwrap_or("none"),
                selected.crdt_state_vector.as_deref().unwrap_or("none"),
                selected.crdt_stale_base_ref.as_deref().unwrap_or("none"),
                selected.trace_id,
                selected.message_span_id,
                selected.parent_span_id.as_deref().unwrap_or("none"),
                selected.linked_span_contexts,
                selected.work_packet_id.as_deref().unwrap_or("missing"),
                selected.micro_task_id.as_deref().unwrap_or("missing"),
                selected.owner_session,
                selected.recovery_hint_ref.as_deref().unwrap_or("missing")
            ),
        );
    }

    ui.separator();
    ui.label("Diagnostic Tiers");
    for tier in &projection.diagnostic_tiers {
        tagged_label(
            ui,
            ctx.egui_id.with(("tier", &tier.tier)),
            &tier_author_id(&tier.tier),
            &format!(
                "{} | {} | reason {} | evidence {} | follow-up {}",
                tier.tier,
                tier.state,
                tier.reason,
                tier.evidence_ref,
                tier.follow_up_ref.as_deref().unwrap_or("none")
            ),
        );
    }

    ui.separator();
    ui.label("MT Runtime Status");
    for status in &projection.mt_runtime_statuses {
        tagged_label(
            ui,
            ctx.egui_id.with(("mt-status", &status.micro_task_id)),
            &mt_status_author_id(&status.micro_task_id),
            &format!(
                "{} | {} | proof {} | HBR {} | event {}#{}",
                status.micro_task_id,
                status.status,
                status.proof_status_ref.as_deref().unwrap_or("none"),
                status.hbr_status_ref.as_deref().unwrap_or("none"),
                status.event_ledger_event_id,
                status.event_ledger_seq
            ),
        );
    }
}

fn tagged_label(ui: &mut egui::Ui, id: egui::Id, author_id: &str, text: &str) {
    let response = ui.label(text);
    let label = text.to_owned();
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_role(accesskit::Role::Label);
        node.set_author_id(author_id.to_owned());
        node.set_label(label);
    });
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_author_id(format!("{author_id}::egui-response"));
    });
}

fn diagnostics_button(
    ui: &mut egui::Ui,
    id: egui::Id,
    author_id: String,
    visible_label: &str,
    access_label: String,
) -> egui::Response {
    let font = egui::FontId::proportional(12.0);
    let text_color = ui.visuals().text_color();
    let galley = ui
        .painter()
        .layout_no_wrap(visible_label.to_owned(), font, text_color);
    let padding = egui::vec2(8.0, 3.0);
    let desired = galley.size() + padding * 2.0;
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let response = ui.interact(rect, id, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        ui.painter().rect(
            rect,
            3.0,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let text_pos = egui::pos2(
            rect.left() + padding.x,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(text_pos, galley, text_color);
    }

    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            access_label.clone(),
        )
    });
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_role(accesskit::Role::Button);
        node.add_action(accesskit::Action::Click);
        node.set_author_id(author_id);
        node.set_label(access_label);
    });

    response
}

fn token(raw: &str) -> String {
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn lane_matches_filter(lane: &SwarmLaneDiagnosticsLane, lane_filter: &str) -> bool {
    let lane_filter = lane_filter.trim().to_ascii_lowercase();
    lane_filter.is_empty()
        || lane.lane_id.to_ascii_lowercase().contains(&lane_filter)
        || lane.kind.to_ascii_lowercase().contains(&lane_filter)
        || lane.status.to_ascii_lowercase().contains(&lane_filter)
}

fn visible_lane_ids_for_filter(
    projection: &SwarmLaneDiagnosticsProjection,
    lane_filter: &str,
) -> BTreeSet<String> {
    projection
        .lanes
        .iter()
        .filter(|lane| lane_matches_filter(lane, lane_filter))
        .map(|lane| lane.lane_id.clone())
        .collect()
}

fn message_matches_filter(message: &SwarmLaneDiagnosticsMessage, message_filter: &str) -> bool {
    let message_filter = message_filter.trim().to_ascii_lowercase();
    message_filter.is_empty()
        || message
            .message_id
            .to_ascii_lowercase()
            .contains(&message_filter)
        || message
            .from_lane_id
            .to_ascii_lowercase()
            .contains(&message_filter)
        || message
            .payload_ref
            .to_ascii_lowercase()
            .contains(&message_filter)
        || message
            .promotion_state
            .to_ascii_lowercase()
            .contains(&message_filter)
}

fn message_is_visible_for_filters(
    message: &SwarmLaneDiagnosticsMessage,
    visible_lane_ids: &BTreeSet<String>,
    message_filter: &str,
) -> bool {
    visible_lane_ids.contains(&message.from_lane_id)
        && message_matches_filter(message, message_filter)
}

pub fn visible_message_ids_for_filters(
    projection: &SwarmLaneDiagnosticsProjection,
    lane_filter: &str,
    message_filter: &str,
) -> Vec<String> {
    let visible_lane_ids = visible_lane_ids_for_filter(projection, lane_filter);
    projection
        .messages
        .iter()
        .filter(|message| {
            message_is_visible_for_filters(message, &visible_lane_ids, message_filter)
        })
        .map(|message| message.message_id.clone())
        .collect()
}

fn register_author_id(
    author_ids: &mut BTreeMap<String, String>,
    scope: &str,
    raw_id: &str,
    author_id: String,
) -> Result<(), String> {
    let raw_id = raw_id.trim();
    if raw_id.is_empty() {
        return Err(format!("{scope} author_id missing"));
    }
    let rendered_token = token(raw_id);
    if rendered_token.trim_matches('-').is_empty() {
        return Err(format!("{scope} author_id unusable for {raw_id}"));
    }
    let source = format!("{scope}:{raw_id}");
    if let Some(existing) = author_ids.insert(author_id.clone(), source.clone()) {
        return Err(format!(
            "duplicate AccessKit author_id {author_id} for {existing} and {source}"
        ));
    }
    Ok(())
}

pub fn run_author_id(run_id: &str) -> String {
    format!("swarm-lane-diagnostics.run.{}", token(run_id))
}

pub fn lane_author_id(lane_id: &str) -> String {
    format!("swarm-lane-diagnostics.lane.{}", token(lane_id))
}

pub fn message_author_id(message_id: &str) -> String {
    format!("swarm-lane-diagnostics.message.{}", token(message_id))
}

pub fn message_payload_author_id(message_id: &str) -> String {
    format!(
        "swarm-lane-diagnostics.message.{}.payload",
        token(message_id)
    )
}

pub fn message_promotion_author_id(message_id: &str) -> String {
    format!(
        "swarm-lane-diagnostics.message.{}.promotion",
        token(message_id)
    )
}

pub fn selected_message_author_id(message_id: &str) -> String {
    format!(
        "swarm-lane-diagnostics.message.{}.selected",
        token(message_id)
    )
}

pub fn tier_author_id(tier: &str) -> String {
    format!("swarm-lane-diagnostics.tier.{}", token(tier))
}

pub fn mt_status_author_id(mt_id: &str) -> String {
    format!("swarm-lane-diagnostics.mt-status.{}", token(mt_id))
}

pub fn validate_projection_for_native_surface(
    projection: &SwarmLaneDiagnosticsProjection,
) -> Result<(), String> {
    let mut author_ids = BTreeMap::new();
    if projection.schema_id != PROJECTION_SCHEMA_ID {
        return Err(format!("schema_id mismatch: {}", projection.schema_id));
    }
    if projection.surface_contract_id != SURFACE_CONTRACT_ID {
        return Err(format!(
            "surface_contract_id mismatch: {}",
            projection.surface_contract_id
        ));
    }
    if projection.run.run_id.trim().is_empty() {
        return Err("run_id missing".to_owned());
    }
    register_author_id(
        &mut author_ids,
        "run",
        &projection.run.run_id,
        run_author_id(&projection.run.run_id),
    )?;
    if projection.run.coordinator_session_id.trim().is_empty()
        || projection.run.routing_policy.trim().is_empty()
        || projection.run.artifact_namespace.trim().is_empty()
        || projection.run.owner_session.trim().is_empty()
        || projection.run.budget_summary_ref.trim().is_empty()
        || projection.run.determinism_mode.trim().is_empty()
    {
        return Err("run coordinator/routing/owner/recovery refs missing".to_owned());
    }
    if projection
        .run
        .work_packet_id
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
        || projection
            .run
            .micro_task_id
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        || projection
            .run
            .task_board_id
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        || projection
            .run
            .recovery_hint_ref
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
    {
        return Err("run taskboard or recovery refs missing".to_owned());
    }
    if projection.run.event_ledger_event_id.trim().is_empty()
        || projection
            .run
            .flight_recorder_correlation_id
            .trim()
            .is_empty()
    {
        return Err("run EventLedger or FlightRecorder ref missing".to_owned());
    }
    let lane_count: usize = projection.lanes.iter().map(|lane| lane.message_count).sum();
    if lane_count != projection.messages.len() {
        return Err(format!(
            "lane message_count mismatch: lanes sum {lane_count}, messages {}",
            projection.messages.len()
        ));
    }
    let mut lane_message_counts = BTreeMap::<String, usize>::new();
    let mut lane_ids = BTreeSet::new();
    for lane in &projection.lanes {
        register_author_id(
            &mut author_ids,
            "lane",
            &lane.lane_id,
            lane_author_id(&lane.lane_id),
        )?;
        lane_ids.insert(lane.lane_id.clone());
    }
    for message in &projection.messages {
        if !lane_ids.contains(&message.from_lane_id) {
            return Err(format!(
                "message {} references unknown lane {}",
                message.message_id, message.from_lane_id
            ));
        }
        *lane_message_counts
            .entry(message.from_lane_id.clone())
            .or_insert(0) += 1;
        if let Some(target_lane_id) = message.to_lane.strip_prefix("lane:") {
            if target_lane_id.trim().is_empty() || !lane_ids.contains(target_lane_id) {
                return Err(format!(
                    "message {} references unknown to_lane {}",
                    message.message_id, message.to_lane
                ));
            }
        } else if message.to_lane != "coordinator" && message.to_lane != "broadcast" {
            return Err(format!(
                "message {} routing target unsupported: {}",
                message.message_id, message.to_lane
            ));
        }
    }
    for lane in &projection.lanes {
        let actual_message_count = lane_message_counts
            .get(&lane.lane_id)
            .copied()
            .unwrap_or_default();
        if actual_message_count != lane.message_count {
            return Err(format!(
                "lane {} message_count mismatch: lane says {}, messages contain {}",
                lane.lane_id, lane.message_count, actual_message_count
            ));
        }
        if lane.role.trim().is_empty()
            || lane.backend.trim().is_empty()
            || lane.session_id.trim().is_empty()
            || lane.model_session_id.trim().is_empty()
            || lane.adapter_id.trim().is_empty()
            || lane.launch_authority.trim().is_empty()
            || lane.owner_session.trim().is_empty()
        {
            return Err(format!(
                "lane {} session/model/runtime authority refs missing",
                lane.lane_id
            ));
        }
        if lane
            .locus_ref
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            || lane
                .work_packet_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || lane
                .micro_task_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || lane
                .task_board_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(format!(
                "lane {} Locus/taskboard refs missing",
                lane.lane_id
            ));
        }
        if lane
            .last_runtime_status_ref
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            || lane
                .recovery_hint_ref
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(format!(
                "lane {} runtime recovery refs missing",
                lane.lane_id
            ));
        }
        if lane.flight_recorder_correlation_id.trim().is_empty() {
            return Err(format!("lane {} FlightRecorder ref missing", lane.lane_id));
        }
    }
    for message in &projection.messages {
        register_author_id(
            &mut author_ids,
            "message",
            &message.message_id,
            message_author_id(&message.message_id),
        )?;
        register_author_id(
            &mut author_ids,
            "message payload",
            &message.message_id,
            message_payload_author_id(&message.message_id),
        )?;
        register_author_id(
            &mut author_ids,
            "message promotion",
            &message.message_id,
            message_promotion_author_id(&message.message_id),
        )?;
        register_author_id(
            &mut author_ids,
            "selected message",
            &message.message_id,
            selected_message_author_id(&message.message_id),
        )?;
        if message.payload_ref.trim().is_empty() || message.event_ledger_event_id.trim().is_empty()
        {
            return Err(format!(
                "message {} payload/EventLedger ref missing",
                message.message_id
            ));
        }
        if message.flight_recorder_correlation_id.trim().is_empty() {
            return Err(format!(
                "message {} FlightRecorder ref missing",
                message.message_id
            ));
        }
        if message.to_lane.trim().is_empty()
            || message.coordinator_session_id.trim().is_empty()
            || message.owner_session.trim().is_empty()
        {
            return Err(format!(
                "message {} routing/session refs missing",
                message.message_id
            ));
        }
        if message
            .routing_target_role
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            || message
                .routing_target_session
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || message
                .routing_correlation_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(format!(
                "message {} routing target refs missing",
                message.message_id
            ));
        }
        if message
            .work_packet_id
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            || message
                .micro_task_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || message
                .task_board_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || message
                .locus_ref
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(format!(
                "message {} Locus/taskboard refs missing",
                message.message_id
            ));
        }
        if message.promotion_state == "decision_recorded"
            && (message
                .promotion_decision_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
                || message
                    .promotion_gate_ref
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                || message
                    .promotion_receipt_ref
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty())
        {
            return Err(format!(
                "message {} promotion refs missing",
                message.message_id
            ));
        }
        if message
            .proposal_ref
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            || message
                .crdt_update_ref
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || message
                .crdt_proposal_ref
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(format!("message {} CRDT refs missing", message.message_id));
        }
    }
    for expected in ["flight_recorder", "internal_diagnostics", "palmistry"] {
        if !projection
            .diagnostic_tiers
            .iter()
            .any(|tier| tier.tier == expected)
        {
            return Err(format!("diagnostic tier {expected} missing"));
        }
    }
    for tier in &projection.diagnostic_tiers {
        register_author_id(
            &mut author_ids,
            "diagnostic tier",
            &tier.tier,
            tier_author_id(&tier.tier),
        )?;
        if tier.reason.trim().is_empty() {
            return Err(format!("diagnostic tier {} reason missing", tier.tier));
        }
        if tier.evidence_ref.trim().is_empty() {
            return Err(format!("diagnostic tier {} evidence missing", tier.tier));
        }
        if tier.state.trim().is_empty() || tier.state == "missing" {
            return Err(format!(
                "diagnostic tier {} state cannot be missing",
                tier.tier
            ));
        }
        if tier.state != "wired"
            && tier.state != "not_applicable_with_reason"
            && tier.state != "deferred_with_reason"
        {
            return Err(format!(
                "diagnostic tier {} state unsupported: {}",
                tier.tier, tier.state
            ));
        }
        if tier.state == "deferred_with_reason" {
            if tier
                .follow_up_ref
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                return Err(format!(
                    "diagnostic tier {} deferred follow_up_ref missing",
                    tier.tier
                ));
            }
        }
    }
    if projection.mt_runtime_statuses.is_empty() {
        return Err("MT runtime status rows missing".to_owned());
    }
    let expected_micro_task_id = projection.run.micro_task_id.as_deref().unwrap_or_default();
    let mut covers_run_micro_task = false;
    for status in &projection.mt_runtime_statuses {
        register_author_id(
            &mut author_ids,
            "MT runtime status",
            &status.micro_task_id,
            mt_status_author_id(&status.micro_task_id),
        )?;
        if status.micro_task_id == expected_micro_task_id {
            covers_run_micro_task = true;
        }
        if status.status.trim().is_empty()
            || status.event_ledger_event_id.trim().is_empty()
            || status.event_ledger_seq <= 0
            || status
                .proof_status_ref
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || status
                .hbr_status_ref
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(format!(
                "MT runtime status {} status/proof/HBR/EventLedger ref missing",
                status.micro_task_id
            ));
        }
    }
    if !covers_run_micro_task {
        return Err(format!(
            "MT runtime status for run micro_task_id {expected_micro_task_id} missing"
        ));
    }
    Ok(())
}
