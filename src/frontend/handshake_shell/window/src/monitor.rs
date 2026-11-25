//! Monitors, and where a window goes on them.

use crate::geometry::{PhysicalPosition, PhysicalRect, PhysicalSize, ScaleFactor};

/// One monitor. Valid until the next [`crate::Event::MonitorsChanged`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MonitorKey(pub(crate) u64);

/// What the operating system reports about one monitor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Monitor {
    /// Its key.
    pub key: MonitorKey,
    /// The whole monitor.
    pub bounds: PhysicalRect,
    /// The part not covered by the taskbar and other docked bars.
    pub work_area: PhysicalRect,
    /// Its scale.
    pub scale: ScaleFactor,
    /// The monitor that holds the origin of the virtual desktop.
    pub primary: bool,
}

fn position(x: i64, y: i64) -> PhysicalPosition {
    let whole = |value: i64| value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    PhysicalPosition::new(whole(x), whole(y))
}

/// Where a window of `size` sits in the middle of `area`. One that does not fit starts at the area's origin, so its
/// title bar stays reachable.
pub fn centered(size: PhysicalSize, area: PhysicalRect) -> PhysicalPosition {
    let free = |extent: u32, outer: u32| (i64::from(outer) - i64::from(extent)).max(0) / 2;
    position(
        i64::from(area.origin.x) + free(size.width, area.size.width),
        i64::from(area.origin.y) + free(size.height, area.size.height),
    )
}

/// The nearest position at which a window of `size` is inside `area`, or at the area's origin where it does not fit.
pub fn clamped(
    wanted: PhysicalPosition,
    size: PhysicalSize,
    area: PhysicalRect,
) -> PhysicalPosition {
    let inside = |at: i32, extent: u32, from: i32, to: i64| {
        let last = (to - i64::from(extent)).max(i64::from(from));
        i64::from(at).clamp(i64::from(from), last)
    };
    position(
        inside(wanted.x, size.width, area.origin.x, area.right()),
        inside(wanted.y, size.height, area.origin.y, area.bottom()),
    )
}

/// Where a window at `outer` on monitor `from` goes on monitor `to`: the same share of the free space on both axes,
/// with the size it will have there.
pub fn carried(outer: PhysicalRect, from: &Monitor, to: &Monitor) -> PhysicalPosition {
    let size = size_on(outer.size, from.scale, to.scale);
    let along = |at: i32, extent: u32, from: (i32, u32), to: (i32, u32), there: u32| {
        let free = i64::from(from.1) - i64::from(extent);
        let room = (i64::from(to.1) - i64::from(there)).max(0);
        let offset = (i64::from(at) - i64::from(from.0)).clamp(0, free.max(0));
        let share = if free > 0 { offset * room / free } else { 0 };
        i64::from(to.0) + share
    };
    let (a, b) = (from.work_area, to.work_area);
    position(
        along(
            outer.origin.x,
            outer.size.width,
            (a.origin.x, a.size.width),
            (b.origin.x, b.size.width),
            size.width,
        ),
        along(
            outer.origin.y,
            outer.size.height,
            (a.origin.y, a.size.height),
            (b.origin.y, b.size.height),
            size.height,
        ),
    )
}

/// The physical size that keeps the logical size of `size` when it moves from one scale to another.
pub fn size_on(size: PhysicalSize, from: ScaleFactor, to: ScaleFactor) -> PhysicalSize {
    size.to_logical(from).to_physical(to)
}

/// A position at which a window of `size` touches no monitor.
pub fn outside_all(monitors: &[Monitor], size: PhysicalSize) -> PhysicalPosition {
    const GAP: i64 = 64;
    let left = monitors.iter().map(|m| i64::from(m.bounds.origin.x)).min();
    let top = monitors.iter().map(|m| i64::from(m.bounds.origin.y)).min();
    position(
        left.unwrap_or(0) - i64::from(size.width) - GAP,
        top.unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(key: u64, x: i32, y: i32, width: u32, height: u32, dpi: u32) -> Monitor {
        // A taskbar of 40 logical pixels at the bottom.
        let bar = 40 * dpi / 96;
        Monitor {
            key: MonitorKey(key),
            bounds: PhysicalRect::new(x, y, width, height),
            work_area: PhysicalRect::new(x, y, width, height - bar),
            scale: ScaleFactor::from_dpi(dpi).unwrap(),
            primary: key == 1,
        }
    }

    #[test]
    fn a_window_is_centred_or_starts_where_its_title_bar_is_reachable() {
        let area = PhysicalRect::new(-1920, 100, 1920, 1000);
        assert_eq!(
            centered(PhysicalSize::new(920, 400), area),
            PhysicalPosition::new(-1420, 400)
        );
        assert_eq!(
            centered(PhysicalSize::new(5000, 5000), area),
            PhysicalPosition::new(-1920, 100)
        );
    }

    #[test]
    fn clamping_keeps_a_window_inside_wherever_it_was() {
        let area = PhysicalRect::new(-1920, 0, 1920, 1040);
        let size = PhysicalSize::new(800, 600);
        for wanted in [
            PhysicalPosition::new(i32::MIN, i32::MIN),
            PhysicalPosition::new(i32::MAX, i32::MAX),
            PhysicalPosition::new(-900, 500),
            PhysicalPosition::new(-2000, 2000),
        ] {
            let at = clamped(wanted, size, area);
            let window = PhysicalRect { origin: at, size };
            assert_eq!(window.shared_area(area), 800 * 600, "{wanted:?}");
            assert_eq!(clamped(at, size, area), at, "and stays");
        }
        let too_big = PhysicalSize::new(9000, 9000);
        assert_eq!(
            clamped(PhysicalPosition::new(0, 0), too_big, area),
            PhysicalPosition::new(-1920, 0)
        );
    }

    #[test]
    fn a_carried_window_keeps_its_logical_size_and_is_inside_the_other_monitor() {
        let left = monitor(1, 0, 0, 1920, 1080, 96);
        let right = monitor(2, 1920, -300, 3840, 2160, 192);
        for (x, y) in [(0, 0), (560, 220), (1120, 440), (-400, 900), (5000, 5000)] {
            let outer = PhysicalRect::new(x, y, 800, 600);
            let at = carried(outer, &left, &right);
            let there = PhysicalRect {
                origin: at,
                size: size_on(outer.size, left.scale, right.scale),
            };
            assert_eq!(there.size, PhysicalSize::new(1600, 1200));
            assert_eq!(
                there.shared_area(right.work_area),
                1600 * 1200,
                "from ({x}, {y}) to {at:?}"
            );
        }
        // The same corner of the free space.
        let corner = PhysicalRect::new(1120, 440, 800, 600);
        let at = carried(corner, &left, &right);
        assert_eq!(i64::from(at.x) + 1600, right.work_area.right());
        assert_eq!(i64::from(at.y) + 1200, right.work_area.bottom());
        // One that fills its monitor.
        let full = carried(PhysicalRect::new(0, 0, 1920, 1040), &left, &right);
        assert_eq!(full, right.work_area.origin);
    }

    #[test]
    fn outside_all_monitors_is_on_none_of_them() {
        let monitors = [
            monitor(1, 0, 0, 1920, 1080, 96),
            monitor(2, 1920, -300, 3840, 2160, 192),
            monitor(3, -2560, 200, 2560, 1440, 120),
        ];
        for size in [PhysicalSize::new(1, 1), PhysicalSize::new(6000, 4000)] {
            let window = PhysicalRect {
                origin: outside_all(&monitors, size),
                size,
            };
            assert!(monitors.iter().all(|m| m.bounds.shared_area(window) == 0));
        }
        let alone = PhysicalRect {
            origin: outside_all(&[], PhysicalSize::new(800, 600)),
            size: PhysicalSize::new(800, 600),
        };
        assert_eq!(alone.shared_area(PhysicalRect::new(0, 0, 800, 600)), 0);
    }
}
