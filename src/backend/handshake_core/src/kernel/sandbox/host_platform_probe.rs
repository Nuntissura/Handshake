//! Host platform probe used by hard-isolation adapter stubs.
//!
//! Pure, no-shell-out. Returns the deterministic host kind so adapter
//! availability decisions are reproducible across replays.
//!
//! Belongs to the MT-020 HardIsolation Adapter Stub family: hard-isolation
//! stubs reach for this probe to decide whether their backing runtime is
//! UNSUPPORTED on the current host.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HostKind {
    Windows,
    Linux,
    MacOs,
    Other,
}

impl HostKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::MacOs => "macos",
            Self::Other => "other",
        }
    }
}

pub struct HostPlatformProbe;

impl HostPlatformProbe {
    pub fn detect() -> HostKind {
        if cfg!(target_os = "windows") {
            HostKind::Windows
        } else if cfg!(target_os = "linux") {
            HostKind::Linux
        } else if cfg!(target_os = "macos") {
            HostKind::MacOs
        } else {
            HostKind::Other
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_a_host_kind() {
        let k = HostPlatformProbe::detect();
        // No matter the build target, the probe must classify the host.
        assert!(matches!(
            k,
            HostKind::Windows | HostKind::Linux | HostKind::MacOs | HostKind::Other
        ));
        // Label is non-empty.
        assert!(!k.as_str().is_empty());
    }

    #[test]
    fn labels_are_lowercase_stable() {
        assert_eq!(HostKind::Windows.as_str(), "windows");
        assert_eq!(HostKind::Linux.as_str(), "linux");
        assert_eq!(HostKind::MacOs.as_str(), "macos");
        assert_eq!(HostKind::Other.as_str(), "other");
    }
}
