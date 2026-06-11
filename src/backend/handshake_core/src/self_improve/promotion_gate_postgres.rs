//! MT-170 remediation: production [`PromotionGateSubmitter`] backed by the
//! kernel EventLedger (PostgreSQL).
//!
//! Integration validation v2 found that the operator-review pause invariant
//! (apply must not run while a ticket is Pending/Rejected) was only ever
//! proven against in-memory mock gates. This module gives the self-improve
//! loop a durable gate: tickets and operator decisions are kernel event
//! ledger rows, so the review state survives process restarts and a fresh
//! gate instance re-reads the same persisted state (the pause persists no
//! matter who polls).
//!
//! Ledger model (aggregate type [`PROMOTION_TICKET_AGGREGATE_TYPE`],
//! aggregate id = `ticket_id`):
//!  - `PROMOTION_REQUESTED` event with `status="pending"` at submit time,
//!    carrying the full [`PromotionRequest`] evidence bundle for the
//!    reviewer.
//!  - `PROMOTION_ACCEPTED` / `PROMOTION_REJECTED` event with
//!    `status="approved"` / `status="rejected"` when the operator decides.
//!    The latest event by `event_sequence` is the authoritative status.
//!
//! Sub-rule 1 compliance: no placeholders; storage failures surface as the
//! typed [`GateError::Io`] variant.

use std::sync::Arc;

use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use super::promotion_gate_adapter::{
    GateError, PromotionApproval, PromotionGateSubmitter, PromotionRejection, PromotionRequest,
    PromotionStatus, PromotionTicket,
};
use crate::kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent};
use crate::memory::persistence_postgres::block_on;
use crate::storage::{Database, StorageError};

/// Aggregate type for self-improve promotion review tickets in the kernel
/// event ledger. The aggregate id is the `ticket_id`.
pub const PROMOTION_TICKET_AGGREGATE_TYPE: &str = "self_improve_promotion_ticket";

/// Source component label written to `kernel_event_ledger.source_component`
/// so operators can filter MT-170 promotion-gate traffic in queries.
pub const PROMOTION_GATE_SOURCE_COMPONENT: &str = "self_improve_promotion_gate_postgres";

/// Payload schema for persisted promotion ticket events.
pub const PROMOTION_TICKET_PAYLOAD_SCHEMA_ID: &str = "hsk.self_improve.promotion_ticket@1";

/// Durable PostgreSQL-backed promotion gate.
///
/// Sync trait over async storage: uses the shared Tokio bridge, so async
/// tests must run on a multi-thread runtime
/// (`#[tokio::test(flavor = "multi_thread")]`).
pub struct PostgresPromotionGate {
    db: Arc<dyn Database>,
}

impl PostgresPromotionGate {
    pub fn with_db(db: Arc<dyn Database>) -> Self {
        Self { db }
    }

    /// Record the operator's approval for a pending ticket. Fails typed if
    /// the ticket is unknown or already decided (no silent double-decision).
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

    /// Record the operator's rejection for a pending ticket. Fails typed if
    /// the ticket is unknown or already decided.
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
        // Guard: the ticket must exist and still be pending. Re-reads the
        // persisted state so a stale caller cannot overwrite a decision.
        match self.poll(ticket)? {
            PromotionStatus::Pending { .. } => {}
            PromotionStatus::Approved { .. } | PromotionStatus::Rejected { .. } => {
                return Err(GateError::Io {
                    message: format!(
                        "promotion ticket {} is already decided; double-decision rejected",
                        ticket.ticket_id
                    ),
                });
            }
        }

        let mut payload = json!({
            "schema_id": PROMOTION_TICKET_PAYLOAD_SCHEMA_ID,
            "status": status_label,
            "ticket": ticket,
        });
        if let (Value::Object(target), Value::Object(fields)) =
            (&mut payload, decision_fields)
        {
            target.extend(fields);
        }

        let event = NewKernelEvent::builder(
            format!("KTR-SELF-IMPROVE-PROMOTION-{}", ticket.ticket_id),
            format!("SR-SELF-IMPROVE-PROMOTION-{}", ticket.ticket_id),
            event_type,
            KernelActor::PromotionGate("self_improve_loop".to_string()),
        )
        .aggregate(PROMOTION_TICKET_AGGREGATE_TYPE, ticket.ticket_id.to_string())
        .idempotency_key(format!(
            "self_improve_promotion_decision:{}:{status_label}",
            ticket.ticket_id
        ))
        .correlation_id(ticket.iteration_id.to_string())
        .event_version("kernel_event_v1")
        .source_component(PROMOTION_GATE_SOURCE_COMPONENT)
        .payload(payload)
        .build()
        .map_err(|err| GateError::Io {
            message: format!("promotion decision event build failed: {err}"),
        })?;

        let db = Arc::clone(&self.db);
        match block_on(async move { db.append_kernel_event(event).await }) {
            Ok(_) => Ok(()),
            Err(error) if is_idempotency_conflict(&error) => Ok(()),
            Err(error) => Err(GateError::Io {
                message: format!(
                    "appending promotion decision to kernel_event_ledger failed: {error}"
                ),
            }),
        }
    }

    fn ticket_events(&self, ticket_id: Uuid) -> Result<Vec<KernelEvent>, GateError> {
        let db = Arc::clone(&self.db);
        let aggregate_id = ticket_id.to_string();
        block_on(async move {
            db.list_kernel_events_for_aggregate(PROMOTION_TICKET_AGGREGATE_TYPE, &aggregate_id)
                .await
        })
        .map_err(|error| GateError::Io {
            message: format!(
                "reading promotion ticket from kernel_event_ledger failed: {error}"
            ),
        })
    }
}

fn is_idempotency_conflict(error: &StorageError) -> bool {
    matches!(
        error,
        StorageError::Validation(message) if *message == "kernel event idempotency conflict"
    )
}

fn decode_status(payload: &Value) -> Option<PromotionStatus> {
    if payload.get("schema_id")?.as_str()? != PROMOTION_TICKET_PAYLOAD_SCHEMA_ID {
        return None;
    }
    match payload.get("status")?.as_str()? {
        "pending" => {
            let ticket: PromotionTicket =
                serde_json::from_value(payload.get("ticket")?.clone()).ok()?;
            Some(PromotionStatus::Pending {
                submitted_at_utc: ticket.submitted_at_utc,
            })
        }
        "approved" => {
            let approval: PromotionApproval =
                serde_json::from_value(payload.get("approval")?.clone()).ok()?;
            Some(PromotionStatus::Approved { approval })
        }
        "rejected" => {
            let rejection: PromotionRejection =
                serde_json::from_value(payload.get("rejection")?.clone()).ok()?;
            Some(PromotionStatus::Rejected { rejection })
        }
        _ => None,
    }
}

impl PromotionGateSubmitter for PostgresPromotionGate {
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
            KernelActor::PromotionGate("self_improve_loop".to_string()),
        )
        .aggregate(PROMOTION_TICKET_AGGREGATE_TYPE, ticket.ticket_id.to_string())
        .idempotency_key(format!(
            "self_improve_promotion_submit:{}",
            ticket.ticket_id
        ))
        .correlation_id(ticket.iteration_id.to_string())
        .event_version("kernel_event_v1")
        .source_component(PROMOTION_GATE_SOURCE_COMPONENT)
        .payload(payload)
        .build()
        .map_err(|err| GateError::Io {
            message: format!("promotion ticket event build failed: {err}"),
        })?;

        let db = Arc::clone(&self.db);
        block_on(async move { db.append_kernel_event(event).await }).map_err(|error| {
            GateError::Io {
                message: format!(
                    "appending promotion ticket to kernel_event_ledger failed: {error}"
                ),
            }
        })?;
        Ok(ticket)
    }

    fn poll(&self, ticket: &PromotionTicket) -> Result<PromotionStatus, GateError> {
        let events = self.ticket_events(ticket.ticket_id)?;
        let mut latest: Option<PromotionStatus> = None;
        let mut latest_sequence = i64::MIN;
        for event in events {
            if let Some(status) = decode_status(&event.payload) {
                if event.event_sequence > latest_sequence {
                    latest_sequence = event.event_sequence;
                    latest = Some(status);
                }
            }
        }
        latest.ok_or(GateError::UnknownTicket)
    }
}
