//! Typed outcomes of owner resources and presentation (MT-203 slice 2). Closed codes like those of `outcome`: no
//! driver text, no content, and never a substitute device or API.

use crate::outcome::{AuthorizeRefusal, ChargeRefusal, DeviceFault};

/// Why no image, buffer or surface was made. Nothing is charged by a refused request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceRefusal {
    /// The claim may not use its device now.
    NotAuthorized(AuthorizeRefusal),
    /// The resource does not fit what is left of the claim's reservation.
    Charge(ChargeRefusal),
    /// A zero size or extent, or no use at all.
    InvalidRequest,
    /// The device does not offer the format for this use.
    FormatUnsupported,
    /// The extent is above what the device allows for the format and use.
    ExtentUnsupported,
    /// The memory would take heap `heap` past the driver's own budget (`VK_EXT_memory_budget`).
    OverBudget {
        heap: u32,
    },
    OutOfDeviceMemory,
    OutOfHostMemory,
    /// No memory type the resource allows serves its use.
    NoMemoryType,
    /// The device's limit on memory objects is reached.
    TooManyObjects,
    /// The owner was opened without presentation ([`crate::OwnerOptions::presentation`]).
    PresentationNotEnabled,
    /// The window cannot carry a surface on this device: the platform is not the one the owner presents to, the
    /// driver refused the surface, or the owner's queue cannot present to it.
    WindowUnsupported,
    Device(DeviceFault),
}

impl ResourceRefusal {
    pub const fn key(self) -> &'static str {
        match self {
            Self::NotAuthorized(refusal) => refusal.key(),
            Self::Charge(refusal) => refusal.key(),
            Self::InvalidRequest => "resource_invalid_request",
            Self::FormatUnsupported => "format_unsupported",
            Self::ExtentUnsupported => "extent_unsupported",
            Self::OverBudget { .. } => "over_driver_budget",
            Self::OutOfDeviceMemory => "out_of_device_memory",
            Self::OutOfHostMemory => "out_of_host_memory",
            Self::NoMemoryType => "no_memory_type",
            Self::TooManyObjects => "too_many_memory_objects",
            Self::PresentationNotEnabled => "presentation_not_enabled",
            Self::WindowUnsupported => "window_unsupported",
            Self::Device(fault) => fault.key(),
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::NotAuthorized(_) => 1,
            Self::Charge(_) => 2,
            Self::InvalidRequest => 3,
            Self::FormatUnsupported => 4,
            Self::ExtentUnsupported => 5,
            Self::OverBudget { .. } => 6,
            Self::OutOfDeviceMemory => 7,
            Self::OutOfHostMemory => 8,
            Self::NoMemoryType => 9,
            Self::TooManyObjects => 10,
            Self::PresentationNotEnabled => 11,
            Self::WindowUnsupported => 12,
            Self::Device(_) => 13,
        }
    }
}

/// Why the host did not read or write an owner buffer. Nothing was read or written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessRefusal {
    /// Work of this client on the buffer's device is not observed finished; it may use the buffer.
    InFlight { in_flight: u64 },
    /// The buffer's memory is not visible to the host ([`crate::BufferMemory::Device`]).
    NotHostVisible,
    /// The range does not lie inside the buffer.
    OutOfRange,
    /// The buffer's device was lost: what it holds is not what any work wrote.
    Lost { generation: u64 },
    /// The buffer's device is gone, and the buffer with it.
    Gone,
    /// The driver refused to map or synchronize the memory.
    MapFailed,
}

impl AccessRefusal {
    pub const fn key(self) -> &'static str {
        match self {
            Self::InFlight { .. } => "access_in_flight",
            Self::NotHostVisible => "not_host_visible",
            Self::OutOfRange => "access_out_of_range",
            Self::Lost { .. } => "device_lost",
            Self::Gone => "resource_gone",
            Self::MapFailed => "map_failed",
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::InFlight { .. } => 1,
            Self::NotHostVisible => 2,
            Self::OutOfRange => 3,
            Self::Lost { .. } => 4,
            Self::Gone => 5,
            Self::MapFailed => 6,
        }
    }
}

/// Why the owner did not record a command into a client's command buffer. Nothing was recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordRefusal {
    /// The image or buffer is another client's.
    Foreign,
    /// The image or buffer belongs to another device generation than the command buffer.
    OtherGeneration,
    /// The image or buffer was not made with the use the command needs.
    UsageMissing,
    /// `UNDEFINED` was given as the layout to leave the image in, or a layout the image cannot be in with its use
    /// (`PRESENT` for an image that is not a frame).
    LayoutInvalid,
    /// The buffer is smaller than the image's texels.
    BufferTooSmall,
    /// The image is a frame, and this is not the recording of its present: only that submission waits for the frame's
    /// acquire.
    FrameNotPresented,
}

impl RecordRefusal {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Foreign => "record_foreign",
            Self::OtherGeneration => "record_other_generation",
            Self::UsageMissing => "record_usage_missing",
            Self::LayoutInvalid => "record_layout_invalid",
            Self::BufferTooSmall => "record_buffer_too_small",
            Self::FrameNotPresented => "record_frame_not_presented",
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::Foreign => 1,
            Self::OtherGeneration => 2,
            Self::UsageMissing => 3,
            Self::LayoutInvalid => 4,
            Self::BufferTooSmall => 5,
            Self::FrameNotPresented => 6,
        }
    }
}

/// Why no swapchain was configured, no image acquired or nothing presented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentRefusal {
    /// No image can be acquired now (`VK_NOT_READY` or `VK_TIMEOUT` of an acquire that does not wait). Ask again on the
    /// next frame.
    NotReady,
    /// The swapchain no longer matches its window: configure it again.
    OutOfDate,
    /// The window's surface is lost: make a new surface.
    SurfaceLost,
    /// The window is minimised or has no size: no swapchain exists until it is configured with one.
    Suspended,
    /// The surface has no swapchain yet: configure it.
    NoSwapchain,
    /// The window still has a swapchain of another surface (a lost device's, ending): configure again later.
    WindowInUse,
    /// No format the owner renders to is offered for this window.
    FormatUnsupported,
    /// The frame or surface is another client's, or of another generation.
    Foreign,
    /// The surface was released.
    Released,
    /// The claim may not submit now.
    NotAuthorized(AuthorizeRefusal),
    /// The client's recording call returned [`crate::RecordAborted`]; nothing was submitted or presented. The image
    /// stays acquired: configure the surface again.
    RecordAborted,
    Device(DeviceFault),
}

impl PresentRefusal {
    pub const fn key(self) -> &'static str {
        match self {
            Self::NotReady => "present_not_ready",
            Self::OutOfDate => "swapchain_out_of_date",
            Self::SurfaceLost => "surface_lost",
            Self::Suspended => "swapchain_suspended",
            Self::NoSwapchain => "no_swapchain",
            Self::WindowInUse => "native_window_in_use",
            Self::FormatUnsupported => "surface_format_unsupported",
            Self::Foreign => "present_foreign",
            Self::Released => "surface_released",
            Self::NotAuthorized(refusal) => refusal.key(),
            Self::RecordAborted => "record_aborted",
            Self::Device(fault) => fault.key(),
        }
    }

    pub const fn number(self) -> u16 {
        match self {
            Self::NotReady => 1,
            Self::OutOfDate => 2,
            Self::SurfaceLost => 3,
            Self::Suspended => 4,
            Self::NoSwapchain => 5,
            Self::WindowInUse => 6,
            Self::FormatUnsupported => 7,
            Self::Foreign => 8,
            Self::Released => 9,
            Self::NotAuthorized(_) => 10,
            Self::RecordAborted => 11,
            Self::Device(_) => 12,
        }
    }
}
