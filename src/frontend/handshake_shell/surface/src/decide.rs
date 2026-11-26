//! What a window's presentation does next, decided without a window or a device: which events make the swapchain
//! stale, suspend it or damage the window, what a frame's result means, and when the presenter's own timer is due.
//!
//! Frames are damage-driven (AC214-4). A frame is drawn when the window crate hands one out, and it hands one out only
//! when something asked: the operating system exposed the window, the host or an event here damaged it, or a timer
//! armed here for a frame that is still owed came due. Input damages nothing in this slice, since nothing drawn
//! depends on it. A frame that failed is owed again; one that was presented is owed to nobody.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use handshake_shell_window::{Display, FrameOutcome, WindowEvent, WindowKey};
use handshake_vulkan_renderer::{PresentRefusal, PresentResult};

/// The swapchain of one window, as the presenter knows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Swap {
    /// None was made yet, or the last configuration failed.
    None,
    /// Made for the window as it was last seen.
    Ready,
    /// The window has no size: nothing is acquired until it is configured with one.
    Suspended,
    /// The window changed: configure before the next acquire.
    Stale,
}

/// What an event of the window asks of its presentation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Steps {
    /// Stop acquiring now: the window has no size (minimised).
    pub(crate) suspend: bool,
    /// Configure before the next frame: the window's size is another now, or it is back from minimised.
    pub(crate) stale: bool,
    /// Ask for a frame: what is shown no longer fits.
    pub(crate) damage: bool,
}

pub(crate) fn on_window_event(swap: Swap, event: &WindowEvent) -> Steps {
    let stale_and_damaged = Steps {
        stale: true,
        damage: true,
        ..Steps::default()
    };
    match event {
        // Recreated on the next frame, so a drag that resizes many times makes one swapchain per frame drawn.
        WindowEvent::Resized { inner, .. } if !inner.is_empty() => stale_and_damaged,
        WindowEvent::DisplayChanged {
            display: Display::Minimized,
        } => Steps {
            suspend: true,
            ..Steps::default()
        },
        WindowEvent::DisplayChanged {
            display: Display::Normal | Display::Maximized,
        } if swap != Swap::Ready => stale_and_damaged,
        // The logical size stays; a physical change follows as `Resized`.
        WindowEvent::ScaleChanged { .. } => Steps {
            damage: true,
            ..Steps::default()
        },
        _ => Steps::default(),
    }
}

/// How one frame went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tried {
    /// The queue took the present.
    Shown(PresentResult),
    /// No frame: configuring, acquiring or presenting was refused.
    Refused(PresentRefusal),
    /// The swapchain's images cannot be cleared.
    NotClearable,
}

/// When to try a frame again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Again {
    No,
    /// At once: the frame is owed and nothing has to pass first.
    Now,
    /// After a while: no image is free yet, or the window's old swapchain is still ending.
    After(Duration),
}

/// What the presenter does after a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct After {
    pub(crate) outcome: FrameOutcome,
    pub(crate) stale: bool,
    pub(crate) new_surface: bool,
    pub(crate) again: Again,
    /// Not retried on its own: recorded as the window's error.
    pub(crate) stop: bool,
}

/// How long after an acquire found no free image the frame is tried again. FIFO at 60 Hz frees one each 16.7 ms.
pub(crate) const NOT_READY_RETRY: Duration = Duration::from_millis(4);

/// How long after the window still had another surface's swapchain (a lost device's, ending) it is tried again.
pub(crate) const WINDOW_IN_USE_RETRY: Duration = Duration::from_millis(50);

/// Frames failing in a row before the window asks for none on its own.
pub(crate) const MAX_FAILURES: u32 = 8;

/// Acquires in a row that found no free image before the window asks for none on its own: about a second of retries,
/// sixty vertical blanks at 60 Hz in which the presentation engine kept every image.
pub(crate) const MAX_NOT_READY: u32 = 250;

pub(crate) fn after_frame(tried: Tried) -> After {
    let failed = After {
        outcome: FrameOutcome::Failed,
        stale: false,
        new_surface: false,
        again: Again::No,
        stop: false,
    };
    match tried {
        Tried::Shown(PresentResult::Presented) => After {
            outcome: FrameOutcome::Presented,
            ..failed
        },
        // Shown, at a size that no longer fits: drawn again at once with a swapchain that does.
        Tried::Shown(PresentResult::Suboptimal) => After {
            outcome: FrameOutcome::Presented,
            stale: true,
            again: Again::Now,
            ..failed
        },
        Tried::Shown(PresentResult::OutOfDate)
        | Tried::Refused(
            PresentRefusal::OutOfDate | PresentRefusal::NoSwapchain | PresentRefusal::RecordAborted,
        ) => After {
            stale: true,
            again: Again::Now,
            ..failed
        },
        Tried::Shown(PresentResult::SurfaceLost) | Tried::Refused(PresentRefusal::SurfaceLost) => {
            After {
                new_surface: true,
                again: Again::Now,
                ..failed
            }
        }
        Tried::Refused(PresentRefusal::NotReady) => After {
            again: Again::After(NOT_READY_RETRY),
            ..failed
        },
        Tried::Refused(PresentRefusal::WindowInUse) => After {
            again: Again::After(WINDOW_IN_USE_RETRY),
            ..failed
        },
        // The window has no size after all: nothing to show until it has one, which configures it again.
        Tried::Refused(PresentRefusal::Suspended) => After {
            outcome: FrameOutcome::Unchanged,
            ..failed
        },
        Tried::NotClearable
        | Tried::Refused(
            PresentRefusal::FormatUnsupported
            | PresentRefusal::Foreign
            | PresentRefusal::Released
            | PresentRefusal::NotAuthorized(_)
            | PresentRefusal::Device(_),
        ) => After {
            stop: true,
            ..failed
        },
    }
}

/// Extra frames a window may draw after its swapchain was replaced, without
/// `VK_EXT_swapchain_maintenance1`: the owner ends the old one once a present of the new one is proven ended by the
/// re-acquire of its image, which takes at most one acquire more than the swapchain has images when they come back in
/// order, and twice that otherwise. The burst is bounded: an idle window stops drawing after it.
pub(crate) fn retire_frames(images: u32) -> u32 {
    images.saturating_mul(2).saturating_add(2)
}

/// How long after a look at the owner the next one is due, for the `step`-th look in a row: 8 ms doubling to 512 ms.
pub(crate) fn poll_delay(step: u32) -> Duration {
    Duration::from_millis(8 << step.min(6))
}

/// Looks at the owner in a row before the presenter stops looking on its own (about 10 s).
pub(crate) const POLL_LOOKS: u32 = 24;

/// How long the presenter waits between frames it draws only to end an old swapchain.
pub(crate) const RETIRE_FRAME_DELAY: Duration = Duration::from_millis(16);

/// What the presenter's own timer is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Need {
    /// A frame that is owed and waits for an image.
    Frame(WindowKey),
    /// A look at the owner: old swapchains, captures, closing windows.
    Poll,
    /// Another try to end the loop.
    Finish,
}

/// When each need is due. The earliest one is what the presenter's wake lane is asked for.
#[derive(Debug, Default)]
pub(crate) struct Schedule {
    due: BTreeMap<Need, Instant>,
}

impl Schedule {
    /// `need` is due at `at`, or earlier when it already was.
    pub(crate) fn arm(&mut self, need: Need, at: Instant) {
        let due = self.due.entry(need).or_insert(at);
        *due = (*due).min(at);
    }

    pub(crate) fn cancel(&mut self, need: Need) {
        self.due.remove(&need);
    }

    pub(crate) fn next(&self) -> Option<Instant> {
        self.due.values().min().copied()
    }

    /// The needs due at `now`, taken out.
    pub(crate) fn take_due(&mut self, now: Instant) -> Vec<Need> {
        let due: Vec<Need> = self
            .due
            .iter()
            .filter(|(_, at)| **at <= now)
            .map(|(need, _)| *need)
            .collect();
        for need in &due {
            self.due.remove(need);
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_shell_window::{PhysicalPosition, PhysicalSize, ScaleFactor, WindowEvent};

    fn resized(width: u32, height: u32) -> WindowEvent {
        WindowEvent::Resized {
            inner: PhysicalSize::new(width, height),
            scale: ScaleFactor::ONE,
        }
    }

    fn display(display: Display) -> WindowEvent {
        WindowEvent::DisplayChanged { display }
    }

    #[test]
    fn a_resize_makes_the_swapchain_stale_and_damages_the_window() {
        let steps = on_window_event(Swap::Ready, &resized(300, 200));
        assert_eq!(
            steps,
            Steps {
                suspend: false,
                stale: true,
                damage: true
            }
        );
        assert_eq!(
            on_window_event(Swap::Ready, &resized(0, 200)),
            Steps::default()
        );
    }

    #[test]
    fn minimise_suspends_and_draws_nothing_restore_configures_and_draws() {
        let minimised = on_window_event(Swap::Ready, &display(Display::Minimized));
        assert!(minimised.suspend && !minimised.damage && !minimised.stale);
        for back in [Display::Normal, Display::Maximized] {
            let restored = on_window_event(Swap::Suspended, &display(back));
            assert!(restored.stale && restored.damage && !restored.suspend);
        }
        let shown = on_window_event(Swap::Ready, &display(Display::Normal));
        assert_eq!(shown, Steps::default(), "the system exposes a shown window itself");
        assert_eq!(
            on_window_event(Swap::Ready, &display(Display::Hidden)),
            Steps::default()
        );
    }

    #[test]
    fn a_scale_change_damages_without_a_new_swapchain() {
        let to = ScaleFactor::from_dpi(144).unwrap();
        let steps = on_window_event(
            Swap::Ready,
            &WindowEvent::ScaleChanged {
                from: ScaleFactor::ONE,
                to,
            },
        );
        assert_eq!(
            steps,
            Steps {
                damage: true,
                ..Steps::default()
            }
        );
    }

    /// AC214-4: what idles does not draw.
    #[test]
    fn events_that_change_nothing_shown_damage_nothing() {
        let quiet = [
            WindowEvent::Moved {
                outer: PhysicalPosition::new(4, 5),
            },
            WindowEvent::FocusChanged { focused: true },
            WindowEvent::CloseRequested,
            WindowEvent::MonitorChanged { monitor: None },
            WindowEvent::QuietBroken,
            WindowEvent::ModalLoop {
                kind: handshake_shell_window::ModalLoop::MoveOrSize,
                began: true,
            },
        ];
        for swap in [Swap::None, Swap::Ready, Swap::Suspended, Swap::Stale] {
            for event in &quiet {
                assert_eq!(on_window_event(swap, event), Steps::default(), "{event:?}");
            }
        }
    }

    #[test]
    fn a_presented_frame_is_owed_to_nobody() {
        let after = after_frame(Tried::Shown(PresentResult::Presented));
        assert_eq!(after.outcome, FrameOutcome::Presented);
        assert_eq!(after.again, Again::No);
        assert!(!after.stale && !after.new_surface && !after.stop);
    }

    #[test]
    fn a_suboptimal_present_counts_and_is_drawn_again_with_a_new_swapchain() {
        let after = after_frame(Tried::Shown(PresentResult::Suboptimal));
        assert_eq!(
            (after.outcome, after.stale, after.again),
            (FrameOutcome::Presented, true, Again::Now)
        );
    }

    #[test]
    fn an_out_of_date_swapchain_is_configured_and_the_frame_drawn_again() {
        for tried in [
            Tried::Shown(PresentResult::OutOfDate),
            Tried::Refused(PresentRefusal::OutOfDate),
            Tried::Refused(PresentRefusal::NoSwapchain),
            Tried::Refused(PresentRefusal::RecordAborted),
        ] {
            let after = after_frame(tried);
            assert_eq!(
                (after.outcome, after.stale, after.again, after.stop),
                (FrameOutcome::Failed, true, Again::Now, false),
                "{tried:?}"
            );
        }
    }

    #[test]
    fn a_lost_surface_is_made_again() {
        for tried in [
            Tried::Shown(PresentResult::SurfaceLost),
            Tried::Refused(PresentRefusal::SurfaceLost),
        ] {
            let after = after_frame(tried);
            assert!(after.new_surface && after.again == Again::Now, "{tried:?}");
        }
    }

    #[test]
    fn no_free_image_waits_for_a_timer_not_for_the_device() {
        let after = after_frame(Tried::Refused(PresentRefusal::NotReady));
        assert_eq!(after.outcome, FrameOutcome::Failed, "still owed");
        assert_eq!(after.again, Again::After(NOT_READY_RETRY));
        let in_use = after_frame(Tried::Refused(PresentRefusal::WindowInUse));
        assert_eq!(in_use.again, Again::After(WINDOW_IN_USE_RETRY));
    }

    #[test]
    fn what_cannot_get_better_by_itself_is_not_retried() {
        use handshake_vulkan_renderer::{AuthorizeRefusal, DeviceFault};
        for tried in [
            Tried::NotClearable,
            Tried::Refused(PresentRefusal::FormatUnsupported),
            Tried::Refused(PresentRefusal::Released),
            Tried::Refused(PresentRefusal::Foreign),
            Tried::Refused(PresentRefusal::Device(DeviceFault::Lost { generation: 1 })),
            Tried::Refused(PresentRefusal::NotAuthorized(
                AuthorizeRefusal::DeviceUnavailable,
            )),
        ] {
            let after = after_frame(tried);
            assert!(after.stop && after.again == Again::No, "{tried:?}");
        }
        let suspended = after_frame(Tried::Refused(PresentRefusal::Suspended));
        assert_eq!(
            (suspended.outcome, suspended.again),
            (FrameOutcome::Unchanged, Again::No)
        );
    }

    #[test]
    fn the_retirement_burst_and_the_looks_are_bounded() {
        assert_eq!(retire_frames(3), 8);
        assert_eq!(retire_frames(u32::MAX), u32::MAX);
        assert_eq!(poll_delay(0), Duration::from_millis(8));
        assert_eq!(poll_delay(6), Duration::from_millis(512));
        assert_eq!(poll_delay(99), Duration::from_millis(512));
        let total: Duration = (0..POLL_LOOKS).map(poll_delay).sum();
        assert!(total < Duration::from_secs(12), "{total:?}");
    }

    #[test]
    fn the_schedule_hands_out_what_is_due_and_keeps_the_earliest() {
        let start = Instant::now();
        let mut schedule = Schedule::default();
        assert_eq!(schedule.next(), None);
        schedule.arm(Need::Poll, start + Duration::from_millis(50));
        schedule.arm(Need::Poll, start + Duration::from_millis(20));
        schedule.arm(Need::Poll, start + Duration::from_millis(90));
        schedule.arm(Need::Finish, start + Duration::from_millis(30));
        assert_eq!(schedule.next(), Some(start + Duration::from_millis(20)));
        assert_eq!(schedule.take_due(start), []);
        assert_eq!(
            schedule.take_due(start + Duration::from_millis(25)),
            [Need::Poll]
        );
        assert_eq!(schedule.next(), Some(start + Duration::from_millis(30)));
        schedule.cancel(Need::Finish);
        assert_eq!(schedule.next(), None);
    }
}
