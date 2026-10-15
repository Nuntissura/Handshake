use std::io;

use serde::{Deserialize, Serialize};

/// Non-secret original process/ring identity signed by the current owned watcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatcherSigningIdentity {
    pub protocol_version: u32,
    pub session_id: uuid::Uuid,
    pub process_instance_id: uuid::Uuid,
    pub parent_pid: u32,
    pub watcher_pid: u32,
    pub ring_version: u32,
    pub record_size: u32,
    pub launch_nonce: [u8; 16],
    pub runtime_instance_schema_id: String,
    pub runtime_instance_id: uuid::Uuid,
    pub runtime_host_scope_id: String,
    pub runtime_lease_protocol: String,
    pub runtime_lease_address: std::net::IpAddr,
    pub runtime_lease_port: u16,
    pub parent_os_creation_time_100ns: u64,
}

pub fn watcher_challenge_bytes(
    identity: &WatcherSigningIdentity,
    challenge: [u8; 32],
    transport_public_key: [u8; 32],
) -> io::Result<Vec<u8>> {
    let mut bytes = b"handshake-current-watcher-provenance-v1\0".to_vec();
    let identity_bytes = serde_json::to_vec(identity).map_err(io::Error::other)?;
    bytes.extend_from_slice(&(identity_bytes.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&identity_bytes);
    bytes.extend_from_slice(&challenge);
    bytes.extend_from_slice(&transport_public_key);
    Ok(bytes)
}
