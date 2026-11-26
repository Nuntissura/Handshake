//! Images and buffers the owner creates, owns and destroys for its clients (MT-203 slice 2, G203-H1).
//!
//! A resource lives in its generation's registry from the moment its charge was accepted until the owner destroyed it.
//! Its client holds only an identity and raw handles. When the client lets it go it is retired, and destroyed once the
//! owner has observed that no work that may use it can still run (VUID-vkDestroyImage-image-01000,
//! VUID-vkDestroyImageView-imageView-01026, VUID-vkDestroyBuffer-buffer-00922, VUID-vkFreeMemory-memory-00677: all
//! submitted commands that refer to it have completed execution):
//! - on a healthy device, when the fence of the last value the queue was given at its release has signalled (a fence
//!   signals after everything earlier in submission order). A release while a command buffer is being recorded or
//!   submitted waits for the end of every such recording, which may refer to it, and takes the last value then.
//! - on a lost device, when the queue is drained ("Lost Device": a device that answers success or loss on every fence
//!   no longer uses the work) and no recording runs.
//! - at the destruction of the generation, whatever is left, children before the device
//!   (VUID-vkDestroyDevice-device-05137).
//!
//! Each is destroyed exactly once: it leaves the registry under its lock, and only the thread that took it out destroys
//! it and reports it.
//!
//! The host reads and writes a buffer within the buffer's own size (its `VkBuffer`), never within its block, which
//! `gpu-alloc` may have made larger (a power of two). The block is mapped under the buffer's own lock; the registry's
//! lock is held only to find it (MT-203 E3-1).

use super::memory::{Block, MemoryRefused, Placement, Use};
use super::queue::QueueFault;
use super::VulkanGeneration;
use crate::records::ChargeClass;
use crate::resource::{BufferMemory, BufferRequest, ImageRequest};
use ash::vk::{self, Handle};
use gpu_alloc::Dedicated;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// Process-unique resource identities.
static NEXT_KEY: AtomicU64 = AtomicU64::new(1);

/// An image the owner was asked for.
#[derive(Clone, Copy)]
pub(crate) struct ImageSpec {
    pub(crate) format: vk::Format,
    pub(crate) extent: vk::Extent2D,
    pub(crate) usage: vk::ImageUsageFlags,
}

impl ImageSpec {
    pub(crate) fn of(request: &ImageRequest) -> Self {
        Self {
            format: vk::Format::from_raw(request.format.vk_format()),
            extent: vk::Extent2D {
                width: request.extent.width,
                height: request.extent.height,
            },
            usage: vk::ImageUsageFlags::from_raw(request.usage.vk_flags()),
        }
    }
}

/// A buffer the owner was asked for.
#[derive(Clone, Copy)]
pub(crate) struct BufferSpec {
    pub(crate) size: u64,
    pub(crate) usage: vk::BufferUsageFlags,
    pub(crate) memory: Use,
}

impl BufferSpec {
    pub(crate) fn of(request: &BufferRequest) -> Self {
        Self {
            size: request.size,
            usage: vk::BufferUsageFlags::from_raw(request.usage.vk_flags()),
            memory: match request.memory {
                BufferMemory::Device => Use::Device,
                BufferMemory::Upload => Use::Upload,
                BufferMemory::Readback => Use::Readback,
            },
        }
    }
}

/// Why a resource was not made. Closed codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResourceFault {
    /// The device does not offer the format with this use as an optimally tiled 2D image.
    FormatUnsupported,
    /// The extent is zero or above what the device allows for the format and use.
    ExtentUnsupported,
    Memory(MemoryRefused),
    Device(QueueFault),
}

/// Whose a resource is, and what it is charged with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Owned {
    /// The claim it is charged to.
    pub(crate) claim: u64,
    pub(crate) class: ChargeClass,
    pub(crate) bytes: u64,
}

enum Objects {
    Image {
        image: vk::Image,
        view: vk::ImageView,
        block: Block,
    },
    Buffer {
        buffer: vk::Buffer,
        /// The buffer's own size (`VkBufferCreateInfo::size`): the bound of every host access.
        size: u64,
        /// Shared only for the length of a host access, which holds the buffer's handle, so never at its release.
        block: Arc<Mutex<Block>>,
    },
}

fn lock_block(block: &Mutex<Block>) -> MutexGuard<'_, Block> {
    block.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Objects {
    /// No host access of it runs: it may be destroyed.
    fn is_unshared(&self) -> bool {
        match self {
            Self::Image { .. } => true,
            Self::Buffer { block, .. } => Arc::strong_count(block) == 1,
        }
    }
}

/// A resource that exists and is not kept yet: [`VulkanGeneration::keep`] or [`VulkanGeneration::discard`] it.
pub(crate) struct Made {
    objects: Objects,
}

impl Made {
    /// Raw handle of the image or buffer, and of the view (0 for a buffer).
    pub(crate) fn raw(&self) -> (u64, u64) {
        match &self.objects {
            Objects::Image { image, view, .. } => (image.as_raw(), view.as_raw()),
            Objects::Buffer { buffer, .. } => (buffer.as_raw(), 0),
        }
    }

    /// Bytes of device memory placed for it.
    pub(crate) fn bytes(&self) -> u64 {
        match &self.objects {
            Objects::Image { block, .. } => block.size(),
            Objects::Buffer { block, .. } => lock_block(block).size(),
        }
    }
}

struct Entry {
    objects: Objects,
    owned: Owned,
}

struct Retiring {
    key: u64,
    /// Destroyed once this value was observed; `None` while recordings that began before the release run.
    after: Option<u64>,
}

/// The registry of one generation.
#[derive(Default)]
pub(crate) struct Resources {
    live: Mutex<BTreeMap<u64, Entry>>,
    retiring: Mutex<Vec<Retiring>>,
}

impl Resources {
    fn lock_live(&self) -> MutexGuard<'_, BTreeMap<u64, Entry>> {
        self.live.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_retiring(&self) -> MutexGuard<'_, Vec<Retiring>> {
        self.retiring.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The last recording ended at queue value `last_value`: releases that waited for it wait for that value now.
    pub(super) fn recordings_ended(&self, last_value: u64) {
        for retiring in self.lock_retiring().iter_mut() {
            retiring.after.get_or_insert(last_value);
        }
    }

    /// Resources that exist now.
    pub(crate) fn count(&self) -> usize {
        self.lock_live().len()
    }
}

fn device_fault(result: vk::Result) -> ResourceFault {
    ResourceFault::Device(QueueFault::of(result))
}

impl VulkanGeneration {
    /// Destroy objects that are no longer in use, and give their memory back.
    ///
    /// # Safety
    /// Nothing refers to the objects any more ([`Objects::is_unshared`]) and no work that used them may still run.
    unsafe fn destroy_objects(&self, objects: Objects) {
        match objects {
            Objects::Image { image, view, block } => {
                // SAFETY: the caller's; the view goes before its image, the image before its memory.
                unsafe {
                    self.device.destroy_image_view(view, None);
                    self.device.destroy_image(image, None);
                    self.free_block(block);
                }
            }
            Objects::Buffer { buffer, block, .. } => {
                // SAFETY: the caller's.
                unsafe { self.device.destroy_buffer(buffer, None) };
                match Arc::try_unwrap(block) {
                    // SAFETY: the caller's; the buffer is gone, so nothing is bound to the block.
                    Ok(block) => unsafe {
                        self.free_block(block.into_inner().unwrap_or_else(PoisonError::into_inner))
                    },
                    // Unreachable under the caller's contract; a block that is still shared is left, not freed.
                    Err(shared) => std::mem::forget(shared),
                }
            }
        }
    }

    /// Create a 2D, optimally tiled, single-sampled image with its own memory and a color view of it, in the
    /// `UNDEFINED` layout.
    pub(crate) fn make_image(&self, spec: ImageSpec) -> Result<Made, ResourceFault> {
        if spec.extent.width == 0 || spec.extent.height == 0 {
            // VUID-VkImageCreateInfo-extent-00944/00945.
            return Err(ResourceFault::ExtentUnsupported);
        }
        // SAFETY: read-only query of this generation's physical device.
        let limits = unsafe {
            self.instance.get_physical_device_image_format_properties(
                self.physical,
                spec.format,
                vk::ImageType::TYPE_2D,
                vk::ImageTiling::OPTIMAL,
                spec.usage,
                vk::ImageCreateFlags::empty(),
            )
        };
        let limits = match limits {
            Ok(limits) => limits,
            Err(vk::Result::ERROR_FORMAT_NOT_SUPPORTED) => {
                return Err(ResourceFault::FormatUnsupported)
            }
            Err(result) => return Err(device_fault(result)),
        };
        if spec.extent.width > limits.max_extent.width
            || spec.extent.height > limits.max_extent.height
        {
            // VUID-VkImageCreateInfo-extent-02252/02253.
            return Err(ResourceFault::ExtentUnsupported);
        }
        let create = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(spec.format)
            .extent(vk::Extent3D {
                width: spec.extent.width,
                height: spec.extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(spec.usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        // SAFETY: the format, use and extent were checked above against what the device reports for them.
        let image = unsafe { self.device.create_image(&create, None) }.map_err(device_fault)?;
        let mut dedicated = vk::MemoryDedicatedRequirements::default();
        let requirements = {
            let info = vk::ImageMemoryRequirementsInfo2::default().image(image);
            let mut out = vk::MemoryRequirements2::default().push_next(&mut dedicated);
            // SAFETY: core 1.1 query on an image of this device; the chain lives for the call.
            unsafe { self.device.get_image_memory_requirements2(&info, &mut out) };
            out.memory_requirements
        };
        let wants_own = dedicated.requires_dedicated_allocation == vk::TRUE
            || dedicated.prefers_dedicated_allocation == vk::TRUE;
        let placement = Placement {
            requirements,
            usage: Use::Device,
            // VUID-vkBindImageMemory-image-01445: an image that requires it is bound to memory made for it alone.
            image: wants_own.then_some((image, Dedicated::Required)),
        };
        let block = match self.place(&placement) {
            Ok(block) => block,
            Err(refused) => {
                // SAFETY: the image was never bound, recorded or submitted.
                unsafe { self.device.destroy_image(image, None) };
                return Err(ResourceFault::Memory(refused));
            }
        };
        // SAFETY: unbound image (VUID-vkBindImageMemory-image-07460); memory of a type its requirements allow, at an
        // offset aligned to them, with room for its size (`gpu-alloc` placed it for exactly these requirements).
        let bound = unsafe {
            self.device
                .bind_image_memory(image, *block.memory(), block.offset())
        };
        let view = bound.and_then(|()| {
            let range = vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .level_count(1)
                .layer_count(1);
            let create = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(spec.format)
                .subresource_range(range);
            // SAFETY: a bound image of this device with a color format, one level and one layer.
            unsafe { self.device.create_image_view(&create, None) }
        });
        match view {
            Ok(view) => Ok(Made {
                objects: Objects::Image { image, view, block },
            }),
            Err(result) => {
                // SAFETY: nothing was recorded with the image; it goes before its memory.
                unsafe {
                    self.device.destroy_image(image, None);
                    self.free_block(block);
                }
                Err(device_fault(result))
            }
        }
    }

    /// Create a buffer with memory for `spec.memory`.
    pub(crate) fn make_buffer(&self, spec: BufferSpec) -> Result<Made, ResourceFault> {
        if spec.size == 0 {
            // VUID-VkBufferCreateInfo-size-00912.
            return Err(ResourceFault::ExtentUnsupported);
        }
        let create = vk::BufferCreateInfo::default()
            .size(spec.size)
            .usage(spec.usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        // SAFETY: valid device; a non-zero size and a non-empty use (checked by the caller).
        let buffer = unsafe { self.device.create_buffer(&create, None) }.map_err(device_fault)?;
        // SAFETY: a buffer of this device.
        let requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };
        let placement = Placement {
            requirements,
            usage: spec.memory,
            image: None,
        };
        let block = match self.place(&placement) {
            Ok(block) => block,
            Err(refused) => {
                // SAFETY: never bound, recorded or submitted.
                unsafe { self.device.destroy_buffer(buffer, None) };
                return Err(ResourceFault::Memory(refused));
            }
        };
        // SAFETY: as for the image in `make_image` (VUID-vkBindBufferMemory-buffer-07459).
        let bound = unsafe {
            self.device
                .bind_buffer_memory(buffer, *block.memory(), block.offset())
        };
        match bound {
            Ok(()) => Ok(Made {
                objects: Objects::Buffer {
                    buffer,
                    size: spec.size,
                    block: Arc::new(Mutex::new(block)),
                },
            }),
            Err(result) => {
                // SAFETY: never recorded or submitted.
                unsafe {
                    self.device.destroy_buffer(buffer, None);
                    self.free_block(block);
                }
                Err(device_fault(result))
            }
        }
    }

    /// Keep `made` for the claim in `owned`; returns its identity.
    pub(crate) fn keep(&self, made: Made, owned: Owned) -> u64 {
        let key = NEXT_KEY.fetch_add(1, Ordering::Relaxed);
        self.resources.lock_live().insert(
            key,
            Entry {
                objects: made.objects,
                owned,
            },
        );
        key
    }

    /// Destroy `made` at once: it was never handed to a client.
    pub(crate) fn discard(&self, made: Made) {
        // SAFETY: never recorded or submitted, and nobody else knows its handles.
        unsafe { self.destroy_objects(made.objects) };
    }

    /// The client let go of resource `key`: it is destroyed once nothing that may use it can still run. `false` when it
    /// is not here (destroyed with an earlier generation end, or released already).
    pub(crate) fn release(&self, key: u64) -> bool {
        if !self.resources.lock_live().contains_key(&key) {
            return false;
        }
        // Under the state lock, so a recording that starts after this cannot refer to the resource.
        let state = self.lock_state();
        let after = (state.recording == 0).then_some(state.last_value);
        let mut retiring = self.resources.lock_retiring();
        if retiring.iter().any(|kept| kept.key == key) {
            return false;
        }
        retiring.push(Retiring { key, after });
        true
    }

    /// Destroy what was released and can no longer be in use. Returns what each was charged with, once.
    pub(crate) fn retire_ended(&self) -> Vec<Owned> {
        let view = self.view();
        let due: Vec<u64> = {
            let mut retiring = self.resources.lock_retiring();
            let mut due = Vec::new();
            retiring.retain(|kept| {
                let ended = if view.lost {
                    view.drained && view.recording == 0
                } else {
                    kept.after.is_some_and(|after| after <= view.observed)
                };
                if ended {
                    due.push(kept.key);
                }
                !ended
            });
            due
        };
        let mut ended = Vec::with_capacity(due.len());
        for key in due {
            let entry = {
                let mut live = self.resources.lock_live();
                match live.get(&key) {
                    Some(entry) if entry.objects.is_unshared() => live.remove(&key),
                    Some(_) => {
                        // A host access that began before the release still runs: the next look takes it.
                        self.resources.lock_retiring().push(Retiring {
                            key,
                            after: Some(0),
                        });
                        None
                    }
                    None => None,
                }
            };
            let Some(entry) = entry else {
                continue;
            };
            // SAFETY: released by its client, so nothing refers to it any more, and the queue says that every
            // submission that may refer to it has ended.
            unsafe { self.destroy_objects(entry.objects) };
            ended.push(entry.owned);
        }
        ended
    }

    /// The block of buffer `key` when `length` bytes at `offset` lie inside the buffer itself (not merely inside its
    /// block, which may be larger), found under the registry's lock and returned without it.
    fn buffer_range(&self, key: u64, offset: u64, length: usize) -> Option<Arc<Mutex<Block>>> {
        let live = self.resources.lock_live();
        let Some(Entry {
            objects: Objects::Buffer { size, block, .. },
            ..
        }) = live.get(&key)
        else {
            return None;
        };
        let length = u64::try_from(length).ok()?;
        // VkMappedMemoryRange and the buffer's binding cover `size` bytes; what lies beyond belongs to no resource.
        (offset.checked_add(length)? <= *size).then(|| Arc::clone(block))
    }

    /// Read from a host-visible buffer. `None` when there is no such buffer here, or the range does not lie inside it.
    ///
    /// The owner calls this only after it observed that no submission of the buffer's client on this generation is
    /// pending (`owner/resources.rs`), which is the condition of [`VulkanGeneration::read_block`]; a submission the same
    /// client starts meanwhile with this buffer breaks the documented rule of [`crate::OwnerBuffer::read`].
    pub(crate) fn read_buffer(&self, key: u64, offset: u64, out: &mut [u8]) -> Option<bool> {
        let block = self.buffer_range(key, offset, out.len())?;
        if out.is_empty() {
            return Some(true);
        }
        let mut block = lock_block(&block);
        // SAFETY: no submission that may write the block is pending (see above), and the commands that wrote it for the
        // host recorded a dependency to HOST_READ (`Recording::read_back`); the buffer's lock is held; the range lies
        // inside the buffer, which lies inside the block.
        Some(unsafe { self.read_block(&mut block, offset, out) })
    }

    /// Write into a host-visible buffer. `None` when there is no such buffer here, or the range does not lie inside it.
    /// Called on the same condition as [`Self::read_buffer`].
    pub(crate) fn write_buffer(&self, key: u64, offset: u64, data: &[u8]) -> Option<bool> {
        let block = self.buffer_range(key, offset, data.len())?;
        if data.is_empty() {
            return Some(true);
        }
        let mut block = lock_block(&block);
        // SAFETY: no submission that may use the block is pending (see `read_buffer`); the buffer's lock is held; the
        // range lies inside the buffer. A later submission sees the write: submitting is a host-to-device memory
        // dependency for host-coherent memory, and memory that is not coherent is flushed by `write_bytes` ("Host Write
        // Ordering Guarantees").
        Some(unsafe { self.write_block(&mut block, offset, data) })
    }

    /// The memory properties of a buffer's block, when it is a buffer here.
    fn buffer_props(&self, key: u64) -> Option<gpu_alloc::MemoryPropertyFlags> {
        let block = match self.resources.lock_live().get(&key) {
            Some(Entry {
                objects: Objects::Buffer { block, .. },
                ..
            }) => Arc::clone(block),
            _ => return None,
        };
        let props = lock_block(&block).props();
        Some(props)
    }

    /// Whether a buffer's memory is visible to the host.
    pub(crate) fn is_host_visible(&self, key: u64) -> bool {
        self.buffer_props(key)
            .is_some_and(|props| props.contains(gpu_alloc::MemoryPropertyFlags::HOST_VISIBLE))
    }

    /// Whether a buffer's memory is host-visible and not coherent, so that reading it needs an invalidate.
    #[cfg(test)]
    pub(crate) fn needs_invalidate(&self, key: u64) -> bool {
        use gpu_alloc::MemoryPropertyFlags as Props;
        self.buffer_props(key).is_some_and(|props| {
            props.contains(Props::HOST_VISIBLE) && !props.contains(Props::HOST_COHERENT)
        })
    }

    /// Everything that is left, destroyed at the end of the generation.
    ///
    /// # Safety
    /// `&mut self` at the destruction of the generation, with the evidence that nothing it took is still in use.
    pub(super) unsafe fn destroy_all_resources(&mut self) {
        let live = std::mem::take(
            self.resources
                .live
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner),
        );
        self.resources
            .retiring
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        for entry in live.into_values() {
            // SAFETY: the caller's.
            unsafe { self.destroy_objects(entry.objects) };
        }
    }

    /// Leave everything alive with a device that is left alive: nothing may be destroyed, and no block reported.
    pub(super) fn abandon_resources(&mut self) {
        let live = std::mem::take(
            self.resources
                .live
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner),
        );
        std::mem::forget(live);
    }
}
