//! Presentation (MT-203 slice 2, G203-H2; MT-214 AC214-3): a typed surface on a window the host's window crate names,
//! its swapchain on the owner's device under an admitted claim, and frames acquired and presented through the owner's
//! queue without any wait on the frame path.
//!
//! # The window token
//!
//! This crate creates no window and depends on no window crate. The host hands it a [`SurfaceTarget`]: an object that
//! names the native window ([`NativeWindow`], plain integers) and keeps that window alive for as long as it exists. The
//! owner keeps the target until it destroyed the surface, so a surface never outlives its window. `handshake_shell_window`
//! supplies one without either crate knowing the other: its `NativeWindowToken` keeps its window from being destroyed
//! while a clone exists and reads out `Win32Window { hwnd, hinstance }`; the composing crate wraps a clone in a newtype
//! that implements [`SurfaceTarget`].
//!
//! # Swapchains and their end
//!
//! [`OwnerSurface::configure`] creates the swapchain for the window's current size, and creates it again on resize and
//! restore, passing the current one as `oldSwapchain` (which retires it, VUID-VkSwapchainCreateInfoKHR-oldSwapchain-01933).
//! A window without size (minimised) suspends it: no image is acquired until it is configured with a size again. A
//! retired swapchain is destroyed once (VUID-vkDestroySwapchainKHR-swapchain-01282,
//! VUID-vkDestroySemaphore-semaphore-05149), after the owner observed that no frame of it is out, the fences of every
//! submission that rendered into its images and of its acquires, and that its presents have ended. That proof is the
//! swapchain's own surface's, never another window's:
//! - with `VK_EXT_swapchain_maintenance1`, the fences of all its presents (VkSwapchainPresentFenceInfoKHR);
//! - without it, a present of a later swapchain of the same surface that is proven ended by an acquire handing back
//!   the same image (the Khronos `swapchain_recreation` sample: when "the first present operation of the new swapchain
//!   is done ... the old swapchain is no longer being presented").
//!
//! A swapchain that was never presented waits only for its work. Without the extension, the last swapchain of a
//! released surface that was presented has nothing to prove its presents (the Vulkan guide: no good solution in
//! unextended Vulkan): it is kept, observable in [`crate::OwnerMemoryUse::swapchains`], with its surface and its
//! window's target, until its device is destroyed after its queue drained.
//!
//! # Frames
//!
//! A frame is recorded on only in its own [`crate::GpuClient::present`], whose submission waits for its acquire and is
//! the last use of its swapchain; a plain submit refuses it ([`crate::RecordRefusal::FrameNotPresented`]).
//!
//! # No waits
//!
//! An acquire does not wait (`timeout` 0): no image now is [`PresentRefusal::NotReady`]. A present hands the frame to the
//! queue behind its submission and returns. The owner never calls `vkDeviceWaitIdle`, `vkQueueWaitIdle` or a fence wait.

use crate::ids::ClaimId;
use crate::owner::OwnerShared;
use crate::records::{ByteQuantity, ClaimInfo};
use crate::refusal::PresentRefusal;
use crate::resource::sealed::{Target, TargetOf};
use crate::resource::{
    AttachmentFormat, Extent, FrameKey, ImageTarget, ImageUsage, RenderAttachment,
};
use crate::Submission;
use std::num::NonZeroIsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How the owner is opened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OwnerOptions {
    /// Enable presentation to windows: the instance gets `VK_KHR_surface` and `VK_KHR_win32_surface`, the device
    /// `VK_KHR_swapchain`, and the device's queue must be able to present on this platform. Without it the owner renders
    /// offscreen only, and a surface is refused.
    pub presentation: bool,
}

/// A native window, as plain integers.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NativeWindow {
    /// The two numbers a Win32 surface is made from (`VkWin32SurfaceCreateInfoKHR`).
    Win32 {
        hwnd: NonZeroIsize,
        hinstance: NonZeroIsize,
    },
}

impl std::fmt::Debug for NativeWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The handles are not printed.
        match self {
            Self::Win32 { .. } => f.write_str("NativeWindow::Win32"),
        }
    }
}

/// What a surface is made on: the window, kept alive by this object for as long as it exists.
///
/// The owner keeps the target until it destroyed the surface, so a surface never outlives its window. A window crate
/// supplies one without either crate depending on the other: `handshake_shell_window`'s `NativeWindowToken` keeps its
/// window from being destroyed while a clone exists and reads out `Win32Window { hwnd, hinstance }`; the crate that
/// composes both wraps a clone in a newtype and implements this trait for it:
///
/// ```ignore
/// struct ShellWindow(handshake_shell_window::NativeWindowToken);
///
/// impl handshake_vulkan_renderer::SurfaceTarget for ShellWindow {
///     fn native_window(&self) -> handshake_vulkan_renderer::NativeWindow {
///         let window = self.0.win32();
///         handshake_vulkan_renderer::NativeWindow::Win32 {
///             hwnd: window.hwnd,
///             hinstance: window.hinstance,
///         }
///     }
/// }
/// ```
pub trait SurfaceTarget: Send + Sync + 'static {
    /// The window. Asked once, when the surface is made.
    fn native_window(&self) -> NativeWindow;
}

/// How frames are paced. FIFO is offered by every device; another mode that the surface does not offer is replaced by it,
/// and [`SwapchainInfo::mode`] says which one runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PresentMode {
    Fifo,
    Mailbox,
    Immediate,
}

impl PresentMode {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Fifo => "fifo",
            Self::Mailbox => "mailbox",
            Self::Immediate => "immediate",
        }
    }
}

/// What the host wants of a swapchain. The extent is used only where the window lets the swapchain decide; otherwise
/// the window's own size wins and is reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwapchainRequest {
    pub extent: Extent,
    pub format: AttachmentFormat,
    pub mode: PresentMode,
}

/// The swapchain a surface has now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwapchainInfo {
    /// Its number, unique within the owner.
    pub swapchain: u64,
    pub extent: Extent,
    pub format: AttachmentFormat,
    pub mode: PresentMode,
    pub images: u32,
    pub usage: ImageUsage,
    /// The presentation engine allocates the images; their size is not reported, only estimated from extent, format
    /// and count. It is not charged to the claim (whose reservation covers it).
    pub image_bytes: ByteQuantity,
}

/// What [`OwnerSurface::configure`] left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwapchainStatus {
    Ready(SwapchainInfo),
    /// The window has no size (minimised): nothing is acquired until it is configured with one.
    Suspended,
}

/// How a present went. In every case the frame's submission was queued and is observed like any other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentResult {
    Presented,
    /// Presented, but the swapchain no longer matches the window exactly: configure it again soon.
    Suboptimal,
    /// Not shown: the swapchain no longer matches the window. Configure it again.
    OutOfDate,
    /// Not shown: the surface is lost. Make a new one.
    SurfaceLost,
}

/// What [`crate::GpuClient::present`] returns.
#[derive(Debug)]
pub struct Presented {
    pub submission: Submission,
    pub result: PresentResult,
}

/// Counters of one surface, written by the owner and read by anyone through a [`PresentProbe`].
#[derive(Debug)]
pub(crate) struct PresentCounters {
    pub(crate) acquired: AtomicU64,
    pub(crate) not_ready: AtomicU64,
    pub(crate) presented: AtomicU64,
    pub(crate) suboptimal: AtomicU64,
    pub(crate) failed: AtomicU64,
    pub(crate) abandoned: AtomicU64,
    pub(crate) swapchains: AtomicU64,
    /// Nanoseconds after `since` of the last successful present, plus one; 0 for none.
    last_present: AtomicU64,
    since: Instant,
}

impl PresentCounters {
    pub(crate) fn new() -> Self {
        Self {
            acquired: AtomicU64::new(0),
            not_ready: AtomicU64::new(0),
            presented: AtomicU64::new(0),
            suboptimal: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            abandoned: AtomicU64::new(0),
            swapchains: AtomicU64::new(0),
            last_present: AtomicU64::new(0),
            since: Instant::now(),
        }
    }

    pub(crate) fn bump(counter: &AtomicU64) {
        counter.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn mark_presented(&self) {
        Self::bump(&self.presented);
        let nanos = u64::try_from(self.since.elapsed().as_nanos()).unwrap_or(u64::MAX - 1);
        self.last_present
            .store(nanos.saturating_add(1), Ordering::Release);
    }
}

/// How far one surface got, read without any lock of the owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentProgress {
    pub acquired: u64,
    /// Acquires that found no image ready.
    pub not_ready: u64,
    /// Presents the queue took, suboptimal ones included.
    pub presented: u64,
    pub suboptimal: u64,
    /// Acquires and presents that failed (out of date, surface lost, device faults).
    pub failed: u64,
    /// Frames dropped without being presented.
    pub abandoned: u64,
    pub swapchains: u64,
    /// Time since the last present the queue took. `None` when none was.
    pub since_last_present: Option<Duration>,
}

/// A cheap, `Send` and `Sync` view of one surface's progress for a watcher (MT-215 HB-4): a loop that wants frames and
/// never presents shows as `presented` standing still and `since_last_present` growing. Reading it touches no lock of the
/// owner and makes no call per frame.
#[derive(Clone, Debug)]
pub struct PresentProbe {
    pub(crate) counters: Arc<PresentCounters>,
}

impl PresentProbe {
    pub fn read(&self) -> PresentProgress {
        let counters = &self.counters;
        let load = |counter: &AtomicU64| counter.load(Ordering::Relaxed);
        let last = counters.last_present.load(Ordering::Acquire);
        PresentProgress {
            acquired: load(&counters.acquired),
            not_ready: load(&counters.not_ready),
            presented: load(&counters.presented),
            suboptimal: load(&counters.suboptimal),
            failed: load(&counters.failed),
            abandoned: load(&counters.abandoned),
            swapchains: load(&counters.swapchains),
            since_last_present: (last > 0).then(|| {
                counters
                    .since
                    .elapsed()
                    .saturating_sub(Duration::from_nanos(last - 1))
            }),
        }
    }
}

/// Whose a surface is.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SurfaceOrigin {
    pub(crate) ledger: u64,
    pub(crate) seat: u64,
    pub(crate) claim: ClaimInfo,
    pub(crate) key: u64,
}

/// A surface the owner made on a window for a claim. Dropping it releases it: its swapchains and the surface are
/// destroyed once their work and presents ended, and only then is the window's target dropped. Not `Clone`.
pub struct OwnerSurface {
    pub(crate) owner: Arc<OwnerShared>,
    pub(crate) origin: SurfaceOrigin,
    pub(crate) counters: Arc<PresentCounters>,
}

impl std::fmt::Debug for OwnerSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnerSurface")
            .field("claim", &self.origin.claim.id)
            .finish_non_exhaustive()
    }
}

impl OwnerSurface {
    pub fn claim(&self) -> ClaimId {
        self.origin.claim.id
    }

    pub fn generation(&self) -> u64 {
        self.origin.claim.generation
    }

    /// Create the swapchain for the window as it is now, or create it again (resize, restore, out of date). A window
    /// without size suspends it. Does not wait for the device.
    pub fn configure(&self, request: &SwapchainRequest) -> Result<SwapchainStatus, PresentRefusal> {
        self.owner.configure_surface(&self.origin, request)
    }

    /// Stop acquiring until the next [`Self::configure`] (the window was minimised). The swapchain is kept so that it is
    /// the old one of the next.
    pub fn suspend(&self) -> Result<(), PresentRefusal> {
        self.owner.suspend_surface(&self.origin)
    }

    /// Acquire the next image without waiting: [`PresentRefusal::NotReady`] when none is ready now.
    pub fn acquire(&self) -> Result<Frame, PresentRefusal> {
        self.owner.acquire_frame(&self.origin, &self.counters)
    }

    /// The swapchain, when there is one and it is not suspended.
    pub fn swapchain(&self) -> Option<SwapchainInfo> {
        self.owner.swapchain_info(&self.origin)
    }

    pub fn probe(&self) -> PresentProbe {
        PresentProbe {
            counters: Arc::clone(&self.counters),
        }
    }
}

impl Drop for OwnerSurface {
    fn drop(&mut self) {
        self.owner.release_surface(&self.origin);
    }
}

/// One acquired swapchain image, to be recorded into and presented with [`crate::GpuClient::present`]. Its attachment
/// starts in [`crate::ImageLayout::Undefined`] and must be left in [`crate::ImageLayout::Present`]. Dropping it without
/// presenting it leaves the image acquired: the swapchain must be configured again.
pub struct Frame {
    pub(crate) owner: Arc<OwnerShared>,
    pub(crate) surface: SurfaceOrigin,
    pub(crate) swapchain: u64,
    pub(crate) index: u32,
    pub(crate) semaphore: u64,
    pub(crate) attachment: RenderAttachment,
    pub(crate) suboptimal: bool,
    pub(crate) counters: Arc<PresentCounters>,
    /// Taken by a present.
    pub(crate) pending: bool,
}

impl std::fmt::Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frame")
            .field("swapchain", &self.swapchain)
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}

impl Frame {
    pub fn attachment(&self) -> &RenderAttachment {
        &self.attachment
    }

    pub fn image_index(&self) -> u32 {
        self.index
    }

    pub fn swapchain(&self) -> u64 {
        self.swapchain
    }

    /// The acquire said that the swapchain no longer matches the window exactly.
    pub fn is_suboptimal(&self) -> bool {
        self.suboptimal
    }

    pub(crate) fn key(&self) -> FrameKey {
        (self.surface.key, self.swapchain, self.index)
    }
}

impl Target for Frame {
    fn target(&self) -> TargetOf {
        TargetOf {
            ledger: self.surface.ledger,
            seat: self.surface.seat,
            generation: self.surface.claim.generation,
            image: self.attachment.image,
            extent: self.attachment.extent,
            format: self.attachment.format,
            usage: self.attachment.usage,
            frame: Some(self.key()),
        }
    }
}

impl ImageTarget for Frame {}

impl Drop for Frame {
    fn drop(&mut self) {
        if self.pending {
            PresentCounters::bump(&self.counters.abandoned);
            self.owner.abandon_frame(self);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PresentCounters, PresentProbe};
    use crate::vulkan::{QueueFault, Shown};
    use std::sync::Arc;
    use std::time::Duration;

    /// The counting a present does (`vulkan/present/frames.rs` ends every present with `Shown::count`), read through
    /// the probe a watcher holds.
    #[test]
    fn a_probe_shows_a_loop_that_stops_presenting() {
        let counters = Arc::new(PresentCounters::new());
        let probe = PresentProbe {
            counters: Arc::clone(&counters),
        };
        let never = probe.read();
        assert_eq!((never.presented, never.since_last_present), (0, None));
        PresentCounters::bump(&counters.acquired);
        Shown::Presented.count(&counters);
        let fresh = probe.read();
        assert_eq!((fresh.acquired, fresh.presented), (1, 1));
        let since = fresh.since_last_present.expect("presented");
        std::thread::sleep(Duration::from_millis(30));
        // Presents that are not shown move nothing but the failures.
        for failed in [
            Shown::OutOfDate,
            Shown::SurfaceLost,
            Shown::Fault(QueueFault::OutOfMemory),
        ] {
            failed.count(&counters);
        }
        let stale = probe.read();
        assert_eq!((stale.presented, stale.failed), (1, 3), "standing still");
        let later = stale.since_last_present.expect("presented");
        assert!(
            later >= since + Duration::from_millis(30),
            "{since:?} {later:?}"
        );
        Shown::Suboptimal.count(&counters);
        let again = probe.read();
        assert_eq!((again.presented, again.suboptimal), (2, 1));
        assert!(again.since_last_present.expect("presented") < later);
    }
}
