//! Real GPU work for tests, made from a lease as a client with a binding of its own would make it: fill a device
//! buffer, copy it into host memory and read every word of it. A submission that is called completed before the GPU
//! did the work is found out by what the host reads.
#![allow(unsafe_code)]

use ash::vk::{self, Handle};
use handshake_vulkan_renderer::{
    ChargeClass, Claim, DeviceHandles, DeviceLease, GpuClient, MemoryPlacement, Recording,
    SettleOutcome, SettleReceipt, Settlement,
};

/// Words in each of the two buffers: 4 MiB.
const WORDS: usize = 1 << 20;
const BYTES: u64 = 4 << 20;

pub struct GpuWork {
    device: ash::Device,
    source: vk::Buffer,
    source_memory: vk::DeviceMemory,
    target: vk::Buffer,
    target_memory: vk::DeviceMemory,
    words: *const u32,
    allocated: u64,
}

// SAFETY: the handles are plain integers of a device that allows access from any thread, and the mapped words are
// only read.
unsafe impl Send for GpuWork {}
// SAFETY: as above; nothing is changed through a shared reference.
unsafe impl Sync for GpuWork {}

/// # Safety
/// The handles are those of a live device generation.
unsafe fn instance_of(handles: &DeviceHandles) -> ash::Instance {
    // SAFETY: the address is the loader's `vkGetInstanceProcAddr`, and the instance is live.
    unsafe {
        let get_instance_proc_addr = std::mem::transmute::<*const (), vk::PFN_vkGetInstanceProcAddr>(
            handles.get_instance_proc_addr as *const (),
        );
        let static_fn = ash::StaticFn {
            get_instance_proc_addr,
        };
        ash::Instance::load(&static_fn, vk::Instance::from_raw(handles.instance))
    }
}

/// Size and placement of each memory heap, from what the driver itself answers: a discrete device's own heaps are
/// dedicated, what is not device-local is shared, and of everything else the driver does not say.
pub fn heaps_by_the_driver(handles: &DeviceHandles) -> Vec<(u64, MemoryPlacement)> {
    // SAFETY: the handles are those of a live device generation, kept alive by the caller's lease.
    let (kind, memory) = unsafe {
        let instance = instance_of(handles);
        let physical = vk::PhysicalDevice::from_raw(handles.physical_device);
        (
            instance
                .get_physical_device_properties(physical)
                .device_type,
            instance.get_physical_device_memory_properties(physical),
        )
    };
    memory
        .memory_heaps
        .iter()
        .take(memory.memory_heap_count as usize)
        .map(|heap| {
            let own = heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL);
            let placement = match kind {
                _ if !own => MemoryPlacement::Shared,
                vk::PhysicalDeviceType::DISCRETE_GPU => MemoryPlacement::Dedicated,
                _ => MemoryPlacement::Unknown,
            };
            (heap.size, placement)
        })
        .collect()
}

impl GpuWork {
    pub fn new(handles: &DeviceHandles) -> Self {
        // SAFETY: the handles are those of a live device generation, kept alive by the caller's lease.
        unsafe {
            let instance = instance_of(handles);
            let device =
                ash::Device::load(instance.fp_v1_0(), vk::Device::from_raw(handles.device));
            let types = instance.get_physical_device_memory_properties(
                vk::PhysicalDevice::from_raw(handles.physical_device),
            );
            let (source, source_memory, source_size) = buffer(
                &device,
                &types,
                vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            );
            let (target, target_memory, target_size) = buffer(
                &device,
                &types,
                vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
            let mapped = device
                .map_memory(
                    target_memory,
                    0,
                    vk::WHOLE_SIZE,
                    vk::MemoryMapFlags::empty(),
                )
                .expect("maps");
            std::ptr::write_bytes(mapped.cast::<u8>(), 0, BYTES as usize);
            Self {
                device,
                source,
                source_memory,
                target,
                target_memory,
                words: mapped.cast::<u32>().cast_const(),
                allocated: source_size + target_size,
            }
        }
    }

    /// Device memory of the two buffers.
    pub fn allocated_bytes(&self) -> u64 {
        self.allocated
    }

    /// Record: fill the device buffer with `value` and copy it to where the host reads it.
    pub fn record(&self, recording: &Recording, value: u32) {
        let commands = vk::CommandBuffer::from_raw(recording.command_buffer());
        let filled = [vk::MemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::TRANSFER_READ)];
        // Orders this work after earlier work on the same buffers, and the host read after this work.
        let before = [vk::MemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE | vk::AccessFlags::TRANSFER_READ)
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)];
        let copied = [vk::MemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::HOST_READ)];
        let region = [vk::BufferCopy::default().size(BYTES)];
        let transfer = vk::PipelineStageFlags::TRANSFER;
        let flags = vk::DependencyFlags::empty();
        // SAFETY: the command buffer is in the recording state for the length of the recording call, and the buffers
        // live until the submission has ended (the tests drop `GpuWork` after that).
        unsafe {
            let device = &self.device;
            device.cmd_pipeline_barrier(commands, transfer, transfer, flags, &before, &[], &[]);
            device.cmd_fill_buffer(commands, self.source, 0, vk::WHOLE_SIZE, value);
            device.cmd_pipeline_barrier(commands, transfer, transfer, flags, &filled, &[], &[]);
            device.cmd_copy_buffer(commands, self.source, self.target, &region);
            let host = vk::PipelineStageFlags::HOST;
            device.cmd_pipeline_barrier(commands, transfer, host, flags, &copied, &[], &[]);
        }
    }

    /// Whether every word the host reads is `value`.
    pub fn holds(&self, value: u32) -> bool {
        (0..WORDS).all(|index| {
            // SAFETY: the mapping covers `WORDS` words and lives as long as `self`; a volatile read of a word the
            // device may still be writing yields some value, which is what a premature completion shows.
            unsafe { self.words.add(index).read_volatile() == value }
        })
    }
}

impl Drop for GpuWork {
    fn drop(&mut self) {
        // A failing test may leave its work pending: then the buffers stay.
        if std::thread::panicking() {
            return;
        }
        // SAFETY: the tests drop this after the last submission that uses it has ended, or after its device was lost,
        // and before they drop the lease.
        unsafe {
            self.device.unmap_memory(self.target_memory);
            self.device.destroy_buffer(self.source, None);
            self.device.destroy_buffer(self.target, None);
            self.device.free_memory(self.source_memory, None);
            self.device.free_memory(self.target_memory, None);
        }
    }
}

/// # Safety
/// `device` and `types` are of the same live device.
unsafe fn buffer(
    device: &ash::Device,
    types: &vk::PhysicalDeviceMemoryProperties,
    usage: vk::BufferUsageFlags,
    wanted: vk::MemoryPropertyFlags,
) -> (vk::Buffer, vk::DeviceMemory, u64) {
    let create = vk::BufferCreateInfo::default()
        .size(BYTES)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    // SAFETY: a live device; `create` outlives the call.
    let buffer = unsafe { device.create_buffer(&create, None) }.expect("buffer");
    // SAFETY: the buffer was just created from this device.
    let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
    let index = types
        .memory_types
        .iter()
        .take(types.memory_type_count as usize)
        .enumerate()
        .position(|(index, kind)| {
            requirements.memory_type_bits & (1 << index) != 0
                && kind.property_flags.contains(wanted)
        })
        .expect("Vulkan guarantees such a memory type");
    let allocate = vk::MemoryAllocateInfo::default()
        .allocation_size(requirements.size)
        .memory_type_index(u32::try_from(index).expect("below 32"));
    // SAFETY: the memory type is one of this device and fits the buffer; the buffer is bound once, to memory of the
    // size it asked for.
    let memory = unsafe {
        let memory = device.allocate_memory(&allocate, None).expect("memory");
        device.bind_buffer_memory(buffer, memory, 0).expect("binds");
        memory
    };
    (buffer, memory, requirements.size)
}

/// A claim with a lease and real buffers made from it, all charged.
pub struct Worker {
    // Dropped in this order: the buffers are gone before their lease.
    pub work: GpuWork,
    pub lease: DeviceLease,
    pub claim: Claim,
}

impl Worker {
    /// Bytes a claim needs for the buffers of one worker.
    pub const RESERVE: u64 = 32 << 20;

    pub fn new(client: &GpuClient, claim: Claim) -> Self {
        let lease = client.lease(&claim).expect("authorized");
        let work = GpuWork::new(lease.handles().expect("may use its device"));
        client
            .charge(&claim, ChargeClass::Committed, work.allocated_bytes())
            .expect("reserved");
        Self { work, lease, claim }
    }

    /// Free the buffers, report them freed, and return the lease.
    pub fn let_go(self, client: &GpuClient) -> Claim {
        let bytes = self.work.allocated_bytes();
        drop(self.work);
        client
            .discharge(&self.claim, ChargeClass::Committed, bytes)
            .expect("charged");
        drop(self.lease);
        self.claim
    }

    /// Let go of everything and settle what completed.
    pub fn settle(self, client: &GpuClient) -> SettleReceipt {
        let claim = self.let_go(client);
        client
            .settle(&claim, Settlement::clean(SettleOutcome::Completed))
            .expect("settles")
    }
}
