//! MT-192 EventLedger replay reconstruction primitive.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::flight_recorder::{
    fr_event_registry::FrEventId, FlightRecorder, FlightRecorderActor, FlightRecorderEvent,
    FlightRecorderEventType,
};

use super::checkpoint::SessionCheckpoint;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventLedgerRow {
    pub event_id: String,
    pub event_sequence: i64,
    pub session_id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayPlan {
    pub from_checkpoint: SessionCheckpoint,
    pub events_to_replay: Vec<EventLedgerRow>,
    pub expected_final_seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReplayError {
    #[error("event not applicable at seq {seq}: {reason}")]
    EventNotApplicable { seq: i64, reason: String },
    #[error("state invariant violated at seq {seq}: {invariant}")]
    StateInvariantViolated { seq: i64, invariant: String },
    #[error("missing event at gap seq {gap_at_seq}")]
    MissingEvent { gap_at_seq: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayProgress {
    pub applied: u32,
    pub last_seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayResult<State: Clone + serde::Serialize> {
    pub final_state: State,
    pub final_seq: i64,
    pub applied_count: u32,
}

pub struct StateReplayer;

impl StateReplayer {
    /// Build a replay plan from a checkpoint and a slice of event rows. Events
    /// are filtered to those with `seq > checkpoint.last_event_ledger_seq` and
    /// matching `session_id`.
    pub fn plan(
        checkpoint: SessionCheckpoint,
        all_events_for_session: &[EventLedgerRow],
    ) -> ReplayPlan {
        let last = checkpoint.last_event_ledger_seq;
        let mut events: Vec<EventLedgerRow> = all_events_for_session
            .iter()
            .filter(|e| e.event_sequence > last && e.session_id == checkpoint.session_id)
            .cloned()
            .collect();
        events.sort_by_key(|e| e.event_sequence);
        let expected_final_seq = events.last().map(|e| e.event_sequence).unwrap_or(last);
        ReplayPlan {
            from_checkpoint: checkpoint,
            events_to_replay: events,
            expected_final_seq,
        }
    }

    /// Apply each event in seq order. The applicator returns
    /// `Result<(), ReplayError>` per event.
    pub fn execute<State, F>(
        plan: ReplayPlan,
        mut state: State,
        mut applier: F,
    ) -> Result<ReplayResult<State>, ReplayError>
    where
        State: Clone + serde::Serialize,
        F: FnMut(&mut State, &EventLedgerRow) -> Result<(), ReplayError>,
    {
        let mut prev_seq = plan.from_checkpoint.last_event_ledger_seq;
        let mut applied: u32 = 0;
        for ev in &plan.events_to_replay {
            if ev.event_sequence != prev_seq + 1 {
                return Err(ReplayError::MissingEvent {
                    gap_at_seq: prev_seq + 1,
                });
            }
            applier(&mut state, ev)?;
            prev_seq = ev.event_sequence;
            applied += 1;
        }
        Ok(ReplayResult {
            final_state: state,
            final_seq: prev_seq,
            applied_count: applied,
        })
    }

    pub async fn execute_with_flight_recorder<State, F>(
        plan: ReplayPlan,
        mut state: State,
        mut applier: F,
        recorder: &dyn FlightRecorder,
    ) -> Result<ReplayResult<State>, ReplayError>
    where
        State: Clone + serde::Serialize,
        F: FnMut(&mut State, &EventLedgerRow) -> Result<(), ReplayError>,
    {
        let session_id = plan.from_checkpoint.session_id;
        let from_seq = plan.from_checkpoint.last_event_ledger_seq;
        let expected_to_seq = plan.expected_final_seq;
        record_replay_event(
            recorder,
            FrEventId::ReplayStarted,
            session_id,
            from_seq,
            expected_to_seq,
        )
        .await;

        let mut prev_seq = from_seq;
        let mut applied: u32 = 0;
        for ev in &plan.events_to_replay {
            if ev.event_sequence != prev_seq + 1 {
                let err = ReplayError::MissingEvent {
                    gap_at_seq: prev_seq + 1,
                };
                record_replay_event(
                    recorder,
                    FrEventId::ReplayFailed,
                    session_id,
                    from_seq,
                    replay_error_seq(&err),
                )
                .await;
                return Err(err);
            }
            if let Err(err) = applier(&mut state, ev) {
                record_replay_event(
                    recorder,
                    FrEventId::ReplayFailed,
                    session_id,
                    from_seq,
                    replay_error_seq(&err),
                )
                .await;
                return Err(err);
            }
            prev_seq = ev.event_sequence;
            applied += 1;
            record_replay_event(
                recorder,
                FrEventId::ReplayProgress,
                session_id,
                from_seq,
                prev_seq,
            )
            .await;
        }

        record_replay_event(
            recorder,
            FrEventId::ReplayCompleted,
            session_id,
            from_seq,
            prev_seq,
        )
        .await;
        Ok(ReplayResult {
            final_state: state,
            final_seq: prev_seq,
            applied_count: applied,
        })
    }
}

fn replay_error_seq(error: &ReplayError) -> i64 {
    match error {
        ReplayError::EventNotApplicable { seq, .. }
        | ReplayError::StateInvariantViolated { seq, .. } => *seq,
        ReplayError::MissingEvent { gap_at_seq } => *gap_at_seq,
    }
}

async fn record_replay_event(
    recorder: &dyn FlightRecorder,
    event_id: FrEventId,
    session_id: Uuid,
    from_seq: i64,
    to_seq: i64,
) {
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::System,
        FlightRecorderActor::System,
        session_id,
        serde_json::json!({
            "schema_version": "hsk.fr.session_checkpoint_replay@1",
            "event_id": event_id.as_str(),
            "session_id": session_id.to_string(),
            "from_seq": from_seq,
            "to_seq": to_seq,
        }),
    )
    .with_actor_id("session_checkpoint_replay");
    let _ = recorder.record_event(event).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_checkpoint::checkpoint::{CheckpointStateKind, SessionCheckpoint};

    fn make_checkpoint(session_id: Uuid, last_seq: i64) -> SessionCheckpoint {
        SessionCheckpoint::new(
            session_id,
            Uuid::now_v7(),
            last_seq,
            serde_json::json!({"counter": 0}),
            CheckpointStateKind::Periodic,
        )
        .unwrap()
    }

    fn make_event(session: Uuid, seq: i64, value: i64) -> EventLedgerRow {
        EventLedgerRow {
            event_id: format!("E-{seq}"),
            event_sequence: seq,
            session_id: session,
            event_type: "increment".to_string(),
            payload: serde_json::json!({"by": value}),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn replay_applies_events_in_seq_order() {
        let s = Uuid::now_v7();
        let cp = make_checkpoint(s, 0);
        let events = vec![
            make_event(s, 1, 10),
            make_event(s, 2, 20),
            make_event(s, 3, 30),
        ];
        let plan = StateReplayer::plan(cp, &events);
        let mut state = serde_json::json!({"counter": 0});
        let result = StateReplayer::execute(plan, state.clone(), |st, ev| {
            let by = ev.payload.get("by").and_then(|v| v.as_i64()).unwrap();
            let c = st.get("counter").and_then(|v| v.as_i64()).unwrap();
            *st = serde_json::json!({"counter": c + by});
            Ok(())
        })
        .unwrap();
        assert_eq!(result.applied_count, 3);
        assert_eq!(result.final_seq, 3);
        assert_eq!(
            result.final_state.get("counter").and_then(|v| v.as_i64()),
            Some(60)
        );
        let _ = state;
    }

    #[test]
    fn replay_detects_gap() {
        let s = Uuid::now_v7();
        let cp = make_checkpoint(s, 0);
        // Skip seq=2.
        let events = vec![make_event(s, 1, 10), make_event(s, 3, 30)];
        let plan = StateReplayer::plan(cp, &events);
        let r: Result<ReplayResult<serde_json::Value>, ReplayError> =
            StateReplayer::execute(plan, serde_json::json!({}), |_, _| Ok(()));
        assert!(matches!(
            r,
            Err(ReplayError::MissingEvent { gap_at_seq: 2 })
        ));
    }

    #[test]
    fn replay_filter_other_sessions() {
        let s = Uuid::now_v7();
        let other = Uuid::now_v7();
        let cp = make_checkpoint(s, 0);
        let events = vec![make_event(s, 1, 10), make_event(other, 2, 20)];
        let plan = StateReplayer::plan(cp, &events);
        assert_eq!(plan.events_to_replay.len(), 1);
        assert_eq!(plan.expected_final_seq, 1);
    }
}
