//! Plain-data views of the device generation, and the hand-over a client records against. No Vulkan binding type
//! appears here or anywhere else in the public surface of this unit (HBR-HOST-001/003).

use crate::ids::{ClaimId, DeviceKey};
use crate::ledger::ClaimHold;
use crate::outcome::{AuthorizeRefusal, ReleaseRefusal};
use crate::owner::OwnerShared;
use crate::records::ByteQuantity;
use crate::refusal::RecordRefusal;
use crate::resource::sealed::TargetOf;
use crate::resource::{FrameKey, ImageLayout, ImageTarget, OwnerBuffer};
use crate::select::DeviceClass;
use crate::vulkan::{ImageCommand, RecordedWork, VulkanGeneration};
use std::sync::Arc;
use std::time::Duration;

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

/// Where a heap's memory is, as far as the device says so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryPlacement {
    /// The device's own memory.
    Dedicated,
    /// Host memory the device shares with the rest of the system.
    Shared,
    /// The device does not say. An integrated or virtual device reports memory as device-local whether it is carved
    /// out for it or ordinary system memory, so its device-local heaps are attributed to neither.
    Unknown,
}

impl MemoryPlacement {
    pub const fn of(class: DeviceClass, device_local: bool) -> Self {
        match (class, device_local) {
            (_, false) => Self::Shared,
            (DeviceClass::Discrete, true) => Self::Dedicated,
            (_, true) => Self::Unknown,
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Dedicated => "dedicated",
            Self::Shared => "shared",
            Self::Unknown => "unknown",
        }
    }
}

/// One memory heap of the device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeapSample {
    pub index: u32,
    /// Heap size as the device reports it.
    pub size_bytes: u64,
    /// `VK_MEMORY_HEAP_DEVICE_LOCAL_BIT`, as the device reports it. It says nothing about shared or dedicated memory
    /// on its own; [`Self::placement`] does.
    pub device_local: bool,
    pub placement: MemoryPlacement,
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
/// nor rebinds. Dropping it is the return. It is not `Clone`. A lease of a lost device does not keep the owner from
/// recovering: the lost device is set aside for it, and [`Self::handles`] says that it is lost.
///
/// What whoever uses the handles must keep to:
/// - They are asked for with [`Self::handles`] before each use, not kept: that call is what tells an idle holder of a
///   loss, a revocation, a dropped claim or a closed owner.
/// - Objects created from them are destroyed before the lease is dropped, and not while a submission that uses
///   them is [`crate::SubmissionState::Pending`]. [`Self::may_release`] says when both are allowed, also after a loss,
///   a close, or without the client's handle; [`Self::wait_release`] waits for it.
/// - Memory allocated with them is charged to the claim ([`crate::GpuClient::charge`]) before it is allocated.
/// - The lease carries no queue right. The queue is the owner's: work reaches it only through
///   [`crate::GpuClient::submit`]. Submitting to it, or waiting on it or on the whole device, through these
///   handles would race the owner and is not allowed.
///
/// A lease that is dropped too early takes nothing from under running work: the owner keeps the device until the work
/// has ended and the next call of the owner, a client or another lease finds that out.
pub struct DeviceLease {
    handles: DeviceHandles,
    claim: ClaimId,
    // Released in this order, so a claim that may settle never still keeps the device.
    device: Option<Arc<VulkanGeneration>>,
    hold: Option<ClaimHold>,
    owner: Arc<OwnerShared>,
}

impl std::fmt::Debug for DeviceLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceLease")
            .field("handles", &self.handles)
            .field("claim", &self.claim)
            .finish_non_exhaustive()
    }
}

impl Drop for DeviceLease {
    fn drop(&mut self) {
        drop(self.device.take());
        drop(self.hold.take());
        self.owner.reap();
    }
}

impl DeviceLease {
    pub(crate) fn new(
        handles: DeviceHandles,
        claim: ClaimId,
        device: Arc<VulkanGeneration>,
        hold: ClaimHold,
        owner: Arc<OwnerShared>,
    ) -> Self {
        Self {
            handles,
            claim,
            device: Some(device),
            hold: Some(hold),
            owner,
        }
    }

    /// The handles, when the claim may use its device now. Otherwise why not: the device is lost, the client was
    /// revoked, the claim is settled or was dropped, or the owner closed.
    pub fn handles(&self) -> Result<&DeviceHandles, AuthorizeRefusal> {
        match &self.hold {
            Some(hold) => hold.standing()?,
            None => return Err(AuthorizeRefusal::Unknown),
        }
        Ok(&self.handles)
    }

    /// Whether the objects made from the handles may be destroyed and the lease dropped now: no work of this client on
    /// its device runs any more. Asks the device, without waiting, as [`crate::GpuClient::observe`] does, and works as
    /// long as the lease exists: also when the device is lost, the owner closed or the client's handle is gone.
    pub fn may_release(&self) -> Result<(), ReleaseRefusal> {
        match &self.hold {
            Some(hold) => self.owner.may_release(self.handles.generation, hold),
            None => Ok(()),
        }
    }

    /// Wait at most `timeout`, and never longer than [`crate::MAX_WAIT`], until [`Self::may_release`] allows it, then
    /// answer as it does. Other calls are not blocked by the wait.
    pub fn wait_release(&self, timeout: Duration) -> Result<(), ReleaseRefusal> {
        match &self.hold {
            Some(hold) => self
                .owner
                .wait_release(self.handles.generation, hold, timeout),
            None => Ok(()),
        }
    }

    /// The handles for one purpose only: destroying the objects made from them before the lease is dropped. The
    /// device exists as long as the lease does, also when it is lost or its owner closed.
    pub fn handles_to_release(&self) -> &DeviceHandles {
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

/// What a client records into, inside [`crate::GpuClient::submit`] and [`crate::GpuClient::present`].
///
/// The command buffer is the owner's: primary, for the owner's queue family, begun for one submission. The client
/// only records commands into it, only during the recording call, and leaves it in the recording state; the owner
/// ends, submits and frees it. The owner also records into it on request: layout transitions, clears and readbacks of
/// the client's own images ([`Self::transition`], [`Self::clear`], [`Self::read_back`]).
///
/// A [`crate::Frame`] is recorded on only inside the [`crate::GpuClient::present`] of that frame, whose submission
/// waits for the frame's acquire and counts as the last use of its swapchain; anywhere else it is
/// [`RecordRefusal::FrameNotPresented`].
///
/// Layouts: the owner cannot see what the client's own commands do to an image, so the layout an image is in now is
/// the caller's statement. The owner refuses a layout the image cannot be in with its use, before or after a command
/// ([`RecordRefusal::LayoutInvalid`]: VUID-VkImageMemoryBarrier-oldLayout-01208/01211/01212/01213, and
/// `PRESENT_SRC_KHR` only for a frame); [`ImageLayout::Undefined`] is always a valid "now" and discards the contents.
pub struct Recording<'a> {
    work: &'a RecordedWork,
    generation: u64,
    /// The ledger and attachment of the recording client.
    ledger: u64,
    seat: u64,
    /// The frame a present records for: the only one this recording may use.
    frame: Option<FrameKey>,
}

/// Whether an image with the use of `target` may be in `layout`.
fn layout_allowed(target: &TargetOf, layout: ImageLayout) -> bool {
    let usage = target.usage;
    match layout {
        ImageLayout::Undefined | ImageLayout::General => true,
        ImageLayout::ColorAttachment => usage.color_attachment,
        ImageLayout::ShaderRead => usage.sampled,
        ImageLayout::TransferSrc => usage.transfer_src,
        ImageLayout::TransferDst => usage.transfer_dst,
        ImageLayout::Present => target.frame.is_some(),
    }
}

/// `from` and `to` are layouts the image may be in, and `to` is not `UNDEFINED` (VUID-VkImageMemoryBarrier-newLayout-01198).
fn layouts(target: &TargetOf, from: ImageLayout, to: ImageLayout) -> Result<(), RecordRefusal> {
    if to == ImageLayout::Undefined || !layout_allowed(target, from) || !layout_allowed(target, to)
    {
        return Err(RecordRefusal::LayoutInvalid);
    }
    Ok(())
}

impl std::fmt::Debug for Recording<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Recording")
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl<'a> Recording<'a> {
    pub(crate) fn new(
        work: &'a RecordedWork,
        generation: u64,
        ledger: u64,
        seat: u64,
        frame: Option<FrameKey>,
    ) -> Self {
        Self {
            work,
            generation,
            ledger,
            seat,
            frame,
        }
    }

    /// Raw `VkCommandBuffer`.
    pub fn command_buffer(&self) -> u64 {
        self.work.raw()
    }

    /// The device generation the command buffer belongs to.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The image, when it is this client's on this generation.
    fn mine(&self, target: TargetOf) -> Result<ImageCommand, RecordRefusal> {
        if target.ledger != self.ledger || target.seat != self.seat {
            return Err(RecordRefusal::Foreign);
        }
        if target.generation != self.generation {
            return Err(RecordRefusal::OtherGeneration);
        }
        if target.frame.is_some() && target.frame != self.frame {
            return Err(RecordRefusal::FrameNotPresented);
        }
        Ok(ImageCommand::new(
            target.image,
            target.extent.width,
            target.extent.height,
        ))
    }

    /// Move `image` from layout `from`, which it is in now, to `to`, after every earlier command of this buffer and
    /// before every later one.
    pub fn transition(
        &self,
        image: &impl ImageTarget,
        from: ImageLayout,
        to: ImageLayout,
    ) -> Result<(), RecordRefusal> {
        let target = image.target();
        let command = self.mine(target)?;
        layouts(&target, from, to)?;
        self.work
            .transition(command, from.vk_layout(), to.vk_layout());
        Ok(())
    }

    /// Fill `image`, which is in layout `from` now, with `color` (linear components), and leave it in `to`.
    pub fn clear(
        &self,
        image: &impl ImageTarget,
        from: ImageLayout,
        color: [f32; 4],
        to: ImageLayout,
    ) -> Result<(), RecordRefusal> {
        let target = image.target();
        let command = self.mine(target)?;
        if !target.usage.transfer_dst {
            return Err(RecordRefusal::UsageMissing);
        }
        layouts(&target, from, to)?;
        self.work
            .clear(command, from.vk_layout(), color, to.vk_layout());
        Ok(())
    }

    /// Copy every texel of `image`, which is in layout `from` now, row after row with no padding, to the start of
    /// `buffer`, make the copy visible to the host when the submission's fence has signalled, and leave the image in
    /// `to`. [`OwnerBuffer::read`] then returns what the device wrote.
    pub fn read_back(
        &self,
        image: &impl ImageTarget,
        from: ImageLayout,
        buffer: &OwnerBuffer,
        to: ImageLayout,
    ) -> Result<(), RecordRefusal> {
        let target = image.target();
        let command = self.mine(target)?;
        let origin = buffer.origin();
        if origin.ledger != self.ledger || origin.seat != self.seat {
            return Err(RecordRefusal::Foreign);
        }
        if origin.generation() != self.generation {
            return Err(RecordRefusal::OtherGeneration);
        }
        if !target.usage.transfer_src || !buffer.handles.usage.transfer_dst {
            return Err(RecordRefusal::UsageMissing);
        }
        layouts(&target, from, to)?;
        let needed = target
            .extent
            .texels()
            .saturating_mul(u64::from(target.format.texel_bytes()));
        if buffer.handles.size < needed {
            return Err(RecordRefusal::BufferTooSmall);
        }
        self.work.read_back(
            command,
            from.vk_layout(),
            buffer.handles.buffer,
            to.vk_layout(),
        );
        Ok(())
    }
}
