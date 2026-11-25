//! Opaque identities carried by every owner record.
//!
//! Widths follow the canonical Studio client port (`hsk_studio_render_gpu::client::{ModuleRef, ClientRef,
//! GrantId}`) so the later port adapter is a field copy, never a lossy conversion. None of these values
//! grants anything: identity and correlation are attribution only (GUI-DESKTOP-050, GPU-QUOTA-011).

/// Module identity assigned by the host registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(pub u32);

/// Client identity within a module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(pub u64);

/// The original operation a claim belongs to (GPU-QUOTA-010: "original operation/claim identity").
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(pub u128);

/// Revision of the quota policy a claim was admitted under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PolicyRevision(pub u64);

/// Private opaque reference to the original authorization of a claim (GPU-QUOTA-008).
///
/// The owner records it and never interprets it. Supplying a value here authorizes nothing: the grant,
/// capability and issuer checks of GPU-QUOTA-011 and NATIVE-RENDER-BOOT-002 are a separate gate that this
/// unit neither performs nor implies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AuthorizationProvenance(pub [u8; 32]);

/// Claim identity, unique within the ledger that issued it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClaimId(pub(crate) u64);

impl ClaimId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Stable identity of a physical adapter.
///
/// A device generation is not an identity (GPU-QUOTA-001): the same adapter keeps this key across
/// generations, and a different adapter never silently takes its place (GUI-DESKTOP-026).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DeviceKey {
    pub vendor_id: u32,
    pub device_id: u32,
    /// `VkPhysicalDeviceIDProperties::deviceUUID`.
    pub device_uuid: [u8; 16],
}
