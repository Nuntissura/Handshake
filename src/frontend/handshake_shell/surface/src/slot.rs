//! One window's presentation: its client handle, its Display claim, its surface and what the presenter knows of its
//! swapchain. The GPU calls are here; when they are made is `host.rs`'s, what follows from them `decide.rs`'s.

use handshake_shell_window::{PhysicalSize, WindowKey};
use handshake_vulkan_renderer::{
    AuthorizationProvenance, Claim, ClaimDisposition, ClaimRequest, ClientId, Extent, GpuClient,
    GpuOwner, ImageLayout, OperationId, OperationKind, OwnerSurface, PresentRefusal,
    PresentResult, Presented, RecordAborted, SettleOutcome, SettleRefusal, Settlement,
    SurfaceTarget, SwapchainInfo, SwapchainRequest, SwapchainStatus,
};

use crate::clear::ClearFrame;
use crate::decide::{Swap, Tried};
use crate::error::PresentError;
use crate::shell::PresentConfig;
use crate::readback::{readback_buffer, FrameReadback, PendingReadback, ReadbackError};
use crate::watch::PresentWatch;

/// A capture of a presented frame (the swapchain image itself, read back in the frame's own submission).
pub(crate) enum Capture {
    None,
    /// The next presented frame is read back.
    Wanted,
    Pending(Box<PendingReadback>),
    Done(Result<FrameReadback, ReadbackError>),
}

/// How a claim's settlement stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Settled {
    /// This call settled it (and released what it retained).
    Now,
    /// There is none (any more).
    Nothing,
    /// Work or resources of it may still exist: again later.
    Waiting,
    /// Refused for a reason that does not pass: recorded, and the claim is let go.
    Refused(SettleRefusal),
}

pub(crate) struct Slot {
    pub(crate) key: WindowKey,
    client: GpuClient,
    claim: Option<Claim>,
    surface: Option<OwnerSurface>,
    pub(crate) swap: Swap,
    info: Option<SwapchainInfo>,
    /// The current swapchain was presented: replacing it leaves the owner a swapchain to end.
    presented_on_current: bool,
    pub(crate) watch: PresentWatch,
    pub(crate) closing: bool,
    pub(crate) destroy_asked: bool,
    pub(crate) error: Option<PresentError>,
    pub(crate) failures: u32,
    /// Suboptimal presents in a row: drawn again at once only twice, so a surface that stays suboptimal does not draw
    /// without end.
    pub(crate) suboptimal: u32,
    /// Acquires in a row that found no free image.
    pub(crate) not_ready: u32,
    pub(crate) retire_frames: u32,
    pub(crate) capture: Capture,
}

impl Slot {
    /// A client handle, a Display claim and a surface for the window `target` names.
    pub(crate) fn open(
        owner: &GpuOwner,
        config: &PresentConfig,
        key: WindowKey,
        target: Box<dyn SurfaceTarget>,
    ) -> Result<Self, PresentError> {
        let client = owner
            .attach_client(
                config.module,
                ClientId(config.client_base.saturating_add(key.number())),
            )
            .map_err(PresentError::Attach)?;
        let claim = admit(&client, key, config.reserve_per_window, config.provenance)?;
        let surface = client
            .create_surface(&claim, target)
            .map_err(PresentError::Surface)?;
        let watch = PresentWatch::new();
        watch.set_probe(Some(surface.probe()));
        Ok(Self {
            key,
            client,
            claim: Some(claim),
            surface: Some(surface),
            swap: Swap::None,
            info: None,
            presented_on_current: false,
            watch,
            closing: false,
            destroy_asked: false,
            error: None,
            failures: 0,
            suboptimal: 0,
            not_ready: 0,
            retire_frames: 0,
            capture: Capture::None,
        })
    }

    pub(crate) fn swapchain(&self) -> Option<SwapchainInfo> {
        self.surface.as_ref().and_then(OwnerSurface::swapchain)
    }

    pub(crate) fn has_surface(&self) -> bool {
        self.surface.is_some()
    }

    /// A swapchain of the window exists, suspended ones included: the owner counts it as live.
    pub(crate) fn holds_swapchain(&self) -> bool {
        self.surface.is_some() && self.info.is_some()
    }

    /// Create the swapchain for the window's size now, or create it again with the current one as `oldSwapchain`
    /// (VUID-VkSwapchainCreateInfoKHR-oldSwapchain-01933). A window without size suspends it: on Win32 a surface's
    /// extent is always the window's, and "the window size may become (0, 0) ... (e.g. when the window is minimized),
    /// and so a swapchain cannot be created until the size changes" (Vulkan specification, WSI, Win32 Platform).
    /// Returns whether a presented swapchain was replaced, which the owner ends once its presents are proven ended.
    pub(crate) fn configure(
        &mut self,
        inner: PhysicalSize,
        config: &PresentConfig,
    ) -> Result<bool, PresentRefusal> {
        let surface = self.surface.as_ref().ok_or(PresentRefusal::Released)?;
        let request = SwapchainRequest {
            extent: Extent {
                width: inner.width,
                height: inner.height,
            },
            format: config.format,
            mode: config.mode,
        };
        let before = self.info.map(|info| info.swapchain);
        match surface.configure(&request) {
            Ok(SwapchainStatus::Ready(info)) => {
                self.swap = Swap::Ready;
                self.info = Some(info);
                let replaced = before.is_some_and(|old| old != info.swapchain);
                let presented = std::mem::take(&mut self.presented_on_current);
                Ok(replaced && presented)
            }
            // The old swapchain is kept, as the old one of the next configuration.
            Ok(SwapchainStatus::Suspended) => {
                self.swap = Swap::Suspended;
                Ok(false)
            }
            Err(refusal) => {
                self.swap = Swap::None;
                Err(refusal)
            }
        }
    }

    /// Stop acquiring: the window was minimised.
    pub(crate) fn suspend(&mut self) -> Result<(), PresentRefusal> {
        if let Some(surface) = &self.surface {
            surface.suspend()?;
        }
        self.swap = Swap::Suspended;
        Ok(())
    }

    /// Acquire without waiting, clear the image to the frame and present it; read it back too when a capture is wanted.
    pub(crate) fn draw(&mut self, frame: &ClearFrame) -> Tried {
        let (Some(surface), Some(claim)) = (self.surface.as_ref(), self.claim.as_ref()) else {
            return Tried::Refused(PresentRefusal::Released);
        };
        let Some(info) = self.info else {
            return Tried::Refused(PresentRefusal::NoSwapchain);
        };
        // Checked before the acquire, so an image that could not be cleared is never shown.
        if !info.usage.transfer_dst {
            return Tried::NotClearable;
        }
        let image = match surface.acquire() {
            Ok(image) => image,
            Err(refusal) => return Tried::Refused(refusal),
        };
        let attachment = *image.attachment();
        let capture = match self.capture {
            Capture::Wanted if !attachment.usage.transfer_src => {
                self.capture = Capture::Done(Err(ReadbackError::NotReadable));
                None
            }
            Capture::Wanted => {
                match readback_buffer(&self.client, claim, attachment.format, attachment.extent) {
                    Ok(buffer) => Some(buffer),
                    Err(error) => {
                        self.capture = Capture::Done(Err(error));
                        None
                    }
                }
            }
            _ => None,
        };
        let presented = self.client.present(claim, image, |recording, image| {
            // Leaves the image in PRESENT_SRC_KHR (VUID-VkPresentInfoKHR-pImageIndices-01430).
            let drawn = match &capture {
                None => frame.record(
                    recording,
                    image,
                    attachment.format,
                    ImageLayout::Undefined,
                    ImageLayout::Present,
                ),
                Some(buffer) => frame
                    .record(
                        recording,
                        image,
                        attachment.format,
                        ImageLayout::Undefined,
                        ImageLayout::TransferSrc,
                    )
                    .and_then(|()| {
                        recording.read_back(
                            image,
                            ImageLayout::TransferSrc,
                            buffer,
                            ImageLayout::Present,
                        )
                    }),
            };
            drawn.map_err(|_| RecordAborted)
        });
        match presented {
            Ok(Presented { submission, result }) => {
                let shown = matches!(
                    result,
                    PresentResult::Presented | PresentResult::Suboptimal
                );
                if shown {
                    self.presented_on_current = true;
                    if let Some(buffer) = capture {
                        self.capture = Capture::Pending(Box::new(PendingReadback::new(
                            None,
                            buffer,
                            submission,
                            attachment.format,
                            attachment.extent,
                        )));
                    }
                }
                Tried::Shown(result)
            }
            Err(refusal) => Tried::Refused(refusal),
        }
    }

    /// Look at a pending capture without waiting. Returns whether it is still pending.
    pub(crate) fn poll_capture(&mut self) -> bool {
        let Capture::Pending(pending) = &self.capture else {
            return false;
        };
        match pending.poll(&self.client) {
            Ok(None) => true,
            Ok(Some(readback)) => {
                self.capture = Capture::Done(Ok(readback));
                false
            }
            Err(error) => {
                self.capture = Capture::Done(Err(error));
                false
            }
        }
    }

    /// Release the surface: the owner destroys its swapchains and the surface once their work and presents ended, and
    /// then drops the window's target. Returns whether there was one.
    pub(crate) fn release_surface(&mut self) -> bool {
        self.info = None;
        self.swap = Swap::None;
        self.presented_on_current = false;
        if let Capture::Pending(_) | Capture::Wanted = self.capture {
            self.capture = Capture::None;
        }
        self.watch.set_probe(None);
        self.surface.take().is_some()
    }

    /// Make the surface again on the window `target` names, after the old one was lost.
    pub(crate) fn remake_surface(
        &mut self,
        target: Box<dyn SurfaceTarget>,
    ) -> Result<(), PresentError> {
        self.release_surface();
        let claim = self.claim.as_ref().ok_or(PresentError::Frame(PresentRefusal::Released))?;
        let surface = self
            .client
            .create_surface(claim, target)
            .map_err(PresentError::Surface)?;
        self.watch.set_probe(Some(surface.probe()));
        self.surface = Some(surface);
        self.swap = Swap::Stale;
        Ok(())
    }

    /// Settle the window's claim once, with what the owner observed, and release what it retains. Only after the
    /// surface was released.
    pub(crate) fn settle(&mut self) -> Settled {
        if self.surface.is_some() {
            return Settled::Waiting;
        }
        let Some(claim) = self.claim.as_ref() else {
            return Settled::Nothing;
        };
        let _ = self.client.poll();
        let mut outcome = SettleOutcome::Completed;
        for _ in 0..2 {
            match self.client.settle(claim, Settlement::clean(outcome)) {
                Ok(receipt) => {
                    if let ClaimDisposition::Retained { .. } = receipt.disposition {
                        let _ = self.client.release_retained(claim);
                    }
                    self.claim = None;
                    return Settled::Now;
                }
                Err(SettleRefusal::OutcomeContradicted { observed }) => {
                    outcome = if observed.lost > 0 {
                        SettleOutcome::Lost
                    } else if observed.completed == 0 {
                        SettleOutcome::NoEffect
                    } else {
                        SettleOutcome::Completed
                    };
                }
                Err(
                    SettleRefusal::InFlight { .. }
                    | SettleRefusal::LeaseOutstanding { .. }
                    | SettleRefusal::ResourcesOutstanding { .. },
                ) => return Settled::Waiting,
                Err(
                    SettleRefusal::DoubleRelease
                    | SettleRefusal::LateRelease
                    | SettleRefusal::AlreadySettled,
                ) => {
                    self.claim = None;
                    return Settled::Nothing;
                }
                Err(refusal) => {
                    self.claim = None;
                    return Settled::Refused(refusal);
                }
            }
        }
        Settled::Waiting
    }

    /// Let the claim go unsettled: the owner closes it once it observed the end of its work.
    pub(crate) fn abandon_claim(&mut self) -> bool {
        self.claim.take().is_some()
    }

    pub(crate) fn holds_claim(&self) -> bool {
        self.claim.is_some()
    }
}

fn admit(
    client: &GpuClient,
    key: WindowKey,
    reserve_bytes: u64,
    provenance: AuthorizationProvenance,
) -> Result<Claim, PresentError> {
    let request = ClaimRequest {
        operation: OperationKind::Display,
        operation_id: OperationId(u128::from(key.number())),
        reserve_bytes,
        provenance,
    };
    client.admit(&request).map_err(PresentError::Admission)
}
