//! Swapchains: creation for the window's current size, recreation with the current one as `oldSwapchain` on resize,
//! restore and out-of-date, suspension when the window has no size, and destruction of one that nothing uses.

use super::super::VulkanGeneration;
use super::{fault, refused, PresentFault, PresentFns, SurfaceEntry, Swapchain};
use crate::present::{
    PresentCounters, PresentMode, SwapchainInfo, SwapchainRequest, SwapchainStatus,
};
use crate::refusal::PresentRefusal;
use crate::resource::{AttachmentFormat, Extent, ImageUsage};
use ash::vk;

fn vk_mode(mode: PresentMode) -> vk::PresentModeKHR {
    match mode {
        PresentMode::Fifo => vk::PresentModeKHR::FIFO,
        PresentMode::Mailbox => vk::PresentModeKHR::MAILBOX,
        PresentMode::Immediate => vk::PresentModeKHR::IMMEDIATE,
    }
}

/// The swapchain's extent: the window's, or the asked one within its bounds where the window lets the swapchain decide
/// (`currentExtent` of 0xFFFFFFFF).
fn extent_of(caps: &vk::SurfaceCapabilitiesKHR, asked: Extent) -> Extent {
    if caps.current_extent.width == u32::MAX {
        Extent {
            width: asked
                .width
                .clamp(caps.min_image_extent.width, caps.max_image_extent.width),
            height: asked
                .height
                .clamp(caps.min_image_extent.height, caps.max_image_extent.height),
        }
    } else {
        Extent {
            width: caps.current_extent.width,
            height: caps.current_extent.height,
        }
    }
}

impl VulkanGeneration {
    /// Create the swapchain of surface `key` for its window's current size, or again; suspend it for no size.
    pub(crate) fn configure(
        &self,
        key: u64,
        request: &SwapchainRequest,
    ) -> Result<SwapchainStatus, PresentFault> {
        let fns = self.fns()?;
        let mut guard = self.lock_surfaces();
        let surfaces = &mut *guard;
        let entry = match surfaces.entries.get_mut(&key) {
            Some(entry) if !entry.released => entry,
            _ => return Err(refused(PresentRefusal::Released)),
        };
        if entry.lost {
            return Err(refused(PresentRefusal::SurfaceLost));
        }
        // SAFETY: a surface of this instance, externally synchronized by the surfaces lock; read-only queries.
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
        let extent = extent_of(&caps, request.extent);
        if extent.width == 0 || extent.height == 0 {
            // VUID-VkSwapchainCreateInfoKHR-imageExtent-01689: no swapchain of no size. The current one stays, so it is
            // the old one of the next.
            if let Some(current) = entry.current.as_mut() {
                current.suspended = true;
            }
            return Ok(SwapchainStatus::Suspended);
        }
        let srgb = vk::ColorSpaceKHR::SRGB_NONLINEAR;
        let offered = |format: AttachmentFormat| {
            formats.iter().any(|offer| {
                offer.format.as_raw() == format.vk_format() && offer.color_space == srgb
            })
        };
        let format = std::iter::once(request.format)
            .chain(AttachmentFormat::ALL)
            .find(|format| offered(*format))
            .ok_or(refused(PresentRefusal::FormatUnsupported))?;
        // FIFO is the one mode every surface must support (VkPresentModeKHR).
        let mode = if modes.contains(&vk_mode(request.mode)) {
            request.mode
        } else {
            PresentMode::Fifo
        };
        let mut count = caps.min_image_count.saturating_add(1);
        if caps.max_image_count != 0 {
            count = count.min(caps.max_image_count);
        }
        // COLOR_ATTACHMENT is in every surface's supported usage (VkSurfaceCapabilitiesKHR); the transfer uses only when offered
        // (VUID-VkSwapchainCreateInfoKHR-presentMode-01427).
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
        // Passing the current swapchain retires it, also when the creation fails.
        let old = entry.current.take();
        let old_handle = old
            .as_ref()
            .map_or(vk::SwapchainKHR::null(), |old| old.handle);
        entry.retired.extend(old);
        entry.stale = false;
        let create = vk::SwapchainCreateInfoKHR::default()
            .surface(entry.surface)
            .min_image_count(count)
            .image_format(vk::Format::from_raw(format.vk_format()))
            .image_color_space(srgb)
            .image_extent(vk::Extent2D {
                width: extent.width,
                height: extent.height,
            })
            .image_array_layers(1)
            .image_usage(usage)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(caps.current_transform)
            .composite_alpha(alpha)
            .present_mode(vk_mode(mode))
            .clipped(true)
            .old_swapchain(old_handle);
        // SAFETY: the device has VK_KHR_swapchain enabled; the surface is supported by its queue family
        // (VUID-VkSwapchainCreateInfoKHR-surface-01270); the count lies within the surface's bounds (-minImageCount-01272,
        // -presentMode-02839), format and color space are a reported pair (-imageFormat-01273), the extent is not zero
        // (-imageExtent-01689), and usage, transform, alpha and mode are reported ones (-presentMode-01427,
        // -preTransform-01279, -compositeAlpha-01280, -presentMode-01281); the old swapchain is this surface's and not
        // retired (-oldSwapchain-01933); the surfaces lock synchronizes both.
        let handle = unsafe { fns.swapchain.create_swapchain(&create, None) }
            .map_err(|result| lose(&mut *entry, result))?;
        surfaces.next_swapchain += 1;
        let number = surfaces.next_swapchain;
        let made = self.complete_swapchain(fns, handle, number, format, extent, mode, usage);
        let swapchain = match made {
            Ok(swapchain) => swapchain,
            Err(result) => return Err(fault(result)),
        };
        let info = swapchain.info();
        PresentCounters::bump(&entry.counters.swapchains);
        entry.current = Some(swapchain);
        Ok(SwapchainStatus::Ready(info))
    }

    /// Images, views and render semaphores of a new swapchain. On a failure everything of it is destroyed: it was never
    /// acquired from.
    #[allow(clippy::too_many_arguments)]
    fn complete_swapchain(
        &self,
        fns: &PresentFns,
        handle: vk::SwapchainKHR,
        number: u64,
        format: AttachmentFormat,
        extent: Extent,
        mode: PresentMode,
        usage: vk::ImageUsageFlags,
    ) -> Result<Swapchain, vk::Result> {
        let mut swapchain = Swapchain {
            handle,
            number,
            images: Vec::new(),
            views: Vec::new(),
            rendered: Vec::new(),
            last_present_of: Vec::new(),
            acquired: Vec::new(),
            format,
            extent,
            mode,
            usage: ImageUsage::of_vk(usage.as_raw()),
            last_use: 0,
            last_present: 0,
            pending_proofs: 0,
            suspended: false,
        };
        let filled = (|| -> Result<(), vk::Result> {
            // SAFETY: a swapchain of this device, synchronized by the surfaces lock.
            swapchain.images = unsafe { fns.swapchain.get_swapchain_images(handle) }?;
            for image in swapchain.images.clone() {
                let range = vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1);
                let create = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(vk::Format::from_raw(format.vk_format()))
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
        swapchain.acquired = vec![false; images];
        match filled {
            Ok(()) => Ok(swapchain),
            Err(result) => {
                // SAFETY: never acquired from, so nothing refers to any of it.
                unsafe { self.destroy_swapchain(fns, swapchain) };
                Err(result)
            }
        }
    }

    /// # Safety
    /// No work that used its images may still run, and its presents have ended or its device is lost and drained.
    pub(super) unsafe fn destroy_swapchain(&self, fns: &PresentFns, swapchain: Swapchain) {
        // SAFETY: the caller's (VUID-vkDestroySwapchainKHR-swapchain-01282, VUID-vkDestroySemaphore-semaphore-05149,
        // VUID-vkDestroyImageView-imageView-01026); views go before the swapchain that owns their images.
        unsafe {
            for view in swapchain.views {
                self.device.destroy_image_view(view, None);
            }
            for semaphore in swapchain.rendered {
                self.device.destroy_semaphore(semaphore, None);
            }
            fns.swapchain.destroy_swapchain(swapchain.handle, None);
        }
    }

    /// Stop acquiring from surface `key` until it is configured again.
    pub(crate) fn suspend(&self, key: u64) -> Result<(), PresentFault> {
        let mut surfaces = self.lock_surfaces();
        match surfaces.entries.get_mut(&key) {
            Some(entry) if !entry.released => {
                if let Some(current) = entry.current.as_mut() {
                    current.suspended = true;
                }
                Ok(())
            }
            _ => Err(refused(PresentRefusal::Released)),
        }
    }

    /// The swapchain surface `key` presents with now.
    pub(crate) fn swapchain_info(&self, key: u64) -> Option<SwapchainInfo> {
        let surfaces = self.lock_surfaces();
        let entry = surfaces.entries.get(&key)?;
        entry
            .current
            .as_ref()
            .filter(|current| !current.suspended && !entry.stale && !entry.released)
            .map(Swapchain::info)
    }
}

#[cfg(test)]
mod tests {
    use super::extent_of;
    use crate::resource::Extent;
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
            ..Default::default()
        }
    }

    #[test]
    fn the_window_size_wins_unless_the_window_lets_the_swapchain_decide() {
        let asked = Extent {
            width: 5000,
            height: 10,
        };
        let window = caps((800, 600), (1, 1), (4096, 4096));
        assert_eq!(
            extent_of(&window, asked),
            Extent {
                width: 800,
                height: 600
            }
        );
        let minimised = caps((0, 0), (0, 0), (0, 0));
        assert_eq!(
            extent_of(&minimised, asked),
            Extent {
                width: 0,
                height: 0
            }
        );
        let free = caps((u32::MAX, u32::MAX), (16, 16), (4096, 4096));
        assert_eq!(
            extent_of(&free, asked),
            Extent {
                width: 4096,
                height: 16
            }
        );
    }
}
