//! Swapchains: creation for the window's current size, recreation with the current one as `oldSwapchain` on resize,
//! restore and out-of-date, suspension when the window has no size, and destruction of one that nothing uses.
//!
//! What a configuration creates is decided by [`plan`] alone, from what the surface reports and the host asks for, so
//! that decision is checked without a window.

use super::super::VulkanGeneration;
use super::{fault, refused, PresentFault, PresentFns, SurfaceEntry, Swapchain};
use crate::present::{
    PresentCounters, PresentMode, SwapchainInfo, SwapchainRequest, SwapchainStatus,
};
use crate::refusal::PresentRefusal;
use crate::resource::{AttachmentFormat, Extent, ImageUsage};
use ash::vk;
use std::sync::atomic::Ordering;

fn vk_mode(mode: PresentMode) -> vk::PresentModeKHR {
    match mode {
        PresentMode::Fifo => vk::PresentModeKHR::FIFO,
        PresentMode::Mailbox => vk::PresentModeKHR::MAILBOX,
        PresentMode::Immediate => vk::PresentModeKHR::IMMEDIATE,
    }
}

/// What a configuration creates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Plan {
    pub(super) extent: Extent,
    pub(super) format: AttachmentFormat,
    pub(super) mode: PresentMode,
    pub(super) count: u32,
    /// Raw `VkImageUsageFlags` (the flags of this binding have no `Debug`).
    pub(super) usage: u32,
    /// Raw `VkCompositeAlphaFlagBitsKHR`.
    pub(super) alpha: u32,
}

/// The swapchain for a surface that reports `caps`, `formats` and `modes`, as the host asks for it in `request`.
/// `None` when the window has no size (minimised): then no swapchain is made (VUID-VkSwapchainCreateInfoKHR-imageExtent-01689).
pub(super) fn plan(
    caps: &vk::SurfaceCapabilitiesKHR,
    formats: &[vk::SurfaceFormatKHR],
    modes: &[vk::PresentModeKHR],
    request: &SwapchainRequest,
) -> Result<Option<Plan>, PresentRefusal> {
    // The window's extent, or the asked one within its bounds where the window lets the swapchain decide
    // (`currentExtent` of 0xFFFFFFFF; VkSurfaceCapabilitiesKHR).
    let extent = if caps.current_extent.width == u32::MAX {
        Extent {
            width: request
                .extent
                .width
                .clamp(caps.min_image_extent.width, caps.max_image_extent.width),
            height: request
                .extent
                .height
                .clamp(caps.min_image_extent.height, caps.max_image_extent.height),
        }
    } else {
        Extent {
            width: caps.current_extent.width,
            height: caps.current_extent.height,
        }
    };
    if extent.width == 0 || extent.height == 0 {
        return Ok(None);
    }
    let srgb = vk::ColorSpaceKHR::SRGB_NONLINEAR;
    let offered = |format: AttachmentFormat| {
        formats
            .iter()
            .any(|offer| offer.format.as_raw() == format.vk_format() && offer.color_space == srgb)
    };
    // The asked format, else the first of the owner's that is offered with the sRGB color space
    // (VUID-VkSwapchainCreateInfoKHR-imageFormat-01273).
    let format = std::iter::once(request.format)
        .chain(AttachmentFormat::ALL)
        .find(|format| offered(*format))
        .ok_or(PresentRefusal::FormatUnsupported)?;
    // FIFO is the one mode every surface must support (VkPresentModeKHR).
    let mode = if modes.contains(&vk_mode(request.mode)) {
        request.mode
    } else {
        PresentMode::Fifo
    };
    // One above the minimum, within the maximum when there is one (VUID-VkSwapchainCreateInfoKHR-minImageCount-01272).
    let mut count = caps.min_image_count.saturating_add(1);
    if caps.max_image_count != 0 {
        count = count.min(caps.max_image_count);
    }
    // COLOR_ATTACHMENT is in every surface's supported usage (VkSurfaceCapabilitiesKHR); the transfer uses only when
    // offered (VUID-VkSwapchainCreateInfoKHR-presentMode-01427).
    let transfer = vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST;
    let usage = vk::ImageUsageFlags::COLOR_ATTACHMENT | (caps.supported_usage_flags & transfer);
    let alpha = if caps
        .supported_composite_alpha
        .contains(vk::CompositeAlphaFlagsKHR::OPAQUE)
    {
        vk::CompositeAlphaFlagsKHR::OPAQUE
    } else {
        // The lowest bit that is offered (VUID-VkSwapchainCreateInfoKHR-compositeAlpha-01280).
        let raw = caps.supported_composite_alpha.as_raw();
        vk::CompositeAlphaFlagsKHR::from_raw(raw & raw.wrapping_neg())
    };
    Ok(Some(Plan {
        extent,
        format,
        mode,
        count,
        usage: usage.as_raw(),
        alpha: alpha.as_raw(),
    }))
}

impl VulkanGeneration {
    /// Create the swapchain of surface `key` for its window's current size, or again; suspend it for no size.
    pub(crate) fn configure(
        &self,
        key: u64,
        request: &SwapchainRequest,
    ) -> Result<SwapchainStatus, PresentFault> {
        let fns = self.fns()?;
        let slot = self.surface_slot(key)?;
        let mut guard = slot.lock(&self.surfaces);
        let entry = &mut *guard;
        if entry.is_closed() {
            return Err(refused(PresentRefusal::Released));
        }
        if entry.lost {
            return Err(refused(PresentRefusal::SurfaceLost));
        }
        // SAFETY: a surface of this instance, externally synchronized by its lock; read-only queries.
        let (caps, formats, modes) = unsafe {
            let caps = fns
                .surface
                .get_physical_device_surface_capabilities(self.physical, entry.surface);
            let formats = fns
                .surface
                .get_physical_device_surface_formats(self.physical, entry.surface);
            let modes = fns
                .surface
                .get_physical_device_surface_present_modes(self.physical, entry.surface);
            (caps, formats, modes)
        };
        let lose = |entry: &mut SurfaceEntry, result: vk::Result| {
            if result == vk::Result::ERROR_SURFACE_LOST_KHR {
                entry.lost = true;
            }
            fault(result)
        };
        let caps = caps.map_err(|result| lose(&mut *entry, result))?;
        let formats = formats.map_err(|result| lose(&mut *entry, result))?;
        let modes = modes.map_err(|result| lose(&mut *entry, result))?;
        let Some(plan) = plan(&caps, &formats, &modes, request).map_err(refused)? else {
            // The current swapchain stays, so it is the old one of the next.
            if let Some(current) = entry.current.as_mut() {
                current.suspended = true;
            }
            return Ok(SwapchainStatus::Suspended);
        };
        // Passing the current swapchain retires it, also when the creation fails.
        let old = entry.current.take();
        let old_handle = old
            .as_ref()
            .map_or(vk::SwapchainKHR::null(), |old| old.handle);
        entry.retired.extend(old);
        entry.stale = false;
        let create = vk::SwapchainCreateInfoKHR::default()
            .surface(entry.surface)
            .min_image_count(plan.count)
            .image_format(vk::Format::from_raw(plan.format.vk_format()))
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_extent(vk::Extent2D {
                width: plan.extent.width,
                height: plan.extent.height,
            })
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::from_raw(plan.usage))
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(caps.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::from_raw(plan.alpha))
            .present_mode(vk_mode(plan.mode))
            .clipped(true)
            .old_swapchain(old_handle);
        // SAFETY: the device has VK_KHR_swapchain enabled; the surface is supported by its queue family
        // (VUID-VkSwapchainCreateInfoKHR-surface-01270); count, format and color space, extent, usage, alpha and mode
        // are what `plan` chose from the surface's own report (-minImageCount-01272, -imageFormat-01273,
        // -imageExtent-01689, -presentMode-01427, -compositeAlpha-01280, -presentMode-01281), the transform is the current
        // one (-preTransform-01279); the old swapchain is this surface's and not retired (-oldSwapchain-01933); the
        // surface's lock synchronizes both.
        let handle = unsafe { fns.swapchain.create_swapchain(&create, None) }
            .map_err(|result| lose(&mut *entry, result))?;
        let number = self.surfaces.next_swapchain.fetch_add(1, Ordering::Relaxed) + 1;
        let swapchain = self
            .complete_swapchain(fns, handle, number, &plan)
            .map_err(fault)?;
        let info = swapchain.info();
        self.surfaces.counted(&swapchain);
        PresentCounters::bump(&entry.counters.swapchains);
        entry.current = Some(swapchain);
        Ok(SwapchainStatus::Ready(info))
    }

    /// Images, views and render semaphores of a new swapchain. On a failure everything of it is destroyed: it was never
    /// acquired from.
    fn complete_swapchain(
        &self,
        fns: &PresentFns,
        handle: vk::SwapchainKHR,
        number: u64,
        plan: &Plan,
    ) -> Result<Swapchain, vk::Result> {
        let mut swapchain = Swapchain {
            handle,
            number,
            images: Vec::new(),
            views: Vec::new(),
            rendered: Vec::new(),
            last_present_of: Vec::new(),
            proven_of: Vec::new(),
            acquired: Vec::new(),
            out: 0,
            stranded: Vec::new(),
            presents: 0,
            unproven: 0,
            acquiring: 0,
            format: plan.format,
            extent: plan.extent,
            mode: plan.mode,
            usage: ImageUsage::of_vk(plan.usage),
            last_use: 0,
            suspended: false,
            image_bytes: 0,
        };
        let filled = (|| -> Result<(), vk::Result> {
            // SAFETY: a swapchain of this device, synchronized by its surface's lock.
            swapchain.images = unsafe { fns.swapchain.get_swapchain_images(handle) }?;
            for image in swapchain.images.clone() {
                let range = vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1);
                let create = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(vk::Format::from_raw(plan.format.vk_format()))
                    .subresource_range(range);
                // SAFETY: a presentable image of this swapchain, in its own format, one level and one layer.
                swapchain
                    .views
                    .push(unsafe { self.device.create_image_view(&create, None) }?);
                let info = vk::SemaphoreCreateInfo::default();
                // SAFETY: valid device; a binary semaphore.
                swapchain
                    .rendered
                    .push(unsafe { self.device.create_semaphore(&info, None) }?);
            }
            Ok(())
        })();
        let images = swapchain.images.len();
        swapchain.last_present_of = vec![0; images];
        swapchain.proven_of = vec![0; images];
        swapchain.acquired = vec![false; images];
        swapchain.image_bytes = plan
            .extent
            .texels()
            .saturating_mul(u64::from(plan.format.texel_bytes()))
            .saturating_mul(u64::try_from(images).unwrap_or(u64::MAX));
        match filled {
            Ok(()) => Ok(swapchain),
            Err(result) => {
                // SAFETY: never acquired from, so nothing refers to any of it.
                unsafe { self.destroy_swapchain_objects(fns, swapchain) };
                Err(result)
            }
        }
    }

    /// Destroy a swapchain that was counted, and its semaphores.
    ///
    /// # Safety
    /// No frame of it is out, no work that used its images may still run, its acquires have ended, and its presents have
    /// ended or its device is lost and drained.
    pub(super) unsafe fn destroy_swapchain(&self, fns: &PresentFns, swapchain: Swapchain) {
        self.surfaces.uncounted(&swapchain);
        // SAFETY: the caller's.
        unsafe { self.destroy_swapchain_objects(fns, swapchain) };
    }

    /// # Safety
    /// As for [`Self::destroy_swapchain`].
    unsafe fn destroy_swapchain_objects(&self, fns: &PresentFns, swapchain: Swapchain) {
        // SAFETY: the caller's (VUID-vkDestroySwapchainKHR-swapchain-01282, VUID-vkDestroySemaphore-semaphore-05149: the
        // render semaphores' batches were observed, and the stranded acquire semaphores are referred to by no batch and
        // their acquires have ended; VUID-vkDestroyImageView-imageView-01026); views go before the swapchain that owns
        // their images.
        unsafe {
            for view in swapchain.views {
                self.device.destroy_image_view(view, None);
            }
            for semaphore in swapchain.rendered.into_iter().chain(swapchain.stranded) {
                self.device.destroy_semaphore(semaphore, None);
            }
            fns.swapchain.destroy_swapchain(swapchain.handle, None);
        }
    }

    /// Stop acquiring from surface `key` until it is configured again.
    pub(crate) fn suspend(&self, key: u64) -> Result<(), PresentFault> {
        let slot = self.surface_slot(key)?;
        let mut entry = slot.lock(&self.surfaces);
        if entry.is_closed() {
            return Err(refused(PresentRefusal::Released));
        }
        if let Some(current) = entry.current.as_mut() {
            current.suspended = true;
        }
        Ok(())
    }

    /// The swapchain surface `key` presents with now.
    pub(crate) fn swapchain_info(&self, key: u64) -> Option<SwapchainInfo> {
        let slot = self.surfaces.slot(key)?;
        let entry = slot.lock(&self.surfaces);
        entry
            .current
            .as_ref()
            .filter(|current| !current.suspended && !entry.stale && !entry.is_closed())
            .map(Swapchain::info)
    }
}

#[cfg(test)]
mod tests {
    use super::{plan, Plan};
    use crate::present::{PresentMode, SwapchainRequest};
    use crate::refusal::PresentRefusal;
    use crate::resource::{AttachmentFormat, Extent};
    use ash::vk;

    fn caps(current: (u32, u32), min: (u32, u32), max: (u32, u32)) -> vk::SurfaceCapabilitiesKHR {
        vk::SurfaceCapabilitiesKHR {
            current_extent: vk::Extent2D {
                width: current.0,
                height: current.1,
            },
            min_image_extent: vk::Extent2D {
                width: min.0,
                height: min.1,
            },
            max_image_extent: vk::Extent2D {
                width: max.0,
                height: max.1,
            },
            min_image_count: 2,
            max_image_count: 3,
            supported_usage_flags: vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::TRANSFER_DST,
            supported_composite_alpha: vk::CompositeAlphaFlagsKHR::INHERIT
                | vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED,
            ..Default::default()
        }
    }

    fn srgb(format: vk::Format) -> vk::SurfaceFormatKHR {
        vk::SurfaceFormatKHR {
            format,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        }
    }

    fn request(width: u32, height: u32, mode: PresentMode) -> SwapchainRequest {
        SwapchainRequest {
            extent: Extent { width, height },
            format: AttachmentFormat::Rgba8Unorm,
            mode,
        }
    }

    #[test]
    fn a_configuration_follows_what_the_window_reports() {
        let window = caps((800, 600), (1, 1), (4096, 4096));
        let formats = [
            vk::SurfaceFormatKHR {
                format: vk::Format::R8G8B8A8_UNORM,
                color_space: vk::ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT,
            },
            srgb(vk::Format::B8G8R8A8_UNORM),
        ];
        let modes = [vk::PresentModeKHR::FIFO, vk::PresentModeKHR::IMMEDIATE];
        let planned = plan(
            &window,
            &formats,
            &modes,
            &request(5000, 10, PresentMode::Mailbox),
        )
        .expect("offered");
        assert_eq!(
            planned,
            Some(Plan {
                // The window's size wins over the asked one.
                extent: Extent {
                    width: 800,
                    height: 600
                },
                // The asked format is offered only in another color space.
                format: AttachmentFormat::Bgra8Unorm,
                // Mailbox is not offered.
                mode: PresentMode::Fifo,
                // One above the minimum, at most the maximum.
                count: 3,
                // Transfer source is not offered.
                usage: (vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
                    .as_raw(),
                // Opaque is not offered: the lowest bit that is.
                alpha: vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED.as_raw(),
            })
        );
        let immediate = plan(
            &window,
            &formats,
            &modes,
            &request(1, 1, PresentMode::Immediate),
        )
        .expect("offered");
        assert_eq!(
            immediate.map(|plan| plan.mode),
            Some(PresentMode::Immediate)
        );
        let unbounded = vk::SurfaceCapabilitiesKHR {
            max_image_count: 0,
            ..window
        };
        let more = plan(
            &unbounded,
            &formats,
            &modes,
            &request(1, 1, PresentMode::Fifo),
        )
        .expect("offered");
        assert_eq!(more.map(|plan| plan.count), Some(3));
    }

    #[test]
    fn a_window_without_size_suspends_and_one_that_decides_lets_the_request_choose() {
        let formats = [srgb(vk::Format::R8G8B8A8_UNORM)];
        let modes = [vk::PresentModeKHR::FIFO];
        let minimised = caps((0, 0), (0, 0), (0, 0));
        assert_eq!(
            plan(
                &minimised,
                &formats,
                &modes,
                &request(5000, 10, PresentMode::Fifo)
            ),
            Ok(None)
        );
        let free = caps((u32::MAX, u32::MAX), (16, 16), (4096, 4096));
        let planned = plan(
            &free,
            &formats,
            &modes,
            &request(5000, 10, PresentMode::Fifo),
        )
        .expect("offered");
        assert_eq!(
            planned.map(|plan| (plan.extent, plan.format)),
            Some((
                Extent {
                    width: 4096,
                    height: 16
                },
                AttachmentFormat::Rgba8Unorm
            ))
        );
        let unusable = [vk::SurfaceFormatKHR {
            format: vk::Format::A2B10G10R10_UNORM_PACK32,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        }];
        let window = caps((640, 480), (1, 1), (4096, 4096));
        assert_eq!(
            plan(
                &window,
                &unusable,
                &modes,
                &request(1, 1, PresentMode::Fifo)
            ),
            Err(PresentRefusal::FormatUnsupported)
        );
    }
}
