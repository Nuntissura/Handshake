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
//! # How a client uses it
//!
//! 1. The host opens the owner once: [`GpuOwner::open`]. The result is a real hardware Vulkan device or
//!    a typed [`Unavailable`]. There is no fallback to a CPU device, Direct3D or OpenGL.
//! 2. A client asks for admission with a [`ClaimRequest`] (module, client, operation, bytes to reserve):
//!    [`GpuOwner::admit`] returns a [`Claim`] or a typed [`AdmissionDenial`]. Without a configured
//!    [`QuotaPolicy`] every admission is denied.
//! 3. [`GpuOwner::lease`] hands over the device for pipelines and other objects. Memory a client holds is
//!    reported with [`GpuOwner::charge`] and [`GpuOwner::discharge`].
//! 4. [`GpuOwner::submit`] lets the client record into the owner's command buffer and puts it on the owner's
//!    queue. It returns a [`Submission`] at once: attachment, operation, generation and fence value. The owner
//!    itself observes the end of it: [`GpuOwner::observe`] without waiting, [`GpuOwner::wait`] with a bound.
//! 5. When its work has ended the client settles the claim once: [`GpuOwner::settle`]. A claim closes when its
//!    leases are dropped and nothing is retained. Memory it still holds stays charged, under its lease, until
//!    [`GpuOwner::release_retained`].
//! 6. After a device loss ([`GpuOwner::probe`], [`GpuOwner::observe`] or [`GpuOwner::submit`] report it) clients
//!    drop their leases, the host calls [`GpuOwner::rebind_device`] and each client moves its claim with
//!    [`GpuOwner::rebind_claim`]. Charges are never refunded by a loss.
//!
//! # Failure modes and recovery
//!
//! | Outcome | Meaning | Recovery |
//! | --- | --- | --- |
//! | [`Unavailable`] | no acceptable Vulkan device | fix the driver or hardware; the host shows the typed reason |
//! | [`AdmissionDenial::HostQuotaUnavailable`] | no quota policy configured | the host supplies its real policy with [`GpuOwner::set_policy`] |
//! | [`AdmissionDenial::BudgetExceeded`], [`AdmissionDenial::ClaimLimit`] | the claim does not fit the policy | settle or release other claims, or ask for less |
//! | [`AuthorizeRefusal::InFlightLimit`] | too much unfinished work under one claim | observe it, then submit again |
//! | [`DeviceFault::Lost`] | `VK_ERROR_DEVICE_LOST` | drop leases, [`GpuOwner::rebind_device`], then [`GpuOwner::rebind_claim`] per claim |
//! | [`DeviceRebindRefusal::DeviceHeld`] | a client still holds the lost device | it drops its lease, then rebind again |
//! | [`SettleRefusal::InFlight`] | work may still run | observe or wait for its submissions, then settle again |
//! | [`SettleRefusal::LeaseOutstanding`] | objects on the device may exist | destroy them and drop the lease, then settle or release again |
//! | [`SettleRefusal::RetainedBelowCharged`] | the report contradicts the claim's own charges | discharge what was freed, then settle again |
//! | [`SettleRefusal::DoubleRelease`] | claim already closed | none needed: nothing was released twice |
//!
//! # Diagnostics
//!
//! The owner reports through [`OwnerDiagnostics`]: closed codes and counters only, emitted by the call that caused
//! them. Forwarding those events into Handshake's `internal_diagnostics` ring (including its `VulkanDeviceLost`
//! code) is host wiring and not part of this unit.
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
    DeviceHandles, DeviceIdentity, DeviceLease, HeapSample, MemoryBudgetSource, MemorySnapshot,
    Recording,
};
pub use crate::diag::{NullDiagnostics, OwnerDiagnostics, OwnerEvent, OwnerPhase};
pub use crate::ids::{
    AuthorizationProvenance, ClaimId, ClientId, DeviceKey, ModuleId, OperationId, PolicyRevision,
};
pub use crate::ledger::{Claim, ClaimHold, ClaimLedger, Submission};
pub use crate::outcome::{
    AdmissionDenial, AuthorizeRefusal, ChargeRefusal, DeviceFault, DeviceRebindRefusal,
    RebindRefusal, RecordAborted, SettleRefusal, SubmissionRefusal, SubmitRefusal, Unavailable,
};
pub use crate::owner::{GpuOwner, MAX_WAIT};
pub use crate::owner_core::{OwnerCore, ShutdownReport};
pub use crate::records::{
    ByteQuantity, ChargeClass, ClaimDisposition, ClaimInfo, ClaimRequest, DeviceBinding,
    LedgerCounters, LedgerUsage, ModuleUsage, OperationKind, QuotaPolicy, ReleaseReceipt,
    SettleOutcome, SettleReceipt, Settlement, SubmissionObservation, SubmissionState,
};
pub use crate::select::{DeviceClass, BASELINE_API_VERSION};

/// Self description for models and tooling.
pub const DESCRIPTOR: &str = r#"{"schema":"handshake.vulkan_renderer.descriptor@1","crate":"handshake-vulkan-renderer","slice":1,"owner":"WP-KERNEL-012 MT-203","binding":"ash =0.38.0, Vulkan 1.3 baseline, direct Vulkan; closure excludes wgpu, egui, eframe, winit","state":"one private Vulkan generation (instance, strict device pick, device, queue, timeline fence), the owner-side claim ledger and the submit/observe/settle path; not mounted in any host; no window, surface, swapchain or drawing; no thread, timer or per-frame call","device_pick":"discrete, integrated or virtual GPU with a graphics queue; refuses CPU devices, SwiftShader, llvmpipe and the Direct3D-layered Dozen driver; no fallback","features_enabled":["dynamicRendering","timelineSemaphore"],"extensions_enabled_when_offered":["VK_EXT_memory_budget"],"ledger":"admit, authorize, lease, charge, discharge, submit, observe, wait, settle, release_retained, rebind_claim, revoke_client; quota unavailable denies; bounded by bytes, claims and submissions per claim; one process only","submission":"attachment (module, client), operation, generation, fence value; the owner observes completion on its own timeline semaphore, a client never reports it","device_loss":"only VK_ERROR_DEVICE_LOST; rebind_device recreates the same adapter with generation + 1 after every lease is returned; charges are not refunded","diagnostics":"OwnerDiagnostics port, codes and counters only, event-driven; host wiring to internal_diagnostics not included","unavailable":["vulkan_loader_missing","vulkan_instance_refused","api_below_baseline","no_physical_device","software_device_refused","layered_driver_refused","no_suitable_device","required_device_missing","feature_missing","device_creation_refused","physical_device_lost","out_of_memory","owner_already_open","vk_error"],"not_here":["kernel issuer and grants","installation-wide persisted quota ledger","GpuClientPort adapter (asset port lives in a wgpu crate)","memory allocation service and owner images","presentation waits and signals, surface extensions","windows, swapchains, UI pass, atlas, text"],"platform":"x86_64-pc-windows-msvc first"}"#;
