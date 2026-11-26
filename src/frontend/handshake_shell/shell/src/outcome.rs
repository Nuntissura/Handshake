//! How a program ended, as a typed kind with a stable name and exit code (MT-214 AC214-9, AC214-10, AC214-11), and the
//! decisions that make a shutdown settled and decide what ends it. They are plain functions of numbers, so tests check
//! them without a device.

use crate::shutdown::WatcherAck;

/// How the shell program ended. Each kind has one process exit code; no two share one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExitKind {
    /// Closed, drained, every claim settled, the watcher acknowledged or none connected.
    Clean,
    /// A test or example binary run without its explicit flag: nothing was opened.
    Usage,
    /// Not Windows: the shell runs on Windows only so far.
    Unsupported,
    /// No device that renders and presents here: the owner's typed reason is the detail.
    DeviceUnavailable,
    /// The root window was refused or failed.
    NoWindow,
    /// The window crate's loop did not run, or failed.
    LoopFailed,
    /// The product has no `device.gpu.shell` under the kernel issuer; the device was never opened.
    NoDeviceAuthority,
    /// The loop ended but the owner reported claims, work or a device still held: nothing was acknowledged.
    ShutdownUnsettled,
    /// The shutdown was settled and the watcher did not acknowledge it.
    WatcherNotAcknowledged,
    /// The heartbeat could not be opened or started: a watcher would read the shell as frozen.
    HeartbeatRefused,
}

impl ExitKind {
    /// Every kind, so a test sees them all.
    pub const ALL: [Self; 10] = [
        Self::Clean,
        Self::Usage,
        Self::Unsupported,
        Self::DeviceUnavailable,
        Self::NoWindow,
        Self::LoopFailed,
        Self::NoDeviceAuthority,
        Self::ShutdownUnsettled,
        Self::WatcherNotAcknowledged,
        Self::HeartbeatRefused,
    ];

    /// The process exit code.
    pub const fn code(self) -> u8 {
        match self {
            Self::Clean => 0,
            Self::Usage => 2,
            Self::Unsupported => 3,
            Self::DeviceUnavailable => 4,
            Self::NoWindow => 5,
            Self::LoopFailed => 6,
            Self::NoDeviceAuthority => 7,
            Self::ShutdownUnsettled => 8,
            Self::WatcherNotAcknowledged => 9,
            Self::HeartbeatRefused => 10,
        }
    }

    /// A stable name, for diagnostics.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Usage => "usage",
            Self::Unsupported => "unsupported",
            Self::DeviceUnavailable => "device_unavailable",
            Self::NoWindow => "no_window",
            Self::LoopFailed => "loop_failed",
            Self::NoDeviceAuthority => "no_device_authority",
            Self::ShutdownUnsettled => "shutdown_unsettled",
            Self::WatcherNotAcknowledged => "watcher_not_acknowledged",
            Self::HeartbeatRefused => "heartbeat_refused",
        }
    }
}

/// Whether a shutdown is settled: the owner reported every claim settled, nothing retained or in flight and the device
/// destroyed (`owner_settled`, `None` when it reported nothing), no window claim was let go unsettled, and every window
/// the loop created is destroyed. Anything less is not settled, and is never acknowledged (the red team's "shutdown
/// reports settled work that is still in flight").
pub const fn shutdown_settled(
    owner_settled: Option<bool>,
    claims_abandoned: u64,
    windows_created: u64,
    windows_destroyed: u64,
) -> bool {
    matches!(owner_settled, Some(true)) && claims_abandoned == 0 && windows_created == windows_destroyed
}

/// What the watcher may be told: only a settled shutdown is acknowledged.
pub const fn may_acknowledge(settled: bool) -> bool {
    settled
}

/// How the program ends. An unsettled shutdown outranks everything: nothing is reported clean over work still held.
/// Then a failure of the run itself, then a watcher that did not acknowledge.
pub const fn conclude(settled: bool, failure: Option<ExitKind>, ack: Option<WatcherAck>) -> ExitKind {
    if !settled {
        return ExitKind::ShutdownUnsettled;
    }
    if let Some(failure) = failure {
        return failure;
    }
    match ack {
        Some(WatcherAck::NotAcknowledged) => ExitKind::WatcherNotAcknowledged,
        _ => ExitKind::Clean,
    }
}
