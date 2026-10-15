use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{ProcessRuntimeOwner, RuntimeInstanceError};

pub const EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID: &str = "hsk.embedded_runtime.instance@2";
pub const EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL: &str =
    "hsk.embedded_runtime.loopback_udp_exclusive@1";
pub const HANDSHAKE_HOST_SCOPE_ID_ENV: &str = "HANDSHAKE_HOST_SCOPE_ID";
/// Non-optional process-liveness evidence carried by every pid-less embedded
/// runtime START row.
///
/// The UDP socket is deliberately loopback-only and never carries traffic. Its
/// exclusive OS bind is the lease: process crash releases the port, while a
/// durable-store restart cannot. `host_scope_id` prevents a reclaimer connected
/// to shared durable storage from treating its own loopback namespace as another
/// host's namespace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddedRuntimeInstanceDescriptor {
    #[serde(rename = "runtime_instance_id")]
    pub instance_id: Uuid,
    #[serde(rename = "runtime_host_scope_id")]
    pub host_scope_id: String,
    #[serde(rename = "runtime_lease_protocol")]
    pub lease_protocol: String,
    #[serde(rename = "runtime_lease_address")]
    pub loopback_address: IpAddr,
    #[serde(rename = "runtime_lease_port")]
    pub loopback_port: u16,
}

impl EmbeddedRuntimeInstanceDescriptor {
    pub fn metadata_fields(&self) -> Value {
        serde_json::json!({
            "runtime_instance_schema_id": EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID,
            "runtime_instance_id": self.instance_id.to_string(),
            "runtime_host_scope_id": self.host_scope_id.clone(),
            "runtime_lease_protocol": self.lease_protocol.clone(),
            "runtime_lease_address": self.loopback_address.to_string(),
            "runtime_lease_port": self.loopback_port,
        })
    }

    pub fn process_runtime_owner(&self) -> ProcessRuntimeOwner {
        ProcessRuntimeOwner {
            runtime_instance_id: self.instance_id,
            host_scope_id: self.host_scope_id.clone(),
            lease_schema_id: EMBEDDED_RUNTIME_INSTANCE_SCHEMA_ID.to_string(),
            lease_protocol: self.lease_protocol.clone(),
            lease_address: self.loopback_address.to_string(),
            lease_port: self.loopback_port,
        }
    }
}

/// An OS-owned liveness lease for one Handshake backend instance.
///
/// Keeping `_socket` alive keeps the exact loopback endpoint exclusively bound.
/// No database connection is held, so database restart/session loss cannot be
/// confused with process death and the application pool loses no capacity.
pub struct EmbeddedRuntimeInstanceLease {
    descriptor: EmbeddedRuntimeInstanceDescriptor,
    _socket: UdpSocket,
}

impl EmbeddedRuntimeInstanceLease {
    pub fn instance_id(&self) -> Uuid {
        self.descriptor.instance_id
    }

    pub fn descriptor(&self) -> &EmbeddedRuntimeInstanceDescriptor {
        &self.descriptor
    }

    pub async fn release(self) -> Result<(), RuntimeInstanceError> {
        drop(self);
        Ok(())
    }
}

/// Resolve the explicit host identity used to keep loopback liveness evidence
/// local to one machine. Database endpoint identity is never used as a host
/// identity in the embedded-Surreal runtime.
pub fn resolve_embedded_runtime_host_scope() -> Result<String, RuntimeInstanceError> {
    let explicit = std::env::var(HANDSHAKE_HOST_SCOPE_ID_ENV).ok();
    resolve_embedded_runtime_host_scope_with_override(explicit.as_deref())
}

pub fn resolve_embedded_runtime_host_scope_with_override(
    explicit_host_scope: Option<&str>,
) -> Result<String, RuntimeInstanceError> {
    let host_scope = explicit_host_scope
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            RuntimeInstanceError::InvalidConfig(format!(
                "embedded Surreal runtime requires explicit {HANDSHAKE_HOST_SCOPE_ID_ENV}"
            ))
        })?;
    if host_scope.len() > 256 {
        return Err(RuntimeInstanceError::InvalidConfig(format!(
            "{HANDSHAKE_HOST_SCOPE_ID_ENV} exceeds 256 bytes"
        )));
    }
    Ok(host_scope.to_owned())
}

/// Acquire the process-lifetime OS lease before any model artifact is opened.
/// This is synchronous so callers cannot accidentally spawn work between lease
/// selection and ownership. The second-bind self-test fails closed on a platform
/// whose ordinary UDP bind semantics do not provide exclusivity.
pub fn acquire_embedded_runtime_instance_lease(
    instance_id: Uuid,
    host_scope_id: impl Into<String>,
) -> Result<EmbeddedRuntimeInstanceLease, RuntimeInstanceError> {
    let host_scope_id = host_scope_id.into();
    if host_scope_id.trim().is_empty() {
        return Err(RuntimeInstanceError::InvalidConfig(
            "embedded runtime host_scope_id must not be empty".to_string(),
        ));
    }
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|error| {
        RuntimeInstanceError::Store(format!(
            "failed to bind embedded runtime loopback UDP lease: {error}"
        ))
    })?;
    let address = socket.local_addr().map_err(|error| {
        RuntimeInstanceError::Store(format!(
            "failed to inspect embedded runtime loopback UDP lease: {error}"
        ))
    })?;
    verify_second_udp_bind_is_rejected(address)?;
    Ok(EmbeddedRuntimeInstanceLease {
        descriptor: EmbeddedRuntimeInstanceDescriptor {
            instance_id,
            host_scope_id,
            lease_protocol: EMBEDDED_RUNTIME_LOOPBACK_UDP_PROTOCOL.to_string(),
            loopback_address: address.ip(),
            loopback_port: address.port(),
        },
        _socket: socket,
    })
}

fn verify_second_udp_bind_is_rejected(address: SocketAddr) -> Result<(), RuntimeInstanceError> {
    match UdpSocket::bind(address) {
        Ok(second) => {
            drop(second);
            Err(RuntimeInstanceError::Store(format!(
                "loopback UDP lease endpoint {address} accepted a second ordinary bind"
            )))
        }
        // The platform-specific error kind differs (notably AddrInUse vs
        // PermissionDenied on Windows). The invariant is simply that the exact
        // second ordinary bind was rejected after the first succeeded.
        Err(_) => Ok(()),
    }
}

#[derive(Debug)]
enum UdpLeaseClaim {
    Claimed(UdpSocket),
    Protected,
    Ambiguous(io::Error),
}

fn try_claim_udp_lease(descriptor: &EmbeddedRuntimeInstanceDescriptor) -> UdpLeaseClaim {
    let address = SocketAddr::new(descriptor.loopback_address, descriptor.loopback_port);
    match UdpSocket::bind(address) {
        Ok(socket) => match UdpSocket::bind(address) {
            Ok(second) => {
                drop(second);
                drop(socket);
                UdpLeaseClaim::Ambiguous(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    "claimed UDP endpoint did not enforce exclusive second-bind rejection",
                ))
            }
            // As in acquisition, the platform-specific rejection kind is not
            // stable. The first exact bind succeeded and the second exact bind
            // failed, which is the OS-ownership invariant we need.
            Err(_) => UdpLeaseClaim::Claimed(socket),
        },
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => UdpLeaseClaim::Protected,
        Err(error) => UdpLeaseClaim::Ambiguous(error),
    }
}

pub fn runtime_owner_loopback_lease_is_free(owner: &ProcessRuntimeOwner) -> bool {
    let Some(loopback_address) = owner
        .lease_address
        .parse::<IpAddr>()
        .ok()
        .filter(IpAddr::is_loopback)
    else {
        return false;
    };
    let descriptor = EmbeddedRuntimeInstanceDescriptor {
        instance_id: owner.runtime_instance_id,
        host_scope_id: owner.host_scope_id.clone(),
        lease_protocol: owner.lease_protocol.clone(),
        loopback_address,
        loopback_port: owner.lease_port,
    };
    matches!(try_claim_udp_lease(&descriptor), UdpLeaseClaim::Claimed(_))
}
