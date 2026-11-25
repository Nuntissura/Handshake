//! The owner-side claim ledger: admission against the quota, accounting inside each reservation and
//! exactly-once settlement. Pure state, no Vulkan: every rule here is testable without a GPU.
//!
//! Rules it holds (Master Spec module 15):
//! - Admission is one atomic check-and-reserve under the ledger lock, so two admissions never consume the
//!   same remaining quota (GPU-QUOTA-010). A missing quota denies (GPU-QUOTA-012).
//! - A claim stays charged until its own settlement. Device loss, a new generation, a policy change, a
//!   revoked client or a dropped [`Claim`] handle refund nothing (GPU-QUOTA-013, GUI-DESKTOP-033).
//! - Settlement is refused while work is in flight, closing while device leases are held; it releases each reservation
//!   exactly once, and keeps retained memory charged until its own release (GUI-DESKTOP-034, GPU-QUOTA-014).
//! - A rebind moves the charge to the successor claim in one step: never refunded, never counted twice.
//! - Everything it holds is bounded by the host's policy: bytes, claims, and submissions per claim
//!   (HBR-HOST-004).
//!
//! Scope limit: this ledger accounts for one process. The installation-wide persisted ledger across
//! processes and restarts (GPU-QUOTA-006..010) is the Core accounting port and is not implemented here.

mod rebind;
mod report;
mod settle;
mod submit;

pub use submit::Submission;

use crate::ids::{ClaimId, ClientId, DeviceKey, ModuleId, PolicyRevision};
use crate::outcome::{AdmissionDenial, AuthorizeRefusal, ChargeRefusal};
use crate::records::{
    ChargeClass, ClaimInfo, ClaimRequest, DeviceBinding, LedgerCounters, QuotaPolicy,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use submit::Flight;

/// Unique ledger identities within the process: a claim names the ledger that issued it.
static NEXT_LEDGER_KEY: AtomicU64 = AtomicU64::new(1);

const TOMB_LIVE: u8 = 0;
const TOMB_CLOSED: u8 = 1;
const TOMB_SUPERSEDED: u8 = 2;

/// An admitted claim. Not `Clone`: one holder settles it once. Dropping the handle releases nothing; the
/// ledger keeps the charge until an actual settlement (GPU-QUOTA-013).
pub struct Claim {
    info: ClaimInfo,
    ledger_key: u64,
    /// How the ledger closed the claim once it dropped the live record, so a closed claim is classified
    /// without the ledger keeping a record for ever.
    tomb: Arc<AtomicU8>,
}

impl Claim {
    pub fn info(&self) -> ClaimInfo {
        self.info
    }

    pub fn id(&self) -> ClaimId {
        self.info.id
    }

    pub fn generation(&self) -> u64 {
        self.info.generation
    }

    pub fn reserved_bytes(&self) -> u64 {
        self.info.reserved_bytes
    }
}

impl std::fmt::Debug for Claim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Claim").field("info", &self.info).finish()
    }
}

/// A client's hold on a device under one claim. While it exists the claim neither closes nor rebinds.
/// Not `Clone`; dropping it is the release.
#[derive(Debug)]
pub struct ClaimHold {
    _count: Arc<()>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClaimState {
    /// Work may run; the whole reservation is charged.
    Open,
    /// Work is settled; only retained memory is charged.
    Retained,
}

struct Record {
    info: ClaimInfo,
    state: ClaimState,
    revoked: bool,
    /// Bytes this claim counts against the quota.
    charged_bytes: u64,
    committed_bytes: u64,
    staging_bytes: u64,
    /// Submissions the owner counts as not finished.
    in_flight: u32,
    /// Bound on `in_flight`, from the policy the claim was admitted under.
    max_in_flight: u32,
    /// Shared with every live [`ClaimHold`] of the claim; its strong count is the hold count plus one. New
    /// holds are made under the ledger lock only, so a count of zero seen under the lock stays zero.
    holds: Arc<()>,
    tomb: Arc<AtomicU8>,
}

impl Record {
    fn open(info: ClaimInfo, charged_bytes: u64, max_in_flight: u32, tomb: &Arc<AtomicU8>) -> Self {
        Self {
            info,
            state: ClaimState::Open,
            revoked: false,
            charged_bytes,
            committed_bytes: 0,
            staging_bytes: 0,
            in_flight: 0,
            max_in_flight,
            holds: Arc::new(()),
            tomb: Arc::clone(tomb),
        }
    }

    /// Memory the claim reported inside its reservation. Never above `info.reserved_bytes`.
    fn held_bytes(&self) -> u64 {
        self.committed_bytes.saturating_add(self.staging_bytes)
    }

    fn leases(&self) -> u32 {
        u32::try_from(Arc::strong_count(&self.holds).saturating_sub(1)).unwrap_or(u32::MAX)
    }
}

enum Located {
    Live,
    Closed,
    Superseded,
    Foreign,
}

struct State {
    policy: QuotaPolicy,
    admission_open: bool,
    device: Option<DeviceBinding>,
    last_generation: u64,
    next_claim: u64,
    /// Next fence value of the bound generation; restarts at 1 with each generation.
    next_sequence: u64,
    live: BTreeMap<u64, Record>,
    /// Submissions not observed finished, by (generation, fence value).
    flights: BTreeMap<(u64, u64), Flight>,
    charged_bytes: u64,
    counters: LedgerCounters,
}

fn bump(counter: &mut u64) {
    *counter = counter.saturating_add(1);
}

/// See the module documentation.
pub struct ClaimLedger {
    key: u64,
    state: Mutex<State>,
}

impl ClaimLedger {
    pub fn new(policy: QuotaPolicy) -> Self {
        Self {
            key: NEXT_LEDGER_KEY.fetch_add(1, Ordering::Relaxed),
            state: Mutex::new(State {
                policy,
                admission_open: true,
                device: None,
                last_generation: 0,
                next_claim: 1,
                next_sequence: 1,
                live: BTreeMap::new(),
                flights: BTreeMap::new(),
                charged_bytes: 0,
                counters: LedgerCounters::default(),
            }),
        }
    }

    /// A poisoned lock is taken over: no step below can panic between two writes of the state.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn locate(&self, state: &State, claim: &Claim) -> Located {
        let id = claim.info.id.0;
        if claim.ledger_key != self.key || id == 0 || id >= state.next_claim {
            return Located::Foreign;
        }
        if state.live.contains_key(&id) {
            return Located::Live;
        }
        match claim.tomb.load(Ordering::Acquire) {
            TOMB_CLOSED => Located::Closed,
            TOMB_SUPERSEDED => Located::Superseded,
            _ => Located::Foreign,
        }
    }

    /// Replace the quota policy. Existing claims keep their charge and their original revision and bounds;
    /// only later admissions see the new policy.
    pub fn set_policy(&self, policy: QuotaPolicy) {
        self.lock().policy = policy;
    }

    /// Bind a new device generation and return its number. Generations only grow.
    pub fn bind_device(&self, key: DeviceKey) -> u64 {
        let mut state = self.lock();
        let generation = state.last_generation.saturating_add(1);
        state.last_generation = generation;
        state.next_sequence = 1;
        state.device = Some(DeviceBinding {
            key,
            generation,
            lost: false,
        });
        bump(&mut state.counters.generations);
        generation
    }

    /// The owner has no device (a rebind failed, or it closed). Claims keep their charge.
    pub fn unbind_device(&self) {
        self.lock().device = None;
    }

    /// Mark `generation` lost. `true` only for the first report of the current generation.
    pub fn mark_device_lost(&self, generation: u64) -> bool {
        let mut guard = self.lock();
        let state = &mut *guard;
        match state.device.as_mut() {
            Some(device) if device.generation == generation && !device.lost => {
                device.lost = true;
                bump(&mut state.counters.device_losses);
                true
            }
            _ => false,
        }
    }

    pub fn device(&self) -> Option<DeviceBinding> {
        self.lock().device
    }

    /// Stop admitting. Settlement, release and rebind keep working.
    pub fn close_admission(&self) {
        self.lock().admission_open = false;
    }

    fn admission_check(
        state: &State,
        request: &ClaimRequest,
    ) -> Result<(DeviceBinding, PolicyRevision, u32), AdmissionDenial> {
        if !state.admission_open {
            return Err(AdmissionDenial::AdmissionClosed);
        }
        let device = match state.device {
            None => return Err(AdmissionDenial::DeviceUnavailable),
            Some(device) if device.lost => {
                return Err(AdmissionDenial::DeviceLost {
                    generation: device.generation,
                })
            }
            Some(device) => device,
        };
        let (revision, limit_bytes, max_claims, max_in_flight) = match state.policy {
            QuotaPolicy::Unavailable => return Err(AdmissionDenial::HostQuotaUnavailable),
            QuotaPolicy::Configured {
                revision,
                limit_bytes,
                max_claims,
                max_in_flight_per_claim,
            } => (revision, limit_bytes, max_claims, max_in_flight_per_claim),
        };
        let claims = u32::try_from(state.live.len()).unwrap_or(u32::MAX);
        if claims >= max_claims {
            return Err(AdmissionDenial::ClaimLimit { max_claims });
        }
        let remaining_bytes = limit_bytes.saturating_sub(state.charged_bytes);
        if request.reserve_bytes > remaining_bytes {
            return Err(AdmissionDenial::BudgetExceeded {
                requested_bytes: request.reserve_bytes,
                remaining_bytes,
            });
        }
        Ok((device, revision, max_in_flight))
    }

    /// Admit `request`: reserve its bytes against the quota on the current generation, or deny.
    pub fn admit(&self, request: &ClaimRequest) -> Result<Claim, AdmissionDenial> {
        let mut state = self.lock();
        let (device, policy_revision, max_in_flight) = match Self::admission_check(&state, request)
        {
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
            module: request.module,
            client: request.client,
            operation: request.operation,
            operation_id: request.operation_id,
            device: device.key,
            generation: device.generation,
            policy_revision,
            reserved_bytes: request.reserve_bytes,
            provenance: request.provenance,
        };
        let tomb = Arc::new(AtomicU8::new(TOMB_LIVE));
        let record = Record::open(info, request.reserve_bytes, max_in_flight, &tomb);
        state.live.insert(id, record);
        // Cannot overflow: the reservation fits the limit next to what is already charged.
        state.charged_bytes = state.charged_bytes.saturating_add(request.reserve_bytes);
        bump(&mut state.counters.admitted);
        Ok(Claim {
            info,
            ledger_key: self.key,
            tomb,
        })
    }

    fn authorize_locked(&self, state: &State, claim: &Claim) -> Result<(), AuthorizeRefusal> {
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
        let Some(record) = state.live.get(&claim.info.id.0) else {
            return Err(AuthorizeRefusal::Unknown);
        };
        if record.state == ClaimState::Retained {
            return Err(AuthorizeRefusal::Settled);
        }
        if record.revoked {
            return Err(AuthorizeRefusal::Revoked);
        }
        match state.device {
            None => Err(AuthorizeRefusal::DeviceUnavailable),
            Some(device) if device.lost || device.generation != record.info.generation => {
                Err(AuthorizeRefusal::Lost {
                    generation: record.info.generation,
                })
            }
            Some(_) => Ok(()),
        }
    }

    /// Whether `claim` may submit now. Called before every submission: a stale, revoked or settled claim
    /// never submits (GUI-DESKTOP-036).
    pub fn authorize(&self, claim: &Claim) -> Result<(), AuthorizeRefusal> {
        let mut state = self.lock();
        let result = self.authorize_locked(&state, claim);
        if result.is_err() {
            bump(&mut state.counters.authorize_refused);
        }
        result
    }

    /// Authorize `claim` and take a hold on its device: until the hold is dropped the claim neither closes
    /// nor rebinds.
    pub fn hold(&self, claim: &Claim) -> Result<ClaimHold, AuthorizeRefusal> {
        let mut state = self.lock();
        let result = match self.authorize_locked(&state, claim) {
            Err(refusal) => Err(refusal),
            Ok(()) => match state.live.get(&claim.info.id.0) {
                Some(record) => Ok(ClaimHold {
                    _count: Arc::clone(&record.holds),
                }),
                None => Err(AuthorizeRefusal::Unknown),
            },
        };
        if result.is_err() {
            bump(&mut state.counters.authorize_refused);
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
        let mut state = self.lock();
        let result = match self.authorize_locked(&state, claim) {
            Err(refusal) => Err(ChargeRefusal::NotAuthorized(refusal)),
            Ok(()) => match state.live.get_mut(&claim.info.id.0) {
                None => Err(ChargeRefusal::NotAuthorized(AuthorizeRefusal::Unknown)),
                Some(record) => {
                    let held = record.held_bytes();
                    match held.checked_add(bytes) {
                        Some(total) if total <= record.info.reserved_bytes => {
                            let slot = match class {
                                ChargeClass::Committed => &mut record.committed_bytes,
                                ChargeClass::Staging => &mut record.staging_bytes,
                            };
                            *slot = slot.saturating_add(bytes);
                            Ok(())
                        }
                        _ => Err(ChargeRefusal::ReservationExceeded {
                            reserved_bytes: record.info.reserved_bytes,
                            charged_bytes: held,
                            requested_bytes: bytes,
                        }),
                    }
                }
            },
        };
        if result.is_err() {
            bump(&mut state.counters.charge_refused);
        }
        result
    }

    /// Record that the claim freed `bytes` of `class`. Allowed for a revoked claim and on a lost
    /// generation: reporting freed memory never needs submission rights. The reservation stays charged.
    pub fn discharge(
        &self,
        claim: &Claim,
        class: ChargeClass,
        bytes: u64,
    ) -> Result<(), ChargeRefusal> {
        let mut state = self.lock();
        let result = match self.locate(&state, claim) {
            Located::Foreign => Err(ChargeRefusal::NotAuthorized(AuthorizeRefusal::Unknown)),
            Located::Closed => Err(ChargeRefusal::NotAuthorized(AuthorizeRefusal::Settled)),
            Located::Superseded => Err(ChargeRefusal::NotAuthorized(AuthorizeRefusal::Lost {
                generation: claim.info.generation,
            })),
            Located::Live => match state.live.get_mut(&claim.info.id.0) {
                None => Err(ChargeRefusal::NotAuthorized(AuthorizeRefusal::Unknown)),
                Some(record) if record.state == ClaimState::Retained => {
                    Err(ChargeRefusal::NotAuthorized(AuthorizeRefusal::Settled))
                }
                Some(record) => {
                    let slot = match class {
                        ChargeClass::Committed => &mut record.committed_bytes,
                        ChargeClass::Staging => &mut record.staging_bytes,
                    };
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
            },
        };
        if result.is_err() {
            bump(&mut state.counters.charge_refused);
        }
        result
    }

    /// Revoke every open claim of one client (logout, lost grant, detach). Revoked claims no longer submit;
    /// they keep their charge and still settle once (NATIVE-RENDER-BOOT-005). Returns how many were revoked.
    pub fn revoke_client(&self, module: ModuleId, client: ClientId) -> u64 {
        let mut state = self.lock();
        let mut revoked: u64 = 0;
        for record in state.live.values_mut() {
            if record.info.module == module
                && record.info.client == client
                && record.state == ClaimState::Open
                && !record.revoked
            {
                record.revoked = true;
                revoked = revoked.saturating_add(1);
            }
        }
        state.counters.revoked = state.counters.revoked.saturating_add(revoked);
        revoked
    }
}
