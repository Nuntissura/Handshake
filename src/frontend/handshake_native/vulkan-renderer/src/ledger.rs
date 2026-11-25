//! The owner-side claim ledger: admission against the quota, accounting inside each reservation and
//! exactly-once settlement. Pure state, no Vulkan: every rule here is testable without a GPU.
//!
//! Rules it holds (Master Spec module 15):
//! - Two capabilities. [`ClaimLedger`] is the owner's: policy, clients, device generations and what was observed.
//!   [`LedgerClient`] is one client's: its identity is fixed when the owner issues it, and it reaches only its own
//!   claims (GUI-DESKTOP-029/032/035).
//! - Admission is one atomic check-and-reserve under the ledger lock, so two admissions never consume the
//!   same remaining quota (GPU-QUOTA-010). A missing quota denies (GPU-QUOTA-012). A revoked client is denied until
//!   the owner attaches it anew.
//! - A claim stays charged until its own settlement. Device loss, a new generation, a policy change, a
//!   revoked client or a dropped [`Claim`] refund nothing (GPU-QUOTA-013, GUI-DESKTOP-033). A dropped claim is
//!   closed by the owner, and only after it observed the end of everything under it.
//! - Settlement is refused while work is in flight, closing while device leases are held, and when the reported
//!   outcome contradicts what the owner observed. It releases each reservation exactly once, and keeps retained
//!   memory charged until its own release (GUI-DESKTOP-034, GPU-QUOTA-014).
//! - A rebind moves the charge to the successor claim in one step: never refunded, never counted twice.
//! - Everything it holds is bounded by the host: bytes, claims and submissions per claim by its policy, attached
//!   clients by the handles it asks for (HBR-HOST-004).
//!
//! Every refusal and every change nobody called for (a dropped handle, a reclaimed or leaked claim) is reported
//! through the diagnostics port, with the ledger lock released.
//!
//! Scope limit: this ledger accounts for one process. The installation-wide persisted ledger across
//! processes and restarts (GPU-QUOTA-006..010) is the Core accounting port and is not implemented here.

mod client;
mod rebind;
mod reclaim;
mod report;
mod settle;
mod submit;

pub use client::LedgerClient;
pub use submit::Submission;

use crate::diag::{OwnerDiagnostics, OwnerEvent, Subject};
use crate::ids::{ClaimId, ClientId, DeviceKey, ModuleId, PolicyRevision};
use crate::outcome::{AdmissionDenial, AttachRefusal, AuthorizeRefusal};
use crate::records::{
    ClaimInfo, ClaimRequest, DeviceBinding, LedgerCounters, QuotaPolicy, WorkObserved,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use submit::Flight;

/// Unique ledger identities within the process: a submission names the ledger that issued it.
static NEXT_LEDGER_KEY: AtomicU64 = AtomicU64::new(1);

const TOMB_LIVE: u8 = 0;
const TOMB_CLOSED: u8 = 1;
const TOMB_SUPERSEDED: u8 = 2;

/// An admitted claim of one client. Not `Clone`: one holder settles it once.
///
/// Its holder can always report what was freed and end it, also when the client's handle is gone:
/// [`Self::discharge`], [`Self::settle`], [`Self::release_retained`] (GUI-DESKTOP-035).
///
/// Dropping it without a settlement releases nothing (GPU-QUOTA-013). The owner reports it as abandoned and closes
/// it once it observed the end of its work and no lease or reported memory is left; otherwise it stays charged.
#[must_use = "a dropped claim is abandoned"]
pub struct Claim {
    info: ClaimInfo,
    /// The attachment it was admitted under.
    seat: u64,
    ledger: Arc<Shared>,
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

impl Drop for Claim {
    fn drop(&mut self) {
        if self.tomb.load(Ordering::Acquire) == TOMB_LIVE {
            self.ledger.abandon(self.info.id.0);
        }
    }
}

/// A client's hold on a device under one claim. While it exists the claim neither closes nor rebinds.
/// Not `Clone`; dropping it is the release.
pub struct ClaimHold {
    ledger: Arc<Shared>,
    info: ClaimInfo,
    /// The attachment of the claim.
    seat: u64,
}

impl ClaimHold {
    /// Whether the claim may still use its device. This is how the holder of an idle lease learns of a loss, a
    /// revocation, a dropped claim or a closed owner.
    pub fn standing(&self) -> Result<(), AuthorizeRefusal> {
        let result = {
            let mut guard = self.ledger.lock();
            let state = &mut *guard;
            let result = match state.live.get(&self.info.id.0) {
                Some(record) => standing(state.device, record),
                None => Err(AuthorizeRefusal::Settled),
            };
            if result.is_err() {
                bump(&mut state.counters.authorize_refused);
            }
            result
        };
        if let Err(refusal) = result {
            self.ledger.emit(OwnerEvent::AuthorizeRefused {
                subject: Subject::claim(&self.info),
                reason: refusal.number(),
            });
        }
        result
    }
}

impl std::fmt::Debug for ClaimHold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaimHold")
            .field("claim", &self.info.id)
            .finish()
    }
}

impl Drop for ClaimHold {
    fn drop(&mut self) {
        self.ledger.release_hold(self.info.id.0);
    }
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
    seat: u64,
    state: ClaimState,
    revoked: bool,
    /// Its handle was dropped.
    abandoned: bool,
    leak_reported: bool,
    /// Its generation was observed lost.
    device_lost: bool,
    /// The device of its generation was destroyed, and the memory on it with it.
    device_gone: bool,
    /// It took a lease or reported memory.
    touched: bool,
    /// It replaces a claim on a lost generation.
    rebound: bool,
    /// Bytes this claim counts against the quota.
    charged_bytes: u64,
    committed_bytes: u64,
    staging_bytes: u64,
    /// Submissions the owner counts as not finished.
    in_flight: u32,
    /// Bound on `in_flight`, from the policy the claim was admitted under.
    max_in_flight: u32,
    /// Live [`ClaimHold`]s.
    leases: u32,
    /// What was observed of its own submissions.
    work: WorkObserved,
    /// What was observed of the claims it replaces.
    prior: WorkObserved,
    tomb: Arc<AtomicU8>,
}

impl Record {
    fn open(info: ClaimInfo, seat: u64, charged_bytes: u64, max_in_flight: u32) -> Self {
        Self {
            info,
            seat,
            state: ClaimState::Open,
            revoked: false,
            abandoned: false,
            leak_reported: false,
            device_lost: false,
            device_gone: false,
            touched: false,
            rebound: false,
            charged_bytes,
            committed_bytes: 0,
            staging_bytes: 0,
            in_flight: 0,
            max_in_flight,
            leases: 0,
            work: WorkObserved::default(),
            prior: WorkObserved::default(),
            tomb: Arc::new(AtomicU8::new(TOMB_LIVE)),
        }
    }

    /// Memory the claim reported inside its reservation. Never above `info.reserved_bytes`.
    fn held_bytes(&self) -> u64 {
        self.committed_bytes.saturating_add(self.staging_bytes)
    }
}

/// Whether `record` may use its device now.
fn standing(device: Option<DeviceBinding>, record: &Record) -> Result<(), AuthorizeRefusal> {
    if record.state == ClaimState::Retained {
        return Err(AuthorizeRefusal::Settled);
    }
    if record.abandoned {
        return Err(AuthorizeRefusal::Abandoned);
    }
    if record.revoked {
        return Err(AuthorizeRefusal::Revoked);
    }
    match device {
        None => Err(AuthorizeRefusal::DeviceUnavailable),
        Some(device) if device.lost || device.generation != record.info.generation => {
            Err(AuthorizeRefusal::Lost {
                generation: record.info.generation,
            })
        }
        Some(_) => Ok(()),
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
    /// Highest generation marked lost. A generation is bound after the loss of the one before it.
    last_lost: u64,
    next_claim: u64,
    next_seat: u64,
    /// Next fence value of the bound generation; restarts at 1 with each generation.
    next_sequence: u64,
    /// Attached clients and the attachment each one's handle carries.
    seats: BTreeMap<(ModuleId, ClientId), u64>,
    live: BTreeMap<u64, Record>,
    /// Submissions not observed finished, by (generation, fence value).
    flights: BTreeMap<(u64, u64), Flight>,
    charged_bytes: u64,
    counters: LedgerCounters,
}

impl State {
    /// Whether `generation` is bound and healthy: its work may still run.
    fn is_running(&self, generation: u64) -> bool {
        matches!(self.device, Some(device) if device.generation == generation && !device.lost)
    }

    /// Detach the client that holds `seat`: nothing is admitted under it any more and its open claims stop
    /// submitting. Returns how many did.
    fn revoke_seat(&mut self, seat: u64) -> u64 {
        let mut revoked: u64 = 0;
        for record in self.live.values_mut() {
            if record.seat == seat && record.state == ClaimState::Open && !record.revoked {
                record.revoked = true;
                bump(&mut revoked);
            }
        }
        self.counters.revoked = self.counters.revoked.saturating_add(revoked);
        revoked
    }

    fn admission_check(
        &self,
        key: (ModuleId, ClientId),
        seat: u64,
        request: &ClaimRequest,
    ) -> Result<(DeviceBinding, PolicyRevision, u32), AdmissionDenial> {
        if !self.admission_open {
            return Err(AdmissionDenial::AdmissionClosed);
        }
        if self.seats.get(&key) != Some(&seat) {
            return Err(AdmissionDenial::ClientDetached);
        }
        let device = match self.device {
            None => return Err(AdmissionDenial::DeviceUnavailable),
            Some(device) if device.lost => {
                return Err(AdmissionDenial::DeviceLost {
                    generation: device.generation,
                })
            }
            Some(device) => device,
        };
        let (revision, limit_bytes, max_claims, max_in_flight) = match self.policy {
            QuotaPolicy::Unavailable => return Err(AdmissionDenial::HostQuotaUnavailable),
            QuotaPolicy::Configured {
                revision,
                limit_bytes,
                max_claims,
                max_in_flight_per_claim,
            } => (revision, limit_bytes, max_claims, max_in_flight_per_claim),
        };
        let claims = u32::try_from(self.live.len()).unwrap_or(u32::MAX);
        if claims >= max_claims {
            return Err(AdmissionDenial::ClaimLimit { max_claims });
        }
        // What is left of the quota says what others hold: a client is not told (GPU-QUOTA-015).
        if request.reserve_bytes > limit_bytes.saturating_sub(self.charged_bytes) {
            return Err(AdmissionDenial::BudgetExceeded {
                requested_bytes: request.reserve_bytes,
            });
        }
        Ok((device, revision, max_in_flight))
    }
}

fn bump(counter: &mut u64) {
    *counter = counter.saturating_add(1);
}

/// What the ledger, its clients, claims and holds share.
struct Shared {
    key: u64,
    state: Mutex<State>,
    diagnostics: Arc<dyn OwnerDiagnostics>,
}

impl Shared {
    /// A poisoned lock is taken over: no step below can panic between two writes of the state.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Never called with the lock held.
    fn emit(&self, event: OwnerEvent) {
        self.diagnostics.record(event);
    }

    fn emit_all(&self, events: Vec<OwnerEvent>) {
        for event in events {
            self.emit(event);
        }
    }
}

/// The owner's side of the ledger. See the module documentation.
pub struct ClaimLedger {
    shared: Arc<Shared>,
}

impl ClaimLedger {
    pub fn new(policy: QuotaPolicy, diagnostics: Arc<dyn OwnerDiagnostics>) -> Self {
        let state = State {
            policy,
            admission_open: true,
            device: None,
            last_generation: 0,
            last_lost: 0,
            next_claim: 1,
            next_seat: 1,
            next_sequence: 1,
            seats: BTreeMap::new(),
            live: BTreeMap::new(),
            flights: BTreeMap::new(),
            charged_bytes: 0,
            counters: LedgerCounters::default(),
        };
        Self {
            shared: Arc::new(Shared {
                key: NEXT_LEDGER_KEY.fetch_add(1, Ordering::Relaxed),
                state: Mutex::new(state),
                diagnostics,
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.shared.lock()
    }

    /// Issue the handle of one client. Its identity is what the owner says here and never changes; a client that
    /// already has a live handle gets no second one.
    pub fn attach(
        &self,
        module: ModuleId,
        client: ClientId,
    ) -> Result<LedgerClient, AttachRefusal> {
        let result = {
            let mut guard = self.lock();
            let state = &mut *guard;
            if !state.admission_open {
                Err(AttachRefusal::Closed)
            } else if state.seats.contains_key(&(module, client)) {
                Err(AttachRefusal::AlreadyAttached)
            } else {
                let seat = state.next_seat;
                state.next_seat = seat.saturating_add(1);
                state.seats.insert((module, client), seat);
                bump(&mut state.counters.attached);
                Ok(seat)
            }
        };
        let subject = Subject::client(module, client);
        match result {
            Ok(seat) => {
                self.shared.emit(OwnerEvent::ClientAttached { subject });
                Ok(LedgerClient::new(
                    Arc::clone(&self.shared),
                    module,
                    client,
                    seat,
                ))
            }
            Err(refusal) => {
                self.shared.emit(OwnerEvent::AttachRefused {
                    subject,
                    reason: refusal.number(),
                });
                Err(refusal)
            }
        }
    }

    /// Revoke a client (logout, lost grant, detach): its handle admits nothing any more, also not later, and its
    /// open claims stop submitting. They keep their charge and still settle once (NATIVE-RENDER-BOOT-005). Other
    /// clients are not touched. Returns how many claims were revoked.
    pub fn revoke_client(&self, module: ModuleId, client: ClientId) -> u64 {
        let claims = {
            let mut guard = self.lock();
            let state = &mut *guard;
            match state.seats.remove(&(module, client)) {
                Some(seat) => state.revoke_seat(seat),
                None => 0,
            }
        };
        self.shared.emit(OwnerEvent::ClientRevoked {
            subject: Subject::client(module, client),
            claims,
        });
        claims
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

    /// Mark `generation` lost. `true` only for the first report of the latest generation, bound or, after the owner
    /// closed, kept for its holders.
    pub fn mark_device_lost(&self, generation: u64) -> bool {
        let mut guard = self.lock();
        let state = &mut *guard;
        if generation != state.last_generation || generation <= state.last_lost {
            return false;
        }
        state.last_lost = generation;
        if let Some(device) = state.device.as_mut() {
            device.lost |= device.generation == generation;
        }
        bump(&mut state.counters.device_losses);
        for record in state.live.values_mut() {
            if record.info.generation == generation {
                record.device_lost = true;
            }
        }
        true
    }

    pub fn device(&self) -> Option<DeviceBinding> {
        self.lock().device
    }

    /// Stop admitting and attaching. Settlement, release and rebind keep working.
    pub fn close_admission(&self) {
        self.lock().admission_open = false;
    }
}
