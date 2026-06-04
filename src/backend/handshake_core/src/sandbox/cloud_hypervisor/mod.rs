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
//! Ubuntu distribution. In the ephemeral-per-exec model the initramfs `/init`
//! decodes the command, runs it, frames its combined stdout/stderr between
//! `---HSK-BEGIN---` and `---HSK-END rc=<code>---` markers on the serial
//! console, then powers that per-exec VM off.
//!
//! # Persistent-VM + snapshot/restore model
//!
//! Selecting [`SANDBOX_MODE_PERSISTENT`] via the [`SANDBOX_MODE_METADATA_KEY`]
//! spec metadata switches `spawn` to a long-lived microVM driven over a Cloud
//! Hypervisor API socket (it loops, never powers off) plus a serial-socket
//! command agent. Such handles support [`SandboxAdapter::exec`] over that guest
//! agent, [`SandboxAdapter::snapshot`] (`ch-remote pause` + `snapshot`), and
//! [`SandboxAdapter::restore`] (`--restore … resume=true`), which resume a
//! captured VM from its in-RAM state (not a reboot) — the foundation of the
//! validate-then-promote flow (Master Spec v02.187 §3.5.7 #7). Warm model RPC
//! and live token streaming still require a resident model-serving guest agent
//! or image; this layer only provides the generic command channel.
//!
//! # Environment / host dependencies
//!
//! Every host path defaults to a proven WSL2 layout and is overridable via a
//! `HANDSHAKE_CH_*` environment variable (see [`CloudHypervisorConfig`] for the
//! full table) so the adapter stays disk-agnostic ([GLOBAL-PORTABILITY]).
//!
//! # Failure modes / recovery
//!
//! On a host without WSL2 + KVM + the staged VM artifacts, `try_new` returns
//! [`crate::sandbox::SandboxAdapterError::AdapterUnavailable`]; the bootstrap
//! registry skips the adapter rather than failing sandbox bring-up. Snapshot
//! steps that fail after pause best-effort resume the source VM and drop the
//! partial snapshot dir; persistent VMs are reclaimed by an explicit `kill`
//! (the retained child also reaps on drop via `kill_on_drop`).

pub mod adapter;
pub mod guest_agent;
pub mod warm_agent_package;

pub use adapter::*;
pub use guest_agent::*;
pub use warm_agent_package::*;
