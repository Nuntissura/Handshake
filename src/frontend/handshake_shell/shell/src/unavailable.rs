//! The unavailable state (MT-214 AC214-9): what the shell says when no device is available to draw on, whether none was
//! there at the start or none came back after a loss.
//!
//! The UI layer (Kumihimo) is not published, so nothing here is drawn, and no other renderer exists to draw it with: no
//! Direct3D, OpenGL or software path, in this crate or below it. The state is a typed value with three readable forms:
//! its typed exit ([`ExitKind::DeviceUnavailable`], code 4), its sentence for the accessibility tree
//! ([`UnavailableState::frame`]: the window's name is the headline and its one group's name is the sentence), and its
//! receipt line ([`UnavailableState::line`]), which names codes and numbers only. The drawn presentation waits for the UI
//! layer; the text, the code and the line are what this crate can state without drawing.

use handshake_shell_accessibility::SemanticFrame;
use handshake_shell_surface::{DeviceEvent, GaveUp};
use handshake_vulkan_renderer::Unavailable;

use crate::outcome::ExitKind;

/// What the state says first, whatever the cause.
pub const HEADLINE: &str = "Handshake cannot draw its window";

/// When the device became unavailable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum When {
    /// The device was never there: the owner refused to open.
    AtStart,
    /// A device was there, was lost, and none came back.
    AfterLoss,
}

impl When {
    /// A stable name, for diagnostics.
    pub const fn name(self) -> &'static str {
        match self {
            Self::AtStart => "at_start",
            Self::AfterLoss => "after_loss",
        }
    }
}

/// Why no device is available, from the owner's own typed reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Cause {
    /// The owner refused to open.
    Device(Unavailable),
    /// The device was lost and the presenter gave up.
    Recovery(GaveUp),
}

/// A typed, readable state: no device. It ends the program with [`ExitKind::DeviceUnavailable`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnavailableState {
    when: When,
    cause: Cause,
}

impl UnavailableState {
    /// The device did not open.
    pub const fn at_start(reason: Unavailable) -> Self {
        Self {
            when: When::AtStart,
            cause: Cause::Device(reason),
        }
    }

    /// The device was lost and did not come back.
    pub const fn after_loss(why: GaveUp) -> Self {
        Self {
            when: When::AfterLoss,
            cause: Cause::Recovery(why),
        }
    }

    /// When it became unavailable.
    pub const fn when(&self) -> When {
        self.when
    }

    /// Why.
    pub const fn cause(&self) -> Cause {
        self.cause
    }

    /// The owner's or presenter's own code for the cause; no driver text.
    pub fn code(&self) -> &'static str {
        match self.cause {
            Cause::Device(reason) => reason.key(),
            Cause::Recovery(why) => why.code(),
        }
    }

    /// How the program ends: the same typed exit for every cause.
    pub const fn end(&self) -> ExitKind {
        ExitKind::DeviceUnavailable
    }

    /// The sentence: what happened, in plain words, and that nothing else will draw. It names no path, title or driver.
    pub fn text(&self) -> String {
        let what = match self.cause {
            Cause::Device(reason) => match reason {
                Unavailable::LoaderMissing => "the system has no Vulkan loader",
                Unavailable::InstanceRefused => "the Vulkan driver refused to start",
                Unavailable::ApiBelowBaseline => "the Vulkan driver is older than Handshake needs",
                Unavailable::NoPhysicalDevice => "the system reports no graphics device",
                Unavailable::SoftwareDeviceRefused => "only a software graphics device is present, which Handshake does not use",
                Unavailable::LayeredDriverRefused => "only a graphics driver layered over another graphics interface is present, which Handshake does not use",
                Unavailable::NoSuitableDevice => "no graphics device can render and present here",
                Unavailable::RequiredDeviceMissing => "the graphics device Handshake requires is missing",
                Unavailable::DeviceCreationRefused => "the graphics driver refused to create the device",
                Unavailable::PhysicalDeviceLost => "the graphics device was lost while it was opened",
                Unavailable::OutOfMemory => "the graphics device is out of memory",
                Unavailable::OwnerAlreadyOpen => "the graphics device is already open in this process",
                Unavailable::FeatureMissing => "the graphics device lacks a feature Handshake requires",
                Unavailable::PresentationUnsupported => "the graphics device cannot present to windows here",
                Unavailable::Driver(_) => "the graphics driver reported an error",
            },
            Cause::Recovery(GaveUp::RepeatedLoss) => "the graphics device was lost again and again",
            Cause::Recovery(_) => "the graphics device was lost and could not be recovered",
        };
        let when = match self.when {
            When::AtStart => "at start",
            When::AfterLoss => "after it was lost",
        };
        format!(
            "{HEADLINE}: {what} ({when}, reason {}). Nothing else will draw it. Handshake exits with code {}.",
            self.code(),
            self.end().code()
        )
    }

    /// The accessibility tree of the state: the headline is the window's name and the sentence is the name of its one
    /// group, which an assistive tool reads (the shell frame of [`SemanticFrame::named_group`]).
    pub fn frame(&self) -> SemanticFrame {
        SemanticFrame::named_group(HEADLINE, &self.text())
    }

    /// The receipt line: codes and numbers, no content.
    pub fn line(&self) -> String {
        format!(
            "HSK_SHELL_UNAVAILABLE kind={} code={} when={} reason={} drawn=false drawn_waits_for=ui_layer fallback_renderer=none",
            self.end().name(),
            self.end().code(),
            self.when.name(),
            self.code()
        )
    }
}

/// What the presenter's report of the device means for the program: only a device that is gone for good ends it. A loss
/// is recovered by the presenter, and a rebind means every window draws again; neither is an end.
pub fn unavailable_after(event: DeviceEvent) -> Option<UnavailableState> {
    match event {
        DeviceEvent::GaveUp(why) => Some(UnavailableState::after_loss(why)),
        _ => None,
    }
}
