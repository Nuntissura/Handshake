//! Shell windows presenting real Vulkan frames (WP-KERNEL-012 MT-214 S2).
//!
//! This crate composes `handshake_shell_window` (windows, the event loop) with the central GPU owner
//! `handshake-vulkan-renderer` (MT-203), and neither of them knows the other: the window crate's `NativeWindowToken`
//! becomes the owner's `SurfaceTarget` here ([`ShellWindowTarget`]). Every window opened through [`Shell::open_window`]
//! gets a client handle of its own, an admitted Display claim, a surface and a swapchain on the owner's one device
//! (GUI-DESKTOP-025, GUI-DESKTOP-037, AC214-3). No window library, Vulkan binding or Win32 type is in this crate's
//! interface or code; no egui, eframe or wgpu in its closure.
//!
//! # How a host uses it
//!
//! 1. Implement [`ShellApp`]. Make a [`PresentConfig`] with the host's real quota policy and module number.
//! 2. [`PresentHost::open`] opens the owner with presentation; then `handshake_shell_window::run(loop_config, host)`.
//! 3. Open windows with [`Shell::open_window`]. On `WindowEvent::Created` the window has its surface and swapchain;
//!    show it with `WindowCommand::Show`.
//! 4. When something shown changes, [`Shell::damage`]. The presenter answers the window's frames itself.
//! 5. [`Shell::close_window`] to close one, [`Shell::shutdown`] to end the loop.
//!
//! # Frames
//!
//! Damage-driven (AC214-4): a frame is drawn only when the window crate hands one out, which it does when the operating
//! system exposed the window, when the host or a window change here damaged it (the table in README.md), or when the
//! presenter's own timer came due for a frame still owed: no image was free, or a replaced swapchain needs a
//! later present to be proven ended. An idle window submits no GPU work. A frame is acquired with timeout 0
//! (`PresentRefusal::NotReady` is retried on a timer, never waited for), cleared to the theme background and
//! presented; nothing on the frame path waits for the device, and `vkDeviceWaitIdle` is never called (the owner calls
//! no device-wide wait at all).
//!
//! # Swapchains
//!
//! - Resize, and restore from minimised: the swapchain is stale and configured again before the next frame, with the
//!   old one as `oldSwapchain`, which retires it (VUID-VkSwapchainCreateInfoKHR-oldSwapchain-01933).
//! - Minimise: suspended at once. On Win32 the surface's extent is the window's, which is (0, 0) when minimised, "and
//!   so a swapchain cannot be created until the size changes" (Vulkan specification, WSI, Win32 Platform).
//! - Scale change: damage; the size change that follows it is a resize.
//! - Out of date, suboptimal: configured again and drawn again. Surface lost: a new surface on a new token.
//! - A replaced swapchain is destroyed by the owner, once, after it observed its work and presents ended. With
//!   `VK_EXT_swapchain_maintenance1` its present fences prove it and the presenter only lets the owner look; without
//!   it, a later present of the same surface proven by the re-acquire of its image does, so the presenter draws a
//!   bounded burst of frames after a replacement ([`PresentReport::retire_frames`]).
//! - Close: the surface is released, the window hidden, its claim settled, and the window destroyed once the owner let
//!   go of its target, which it does once after it destroyed the surface. Without the extension that is when the owner
//!   is closed, for a surface that presented (the owner's rule).
//!
//! # Present progress
//!
//! [`Shell::watch`] gives a [`PresentWatch`] per window: the owner's `PresentProbe` and since when a frame is wanted
//! and not presented, readable on any thread, so a watcher can tell a live loop that never presents (MT-215 HB-4).
//!
//! # Headless frames
//!
//! [`render_offscreen`] draws the same [`ClearFrame`] into an owner image and reads it back; [`Shell::capture_next`]
//! reads back a presented swapchain image in its own submission. [`FrameReadback::compare`] compares either with the
//! frame, texel by texel.
//!
//! # Failure modes
//!
//! | Seen | Meaning | What to do |
//! | --- | --- | --- |
//! | `Unavailable` from [`PresentHost::open`] | no device that renders and presents here | the host shows the typed reason; no other renderer exists |
//! | [`PresentError::Admission`] | no quota policy, or no room for the window's claim | configure the policy; close windows |
//! | [`PresentError::Surface`] | the owner made no surface on the window | read the owner's key; the window stays without frames |
//! | [`PresentError::NotClearable`] | the surface's images lack transfer destination use | none here: needs a render-pass clear (Kumihimo's pipelines) |
//! | [`PresentError::Failing`] | eight frames in a row failed, or no image was free for about a second | the window asks for none on its own; the next change or damage tries again |
//! | [`PresentError::Frame`] with a device fault | the device was lost | rebind through the owner (AC214-9, a later slice) |
//! | [`PresentReport::finish_refused`] grows | a window target is still held at the end | the owner's device is held or still runs work; the process may end itself |
//! | [`PresentReading::never_presents`] | frames are wanted and none is presented | the renderer's or the GPU's failure, or the loop stopped |

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod clear;
#[cfg_attr(not(windows), allow(dead_code))]
mod decide;
mod error;
mod readback;
#[cfg_attr(not(windows), allow(dead_code))]
mod watch;

#[cfg(windows)]
mod host;
#[cfg(windows)]
mod shell;
#[cfg(windows)]
mod slot;
#[cfg(windows)]
mod target;

pub use clear::{
    compare_texels, Background, ClearFrame, CompareRefusal, Comparison, Mismatch, Texel,
    TOLERANCE,
};
pub use error::PresentError;
pub use readback::{render_offscreen, FrameReadback, PendingReadback, ReadbackError};
pub use watch::{PresentReading, PresentWatch};

#[cfg(windows)]
pub use host::PresentHost;
#[cfg(windows)]
pub use shell::{
    PresentConfig, PresentReport, Shell, ShellApp, WindowPresentation,
    DEFAULT_RESERVE_PER_WINDOW,
};
#[cfg(windows)]
pub use target::ShellWindowTarget;
