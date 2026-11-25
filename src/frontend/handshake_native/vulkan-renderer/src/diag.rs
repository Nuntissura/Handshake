//! The owner's diagnostics port.
//!
//! Content-free by construction: an event is a closed code plus counters and generations. No text, no
//! path, no module or client identity and no adapter name can travel through it, so the host can map it
//! onto the existing typed diagnostics ring without widening that ring's allowlist (GUI-DESKTOP-049/053,
//! HBR-HOST-005). The host adapter that forwards these events into `internal_diagnostics` (the
//! `VulkanDeviceLost` ring code and the GUI observation producer) is host wiring and is not part of this
//! unit. A diagnostic event never proves a committed operation and grants nothing.
//!
//! Event-driven only: each event is emitted by the call that caused it. Nothing here runs on a timer, once per
//! frame or per successful submission, so a host that draws only on damage receives nothing while it is idle.

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
        generation: u64,
        reserved_bytes: u64,
    },
    /// `reason` is `AdmissionDenial::number()`.
    AdmissionDenied {
        reason: u16,
    },
    /// `reason` is `AuthorizeRefusal::number()`.
    AuthorizeRefused {
        reason: u16,
    },
    /// `reason` is `ChargeRefusal::number()`.
    ChargeRefused {
        reason: u16,
    },
    /// `outcome` is `SettleOutcome::number()`.
    Settled {
        outcome: u16,
        released_bytes: u64,
        retained_bytes: u64,
    },
    /// `reason` is `SettleRefusal::number()`.
    SettleRefused {
        reason: u16,
    },
    RetainedReleased {
        released_bytes: u64,
    },
    ClaimRebound {
        generation: u64,
    },
    /// `reason` is `RebindRefusal::number()`.
    ClaimRebindRefused {
        reason: u16,
    },
    ClientRevoked {
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
        reason: u16,
    },
    /// `reason` is `SubmissionRefusal::number()`.
    SubmissionRefused {
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
    /// The owner closed while its device could not be destroyed: `holders` clients still hold it (0: the
    /// device did not confirm that its work has ended).
    DeviceRetainedAtClose {
        generation: u64,
        holders: u64,
    },
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
        }
    }

    /// The event's numbers as three plain counters, in declaration order of its fields.
    pub const fn counters(self) -> [u64; 3] {
        match self {
            Self::Phase { phase, generation } => [phase.number() as u64, generation, 0],
            Self::VulkanDeviceLost { generation } => [generation, 0, 0],
            Self::Unavailable { reason, vk_result } => [reason as u64, vk_result as i64 as u64, 0],
            Self::Admitted {
                generation,
                reserved_bytes,
            } => [generation, reserved_bytes, 0],
            Self::AdmissionDenied { reason }
            | Self::AuthorizeRefused { reason }
            | Self::ChargeRefused { reason }
            | Self::SettleRefused { reason }
            | Self::ClaimRebindRefused { reason }
            | Self::SubmitRefused { reason }
            | Self::SubmissionRefused { reason }
            | Self::DeviceRebindRefused { reason } => [reason as u64, 0, 0],
            Self::Settled {
                outcome,
                released_bytes,
                retained_bytes,
            } => [outcome as u64, released_bytes, retained_bytes],
            Self::RetainedReleased { released_bytes } => [released_bytes, 0, 0],
            Self::ClaimRebound { generation } => [generation, 0, 0],
            Self::ClientRevoked { claims } => [claims, 0, 0],
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
            } => [generation, resolved, 0],
            Self::DeviceRetainedAtClose {
                generation,
                holders,
            } => [generation, holders, 0],
        }
    }

    /// Whether the event reports a refusal, a loss or missing capability rather than normal progress.
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
            | Self::DeviceFault { .. }
            | Self::DeviceRebindRefused { .. }
            | Self::DeviceRetainedAtClose { .. } => true,
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
            | Self::GenerationRetired { .. } => false,
        }
    }
}

/// Where the owner reports. Implementations must not panic, block, allocate without bound or call back into the
/// owner: events are emitted on the caller's thread, with the ledger lock released but possibly while the owner
/// is replacing or observing its device.
pub trait OwnerDiagnostics: Send + Sync {
    fn record(&self, event: OwnerEvent);
}

/// Discards every event.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullDiagnostics;

impl OwnerDiagnostics for NullDiagnostics {
    fn record(&self, _event: OwnerEvent) {}
}
