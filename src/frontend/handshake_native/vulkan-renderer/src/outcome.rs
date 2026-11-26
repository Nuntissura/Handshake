//! Typed outcomes. Every refusal is a closed code; none carries driver text or content, and none is ever
//! answered with a CPU, Direct3D or OpenGL substitute (GUI-DESKTOP-020/023/026, HBR-HOST-003).

use crate::records::WorkObserved;

/// Why no Vulkan generation exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    /// The Vulkan loader could not be loaded.
    LoaderMissing,
    /// The loader refused to create an instance.
    InstanceRefused,
    /// The loader, or every enumerated device, is below the Vulkan baseline.
    ApiBelowBaseline,
    /// The loader enumerated no physical device.
    NoPhysicalDevice,
    /// Only software (CPU) implementations were found.
    SoftwareDeviceRefused,
    /// Only drivers layered over a non-Vulkan API (Direct3D 12) were found.
    LayeredDriverRefused,
    /// A device exists but offers no graphics queue, or none of a known hardware type.
    NoSuitableDevice,
    /// The device required for a rebind (same stable identity) is no longer enumerated.
    RequiredDeviceMissing,
    /// The picked device does not report a Vulkan feature the owner enables for its clients.
    FeatureMissing,
    /// The driver refused to create a logical device.
    DeviceCreationRefused,
    /// The physical device is lost: creating a logical device returned `VK_ERROR_DEVICE_LOST`.
    PhysicalDeviceLost,
    /// Host or device memory ran out during startup.
    OutOfMemory,
    /// The process already has its one GPU owner.
    OwnerAlreadyOpen,
    /// Any other Vulkan result (raw `VkResult`, never text).
    Driver(i32),
    /// The host asked for presentation, and the loader, the device or its queue cannot present to windows of this
    /// platform (`VK_KHR_surface`, `VK_KHR_win32_surface`, `VK_KHR_swapchain`, Win32 presentation support).
    PresentationUnsupported,
}

impl Unavailable {
    /// Every reason once, `Driver` with a placeholder result.
    pub const ALL: [Self; 15] = [
        Self::LoaderMissing,
        Self::InstanceRefused,
        Self::ApiBelowBaseline,
        Self::NoPhysicalDevice,
        Self::SoftwareDeviceRefused,
        Self::LayeredDriverRefused,
        Self::NoSuitableDevice,
        Self::RequiredDeviceMissing,
        Self::DeviceCreationRefused,
        Self::PhysicalDeviceLost,
        Self::OutOfMemory,
        Self::OwnerAlreadyOpen,
        Self::Driver(0),
        Self::FeatureMissing,
        Self::PresentationUnsupported,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::LoaderMissing => "vulkan_loader_missing",
            Self::InstanceRefused => "vulkan_instance_refused",
            Self::ApiBelowBaseline => "api_below_baseline",
            Self::NoPhysicalDevice => "no_physical_device",
            Self::SoftwareDeviceRefused => "software_device_refused",
            Self::LayeredDriverRefused => "layered_driver_refused",
            Self::NoSuitableDevice => "no_suitable_device",
            Self::RequiredDeviceMissing => "required_device_missing",
            Self::DeviceCreationRefused => "device_creation_refused",
            Self::PhysicalDeviceLost => "physical_device_lost",
            Self::OutOfMemory => "out_of_memory",
            Self::OwnerAlreadyOpen => "owner_already_open",
            Self::FeatureMissing => "feature_missing",
            Self::Driver(_) => "vk_error",
            Self::PresentationUnsupported => "presentation_unsupported",
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::LoaderMissing => 1,
            Self::InstanceRefused => 2,
            Self::ApiBelowBaseline => 3,
            Self::NoPhysicalDevice => 4,
            Self::SoftwareDeviceRefused => 5,
            Self::LayeredDriverRefused => 6,
            Self::NoSuitableDevice => 7,
            Self::RequiredDeviceMissing => 8,
            Self::DeviceCreationRefused => 9,
            Self::PhysicalDeviceLost => 10,
            Self::OutOfMemory => 11,
            Self::OwnerAlreadyOpen => 12,
            Self::Driver(_) => 13,
            Self::FeatureMissing => 14,
            Self::PresentationUnsupported => 15,
        }
    }
}

/// A fault observed on a live generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceFault {
    /// `VK_ERROR_DEVICE_LOST`, or a fence that only a lost device signals. The generation is finished; rebind through
    /// the owner.
    Lost {
        generation: u64,
    },
    OutOfMemory,
    /// No generation is bound (startup failed, a rebind failed, or the owner is closed).
    NoDevice,
    /// Any other Vulkan result (raw `VkResult`, never text).
    Driver(i32),
}

impl DeviceFault {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Lost { .. } => "device_lost",
            Self::OutOfMemory => "out_of_memory",
            Self::NoDevice => "no_device",
            Self::Driver(_) => "vk_error",
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::Lost { .. } => 1,
            Self::OutOfMemory => 2,
            Self::NoDevice => 3,
            Self::Driver(_) => 4,
        }
    }
}

/// Why the host does not get a client handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachRefusal {
    /// This module and client already have a live handle. Use it, or revoke the client first.
    AlreadyAttached,
    /// The owner stopped admitting (shutdown).
    Closed,
}

impl AttachRefusal {
    pub const fn key(self) -> &'static str {
        match self {
            Self::AlreadyAttached => "client_already_attached",
            Self::Closed => "owner_closed",
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::AlreadyAttached => 1,
            Self::Closed => 2,
        }
    }
}

/// Why an admission is refused. Nothing is charged by a refused admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionDenial {
    /// No configured quota exists (GPU-QUOTA-012: fail closed, never an unbounded default).
    HostQuotaUnavailable,
    /// The reservation does not fit what the quota still has. How much that is, is the host's to know
    /// ([`crate::GpuOwner::usage`]): it says what other clients hold.
    BudgetExceeded { requested_bytes: u64 },
    /// The ledger already holds as many claims as the policy allows.
    ClaimLimit { max_claims: u32 },
    /// The current generation is lost; nothing new is admitted until the owner rebinds.
    DeviceLost { generation: u64 },
    /// No generation is bound.
    DeviceUnavailable,
    /// The owner stopped admitting (shutdown).
    AdmissionClosed,
    /// The host revoked this client. Only a new handle from the host is admitted again.
    ClientDetached,
}

impl AdmissionDenial {
    pub const fn key(self) -> &'static str {
        match self {
            Self::HostQuotaUnavailable => "host_quota_unavailable",
            Self::BudgetExceeded { .. } => "budget_exceeded",
            Self::ClaimLimit { .. } => "claim_limit",
            Self::DeviceLost { .. } => "device_lost",
            Self::DeviceUnavailable => "device_unavailable",
            Self::AdmissionClosed => "admission_closed",
            Self::ClientDetached => "client_detached",
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::HostQuotaUnavailable => 1,
            Self::BudgetExceeded { .. } => 2,
            Self::DeviceLost { .. } => 3,
            Self::DeviceUnavailable => 4,
            Self::AdmissionClosed => 5,
            Self::ClaimLimit { .. } => 6,
            Self::ClientDetached => 7,
        }
    }
}

/// Why a claim may not submit, charge or lease now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorizeRefusal {
    /// The claim's generation is lost or superseded; rebind the claim.
    Lost { generation: u64 },
    /// The client was revoked. The claim still settles once.
    Revoked,
    /// The claim's work was settled; it only holds retained memory, or nothing.
    Settled,
    /// No generation is bound.
    DeviceUnavailable,
    /// The claim was not issued by this owner to this client.
    Unknown,
    /// The claim already has as many submissions in flight as its policy allows. Submit again once one of
    /// them is observed finished.
    InFlightLimit { max_in_flight: u32 },
    /// The claim was dropped. Its lease is only there to clean up.
    Abandoned,
}

impl AuthorizeRefusal {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Lost { .. } => "device_lost",
            Self::Revoked => "claim_revoked",
            Self::Settled => "claim_settled",
            Self::DeviceUnavailable => "device_unavailable",
            Self::Unknown => "claim_unknown",
            Self::InFlightLimit { .. } => "in_flight_limit",
            Self::Abandoned => "claim_abandoned",
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::Lost { .. } => 1,
            Self::Revoked => 2,
            Self::Settled => 3,
            Self::DeviceUnavailable => 4,
            Self::Unknown => 5,
            Self::InFlightLimit { .. } => 6,
            Self::Abandoned => 7,
        }
    }
}

/// Why a lease is not to be returned yet, nor the objects made from it destroyed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseRefusal {
    /// Work of this client on the device is not observed finished, under this claim or another one.
    InFlight { in_flight: u64 },
}

impl ReleaseRefusal {
    pub const fn key(self) -> &'static str {
        match self {
            Self::InFlight { .. } => "release_in_flight",
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::InFlight { .. } => 1,
        }
    }
}

/// Why a charge against a claim's reservation is refused. A refused charge changes nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChargeRefusal {
    NotAuthorized(AuthorizeRefusal),
    /// The charge would exceed what the claim reserved at admission.
    ReservationExceeded {
        reserved_bytes: u64,
        charged_bytes: u64,
        requested_bytes: u64,
    },
    /// A discharge larger than what is charged in that class.
    NotCharged {
        charged_bytes: u64,
        requested_bytes: u64,
    },
}

impl ChargeRefusal {
    pub const fn key(self) -> &'static str {
        match self {
            Self::NotAuthorized(refusal) => refusal.key(),
            Self::ReservationExceeded { .. } => "reservation_exceeded",
            Self::NotCharged { .. } => "not_charged",
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::NotAuthorized(_) => 1,
            Self::ReservationExceeded { .. } => 2,
            Self::NotCharged { .. } => 3,
        }
    }
}

/// Why a settlement or a retained release is refused. A refused call changes nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettleRefusal {
    /// The claim was already closed.
    DoubleRelease,
    /// The owner already closed the claim when it was rebound to a successor.
    LateRelease,
    /// The claim was not issued by this owner to this client.
    StaleClaim,
    /// Work under the claim may still run; settle again once it has finished.
    InFlight { in_flight: u32 },
    /// The claim still holds device leases: objects made from them may exist. Drop the leases, then settle
    /// or release again. A settlement that retains memory keeps its leases.
    LeaseOutstanding { leases: u32 },
    /// The claim reports more retained memory than it reserved.
    RetainedExceedsReservation {
        reserved_bytes: u64,
        retained_bytes: u64,
    },
    /// The claim reports less retained memory than it still has charged. Discharge what was freed, or
    /// report what is kept.
    RetainedBelowCharged {
        charged_bytes: u64,
        retained_bytes: u64,
    },
    /// A retained release was asked for a claim whose work is not settled yet.
    NotSettled,
    /// The claim's work is already settled; only a retained release remains.
    AlreadySettled,
    /// The reported outcome is not what the owner observed: `Completed` needs work that completed and none that was
    /// lost, `NoEffect` that nothing ran, `Lost` a loss.
    OutcomeContradicted { observed: WorkObserved },
    /// Images, buffers or surfaces the owner made for the claim still exist. Drop them; the owner destroys them once
    /// nothing that may use them runs ([`crate::GpuClient::poll`] lets it look), then settle or release again.
    ResourcesOutstanding { resources: u32 },
}

impl SettleRefusal {
    pub const fn key(self) -> &'static str {
        match self {
            Self::DoubleRelease => "double_release",
            Self::LateRelease => "late_release",
            Self::StaleClaim => "stale_claim",
            Self::InFlight { .. } => "settle_in_flight",
            Self::LeaseOutstanding { .. } => "settle_lease_outstanding",
            Self::RetainedExceedsReservation { .. } => "retained_exceeds_reservation",
            Self::RetainedBelowCharged { .. } => "retained_below_charged",
            Self::NotSettled => "not_settled",
            Self::AlreadySettled => "already_settled",
            Self::OutcomeContradicted { .. } => "outcome_contradicted",
            Self::ResourcesOutstanding { .. } => "settle_resources_outstanding",
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::DoubleRelease => 1,
            Self::LateRelease => 2,
            Self::StaleClaim => 3,
            Self::InFlight { .. } => 4,
            Self::RetainedExceedsReservation { .. } => 5,
            Self::NotSettled => 6,
            Self::AlreadySettled => 7,
            Self::LeaseOutstanding { .. } => 8,
            Self::RetainedBelowCharged { .. } => 9,
            Self::OutcomeContradicted { .. } => 10,
            Self::ResourcesOutstanding { .. } => 11,
        }
    }
}

/// Why a claim is not rebound to the current generation. A refused rebind keeps the claim as it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RebindRefusal {
    /// The claim already belongs to the current generation.
    NotLost,
    /// The current generation is itself lost; the owner must rebind its device first.
    Lost { generation: u64 },
    /// No generation is bound.
    DeviceUnavailable,
    /// The current generation runs on a different adapter than the claim's.
    DeviceChanged,
    /// The claim is revoked, settled or closed.
    NotOpen,
    /// Submissions under the claim are not known to be finished. The owner resolves them when it observes
    /// the loss or destroys the lost generation; rebind after that.
    InFlight { in_flight: u32 },
    /// The claim still holds leases on its lost generation. Drop them, then rebind.
    LeaseOutstanding { leases: u32 },
    /// The lost device still exists and the claim reports memory on it. Free and discharge it, or wait until the
    /// device is destroyed, then rebind.
    MemoryHeld { bytes: u64 },
    /// The claim was not issued by this owner to this client.
    Unknown,
    /// Resources the owner made for the claim on the lost device still exist. Drop them; they end with the device or
    /// once the owner sees that nothing uses them, then rebind.
    ResourcesHeld { resources: u32 },
}

impl RebindRefusal {
    pub const fn key(self) -> &'static str {
        match self {
            Self::NotLost => "claim_not_lost",
            Self::Lost { .. } => "device_lost",
            Self::DeviceUnavailable => "device_unavailable",
            Self::DeviceChanged => "device_changed",
            Self::NotOpen => "claim_not_open",
            Self::InFlight { .. } => "rebind_in_flight",
            Self::LeaseOutstanding { .. } => "rebind_lease_outstanding",
            Self::MemoryHeld { .. } => "rebind_memory_held",
            Self::Unknown => "claim_unknown",
            Self::ResourcesHeld { .. } => "rebind_resources_held",
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::NotLost => 1,
            Self::Lost { .. } => 2,
            Self::DeviceUnavailable => 3,
            Self::DeviceChanged => 4,
            Self::NotOpen => 5,
            Self::Unknown => 6,
            Self::InFlight { .. } => 7,
            Self::LeaseOutstanding { .. } => 8,
            Self::MemoryHeld { .. } => 9,
            Self::ResourcesHeld { .. } => 10,
        }
    }
}

/// Why a submission is not resolved or not observed. A refused call changes nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmissionRefusal {
    /// It is already resolved; its outcome is in [`crate::Submission::state`].
    AlreadyResolved,
    /// The submission was not issued by this owner to this client.
    Unknown,
    /// The device did not answer; the submission stays pending.
    Device(DeviceFault),
}

impl SubmissionRefusal {
    pub const fn key(self) -> &'static str {
        match self {
            Self::AlreadyResolved => "submission_already_resolved",
            Self::Unknown => "submission_unknown",
            Self::Device(fault) => fault.key(),
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::AlreadyResolved => 1,
            Self::Unknown => 2,
            Self::Device(_) => 3,
        }
    }
}

/// The client gave up recording: nothing is submitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordAborted;

/// What a bounded look at the device found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceHealth {
    /// Everything submitted has completed.
    Idle,
    /// Submitted work was still running when the bound passed. That is neither a loss nor a fault.
    Busy,
}

/// Why no submission ticket was returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitRefusal {
    NotAuthorized(AuthorizeRefusal),
    /// The client's recording call returned [`RecordAborted`].
    RecordAborted,
    /// The device refused a step. With [`DeviceFault::Lost`] the work may have reached the queue before the
    /// device was lost; in every other case nothing ran.
    Device(DeviceFault),
}

impl SubmitRefusal {
    pub const fn key(self) -> &'static str {
        match self {
            Self::NotAuthorized(refusal) => refusal.key(),
            Self::RecordAborted => "record_aborted",
            Self::Device(fault) => fault.key(),
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::NotAuthorized(_) => 1,
            Self::RecordAborted => 2,
            Self::Device(_) => 3,
        }
    }
}

/// Why the owner did not replace its device generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceRebindRefusal {
    /// The current generation is not lost; a healthy device is never replaced.
    NotLost,
    /// The owner is closed.
    Closed,
    /// Clients hold the lost device (leases, or a recording or wait in progress), and
    /// [`crate::MAX_HELD_LOST_DEVICES`] earlier lost devices are still held as well. Below that bound a held device
    /// is set aside and recovery goes on without its holders. No device is destroyed under a client: once one of
    /// the held devices is returned, the host rebinds again.
    DeviceHeld { holders: u32 },
    /// The lost device did not confirm that its work has ended, so it is not destroyed. Rebind again later.
    NotQuiescent,
    /// No successor generation could be created. The owner stays without a device.
    Unavailable(Unavailable),
    /// Another call is replacing the device right now.
    InProgress,
}

impl DeviceRebindRefusal {
    pub const fn key(self) -> &'static str {
        match self {
            Self::NotLost => "device_not_lost",
            Self::Closed => "owner_closed",
            Self::DeviceHeld { .. } => "device_held",
            Self::NotQuiescent => "device_not_quiescent",
            Self::Unavailable(reason) => reason.key(),
            Self::InProgress => "rebind_in_progress",
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::NotLost => 1,
            Self::Closed => 2,
            Self::DeviceHeld { .. } => 3,
            Self::NotQuiescent => 4,
            Self::Unavailable(_) => 5,
            Self::InProgress => 6,
        }
    }
}
