mod lease;
mod process_identity;

pub use lease::{
    acquire_embedded_runtime_instance_lease, resolve_embedded_runtime_host_scope,
    resolve_embedded_runtime_host_scope_with_override, runtime_owner_loopback_lease_is_free,
    EmbeddedRuntimeInstanceDescriptor, EmbeddedRuntimeInstanceLease,
    EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID, EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL,
    HANDSHAKE_HOST_SCOPE_ID_ENV,
};
pub use process_identity::process_creation_time_100ns;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessRuntimeOwner {
    pub runtime_instance_id: Uuid,
    pub host_scope_id: String,
    pub lease_schema_id: String,
    pub lease_protocol: String,
    pub lease_address: String,
    pub lease_port: u16,
}

#[derive(Debug, Error)]
pub enum RuntimeInstanceError {
    #[error("PROCESS_LEDGER_INVALID_CONFIG: {0}")]
    InvalidConfig(String),
    #[error("PROCESS_LEDGER_STORE: {0}")]
    Store(String),
}
