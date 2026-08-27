//! WP-KERNEL-005 MT-190..MT-194: DCC workflow panels + Flight Recorder
//! workflow events + reset/orphan diagnostic validation rows.
//!
//! These are TYPED RUNTIME surfaces (durable rows + EventLedger events),
//! never governance markdown. Storage authority is the single embedded
//! Handshake SurrealDB store; SQLite is forbidden (MT-004).
//!
//!   * MT-190 -- DCC Approvals + Visual-Capture panel projections: DCC
//!     visibility for approvals and visual captures only (projection, GUI
//!     later). One typed JSON row per panel instance, mirroring the MT-148
//!     pattern with its own closed panel-kind vocabulary.
//!   * MT-191..MT-193 -- Flight Recorder workflow events: one persisted row
//!     per FR workflow event, typed by
//!     [`crate::flight_recorder::workflow_event_kinds::FrWorkflowEventKind`]
//!     (tool call / proposal / apply decision; visual capture / validation /
//!     recovery; build guard / package guard / stale-doc detection). The
//!     payload is validated against the kind's required fields so an event
//!     can never land hollow, and every record emits an EventLedger event.
//!   * MT-194 -- reset/orphan diagnostic validation matrix: the
//!     `MT-194.reset-orphan` rows for the existing
//!     `atelier_diagnostics_validation_matrix` runtime surface (MT-171..175),
//!     grounded in the real reset/orphan product modules (intake reset +
//!     orphan adoption, kernel reset invariants, diagnostics reset/orphan
//!     projection). Recorded through the existing
//!     [`AtelierStore::record_diagnostics_validation_row`] write path. The
//!     kind constant lives here because `state_probe.rs` is owned by another
//!     microtask lane.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use surrealdb::types::{Datetime, SurrealValue};

use super::state_probe::{DiagnosticsValidationStatus, NewDiagnosticsValidationRow};
use super::{
    atelier_event_sql, reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore,
};
use crate::flight_recorder::workflow_event_kinds::FrWorkflowEventKind;

/// Event families for the MT-190..MT-193 DCC/Flight-Recorder surfaces.
///
/// Defined here so the parent module folds these into
/// [`super::event_family::ALL`].
pub mod dcc_flight_recorder_event_family {
    /// A DCC Approvals/Visual-Capture workflow panel projection row was
    /// recorded (MT-190).
    pub const DCC_WORKFLOW_PANEL_PROJECTION_RECORDED: &str =
        "atelier.dcc_flight_recorder.workflow_panel_projection_recorded";
    /// A Flight Recorder workflow event row was recorded (MT-191..MT-193).
    pub const FR_WORKFLOW_EVENT_RECORDED: &str =
        "atelier.dcc_flight_recorder.fr_workflow_event_recorded";

    /// All DCC/Flight-Recorder event families (parity/coverage helper).
    pub const ALL: &[&str] = &[
        DCC_WORKFLOW_PANEL_PROJECTION_RECORDED,
        FR_WORKFLOW_EVENT_RECORDED,
    ];
}

// ---------------------------------------------------------------------------
// MT-190: DCC Approvals + Visual-Capture panel projections.
// ---------------------------------------------------------------------------

/// Kind of DCC workflow panel a projection row carries (MT-190). Mirrors the
/// `panel_kind` literal type in the canonical schema. Deliberately disjoint
/// from the MT-148 [`super::state_probe::DccPanelKind`] vocabulary
/// (SESSION/LEASE/COMMAND_LOG/RECOVERY): MT-190 covers approvals and visual
/// captures only.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DccWorkflowPanelKind {
    /// Pending/decided approvals visible to a no-context model.
    Approvals,
    /// Visual captures (screenshots/snapshots) visible to a no-context model.
    VisualCapture,
}

impl DccWorkflowPanelKind {
    pub fn as_token(self) -> &'static str {
        match self {
            DccWorkflowPanelKind::Approvals => "APPROVALS",
            DccWorkflowPanelKind::VisualCapture => "VISUAL_CAPTURE",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "APPROVALS" => Ok(DccWorkflowPanelKind::Approvals),
            "VISUAL_CAPTURE" => Ok(DccWorkflowPanelKind::VisualCapture),
            other => Err(AtelierError::Validation(format!(
                "unknown DCC workflow panel kind: {other}"
            ))),
        }
    }

    /// Both panel kinds, for seeding/coverage.
    pub const ALL: &'static [DccWorkflowPanelKind] = &[
        DccWorkflowPanelKind::Approvals,
        DccWorkflowPanelKind::VisualCapture,
    ];
}

/// Input to record one DCC workflow panel projection row (MT-190).
#[derive(Clone, Debug)]
pub struct NewDccWorkflowPanelProjection {
    /// Stable panel id (the PK). MUST be non-empty / unpadded.
    pub panel_id: String,
    pub panel_kind: DccWorkflowPanelKind,
    /// Typed panel state (approvals/visual-capture payload). MUST be a JSON
    /// object, never a bare scalar.
    pub state_json: Value,
}

/// Persisted DCC workflow panel projection row (MT-190).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DccWorkflowPanelProjection {
    pub panel_id: String,
    pub panel_kind: DccWorkflowPanelKind,
    pub state_json: Value,
    pub created_at_utc: DateTime<Utc>,
}

/// One `atelier_dcc_workflow_panel_projection` row as the store returns it.
#[derive(SurrealValue)]
struct DccWorkflowPanelProjectionRow {
    panel_id: String,
    panel_kind: String,
    state_json: Value,
    created_at_utc: Datetime,
}

impl TryFrom<DccWorkflowPanelProjectionRow> for DccWorkflowPanelProjection {
    type Error = AtelierError;

    fn try_from(row: DccWorkflowPanelProjectionRow) -> AtelierResult<Self> {
        Ok(DccWorkflowPanelProjection {
            panel_id: row.panel_id,
            panel_kind: DccWorkflowPanelKind::from_token(&row.panel_kind)?,
            state_json: row.state_json,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

fn validate_dcc_workflow_panel_projection(
    new: &NewDccWorkflowPanelProjection,
) -> AtelierResult<()> {
    if new.panel_id.trim().is_empty() || new.panel_id.trim() != new.panel_id {
        return Err(AtelierError::Validation(
            "dcc workflow panel projection panel_id must be non-empty and unpadded".into(),
        ));
    }
    if !new.state_json.is_object() {
        return Err(AtelierError::Validation(
            "dcc workflow panel projection state_json must be a JSON object".into(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// MT-191 / MT-192 / MT-193: Flight Recorder workflow event records.
// ---------------------------------------------------------------------------

/// Input to record one Flight Recorder workflow event (MT-191..MT-193).
#[derive(Clone, Debug)]
pub struct NewFrWorkflowEvent {
    /// Stable record id (the PK). MUST be non-empty / unpadded.
    pub record_id: String,
    /// Typed event kind; the owning microtask (`mt_owner`) is derived from it.
    pub event_kind: FrWorkflowEventKind,
    /// Optional governed session ref (no .GOV / SQLite / localhost / local path).
    pub session_ref: Option<String>,
    /// Event payload. MUST be a JSON object carrying every
    /// [`FrWorkflowEventKind::required_payload_fields`] entry as a non-empty
    /// string, so an event can never land hollow.
    pub payload: Value,
}

/// Persisted Flight Recorder workflow event row (MT-191..MT-193).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrWorkflowEvent {
    pub record_id: String,
    pub event_kind: FrWorkflowEventKind,
    /// The owning microtask, derived from `event_kind` and re-checked by the
    /// pairing CHECK in migration 0116.
    pub mt_owner: String,
    pub session_ref: Option<String>,
    pub payload: Value,
    pub created_at_utc: DateTime<Utc>,
}

/// One `atelier_fr_workflow_event` row as the store returns it.
#[derive(SurrealValue)]
struct FrWorkflowEventRow {
    record_id: String,
    event_kind: String,
    mt_owner: String,
    session_ref: Option<String>,
    payload: Value,
    created_at_utc: Datetime,
}

impl TryFrom<FrWorkflowEventRow> for FrWorkflowEvent {
    type Error = AtelierError;

    fn try_from(row: FrWorkflowEventRow) -> AtelierResult<Self> {
        let event_kind = FrWorkflowEventKind::from_str_id(&row.event_kind).map_err(|err| {
            AtelierError::Validation(format!("persisted FR workflow event kind invalid: {err}"))
        })?;
        Ok(FrWorkflowEvent {
            record_id: row.record_id,
            event_kind,
            mt_owner: row.mt_owner,
            session_ref: row.session_ref,
            payload: row.payload,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

fn validate_fr_workflow_event(new: &NewFrWorkflowEvent) -> AtelierResult<()> {
    if new.record_id.trim().is_empty() || new.record_id.trim() != new.record_id {
        return Err(AtelierError::Validation(
            "fr workflow event record_id must be non-empty and unpadded".into(),
        ));
    }
    if let Some(session_ref) = &new.session_ref {
        reject_legacy_runtime_ref("session_ref", session_ref)?;
    }
    let Some(payload) = new.payload.as_object() else {
        return Err(AtelierError::Validation(
            "fr workflow event payload must be a JSON object".into(),
        ));
    };
    for field in new.event_kind.required_payload_fields() {
        let present = payload
            .get(*field)
            .and_then(Value::as_str)
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !present {
            return Err(AtelierError::Validation(format!(
                "fr workflow event {} payload must carry non-empty string field `{field}`",
                new.event_kind.as_str()
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// MT-194: reset + orphan diagnostic validation-matrix rows.
// ---------------------------------------------------------------------------

/// Matrix kind for the MT-194 reset/orphan diagnostics validation rows.
///
/// Re-exports the canonical constant from
/// [`super::state_probe::diagnostics_validation_matrix_kind`] so the two
/// lanes can never drift apart on the kind string.
pub mod reset_orphan_validation_matrix_kind {
    /// MT-194: reset and orphan diagnostics.
    pub use super::super::state_probe::diagnostics_validation_matrix_kind::RESET_ORPHAN;
}

/// One helper to build a COVERED MT-194 row that cites a real product module.
fn covered_reset_orphan_row(
    row_id: &str,
    surface: &str,
    check_id: &str,
    requirement: &str,
    evidence_ref: &str,
) -> NewDiagnosticsValidationRow {
    NewDiagnosticsValidationRow {
        row_id: row_id.to_string(),
        matrix_kind: reset_orphan_validation_matrix_kind::RESET_ORPHAN.to_string(),
        surface: surface.to_string(),
        check_id: check_id.to_string(),
        requirement: requirement.to_string(),
        status: DiagnosticsValidationStatus::Covered,
        evidence_ref: Some(evidence_ref.to_string()),
    }
}

/// The real MT-194 reset/orphan diagnostics validation-matrix rows.
///
/// Mirrors [`super::state_probe::diagnostics_validation_matrix_catalog`]
/// (MT-171..175): every row names a real surface + check and cites a product
/// module that exists in the source tree, so a test can persist this catalog
/// through the existing `record_diagnostics_validation_row` write path and
/// reload it to prove the runtime surface.
pub fn reset_orphan_diagnostics_validation_catalog() -> Vec<NewDiagnosticsValidationRow> {
    vec![
        covered_reset_orphan_row(
            "mt-194.reset.operation-recorded",
            "intake_reset",
            "reset.operation_recorded",
            "A destructive reset must persist a typed atelier_reset_operation row so the reset is observable after the fact.",
            "src/backend/handshake_core/src/atelier/intake.rs",
        ),
        covered_reset_orphan_row(
            "mt-194.reset.event-emitted",
            "intake_reset",
            "reset.event_emitted",
            "Recording a reset must emit the RESET_RECORDED EventLedger family so no-context models can audit resets.",
            "src/backend/handshake_core/src/atelier/intake.rs",
        ),
        covered_reset_orphan_row(
            "mt-194.reset.invariants-enforced",
            "kernel_reset",
            "reset.invariants_enforced",
            "Kernel reset invariants must be enforced so a reset can never leave the control plane in a forbidden state.",
            "src/backend/handshake_core/src/kernel/reset_invariants.rs",
        ),
        covered_reset_orphan_row(
            "mt-194.orphan.manifest-recorded",
            "intake_orphan",
            "orphan.manifest_recorded",
            "Orphaned items surviving a reset must be captured in a typed atelier_orphan_manifest row, never silently dropped.",
            "src/backend/handshake_core/src/atelier/intake.rs",
        ),
        covered_reset_orphan_row(
            "mt-194.orphan.item-adoption",
            "intake_orphan",
            "orphan.item_adoption",
            "Each orphan manifest item must support adoption with an ORPHAN_MANIFEST_ITEM_ADOPTED EventLedger trail.",
            "src/backend/handshake_core/src/atelier/intake.rs",
        ),
        covered_reset_orphan_row(
            "mt-194.reset-orphan.diagnostics-projection",
            "diagnostics",
            "reset_orphan.diagnostics_projection",
            "Reset and orphan state must project into the diagnostics surface (DIAGNOSTICS_RESET_ORPHAN_PROJECTED) for no-context models.",
            "src/backend/handshake_core/src/atelier/command_corpus.rs",
        ),
    ]
}

#[derive(Clone, SurrealValue)]
struct RecordPanelProjectionBindings {
    panel_id: String,
    panel_kind: String,
    state_json: Value,
}

/// Upsert one panel projection keyed on `panel_id` and append its event in
/// the same atomic statement. `created_at_utc` comes from the schema default
/// and survives replays, matching the former conflict-update contract.
const RECORD_PANEL_PROJECTION_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = type::record('atelier_dcc_workflow_panel_projection', $domain.panel_id); ",
    atelier_event_sql!(),
    " RETURN (UPSERT $rid SET \
         panel_id = $domain.panel_id, \
         panel_kind = $domain.panel_kind, \
         state_json = $domain.state_json \
       RETURN AFTER); };"
);

#[derive(SurrealValue)]
struct PanelKindBinding {
    panel_kind: String,
}

const LIST_PANEL_PROJECTIONS_STATEMENT: &str =
    "SELECT panel_id, panel_kind, state_json, created_at_utc \
     FROM atelier_dcc_workflow_panel_projection \
     WHERE panel_kind = $panel_kind \
     ORDER BY created_at_utc DESC, panel_id ASC;";

#[derive(Clone, SurrealValue)]
struct RecordFrWorkflowEventBindings {
    record_id: String,
    event_kind: String,
    mt_owner: String,
    session_ref: Option<String>,
    payload: Value,
}

const RECORD_FR_WORKFLOW_EVENT_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = type::record('atelier_fr_workflow_event', $domain.record_id); ",
    atelier_event_sql!(),
    " RETURN (UPSERT $rid SET \
         record_id = $domain.record_id, \
         event_kind = $domain.event_kind, \
         mt_owner = $domain.mt_owner, \
         session_ref = $domain.session_ref, \
         payload = $domain.payload \
       RETURN AFTER); };"
);

#[derive(SurrealValue)]
struct EventKindBinding {
    event_kind: String,
}

const LIST_FR_WORKFLOW_EVENTS_BY_KIND_STATEMENT: &str =
    "SELECT record_id, event_kind, mt_owner, session_ref, payload, created_at_utc \
     FROM atelier_fr_workflow_event \
     WHERE event_kind = $event_kind \
     ORDER BY created_at_utc DESC, record_id ASC;";

#[derive(SurrealValue)]
struct MtOwnerBinding {
    mt_owner: String,
}

const LIST_FR_WORKFLOW_EVENTS_BY_OWNER_STATEMENT: &str =
    "SELECT record_id, event_kind, mt_owner, session_ref, payload, created_at_utc \
     FROM atelier_fr_workflow_event \
     WHERE mt_owner = $mt_owner \
     ORDER BY created_at_utc DESC, record_id ASC;";

impl AtelierStore {
    /// Record one DCC workflow panel projection row (MT-190).
    ///
    /// Idempotent on `panel_id`. Emits
    /// `DCC_WORKFLOW_PANEL_PROJECTION_RECORDED`.
    pub async fn record_dcc_workflow_panel_projection(
        &self,
        new: &NewDccWorkflowPanelProjection,
    ) -> AtelierResult<DccWorkflowPanelProjection> {
        validate_dcc_workflow_panel_projection(new)?;

        let bindings = RecordPanelProjectionBindings {
            panel_id: new.panel_id.clone(),
            panel_kind: new.panel_kind.as_token().to_owned(),
            state_json: new.state_json.clone(),
        };
        let row: Option<DccWorkflowPanelProjectionRow> = self
            .write_with_event(
                RECORD_PANEL_PROJECTION_STATEMENT,
                bindings,
                dcc_flight_recorder_event_family::DCC_WORKFLOW_PANEL_PROJECTION_RECORDED,
                "atelier_dcc_workflow_panel_projection",
                &new.panel_id,
                json!({
                    "panel_id": new.panel_id,
                    "panel_kind": new.panel_kind.as_token(),
                    "schema": "hsk.atelier.dcc_workflow_panel_projection@1",
                }),
            )
            .await?;
        row.ok_or_else(|| {
            AtelierError::Internal(
                "recording a dcc workflow panel projection returned no row".to_owned(),
            )
        })?
        .try_into()
    }

    /// List DCC workflow panel projection rows for a given panel kind, newest
    /// first (MT-190).
    pub async fn list_dcc_workflow_panel_projections_by_kind(
        &self,
        panel_kind: DccWorkflowPanelKind,
    ) -> AtelierResult<Vec<DccWorkflowPanelProjection>> {
        let bindings = PanelKindBinding {
            panel_kind: panel_kind.as_token().to_owned(),
        };
        let rows: Vec<DccWorkflowPanelProjectionRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_PANEL_PROJECTIONS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(DccWorkflowPanelProjection::try_from)
            .collect()
    }

    /// Record one Flight Recorder workflow event (MT-191..MT-193).
    ///
    /// The payload is validated against the kind's required fields and the
    /// owning microtask is derived from the kind. Idempotent on `record_id`.
    /// Emits `FR_WORKFLOW_EVENT_RECORDED`.
    pub async fn record_fr_workflow_event(
        &self,
        new: &NewFrWorkflowEvent,
    ) -> AtelierResult<FrWorkflowEvent> {
        validate_fr_workflow_event(new)?;

        let bindings = RecordFrWorkflowEventBindings {
            record_id: new.record_id.clone(),
            event_kind: new.event_kind.as_str().to_owned(),
            mt_owner: new.event_kind.mt_owner().to_owned(),
            session_ref: new.session_ref.clone(),
            payload: new.payload.clone(),
        };
        let row: Option<FrWorkflowEventRow> = self
            .write_with_event(
                RECORD_FR_WORKFLOW_EVENT_STATEMENT,
                bindings,
                dcc_flight_recorder_event_family::FR_WORKFLOW_EVENT_RECORDED,
                "atelier_fr_workflow_event",
                &new.record_id,
                json!({
                    "record_id": new.record_id,
                    "event_kind": new.event_kind.as_str(),
                    "mt_owner": new.event_kind.mt_owner(),
                    "subsystem": new.event_kind.subsystem(),
                    "schema": "hsk.atelier.fr_workflow_event@1",
                }),
            )
            .await?;
        row.ok_or_else(|| {
            AtelierError::Internal("recording a fr workflow event returned no row".to_owned())
        })?
        .try_into()
    }

    /// List Flight Recorder workflow events of one kind, newest first
    /// (MT-191..MT-193).
    pub async fn list_fr_workflow_events_by_kind(
        &self,
        event_kind: FrWorkflowEventKind,
    ) -> AtelierResult<Vec<FrWorkflowEvent>> {
        let bindings = EventKindBinding {
            event_kind: event_kind.as_str().to_owned(),
        };
        let rows: Vec<FrWorkflowEventRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_FR_WORKFLOW_EVENTS_BY_KIND_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        rows.into_iter().map(FrWorkflowEvent::try_from).collect()
    }

    /// List Flight Recorder workflow events owned by one microtask
    /// (`MT-191` / `MT-192` / `MT-193`), newest first.
    pub async fn list_fr_workflow_events_by_mt_owner(
        &self,
        mt_owner: &str,
    ) -> AtelierResult<Vec<FrWorkflowEvent>> {
        let bindings = MtOwnerBinding {
            mt_owner: mt_owner.to_owned(),
        };
        let rows: Vec<FrWorkflowEventRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_FR_WORKFLOW_EVENTS_BY_OWNER_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        rows.into_iter().map(FrWorkflowEvent::try_from).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_kind_round_trips_and_rejects_unknown() {
        for kind in DccWorkflowPanelKind::ALL.iter().copied() {
            let token = kind.as_token();
            let back = DccWorkflowPanelKind::from_token(token).expect("round-trip");
            assert_eq!(kind, back);
        }
        assert!(DccWorkflowPanelKind::from_token("SESSION").is_err());
        assert!(DccWorkflowPanelKind::from_token("approvals").is_err());
    }

    #[test]
    fn panel_projection_rejects_padded_id_and_scalar_state() {
        let padded = NewDccWorkflowPanelProjection {
            panel_id: " dcc-approvals-1".to_string(),
            panel_kind: DccWorkflowPanelKind::Approvals,
            state_json: json!({}),
        };
        assert!(validate_dcc_workflow_panel_projection(&padded).is_err());

        let scalar = NewDccWorkflowPanelProjection {
            panel_id: "dcc-approvals-1".to_string(),
            panel_kind: DccWorkflowPanelKind::Approvals,
            state_json: json!("not-an-object"),
        };
        assert!(validate_dcc_workflow_panel_projection(&scalar).is_err());
    }

    #[test]
    fn fr_workflow_event_rejects_missing_required_payload_fields() {
        let missing = NewFrWorkflowEvent {
            record_id: "frwf-1".to_string(),
            event_kind: FrWorkflowEventKind::ToolApplyDecision,
            session_ref: None,
            payload: json!({ "proposal_id": "prop-1" }), // missing `decision`
        };
        let err = validate_fr_workflow_event(&missing).expect_err("must reject");
        assert!(err.to_string().contains("decision"), "{err}");

        let blank = NewFrWorkflowEvent {
            record_id: "frwf-2".to_string(),
            event_kind: FrWorkflowEventKind::StaleDocDetected,
            session_ref: None,
            payload: json!({ "doc_ref": "  ", "staleness_kind": "hash_mismatch" }),
        };
        assert!(validate_fr_workflow_event(&blank).is_err());
    }

    #[test]
    fn fr_workflow_event_rejects_machine_local_session_ref() {
        let bad = NewFrWorkflowEvent {
            record_id: "frwf-3".to_string(),
            event_kind: FrWorkflowEventKind::ToolCall,
            session_ref: Some("C:/Users/op/session.json".to_string()),
            payload: json!({ "tool_id": "fs.read", "status": "ok" }),
        };
        assert!(validate_fr_workflow_event(&bad).is_err());
    }

    #[test]
    fn mt194_catalog_rows_are_well_formed() {
        let catalog = reset_orphan_diagnostics_validation_catalog();
        assert!(
            catalog.len() >= 6,
            "MT-194 catalog must carry the reset + orphan checks"
        );
        for row in &catalog {
            assert_eq!(
                row.matrix_kind,
                reset_orphan_validation_matrix_kind::RESET_ORPHAN
            );
            assert!(row.row_id.starts_with("mt-194."), "{}", row.row_id);
            assert!(!row.requirement.trim().is_empty());
            assert!(row.evidence_ref.is_some());
        }
        let reset_rows = catalog
            .iter()
            .filter(|r| r.row_id.contains(".reset"))
            .count();
        let orphan_rows = catalog
            .iter()
            .filter(|r| r.row_id.contains("orphan"))
            .count();
        assert!(reset_rows >= 3, "must cover reset diagnostics");
        assert!(orphan_rows >= 2, "must cover orphan diagnostics");
    }
}
