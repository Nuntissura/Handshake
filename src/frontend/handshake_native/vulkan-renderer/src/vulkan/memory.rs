//! Device memory of the owner's own resources: `gpu-alloc` places blocks, this module makes the Vulkan calls and keeps
//! the books.
//!
//! Why `gpu-alloc` (0.6.2): it is the sub-allocator of wgpu-hal's Vulkan backend, field-tested there, pure Rust with
//! only `bitflags` besides its types crate, and backend-agnostic, so the owner supplies the calls (`MemoryDevice`) as
//! wgpu-hal does and nothing of it reaches the public surface. It selects among the memory types the resource allows by
//! usage (device-local for attachments, host-visible coherent for uploads, host-cached for readback), sub-allocates
//! small requests from power-of-two buddy chunks and gives large ones an object of their own. A block can therefore be
//! larger than its resource: every host access is bounded by the resource's own size, never by the block's
//! (`resources.rs`). `gpu-allocator` was not taken: its 0.27 ignores the configured block sizes (fixed 256 MiB device
//! blocks) and brings proc macros and `log`.
//!
//! What the books say is measured, never estimated: every `vkAllocateMemory` and `vkFreeMemory` of the owner passes
//! here. The driver's own budget and usage (`VK_EXT_memory_budget`) are a separate view ([`crate::MemorySnapshot`]). A
//! new memory object that would take a heap past the driver's budget ends the placement: the resource is refused
//! [`MemoryRefused::OverBudget`] and never placed in another memory type behind the caller's back. Without the
//! extension the budget is unknown and nothing is refused for it.
//!
//! # Locks (MT-203 E3-1)
//!
//! Three, none held across a wait for the device and none while another of them is taken:
//! - the pool: the allocator. Only placing and returning blocks take it; `gpu-alloc` makes its `vkAllocateMemory`,
//!   `vkMapMemory` (of a new host-visible chunk) and `vkFreeMemory` inside those, so it is held across them and is the
//!   narrowest lock that can be. A block returned while it is busy waits in `returned` and is given back by its next
//!   holder, so the owner's look at the device never waits for an allocation.
//! - the books: held for an entry or a copy only.
//! - `returned`: held for a push or a take only.
//!
//! Reading and writing a buffer take none of them: the block is mapped with calls that touch neither allocator nor
//! books, under the buffer's own lock (`resources.rs`).

use super::VulkanGeneration;
use ash::vk::{self, Handle};
use gpu_alloc::{
    AllocationError, AllocationFlags, Config, Dedicated, DeviceMapError, DeviceProperties,
    GpuAllocator, MappedMemoryRange, MemoryBlock, MemoryDevice, MemoryHeap, MemoryPropertyFlags,
    MemoryType, OutOfMemory, Request, UsageFlags,
};
use host::{flush, invalidate, map};
use std::borrow::Cow;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::ptr::NonNull;
use std::sync::{Mutex, MutexGuard, PoisonError, TryLockError};

mod host;

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

impl Books {
    fn allocated(&mut self, memory: vk::DeviceMemory, size: u64, heap: usize) {
        if let Some(book) = self.heaps.get_mut(heap) {
            book.bytes = book.bytes.saturating_add(size);
            book.objects = book.objects.saturating_add(1);
        }
        self.objects.insert(memory.as_raw(), (size, heap));
    }

    fn freed(&mut self, memory: vk::DeviceMemory) {
        if let Some((size, heap)) = self.objects.remove(&memory.as_raw()) {
            if let Some(book) = self.heaps.get_mut(heap) {
                book.bytes = book.bytes.saturating_sub(size);
                book.objects = book.objects.saturating_sub(1);
            }
        }
    }
}

/// The allocator and the rules every placement keeps to.
struct MemoryPool {
    allocator: GpuAllocator<vk::DeviceMemory>,
    /// Heap of each memory type.
    type_heaps: Vec<usize>,
    rules: Rules,
    budget: bool,
}

/// The owner's memory on one generation: see the module documentation for its locks.
pub(crate) struct Memory {
    pool: Mutex<MemoryPool>,
    books: Mutex<Books>,
    returned: Mutex<Vec<Block>>,
}

#[derive(Clone, Copy)]
struct Rules {
    /// Memory types the owner may use.
    usable: u32,
    /// `bufferImageGranularity`: every block starts and ends on such a page, so a buffer and an optimally tiled image
    /// never share one (Vulkan "Buffer-Image Granularity"; `gpu-alloc` does not know the two kinds apart).
    granularity: u64,
    /// `nonCoherentAtomSize`: sizes are whole atoms, so a flush or invalidate of a block's last bytes stays inside its
    /// memory object (VUID-VkMappedMemoryRange-size-01390).
    atom: u64,
}

/// The calls `gpu-alloc` makes when it places and returns blocks, on the generation's device, entered in the books.
struct Calls<'a> {
    generation: &'a VulkanGeneration,
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
        if self.generation.faults.over_budget() {
            return false;
        }
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
            // Ends the search (`gpu-alloc` tries the next memory type only after `OutOfDeviceMemory`): the resource is
            // refused, not moved to another memory type.
            return Err(OutOfMemory::OutOfHostMemory);
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
                self.generation
                    .memory
                    .lock_books()
                    .allocated(memory, size, heap);
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
        self.generation.memory.lock_books().freed(memory);
    }

    unsafe fn map_memory(
        &self,
        memory: &mut vk::DeviceMemory,
        offset: u64,
        size: u64,
    ) -> Result<NonNull<u8>, DeviceMapError> {
        map(&self.generation.device, *memory, offset, size)
    }

    unsafe fn unmap_memory(&self, memory: &mut vk::DeviceMemory) {
        // SAFETY: mapped by `map_memory` above and not unmapped since.
        unsafe { self.generation.device.unmap_memory(*memory) };
    }

    unsafe fn invalidate_memory_ranges(
        &self,
        ranges: &[MappedMemoryRange<'_, vk::DeviceMemory>],
    ) -> Result<(), OutOfMemory> {
        invalidate(&self.generation.device, &self.generation.faults, ranges)
    }

    unsafe fn flush_memory_ranges(
        &self,
        ranges: &[MappedMemoryRange<'_, vk::DeviceMemory>],
    ) -> Result<(), OutOfMemory> {
        flush(&self.generation.device, ranges)
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

impl Memory {
    /// The memory of the device created on `physical`.
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
        let atom = limits.non_coherent_atom_size.max(1).next_power_of_two();
        let properties = DeviceProperties {
            memory_types: Cow::Owned(kinds),
            memory_heaps: Cow::Owned(heaps),
            max_memory_allocation_count: limits.max_memory_allocation_count,
            max_memory_allocation_size: max_allocation.max(1),
            non_coherent_atom_size: atom,
            buffer_device_address: false,
        };
        Self {
            pool: Mutex::new(MemoryPool {
                allocator: GpuAllocator::new(config(), properties),
                type_heaps,
                rules: Rules {
                    usable,
                    granularity: limits.buffer_image_granularity.max(1).next_power_of_two(),
                    atom,
                },
                budget,
            }),
            books: Mutex::new(Books {
                heaps: vec![HeapBook::default(); heap_count],
                objects: BTreeMap::new(),
            }),
            returned: Mutex::new(Vec::new()),
        }
    }

    fn lock_books(&self) -> MutexGuard<'_, Books> {
        self.books.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_returned(&self) -> MutexGuard<'_, Vec<Block>> {
        self.returned.lock().unwrap_or_else(PoisonError::into_inner)
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
    /// Run `call` with the allocator and the calls it may make, then give back what waits in `returned`.
    fn with_pool<T>(
        &self,
        pool: MutexGuard<'_, MemoryPool>,
        dedicated: Option<vk::Image>,
        call: impl FnOnce(&mut GpuAllocator<vk::DeviceMemory>, &Calls<'_>, Rules) -> T,
    ) -> (T, Option<MemoryRefused>) {
        let mut pool = pool;
        let MemoryPool {
            allocator,
            type_heaps,
            rules,
            budget,
        } = &mut *pool;
        let calls = Calls {
            generation: self,
            type_heaps,
            budget: *budget,
            dedicated,
            refused: Cell::new(None),
        };
        let result = call(allocator, &calls, *rules);
        let refused = calls.refused.get();
        let returned = std::mem::take(&mut *self.memory.lock_returned());
        for block in returned {
            // SAFETY: returned by `free_block` under its contract while the pool was busy.
            unsafe { allocator.dealloc(&calls, block) };
        }
        drop(pool);
        (result, refused)
    }

    fn lock_pool(&self) -> MutexGuard<'_, MemoryPool> {
        self.memory
            .pool
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Place memory for one resource. The block is the caller's until it gives it back with [`Self::free_block`].
    pub(crate) fn place(&self, placement: &Placement) -> Result<Block, MemoryRefused> {
        let requirements = placement.requirements;
        let dedicated = placement.image.map(|(image, _)| image);
        let (result, refused) =
            self.with_pool(self.lock_pool(), dedicated, |allocator, calls, rules| {
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
            });
        self.give_back_returned();
        result.map_err(|error| refused_by(error, refused))
    }

    /// Give a block back. When the pool is busy (placing, maybe inside `vkAllocateMemory`) the block waits for its
    /// next holder: the caller does not wait for the driver.
    ///
    /// # Safety
    /// The block was placed by this generation, nothing bound to it exists any more, and no work that used it may
    /// still run.
    pub(crate) unsafe fn free_block(&self, block: Block) {
        self.memory.lock_returned().push(block);
        self.give_back_returned();
    }

    /// Give back what waits in `returned`, when the pool is free now.
    fn give_back_returned(&self) {
        if self.memory.lock_returned().is_empty() {
            return;
        }
        let pool = match self.memory.pool.try_lock() {
            Ok(pool) => pool,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            // Its holder gives them back when it is done.
            Err(TryLockError::WouldBlock) => return,
        };
        let ((), _) = self.with_pool(pool, None, |_, _, _| ());
    }

    /// Leave the blocks that wait in `returned` alive with a device that is left alive, as its resources are.
    pub(super) fn abandon_returned(&mut self) {
        let returned = std::mem::take(
            self.memory
                .returned
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner),
        );
        std::mem::forget(returned);
    }

    /// What each heap holds of the owner's memory objects.
    pub(crate) fn memory_books(&self) -> Vec<HeapBook> {
        self.memory.lock_books().heaps.clone()
    }

    /// The final step of the memory: what waits in `returned` goes back to the allocator, then whatever memory object
    /// is still in the books (nothing, by `gpu-alloc`'s rules) is freed, and the allocator goes with the generation.
    ///
    /// # Safety
    /// `&mut self` at the destruction of the generation, after every resource was destroyed and its block returned.
    pub(super) unsafe fn free_memory_leftovers(&mut self) {
        let ((), _) = self.with_pool(self.lock_pool(), None, |_, _, _| ());
        let books = self
            .memory
            .books
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        for memory in std::mem::take(&mut books.objects).into_keys() {
            // SAFETY: the caller's: nothing uses the memory, and `gpu-alloc` is dropped right after without touching it.
            unsafe {
                self.device
                    .free_memory(vk::DeviceMemory::from_raw(memory), None)
            };
        }
        books
            .heaps
            .iter_mut()
            .for_each(|book| *book = HeapBook::default());
    }
}
