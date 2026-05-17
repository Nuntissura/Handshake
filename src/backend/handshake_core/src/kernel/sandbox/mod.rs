//! KB003 sandbox primitives.
//!
//! WP-KERNEL-003 MT-010..MT-029 land here. The module is organised as small
//! focused submodules so each microtask owns one file:
//!
//! - `dcc_projection`   — MT-010 operator projection contract (DCC view of sandbox/promotion state).
//! - `run`              — sandbox run lifecycle types shared across MTs.
//! - `policy`           — `SandboxPolicy` with default-deny stance.
//! - `workspace`        — sandbox workspace boundary descriptors.
//! - `denial`           — typed denial evidence records.
//! - `no_sqlite_tripwire` — MT-015 fail-closed guard for KB003 authority writes.
//! - `replay_projection`  — MT-016 replay-from-durable-storage contract.
//! - `compat_blocker`     — MT-017 missing-API blocker detector.
//! - `adapter`            — MT-018 `SandboxAdapter` trait + extension slots.
//! - `policy_scoped_local` — MT-019 PolicyScopedLocal default adapter.
//! - `hard_isolation`           — MT-020 hard-isolation adapter slot (typed BLOCKED/UNSUPPORTED).
//! - `hard_isolation_container` — MT-020 non-executing container stub.
//! - `hard_isolation_microvm`   — MT-020 non-executing microVM stub.
//! - `host_platform_probe`      — MT-020 deterministic host-kind probe.
//! - `adapter_selection`        — MT-020 deterministic adapter selection + typed fallback evidence.
//! - `policy_default_deny` — MT-021 extended default-deny policy bundle.
//! - `fs_guard`            — MT-022 filesystem scope guard with typed denials.
//! - `network_gate`        — MT-023 network capability gate (grants require approval+provenance).
//! - `exec_allowlist`      — MT-024 process execution descriptor allowlist.
//! - `redaction`           — MT-025 env/secret redaction for logs and reports.
//! - `resource_caps`       — MT-026 deterministic resource cap evaluation.
//! - `cancellation`        — MT-027 cancellation, timeout, and promotion guard.
//! - `workspace_materializer` — MT-028 candidate-input materialization with manifest.
//! - `cleanup`             — MT-029 cleanup planner that preserves artifacts and authority rows.
//!
//! The sandbox module is the canonical home for sandbox primitives; storage
//! glue lives under `crate::storage::kb003_storage`.

pub mod adapter;
pub mod adapter_selection;
pub mod cancellation;
pub mod cleanup;
pub mod compat_blocker;
pub mod dcc_projection;
pub mod denial;
pub mod exec_allowlist;
pub mod fs_guard;
pub mod hard_isolation;
pub mod hard_isolation_container;
pub mod hard_isolation_microvm;
pub mod host_platform_probe;
pub mod network_gate;
pub mod no_sqlite_tripwire;
pub mod policy;
pub mod policy_default_deny;
pub mod policy_scoped_local;
pub mod redaction;
pub mod replay_projection;
pub mod resource_caps;
pub mod run;
pub mod workspace;
pub mod workspace_materializer;

pub use adapter::*;
pub use adapter_selection::*;
pub use cancellation::*;
pub use cleanup::*;
pub use compat_blocker::*;
pub use dcc_projection::*;
pub use denial::*;
pub use exec_allowlist::*;
pub use fs_guard::*;
pub use hard_isolation::*;
pub use hard_isolation_container::*;
pub use hard_isolation_microvm::*;
pub use host_platform_probe::*;
pub use network_gate::*;
pub use no_sqlite_tripwire::*;
pub use policy::*;
pub use policy_default_deny::*;
pub use policy_scoped_local::*;
pub use redaction::*;
pub use replay_projection::*;
pub use resource_caps::*;
pub use run::*;
pub use workspace::*;
pub use workspace_materializer::*;
