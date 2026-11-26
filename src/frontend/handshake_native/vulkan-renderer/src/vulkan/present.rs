//! Surfaces and swapchains of one generation: creation, recreation, acquire and present, and their destruction after
//! settlement. The rules are in the documentation of `crate::present`; this module holds the Vulkan calls.
//!
//! One lock (`surfaces`) is the external synchronization of every surface and swapchain (vkAcquireNextImageKHR,
//! vkQueuePresentKHR, vkCreateSwapchainKHR and vkDestroySwapchainKHR require it) and guards the books: which images are
//! acquired, which semaphores wait for which submission, which fences prove which present. A present takes it after the
//! lane; nothing that holds it takes the lane. The owner's look at the device takes it only if it is free, and a dropped
//! surface or frame never waits for it: they leave a note ([`Note`]) that whoever takes the lock next applies first.

mod frames;
mod retire;
mod swapchain;

use super::queue::QueueFault;
use super::resources::Owned;
use super::VulkanGeneration;
use crate::present::{NativeWindow, PresentCounters, PresentMode, SurfaceTarget, SwapchainInfo};
use crate::records::{ByteQuantity, ClaimInfo};
use crate::refusal::PresentRefusal;
use crate::resource::{AttachmentFormat, Extent, ImageUsage};
use ash::vk::{self, Handle};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

static NEXT_SURFACE: AtomicU64 = AtomicU64::new(1);

/// The presentation functions of a generation that was opened with presentation.
pub(crate) struct PresentFns {
    surface: ash::khr::surface::Instance,
    win32: ash::khr::win32_surface::Instance,
    swapchain: ash::khr::swapchain::Device,
    /// `VK_EXT_swapchain_maintenance1` is enabled: every present gets a fence.
    present_fences: bool,
}

impl PresentFns {
    pub(super) fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &ash::Device,
        present_fences: bool,
    ) -> Self {
        Self {
            surface: ash::khr::surface::Instance::new(entry, instance),
            win32: ash::khr::win32_surface::Instance::new(entry, instance),
            swapchain: ash::khr::swapchain::Device::new(instance, device),
            present_fences,
        }
    }
}

/// Why presentation did not happen: a typed refusal, or a fault of the device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PresentFault {
    Refused(PresentRefusal),
    Device(QueueFault),
}

/// Why no surface was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceFault {
    NotEnabled,
    WindowUnsupported,
    Device(QueueFault),
}

/// A surface that exists and is not kept yet.
pub(crate) struct MadeSurface {
    surface: vk::SurfaceKHR,
    target: Box<dyn SurfaceTarget>,
}

struct Swapchain {
    handle: vk::SwapchainKHR,
    number: u64,
    images: Vec<vk::Image>,
    views: Vec<vk::ImageView>,
    /// Signalled by the submission that rendered an image, waited for by its present; one per image (the Vulkan guide,
    /// "Swapchain Semaphore Reuse").
    rendered: Vec<vk::Semaphore>,
    /// Queue-wide number of the last present of each image, 0 for none.
    last_present_of: Vec<u64>,
    acquired: Vec<bool>,
    format: AttachmentFormat,
    extent: Extent,
    mode: PresentMode,
    usage: ImageUsage,
    /// Highest submission value that used its images.
    last_use: u64,
    /// Queue-wide number of its last present.
    last_present: u64,
    /// Fences of its acquires and presents that have not signalled.
    pending_proofs: u32,
    suspended: bool,
}

impl Swapchain {
    fn info(&self) -> SwapchainInfo {
        let count = u64::try_from(self.images.len()).unwrap_or(u64::MAX);
        let bytes = self
            .extent
            .texels()
            .saturating_mul(u64::from(self.format.texel_bytes()))
            .saturating_mul(count);
        SwapchainInfo {
            swapchain: self.number,
            extent: self.extent,
            format: self.format,
            mode: self.mode,
            images: u32::try_from(self.images.len()).unwrap_or(u32::MAX),
            usage: self.usage,
            image_bytes: ByteQuantity::Estimated(bytes),
        }
    }
}

/// A fence whose signal proves that every present up to `proves` has ended.
struct Proof {
    fence: vk::Fence,
    proves: u64,
    swapchain: u64,
}

struct SurfaceEntry {
    surface: vk::SurfaceKHR,
    /// Keeps the window alive. Dropped only after the surface is destroyed.
    target: Box<dyn SurfaceTarget>,
    owned: Owned,
    claim: ClaimInfo,
    counters: Arc<PresentCounters>,
    current: Option<Swapchain>,
    retired: Vec<Swapchain>,
    /// Binary semaphores for acquires, unsignalled with nothing pending.
    free: Vec<vk::Semaphore>,
    /// Semaphores of acquires, and the submission value that waits for each.
    waiting: Vec<(vk::Semaphore, u64)>,
    proofs: Vec<Proof>,
    released: bool,
    lost: bool,
    stale: bool,
}

/// The surfaces of one generation, and what is known of the presents on its queue.
#[derive(Default)]
pub(crate) struct Surfaces {
    entries: BTreeMap<u64, SurfaceEntry>,
    /// Presents queued on the queue, numbered from 1.
    queued: u64,
    /// Presents up to this number are known to have ended (they are performed in queue order).
    proven: u64,
    next_swapchain: u64,
    /// Semaphores with an acquire signal nobody waits for, and fences of failed acquires: destroyed with the device.
    stranded: Vec<vk::Semaphore>,
    stranded_fences: Vec<vk::Fence>,
}

/// What a dropped handle leaves for the next holder of the surfaces lock.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Note {
    /// The client let go of a surface.
    Released(u64),
    /// A frame was dropped without a present.
    Dropped(FrameRef),
}

/// Notes waiting for the surfaces lock. Their own lock is held for nothing but a push or a take.
#[derive(Default)]
pub(crate) struct Notes(Mutex<Vec<Note>>);

impl Notes {
    fn take(&self) -> Vec<Note> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }

    fn push(&self, note: Note) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(note);
    }
}

impl Surfaces {
    /// Apply what dropped handles left.
    fn apply(&mut self, notes: Vec<Note>) {
        for note in notes {
            match note {
                Note::Released(key) => {
                    if let Some(entry) = self.entries.get_mut(&key) {
                        entry.released = true;
                        if let Some(current) = entry.current.take() {
                            entry.retired.push(current);
                        }
                    }
                }
                Note::Dropped(frame) => {
                    // Its image stays acquired and its semaphore signalled: the swapchain has to be configured again,
                    // and the semaphore goes with the device.
                    self.stranded.push(vk::Semaphore::from_raw(frame.semaphore));
                    if let Some(entry) = self.entries.get_mut(&frame.surface) {
                        entry.stale = true;
                    }
                }
            }
        }
    }
}

/// One acquired image.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Acquired {
    pub(crate) swapchain: u64,
    pub(crate) index: u32,
    pub(crate) image: u64,
    pub(crate) view: u64,
    pub(crate) semaphore: u64,
    pub(crate) format: AttachmentFormat,
    pub(crate) extent: Extent,
    pub(crate) usage: ImageUsage,
    pub(crate) suboptimal: bool,
}

/// What became of a present.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shown {
    Presented,
    Suboptimal,
    OutOfDate,
    SurfaceLost,
    Fault(QueueFault),
}

/// What the owner destroyed of the presentation.
#[derive(Clone, Copy, Debug)]
pub(crate) enum PresentEnd {
    Swapchain { claim: ClaimInfo, swapchain: u64 },
    Surface(Owned),
}

/// The identity of an acquired frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FrameRef {
    pub(crate) surface: u64,
    pub(crate) swapchain: u64,
    pub(crate) index: u32,
    pub(crate) semaphore: u64,
}

fn fault(result: vk::Result) -> PresentFault {
    match result {
        vk::Result::ERROR_OUT_OF_DATE_KHR => PresentFault::Refused(PresentRefusal::OutOfDate),
        vk::Result::ERROR_SURFACE_LOST_KHR => PresentFault::Refused(PresentRefusal::SurfaceLost),
        vk::Result::ERROR_NATIVE_WINDOW_IN_USE_KHR => {
            PresentFault::Refused(PresentRefusal::WindowInUse)
        }
        other => PresentFault::Device(QueueFault::of(other)),
    }
}

fn refused(refusal: PresentRefusal) -> PresentFault {
    PresentFault::Refused(refusal)
}

impl VulkanGeneration {
    /// The surfaces lock, with every note applied.
    fn lock_surfaces(&self) -> MutexGuard<'_, Surfaces> {
        let mut surfaces = self.surfaces.lock().unwrap_or_else(PoisonError::into_inner);
        surfaces.apply(self.notes.take());
        surfaces
    }

    /// Leave a note for the next holder of the surfaces lock, without waiting for it.
    pub(super) fn note(&self, note: Note) {
        self.notes.push(note);
        // Applied at once when nobody holds the lock.
        if let Ok(mut surfaces) = self.surfaces.try_lock() {
            surfaces.apply(self.notes.take());
        }
    }

    fn fns(&self) -> Result<&PresentFns, PresentFault> {
        self.presentation
            .as_ref()
            .ok_or(PresentFault::Refused(PresentRefusal::Foreign))
    }

    /// Create a surface on the target's window and check that the owner's queue can present to it
    /// (VUID-VkSwapchainCreateInfoKHR-surface-01270).
    pub(crate) fn make_surface(
        &self,
        target: Box<dyn SurfaceTarget>,
    ) -> Result<MadeSurface, SurfaceFault> {
        let Some(fns) = &self.presentation else {
            return Err(SurfaceFault::NotEnabled);
        };
        let created = match target.native_window() {
            NativeWindow::Win32 { hwnd, hinstance } if cfg!(windows) => {
                let info = vk::Win32SurfaceCreateInfoKHR::default()
                    .hinstance(hinstance.get())
                    .hwnd(hwnd.get());
                // SAFETY: the instance has VK_KHR_win32_surface enabled; the target keeps the window, and with it the
                // HWND and its module, valid for as long as the surface exists (VUID-VkWin32SurfaceCreateInfoKHR-hwnd-01308,
                // -hinstance-01307).
                unsafe { fns.win32.create_win32_surface(&info, None) }
            }
            NativeWindow::Win32 { .. } => return Err(SurfaceFault::WindowUnsupported),
        };
        let surface = match created {
            Ok(surface) => surface,
            Err(vk::Result::ERROR_OUT_OF_HOST_MEMORY | vk::Result::ERROR_OUT_OF_DEVICE_MEMORY) => {
                return Err(SurfaceFault::Device(QueueFault::OutOfMemory))
            }
            Err(_) => return Err(SurfaceFault::WindowUnsupported),
        };
        let family = self.identity.queue_family;
        // SAFETY: a surface of this instance; the family is one of this physical device.
        let supported = unsafe {
            fns.surface
                .get_physical_device_surface_support(self.physical, family, surface)
        };
        match supported {
            Ok(true) => Ok(MadeSurface { surface, target }),
            other => {
                // SAFETY: no swapchain refers to the surface yet.
                unsafe { fns.surface.destroy_surface(surface, None) };
                match other {
                    Err(result)
                        if result != vk::Result::ERROR_SURFACE_LOST_KHR
                            && result != vk::Result::ERROR_DEVICE_LOST =>
                    {
                        Err(SurfaceFault::Device(QueueFault::of(result)))
                    }
                    _ => Err(SurfaceFault::WindowUnsupported),
                }
            }
        }
    }

    /// Keep a surface for the claim of `owned`; returns its identity.
    pub(crate) fn keep_surface(
        &self,
        made: MadeSurface,
        owned: Owned,
        claim: ClaimInfo,
        counters: Arc<PresentCounters>,
    ) -> u64 {
        let key = NEXT_SURFACE.fetch_add(1, Ordering::Relaxed);
        self.lock_surfaces().entries.insert(
            key,
            SurfaceEntry {
                surface: made.surface,
                target: made.target,
                owned,
                claim,
                counters,
                current: None,
                retired: Vec::new(),
                free: Vec::new(),
                waiting: Vec::new(),
                proofs: Vec::new(),
                released: false,
                lost: false,
                stale: false,
            },
        );
        key
    }

    /// Destroy a surface that was never handed out.
    pub(crate) fn discard_surface(&self, made: MadeSurface) {
        if let Some(fns) = &self.presentation {
            // SAFETY: no swapchain was made on it.
            unsafe { fns.surface.destroy_surface(made.surface, None) };
        }
        drop(made.target);
    }
}
