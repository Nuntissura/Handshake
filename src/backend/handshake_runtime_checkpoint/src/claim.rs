//! Pure claim observations and deterministic reduction. Inputs and transitions
//! are data, never authorization, durable ownership, or evidence of effects.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClaimError {
    EmptyId, InvalidRevision, ForeignDomain, DivergentEvent, Fork, Gap,
    Cycle, Disconnected, InvalidGenesis, InvalidTransition, RevisionOverflow,
    RevisionMismatch, IncarnationReuse, RecoveryLinkMismatch, EmptyClaim,
    ClosedClaim, HeadMismatch, SnapshotMismatch, CursorRegression, NotExpired,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanonicalId(Vec<u8>);
impl CanonicalId {
    pub fn new(bytes: Vec<u8>) -> Result<Self, ClaimError> {
        if bytes.is_empty() { Err(ClaimError::EmptyId) } else { Ok(Self(bytes)) }
    }
    pub fn as_bytes(&self) -> &[u8] { &self.0 }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimDomain {
    original_domain: CanonicalId,
    run: CanonicalId,
    lane: CanonicalId,
    stream: CanonicalId,
}
impl ClaimDomain {
    pub fn new(original_domain: CanonicalId, run: CanonicalId, lane: CanonicalId, stream: CanonicalId) -> Self {
        Self { original_domain, run, lane, stream }
    }
    pub fn original_domain(&self) -> &CanonicalId { &self.original_domain }
    pub fn run(&self) -> &CanonicalId { &self.run }
    pub fn lane(&self) -> &CanonicalId { &self.lane }
    pub fn stream(&self) -> &CanonicalId { &self.stream }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Incarnation {
    instance_id: CanonicalId,
    spawn_generation: u64,
    process_record_id: CanonicalId,
}
impl Incarnation {
    pub fn new(instance_id: CanonicalId, spawn_generation: u64, process_record_id: CanonicalId) -> Self {
        Self { instance_id, spawn_generation, process_record_id }
    }
    pub fn instance_id(&self) -> &CanonicalId { &self.instance_id }
    pub fn spawn_generation(&self) -> u64 { self.spawn_generation }
    pub fn process_record_id(&self) -> &CanonicalId { &self.process_record_id }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LeaseRevision(u64);
impl LeaseRevision {
    pub fn new(value: u64) -> Result<Self, ClaimError> {
        if value == 0 { Err(ClaimError::InvalidRevision) } else { Ok(Self(value)) }
    }
    pub fn initial() -> Self { Self(1) }
    pub fn value(self) -> u64 { self.0 }
    pub fn checked_next(self) -> Result<Self, ClaimError> {
        self.0.checked_add(1).map(Self).ok_or(ClaimError::RevisionOverflow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CoordinationRevision(u64);
impl CoordinationRevision {
    pub fn new(value: u64) -> Self { Self(value) }
    pub fn value(self) -> u64 { self.0 }
    pub fn checked_next(self) -> Result<Self, ClaimError> {
        self.0.checked_add(1).map(Self).ok_or(ClaimError::RevisionOverflow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageCursor(u64);
impl MessageCursor {
    pub fn new(value: u64) -> Self { Self(value) }
    pub fn value(self) -> u64 { self.0 }
    pub fn checked_next(self) -> Result<Self, ClaimError> {
        self.0.checked_add(1).map(Self).ok_or(ClaimError::RevisionOverflow)
    }
}

// Deliberately no Debug, serialization, or byte getter.
#[derive(Clone, PartialEq, Eq)]
pub struct CoordinationToken(Vec<u8>);
impl CoordinationToken {
    pub fn new(bytes: Vec<u8>) -> Result<Self, ClaimError> {
        if bytes.is_empty() { Err(ClaimError::EmptyId) } else { Ok(Self(bytes)) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LeaseTime(u64);
impl LeaseTime {
    pub fn new(value: u64) -> Self { Self(value) }
    pub fn value(self) -> u64 { self.0 }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryExecuteEvidence {
    event_id: CanonicalId,
    predecessor_event_id: CanonicalId,
    prior_incarnation: Incarnation,
}
impl RecoveryExecuteEvidence {
    /// Carries externally checked evidence DATA; does not check permission.
    pub fn new(event_id: CanonicalId, predecessor_event_id: CanonicalId, prior_incarnation: Incarnation) -> Self {
        Self { event_id, predecessor_event_id, prior_incarnation }
    }
    pub fn event_id(&self) -> &CanonicalId { &self.event_id }
    pub fn predecessor_event_id(&self) -> &CanonicalId { &self.predecessor_event_id }
    pub fn prior_incarnation(&self) -> &Incarnation { &self.prior_incarnation }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeaseObservation {
    Acquire { incarnation: Incarnation, revision: LeaseRevision, expires_at: LeaseTime, recovery: Option<RecoveryExecuteEvidence> },
    Renew { incarnation: Incarnation, revision: LeaseRevision, expires_at: LeaseTime },
    Terminal { incarnation: Incarnation, revision: LeaseRevision, expires_at: LeaseTime, reason: CanonicalId },
}
impl LeaseObservation {
    pub fn incarnation(&self) -> &Incarnation {
        match self { Self::Acquire { incarnation, .. } | Self::Renew { incarnation, .. } | Self::Terminal { incarnation, .. } => incarnation }
    }
    pub fn revision(&self) -> LeaseRevision {
        match self { Self::Acquire { revision, .. } | Self::Renew { revision, .. } | Self::Terminal { revision, .. } => *revision }
    }
    pub fn expires_at(&self) -> LeaseTime {
        match self { Self::Acquire { expires_at, .. } | Self::Renew { expires_at, .. } | Self::Terminal { expires_at, .. } => *expires_at }
    }
    pub fn is_terminal(&self) -> bool { matches!(self, Self::Terminal { .. }) }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimEvent {
    domain: ClaimDomain,
    event_id: CanonicalId,
    predecessor: Option<CanonicalId>,
    observation: LeaseObservation,
    cursor: MessageCursor,
}
impl ClaimEvent {
    pub fn new(domain: ClaimDomain, event_id: CanonicalId, predecessor: Option<CanonicalId>, observation: LeaseObservation, cursor: MessageCursor) -> Self {
        Self { domain, event_id, predecessor, observation, cursor }
    }
    pub fn domain(&self) -> &ClaimDomain { &self.domain }
    pub fn event_id(&self) -> &CanonicalId { &self.event_id }
    pub fn predecessor(&self) -> Option<&CanonicalId> { self.predecessor.as_ref() }
    pub fn observation(&self) -> &LeaseObservation { &self.observation }
    pub fn incarnation(&self) -> &Incarnation { self.observation.incarnation() }
    pub fn revision(&self) -> LeaseRevision { self.observation.revision() }
    pub fn expires_at(&self) -> LeaseTime { self.observation.expires_at() }
    pub fn cursor(&self) -> MessageCursor { self.cursor }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimHead { event: ClaimEvent }
impl ClaimHead { pub fn event(&self) -> &ClaimEvent { &self.event } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReducedClaim { Empty, Open(ClaimHead), Closed(ClaimHead) }

/// Reduces the entire supplied domain. It never filters foreign rows or selects
/// a head by a timestamp, UUID, generation, or maximum revision.
pub fn reduce_lane(domain: &ClaimDomain, events: &[ClaimEvent]) -> Result<ReducedClaim, ClaimError> {
    if events.is_empty() { return Ok(ReducedClaim::Empty); }
    let mut unique: BTreeMap<CanonicalId, &ClaimEvent> = BTreeMap::new();
    for event in events {
        if event.domain() != domain { return Err(ClaimError::ForeignDomain); }
        if let Some(previous) = unique.get(event.event_id()) {
            if *previous != event { return Err(ClaimError::DivergentEvent); }
        } else { unique.insert(event.event_id().clone(), event); }
    }
    let mut genesis = None;
    let mut successors: BTreeMap<CanonicalId, CanonicalId> = BTreeMap::new();
    for event in unique.values() {
        match event.predecessor() {
            None => {
                if genesis.replace(event.event_id().clone()).is_some() { return Err(ClaimError::InvalidGenesis); }
            }
            Some(predecessor) => {
                if !unique.contains_key(predecessor) { return Err(ClaimError::Gap); }
                if successors.insert(predecessor.clone(), event.event_id().clone()).is_some() { return Err(ClaimError::Fork); }
            }
        }
    }
    let mut next = genesis.ok_or(ClaimError::Cycle)?;
    let mut visited = BTreeSet::new();
    let mut ordered = Vec::with_capacity(unique.len());
    loop {
        if !visited.insert(next.clone()) { return Err(ClaimError::Cycle); }
        let event = unique.get(&next).ok_or(ClaimError::Gap)?;
        ordered.push(*event);
        match successors.get(&next) { Some(successor) => next = successor.clone(), None => break }
    }
    if visited.len() != unique.len() { return Err(ClaimError::Disconnected); }
    let first = ordered[0];
    match first.observation() {
        LeaseObservation::Acquire { revision, recovery: None, .. } if *revision == LeaseRevision::initial() => {},
        _ => return Err(ClaimError::InvalidGenesis),
    }
    let mut seen = BTreeSet::new();
    seen.insert(first.incarnation().clone());
    let mut previous = first;
    for event in ordered.into_iter().skip(1) {
        if event.cursor() < previous.cursor() { return Err(ClaimError::CursorRegression); }
        match event.observation() {
            LeaseObservation::Acquire { incarnation, revision, recovery, .. } => {
                if *revision != LeaseRevision::initial() { return Err(ClaimError::RevisionMismatch); }
                if !seen.insert(incarnation.clone()) { return Err(ClaimError::IncarnationReuse); }
                match (previous.observation().is_terminal(), recovery) {
                    (true, None) => {},
                    (_, Some(evidence)) if evidence.predecessor_event_id() == previous.event_id()
                        && evidence.prior_incarnation() == previous.incarnation() => {},
                    _ => return Err(ClaimError::RecoveryLinkMismatch),
                }
            }
            LeaseObservation::Renew { .. } | LeaseObservation::Terminal { .. } => {
                if previous.observation().is_terminal() || event.incarnation() != previous.incarnation() {
                    return Err(ClaimError::InvalidTransition);
                }
                if event.revision() != previous.revision().checked_next()? { return Err(ClaimError::RevisionMismatch); }
            }
        }
        previous = event;
    }
    let head = ClaimHead { event: previous.clone() };
    if previous.observation().is_terminal() { Ok(ReducedClaim::Closed(head)) } else { Ok(ReducedClaim::Open(head)) }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ClaimSnapshot {
    head: ClaimEvent,
    coordination_revision: CoordinationRevision,
    coordination_token: CoordinationToken,
    cursor: MessageCursor,
}
impl ClaimSnapshot {
    pub fn new(head: ClaimEvent, coordination_revision: CoordinationRevision, coordination_token: CoordinationToken, cursor: MessageCursor) -> Self {
        Self { head, coordination_revision, coordination_token, cursor }
    }
    pub fn head(&self) -> &ClaimEvent { &self.head }
    pub fn coordination_revision(&self) -> CoordinationRevision { self.coordination_revision }
    pub fn coordination_token(&self) -> &CoordinationToken { &self.coordination_token }
    pub fn cursor(&self) -> MessageCursor { self.cursor }
}

/// New-effect DATA check only. Historical exact outcomes are reconciled first
/// by the host/capture caller; success here is not a durable CAS or permission.
pub fn validate_current(reduced: &ReducedClaim, actual: &ClaimSnapshot, claimed: &ClaimSnapshot) -> Result<(), ClaimError> {
    let head = match reduced {
        ReducedClaim::Empty => return Err(ClaimError::EmptyClaim),
        ReducedClaim::Closed(_) => return Err(ClaimError::ClosedClaim),
        ReducedClaim::Open(head) => head,
    };
    if head.event() != actual.head() { return Err(ClaimError::HeadMismatch); }
    if actual.cursor() < head.event().cursor() { return Err(ClaimError::CursorRegression); }
    if actual != claimed { return Err(ClaimError::SnapshotMismatch); }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaperSnapshot { head: ClaimEvent }
impl ReaperSnapshot {
    pub fn new(head: ClaimEvent) -> Self { Self { head } }
    pub fn head(&self) -> &ClaimEvent { &self.head }
}
pub fn validate_reaper(snapshot: &ReaperSnapshot, current: &ReducedClaim, now: LeaseTime) -> Result<(), ClaimError> {
    let head = match current {
        ReducedClaim::Empty => return Err(ClaimError::EmptyClaim),
        ReducedClaim::Closed(_) => return Err(ClaimError::ClosedClaim),
        ReducedClaim::Open(head) => head,
    };
    if snapshot.head() != head.event() { return Err(ClaimError::HeadMismatch); }
    if now < snapshot.head().expires_at() { return Err(ClaimError::NotExpired); }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReservationPhase { Pending, Unknown, Settled }

#[derive(Clone, PartialEq, Eq)]
pub struct ReservationRecord<C: Eq> {
    operation_id: CanonicalId,
    ticket: CanonicalId,
    command: C,
    phase: ReservationPhase,
}
impl<C: Eq> ReservationRecord<C> {
    pub fn new(operation_id: CanonicalId, ticket: CanonicalId, command: C) -> Self {
        Self { operation_id, ticket, command, phase: ReservationPhase::Pending }
    }
    pub fn operation_id(&self) -> &CanonicalId { &self.operation_id }
    pub fn ticket(&self) -> &CanonicalId { &self.ticket }
    pub fn command(&self) -> &C { &self.command }
    pub fn phase(&self) -> ReservationPhase { self.phase }
    /// Records supplied DATA only, not actual write completion or cleanup.
    pub fn observe_unknown(&mut self) {
        if self.phase != ReservationPhase::Settled { self.phase = ReservationPhase::Unknown; }
    }
    /// Records supplied DATA only, not actual write completion or cleanup.
    pub fn observe_settled(&mut self) { self.phase = ReservationPhase::Settled; }
}
pub enum ReservationDecision<'a, C: Eq> {
    Available,
    SameOriginal(&'a ReservationRecord<C>),
    Busy { original_operation: &'a CanonicalId, original_ticket: &'a CanonicalId },
    Conflict,
}
pub fn assess_reservation<'a, C: Eq>(existing: Option<&'a ReservationRecord<C>>, incoming: &ReservationRecord<C>) -> ReservationDecision<'a, C> {
    let Some(original) = existing else { return ReservationDecision::Available; };
    if original.operation_id() == incoming.operation_id() {
        if original.command() == incoming.command() { ReservationDecision::SameOriginal(original) }
        else { ReservationDecision::Conflict }
    } else if original.phase() == ReservationPhase::Settled { ReservationDecision::Available }
    else { ReservationDecision::Busy { original_operation: original.operation_id(), original_ticket: original.ticket() } }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(value: &str) -> CanonicalId { CanonicalId::new(value.as_bytes().to_vec()).unwrap() }
    fn domain() -> ClaimDomain { ClaimDomain::new(id("opaque-original-domain"), id("run"), id("lane"), id("stream")) }
    fn incarnation(value: &str) -> Incarnation { Incarnation::new(id(value), 7, id(&format!("process-{value}"))) }
    fn event(name: &str, predecessor: Option<&str>, who: &str, revision: u64, kind: u8, expiry: u64, cursor: u64) -> ClaimEvent {
        let incarnation = incarnation(who);
        let revision = LeaseRevision::new(revision).unwrap();
        let expires_at = LeaseTime::new(expiry);
        let observation = match kind {
            0 => LeaseObservation::Acquire { incarnation, revision, expires_at, recovery: None },
            1 => LeaseObservation::Renew { incarnation, revision, expires_at },
            _ => LeaseObservation::Terminal { incarnation, revision, expires_at, reason: id("done") },
        };
        ClaimEvent::new(domain(), id(name), predecessor.map(id), observation, MessageCursor::new(cursor))
    }
    fn snapshot(head: ClaimEvent, coord: u64, token: &str, cursor: u64) -> ClaimSnapshot {
        ClaimSnapshot::new(head, CoordinationRevision::new(coord), CoordinationToken::new(token.as_bytes().to_vec()).unwrap(), MessageCursor::new(cursor))
    }
    #[test]
    fn empty_and_byte_preservation_and_checked_counters() {
        assert_eq!(reduce_lane(&domain(), &[]), Ok(ReducedClaim::Empty));
        assert_eq!(CanonicalId::new(vec![]), Err(ClaimError::EmptyId));
        assert_eq!(CanonicalId::new(vec![0, 255, 32]).unwrap().as_bytes(), &[0, 255, 32]);
        assert_eq!(LeaseRevision::new(0), Err(ClaimError::InvalidRevision));
        assert_eq!(LeaseRevision::new(451).unwrap().value(), 451);
        assert_eq!(LeaseRevision::new(u64::MAX).unwrap().checked_next(), Err(ClaimError::RevisionOverflow));
        assert_eq!(CoordinationRevision::new(u64::MAX).checked_next(), Err(ClaimError::RevisionOverflow));
        assert_eq!(MessageCursor::new(u64::MAX).checked_next(), Err(ClaimError::RevisionOverflow));
    }
    #[test]
    fn whole_lane_chain_is_order_independent_and_exact_duplicates_are_data() {
        let a = event("a", None, "A", 1, 0, 10, 0);
        let b = event("b", Some("a"), "A", 2, 1, 20, 3);
        let c = event("c", Some("b"), "A", 3, 2, 20, 3);
        let d = event("d", Some("c"), "B", 1, 0, 40, 4);
        let expected = reduce_lane(&domain(), &[a.clone(), b.clone(), c.clone(), d.clone()]).unwrap();
        for input in [vec![d.clone(), b.clone(), a.clone(), c.clone()], vec![c.clone(), a.clone(), d.clone(), b.clone(), a.clone()]] {
            assert_eq!(reduce_lane(&domain(), &input), Ok(expected.clone()));
        }
        match expected { ReducedClaim::Open(head) => assert_eq!(head.event(), &d), _ => panic!("expected actual chain head") }
    }
    #[test]
    fn structural_corruption_is_not_filtered_or_ranked() {
        let a = event("a", None, "A", 1, 0, 10, 0);
        let b = event("b", Some("a"), "A", 2, 1, 20, 0);
        let fork = event("fork", Some("a"), "A", 2, 1, 30, 0);
        assert_eq!(reduce_lane(&domain(), &[a.clone(), b.clone(), fork]), Err(ClaimError::Fork));
        assert_eq!(reduce_lane(&domain(), &[b.clone()]), Err(ClaimError::Gap));
        let mut foreign = b.clone();
        foreign.domain = ClaimDomain::new(id("foreign"), id("run"), id("lane"), id("stream"));
        assert_eq!(reduce_lane(&domain(), &[a.clone(), foreign]), Err(ClaimError::ForeignDomain));
        let mut divergent = a.clone(); divergent.cursor = MessageCursor::new(9);
        assert_eq!(reduce_lane(&domain(), &[a.clone(), divergent]), Err(ClaimError::DivergentEvent));
        let x = event("x", Some("y"), "A", 1, 0, 10, 0);
        let y = event("y", Some("x"), "A", 2, 1, 10, 0);
        assert_eq!(reduce_lane(&domain(), &[x.clone(), y.clone()]), Err(ClaimError::Cycle));
        assert_eq!(reduce_lane(&domain(), &[a, x, y]), Err(ClaimError::Disconnected));
    }
    #[test]
    fn revision_closed_and_late_incarnation_paths_reject() {
        let a = event("a", None, "A", 1, 0, 10, 2);
        assert_eq!(reduce_lane(&domain(), &[a.clone(), event("b", Some("a"), "A", 1, 1, 90, 2)]), Err(ClaimError::RevisionMismatch));
        assert_eq!(reduce_lane(&domain(), &[a.clone(), event("b", Some("a"), "A", 2, 1, 90, 1)]), Err(ClaimError::CursorRegression));
        let end = event("end", Some("a"), "A", 2, 2, 10, 2);
        let closed = reduce_lane(&domain(), &[a.clone(), end.clone()]).unwrap();
        let old = snapshot(a.clone(), 9, "fresh-guard", 2);
        assert_eq!(validate_current(&closed, &old, &old), Err(ClaimError::ClosedClaim));
        assert_eq!(reduce_lane(&domain(), &[a.clone(), end.clone(), event("late", Some("end"), "A", 3, 1, 90, 2)]), Err(ClaimError::InvalidTransition));
        let next = event("next", Some("end"), "B", 1, 0, 90, 2);
        let active_b = reduce_lane(&domain(), &[a.clone(), end.clone(), next.clone()]).unwrap();
        let actual_b = snapshot(next.clone(), 42, "same-fresh-guard", 8);
        let claimed_old_a = snapshot(a.clone(), 42, "same-fresh-guard", 8);
        assert_eq!(validate_current(&active_b, &actual_b, &claimed_old_a), Err(ClaimError::SnapshotMismatch));
        assert_eq!(validate_current(&active_b, &claimed_old_a, &claimed_old_a), Err(ClaimError::HeadMismatch));
        let bend = event("bend", Some("next"), "B", 2, 2, 90, 2);
        assert_eq!(reduce_lane(&domain(), &[a, end, next, bend, event("aba", Some("bend"), "A", 1, 0, 100, 2)]), Err(ClaimError::IncarnationReuse));
    }
    #[test]
    fn takeover_evidence_is_exact_linked_data_only() {
        let a = event("a", None, "A", 1, 0, 10, 0);
        let mut takeover = event("b", Some("a"), "B", 1, 0, 40, 0);
        assert_eq!(reduce_lane(&domain(), &[a.clone(), takeover.clone()]), Err(ClaimError::RecoveryLinkMismatch));
        if let LeaseObservation::Acquire { recovery, .. } = &mut takeover.observation {
            *recovery = Some(RecoveryExecuteEvidence::new(id("external-recovery-event"), id("a"), incarnation("A")));
        }
        assert!(matches!(reduce_lane(&domain(), &[takeover.clone(), a.clone()]), Ok(ReducedClaim::Open(_))));
        if let LeaseObservation::Acquire { recovery: Some(recovery), .. } = &mut takeover.observation { recovery.predecessor_event_id = id("wrong"); }
        assert_eq!(reduce_lane(&domain(), &[a, takeover]), Err(ClaimError::RecoveryLinkMismatch));
    }
    #[test]
    fn current_validation_compares_full_head_and_every_snapshot_field() {
        let a = event("a", None, "A", 1, 0, 10, 2);
        let reduced = reduce_lane(&domain(), &[a.clone()]).unwrap();
        let actual = snapshot(a.clone(), 8, "token-a", 3);
        assert_eq!(validate_current(&reduced, &actual, &actual), Ok(()));
        for claimed in [snapshot(a.clone(), 9, "token-a", 3), snapshot(a.clone(), 8, "token-b", 3), snapshot(a.clone(), 8, "token-a", 4)] {
            assert_eq!(validate_current(&reduced, &actual, &claimed), Err(ClaimError::SnapshotMismatch));
        }
        let wrong_head = event("wrong", None, "A", 1, 0, 10, 2);
        assert_eq!(validate_current(&reduced, &snapshot(wrong_head, 8, "token-a", 3), &actual), Err(ClaimError::HeadMismatch));
        assert_eq!(validate_current(&reduced, &snapshot(a, 8, "token-a", 1), &actual), Err(ClaimError::CursorRegression));
    }
    #[test]
    fn renewed_revision_invalidates_old_expiry_even_when_both_are_expired() {
        let a = event("a", None, "A", 1, 0, 10, 0);
        let captured = ReaperSnapshot::new(a.clone());
        let old = reduce_lane(&domain(), &[a.clone()]).unwrap();
        assert_eq!(validate_reaper(&captured, &old, LeaseTime::new(9)), Err(ClaimError::NotExpired));
        assert_eq!(validate_reaper(&captured, &old, LeaseTime::new(10)), Ok(()));
        let renewed = reduce_lane(&domain(), &[a, event("renew", Some("a"), "A", 2, 1, 20, 0)]).unwrap();
        assert_eq!(validate_reaper(&captured, &renewed, LeaseTime::new(100)), Err(ClaimError::HeadMismatch));
    }
    #[test]
    fn reservations_preserve_original_ticket_intent_and_unknown() {
        let mut original = ReservationRecord::new(id("op"), id("original-ticket"), vec![1, 2]);
        let retry = ReservationRecord::new(id("op"), id("new-ticket"), vec![1, 2]);
        assert!(matches!(assess_reservation(Some(&original), &retry), ReservationDecision::SameOriginal(record) if record.ticket() == &id("original-ticket")));
        let mismatch = ReservationRecord::new(id("op"), id("new-ticket"), vec![1, 9]);
        assert!(matches!(assess_reservation(Some(&original), &mismatch), ReservationDecision::Conflict));
        original.observe_unknown();
        let other = ReservationRecord::new(id("other"), id("other-ticket"), vec![3]);
        assert!(matches!(assess_reservation(Some(&original), &other), ReservationDecision::Busy { original_operation, original_ticket } if original_operation == &id("op") && original_ticket == &id("original-ticket")));
        original.observe_settled(); original.observe_unknown();
        assert_eq!(original.phase(), ReservationPhase::Settled);
        assert!(matches!(assess_reservation(Some(&original), &retry), ReservationDecision::SameOriginal(_)));
        assert!(matches!(assess_reservation(Some(&original), &other), ReservationDecision::Available));
        assert_eq!(original.command(), &vec![1, 2]);
    }
}