//! In-flight bookkeeping: the owner counts every submission until it observes the end of it itself.
//!
//! A claim with a submission in flight cannot settle and cannot be rebound, whatever its client reports, so a
//! cancelled, timed-out or dropped client never turns unfinished work into "settled" (GUI-DESKTOP-033,
//! GPU-QUOTA-013). A dropped [`Submission`] stays counted. There are exactly four ways a count clears, and all
//! of them are the owner's: its fence value was reached, its device was observed lost and quiescent, its device
//! was destroyed, or it never reached the queue.

use super::client::has_flight_room;
use super::reclaim::reclaim;
use super::{bump, Claim, ClaimHold, ClaimLedger, LedgerClient, State};
use crate::diag::OwnerEvent;
use crate::ids::ClaimId;
use crate::outcome::{AuthorizeRefusal, SubmissionRefusal};
use crate::records::{ClaimInfo, SubmissionObservation, SubmissionState};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

const PENDING: u8 = 0;
const COMPLETED: u8 = 1;
const DEVICE_LOST: u8 = 2;
const NO_EFFECT: u8 = 3;

const fn code(observation: SubmissionObservation) -> u8 {
    match observation {
        SubmissionObservation::Completed => COMPLETED,
        SubmissionObservation::DeviceLost => DEVICE_LOST,
        SubmissionObservation::NoEffect => NO_EFFECT,
    }
}

/// A submission the ledger counts as not finished.
pub(super) struct Flight {
    claim: u64,
    /// The attachment of its claim.
    seat: u64,
    outcome: Arc<AtomicU8>,
}

/// One authorized submission: the attachment (module and client), operation and device generation of its claim,
/// and its fence value. It settles nothing by itself and may be dropped: the owner keeps counting the work.
#[derive(Debug)]
pub struct Submission {
    claim: ClaimInfo,
    ledger_key: u64,
    seat: u64,
    sequence: u64,
    outcome: Arc<AtomicU8>,
}

impl Submission {
    pub fn claim(&self) -> ClaimId {
        self.claim.id
    }

    /// The claim the work runs under, with its module, client and operation.
    pub fn claim_info(&self) -> ClaimInfo {
        self.claim
    }

    pub fn generation(&self) -> u64 {
        self.claim.generation
    }

    /// Fence value: strictly increasing within its generation, from 1.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// What the owner observed so far. Never blocks and never asks the device.
    pub fn state(&self) -> SubmissionState {
        match self.outcome.load(Ordering::Acquire) {
            COMPLETED => SubmissionState::Finished(SubmissionObservation::Completed),
            DEVICE_LOST => SubmissionState::Finished(SubmissionObservation::DeviceLost),
            NO_EFFECT => SubmissionState::Finished(SubmissionObservation::NoEffect),
            _ => SubmissionState::Pending,
        }
    }
}

/// End the flight at `key`, and with it an abandoned claim that has nothing else left. `false` when nothing is in
/// flight there.
fn resolve(
    state: &mut State,
    key: (u64, u64),
    observation: SubmissionObservation,
    events: &mut Vec<OwnerEvent>,
) -> bool {
    let Some(flight) = state.flights.remove(&key) else {
        return false;
    };
    flight.outcome.store(code(observation), Ordering::Release);
    if let Some(record) = state.live.get_mut(&flight.claim) {
        record.in_flight = record.in_flight.saturating_sub(1);
        match observation {
            SubmissionObservation::Completed => bump(&mut record.work.completed),
            SubmissionObservation::DeviceLost => bump(&mut record.work.lost),
            SubmissionObservation::NoEffect => bump(&mut record.work.no_effect),
        }
    }
    match observation {
        SubmissionObservation::Completed => bump(&mut state.counters.completed),
        SubmissionObservation::DeviceLost => bump(&mut state.counters.completed_lost),
        SubmissionObservation::NoEffect => bump(&mut state.counters.completed_no_effect),
    }
    reclaim(state, flight.claim, events);
    true
}

/// End every flight of `generation` up to and including fence value `last`. Returns how many.
fn resolve_range(
    state: &mut State,
    generation: u64,
    last: u64,
    observation: SubmissionObservation,
    events: &mut Vec<OwnerEvent>,
) -> u64 {
    let keys: Vec<(u64, u64)> = state
        .flights
        .range((generation, 0)..=(generation, last))
        .map(|(key, _)| *key)
        .collect();
    let mut resolved: u64 = 0;
    for key in keys {
        if resolve(state, key, observation, events) {
            bump(&mut resolved);
        }
    }
    resolved
}

impl LedgerClient {
    /// Authorize one submission under `claim` and count it in flight.
    pub fn begin_submission(&self, claim: &Claim) -> Result<Submission, AuthorizeRefusal> {
        let result = {
            let mut guard = self.shared.lock();
            let state = &mut *guard;
            let result = self.begin_locked(state, claim);
            if result.is_err() {
                bump(&mut state.counters.authorize_refused);
            }
            result
        };
        self.report_authorize(claim, result)
    }

    fn begin_locked(
        &self,
        state: &mut State,
        claim: &Claim,
    ) -> Result<Submission, AuthorizeRefusal> {
        let record = self.authorized(state, claim)?;
        has_flight_room(record)?;
        // Cannot overflow: it is below a `u32` bound.
        record.in_flight = record.in_flight.saturating_add(1);
        let sequence = state.next_sequence;
        state.next_sequence = sequence.saturating_add(1);
        let outcome = Arc::new(AtomicU8::new(PENDING));
        state.flights.insert(
            (claim.info.generation, sequence),
            Flight {
                claim: claim.info.id.0,
                seat: self.seat,
                outcome: Arc::clone(&outcome),
            },
        );
        bump(&mut state.counters.submitted);
        Ok(Submission {
            claim: claim.info,
            ledger_key: self.shared.key,
            seat: self.seat,
            sequence,
            outcome,
        })
    }

    /// Whether this client's work is what `submission` stands for.
    pub fn owns(&self, submission: &Submission) -> bool {
        submission.ledger_key == self.shared.key && submission.seat == self.seat
    }
}

impl ClaimHold {
    /// Submissions of this client on the device of the hold that are not observed finished, under whichever of its
    /// claims: any of them may use an object made from the lease.
    pub fn in_flight(&self) -> u64 {
        self.flights(|flights| u64::try_from(flights.count()).unwrap_or(u64::MAX))
    }

    /// The highest value among them.
    pub(crate) fn last_in_flight(&self) -> Option<u64> {
        self.flights(|flights| flights.last())
    }

    fn flights<T>(&self, of: impl FnOnce(&mut dyn Iterator<Item = u64>) -> T) -> T {
        let state = self.ledger.lock();
        let generation = self.info.generation;
        let mut values = state
            .flights
            .range((generation, 0)..=(generation, u64::MAX))
            .filter(|(_, flight)| flight.seat == self.seat)
            .map(|((_, value), _)| *value);
        of(&mut values)
    }
}

impl ClaimLedger {
    /// `submission` never reached the queue: nothing was invoked on the device.
    pub fn submission_no_effect(&self, submission: &Submission) -> Result<(), SubmissionRefusal> {
        let mut events = Vec::new();
        let result = {
            let mut guard = self.lock();
            let state = &mut *guard;
            let key = (submission.claim.generation, submission.sequence);
            let result = if submission.ledger_key != self.shared.key {
                Err(SubmissionRefusal::Unknown)
            } else if resolve(state, key, SubmissionObservation::NoEffect, &mut events) {
                Ok(())
            } else {
                Err(SubmissionRefusal::AlreadyResolved)
            };
            if result.is_err() {
                bump(&mut state.counters.submission_refused);
            }
            result
        };
        self.shared.emit_all(events);
        result
    }

    /// The owner observed the queue of `generation` at `value`: every submission up to it completed. Returns how
    /// many submissions that resolved. Resolves nothing for a generation that is known to be lost: what it says of
    /// its fences then is not that the work completed.
    pub fn resolve_through(&self, generation: u64, value: u64) -> u64 {
        let mut events = Vec::new();
        let resolved = {
            let mut guard = self.lock();
            let state = &mut *guard;
            if generation <= state.last_lost {
                return 0;
            }
            resolve_range(
                state,
                generation,
                value,
                SubmissionObservation::Completed,
                &mut events,
            )
        };
        self.shared.emit_all(events);
        resolved
    }

    /// The owner observed the loss of `generation` and that nothing submitted on it is still in use by the
    /// device. Resolves its submissions as lost and returns how many. Resolves nothing for a healthy generation.
    pub fn resolve_lost(&self, generation: u64) -> u64 {
        let mut events = Vec::new();
        let resolved = {
            let mut guard = self.lock();
            let state = &mut *guard;
            if state.is_running(generation) {
                return 0;
            }
            resolve_range(
                state,
                generation,
                u64::MAX,
                SubmissionObservation::DeviceLost,
                &mut events,
            )
        };
        self.shared.emit_all(events);
        resolved
    }

    /// The owner destroyed the device of `generation`: nothing submitted on it can run and the memory on it is
    /// gone. Resolves its submissions, clears the memory its open claims reported, closes its abandoned claims, and
    /// returns how many submissions that resolved. Every other reservation stays charged. Does nothing for a
    /// healthy generation.
    pub fn generation_retired(&self, generation: u64) -> u64 {
        let mut events = Vec::new();
        let resolved = {
            let mut guard = self.lock();
            let state = &mut *guard;
            if state.is_running(generation) {
                return 0;
            }
            for record in state.live.values_mut() {
                if record.info.generation == generation {
                    record.device_gone = true;
                    record.committed_bytes = 0;
                    record.staging_bytes = 0;
                }
            }
            let resolved = resolve_range(
                state,
                generation,
                u64::MAX,
                SubmissionObservation::DeviceLost,
                &mut events,
            );
            state.counters.retired_in_flight =
                state.counters.retired_in_flight.saturating_add(resolved);
            let abandoned: Vec<u64> = state
                .live
                .iter()
                .filter(|(_, record)| record.abandoned && record.info.generation == generation)
                .map(|(id, _)| *id)
                .collect();
            for id in abandoned {
                reclaim(state, id, &mut events);
            }
            resolved
        };
        self.shared.emit_all(events);
        resolved
    }
}
