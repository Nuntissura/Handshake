//! The one module that holds Vulkan `unsafe`: a single generation of instance, physical device, logical
//! device and queue, through `ash`. Nothing here is public outside the crate and no `ash` type leaves it.
//! Every `unsafe` block names the rule it relies on.
//!
//! The generation's own child objects are its sentinel fence, the fences and command pools of submitted work, the
//! images, buffers and memory of its resources (`resources`, `memory`) and, when it was opened with presentation, the
//! surfaces, swapchains and their semaphores and fences (`present`). Clients may create further child objects through a
//! [`crate::DeviceLease`], which keeps the generation alive until they returned it.
//!
//! Nothing here waits inside Vulkan for the device (see `queue`). The calls without a bound of their own are the
//! creation of an instance and a device, which runs with no lock held, their destruction, which is reached only with the
//! evidence that nothing is left in use, and `vkQueueSubmit` and `vkQueuePresentKHR`, which hold only the queue's lane
//! (and a present its own surface's lock).
#![allow(unsafe_code)]

mod backoff;
pub(crate) mod fault;
mod loader;
mod memory;
mod open;
mod present;
mod queue;
mod record;
mod resources;

pub(crate) use backoff::Backoff;
pub(crate) use memory::{HeapBook, MemoryRefused};
#[cfg(test)]
pub(crate) use present::Retirement;
pub(crate) use present::{FrameRef, PresentEnd, PresentFault, Shown, SurfaceFault, Targets};
pub(crate) use queue::{QueueFault, RecordedWork, SubmitFault, SubmitSync};
pub(crate) use record::ImageCommand;
pub(crate) use resources::{BufferSpec, ImageSpec, Owned, ResourceFault};

use crate::device::{
    DeviceHandles, DeviceIdentity, HeapSample, MemoryBudgetSource, MemoryPlacement, MemorySnapshot,
};
use crate::ids::DeviceKey;
use crate::outcome::Unavailable;
use crate::process::ProcessClaim;
use crate::records::ByteQuantity;
use crate::select::{self, BASELINE_API_VERSION};
use ash::vk::{self, Handle};
use fault::Faults;
use loader::LoaderLibrary;
use memory::Memory;
use present::{PresentFns, Surfaces};
use queue::{Lane, QueueState, Quiescent};
use resources::Resources;
use std::sync::{Arc, Mutex};

const _: () = assert!(BASELINE_API_VERSION == vk::API_VERSION_1_3);
const _: () = assert!(select::DRIVER_ID_SWIFTSHADER == vk::DriverId::GOOGLE_SWIFTSHADER.as_raw());
const _: () = assert!(select::DRIVER_ID_LLVMPIPE == vk::DriverId::MESA_LLVMPIPE.as_raw());
const _: () = assert!(select::DRIVER_ID_DOZEN == vk::DriverId::MESA_DOZEN.as_raw());

#[derive(Clone, Copy, PartialEq, Eq)]
enum End {
    /// The Vulkan objects exist and are used.
    Not,
    Destroyed,
    /// Deliberately left alive.
    Left,
}

pub(crate) struct VulkanGeneration {
    entry: ash::Entry,
    instance: ash::Instance,
    physical: vk::PhysicalDevice,
    device: ash::Device,
    /// Nothing is ever submitted with it, so it is signalled on no healthy device.
    sentinel: vk::Fence,
    /// What the queue was given and what of it may still run.
    state: Mutex<QueueState>,
    /// The queue itself: the external synchronization of `vkQueueSubmit` and `vkQueuePresentKHR`.
    lane: Mutex<Lane>,
    /// The allocator of the owner's resources and its books, each behind its own lock (`memory`).
    memory: Memory,
    resources: Resources,
    /// Present only when the generation was opened with presentation.
    presentation: Option<PresentFns>,
    /// One lock per surface (`present`).
    surfaces: Surfaces,
    /// Windows' targets of the surfaces destroyed with the generation, for the owner to drop with no lock held.
    buried: Targets,
    identity: DeviceIdentity,
    memory_budget: bool,
    end: End,
    faults: Faults,
    /// Keeps the Vulkan loader loaded. Dropped after `Drop::drop` destroyed device and instance.
    library: LoaderLibrary,
    /// The process's one owner slot stays taken while this device exists. Dropped last.
    process: Arc<ProcessClaim>,
}

impl std::fmt::Debug for VulkanGeneration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VulkanGeneration")
            .field("key", &self.identity.key)
            .finish_non_exhaustive()
    }
}

impl VulkanGeneration {
    /// Open a generation. With `required`, only that adapter is acceptable; with `presentation`, the surface and
    /// swapchain extensions are enabled and a device whose queue cannot present is refused. The generation keeps
    /// `process` until its device is gone.
    pub(crate) fn open(
        process: Arc<ProcessClaim>,
        required: Option<DeviceKey>,
        presentation: bool,
    ) -> Result<Self, Unavailable> {
        fault::opening()?;
        let (entry, library) = loader::load()?;
        // SAFETY: valid entry; `vkEnumerateInstanceVersion` has no preconditions (absent on 1.0 loaders,
        // which ash reports as `None`).
        let loader_version = match unsafe { entry.try_enumerate_instance_version() } {
            Ok(Some(version)) => version,
            Ok(None) => vk::API_VERSION_1_0,
            Err(result) => return Err(open::startup_error(result, Unavailable::InstanceRefused)),
        };
        if !select::meets_baseline(loader_version) {
            return Err(Unavailable::ApiBelowBaseline);
        }
        let (instance, features) = open::create_instance(&entry, presentation)?;
        match open::open_device(&entry, &instance, features, required) {
            Ok(parts) => {
                let presentation = features.presentation.then(|| {
                    PresentFns::new(&entry, &instance, &parts.device, parts.present_fences)
                });
                let memory = Memory::new(&instance, parts.physical, parts.memory_budget);
                Ok(Self {
                    entry,
                    instance,
                    physical: parts.physical,
                    device: parts.device,
                    sentinel: parts.sentinel,
                    state: Mutex::new(QueueState::new()),
                    lane: Mutex::new(Lane { queue: parts.queue }),
                    memory,
                    resources: Resources::default(),
                    presentation,
                    surfaces: Surfaces::default(),
                    buried: Targets::new(),
                    identity: parts.identity,
                    memory_budget: parts.memory_budget,
                    end: End::Not,
                    faults: Faults::default(),
                    library,
                    process,
                })
            }
            Err(reason) => {
                // SAFETY: on `Err`, `open_device` left no logical device behind, so the instance has no
                // child object left.
                unsafe { instance.destroy_instance(None) };
                Err(reason)
            }
        }
    }

    pub(crate) fn identity(&self) -> &DeviceIdentity {
        &self.identity
    }

    /// Resources of the owner that exist on this generation.
    pub(crate) fn resource_count(&self) -> usize {
        self.resources.count()
    }

    /// What each heap holds of the owner's own memory objects.
    pub(crate) fn owner_memory(&self) -> Vec<HeapBook> {
        self.memory_books()
    }

    /// The device's own memory view for `generation`.
    pub(crate) fn memory(&self, generation: u64) -> MemorySnapshot {
        let mut budget = vk::PhysicalDeviceMemoryBudgetPropertiesEXT::default();
        let memory = if self.memory_budget {
            let mut properties2 =
                vk::PhysicalDeviceMemoryProperties2::default().push_next(&mut budget);
            // SAFETY: core 1.1 query on a 1.3 device; VK_EXT_memory_budget is enabled on the device, so
            // the budget struct may be chained; it lives for the call.
            unsafe {
                self.instance
                    .get_physical_device_memory_properties2(self.physical, &mut properties2)
            };
            properties2.memory_properties
        } else {
            // SAFETY: valid physical device; Vulkan 1.0 read-only query.
            unsafe {
                self.instance
                    .get_physical_device_memory_properties(self.physical)
            }
        };
        let count = (memory.memory_heap_count as usize).min(memory.memory_heaps.len());
        let mut heaps = Vec::with_capacity(count);
        for (index, heap) in memory.memory_heaps.iter().take(count).enumerate() {
            let sample = |values: &[u64; vk::MAX_MEMORY_HEAPS]| match values.get(index) {
                Some(bytes) if self.memory_budget => ByteQuantity::Estimated(*bytes),
                _ => ByteQuantity::Unknown,
            };
            let device_local = heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL);
            heaps.push(HeapSample {
                index: u32::try_from(index).unwrap_or(u32::MAX),
                size_bytes: heap.size,
                device_local,
                placement: MemoryPlacement::of(self.identity.class, device_local),
                budget: sample(&budget.heap_budget),
                usage: sample(&budget.heap_usage),
            });
        }
        let source = if self.memory_budget {
            MemoryBudgetSource::DriverEstimate
        } else {
            MemoryBudgetSource::Unavailable
        };
        MemorySnapshot {
            generation,
            source,
            heaps,
        }
    }

    /// Raw hand-over data for `generation`. Handles are plain integers; no binding type crosses.
    pub(crate) fn handles(&self, generation: u64) -> DeviceHandles {
        DeviceHandles {
            get_instance_proc_addr: self.entry.static_fn().get_instance_proc_addr as usize,
            instance: self.instance.handle().as_raw(),
            physical_device: self.physical.as_raw(),
            device: self.device.handle().as_raw(),
            queue_family: self.identity.queue_family,
            api_version: BASELINE_API_VERSION,
            extensions: self.identity.extensions.clone(),
            features: self.identity.features.clone(),
            generation,
        }
    }

    /// Destroy the generation when its device confirms, without being waited for, that its work has ended. `false`
    /// when it does not: then nothing is destroyed, because objects that may be in use never are. Every object stays
    /// alive, the loader loaded and the process's owner slot taken. Returns the targets of the windows whose surfaces
    /// were destroyed, for the caller to drop once it holds no lock.
    pub(crate) fn dispose(mut self) -> (bool, Targets) {
        self.end_or_leave();
        (self.end == End::Destroyed, std::mem::take(&mut self.buried))
    }

    fn end_or_leave(&mut self) {
        if self.end != End::Not {
            return;
        }
        self.seal();
        // The result is in the drained mark.
        self.confirm();
        match self.evidence() {
            Some(evidence) => self.end(evidence),
            None => {
                self.end = End::Left;
                self.abandon_resources();
                self.abandon_returned();
                self.abandon_presentation();
                std::mem::forget(Arc::clone(&self.library));
                std::mem::forget(Arc::clone(&self.process));
            }
        }
    }

    fn end(&mut self, _evidence: Quiescent) {
        if self.end != End::Not {
            return;
        }
        #[cfg(test)]
        self.let_it_end();
        self.end = End::Destroyed;
        let objects = self.take_objects();
        // SAFETY: `&mut self` in `dispose` or `drop`: no lease, recording or wait holds this generation any more
        // (each owns an `Arc` of it), so every client object made from a lease was destroyed first (the rule of
        // `DeviceLease`), and this thread alone uses the device and its queue. `_evidence` says that the queue takes
        // nothing more and that nothing it took is still in use by the device, so the objects of its work, of the
        // owner's resources and of its presentation may be destroyed, and the sentinel was never submitted with; a lost
        // device must still be destroyed (Vulkan "Lost Device"). Children first (swapchains, views, images, buffers,
        // memory, semaphores, fences), then the device, then the instance after its surfaces; the loader (`library`) is
        // released after this body (VUID-vkDestroySwapchainKHR-swapchain-01282, VUID-vkFreeMemory-memory-00677,
        // VUID-vkDestroyFence-fence-01120, VUID-vkDestroyDevice-device-05137, VUID-vkDestroySurfaceKHR-surface-01266,
        // VUID-vkDestroyInstance-instance-00629).
        unsafe {
            self.buried = self.destroy_presentation();
            self.destroy_all_resources();
            self.free_memory_leftovers();
            for one in objects {
                one.destroy(&self.device);
            }
            self.device.destroy_fence(self.sentinel, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

impl Drop for VulkanGeneration {
    fn drop(&mut self) {
        self.end_or_leave();
    }
}

#[cfg(test)]
impl VulkanGeneration {
    pub(crate) fn faults(&self) -> &Faults {
        &self.faults
    }

    /// Every later submission waits until [`Self::open_gate`].
    pub(crate) fn close_gate(&self) {
        self.faults.close_gate(&self.device);
    }

    pub(crate) fn open_gate(&self) {
        self.faults.open_gate(&self.device);
    }

    /// Whether nobody holds the lane now.
    pub(crate) fn lane_is_free(&self) -> bool {
        self.lane.try_lock().is_ok()
    }

    /// How retired swapchains end on this generation; `None` without presentation.
    pub(crate) fn retirement(&self) -> Option<Retirement> {
        self.presentation.as_ref().map(PresentFns::retirement)
    }
}
