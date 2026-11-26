//! What the GPU tests share: the window crate's private desktop and its watches (`desktop`), the GPU owner's test
//! process and device rules (`gpu_env`) and its presentation cases (`present_cases`), all used from their own crates
//! as those crates wrote them, and the owner opened on this machine's real device.
//!
//! Each GPU test runs in a process of its own: the owner and the event loop are both one per process.

#![allow(dead_code, unused_imports, unused_macros)]

#[cfg(windows)]
#[path = "../../../window/tests/support/mod.rs"]
pub mod desktop;
#[path = "../../../../handshake_native/vulkan-renderer/tests/support/gpu_env.rs"]
pub mod gpu_env;
#[cfg(windows)]
#[path = "../../../../handshake_native/vulkan-renderer/tests/support/present_cases.rs"]
pub mod present_cases;

use std::sync::{Arc, Mutex, PoisonError};

use handshake_vulkan_renderer::{
    GpuOwner, OwnerDiagnostics, OwnerEvent, OwnerOptions, PolicyRevision, QuotaPolicy, Unavailable,
};

/// Room for every claim a case makes: 256 MiB each, FIFO presents in flight on two surfaces of one claim.
pub fn policy() -> QuotaPolicy {
    QuotaPolicy::Configured {
        revision: PolicyRevision(214),
        limit_bytes: 4 << 30,
        max_claims: 16,
        max_in_flight_per_claim: 64,
    }
}

/// Keeps every event of the owner, in order (the owner's own fixture has the same; `gpu_env` names it).
#[derive(Default)]
pub struct EventLog {
    events: Mutex<Vec<OwnerEvent>>,
}

impl EventLog {
    pub fn events(&self) -> Vec<OwnerEvent> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl OwnerDiagnostics for EventLog {
    fn record(&self, event: OwnerEvent) {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
}

/// Whether `reason` means this machine has no device a test may run on, rather than one that failed.
pub fn no_device(reason: Unavailable) -> bool {
    matches!(
        reason,
        Unavailable::LoaderMissing
            | Unavailable::InstanceRefused
            | Unavailable::ApiBelowBaseline
            | Unavailable::NoPhysicalDevice
            | Unavailable::SoftwareDeviceRefused
            | Unavailable::LayeredDriverRefused
            | Unavailable::NoSuitableDevice
            | Unavailable::FeatureMissing
            | Unavailable::PresentationUnsupported
    )
}

/// Fails the test when a device is required (`gpu_env::REQUIRE_GPU` is not 0); otherwise says why it skips.
pub fn skip_without_device(reason: Unavailable) {
    assert!(
        no_device(reason),
        "a device exists but did not open: {}",
        reason.key()
    );
    assert!(
        !gpu_env::is_required(std::env::var_os(gpu_env::REQUIRE_GPU).as_deref()),
        "no device to run on ({}), and {} is not 0",
        reason.key(),
        gpu_env::REQUIRE_GPU
    );
    println!(
        "HSK_GPU_SKIP reason={} number={}",
        reason.key(),
        reason.number()
    );
}

/// The owner on this machine's device, with presentation or without, or `None` after saying why there is none.
pub fn open_owner(presentation: bool) -> Option<(GpuOwner, Arc<EventLog>)> {
    let log = Arc::new(EventLog::default());
    let options = OwnerOptions { presentation };
    match GpuOwner::open_with(policy(), options, log.clone()) {
        Ok(owner) => Some((owner, log)),
        Err(reason) => {
            skip_without_device(reason);
            None
        }
    }
}

/// Whether the owner proves the end of a swapchain by its present fences.
pub fn has_present_fences(owner: &GpuOwner) -> bool {
    owner
        .identity()
        .extensions
        .contains(&"VK_EXT_swapchain_maintenance1")
}
