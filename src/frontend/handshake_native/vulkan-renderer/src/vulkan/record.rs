//! Commands the owner records itself into a client's command buffer: layout transitions, clears and the readback of an
//! image into a host-visible buffer. The owner checked beforehand that the image and buffer are the recording client's,
//! live on this generation and allow the use; the barriers are conservative (every earlier command, every later one).

use super::queue::RecordedWork;
use ash::vk::{self, Handle};

/// What a recorded command touches of an image.
#[derive(Clone, Copy)]
pub(crate) struct ImageCommand {
    image: u64,
    extent: vk::Extent2D,
}

impl ImageCommand {
    pub(crate) fn new(image: u64, width: u32, height: u32) -> Self {
        Self {
            image,
            extent: vk::Extent2D { width, height },
        }
    }
}

fn color_range() -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .level_count(1)
        .layer_count(1)
}

impl RecordedWork {
    /// Move the image from layout `from` to `to`, after every earlier command and before every later one.
    pub(crate) fn transition(&self, image: ImageCommand, from: i32, to: i32) {
        self.barrier(
            image,
            vk::ImageLayout::from_raw(from),
            vk::ImageLayout::from_raw(to),
        );
    }

    fn barrier(&self, image: ImageCommand, from: vk::ImageLayout, to: vk::ImageLayout) {
        let barrier = vk::ImageMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
            .dst_access_mask(vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE)
            .old_layout(from)
            .new_layout(to)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(vk::Image::from_raw(image.image))
            .subresource_range(color_range());
        let all = vk::PipelineStageFlags::ALL_COMMANDS;
        // SAFETY: the command buffer is in the recording state for the length of the client's recording call, which is
        // when the owner records; the image is a live image of this device whose layout the caller names
        // (VUID-VkImageMemoryBarrier-oldLayout-01197), and `to` is not UNDEFINED (checked by the caller,
        // VUID-VkImageMemoryBarrier-newLayout-01198).
        unsafe {
            self.generation().device.cmd_pipeline_barrier(
                self.commands(),
                all,
                all,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    }

    /// Fill the image with `color`, leaving it in layout `to`.
    pub(crate) fn clear(&self, image: ImageCommand, from: i32, color: [f32; 4], to: i32) {
        let target = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        self.barrier(image, vk::ImageLayout::from_raw(from), target);
        let value = vk::ClearColorValue { float32: color };
        // SAFETY: as in `transition`; the image has TRANSFER_DST use and a color format (checked by the caller,
        // VUID-vkCmdClearColorImage-image-00002), and is in TRANSFER_DST_OPTIMAL now.
        unsafe {
            self.generation().device.cmd_clear_color_image(
                self.commands(),
                vk::Image::from_raw(image.image),
                target,
                &value,
                &[color_range()],
            );
        }
        self.barrier(image, target, vk::ImageLayout::from_raw(to));
    }

    /// Copy the whole image, tightly packed row after row, to the start of `buffer`, and make the copy visible to the
    /// host once the submission's fence has signalled. The image is left in layout `to`.
    pub(crate) fn read_back(&self, image: ImageCommand, from: i32, buffer: u64, to: i32) {
        let source = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
        self.barrier(image, vk::ImageLayout::from_raw(from), source);
        let region = vk::BufferImageCopy::default()
            .image_subresource(
                vk::ImageSubresourceLayers::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(1),
            )
            .image_extent(vk::Extent3D {
                width: image.extent.width,
                height: image.extent.height,
                depth: 1,
            });
        // The host reads what the transfer wrote: a memory dependency with destination HOST_READ, without which the
        // fence alone makes nothing visible to the host (Vulkan "Host Write Ordering Guarantees", "Fence Signaling").
        let visible = vk::BufferMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::HOST_READ)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .buffer(vk::Buffer::from_raw(buffer))
            .size(vk::WHOLE_SIZE);
        // SAFETY: as in `transition`; the image has TRANSFER_SRC use and is in TRANSFER_SRC_OPTIMAL now, the buffer has
        // TRANSFER_DST use and room for the whole image (checked by the caller, VUID-vkCmdCopyImageToBuffer-pRegions-00183).
        unsafe {
            let device = &self.generation().device;
            device.cmd_copy_image_to_buffer(
                self.commands(),
                vk::Image::from_raw(image.image),
                source,
                vk::Buffer::from_raw(buffer),
                &[region],
            );
            device.cmd_pipeline_barrier(
                self.commands(),
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::HOST,
                vk::DependencyFlags::empty(),
                &[],
                &[visible],
                &[],
            );
        }
        self.barrier(image, source, vk::ImageLayout::from_raw(to));
    }
}
