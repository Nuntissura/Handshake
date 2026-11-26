//! Resources the owner makes for a claim: charged when they are made, inside the claim's reservation, and released
//! exactly once, when the owner destroyed them after observing the end of every submission that may use them.
//!
//! A claim with such a resource neither closes nor rebinds, like one with a lease: settling it clean, releasing what it
//! retains, reclaiming it after its holder dropped it and moving it to a new generation all wait until the owner has
//! destroyed its resources. The destruction of their device does that for every one of them at once.

use super::client::Actor;
use super::reclaim::reclaim;
use super::{bump, slot_of_owned, standing, Claim, ClaimLedger, LedgerClient, Shared};
use crate::diag::{OwnerEvent, Subject};
use crate::outcome::{AuthorizeRefusal, ChargeRefusal};
use crate::records::ChargeClass;

/// What one destroyed resource of a claim was charged with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OwnedEnd {
    pub(crate) claim: u64,
    pub(crate) class: ChargeClass,
    pub(crate) bytes: u64,
}

impl LedgerClient {
    /// The owner made a resource of `bytes` for `claim`: charge it inside the claim's reservation, or refuse and change
    /// nothing. A charge of 0 still counts the resource.
    pub(crate) fn own(
        &self,
        claim: &Claim,
        class: ChargeClass,
        bytes: u64,
    ) -> Result<(), ChargeRefusal> {
        let result = {
            let mut state = self.shared.lock();
            match self.authorized(&mut state, claim) {
                Err(refusal) => Err(ChargeRefusal::NotAuthorized(refusal)),
                Ok(record) => {
                    let held = record.held_bytes();
                    match held.checked_add(bytes) {
                        Some(total) if total <= record.info.reserved_bytes => {
                            let slot = slot_of_owned(record, class);
                            *slot = slot.saturating_add(bytes);
                            record.owned = record.owned.saturating_add(1);
                            record.touched = true;
                            Ok(())
                        }
                        _ => Err(ChargeRefusal::ReservationExceeded {
                            reserved_bytes: record.info.reserved_bytes,
                            charged_bytes: held,
                            requested_bytes: bytes,
                        }),
                    }
                }
            }
        };
        let actor = self.actor(claim);
        if result.is_ok() {
            bump(&mut self.shared.lock().counters.resources_made);
            self.shared.emit(OwnerEvent::ResourceMade {
                subject: actor.subject,
                bytes,
            });
        }
        self.shared.report_owned_charge(actor, result)
    }

    /// Identity of the ledger that issued this handle.
    pub(crate) fn ledger_key(&self) -> u64 {
        self.shared.key
    }

    /// The attachment this handle carries.
    pub(crate) fn seat(&self) -> u64 {
        self.seat
    }

    /// Attribution of something this client did with `claim`.
    pub(crate) fn subject_of(&self, claim: &Claim) -> Subject {
        self.subject(&claim.info)
    }
}

impl ClaimLedger {
    /// Whether claim `claim` may still use its device: what [`crate::DeviceLease::handles`] answers, for a resource
    /// whose holder may no longer have the claim.
    pub(crate) fn standing_of(&self, claim: u64) -> Result<(), AuthorizeRefusal> {
        let state = self.lock();
        match state.live.get(&claim) {
            Some(record) => standing(state.device, record),
            None => Err(AuthorizeRefusal::Settled),
        }
    }

    /// Submissions of the client of attachment `seat` on `generation` that are not observed finished, under any of
    /// its claims: any of them may use a resource of the client.
    pub(crate) fn seat_in_flight(&self, generation: u64, seat: u64) -> u64 {
        let state = self.lock();
        let count = state
            .flights
            .range((generation, 0)..=(generation, u64::MAX))
            .filter(|(_, flight)| flight.seat == seat)
            .count();
        u64::try_from(count).unwrap_or(u64::MAX)
    }
}

impl Shared {
    fn report_owned_charge(
        &self,
        actor: Actor,
        result: Result<(), ChargeRefusal>,
    ) -> Result<(), ChargeRefusal> {
        if let Err(refusal) = &result {
            bump(&mut self.lock().counters.charge_refused);
            self.emit(OwnerEvent::ChargeRefused {
                subject: actor.subject,
                reason: refusal.number(),
            });
        }
        result
    }
}

impl ClaimLedger {
    /// The owner destroyed these resources: release what each was charged with, once, and close claims that were only
    /// waiting for them after their holder dropped them.
    pub(crate) fn owned_ended(&self, ended: &[OwnedEnd]) {
        if ended.is_empty() {
            return;
        }
        let mut events = Vec::new();
        {
            let mut guard = self.lock();
            let state = &mut *guard;
            for end in ended {
                let Some(record) = state.live.get_mut(&end.claim) else {
                    // Its device was destroyed first, which ended the resource with the claim's other ones.
                    continue;
                };
                if record.owned == 0 {
                    continue;
                }
                record.owned -= 1;
                let slot = slot_of_owned(record, end.class);
                *slot = slot.saturating_sub(end.bytes);
                bump(&mut state.counters.resources_ended);
                events.push(OwnerEvent::ResourceEnded {
                    subject: Subject::claim(&record.info),
                    bytes: end.bytes,
                });
                reclaim(state, end.claim, &mut events);
            }
        }
        self.shared.emit_all(events);
    }
}
