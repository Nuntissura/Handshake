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

/// How many proofs a shutdown begins: none with present fences, one per window with a surface without them.
pub fn expected_shutdown_proofs(present_fences: bool, windows_with_surface: u64) -> u64 {
    if present_fences {
        0
    } else {
        windows_with_surface
    }
}

/// MT-214 E9-F1, on a GPU without `VK_EXT_swapchain_maintenance1`: a shutdown proved the presents of every window that
/// still had a surface (`windows_with_surface`) ended before it released the surface, none ended unproven, and the
/// owner's own single-shot report at its close held nothing: no claim open, nothing in flight, no lease. The settle wait
/// before the close was not needed (one look, in a fraction of its window). The verdict is the owner's report, not these
/// counters, which are the presenter's own record of it.
///
/// With `VK_EXT_swapchain_maintenance1` (`report.present_fences`) a shutdown begins no proof: the owner ends the
/// surfaces by their own present fences, so `shutdown_proofs_begun` is 0 whatever the window count.
pub fn assert_shutdown_proved(report: &handshake_shell_surface::PresentReport, windows_with_surface: u64) {
    assert_eq!(report.closes_unproven, 0, "{report:?}");
    assert_eq!(
        report.shutdown_proofs_begun,
        expected_shutdown_proofs(report.present_fences, windows_with_surface),
        "{report:?}"
    );
    assert!(!report.shutdown_prove_gave_up, "{report:?}");
    assert!(!report.settle_wait_gave_up, "{report:?}");
    assert!(report.settle_wait_looks <= 2, "{report:?}");
    assert!(
        std::time::Duration::from_micros(report.settle_wait_micros) < std::time::Duration::from_millis(500),
        "{report:?}"
    );
    let owner = report.shutdown.expect("the owner's report at its close");
    assert_eq!(owner.open_claims, 0, "{owner:?}");
    assert_eq!(owner.in_flight, 0, "{owner:?}");
    assert_eq!(owner.leases, 0, "{owner:?}");
    assert!(owner.is_settled(), "{owner:?}");
    assert_eq!(report.claims_abandoned, 0, "{report:?}");
}
