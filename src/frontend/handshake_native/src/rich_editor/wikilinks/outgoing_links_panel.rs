//! The Outgoing Links pane (WP-KERNEL-012 MT-062) — the third leg of the Obsidian "links triad" for
//! the native knowledge surface: backlinks (MT-015 — links INTO this doc) + unlinked mentions
//! (MT-024 — text mentions not yet linked) + outgoing links (THIS MT — links OUT of this doc).
//!
//! The pane lists every link emanating FROM the active document — both wikilinks
//! (`hsLink` atoms: `[[target]]` / `[[target|alias]]`) and transclusions (`loomTransclusion` atoms:
//! `![[target]]` embeds) — extracted from the document's Tiptap `content_json` DocModel tree. Entries
//! are split into two buckets, **Resolved** and **Unresolved** (dangling), and every entry is
//! clickable: clicking routes the target through the shell navigation seam (the MT-030 navigation bus
//! reused over the WP-011 command/event bus) via the `on_open: FnMut(NavTarget)` callback.
//!
//! ## What is REUSED (no fork — RISK-001/MC-001, RISK-008/MC-008, RISK-007/MC-007)
//!
//! - The MT-015 wikilink parser ([`crate::rich_editor::wikilinks::parser`]): [`extract_outgoing_links`]
//!   classifies each extracted `hsLink` atom via [`parser::classify_wikilink`] — it defines NO local
//!   regex / parser (AC-003 grep-gate). Parse drift between the backlinks pane and this pane would
//!   silently break the triad, so both go through the one parser.
//! - The MT-057 resolution engine ([`crate::rich_editor::wikilinks::resolver`]): [`bucket_links`]
//!   resolves each link with [`resolver::resolve_wikilink`] and keys de-duplication on
//!   [`resolver::normalize_target`] — the SAME normalization key the resolver looks targets up by, so a
//!   link can never be bucketed against a different key than the resolver used (RISK-003/MC-003).
//! - The WP-011 AccessKit live-emission pattern (`accesskit_node_builder` + `set_author_id`, exactly as
//!   `backlinks_panel.rs` does it): every entry + section container carries a stable, de-duplicated
//!   `author_id` so swarm agents can address rows deterministically (RISK-004/MC-004).
//! - The WP-011 theme tokens ([`crate::theme::HsPalette`]): every color is a semantic token; NO
//!   `Color32` literal lives here (CONTROL-4, grep-enforced by `tests/test_theme.rs`).
//! - The backend is touched READ-ONLY through the two already-bound GETs
//!   (`GET /knowledge/documents/{id}` for `content_json`, `GET /loom/blocks/{id}` to resolve targets);
//!   this MT adds NO endpoint and NO SQLite (RISK-007/MC-007). Resolution runs OFF the egui render
//!   path: the host fills [`OutgoingLinksPanel::resolved`]/[`OutgoingLinksPanel::unresolved`] from the
//!   backend client (set `loading=true` while in flight); [`OutgoingLinksPanel::show`] renders the
//!   CACHED struct ONLY and never performs I/O (RISK-002/MC-002).
//!
//! ## What this MT proves NOW vs at E11
//!
//! The pure extraction + bucketing, the widget (`show`), the AccessKit ids, and the `on_open` seam are
//! all provable at the widget level by this MT. The live pane DOCK (`loom.outgoing_links` registered in
//! [`crate::pane_registry`]) and the host wiring of the `on_open` closure to the REAL navigation bus
//! land with the other panes at E11 (MT-069 host-mount) — exactly like the MT-015 backlinks pane.

use egui::accesskit;

use crate::rich_editor::wikilinks::parser::{self, WikilinkKind};
use crate::rich_editor::wikilinks::resolver::{self, ResolverIndex, WikilinkResolution};
use crate::theme::HsPalette;

/// The AccessKit author_id for the outgoing-links pane container.
pub const PANEL_AUTHOR_ID: &str = "outgoing.panel";

/// The AccessKit author_id for the Resolved section container.
pub const RESOLVED_SECTION_AUTHOR_ID: &str = "outgoing.section.resolved";

/// The AccessKit author_id for the Unresolved section container.
pub const UNRESOLVED_SECTION_AUTHOR_ID: &str = "outgoing.section.unresolved";

/// The exact text shown when the document has no outgoing links (RISK-006/MC-006). A literal the
/// kittest asserts; NO spinner, NO panic on this path.
pub const EMPTY_TEXT: &str = "No outgoing links";

/// The AccessKit author_id for one RESOLVED entry (`outgoing.resolved.{resolved_target_id}`), per the
/// MT contract. Keyed on the resolved target id so two links to the SAME document share one stable,
/// de-duplicated id (RISK-004/MC-004).
pub fn resolved_author_id(resolved_target_id: &str) -> String {
    format!("outgoing.resolved.{resolved_target_id}")
}

/// The AccessKit author_id for one UNRESOLVED entry (`outgoing.unresolved.{target_value}`), per the
/// MT contract. Keyed on the NORMALIZED target value (the resolver's key) so `[[Foo]]` and `[[ foo ]]`
/// — the same logical dangling target — share one id and never produce a duplicate author_id
/// (RISK-004/MC-004).
pub fn unresolved_author_id(target_value: &str) -> String {
    format!("outgoing.unresolved.{}", resolver::normalize_target(target_value))
}

/// The kind of outgoing link an entry represents. A `[[target]]` wikilink (`hsLink` atom) is
/// [`LinkKind::Wikilink`]; a `![[target]]` transclusion (`loomTransclusion` atom) is
/// [`LinkKind::Transclusion`]. The kind drives the small glyph an entry shows (link vs embed) and is
/// part of the de-duplication key, so a wikilink and a transclusion to the same target are TWO
/// distinct outgoing links (Obsidian shows them separately).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkKind {
    /// A `[[target]]` / `[[target|alias]]` wikilink (the `hsLink` inline atom).
    Wikilink,
    /// A `![[target]]` transclusion / embed (the `loomTransclusion` inline atom).
    Transclusion,
}

impl LinkKind {
    /// The small leading glyph the entry row shows: a link glyph for a wikilink, an embed glyph for a
    /// transclusion. Lets a reader (and a screenshot reviewer) tell the two kinds apart at a glance.
    pub fn glyph(self) -> &'static str {
        match self {
            LinkKind::Wikilink => "🔗",
            LinkKind::Transclusion => "📄",
        }
    }
}

/// One outgoing link extracted from the active document's `content_json`. `raw` is the original token
/// text (`prefix:value` / `prefix:value|alias`) for diagnostics; `target_value` is the resolved-against
/// target; `alias` is the explicit `|alias` display label when present; `kind` discriminates wikilink
/// vs transclusion; `resolved_target_id` is `Some(id)` once the link resolved to a live document/block,
/// else `None` (dangling).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingLink {
    /// The raw token text (e.g. `"note:My Note"` or `"My Note|alias"`), for diagnostics / titles.
    pub raw: String,
    /// The target value the resolver looks up (e.g. `"My Note"`, `"WP-KERNEL-012"`), trimmed.
    pub target_value: String,
    /// The explicit `|alias` display label, if the token carried one.
    pub alias: Option<String>,
    /// Wikilink (`[[ ]]`) vs Transclusion (`![[ ]]`).
    pub kind: LinkKind,
    /// The resolved live document/block id (navigation target), or `None` if dangling.
    pub resolved_target_id: Option<String>,
}

impl OutgoingLink {
    /// The text the entry row displays: the explicit alias when present, else the target value. Matches
    /// the backlinks/unlinked panes' label rule for triad visual consistency.
    pub fn display_text(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.target_value)
    }

    /// The de-duplication key: `(kind, normalized target_value)`. Shared with the resolver's lookup key
    /// ([`resolver::normalize_target`]) so a link is bucketed against exactly the key the resolver
    /// resolves it by (RISK-003/MC-003). A wikilink and a transclusion to the same target are distinct
    /// (the kind is part of the key), matching Obsidian.
    fn dedup_key(&self) -> (LinkKind, String) {
        (self.kind, resolver::normalize_target(&self.target_value))
    }
}

/// Extract every outgoing link from a document's Tiptap `content_json` tree — a PURE, side-effect-free
/// function (no I/O, no async) so it is trivially unit-testable (AC-001). Walks the DocModel JSON
/// looking for the EXISTING inline atoms the rich editor stores:
///
/// - `{type:"hsLink", attrs:{refKind, refValue, label, resolved}}` -> a [`LinkKind::Wikilink`]. The
///   `refValue`/`label` are classified through the MT-015 parser ([`parser::classify_wikilink`]) — NO
///   local regex/parser is defined here (AC-003). The `refKind`+`refValue` reconstruct the
///   `prefix:value` token the parser classifies, so the alias/label/target semantics are byte-for-byte
///   the same the backlinks pane uses (no triad parse drift).
/// - `{type:"loomTransclusion", attrs:{refValue}}` -> a [`LinkKind::Transclusion`].
///
/// De-duplicates identical `(kind, normalized target_value)` pairs, keeping the FIRST occurrence (and
/// its alias) — Obsidian shows each distinct outgoing target once. `resolved_target_id` is left `None`
/// here; resolution is a SEPARATE step ([`bucket_links`]) that needs the backend.
pub fn extract_outgoing_links(content_json: &serde_json::Value) -> Vec<OutgoingLink> {
    let mut out: Vec<OutgoingLink> = Vec::new();
    let mut seen: std::collections::HashSet<(LinkKind, String)> = std::collections::HashSet::new();
    walk_node(content_json, &mut out, &mut seen);
    out
}

/// Recursively walk a `content_json` node, pushing each `hsLink`/`loomTransclusion` atom (deduped) as
/// an [`OutgoingLink`]. Document order is preserved (children visited in order); the dedup set keeps the
/// first occurrence of each `(kind, normalized target)` pair.
fn walk_node(
    node: &serde_json::Value,
    out: &mut Vec<OutgoingLink>,
    seen: &mut std::collections::HashSet<(LinkKind, String)>,
) {
    if let Some(link) = node_to_outgoing_link(node) {
        let key = link.dedup_key();
        if seen.insert(key) {
            out.push(link);
        }
    }
    // Recurse into the `content` array (the Tiptap children key). A node may be BOTH a link atom and
    // a container in theory; in the real schema atoms are leaves, but recursing unconditionally is
    // harmless (an atom carries no `content`) and keeps the walk total.
    if let Some(children) = node.get("content").and_then(serde_json::Value::as_array) {
        for child in children {
            walk_node(child, out, seen);
        }
    }
}

/// Map a single `content_json` node to an [`OutgoingLink`] if it is an `hsLink` or `loomTransclusion`
/// atom, else `None`. The wikilink path reconstructs the `prefix:value`/`prefix:value|alias` token from
/// the atom attrs and runs it through the MT-015 [`parser::classify_wikilink`] so the kind/alias/target
/// semantics match the rest of the triad exactly (AC-003).
fn node_to_outgoing_link(node: &serde_json::Value) -> Option<OutgoingLink> {
    let ty = node.get("type").and_then(serde_json::Value::as_str)?;
    let attrs = node.get("attrs");
    match ty {
        "hsLink" => {
            let ref_value = attrs
                .and_then(|a| a.get("refValue"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())?;
            let ref_kind = attrs
                .and_then(|a| a.get("refKind"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            // An explicit, non-empty `label` that DIFFERS from the bare ref value is an alias the user
            // typed (`[[target|alias]]`); a label equal to the value (the parser's default) is NOT an
            // alias. We classify through the MT-015 parser to stay byte-identical with the rest of the
            // triad rather than re-deriving alias rules locally.
            let label = attrs
                .and_then(|a| a.get("label"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|l| !l.is_empty());
            let parsed = parser::classify_wikilink(ref_kind, ref_value, label);
            // `target_value` is the parser's trimmed ref value; the explicit alias is the label only
            // when it is NOT the parser's default (the bare value / `prefix:value`).
            let alias = match &parsed.kind {
                WikilinkKind::Known(_) if parsed.label != parsed.ref_value => Some(parsed.label.clone()),
                WikilinkKind::Unknown(_)
                    if parsed.label != format!("{}:{}", parsed.raw_prefix, parsed.ref_value) =>
                {
                    Some(parsed.label.clone())
                }
                _ => None,
            };
            Some(OutgoingLink {
                raw: format!("{ref_kind}:{ref_value}"),
                target_value: parsed.ref_value,
                alias,
                kind: LinkKind::Wikilink,
                resolved_target_id: None,
            })
        }
        "loomTransclusion" => {
            let ref_value = attrs
                .and_then(|a| a.get("refValue"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())?;
            Some(OutgoingLink {
                raw: ref_value.to_owned(),
                target_value: ref_value.to_owned(),
                alias: None,
                kind: LinkKind::Transclusion,
                resolved_target_id: None,
            })
        }
        _ => None,
    }
}

/// Resolve + bucket extracted links into `(resolved, unresolved)` using the MT-057 resolution engine —
/// a PURE function over the pure extraction output and the [`ResolverIndex`] the host builds from the
/// backend (`GET /loom/blocks/{id}` lookups). Each link is resolved with
/// [`resolver::resolve_wikilink`]: a [`WikilinkResolution::Resolved`] stamps `resolved_target_id` and
/// goes to the resolved bucket; a [`WikilinkResolution::Unresolved`] leaves it `None` and goes to the
/// unresolved (dangling) bucket. Because resolution uses the resolver's own normalization, a link can
/// never be bucketed against a different key than the resolver used (RISK-003/MC-003).
pub fn bucket_links(
    links: Vec<OutgoingLink>,
    index: &ResolverIndex,
) -> (Vec<OutgoingLink>, Vec<OutgoingLink>) {
    let mut resolved = Vec::new();
    let mut unresolved = Vec::new();
    for mut link in links {
        match resolver::resolve_wikilink(index, &link.target_value) {
            WikilinkResolution::Resolved { document_id, .. } => {
                link.resolved_target_id = Some(document_id);
                resolved.push(link);
            }
            WikilinkResolution::Unresolved { .. } => {
                link.resolved_target_id = None;
                unresolved.push(link);
            }
        }
    }
    (resolved, unresolved)
}

/// The navigation target an outgoing-link click hands to the shell navigation seam (the MT-030
/// navigation bus, reached over the WP-011 `command_registry` + `event_bus`). This is a small value
/// type carried by the `on_open: FnMut(NavTarget)` callback so [`OutgoingLinksPanel`] stays decoupled
/// from the concrete bus type — the host wires the closure to the real bus at E11 (RISK-008/MC-008,
/// AC-007). No new navigation CHANNEL is created here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavTarget {
    /// Open a resolved live block/document by its id (a resolved entry click).
    Block {
        /// The resolved target id (`OutgoingLink::resolved_target_id`).
        id: String,
    },
    /// A dangling/unresolved target the bus decides a fallback for (e.g. a future create-note flow). An
    /// unresolved entry click still fires this (the link is NEVER silently dropped — RISK-005/MC-005).
    Unresolved {
        /// The (trimmed) target value the unresolved link pointed at.
        value: String,
    },
}

impl NavTarget {
    /// Build a [`NavTarget::Block`] for a resolved entry click.
    pub fn block(id: impl Into<String>) -> Self {
        NavTarget::Block { id: id.into() }
    }

    /// Build a [`NavTarget::Unresolved`] for a dangling entry click.
    pub fn unresolved(value: impl Into<String>) -> Self {
        NavTarget::Unresolved { value: value.into() }
    }
}

/// The Outgoing Links pane widget for the currently active rich document / loom block.
///
/// State is filled OFF the render path by the host: `active_document_id`/`active_block_id` identify the
/// document; `resolved`/`unresolved` are the bucketed [`OutgoingLink`]s the host computed from
/// [`extract_outgoing_links`] + [`bucket_links`] after its backend lookups; `loading` is `true` while a
/// fetch is in flight; `error` carries a typed fetch error. [`Self::show`] renders this CACHED state
/// ONLY and performs NO I/O (RISK-002/MC-002).
#[derive(Debug, Clone, Default)]
pub struct OutgoingLinksPanel {
    /// The active document id (the source of outgoing links), if any.
    pub active_document_id: Option<String>,
    /// The active loom block id, if the surface is a block rather than a document.
    pub active_block_id: Option<String>,
    /// The resolved outgoing links (each has `resolved_target_id = Some(..)`).
    pub resolved: Vec<OutgoingLink>,
    /// The unresolved (dangling) outgoing links (each has `resolved_target_id = None`).
    pub unresolved: Vec<OutgoingLink>,
    /// True while the host is fetching/resolving off the render path.
    pub loading: bool,
    /// A typed fetch/resolution error to surface inline, if any.
    pub error: Option<String>,
}

impl OutgoingLinksPanel {
    /// Build an empty pane.
    pub fn new() -> Self {
        Self::default()
    }

    /// Total outgoing-link count across both buckets.
    pub fn total(&self) -> usize {
        self.resolved.len() + self.unresolved.len()
    }

    /// Render the pane into `ui`. `on_open` is the navigation seam the shell wires to the MT-030 nav
    /// bus: a resolved entry click fires `on_open(NavTarget::block(id))`; an unresolved entry click
    /// fires `on_open(NavTarget::unresolved(value))` (the link is never dropped — RISK-005/MC-005).
    ///
    /// Renders the CACHED struct state ONLY — NO network/disk I/O happens here (RISK-002/MC-002). If
    /// `loading`, shows a spinner; if `error`, shows the error text; if BOTH buckets are empty (and not
    /// loading / no error), shows the literal [`EMPTY_TEXT`] with NO spinner and NO panic
    /// (RISK-006/MC-006).
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette, on_open: &mut dyn FnMut(NavTarget)) {
        let header = format!("Outgoing Links ({})", self.total());
        ui.label(egui::RichText::new(header).strong().color(palette.text));

        if self.loading {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new());
                ui.colored_label(palette.text_subtle, "Resolving outgoing links…");
            });
            return;
        }

        if let Some(err) = &self.error {
            ui.colored_label(palette.error_text, format!("Outgoing links failed: {err}"));
            return;
        }

        if self.resolved.is_empty() && self.unresolved.is_empty() {
            // Empty document path: the exact literal, no spinner, no panic (RISK-006/MC-006).
            ui.colored_label(palette.text_subtle, EMPTY_TEXT);
            return;
        }

        // ── Resolved section ─────────────────────────────────────────────────────────────────────
        let resolved_header = format!("Resolved ({})", self.resolved.len());
        let resolved_resp = egui::CollapsingHeader::new(resolved_header)
            .id_salt("outgoing-links-resolved")
            .default_open(true)
            .show(ui, |ui| {
                for link in &self.resolved {
                    // A resolved entry always has Some(id) here (bucket_links guarantees it); fall back
                    // to the target value defensively so a malformed cache can never panic.
                    let id = link
                        .resolved_target_id
                        .clone()
                        .unwrap_or_else(|| link.target_value.clone());
                    let row_text = format!("{} {}", link.kind.glyph(), link.display_text());
                    let row = ui.add(
                        egui::Label::new(egui::RichText::new(row_text).color(palette.accent))
                            .sense(egui::Sense::click()),
                    );
                    emit_node_author(ui.ctx(), row.id, accesskit::Role::Link, &resolved_author_id(&id));
                    if row.clicked() {
                        on_open(NavTarget::block(id));
                    }
                }
            });
        emit_node_author(
            ui.ctx(),
            resolved_resp.header_response.id,
            accesskit::Role::Group,
            RESOLVED_SECTION_AUTHOR_ID,
        );

        // ── Unresolved (dangling) section ────────────────────────────────────────────────────────
        let unresolved_header = format!("Unresolved ({})", self.unresolved.len());
        let unresolved_resp = egui::CollapsingHeader::new(unresolved_header)
            .id_salt("outgoing-links-unresolved")
            .default_open(true)
            .show(ui, |ui| {
                for link in &self.unresolved {
                    // Dangling entries render DIMMED (text_subtle) but remain present + addressable +
                    // clickable (never dropped). A click still fires on_open so the bus can decide a
                    // fallback (RISK-005/MC-005).
                    let row_text = format!("{} {}", link.kind.glyph(), link.display_text());
                    let row = ui.add(
                        egui::Label::new(egui::RichText::new(row_text).color(palette.text_subtle))
                            .sense(egui::Sense::click()),
                    );
                    emit_node_author(
                        ui.ctx(),
                        row.id,
                        accesskit::Role::Link,
                        &unresolved_author_id(&link.target_value),
                    );
                    if row.clicked() {
                        on_open(NavTarget::unresolved(link.target_value.clone()));
                    }
                }
            });
        emit_node_author(
            ui.ctx(),
            unresolved_resp.header_response.id,
            accesskit::Role::Group,
            UNRESOLVED_SECTION_AUTHOR_ID,
        );
    }
}

/// Emit a stable AccessKit author_id (+ role) onto an already-rendered node — the SAME helper shape the
/// backlinks pane (`backlinks_panel.rs`) and transclusion dispatch use, so the live-emission path is
/// shared, not forked. A Button keeps egui's own role; any other role is set explicitly.
fn emit_node_author(ctx: &egui::Context, id: egui::Id, role: accesskit::Role, author_id: &str) {
    let role_for_closure = role;
    let author = author_id.to_owned();
    ctx.accesskit_node_builder(id, move |node| {
        if !matches!(role_for_closure, accesskit::Role::Button) {
            node.set_role(role_for_closure);
        }
        node.set_author_id(author);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A `content_json` doc with one valid (`[[note:ExistingNote]]`) + one dangling
    /// (`[[note:DoesNotExist]]`) wikilink, plus one transclusion, in a single paragraph.
    fn doc_with_links() -> serde_json::Value {
        json!({
            "type": "doc",
            "content": [
                {
                    "type": "paragraph",
                    "content": [
                        { "type": "text", "text": "See " },
                        { "type": "hsLink", "attrs": { "refKind": "note", "refValue": "ExistingNote", "label": "", "resolved": true } },
                        { "type": "text", "text": " and " },
                        { "type": "hsLink", "attrs": { "refKind": "note", "refValue": "DoesNotExist", "label": "", "resolved": true } },
                        { "type": "loomTransclusion", "attrs": { "refValue": "BLK-embed-1" } }
                    ]
                }
            ]
        })
    }

    #[test]
    fn extracts_wikilinks_and_transclusions_in_order() {
        // AC-001 (extraction half) + AC-002: both `hsLink` (Wikilink) and `loomTransclusion`
        // (Transclusion) atoms are extracted with the right LinkKind.
        let links = extract_outgoing_links(&doc_with_links());
        assert_eq!(links.len(), 3, "two wikilinks + one transclusion extracted");
        assert_eq!(links[0].target_value, "ExistingNote");
        assert_eq!(links[0].kind, LinkKind::Wikilink);
        assert_eq!(links[1].target_value, "DoesNotExist");
        assert_eq!(links[1].kind, LinkKind::Wikilink);
        assert_eq!(links[2].target_value, "BLK-embed-1");
        assert_eq!(links[2].kind, LinkKind::Transclusion);
    }

    #[test]
    fn alias_is_preserved_from_explicit_label() {
        // AC-002: a `[[target|alias]]` (an hsLink whose label DIFFERS from the value) preserves the
        // alias; a label equal to the value (the parser default) is NOT an alias.
        let doc = json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    { "type": "hsLink", "attrs": { "refKind": "note", "refValue": "RealTarget", "label": "My Alias", "resolved": true } },
                    { "type": "hsLink", "attrs": { "refKind": "note", "refValue": "Plain", "label": "Plain", "resolved": true } }
                ]
            }]
        });
        let links = extract_outgoing_links(&doc);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].alias.as_deref(), Some("My Alias"), "explicit alias preserved");
        assert_eq!(links[0].display_text(), "My Alias");
        assert_eq!(links[1].alias, None, "a label equal to the value is not an alias");
        assert_eq!(links[1].display_text(), "Plain");
    }

    #[test]
    fn deduplicates_identical_kind_and_target_keeping_first_alias() {
        // De-dup on (kind, normalized target): two wikilinks to the same target (one aliased) collapse
        // to ONE entry keeping the first alias. A transclusion to the same target is DISTINCT (kind
        // differs), matching Obsidian.
        let doc = json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    { "type": "hsLink", "attrs": { "refKind": "note", "refValue": "Dup", "label": "First", "resolved": true } },
                    { "type": "hsLink", "attrs": { "refKind": "note", "refValue": " dup ", "label": "Second", "resolved": true } },
                    { "type": "loomTransclusion", "attrs": { "refValue": "Dup" } }
                ]
            }]
        });
        let links = extract_outgoing_links(&doc);
        assert_eq!(links.len(), 2, "the two wikilinks dedup to one; the transclusion is distinct");
        assert_eq!(links[0].alias.as_deref(), Some("First"), "first alias kept on dedup");
        assert_eq!(links[0].kind, LinkKind::Wikilink);
        assert_eq!(links[1].kind, LinkKind::Transclusion);
    }

    #[test]
    fn empty_or_malformed_nodes_yield_no_links_and_no_panic() {
        // RISK-006: an empty doc yields zero links; an hsLink with an empty/missing refValue is skipped
        // (never a panic, never an empty-target entry).
        assert!(extract_outgoing_links(&json!({"type":"doc","content":[]})).is_empty());
        let malformed = json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    { "type": "hsLink", "attrs": { "refKind": "note", "refValue": "   ", "label": "", "resolved": true } },
                    { "type": "loomTransclusion", "attrs": {} }
                ]
            }]
        });
        assert!(extract_outgoing_links(&malformed).is_empty(), "blank/missing targets are skipped");
    }

    /// A resolver index that knows `ExistingNote` (and the transclusion block) but NOT `DoesNotExist`.
    fn seeded_index() -> ResolverIndex {
        let mut idx = ResolverIndex::new();
        idx.add_document("DOC-existing", "ExistingNote");
        // The transclusion target resolves to a live block id (the loom-block lookup result).
        idx.add_document("BLK-embed-1", "BLK-embed-1");
        idx
    }

    #[test]
    fn bucketing_splits_resolved_and_unresolved_exactly_one_each() {
        // AC-001: bucket the extracted links from a content_json with one valid + one dangling wikilink.
        // Exactly one wikilink resolves (ExistingNote -> DOC-existing) and one is dangling (DoesNotExist).
        let links = extract_outgoing_links(&doc_with_links());
        let (resolved, unresolved) = bucket_links(links, &seeded_index());
        // ExistingNote + the transclusion resolve; DoesNotExist does not.
        let resolved_wikilinks: Vec<_> =
            resolved.iter().filter(|l| l.kind == LinkKind::Wikilink).collect();
        let unresolved_wikilinks: Vec<_> =
            unresolved.iter().filter(|l| l.kind == LinkKind::Wikilink).collect();
        assert_eq!(resolved_wikilinks.len(), 1, "exactly one resolved wikilink (ExistingNote)");
        assert_eq!(unresolved_wikilinks.len(), 1, "exactly one unresolved wikilink (DoesNotExist)");
        assert_eq!(resolved_wikilinks[0].target_value, "ExistingNote");
        assert_eq!(
            resolved_wikilinks[0].resolved_target_id.as_deref(),
            Some("DOC-existing"),
            "resolved entry carries the live document id (the nav target)"
        );
        assert_eq!(unresolved_wikilinks[0].resolved_target_id, None, "dangling entry has no id");
    }

    #[test]
    fn author_ids_match_contract_shape_and_dedup() {
        // RISK-004/MC-004: resolved id keys on the resolved target id; unresolved id keys on the
        // NORMALIZED value, so `[[Foo]]` and `[[ foo ]]` share one unresolved id (no duplicate author_id).
        assert_eq!(PANEL_AUTHOR_ID, "outgoing.panel");
        assert_eq!(RESOLVED_SECTION_AUTHOR_ID, "outgoing.section.resolved");
        assert_eq!(UNRESOLVED_SECTION_AUTHOR_ID, "outgoing.section.unresolved");
        assert_eq!(resolved_author_id("DOC-7"), "outgoing.resolved.DOC-7");
        assert_eq!(unresolved_author_id("Foo Bar"), "outgoing.unresolved.foo bar");
        assert_eq!(
            unresolved_author_id("  foo   bar "),
            unresolved_author_id("Foo Bar"),
            "the unresolved author_id is normalized so the same logical target dedups to one id"
        );
    }

    #[test]
    fn nav_target_constructors() {
        assert_eq!(NavTarget::block("X"), NavTarget::Block { id: "X".to_owned() });
        assert_eq!(
            NavTarget::unresolved("Y"),
            NavTarget::Unresolved { value: "Y".to_owned() }
        );
    }

    /// The PRODUCTION source of THIS module (everything ABOVE the `#[cfg(test)]` block), for the grep
    /// gates below. We slice off the test module so the gates scan only shipped code and are not tripped
    /// by the forbidden-literal strings that necessarily appear inside the gate assertions themselves.
    fn production_source() -> &'static str {
        const FULL: &str = include_str!("outgoing_links_panel.rs");
        // Split at the test-module marker; the production code is everything before it.
        FULL.split("#[cfg(te").next().unwrap_or(FULL)
    }

    #[test]
    fn ac003_reuses_mt015_parser_and_defines_no_local_parser() {
        // AC-003 / RISK-001 / MC-001: the module IMPORTS the MT-015 parser and defines NO local
        // regex/parser (a second parser would drift the triad). Grep the source.
        assert!(
            production_source().contains("use crate::rich_editor::wikilinks::parser"),
            "AC-003: the MT-015 wikilinks parser must be imported"
        );
        assert!(
            production_source().contains("parser::classify_wikilink"),
            "AC-003: extraction must classify through the MT-015 parser entrypoint"
        );
        // No local regex/parser: forbid `Regex::new` and a `WIKILINK_REGEX`-style local pattern here.
        assert!(
            !production_source().contains("Regex::new") && !production_source().contains("regex::Regex"),
            "AC-003 / MC-001: no local regex parser may be defined in outgoing_links_panel.rs"
        );
    }

    #[test]
    fn ac007_navigation_uses_fnmut_navtarget_seam_no_new_channel() {
        // AC-007 / RISK-008 / MC-008: navigation flows through the `&mut dyn FnMut(NavTarget)` seam (the
        // MT-030 nav bus the host wires); NO new navigation channel/sender/registry is created here.
        assert!(
            production_source().contains("on_open: &mut dyn FnMut(NavTarget)"),
            "AC-007: the navigation seam is a &mut dyn FnMut(NavTarget) closure"
        );
        // The widget must not instantiate its own event channel / sender (a new nav channel).
        assert!(
            !production_source().contains("std::sync::mpsc") && !production_source().contains("channel("),
            "MC-008: no new navigation channel may be created in the panel"
        );
    }

    #[test]
    fn ac008_no_backend_rewrite_and_no_sqlite() {
        // AC-008 / RISK-007 / MC-007: read-only via the bound GETs; the module adds NO HTTP client, NO
        // backend write, and NO SQLite. (Resolution is the host's job off the render path; this module
        // is pure extraction + bucketing + the widget.)
        for forbidden in ["sqlite", "rusqlite", "reqwest::Client", "POST ", "PUT ", "DELETE "] {
            assert!(
                !production_source().contains(forbidden),
                "MC-007: outgoing_links_panel.rs must not contain '{forbidden}' (read-only, no SQLite, no backend write)"
            );
        }
    }

    #[test]
    fn pane_is_registered_under_loom_outgoing_links() {
        // PT-005 / AC-007 (pane half): the pane registry exposes the stable `loom.outgoing_links` id +
        // a registration helper. Prove the id constant value here (the registry test proves the insert).
        assert_eq!(
            crate::pane_registry::OUTGOING_LINKS_PANE_ID,
            "loom.outgoing_links",
            "PT-005: the pane registers under the stable id 'loom.outgoing_links'"
        );
    }
}
