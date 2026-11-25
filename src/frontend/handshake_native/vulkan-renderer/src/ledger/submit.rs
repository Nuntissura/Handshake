//! In-flight bookkeeping: the owner counts every submission until it observes the end of it itself.
//!
//! A claim with a submission in flight cannot settle and cannot be rebound, whatever its client reports, so a
//! cancelled, timed-out or dropped client never turns unfinished work into "settled" (GUI-DESKTOP-033,
//! GPU-QUOTA-013). A dropped [`Submission`] stays counted. There are exactly four ways a count clears, and all
//! of them are the owner's: its fence value was reached, its device was observed lost and quiescent, its device
//! was destroyed, or it never reached the queue.

use super::{bump, Claim, ClaimLedger, State};
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
    outcome: Arc<AtomicU8>,
}

/// One authorized submission: the attachment (module and client), operation and device generation of its claim,
/// and its fence value. It settles nothing by itself and may be dropped: the owner keeps counting the work.
#[derive(Debug)]
pub struct Submission {
    claim: ClaimInfo,
    ledger_key: u64,
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

/// End the flight at `key`. `false` when nothing is in flight there.
fn resolve(state: &mut State, key: (u64, u64), observation: SubmissionObservation) -> bool {
    let Some(flight) = state.flights.remove(&key) else {
        return false;
    };
    flight.outcome.store(code(observation), Ordering::Release);
    if let Some(record) = state.live.get_mut(&flight.claim) {
        record.in_flight = record.in_flight.saturating_sub(1);
    }
    match observation {
        SubmissionObservation::Completed => bump(&mut state.counters.completed),
        SubmissionObservation::DeviceLost => bump(&mut state.counters.completed_lost),
        SubmissionObservation::NoEffect => bump(&mut state.counters.completed_no_effect),
    }
    true
}

/// End every flight of `generation` up to and including fence value `last`. Returns how many.
fn resolve_range(
    state: &mut State,
    generation: u64,
    last: u64,
    observation: SubmissionObservation,
) -> u64 {
    let keys: Vec<(u64, u64)> = state
        .flights
        .range((generation, 0)..=(generation, last))
        .map(|(key, _)| *key)
        .collect();
    let mut resolved: u64 = 0;
    for key in keys {
        if resolve(state, key, observation) {
            bump(&mut resolved);
        }
    }
    resolved
}

/// Whether `generation` is bound and healthy: its work may still run.
fn is_running(state: &State, generation: u64) -> bool {
    matches!(state.device, Some(device) if device.generation == generation && !device.lost)
}

impl ClaimLedger {
    /// Authorize one submission under `claim` and count it in flight.
    pub fn begin_submission(&self, claim: &Claim) -> Result<Submission, AuthorizeRefusal> {
        let mut guard = self.lock();
        let state = &mut *guard;
        if let Err(refusal) = self.authorize_locked(state, claim) {
            bump(&mut state.counters.authorize_refused);
            return Err(refusal);
        }
        let id = claim.info.id.0;
        match state.live.get_mut(&id) {
            None => return Err(AuthorizeRefusal::Unknown),
            Some(record) if record.in_flight >= record.max_in_flight => {
                let max_in_flight = record.max_in_flight;
                bump(&mut state.counters.authorize_refused);
                return Err(AuthorizeRefusal::InFlightLimit { max_in_flight });
            }
            // Cannot overflow: it is below a `u32` bound.
            Some(record) => record.in_flight = record.in_flight.saturating_add(1),
        }
        let sequence = state.next_sequence;
        state.next_sequence = sequence.saturating_add(1);
        let outcome = Arc::new(AtomicU8::new(PENDING));
        state.flights.insert(
            (claim.info.generation, sequence),
            Flight {
                claim: id,
                outcome: Arc::clone(&outcome),
            },
        );
        bump(&mut state.counters.submitted);
        Ok(Submission {
            claim: claim.info,
            ledger_key: self.key,
            sequence,
            outcome,
        })
    }

    /// Whether this ledger issued `submission`.
    pub fn issued(&self, submission: &Submission) -> bool {
        submission.ledger_key == self.key
    }

    /// `submission` never reached the queue: nothing was invoked on the device.
    pub fn submission_no_effect(&self, submission: &Submission) -> Result<(), SubmissionRefusal> {
        let mut state = self.lock();
        let result = if submission.ledger_key != self.key {
            Err(SubmissionRefusal::Unknown)
        } else {
            let key = (submission.claim.generation, submission.sequence);
            if resolve(&mut state, key, SubmissionObservation::NoEffect) {
                Ok(())
            } else {
                Err(SubmissionRefusal::AlreadyResolved)
            }
        };
        if result.is_err() {
            bump(&mut state.counters.submission_refused);
        }
        result
    }

    /// The owner observed the fence of `generation` at `value`: every submission up to it completed. Returns how
    /// many submissions that resolved.
    pub fn resolve_through(&self, generation: u64, value: u64) -> u64 {
        resolve_range(
            &mut self.lock(),
            generation,
            value,
            SubmissionObservation::Completed,
        )
    }

    /// The owner observed the loss of `generation` and that nothing submitted on it is still in use by the
    /// device. Resolves its submissions as lost and returns how many. Resolves nothing for a healthy generation.
    pub fn resolve_lost(&self, generation: u64) -> u64 {
        let mut state = self.lock();
        if is_running(&state, generation) {
            return 0;
        }
        resolve_range(
            &mut state,
            generation,
            u64::MAX,
            SubmissionObservation::DeviceLost,
        )
    }

    /// The owner destroyed the device of `generation`: nothing submitted on it can run and the memory on it is
    /// gone. Resolves its submissions, clears the memory its open claims reported, and returns how many
    /// submissions that resolved. Every reservation stays charged. Does nothing for a healthy generation.
    pub fn generation_retired(&self, generation: u64) -> u64 {
        let mut guard = self.lock();
        let state = &mut *guard;
        if is_running(state, generation) {
            return 0;
        }
        let resolved = resolve_range(
            state,
            generation,
            u64::MAX,
            SubmissionObservation::DeviceLost,
        );
        for record in state.live.values_mut() {
            if record.info.generation == generation {
                record.committed_bytes = 0;
                record.staging_bytes = 0;
            }
        }
        state.counters.retired_in_flight =
            state.counters.retired_in_flight.saturating_add(resolved);
        resolved
    }
}
