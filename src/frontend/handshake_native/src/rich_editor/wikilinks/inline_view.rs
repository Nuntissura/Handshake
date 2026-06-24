//! Inline wikilink chip rendering + the editor-event enqueue (WP-KERNEL-012 MT-015).
//!
//! A wikilink is the `hsLink` inline atom ([`HsLinkNode`]). This module renders it as a colored,
//! rounded, clickable chip overlaid on the paragraph's egui [`epaint::Galley`] glyph positions
//! (MT-012's layout engine — NOT cosmic-text), at the chip's char span. Clicking enqueues a
//! [`EditorEvent::WikilinkActivated`] into `RichEditorState.pending_events` for the WP-011 shell to
//! drain + route (E11/MT-069 host wiring) — this MT does NOT route it.
//!
//! ## Chip color (theme tokens only — CONTROL-4, no hardcoded hex)
//!
//! - resolved known kind  -> `accent_soft` background + `accent` text (the standard link affordance),
//! - unresolved / unknown -> `error_bg` background + `error_text` text + a `?` prefix (a broken link
//!   is VISIBLE, never silent — RISK-5).
//!
//! ## Scroll-adjusted Y (RISK-1 / MC-001)
//!
//! The chip rect is computed in GALLEY-LOCAL coordinates from `Galley::pos_from_cursor`, then offset
//! by the block's painted screen origin. Because the renderer paints blocks at their already
//! scroll-adjusted screen origin (the ScrollArea translates the content), the chip Y is correct under
//! scroll WITHOUT a second manual subtraction — the single source of the paint origin is the
//! scroll-adjusted `top` the renderer threads in. [`chip_rect_for_span`] is unit-tested to prove the
//! local rect maps to the right screen rect for a non-zero origin (the scroll-offset case).

use egui::accesskit;
use egui::{Color32, Rect, Vec2};

use crate::rich_editor::document_model::node::HsLinkNode;
use crate::theme::HsPalette;

/// An editor event enqueued into `RichEditorState.pending_events` for the WP-011 host shell to drain
/// and route (E11/MT-069). This MT only ENQUEUES; routing (open the Loom block / navigate to the
/// document) is owned by the parent shell (`app.rs` + `event_bus.rs` + `command_registry.rs`).
///
/// EXPECTED EVENT SHAPE (documented per MT impl note): the shell matches on the variant and uses the
/// carried `ref_kind`/`ref_value` (for a wikilink) or `source_document_id` (for a backlink) to route
/// through the existing navigation bus. The events are intentionally small value types (no borrows)
/// so they survive being parked across frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorEvent {
    /// A wikilink chip was clicked. The shell routes `ref_kind`/`ref_value` to Loom or the document
    /// viewer (e.g. `ref_kind="wp"` -> open the WP record; `ref_kind="note"` -> open the document).
    WikilinkActivated {
        /// The backend ref kind (`wp`, `file`, `note`, … or `unknown`).
        ref_kind: String,
        /// The target value the shell resolves.
        ref_value: String,
        /// Whether the link resolved to a known kind (an unknown link still emits the event so the
        /// shell can show a "cannot resolve" toast rather than silently doing nothing).
        resolved: bool,
    },
    /// A backlink entry was clicked. The shell navigates to `source_document_id`.
    BacklinkActivated {
        /// The document that links to the current one (the navigation target).
        source_document_id: String,
    },
    /// A transclusion's "Open block" button was clicked. The shell opens the referenced LoomBlock.
    TransclusionOpenRequested {
        /// The transcluded block id.
        ref_value: String,
    },
    /// WP-KERNEL-012 MT-057: the operator confirmed "Create note \"{title}\"" on an UNRESOLVED
    /// wikilink. This is the COMMAND-BUS intent the click handler emits INSTEAD of calling
    /// `POST /knowledge/documents` inline on the egui frame (RISK-007 / MC-007 — frame-freeze
    /// avoidance). The async intent handler ([`super::runtime::WikilinkRuntime::dispatch_create_note`])
    /// performs the create, then rewrites the originating mark Unresolved -> Resolved (AC-002).
    CreateNote {
        /// The (trimmed) title of the unresolved link to create.
        title: String,
    },
}

/// WP-KERNEL-012 MT-057: the editor command-bus intent vocabulary is carried on [`EditorEvent`]
/// (the events the shell drains from `RichEditorState.pending_events`). The MT contract names the
/// create intent `EditorIntent::CreateNote`; this alias makes that name available without forking a
/// second event enum, so `EditorIntent::CreateNote { title }` and `EditorEvent::CreateNote { title }`
/// are the SAME value type (one command bus, one drain path — REUSE-NOT-FORK).
pub type EditorIntent = EditorEvent;

/// WP-KERNEL-012 MT-057: the AccessKit author_id for the "Create note" affordance on an UNRESOLVED
/// wikilink, of the contract form `wikilink-create-{hash}` where `{hash}` is a short STABLE hex hash
/// of the NORMALIZED title (so the same unresolved title yields the same id across repaints, and a
/// swarm agent / kittest can target it deterministically — MC-005). The hash is over the NORMALIZED
/// title (trim + collapse-whitespace + lower-case) so `[[Foo]]` and `[[ foo ]]` — the same logical
/// target — share one create affordance id.
pub fn create_affordance_author_id(title: &str) -> String {
    let norm = crate::rich_editor::wikilinks::resolver::normalize_target(title);
    format!("wikilink-create-{}", short_hex_hash(norm.as_bytes()))
}

/// WP-KERNEL-012 MT-057: the AccessKit author_id for one alias-autocomplete candidate row, of the
/// contract form `wikilink-candidate-{document_id}` (the document the row inserts a link to). The
/// document id is used verbatim (it is already a stable opaque id), so a swarm agent / kittest targets
/// a candidate by the document it resolves to.
pub fn candidate_author_id(document_id: &str) -> String {
    format!("wikilink-candidate-{document_id}")
}

/// A short, stable 32-bit hex hash (FNV-1a) for an author_id suffix — the SAME deterministic
/// no-random-seed hash [`chip_author_id`] uses, so create-affordance ids are stable across runs (NOT
/// `RandomState`, which would re-seed per process and break kittest/swarm targeting — MC-005).
fn short_hex_hash(bytes: &[u8]) -> String {
    format!("{:08x}", fnv1a_hash(bytes))
}

/// The AccessKit author_id for a wikilink chip (`wikilink-chip-{ref_value_hash}` per the MT contract).
/// The hash is a deterministic FNV-1a of the ref value, so the same wikilink yields the same id
/// across repaints (an out-of-process agent can target a chip by a value it computes independently).
pub fn chip_author_id(ref_value: &str) -> String {
    format!("wikilink-chip-{}", fnv1a_hash(ref_value.as_bytes()))
}

/// Deterministic 32-bit FNV-1a hash (the same stable, no-random-seed hash the renderer uses for block
/// ids). Used for the chip author_id suffix so it is stable across runs (NOT `RandomState`).
fn fnv1a_hash(bytes: &[u8]) -> u32 {
    const FNV_OFFSET: u32 = 0x811c_9dc5;
    const FNV_PRIME: u32 = 0x0100_0193;
    let mut hash = FNV_OFFSET;
    for &b in bytes {
        hash ^= b as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// True when this hsLink atom is a WP-KERNEL-012 MT-034 CODE cross-reference (ref_kind="code"). A code
/// ref gets a distinct chip author_id (`code-ref-chip-{ref_value}`) and a code-styled short label so
/// it reads as a code-symbol pill, not a generic wikilink.
pub fn is_code_ref(link: &HsLinkNode) -> bool {
    link.ref_kind == crate::interop::cross_ref::CODE_REF_KIND
}

/// The AccessKit author_id for a CODE-reference chip (`code-ref-chip-{symbol_entity_id}` per the MT-034
/// contract). For a code ref the `ref_value` IS the symbol entity id / resolution key, so it is used
/// verbatim (NOT the hashed wikilink-chip id) — the contract names the id by the symbol entity id so a
/// swarm agent / kittest can target the chip by the symbol it references.
pub fn code_ref_chip_author_id(symbol_ref: &str) -> String {
    format!("code-ref-chip-{symbol_ref}")
}

/// The SHORT display name for a code-symbol key/label (the last `::` segment, then the last `#`
/// segment — `path/to/file.rs#Mod::MyStruct` -> `MyStruct`), per the MT-034 chip rendering note
/// ("Show the symbol_key short form: last '::' segment"). Falls back to the whole string when there is
/// no separator.
pub fn code_ref_short_name(symbol_key_or_label: &str) -> String {
    let after_hash = symbol_key_or_label.rsplit('#').next().unwrap_or(symbol_key_or_label);
    let last_seg = after_hash.rsplit("::").next().unwrap_or(after_hash);
    let trimmed = last_seg.trim();
    if trimmed.is_empty() {
        symbol_key_or_label.to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// The chip's display label: the explicit label, else `ref_kind:ref_value` (the React `hsLink`
/// default), with a `?` prefix for an unresolved/unknown link so a broken chip is visible (RISK-5).
///
/// MT-034: a CODE ref renders the SHORT symbol name (the last `::`/`#` segment) with a small code
/// glyph prefix, so it reads as a monospace code-symbol pill. An UNRESOLVED code ref (the symbol was
/// deleted -> a 404 marked it `resolved=false`) shows `unresolved` text + the `?` prefix, greyed (the
/// caller's `chip_colors` gives it the error affordance), without crashing (AC-4 / RISK pt(e)).
pub fn chip_label(link: &HsLinkNode) -> String {
    if is_code_ref(link) {
        let name = if link.label.trim().is_empty() {
            code_ref_short_name(&link.ref_value)
        } else {
            code_ref_short_name(&link.label)
        };
        return if link.resolved {
            format!("‹›{name}")
        } else {
            format!("? {name} (unresolved)")
        };
    }
    let base = if link.label.is_empty() {
        format!("{}:{}", link.ref_kind, link.ref_value)
    } else {
        link.label.clone()
    };
    if link.resolved {
        base
    } else {
        format!("? {base}")
    }
}

/// The chip's (background, text) colors from the theme palette — resolved links use the accent
/// affordance; unresolved/unknown links use the error affordance so they read as broken. NEVER a
/// hardcoded hex (CONTROL-4).
pub fn chip_colors(link: &HsLinkNode, palette: &HsPalette) -> (Color32, Color32) {
    if link.resolved {
        (palette.accent_soft, palette.accent)
    } else {
        (palette.error_bg, palette.error_text)
    }
}

/// Compute the chip's SCREEN rect from the galley-local glyph span rect + the block's painted screen
/// origin. `local_min`/`local_max` are the `Galley::pos_from_cursor` rects for the chip's start/end
/// char offsets (galley-local, top=0); `origin` is the block's painted top-left in SCREEN space (the
/// scroll-adjusted paint origin the renderer threads in — RISK-1 / MC-001: scroll adjustment lives in
/// the single paint origin, so this is a pure offset). A small vertical padding makes the chip read
/// as a pill around the glyphs.
pub fn chip_rect_for_span(
    local_start: Rect,
    local_end: Rect,
    origin: egui::Pos2,
) -> Rect {
    // The chip spans from the start glyph's left to the end glyph's right, the row's full height.
    let x0 = origin.x + local_start.min.x;
    let x1 = origin.x + local_end.max.x;
    let y0 = origin.y + local_start.min.y;
    let height = local_start.height().max(local_end.height());
    // 1px pad each side horizontally so the pill does not clip the text; the height is the row height.
    Rect::from_min_size(egui::pos2(x0 - 1.0, y0), Vec2::new((x1 - x0) + 2.0, height))
}

/// The AccessKit role for a wikilink chip — the field-correct nearest variant in accesskit 0.21.1
/// (the MT names `Role::Link`).
pub const CHIP_ROLE: accesskit::Role = accesskit::Role::Link;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::HsTheme;

    fn dark() -> HsPalette {
        HsTheme::Dark.palette()
    }

    #[test]
    fn author_id_is_deterministic_and_prefixed() {
        let a = chip_author_id("WP-KERNEL-012");
        let b = chip_author_id("WP-KERNEL-012");
        assert_eq!(a, b, "the chip id is deterministic for the same ref value");
        assert!(a.starts_with("wikilink-chip-"), "the contract author_id prefix");
        assert_ne!(chip_author_id("a"), chip_author_id("b"), "distinct refs -> distinct ids");
    }

    #[test]
    fn label_uses_explicit_then_falls_back() {
        let with_label = HsLinkNode { ref_kind: "wp".into(), ref_value: "WP-7".into(), label: "Seven".into(), resolved: true };
        assert_eq!(chip_label(&with_label), "Seven");
        let no_label = HsLinkNode { ref_kind: "wp".into(), ref_value: "WP-7".into(), label: String::new(), resolved: true };
        assert_eq!(chip_label(&no_label), "wp:WP-7", "falls back to ref_kind:ref_value");
    }

    #[test]
    fn code_ref_chip_id_uses_symbol_ref_verbatim() {
        // MT-034: the code-ref chip id is `code-ref-chip-{symbol_entity_id}` (the symbol the chip
        // references), used verbatim — NOT the hashed wikilink-chip id.
        assert_eq!(code_ref_chip_author_id("ent-42"), "code-ref-chip-ent-42");
        let link = HsLinkNode { ref_kind: "code".into(), ref_value: "ent-42".into(), label: String::new(), resolved: true };
        assert!(is_code_ref(&link));
        let wp = HsLinkNode { ref_kind: "wp".into(), ref_value: "WP-1".into(), label: String::new(), resolved: true };
        assert!(!is_code_ref(&wp));
    }

    #[test]
    fn code_ref_short_name_takes_last_segment() {
        // MT-034 chip note: the short form is the last `::`/`#` segment.
        assert_eq!(code_ref_short_name("src/main.rs#Mod::MyStruct"), "MyStruct");
        assert_eq!(code_ref_short_name("src/main.rs#add"), "add");
        assert_eq!(code_ref_short_name("bare"), "bare");
        assert_eq!(code_ref_short_name(""), "");
    }

    #[test]
    fn code_ref_label_resolved_vs_unresolved() {
        // A resolved code ref shows the short name with a code glyph; an UNRESOLVED one (deleted symbol,
        // 404 -> resolved=false) shows `(unresolved)` greyed, never crashing (AC-4 / RISK pt(e)).
        let resolved = HsLinkNode { ref_kind: "code".into(), ref_value: "ent-1".into(), label: "src/main.rs#MyStruct".into(), resolved: true };
        let lbl = chip_label(&resolved);
        assert!(lbl.contains("MyStruct"), "resolved code chip shows the short symbol name");
        assert!(!lbl.contains("unresolved"));
        let unresolved = HsLinkNode { ref_kind: "code".into(), ref_value: "ent-9".into(), label: "src/gone.rs#Gone".into(), resolved: false };
        let ul = chip_label(&unresolved);
        assert!(ul.contains("unresolved"), "an unresolved code chip reads as broken");
        assert!(ul.starts_with("? "), "unresolved keeps the broken-link `?` prefix");
        assert!(ul.contains("Gone"));
    }

    #[test]
    fn unresolved_label_carries_question_prefix() {
        let unknown = HsLinkNode { ref_kind: "unknown".into(), ref_value: "xyz".into(), label: String::new(), resolved: false };
        assert_eq!(chip_label(&unknown), "? unknown:xyz", "an unresolved chip reads as broken");
    }

    #[test]
    fn colors_come_from_theme_resolved_vs_unresolved() {
        let pal = dark();
        let resolved = HsLinkNode { ref_kind: "wp".into(), ref_value: "x".into(), label: String::new(), resolved: true };
        let (bg, fg) = chip_colors(&resolved, &pal);
        assert_eq!(bg, pal.accent_soft);
        assert_eq!(fg, pal.accent);
        let unresolved = HsLinkNode { ref_kind: "unknown".into(), ref_value: "x".into(), label: String::new(), resolved: false };
        let (bg2, fg2) = chip_colors(&unresolved, &pal);
        assert_eq!(bg2, pal.error_bg, "unresolved uses the error background (visible broken link)");
        assert_eq!(fg2, pal.error_text);
    }

    #[test]
    fn chip_rect_offsets_by_scroll_adjusted_origin_mc001() {
        // MC-001: the chip rect = galley-local span + the (scroll-adjusted) block paint origin. A
        // non-zero origin Y (the scrolled case) shifts the chip exactly by that origin, no double
        // subtraction.
        let local_start = Rect::from_min_size(egui::pos2(10.0, 0.0), Vec2::new(2.0, 18.0));
        let local_end = Rect::from_min_size(egui::pos2(60.0, 0.0), Vec2::new(2.0, 18.0));
        // Scrolled down: the block paints at screen y = 200 (origin already scroll-adjusted).
        let origin = egui::pos2(40.0, 200.0);
        let rect = chip_rect_for_span(local_start, local_end, origin);
        // x spans from origin.x+10-1 to origin.x+62+1; y starts at origin.y+0.
        assert_eq!(rect.min.x, 40.0 + 10.0 - 1.0);
        assert_eq!(rect.max.x, 40.0 + 62.0 + 1.0);
        assert_eq!(rect.min.y, 200.0, "chip Y follows the scroll-adjusted origin exactly");
        assert!((rect.height() - 18.0).abs() < 0.01, "chip height is the glyph row height");
    }

    #[test]
    fn editor_event_shapes_round_trip_for_shell_routing() {
        // The events are small value types the shell drains; assert their fields carry the routing
        // payload the WP-011 host needs.
        let wl = EditorEvent::WikilinkActivated { ref_kind: "wp".into(), ref_value: "WP-1".into(), resolved: true };
        match wl {
            EditorEvent::WikilinkActivated { ref_kind, ref_value, resolved } => {
                assert_eq!(ref_kind, "wp");
                assert_eq!(ref_value, "WP-1");
                assert!(resolved);
            }
            _ => panic!("variant"),
        }
        let bl = EditorEvent::BacklinkActivated { source_document_id: "DOC-2".into() };
        assert!(matches!(bl, EditorEvent::BacklinkActivated { source_document_id } if source_document_id == "DOC-2"));
    }
}
