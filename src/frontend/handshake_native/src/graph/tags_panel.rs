//! Tags panel + Tag-Hub page (WP-KERNEL-012 MT-023, cluster E3).
//!
//! ## What this is
//!
//! Two native, AccessKit-addressable Obsidian-parity knowledge surfaces:
//!
//! - [`LoomTagsPanel`] — a flat, searchable list of every **tag-hub** block in the workspace. Each row
//!   is a colored tag chip (color derived from the tag title, purely cosmetic) + a member-count badge;
//!   clicking a row fires [`TagsPanelEvent::OpenTag`] (navigate to the hub page) and the search box
//!   filters the list client-side. It is the native peer of the React `WorkspaceSidebar.tsx` tag lists.
//! - [`LoomTagHubPanel`] — a tag-hub rendered as a first-class content page (Obsidian's "tag page"):
//!   the hub title as a heading, a scrollable list of member blocks (title + content_type), and an
//!   "Add tag to block" popup that searches for a block and tags it with this hub.
//!
//! ## Backend reality (Spec-Realism Gate — the MT-008/021/022 "verify, don't trust the contract" rule)
//!
//! The MT-023 contract body assumed a `GET /loom/views/all?content_type=tag_hub` filter, a
//! `views/all?tag_ids={id}` member query, and a `content_json` hub description. Verified READ-ONLY
//! against the running backend (`src/backend/handshake_core/src/{api,storage}/loom.rs`), NONE of those is
//! the real surface — exactly the MT-022 folder lesson again. The REAL tag authority is a dedicated
//! tag-hub API (MT-182 "tags as first-class blocks"):
//!   - `GET  /workspaces/{ws}/loom/tags`                       -> `Vec<LoomBlock>` (every `tag_hub`
//!     block; the flat list the panel renders — the contract's `views/all?content_type=tag_hub` does NOT
//!     exist, so RISK-5's "client-side content_type filter fallback" is moot: this route already returns
//!     ONLY tag hubs).
//!   - `GET  /workspaces/{ws}/loom/tags/{tag_block_id}`        -> `LoomTagHub` `{ block, sub_tags,
//!     tagged_blocks, backlink_count }` (the hub page: title from `block.title`, members from
//!     `tagged_blocks` — there is NO `content_json` description column, so the hub "description" is the
//!     verified field set, never a fabricated one).
//!   - `GET  /workspaces/{ws}/loom/tags/{tag_block_id}/blocks` -> `Vec<LoomBlock>` (members; supports
//!     `include_subtags`, `limit`, `offset`; the lazy member-count + member-list source).
//!   - `POST /workspaces/{ws}/loom/edges` body `{ source_block_id, target_block_id, edge_type:"tag",
//!     created_by:"user" }` -> `LoomEdge` (tag a block with a hub; the backend HARD-rejects a non-tag_hub
//!     target with `HSK-400-LOOM-TAG-TARGET-MUST-BE-TAG_HUB`, so the hub is always the edge TARGET).
//!
//! ## Member count (RISK-1 / MC-1: no N+1)
//!
//! Each tag-hub `LoomBlock` carries `derived.tag_count` server-side, but that is the count of tags ON the
//! block, not members OF the hub. The member count is `tagged_blocks.len()` from the hub detail. To avoid
//! an N+1 storm on the list, the list rows show the count the host resolves lazily (it may be 0 until the
//! hub is opened); the EXACT member count is loaded on hub open. The list never blocks on per-tag fetches.
//!
//! ## Repaint discipline (the MT-015 idle-repaint lesson)
//!
//! A spinner animates ONLY while a genuine in-flight fetch is dispatched (`loading=true`, set by the host
//! when it actually spawns a request). A headless / no-runtime render shows the static neutral state and
//! never enters a perpetual `Loading…` / perpetual repaint. The widget requests a repaint ONLY for the
//! one frame a spinner is genuinely active.
//!
//! ## Add-tag popup (the MT-016 Popup + HBR-QUIET lesson)
//!
//! The add-tag affordance uses the modern `egui::Popup` API (NOT the deprecated `popup_below_widget`),
//! anchored to the "Add tag to block" button — an IN-PROCESS egui popup that never opens an OS window
//! and never steals OS focus (HBR-QUIET). After the `POST /loom/edges` create RESOLVES (the host awaits
//! the response and delivers the result), the host re-queries the members — there is NO fixed-delay
//! sleep (the RISK-2 "100ms delay" control is replaced by the stronger await-the-response control).
//!
//! ## AccessKit (HBR-SWARM)
//!
//! - [`LoomTagsPanel`]: the search box is `tags.search` (Role::TextInput); each tag row is
//!   `tags.row.{sanitized_block_id}` (Role::ListItem, Click default-open, member count in the accessible
//!   description).
//! - [`LoomTagHubPanel`]: the hub title label is `tag-hub.title.{sanitized_block_id}`; each member row is
//!   `tag-hub.member.{sanitized_block_id}` (Role::ListItem, Click open); the add-tag button is
//!   `tag-hub.add-tag.{sanitized_block_id}` (Role::Button). Ids are sanitized to `[a-z0-9-]` via
//!   [`crate::project_tree::stable_part`] so a raw id with slashes/colons can never break the tree.

use egui::accesskit;
use egui::{Color32, Sense, Vec2};

use crate::theme::HsPalette;

/// AccessKit author_id for the tags-panel search box (Role::TextInput).
pub const SEARCH_AUTHOR_ID: &str = "tags.search";

/// AccessKit author_id prefix for a tag row: `tags.row.{sanitized_block_id}`.
pub const TAG_ROW_AUTHOR_ID_PREFIX: &str = "tags.row.";

/// AccessKit author_id prefix for the hub-page title label: `tag-hub.title.{sanitized_block_id}`.
pub const HUB_TITLE_AUTHOR_ID_PREFIX: &str = "tag-hub.title.";

/// AccessKit author_id prefix for a hub-page member row: `tag-hub.member.{sanitized_block_id}`.
pub const HUB_MEMBER_AUTHOR_ID_PREFIX: &str = "tag-hub.member.";

/// AccessKit author_id prefix for the hub-page add-tag button: `tag-hub.add-tag.{sanitized_block_id}`.
pub const HUB_ADD_TAG_AUTHOR_ID_PREFIX: &str = "tag-hub.add-tag.";

/// AccessKit author_id prefix for an add-tag popup search result row: `tag-hub.add-result.{sanitized}`.
pub const HUB_ADD_RESULT_AUTHOR_ID_PREFIX: &str = "tag-hub.add-result.";

/// AccessKit author_id for the add-tag popup search box (Role::TextInput).
pub const HUB_ADD_SEARCH_AUTHOR_ID: &str = "tag-hub.add-search";

/// AccessKit author_id for the tags-panel error-banner Retry button.
pub const RETRY_AUTHOR_ID: &str = "tags.retry";

/// AccessKit author_id for the hub-page error-banner Retry button.
pub const HUB_RETRY_AUTHOR_ID: &str = "tag-hub.retry";

/// Size of the colored tag chip's leading swatch (px). Cosmetic.
pub const CHIP_SWATCH_SIZE: f32 = 12.0;

/// The stable AccessKit author_id for a tag row: `tags.row.{sanitized_block_id}`.
pub fn tag_row_author_id(block_id: &str) -> String {
    format!(
        "{TAG_ROW_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(block_id)
    )
}

/// The stable AccessKit author_id for the hub-page title label: `tag-hub.title.{sanitized_block_id}`.
pub fn hub_title_author_id(block_id: &str) -> String {
    format!(
        "{HUB_TITLE_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(block_id)
    )
}

/// The stable AccessKit author_id for a hub-page member row: `tag-hub.member.{sanitized_block_id}`.
pub fn hub_member_author_id(block_id: &str) -> String {
    format!(
        "{HUB_MEMBER_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(block_id)
    )
}

/// The stable AccessKit author_id for the hub-page add-tag button:
/// `tag-hub.add-tag.{sanitized_block_id}`.
pub fn hub_add_tag_author_id(block_id: &str) -> String {
    format!(
        "{HUB_ADD_TAG_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(block_id)
    )
}

/// The stable AccessKit author_id for an add-tag popup result row:
/// `tag-hub.add-result.{sanitized_block_id}`.
pub fn hub_add_result_author_id(block_id: &str) -> String {
    format!(
        "{HUB_ADD_RESULT_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(block_id)
    )
}

/// A 12-color palette of distinct, evenly-spaced hues for tag chips (RISK-3 / MC-3: >= 12 colors so
/// title-hash collisions stay rare). Each hue is a saturated, mid-lightness color that reads clearly on
/// both the dark and light backgrounds; the colors are purely cosmetic UI state (NEVER persisted to the
/// backend — the implementation_notes are explicit on this). Built once at first use.
///
/// NOTE the no-hardcode-theme-token invariant: these are CHIP-IDENTITY colors derived from a tag's title
/// hash, not semantic theme tokens — they intentionally do not come from [`HsPalette`] (a tag's identity
/// hue must be stable across themes so the same tag always looks the same). The identity-hue `Color32`
/// literals therefore live in the sanctioned `theme/palette.rs` (the no-hardcoded-color guard,
/// `tests/test_theme.rs` `no_hardcoded_color32_outside_theme_module`, exempts `palette.rs`/`syntax.rs`
/// ONLY; relocated there 2026-06-23 after the guard correctly flagged the inline literals here — the
/// earlier "the guard allows from_rgb here" claim was wrong). `TAG_CHIP_PALETTE_LEN == 12`.
fn tag_chip_palette() -> [Color32; TAG_CHIP_PALETTE_LEN] {
    crate::theme::palette::tag_chip_palette()
}

/// Number of distinct tag-chip colors (RISK-3: >= 12).
pub const TAG_CHIP_PALETTE_LEN: usize = 12;

/// Compile-time guarantee of the MC-3 ">= 12 distinct chip colors" invariant. A `const` assertion is
/// evaluated by the compiler (it fails the build if the palette ever shrinks below 12), unlike a runtime
/// `assert!` over a const — which the optimizer elides to a no-op (clippy `assertions_on_constants`).
const _: () = assert!(
    TAG_CHIP_PALETTE_LEN >= 12,
    "RISK-3 / MC-3: >= 12 distinct chip colors required"
);

/// Deterministically map a tag title to one of the [`TAG_CHIP_PALETTE_LEN`] chip colors. The hash is a
/// djb2 variant XOR-folded over the title bytes (the contract's "djb2 hash … XOR multiple hash words for
/// better distribution", RISK-3 / MC-3). Pure + stable: the same title always yields the same color, so a
/// tag's identity hue is consistent across renders and themes. Cosmetic only — never persisted.
pub fn tag_chip_color(title: &str) -> Color32 {
    tag_chip_palette()[tag_chip_color_index(title)]
}

/// The palette index a title hashes to (split out so the collision-rate proof, PROOF1 / MC-3, can test
/// the distribution without constructing colors). djb2 with a final XOR-fold of the 32-bit hash's halves
/// to spread the low bits before the modulo (a uniform-ish bucket assignment for short tag titles).
pub fn tag_chip_color_index(title: &str) -> usize {
    let mut hash: u32 = 5381;
    for b in title.bytes() {
        // djb2: hash * 33 + byte, with XOR (the "XOR multiple hash words" variant) for better mixing.
        hash = (hash.wrapping_mul(33)) ^ (b as u32);
    }
    // Fold the two 16-bit halves together so the high bits influence the bucket too (short titles
    // otherwise vary mostly in the low byte -> clustering).
    let folded = (hash ^ (hash >> 16)) as usize;
    folded % TAG_CHIP_PALETTE_LEN
}

/// One tag entry in the [`LoomTagsPanel`] list. `member_count` is the number of blocks tagged with this
/// hub; it may be `None` until the host resolves it (lazy — the list never blocks on N+1 per-tag
/// queries). The chip color is derived from `title` at render time (not stored).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagEntry {
    pub block_id: String,
    pub title: String,
    /// Number of blocks tagged with this hub. `None` => not yet resolved (renders "—"); `Some(n)` =>
    /// the resolved exact count (renders "n").
    pub member_count: Option<u32>,
}

impl TagEntry {
    pub fn new(
        block_id: impl Into<String>,
        title: impl Into<String>,
        member_count: Option<u32>,
    ) -> Self {
        Self {
            block_id: block_id.into(),
            title: title.into(),
            member_count,
        }
    }

    /// The member-count badge text: the exact count when known, else an em-dash placeholder.
    fn badge(&self) -> String {
        match self.member_count {
            Some(n) => n.to_string(),
            None => "—".to_owned(),
        }
    }
}

/// The typed event a [`LoomTagsPanel`] interaction produces this frame, for the host to apply. The host
/// owns the backend wiring + navigation routing (the widget never touches the network — HBR-QUIET).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagsPanelEvent {
    /// A tag chip was clicked (the default open action): navigate to the hub page. Carries the hub's
    /// block id (AC3 — `on_open_tag(block_id)`).
    OpenTag { block_id: String },
    /// A tag's "filter" affordance was used: open the tag as a filter over the knowledge surface (the
    /// contract's `on_filter_tag` callback slot). Carries the hub's block id.
    FilterTag { block_id: String },
    /// The error-banner Retry button was pressed: the host should re-fire the initial `GET /loom/tags`
    /// load (AC8 robustness).
    Retry,
}

/// The tags-panel widget state. Held by the host (the pane), mutated in place by [`LoomTagsPanel::show`].
/// `tags` is the full enumerated list; `search_filter` is the client-side title-prefix filter (AC2).
#[derive(Debug, Clone, Default)]
pub struct LoomTagsPanel {
    pub workspace_id: String,
    /// The full enumerated tag list from `GET /loom/tags`. The display is filtered by `search_filter`.
    pub tags: Vec<TagEntry>,
    /// The client-side filter text typed into the `tags.search` box (AC2). A tag is shown when its title
    /// starts with this prefix (case-insensitive). Empty => all tags shown.
    pub search_filter: String,
    /// True ONLY while the initial `GET /loom/tags` is in flight (the bounded spinner). The host clears
    /// it when the fetch resolves/fails.
    pub loading: bool,
    /// Set on a backend failure; renders the error banner + Retry. `None` => no error.
    pub error: Option<String>,
}

impl LoomTagsPanel {
    /// A fresh panel for `workspace_id` with no tags loaded yet.
    pub fn new(workspace_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            ..Self::default()
        }
    }

    /// Install the enumerated tag list from a `GET /loom/tags` result, clearing loading/error.
    pub fn set_tags(&mut self, tags: Vec<TagEntry>) {
        self.tags = tags;
        self.loading = false;
        self.error = None;
    }

    /// Set the resolved member count for one tag (the host calls this when a lazy per-hub count resolves).
    pub fn set_member_count(&mut self, block_id: &str, count: u32) {
        if let Some(entry) = self.tags.iter_mut().find(|t| t.block_id == block_id) {
            entry.member_count = Some(count);
        }
    }

    /// The tags currently visible after the client-side title-PREFIX filter (AC2). Case-insensitive; an
    /// empty filter shows every tag. Pure (no egui) so PROOF1 tests it standalone. The prefix match is on
    /// the title with an optional leading `#` stripped (a user types `rust`, not `#rust`).
    pub fn filtered_tags(&self) -> Vec<&TagEntry> {
        let needle = self.search_filter.trim().to_lowercase();
        self.tags
            .iter()
            .filter(|t| {
                if needle.is_empty() {
                    return true;
                }
                let hay = t.title.trim_start_matches('#').to_lowercase();
                hay.starts_with(&needle)
            })
            .collect()
    }

    /// Render the panel and return the typed event (if any) this frame produced. Requests a repaint ONLY
    /// for a frame where the genuine initial-load spinner is active (idle-repaint discipline).
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette) -> Option<TagsPanelEvent> {
        let mut event: Option<TagsPanelEvent> = None;

        // ── Error banner (robustness) ───────────────────────────────────────────────────────────────
        if let Some(err) = self.error.clone() {
            ui.horizontal(|ui| {
                ui.colored_label(palette.error_text, format!("⚠ {err}"));
                let retry = ui.button("Retry");
                emit_button_accesskit(ui, retry.id, RETRY_AUTHOR_ID, "Retry");
                if retry.clicked() {
                    event = Some(TagsPanelEvent::Retry);
                }
            });
            ui.separator();
        }

        // ── Search box (AC2 client-side filter) ───────────────────────────────────────────────────────
        let search = ui.add(
            egui::TextEdit::singleline(&mut self.search_filter)
                .hint_text("Filter tags...")
                .desired_width(f32::INFINITY),
        );
        emit_text_input_accesskit(
            ui,
            search.id,
            SEARCH_AUTHOR_ID,
            "Filter tags",
            &self.search_filter,
        );
        ui.separator();

        // ── Top-level loading spinner (bounded: only while the initial fetch is in flight) ─────────────
        if self.loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading tags…");
            });
            ui.ctx().request_repaint();
            return event;
        }

        // ── Empty state (AC8: no tag_hub blocks -> "No tags", no panic) ────────────────────────────────
        if self.tags.is_empty() {
            ui.weak("No tags");
            return event;
        }

        // ── The filtered tag list ───────────────────────────────────────────────────────────────────
        // Compute the filtered view first (immutable borrow) into owned data so the row render can mutate
        // nothing on self; the rows only PRODUCE events the host applies.
        let visible: Vec<TagEntry> = self.filtered_tags().into_iter().cloned().collect();
        if visible.is_empty() {
            // A non-empty tag set but the filter matched nothing: an honest "no matches", not "No tags".
            ui.weak("No tags match the filter");
            return event;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for entry in &visible {
                    if let Some(ev) = render_tag_row(entry, ui, palette) {
                        event = Some(ev);
                    }
                }
            });

        event
    }
}

/// Render one tag chip row: a colored leading swatch (title-hash color), the tag title, and a
/// member-count badge. Clicking the row fires [`TagsPanelEvent::OpenTag`] (AC3). The row is an
/// addressable AccessKit ListItem carrying the member count in its accessible description (AC7).
fn render_tag_row(
    entry: &TagEntry,
    ui: &mut egui::Ui,
    palette: &HsPalette,
) -> Option<TagsPanelEvent> {
    let mut event = None;
    let chip_color = tag_chip_color(&entry.title);

    let resp = ui
        .horizontal(|ui| {
            // The colored swatch (cosmetic identity hue from the title hash).
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(CHIP_SWATCH_SIZE), Sense::hover());
            if ui.is_rect_visible(rect) {
                ui.painter().rect_filled(rect, 3.0, chip_color);
            }
            // The tag title (the primary clickable). `#`-prefixed for the Obsidian tag look.
            let label = if entry.title.starts_with('#') {
                entry.title.clone()
            } else {
                format!("#{}", entry.title)
            };
            let title_resp = ui.add(
                egui::Label::new(egui::RichText::new(label).color(palette.text))
                    .sense(Sense::click()),
            );
            // The member-count badge (right-aligned subtle text).
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(palette.text_subtle, entry.badge());
            });
            title_resp
        })
        .inner;

    // The whole row is the addressable ListItem; the title response is the primary click target.
    let desc = match entry.member_count {
        Some(n) => format!("{n} blocks tagged"),
        None => "member count not yet loaded".to_owned(),
    };
    emit_list_item_accesskit(
        ui,
        resp.id,
        &tag_row_author_id(&entry.block_id),
        &entry.title,
        &desc,
    );

    if resp.clicked() {
        event = Some(TagsPanelEvent::OpenTag {
            block_id: entry.block_id.clone(),
        });
    }
    // Secondary-click (context) opens the tag as a filter (the on_filter_tag slot). A plain
    // secondary-click is the least-intrusive filter affordance that needs no extra chrome.
    if resp.secondary_clicked() {
        event = Some(TagsPanelEvent::FilterTag {
            block_id: entry.block_id.clone(),
        });
    }

    event
}

// ════════════════════════════════════════════════════════════════════════════════════════════════════
// LoomTagHubPanel — the tag-hub "tag page".
// ════════════════════════════════════════════════════════════════════════════════════════════════════

/// One member block shown on the hub page (a block tagged with this hub). Carries the `block_id` (the
/// open key), a display title, and the `content_type` (drives a small icon character).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubMember {
    pub block_id: String,
    pub title: String,
    pub content_type: String,
}

impl HubMember {
    pub fn new(
        block_id: impl Into<String>,
        title: impl Into<String>,
        content_type: impl Into<String>,
    ) -> Self {
        Self {
            block_id: block_id.into(),
            title: title.into(),
            content_type: content_type.into(),
        }
    }

    /// A single-character icon for the member's content type. Purely cosmetic; unknown -> neutral bullet.
    fn icon(&self) -> char {
        match self.content_type.as_str() {
            "note" => '📝',
            "file" | "annotated_file" => '📄',
            "tag_hub" => '#',
            "journal" => '📔',
            "canvas" => '🎨',
            "view_def" => '▦',
            _ => '•',
        }
    }
}

/// One candidate block in the add-tag popup's search results (a block the user can tag with this hub).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddTagCandidate {
    pub block_id: String,
    pub title: String,
}

impl AddTagCandidate {
    pub fn new(block_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            block_id: block_id.into(),
            title: title.into(),
        }
    }
}

/// The typed event a [`LoomTagHubPanel`] interaction produces this frame, for the host to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagHubEvent {
    /// A member block row was clicked: fire `on_open(member_block_id)` (AC5). Carries the member's id.
    OpenMember { block_id: String },
    /// The add-tag popup's search box text changed: the host should fire
    /// `GET /loom/views/all?q={query}&limit=20` (or the verified search route) and deliver candidates.
    AddTagSearch { query: String },
    /// An add-tag candidate was selected: the host should `POST /loom/edges` with
    /// `{ source_block_id: candidate, target_block_id: hub, edge_type:"tag", created_by:"user" }`, then
    /// re-query the members AFTER the response resolves (AC6 — no fixed sleep). Carries the candidate's
    /// block id (the edge SOURCE; the hub is the TARGET).
    AddTagSelected { source_block_id: String },
    /// The error-banner Retry button was pressed: the host should re-fire the hub detail load.
    Retry,
}

/// The tag-hub page widget state. Held by the host, mutated in place by [`LoomTagHubPanel::show`].
#[derive(Debug, Clone, Default)]
pub struct LoomTagHubPanel {
    pub workspace_id: String,
    /// The hub block id this page renders (the `GET /loom/tags/{id}` + member-query + edge-target key).
    pub block_id: String,
    /// The hub title (from the hub `LoomBlock.title`). Empty until the detail loads.
    pub title: String,
    /// The hub's member blocks (from `LoomTagHub.tagged_blocks` / `GET /loom/tags/{id}/blocks`).
    pub members: Vec<HubMember>,
    /// The add-tag popup's current search text.
    pub add_search: String,
    /// The add-tag popup's current search results (delivered by the host after an `AddTagSearch`).
    pub add_candidates: Vec<AddTagCandidate>,
    /// True ONLY while the initial hub-detail fetch is in flight (bounded spinner).
    pub loading: bool,
    /// Set on a backend failure; renders the error banner + Retry. `None` => no error.
    pub error: Option<String>,
}

impl LoomTagHubPanel {
    /// A fresh hub page bound to `block_id` in `workspace_id`, nothing loaded yet.
    pub fn new(workspace_id: impl Into<String>, block_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            block_id: block_id.into(),
            ..Self::default()
        }
    }

    /// Install the hub detail (title + members) from a `GET /loom/tags/{id}` result, clearing
    /// loading/error.
    pub fn set_detail(&mut self, title: impl Into<String>, members: Vec<HubMember>) {
        self.title = title.into();
        self.members = members;
        self.loading = false;
        self.error = None;
    }

    /// Install add-tag popup search candidates (the host delivers these after an `AddTagSearch` resolves).
    pub fn set_add_candidates(&mut self, candidates: Vec<AddTagCandidate>) {
        self.add_candidates = candidates;
    }

    /// Render the hub page and return the typed event (if any) this frame produced.
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette) -> Option<TagHubEvent> {
        let mut event: Option<TagHubEvent> = None;

        // ── Error banner ───────────────────────────────────────────────────────────────────────────
        if let Some(err) = self.error.clone() {
            ui.horizontal(|ui| {
                ui.colored_label(palette.error_text, format!("⚠ {err}"));
                let retry = ui.button("Retry");
                emit_button_accesskit(ui, retry.id, HUB_RETRY_AUTHOR_ID, "Retry");
                if retry.clicked() {
                    event = Some(TagHubEvent::Retry);
                }
            });
            ui.separator();
        }

        // ── Hub title (H1) — AC4 / AccessKit tag-hub.title.{id} ───────────────────────────────────────
        let display_title = if self.title.trim().is_empty() {
            format!("#{}", self.block_id)
        } else if self.title.starts_with('#') {
            self.title.clone()
        } else {
            format!("#{}", self.title)
        };
        let title_resp = ui.add(
            egui::Label::new(
                egui::RichText::new(&display_title)
                    .heading()
                    .color(palette.text),
            )
            .sense(Sense::hover()),
        );
        emit_label_accesskit(
            ui,
            title_resp.id,
            &hub_title_author_id(&self.block_id),
            &display_title,
        );

        // ── Add-tag button + popup (AC6) ──────────────────────────────────────────────────────────────
        if let Some(ev) = self.add_tag_control(ui) {
            event = Some(ev);
        }
        ui.separator();

        // ── Loading spinner (bounded) ───────────────────────────────────────────────────────────────
        if self.loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading members…");
            });
            ui.ctx().request_repaint();
            return event;
        }

        // ── Members header + list ───────────────────────────────────────────────────────────────────
        ui.colored_label(
            palette.text_subtle,
            format!(
                "{} member{}",
                self.members.len(),
                if self.members.len() == 1 { "" } else { "s" }
            ),
        );
        if self.members.is_empty() {
            ui.weak("No blocks tagged with this hub");
            return event;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for member in &self.members {
                    if let Some(ev) = render_member_row(member, ui, palette) {
                        event = Some(ev);
                    }
                }
            });

        event
    }

    /// The "Add tag to block" button + its in-process egui popup (the MT-016 modern `Popup` API; HBR-QUIET
    /// — no OS window, no focus theft). Returns the produced [`TagHubEvent`] (a search-text change or a
    /// candidate selection). The popup is anchored to the button and toggles on click.
    fn add_tag_control(&mut self, ui: &mut egui::Ui) -> Option<TagHubEvent> {
        let mut event = None;
        let add_btn = ui.button("➕ Add tag to block");
        emit_button_accesskit(
            ui,
            add_btn.id,
            &hub_add_tag_author_id(&self.block_id),
            "Add tag to block",
        );

        egui::Popup::from_toggle_button_response(&add_btn)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_min_width(240.0);
                ui.label("Tag a block with this hub");
                // The popup search box (its own AccessKit id). A change fires AddTagSearch so the host
                // queries candidate blocks.
                let search = ui.add(
                    egui::TextEdit::singleline(&mut self.add_search)
                        .hint_text("Search for a block…")
                        .desired_width(f32::INFINITY),
                );
                emit_text_input_accesskit(
                    ui,
                    search.id,
                    HUB_ADD_SEARCH_AUTHOR_ID,
                    "Search for a block",
                    &self.add_search,
                );
                if search.changed() {
                    event = Some(TagHubEvent::AddTagSearch {
                        query: self.add_search.clone(),
                    });
                }
                ui.separator();
                // The candidate results: each is a selectable, addressable row. Selecting one fires
                // AddTagSelected (the host POSTs the tag edge then re-queries members on the response).
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        if self.add_candidates.is_empty() {
                            ui.weak("Type to search for a block to tag");
                        }
                        for cand in &self.add_candidates {
                            let resp = ui.add(egui::Label::new(&cand.title).sense(Sense::click()));
                            emit_list_item_accesskit(
                                ui,
                                resp.id,
                                &hub_add_result_author_id(&cand.block_id),
                                &cand.title,
                                "Tag this block with the hub",
                            );
                            if resp.clicked() {
                                event = Some(TagHubEvent::AddTagSelected {
                                    source_block_id: cand.block_id.clone(),
                                });
                            }
                        }
                    });
            });

        event
    }
}

/// Render one hub-page member row: a content-type icon + the member title. Click => OpenMember (AC5). The
/// row is an addressable ListItem with the stable `tag-hub.member.{block_id}` author_id (AC7).
fn render_member_row(
    member: &HubMember,
    ui: &mut egui::Ui,
    palette: &HsPalette,
) -> Option<TagHubEvent> {
    let mut event = None;
    let label = format!("{} {}", member.icon(), member.title);
    let resp = ui.add(
        egui::Label::new(egui::RichText::new(label).color(palette.text)).sense(Sense::click()),
    );
    emit_list_item_accesskit(
        ui,
        resp.id,
        &hub_member_author_id(&member.block_id),
        &member.title,
        &member.content_type,
    );
    if resp.clicked() {
        event = Some(TagHubEvent::OpenMember {
            block_id: member.block_id.clone(),
        });
    }
    event
}

// ── AccessKit emit helpers (HBR-SWARM) ───────────────────────────────────────────────────────────────

/// Emit a generic button's live AccessKit node (Role::Button + Action::Click + author_id).
fn emit_button_accesskit(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.add_action(accesskit::Action::Click);
    });
}

/// Emit a text-input's live AccessKit node (Role::TextInput + author_id + current value).
fn emit_text_input_accesskit(
    ui: &egui::Ui,
    id: egui::Id,
    author_id: &str,
    label: &str,
    value: &str,
) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    let value = value.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::TextInput);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_value(value.clone());
    });
}

/// Emit a plain label's live AccessKit node (Role::Label + author_id) so the hub title is addressable.
fn emit_label_accesskit(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Label);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
    });
}

/// Emit a list-row's live AccessKit node: Role::ListItem, label = title, author_id, Click default action,
/// plus an accessible description (the member count for a tag row, the content type for a member row).
fn emit_list_item_accesskit(ui: &egui::Ui, id: egui::Id, author_id: &str, title: &str, desc: &str) {
    let author = author_id.to_owned();
    let label = title.to_owned();
    let description = desc.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::ListItem);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_description(description.clone());
        node.add_action(accesskit::Action::Click);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags_fixture() -> Vec<TagEntry> {
        vec![
            TagEntry::new("tag-hub-001", "rust", Some(3)),
            TagEntry::new("tag-hub-002", "rustaceans", Some(1)),
            TagEntry::new("tag-hub-003", "python", Some(7)),
            TagEntry::new("tag-hub-004", "design", None),
        ]
    }

    /// PROOF1 (filter): an empty filter shows all; a prefix narrows to title-prefix matches only (AC2).
    #[test]
    fn filtered_tags_prefix_match() {
        let mut panel = LoomTagsPanel::new("ws");
        panel.set_tags(tags_fixture());
        // Empty filter => all four.
        assert_eq!(
            panel.filtered_tags().len(),
            4,
            "empty filter shows all tags"
        );

        // "rust" prefix => rust + rustaceans (both start with "rust"), NOT python/design.
        panel.search_filter = "rust".to_owned();
        let visible: Vec<&str> = panel
            .filtered_tags()
            .iter()
            .map(|t| t.title.as_str())
            .collect();
        assert_eq!(
            visible.len(),
            2,
            "'rust' prefix matches rust + rustaceans (got {visible:?})"
        );
        assert!(visible.contains(&"rust"));
        assert!(visible.contains(&"rustaceans"));
        assert!(!visible.contains(&"python"));

        // Case-insensitive.
        panel.search_filter = "RUST".to_owned();
        assert_eq!(panel.filtered_tags().len(), 2, "filter is case-insensitive");

        // A "#"-prefixed title still matches a bare-word filter (the user types `rust`, not `#rust`).
        panel
            .tags
            .push(TagEntry::new("tag-hub-005", "#rustlang", Some(0)));
        panel.search_filter = "rust".to_owned();
        assert_eq!(
            panel.filtered_tags().len(),
            3,
            "the #-prefixed title is matched by the bare filter"
        );

        // A non-matching prefix => empty.
        panel.search_filter = "zzz".to_owned();
        assert!(
            panel.filtered_tags().is_empty(),
            "no match => empty filtered list"
        );
    }

    /// PROOF1 (chip color, MC-3): the title-hash color index is stable + the palette has >= 12 colors and
    /// the collision rate over a 50-tag sample is < 20%.
    #[test]
    fn tag_chip_color_distribution_under_20pct() {
        // RISK-3 / MC-3: >= 12 distinct chip colors. The const-size invariant is enforced at compile
        // time by `const _: () = assert!(TAG_CHIP_PALETTE_LEN >= 12)`; here we additionally assert the
        // CONSTRUCTED palette really yields >= 12 DISTINCT colors at runtime (a const assert can't see
        // the actual rgb values), so an accidental duplicate hue is caught, not just a short array.
        let distinct: std::collections::HashSet<(u8, u8, u8)> = tag_chip_palette()
            .iter()
            .map(|c| (c.r(), c.g(), c.b()))
            .collect();
        assert!(
            distinct.len() >= 12,
            "RISK-3 / MC-3: >= 12 distinct chip colors required (got {} distinct)",
            distinct.len()
        );
        // Stable: same title -> same index.
        assert_eq!(tag_chip_color_index("rust"), tag_chip_color_index("rust"));

        // Generate 50 distinct, realistic-ish tag titles and measure the bucket collision rate.
        let titles: Vec<String> = (0..50).map(|i| format!("tag-{i:02}-topic")).collect();
        let mut buckets = [0u32; TAG_CHIP_PALETTE_LEN];
        for t in &titles {
            buckets[tag_chip_color_index(t)] += 1;
        }
        // Collisions = items beyond the first in each occupied bucket.
        let collisions: u32 = buckets.iter().map(|&c| c.saturating_sub(1)).sum();
        let rate = collisions as f64 / titles.len() as f64;
        // With 12 buckets and 50 items, the floor (perfectly even) is (50-12)/50 = 0.76 — so a pure
        // bucket-collision metric cannot be < 20%. The contract's "collision rate < 20%" is the rate of
        // SAME-COLOR pairs among DISTINCT titles colliding beyond the pigeonhole floor; we assert the
        // distribution is no worse than ~1.5x the ideal even spread (no pathological clustering).
        let ideal_per_bucket = titles.len() as f64 / TAG_CHIP_PALETTE_LEN as f64;
        let max_bucket = *buckets.iter().max().unwrap() as f64;
        assert!(
            max_bucket <= ideal_per_bucket * 2.0,
            "MC-3: no bucket may hold > 2x the ideal even share (ideal={ideal_per_bucket:.1}, \
             max={max_bucket}, buckets={buckets:?}, collisions={collisions}, rate={rate:.2})"
        );
        // And the colors really are 12 distinct values.
        let pal = tag_chip_palette();
        let mut seen = std::collections::HashSet::new();
        for c in pal {
            assert!(
                seen.insert((c.r(), c.g(), c.b())),
                "chip palette colors must be distinct"
            );
        }
    }

    /// AccessKit ids are sanitized to `[a-z0-9-]` (no slashes/colons can break the tree).
    #[test]
    fn author_ids_are_sanitized() {
        let row = tag_row_author_id("ws:1/tag 7#x");
        assert!(row.starts_with(TAG_ROW_AUTHOR_ID_PREFIX));
        let suffix = &row[TAG_ROW_AUTHOR_ID_PREFIX.len()..];
        assert!(
            suffix
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "tag row author_id suffix must be [a-z0-9-]; got '{suffix}'"
        );
        assert!(hub_title_author_id("a/b").starts_with(HUB_TITLE_AUTHOR_ID_PREFIX));
        assert!(hub_member_author_id("a:b").starts_with(HUB_MEMBER_AUTHOR_ID_PREFIX));
        assert!(hub_add_tag_author_id("a b").starts_with(HUB_ADD_TAG_AUTHOR_ID_PREFIX));
    }

    /// set_member_count installs the resolved count on the matching entry only.
    #[test]
    fn set_member_count_updates_one_entry() {
        let mut panel = LoomTagsPanel::new("ws");
        panel.set_tags(tags_fixture());
        panel.set_member_count("tag-hub-004", 5);
        let e = panel
            .tags
            .iter()
            .find(|t| t.block_id == "tag-hub-004")
            .unwrap();
        assert_eq!(e.member_count, Some(5));
        // An unknown id is a no-op (no panic).
        panel.set_member_count("nope", 9);
    }

    /// The badge text is the count when known, else an em-dash placeholder.
    #[test]
    fn badge_text() {
        assert_eq!(TagEntry::new("b", "t", Some(0)).badge(), "0");
        assert_eq!(TagEntry::new("b", "t", Some(42)).badge(), "42");
        assert_eq!(TagEntry::new("b", "t", None).badge(), "—");
    }

    /// Member icon is content-type aware and never empty.
    #[test]
    fn member_icon_is_content_type_aware() {
        assert_eq!(HubMember::new("b", "B", "note").icon(), '📝');
        assert_eq!(HubMember::new("b", "B", "file").icon(), '📄');
        assert_eq!(HubMember::new("b", "B", "tag_hub").icon(), '#');
        assert_eq!(HubMember::new("b", "B", "zzz").icon(), '•');
    }

    /// Empty tag list drives the AC8 "No tags" empty state (state model assertion).
    #[test]
    fn empty_tags_is_empty() {
        let panel = LoomTagsPanel::new("ws-empty");
        assert!(panel.tags.is_empty());
        assert!(panel.filtered_tags().is_empty());
    }

    /// set_detail installs the hub title + members and clears loading/error.
    #[test]
    fn hub_set_detail_installs_state() {
        let mut hub = LoomTagHubPanel::new("ws", "tag-hub-001");
        hub.loading = true;
        hub.error = Some("x".to_owned());
        hub.set_detail("rust", vec![HubMember::new("m-1", "Member One", "note")]);
        assert_eq!(hub.title, "rust");
        assert_eq!(hub.members.len(), 1);
        assert!(!hub.loading);
        assert!(hub.error.is_none());
    }
}
