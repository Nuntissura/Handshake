//! The owner's diagnostics port.
//!
//! Content-free by construction: an event is a closed code, counters, generations and the opaque numbers of whom it
//! concerns ([`Subject`]). No text, path, adapter name or authorization provenance can travel through it, so the
//! host can map it onto its typed diagnostics without a free-text channel (GUI-DESKTOP-029/049/050/053,
//! HBR-HOST-005). The host adapter that forwards these events into `internal_diagnostics` (the `VulkanDeviceLost`
//! ring code and the GUI observation producer) is host wiring and is not part of this unit. A diagnostic event
//! never proves a committed operation and grants nothing.
//!
//! Event-driven only: each event is emitted by the call, or the dropped handle, that caused it. Nothing here runs on
//! a timer, once per frame or per successful submission, so a host that draws only on damage receives nothing while
//! it is idle.

use crate::ids::{ClaimId, ClientId, ModuleId, OperationId};
use crate::records::{ClaimInfo, ClaimRequest};

/// Lifecycle phases of the owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerPhase {
    Opening,
    Open,
    DeviceLost,
    Rebinding,
    /// No device: startup or a rebind ended in a typed `Unavailable`.
    Unavailable,
    ShuttingDown,
    Closed,
}

impl OwnerPhase {
    pub const fn number(self) -> u16 {
        match self {
            Self::Opening => 1,
            Self::Open => 2,
            Self::DeviceLost => 3,
            Self::Rebinding => 4,
            Self::Unavailable => 5,
            Self::ShuttingDown => 6,
            Self::Closed => 7,
        }
    }
}

/// Whom an event concerns: opaque numbers the host assigned, for attribution and correlation only. They identify
/// the client the host issued the handle to, never whom a caller claims to be, and they grant nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Subject {
    pub module: ModuleId,
    pub client: ClientId,
    pub claim: Option<ClaimId>,
    pub operation_id: Option<OperationId>,
    /// Device generation the claim is bound to, 0 when the event is about no claim.
    pub generation: u64,
}

impl Subject {
    pub(crate) const fn client(module: ModuleId, client: ClientId) -> Self {
        Self {
            module,
            client,
            claim: None,
            operation_id: None,
            generation: 0,
        }
    }

    pub(crate) const fn request(
        module: ModuleId,
        client: ClientId,
        request: &ClaimRequest,
    ) -> Self {
        Self {
            operation_id: Some(request.operation_id),
            ..Self::client(module, client)
        }
    }

    pub(crate) const fn claim(info: &ClaimInfo) -> Self {
        Self {
            module: info.module,
            client: info.client,
            claim: Some(info.id),
            operation_id: Some(info.operation_id),
            generation: info.generation,
        }
    }

    /// `module` and `client` did something with `info`, which may be another client's claim.
    pub(crate) const fn claim_used_by(
        module: ModuleId,
        client: ClientId,
        info: &ClaimInfo,
    ) -> Self {
        Self {
            module,
            client,
            ..Self::claim(info)
        }
    }
}

/// One mechanical observation of the owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerEvent {
    Phase {
        phase: OwnerPhase,
        generation: u64,
    },
    /// The owning renderer observed `VK_ERROR_DEVICE_LOST` on `generation`. Emitted once per generation.
    VulkanDeviceLost {
        generation: u64,
    },
    /// No Vulkan generation: `reason` is `Unavailable::number()`, `vk_result` the raw result or 0.
    Unavailable {
        reason: u16,
        vk_result: i32,
    },
    Admitted {
        subject: Subject,
        reserved_bytes: u64,
    },
    /// `reason` is `AdmissionDenial::number()`.
    AdmissionDenied {
        subject: Subject,
        reason: u16,
    },
    /// `reason` is `AuthorizeRefusal::number()`.
    AuthorizeRefused {
        subject: Subject,
        reason: u16,
    },
    /// `reason` is `ChargeRefusal::number()`.
    ChargeRefused {
        subject: Subject,
        reason: u16,
    },
    /// `outcome` is `SettleOutcome::number()`.
    Settled {
        subject: Subject,
        outcome: u16,
        released_bytes: u64,
        retained_bytes: u64,
    },
    /// `reason` is `SettleRefusal::number()`.
    SettleRefused {
        subject: Subject,
        reason: u16,
    },
    RetainedReleased {
        subject: Subject,
        released_bytes: u64,
    },
    /// The subject is the successor claim; `superseded` is the claim it replaces.
    ClaimRebound {
        subject: Subject,
        superseded: ClaimId,
    },
    /// `reason` is `RebindRefusal::number()`.
    ClaimRebindRefused {
        subject: Subject,
        reason: u16,
    },
    /// The host revoked a client; `claims` of its claims stopped submitting.
    ClientRevoked {
        subject: Subject,
        claims: u64,
    },
    /// What the owner still held when it closed.
    ShutdownOutstanding {
        open_claims: u64,
        retained_claims: u64,
        charged_bytes: u64,
    },
    /// `reason` is `SubmitRefusal::number()`.
    SubmitRefused {
        subject: Subject,
        reason: u16,
    },
    /// `reason` is `SubmissionRefusal::number()`.
    SubmissionRefused {
        subject: Subject,
        reason: u16,
    },
    /// The device answered a call with a fault that is not device loss: `fault` is `DeviceFault::number()`,
    /// `vk_result` the raw result or 0.
    DeviceFault {
        generation: u64,
        fault: u16,
        vk_result: i32,
    },
    /// The owner destroyed the device of `generation`; `resolved` submissions ended with it.
    GenerationRetired {
        generation: u64,
        resolved: u64,
    },
    /// `reason` is `DeviceRebindRefusal::number()`.
    DeviceRebindRefused {
        reason: u16,
    },
    /// The owner closed and kept its device: `holders` clients hold it, or it did not confirm in time that its work has
    /// ended (0).
    DeviceRetainedAtClose {
        generation: u64,
        holders: u64,
    },
    /// The host issued a handle to this client.
    ClientAttached {
        subject: Subject,
    },
    /// `reason` is `AttachRefusal::number()`.
    AttachRefused {
        subject: Subject,
        reason: u16,
    },
    /// The client dropped its handle; `claims` of its claims stopped submitting.
    ClientDropped {
        subject: Subject,
        claims: u64,
    },
    /// The client dropped the claim without settling it. Nothing is released by that.
    ClaimAbandoned {
        subject: Subject,
        in_flight: u64,
        charged_bytes: u64,
    },
    /// The owner closed an abandoned claim, after it observed the end of everything under it.
    ClaimReclaimed {
        subject: Subject,
        released_bytes: u64,
    },
    /// An abandoned claim stays charged: memory it reported, or work under it, is not known to have ended. Emitted
    /// once per claim. It is reclaimed when its device is destroyed.
    ClaimLeaked {
        subject: Subject,
        leaked_bytes: u64,
    },
    /// A lost device is still held by `holders` clients (never 0). Recovery goes on without them.
    LostDeviceSetAside {
        generation: u64,
        holders: u64,
    },
    /// A device its owner could not destroy at once ([`Self::LostDeviceSetAside`], [`Self::DeviceRetainedAtClose`])
    /// was destroyed, after its last holder and its work were gone; `resolved` submissions ended with it.
    RetainedDeviceDestroyed {
        generation: u64,
        resolved: u64,
    },
    /// The last handle of the owner is gone and the device of `generation` did not confirm that its work has ended: it
    /// was left alive, with its memory and the process's owner slot, and nobody can look at it again.
    DeviceLeaked {
        generation: u64,
    },
    /// The owner closed while a rebind was creating a device on another thread; see [`crate::DeviceFate::Creating`].
    DeviceCreatingAtClose,
}

impl OwnerEvent {
    /// Stable event code.
    pub const fn code(self) -> u16 {
        match self {
            Self::Phase { .. } => 1,
            Self::VulkanDeviceLost { .. } => 2,
            Self::Unavailable { .. } => 3,
            Self::Admitted { .. } => 4,
            Self::AdmissionDenied { .. } => 5,
            Self::AuthorizeRefused { .. } => 6,
            Self::ChargeRefused { .. } => 7,
            Self::Settled { .. } => 8,
            Self::SettleRefused { .. } => 9,
            Self::RetainedReleased { .. } => 10,
            Self::ClaimRebound { .. } => 11,
            Self::ClaimRebindRefused { .. } => 12,
            Self::ClientRevoked { .. } => 13,
            Self::ShutdownOutstanding { .. } => 14,
            Self::SubmitRefused { .. } => 15,
            Self::SubmissionRefused { .. } => 16,
            Self::DeviceFault { .. } => 17,
            Self::GenerationRetired { .. } => 18,
            Self::DeviceRebindRefused { .. } => 19,
            Self::DeviceRetainedAtClose { .. } => 20,
            Self::ClientAttached { .. } => 21,
            Self::AttachRefused { .. } => 22,
            Self::ClientDropped { .. } => 23,
            Self::ClaimAbandoned { .. } => 24,
            Self::ClaimReclaimed { .. } => 25,
            Self::ClaimLeaked { .. } => 26,
            Self::LostDeviceSetAside { .. } => 27,
            Self::RetainedDeviceDestroyed { .. } => 28,
            Self::DeviceLeaked { .. } => 29,
            Self::DeviceCreatingAtClose => 30,
        }
    }

    /// Whom the event concerns. `None` for events about the owner or its device.
    pub const fn subject(self) -> Option<Subject> {
        match self {
            Self::Admitted { subject, .. }
            | Self::AdmissionDenied { subject, .. }
            | Self::AuthorizeRefused { subject, .. }
            | Self::ChargeRefused { subject, .. }
            | Self::Settled { subject, .. }
            | Self::SettleRefused { subject, .. }
            | Self::RetainedReleased { subject, .. }
            | Self::ClaimRebound { subject, .. }
            | Self::ClaimRebindRefused { subject, .. }
            | Self::ClientRevoked { subject, .. }
            | Self::SubmitRefused { subject, .. }
            | Self::SubmissionRefused { subject, .. }
            | Self::ClientAttached { subject }
            | Self::AttachRefused { subject, .. }
            | Self::ClientDropped { subject, .. }
            | Self::ClaimAbandoned { subject, .. }
            | Self::ClaimReclaimed { subject, .. }
            | Self::ClaimLeaked { subject, .. } => Some(subject),
            Self::Phase { .. }
            | Self::VulkanDeviceLost { .. }
            | Self::Unavailable { .. }
            | Self::ShutdownOutstanding { .. }
            | Self::DeviceFault { .. }
            | Self::GenerationRetired { .. }
            | Self::DeviceRebindRefused { .. }
            | Self::DeviceRetainedAtClose { .. }
            | Self::LostDeviceSetAside { .. }
            | Self::RetainedDeviceDestroyed { .. }
            | Self::DeviceLeaked { .. }
            | Self::DeviceCreatingAtClose => None,
        }
    }

    /// The event's numbers as three plain counters, in declaration order of its fields. The subject is separate:
    /// see [`Self::subject`].
    pub const fn counters(self) -> [u64; 3] {
        match self {
            Self::Phase { phase, generation } => [phase.number() as u64, generation, 0],
            Self::VulkanDeviceLost { generation } | Self::DeviceLeaked { generation } => {
                [generation, 0, 0]
            }
            Self::Unavailable { reason, vk_result } => [reason as u64, vk_result as i64 as u64, 0],
            Self::Admitted { reserved_bytes, .. } => [reserved_bytes, 0, 0],
            Self::AdmissionDenied { reason, .. }
            | Self::AuthorizeRefused { reason, .. }
            | Self::ChargeRefused { reason, .. }
            | Self::SettleRefused { reason, .. }
            | Self::ClaimRebindRefused { reason, .. }
            | Self::SubmitRefused { reason, .. }
            | Self::SubmissionRefused { reason, .. }
            | Self::AttachRefused { reason, .. }
            | Self::DeviceRebindRefused { reason } => [reason as u64, 0, 0],
            Self::Settled {
                outcome,
                released_bytes,
                retained_bytes,
                ..
            } => [outcome as u64, released_bytes, retained_bytes],
            Self::RetainedReleased { released_bytes, .. }
            | Self::ClaimReclaimed { released_bytes, .. } => [released_bytes, 0, 0],
            Self::ClaimLeaked { leaked_bytes, .. } => [leaked_bytes, 0, 0],
            Self::ClaimRebound { superseded, .. } => [superseded.get(), 0, 0],
            Self::ClientRevoked { claims, .. } | Self::ClientDropped { claims, .. } => {
                [claims, 0, 0]
            }
            Self::ClientAttached { .. } | Self::DeviceCreatingAtClose => [0, 0, 0],
            Self::ClaimAbandoned {
                in_flight,
                charged_bytes,
                ..
            } => [in_flight, charged_bytes, 0],
            Self::ShutdownOutstanding {
                open_claims,
                retained_claims,
                charged_bytes,
            } => [open_claims, retained_claims, charged_bytes],
            Self::DeviceFault {
                generation,
                fault,
                vk_result,
            } => [generation, fault as u64, vk_result as i64 as u64],
            Self::GenerationRetired {
                generation,
                resolved,
            }
            | Self::RetainedDeviceDestroyed {
                generation,
                resolved,
            } => [generation, resolved, 0],
            Self::DeviceRetainedAtClose {
                generation,
                holders,
            }
            | Self::LostDeviceSetAside {
                generation,
                holders,
            } => [generation, holders, 0],
        }
    }

    /// Whether the event reports a refusal, a loss, a leak or missing capability rather than normal progress.
    pub const fn is_degraded(self) -> bool {
        match self {
            Self::Phase { phase, .. } => {
                matches!(phase, OwnerPhase::DeviceLost | OwnerPhase::Unavailable)
            }
            Self::VulkanDeviceLost { .. }
            | Self::Unavailable { .. }
            | Self::AdmissionDenied { .. }
            | Self::AuthorizeRefused { .. }
            | Self::ChargeRefused { .. }
            | Self::SettleRefused { .. }
            | Self::ClaimRebindRefused { .. }
            | Self::SubmitRefused { .. }
            | Self::SubmissionRefused { .. }
            | Self::AttachRefused { .. }
            | Self::DeviceFault { .. }
            | Self::DeviceRebindRefused { .. }
            | Self::DeviceRetainedAtClose { .. }
            | Self::ClaimAbandoned { .. }
            | Self::ClaimLeaked { .. }
            | Self::LostDeviceSetAside { .. }
            | Self::DeviceLeaked { .. }
            | Self::DeviceCreatingAtClose => true,
            Self::ShutdownOutstanding {
                open_claims,
                retained_claims,
                ..
            } => open_claims > 0 || retained_claims > 0,
            Self::Admitted { .. }
            | Self::Settled { .. }
            | Self::RetainedReleased { .. }
            | Self::ClaimRebound { .. }
            | Self::ClientRevoked { .. }
            | Self::ClientAttached { .. }
            | Self::ClientDropped { .. }
            | Self::ClaimReclaimed { .. }
            | Self::GenerationRetired { .. }
            | Self::RetainedDeviceDestroyed { .. } => false,
        }
    }
}

/// Where the owner reports. Implementations must not panic, block, allocate without bound or call back into the
/// owner: events are emitted on the thread of the call or dropped handle that caused them, with the ledger lock
/// released but possibly while the owner is replacing or observing its device.
pub trait OwnerDiagnostics: Send + Sync {
    fn record(&self, event: OwnerEvent);
}

/// Discards every event.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullDiagnostics;

impl OwnerDiagnostics for NullDiagnostics {
    fn record(&self, _event: OwnerEvent) {}
}
