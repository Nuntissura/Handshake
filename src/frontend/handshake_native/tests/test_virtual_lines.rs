//! MT-002 viewport-virtualization calculator proofs (WP-KERNEL-012).
//!
//! AC-001 / PT-001 — black-box proof of the public [`VirtualLineLayout`] API:
//!   - `visible_range` for scroll=0 (first window + overscan, NOT the whole doc),
//!   - scroll to the middle of a 100k-line buffer (correct centered mid-range),
//!   - scroll to the end (last lines, clamped to line_count),
//!   - `total_height_px` == line_count * line_height,
//!   - `y_for_line` monotonically increasing.
//!
//! PT-001 runs these via `cargo test -p handshake-native virtual_lines`; the file + test fn names
//! contain `virtual_lines`/`visible_range`/... so that filter selects this proof set together with the
//! in-module unit tests.

use handshake_native::code_editor::{VirtualLineLayout, OVERSCAN_LINES};

/// 100 000-line document at a 16px line height in an 800px viewport — the AC-002 perf workload shape,
/// reused here so the range asserts mirror the real large-file scenario.
const LINE_COUNT: usize = 100_000;
const LINE_HEIGHT: f32 = 16.0;
const VIEWPORT: f32 = 800.0;

#[test]
fn virtual_lines_scroll_zero_paints_first_window_only() {
    let layout = VirtualLineLayout::new(LINE_COUNT, LINE_HEIGHT, VIEWPORT, 0.0);
    let range = layout.visible_range();
    assert_eq!(range.start, 0, "AC-001: scroll=0 starts at line 0");
    // ceil(800/16)=50 visible + overscan; never the whole 100k document.
    let visible = (VIEWPORT / LINE_HEIGHT).ceil() as usize;
    assert!(
        range.end >= visible && range.end <= visible + OVERSCAN_LINES + 2,
        "AC-001: end {} is ~{visible} visible + overscan",
        range.end
    );
    assert!(
        range.len() < LINE_COUNT,
        "AC-001: virtualized — far fewer than {LINE_COUNT} lines painted (got {})",
        range.len()
    );
    println!("PT-001 scroll0: range={range:?} ({} lines)", range.len());
}

#[test]
fn virtual_lines_scroll_to_middle_returns_centered_range() {
    let mid = 50_000usize;
    let layout =
        VirtualLineLayout::new(LINE_COUNT, LINE_HEIGHT, VIEWPORT, mid as f32 * LINE_HEIGHT);
    let range = layout.visible_range();
    assert_eq!(
        range.start,
        mid - OVERSCAN_LINES,
        "AC-001: mid-scroll first painted line is mid - overscan"
    );
    assert!(
        range.contains(&mid),
        "AC-001: scrolled-to line {mid} is inside {range:?}"
    );
    assert!(
        !range.contains(&0),
        "AC-001: line 0 is NOT painted in the middle"
    );
    assert!(
        range.len() < LINE_COUNT,
        "AC-001: still a small window (got {})",
        range.len()
    );
    println!("PT-001 mid: range={range:?}");
}

#[test]
fn virtual_lines_scroll_to_end_clamps_to_last_lines() {
    let layout = VirtualLineLayout::new(
        LINE_COUNT,
        LINE_HEIGHT,
        VIEWPORT,
        LINE_COUNT as f32 * LINE_HEIGHT,
    );
    let range = layout.visible_range();
    assert_eq!(
        range.end, LINE_COUNT,
        "AC-001: end clamps to line_count at the bottom"
    );
    assert!(
        range.contains(&(LINE_COUNT - 1)),
        "AC-001: the final line {} is painted at the end",
        LINE_COUNT - 1
    );
    assert!(
        range.len() < LINE_COUNT,
        "AC-001: end window bounded (got {})",
        range.len()
    );
    println!("PT-001 end: range={range:?}");
}

#[test]
fn virtual_lines_total_height_matches_line_count_times_height() {
    let layout = VirtualLineLayout::new(LINE_COUNT, LINE_HEIGHT, VIEWPORT, 0.0);
    assert_eq!(
        layout.total_height_px(),
        LINE_COUNT as f32 * LINE_HEIGHT,
        "AC-001: total_height_px == line_count * line_height"
    );
}

#[test]
fn virtual_lines_y_for_line_is_monotonic() {
    let layout = VirtualLineLayout::new(LINE_COUNT, LINE_HEIGHT, VIEWPORT, 0.0);
    let mut prev = f32::NEG_INFINITY;
    for line in [0usize, 1, 100, 1_000, 50_000, LINE_COUNT - 1] {
        let y = layout.y_for_line(line);
        assert!(
            y > prev,
            "AC-001: y_for_line({line})={y} must increase past {prev}"
        );
        assert_eq!(
            y,
            line as f32 * LINE_HEIGHT,
            "AC-001: y_for_line == line * line_height"
        );
        prev = y;
    }
}

#[test]
fn virtual_lines_boundary_buffers_do_not_panic() {
    // 0-line, 1-line, and scroll-past-end (MC-002 boundary coverage on the public API).
    assert_eq!(
        VirtualLineLayout::new(0, LINE_HEIGHT, VIEWPORT, 0.0).visible_range(),
        0..0,
        "empty doc paints nothing"
    );
    assert_eq!(
        VirtualLineLayout::new(1, LINE_HEIGHT, VIEWPORT, 0.0).visible_range(),
        0..1,
        "single-line doc paints exactly line 0"
    );
    let past_end = VirtualLineLayout::new(3, LINE_HEIGHT, VIEWPORT, 1_000_000.0).visible_range();
    assert!(
        past_end.start <= past_end.end,
        "scroll-past-end never inverts (got {past_end:?})"
    );
    assert_eq!(past_end.end, 3, "clamped to the 3-line doc");
}
