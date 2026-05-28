use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use crate::kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent};
use crate::storage::Database;

const EVENT_TYPE_HBR_HANDOFF_GATE: &str = "HBR_HANDOFF_GATE";
const EVENT_SOURCE_COMPONENT: &str = "hbr_handoff_gate";
// MT-004 finding #2: an EventLedger append failure is infrastructure, not an
// HBR rule violation. The sentinel is namespaced `INFRA-` (not `HBR-`) so a
// downstream tool joining failing_rules.hbr_id against the HBR registry can
// recognise it as a non-rule sentinel instead of recording a phantom registry
// miss.
const EVENT_LEDGER_FAILURE_RULE: &str = "INFRA-HANDOFF-GATE-EVENT-LEDGER";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbrPacket {
    pub wp_id: String,
    pub acceptance_matrix: HbrAcceptanceMatrix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbrAcceptanceMatrix {
    pub hbr: Vec<HbrMatrixRow>,
    pub hbr_not_applicable: Vec<HbrNotApplicableRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbrMatrixRow {
    pub hbr_id: String,
    pub status: String,
    pub evidence_pointer: Option<String>,
    pub validator_verdict: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbrNotApplicableRow {
    pub hbr_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffRule {
    pub hbr_id: String,
    pub evidence_kind: String,
}

impl HandoffRule {
    pub fn new(hbr_id: impl Into<String>, evidence_kind: impl Into<String>) -> Self {
        Self {
            hbr_id: hbr_id.into(),
            evidence_kind: evidence_kind.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HandoffTransition {
    RefinementToCoder,
    CoderToWpValidator,
    WpValidatorToIntegrationValidator,
    IntegrationValidatorToOrchestrator,
}

impl HandoffTransition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RefinementToCoder => "RefinementToCoder",
            Self::CoderToWpValidator => "CoderToWpValidator",
            Self::WpValidatorToIntegrationValidator => "WpValidatorToIntegrationValidator",
            Self::IntegrationValidatorToOrchestrator => "IntegrationValidatorToOrchestrator",
        }
    }

    pub fn checkable_evidence_kinds(self) -> &'static [&'static str] {
        match self {
            Self::RefinementToCoder => &[
                "manual_diff_in_same_commit",
                "manual_self_consistency_test",
                "open_blocker_with_missing_surface_named",
            ],
            Self::CoderToWpValidator => &[
                "test_run_with_ledger_replay",
                "test_run_with_session_trace",
                "test_run_with_promotion_event_and_negative_guard",
                "test_run_with_reconnect_and_promotion",
                "test_run_with_cross_lane_trace_comparison",
                "test_run_with_fems_record_and_retrieval_policy",
                "test_run_with_trace_projection_replay",
                "grep_zero_v4_in_touched_files_plus_v7_version_assert_test",
                "process_lifecycle_test_run",
                // STOP pillar (registry v1.3.0, scope/session discipline
                // [CX-971]). These are evaluated at the coder handoff so a
                // PENDING/BLOCKED STOP row blocks the gate instead of
                // silently passing by omission.
                "absence_of_capacity_reasoning_in_receipts_and_lifecycle",
                "absence_of_capacity_stop_reasons_in_receipts_and_lifecycle",
                "proof_block_with_real_external_resource_touch_per_Spec_Realism_Gate_sub_rule_2",
                "receipts_show_dependency_work_then_return_to_outstanding_work",
                "end_of_task_self_evaluation_with_full_disclosure_waiver_request_to_operator",
            ],
            Self::WpValidatorToIntegrationValidator => &[
                "concurrent_session_test_run",
                "concurrency_primitive_test_run",
                "parallel_operator_agent_ui_test_run",
                "typed_routing_test_run",
                "gui_drive_with_screenshot_capture",
                "renderer_console_error_report",
                "headless_run_with_focus_audit",
                "automation_path_audit",
            ],
            Self::IntegrationValidatorToOrchestrator => &[
                "capture_matrix",
                "trace_projection_inspection",
                "foreground_exception_declaration_and_operator_warning",
                "no_context_model_operation_test",
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvaluatedHbrRule {
    pub hbr_id: String,
    pub evidence_kind: String,
    pub status: String,
    pub evidence_pointer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HandoffFailingRule {
    pub hbr_id: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct HandoffEvidence {
    pub wp_id: String,
    pub transition: HandoffTransition,
    pub gate_uuid: Uuid,
    pub timestamp_utc: DateTime<Utc>,
    pub evaluated_rules: Vec<EvaluatedHbrRule>,
    pub event: KernelEvent,
}

#[derive(Debug, Clone)]
pub struct HandoffBlock {
    pub wp_id: String,
    pub transition: HandoffTransition,
    pub gate_uuid: Uuid,
    pub timestamp_utc: DateTime<Utc>,
    pub evaluated_rules: Vec<EvaluatedHbrRule>,
    pub failing_rules: Vec<HandoffFailingRule>,
    pub event: Option<KernelEvent>,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("{message}")]
pub struct HandoffEventLedgerError {
    message: String,
}

impl HandoffEventLedgerError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[async_trait]
pub trait HandoffEventLedger: Send + Sync {
    async fn append_handoff_event(
        &self,
        event: NewKernelEvent,
    ) -> Result<KernelEvent, HandoffEventLedgerError>;
}

#[async_trait]
impl<T> HandoffEventLedger for T
where
    T: Database + Send + Sync,
{
    async fn append_handoff_event(
        &self,
        event: NewKernelEvent,
    ) -> Result<KernelEvent, HandoffEventLedgerError> {
        self.append_kernel_event(event)
            .await
            .map_err(|error| HandoffEventLedgerError::new(error.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct HandoffGate<L> {
    ledger: L,
    rules: Vec<HandoffRule>,
}

impl<L> HandoffGate<L>
where
    L: HandoffEventLedger,
{
    pub fn new(ledger: L, rules: Vec<HandoffRule>) -> Self {
        Self { ledger, rules }
    }

    pub async fn evaluate(
        &self,
        packet: &HbrPacket,
        transition: HandoffTransition,
    ) -> Result<HandoffEvidence, HandoffBlock> {
        let gate_uuid = Uuid::now_v7();
        let timestamp_utc = Utc::now();
        let (evaluated_rules, failing_rules) = self.evaluate_rows(packet, transition);
        let verdict = if failing_rules.is_empty() {
            HandoffVerdict::Pass
        } else {
            HandoffVerdict::Block {
                failing_rules: failing_rules.clone(),
            }
        };

        let event = match self
            .append_event(
                packet,
                transition,
                gate_uuid,
                timestamp_utc,
                evaluated_rules.clone(),
                verdict,
            )
            .await
        {
            Ok(event) => event,
            Err(message) => {
                // MT-004 finding #3: do NOT drop the matrix violations when the
                // EventLedger append fails. The operator must see both "these
                // HBRs were violated" AND "the gate event could not be
                // recorded", so merge the matrix failing_rules with the infra
                // sentinel rather than returning the sentinel alone.
                let mut merged_failing_rules = failing_rules;
                merged_failing_rules.push(HandoffFailingRule {
                    hbr_id: EVENT_LEDGER_FAILURE_RULE.to_string(),
                    reason: message,
                });
                return Err(HandoffBlock {
                    wp_id: packet.wp_id.clone(),
                    transition,
                    gate_uuid,
                    timestamp_utc,
                    evaluated_rules,
                    failing_rules: merged_failing_rules,
                    event: None,
                });
            }
        };

        if failing_rules.is_empty() {
            Ok(HandoffEvidence {
                wp_id: packet.wp_id.clone(),
                transition,
                gate_uuid,
                timestamp_utc,
                evaluated_rules,
                event,
            })
        } else {
            Err(HandoffBlock {
                wp_id: packet.wp_id.clone(),
                transition,
                gate_uuid,
                timestamp_utc,
                evaluated_rules,
                failing_rules,
                event: Some(event),
            })
        }
    }

    fn evaluate_rows(
        &self,
        packet: &HbrPacket,
        transition: HandoffTransition,
    ) -> (Vec<EvaluatedHbrRule>, Vec<HandoffFailingRule>) {
        let mut evaluated_rules = Vec::new();
        let mut failing_rules = Vec::new();

        for row in &packet.acceptance_matrix.hbr {
            let Some(rule) = self
                .rules
                .iter()
                .find(|candidate| candidate.hbr_id == row.hbr_id)
            else {
                continue;
            };

            if !transition
                .checkable_evidence_kinds()
                .contains(&rule.evidence_kind.as_str())
            {
                continue;
            }

            evaluated_rules.push(EvaluatedHbrRule {
                hbr_id: row.hbr_id.clone(),
                evidence_kind: rule.evidence_kind.clone(),
                status: normalize_status(&row.status),
                evidence_pointer: non_empty(row.evidence_pointer.as_deref()).map(str::to_string),
            });

            if let Some(reason) =
                row_failure_reason(row, &packet.acceptance_matrix.hbr_not_applicable)
            {
                failing_rules.push(HandoffFailingRule {
                    hbr_id: row.hbr_id.clone(),
                    reason,
                });
            }
        }

        (evaluated_rules, failing_rules)
    }

    async fn append_event(
        &self,
        packet: &HbrPacket,
        transition: HandoffTransition,
        gate_uuid: Uuid,
        timestamp_utc: DateTime<Utc>,
        evaluated_rules: Vec<EvaluatedHbrRule>,
        verdict: HandoffVerdict,
    ) -> Result<KernelEvent, String> {
        let payload = HandoffGateEventPayload {
            event_type: EVENT_TYPE_HBR_HANDOFF_GATE,
            wp_id: packet.wp_id.clone(),
            transition: transition.as_str(),
            gate_uuid,
            evaluated_rules,
            verdict,
            timestamp_utc,
        };
        let payload = serde_json::to_value(payload)
            .map_err(|error| format!("failed to serialize HBR handoff payload: {error}"))?;
        let event = NewKernelEvent::builder(
            format!("KTR-HBR-HANDOFF-{gate_uuid}"),
            format!("SR-HBR-HANDOFF-{gate_uuid}"),
            KernelEventType::HbrHandoffGate,
            KernelActor::SessionBroker(EVENT_SOURCE_COMPONENT.to_string()),
        )
        .aggregate("work_packet", packet.wp_id.clone())
        .idempotency_key(format!(
            "{EVENT_TYPE_HBR_HANDOFF_GATE}:{}:{}:{gate_uuid}",
            packet.wp_id,
            transition.as_str()
        ))
        .correlation_id(gate_uuid.to_string())
        .source_component(EVENT_SOURCE_COMPONENT)
        .payload(payload)
        .build()
        .map_err(|error| format!("failed to build HBR handoff EventLedger row: {error}"))?;

        self.ledger
            .append_handoff_event(event)
            .await
            .map_err(|error| format!("failed to append HBR handoff EventLedger row: {error}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
enum HandoffVerdict {
    Pass,
    Block {
        failing_rules: Vec<HandoffFailingRule>,
    },
}

#[derive(Debug, Serialize)]
struct HandoffGateEventPayload {
    event_type: &'static str,
    wp_id: String,
    transition: &'static str,
    gate_uuid: Uuid,
    evaluated_rules: Vec<EvaluatedHbrRule>,
    verdict: HandoffVerdict,
    timestamp_utc: DateTime<Utc>,
}

fn row_failure_reason(
    row: &HbrMatrixRow,
    not_applicable: &[HbrNotApplicableRow],
) -> Option<String> {
    let status = normalize_status(&row.status);
    match status.as_str() {
        "PROVED" => {
            if non_empty(row.evidence_pointer.as_deref()).is_none() {
                return Some("PROVED HBR row requires a non-empty evidence_pointer".to_string());
            }
            if row.validator_verdict.as_deref() != Some("PROVED") {
                return Some("PROVED HBR row requires validator_verdict == \"PROVED\"".to_string());
            }
            None
        }
        "NOT_APPLICABLE" => {
            let has_reason = not_applicable.iter().any(|entry| {
                entry.hbr_id == row.hbr_id && non_empty(Some(entry.reason.as_str())).is_some()
            });
            if has_reason {
                None
            } else {
                Some(
                    "NOT_APPLICABLE HBR row requires matching hbr_not_applicable reason"
                        .to_string(),
                )
            }
        }
        "PENDING" | "STEER" | "BLOCKED" => {
            Some(format!("HBR status {status} is not accepted for handoff"))
        }
        "" => Some("HBR status is required for handoff".to_string()),
        other => Some(format!("HBR status {other} is not recognized for handoff")),
    }
}

fn normalize_status(status: &str) -> String {
    status.trim().to_ascii_uppercase()
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[allow(dead_code)]
fn _payload_shape_example() -> serde_json::Value {
    json!({
        "event_type": EVENT_TYPE_HBR_HANDOFF_GATE,
        "wp_id": "WP-EXAMPLE",
        "transition": HandoffTransition::CoderToWpValidator.as_str(),
        "gate_uuid": Uuid::now_v7(),
        "evaluated_rules": [],
        "verdict": {"kind": "Pass"},
        "timestamp_utc": Utc::now(),
    })
}
