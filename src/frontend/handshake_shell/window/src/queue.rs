//! The events between the operating system's callbacks and the host. Bounded, and in order.

use std::collections::VecDeque;

use crate::event::{Event, WindowEvent};

/// The most events kept for a host that does not take them.
pub(crate) const MAX_EVENTS: usize = 4096;

/// Events in the order they happened. A run of pointer moves, sizes or positions of one window is kept as its newest
/// member, so the order between different events never changes.
///
/// A loss is reported where it happened: [`Event::EventsLost`] comes after every event that was queued when the newest
/// loss of the report happened, and before every later one (MT-214 W7).
#[derive(Debug, Default)]
pub(crate) struct EventQueue {
    events: VecDeque<Event>,
    /// Events left out that the host was not told about yet.
    lost: u64,
    /// How many queued events come before that report.
    lost_after: usize,
}

impl EventQueue {
    pub(crate) fn push(&mut self, event: Event) {
        // An event after a loss is not folded into one from before it.
        let behind_loss = self.lost > 0 && self.lost_after == self.events.len();
        if let Some(last) = self.events.back_mut() {
            if !behind_loss && replaces(&event, last) {
                *last = event;
                return;
            }
        }
        let full = self.events.len() >= MAX_EVENTS && !self.make_room();
        if full && (!is_vital(&event) || self.events.len() >= MAX_EVENTS * 2) {
            self.note_loss();
            return;
        }
        self.events.push_back(event);
    }

    pub(crate) fn pop(&mut self) -> Option<Event> {
        if self.lost > 0 && self.lost_after == 0 {
            let count = std::mem::take(&mut self.lost);
            return Some(Event::EventsLost { count });
        }
        let event = self.events.pop_front()?;
        if self.lost > 0 {
            self.lost_after -= 1;
        }
        Some(event)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.events.is_empty() && self.lost == 0
    }

    /// An event was left out now, after everything queued.
    fn note_loss(&mut self) {
        self.lost = self.lost.saturating_add(1);
        self.lost_after = self.events.len();
    }

    /// Leave out the oldest pointer move. Nothing else can be left out without the host noticing.
    fn make_room(&mut self) -> bool {
        let Some(at) = self.events.iter().position(is_move) else {
            return false;
        };
        self.events.remove(at);
        self.note_loss();
        true
    }
}

fn is_move(event: &Event) -> bool {
    matches!(
        event,
        Event::Window {
            event: WindowEvent::PointerMoved(_),
            ..
        }
    )
}

/// Events without which the host would keep or wait for something that is gone.
fn is_vital(event: &Event) -> bool {
    match event {
        Event::Window { event, .. } => matches!(
            event,
            WindowEvent::Created { .. }
                | WindowEvent::CreateFailed { .. }
                | WindowEvent::Destroyed
                | WindowEvent::CloseRequested
                | WindowEvent::FrameDue { .. }
        ),
        Event::LoopStarted
        | Event::SystemSuspending
        | Event::SystemResumed
        | Event::SessionEnding
        | Event::ShutdownBegan
        | Event::LoopEnding => true,
        Event::MonitorsChanged | Event::EventsLost { .. } => false,
    }
}

fn replaces(newer: &Event, older: &Event) -> bool {
    match (newer, older) {
        (
            Event::Window { window, event },
            Event::Window {
                window: older_window,
                event: older_event,
            },
        ) if window == older_window => match (event, older_event) {
            (WindowEvent::PointerMoved(newer), WindowEvent::PointerMoved(older)) => {
                newer.stamp.origin == older.stamp.origin
                    && newer.stamp.device == older.stamp.device
                    && newer.stamp.modifiers == older.stamp.modifiers
            }
            (WindowEvent::Resized { .. }, WindowEvent::Resized { .. })
            | (WindowEvent::Moved { .. }, WindowEvent::Moved { .. })
            | (WindowEvent::CloseRequested, WindowEvent::CloseRequested) => true,
            _ => false,
        },
        (Event::MonitorsChanged, Event::MonitorsChanged) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::WindowKey;
    use crate::geometry::{ClientPoint, PhysicalPosition, PhysicalSize, ScaleFactor};
    use crate::input::{Modifiers, PointerMoved};
    use crate::origin::{DeviceKind, EventTime, InputStamp, Origin};

    fn stamp(origin: Origin) -> InputStamp {
        InputStamp {
            received: EventTime { nanos: 1 },
            os_millis: Some(1),
            origin,
            device: DeviceKind::Mouse,
            modifiers: Modifiers::default(),
            serial: 1,
        }
    }

    fn of(window: u64, event: WindowEvent) -> Event {
        Event::Window {
            window: WindowKey(window),
            event,
        }
    }

    fn moved(window: u64, x: f64) -> Event {
        moved_by(window, x, Origin::OperatorDevice)
    }

    fn moved_by(window: u64, x: f64, origin: Origin) -> Event {
        of(
            window,
            WindowEvent::PointerMoved(PointerMoved {
                position: ClientPoint { x, y: 0.0 },
                stamp: stamp(origin),
            }),
        )
    }

    fn entered(window: u64) -> Event {
        of(
            window,
            WindowEvent::PointerEntered(stamp(Origin::OperatorDevice)),
        )
    }

    fn drain(queue: &mut EventQueue) -> Vec<Event> {
        std::iter::from_fn(|| queue.pop()).collect()
    }

    #[test]
    fn events_keep_their_order_across_windows_and_kinds() {
        let mut queue = EventQueue::default();
        let sent = [
            Event::LoopStarted,
            entered(1),
            moved(1, 1.0),
            of(1, WindowEvent::CloseRequested),
            moved(1, 2.0),
            moved(2, 3.0),
            of(2, WindowEvent::Destroyed),
        ];
        for event in &sent {
            queue.push(event.clone());
        }
        assert_eq!(drain(&mut queue), sent);
        assert!(queue.is_empty());
    }

    #[test]
    fn only_a_run_of_moves_of_one_window_is_folded() {
        let mut queue = EventQueue::default();
        queue.push(moved(1, 1.0));
        queue.push(moved(1, 2.0));
        queue.push(entered(1));
        queue.push(moved(1, 3.0));
        queue.push(moved(2, 4.0));
        queue.push(moved(1, 5.0));
        let expected = [
            moved(1, 2.0),
            entered(1),
            moved(1, 3.0),
            moved(2, 4.0),
            moved(1, 5.0),
        ];
        assert_eq!(drain(&mut queue), expected);
    }

    #[test]
    fn moves_of_different_origin_are_kept_apart() {
        let mut queue = EventQueue::default();
        queue.push(moved(1, 1.0));
        queue.push(moved_by(1, 2.0, Origin::OtherProgram));
        assert_eq!(drain(&mut queue).len(), 2);
    }

    #[test]
    fn a_run_of_sizes_or_positions_keeps_the_newest() {
        let mut queue = EventQueue::default();
        for width in 1..=3 {
            queue.push(of(
                1,
                WindowEvent::Resized {
                    inner: PhysicalSize::new(width, 1),
                    scale: ScaleFactor::ONE,
                },
            ));
        }
        for x in 1..=3 {
            queue.push(of(
                1,
                WindowEvent::Moved {
                    outer: PhysicalPosition::new(x, 0),
                },
            ));
        }
        let left = drain(&mut queue);
        assert_eq!(left.len(), 2);
        assert!(matches!(
            left[0],
            Event::Window { event: WindowEvent::Resized { inner, .. }, .. } if inner.width == 3
        ));
        assert!(matches!(
            left[1],
            Event::Window { event: WindowEvent::Moved { outer }, .. } if outer.x == 3
        ));
    }

    #[test]
    fn a_full_queue_leaves_out_moves_first_and_says_so() {
        let mut queue = EventQueue::default();
        queue.push(moved(1, 0.0));
        while queue.events.len() < MAX_EVENTS {
            queue.push(entered(1));
        }
        queue.push(of(1, WindowEvent::FocusChanged { focused: true }));
        assert_eq!(queue.events.len(), MAX_EVENTS);
        assert_eq!(queue.lost, 1, "the move made room");
        queue.push(entered(2));
        assert_eq!(queue.events.len(), MAX_EVENTS, "bounded");
        assert_eq!(queue.lost, 2);

        let drained = drain(&mut queue);
        assert_eq!(drained.first(), Some(&entered(1)));
        assert_eq!(
            drained.last(),
            Some(&Event::EventsLost { count: 2 }),
            "after every event that was queued when the last of them was lost"
        );
        assert_eq!(
            drained[drained.len() - 2],
            of(1, WindowEvent::FocusChanged { focused: true })
        );
        assert!(queue.is_empty());
    }

    #[test]
    fn a_loss_is_reported_after_the_older_events_and_before_the_newer_ones() {
        let mut queue = EventQueue::default();
        while queue.events.len() < MAX_EVENTS {
            queue.push(entered(1));
        }
        queue.push(entered(2));
        assert_eq!(queue.pop(), Some(entered(1)));
        assert_eq!(queue.pop(), Some(entered(1)), "older events come first");
        let newer = of(1, WindowEvent::FocusChanged { focused: true });
        queue.push(newer.clone());
        queue.push(moved(1, 1.0));
        let drained = drain(&mut queue);
        assert_eq!(drained.len(), MAX_EVENTS + 1);
        assert!(drained[..MAX_EVENTS - 2]
            .iter()
            .all(|event| *event == entered(1)));
        assert_eq!(drained[MAX_EVENTS - 2], Event::EventsLost { count: 1 });
        assert_eq!(drained[MAX_EVENTS - 1], newer);
        assert_eq!(drained[MAX_EVENTS], moved(1, 1.0));
    }

    #[test]
    fn an_event_after_a_loss_is_not_folded_into_one_before_it() {
        let moved_to = |x| of(1, WindowEvent::Moved { outer: PhysicalPosition::new(x, 0) });
        let mut queue = EventQueue::default();
        while queue.events.len() < MAX_EVENTS - 1 {
            queue.push(entered(1));
        }
        queue.push(moved_to(1));
        queue.push(entered(2));
        assert_eq!(queue.pop(), Some(entered(1)));
        queue.push(moved_to(2));
        let drained = drain(&mut queue);
        let at = drained.len() - 2;
        assert_eq!(drained[at], Event::EventsLost { count: 1 });
        assert_eq!(drained[at - 1], moved_to(1));
        assert_eq!(drained[at + 1], moved_to(2));
    }

    #[test]
    fn a_full_queue_still_takes_what_the_host_must_not_miss() {
        let mut queue = EventQueue::default();
        while queue.events.len() < MAX_EVENTS {
            queue.push(entered(1));
        }
        queue.push(of(1, WindowEvent::Destroyed));
        queue.push(Event::LoopEnding);
        let drained = drain(&mut queue);
        assert!(drained.contains(&of(1, WindowEvent::Destroyed)));
        assert_eq!(drained.last(), Some(&Event::LoopEnding));
    }

    #[test]
    fn even_those_are_bounded() {
        let mut queue = EventQueue::default();
        for _ in 0..MAX_EVENTS * 4 {
            queue.push(Event::SystemSuspending);
            queue.push(Event::SystemResumed);
        }
        assert_eq!(queue.events.len(), MAX_EVENTS * 2);
        assert!(queue.lost > 0);
    }
}
