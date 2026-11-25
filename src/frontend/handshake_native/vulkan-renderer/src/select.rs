//! Strict physical-device selection, as pure logic over plain facts (testable without a GPU).
//!
//! A device is accepted only when it is a hardware Vulkan implementation: a discrete, integrated or
//! virtual GPU at or above the baseline, with a graphics queue, whose driver is neither a software
//! rasterizer nor a layer over Direct3D. Everything else is a typed refusal. There is no fallback tier:
//! when no device is accepted the owner has no device (GUI-DESKTOP-020/026, HBR-HOST-001/003).

use crate::ids::DeviceKey;
use crate::outcome::Unavailable;

/// Vulkan 1.3, packed as `VK_MAKE_API_VERSION(0, 1, 3, 0)`. The same baseline as Studio's Vulkan leaf,
/// whose UI pass is the first planned client of this owner.
pub const BASELINE_API_VERSION: u32 = (1 << 22) | (3 << 12);

/// `VkPhysicalDeviceType`, reduced to what selection needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceClass {
    Discrete,
    Integrated,
    Virtual,
    /// `VK_PHYSICAL_DEVICE_TYPE_CPU`: an implementation running on the host processors.
    Cpu,
    /// `VK_PHYSICAL_DEVICE_TYPE_OTHER` or a value this unit does not know.
    Other,
}

/// Whether a packed API version is standard Vulkan (variant 0) at or above the baseline. The variant sits in
/// the top bits, so another variant (Vulkan SC) would pass a plain numeric comparison.
pub const fn meets_baseline(api_version: u32) -> bool {
    api_version >> 29 == 0 && api_version >= BASELINE_API_VERSION
}

impl DeviceClass {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Discrete => "discrete_gpu",
            Self::Integrated => "integrated_gpu",
            Self::Virtual => "virtual_gpu",
            Self::Cpu => "cpu",
            Self::Other => "other",
        }
    }
}

/// `VK_DRIVER_ID_GOOGLE_SWIFTSHADER`: a CPU rasterizer.
pub const DRIVER_ID_SWIFTSHADER: i32 = 10;
/// `VK_DRIVER_ID_MESA_LLVMPIPE`: a CPU rasterizer.
pub const DRIVER_ID_LLVMPIPE: i32 = 13;
/// `VK_DRIVER_ID_MESA_DOZEN`: Vulkan layered over Direct3D 12.
pub const DRIVER_ID_DOZEN: i32 = 23;

/// What selection knows about one enumerated device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateFacts {
    pub key: DeviceKey,
    pub class: DeviceClass,
    /// Packed device `apiVersion`.
    pub api_version: u32,
    /// Raw `VkDriverId`, or 0 when the device is too old to report one.
    pub driver_id: i32,
    pub has_graphics_queue: bool,
}

/// Why one device is not acceptable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    ApiBelowBaseline,
    SoftwareDevice,
    LayeredDriver,
    UnknownDeviceType,
    NoGraphicsQueue,
}

/// Rank of an acceptable device (lower is preferred), or why it is refused.
pub fn evaluate(facts: &CandidateFacts) -> Result<u8, Rejection> {
    if !meets_baseline(facts.api_version) {
        return Err(Rejection::ApiBelowBaseline);
    }
    if facts.class == DeviceClass::Cpu
        || facts.driver_id == DRIVER_ID_SWIFTSHADER
        || facts.driver_id == DRIVER_ID_LLVMPIPE
    {
        return Err(Rejection::SoftwareDevice);
    }
    if facts.driver_id == DRIVER_ID_DOZEN {
        return Err(Rejection::LayeredDriver);
    }
    let rank = match facts.class {
        DeviceClass::Discrete => 0,
        DeviceClass::Integrated => 1,
        DeviceClass::Virtual => 2,
        DeviceClass::Cpu => return Err(Rejection::SoftwareDevice),
        DeviceClass::Other => return Err(Rejection::UnknownDeviceType),
    };
    if !facts.has_graphics_queue {
        return Err(Rejection::NoGraphicsQueue);
    }
    Ok(rank)
}

/// Pick the device to open, by index into `candidates`.
///
/// With `required`, only that adapter is acceptable (a rebind keeps its adapter; another one never takes
/// its place). Among acceptable devices the best rank wins and the earliest enumerated breaks a tie.
pub fn pick(
    candidates: &[CandidateFacts],
    required: Option<DeviceKey>,
) -> Result<usize, Unavailable> {
    if candidates.is_empty() {
        return Err(Unavailable::NoPhysicalDevice);
    }
    let mut best: Option<(u8, usize)> = None;
    let mut saw_required = false;
    let mut saw_software = false;
    let mut saw_layered = false;
    let mut saw_old = false;
    for (index, facts) in candidates.iter().enumerate() {
        if let Some(key) = required {
            if facts.key != key {
                continue;
            }
            saw_required = true;
        }
        match evaluate(facts) {
            Ok(rank) => {
                if best.is_none_or(|(best_rank, _)| rank < best_rank) {
                    best = Some((rank, index));
                }
            }
            Err(Rejection::SoftwareDevice) => saw_software = true,
            Err(Rejection::LayeredDriver) => saw_layered = true,
            Err(Rejection::ApiBelowBaseline) => saw_old = true,
            Err(Rejection::UnknownDeviceType | Rejection::NoGraphicsQueue) => {}
        }
    }
    if let Some((_, index)) = best {
        return Ok(index);
    }
    if required.is_some() && !saw_required {
        return Err(Unavailable::RequiredDeviceMissing);
    }
    Err(if saw_layered {
        Unavailable::LayeredDriverRefused
    } else if saw_software {
        Unavailable::SoftwareDeviceRefused
    } else if saw_old {
        Unavailable::ApiBelowBaseline
    } else {
        Unavailable::NoSuitableDevice
    })
}
