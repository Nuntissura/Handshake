//! Device memory of the owner's own resources: `gpu-alloc` places blocks, this module makes the Vulkan calls and keeps
//! the books.
//!
//! Why `gpu-alloc` (0.6.2): it is the sub-allocator of wgpu-hal's Vulkan backend, field-tested there, pure Rust with
//! only `bitflags` besides its types crate, and backend-agnostic, so the owner supplies the calls (`MemoryDevice`) as
//! wgpu-hal does and nothing of it reaches the public surface. It selects among the memory types the resource allows by
//! usage (device-local for attachments, host-visible coherent for uploads, host-cached for readback), sub-allocates
//! small requests from power-of-two buddy chunks and gives large ones an object of their own. `gpu-allocator` was not
//! taken: its 0.27 ignores the configured block sizes (fixed 256 MiB device blocks) and brings proc macros and `log`.
//!
//! What the books say is measured, never estimated: every `vkAllocateMemory` and `vkFreeMemory` of the owner passes
//! here. The driver's own budget and usage (`VK_EXT_memory_budget`) are a separate view ([`crate::MemorySnapshot`]); a
//! new memory object that would take a heap past the driver's budget is refused. Without the extension the budget is
//! unknown and nothing is refused for it.

use super::VulkanGeneration;
use ash::vk::{self, Handle};
use gpu_alloc::{
    AllocationError, AllocationFlags, Config, Dedicated, DeviceMapError, DeviceProperties,
    GpuAllocator, MappedMemoryRange, MemoryBlock, MemoryDevice, MemoryHeap, MemoryPropertyFlags,
    MemoryType, OutOfMemory, Request, UsageFlags,
};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ptr::NonNull;
use std::sync::{MutexGuard, PoisonError};

/// One block of device memory the owner placed for a resource.
pub(crate) type Block = MemoryBlock<vk::DeviceMemory>;

/// Memory types that need a device feature the owner does not enable (`VK_MEMORY_PROPERTY_DEVICE_COHERENT_BIT_AMD`,
/// `..._DEVICE_UNCACHED_BIT_AMD`, `..._RDMA_CAPABLE_BIT_NV`; VUID-vkAllocateMemory-deviceCoherentMemory-02790) or
/// that are protected or lazily allocated: never used.
const UNUSABLE: vk::MemoryPropertyFlags = vk::MemoryPropertyFlags::from_raw(
    vk::MemoryPropertyFlags::DEVICE_COHERENT_AMD.as_raw()
        | vk::MemoryPropertyFlags::DEVICE_UNCACHED_AMD.as_raw()
        | vk::MemoryPropertyFlags::RDMA_CAPABLE_NV.as_raw()
        | vk::MemoryPropertyFlags::PROTECTED.as_raw()
        | vk::MemoryPropertyFlags::LAZILY_ALLOCATED.as_raw(),
);

/// Why no memory was placed. Closed codes, no driver text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MemoryRefused {
    /// A new memory object would take heap `heap` past the driver's budget (`VK_EXT_memory_budget`).
    OverBudget {
        heap: u32,
    },
    OutOfDeviceMemory,
    OutOfHostMemory,
    /// No memory type the resource allows serves its use.
    NoMemoryType,
    /// The device's limit on memory objects is reached.
    TooManyObjects,
    Driver(i32),
}

/// What one heap holds of the owner's memory objects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct HeapBook {
    pub(crate) bytes: u64,
    pub(crate) objects: u64,
}

#[derive(Default)]
struct Books {
    heaps: Vec<HeapBook>,
    /// Live memory objects: size and heap.
    objects: BTreeMap<u64, (u64, usize)>,
}

/// The allocator and its books. Behind the generation's memory lock, which is held for placing and returning blocks
/// and for nothing that waits for the device.
pub(crate) struct MemoryPool {
    allocator: GpuAllocator<vk::DeviceMemory>,
    books: Books,
    /// Heap of each memory type.
    type_heaps: Vec<usize>,
    /// Memory types the owner may use.
    usable: u32,
    /// `bufferImageGranularity`: every block starts and ends on such a page, so a buffer and an optimally tiled image
    /// never share one (Vulkan "Buffer-Image Granularity"; `gpu-alloc` does not know the two kinds apart).
    granularity: u64,
    /// `nonCoherentAtomSize`: sizes are whole atoms, so a flush or invalidate of a block's last bytes stays inside its
    /// memory object (VUID-VkMappedMemoryRange-size-01390).
    atom: u64,
    budget: bool,
}

/// The rules every placement keeps to.
#[derive(Clone, Copy)]
struct Rules {
    usable: u32,
    granularity: u64,
    atom: u64,
}

/// The calls `gpu-alloc` makes, on the generation's device, entered in the books.
struct Calls<'a> {
    generation: &'a VulkanGeneration,
    books: RefCell<&'a mut Books>,
    type_heaps: &'a [usize],
    budget: bool,
    /// Allocate the next memory object for this image only (VkMemoryDedicatedAllocateInfo).
    dedicated: Option<vk::Image>,
    /// Why the last allocation was refused, beyond what `gpu-alloc` can say.
    refused: Cell<Option<MemoryRefused>>,
}

impl Calls<'_> {
    /// Whether `size` more bytes on `heap` stay within the driver's budget. Always, without the extension.
    fn within_budget(&self, heap: usize, size: u64) -> bool {
        if !self.budget {
            return true;
        }
        let mut budget = vk::PhysicalDeviceMemoryBudgetPropertiesEXT::default();
        {
            let mut properties =
                vk::PhysicalDeviceMemoryProperties2::default().push_next(&mut budget);
            // SAFETY: core 1.1 query on a 1.3 device with VK_EXT_memory_budget enabled; the chain lives for the call.
            unsafe {
                self.generation
                    .instance
                    .get_physical_device_memory_properties2(
                        self.generation.physical,
                        &mut properties,
                    )
            };
        }
        let usage = budget.heap_usage.get(heap).copied().unwrap_or(0);
        let limit = budget.heap_budget.get(heap).copied().unwrap_or(0);
        usage.saturating_add(size) <= limit
    }
}

impl MemoryDevice<vk::DeviceMemory> for Calls<'_> {
    unsafe fn allocate_memory(
        &self,
        size: u64,
        memory_type: u32,
        _flags: AllocationFlags,
    ) -> Result<vk::DeviceMemory, OutOfMemory> {
        let heap = self
            .type_heaps
            .get(memory_type as usize)
            .copied()
            .unwrap_or(0);
        if !self.within_budget(heap, size) {
            self.refused.set(Some(MemoryRefused::OverBudget {
                heap: u32::try_from(heap).unwrap_or(u32::MAX),
            }));
            // Lets `gpu-alloc` try the next memory type that serves the use.
            return Err(OutOfMemory::OutOfDeviceMemory);
        }
        let mut dedicated = vk::MemoryDedicatedAllocateInfo::default();
        let mut info = vk::MemoryAllocateInfo::default()
            .allocation_size(size)
            .memory_type_index(memory_type);
        if let Some(image) = self.dedicated {
            dedicated = dedicated.image(image);
            info = info.push_next(&mut dedicated);
        }
        // SAFETY: valid device; the memory type is one of its own and not of the unusable kinds (`usable` mask); a
        // dedicated image is unbound and `size` is at least its required size (VUID-VkMemoryDedicatedAllocateInfo-image-02964);
        // the chain lives for the call.
        match unsafe { self.generation.device.allocate_memory(&info, None) } {
            Ok(memory) => {
                let mut books = self.books.borrow_mut();
                if let Some(book) = books.heaps.get_mut(heap) {
                    book.bytes = book.bytes.saturating_add(size);
                    book.objects = book.objects.saturating_add(1);
                }
                books.objects.insert(memory.as_raw(), (size, heap));
                Ok(memory)
            }
            Err(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY) => Err(OutOfMemory::OutOfDeviceMemory),
            Err(vk::Result::ERROR_OUT_OF_HOST_MEMORY) => Err(OutOfMemory::OutOfHostMemory),
            Err(vk::Result::ERROR_TOO_MANY_OBJECTS) => {
                self.refused.set(Some(MemoryRefused::TooManyObjects));
                Err(OutOfMemory::OutOfHostMemory)
            }
            Err(other) => {
                self.refused
                    .set(Some(MemoryRefused::Driver(other.as_raw())));
                // Ends the search: another memory type would not answer differently.
                Err(OutOfMemory::OutOfHostMemory)
            }
        }
    }

    unsafe fn deallocate_memory(&self, memory: vk::DeviceMemory) {
        // SAFETY: `gpu-alloc` returns a memory object once, after its last block was returned, and the owner returns a
        // block only when nothing that may use it runs any more. A mapped object is unmapped by being freed.
        unsafe { self.generation.device.free_memory(memory, None) };
        let mut books = self.books.borrow_mut();
        if let Some((size, heap)) = books.objects.remove(&memory.as_raw()) {
            if let Some(book) = books.heaps.get_mut(heap) {
                book.bytes = book.bytes.saturating_sub(size);
                book.objects = book.objects.saturating_sub(1);
            }
        }
    }

    unsafe fn map_memory(
        &self,
        memory: &mut vk::DeviceMemory,
        offset: u64,
        size: u64,
    ) -> Result<NonNull<u8>, DeviceMapError> {
        // SAFETY: host-visible memory of this device, not mapped now (`gpu-alloc` maps each object once), and the range
        // lies inside it.
        let mapped = unsafe {
            self.generation
                .device
                .map_memory(*memory, offset, size, vk::MemoryMapFlags::empty())
        };
        match mapped {
            Ok(pointer) => NonNull::new(pointer.cast::<u8>()).ok_or(DeviceMapError::MapFailed),
            Err(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY) => Err(DeviceMapError::OutOfDeviceMemory),
            Err(vk::Result::ERROR_OUT_OF_HOST_MEMORY) => Err(DeviceMapError::OutOfHostMemory),
            Err(_) => Err(DeviceMapError::MapFailed),
        }
    }

    unsafe fn unmap_memory(&self, memory: &mut vk::DeviceMemory) {
        // SAFETY: mapped by `map_memory` above and not unmapped since.
        unsafe { self.generation.device.unmap_memory(*memory) };
    }

    unsafe fn invalidate_memory_ranges(
        &self,
        ranges: &[MappedMemoryRange<'_, vk::DeviceMemory>],
    ) -> Result<(), OutOfMemory> {
        let ranges = vk_ranges(ranges);
        // SAFETY: mapped ranges of this device's memory, aligned to `nonCoherentAtomSize` by `gpu-alloc`.
        unsafe {
            self.generation
                .device
                .invalidate_mapped_memory_ranges(&ranges)
        }
        .map_err(out_of_memory)
    }

    unsafe fn flush_memory_ranges(
        &self,
        ranges: &[MappedMemoryRange<'_, vk::DeviceMemory>],
    ) -> Result<(), OutOfMemory> {
        let ranges = vk_ranges(ranges);
        // SAFETY: as for `invalidate_memory_ranges`.
        unsafe { self.generation.device.flush_mapped_memory_ranges(&ranges) }.map_err(out_of_memory)
    }
}

fn vk_ranges<'a>(
    ranges: &[MappedMemoryRange<'_, vk::DeviceMemory>],
) -> Vec<vk::MappedMemoryRange<'a>> {
    ranges
        .iter()
        .map(|range| {
            vk::MappedMemoryRange::default()
                .memory(*range.memory)
                .offset(range.offset)
                .size(range.size)
        })
        .collect()
}

fn out_of_memory(result: vk::Result) -> OutOfMemory {
    match result {
        vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => OutOfMemory::OutOfDeviceMemory,
        _ => OutOfMemory::OutOfHostMemory,
    }
}

/// Placement rules: objects of their own at 4 MiB (and for images at 256 KiB, where the driver prefers it), buddy
/// chunks of 4 MiB below. What a chunk holds beyond its blocks is the owner's and never charged to a claim.
fn config() -> Config {
    Config {
        dedicated_threshold: 4 << 20,
        preferred_dedicated_threshold: 256 << 10,
        transient_dedicated_threshold: 4 << 20,
        starting_free_list_chunk: 4 << 20,
        final_free_list_chunk: 4 << 20,
        minimal_buddy_size: 256,
        initial_buddy_dedicated_size: 4 << 20,
    }
}

impl MemoryPool {
    /// The pool of the device created on `physical`.
    pub(super) fn new(
        instance: &ash::Instance,
        physical: vk::PhysicalDevice,
        budget: bool,
    ) -> Self {
        // SAFETY: `physical` was enumerated from `instance`; read-only queries of a 1.3 device.
        let (memory, limits, max_allocation) = unsafe {
            let memory = instance.get_physical_device_memory_properties(physical);
            let mut vulkan11 = vk::PhysicalDeviceVulkan11Properties::default();
            let limits = {
                let mut properties =
                    vk::PhysicalDeviceProperties2::default().push_next(&mut vulkan11);
                instance.get_physical_device_properties2(physical, &mut properties);
                properties.properties.limits
            };
            (memory, limits, vulkan11.max_memory_allocation_size)
        };
        let types = memory
            .memory_types
            .iter()
            .take((memory.memory_type_count as usize).min(vk::MAX_MEMORY_TYPES));
        let heap_count = (memory.memory_heap_count as usize).min(vk::MAX_MEMORY_HEAPS);
        let mut usable = 0u32;
        let mut kinds = Vec::new();
        let mut type_heaps = Vec::new();
        for (index, kind) in types.enumerate() {
            if !kind.property_flags.intersects(UNUSABLE) {
                usable |= 1 << index;
            }
            kinds.push(MemoryType {
                heap: kind.heap_index,
                // The six flags `gpu-alloc` knows have the bit values of Vulkan's.
                props: MemoryPropertyFlags::from_bits_truncate(
                    u8::try_from(kind.property_flags.as_raw() & 0x3f).unwrap_or(0),
                ),
            });
            type_heaps.push(kind.heap_index as usize);
        }
        let heaps: Vec<MemoryHeap> = memory
            .memory_heaps
            .iter()
            .take(heap_count)
            .map(|heap| MemoryHeap { size: heap.size })
            .collect();
        let properties = DeviceProperties {
            memory_types: Cow::Owned(kinds),
            memory_heaps: Cow::Owned(heaps),
            max_memory_allocation_count: limits.max_memory_allocation_count,
            max_memory_allocation_size: max_allocation.max(1),
            non_coherent_atom_size: limits.non_coherent_atom_size.max(1).next_power_of_two(),
            buffer_device_address: false,
        };
        Self {
            allocator: GpuAllocator::new(config(), properties),
            books: Books {
                heaps: vec![HeapBook::default(); heap_count],
                objects: BTreeMap::new(),
            },
            type_heaps,
            usable,
            granularity: limits.buffer_image_granularity.max(1).next_power_of_two(),
            atom: limits.non_coherent_atom_size.max(1).next_power_of_two(),
            budget,
        }
    }

    /// What each heap holds of the owner's memory objects, by heap index.
    pub(crate) fn books(&self) -> Vec<HeapBook> {
        self.books.heaps.clone()
    }
}

/// How a resource uses its memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Use {
    /// Read and written by the device only.
    Device,
    /// Written by the host, read by the device.
    Upload,
    /// Written by the device, read by the host.
    Readback,
}

impl Use {
    fn flags(self) -> UsageFlags {
        match self {
            Self::Device => UsageFlags::FAST_DEVICE_ACCESS,
            Self::Upload => UsageFlags::UPLOAD | UsageFlags::HOST_ACCESS,
            Self::Readback => UsageFlags::DOWNLOAD | UsageFlags::HOST_ACCESS,
        }
    }
}

/// One placement request: the resource's own memory requirements and its use.
pub(crate) struct Placement {
    pub(crate) requirements: vk::MemoryRequirements,
    pub(crate) usage: Use,
    /// The image the memory is for, when it must or should have an object of its own.
    pub(crate) image: Option<(vk::Image, Dedicated)>,
}

fn refused_by(error: AllocationError, refused: Option<MemoryRefused>) -> MemoryRefused {
    match (refused, error) {
        (Some(refused), _) => refused,
        (None, AllocationError::OutOfDeviceMemory) => MemoryRefused::OutOfDeviceMemory,
        (None, AllocationError::OutOfHostMemory) => MemoryRefused::OutOfHostMemory,
        (None, AllocationError::NoCompatibleMemoryTypes) => MemoryRefused::NoMemoryType,
        (None, AllocationError::TooManyObjects) => MemoryRefused::TooManyObjects,
    }
}

impl VulkanGeneration {
    fn lock_memory(&self) -> MutexGuard<'_, MemoryPool> {
        self.memory.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Run `call` with the allocator and the calls it may make.
    fn with_pool<T>(
        &self,
        dedicated: Option<vk::Image>,
        call: impl FnOnce(&mut GpuAllocator<vk::DeviceMemory>, &Calls<'_>, Rules) -> T,
    ) -> (T, Option<MemoryRefused>) {
        let mut pool = self.lock_memory();
        let MemoryPool {
            allocator,
            books,
            type_heaps,
            usable,
            granularity,
            atom,
            budget,
        } = &mut *pool;
        let rules = Rules {
            usable: *usable,
            granularity: *granularity,
            atom: *atom,
        };
        let calls = Calls {
            generation: self,
            books: RefCell::new(books),
            type_heaps,
            budget: *budget,
            dedicated,
            refused: Cell::new(None),
        };
        let result = call(allocator, &calls, rules);
        (result, calls.refused.get())
    }

    /// Place memory for one resource. The block is the caller's until it gives it back with [`Self::free_block`].
    pub(crate) fn place(&self, placement: &Placement) -> Result<Block, MemoryRefused> {
        let requirements = placement.requirements;
        let (result, refused) = self.with_pool(
            placement.image.map(|(image, _)| image),
            |allocator, calls, rules| {
                let alignment = requirements.alignment.max(1);
                if !alignment.is_power_of_two() {
                    return Err(AllocationError::NoCompatibleMemoryTypes);
                }
                // At least the required size (VUID-VkMemoryDedicatedAllocateInfo-image-02964 allows more).
                let size = requirements
                    .size
                    .checked_next_multiple_of(rules.atom)
                    .ok_or(AllocationError::OutOfDeviceMemory)?;
                let request = Request {
                    size,
                    // Buddy blocks are powers of two at least this large, so whole granularity pages.
                    align_mask: alignment.max(rules.granularity) - 1,
                    usage: placement.usage.flags(),
                    memory_types: requirements.memory_type_bits & rules.usable,
                };
                if request.memory_types == 0 {
                    return Err(AllocationError::NoCompatibleMemoryTypes);
                }
                // SAFETY: the calls are those of this generation's device, and every block placed here is given back to
                // this allocator only (`free_block`, also at the end of the generation).
                unsafe {
                    match placement.image {
                        Some((_, dedicated)) => {
                            allocator.alloc_with_dedicated(calls, request, dedicated)
                        }
                        None => allocator.alloc(calls, request),
                    }
                }
            },
        );
        result.map_err(|error| refused_by(error, refused))
    }

    /// Give a block back.
    ///
    /// # Safety
    /// The block was placed by this generation, nothing bound to it exists any more, and no work that used it may
    /// still run.
    pub(crate) unsafe fn free_block(&self, block: Block) {
        let ((), _) = self.with_pool(None, |allocator, calls, _| {
            // SAFETY: the caller's.
            unsafe { allocator.dealloc(calls, block) }
        });
    }

    /// Read `out.len()` bytes at `offset` of a host-visible block, after making the device's writes visible to the host
    /// (invalidate on memory that is not coherent).
    ///
    /// # Safety
    /// No work that may write the block runs now, and the device's writes were made available to the host by a memory
    /// dependency with destination `VK_ACCESS_HOST_READ_BIT` before the fence that was observed signalled.
    pub(crate) unsafe fn read_block(&self, block: &mut Block, offset: u64, out: &mut [u8]) -> bool {
        let (read, _) = self.with_pool(None, |_, calls, _| {
            // SAFETY: the caller's; the block is this generation's and `offset + len` lies in it (checked by the
            // caller).
            unsafe { block.read_bytes(calls, offset, out) }.is_ok()
        });
        read
    }

    /// Write `data` at `offset` of a host-visible block, flushing memory that is not coherent.
    ///
    /// # Safety
    /// No work that may use the block runs now.
    pub(crate) unsafe fn write_block(&self, block: &mut Block, offset: u64, data: &[u8]) -> bool {
        let (written, _) = self.with_pool(None, |_, calls, _| {
            // SAFETY: as in `read_block`.
            unsafe { block.write_bytes(calls, offset, data) }.is_ok()
        });
        written
    }

    /// What each heap holds of the owner's memory objects.
    pub(crate) fn memory_books(&self) -> Vec<HeapBook> {
        self.lock_memory().books()
    }

    /// The final step of the memory: every block is back, so this frees only what `gpu-alloc` still keeps (nothing,
    /// by its rules), and then the allocator itself goes.
    ///
    /// # Safety
    /// `&mut self` at the destruction of the generation, after every resource was destroyed and its block returned.
    pub(super) unsafe fn free_memory_leftovers(&mut self) {
        let pool = self
            .memory
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        for memory in std::mem::take(&mut pool.books.objects).into_keys() {
            // SAFETY: the caller's: nothing uses the memory, and `gpu-alloc` is dropped right after without touching it.
            unsafe {
                self.device
                    .free_memory(vk::DeviceMemory::from_raw(memory), None)
            };
        }
        pool.books
            .heaps
            .iter_mut()
            .for_each(|book| *book = HeapBook::default());
    }
}
