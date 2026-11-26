//! The host's side of owner memory: mapping, flushing and invalidating, and the reads and writes of a buffer's block.
//! None of it touches the allocator or the books, so a host access takes no lock of the memory; it runs under the
//! buffer's own lock (`resources.rs`).

use super::super::fault::Faults;
use super::super::VulkanGeneration;
use super::Block;
use ash::vk;
use gpu_alloc::{AllocationFlags, DeviceMapError, MappedMemoryRange, MemoryDevice, OutOfMemory};
use std::ptr::NonNull;

/// `vkMapMemory`.
pub(super) fn map(
    device: &ash::Device,
    memory: vk::DeviceMemory,
    offset: u64,
    size: u64,
) -> Result<NonNull<u8>, DeviceMapError> {
    // SAFETY: host-visible memory of this device, not mapped now (`gpu-alloc` maps a chunk once, at its allocation, and
    // a dedicated block only while it is accessed under its buffer's lock), and the range lies inside it.
    let mapped = unsafe { device.map_memory(memory, offset, size, vk::MemoryMapFlags::empty()) };
    match mapped {
        Ok(pointer) => NonNull::new(pointer.cast::<u8>()).ok_or(DeviceMapError::MapFailed),
        Err(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY) => Err(DeviceMapError::OutOfDeviceMemory),
        Err(vk::Result::ERROR_OUT_OF_HOST_MEMORY) => Err(DeviceMapError::OutOfHostMemory),
        Err(_) => Err(DeviceMapError::MapFailed),
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

/// `vkInvalidateMappedMemoryRanges`.
pub(super) fn invalidate(
    device: &ash::Device,
    faults: &Faults,
    ranges: &[MappedMemoryRange<'_, vk::DeviceMemory>],
) -> Result<(), OutOfMemory> {
    faults.invalidated();
    let ranges = vk_ranges(ranges);
    // SAFETY: mapped ranges of this device's memory, aligned to `nonCoherentAtomSize` by `gpu-alloc`.
    unsafe { device.invalidate_mapped_memory_ranges(&ranges) }.map_err(out_of_memory)
}

/// `vkFlushMappedMemoryRanges`.
pub(super) fn flush(
    device: &ash::Device,
    ranges: &[MappedMemoryRange<'_, vk::DeviceMemory>],
) -> Result<(), OutOfMemory> {
    let ranges = vk_ranges(ranges);
    // SAFETY: as for `invalidate`.
    unsafe { device.flush_mapped_memory_ranges(&ranges) }.map_err(out_of_memory)
}

/// The calls of a host access to one block: map, unmap, flush and invalidate. `MemoryBlock::read_bytes` and
/// `write_bytes` make no other (gpu-alloc 0.6.2 `block.rs`), so allocation is refused and freeing does nothing.
struct HostAccess<'a> {
    device: &'a ash::Device,
    faults: &'a Faults,
}

impl MemoryDevice<vk::DeviceMemory> for HostAccess<'_> {
    unsafe fn allocate_memory(
        &self,
        _size: u64,
        _memory_type: u32,
        _flags: AllocationFlags,
    ) -> Result<vk::DeviceMemory, OutOfMemory> {
        Err(OutOfMemory::OutOfHostMemory)
    }

    unsafe fn deallocate_memory(&self, _memory: vk::DeviceMemory) {}

    unsafe fn map_memory(
        &self,
        memory: &mut vk::DeviceMemory,
        offset: u64,
        size: u64,
    ) -> Result<NonNull<u8>, DeviceMapError> {
        map(self.device, *memory, offset, size)
    }

    unsafe fn unmap_memory(&self, memory: &mut vk::DeviceMemory) {
        // SAFETY: mapped by `map_memory` above and not unmapped since.
        unsafe { self.device.unmap_memory(*memory) };
    }

    unsafe fn invalidate_memory_ranges(
        &self,
        ranges: &[MappedMemoryRange<'_, vk::DeviceMemory>],
    ) -> Result<(), OutOfMemory> {
        invalidate(self.device, self.faults, ranges)
    }

    unsafe fn flush_memory_ranges(
        &self,
        ranges: &[MappedMemoryRange<'_, vk::DeviceMemory>],
    ) -> Result<(), OutOfMemory> {
        flush(self.device, ranges)
    }
}

impl VulkanGeneration {
    fn host_access(&self) -> HostAccess<'_> {
        HostAccess {
            device: &self.device,
            faults: &self.faults,
        }
    }

    /// Read `out.len()` bytes at `offset` of a host-visible block, after making the device's writes visible to the host
    /// (invalidate on memory that is not coherent). Takes no lock of the memory.
    ///
    /// # Safety
    /// No work that may write the block runs now, the caller holds the block's own lock, and the device's writes were
    /// made available to the host by a memory dependency with destination `VK_ACCESS_HOST_READ_BIT` before the fence
    /// that was observed signalled.
    pub(crate) unsafe fn read_block(&self, block: &mut Block, offset: u64, out: &mut [u8]) -> bool {
        // SAFETY: the caller's; the block is this generation's and `offset + len` lies in it (checked by the caller).
        unsafe { block.read_bytes(&self.host_access(), offset, out) }.is_ok()
    }

    /// Write `data` at `offset` of a host-visible block, flushing memory that is not coherent. Takes no lock of the
    /// memory.
    ///
    /// # Safety
    /// No work that may use the block runs now, and the caller holds the block's own lock.
    pub(crate) unsafe fn write_block(&self, block: &mut Block, offset: u64, data: &[u8]) -> bool {
        // SAFETY: as in `read_block`.
        unsafe { block.write_bytes(&self.host_access(), offset, data) }.is_ok()
    }
}
