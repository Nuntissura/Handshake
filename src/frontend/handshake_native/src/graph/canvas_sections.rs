//! Loom canvas section/group FRAMES (WP-KERNEL-012 MT-061, cluster E3) — the Obsidian-Canvas-class
//! titled-container layer for the native [`crate::graph::canvas_board::LoomCanvasBoard`].
//!
//! ## What this is
//!
//! MT-026 delivered the abstract `group_id` on a placement (set by the `Group (N)` toolbar action and
//! persisted via `PATCH .../canvas-placements/:id {group_id}`), but a group was an INVISIBLE attribute —
//! there was no on-canvas container. This module promotes each distinct non-null `group_id` returned by
//! `getCanvasBoard` into a VISIBLE, titled, rounded-rectangle FRAME drawn BEHIND its member cards
//! (Obsidian-Canvas "section"). It owns:
//!   - [`SectionFrame`] — one derived frame: `{ id, label, rect (canvas space), color }`.
//!   - [`SectionLayer`] — the per-frame set derived from the placements + board section metadata.
//!   - [`SectionLayer::which_section`] — deterministic drop-target hit-testing used during a card-move
//!     drag-drop to assign the dropped card to a section (or clear it when the drop is outside all
//!     frames).
//!
//! ## Reference-not-copy (the load-bearing MT-061 invariant)
//!
//! A section frame is PURE PROJECTION: it is DERIVED from the placements' `group_id` each frame and owns
//! NO authority. Assigning a card to a section mutates ONLY the placement record's `group_id` (the SAME
//! `PATCH .../canvas-placements/:id` route MT-026 already uses for the abstract group); it NEVER touches
//! the underlying loom block. So sectioning a block-backed card can never duplicate or fork a block — the
//! placement's `placed_block_id` is unchanged (proven by the block_id-set-invariance test, AC-061-5).
//!
//! ## Deterministic hit-test precedence (RISK-061-4 / MC-061-4)
//!
//! Frames derived from member-card bounds can OVERLAP or NEST. [`SectionLayer::which_section`] resolves
//! ambiguity deterministically: among all frames CONTAINING the drop point, the SMALLEST-area frame wins
//! (the topmost / most-specific enclosing section — a nested child beats its parent). Ties on area break
//! by the frame `id` (lexicographic) so the result is fully deterministic regardless of derivation order.
//!
//! ## Theme tokens (CONTROL-4)
//!
//! Every frame color comes from [`crate::theme::palette::canvas_section_palette`] (a sanctioned `Color32`
//! home). This module constructs NO `Color32` literal — the no-hardcoded-color guard
//! (`tests/test_theme.rs`) exempts only `palette.rs`/`syntax.rs`.
//!
//! ## AccessKit (HBR-SWARM)
//!
//! Each frame emits a live AccessKit node `canvas.section.{group_id}` (sanitized) with `Role::Group` and
//! the section label as the accessible name, so an out-of-process swarm agent reads the canvas's sections
//! by stable id. The author_id format and sanitization match the placement-card convention.

use std::collections::BTreeMap;

use egui::{Pos2, Rect, Vec2};

use crate::graph::canvas_board::CanvasPlacementCard;
use crate::theme::palette::{canvas_section_palette, CANVAS_SECTION_PALETTE_LEN};

/// Author_id prefix for a section frame. The full id is `canvas.section.{sanitized_group_id}`.
pub const SECTION_AUTHOR_ID_PREFIX: &str = "canvas.section.";

/// Padding (canvas units) added around the union of a section's member-card rects when the frame is
/// derived from member bounds, so the frame reads as a container with breathing room around its cards.
pub const SECTION_PADDING: f32 = 24.0;

/// Extra height (canvas units) reserved at the top of a derived frame for the section title label, so the
/// title sits above the member cards rather than overlapping the topmost card.
pub const SECTION_TITLE_BAND: f32 = 22.0;

/// The stable AccessKit author_id for a section frame, sanitizing `group_id` to `[a-z0-9-]` (reusing the
/// shell's slugger) so a raw group id with slashes/colons can never break AccessKit tree integrity.
pub fn section_author_id(group_id: &str) -> String {
    format!(
        "{SECTION_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(group_id)
    )
}

/// One derived section/group FRAME: a titled rounded-rectangle container drawn behind its member cards.
/// `rect` is in CANVAS space (the board applies the canvas->screen transform at paint time). The frame is
/// a pure projection of the placements' `group_id` + board section metadata — it owns no authority.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionFrame {
    /// The `group_id` this frame visualizes (the placement field; the section's stable identity).
    pub id: String,
    /// The human-readable section label (board section/group metadata, falling back to the `group_id`).
    pub label: String,
    /// The frame's bounds in CANVAS space (union of member-card rects, padded — or board-supplied frame
    /// geometry when the payload carries one).
    pub rect: Rect,
    /// The per-section identity color (theme token from [`canvas_section_palette`]). Drawn translucent
    /// for the fill + opaque for the border/title at paint time.
    pub color: egui::Color32,
}

impl SectionFrame {
    /// The frame's area in canvas units (used by [`SectionLayer::which_section`]'s smallest-wins
    /// precedence). A degenerate (non-positive) rect yields `0.0` so it never wins a containment tie.
    pub fn area(&self) -> f32 {
        (self.rect.width().max(0.0)) * (self.rect.height().max(0.0))
    }
}

/// The set of section frames derived for the current board state. Re-derived each frame from the
/// placements' `group_id` so a removed/cleared group's frame DISAPPEARS (deletion-by-absence) and a new
/// group's frame appears without any per-frame mutation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SectionLayer {
    pub frames: Vec<SectionFrame>,
}

impl SectionLayer {
    /// Derive the section layer from `placements` and an optional `labels` map (board section/group
    /// metadata: `group_id -> label`). One frame per distinct NON-NULL `group_id`; its rect is the union
    /// of its member cards' canvas rects, padded by [`SECTION_PADDING`] (+ a [`SECTION_TITLE_BAND`] at the
    /// top for the title). The label is `labels[group_id]` when present, else the `group_id` string.
    ///
    /// Colors are assigned by the group's discovery order (first appearance scanning `placements`) modulo
    /// the [`canvas_section_palette`], so the same `group_id` reads with the same hue across reloads as
    /// long as the membership order is stable; the assignment is recomputed deterministically here.
    pub fn derive(placements: &[CanvasPlacementCard], labels: &BTreeMap<String, String>) -> Self {
        // Group member rects by group_id, preserving first-appearance order for stable color assignment.
        let mut order: Vec<String> = Vec::new();
        let mut bounds: BTreeMap<String, Rect> = BTreeMap::new();
        for card in placements {
            let Some(gid) = card.group_id.as_ref() else {
                continue;
            };
            if gid.trim().is_empty() {
                continue; // an empty-string group_id is "ungrouped" — never a frame
            }
            let card_rect =
                Rect::from_min_size(Pos2::new(card.x, card.y), Vec2::new(card.w, card.h));
            match bounds.get_mut(gid) {
                Some(existing) => *existing = existing.union(card_rect),
                None => {
                    order.push(gid.clone());
                    bounds.insert(gid.clone(), card_rect);
                }
            }
        }

        let palette = canvas_section_palette();
        let mut frames = Vec::with_capacity(order.len());
        for (idx, gid) in order.iter().enumerate() {
            let raw = bounds[gid];
            // Pad the member union, reserving a title band at the top so the label never overlaps a card.
            let rect = Rect::from_min_max(
                Pos2::new(
                    raw.min.x - SECTION_PADDING,
                    raw.min.y - SECTION_PADDING - SECTION_TITLE_BAND,
                ),
                Pos2::new(raw.max.x + SECTION_PADDING, raw.max.y + SECTION_PADDING),
            );
            let label = labels
                .get(gid)
                .filter(|l| !l.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| gid.clone());
            frames.push(SectionFrame {
                id: gid.clone(),
                label,
                rect,
                color: palette[idx % CANVAS_SECTION_PALETTE_LEN],
            });
        }
        Self { frames }
    }

    /// The `group_id` of the section frame that should claim a card dropped at `canvas_pos`, or `None`
    /// when the drop falls outside ALL frames (the caller then CLEARS the card's group assignment).
    ///
    /// Deterministic precedence (RISK-061-4 / MC-061-4): among all frames CONTAINING `canvas_pos`, the
    /// SMALLEST-area frame wins (the most-specific enclosing section — a nested child beats its parent).
    /// Ties on area break by the frame `id` lexicographically, so the result is fully deterministic
    /// regardless of derivation order or floating-point equality.
    pub fn which_section(&self, canvas_pos: Pos2) -> Option<&str> {
        self.frames
            .iter()
            .filter(|f| f.rect.contains(canvas_pos))
            .min_by(|a, b| {
                a.area()
                    .partial_cmp(&b.area())
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.id.cmp(&b.id))
            })
            .map(|f| f.id.as_str())
    }

    /// The frame for a given `group_id`, if derived (used by the board to look up a frame's color/rect).
    pub fn frame(&self, group_id: &str) -> Option<&SectionFrame> {
        self.frames.iter().find(|f| f.id == group_id)
    }

    /// `true` when there are no frames (no grouped placements) — the board skips the section-draw pass.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str, block: &str, x: f32, y: f32, group: Option<&str>) -> CanvasPlacementCard {
        let mut c = CanvasPlacementCard::new(id, block, x, y, 100.0, 60.0);
        c.group_id = group.map(ToOwned::to_owned);
        c
    }

    /// One distinct group_id => one frame; the frame rect encloses (and pads) its members; an ungrouped
    /// card contributes no frame.
    #[test]
    fn derive_one_frame_per_group() {
        let placements = vec![
            card("p1", "b1", 0.0, 0.0, Some("g-a")),
            card("p2", "b2", 200.0, 0.0, Some("g-a")),
            card("p3", "b3", 0.0, 400.0, None), // ungrouped -> no frame
        ];
        let layer = SectionLayer::derive(&placements, &BTreeMap::new());
        assert_eq!(
            layer.frames.len(),
            1,
            "one distinct non-null group => one frame"
        );
        let f = &layer.frames[0];
        assert_eq!(f.id, "g-a");
        // Member union is x:[0,300] y:[0,60]; padded + title band must enclose it.
        assert!(
            f.rect.min.x < 0.0 && f.rect.max.x > 300.0,
            "frame encloses + pads member x bounds"
        );
        assert!(
            f.rect.min.y < 0.0 && f.rect.max.y > 60.0,
            "frame encloses + pads member y bounds"
        );
        // The title band makes the top padding larger than the bottom padding.
        assert!(
            (0.0 - f.rect.min.y) > (f.rect.max.y - 60.0),
            "top inset includes the title band (larger than the bottom inset)"
        );
    }

    /// An empty-string group_id is treated as ungrouped (never a frame).
    #[test]
    fn empty_group_id_is_ungrouped() {
        let placements = vec![card("p1", "b1", 0.0, 0.0, Some(""))];
        let layer = SectionLayer::derive(&placements, &BTreeMap::new());
        assert!(
            layer.is_empty(),
            "an empty-string group_id is ungrouped, not a frame"
        );
    }

    /// Board section metadata supplies the label; absent metadata falls back to the group_id string.
    #[test]
    fn label_from_metadata_then_group_id() {
        let placements = vec![
            card("p1", "b1", 0.0, 0.0, Some("g-a")),
            card("p2", "b2", 0.0, 0.0, Some("g-b")),
        ];
        let mut labels = BTreeMap::new();
        labels.insert("g-a".to_owned(), "Research".to_owned());
        let layer = SectionLayer::derive(&placements, &labels);
        let fa = layer.frame("g-a").unwrap();
        let fb = layer.frame("g-b").unwrap();
        assert_eq!(fa.label, "Research", "metadata label used when present");
        assert_eq!(
            fb.label, "g-b",
            "fallback to group_id string when no metadata label"
        );
    }

    /// which_section: a drop inside a frame returns its group_id; a drop outside all frames returns None
    /// (the caller clears the assignment).
    #[test]
    fn which_section_inside_and_outside() {
        let placements = vec![card("p1", "b1", 100.0, 100.0, Some("g-a"))];
        let layer = SectionLayer::derive(&placements, &BTreeMap::new());
        let f = &layer.frames[0];
        let inside = f.rect.center();
        assert_eq!(
            layer.which_section(inside),
            Some("g-a"),
            "drop inside the frame assigns its group"
        );
        // A point far outside any member bounds.
        let outside = Pos2::new(5000.0, 5000.0);
        assert_eq!(
            layer.which_section(outside),
            None,
            "drop outside all frames clears (None)"
        );
    }

    /// RISK-061-4 / MC-061-4: with NESTED/OVERLAPPING frames, the smallest-area enclosing frame wins
    /// (the most-specific section). Frames are constructed directly here to control nesting precisely.
    #[test]
    fn which_section_smallest_enclosing_wins() {
        let palette = canvas_section_palette();
        let big = SectionFrame {
            id: "outer".to_owned(),
            label: "Outer".to_owned(),
            rect: Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(400.0, 400.0)),
            color: palette[0],
        };
        let small = SectionFrame {
            id: "inner".to_owned(),
            label: "Inner".to_owned(),
            rect: Rect::from_min_max(Pos2::new(100.0, 100.0), Pos2::new(200.0, 200.0)),
            color: palette[1],
        };
        let layer = SectionLayer {
            frames: vec![big, small],
        };
        // A point inside BOTH frames must resolve to the smaller (inner) one.
        assert_eq!(
            layer.which_section(Pos2::new(150.0, 150.0)),
            Some("inner"),
            "MC-061-4: the smallest enclosing frame wins a containment tie"
        );
        // A point only inside the big frame resolves to it.
        assert_eq!(layer.which_section(Pos2::new(50.0, 50.0)), Some("outer"));
    }

    /// Tie-break on equal area is deterministic by id (lexicographic), independent of vec order.
    #[test]
    fn which_section_equal_area_tie_breaks_by_id() {
        let palette = canvas_section_palette();
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 100.0));
        let z = SectionFrame {
            id: "z".to_owned(),
            label: "z".to_owned(),
            rect,
            color: palette[0],
        };
        let a = SectionFrame {
            id: "a".to_owned(),
            label: "a".to_owned(),
            rect,
            color: palette[1],
        };
        let layer = SectionLayer { frames: vec![z, a] };
        assert_eq!(
            layer.which_section(Pos2::new(50.0, 50.0)),
            Some("a"),
            "equal-area containment ties break by lexicographic id (deterministic)"
        );
    }

    /// Section author_id sanitizes a raw group_id with slashes/colons to a [a-z0-9-] suffix.
    #[test]
    fn section_author_id_is_sanitized() {
        let id = section_author_id("grp:1/x 7");
        assert!(id.starts_with(SECTION_AUTHOR_ID_PREFIX));
        let suffix = &id[SECTION_AUTHOR_ID_PREFIX.len()..];
        assert!(
            suffix
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "author_id suffix must be [a-z0-9-]; got '{suffix}'"
        );
    }
}
