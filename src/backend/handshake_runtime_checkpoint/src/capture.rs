//! Pure capture bookkeeping. Host-owned results, effects, authorization and cleanup remain external.
use std::sync::Arc;
use crate::claim::{
    validate_current, CanonicalId, ClaimDomain, ClaimError, ClaimSnapshot, Incarnation, MessageCursor,
    ReducedClaim, ReservationRecord,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureKind { Newline, FinalTail }

#[derive(PartialEq, Eq)]
pub struct CaptureCommand<I: Eq> {
    operation_id: CanonicalId,
    message_id: CanonicalId,
    idempotency_key: CanonicalId,
    original_owner: Incarnation,
    expected: ClaimSnapshot,
    payload_ref: CanonicalId,
    payload_sha256: [u8; 32],
    kind: CaptureKind,
    intent: I,
}

impl<I: Eq> CaptureCommand<I> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        operation_id: CanonicalId, message_id: CanonicalId, idempotency_key: CanonicalId,
        original_owner: Incarnation, expected: ClaimSnapshot, payload_ref: CanonicalId,
        payload_sha256: [u8; 32], kind: CaptureKind, intent: I,
    ) -> Result<Self, CaptureError> {
        if expected.head().incarnation() != &original_owner {
            return Err(CaptureError::CommandConflict);
        }
        Ok(Self { operation_id, message_id, idempotency_key, original_owner, expected,
            payload_ref, payload_sha256, kind, intent })
    }
    pub fn operation_id(&self) -> &CanonicalId { &self.operation_id }
    pub fn message_id(&self) -> &CanonicalId { &self.message_id }
    pub fn idempotency_key(&self) -> &CanonicalId { &self.idempotency_key }
    pub fn original_owner(&self) -> &Incarnation { &self.original_owner }
    pub fn expected(&self) -> &ClaimSnapshot { &self.expected }
    pub fn payload_ref(&self) -> &CanonicalId { &self.payload_ref }
    pub fn payload_sha256(&self) -> &[u8; 32] { &self.payload_sha256 }
    pub fn kind(&self) -> CaptureKind { self.kind }
    pub fn intent(&self) -> &I { &self.intent }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CaptureTicket {
    operation_id: CanonicalId,
    reservation_id: CanonicalId,
    original_owner: Incarnation,
}
impl CaptureTicket {
    pub fn operation_id(&self) -> &CanonicalId { &self.operation_id }
    pub fn reservation_id(&self) -> &CanonicalId { &self.reservation_id }
    pub fn original_owner(&self) -> &Incarnation { &self.original_owner }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapturePhase { Prepared, WriteInstalled, Pending, Settled, Unknown, Adopted, Staged, FlightRecorded }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reconciliation { HistoricalData, AbsentUnstarted, AbsentSettled, NewEffectConsistent, Unknown }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdoptionStatus { Adopted, AlreadyAdopted }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageStatus { Staged, AlreadyStaged }
#[derive(Debug, PartialEq, Eq)]
pub enum CaptureError {
    TicketMismatch, CommandConflict, CanonicalConflict, PhaseConflict, StaleMark,
    CursorConflict, CounterExhausted, Claim(ClaimError), Unknown,
}

/// A temporary view of the actual cached MESSAGE result, not a checkpoint or a result owner.
pub struct CanonicalCaptureRef<'a, I: Eq> {
    command: &'a CaptureCommand<I>,
    message_record: &'a [u8],
    ledger_event: &'a [u8],
    transaction_outcome: &'a [u8],
    observed: &'a ClaimSnapshot,
}
impl<'a, I: Eq> Copy for CanonicalCaptureRef<'a, I> {}
impl<'a, I: Eq> Clone for CanonicalCaptureRef<'a, I> { fn clone(&self) -> Self { *self } }
impl<'a, I: Eq> CanonicalCaptureRef<'a, I> {
    pub fn new(
        command: &'a CaptureCommand<I>, message_record: &'a [u8], ledger_event: &'a [u8],
        transaction_outcome: &'a [u8], observed: &'a ClaimSnapshot,
    ) -> Result<Self, CaptureError> {
        if message_record.is_empty() || ledger_event.is_empty() || transaction_outcome.is_empty() {
            return Err(CaptureError::CanonicalConflict);
        }
        Ok(Self { command, message_record, ledger_event, transaction_outcome, observed })
    }
    pub fn command(&self) -> &CaptureCommand<I> { self.command }
    pub fn message_record(&self) -> &[u8] { self.message_record }
    pub fn ledger_event(&self) -> &[u8] { self.ledger_event }
    pub fn transaction_outcome(&self) -> &[u8] { self.transaction_outcome }
    pub fn observed(&self) -> &ClaimSnapshot { self.observed }
    fn same_record(&self, other: &Self) -> bool {
        self.command == other.command && self.message_record == other.message_record
            && self.ledger_event == other.ledger_event
            && self.transaction_outcome == other.transaction_outcome && self.observed == other.observed
    }
}
pub enum CanonicalLookup<'a, I: Eq> { Committed(CanonicalCaptureRef<'a, I>), NotFound, Unavailable }

struct MarkBinding {
    instance: Arc<()>,
    ticket: CaptureTicket,
    phase: CapturePhase,
    revision: u64,
    cursor: MessageCursor,
}
pub struct AdoptionMark { binding: MarkBinding, observed: ClaimSnapshot }
pub struct StageMark { binding: MarkBinding, observed: ClaimSnapshot }

/// These flags describe supplied observations; they cannot establish physical task custody.
pub struct PreparedCapture<I: Eq> {
    instance: Arc<()>,
    reservation: ReservationRecord<CaptureCommand<I>>,
    ticket: CaptureTicket,
    transition_revision: u64,
    installed: bool,
    started: bool,
    settled: bool,
    unknown: bool,
    observed: Option<ClaimSnapshot>,
    staged: bool,
    flight_recorded: bool,
}
impl<I: Eq> PreparedCapture<I> {
    pub fn new(command: CaptureCommand<I>, original_ticket: CanonicalId) -> Self {
        let ticket = CaptureTicket {
            operation_id: command.operation_id.clone(), reservation_id: original_ticket.clone(),
            original_owner: command.original_owner.clone(),
        };
        let reservation = ReservationRecord::new(command.operation_id.clone(), original_ticket, command);
        Self { instance: Arc::new(()), reservation, ticket, transition_revision: 0, installed: false, started: false,
            settled: false, unknown: false, observed: None, staged: false, flight_recorded: false }
    }
    pub fn command(&self) -> &CaptureCommand<I> { self.reservation.command() }
    pub fn ticket(&self) -> &CaptureTicket { &self.ticket }
    pub fn phase(&self) -> CapturePhase {
        if self.flight_recorded { CapturePhase::FlightRecorded }
        else if self.staged { CapturePhase::Staged }
        else if self.observed.is_some() { CapturePhase::Adopted }
        else if self.unknown { CapturePhase::Unknown }
        else if self.settled { CapturePhase::Settled }
        else if self.started { CapturePhase::Pending }
        else if self.installed { CapturePhase::WriteInstalled }
        else { CapturePhase::Prepared }
    }
    /// Current unresolved DATA uncertainty, not history. Known settlement or matching
    /// reconciliation without an unsettled original write can resolve it.
    pub fn has_unknown(&self) -> bool { self.unknown }
    pub fn observed(&self) -> Option<&ClaimSnapshot> { self.observed.as_ref() }
    pub fn write_settled(&self) -> bool { self.settled }
    fn check_ticket(&self, ticket: &CaptureTicket) -> Result<(), CaptureError> {
        if ticket == &self.ticket { Ok(()) } else { Err(CaptureError::TicketMismatch) }
    }
    fn next_revision(&self) -> Result<u64, CaptureError> {
        self.transition_revision.checked_add(1).ok_or(CaptureError::CounterExhausted)
    }
    fn cursor(&self) -> MessageCursor {
        self.observed.as_ref().unwrap_or(self.command().expected()).cursor()
    }
    fn binding(&self) -> MarkBinding {
        MarkBinding { instance: Arc::clone(&self.instance), ticket: self.ticket.clone(), phase: self.phase(),
            revision: self.transition_revision, cursor: self.cursor() }
    }
    fn check_binding(&self, binding: &MarkBinding) -> Result<(), CaptureError> {
        if !Arc::ptr_eq(&binding.instance, &self.instance) || binding.ticket != self.ticket || binding.phase != self.phase()
            || binding.revision != self.transition_revision || binding.cursor != self.cursor() {
            Err(CaptureError::StaleMark)
        } else { Ok(()) }
    }
    pub fn mark_write_installed(&mut self, ticket: &CaptureTicket) -> Result<(), CaptureError> {
        self.check_ticket(ticket)?;
        if self.phase() != CapturePhase::Prepared { return Err(CaptureError::PhaseConflict); }
        let revision = self.next_revision()?;
        self.installed = true;
        self.transition_revision = revision;
        Ok(())
    }
    pub fn mark_started(&mut self, ticket: &CaptureTicket) -> Result<(), CaptureError> {
        self.check_ticket(ticket)?;
        if self.phase() != CapturePhase::WriteInstalled { return Err(CaptureError::PhaseConflict); }
        let revision = self.next_revision()?;
        self.started = true;
        self.transition_revision = revision;
        Ok(())
    }
    /// Records the host's actual known-settlement observation, including a repeated
    /// notification. This does not establish a canonical commit, absence or cleanup.
    pub fn mark_write_settled(&mut self, ticket: &CaptureTicket) -> Result<(), CaptureError> {
        self.check_ticket(ticket)?;
        if !self.installed { return Err(CaptureError::PhaseConflict); }
        if self.settled && !self.unknown { return Ok(()); }
        let revision = self.next_revision()?;
        self.reservation.observe_settled();
        self.settled = true;
        self.unknown = false;
        self.transition_revision = revision;
        Ok(())
    }
    pub fn mark_unknown(&mut self, ticket: &CaptureTicket) -> Result<(), CaptureError> {
        self.check_ticket(ticket)?;
        let revision = self.next_revision()?;
        self.reservation.observe_unknown();
        self.unknown = true;
        self.transition_revision = revision;
        Ok(())
    }
    fn validate_record(&self, canonical: CanonicalCaptureRef<'_, I>) -> Result<(), CaptureError> {
        if canonical.command != self.command() { return Err(CaptureError::CommandConflict); }
        let expected = self.command().expected();
        if canonical.observed.head() != expected.head()
            || canonical.observed.head().incarnation() != self.command().original_owner() {
            return Err(CaptureError::CanonicalConflict);
        }
        if canonical.observed.cursor().value() <= expected.cursor().value() {
            return Err(CaptureError::CursorConflict);
        }
        if let Some(observed) = &self.observed {
            if observed != canonical.observed { return Err(CaptureError::CanonicalConflict); }
        }
        Ok(())
    }
    pub fn evaluate_existing(&self, lookup: CanonicalLookup<'_, I>) -> Result<Reconciliation, CaptureError> {
        match lookup {
            CanonicalLookup::Committed(canonical) => {
                self.validate_record(canonical)?;
                Ok(Reconciliation::HistoricalData)
            }
            CanonicalLookup::Unavailable => Ok(Reconciliation::Unknown),
            CanonicalLookup::NotFound => {
                if self.observed.is_some() || self.unknown { Ok(Reconciliation::Unknown) }
                else if self.settled { Ok(Reconciliation::AbsentSettled) }
                else if self.installed || self.started { Ok(Reconciliation::Unknown) }
                else { Ok(Reconciliation::AbsentUnstarted) }
            }
        }
    }
    pub fn assess_new_effect(
        &self, current: &ReducedClaim, actual: &ClaimSnapshot, lookup: CanonicalLookup<'_, I>,
    ) -> Result<Reconciliation, CaptureError> {
        let existing = self.evaluate_existing(lookup)?;
        if existing != Reconciliation::AbsentUnstarted { return Ok(existing); }
        if self.phase() != CapturePhase::Prepared { return Err(CaptureError::PhaseConflict); }
        validate_current(current, actual, self.command().expected()).map_err(CaptureError::Claim)?;
        Ok(Reconciliation::NewEffectConsistent)
    }
    pub fn validate_adoption(
        &self, ticket: &CaptureTicket, cached: CanonicalCaptureRef<'_, I>,
        copied: CanonicalCaptureRef<'_, I>,
    ) -> Result<AdoptionMark, CaptureError> {
        self.check_ticket(ticket)?;
        self.validate_record(cached)?;
        self.validate_record(copied)?;
        if !cached.same_record(&copied) { return Err(CaptureError::CanonicalConflict); }
        Ok(AdoptionMark { binding: self.binding(), observed: cached.observed.clone() })
    }
    pub fn record_adoption(&mut self, mark: AdoptionMark) -> Result<AdoptionStatus, CaptureError> {
        self.check_binding(&mark.binding)?;
        if let Some(observed) = &self.observed {
            if observed != &mark.observed { return Err(CaptureError::CanonicalConflict); }
            // A fresh full-result match resolves uncertainty only without an unsettled write.
            if self.unknown && (self.settled || !self.installed) {
                let revision = self.next_revision()?;
                self.unknown = false;
                self.transition_revision = revision;
            }
            return Ok(AdoptionStatus::AlreadyAdopted);
        }
        let revision = self.next_revision()?;
        self.observed = Some(mark.observed);
        if self.settled || !self.installed { self.unknown = false; }
        self.transition_revision = revision;
        Ok(AdoptionStatus::Adopted)
    }
    pub fn validate_staging(
        &self, ticket: &CaptureTicket, canonical: CanonicalCaptureRef<'_, I>,
    ) -> Result<StageMark, CaptureError> {
        self.check_ticket(ticket)?;
        if self.observed.is_none() { return Err(CaptureError::PhaseConflict); }
        self.validate_record(canonical)?;
        Ok(StageMark { binding: self.binding(), observed: canonical.observed.clone() })
    }
    pub fn record_staged(&mut self, mark: StageMark) -> Result<StageStatus, CaptureError> {
        self.check_binding(&mark.binding)?;
        if self.observed.as_ref() != Some(&mark.observed) { return Err(CaptureError::CanonicalConflict); }
        if self.staged { return Ok(StageStatus::AlreadyStaged); }
        let revision = self.next_revision()?;
        self.staged = true;
        self.transition_revision = revision;
        Ok(StageStatus::Staged)
    }
    pub fn mark_flight_recorded(&mut self, ticket: &CaptureTicket) -> Result<(), CaptureError> {
        self.check_ticket(ticket)?;
        if !self.staged { return Err(CaptureError::PhaseConflict); }
        if self.flight_recorded { return Ok(()); }
        let revision = self.next_revision()?;
        self.flight_recorded = true;
        self.transition_revision = revision;
        Ok(())
    }
}

pub struct CaptureProgress { cursor: MessageCursor, final_tail_staged: bool, domain: Option<ClaimDomain> }
impl CaptureProgress {
    pub fn new(actual_initial_cursor: MessageCursor) -> Self {
        Self { cursor: actual_initial_cursor, final_tail_staged: false, domain: None }
    }
    pub fn accept_staged<I: Eq>(&mut self, prepared: &PreparedCapture<I>) -> Result<(), CaptureError> {
        if self.final_tail_staged || !prepared.staged { return Err(CaptureError::PhaseConflict); }
        let observed = prepared.observed.as_ref().ok_or(CaptureError::PhaseConflict)?;
        if prepared.command().expected().cursor() != self.cursor
            || observed.cursor().value() <= self.cursor.value() {
            return Err(CaptureError::CursorConflict);
        }
        let domain = prepared.command().expected().head().domain();
        if self.domain.as_ref().is_some_and(|original| original != domain) {
            return Err(CaptureError::CursorConflict);
        }
        if self.domain.is_none() { self.domain = Some(domain.clone()); }
        self.cursor = observed.cursor();
        self.final_tail_staged = prepared.command().kind() == CaptureKind::FinalTail;
        Ok(())
    }
    pub fn cursor(&self) -> MessageCursor { self.cursor }
    pub fn is_final_tail_staged(&self) -> bool { self.final_tail_staged }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
use crate::claim::{ClaimDomain, ClaimEvent, CoordinationRevision, CoordinationToken,
        LeaseObservation, LeaseRevision, LeaseTime};

    fn id(value: &str) -> CanonicalId { CanonicalId::new(value.as_bytes().to_vec()).unwrap() }
    fn owner(name: &str) -> Incarnation { Incarnation::new(id(name), 1, id("process")) }
    fn snapshot(cursor: u64, coordinator: u64, actor: &str) -> ClaimSnapshot {
        snapshot_for_domain(cursor, coordinator, actor, "opaque-original-domain")
    }
    fn snapshot_for_domain(cursor: u64, coordinator: u64, actor: &str, original: &str) -> ClaimSnapshot {
        let domain = ClaimDomain::new(id(original), id("run"), id("lane"), id("stream"));
        let head = ClaimEvent::new(domain, id(actor), None, LeaseObservation::Acquire {
            incarnation: owner(actor), revision: LeaseRevision::new(1).unwrap(),
            expires_at: LeaseTime::new(100), recovery: None,
        }, MessageCursor::new(4));
        ClaimSnapshot::new(head, CoordinationRevision::new(coordinator),
            CoordinationToken::new(format!("token-{coordinator}").into_bytes()).unwrap(), MessageCursor::new(cursor))
    }
    fn command(body: &str, cursor: u64, kind: CaptureKind) -> CaptureCommand<String> {
        CaptureCommand::new(id("operation"), id("message"), id("idempotency"), owner("a"),
            snapshot(cursor, 7, "a"), id("payload"), [5; 32], kind, body.to_owned()).unwrap()
    }
    fn prepared() -> PreparedCapture<String> {
        PreparedCapture::new(command("whole original command", 10, CaptureKind::Newline), id("ticket"))
    }
    fn canonical<'a>(command: &'a CaptureCommand<String>, observed: &'a ClaimSnapshot) -> CanonicalCaptureRef<'a, String> {
        CanonicalCaptureRef::new(command, b"FULL message record", b"FULL ledger event",
            b"FULL transaction outcome", observed).unwrap()
    }
    fn adopt_and_stage(value: &mut PreparedCapture<String>, observed: &ClaimSnapshot) {
        let ticket = value.ticket().clone();
        let mark = { let record = canonical(value.command(), observed);
            value.validate_adoption(&ticket, record, record).unwrap() };
        assert_eq!(value.record_adoption(mark).unwrap(), AdoptionStatus::Adopted);
        let mark = value.validate_staging(&ticket, canonical(value.command(), observed)).unwrap();
        assert_eq!(value.record_staged(mark).unwrap(), StageStatus::Staged);
    }

    #[test]
    fn preparation_needs_no_future_sequence_and_retains_full_intent() {
        let value = prepared();
        assert_eq!(value.phase(), CapturePhase::Prepared);
        assert_eq!(value.command().expected().cursor().value(), 10);
        assert_eq!(value.command().intent(), "whole original command");
        assert!(value.observed().is_none());
        assert!(value.command() != &command("changed full body", 10, CaptureKind::Newline));
    }
    #[test]
    fn wrong_original_owner_rejects_without_future_record() {
        let result = CaptureCommand::new(id("op"), id("msg"), id("key"), owner("b"),
            snapshot(10, 7, "a"), id("payload"), [0; 32], CaptureKind::Newline, "intent");
        assert!(matches!(result, Err(CaptureError::CommandConflict)));
    }
    #[test]
    fn historical_outcome_is_first_even_when_current_claim_is_closed_or_empty() {
        let value = prepared();
        let observed = snapshot(37, 99, "a");
        let record = canonical(value.command(), &observed);
        assert_eq!(value.assess_new_effect(&ReducedClaim::Empty, &snapshot(99, 100, "b"),
            CanonicalLookup::Committed(record)).unwrap(), Reconciliation::HistoricalData);
        assert_eq!(value.phase(), CapturePhase::Prepared);
    }
    #[test]
    fn full_command_conflict_precedes_historical_reconciliation() {
        let value = prepared();
        let changed = command("changed immutable body", 10, CaptureKind::Newline);
        let observed = snapshot(37, 8, "a");
        assert!(matches!(value.evaluate_existing(CanonicalLookup::Committed(canonical(&changed, &observed))),
            Err(CaptureError::CommandConflict)));
        assert_eq!(value.command().intent(), "whole original command");
    }
    #[test]
    fn missing_canonical_result_does_not_restart_installed_pending_or_unknown_write() {
        let mut value = prepared();
        let ticket = value.ticket().clone();
        value.mark_write_installed(&ticket).unwrap();
        assert_eq!(value.evaluate_existing(CanonicalLookup::NotFound).unwrap(), Reconciliation::Unknown);
        value.mark_started(&ticket).unwrap();
        value.mark_unknown(&ticket).unwrap();
        assert_eq!(value.assess_new_effect(&ReducedClaim::Empty, value.command().expected(),
            CanonicalLookup::NotFound).unwrap(), Reconciliation::Unknown);
        assert_eq!(value.command().intent(), "whole original command");
        assert_eq!(value.phase(), CapturePhase::Unknown);
        assert_eq!(value.mark_write_installed(&ticket), Err(CaptureError::PhaseConflict));
    }
    #[test]
    fn absent_unstarted_branch_requires_actual_current_head() {
        let value = prepared();
        assert!(matches!(value.assess_new_effect(&ReducedClaim::Empty, value.command().expected(),
            CanonicalLookup::NotFound), Err(CaptureError::Claim(_))));
        assert_eq!(value.phase(), CapturePhase::Prepared);
    }
    #[test]
    fn consistent_absent_new_effect_requires_complete_current_snapshot() {
        let value = prepared();
        let expected = value.command().expected();
        let reduced = crate::claim::reduce_lane(expected.head().domain(), std::slice::from_ref(expected.head())).unwrap();
        assert_eq!(value.assess_new_effect(&reduced, expected, CanonicalLookup::NotFound).unwrap(),
            Reconciliation::NewEffectConsistent);
        let changed_fence = snapshot(10, 8, "a");
        assert!(matches!(value.assess_new_effect(&reduced, &changed_fence, CanonicalLookup::NotFound),
            Err(CaptureError::Claim(_))));
        assert_eq!(value.phase(), CapturePhase::Prepared);
    }
    #[test]
    fn adoption_compares_entire_message_event_transaction_and_observed_snapshot() {
        let value = prepared();
        let observed = snapshot(37, 8, "a");
        let actual = canonical(value.command(), &observed);
        for (message, event, transaction) in [
            (&b"different message"[..], &b"FULL ledger event"[..], &b"FULL transaction outcome"[..]),
            (&b"FULL message record"[..], &b"different event"[..], &b"FULL transaction outcome"[..]),
            (&b"FULL message record"[..], &b"FULL ledger event"[..], &b"different transaction"[..]),
        ] {
            let changed = CanonicalCaptureRef::new(value.command(), message, event, transaction, &observed).unwrap();
            assert!(matches!(value.validate_adoption(value.ticket(), actual, changed), Err(CaptureError::CanonicalConflict)));
        }
        let changed_observation = snapshot(37, 9, "a");
        assert!(matches!(value.validate_adoption(value.ticket(), actual, canonical(value.command(), &changed_observation)),
            Err(CaptureError::CanonicalConflict)));
        assert!(CanonicalCaptureRef::new(value.command(), b"", b"event", b"tx", &observed).is_err());
    }
    #[test]
    fn actual_cursor_and_original_head_are_checked_after_commit() {
        let value = prepared();
        for cursor in [0, 9, 10] {
            let observed = snapshot(cursor, 8, "a");
            assert!(matches!(value.validate_adoption(value.ticket(), canonical(value.command(), &observed),
                canonical(value.command(), &observed)), Err(CaptureError::CursorConflict)));
        }
        let foreign = snapshot(37, 8, "b");
        assert!(matches!(value.evaluate_existing(CanonicalLookup::Committed(canonical(value.command(), &foreign))),
            Err(CaptureError::CanonicalConflict)));
    }
    #[test]
    fn late_completion_adopts_original_without_replacing_pending_custody() {
        let mut value = prepared();
        let ticket = value.ticket().clone();
        value.mark_write_installed(&ticket).unwrap();
        value.mark_started(&ticket).unwrap();
        value.mark_unknown(&ticket).unwrap();
        adopt_and_stage(&mut value, &snapshot(37, 9, "a"));
        assert!(value.has_unknown());
        assert!(!value.write_settled());
        assert_eq!(value.phase(), CapturePhase::Staged);
        value.mark_write_settled(&ticket).unwrap();
        assert!(!value.has_unknown());
        assert!(value.write_settled());
        assert_eq!(value.phase(), CapturePhase::Staged);
    }
    #[test]
    fn duplicate_adoption_still_checks_full_cached_result_and_is_idempotent() {
        let mut value = prepared();
        let observed = snapshot(37, 9, "a");
        adopt_and_stage(&mut value, &observed);
        let ticket = value.ticket().clone();
        let mark = { let record = canonical(value.command(), &observed);
            value.validate_adoption(&ticket, record, record).unwrap() };
        assert_eq!(value.record_adoption(mark).unwrap(), AdoptionStatus::AlreadyAdopted);
        assert_eq!(value.phase(), CapturePhase::Staged);
        let changed = CanonicalCaptureRef::new(value.command(), b"changed", b"FULL ledger event",
            b"FULL transaction outcome", &observed).unwrap();
        assert!(matches!(value.validate_adoption(&ticket, canonical(value.command(), &observed), changed),
            Err(CaptureError::CanonicalConflict)));
    }
    #[test]
    fn stale_adoption_and_staging_marks_reject_without_mutating_state() {
        let mut value = prepared();
        let observed = snapshot(37, 9, "a");
        let ticket = value.ticket().clone();
        let mark = { let record = canonical(value.command(), &observed);
            value.validate_adoption(&ticket, record, record).unwrap() };
        value.mark_unknown(&ticket).unwrap();
        assert_eq!(value.record_adoption(mark), Err(CaptureError::StaleMark));
        adopt_and_stage(&mut value, &observed);
        let mark = value.validate_staging(&ticket, canonical(value.command(), &observed)).unwrap();
        value.mark_flight_recorded(&ticket).unwrap();
        assert_eq!(value.record_staged(mark), Err(CaptureError::StaleMark));
        assert_eq!(value.phase(), CapturePhase::FlightRecorded);
    }
    #[test]
    fn repeated_unknown_observation_invalidates_outstanding_adoption_mark() {
        let mut value = prepared();
        let ticket = value.ticket().clone();
        value.mark_unknown(&ticket).unwrap();
        let observed = snapshot(37, 9, "a");
        let mark = { let record = canonical(value.command(), &observed);
            value.validate_adoption(&ticket, record, record).unwrap() };
        value.mark_unknown(&ticket).unwrap();
        assert_eq!(value.record_adoption(mark), Err(CaptureError::StaleMark));
        assert_eq!(value.phase(), CapturePhase::Unknown);
        assert_eq!(value.command().intent(), "whole original command");
    }
    #[test]
    fn wrong_ticket_never_mutates_the_original_operation() {
        let mut value = prepared();
        let other = PreparedCapture::new(command("whole original command", 10, CaptureKind::Newline), id("other-ticket"));
        assert_eq!(value.mark_write_installed(other.ticket()), Err(CaptureError::TicketMismatch));
        assert_eq!(value.phase(), CapturePhase::Prepared);
    }
    #[test]
    fn staged_prefix_survives_unknown_and_uses_real_cursor_not_record_count() {
        let mut value = prepared();
        adopt_and_stage(&mut value, &snapshot(37, 9, "a"));
        let ticket = value.ticket().clone();
        value.mark_unknown(&ticket).unwrap();
        assert_eq!(value.phase(), CapturePhase::Staged);
        let mut progress = CaptureProgress::new(MessageCursor::new(10));
        progress.accept_staged(&value).unwrap();
        assert_eq!(progress.cursor().value(), 37);
        assert!(!progress.is_final_tail_staged());
        assert_eq!(progress.accept_staged(&value), Err(CaptureError::CursorConflict));
        assert_eq!(progress.cursor().value(), 37);
    }
    #[test]
    fn final_tail_requires_staging_and_prevents_later_advance() {
        let mut tail = PreparedCapture::new(command("tail", 37, CaptureKind::FinalTail), id("tail-ticket"));
        let mut progress = CaptureProgress::new(MessageCursor::new(37));
        assert_eq!(progress.accept_staged(&tail), Err(CaptureError::PhaseConflict));
        adopt_and_stage(&mut tail, &snapshot(55, 10, "a"));
        progress.accept_staged(&tail).unwrap();
        assert_eq!(progress.cursor().value(), 55);
        assert!(progress.is_final_tail_staged());
        assert_eq!(progress.accept_staged(&tail), Err(CaptureError::PhaseConflict));
    }
    #[test]
    fn progress_rejects_other_canonical_domain_at_the_same_cursor() {
        let mut first = prepared();
        adopt_and_stage(&mut first, &snapshot(37, 9, "a"));
        let mut progress = CaptureProgress::new(MessageCursor::new(10));
        progress.accept_staged(&first).unwrap();
        let mut incoming = command("foreign stream record", 37, CaptureKind::Newline);
        incoming.expected = snapshot_for_domain(37, 9, "a", "other-original-domain");
        let mut foreign = PreparedCapture::new(incoming, id("foreign-ticket"));
        adopt_and_stage(&mut foreign, &snapshot_for_domain(55, 10, "a", "other-original-domain"));
        assert_eq!(progress.accept_staged(&foreign), Err(CaptureError::CursorConflict));
        assert_eq!(progress.cursor().value(), 37);
        assert!(!progress.is_final_tail_staged());
    }
    #[test]
    fn settled_reconciliation_resolves_current_unknown_and_invalidates_marks() {
        let mut value = prepared();
        let ticket = value.ticket().clone();
        let observed = snapshot(37, 9, "a");
        value.mark_write_installed(&ticket).unwrap();
        value.mark_started(&ticket).unwrap();
        value.mark_write_settled(&ticket).unwrap();
        adopt_and_stage(&mut value, &observed);
        value.mark_unknown(&ticket).unwrap();
        let stale_stage = value.validate_staging(&ticket, canonical(value.command(), &observed)).unwrap();
        let mark = { let record = canonical(value.command(), &observed);
            value.validate_adoption(&ticket, record, record).unwrap() };
        assert_eq!(value.record_adoption(mark).unwrap(), AdoptionStatus::AlreadyAdopted);
        assert!(!value.has_unknown());
        assert!(value.write_settled());
        assert_eq!(value.phase(), CapturePhase::Staged);
        assert_eq!(value.observed().unwrap().cursor().value(), 37);
        assert_eq!(value.command().intent(), "whole original command");
        assert_eq!(value.record_staged(stale_stage), Err(CaptureError::StaleMark));
    }
    #[test]
    fn same_reconciliation_keeps_unsettled_original_unknown() {
        let mut value = prepared();
        let ticket = value.ticket().clone();
        let observed = snapshot(37, 9, "a");
        value.mark_write_installed(&ticket).unwrap();
        value.mark_started(&ticket).unwrap();
        adopt_and_stage(&mut value, &observed);
        value.mark_unknown(&ticket).unwrap();
        let mark = { let record = canonical(value.command(), &observed);
            value.validate_adoption(&ticket, record, record).unwrap() };
        assert_eq!(value.record_adoption(mark).unwrap(), AdoptionStatus::AlreadyAdopted);
        assert!(value.has_unknown());
        assert!(!value.write_settled());
        assert_eq!(value.phase(), CapturePhase::Staged);
        assert_eq!(value.observed().unwrap().cursor().value(), 37);
        assert_eq!(value.evaluate_existing(CanonicalLookup::NotFound).unwrap(), Reconciliation::Unknown);
    }
    #[test]
    fn settlement_without_canonical_record_reports_only_absent_settled() {
        let mut value = prepared();
        let ticket = value.ticket().clone();
        value.mark_write_installed(&ticket).unwrap();
        value.mark_started(&ticket).unwrap();
        value.mark_unknown(&ticket).unwrap();
        value.mark_write_settled(&ticket).unwrap();
        assert!(!value.has_unknown());
        assert!(value.write_settled());
        assert!(value.observed().is_none());
        assert_eq!(value.phase(), CapturePhase::Settled);
        assert_eq!(value.assess_new_effect(&ReducedClaim::Empty, value.command().expected(),
            CanonicalLookup::NotFound).unwrap(), Reconciliation::AbsentSettled);
        assert_eq!(value.mark_write_installed(&ticket), Err(CaptureError::PhaseConflict));
        assert_eq!(value.command().intent(), "whole original command");
    }
    #[test]
    fn repeat_settlement_resolves_new_unknown_and_invalidates_marks() {
        let mut value = prepared();
        let ticket = value.ticket().clone();
        value.mark_write_installed(&ticket).unwrap();
        value.mark_started(&ticket).unwrap();
        value.mark_write_settled(&ticket).unwrap();
        value.mark_unknown(&ticket).unwrap();
        let observed = snapshot(37, 9, "a");
        let stale = { let record = canonical(value.command(), &observed);
            value.validate_adoption(&ticket, record, record).unwrap() };
        value.mark_write_settled(&ticket).unwrap();
        assert!(!value.has_unknown());
        assert_eq!(value.phase(), CapturePhase::Settled);
        assert_eq!(value.record_adoption(stale), Err(CaptureError::StaleMark));
        assert!(value.observed().is_none());
        assert_eq!(value.evaluate_existing(CanonicalLookup::NotFound).unwrap(), Reconciliation::AbsentSettled);
    }
    #[test]
    fn resolution_counter_overflow_preserves_unknown_and_prefix() {
        let mut value = prepared();
        let ticket = value.ticket().clone();
        let observed = snapshot(37, 9, "a");
        value.mark_write_installed(&ticket).unwrap();
        value.mark_started(&ticket).unwrap();
        value.mark_write_settled(&ticket).unwrap();
        adopt_and_stage(&mut value, &observed);
        value.mark_unknown(&ticket).unwrap();
        value.transition_revision = u64::MAX;
        let mark = { let record = canonical(value.command(), &observed);
            value.validate_adoption(&ticket, record, record).unwrap() };
        assert_eq!(value.record_adoption(mark), Err(CaptureError::CounterExhausted));
        assert_eq!(value.mark_write_settled(&ticket), Err(CaptureError::CounterExhausted));
        assert!(value.has_unknown());
        assert!(value.write_settled());
        assert_eq!(value.phase(), CapturePhase::Staged);
        assert_eq!(value.observed().unwrap().cursor().value(), 37);
        assert_eq!(value.transition_revision, u64::MAX);
        let mut unsettled = prepared();
        let ticket = unsettled.ticket().clone();
        unsettled.mark_write_installed(&ticket).unwrap();
        unsettled.mark_started(&ticket).unwrap();
        unsettled.mark_unknown(&ticket).unwrap();
        unsettled.transition_revision = u64::MAX;
        assert_eq!(unsettled.mark_write_settled(&ticket), Err(CaptureError::CounterExhausted));
        assert!(unsettled.has_unknown());
        assert!(!unsettled.write_settled());
        assert_eq!(unsettled.phase(), CapturePhase::Unknown);
    }
    #[test]
    fn adoption_marks_reject_other_prepared_commands_with_colliding_ticket_and_counters() {
        for variant in 0..5 {
            let mut source = prepared();
            let mut target = PreparedCapture::new(changed_command(variant), id("ticket"));
            assert!(source.ticket() == target.ticket());
            assert_eq!(source.phase(), target.phase());
            assert_eq!(source.transition_revision, target.transition_revision);
            assert_eq!(source.cursor(), target.cursor());
            let observed = snapshot(37, 9, "a");
            let (foreign, original) = {
                let record = canonical(source.command(), &observed);
                (source.validate_adoption(source.ticket(), record, record).unwrap(),
                 source.validate_adoption(source.ticket(), record, record).unwrap())
            };
            assert_eq!(target.record_adoption(foreign), Err(CaptureError::StaleMark));
            assert!(target.command() == &changed_command(variant));
            assert_eq!(target.phase(), CapturePhase::Prepared);
            assert_eq!(target.transition_revision, 0);
            assert!(!target.installed && !target.started && !target.settled && !target.unknown);
            assert!(!target.staged && !target.flight_recorded && target.observed.is_none());
            assert_eq!(source.record_adoption(original).unwrap(), AdoptionStatus::Adopted);
        }
    }
    #[test]
    fn staging_marks_reject_other_prepared_commands_with_colliding_ticket_and_counters() {
        for variant in 0..5 {
            let mut source = prepared();
            let mut target = PreparedCapture::new(changed_command(variant), id("ticket"));
            let observed = snapshot(37, 9, "a");
            for value in [&mut source, &mut target] {
                let mark = { let record = canonical(value.command(), &observed);
                    value.validate_adoption(value.ticket(), record, record).unwrap() };
                assert_eq!(value.record_adoption(mark).unwrap(), AdoptionStatus::Adopted);
            }
            assert!(source.ticket() == target.ticket());
            assert_eq!(source.phase(), target.phase());
            assert_eq!(source.transition_revision, target.transition_revision);
            assert_eq!(source.cursor(), target.cursor());
            let foreign = source.validate_staging(source.ticket(), canonical(source.command(), &observed)).unwrap();
            let original = source.validate_staging(source.ticket(), canonical(source.command(), &observed)).unwrap();
            assert_eq!(target.record_staged(foreign), Err(CaptureError::StaleMark));
            assert!(target.command() == &changed_command(variant));
            assert_eq!(target.phase(), CapturePhase::Adopted);
            assert_eq!(target.transition_revision, 1);
            assert!(!target.installed && !target.started && !target.settled && !target.unknown);
            assert!(!target.staged && !target.flight_recorded);
            assert!(target.observed.as_ref() == Some(&observed));
            assert_eq!(source.record_staged(original).unwrap(), StageStatus::Staged);
        }
    }
    fn changed_command(variant: usize) -> CaptureCommand<String> {
        let mut value = command("whole original command", 10, CaptureKind::Newline);
        match variant {
            0 => value.intent.push_str(" changed"),
            1 => value.message_id = id("different-message"),
            2 => value.payload_ref = id("different-payload"),
            3 => value.payload_sha256[0] ^= 1,
            4 => value.expected = snapshot(10, 8, "a"),
            _ => unreachable!(),
        }
        value
    }
    #[test]
    fn instance_affinity_needs_no_clone_intent_and_preserves_send() {
        #[derive(PartialEq, Eq)]
        struct NonCloneIntent(Vec<u8>);
        fn require_send<T: Send>() {}
        require_send::<PreparedCapture<NonCloneIntent>>();
        require_send::<AdoptionMark>();
        require_send::<StageMark>();
        let command = CaptureCommand::new(id("operation"), id("message"), id("key"), owner("a"),
            snapshot(10, 7, "a"), id("payload"), [5; 32], CaptureKind::Newline,
            NonCloneIntent(vec![1, 2, 3])).unwrap();
        let mut value = PreparedCapture::new(command, id("ticket"));
        let observed = snapshot(37, 9, "a");
        let mark = {
            let record = CanonicalCaptureRef::new(value.command(), b"message", b"event", b"tx", &observed).unwrap();
            value.validate_adoption(value.ticket(), record, record).unwrap()
        };
        assert_eq!(value.record_adoption(mark).unwrap(), AdoptionStatus::Adopted);
        assert_eq!(value.command().intent().0, vec![1, 2, 3]);
    }
    #[test]
    fn counter_exhaustion_and_failed_order_leave_state_unchanged() {
        let mut value = prepared();
        let ticket = value.ticket().clone();
        assert_eq!(value.mark_started(&ticket), Err(CaptureError::PhaseConflict));
        assert_eq!(value.mark_flight_recorded(&ticket), Err(CaptureError::PhaseConflict));
        value.transition_revision = u64::MAX;
        assert_eq!(value.mark_write_installed(&ticket), Err(CaptureError::CounterExhausted));
        assert_eq!(value.phase(), CapturePhase::Prepared);
        assert_eq!(value.transition_revision, u64::MAX);
    }
}