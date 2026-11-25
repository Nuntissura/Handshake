//! The one module that holds Vulkan `unsafe`: a single generation of instance, physical device, logical
//! device and queue, through `ash`. Nothing here is public outside the crate and no `ash` type leaves it.
//! Every `unsafe` block names the rule it relies on.
//!
//! No window, surface or swapchain exists in this slice. The generation's own child objects are one timeline
//! semaphore and the command pools of submitted work. Clients may create further child objects through a
//! [`crate::DeviceLease`], which keeps the generation alive until they returned it.
#![allow(unsafe_code)]

mod queue;

pub(crate) use queue::{QueueFault, Quiescent, RecordedWork};

use crate::device::{
    DeviceHandles, DeviceIdentity, HeapSample, MemoryBudgetSource, MemorySnapshot,
};
use crate::ids::DeviceKey;
use crate::outcome::Unavailable;
use crate::process::ProcessClaim;
use crate::records::ByteQuantity;
use crate::select::{self, CandidateFacts, DeviceClass, BASELINE_API_VERSION};
use ash::vk::{self, Handle};
use queue::QueueState;
use std::ffi::{c_char, CStr};
use std::sync::{Arc, Mutex};

const _: () = assert!(BASELINE_API_VERSION == vk::API_VERSION_1_3);
const _: () = assert!(select::DRIVER_ID_SWIFTSHADER == vk::DriverId::GOOGLE_SWIFTSHADER.as_raw());
const _: () = assert!(select::DRIVER_ID_LLVMPIPE == vk::DriverId::MESA_LLVMPIPE.as_raw());
const _: () = assert!(select::DRIVER_ID_DOZEN == vk::DriverId::MESA_DOZEN.as_raw());

/// Bound on the physical devices inspected at startup.
const MAX_ENUMERATED_DEVICES: usize = 64;

/// The device features the owner enables for its clients and its own queue.
const FEATURES: [&str; 2] = ["dynamicRendering", "timelineSemaphore"];

struct Inspected {
    physical: vk::PhysicalDevice,
    graphics_family: Option<u32>,
    facts: CandidateFacts,
    properties: vk::PhysicalDeviceProperties,
    driver_name: String,
    driver_info: String,
}

struct DeviceParts {
    physical: vk::PhysicalDevice,
    device: ash::Device,
    queue: vk::Queue,
    timeline: vk::Semaphore,
    identity: DeviceIdentity,
    memory_budget: bool,
}

pub(crate) struct VulkanGeneration {
    /// Keeps the Vulkan loader library loaded; dropped after `Drop::drop` destroyed device and instance.
    entry: ash::Entry,
    instance: ash::Instance,
    physical: vk::PhysicalDevice,
    device: ash::Device,
    /// Signalled by every submission with its fence value.
    timeline: vk::Semaphore,
    /// The one queue, and the command pools of work that may still run. This lock is the external
    /// synchronization of the queue.
    queue: Mutex<QueueState>,
    identity: DeviceIdentity,
    memory_budget: bool,
    /// Set once the Vulkan objects are destroyed, or deliberately left alive.
    ended: bool,
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

fn text<E>(value: Result<&CStr, E>) -> String {
    value
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn class_of(kind: vk::PhysicalDeviceType) -> DeviceClass {
    match kind {
        vk::PhysicalDeviceType::DISCRETE_GPU => DeviceClass::Discrete,
        vk::PhysicalDeviceType::INTEGRATED_GPU => DeviceClass::Integrated,
        vk::PhysicalDeviceType::VIRTUAL_GPU => DeviceClass::Virtual,
        vk::PhysicalDeviceType::CPU => DeviceClass::Cpu,
        _ => DeviceClass::Other,
    }
}

/// Map a startup failure. `fallback` names the step for results that only say "refused".
fn startup_error(result: vk::Result, fallback: Unavailable) -> Unavailable {
    match result {
        vk::Result::ERROR_OUT_OF_HOST_MEMORY | vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => {
            Unavailable::OutOfMemory
        }
        vk::Result::ERROR_DEVICE_LOST => Unavailable::PhysicalDeviceLost,
        vk::Result::ERROR_INCOMPATIBLE_DRIVER
        | vk::Result::ERROR_INITIALIZATION_FAILED
        | vk::Result::ERROR_FEATURE_NOT_PRESENT
        | vk::Result::ERROR_EXTENSION_NOT_PRESENT => fallback,
        other => Unavailable::Driver(other.as_raw()),
    }
}

impl VulkanGeneration {
    /// Open a generation. With `required`, only that adapter is acceptable. The generation keeps `process`
    /// until its device is gone.
    pub(crate) fn open(
        process: Arc<ProcessClaim>,
        required: Option<DeviceKey>,
    ) -> Result<Self, Unavailable> {
        // SAFETY: loads the system Vulkan loader through libloading (ash feature `loaded`). A missing or
        // unloadable library is an `Err`, not undefined behaviour; the returned `Entry` keeps it loaded.
        let entry = unsafe { ash::Entry::load() }.map_err(|_| Unavailable::LoaderMissing)?;
        // SAFETY: valid entry; `vkEnumerateInstanceVersion` has no preconditions (absent on 1.0 loaders,
        // which ash reports as `None`).
        let loader_version = match unsafe { entry.try_enumerate_instance_version() } {
            Ok(Some(version)) => version,
            Ok(None) => vk::API_VERSION_1_0,
            Err(result) => return Err(startup_error(result, Unavailable::InstanceRefused)),
        };
        if !select::meets_baseline(loader_version) {
            return Err(Unavailable::ApiBelowBaseline);
        }
        let application = vk::ApplicationInfo::default()
            .application_name(c"Handshake")
            .engine_name(c"handshake-vulkan-renderer")
            .api_version(BASELINE_API_VERSION);
        let create = vk::InstanceCreateInfo::default().application_info(&application);
        // SAFETY: `create` and the application info it points to outlive the call; no layer and no
        // instance extension is requested, so no name pointer is involved.
        let instance = unsafe { entry.create_instance(&create, None) }
            .map_err(|result| startup_error(result, Unavailable::InstanceRefused))?;
        match Self::open_device(&instance, required) {
            Ok(parts) => Ok(Self {
                entry,
                instance,
                physical: parts.physical,
                device: parts.device,
                timeline: parts.timeline,
                queue: Mutex::new(QueueState::new(parts.queue)),
                identity: parts.identity,
                memory_budget: parts.memory_budget,
                ended: false,
                process,
            }),
            Err(reason) => {
                // SAFETY: on `Err`, `open_device` left no logical device behind, so the instance has no
                // child object left.
                unsafe { instance.destroy_instance(None) };
                Err(reason)
            }
        }
    }

    fn inspect(instance: &ash::Instance, physical: vk::PhysicalDevice) -> Inspected {
        // SAFETY: `physical` was enumerated from `instance`; both are Vulkan 1.0 read-only queries.
        let (properties, families) = unsafe {
            (
                instance.get_physical_device_properties(physical),
                instance.get_physical_device_queue_family_properties(physical),
            )
        };
        let graphics_family = families
            .iter()
            .position(|family| {
                family.queue_count > 0 && family.queue_flags.contains(vk::QueueFlags::GRAPHICS)
            })
            .and_then(|index| u32::try_from(index).ok());
        let mut driver_id = 0;
        let mut device_uuid = [0u8; 16];
        let mut driver_name = String::new();
        let mut driver_info = String::new();
        // `vkGetPhysicalDeviceProperties2` with the ID and driver structs is core for a 1.2 device; an
        // older device is rejected by selection anyway and is not asked.
        if properties.api_version >= vk::API_VERSION_1_2 {
            let mut id_properties = vk::PhysicalDeviceIDProperties::default();
            let mut driver_properties = vk::PhysicalDeviceDriverProperties::default();
            {
                let mut properties2 = vk::PhysicalDeviceProperties2::default()
                    .push_next(&mut id_properties)
                    .push_next(&mut driver_properties);
                // SAFETY: the instance was created for 1.3 and the device reports at least 1.2; the
                // chained structs live for the call.
                unsafe { instance.get_physical_device_properties2(physical, &mut properties2) };
            }
            driver_id = driver_properties.driver_id.as_raw();
            device_uuid = id_properties.device_uuid;
            driver_name = text(driver_properties.driver_name_as_c_str());
            driver_info = text(driver_properties.driver_info_as_c_str());
        }
        let facts = CandidateFacts {
            key: DeviceKey {
                vendor_id: properties.vendor_id,
                device_id: properties.device_id,
                device_uuid,
            },
            class: class_of(properties.device_type),
            api_version: properties.api_version,
            driver_id,
            has_graphics_queue: graphics_family.is_some(),
        };
        Inspected {
            physical,
            graphics_family,
            facts,
            properties,
            driver_name,
            driver_info,
        }
    }

    /// Pick and create the logical device. On `Err` nothing created here is left.
    fn open_device(
        instance: &ash::Instance,
        required: Option<DeviceKey>,
    ) -> Result<DeviceParts, Unavailable> {
        // SAFETY: valid instance; read-only enumeration.
        let physicals = unsafe { instance.enumerate_physical_devices() }
            .map_err(|result| startup_error(result, Unavailable::NoPhysicalDevice))?;
        let inspected: Vec<Inspected> = physicals
            .iter()
            .take(MAX_ENUMERATED_DEVICES)
            .map(|&physical| Self::inspect(instance, physical))
            .collect();
        let facts: Vec<CandidateFacts> = inspected.iter().map(|device| device.facts).collect();
        let index = select::pick(&facts, required)?;
        let chosen = inspected.get(index).ok_or(Unavailable::NoSuitableDevice)?;
        let family = chosen
            .graphics_family
            .ok_or(Unavailable::NoSuitableDevice)?;

        // The first planned client (the Vulkan UI pass of the renderer asset) draws with dynamic rendering, and
        // the owner's fence is a timeline semaphore. A feature cannot be enabled after device creation.
        let mut supported12 = vk::PhysicalDeviceVulkan12Features::default();
        let mut supported13 = vk::PhysicalDeviceVulkan13Features::default();
        {
            let mut features2 = vk::PhysicalDeviceFeatures2::default()
                .push_next(&mut supported12)
                .push_next(&mut supported13);
            // SAFETY: core query on a device selection accepted as at least 1.3; the chained structs live
            // for the call.
            unsafe { instance.get_physical_device_features2(chosen.physical, &mut features2) };
        }
        if supported12.timeline_semaphore != vk::TRUE || supported13.dynamic_rendering != vk::TRUE {
            return Err(Unavailable::FeatureMissing);
        }

        // SAFETY: `chosen.physical` belongs to `instance`; read-only enumeration.
        let offered = unsafe { instance.enumerate_device_extension_properties(chosen.physical) }
            .unwrap_or_default();
        let memory_budget = offered.iter().any(|extension| {
            extension
                .extension_name_as_c_str()
                .is_ok_and(|name| name == ash::ext::memory_budget::NAME)
        });
        let mut extension_names: Vec<*const c_char> = Vec::new();
        let mut extensions: Vec<&'static str> = Vec::new();
        if memory_budget {
            extension_names.push(ash::ext::memory_budget::NAME.as_ptr());
            extensions.push("VK_EXT_memory_budget");
        }
        let mut enable12 = vk::PhysicalDeviceVulkan12Features::default().timeline_semaphore(true);
        let mut enable13 = vk::PhysicalDeviceVulkan13Features::default().dynamic_rendering(true);
        let priorities = [1.0f32];
        let queues = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(family)
            .queue_priorities(&priorities)];
        let create = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queues)
            .enabled_extension_names(&extension_names)
            .push_next(&mut enable12)
            .push_next(&mut enable13);
        // SAFETY: `family` is a queue family of `chosen.physical` with at least one queue; the extension
        // name is a static C string the device offers; both features were reported as supported; all
        // referenced arrays outlive the call.
        let device = unsafe { instance.create_device(chosen.physical, &create, None) }
            .map_err(|result| startup_error(result, Unavailable::DeviceCreationRefused))?;
        // SAFETY: one queue of `family` was requested above, so index 0 exists.
        let queue = unsafe { device.get_device_queue(family, 0) };
        let timeline = match queue::create_timeline(&device) {
            Ok(timeline) => timeline,
            Err(result) => {
                // SAFETY: the device has no child object (the semaphore was not created) and no work.
                unsafe { device.destroy_device(None) };
                return Err(startup_error(result, Unavailable::DeviceCreationRefused));
            }
        };

        let properties = &chosen.properties;
        let identity = DeviceIdentity {
            key: chosen.facts.key,
            class: chosen.facts.class,
            api_version: [
                vk::api_version_major(properties.api_version),
                vk::api_version_minor(properties.api_version),
                vk::api_version_patch(properties.api_version),
            ],
            driver_id: chosen.facts.driver_id,
            driver_version: properties.driver_version,
            queue_family: family,
            features: FEATURES.to_vec(),
            extensions,
            name: text(properties.device_name_as_c_str()),
            driver_name: chosen.driver_name.clone(),
            driver_info: chosen.driver_info.clone(),
        };
        Ok(DeviceParts {
            physical: chosen.physical,
            device,
            queue,
            timeline,
            identity,
            memory_budget,
        })
    }

    pub(crate) fn identity(&self) -> &DeviceIdentity {
        &self.identity
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
            heaps.push(HeapSample {
                index: u32::try_from(index).unwrap_or(u32::MAX),
                size_bytes: heap.size,
                device_local: heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL),
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

    /// Destroy the generation. The caller is its only holder and took `evidence` with the queue closed to
    /// further submissions.
    pub(crate) fn destroy(mut self, evidence: Quiescent) {
        self.end(evidence);
    }

    /// Destroy the generation when its device confirms that its work has ended. `false` when it does not: then
    /// nothing is destroyed, because objects that may be in use never are.
    pub(crate) fn dispose(mut self) -> bool {
        self.end_or_leave()
    }

    fn end_or_leave(&mut self) -> bool {
        if self.ended {
            return true;
        }
        match self.quiesce() {
            Some(evidence) => {
                self.end(evidence);
                true
            }
            None => {
                // Every object stays alive, the loader loaded and the process's owner slot taken.
                self.ended = true;
                std::mem::forget(self.entry.clone());
                std::mem::forget(Arc::clone(&self.process));
                false
            }
        }
    }

    fn end(&mut self, _evidence: Quiescent) {
        if self.ended {
            return;
        }
        self.ended = true;
        let pools = self.take_pools();
        // SAFETY: `&mut self` in `destroy` or `drop`: no lease, recording or wait holds this generation any more
        // (each owns an `Arc` of it), so every client object made from a lease was destroyed first (the rule of
        // `DeviceLease`), and this thread alone uses the device and its queue. `_evidence` says that nothing
        // submitted is still in use by the device, so the pools and the semaphore may be destroyed; a lost device
        // must still be destroyed (Vulkan "Lost Device"). Children first, then the device, then the instance; the
        // loader (`entry`) is released after this body (VUID-vkDestroyDevice-device-05137,
        // VUID-vkDestroyInstance-instance-00629).
        unsafe {
            for pool in pools {
                self.device.destroy_command_pool(pool, None);
            }
            self.device.destroy_semaphore(self.timeline, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

impl Drop for VulkanGeneration {
    fn drop(&mut self) {
        let _ = self.end_or_leave();
    }
}
