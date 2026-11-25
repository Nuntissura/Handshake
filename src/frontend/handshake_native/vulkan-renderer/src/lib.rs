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
//!    before each use and says when the device is lost. Memory a client holds is reported with
//!    [`GpuClient::charge`] and [`GpuClient::discharge`].
//! 5. [`GpuClient::submit`] lets the client record into the owner's command buffer and puts it on the owner's
//!    queue. It returns a [`Submission`] at once: attachment, operation, generation and fence value. The owner
//!    itself observes the end of it: [`GpuClient::observe`] without waiting, [`GpuClient::wait`] with a bound.
//! 6. When its work has ended the client settles the claim once: [`GpuClient::settle`], with an outcome that does not
//!    contradict what the owner observed. A claim closes when its leases are dropped and nothing is retained. Memory
//!    it still holds stays charged, under its lease, until [`GpuClient::release_retained`].
//! 7. After a device loss ([`GpuOwner::probe`], [`GpuOwner::poll`] and every call of a client that asks the device
//!    report it) the host calls [`GpuOwner::rebind_device`], each client drops its leases and moves its claims with
//!    [`GpuClient::rebind_claim`]. A client that does not keeps its own claims on the lost device and nobody else's.
//!    Charges are never refunded by a loss.
//! 8. A dropped [`Claim`] releases nothing by itself. The owner closes it once it observed the end of its work and no
//!    lease or reported memory is left. [`GpuOwner::poll`] is how the host lets it look, and an admission or a
//!    submission that finds no room looks as well. What cannot be reclaimed is reported as
//!    [`OwnerEvent::ClaimLeaked`].
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
//! | [`OwnerEvent::DeviceLeaked`] | a device did not say that its work has ended and nobody holds it | none in this process |
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
//! - Not here yet: an allocation service (clients allocate from the lease and report it), images or attachments
//!   owned by this unit, waits and signals for presentation, and the surface extensions a window needs.
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
mod process;
mod records;
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
    DeviceRebindRefusal, RebindRefusal, RecordAborted, SettleRefusal, SubmissionRefusal,
    SubmitRefusal, Unavailable,
};
pub use crate::owner::{GpuClient, GpuOwner, MAX_HELD_LOST_DEVICES, MAX_WAIT};
pub use crate::owner_core::{OwnerCore, ShutdownReport};
pub use crate::records::{
    ByteQuantity, ChargeClass, ClaimDisposition, ClaimInfo, ClaimRequest, DeviceBinding,
    DeviceFate, LedgerCounters, LedgerUsage, ModuleUsage, OperationKind, QuotaPolicy,
    ReleaseReceipt, SettleOutcome, SettleReceipt, Settlement, SubmissionObservation,
    SubmissionState, WorkObserved,
};
pub use crate::select::{DeviceClass, BASELINE_API_VERSION};

/// Self description for models and tooling.
pub const DESCRIPTOR: &str = r#"{"schema":"handshake.vulkan_renderer.descriptor@1","crate":"handshake-vulkan-renderer","slice":1,"owner":"WP-KERNEL-012 MT-203","binding":"ash =0.38.0, Vulkan 1.3 baseline, direct Vulkan; loader from the system directory only","locked_crates":["ash","cfg-if","handshake-vulkan-renderer","libloading","windows-link"],"state":"one private Vulkan generation (instance, strict device pick, device, queue, timeline fence), the owner-side claim ledger and the submit/observe/settle path; not mounted in any host; no window, surface, swapchain or drawing; no thread, timer or per-frame call","device_pick":"discrete, integrated or virtual GPU with a graphics queue; refuses CPU devices, SwiftShader, llvmpipe and the Direct3D-layered Dozen driver; no fallback","features_enabled":["dynamicRendering","timelineSemaphore"],"extensions_enabled_when_offered":["VK_EXT_memory_budget"],"capabilities":{"GpuOwner":"host only: policy, attach_client, revoke_client, usage, poll, probe, rebind_device, close","GpuClient":"one client, identity fixed at issue: admit, authorize, lease, charge, discharge, submit, observe, wait, settle, release_retained, rebind_claim, poll"},"ledger":"quota unavailable denies; bounded by bytes, claims and submissions per claim; a settled outcome never contradicts what the owner observed; a dropped claim is reclaimed only after its work was observed finished; one process only","submission":"attachment (module, client), operation, generation, fence value; the owner observes completion on its own timeline semaphore, a client never reports it; no wait for the device holds a lock that a client call needs","device_loss":"only VK_ERROR_DEVICE_LOST; rebind_device recreates the same adapter with generation + 1; a lost device that clients hold is set aside and destroyed with its last holder; charges are not refunded","diagnostics":"OwnerDiagnostics port: codes, counters and opaque module, client, claim and operation numbers, event-driven; host wiring to internal_diagnostics not included","unavailable":["vulkan_loader_missing","vulkan_instance_refused","api_below_baseline","no_physical_device","software_device_refused","layered_driver_refused","no_suitable_device","required_device_missing","feature_missing","device_creation_refused","physical_device_lost","out_of_memory","owner_already_open","vk_error"],"not_here":["kernel issuer and grants","installation-wide persisted quota ledger","GpuClientPort adapter (asset port lives in a wgpu crate)","memory allocation service and owner images","presentation waits and signals, surface extensions","windows, swapchains, UI pass, atlas, text"],"platform":"x86_64-pc-windows-msvc first"}"#;
