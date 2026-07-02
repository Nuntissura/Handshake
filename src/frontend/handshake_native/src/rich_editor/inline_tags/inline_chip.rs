//! Inline `#tag` chip identity + activation event + the autocomplete menu state + the convergence
//! edge-payload builder (WP-KERNEL-012 MT-058).
//!
//! ## The chip is the EXISTING wikilink/hsLink chip render path (REUSE-NOT-FORK)
//!
//! A committed inline tag is an `hsLink` atom with `ref_kind = "tag"`
//! ([`crate::rich_editor::inline_tags::parser::tag_to_hs_link`]), so it renders through the EXISTING
//! MT-012 inline-mark / MT-015 chip pipeline
//! ([`crate::rich_editor::renderer::rich_editor_widget::paint_one_wikilink_chip`]) — there is NO second
//! chip render path. This module supplies (a) the tag-specific AccessKit author_id
//! ([`inline_tag_author_id`], the contract `inline-tag-{name}` id with `Role::Link`), (b) the
//! [`TagActivated`] navigation event the chip emits (NEVER opening the hub directly — RISK-005 / MC-005,
//! the wikilink navigation-request pattern), and (c) the convergence edge-payload builder
//! ([`build_tag_edge_payload`]) that unions inline + property tags into ONE deduped edge per distinct
//! normalized identity on document commit (RISK-004 / MC-004). The host special-cases `ref_kind="tag"`
//! in the chip paint to use THIS author_id + emit [`TagActivated`].
//!
//! No egui `Color32` literal lives here — chip colors come from the theme palette through the shared
//! `chip_colors` path (CONTROL: no hardcoded hex). This module carries only the identity/event/builder
//! logic + the menu state; the egui draw is the shared chip painter.

use crate::rich_editor::document_model::node::HsLinkNode;
use crate::rich_editor::inline_tags::parser::{normalize_tag, Tag};

/// The navigation event a clicked inline-tag chip emits (carried on the editor's
/// [`crate::rich_editor::wikilinks::inline_view::EditorEvent`] command bus as
/// `EditorEvent::TagActivated`). The host routes it onto the WP-011 navigation/command bus
/// (`command_registry` + `event_bus`) so the MT-023 tag hub opens — the chip NEVER opens a window
/// itself (RISK-005 / MC-005, mirroring the wikilink navigation-request pattern). Carries the tag
/// identity so the host resolves the hub for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagActivated {
    /// The activated tag (display name + canonical identity via [`Tag::canonical`]).
    pub tag: Tag,
}

impl TagActivated {
    /// Build the activation event for `tag`.
    pub fn new(tag: Tag) -> Self {
        Self { tag }
    }
}

/// The AccessKit author_id for an inline-tag chip, of the contract form `inline-tag-{name}` where
/// `{name}` is the tag's CANONICAL identity (normalized — so `#Rust` and `#rust` address the same
/// stable id, and a swarm agent / kittest can target a tag by a value it computes independently). The
/// chip carries `Role::Link` (see [`crate::rich_editor::wikilinks::inline_view::CHIP_ROLE`]).
///
/// MC-006 (repeated-tag collision): two `#rust` occurrences in ONE document share this author_id
/// STRING (a match key, not a uniqueness key). They do NOT collide in the AccessKit TREE because the
/// chip's egui `NodeId` is derived per call-position (`ui.id().with(("inline-tag-chip", &author))` in a
/// distinct paint scope), so each occurrence is a distinct node with the same addressable author_id —
/// the same idempotent pattern the wikilink chip uses for a repeated ref value.
pub fn inline_tag_author_id(tag: &Tag) -> String {
    format!("inline-tag-{}", tag.canonical())
}

/// True when an `hsLink` atom is an inline TAG (`ref_kind == "tag"`). Used by the host chip painter to
/// dispatch a tag chip's distinct author_id + [`TagActivated`] event instead of the generic
/// wikilink path.
pub fn is_tag_link(link: &HsLinkNode) -> bool {
    link.ref_kind == TAG_REF_KIND
}

/// The backend `ref_kind` discriminator for an inline tag atom. The atom round-trips the backend
/// `content_json` under this kind, and the backend tag-edge indexer keys a `ref_kind="tag"` atom on
/// its `ref_value` (the normalized identity) — the same "everything is an hsLink by ref_kind" dispatch
/// the wikilink/embed/code-ref atoms use.
pub const TAG_REF_KIND: &str = "tag";

/// Reconstruct the [`Tag`] an inline-tag `hsLink` atom represents. The display name is recovered from
/// the chip label (`"#name"` -> `name`) when present, else from the `ref_value` (the canonical
/// identity). Used by the host to build the [`TagActivated`] event from a clicked chip.
pub fn tag_from_link(link: &HsLinkNode) -> Tag {
    let display = link
        .label
        .strip_prefix('#')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| link.ref_value.clone());
    Tag::new(display)
}

// ════════════════════════════════════════════════════════════════════════════════════════════════════
// Convergence edge-payload builder (RISK-004 / MC-004 — one tag, one hub).
// ════════════════════════════════════════════════════════════════════════════════════════════════════

/// One deduped tag-edge to persist on document commit: the NORMALIZED canonical identity (the dedupe
/// key + the hub the edge tags into) plus the original-case display name (for hub creation / display).
/// The backend persists this as a `POST /loom/edges { source_block_id: <doc block>, target_block_id:
/// <tag_hub for canonical>, edge_type: "tag", created_by: "user" }` — ONE edge per distinct canonical
/// tag, no duplicate (a tag present BOTH inline and as a property tag yields exactly ONE edge).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagEdge {
    /// The canonical normalized identity (`normalize_tag` of the name) — the dedupe key AND the hub
    /// the edge resolves to. Two tags with the same canonical map to the SAME edge (one tag, one hub).
    pub canonical: String,
    /// The original-case display name (first occurrence wins) for hub creation / display.
    pub display: String,
}

/// The full deduped convergence payload for a document's commit: the ordered distinct tag edges. The
/// LIVE `POST /loom/edges` round-trip is gated (NEEDS_MANAGED_RESOURCE_PROOF — it requires a live
/// managed PostgreSQL + the per-canonical tag_hub block id; the verified backend tags an edge into a
/// tag_hub BLOCK, not a name string, so hub resolution is a backend-managed step). This BUILDER output
/// is provable standalone (AC-005): it asserts the union+dedupe is correct BEFORE any network call.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagEdgePayload {
    /// The deduped, document-ordered tag edges to persist (one per distinct canonical identity).
    pub edges: Vec<TagEdge>,
}

impl TagEdgePayload {
    /// The distinct canonical identities in this payload (the hub set the document tags into).
    pub fn canonical_set(&self) -> Vec<String> {
        self.edges.iter().map(|e| e.canonical.clone()).collect()
    }

    /// The number of distinct tag edges (== distinct canonical identities).
    pub fn len(&self) -> usize {
        self.edges.len()
    }

    /// True when the document has no tags to persist.
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

/// Build the deduped convergence edge payload from a document's INLINE tags unioned with its
/// PROPERTY-panel tags (RISK-004 / MC-004 — the one-tag-one-hub union). Each input string is
/// normalized via [`normalize_tag`]; the FIRST occurrence's display name wins; identical canonical
/// identities collapse to ONE edge. Inline-first ordering, then any property-only tags, preserves a
/// stable document order. A tag present BOTH inline and as a property tag yields EXACTLY ONE edge (the
/// AC-005 no-duplicate invariant). Empty / whitespace-only inputs that normalize to an empty canonical
/// are dropped (not a panic, not a blank edge).
///
/// This fires at document COMMIT/SAVE, NEVER per keystroke (RISK-004 / MC-004) — the caller invokes it
/// once on save. Pure over its inputs, so the dedupe is unit-provable with no backend.
pub fn build_tag_edge_payload<I, P>(inline_tags: I, property_tags: P) -> TagEdgePayload
where
    I: IntoIterator,
    I::Item: AsRef<str>,
    P: IntoIterator,
    P::Item: AsRef<str>,
{
    let mut edges: Vec<TagEdge> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Inline tags first (document order), then property-only tags. Both go through the SAME
    // normalization so a tag that exists in both sets dedupes to one edge.
    for raw in inline_tags
        .into_iter()
        .map(|s| s.as_ref().to_owned())
        .chain(property_tags.into_iter().map(|s| s.as_ref().to_owned()))
    {
        let canonical = normalize_tag(&raw);
        if canonical.is_empty() {
            continue; // not a real tag identity.
        }
        if seen.insert(canonical.clone()) {
            // The display name is the original-case body without a leading `#` (first occurrence wins).
            let display = raw.trim().trim_start_matches('#').trim().to_owned();
            let display = if display.is_empty() {
                canonical.clone()
            } else {
                display
            };
            edges.push(TagEdge { canonical, display });
        }
    }
    TagEdgePayload { edges }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════════
// Tag autocomplete menu state (the `#` popup — the LIVE inline menu, mirroring the wikilink popup).
// ════════════════════════════════════════════════════════════════════════════════════════════════════

/// One row in the inline-tag autocomplete menu: a tag the operator can select. Sourced from the MT-023
/// tag-hub list (`GET /loom/tags`, the VERIFIED route — the contract's `?content_type=tag_hub` filter
/// does NOT exist; see [`crate::backend_client::LoomTagClient`]), PLUS a synthetic "create new tag" row
/// for a free-typed tag not yet in the list (AC-006 — authoring a tag in the body is a valid create
/// path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagMenuItem {
    /// The tag this row commits (display name + canonical identity).
    pub tag: Tag,
    /// True when this is the synthetic "create #{query} (new tag)" row (the free-typed tag not in the
    /// existing list). The host renders it with a distinct hint; selecting it commits a brand-new tag.
    pub is_new: bool,
}

impl TagMenuItem {
    /// An existing-tag row (from the tag-hub list).
    pub fn existing(name: impl Into<String>) -> Self {
        Self {
            tag: Tag::new(name),
            is_new: false,
        }
    }

    /// The synthetic "create new tag" row for a free-typed `query`.
    pub fn new_tag(name: impl Into<String>) -> Self {
        Self {
            tag: Tag::new(name),
            is_new: true,
        }
    }

    /// The display label for the menu row (`#name`, with a `(new tag)` suffix for a create row).
    pub fn label(&self) -> String {
        if self.is_new {
            format!("{}  (new tag)", self.tag.display_label())
        } else {
            self.tag.display_label()
        }
    }
}

/// The LIVE inline-tag autocomplete popup state stored on the editor (`RichEditorState.tag_autocomplete`,
/// `Some` while the operator is typing a `#` trigger). Mirrors the wikilink
/// [`crate::rich_editor::wikilinks::autocomplete::AutocompleteState`] shape (trigger span + query +
/// selection) but the item source is the cached tag-hub list (filtered live), NOT a backend search, and
/// it ALWAYS allows committing a free-typed new tag (AC-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagAutocompleteState {
    /// The CHAR offset within the caret's text leaf where the `#` trigger opened (so confirm/cancel can
    /// compute the char span of `#query` to remove — the doc is CHAR-indexed, RISK-003).
    pub trigger_start_char: usize,
    /// The block path of the text leaf the trigger lives in (so confirm edits the right leaf).
    pub leaf_path: Vec<usize>,
    /// The tag body typed after `#` (the live filter; WITHOUT the `#`).
    pub query: String,
    /// The highlighted row index (Arrow keys move it; Enter/Tab commits it).
    pub selected: usize,
}

impl TagAutocompleteState {
    /// Open the popup for a `#` trigger at `trigger_start_char` in the leaf at `leaf_path`, with the
    /// initial `query` typed so far.
    pub fn open(trigger_start_char: usize, leaf_path: Vec<usize>, query: String) -> Self {
        Self {
            trigger_start_char,
            leaf_path,
            query,
            selected: 0,
        }
    }

    /// Update the typed query (a keystroke refined the trigger). Resets the selection to the top (the
    /// filtered list changed). A no-op when the query is unchanged.
    pub fn set_query(&mut self, query: String) {
        if query != self.query {
            self.query = query;
            self.selected = 0;
        }
    }

    /// Move the selection down by one, clamped to `item_count`.
    pub fn select_next(&mut self, item_count: usize) {
        if item_count == 0 {
            self.selected = 0;
        } else {
            self.selected = (self.selected + 1).min(item_count - 1);
        }
    }

    /// Move the selection up by one, clamped at 0.
    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }
}

/// Build the live menu rows for `query` against the cached `available` tag names (the MT-023 tag-hub
/// list). The existing tags whose CANONICAL identity contains the normalized query are listed first
/// (prefix matches before substring matches, then alphabetical), deduped by canonical identity. A
/// synthetic "create new tag" row for the free-typed `query` is appended UNLESS the query's canonical
/// identity already exactly matches an existing tag (so a tag already in the list is not offered as
/// "new"). An empty query lists the available tags with no create row. AC-006: a brand-new tag not in
/// the list is always committable through the create row.
///
/// Pure (no egui) so the filter + the free-typed-create rule are unit-provable standalone.
pub fn tag_menu_items(query: &str, available: &[String]) -> Vec<TagMenuItem> {
    let nq = normalize_tag(query);

    // Dedupe the available list by canonical identity, keeping the first display form.
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut existing: Vec<(i32, String)> = Vec::new(); // (rank, display_name)
    let mut exact_existing = false;
    for name in available {
        let tag = Tag::new(name.strip_prefix('#').unwrap_or(name));
        let canonical = tag.canonical();
        if canonical.is_empty() || !seen.insert(canonical.clone()) {
            continue;
        }
        // Rank: 0 = exact, 1 = prefix, 2 = substring, 3 = (empty query) all. Non-matches dropped.
        let rank = if nq.is_empty() {
            3
        } else if canonical == nq {
            exact_existing = true;
            0
        } else if canonical.starts_with(&nq) {
            1
        } else if canonical.contains(&nq) {
            2
        } else {
            continue;
        };
        existing.push((rank, tag.name));
    }
    // Sort by rank then display name (stable, deterministic).
    existing.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
    });

    let mut items: Vec<TagMenuItem> = existing
        .into_iter()
        .map(|(_, name)| TagMenuItem::existing(name))
        .collect();

    // AC-006: offer a "create new tag" row for the free-typed query unless it already exists exactly.
    if !nq.is_empty() && !exact_existing {
        items.push(TagMenuItem::new_tag(
            query.trim().trim_start_matches('#').trim(),
        ));
    }
    items
}

/// Build the `hsLink` atom a selected/committed [`TagMenuItem`] inserts (delegates to the parser's
/// canonical [`crate::rich_editor::inline_tags::parser::tag_to_hs_link`]). Re-exported here so the
/// commit path imports one place.
pub fn menu_item_to_hs_link(item: &TagMenuItem) -> HsLinkNode {
    crate::rich_editor::inline_tags::parser::tag_to_hs_link(&item.tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn author_id_is_canonical_and_prefixed_mc006() {
        // The author_id is the contract `inline-tag-{name}` with the CANONICAL (normalized) identity,
        // so `#Rust` and `#rust` address the SAME stable id (deterministic for swarm/kittest targeting).
        let a = inline_tag_author_id(&Tag::new("Rust"));
        let b = inline_tag_author_id(&Tag::new("rust"));
        assert_eq!(a, "inline-tag-rust");
        assert_eq!(
            a, b,
            "MC-006: `#Rust` and `#rust` share the canonical author_id"
        );
        assert_ne!(
            inline_tag_author_id(&Tag::new("wip")),
            a,
            "distinct tags -> distinct ids"
        );
    }

    #[test]
    fn tag_link_round_trip_identity() {
        // A tag atom is an hsLink with ref_kind="tag"; reconstructing the tag recovers the display name.
        let tag = Tag::new("Rust");
        let link = menu_item_to_hs_link(&TagMenuItem::existing("Rust"));
        assert!(is_tag_link(&link), "the atom is a tag link");
        assert_eq!(link.ref_kind, TAG_REF_KIND);
        assert_eq!(
            link.ref_value, "rust",
            "ref_value is the canonical identity"
        );
        let recovered = tag_from_link(&link);
        assert_eq!(
            recovered.name, "Rust",
            "the display name is recovered from the chip label"
        );
        assert_eq!(recovered.canonical(), tag.canonical());
    }

    #[test]
    fn convergence_dedupes_inline_and_property_tags_ac005() {
        // AC-005: a document with inline #rust AND a property tag 'rust' persists EXACTLY ONE edge for
        // 'rust' (deduped by canonical identity — no duplicate).
        let payload = build_tag_edge_payload(["rust", "wip"], ["Rust", "design"]);
        let canon = payload.canonical_set();
        assert_eq!(
            payload.len(),
            3,
            "rust (inline+property) dedupes to one; +wip +design (got {canon:?})"
        );
        assert_eq!(
            canon,
            vec!["rust".to_owned(), "wip".to_owned(), "design".to_owned()]
        );
        // The 'rust' edge appears exactly once.
        let rust_edges = payload
            .edges
            .iter()
            .filter(|e| e.canonical == "rust")
            .count();
        assert_eq!(
            rust_edges, 1,
            "AC-005: exactly ONE 'rust' edge despite inline + property occurrence"
        );
    }

    #[test]
    fn convergence_drops_empty_and_preserves_first_display() {
        // Empty/whitespace inputs that normalize to empty are dropped; the first display-case wins.
        let payload = build_tag_edge_payload(["#Rust", "", "  "], ["rust"]);
        assert_eq!(payload.len(), 1);
        assert_eq!(payload.edges[0].canonical, "rust");
        assert_eq!(
            payload.edges[0].display, "Rust",
            "first occurrence's display case wins"
        );
    }

    #[test]
    fn convergence_empty_when_no_tags() {
        let payload = build_tag_edge_payload(Vec::<String>::new(), Vec::<String>::new());
        assert!(payload.is_empty());
        assert_eq!(payload.len(), 0);
    }

    #[test]
    fn menu_items_filter_and_offer_free_typed_create_ac006() {
        let available = vec![
            "rust".to_owned(),
            "rustaceans".to_owned(),
            "python".to_owned(),
        ];
        // Query "rust": rust (exact, no 'new' row) + rustaceans (prefix). NO create row (rust exists).
        let items = tag_menu_items("rust", &available);
        let labels: Vec<String> = items.iter().map(|i| i.label()).collect();
        assert!(
            items
                .iter()
                .any(|i| i.tag.canonical() == "rust" && !i.is_new),
            "existing rust listed"
        );
        assert!(
            items.iter().any(|i| i.tag.canonical() == "rustaceans"),
            "prefix rustaceans listed"
        );
        assert!(
            !items.iter().any(|i| i.is_new),
            "no create row when the query exactly matches an existing tag (got {labels:?})"
        );

        // AC-006: a brand-new free-typed tag not in the list IS committable via a create row.
        let items2 = tag_menu_items("newtag", &available);
        assert!(
            items2
                .iter()
                .any(|i| i.is_new && i.tag.canonical() == "newtag"),
            "AC-006: a free-typed new tag offers a create row (got {:?})",
            items2.iter().map(|i| i.label()).collect::<Vec<_>>()
        );

        // Empty query lists existing tags, no create row.
        let items3 = tag_menu_items("", &available);
        assert!(!items3.is_empty());
        assert!(
            !items3.iter().any(|i| i.is_new),
            "empty query offers no create row"
        );
    }

    #[test]
    fn autocomplete_selection_clamps() {
        let mut st = TagAutocompleteState::open(0, vec![0, 0], String::new());
        st.select_next(2);
        assert_eq!(st.selected, 1);
        st.select_next(2);
        assert_eq!(st.selected, 1, "clamped to item_count-1");
        st.select_prev();
        assert_eq!(st.selected, 0);
        st.select_prev();
        assert_eq!(st.selected, 0, "clamped at 0");
        // set_query resets selection.
        st.selected = 1;
        st.set_query("ru".to_owned());
        assert_eq!(st.selected, 0, "a query change resets the selection");
    }
}
