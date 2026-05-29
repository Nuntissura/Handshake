//! Tier-3 microVM sandbox adapter backed by Cloud Hypervisor running inside
//! WSL2.
//!
//! Master Spec v02.187 §3.5.3 places hardware-virtualized microVMs at the
//! strongest isolation tier ([`crate::sandbox::IsolationTier::Tier3Microvm`]).
//! This adapter boots a fresh, fully-isolated Cloud Hypervisor microVM for
//! every `exec` call (the ephemeral-per-command model), which is the strongest
//! posture for running untrusted agent-authored code: there is no shared guest
//! state across commands and the VM is torn down on power-off.
//!
//! The boot recipe (kernel + initramfs + a base64-encoded command passed on the
//! kernel cmdline via `hsk.cmd=`) is verified end-to-end on the host WSL2
//! Ubuntu distribution. The initramfs `/init` decodes the command, runs it,
//! frames its combined stdout/stderr between `---HSK-BEGIN---` and
//! `---HSK-END rc=<code>---` markers on the serial console, then powers the VM
//! off.

pub mod adapter;

pub use adapter::*;
