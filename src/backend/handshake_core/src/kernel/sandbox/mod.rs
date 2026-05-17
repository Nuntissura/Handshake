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
//!
//! The sandbox module is the canonical home for sandbox primitives; storage
//! glue lives under `crate::storage::kb003_storage`.

pub mod adapter;
pub mod compat_blocker;
pub mod dcc_projection;
pub mod denial;
pub mod no_sqlite_tripwire;
pub mod policy;
pub mod policy_scoped_local;
pub mod replay_projection;
pub mod run;
pub mod workspace;

pub use adapter::*;
pub use compat_blocker::*;
pub use dcc_projection::*;
pub use denial::*;
pub use no_sqlite_tripwire::*;
pub use policy::*;
pub use policy_scoped_local::*;
pub use replay_projection::*;
pub use run::*;
pub use workspace::*;
