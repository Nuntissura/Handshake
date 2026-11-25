//! Plain-data records of the claim ledger: what is asked, what was admitted, how it ended and what the
//! owner holds. Byte numbers that were observed, estimated or are unknown stay distinct (GUI-DESKTOP-031,
//! GPU-QUOTA-015).

use crate::ids::{
    AuthorizationProvenance, ClaimId, ClientId, DeviceKey, ModuleId, OperationId, PolicyRevision,
};

/// What the admitted work does on the device (HBR-HOST-003: render, compute, upload, readback, display).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperationKind {
    Render,
    Compute,
    Upload,
    Readback,
    Display,
}

impl OperationKind {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Render => "render",
            Self::Compute => "compute",
            Self::Upload => "upload",
            Self::Readback => "readback",
            Self::Display => "display",
        }
    }
}

/// A byte number together with how it is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteQuantity {
    Measured(u64),
    Estimated(u64),
    Unknown,
}

impl ByteQuantity {
    /// The value only when it was measured.
    pub const fn measured(self) -> Option<u64> {
        match self {
            Self::Measured(bytes) => Some(bytes),
            Self::Estimated(_) | Self::Unknown => None,
        }
    }
}

/// The quota the ledger admits against. `Unavailable` denies every admission: an unknown quota is never
/// treated as unlimited (GPU-QUOTA-012). This unit invents no numeric capacity (GUI-DESKTOP-030); the host
/// supplies the limit from its real policy source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuotaPolicy {
    Unavailable,
    Configured {
        revision: PolicyRevision,
        limit_bytes: u64,
        /// Claims (open or still retaining memory) the ledger holds at once.
        max_claims: u32,
        /// Submissions one claim may have in flight at once. A claim keeps the bound it was admitted under.
        max_in_flight_per_claim: u32,
    },
}

/// One admission request. Who asks is not part of it: module and client are fixed in the handle the host issued
/// ([`crate::GpuClient`]), so a client cannot ask in the name of another one (GUI-DESKTOP-029).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClaimRequest {
    pub operation: OperationKind,
    pub operation_id: OperationId,
    /// Bytes reserved against the quota from admission until the claim closes.
    pub reserve_bytes: u64,
    pub provenance: AuthorizationProvenance,
}

/// Plain-data snapshot of an admitted claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClaimInfo {
    pub id: ClaimId,
    pub module: ModuleId,
    pub client: ClientId,
    pub operation: OperationKind,
    pub operation_id: OperationId,
    pub device: DeviceKey,
    /// Device generation the claim is bound to.
    pub generation: u64,
    /// Policy revision the claim was admitted under; a later policy change does not rewrite it.
    pub policy_revision: PolicyRevision,
    pub reserved_bytes: u64,
    pub provenance: AuthorizationProvenance,
}

/// Memory a claim holds inside its reservation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChargeClass {
    /// Device memory committed to resources.
    Committed,
    /// Transfer staging (upload and readback buffers).
    Staging,
}

/// How the work under a claim ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettleOutcome {
    Completed,
    Canceled,
    Failed,
    /// The device was lost under the work.
    Lost,
    /// The operation was never invoked on the device.
    NoEffect,
    /// The client was dropped without an explicit release (counted, never silent).
    Abandoned,
}

impl SettleOutcome {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Canceled => "canceled",
            Self::Failed => "failed",
            Self::Lost => "lost",
            Self::NoEffect => "no_effect",
            Self::Abandoned => "abandoned",
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::Completed => 1,
            Self::Canceled => 2,
            Self::Failed => 3,
            Self::Lost => 4,
            Self::NoEffect => 5,
            Self::Abandoned => 6,
        }
    }
}

/// The client's operation-specific evidence at settlement (GPU-QUOTA-013/014). The fields are those of the
/// canonical client port's settlement, so the port adapter copies them without loss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settlement {
    /// Submissions not known to be finished. Any non-zero value refuses the settlement.
    pub in_flight: u32,
    /// Bytes the claim still holds on the device after its work ended.
    pub retained: ByteQuantity,
    /// Highest bytes held over the claim's life. Attribution only: it changes no charge.
    pub peak: ByteQuantity,
    pub evictions: u64,
    /// Bytes the driver holds beyond the client's own accounting. Attribution only.
    pub native_driver: ByteQuantity,
    pub outcome: SettleOutcome,
}

impl Settlement {
    /// A settlement with nothing in flight and nothing retained.
    pub const fn clean(outcome: SettleOutcome) -> Self {
        Self {
            in_flight: 0,
            retained: ByteQuantity::Measured(0),
            peak: ByteQuantity::Unknown,
            evictions: 0,
            native_driver: ByteQuantity::Unknown,
            outcome,
        }
    }
}

/// What the owner itself observed of the submissions under one claim, its predecessors before a rebind included.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkObserved {
    /// Submissions whose fence value was reached.
    pub completed: u64,
    /// Submissions that ended with the loss or destruction of their device.
    pub lost: u64,
    /// Submissions the queue never took.
    pub no_effect: u64,
}

impl WorkObserved {
    pub(crate) const fn plus(self, other: Self) -> Self {
        Self {
            completed: self.completed.saturating_add(other.completed),
            lost: self.lost.saturating_add(other.lost),
            no_effect: self.no_effect.saturating_add(other.no_effect),
        }
    }
}

/// How one submission ended. The owner decides this from its own queue, never from what a client reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmissionObservation {
    /// Its fence value was reached.
    Completed,
    /// Its device was lost or destroyed: the work is no longer in use by the device.
    DeviceLost,
    /// It never reached the queue.
    NoEffect,
}

/// Where one submission stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmissionState {
    /// Not observed finished: the claim stays in flight.
    Pending,
    Finished(SubmissionObservation),
}

/// What a settled claim still holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimDisposition {
    /// Nothing: the claim is closed and its whole reservation was released.
    Closed,
    /// The claim keeps `charged_bytes` against the quota until its retained memory is released.
    Retained { charged_bytes: u64 },
}

/// Proof of one settlement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettleReceipt {
    pub claim: ClaimInfo,
    pub settlement: Settlement,
    /// What the owner observed. The settled outcome does not contradict it.
    pub observed: WorkObserved,
    pub disposition: ClaimDisposition,
    /// Bytes returned to the quota by this settlement.
    pub released_bytes: u64,
}

/// Proof of one retained release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReleaseReceipt {
    pub claim: ClaimInfo,
    pub released_bytes: u64,
}

/// What became of the device when its owner closed.
///
/// A device that is [`Self::Held`] or [`Self::Unconfirmed`] is kept, and every later call of the owner, a client or a
/// returned lease looks at it: it is destroyed once nobody holds it and it says that its work has ended. That, or that
/// the last handle of all went before it, is reported once, as [`crate::OwnerEvent::RetainedDeviceDestroyed`] or
/// [`crate::OwnerEvent::DeviceLeaked`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceFate {
    /// There was no device.
    NoDevice,
    Destroyed,
    /// Clients hold it: `holders` leases, recordings or waits, never 0.
    Held {
        holders: u64,
    },
    /// Nobody holds it, but it did not confirm in time that its work has ended.
    Unconfirmed,
    /// A rebind was replacing the lost one on another thread. Its successor is never bound: that thread destroys it as
    /// soon as the driver returns it, and then reports [`crate::OwnerEvent::DeviceRebindRefused`]. Until then the
    /// process's owner slot is taken.
    Creating,
}

/// The device generation the ledger admits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceBinding {
    pub key: DeviceKey,
    pub generation: u64,
    pub lost: bool,
}

/// What the ledger holds right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LedgerUsage {
    pub policy: QuotaPolicy,
    /// Bytes counted against the quota: open reservations plus retained memory.
    pub charged_bytes: u64,
    /// Part of `charged_bytes` reserved by open claims.
    pub reserved_bytes: u64,
    /// Part of `charged_bytes` held by settled claims that retain memory.
    pub retained_bytes: u64,
    /// Committed memory reported inside open reservations.
    pub committed_bytes: u64,
    /// Staging memory reported inside open reservations.
    pub staging_bytes: u64,
    pub open_claims: u64,
    pub retained_claims: u64,
    /// Open claims whose client was revoked and that have not settled yet.
    pub revoked_claims: u64,
    /// Submissions the owner counts as not finished.
    pub in_flight: u64,
    /// Device leases clients still hold.
    pub leases: u64,
    /// Clients the host attached and did not revoke, and whose handle still exists.
    pub attached_clients: u64,
    /// Claims whose handle was dropped and that the owner has not reclaimed yet.
    pub abandoned_claims: u64,
    /// Part of `abandoned_claims`: their work ended, but the memory they reported is not known to be freed.
    pub leaked_claims: u64,
    pub device: Option<DeviceBinding>,
    pub admission_open: bool,
}

/// Usage attributed to one module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModuleUsage {
    pub module: ModuleId,
    pub charged_bytes: u64,
    pub open_claims: u64,
    pub retained_claims: u64,
}

/// Closed counters of the ledger (no text, no identities).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LedgerCounters {
    pub admitted: u64,
    pub denied: u64,
    pub authorize_refused: u64,
    pub charge_refused: u64,
    pub settled: u64,
    pub settled_retaining: u64,
    pub retained_released: u64,
    pub settle_refused: u64,
    pub double_release: u64,
    pub rebound: u64,
    pub rebind_refused: u64,
    /// Client handles issued.
    pub attached: u64,
    pub revoked: u64,
    /// Claims whose handle was dropped without a settlement.
    pub abandoned: u64,
    /// Abandoned claims the owner closed after it observed the end of their work.
    pub reclaimed: u64,
    /// Abandoned claims reported as leaked.
    pub leaked: u64,
    pub device_losses: u64,
    pub generations: u64,
    pub submitted: u64,
    pub completed: u64,
    pub completed_lost: u64,
    pub completed_no_effect: u64,
    pub submission_refused: u64,
    /// In-flight submissions resolved by the owner retiring their generation.
    pub retired_in_flight: u64,
}
