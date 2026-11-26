//! Opening a generation: the instance and its extensions, the strict pick of a physical device, and the logical device
//! with the features and extensions the owner enables. Presentation extensions are enabled only when the host asked for
//! presentation ([`crate::OwnerOptions`]).

use super::fault;
use crate::device::DeviceIdentity;
use crate::ids::DeviceKey;
use crate::outcome::Unavailable;
use crate::select::{self, CandidateFacts, DeviceClass, BASELINE_API_VERSION};
use ash::vk;
use std::ffi::{c_char, CStr};

/// Bound on the physical devices inspected at startup.
const MAX_ENUMERATED_DEVICES: usize = 64;

/// The device features the owner enables for its clients.
const FEATURES: [&str; 2] = ["dynamicRendering", "timelineSemaphore"];

struct Inspected {
    physical: vk::PhysicalDevice,
    graphics_family: Option<u32>,
    facts: CandidateFacts,
    properties: vk::PhysicalDeviceProperties,
    driver_name: String,
    driver_info: String,
}

pub(super) struct DeviceParts {
    pub(super) physical: vk::PhysicalDevice,
    pub(super) device: ash::Device,
    pub(super) queue: vk::Queue,
    pub(super) sentinel: vk::Fence,
    pub(super) identity: DeviceIdentity,
    pub(super) memory_budget: bool,
    /// `VK_EXT_swapchain_maintenance1` is enabled with its feature.
    pub(super) present_fences: bool,
}

/// What the instance was created with.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct InstanceFeatures {
    pub(super) presentation: bool,
    /// `VK_KHR_get_surface_capabilities2` and `VK_EXT_surface_maintenance1`, which present fences need.
    pub(super) surface_maintenance: bool,
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
pub(super) fn startup_error(result: vk::Result, fallback: Unavailable) -> Unavailable {
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

fn names(offered: &[vk::ExtensionProperties]) -> Vec<&CStr> {
    offered
        .iter()
        .filter_map(|extension| extension.extension_name_as_c_str().ok())
        .collect()
}

/// Create the instance, with the surface extensions when `presentation` is asked for.
pub(super) fn create_instance(
    entry: &ash::Entry,
    presentation: bool,
) -> Result<(ash::Instance, InstanceFeatures), Unavailable> {
    let application = vk::ApplicationInfo::default()
        .application_name(c"Handshake")
        .engine_name(c"handshake-vulkan-renderer")
        .api_version(BASELINE_API_VERSION);
    let mut enabled: Vec<*const c_char> = Vec::new();
    let mut features = InstanceFeatures::default();
    if presentation {
        if !cfg!(windows) {
            return Err(Unavailable::PresentationUnsupported);
        }
        // SAFETY: valid entry; read-only enumeration of the loader's instance extensions.
        let offered = unsafe { entry.enumerate_instance_extension_properties(None) }
            .map_err(|result| startup_error(result, Unavailable::InstanceRefused))?;
        let offered = names(&offered);
        let has = |name: &CStr| offered.contains(&name);
        if !has(ash::khr::surface::NAME) || !has(ash::khr::win32_surface::NAME) {
            return Err(Unavailable::PresentationUnsupported);
        }
        enabled.push(ash::khr::surface::NAME.as_ptr());
        enabled.push(ash::khr::win32_surface::NAME.as_ptr());
        if has(ash::khr::get_surface_capabilities2::NAME)
            && has(ash::ext::surface_maintenance1::NAME)
        {
            enabled.push(ash::khr::get_surface_capabilities2::NAME.as_ptr());
            enabled.push(ash::ext::surface_maintenance1::NAME.as_ptr());
            features.surface_maintenance = true;
        }
        features.presentation = true;
    }
    let create = vk::InstanceCreateInfo::default()
        .application_info(&application)
        .enabled_extension_names(&enabled);
    // SAFETY: `create`, the application info and the static extension names it points to outlive the call; every name
    // was offered by the loader; no layer is requested.
    let instance = unsafe { entry.create_instance(&create, None) }
        .map_err(|result| startup_error(result, Unavailable::InstanceRefused))?;
    Ok((instance, features))
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
    // `vkGetPhysicalDeviceProperties2` with the ID and driver structs is core for a 1.2 device; an older device is
    // rejected by selection anyway and is not asked.
    if properties.api_version >= vk::API_VERSION_1_2 {
        let mut id_properties = vk::PhysicalDeviceIDProperties::default();
        let mut driver_properties = vk::PhysicalDeviceDriverProperties::default();
        {
            let mut properties2 = vk::PhysicalDeviceProperties2::default()
                .push_next(&mut id_properties)
                .push_next(&mut driver_properties);
            // SAFETY: the instance was created for 1.3 and the device reports at least 1.2; the chained structs live
            // for the call.
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
pub(super) fn open_device(
    entry: &ash::Entry,
    instance: &ash::Instance,
    instance_features: InstanceFeatures,
    required: Option<DeviceKey>,
) -> Result<DeviceParts, Unavailable> {
    // SAFETY: valid instance; read-only enumeration.
    let physicals = unsafe { instance.enumerate_physical_devices() }
        .map_err(|result| startup_error(result, Unavailable::NoPhysicalDevice))?;
    let inspected: Vec<Inspected> = physicals
        .iter()
        .take(MAX_ENUMERATED_DEVICES)
        .map(|&physical| inspect(instance, physical))
        .collect();
    let facts: Vec<CandidateFacts> = inspected.iter().map(|device| device.facts).collect();
    let index = select::pick(&facts, required)?;
    let chosen = inspected.get(index).ok_or(Unavailable::NoSuitableDevice)?;
    let family = chosen
        .graphics_family
        .ok_or(Unavailable::NoSuitableDevice)?;

    // SAFETY: `chosen.physical` belongs to `instance`; read-only enumeration.
    let offered = unsafe { instance.enumerate_device_extension_properties(chosen.physical) }
        .unwrap_or_default();
    let offered = names(&offered);
    let has = |name: &CStr| offered.contains(&name);
    let memory_budget = has(ash::ext::memory_budget::NAME);
    let mut extension_names: Vec<*const c_char> = Vec::new();
    let mut extensions: Vec<&'static str> = Vec::new();
    if memory_budget {
        extension_names.push(ash::ext::memory_budget::NAME.as_ptr());
        extensions.push("VK_EXT_memory_budget");
    }
    let mut maintenance_offered = false;
    if instance_features.presentation {
        let win32 = ash::khr::win32_surface::Instance::new(entry, instance);
        // SAFETY: the instance has VK_KHR_win32_surface enabled; `family` is a queue family of `chosen.physical`.
        let presents = unsafe {
            win32.get_physical_device_win32_presentation_support(chosen.physical, family)
        };
        if !presents || !has(ash::khr::swapchain::NAME) {
            return Err(Unavailable::PresentationUnsupported);
        }
        extension_names.push(ash::khr::swapchain::NAME.as_ptr());
        extensions.push("VK_KHR_swapchain");
        maintenance_offered =
            instance_features.surface_maintenance && has(ash::ext::swapchain_maintenance1::NAME);
    }

    // The first planned client (the Vulkan UI pass of the renderer asset) draws with dynamic rendering, and timeline
    // semaphores order work of clients. A feature cannot be enabled after device creation.
    let mut supported12 = vk::PhysicalDeviceVulkan12Features::default();
    let mut supported13 = vk::PhysicalDeviceVulkan13Features::default();
    let mut supported_maintenance = vk::PhysicalDeviceSwapchainMaintenance1FeaturesEXT::default();
    {
        let mut features2 = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut supported12)
            .push_next(&mut supported13);
        if maintenance_offered {
            features2 = features2.push_next(&mut supported_maintenance);
        }
        // SAFETY: core query on a device selection accepted as at least 1.3; the maintenance struct is chained only when
        // the device offers its extension; the chained structs live for the call.
        unsafe { instance.get_physical_device_features2(chosen.physical, &mut features2) };
    }
    if supported12.timeline_semaphore != vk::TRUE || supported13.dynamic_rendering != vk::TRUE {
        return Err(Unavailable::FeatureMissing);
    }
    let present_fences =
        maintenance_offered && supported_maintenance.swapchain_maintenance1 == vk::TRUE;
    let mut features: Vec<&'static str> = FEATURES.to_vec();
    if present_fences {
        extension_names.push(ash::ext::swapchain_maintenance1::NAME.as_ptr());
        extensions.push("VK_EXT_swapchain_maintenance1");
        features.push("swapchainMaintenance1");
    }
    let mut enable12 = vk::PhysicalDeviceVulkan12Features::default().timeline_semaphore(true);
    let mut enable13 = vk::PhysicalDeviceVulkan13Features::default().dynamic_rendering(true);
    let mut enable_maintenance =
        vk::PhysicalDeviceSwapchainMaintenance1FeaturesEXT::default().swapchain_maintenance1(true);
    let priorities = [1.0f32];
    let queues = [vk::DeviceQueueCreateInfo::default()
        .queue_family_index(family)
        .queue_priorities(&priorities)];
    let mut create = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queues)
        .enabled_extension_names(&extension_names)
        .push_next(&mut enable12)
        .push_next(&mut enable13);
    if present_fences {
        create = create.push_next(&mut enable_maintenance);
    }
    // SAFETY: `family` is a queue family of `chosen.physical` with at least one queue; every extension name is a static
    // C string the device offers; every feature was reported as supported; all referenced arrays outlive the call.
    let device = unsafe { instance.create_device(chosen.physical, &create, None) }
        .map_err(|result| startup_error(result, Unavailable::DeviceCreationRefused))?;
    let device = fault::table(instance, device);
    // SAFETY: one queue of `family` was requested above, so index 0 exists.
    let queue = unsafe { device.get_device_queue(family, 0) };
    let sentinel = match super::queue::create_fence(&device) {
        Ok(sentinel) => sentinel,
        Err(result) => {
            // SAFETY: the device has no child object (the fence was not created) and no work.
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
        features,
        extensions,
        name: text(properties.device_name_as_c_str()),
        driver_name: chosen.driver_name.clone(),
        driver_info: chosen.driver_info.clone(),
    };
    Ok(DeviceParts {
        physical: chosen.physical,
        device,
        queue,
        sentinel,
        identity,
        memory_budget,
        present_fences,
    })
}
