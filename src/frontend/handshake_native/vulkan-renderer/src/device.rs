//! Plain-data views of the device generation, and the hand-over a client records against. No Vulkan binding type
//! appears here or anywhere else in the public surface of this unit (HBR-HOST-001/003).

use crate::ids::{ClaimId, DeviceKey};
use crate::ledger::ClaimHold;
use crate::records::ByteQuantity;
use crate::select::DeviceClass;
use crate::vulkan::VulkanGeneration;
use std::sync::Arc;

/// What the owner's device is.
///
/// The integer fields are mechanical identity and may travel in typed diagnostics. The three strings are
/// hardware names for an operator-facing hardware line only; they must never be written to the typed
/// diagnostics ring, which carries no text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceIdentity {
    pub key: DeviceKey,
    pub class: DeviceClass,
    /// Device API version (major, minor, patch).
    pub api_version: [u32; 3],
    /// Raw `VkDriverId`.
    pub driver_id: i32,
    pub driver_version: u32,
    pub queue_family: u32,
    /// Device features enabled at creation (Vulkan core names).
    pub features: Vec<&'static str>,
    /// Device extensions enabled at creation.
    pub extensions: Vec<&'static str>,
    pub name: String,
    pub driver_name: String,
    pub driver_info: String,
}

/// Where a heap's budget and usage numbers come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryBudgetSource {
    /// `VK_EXT_memory_budget`: implementation estimates for this process, which change with system load.
    DriverEstimate,
    /// The device does not offer the extension; budget and usage are unknown.
    Unavailable,
}

/// One memory heap of the device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeapSample {
    pub index: u32,
    /// Heap size as the device reports it.
    pub size_bytes: u64,
    /// `VK_MEMORY_HEAP_DEVICE_LOCAL_BIT`: device-local memory. Without it the heap is host memory the
    /// device shares with the rest of the system.
    pub device_local: bool,
    /// What this process may use from the heap before allocations may fail (an estimate, or unknown).
    pub budget: ByteQuantity,
    /// What this process uses from the heap (an estimate, or unknown).
    pub usage: ByteQuantity,
}

/// The device's own view of memory, to set beside the ledger's accounting. The two are different
/// observations and are never merged: the ledger counts what clients were admitted for, the driver
/// estimates what the process holds (GUI-DESKTOP-031).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemorySnapshot {
    pub generation: u64,
    pub source: MemoryBudgetSource,
    pub heaps: Vec<HeapSample>,
}

/// The raw handles of one device generation, as plain integers.
///
/// The fields follow `hsk_studio_vulkan::DeviceLease` (assets `codex/handshake-studio-assets`): raw Vulkan
/// handles are the C ABI, so the consumer may use any binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceHandles {
    /// Address of `vkGetInstanceProcAddr`.
    pub get_instance_proc_addr: usize,
    pub instance: u64,
    pub physical_device: u64,
    pub device: u64,
    pub queue_family: u32,
    /// Packed API version the instance and device were created for.
    pub api_version: u32,
    pub extensions: Vec<&'static str>,
    pub features: Vec<&'static str>,
    pub generation: u64,
}

/// The device hand-over for a renderer that takes a host-supplied device.
///
/// While a lease exists the owner does not destroy its device, and the claim it was taken under neither closes
/// nor rebinds. Dropping it is the return. It is not `Clone`.
///
/// What whoever uses the handles must keep to:
/// - Objects created from them are destroyed before the lease is dropped, and not while a submission that uses
///   them is [`crate::SubmissionState::Pending`].
/// - Memory allocated with them is charged to the claim ([`crate::GpuOwner::charge`]) before it is allocated.
/// - The lease carries no queue right. The queue is the owner's: work reaches it only through
///   [`crate::GpuOwner::submit`]. Submitting to it, or waiting on it or on the whole device, through these
///   handles would race the owner and is not allowed.
#[derive(Debug)]
pub struct DeviceLease {
    handles: DeviceHandles,
    claim: ClaimId,
    // Released before the hold, so a claim that may settle never still keeps the device.
    _device: Arc<VulkanGeneration>,
    _hold: ClaimHold,
}

impl DeviceLease {
    pub(crate) fn new(
        handles: DeviceHandles,
        claim: ClaimId,
        device: Arc<VulkanGeneration>,
        hold: ClaimHold,
    ) -> Self {
        Self {
            handles,
            claim,
            _device: device,
            _hold: hold,
        }
    }

    pub fn handles(&self) -> &DeviceHandles {
        &self.handles
    }

    /// The claim the lease was taken under.
    pub fn claim(&self) -> ClaimId {
        self.claim
    }

    pub fn generation(&self) -> u64 {
        self.handles.generation
    }
}

/// What a client records into, inside [`crate::GpuOwner::submit`].
///
/// The command buffer is the owner's: primary, for the owner's queue family, begun for one submission. The client
/// only records commands into it, only during the recording call, and leaves it in the recording state; the owner
/// ends, submits and frees it.
#[derive(Debug)]
pub struct Recording {
    command_buffer: u64,
    generation: u64,
}

impl Recording {
    pub(crate) fn new(command_buffer: u64, generation: u64) -> Self {
        Self {
            command_buffer,
            generation,
        }
    }

    /// Raw `VkCommandBuffer`.
    pub fn command_buffer(&self) -> u64 {
        self.command_buffer
    }

    /// The device generation the command buffer belongs to.
    pub fn generation(&self) -> u64 {
        self.generation
    }
}
