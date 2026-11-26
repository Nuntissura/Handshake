//! Images and buffers the owner makes for a claim (MT-203 slice 2, G203-H1): typed requests, the raw handles a client
//! records with, and the handles that end them.
//!
//! The owner creates the Vulkan objects, places their memory (`gpu-alloc`, see `vulkan/memory.rs`), charges them to the
//! claim inside its reservation, and destroys them itself: when the client drops the handle, once the owner observed that
//! no work that may use them can still run. A claim with resources neither settles clean, nor releases what it retains,
//! nor rebinds, until the owner has destroyed them. No binding type appears here: handles and formats are plain integers
//! and closed enums.

use crate::ids::ClaimId;
use crate::outcome::AuthorizeRefusal;
use crate::owner::OwnerShared;
use crate::records::{ByteQuantity, ClaimInfo};
use crate::refusal::AccessRefusal;
use std::sync::Arc;

/// Formats of owner images and swapchains: 8-bit color in both channel orders and both encodings, and half floats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttachmentFormat {
    Rgba8Unorm,
    Rgba8Srgb,
    Bgra8Unorm,
    Bgra8Srgb,
    Rgba16Float,
}

impl AttachmentFormat {
    pub const ALL: [Self; 5] = [
        Self::Rgba8Unorm,
        Self::Rgba8Srgb,
        Self::Bgra8Unorm,
        Self::Bgra8Srgb,
        Self::Rgba16Float,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Rgba8Unorm => "rgba8_unorm",
            Self::Rgba8Srgb => "rgba8_srgb",
            Self::Bgra8Unorm => "bgra8_unorm",
            Self::Bgra8Srgb => "bgra8_srgb",
            Self::Rgba16Float => "rgba16_float",
        }
    }

    /// Bytes of one texel, as the readback lays them out.
    pub const fn texel_bytes(self) -> u32 {
        match self {
            Self::Rgba16Float => 8,
            _ => 4,
        }
    }

    /// Raw `VkFormat`.
    pub const fn vk_format(self) -> i32 {
        match self {
            Self::Rgba8Unorm => 37,
            Self::Rgba8Srgb => 43,
            Self::Bgra8Unorm => 44,
            Self::Bgra8Srgb => 50,
            Self::Rgba16Float => 97,
        }
    }

    /// The format of a raw `VkFormat`, when it is one of these.
    pub fn of_vk(raw: i32) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|format| format.vk_format() == raw)
    }
}

/// Width and height in texels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Extent {
    pub width: u32,
    pub height: u32,
}

impl Extent {
    /// Texels.
    pub const fn texels(self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

/// What an image is used for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ImageUsage {
    pub color_attachment: bool,
    pub sampled: bool,
    pub transfer_src: bool,
    pub transfer_dst: bool,
}

impl ImageUsage {
    /// An offscreen render target that can be cleared, sampled and read back.
    pub const OFFSCREEN: Self = Self {
        color_attachment: true,
        sampled: true,
        transfer_src: true,
        transfer_dst: true,
    };

    pub const fn is_empty(self) -> bool {
        !(self.color_attachment || self.sampled || self.transfer_src || self.transfer_dst)
    }

    /// Raw `VkImageUsageFlags`.
    pub const fn vk_flags(self) -> u32 {
        (self.transfer_src as u32)
            | ((self.transfer_dst as u32) << 1)
            | ((self.sampled as u32) << 2)
            | ((self.color_attachment as u32) << 4)
    }

    pub(crate) const fn of_vk(raw: u32) -> Self {
        Self {
            transfer_src: raw & 1 != 0,
            transfer_dst: raw & 2 != 0,
            sampled: raw & 4 != 0,
            color_attachment: raw & 0x10 != 0,
        }
    }
}

/// One owner image: a 2D, single-level, single-sampled, optimally tiled render attachment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageRequest {
    pub format: AttachmentFormat,
    pub extent: Extent,
    pub usage: ImageUsage,
}

/// Where a buffer's memory is, by who reads and writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferMemory {
    /// Used by the device only. Charged as committed memory.
    Device,
    /// Written by the host, read by the device. Charged as staging memory.
    Upload,
    /// Written by the device, read by the host (host-cached when the device offers it). Charged as staging memory.
    Readback,
}

/// What a buffer is used for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BufferUsage {
    pub transfer_src: bool,
    pub transfer_dst: bool,
    pub uniform: bool,
    pub storage: bool,
    pub index: bool,
    pub vertex: bool,
}

impl BufferUsage {
    /// The target of an image readback.
    pub const READBACK: Self = Self {
        transfer_src: false,
        transfer_dst: true,
        uniform: false,
        storage: false,
        index: false,
        vertex: false,
    };

    pub const fn is_empty(self) -> bool {
        self.vk_flags() == 0
    }

    /// Raw `VkBufferUsageFlags`.
    pub const fn vk_flags(self) -> u32 {
        (self.transfer_src as u32)
            | ((self.transfer_dst as u32) << 1)
            | ((self.uniform as u32) << 4)
            | ((self.storage as u32) << 5)
            | ((self.index as u32) << 6)
            | ((self.vertex as u32) << 7)
    }
}

/// One owner buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferRequest {
    pub size: u64,
    pub usage: BufferUsage,
    pub memory: BufferMemory,
}

/// Image layouts the owner's recorded commands move between.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageLayout {
    /// What a new image, or one whose contents may be discarded, is in. Never a layout to leave an image in.
    Undefined,
    General,
    ColorAttachment,
    ShaderRead,
    TransferSrc,
    TransferDst,
    /// What a swapchain image must be in when it is presented.
    Present,
}

impl ImageLayout {
    /// Raw `VkImageLayout`.
    pub const fn vk_layout(self) -> i32 {
        match self {
            Self::Undefined => 0,
            Self::General => 1,
            Self::ColorAttachment => 2,
            Self::ShaderRead => 5,
            Self::TransferSrc => 6,
            Self::TransferDst => 7,
            Self::Present => 1_000_001_002,
        }
    }
}

/// The raw handles of a render attachment: what a renderer that takes a host-supplied device draws into
/// (dynamic rendering takes the view, its format and extent). Valid while the image or frame it was read from exists
/// and its device is not lost; recorded only into the owner's command buffers of the same generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderAttachment {
    /// Raw `VkImage`.
    pub image: u64,
    /// Raw `VkImageView`: 2D, color aspect, the one level and layer.
    pub view: u64,
    pub format: AttachmentFormat,
    pub extent: Extent,
    pub usage: ImageUsage,
    pub generation: u64,
}

/// The raw handle of an owner buffer, with the same rules as [`RenderAttachment`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferHandles {
    /// Raw `VkBuffer`.
    pub buffer: u64,
    pub size: u64,
    pub usage: BufferUsage,
    pub memory: BufferMemory,
    pub generation: u64,
}

/// Whose a resource is: the client it was made for and its claim, on one generation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Origin {
    pub(crate) ledger: u64,
    pub(crate) seat: u64,
    pub(crate) claim: ClaimInfo,
    pub(crate) key: u64,
}

impl Origin {
    pub(crate) const fn generation(&self) -> u64 {
        self.claim.generation
    }
}

/// An acquired frame: its surface, swapchain and image index.
pub(crate) type FrameKey = (u64, u64, u32);

pub(crate) mod sealed {
    /// What a recorded command needs to know of an image.
    #[derive(Clone, Copy, Debug)]
    pub struct TargetOf {
        pub(crate) ledger: u64,
        pub(crate) seat: u64,
        pub(crate) generation: u64,
        pub(crate) image: u64,
        pub(crate) extent: super::Extent,
        pub(crate) format: super::AttachmentFormat,
        pub(crate) usage: super::ImageUsage,
        /// The frame, for an acquired swapchain image.
        pub(crate) frame: Option<super::FrameKey>,
    }

    pub trait Target {
        fn target(&self) -> TargetOf;
    }
}

/// An image the owner may record commands on: an [`OwnerImage`] or an acquired [`crate::Frame`].
pub trait ImageTarget: sealed::Target {}

/// An image the owner made for a claim. Dropping it releases it: the owner destroys it once no work of its client that
/// was submitted or being recorded at that moment can still run, and only then frees its memory inside the claim's
/// reservation. Not `Clone`.
pub struct OwnerImage {
    pub(crate) owner: Arc<OwnerShared>,
    pub(crate) origin: Origin,
    pub(crate) attachment: RenderAttachment,
    pub(crate) bytes: u64,
}

impl std::fmt::Debug for OwnerImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnerImage")
            .field("claim", &self.origin.claim.id)
            .field("format", &self.attachment.format)
            .field("extent", &self.attachment.extent)
            .finish_non_exhaustive()
    }
}

impl OwnerImage {
    /// The handles, when the claim may still use its device. Asked before each use, like [`crate::DeviceLease::handles`].
    pub fn attachment(&self) -> Result<&RenderAttachment, AuthorizeRefusal> {
        self.owner.standing(&self.origin)?;
        Ok(&self.attachment)
    }

    pub fn claim(&self) -> ClaimId {
        self.origin.claim.id
    }

    pub fn generation(&self) -> u64 {
        self.origin.generation()
    }

    /// Device memory placed for it, as charged to the claim.
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}

impl sealed::Target for OwnerImage {
    fn target(&self) -> sealed::TargetOf {
        sealed::TargetOf {
            ledger: self.origin.ledger,
            seat: self.origin.seat,
            generation: self.origin.generation(),
            image: self.attachment.image,
            extent: self.attachment.extent,
            format: self.attachment.format,
            usage: self.attachment.usage,
            frame: None,
        }
    }
}

impl ImageTarget for OwnerImage {}

impl Drop for OwnerImage {
    fn drop(&mut self) {
        self.owner.release_resource(&self.origin);
    }
}

/// A buffer the owner made for a claim, released like an [`OwnerImage`].
pub struct OwnerBuffer {
    pub(crate) owner: Arc<OwnerShared>,
    pub(crate) origin: Origin,
    pub(crate) handles: BufferHandles,
    pub(crate) bytes: u64,
}

impl std::fmt::Debug for OwnerBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnerBuffer")
            .field("claim", &self.origin.claim.id)
            .field("size", &self.handles.size)
            .field("memory", &self.handles.memory)
            .finish_non_exhaustive()
    }
}

impl OwnerBuffer {
    /// The handle, when the claim may still use its device.
    pub fn handles(&self) -> Result<&BufferHandles, AuthorizeRefusal> {
        self.owner.standing(&self.origin)?;
        Ok(&self.handles)
    }

    pub fn claim(&self) -> ClaimId {
        self.origin.claim.id
    }

    pub fn generation(&self) -> u64 {
        self.origin.generation()
    }

    /// Device memory placed for it, as charged to the claim. At least its size.
    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    /// Copy `out.len()` bytes from `offset` into `out`. Refused while any submission of this client on the buffer's
    /// device is not observed finished (the owner asks the device first, without waiting), after a loss, and for memory
    /// the host cannot see. What the device wrote is visible once the writing submission recorded a memory dependency
    /// to the host ([`crate::Recording::read_back`] does).
    pub fn read(&self, offset: u64, out: &mut [u8]) -> Result<(), AccessRefusal> {
        self.owner.read_buffer(&self.origin, offset, out)
    }

    /// Copy `data` to `offset`, under the same rules as [`Self::read`].
    pub fn write(&self, offset: u64, data: &[u8]) -> Result<(), AccessRefusal> {
        self.owner.write_buffer(&self.origin, offset, data)
    }

    pub(crate) fn origin(&self) -> &Origin {
        &self.origin
    }
}

impl Drop for OwnerBuffer {
    fn drop(&mut self) {
        self.owner.release_resource(&self.origin);
    }
}

/// What one memory heap holds of the owner's own memory objects (the blocks of owner images and buffers). Measured: every
/// allocation and free of the owner passes through its books. Beside it, never merged with it, stand the claims'
/// charges ([`crate::LedgerUsage::owned_bytes`]) and the driver's estimates ([`crate::MemorySnapshot`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnerHeapUse {
    pub index: u32,
    pub bytes: ByteQuantity,
    pub memory_objects: u64,
}

/// The owner's memory objects on one generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerMemoryUse {
    pub generation: u64,
    pub heaps: Vec<OwnerHeapUse>,
    /// Owner images and buffers that exist on the generation, released ones that wait for their work included.
    pub resources: u64,
    /// Swapchains that exist on the generation, retired ones that wait for their end included.
    pub swapchains: u64,
    /// Their images, which the presentation engine allocates without saying how much: estimated from extent, format and
    /// count, never measured, so never added to [`Self::heaps`] nor charged to a claim. A surface's claim reserves for
    /// them.
    pub swapchain_image_bytes: ByteQuantity,
}
