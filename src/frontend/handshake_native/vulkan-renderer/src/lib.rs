//! Handshake's central GPU owner (WP-KERNEL-012 MT-203).
//!
//! # What this is
//!
//! The single place in Handshake that creates a GPU instance, device and queue, and the ledger every
//! module's GPU work is admitted, accounted and settled through. It is direct Vulkan through `ash`: this
//! unit's closure contains no wgpu, egui, eframe or winit (Master Spec module 15 GUI-DESKTOP-003,
//! Operator decisions D7 and D1). It opens no window and draws nothing, assumes no toolkit host and no call per
//! frame, and runs no thread or timer of its own.
//!
//! # Who may do what
//!
//! Two capabilities, never one handle for both. [`GpuOwner`] stays with the host: quota policy, attaching and
//! revoking clients, recovery, totals and shutdown. [`GpuClient`] is what the host hands to one module client: its
//! identity is fixed when it is issued and it reaches only its own claims. A module cannot raise the quota, act for
//! another client or close the owner with it.
//!
//! # How it is used
//!
//! 1. The host opens the owner once: [`GpuOwner::open`]. The result is a real hardware Vulkan device or
//!    a typed [`Unavailable`]. There is no fallback to a CPU device, Direct3D or OpenGL.
//! 2. The host issues a handle per module client: [`GpuOwner::attach_client`].
//! 3. The client asks for admission with a [`ClaimRequest`] (operation and bytes to reserve):
//!    [`GpuClient::admit`] returns a [`Claim`] or a typed [`AdmissionDenial`]. Without a configured
//!    [`QuotaPolicy`] every admission is denied.
//! 4. [`GpuClient::lease`] hands over the device for pipelines and other objects; [`DeviceLease::handles`] is asked
//!    before each use and says when the device is lost. [`DeviceLease::may_release`] says when those objects may be
//!    destroyed and the lease returned, also after a loss or a close. Memory a client holds is reported with
//!    [`GpuClient::charge`] and [`GpuClient::discharge`].
//! 5. [`GpuClient::submit`] lets the client record into the owner's command buffer and puts it on the owner's
//!    queue. It returns a [`Submission`] at once: attachment, operation, generation and its value on that queue. The
//!    owner itself observes the end of it: [`GpuClient::observe`] without waiting, [`GpuClient::wait`] with a bound.
//! 6. When its work has ended the client settles the claim once: [`GpuClient::settle`], with an outcome that does not
//!    contradict what the owner observed. A claim closes when its leases are dropped and nothing is retained. Memory
//!    it still holds stays charged, under its lease, until [`GpuClient::release_retained`].
//! 7. After a device loss ([`GpuOwner::probe`], [`GpuOwner::poll`] and every call of a client that asks the device
//!    report it) the host calls [`GpuOwner::rebind_device`], each client drops its leases and moves its claims with
//!    [`GpuClient::rebind_claim`]. A client that does not keeps its own claims on the lost device and nobody else's.
//!    Charges are never refunded by a loss.
//! 8. A [`Claim`] outlives its client's handle: its holder still reports freed memory and settles it
//!    ([`Claim::discharge`], [`Claim::settle`], [`Claim::release_retained`]). A dropped claim releases nothing by
//!    itself. The owner closes it once it observed the end of its work and no lease or reported memory is left.
//!    [`GpuOwner::poll`] is how the host lets it look, and an admission or a submission that finds no room looks as
//!    well. What cannot be reclaimed is reported as [`OwnerEvent::ClaimLeaked`].
//! 9. [`GpuOwner::close`] is bounded. A device that clients hold or that still runs work is kept, observable, and
//!    destroyed by the first call after both ended. A host that has to end before that ends its process: nothing here
//!    waits for it. A submission stuck inside the driver holds only the queue's lane, so it keeps neither a close nor
//!    any observation waiting.
//! 10. Owner resources: [`GpuClient::create_image`] and [`GpuClient::create_buffer`] make render attachments and
//!     buffers the owner creates, places (`gpu-alloc`), charges to the claim inside its reservation and destroys
//!     itself, once nothing that may use them runs after the client dropped them. [`Recording::clear`],
//!     [`Recording::transition`] and [`Recording::read_back`] record owner commands on them, and
//!     [`OwnerBuffer::read`] hands what the device wrote to the host: the proof path of headless visual checks.
//!     [`GpuOwner::memory_use`] reports the owner's memory objects, measured.
//! 11. Presentation, when the host opened the owner with [`OwnerOptions::presentation`]:
//!     [`GpuClient::create_surface`] on a window a [`SurfaceTarget`] names and keeps alive, [`OwnerSurface::configure`]
//!     on create, resize, minimise and restore, [`OwnerSurface::acquire`] without waiting, [`GpuClient::present`]
//!     with no wait on the frame path, and [`OwnerSurface::probe`] for a watcher that must see a loop that never
//!     presents. Old swapchains are destroyed once their work and presents are known to have ended.
//!
//! # Failure modes and recovery
//!
//! | Outcome | Meaning | Recovery |
//! | --- | --- | --- |
//! | [`Unavailable`] | no acceptable Vulkan device | fix the driver or hardware; the host shows the typed reason |
//! | [`AdmissionDenial::HostQuotaUnavailable`] | no quota policy configured | the host supplies its real policy with [`GpuOwner::set_policy`] |
//! | [`AdmissionDenial::BudgetExceeded`], [`AdmissionDenial::ClaimLimit`] | the claim does not fit the policy | settle or release other claims, or ask for less |
//! | [`AdmissionDenial::ClientDetached`] | the host revoked this client | the host attaches it anew: [`GpuOwner::attach_client`] |
//! | [`AuthorizeRefusal::InFlightLimit`] | too much unfinished work under one claim | observe it, then submit again |
//! | [`AuthorizeRefusal::Abandoned`] | the claim of the lease was dropped | destroy the objects and drop the lease |
//! | [`ReleaseRefusal::InFlight`] | work that may use the objects of the lease runs | [`DeviceLease::wait_release`], then destroy them and drop the lease |
//! | [`DeviceFault::Lost`] | `VK_ERROR_DEVICE_LOST` | [`GpuOwner::rebind_device`], drop leases, then [`GpuClient::rebind_claim`] per claim |
//! | [`DeviceRebindRefusal::DeviceHeld`] | too many lost devices are still held by clients | one of them is returned, then rebind again |
//! | [`DeviceRebindRefusal::NotQuiescent`] | the lost device does not say that its work has ended | rebind again later |
//! | [`RebindRefusal::MemoryHeld`] | the lost device still exists and the claim reports memory on it | free and discharge it, then rebind |
//! | [`SettleRefusal::InFlight`] | work may still run | observe or wait for its submissions, then settle again |
//! | [`SettleRefusal::OutcomeContradicted`] | the outcome is not what the owner observed | settle with the outcome that happened |
//! | [`SettleRefusal::LeaseOutstanding`] | objects on the device may exist | destroy them and drop the lease, then settle or release again |
//! | [`SettleRefusal::RetainedBelowCharged`] | the report contradicts the claim's own charges | discharge what was freed, then settle again |
//! | [`SettleRefusal::DoubleRelease`] | claim already closed | none needed: nothing was released twice |
//! | [`OwnerEvent::ClaimLeaked`] | a dropped claim reported memory that is not known to be freed | it is reclaimed when its device is destroyed |
//! | [`DeviceFate::Held`], [`DeviceFate::Unconfirmed`] | a closed device is held or still runs work | holders return it; [`GpuOwner::poll`] then destroys it |
//! | [`DeviceFate::Creating`] | a rebind was creating a device during the close | it ends with [`OwnerEvent::DeviceRebindRefused`] |
//! | [`OwnerEvent::DeviceLeaked`] | the last handle went before a device said that its work has ended | none in this process |
//! | [`ResourceRefusal::Charge`] | the resource does not fit the claim's reservation | ask for less, or admit a larger claim |
//! | [`ResourceRefusal::OverBudget`] | the driver's budget for a heap would be exceeded | release resources, or wait for other processes |
//! | [`SettleRefusal::ResourcesOutstanding`], [`RebindRefusal::ResourcesHeld`] | owner resources of the claim still exist | drop them, [`GpuClient::poll`], then again |
//! | [`AccessRefusal::InFlight`] | work of the client may still use the buffer | wait for it, then read again |
//! | [`AccessRefusal::OutOfRange`] | the range does not lie inside the buffer's own size | read or write within [`BufferHandles::size`] |
//! | [`RecordRefusal::FrameNotPresented`] | a frame was used outside the recording of its own present | record on it inside [`GpuClient::present`] |
//! | [`RecordRefusal::LayoutInvalid`] | a layout the image cannot be in with its use, or none to leave it in | name the layout it is in, or [`ImageLayout::Undefined`] |
//! | [`PresentRefusal::NotReady`] | no image is ready now | acquire again on the next frame |
//! | [`PresentRefusal::OutOfDate`], [`PresentResult::OutOfDate`] | the swapchain no longer matches its window | [`OwnerSurface::configure`] |
//! | [`PresentRefusal::SurfaceLost`], [`PresentRefusal::WindowInUse`] | the surface is gone, or the window has another swapchain | make a new surface, or configure later |
//! | [`Unavailable::PresentationUnsupported`] | presentation was asked for and the device cannot present here | open without presentation, or fix the driver |
//!
//! # Diagnostics
//!
//! The owner reports through [`OwnerDiagnostics`]: closed codes, counters and the opaque numbers of whom they concern
//! ([`Subject`]), emitted by the call or the dropped handle that caused them. Forwarding those events into
//! Handshake's `internal_diagnostics` ring (including its `VulkanDeviceLost` code) is host wiring and not part of
//! this unit.
//!
//! # Boundaries
//!
//! - The canonical client vocabulary is the renderer asset's `GpuClientPort` (assets repository,
//!   `hsk-studio-render-gpu::client`). This unit is the owner side behind that port; the port adapter is
//!   added when the asset publishes the port in a crate without a wgpu dependency.
//! - The asset's Vulkan UI pass does not take a host device today: it is built from the asset's own owner, which
//!   creates the device itself, and it waits on a fence inside every pass. Device injection
//!   ([`DeviceHandles`]), recording into a host command buffer ([`Recording`]) and asynchronous settlement
//!   ([`Submission`]) are what it needs from here; taking them is the asset's change, not this unit's.
//! - This crate creates no window and depends on no window crate: the host's window crate supplies a
//!   [`SurfaceTarget`] (its documentation says how `handshake_shell_window`'s token becomes one).
//! - Authorization (kernel issuer, grants, capabilities) is a separate gate and is not performed here.
//! - The ledger accounts for one process. The installation-wide persisted quota ledger is Core's.
//! - UI passes, atlases, display lists, pipelines and text belong to the renderer asset, never here.
#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

// The unit tests use the same fixtures as the integration tests, which name this crate.
#[cfg(test)]
extern crate self as handshake_vulkan_renderer;

mod device;
mod diag;
mod ids;
mod ledger;
mod outcome;
mod owner;
mod owner_core;
mod present;
mod process;
mod records;
mod refusal;
mod resource;
pub mod select;
mod vulkan;

pub use crate::device::{
    DeviceHandles, DeviceIdentity, DeviceLease, HeapSample, MemoryBudgetSource, MemoryPlacement,
    MemorySnapshot, Recording,
};
pub use crate::diag::{NullDiagnostics, OwnerDiagnostics, OwnerEvent, OwnerPhase, Subject};
pub use crate::ids::{
    AuthorizationProvenance, ClaimId, ClientId, DeviceKey, ModuleId, OperationId, PolicyRevision,
};
pub use crate::ledger::{Claim, ClaimHold, ClaimLedger, LedgerClient, Submission};
pub use crate::outcome::{
    AdmissionDenial, AttachRefusal, AuthorizeRefusal, ChargeRefusal, DeviceFault, DeviceHealth,
    DeviceRebindRefusal, RebindRefusal, RecordAborted, ReleaseRefusal, SettleRefusal,
    SubmissionRefusal, SubmitRefusal, Unavailable,
};
pub use crate::owner::{GpuClient, GpuOwner, MAX_HELD_LOST_DEVICES, MAX_WAIT};
pub use crate::owner_core::{OwnerCore, ShutdownReport};
pub use crate::present::{
    Frame, NativeWindow, OwnerOptions, OwnerSurface, PresentMode, PresentProbe, PresentProgress,
    PresentResult, Presented, SurfaceTarget, SwapchainInfo, SwapchainRequest, SwapchainStatus,
};
pub use crate::records::{
    ByteQuantity, ChargeClass, ClaimDisposition, ClaimInfo, ClaimRequest, DeviceBinding,
    DeviceFate, LedgerCounters, LedgerUsage, ModuleUsage, OperationKind, QuotaPolicy,
    ReleaseReceipt, SettleOutcome, SettleReceipt, Settlement, SubmissionObservation,
    SubmissionState, WorkObserved,
};
pub use crate::refusal::{AccessRefusal, PresentRefusal, RecordRefusal, ResourceRefusal};
pub use crate::resource::{
    AttachmentFormat, BufferHandles, BufferMemory, BufferRequest, BufferUsage, Extent, ImageLayout,
    ImageRequest, ImageTarget, ImageUsage, OwnerBuffer, OwnerHeapUse, OwnerImage, OwnerMemoryUse,
    RenderAttachment,
};
pub use crate::select::{DeviceClass, BASELINE_API_VERSION};

/// Self description for models and tooling.
pub const DESCRIPTOR: &str = r#"{"schema":"handshake.vulkan_renderer.descriptor@1","crate":"handshake-vulkan-renderer","slice":2,"owner":"WP-KERNEL-012 MT-203","binding":"ash =0.38.0, Vulkan 1.3 baseline, direct Vulkan; loader from the system directory only","locked_crates":["ash","bitflags","cfg-if","gpu-alloc","gpu-alloc-types","handshake-vulkan-renderer","libloading","windows-link"],"state":"one private Vulkan generation (instance, strict device pick, device, queue, a fence per submission), the owner-side claim ledger and the submit/observe/settle path, owner images and buffers with measured memory, and presentation on host-named windows when asked for; not mounted in any host; creates no window and draws nothing of its own; no thread, timer or per-frame call","device_pick":"discrete, integrated or virtual GPU with a graphics queue; refuses CPU devices, SwiftShader, llvmpipe and the Direct3D-layered Dozen driver; no fallback","features_enabled":["dynamicRendering","timelineSemaphore"],"features_enabled_when_offered":["swapchainMaintenance1"],"extensions_enabled_when_offered":["VK_EXT_memory_budget"],"extensions_with_presentation":{"instance":["VK_KHR_surface","VK_KHR_win32_surface"],"instance_when_offered":["VK_KHR_get_surface_capabilities2","VK_EXT_surface_maintenance1"],"device":["VK_KHR_swapchain"],"device_when_offered":["VK_EXT_swapchain_maintenance1"]},"capabilities":{"GpuOwner":"host only: policy, attach_client, revoke_client, usage, poll, probe, rebind_device, close","GpuClient":"one client, identity fixed at issue: admit, authorize, lease, charge, discharge, submit, observe, wait, settle, release_retained, rebind_claim, poll, create_image, create_buffer, create_surface, present","OwnerImage":"attachment (raw image, view, format, extent, usage); dropped = released, destroyed after its work","OwnerBuffer":"handles, read, write (refused while work of its client runs)","OwnerSurface":"configure, suspend, acquire (timeout 0), swapchain, probe","Recording":"command_buffer, transition, clear, read_back","Claim":"its holder, also without the client's handle: discharge, settle, release_retained","DeviceLease":"handles, handles_to_release, may_release, wait_release"},"ledger":"quota unavailable denies; a denial does not tell what others hold; bounded by bytes, claims and submissions per claim; a settled outcome never contradicts what the owner observed; a dropped claim is reclaimed only after its work was observed finished; one process only","submission":"attachment (module, client), operation, generation, value on the queue; the owner observes completion on its own fences, a client never reports it","waits":"every wait is bounded by the owner's own deadline (5 s at most) and looks at fence states between sleeps; no vkWaitSemaphores, vkWaitForFences, vkQueueWaitIdle or vkDeviceWaitIdle; no wait holds a lock that a client call needs","device_loss":"VK_ERROR_DEVICE_LOST or a signalled sentinel fence; a lost device completes nothing any more; rebind_device recreates the same adapter with generation + 1; a lost device that clients hold is set aside and destroyed with its last holder; charges are not refunded","close":"bounded; a device that is held or still runs work is kept, observable, and destroyed by the first call after both ended","memory":"gpu-alloc 0.6.2 buddy and dedicated blocks over the owner's own vkAllocateMemory/vkFreeMemory; memory types by the resource's memoryTypeBits and use; a new memory object past VK_EXT_memory_budget's heap budget is refused (over_driver_budget), never placed in another memory type instead, without the extension the budget is unknown; host reads and writes are bounded by the buffer's own size, not its block; the allocator lock is taken only to place and return blocks, looks never wait for it; reserve on admission, charge on allocation inside the reservation, release once after the owner destroyed the resource","presentation":"surface from a host SurfaceTarget (Win32 hwnd, hinstance), swapchain recreated on resize, minimise and restore with oldSwapchain; acquire with timeout 0, present without waits; an old swapchain is destroyed after its frames, submissions and acquires ended and its presents are proven ended by its own surface (its present fences with VK_EXT_swapchain_maintenance1, otherwise a present of a later swapchain of that surface proven by the re-acquire of its image; never another window's); a presented last swapchain of a released surface without the extension is kept until the device ends; one lock per surface, a present holds the queue's lane and its own surface's lock only; a frame is recorded on only in its own present; swapchain image bytes reported as an estimate in memory_use, never charged; window targets dropped with no owner lock held; PresentProbe for watchers","diagnostics":"OwnerDiagnostics port: codes, counters and opaque module, client, claim and operation numbers, event-driven, never delivered while a lock of the owner is held; host wiring to internal_diagnostics not included","unavailable":["vulkan_loader_missing","vulkan_instance_refused","api_below_baseline","no_physical_device","software_device_refused","layered_driver_refused","no_suitable_device","required_device_missing","feature_missing","device_creation_refused","physical_device_lost","out_of_memory","owner_already_open","vk_error","presentation_unsupported"],"not_here":["kernel issuer and grants","installation-wide persisted quota ledger","GpuClientPort adapter (asset port lives in a wgpu crate)","window creation, UI pass, atlas, text","depth attachments, mip levels, multisampling"],"platform":"x86_64-pc-windows-msvc first"}"#;
