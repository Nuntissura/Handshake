//! Surfaces and swapchains of one generation: creation, recreation, acquire and present, and their destruction after
//! settlement. The rules are in the documentation of `crate::present`; this module holds the Vulkan calls.
//!
//! # Locks (MT-203 E3-1)
//!
//! Every surface has a lock of its own ([`SurfaceSlot`]). It is the external synchronization of that surface, its
//! swapchains and their semaphores and fences (vkAcquireNextImageKHR, vkCreateSwapchainKHR, vkDestroySwapchainKHR and
//! the swapchains of vkQueuePresentKHR require it), and it guards their books: which images are acquired, which
//! semaphores wait for which submission, which fences prove what. A present takes the lane and then its own surface's
//! lock and nothing else, so a present that blocks in the driver (a full FIFO queue) keeps waiting other presents and
//! submissions, which need the queue, and calls on that same surface, but never another window's acquire or configure.
//! Nothing that holds a surface's lock takes the lane or another surface's lock. The registry of surfaces has its own
//! lock, held to find, insert or remove one and never across a Vulkan call. The owner's look takes a surface's lock only
//! if it is free; a dropped surface or frame never waits for it: they leave a note ([`Note`]) that whoever takes that
//! surface's lock next applies first.

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
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};

pub(crate) use retire::Retirement;

static NEXT_SURFACE: AtomicU64 = AtomicU64::new(1);

/// Windows' targets, dropped by the owner after it let go of its locks.
pub(crate) type Targets = Vec<Box<dyn SurfaceTarget>>;

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

    /// How a retired swapchain's presents are proven ended on this device.
    pub(crate) fn retirement(&self) -> Retirement {
        if self.present_fences {
            Retirement::PresentFences
        } else {
            Retirement::SuccessorPresent
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
    /// Number, within this swapchain and from 1, of the last present of each image; 0 for none.
    last_present_of: Vec<u64>,
    acquired: Vec<bool>,
    /// Frames handed out and neither presented nor dropped: their client may still record with the image.
    out: u32,
    /// Acquire semaphores of frames that were dropped or whose submission the queue did not take: given a signal by
    /// their acquire, waited for by nothing. Destroyed with the swapchain, when all its acquires have ended.
    stranded: Vec<vk::Semaphore>,
    /// Presents the queue took.
    presents: u64,
    /// Presents whose present fence has not signalled (`VK_EXT_swapchain_maintenance1`).
    unproven: u32,
    /// Acquires whose fence has not signalled.
    acquiring: u32,
    format: AttachmentFormat,
    extent: Extent,
    mode: PresentMode,
    usage: ImageUsage,
    /// Highest submission value that used its images.
    last_use: u64,
    suspended: bool,
    /// Estimated bytes of its images: the presentation engine allocates them and does not say.
    image_bytes: u64,
}

impl Swapchain {
    fn info(&self) -> SwapchainInfo {
        SwapchainInfo {
            swapchain: self.number,
            extent: self.extent,
            format: self.format,
            mode: self.mode,
            images: u32::try_from(self.images.len()).unwrap_or(u32::MAX),
            usage: self.usage,
            image_bytes: ByteQuantity::Estimated(self.image_bytes),
        }
    }
}

/// What a fence proves once it has signalled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProofKind {
    /// The acquire of an image has ended, and with it the image's earlier present `previous` of the same swapchain (0
    /// for none): the image is acquired again only once the presentation engine let go of it (the Vulkan guide,
    /// "Swapchain Semaphore Reuse").
    Acquire { previous: u64 },
    /// A present of `VK_EXT_swapchain_maintenance1` released what it used.
    Present,
}

struct Proof {
    fence: vk::Fence,
    swapchain: u64,
    kind: ProofKind,
}

struct SurfaceEntry {
    surface: vk::SurfaceKHR,
    /// Keeps the window alive. Taken only after the surface is destroyed.
    target: Option<Box<dyn SurfaceTarget>>,
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
    /// Highest number of a swapchain of this surface one of whose presents is proven ended (0 for none).
    proven_present: u64,
    released: bool,
    lost: bool,
    stale: bool,
    /// The surface was destroyed: the entry only waits to leave the registry.
    ended: bool,
}

impl SurfaceEntry {
    fn swapchain_mut(&mut self, number: u64) -> Option<&mut Swapchain> {
        self.current
            .iter_mut()
            .chain(self.retired.iter_mut())
            .find(|swapchain| swapchain.number == number)
    }

    /// Whether it takes no more calls: released, ending, or ended.
    fn is_closed(&self) -> bool {
        self.released || self.ended
    }
}

/// One surface: its lock, and the notes that wait for it.
pub(super) struct SurfaceSlot {
    entry: Mutex<SurfaceEntry>,
    /// Held for nothing but a push or a take.
    notes: Mutex<Vec<Note>>,
}

/// What a dropped handle leaves for the next holder of its surface's lock.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Note {
    /// The client let go of the surface.
    Released,
    /// A frame was dropped without a present.
    Dropped(FrameRef),
}

/// Semaphores and fences whose state is not known: destroyed with the device.
#[derive(Default)]
struct Unknown {
    semaphores: Vec<vk::Semaphore>,
    fences: Vec<vk::Fence>,
}

/// The surfaces of one generation.
#[derive(Default)]
pub(crate) struct Surfaces {
    slots: Mutex<BTreeMap<u64, Arc<SurfaceSlot>>>,
    next_swapchain: AtomicU64,
    /// Swapchains that exist, and the estimated bytes of their images: reported beside the measured memory.
    swapchains: AtomicU64,
    image_bytes: AtomicU64,
    unknown: Mutex<Unknown>,
}

impl Surfaces {
    fn slot(&self, key: u64) -> Option<Arc<SurfaceSlot>> {
        self.slots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&key)
            .cloned()
    }

    fn all(&self) -> Vec<(u64, Arc<SurfaceSlot>)> {
        let slots = self.slots.lock().unwrap_or_else(PoisonError::into_inner);
        slots
            .iter()
            .map(|(key, slot)| (*key, Arc::clone(slot)))
            .collect()
    }

    fn remove(&self, key: u64) {
        self.slots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&key);
    }

    fn lock_unknown(&self) -> MutexGuard<'_, Unknown> {
        self.unknown.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A semaphore whose state is not known any more.
    fn strand(&self, semaphore: vk::Semaphore) {
        self.lock_unknown().semaphores.push(semaphore);
    }

    fn counted(&self, swapchain: &Swapchain) {
        self.swapchains.fetch_add(1, Ordering::Relaxed);
        self.image_bytes
            .fetch_add(swapchain.image_bytes, Ordering::Relaxed);
    }

    fn uncounted(&self, swapchain: &Swapchain) {
        self.swapchains.fetch_sub(1, Ordering::Relaxed);
        self.image_bytes
            .fetch_sub(swapchain.image_bytes, Ordering::Relaxed);
    }

    /// Swapchains that exist, retired ones included, and the estimated bytes of their images.
    pub(crate) fn estimate(&self) -> (u64, u64) {
        (
            self.swapchains.load(Ordering::Relaxed),
            self.image_bytes.load(Ordering::Relaxed),
        )
    }
}

impl SurfaceSlot {
    /// The surface's lock, with every note applied.
    fn lock(&self, surfaces: &Surfaces) -> MutexGuard<'_, SurfaceEntry> {
        let mut entry = self.entry.lock().unwrap_or_else(PoisonError::into_inner);
        self.apply(&mut entry, surfaces);
        entry
    }

    /// The surface's lock when it is free now, with every note applied.
    fn try_lock(&self, surfaces: &Surfaces) -> Option<MutexGuard<'_, SurfaceEntry>> {
        let mut entry = match self.entry.try_lock() {
            Ok(entry) => entry,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => return None,
        };
        self.apply(&mut entry, surfaces);
        Some(entry)
    }

    fn apply(&self, entry: &mut SurfaceEntry, surfaces: &Surfaces) {
        let notes = std::mem::take(&mut *self.notes.lock().unwrap_or_else(PoisonError::into_inner));
        for note in notes {
            match note {
                Note::Released => {
                    entry.released = true;
                    if let Some(current) = entry.current.take() {
                        entry.retired.push(current);
                    }
                }
                Note::Dropped(frame) => {
                    // Its image stays acquired and its semaphore keeps the signal of the acquire: the swapchain has to
                    // be configured again, and the semaphore ends with it.
                    let semaphore = vk::Semaphore::from_raw(frame.semaphore);
                    match entry.swapchain_mut(frame.swapchain) {
                        Some(swapchain) => {
                            swapchain.out = swapchain.out.saturating_sub(1);
                            swapchain.stranded.push(semaphore);
                        }
                        None => surfaces.strand(semaphore),
                    }
                    if entry
                        .current
                        .as_ref()
                        .is_some_and(|current| current.number == frame.swapchain)
                    {
                        entry.stale = true;
                    }
                }
            }
        }
    }

    /// Leave a note for the next holder of this surface's lock, without waiting for it.
    fn note(&self, note: Note, surfaces: &Surfaces) {
        self.notes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(note);
        // Applied at once when nobody holds the lock.
        drop(self.try_lock(surfaces));
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

impl Shown {
    /// Count it on the surface's counters, which a [`crate::PresentProbe`] reads.
    pub(crate) fn count(self, counters: &PresentCounters) {
        match self {
            Self::Presented => counters.mark_presented(),
            Self::Suboptimal => {
                counters.mark_presented();
                PresentCounters::bump(&counters.suboptimal);
            }
            Self::OutOfDate | Self::SurfaceLost | Self::Fault(_) => {
                PresentCounters::bump(&counters.failed)
            }
        }
    }
}

/// What the owner destroyed of the presentation.
pub(crate) enum PresentEnd {
    Swapchain {
        claim: ClaimInfo,
        swapchain: u64,
    },
    /// The surface; its window's target is the owner's to drop once it holds no lock.
    Surface {
        owned: Owned,
        target: Option<Box<dyn SurfaceTarget>>,
    },
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
    fn fns(&self) -> Result<&PresentFns, PresentFault> {
        self.presentation
            .as_ref()
            .ok_or(PresentFault::Refused(PresentRefusal::Foreign))
    }

    /// The slot of surface `key`, when it exists.
    fn surface_slot(&self, key: u64) -> Result<Arc<SurfaceSlot>, PresentFault> {
        self.surfaces
            .slot(key)
            .ok_or(refused(PresentRefusal::Released))
    }

    /// Swapchains of this generation that exist, and the estimated bytes of their images.
    pub(crate) fn swapchain_estimate(&self) -> (u64, u64) {
        self.surfaces.estimate()
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
        let slot = SurfaceSlot {
            entry: Mutex::new(SurfaceEntry {
                surface: made.surface,
                target: Some(made.target),
                owned,
                claim,
                counters,
                current: None,
                retired: Vec::new(),
                free: Vec::new(),
                waiting: Vec::new(),
                proofs: Vec::new(),
                proven_present: 0,
                released: false,
                lost: false,
                stale: false,
                ended: false,
            }),
            notes: Mutex::new(Vec::new()),
        };
        self.surfaces
            .slots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, Arc::new(slot));
        key
    }

    /// Destroy a surface that was never handed out. Returns the window's target for the caller to drop.
    pub(crate) fn discard_surface(&self, made: MadeSurface) -> Box<dyn SurfaceTarget> {
        if let Some(fns) = &self.presentation {
            // SAFETY: no swapchain was made on it.
            unsafe { fns.surface.destroy_surface(made.surface, None) };
        }
        made.target
    }
}
