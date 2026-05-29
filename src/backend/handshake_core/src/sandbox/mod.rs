//! Adapter-neutral sandbox contracts for KERNEL-004.

pub mod adapter;
pub mod bootstrap;
pub mod cloud_hypervisor;
pub mod docker;
pub mod gvisor;
pub mod ledger_decorator;
pub mod promotion_binding;
pub mod registry;
pub mod selection;
pub mod settings_schema;
pub mod types;
pub mod validation_runner_binding;
pub mod windows_native_jail;
pub mod work_packet_scope_binder;
pub mod wsl2_podman;

pub use adapter::*;
pub use bootstrap::*;
pub use cloud_hypervisor::*;
pub use docker::*;
pub use gvisor::*;
pub use ledger_decorator::*;
pub use promotion_binding::*;
pub use registry::*;
pub use selection::*;
pub use settings_schema::*;
pub use types::*;
pub use validation_runner_binding::*;
pub use windows_native_jail::*;
pub use work_packet_scope_binder::*;
pub use wsl2_podman::*;
