//! Tier-2 syscall-isolation sandbox adapter backed by gVisor (`runsc`) running
//! inside WSL2.
//!
//! Master Spec v02.187 §3.5.3 places a syscall-filtering / user-space-kernel
//! substrate at [`crate::sandbox::IsolationTier::Tier2Syscall`] — stronger than
//! a plain container namespace jail (Tier 1) but lighter than a
//! hardware-virtualized microVM (Tier 3). gVisor intercepts the guest's Linux
//! syscalls in a user-space sentry kernel (`runsc`) instead of letting them hit
//! the host kernel directly, so a compromised workload cannot reach the host
//! kernel's syscall surface.
//!
//! This adapter runs each `exec` as a fresh, throwaway gVisor sandbox via
//! `runsc do` (the lightweight "unmanaged" runsc mode that does not require an
//! OCI bundle). The proven host invocation is:
//!
//! ```text
//! wsl.exe -d <distro> -u <user> -e <runsc> \
//!     --network=none --platform=<platform> [--ignore-cgroups] \
//!     do sh -c "<command>"
//! ```
//!
//! verified end-to-end on the host WSL2 Ubuntu distribution with the
//! `systrap` platform (which needs no KVM). `--network=none` gives the sandbox
//! no network device at all (deny-all). gVisor's user-space sentry kernel runs
//! the command and `runsc do` propagates the command's real stdout, stderr and
//! exit code back through `wsl.exe`.

pub mod adapter;

pub use adapter::*;
