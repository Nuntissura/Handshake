//! One client's side of the ledger: admission, authorization and accounting of its own claims.

use super::{bump, standing, Claim, ClaimHold, Located, Record, Shared, State};
use super::{TOMB_CLOSED, TOMB_SUPERSEDED};
use crate::diag::{OwnerEvent, Subject};
use crate::ids::{ClaimId, ClientId, ModuleId};
use crate::outcome::{
    AdmissionDenial, AuthorizeRefusal, ChargeRefusal, SubmissionRefusal, SubmitRefusal,
};
use crate::records::{ChargeClass, ClaimInfo, ClaimRequest};
use crate::Submission;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// One client's handle on the ledger, issued by [`super::ClaimLedger::attach`] with the client's identity fixed.
///
/// It reaches only claims that were admitted through it: any other claim, also one of the same module, is unknown to
/// it. It cannot see or change the policy, other clients or the device. Not `Clone`; dropping it detaches the
/// client, as a revocation does.
pub struct LedgerClient {
    pub(super) shared: Arc<Shared>,
    module: ModuleId,
    client: ClientId,
    pub(super) seat: u64,
}

impl std::fmt::Debug for LedgerClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LedgerClient")
            .field("module", &self.module)
            .field("client", &self.client)
            .finish()
    }
}

impl Drop for LedgerClient {
    fn drop(&mut self) {
        let key = (self.module, self.client);
        let claims = {
            let mut guard = self.shared.lock();
            let state = &mut *guard;
            // Revoked already, or attached anew since: then there is nothing of this handle to detach.
            if state.seats.get(&key) != Some(&self.seat) {
                return;
            }
            state.seats.remove(&key);
            state.revoke_seat(self.seat)
        };
        self.shared.emit(OwnerEvent::ClientDropped {
            subject: Subject::client(self.module, self.client),
            claims,
        });
    }
}

impl LedgerClient {
    pub(super) fn new(shared: Arc<Shared>, module: ModuleId, client: ClientId, seat: u64) -> Self {
        Self {
            shared,
            module,
            client,
            seat,
        }
    }

    pub fn module(&self) -> ModuleId {
        self.module
    }

    pub fn client(&self) -> ClientId {
        self.client
    }

    /// Whether the host still admits this handle.
    pub fn is_attached(&self) -> bool {
        self.shared.lock().seats.get(&(self.module, self.client)) == Some(&self.seat)
    }

    /// Attribution of something this client did with `claim`: the client is this one, whoever holds the claim.
    pub(super) fn subject(&self, info: &ClaimInfo) -> Subject {
        Subject::claim_used_by(self.module, self.client, info)
    }

    pub(super) fn locate(&self, state: &State, claim: &Claim) -> Located {
        if !Arc::ptr_eq(&claim.ledger, &self.shared) || claim.seat != self.seat {
            return Located::Foreign;
        }
        if state.live.contains_key(&claim.info.id.0) {
            return Located::Live;
        }
        match claim.tomb.load(Ordering::Acquire) {
            TOMB_CLOSED => Located::Closed,
            TOMB_SUPERSEDED => Located::Superseded,
            _ => Located::Foreign,
        }
    }

    /// The record of `claim`, when it may use its device now.
    pub(super) fn authorized<'s>(
        &self,
        state: &'s mut State,
        claim: &Claim,
    ) -> Result<&'s mut Record, AuthorizeRefusal> {
        match self.locate(state, claim) {
            Located::Foreign => return Err(AuthorizeRefusal::Unknown),
            Located::Closed => return Err(AuthorizeRefusal::Settled),
            Located::Superseded => {
                return Err(AuthorizeRefusal::Lost {
                    generation: claim.info.generation,
                })
            }
            Located::Live => {}
        }
        let device = state.device;
        let Some(record) = state.live.get_mut(&claim.info.id.0) else {
            return Err(AuthorizeRefusal::Unknown);
        };
        standing(device, record)?;
        Ok(record)
    }

    /// Authorize `claim` and run `then` on its record, under one lock. A refusal is counted and reported.
    fn with_authorized<T>(
        &self,
        claim: &Claim,
        then: impl FnOnce(&mut Record) -> Result<T, AuthorizeRefusal>,
    ) -> Result<T, AuthorizeRefusal> {
        let result = {
            let mut state = self.shared.lock();
            let result = self.authorized(&mut state, claim).and_then(then);
            if result.is_err() {
                bump(&mut state.counters.authorize_refused);
            }
            result
        };
        self.report_authorize(claim, result)
    }

    pub(super) fn report_authorize<T>(
        &self,
        claim: &Claim,
        result: Result<T, AuthorizeRefusal>,
    ) -> Result<T, AuthorizeRefusal> {
        if let Err(refusal) = &result {
            self.shared.emit(OwnerEvent::AuthorizeRefused {
                subject: self.subject(&claim.info),
                reason: refusal.number(),
            });
        }
        result
    }

    /// Whether `request` fits what the quota has left, as far as bytes and claims go. Reserves nothing.
    pub fn has_room(&self, request: &ClaimRequest) -> bool {
        !matches!(
            self.shared
                .lock()
                .admission_check((self.module, self.client), self.seat, request),
            Err(AdmissionDenial::BudgetExceeded { .. } | AdmissionDenial::ClaimLimit { .. })
        )
    }

    /// Admit `request`: reserve its bytes against the quota on the current generation, or deny.
    pub fn admit(&self, request: &ClaimRequest) -> Result<Claim, AdmissionDenial> {
        let result = self.admit_locked(request);
        let event = match &result {
            Ok(claim) => OwnerEvent::Admitted {
                subject: Subject::claim(&claim.info),
                reserved_bytes: claim.info.reserved_bytes,
            },
            Err(denial) => OwnerEvent::AdmissionDenied {
                subject: Subject::request(self.module, self.client, request),
                reason: denial.number(),
            },
        };
        self.shared.emit(event);
        result
    }

    fn admit_locked(&self, request: &ClaimRequest) -> Result<Claim, AdmissionDenial> {
        let mut guard = self.shared.lock();
        let state = &mut *guard;
        let key = (self.module, self.client);
        let (device, policy_revision, max_in_flight) =
            match state.admission_check(key, self.seat, request) {
                Ok(admitted) => admitted,
                Err(denial) => {
                    bump(&mut state.counters.denied);
                    return Err(denial);
                }
            };
        let id = state.next_claim;
        state.next_claim = id.saturating_add(1);
        let info = ClaimInfo {
            id: ClaimId(id),
            module: self.module,
            client: self.client,
            operation: request.operation,
            operation_id: request.operation_id,
            device: device.key,
            generation: device.generation,
            policy_revision,
            reserved_bytes: request.reserve_bytes,
            provenance: request.provenance,
        };
        let record = Record::open(info, self.seat, request.reserve_bytes, max_in_flight);
        let tomb = Arc::clone(&record.tomb);
        state.live.insert(id, record);
        // Cannot overflow: the reservation fits the limit next to what is already charged.
        state.charged_bytes = state.charged_bytes.saturating_add(request.reserve_bytes);
        bump(&mut state.counters.admitted);
        Ok(Claim {
            info,
            seat: self.seat,
            ledger: Arc::clone(&self.shared),
            tomb,
        })
    }

    /// Whether `claim` may use its device now: a stale, revoked, settled or foreign claim may not
    /// (GUI-DESKTOP-036).
    pub fn authorize(&self, claim: &Claim) -> Result<(), AuthorizeRefusal> {
        self.with_authorized(claim, |_| Ok(()))
    }

    /// [`Self::authorize`], and whether one more submission fits the claim's in-flight bound. Asked before a
    /// client records; [`Self::begin_submission`] decides.
    pub fn authorize_submission(&self, claim: &Claim) -> Result<(), AuthorizeRefusal> {
        self.with_authorized(claim, |record| has_flight_room(record))
    }

    /// Whether `claim` has reached its in-flight bound. Refuses and reports nothing.
    pub fn is_in_flight_full(&self, claim: &Claim) -> bool {
        let mut state = self.shared.lock();
        matches!(self.authorized(&mut state, claim), Ok(record) if has_flight_room(record).is_err())
    }

    /// Authorize `claim` and take a hold on its device: until the hold is dropped the claim neither closes
    /// nor rebinds.
    pub fn hold(&self, claim: &Claim) -> Result<ClaimHold, AuthorizeRefusal> {
        self.with_authorized(claim, |record| {
            record.leases = record.leases.saturating_add(1);
            record.touched = true;
            Ok(())
        })?;
        Ok(ClaimHold {
            ledger: Arc::clone(&self.shared),
            info: claim.info,
        })
    }

    fn report_charge(
        &self,
        claim: &Claim,
        result: Result<(), ChargeRefusal>,
    ) -> Result<(), ChargeRefusal> {
        if let Err(refusal) = &result {
            bump(&mut self.shared.lock().counters.charge_refused);
            self.shared.emit(OwnerEvent::ChargeRefused {
                subject: self.subject(&claim.info),
                reason: refusal.number(),
            });
        }
        result
    }

    /// Record `bytes` of memory the claim now holds inside its reservation.
    pub fn charge(
        &self,
        claim: &Claim,
        class: ChargeClass,
        bytes: u64,
    ) -> Result<(), ChargeRefusal> {
        let result = self.charge_locked(claim, class, bytes);
        self.report_charge(claim, result)
    }

    fn charge_locked(
        &self,
        claim: &Claim,
        class: ChargeClass,
        bytes: u64,
    ) -> Result<(), ChargeRefusal> {
        let mut state = self.shared.lock();
        let record = self
            .authorized(&mut state, claim)
            .map_err(ChargeRefusal::NotAuthorized)?;
        let held = record.held_bytes();
        match held.checked_add(bytes) {
            Some(total) if total <= record.info.reserved_bytes => {
                let slot = slot_of(record, class);
                *slot = slot.saturating_add(bytes);
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

    /// Record that the claim freed `bytes` of `class`. Allowed for a revoked claim and on a lost
    /// generation: reporting freed memory never needs submission rights. The reservation stays charged.
    pub fn discharge(
        &self,
        claim: &Claim,
        class: ChargeClass,
        bytes: u64,
    ) -> Result<(), ChargeRefusal> {
        let result = self.discharge_locked(claim, class, bytes);
        self.report_charge(claim, result)
    }

    fn discharge_locked(
        &self,
        claim: &Claim,
        class: ChargeClass,
        bytes: u64,
    ) -> Result<(), ChargeRefusal> {
        let mut guard = self.shared.lock();
        let state = &mut *guard;
        let refusal = match self.locate(state, claim) {
            Located::Live => None,
            Located::Foreign => Some(AuthorizeRefusal::Unknown),
            Located::Closed => Some(AuthorizeRefusal::Settled),
            Located::Superseded => Some(AuthorizeRefusal::Lost {
                generation: claim.info.generation,
            }),
        };
        if let Some(refusal) = refusal {
            return Err(ChargeRefusal::NotAuthorized(refusal));
        }
        let record = match state.live.get_mut(&claim.info.id.0) {
            Some(record) if record.state == super::ClaimState::Open => record,
            _ => return Err(ChargeRefusal::NotAuthorized(AuthorizeRefusal::Settled)),
        };
        let slot = slot_of(record, class);
        match slot.checked_sub(bytes) {
            Some(left) => {
                *slot = left;
                Ok(())
            }
            None => Err(ChargeRefusal::NotCharged {
                charged_bytes: *slot,
                requested_bytes: bytes,
            }),
        }
    }

    /// Report that nothing was submitted under `claim`, for a reason other than authorization itself.
    pub fn submit_refused(&self, claim: &Claim, refusal: SubmitRefusal) {
        self.shared.emit(OwnerEvent::SubmitRefused {
            subject: self.subject(&claim.info),
            reason: refusal.number(),
        });
    }

    /// Report that `submission` is not observed.
    pub fn submission_refused(&self, submission: &Submission, refusal: SubmissionRefusal) {
        self.shared.emit(OwnerEvent::SubmissionRefused {
            subject: self.subject(&submission.claim_info()),
            reason: refusal.number(),
        });
    }
}

pub(super) fn has_flight_room(record: &Record) -> Result<(), AuthorizeRefusal> {
    if record.in_flight >= record.max_in_flight {
        return Err(AuthorizeRefusal::InFlightLimit {
            max_in_flight: record.max_in_flight,
        });
    }
    Ok(())
}

fn slot_of(record: &mut Record, class: ChargeClass) -> &mut u64 {
    match class {
        ChargeClass::Committed => &mut record.committed_bytes,
        ChargeClass::Staging => &mut record.staging_bytes,
    }
}
