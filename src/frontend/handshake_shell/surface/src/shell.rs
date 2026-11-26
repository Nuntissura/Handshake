//! What the application on top sees: how presentation is configured, the handle its handlers get, and what it reports.

use handshake_shell_window::{
    CommandSource, Control, DispatchedWake, Event, Refusal, WindowKey, WindowSpec,
};
use handshake_vulkan_renderer::{
    AttachmentFormat, AuthorizationProvenance, GpuOwner, ModuleId, PresentMode, PresentProgress,
    QuotaPolicy, ShutdownReport, SwapchainInfo,
};

use crate::clear::{Background, ClearFrame};
use crate::error::PresentError;
use crate::host::Core;
use crate::readback::{FrameReadback, ReadbackError};
use crate::recovery::{DeviceEvent, DeviceState};
use crate::watch::PresentWatch;

/// Bytes each window's Display claim reserves unless the host says otherwise: four BGRA8 swapchain images of a
/// 3840x2160 window are 126.6 MiB (the owner's own estimate, `SwapchainInfo::image_bytes`, is not charged), one capture
/// buffer of the same window 31.6 MiB, and the rest headroom for a larger window or a fifth image.
pub const DEFAULT_RESERVE_PER_WINDOW: u64 = 256 << 20;

/// How the shell presents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentConfig {
    /// The host's quota policy. Without a configured one every window's claim is denied: the owner never takes an
    /// unknown quota for an unlimited one.
    pub policy: QuotaPolicy,
    /// The host's module number for the shell. Attribution only; no canonical number is verified here.
    pub module: ModuleId,
    /// The first client number: each window's client is this plus the window's number, which is never used again.
    pub client_base: u64,
    /// What each window's claim records as its authorization. All zero until the kernel issuer of MT-214 S5
    /// (NATIVE-RENDER-BOOT-001) exists: this crate authorizes nothing.
    pub provenance: AuthorizationProvenance,
    /// What each frame draws.
    pub frame: ClearFrame,
    /// The swapchain format asked for. The owner takes the first of its formats the surface offers when this one is
    /// not; [`WindowPresentation::swapchain`] says which runs.
    pub format: AttachmentFormat,
    /// The present mode asked for. FIFO when the surface does not offer it; the swapchain reports the one that runs.
    pub mode: PresentMode,
    /// Bytes each window's claim reserves.
    pub reserve_per_window: u64,
}

impl PresentConfig {
    /// Dark background, BGRA8 UNORM, FIFO, [`DEFAULT_RESERVE_PER_WINDOW`].
    pub fn new(policy: QuotaPolicy, module: ModuleId) -> Self {
        Self {
            policy,
            module,
            client_base: 1 << 32,
            provenance: AuthorizationProvenance([0; 32]),
            frame: ClearFrame::new(Background::DARK),
            format: AttachmentFormat::Bgra8Unorm,
            mode: PresentMode::Fifo,
            reserve_per_window: DEFAULT_RESERVE_PER_WINDOW,
        }
    }
}

/// Counters of the presenter. Mechanical numbers only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentReport {
    /// Surfaces made on windows, made again after a loss included.
    pub surfaces_made: u64,
    /// Surfaces released (closed windows, lost surfaces, shutdown).
    pub surfaces_released: u64,
    /// Surfaces made again after the owner reported them lost.
    pub surfaces_remade: u64,
    /// Window targets the owner dropped after it destroyed their surface. Each exactly once.
    pub targets_released: u64,
    /// Swapchain configurations asked for.
    pub configures: u64,
    /// Configurations that replaced a presented swapchain, which the owner then ends.
    pub swapchains_replaced: u64,
    /// Swapchains suspended because their window was minimised.
    pub suspends: u64,
    /// Frames the window crate handed out for windows with a surface.
    pub frames_due: u64,
    /// Frames the queue took, suboptimal ones included.
    pub frames_presented: u64,
    /// Part of `frames_presented` the swapchain no longer fitted.
    pub frames_suboptimal: u64,
    /// Frames that were not presented.
    pub frames_failed: u64,
    /// Acquires that found no free image (none waits).
    pub not_ready: u64,
    /// Frames drawn only so that the owner can end a replaced swapchain, or prove the presents of a closing window.
    pub retire_frames: u64,
    /// Looks at the owner on the presenter's timer.
    pub polls: u64,
    /// Frames asked for again by the presenter's timer.
    pub timed_frames: u64,
    /// Captures of presented frames read back.
    pub captures: u64,
    /// Window claims settled.
    pub claims_settled: u64,
    /// Window claims let go unsettled at shutdown; the owner reclaims them after their work.
    pub claims_abandoned: u64,
    /// Errors recorded, of every window.
    pub errors: u64,
    /// The last of them.
    pub last_error: Option<PresentError>,
    /// What the owner reported when it was closed at shutdown.
    pub shutdown: Option<ShutdownReport>,
    /// Times the loop refused to end because a window target was still held.
    pub finish_refused: u64,
    /// Closed windows whose presents were not proven ended in time, without `VK_EXT_swapchain_maintenance1`: their last
    /// swapchain and the window are kept until the owner closes (MT-214 SF-2).
    pub closes_unproven: u64,
    /// Device losses the owner reported and the presenter recovered from or gave up on (AC214-9). One per loss, however
    /// many windows saw it.
    pub device_losses: u64,
    /// Times the owner replaced the lost device for the presenter.
    pub device_rebinds: u64,
    /// Window claims moved onto the replacing device, with a surface made on it: each window draws again.
    pub windows_rebound: u64,
    /// Windows that stayed without a surface because their claim could not be moved or a surface not made.
    pub windows_unrecovered: u64,
    /// Captures that ended with a typed outcome because the device was lost under them.
    pub captures_lost: u64,
    /// Looks at the owner while it or a claim refused to move.
    pub rebind_looks: u64,
    /// Where the device stands.
    pub device: DeviceState,
}

impl Default for PresentReport {
    fn default() -> Self {
        Self {
            surfaces_made: 0,
            surfaces_released: 0,
            surfaces_remade: 0,
            targets_released: 0,
            configures: 0,
            swapchains_replaced: 0,
            suspends: 0,
            frames_due: 0,
            frames_presented: 0,
            frames_suboptimal: 0,
            frames_failed: 0,
            not_ready: 0,
            retire_frames: 0,
            polls: 0,
            timed_frames: 0,
            captures: 0,
            claims_settled: 0,
            claims_abandoned: 0,
            errors: 0,
            last_error: None,
            shutdown: None,
            finish_refused: 0,
            closes_unproven: 0,
            device_losses: 0,
            device_rebinds: 0,
            windows_rebound: 0,
            windows_unrecovered: 0,
            captures_lost: 0,
            rebind_looks: 0,
            device: DeviceState::Healthy,
        }
    }
}

/// How one window presents now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowPresentation {
    /// The swapchain, with the present mode, format, extent and image count that run: reported values, never assumed.
    /// `None` without one, and while suspended.
    pub swapchain: Option<SwapchainInfo>,
    /// Minimised: nothing is acquired.
    pub suspended: bool,
    /// Its surface was released and the window waits to be destroyed.
    pub closing: bool,
    /// What the owner counted for its surface.
    pub progress: Option<PresentProgress>,
    /// The last thing that kept it from presenting.
    pub last_error: Option<PresentError>,
}

/// The application on top of the presenter. Both methods run on the loop's thread.
pub trait ShellApp {
    /// Every event of the loop, after the presenter took from it what it needs. Frames of windows opened with
    /// [`Shell::open_window`] were answered already; frames of other windows are the application's.
    fn on_event(&self, shell: &Shell<'_>, event: &Event);

    /// A wake of a lane the application opened itself.
    fn on_wake(&self, shell: &Shell<'_>, wake: &DispatchedWake<'_>) {
        let _ = (shell, wake);
    }

    /// The device was lost, replaced or given up on (AC214-9). Called after the event or wake that caused it, with the
    /// windows' surfaces already released or made again. The default does nothing.
    fn on_device(&self, shell: &Shell<'_>, event: DeviceEvent) {
        let _ = (shell, event);
    }
}

/// The loop and the presenter, as the application's handlers use them.
pub struct Shell<'a> {
    pub(crate) control: &'a Control,
    pub(crate) core: &'a Core,
}

impl<'a> Shell<'a> {
    /// The window crate's loop, for everything that is not presentation.
    pub fn control(&self) -> &'a Control {
        self.control
    }

    /// The GPU owner. The host's authority: policy, totals, close.
    pub fn owner(&self) -> &'a GpuOwner {
        self.core.owner()
    }

    /// Ask for a window that presents. It is created hidden; before [`handshake_shell_window::WindowEvent::Created`]
    /// reaches the application it has a client handle, a Display claim, a surface and a swapchain, so it can be shown.
    ///
    /// # Errors
    ///
    /// [`Refusal`] of the window crate.
    pub fn open_window(
        &self,
        source: &CommandSource,
        spec: WindowSpec,
    ) -> Result<WindowKey, Refusal> {
        let key = self.control.create_window(source, spec)?;
        self.core.expect_window(key);
        Ok(key)
    }

    /// Something shown in the window changed: ask for a frame. Frames asked for before one is drawn become one.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownWindow`].
    pub fn damage(&self, window: WindowKey) -> Result<(), Refusal> {
        self.control.request_frame(window)?;
        self.core.publish(self.control, window);
        Ok(())
    }

    /// Close a window: release its surface, hide it, settle its claim, and destroy it once the owner let go of its
    /// target. Without `VK_EXT_swapchain_maintenance1` a surface that presented first proves its presents ended: one
    /// present on a new swapchain, whose image the owner acquires back on the presenter's later looks (MT-214 SF-2).
    /// Until that proof ended the window stays shown as it was, never activated, since every acquire of a hidden
    /// window's swapchain is out of date (E5-F2); then it is hidden, the surface released, and the window goes. One
    /// whose proof does not come within two seconds is hidden and released all the same and kept, hidden, until the
    /// owner closes ([`PresentReport::closes_unproven`]). A window that does not present is hidden and destroyed at once.
    ///
    /// # Errors
    ///
    /// [`Refusal`] of the window crate for the hide when it was made in this call, or for the destroy of a window
    /// without presentation; [`Refusal::UnknownWindow`] for a window that is gone.
    pub fn close_window(&self, window: WindowKey) -> Result<(), Refusal> {
        self.core.close_window(self.control, window)
    }

    /// Read back the next presented frame of `window`: the swapchain image itself, in the frame's own submission. Asks
    /// for that frame. [`Self::take_capture`] has it once the owner observed the submission finished.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownWindow`] for a window without a surface.
    pub fn capture_next(&self, window: WindowKey) -> Result<(), Refusal> {
        self.core.want_capture(window)?;
        self.damage(window)
    }

    /// The capture of `window`, once it is there. Taken out.
    pub fn take_capture(&self, window: WindowKey) -> Option<Result<FrameReadback, ReadbackError>> {
        self.core.take_capture(window)
    }

    /// How `window` presents now.
    pub fn presentation(&self, window: WindowKey) -> Option<WindowPresentation> {
        self.core.presentation(window)
    }

    /// Present progress of `window`, for a watcher on any thread (MT-215 HB-4).
    pub fn watch(&self, window: WindowKey) -> Option<PresentWatch> {
        self.core.watch(window)
    }

    /// The presenter's counters so far.
    pub fn report(&self) -> PresentReport {
        self.core.report()
    }

    /// Where the device stands: healthy, recovering from a loss, or failed (no other renderer is tried).
    pub fn device_state(&self) -> DeviceState {
        self.core.device_state()
    }

    /// End the loop with `exit_code`: shutdown begins, [`Event::ShutdownBegan`] reaches the application, which ends
    /// what watches the loop; then every surface is released, the claims settled, the owner closed, and the loop
    /// finished once no window target is held any more.
    ///
    /// # Errors
    ///
    /// [`Refusal::ShuttingDown`] when shutdown has begun.
    pub fn shutdown(&self, exit_code: i32) -> Result<(), Refusal> {
        self.core.set_exit(exit_code);
        self.control.begin_shutdown()
    }
}
