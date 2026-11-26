//! Owner images and buffers: made for a client's claim, charged when made, released by the client, destroyed by the
//! owner once nothing that may use them runs (`vulkan/resources.rs`), and read or written by the host only when no
//! submission of their client on their device is pending.

use super::queue::fault_of;
use super::{Live, OwnerShared};
use crate::diag::OwnerEvent;
use crate::ledger::{Claim, LedgerClient};
use crate::outcome::{AuthorizeRefusal, DeviceFault};
use crate::records::ChargeClass;
use crate::refusal::{AccessRefusal, ResourceRefusal};
use crate::resource::{
    BufferHandles, BufferMemory, BufferRequest, ImageRequest, Origin, OwnerBuffer, OwnerImage,
    RenderAttachment,
};
use crate::vulkan::{BufferSpec, ImageSpec, MemoryRefused, Owned, QueueFault, ResourceFault};
use std::sync::Arc;

fn refusal_of(fault: ResourceFault, generation: u64) -> ResourceRefusal {
    match fault {
        ResourceFault::FormatUnsupported => ResourceRefusal::FormatUnsupported,
        ResourceFault::ExtentUnsupported => ResourceRefusal::ExtentUnsupported,
        ResourceFault::Memory(refused) => match refused {
            MemoryRefused::OverBudget { heap } => ResourceRefusal::OverBudget { heap },
            MemoryRefused::OutOfDeviceMemory => ResourceRefusal::OutOfDeviceMemory,
            MemoryRefused::OutOfHostMemory => ResourceRefusal::OutOfHostMemory,
            MemoryRefused::NoMemoryType => ResourceRefusal::NoMemoryType,
            MemoryRefused::TooManyObjects => ResourceRefusal::TooManyObjects,
            MemoryRefused::Driver(raw) => ResourceRefusal::Device(DeviceFault::Driver(raw)),
        },
        ResourceFault::Device(fault) => ResourceRefusal::Device(fault_of(fault, generation)),
    }
}

impl OwnerShared {
    /// The bound generation of `claim`, after authorizing the claim.
    pub(super) fn resource_device(
        &self,
        client: &LedgerClient,
        claim: &Claim,
    ) -> Result<Live, ResourceRefusal> {
        client
            .authorize(claim)
            .map_err(ResourceRefusal::NotAuthorized)?;
        match self.lock_slot().live.clone() {
            Some(live) if live.number == claim.generation() => Ok(live),
            _ => Err(ResourceRefusal::NotAuthorized(
                AuthorizeRefusal::DeviceUnavailable,
            )),
        }
    }

    /// Report a refusal, and say what a device fault in it means for the generation.
    pub(super) fn resource_refused(
        &self,
        client: &LedgerClient,
        claim: &Claim,
        live: Option<&Live>,
        refusal: ResourceRefusal,
    ) -> ResourceRefusal {
        if let (Some(live), ResourceRefusal::Device(DeviceFault::Lost { .. })) = (live, refusal) {
            self.device_failed(live, QueueFault::Lost);
        }
        self.core.report(OwnerEvent::ResourceRefused {
            subject: client.subject_of(claim),
            reason: refusal.number(),
        });
        refusal
    }

    /// Make an owner image for `claim`.
    pub(super) fn create_image(
        self: &Arc<Self>,
        client: &LedgerClient,
        claim: &Claim,
        request: &ImageRequest,
    ) -> Result<OwnerImage, ResourceRefusal> {
        if request.extent.width == 0 || request.extent.height == 0 || request.usage.is_empty() {
            return Err(self.resource_refused(
                client,
                claim,
                None,
                ResourceRefusal::InvalidRequest,
            ));
        }
        let live = self
            .resource_device(client, claim)
            .map_err(|refusal| self.resource_refused(client, claim, None, refusal))?;
        let made = live
            .vulkan
            .make_image(ImageSpec::of(request))
            .map_err(|fault| {
                self.resource_refused(client, claim, Some(&live), refusal_of(fault, live.number))
            })?;
        let bytes = made.bytes();
        let (image, view) = made.raw();
        if let Err(refusal) = client.own(claim, ChargeClass::Committed, bytes) {
            live.vulkan.discard(made);
            return Err(self.resource_refused(
                client,
                claim,
                None,
                ResourceRefusal::Charge(refusal),
            ));
        }
        let owned = Owned {
            claim: claim.id().get(),
            class: ChargeClass::Committed,
            bytes,
        };
        let key = live.vulkan.keep(made, owned);
        Ok(OwnerImage {
            owner: Arc::clone(self),
            origin: Origin {
                ledger: client.ledger_key(),
                seat: client.seat(),
                claim: claim.info(),
                key,
            },
            attachment: RenderAttachment {
                image,
                view,
                format: request.format,
                extent: request.extent,
                usage: request.usage,
                generation: live.number,
            },
            bytes,
        })
    }

    /// Make an owner buffer for `claim`.
    pub(super) fn create_buffer(
        self: &Arc<Self>,
        client: &LedgerClient,
        claim: &Claim,
        request: &BufferRequest,
    ) -> Result<OwnerBuffer, ResourceRefusal> {
        if request.size == 0 || request.usage.is_empty() {
            return Err(self.resource_refused(
                client,
                claim,
                None,
                ResourceRefusal::InvalidRequest,
            ));
        }
        let live = self
            .resource_device(client, claim)
            .map_err(|refusal| self.resource_refused(client, claim, None, refusal))?;
        let made = live
            .vulkan
            .make_buffer(BufferSpec::of(request))
            .map_err(|fault| {
                self.resource_refused(client, claim, Some(&live), refusal_of(fault, live.number))
            })?;
        let class = match request.memory {
            BufferMemory::Device => ChargeClass::Committed,
            BufferMemory::Upload | BufferMemory::Readback => ChargeClass::Staging,
        };
        let bytes = made.bytes();
        let (buffer, _) = made.raw();
        if let Err(refusal) = client.own(claim, class, bytes) {
            live.vulkan.discard(made);
            return Err(self.resource_refused(
                client,
                claim,
                None,
                ResourceRefusal::Charge(refusal),
            ));
        }
        let owned = Owned {
            claim: claim.id().get(),
            class,
            bytes,
        };
        let key = live.vulkan.keep(made, owned);
        Ok(OwnerBuffer {
            owner: Arc::clone(self),
            origin: Origin {
                ledger: client.ledger_key(),
                seat: client.seat(),
                claim: claim.info(),
                key,
            },
            handles: BufferHandles {
                buffer,
                size: request.size,
                usage: request.usage,
                memory: request.memory,
                generation: live.number,
            },
            bytes,
        })
    }

    /// Whether the claim of a resource may still use its device.
    pub(crate) fn standing(&self, origin: &Origin) -> Result<(), AuthorizeRefusal> {
        self.core.ledger().standing_of(origin.claim.id.get())
    }

    /// The client let go of a resource. It is destroyed once nothing that may use it runs; a resource whose device is
    /// gone went with it.
    pub(crate) fn release_resource(&self, origin: &Origin) {
        let live = self.lock_slot().generation(origin.generation()).cloned();
        if let Some(live) = live {
            if live.vulkan.release(origin.key) {
                let _ = self.look(&live);
            }
        }
        self.reap();
    }

    /// The device of a buffer, once nothing of its client may still use the buffer there.
    fn quiet_buffer(&self, origin: &Origin) -> Result<Live, AccessRefusal> {
        let generation = origin.generation();
        let Some(live) = self.lock_slot().generation(generation).cloned() else {
            return Err(AccessRefusal::Gone);
        };
        // What the device answers is in the ledger.
        let _ = self.look(&live);
        if live.vulkan.is_lost() {
            return Err(AccessRefusal::Lost { generation });
        }
        match self.core.ledger().seat_in_flight(generation, origin.seat) {
            0 => {}
            in_flight => return Err(AccessRefusal::InFlight { in_flight }),
        }
        if !live.vulkan.is_host_visible(origin.key) {
            return Err(AccessRefusal::NotHostVisible);
        }
        Ok(live)
    }

    fn access_refused(&self, origin: &Origin, refusal: AccessRefusal) -> AccessRefusal {
        self.core.report(OwnerEvent::AccessRefused {
            subject: crate::diag::Subject::claim(&origin.claim),
            reason: refusal.number(),
        });
        refusal
    }

    pub(crate) fn read_buffer(
        &self,
        origin: &Origin,
        offset: u64,
        out: &mut [u8],
    ) -> Result<(), AccessRefusal> {
        let live = self
            .quiet_buffer(origin)
            .map_err(|refusal| self.access_refused(origin, refusal))?;
        let result = match live.vulkan.read_buffer(origin.key, offset, out) {
            Some(true) => Ok(()),
            Some(false) => Err(AccessRefusal::MapFailed),
            None => Err(AccessRefusal::OutOfRange),
        };
        result.map_err(|refusal| self.access_refused(origin, refusal))
    }

    pub(crate) fn write_buffer(
        &self,
        origin: &Origin,
        offset: u64,
        data: &[u8],
    ) -> Result<(), AccessRefusal> {
        let live = self
            .quiet_buffer(origin)
            .map_err(|refusal| self.access_refused(origin, refusal))?;
        let result = match live.vulkan.write_buffer(origin.key, offset, data) {
            Some(true) => Ok(()),
            Some(false) => Err(AccessRefusal::MapFailed),
            None => Err(AccessRefusal::OutOfRange),
        };
        result.map_err(|refusal| self.access_refused(origin, refusal))
    }
}
