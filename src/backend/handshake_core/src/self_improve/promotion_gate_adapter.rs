//! MT-154: PromotionGate integration for the self-improvement loop.
//!
//! Wires the loop's AcceptRejectDecision through the KERNEL-001
//! PromotionGate contract. The adapter is asynchronous (submit + poll)
//! because operator review may take hours/days.

use std::{future::Future, sync::Arc};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

use super::editable_surface::EditableSurfaceSnapshot;
use super::evaluator::EvalResult;
use super::goodhart_sentinel::SentinelDecision;
use super::iteration::{LoopTarget, OperatorId};
use super::promotion_floor::PromotionDecision;
use crate::kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent};
use crate::storage::{Database, StorageError};

/// Aggregate type for durable self-improve promotion-review tickets.
pub const PROMOTION_TICKET_AGGREGATE_TYPE: &str = "self_improve_promotion_ticket";

/// Backend-neutral source label for promotion-gate EventLedger evidence.
pub const PROMOTION_GATE_SOURCE_COMPONENT: &str = "self_improve_promotion_gate_event_ledger";

/// Payload schema shared by requested, accepted, and rejected ticket events.
pub const PROMOTION_TICKET_PAYLOAD_SCHEMA_ID: &str = "hsk.self_improve.promotion_ticket@1";

/// Evidence bundle submitted to the gate. The reviewer sees the baseline
/// + proposed surface, the eval, the floor decision, and the sentinel
/// decision in one place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionRequest {
    pub iteration_id: Uuid,
    pub target: LoopTarget,
    pub baseline_snapshot: EditableSurfaceSnapshot,
    pub proposed_snapshot: EditableSurfaceSnapshot,
    pub eval_result: EvalResult,
    pub floor_decision: PromotionDecision,
    pub sentinel_decision: SentinelDecision,
    pub justification_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionTicket {
    pub ticket_id: Uuid,
    pub iteration_id: Uuid,
    pub submitted_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionApproval {
    pub approved_by: OperatorId,
    pub approved_at_utc: DateTime<Utc>,
    pub signoff_evidence_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionRejection {
    pub rejected_by: OperatorId,
    pub rejected_at_utc: DateTime<Utc>,
    pub rejection_reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum PromotionStatus {
    Pending { submitted_at_utc: DateTime<Utc> },
    Approved { approval: PromotionApproval },
    Rejected { rejection: PromotionRejection },
}

impl PromotionStatus {
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved { .. })
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }
}

/// Gate trait consumed by the self-improvement loop. Production uses
/// [`EventLedgerPromotionGate`]; focused unit tests may use narrow mocks.
pub trait PromotionGateSubmitter {
    fn submit(&self, request: PromotionRequest) -> Result<PromotionTicket, GateError>;
    fn poll(&self, ticket: &PromotionTicket) -> Result<PromotionStatus, GateError>;
}

/// Durable promotion gate backed by Handshake's EventLedger authority.
///
/// Production supplies the embedded-Surreal [`Database`] implementation.
/// The adapter intentionally depends on the backend-neutral database seam so
/// ticket semantics remain tied to EventLedger rather than a storage driver.
pub struct EventLedgerPromotionGate {
    db: Arc<dyn Database>,
}

impl EventLedgerPromotionGate {
    pub fn with_db(db: Arc<dyn Database>) -> Self {
        Self { db }
    }

    /// Persist an operator approval with its identity, timestamp, and signoff
    /// evidence. A ticket can transition out of pending exactly once.
    pub fn record_approval(
        &self,
        ticket: &PromotionTicket,
        approval: PromotionApproval,
    ) -> Result<(), GateError> {
        self.record_decision(
            ticket,
            KernelEventType::PromotionAccepted,
            "approved",
            json!({ "approval": approval }),
        )
    }

    /// Persist an operator rejection with its identity, timestamp, and reason.
    /// A ticket can transition out of pending exactly once.
    pub fn record_rejection(
        &self,
        ticket: &PromotionTicket,
        rejection: PromotionRejection,
    ) -> Result<(), GateError> {
        self.record_decision(
            ticket,
            KernelEventType::PromotionRejected,
            "rejected",
            json!({ "rejection": rejection }),
        )
    }

    fn record_decision(
        &self,
        ticket: &PromotionTicket,
        event_type: KernelEventType,
        status_label: &str,
        decision_fields: Value,
    ) -> Result<(), GateError> {
        let events = self.ticket_events(ticket.ticket_id)?;
        match status_from_events(ticket, &events)? {
            PromotionStatus::Pending { .. } => {}
            PromotionStatus::Approved { .. } | PromotionStatus::Rejected { .. } => {
                return Err(already_decided(ticket.ticket_id));
            }
        }

        let requested_event_id = requested_event(ticket, &events)?.event_id.clone();
        let mut payload = json!({
            "schema_id": PROMOTION_TICKET_PAYLOAD_SCHEMA_ID,
            "status": status_label,
            "ticket": ticket,
        });
        if let (Value::Object(target), Value::Object(fields)) = (&mut payload, decision_fields) {
            target.extend(fields);
        }

        let event = NewKernelEvent::builder(
            format!("KTR-SELF-IMPROVE-PROMOTION-{}", ticket.ticket_id),
            format!("SR-SELF-IMPROVE-PROMOTION-{}", ticket.ticket_id),
            event_type,
            KernelActor::PromotionGate("self_improve_loop".to_owned()),
        )
        .aggregate(
            PROMOTION_TICKET_AGGREGATE_TYPE,
            ticket.ticket_id.to_string(),
        )
        // One key for either decision makes concurrent approve/reject attempts
        // conflict at the durable EventLedger boundary instead of both winning.
        .idempotency_key(format!(
            "self_improve_promotion_decision:{}",
            ticket.ticket_id
        ))
        .causation_id(requested_event_id)
        .correlation_id(ticket.iteration_id.to_string())
        .event_version("kernel_event_v1")
        .source_component(PROMOTION_GATE_SOURCE_COMPONENT)
        .payload(payload)
        .build()
        .map_err(|error| GateError::Io {
            message: format!("promotion decision event build failed: {error}"),
        })?;

        let db = Arc::clone(&self.db);
        match block_on_storage(async move { db.append_kernel_event(event).await }) {
            Ok(_) => Ok(()),
            Err(error) if is_idempotency_conflict(&error) => Err(already_decided(ticket.ticket_id)),
            Err(error) => Err(GateError::Io {
                message: format!("appending promotion decision to EventLedger failed: {error}"),
            }),
        }
    }

    fn ticket_events(&self, ticket_id: Uuid) -> Result<Vec<KernelEvent>, GateError> {
        let db = Arc::clone(&self.db);
        let aggregate_id = ticket_id.to_string();
        block_on_storage(async move {
            db.list_kernel_events_for_aggregate(PROMOTION_TICKET_AGGREGATE_TYPE, &aggregate_id)
                .await
        })
        .map_err(|error| GateError::Io {
            message: format!("reading promotion ticket from EventLedger failed: {error}"),
        })
    }
}

impl PromotionGateSubmitter for EventLedgerPromotionGate {
    fn submit(&self, request: PromotionRequest) -> Result<PromotionTicket, GateError> {
        let ticket = PromotionTicket {
            ticket_id: Uuid::now_v7(),
            iteration_id: request.iteration_id,
            submitted_at_utc: Utc::now(),
        };
        let payload = json!({
            "schema_id": PROMOTION_TICKET_PAYLOAD_SCHEMA_ID,
            "status": "pending",
            "ticket": ticket,
            "request": request,
        });
        let event = NewKernelEvent::builder(
            format!("KTR-SELF-IMPROVE-PROMOTION-{}", ticket.ticket_id),
            format!("SR-SELF-IMPROVE-PROMOTION-{}", ticket.ticket_id),
            KernelEventType::PromotionRequested,
            KernelActor::PromotionGate("self_improve_loop".to_owned()),
        )
        .aggregate(
            PROMOTION_TICKET_AGGREGATE_TYPE,
            ticket.ticket_id.to_string(),
        )
        .idempotency_key(format!(
            "self_improve_promotion_submit:{}",
            ticket.ticket_id
        ))
        .correlation_id(ticket.iteration_id.to_string())
        .event_version("kernel_event_v1")
        .source_component(PROMOTION_GATE_SOURCE_COMPONENT)
        .payload(payload)
        .build()
        .map_err(|error| GateError::Io {
            message: format!("promotion ticket event build failed: {error}"),
        })?;

        let db = Arc::clone(&self.db);
        block_on_storage(async move { db.append_kernel_event(event).await }).map_err(|error| {
            GateError::Io {
                message: format!("appending promotion ticket to EventLedger failed: {error}"),
            }
        })?;
        Ok(ticket)
    }

    fn poll(&self, ticket: &PromotionTicket) -> Result<PromotionStatus, GateError> {
        status_from_events(ticket, &self.ticket_events(ticket.ticket_id)?)
    }
}

fn requested_event<'a>(
    ticket: &PromotionTicket,
    events: &'a [KernelEvent],
) -> Result<&'a KernelEvent, GateError> {
    events
        .iter()
        .find(|event| {
            event.event_type == KernelEventType::PromotionRequested
                && is_promotion_gate_event(event)
                && payload_ticket_matches(&event.payload, ticket)
                && event.payload.get("schema_id").and_then(Value::as_str)
                    == Some(PROMOTION_TICKET_PAYLOAD_SCHEMA_ID)
                && event.payload.get("status").and_then(Value::as_str) == Some("pending")
                && event.correlation_id.as_deref() == Some(ticket.iteration_id.to_string().as_str())
        })
        .ok_or(GateError::UnknownTicket)
}

fn status_from_events(
    ticket: &PromotionTicket,
    events: &[KernelEvent],
) -> Result<PromotionStatus, GateError> {
    let requested = requested_event(ticket, events)?;
    let mut latest = None;
    let mut latest_sequence = i64::MIN;
    for event in events {
        if event.correlation_id.as_deref() != Some(ticket.iteration_id.to_string().as_str()) {
            continue;
        }
        let Some(status) = decode_status(ticket, &requested.event_id, event) else {
            continue;
        };
        if event.event_sequence > latest_sequence {
            latest_sequence = event.event_sequence;
            latest = Some(status);
        }
    }
    latest.ok_or(GateError::UnknownTicket)
}

fn payload_ticket_matches(payload: &Value, ticket: &PromotionTicket) -> bool {
    serde_json::from_value::<PromotionTicket>(payload.get("ticket").cloned().unwrap_or(Value::Null))
        .is_ok_and(|persisted| persisted == *ticket)
}

fn is_promotion_gate_event(event: &KernelEvent) -> bool {
    event.source_component == PROMOTION_GATE_SOURCE_COMPONENT
        && matches!(
            &event.actor,
            KernelActor::PromotionGate(actor_id) if actor_id == "self_improve_loop"
        )
}

fn decode_status(
    ticket: &PromotionTicket,
    requested_event_id: &str,
    event: &KernelEvent,
) -> Option<PromotionStatus> {
    if !is_promotion_gate_event(event) {
        return None;
    }
    let payload = &event.payload;
    if payload.get("schema_id")?.as_str()? != PROMOTION_TICKET_PAYLOAD_SCHEMA_ID {
        return None;
    }
    let persisted_ticket: PromotionTicket =
        serde_json::from_value(payload.get("ticket")?.clone()).ok()?;
    if persisted_ticket != *ticket {
        return None;
    }
    match payload.get("status")?.as_str()? {
        "pending"
            if event.event_type == KernelEventType::PromotionRequested
                && event.causation_id.is_none() =>
        {
            Some(PromotionStatus::Pending {
                submitted_at_utc: ticket.submitted_at_utc,
            })
        }
        "approved"
            if event.event_type == KernelEventType::PromotionAccepted
                && event.causation_id.as_deref() == Some(requested_event_id) =>
        {
            serde_json::from_value(payload.get("approval")?.clone())
                .ok()
                .map(|approval| PromotionStatus::Approved { approval })
        }
        "rejected"
            if event.event_type == KernelEventType::PromotionRejected
                && event.causation_id.as_deref() == Some(requested_event_id) =>
        {
            serde_json::from_value(payload.get("rejection")?.clone())
                .ok()
                .map(|rejection| PromotionStatus::Rejected { rejection })
        }
        _ => None,
    }
}

fn is_idempotency_conflict(error: &StorageError) -> bool {
    matches!(
        error,
        StorageError::Conflict(message)
            if message.starts_with("kernel event idempotency key was reused")
    )
}

fn already_decided(ticket_id: Uuid) -> GateError {
    GateError::Io {
        message: format!(
            "promotion ticket {ticket_id} is already decided; double-decision rejected"
        ),
    }
}

fn block_on_storage<F>(future: F) -> F::Output
where
    F: Future + Send,
    F::Output: Send,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle)
            if matches!(
                handle.runtime_flavor(),
                tokio::runtime::RuntimeFlavor::MultiThread
            ) =>
        {
            tokio::task::block_in_place(|| handle.block_on(future))
        }
        Ok(_) => std::thread::scope(|scope| {
            scope
                .spawn(move || {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("tokio current-thread runtime must build")
                        .block_on(future)
                })
                .join()
                .expect("dedicated storage runtime thread must not panic")
        }),
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio current-thread runtime must build")
            .block_on(future),
    }
}

/// Adapter from LoopCore -> PromotionGate.
pub struct LoopPromotionGate<'a> {
    pub gate: &'a dyn PromotionGateSubmitter,
}

impl<'a> LoopPromotionGate<'a> {
    pub fn new(gate: &'a dyn PromotionGateSubmitter) -> Self {
        Self { gate }
    }

    pub fn submit(&self, request: PromotionRequest) -> Result<PromotionTicket, GateError> {
        // MT-154 remediation: the gate's ticket MUST stay bound to the
        // iteration it was submitted for. A gate that returns a ticket
        // carrying a different iteration_id (buggy or malicious) would let
        // one iteration's approval evidence be inherited by another, breaking
        // the audit chain operator-review depends on. Reject the mismatch
        // loudly instead of trusting the returned ticket verbatim.
        let expected_iteration_id = request.iteration_id;
        let ticket = self.gate.submit(request)?;
        if ticket.iteration_id != expected_iteration_id {
            return Err(GateError::Io {
                message: format!(
                    "gate returned mismatched iteration_id (expected {expected_iteration_id}, got {})",
                    ticket.iteration_id
                ),
            });
        }
        Ok(ticket)
    }

    pub fn poll(&self, ticket: &PromotionTicket) -> Result<PromotionStatus, GateError> {
        self.gate.poll(ticket)
    }

    /// Helper: callers (the loop) use this before invoking
    /// `apply_proposal` on the editable surface so a Pending or Rejected
    /// status returns a typed error.
    pub fn require_approved(
        &self,
        ticket: &PromotionTicket,
    ) -> Result<PromotionApproval, GateError> {
        match self.poll(ticket)? {
            PromotionStatus::Approved { approval } => Ok(approval),
            PromotionStatus::Pending { .. } => Err(GateError::ReviewPending),
            PromotionStatus::Rejected { rejection } => Err(GateError::ReviewRejected {
                rationale: rejection.rejection_reason,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum GateError {
    #[error("promotion gate ticket not found")]
    UnknownTicket,
    #[error("promotion review pending — apply blocked")]
    ReviewPending,
    #[error("promotion review rejected: {rationale}")]
    ReviewRejected { rationale: String },
    #[error("promotion gate I/O error: {message}")]
    Io { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_improve::evaluator::SplitMetrics;
    use crate::self_improve::promotion_floor::MetricDelta;
    use std::collections::HashMap;
    use std::sync::Mutex;

    enum TicketState {
        Pending,
        Approved(PromotionApproval),
        Rejected(PromotionRejection),
    }

    struct MockGate {
        tickets: Mutex<HashMap<Uuid, TicketState>>,
    }

    impl MockGate {
        fn new() -> Self {
            Self {
                tickets: Mutex::new(HashMap::new()),
            }
        }

        fn approve(&self, ticket_id: Uuid, op: &str) {
            let approval = PromotionApproval {
                approved_by: OperatorId::new(op),
                approved_at_utc: Utc::now(),
                signoff_evidence_id: Uuid::now_v7(),
            };
            self.tickets
                .lock()
                .unwrap()
                .insert(ticket_id, TicketState::Approved(approval));
        }

        fn reject(&self, ticket_id: Uuid, op: &str, reason: &str) {
            let rej = PromotionRejection {
                rejected_by: OperatorId::new(op),
                rejected_at_utc: Utc::now(),
                rejection_reason: reason.to_string(),
            };
            self.tickets
                .lock()
                .unwrap()
                .insert(ticket_id, TicketState::Rejected(rej));
        }
    }

    impl PromotionGateSubmitter for MockGate {
        fn submit(&self, request: PromotionRequest) -> Result<PromotionTicket, GateError> {
            // A correct gate binds the issued ticket to the request's
            // iteration_id (MT-154 finding #2).
            let ticket = PromotionTicket {
                ticket_id: Uuid::now_v7(),
                iteration_id: request.iteration_id,
                submitted_at_utc: Utc::now(),
            };
            self.tickets
                .lock()
                .unwrap()
                .insert(ticket.ticket_id, TicketState::Pending);
            Ok(ticket)
        }

        fn poll(&self, ticket: &PromotionTicket) -> Result<PromotionStatus, GateError> {
            let guard = self.tickets.lock().unwrap();
            match guard.get(&ticket.ticket_id) {
                Some(TicketState::Pending) => Ok(PromotionStatus::Pending {
                    submitted_at_utc: ticket.submitted_at_utc,
                }),
                Some(TicketState::Approved(a)) => Ok(PromotionStatus::Approved {
                    approval: a.clone(),
                }),
                Some(TicketState::Rejected(r)) => Ok(PromotionStatus::Rejected {
                    rejection: r.clone(),
                }),
                None => Err(GateError::UnknownTicket),
            }
        }
    }

    fn sample_request() -> PromotionRequest {
        let snapshot = EditableSurfaceSnapshot::RetrievalPolicy {
            task_type: crate::memory::TaskType::ValidatorHbrTestPacket,
            parameter: super::super::editable_surface::PolicyParameter::TopK,
            before_value: 6,
            after_value: 8,
        };
        PromotionRequest {
            iteration_id: Uuid::now_v7(),
            target: LoopTarget::RetrievalPolicyParams {
                task_type: crate::memory::TaskType::ValidatorHbrTestPacket,
                parameter: super::super::editable_surface::PolicyParameter::TopK,
            },
            baseline_snapshot: snapshot.clone(),
            proposed_snapshot: snapshot,
            eval_result: EvalResult {
                train: SplitMetrics::empty(),
                dev: SplitMetrics::empty(),
                holdout: SplitMetrics::empty(),
                evaluated_at_utc: Utc::now(),
                snapshot_hash: "0".repeat(64),
            },
            floor_decision: PromotionDecision::Approved {
                delta: MetricDelta {
                    dev_pass_delta_pp: 0.1,
                    latency_p95_delta_ms: 0,
                    capsule_bytes_p95_delta_bytes: 0,
                    holdout_pass_delta_pp: 0.0,
                },
            },
            sentinel_decision: SentinelDecision::Continue,
            justification_text: "test".to_string(),
        }
    }

    #[test]
    fn submit_returns_typed_ticket_then_pending() {
        let gate = MockGate::new();
        let adapter = LoopPromotionGate::new(&gate);
        let ticket = adapter.submit(sample_request()).unwrap();
        let status = adapter.poll(&ticket).unwrap();
        assert!(status.is_pending());
    }

    #[test]
    fn require_approved_returns_err_pending_then_ok_after_approval() {
        let gate = MockGate::new();
        let adapter = LoopPromotionGate::new(&gate);
        let ticket = adapter.submit(sample_request()).unwrap();
        let err = adapter.require_approved(&ticket).unwrap_err();
        assert!(matches!(err, GateError::ReviewPending));

        gate.approve(ticket.ticket_id, "operator-1");
        let approval = adapter.require_approved(&ticket).unwrap();
        assert_eq!(approval.approved_by.as_str(), "operator-1");
        assert_eq!(approval.signoff_evidence_id.get_version_num(), 7);
    }

    #[test]
    fn require_approved_returns_rejected_typed() {
        let gate = MockGate::new();
        let adapter = LoopPromotionGate::new(&gate);
        let ticket = adapter.submit(sample_request()).unwrap();
        gate.reject(ticket.ticket_id, "operator-1", "bad change");
        let err = adapter.require_approved(&ticket).unwrap_err();
        match err {
            GateError::ReviewRejected { rationale } => assert_eq!(rationale, "bad change"),
            _ => panic!("expected rejected variant"),
        }
    }

    #[test]
    fn unknown_ticket_returns_typed_error() {
        let gate = MockGate::new();
        let adapter = LoopPromotionGate::new(&gate);
        let fake_ticket = PromotionTicket {
            ticket_id: Uuid::now_v7(),
            iteration_id: Uuid::now_v7(),
            submitted_at_utc: Utc::now(),
        };
        let err = adapter.require_approved(&fake_ticket).unwrap_err();
        assert!(matches!(err, GateError::UnknownTicket));
    }

    /// A buggy/malicious gate that detaches the issued ticket from the
    /// request's iteration_id. MT-154 finding #1: `LoopPromotionGate::submit`
    /// must reject this rather than propagate a ticket whose approval
    /// evidence could be inherited by a different iteration.
    struct MisbindingGate;
    impl PromotionGateSubmitter for MisbindingGate {
        fn submit(&self, _request: PromotionRequest) -> Result<PromotionTicket, GateError> {
            Ok(PromotionTicket {
                ticket_id: Uuid::now_v7(),
                iteration_id: Uuid::now_v7(), // detached from request.iteration_id
                submitted_at_utc: Utc::now(),
            })
        }
        fn poll(&self, _ticket: &PromotionTicket) -> Result<PromotionStatus, GateError> {
            unreachable!("submit must fail before poll is reached")
        }
    }

    #[test]
    fn submit_rejects_gate_returned_mismatched_iteration_id() {
        let gate = MisbindingGate;
        let adapter = LoopPromotionGate::new(&gate);
        let err = adapter.submit(sample_request()).unwrap_err();
        match err {
            GateError::Io { message } => {
                assert!(
                    message.contains("mismatched iteration_id"),
                    "expected mismatch message; got: {message}"
                );
            }
            other => panic!("expected GateError::Io for mismatched iteration_id; got {other:?}"),
        }
    }
}
