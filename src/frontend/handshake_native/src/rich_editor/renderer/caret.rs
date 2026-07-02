//! Document caret: blink state + galley-resolved pixel positioning (WP-KERNEL-012 MT-012).
//!
//! The caret tracks the editor [`Selection`]'s collapsed head (a [`DocPosition`]) and
//! paints a 2px vertical bar at the correct pixel position. Position resolution uses the
//! NATIVE epaint hit-test [`epaint::Galley::pos_from_cursor`] (de-risks the research's #1
//! gap — no hand-rolled glyph-advance math): the renderer hands the caret the galley for
//! the caret's paragraph plus the char offset within that paragraph's plain text, and
//! the galley returns the exact cursor rect in galley-local points, which the renderer
//! offsets by the block's screen origin.
//!
//! ## Blink (contract step 3) + idle-CPU control (red-team RISK-3 / MC-003)
//!
//! Blink is driven by egui's own clock (`ctx.input(|i| i.time)`), NOT `std::time::Instant`
//! (impl note 4: avoid fighting egui's repaint scheduling). The bar is visible when the
//! half-second phase is in its ON half. The renderer calls
//! [`request_blink_repaint`] ONLY when the editor response has focus, so an UNFOCUSED
//! editor never schedules a repaint and cannot burn idle CPU (RISK-3). A focused editor
//! requests a repaint ~250ms out so the blink animates without a busy loop.

use std::time::Duration;

use crate::rich_editor::document_model::position::DocPosition;
use crate::rich_editor::document_model::selection::Selection;

/// The blink period: the caret completes one on+off cycle per second (500ms on, 500ms
/// off), the standard editor blink rate the contract names.
pub const BLINK_PERIOD_SECS: f64 = 1.0;

/// How far ahead a FOCUSED editor schedules its next repaint to animate the blink
/// (contract step 3: `request_repaint_after(Duration::from_millis(250))`). 250ms is
/// well under the 500ms half-period so the on/off transition is not visibly late, while
/// being coarse enough not to spin the CPU.
pub const BLINK_REPAINT_MILLIS: u64 = 250;

/// The caret bar width in logical points. 2px on a 1x display (contract step 3).
pub const CARET_WIDTH_PTS: f32 = 2.0;

/// The editor caret: the document position it sits at, derived from the active
/// [`Selection`]. Only a COLLAPSED selection (a true caret) is drawn as a bar; a range
/// selection's head is still tracked here so arrow-key collapse lands correctly, but the
/// range highlight itself is painted by the block renderer's selection overlay (a later
/// pass — the MT-012 vertical slice draws the collapsed caret).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocCaret {
    /// The caret head position (the moving end of the selection).
    pub head: DocPosition,
    /// Whether the selection is collapsed (a true blinking caret) vs a range.
    pub collapsed: bool,
}

impl DocCaret {
    /// Derive the caret from the editor's current selection. A text selection's `head`
    /// is the caret position; a node selection has no text caret, so the caret falls
    /// back to the document start (path `[0,0]`, offset 0) and is marked non-collapsed
    /// so it is not drawn as a blinking bar.
    pub fn from_selection(sel: &Selection) -> Self {
        match sel {
            Selection::Text { head, .. } => Self {
                head: head.clone(),
                collapsed: sel.is_collapsed(),
            },
            Selection::Node { .. } => Self {
                head: DocPosition::new(vec![0, 0], 0),
                collapsed: false,
            },
        }
    }

    /// The char offset within the caret's text leaf (the last `path` element addresses
    /// the leaf; `char_offset` is the in-leaf offset). This is the index handed to the
    /// galley as a [`epaint::text::cursor::CCursor`].
    pub fn char_offset(&self) -> usize {
        self.head.char_offset
    }

    /// The block (paragraph) index this caret sits in: the FIRST element of the path
    /// (the child index of the block under the doc root). Used by the renderer to pick
    /// which block's galley to hit-test against. Returns `None` for an empty path (a
    /// degenerate caret that should not be drawn).
    pub fn block_index(&self) -> Option<usize> {
        self.head.path.first().copied()
    }
}

/// True when the blinking caret should be VISIBLE this frame, given egui's clock time
/// (`ctx.input(|i| i.time)` in seconds). Visible during the first half of each 1-second
/// period (contract step 3: `time modulo 1.0 > 0.5` toggles — we make the ON half the
/// FIRST half so a freshly-focused caret at t≈0 starts visible). Pure function of time
/// so it is deterministically unit-testable.
pub fn blink_visible(time_secs: f64) -> bool {
    let phase = time_secs.rem_euclid(BLINK_PERIOD_SECS);
    phase < BLINK_PERIOD_SECS / 2.0
}

/// Schedule the next blink repaint — but ONLY when the editor is focused (RISK-3 /
/// MC-003: an unfocused editor must not call `request_repaint`, or it pins the CPU at
/// 100% while idle). Returns `true` when a repaint was actually scheduled so a test can
/// assert the focus guard (an unfocused call returns `false` and schedules nothing).
pub fn request_blink_repaint(ctx: &egui::Context, has_focus: bool) -> bool {
    if !has_focus {
        return false;
    }
    ctx.request_repaint_after(Duration::from_millis(BLINK_REPAINT_MILLIS));
    true
}

/// Clamp a caret char offset to the valid `0..=leaf_len` window of the leaf it
/// addresses (red-team RISK / MC caret-bound validation: validate at the caret layer,
/// do NOT rely on the rope's silent clamp to mask an off-by-one). Returns the clamped
/// offset; a caller that gets back a different value than it passed has a real
/// off-by-one to investigate (the renderer asserts this in debug).
pub fn clamp_offset(offset: usize, leaf_len: usize) -> usize {
    offset.min(leaf_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blink_is_visible_in_first_half_hidden_in_second() {
        // t=0..0.5 visible; t=0.5..1.0 hidden; wraps each second.
        assert!(blink_visible(0.0));
        assert!(blink_visible(0.25));
        assert!(blink_visible(0.499));
        assert!(!blink_visible(0.5));
        assert!(!blink_visible(0.75));
        assert!(!blink_visible(0.999));
        // Wrap: t=1.25 behaves like t=0.25 (visible), t=1.75 like 0.75 (hidden).
        assert!(blink_visible(1.25));
        assert!(!blink_visible(1.75));
        // Toggles state across the half-period (this is the "blink state toggles" proof
        // PT for caret).
        assert_ne!(blink_visible(0.25), blink_visible(0.75));
    }

    #[test]
    fn unfocused_editor_does_not_schedule_repaint() {
        // RISK-3: the focus guard must prevent the idle repaint entirely.
        let ctx = egui::Context::default();
        assert!(
            !request_blink_repaint(&ctx, false),
            "unfocused must NOT schedule"
        );
        assert!(request_blink_repaint(&ctx, true), "focused MUST schedule");
    }

    #[test]
    fn caret_from_collapsed_selection_tracks_head() {
        let pos = DocPosition::new(vec![0, 0], 3);
        let caret = DocCaret::from_selection(&Selection::caret(pos.clone()));
        assert!(caret.collapsed);
        assert_eq!(caret.char_offset(), 3);
        assert_eq!(caret.block_index(), Some(0));
        assert_eq!(caret.head, pos);
    }

    #[test]
    fn caret_from_range_is_not_collapsed() {
        let a = DocPosition::new(vec![0, 0], 1);
        let h = DocPosition::new(vec![0, 0], 4);
        let caret = DocCaret::from_selection(&Selection::text(a, h));
        assert!(!caret.collapsed);
        assert_eq!(caret.char_offset(), 4, "head is the moving end");
    }

    #[test]
    fn clamp_offset_validates_bounds() {
        assert_eq!(clamp_offset(3, 5), 3);
        assert_eq!(clamp_offset(5, 5), 5);
        assert_eq!(clamp_offset(9, 5), 5, "past-end offset clamps to leaf len");
        assert_eq!(clamp_offset(0, 0), 0, "empty leaf: only offset 0 is valid");
    }
}
