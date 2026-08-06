//! Inline wikilink chip rendering + the editor-event enqueue (WP-KERNEL-012 MT-015).
//!
//! A wikilink is the `hsLink` inline atom ([`HsLinkNode`]). This module renders it as a colored,
//! rounded, clickable chip over the paragraph's egui [`epaint::Galley`] glyph positions
//! (MT-012's layout engine — NOT cosmic-text), at the chip's char span. Per the MT-068
//! glyph-overlap fix, the galley LAYS OUT exactly [`chip_label`]'s text for the atom and (on the
//! chip-covered top-level paint path) paints that run TRANSPARENT — the chip CONSUMES the atom's
//! text layout space and its pill + label are the only visible glyphs, so no doubled runs can stick
//! out around the chip. Clicking enqueues a [`EditorEvent::WikilinkActivated`] into
//! `RichEditorState.pending_events` for the WP-011 shell to drain + route (E11/MT-069 host wiring)
//! — this MT does NOT route it.
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
#[cfg(test)]
use egui::Vec2;
use egui::{Color32, Rect};

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
    /// WP-KERNEL-012 MT-043: the operator or an AccessKit agent selected one exact rich-note code
    /// block for editing in the native code editor. `block_path` is the stable document-model path
    /// captured from the rendered block; the shell binds that exact path and original text to the
    /// code panel so `editor.code.save` can reject a stale/replaced block instead of overwriting the
    /// first code block it happens to find.
    CodeBlockOpenRequested {
        /// The PostgreSQL-backed rich document that owns the code block.
        document_id: String,
        /// Exact owning-document structural snapshot captured with the block path. The save bridge
        /// rejects any intervening document change, including identical-text positional drift.
        document_snapshot: serde_json::Value,
        /// Exact child-index path from the document root to the selected code block.
        block_path: Vec<usize>,
        /// Optional Tiptap `attrs.language` value used to seed native syntax highlighting.
        language: String,
        /// Exact code snapshot displayed when the native code panel was opened.
        code: String,
    },
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
    /// WP-KERNEL-012 MT-058: an inline `#tag` chip was clicked. The shell routes the tag onto the
    /// WP-011 navigation/command bus (`command_registry` + `event_bus`) so the MT-023 tag hub for the
    /// tag opens — the chip NEVER opens the hub directly (RISK-005 / MC-005, mirroring the wikilink
    /// navigation-request pattern). Carries the tag's CANONICAL identity (the hub-resolution key, the
    /// same normalized identity property tags converge on) and the original-case display name.
    TagActivated {
        /// The tag's canonical (normalized) identity — the hub-resolution + convergence key.
        canonical: String,
        /// The original-case display name (without the leading `#`), for the hub title / display.
        display: String,
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
/// canonical form `editor.rich.wikilink.candidate.{document_id}` (the document the row inserts a link to). The
/// document id is used verbatim (it is already a stable opaque id), so a swarm agent / kittest targets
/// a candidate by the document it resolves to.
pub fn candidate_author_id(document_id: &str) -> String {
    format!("editor.rich.wikilink.candidate.{document_id}")
}

/// A short, stable 32-bit hex hash for create-affordance ids. Generic wikilink chip identity does not
/// use this lossy hash; [`chip_author_id`] encodes the complete target bytes injectively.
fn short_hex_hash(bytes: &[u8]) -> String {
    format!("{:08x}", fnv1a_hash(bytes))
}

/// The canonical base AccessKit author_id for a generic wikilink chip. The complete UTF-8 target is
/// hex encoded byte-for-byte, so distinct targets cannot collide through a truncated hash.
pub fn chip_author_id(ref_value: &str) -> String {
    use std::fmt::Write as _;
    let mut encoded = String::with_capacity(ref_value.len() * 2);
    for byte in ref_value.as_bytes() {
        let _ = write!(encoded, "{byte:02x}");
    }
    format!("editor.rich.wikilink.chip.v{encoded}")
}

/// Collision-safe identity for one exact generic wikilink occurrence. `document_path` is encoded as
/// an unambiguous sequence of hexadecimal indices and never includes screen position, so duplicate
/// links remain distinct and stable across wrapping/repaint changes.
pub fn chip_occurrence_author_id(ref_value: &str, document_path: &[usize]) -> String {
    let path = if document_path.is_empty() {
        "root".to_owned()
    } else {
        document_path
            .iter()
            .map(|index| format!("{index:x}"))
            .collect::<Vec<_>>()
            .join("-")
    };
    format!("{}.path.{path}", chip_author_id(ref_value))
}

/// Deterministic 32-bit FNV-1a hash retained for create-affordance and malformed specialized-link
/// fallbacks. Generic chip identity uses the injective full-byte encoding above.
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
/// verbatim (NOT the hashed generic wikilink-chip id) — the contract names the id by the symbol entity id so a
/// swarm agent / kittest can target the chip by the symbol it references.
pub fn code_ref_chip_author_id(symbol_ref: &str) -> String {
    format!("code-ref-chip-{symbol_ref}")
}

/// True when this hsLink atom is a WP-KERNEL-012 MT-068 LOCUS cross-reference (ref_kind="locus"). A locus
/// ref gets a distinct chip author_id (`locus-ref-chip-{kind}-{id}`) and a work-unit-styled label so it
/// reads as a Locus WP/MT pill, not a generic wikilink. The SIBLING of [`is_code_ref`].
pub fn is_locus_ref(link: &HsLinkNode) -> bool {
    link.ref_kind == crate::interop::locus_interop::LOCUS_REF_KIND
}

/// The AccessKit author_id for a LOCUS-reference chip (`locus-ref-chip-{kind}-{id}` per the MT-068
/// contract, e.g. `locus-ref-chip-wp-WP-KERNEL-012`). The `kind` (`wp`/`mt`) + the work-unit `id` are
/// parsed from the chip's `ref_value` (the `locus://` ref); a `ref_value` that does not parse falls back to
/// hashing the raw value so the chip is still addressable (never a panic). Ids are stable across frames so
/// a swarm agent / kittest can target the chip by the work unit it references. Parsed ids escape the
/// occurrence/view-suffix grammar injectively: ordinary ids keep the canonical raw contract form, while
/// `%` and the reserved `--path-` / `--view-` tokens are encoded so an authored id can never alias a
/// repeated occurrence or a secondary-pane view.
pub fn locus_ref_chip_author_id(ref_value: &str) -> String {
    match crate::interop::locus_interop::parse_locus_ref(ref_value) {
        Some(r) => format!(
            "locus-ref-chip-{}-{}",
            r.kind.as_str(),
            encode_locus_author_id_component(&r.id)
        ),
        // A non-parsing locus ref (defensive) is still addressable via the deterministic hash.
        None => format!(
            "locus-ref-chip-unknown-{}",
            fnv1a_hash(ref_value.as_bytes())
        ),
    }
}

const LOCUS_OCCURRENCE_PATH_MARKER: &str = "--path-";
const LOCUS_PANE_VIEW_MARKER: &str = "--view-";

/// Escape the reserved author-id suffix tokens without changing normal WP/MT ids. `%` is escaped first,
/// making the mapping injective even when an authored id literally contains an escape spelling.
fn encode_locus_author_id_component(id: &str) -> String {
    id.replace('%', "%25")
        .replace(LOCUS_OCCURRENCE_PATH_MARKER, "%2D%2Dpath%2D")
        .replace(LOCUS_PANE_VIEW_MARKER, "%2D%2Dview%2D")
}

/// Return the stable AccessKit identity for one occurrence of a Locus chip. The first occurrence keeps
/// the MT-068 contract id (`locus-ref-chip-{kind}-{id}`); repeated identical refs append their stable
/// document-tree path so every live node remains uniquely addressable without depending on screen
/// position or line wrapping. `occurrence_index` is counted in document order for this base identity.
pub fn locus_ref_chip_occurrence_author_id(
    ref_value: &str,
    document_path: &[usize],
    occurrence_index: usize,
) -> String {
    let base = locus_ref_chip_author_id(ref_value);
    if occurrence_index == 0 {
        return base;
    }
    let path = document_path
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join("-");
    if path.is_empty() {
        format!("{base}{LOCUS_OCCURRENCE_PATH_MARKER}root")
    } else {
        format!("{base}{LOCUS_OCCURRENCE_PATH_MARKER}{path}")
    }
}

/// Decode one escaped Locus author-id component. The exact inverse of
/// [`encode_locus_author_id_component`]: the reserved markers are restored first and `%25` last, so the
/// mapping stays injective in both directions.
fn decode_locus_author_id_component(encoded: &str) -> String {
    encoded
        .replace("%2D%2Dpath%2D", LOCUS_OCCURRENCE_PATH_MARKER)
        .replace("%2D%2Dview%2D", LOCUS_PANE_VIEW_MARKER)
        .replace("%25", "%")
}

/// Recover the exact canonical `locus://{kind}/{id}` URI a mounted Locus chip author id addresses — the
/// inverse of [`locus_ref_chip_author_id`] / [`locus_ref_chip_occurrence_author_id`].
///
/// WP-KERNEL-012 MT-068 V5 uses this so the SHELL can publish a chip's click-completion declaration
/// straight from the authoritative MCP snapshot: the declaration a chip carries and the identity its
/// completion binds are then the same tuple the click path itself stages, with no second source of
/// truth. The occurrence/view suffixes are split at their FIRST marker, which is unambiguous because
/// [`encode_locus_author_id_component`] escapes any literal marker inside an authored id.
///
/// Returns `None` for any id that is not a canonical Locus chip id (including the defensive
/// `locus-ref-chip-unknown-{hash}` fallback, which is deliberately not invertible), and for any id
/// whose decoded identity does not reproduce the same author id through the forward mapping.
pub fn locus_ref_uri_from_chip_author_id(author_id: &str) -> Option<String> {
    let rest = author_id.strip_prefix("locus-ref-chip-")?;
    let (kind_segment, encoded) = rest.split_once('-')?;
    if !matches!(kind_segment, "wp" | "mt") {
        return None;
    }
    let encoded = encoded
        .split_once(LOCUS_OCCURRENCE_PATH_MARKER)
        .map_or(encoded, |(head, _)| head);
    let encoded = encoded
        .split_once(LOCUS_PANE_VIEW_MARKER)
        .map_or(encoded, |(head, _)| head);
    if encoded.is_empty() {
        return None;
    }
    let id = decode_locus_author_id_component(encoded);
    let uri = format!(
        "{}{kind_segment}/{id}",
        crate::interop::locus_interop::LOCUS_URI_SCHEME
    );
    // Fail closed unless the decoded identity reproduces this exact base author id through the SAME
    // forward mapping the renderer uses. A lossy or non-canonical id is never silently mis-addressed.
    (locus_ref_chip_author_id(&uri) == format!("locus-ref-chip-{kind_segment}-{encoded}"))
        .then_some(uri)
}

/// The SHORT display name for a Locus `locus://` ref: the work-unit id (e.g. `locus://wp/WP-KERNEL-012` ->
/// `WP-KERNEL-012`). Falls back to the whole value when it does not parse (never a panic).
pub fn locus_ref_short_name(ref_value: &str) -> String {
    match crate::interop::locus_interop::parse_locus_ref(ref_value) {
        Some(r) => r.id,
        None => ref_value.to_owned(),
    }
}

/// The SHORT display name for a code-symbol key/label (the last `::` segment, then the last `#`
/// segment — `path/to/file.rs#Mod::MyStruct` -> `MyStruct`), per the MT-034 chip rendering note
/// ("Show the symbol_key short form: last '::' segment"). Falls back to the whole string when there is
/// no separator.
pub fn code_ref_short_name(symbol_key_or_label: &str) -> String {
    let after_hash = symbol_key_or_label
        .rsplit('#')
        .next()
        .unwrap_or(symbol_key_or_label);
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
    if is_locus_ref(link) {
        // MT-068: a Locus chip shows the short work-unit id with a small work-unit glyph (a clipboard/task
        // marker), so it reads as a Locus WP/MT pill. An UNRESOLVED/UNAVAILABLE locus ref (the record was
        // not found, or — the designed path in this build — the Locus READ API is not exposed) shows the
        // greyed `unresolved` text + the `?` prefix, without crashing (AC-005 / AC-006).
        let name = locus_ref_short_name(&link.ref_value);
        return if link.resolved {
            format!("⎘ {name}")
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
pub fn chip_rect_for_span(local_start: Rect, local_end: Rect, origin: egui::Pos2) -> Rect {
    // A long atom can cross a wrapped-row boundary, where the end cursor's X is left of the start
    // cursor's X. Never construct a negative-width Rect: egui normalizes it for accessibility output,
    // but pointer/AccessKit activation still hit-tests the original invalid response rect and drops the
    // click. Enclose both cursor rects, including both rows when wrapped, then add the pill's 1px
    // horizontal padding.
    let start = local_start.translate(origin.to_vec2());
    let end = local_end.translate(origin.to_vec2());
    Rect::from_min_max(
        egui::pos2(start.min.x.min(end.min.x) - 1.0, start.min.y.min(end.min.y)),
        egui::pos2(start.max.x.max(end.max.x) + 1.0, start.max.y.max(end.max.y)),
    )
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
        assert!(
            a.starts_with("editor.rich.wikilink.chip."),
            "the contract author_id prefix"
        );
        assert_ne!(
            chip_author_id("a"),
            chip_author_id("b"),
            "distinct refs -> distinct ids"
        );
        assert_ne!(
            chip_occurrence_author_id("same", &[1, 23]),
            chip_occurrence_author_id("same", &[12, 3]),
            "path component boundaries are injective"
        );
        assert_ne!(
            chip_occurrence_author_id("same", &[0, 1]),
            chip_occurrence_author_id("same", &[0, 3]),
            "repeated identical targets receive unique occurrence ids"
        );
    }

    #[test]
    fn label_uses_explicit_then_falls_back() {
        let with_label = HsLinkNode {
            ref_kind: "wp".into(),
            ref_value: "WP-7".into(),
            label: "Seven".into(),
            resolved: true,
            provenance: None,
        };
        assert_eq!(chip_label(&with_label), "Seven");
        let no_label = HsLinkNode {
            ref_kind: "wp".into(),
            ref_value: "WP-7".into(),
            label: String::new(),
            resolved: true,
            provenance: None,
        };
        assert_eq!(
            chip_label(&no_label),
            "wp:WP-7",
            "falls back to ref_kind:ref_value"
        );
    }

    #[test]
    fn code_ref_chip_id_uses_symbol_ref_verbatim() {
        // MT-034: the code-ref chip id is `code-ref-chip-{symbol_entity_id}` (the symbol the chip
        // references), used verbatim — NOT the hashed generic wikilink-chip id.
        assert_eq!(code_ref_chip_author_id("ent-42"), "code-ref-chip-ent-42");
        let link = HsLinkNode {
            ref_kind: "code".into(),
            ref_value: "ent-42".into(),
            label: String::new(),
            resolved: true,
            provenance: None,
        };
        assert!(is_code_ref(&link));
        let wp = HsLinkNode {
            ref_kind: "wp".into(),
            ref_value: "WP-1".into(),
            label: String::new(),
            resolved: true,
            provenance: None,
        };
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
        let resolved = HsLinkNode {
            ref_kind: "code".into(),
            ref_value: "ent-1".into(),
            label: "src/main.rs#MyStruct".into(),
            resolved: true,
            provenance: None,
        };
        let lbl = chip_label(&resolved);
        assert!(
            lbl.contains("MyStruct"),
            "resolved code chip shows the short symbol name"
        );
        assert!(!lbl.contains("unresolved"));
        let unresolved = HsLinkNode {
            ref_kind: "code".into(),
            ref_value: "ent-9".into(),
            label: "src/gone.rs#Gone".into(),
            resolved: false,
            provenance: None,
        };
        let ul = chip_label(&unresolved);
        assert!(
            ul.contains("unresolved"),
            "an unresolved code chip reads as broken"
        );
        assert!(
            ul.starts_with("? "),
            "unresolved keeps the broken-link `?` prefix"
        );
        assert!(ul.contains("Gone"));
    }

    #[test]
    fn unresolved_label_carries_question_prefix() {
        let unknown = HsLinkNode {
            ref_kind: "unknown".into(),
            ref_value: "xyz".into(),
            label: String::new(),
            resolved: false,
            provenance: None,
        };
        assert_eq!(
            chip_label(&unknown),
            "? unknown:xyz",
            "an unresolved chip reads as broken"
        );
    }

    #[test]
    fn colors_come_from_theme_resolved_vs_unresolved() {
        let pal = dark();
        let resolved = HsLinkNode {
            ref_kind: "wp".into(),
            ref_value: "x".into(),
            label: String::new(),
            resolved: true,
            provenance: None,
        };
        let (bg, fg) = chip_colors(&resolved, &pal);
        assert_eq!(bg, pal.accent_soft);
        assert_eq!(fg, pal.accent);
        let unresolved = HsLinkNode {
            ref_kind: "unknown".into(),
            ref_value: "x".into(),
            label: String::new(),
            resolved: false,
            provenance: None,
        };
        let (bg2, fg2) = chip_colors(&unresolved, &pal);
        assert_eq!(
            bg2, pal.error_bg,
            "unresolved uses the error background (visible broken link)"
        );
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
        assert_eq!(
            rect.min.y, 200.0,
            "chip Y follows the scroll-adjusted origin exactly"
        );
        assert!(
            (rect.height() - 18.0).abs() < 0.01,
            "chip height is the glyph row height"
        );
    }

    #[test]
    fn chip_rect_normalizes_a_wrapped_span_for_pointer_activation() {
        let local_start = Rect::from_min_size(egui::pos2(120.0, 0.0), Vec2::new(0.0, 18.0));
        let local_end = Rect::from_min_size(egui::pos2(30.0, 18.0), Vec2::new(0.0, 18.0));
        let rect = chip_rect_for_span(local_start, local_end, egui::pos2(10.0, 200.0));

        assert!(
            rect.is_positive(),
            "wrapped chips must have a valid hit-test rect"
        );
        assert_eq!(rect.min, egui::pos2(39.0, 200.0));
        assert_eq!(rect.max, egui::pos2(131.0, 236.0));
    }

    #[test]
    fn editor_event_shapes_round_trip_for_shell_routing() {
        // The events are small value types the shell drains; assert their fields carry the routing
        // payload the WP-011 host needs.
        let wl = EditorEvent::WikilinkActivated {
            ref_kind: "wp".into(),
            ref_value: "WP-1".into(),
            resolved: true,
        };
        match wl {
            EditorEvent::WikilinkActivated {
                ref_kind,
                ref_value,
                resolved,
            } => {
                assert_eq!(ref_kind, "wp");
                assert_eq!(ref_value, "WP-1");
                assert!(resolved);
            }
            _ => panic!("variant"),
        }
        let bl = EditorEvent::BacklinkActivated {
            source_document_id: "DOC-2".into(),
        };
        assert!(
            matches!(bl, EditorEvent::BacklinkActivated { source_document_id } if source_document_id == "DOC-2")
        );
    }
}
