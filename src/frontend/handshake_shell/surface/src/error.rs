//! Why a window shows no frame. Closed codes carrying the owner's own refusals; no driver text, title or content.

use handshake_vulkan_renderer::{
    AdmissionDenial, AttachRefusal, PresentRefusal, ResourceRefusal, SettleRefusal,
};

use crate::readback::ReadbackError;

/// The last thing that kept a window from presenting, or from ending cleanly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PresentError {
    /// The owner issued no client handle for the window.
    Attach(AttachRefusal),
    /// The window's Display claim was not admitted: no quota policy, or no room in it.
    Admission(AdmissionDenial),
    /// The owner made no surface on the window.
    Surface(ResourceRefusal),
    /// The window crate gave no native token for the window: it is gone, or the loop is ending.
    NoToken,
    /// No swapchain was configured.
    Configure(PresentRefusal),
    /// A frame was not acquired or not presented.
    Frame(PresentRefusal),
    /// The surface offers its images without transfer destination use, and the frame is a clear
    /// (VUID-vkCmdClearColorImage-image-00002): nothing is acquired, so nothing undefined is ever shown.
    NotClearable,
    /// Eight frames failed in a row, or no image was free for about a second; the window asks for none on its own
    /// until something changes. Its watch shows the wanted frame waiting (MT-215 HB-4).
    Failing,
    /// The window's claim did not settle.
    Settle(SettleRefusal),
    /// A capture of a presented frame failed.
    Capture(ReadbackError),
}

impl PresentError {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::Attach(_) => "surface.attach",
            Self::Admission(_) => "surface.admission",
            Self::Surface(_) => "surface.create",
            Self::NoToken => "surface.no_token",
            Self::Configure(_) => "surface.configure",
            Self::Frame(_) => "surface.frame",
            Self::NotClearable => "surface.not_clearable",
            Self::Failing => "surface.failing",
            Self::Settle(_) => "surface.settle",
            Self::Capture(_) => "surface.capture",
        }
    }

    /// The owner's own code inside, when there is one.
    pub fn owner_key(self) -> Option<&'static str> {
        match self {
            Self::Attach(refusal) => Some(refusal.key()),
            Self::Admission(denial) => Some(denial.key()),
            Self::Surface(refusal) => Some(refusal.key()),
            Self::Configure(refusal) | Self::Frame(refusal) => Some(refusal.key()),
            Self::Settle(refusal) => Some(refusal.key()),
            Self::Capture(error) => Some(error.code()),
            Self::NoToken | Self::NotClearable | Self::Failing => None,
        }
    }
}
