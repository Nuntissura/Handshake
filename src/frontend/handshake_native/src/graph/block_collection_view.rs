//! Loom block-collection views (WP-KERNEL-012 MT-027, cluster E3) — the native saved-view host.
//!
//! ## What this is
//!
//! [`BlockCollectionView`] is the native peer of the React `app/src/components/BlockCollectionView.tsx`
//! (MT-262 parity). A saved view IS a typed `LoomBlock(content_type='view_def')` whose definition lives
//! in PostgreSQL. The host loads the definition (`getBlockView`), executes its query against the REAL
//! Loom query backend (`queryBlockViewResults`, **POST** with a `{limit,offset}` body — never client
//! params), and renders ONE of three sub-views: [`TableSubView`], [`KanbanSubView`],
//! [`CalendarSubView`]. Authority is PostgreSQL + EventLedger; this widget is a projection.
//!
//! ## The load-bearing invariant — NO client-side sort/filter/group (impl note 71/82, RISK-1)
//!
//! EVERY sort / filter / group / kind / date-range operation is BACKEND-DRIVEN: the host
//! `updateBlockView`s the new definition, then `queryBlockViewResults` re-queries. The widget NEVER
//! sorts, filters, or re-groups the local `results` vec — a client-side reorder would silently lie
//! about the authoritative ordering (the whole point of a saved view). A table header click emits
//! [`BlockViewEvent::Sort`] (flipped direction); a Kanban card drop emits [`BlockViewEvent::CardMove`]
//! (real tag-edge mutation via `updateLoomBlock {add_tags,remove_tags}`), and only the re-query that
//! follows is the source of truth — the local lane vec is NEVER mutated to reflect the move.
//!
//! ## Backend reality (verified read-only — the MT-022/023/024/026 lesson)
//!
//! Verified against `src/backend/handshake_core/src/{api,storage}/loom.rs` + `app/src/lib/api.ts`:
//!   - content_type `view_def` is real (`LoomBlockContentType::ViewDef`).
//!   - `GET    /workspaces/:ws/loom/views/definitions/:block_id`          getBlockView -> BlockViewRecord
//!   - `POST   /workspaces/:ws/loom/views/definitions/:block_id/results`  queryBlockViewResults
//!     body `{limit,offset}` -> BlockViewResults{kind,blocks,groups,total_returned}  (**POST**, RISK-1)
//!   - `PATCH  /workspaces/:ws/loom/views/definitions/:block_id`          updateBlockView body `{definition}`
//!   - `PATCH  /workspaces/:ws/loom/blocks/:block_id`                     updateLoomBlock body
//!     `{add_tags,remove_tags}` (top-level alongside the flattened update) — Kanban lane move
//!   - `POST   /workspaces/:ws/loom/views/definitions`                    createBlockView body
//!     `{block_id?,title?,definition}`
//!
//! The untagged-lane sentinel is `__untagged__` (backend `BLOCK_VIEW_UNTAGGED_LANE`, api.ts:1142),
//! defined here as the named const [`BLOCK_VIEW_UNTAGGED_LANE`] matching EXACTLY (RISK-4 / MC-4).
//!
//! ## Repaint discipline (the MT-015 idle-repaint lesson)
//!
//! The status strip animates ("Re-sorting…" / "Moving card…") ONLY while a genuine in-flight mutation
//! is happening (the host sets `in_flight` when it dispatches a request and clears it on the re-query).
//! A headless / no-runtime render is neutral and non-animating — no perpetual spinner.
//!
//! ## Concurrency guard (RISK-3 / MC-3)
//!
//! A SINGLE `in_flight` flag blocks a concurrent sort + card-move: while a mutation is in flight, sort
//! header clicks and Kanban drops are rejected, so `updateBlockView` and `updateLoomBlock` can never
//! race into an ambiguous re-query.
//!
//! ## AccessKit (HBR-SWARM)
//!
//! Every interactive control emits a live AccessKit node by stable author_id: kind switcher
//! (`bcv.kind.{table|kanban|calendar}`), status (`bcv.status`, Role::Status), new-view button
//! (`bcv.new-view`), table header sort buttons (`bcv.table.sort.{field}`) and rows
//! (`bcv.table.row.{block_id}`), Kanban lanes (`bcv.kanban.lane.{key}`) and cards
//! (`bcv.kanban.card.{block_id}`), calendar day headers (`bcv.calendar.day.{date}`) and entries
//! (`bcv.calendar.entry.{block_id}`), and the calendar date inputs (`bcv.calendar.date-from` /
//! `bcv.calendar.date-to`). Block ids / lane keys / date keys are sanitized to `[a-z0-9-]` via
//! [`crate::project_tree::stable_part`] before forming the author_id suffix (id-integrity).

use egui::accesskit;

use crate::theme::HsPalette;

/// The sentinel Kanban lane key for blocks with no tag in a tag-grouped view. MUST match the backend
/// `handshake_core::storage::loom::BLOCK_VIEW_UNTAGGED_LANE` and `app/src/lib/api.ts` exactly (RISK-4 /
/// MC-4): when the FROM lane is untagged there is no tag to remove (`remove_tags=[]`); when the TO lane
/// is untagged there is no tag to add (`add_tags=[]`).
pub const BLOCK_VIEW_UNTAGGED_LANE: &str = "__untagged__";

/// The egui `DragAndDrop` MIME-equivalent the React Kanban uses (`DRAG_MIME`). Kept as a const for
/// parity/documentation; the native drag uses egui's typed payload channel (RISK-2 / MC-2), not a raw
/// MIME string, so the value is informational.
pub const KANBAN_DRAG_MIME: &str = "application/x-handshake-kanban-card";

// ── AccessKit author_ids (stable strings) ─────────────────────────────────────────────────────────
pub const KIND_TABLE_AUTHOR_ID: &str = "bcv.kind.table";
pub const KIND_KANBAN_AUTHOR_ID: &str = "bcv.kind.kanban";
pub const KIND_CALENDAR_AUTHOR_ID: &str = "bcv.kind.calendar";
pub const STATUS_AUTHOR_ID: &str = "bcv.status";
pub const NEW_VIEW_AUTHOR_ID: &str = "bcv.new-view";
pub const NEW_VIEW_TITLE_AUTHOR_ID: &str = "bcv.new-view.title";
pub const NEW_VIEW_CONFIRM_AUTHOR_ID: &str = "bcv.new-view.confirm";
pub const NEW_VIEW_CANCEL_AUTHOR_ID: &str = "bcv.new-view.cancel";
pub const NEW_VIEW_KIND_TABLE_AUTHOR_ID: &str = "bcv.new-view.kind.table";
pub const NEW_VIEW_KIND_KANBAN_AUTHOR_ID: &str = "bcv.new-view.kind.kanban";
pub const NEW_VIEW_KIND_CALENDAR_AUTHOR_ID: &str = "bcv.new-view.kind.calendar";
pub const CALENDAR_DATE_FROM_AUTHOR_ID: &str = "bcv.calendar.date-from";
pub const CALENDAR_DATE_TO_AUTHOR_ID: &str = "bcv.calendar.date-to";

/// Author_id prefixes (the full id is `prefix.{sanitized_suffix}`).
pub const TABLE_SORT_AUTHOR_ID_PREFIX: &str = "bcv.table.sort.";
pub const TABLE_ROW_AUTHOR_ID_PREFIX: &str = "bcv.table.row.";
pub const KANBAN_LANE_AUTHOR_ID_PREFIX: &str = "bcv.kanban.lane.";
pub const KANBAN_CARD_AUTHOR_ID_PREFIX: &str = "bcv.kanban.card.";
pub const CALENDAR_DAY_AUTHOR_ID_PREFIX: &str = "bcv.calendar.day.";
pub const CALENDAR_ENTRY_AUTHOR_ID_PREFIX: &str = "bcv.calendar.entry.";

/// Row height (px) for the table's VIRTUAL row rendering (RISK-6 / MC-6 — `ScrollArea::show_rows`
/// renders only the visible window, so a 10k-row result never lays out offscreen).
const TABLE_ROW_HEIGHT: f32 = 24.0;

/// The stable AccessKit author_id for a table sort header (`bcv.table.sort.{field}`). The field name is
/// already a fixed `[a-z_]` enum string, so no sanitization is needed; sanitized anyway for safety.
pub fn table_sort_author_id(field: BlockViewField) -> String {
    format!("{TABLE_SORT_AUTHOR_ID_PREFIX}{}", field.as_str())
}

/// The stable AccessKit author_id for a table row (`bcv.table.row.{block_id}`), block id sanitized.
pub fn table_row_author_id(block_id: &str) -> String {
    format!("{TABLE_ROW_AUTHOR_ID_PREFIX}{}", crate::project_tree::stable_part(block_id))
}

/// The stable AccessKit author_id for a Kanban lane (`bcv.kanban.lane.{key}`), lane key sanitized.
pub fn kanban_lane_author_id(lane_key: &str) -> String {
    format!("{KANBAN_LANE_AUTHOR_ID_PREFIX}{}", crate::project_tree::stable_part(lane_key))
}

/// The stable AccessKit author_id for a Kanban card (`bcv.kanban.card.{block_id}`), block id sanitized.
pub fn kanban_card_author_id(block_id: &str) -> String {
    format!("{KANBAN_CARD_AUTHOR_ID_PREFIX}{}", crate::project_tree::stable_part(block_id))
}

/// The stable AccessKit author_id for a calendar day header (`bcv.calendar.day.{date}`), key sanitized.
pub fn calendar_day_author_id(date_key: &str) -> String {
    format!("{CALENDAR_DAY_AUTHOR_ID_PREFIX}{}", crate::project_tree::stable_part(date_key))
}

/// The stable AccessKit author_id for a calendar entry (`bcv.calendar.entry.{block_id}`), id sanitized.
pub fn calendar_entry_author_id(block_id: &str) -> String {
    format!("{CALENDAR_ENTRY_AUTHOR_ID_PREFIX}{}", crate::project_tree::stable_part(block_id))
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// View-definition types (mirror the VERIFIED backend `storage::loom` + `api.ts` shapes exactly). These
// are the native projection of the wire JSON; the host's [`crate::backend_client::BlockViewClient`]
// parses the real response into these.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Which sub-view a saved view renders. Wire value is the snake_case string (`table`/`kanban`/
/// `calendar`) — matches backend `BlockViewKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockViewKind {
    Table,
    Kanban,
    Calendar,
}

impl BlockViewKind {
    /// The wire string (`table`/`kanban`/`calendar`).
    pub fn as_str(self) -> &'static str {
        match self {
            BlockViewKind::Table => "table",
            BlockViewKind::Kanban => "kanban",
            BlockViewKind::Calendar => "calendar",
        }
    }

    /// Parse the wire string; unknown values default to `Table` (never panic on a backend addition).
    /// Named `parse_str` (not `from_str`) so it does not shadow `std::str::FromStr::from_str`.
    pub fn parse_str(s: &str) -> Self {
        match s {
            "kanban" => BlockViewKind::Kanban,
            "calendar" => BlockViewKind::Calendar,
            _ => BlockViewKind::Table,
        }
    }

    /// The human label for the kind-switcher button.
    pub fn label(self) -> &'static str {
        match self {
            BlockViewKind::Table => "Table",
            BlockViewKind::Kanban => "Kanban",
            BlockViewKind::Calendar => "Calendar",
        }
    }
}

/// A typed orderable/displayable block field. Wire value is the snake_case string — matches backend
/// `BlockViewField`. Every variant maps to a SQL column so ORDER BY runs server-side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockViewField {
    Title,
    Created,
    Updated,
    JournalDate,
    ContentType,
    Pinned,
    Favorite,
    BacklinkCount,
    MentionCount,
    TagCount,
}

impl BlockViewField {
    /// The wire string (matches backend `BlockViewField::as_str`).
    pub fn as_str(self) -> &'static str {
        match self {
            BlockViewField::Title => "title",
            BlockViewField::Created => "created",
            BlockViewField::Updated => "updated",
            BlockViewField::JournalDate => "journal_date",
            BlockViewField::ContentType => "content_type",
            BlockViewField::Pinned => "pinned",
            BlockViewField::Favorite => "favorite",
            BlockViewField::BacklinkCount => "backlink_count",
            BlockViewField::MentionCount => "mention_count",
            BlockViewField::TagCount => "tag_count",
        }
    }

    /// Parse the wire string; unknown values are `None` (a malformed field is dropped, not faked).
    /// Named `parse_str` (not `from_str`) so it does not shadow `std::str::FromStr::from_str`.
    pub fn parse_str(s: &str) -> Option<Self> {
        Some(match s {
            "title" => BlockViewField::Title,
            "created" => BlockViewField::Created,
            "updated" => BlockViewField::Updated,
            "journal_date" => BlockViewField::JournalDate,
            "content_type" => BlockViewField::ContentType,
            "pinned" => BlockViewField::Pinned,
            "favorite" => BlockViewField::Favorite,
            "backlink_count" => BlockViewField::BacklinkCount,
            "mention_count" => BlockViewField::MentionCount,
            "tag_count" => BlockViewField::TagCount,
            _ => return None,
        })
    }

    /// The column header label (matches the React `COLUMN_LABELS`).
    pub fn label(self) -> &'static str {
        match self {
            BlockViewField::Title => "Title",
            BlockViewField::Created => "Created",
            BlockViewField::Updated => "Updated",
            BlockViewField::JournalDate => "Journal date",
            BlockViewField::ContentType => "Type",
            BlockViewField::Pinned => "Pinned",
            BlockViewField::Favorite => "Favorite",
            BlockViewField::BacklinkCount => "Backlinks",
            BlockViewField::MentionCount => "Mentions",
            BlockViewField::TagCount => "Tags",
        }
    }
}

/// Server-side sort direction. Wire value is `asc`/`desc` (matches backend `BlockViewSortDirection`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockViewSortDirection {
    Asc,
    Desc,
}

impl BlockViewSortDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            BlockViewSortDirection::Asc => "asc",
            BlockViewSortDirection::Desc => "desc",
        }
    }

    /// The sort-indicator glyph the table header shows (▲ asc / ▼ desc).
    pub fn indicator(self) -> &'static str {
        match self {
            BlockViewSortDirection::Asc => " ▲",
            BlockViewSortDirection::Desc => " ▼",
        }
    }
}

/// A typed `(field, direction)` server-side sort.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockViewSort {
    pub field: BlockViewField,
    pub direction: BlockViewSortDirection,
}

/// The server-side query window for a saved view. The native projection models EVERY field of the
/// verified backend `BlockViewQuery` (`storage::loom::BlockViewQuery`) so a sort / kind / date-range
/// `updateBlockView` round-trip — which the backend persists as a FULL overwrite of
/// `view_definition_json` (`SET view_definition_json = $1`, NOT a merge) — never silently drops a
/// server-side filter the user set elsewhere (the must-fix #2 / backend-shape #4 data-loss defect).
///
/// `date_from`/`date_to` are kept here as the calendar UI's `YYYY-MM-DD` shape (the read path slices
/// the backend's full RFC3339 timestamp to 10 chars); they are EXPANDED back to a full RFC3339 instant
/// at the serialization boundary (`definition_to_json`) because the backend field is
/// `Option<DateTime<Utc>>` with the default chrono serde, which REJECTS a bare date-only string.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockViewQuery {
    /// `date_from` (ISO `YYYY-MM-DD`) — applied SERVER-SIDE by the calendar date window. Expanded to
    /// `<date>T00:00:00Z` on the wire (backend type `Option<DateTime<Utc>>`).
    pub date_from: Option<String>,
    /// `date_to` (ISO `YYYY-MM-DD`) — applied SERVER-SIDE by the calendar date window. Expanded to the
    /// inclusive end-of-day `<date>T23:59:59Z` on the wire (backend type `Option<DateTime<Utc>>`).
    pub date_to: Option<String>,
    /// Server-side `content_type` filter. Carried so a `updateBlockView` round-trip never wipes it.
    pub content_type: Option<String>,
    /// Server-side `mime` filter. Carried so a round-trip never wipes it.
    pub mime: Option<String>,
    /// Server-side `tag_ids` filter (also the Kanban tag-grouping universe). Carried so a round-trip
    /// never wipes the user's tag filter / lane universe.
    pub tag_ids: Vec<String>,
    /// Server-side `mention_ids` filter. Carried so a round-trip never wipes it.
    pub mention_ids: Vec<String>,
}

/// How a Kanban view groups its cards — the native projection of the verified backend
/// `BlockViewGroupBy` (`storage::loom`, `#[serde(tag = "kind", rename_all = "snake_case")]`):
/// `{"kind":"tag"}` or `{"kind":"field","field":"<field>"}`. The backend builds Kanban lanes ONLY when
/// `definition.group_by` is `Some(..)` (the wildcard arm returns zero lanes), so this MUST round-trip
/// faithfully: a natively-created Kanban view defaults to `Tag` (so `+ New view` produces lanes), and
/// an existing Kanban view's grouping survives every sort / kind / date `updateBlockView` (must-fix #3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockViewGroupBy {
    /// Group by tag edge: one lane per tag block id (+ the implicit `__untagged__` lane). Drag between
    /// lanes mutates the real tag edges (the Kanban card-move contract). Wire: `{"kind":"tag"}`.
    Tag,
    /// Group by a typed block field (e.g. content_type). Lane keys are field VALUES, not tag ids.
    /// Wire: `{"kind":"field","field":"<field>"}`.
    Field { field: BlockViewField },
}

/// The full definition of a saved view (the native projection of the verified `BlockViewDefinition`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockViewDefinition {
    pub kind: BlockViewKind,
    pub query: BlockViewQuery,
    /// Table columns (ordered). Empty => the React default `[title, updated]`.
    pub columns: Vec<BlockViewField>,
    /// Kanban grouping (only meaningful for `kind = kanban`). The backend builds lanes ONLY when this
    /// is `Some(..)`, so it MUST round-trip: dropping it would render zero lanes (native-created views)
    /// or permanently wipe the grouping on the first sort/kind/date edit of an existing Kanban view
    /// (the backend overwrites the whole `view_definition_json`). See must-fix #3.
    pub group_by: Option<BlockViewGroupBy>,
    /// Server-side sort (table / calendar ordering). `None` => unsorted.
    pub sort: Option<BlockViewSort>,
    /// Which date field the calendar buckets by. `None` => `created` (the React default).
    pub calendar_date_field: Option<BlockViewField>,
}

impl BlockViewDefinition {
    /// A minimal definition of the given kind (used by the New-view popup + tests). A Kanban view
    /// defaults to `group_by = Tag` so a natively-created Kanban view actually produces lanes (the
    /// backend returns zero lanes for a Kanban view with `group_by = None` — must-fix #3); table and
    /// calendar views carry no grouping.
    pub fn of_kind(kind: BlockViewKind) -> Self {
        let group_by = match kind {
            BlockViewKind::Kanban => Some(BlockViewGroupBy::Tag),
            BlockViewKind::Table | BlockViewKind::Calendar => None,
        };
        Self {
            kind,
            query: BlockViewQuery::default(),
            columns: Vec::new(),
            group_by,
            sort: None,
            calendar_date_field: None,
        }
    }

    /// The columns to render, falling back to the React default `[title, updated]` when none are set.
    pub fn effective_columns(&self) -> Vec<BlockViewField> {
        if self.columns.is_empty() {
            vec![BlockViewField::Title, BlockViewField::Updated]
        } else {
            self.columns.clone()
        }
    }
}

/// One Loom block row in a view result (the native projection of the verified `LoomBlock` fields the
/// table/kanban/calendar sub-views read — NOT the whole block). Only the cell-value + bucket-key +
/// title fields are modeled; the host parses these from the real `LoomBlock` JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoomBlockRow {
    pub block_id: String,
    pub title: Option<String>,
    pub original_filename: Option<String>,
    pub content_type: String,
    pub journal_date: Option<String>,
    /// ISO-8601 `created_at` (the calendar `created` bucket slices `[0..10]`).
    pub created_at: String,
    /// ISO-8601 `updated_at` (the calendar `updated` bucket slices `[0..10]`).
    pub updated_at: String,
    pub pinned: bool,
    pub favorite: bool,
    pub backlink_count: i64,
    pub mention_count: i64,
    pub tag_count: i64,
}

impl LoomBlockRow {
    /// The display title: `title ?? original_filename ?? block_id` (the React `cellValue('title')` /
    /// card-label / entry-label rule).
    pub fn display_title(&self) -> &str {
        match &self.title {
            Some(t) if !t.trim().is_empty() => t.as_str(),
            _ => match &self.original_filename {
                Some(f) if !f.trim().is_empty() => f.as_str(),
                _ => self.block_id.as_str(),
            },
        }
    }

    /// The string cell value for a column (the React `LoomTableView.cellValue`).
    pub fn cell_value(&self, field: BlockViewField) -> String {
        match field {
            BlockViewField::Title => self.display_title().to_owned(),
            BlockViewField::Created => self.created_at.clone(),
            BlockViewField::Updated => self.updated_at.clone(),
            BlockViewField::JournalDate => self.journal_date.clone().unwrap_or_default(),
            BlockViewField::ContentType => self.content_type.clone(),
            BlockViewField::Pinned => yes_no(self.pinned),
            BlockViewField::Favorite => yes_no(self.favorite),
            BlockViewField::BacklinkCount => self.backlink_count.to_string(),
            BlockViewField::MentionCount => self.mention_count.to_string(),
            BlockViewField::TagCount => self.tag_count.to_string(),
        }
    }
}

fn yes_no(b: bool) -> String {
    if b { "yes".to_owned() } else { "no".to_owned() }
}

/// One Kanban lane: a grouping key + the blocks in it (already server-side sorted/grouped). The native
/// projection of the verified `BlockViewLane`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockViewLane {
    pub key: String,
    pub blocks: Vec<LoomBlockRow>,
}

impl BlockViewLane {
    /// The lane header label: `Untagged` for the sentinel, else the raw key (React `laneLabel`).
    pub fn label(&self) -> &str {
        if self.key == BLOCK_VIEW_UNTAGGED_LANE {
            "Untagged"
        } else {
            self.key.as_str()
        }
    }
}

/// The result of executing a saved view's query (native projection of the verified `BlockViewResults`).
/// `blocks` is the flat table/calendar result; `groups` is populated for Kanban views.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockViewResults {
    pub kind_str: String,
    pub blocks: Vec<LoomBlockRow>,
    pub groups: Vec<BlockViewLane>,
    pub total_returned: u32,
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// Standalone backend-driven logic (unit-testable WITHOUT egui or a backend — PROOF1).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Flip the sort for a header click (React `flipDirection`, lines 39-47): same field + currently `asc`
/// -> `desc`; same field + currently `desc` (or unset direction) -> `asc`; a NEW field -> `asc`.
/// Returns the NEW [`BlockViewSort`] the host persists via `updateBlockView`. This is the ONLY place a
/// sort changes — the local results vec is never reordered (the invariant; the re-query is truth).
pub fn flip_direction(current: Option<BlockViewSort>, field: BlockViewField) -> BlockViewSort {
    let direction = match current {
        Some(sort) if sort.field == field && sort.direction == BlockViewSortDirection::Asc => {
            BlockViewSortDirection::Desc
        }
        // same field + desc, OR a different field, OR no current sort => asc
        _ => BlockViewSortDirection::Asc,
    };
    BlockViewSort { field, direction }
}

/// The calendar bucket key for a block (React `bucketKey`, lines 16-24): `journal_date` field ->
/// `block.journal_date ?? "undated"`; `updated` -> `updated_at[0..10]`; `created` (and default) ->
/// `created_at[0..10]`. The slice is the ISO date portion (`YYYY-MM-DD`).
pub fn bucket_key(block: &LoomBlockRow, field: Option<BlockViewField>) -> String {
    match field {
        Some(BlockViewField::JournalDate) => block
            .journal_date
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "undated".to_owned()),
        Some(BlockViewField::Updated) => iso_date_slice(&block.updated_at),
        // created + default (and any other field) bucket by created_at, matching the React `default`.
        _ => iso_date_slice(&block.created_at),
    }
}

/// The first 10 chars of an ISO-8601 timestamp (`YYYY-MM-DD`), char-safe (never panics mid-codepoint
/// on a short/odd string — `slice(0,10)` in JS is by UTF-16 unit; we take up to 10 chars).
fn iso_date_slice(iso: &str) -> String {
    iso.chars().take(10).collect()
}

/// The `(add_tags, remove_tags)` a Kanban card move emits (React `handleCardMove`, lines 110-127 +
/// api.ts:1142): moving FROM `from_key` TO `to_key` adds `[to_key]` unless TO is untagged (then `[]`)
/// and removes `[from_key]` unless FROM is untagged (then `[]`). The untagged sentinel must match
/// [`BLOCK_VIEW_UNTAGGED_LANE`] exactly (RISK-4 / MC-4).
pub fn card_move_tags(from_key: &str, to_key: &str) -> (Vec<String>, Vec<String>) {
    let add_tags = if to_key == BLOCK_VIEW_UNTAGGED_LANE {
        Vec::new()
    } else {
        vec![to_key.to_owned()]
    };
    let remove_tags = if from_key == BLOCK_VIEW_UNTAGGED_LANE {
        Vec::new()
    } else {
        vec![from_key.to_owned()]
    };
    (add_tags, remove_tags)
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// The typed events the host applies through the backend client, then re-fetches. The widget itself
// performs NO network IO (HBR-QUIET — the host spawns each request off the UI thread).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// The typed event a view interaction produces this frame. Every variant maps to a verified backend
/// route the host drives through [`crate::backend_client::BlockViewClient`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockViewEvent {
    /// Persist a new sort (`updateBlockView` with `definition.sort = sort`), then re-query. The host
    /// sets `in_flight` while the PATCH + re-query is in flight (status "Re-sorting…").
    Sort { sort: BlockViewSort },
    /// Change the view kind (`updateBlockView` with `definition.kind = kind`), then re-query.
    KindChange { kind: BlockViewKind },
    /// Move a Kanban card between lanes: `updateLoomBlock(block_id, {add_tags, remove_tags})` then
    /// re-query (status "Moving card…"). The local lane vec is NEVER mutated — the re-query is truth.
    CardMove {
        block_id: String,
        add_tags: Vec<String>,
        remove_tags: Vec<String>,
    },
    /// Apply a new calendar date window (`updateBlockView` with `query.date_from/date_to`), then
    /// re-query. `None` clears that bound. Only emitted after regex validation (RISK-5 / MC-5).
    DateRange {
        date_from: Option<String>,
        date_to: Option<String>,
    },
    /// Create a new saved view (`createBlockView({title, definition})`), then switch to its block id.
    CreateView {
        title: String,
        kind: BlockViewKind,
    },
}

/// Transient state of the "+ New view" popup (Window). `None` => closed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NewViewForm {
    title: String,
    kind: BlockViewKind,
}

impl Default for NewViewForm {
    fn default() -> Self {
        Self { title: String::new(), kind: BlockViewKind::Table }
    }
}

/// The block-collection view host. Held by the host pane, mutated in place by
/// [`BlockCollectionView::show`]. `definition` + `results` are the projection of authoritative backend
/// state the host loads via `getBlockView` + `queryBlockViewResults`; all other fields are ephemeral UI
/// state.
#[derive(Debug, Clone)]
pub struct BlockCollectionView {
    pub workspace_id: String,
    pub view_block_id: String,
    pub definition: Option<BlockViewDefinition>,
    pub results: Option<BlockViewResults>,
    pub loading: bool,
    pub error: Option<String>,
    /// The status strip text ("Re-sorting…" / "Moving card…" / "") driven by the host's mutation
    /// lifecycle (NOT a per-frame animation — MT-015 idle-repaint lesson).
    pub status: String,
    /// SINGLE in-flight guard (RISK-3 / MC-3): true while ANY mutation (sort / kind / card-move /
    /// date-range) is dispatched and the re-query has not yet landed. Sort clicks and Kanban drops are
    /// REJECTED while true, so updateBlockView + updateLoomBlock can never race.
    pub in_flight: bool,
    /// The active drag (a card being dragged) — set on drag start, cleared on drop/refresh. Mirrors
    /// the React `DragState`.
    pub kanban_drag: Option<KanbanDragState>,
    /// Calendar date-range inputs (the editable `YYYY-MM-DD` strings before validation).
    pub date_from_input: String,
    pub date_to_input: String,
    /// Inline validation error for the date inputs (RISK-5 / MC-5).
    date_error: Option<String>,
    /// The "+ New view" popup form (`None` => closed).
    new_view: Option<NewViewForm>,
}

/// The card a Kanban drag started from: its block id + the lane it left. The drop target's lane key
/// completes the move. Must be `Send + Sync + 'static` for egui's `DragAndDrop` store (compile-gated by
/// `kanban_drag_state_is_send_sync_static`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KanbanDragState {
    pub block_id: String,
    pub from_lane_key: String,
}

impl BlockCollectionView {
    /// A fresh host for `workspace_id` + `view_block_id` (no definition/results yet — the host loads
    /// them via `getBlockView` then `queryBlockViewResults`).
    pub fn new(workspace_id: impl Into<String>, view_block_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            view_block_id: view_block_id.into(),
            definition: None,
            results: None,
            loading: false,
            error: None,
            status: String::new(),
            in_flight: false,
            kanban_drag: None,
            date_from_input: String::new(),
            date_to_input: String::new(),
            date_error: None,
            new_view: None,
        }
    }

    /// Install a loaded definition + results (after `getBlockView` + `queryBlockViewResults` resolve).
    /// Clears the in-flight + status state (the re-query has landed — the move/sort is now authority).
    pub fn set_loaded(&mut self, definition: BlockViewDefinition, results: BlockViewResults) {
        // Seed the editable date inputs from the loaded definition so the calendar window reflects PG.
        self.date_from_input = definition.query.date_from.clone().unwrap_or_default();
        self.date_to_input = definition.query.date_to.clone().unwrap_or_default();
        self.definition = Some(definition);
        self.results = Some(results);
        self.loading = false;
        self.error = None;
        self.in_flight = false;
        self.status.clear();
        self.kanban_drag = None;
    }

    /// Set the error state (a `getBlockView` / query failure). Clears in-flight so the UI is reachable.
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.error = Some(message.into());
        self.loading = false;
        self.in_flight = false;
        self.status.clear();
    }

    /// Render the host + return the typed event (if any) this frame produced. The host applies the
    /// event (mutate via the backend client, set `in_flight`, then re-fetch + `set_loaded`). The widget
    /// performs NO network IO.
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette) -> Option<BlockViewEvent> {
        let mut event: Option<BlockViewEvent> = None;

        // ── Error state: a single alert label, nothing else (parity with the React error branch). ───
        if let Some(err) = self.error.clone() {
            let resp = ui.colored_label(palette.error_text, format!("View error: {err}"));
            emit_status_node(ui, resp.id, STATUS_AUTHOR_ID, &format!("View error: {err}"));
            return None;
        }

        // ── Mode strip: kind switcher + status + new-view button ─────────────────────────────────────
        let current_kind = self.definition.as_ref().map(|d| d.kind).unwrap_or(BlockViewKind::Table);
        ui.horizontal(|ui| {
            if let Some(ev) = self.kind_switcher(ui, current_kind) {
                event = Some(ev);
            }
            ui.separator();
            if let Some(ev) = self.new_view_button(ui) {
                event = Some(ev);
            }
        });

        // ── Status bar (Role::Status). Animates ONLY when a mutation is genuinely in flight. ─────────
        let status_text = if !self.status.is_empty() {
            self.status.clone()
        } else if self.loading {
            "Loading view…".to_owned()
        } else {
            String::new()
        };
        let status_resp = ui.label(if status_text.is_empty() { " " } else { status_text.as_str() });
        emit_status_node(ui, status_resp.id, STATUS_AUTHOR_ID, &status_text);
        // Only request a repaint while a real mutation is in flight (no perpetual spinner — MT-015).
        if self.in_flight {
            ui.ctx().request_repaint();
        }

        ui.separator();

        // ── The active sub-view. Loading (no definition/results yet) shows a neutral message. ────────
        let (definition, results) = match (self.definition.clone(), self.results.clone()) {
            (Some(d), Some(r)) => (d, r),
            _ => {
                ui.label("Loading view…");
                return event;
            }
        };

        // Run the New-view popup window (if open). It may emit a CreateView event.
        if let Some(ev) = self.new_view_popup(ui) {
            event = Some(ev);
        }

        match definition.kind {
            BlockViewKind::Table => {
                let mut table = TableSubView { definition: &definition, results: &results };
                if let Some(ev) = table.show(ui, palette, self.in_flight) {
                    event = Some(ev);
                }
            }
            BlockViewKind::Kanban => {
                if let Some(ev) = self.show_kanban(ui, palette, &definition, &results) {
                    event = Some(ev);
                }
            }
            BlockViewKind::Calendar => {
                if let Some(ev) = self.show_calendar(ui, palette, &definition, &results) {
                    event = Some(ev);
                }
            }
        }

        event
    }

    /// The kind-switcher strip (`bcv.kind.{table|kanban|calendar}`). A click on a non-current kind emits
    /// [`BlockViewEvent::KindChange`] (the host persists via updateBlockView + re-queries). Rejected
    /// while a mutation is in flight (RISK-3 / MC-3).
    fn kind_switcher(&self, ui: &mut egui::Ui, current: BlockViewKind) -> Option<BlockViewEvent> {
        let mut event = None;
        for (kind, author_id) in [
            (BlockViewKind::Table, KIND_TABLE_AUTHOR_ID),
            (BlockViewKind::Kanban, KIND_KANBAN_AUTHOR_ID),
            (BlockViewKind::Calendar, KIND_CALENDAR_AUTHOR_ID),
        ] {
            let selected = kind == current;
            let btn = ui.add(egui::Button::selectable(selected, kind.label()));
            emit_button_node(ui, btn.id, author_id, kind.label(), selected);
            if btn.clicked() && !selected && !self.in_flight {
                event = Some(BlockViewEvent::KindChange { kind });
            }
        }
        event
    }

    /// The "+ New view" button. A click opens the popup form. Returns no event itself (the popup's
    /// confirm does).
    fn new_view_button(&mut self, ui: &mut egui::Ui) -> Option<BlockViewEvent> {
        let btn = ui.button("+ New view");
        emit_button_node(ui, btn.id, NEW_VIEW_AUTHOR_ID, "New view", false);
        if btn.clicked() && self.new_view.is_none() {
            self.new_view = Some(NewViewForm::default());
        }
        None
    }

    /// The "+ New view" popup (egui::Window). On confirm, emits [`BlockViewEvent::CreateView`] and
    /// closes; on cancel, just closes. Returns the event (if confirmed this frame).
    fn new_view_popup(&mut self, ui: &mut egui::Ui) -> Option<BlockViewEvent> {
        // `?` early-returns when the popup is closed; clone into a mutable binding for the form edits.
        let mut form = self.new_view.clone()?;
        let mut event = None;
        let mut open = true;
        egui::Window::new("New view")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                let title_resp = ui.add(
                    egui::TextEdit::singleline(&mut form.title)
                        .hint_text("View title")
                        .desired_width(220.0),
                );
                emit_text_field_node(ui, title_resp.id, NEW_VIEW_TITLE_AUTHOR_ID, &form.title);

                ui.horizontal(|ui| {
                    for (kind, author_id) in [
                        (BlockViewKind::Table, NEW_VIEW_KIND_TABLE_AUTHOR_ID),
                        (BlockViewKind::Kanban, NEW_VIEW_KIND_KANBAN_AUTHOR_ID),
                        (BlockViewKind::Calendar, NEW_VIEW_KIND_CALENDAR_AUTHOR_ID),
                    ] {
                        let selected = form.kind == kind;
                        let r = ui.add(egui::Button::selectable(selected, kind.label()));
                        emit_button_node(ui, r.id, author_id, kind.label(), selected);
                        if r.clicked() {
                            form.kind = kind;
                        }
                    }
                });

                ui.horizontal(|ui| {
                    let confirm = ui.button("Create");
                    emit_button_node(ui, confirm.id, NEW_VIEW_CONFIRM_AUTHOR_ID, "Create view", false);
                    if confirm.clicked() {
                        event = Some(BlockViewEvent::CreateView {
                            title: form.title.trim().to_owned(),
                            kind: form.kind,
                        });
                    }
                    let cancel = ui.button("Cancel");
                    emit_button_node(ui, cancel.id, NEW_VIEW_CANCEL_AUTHOR_ID, "Cancel", false);
                    if cancel.clicked() {
                        self.new_view = None;
                    }
                });
            });

        if event.is_some() || !open {
            // Confirmed or the window's close 'x' was pressed -> close the popup.
            self.new_view = None;
        } else {
            // Persist the in-progress form edits for the next frame.
            self.new_view = Some(form);
        }
        event
    }

    /// Render the Kanban sub-view and apply any card-move drop (RISK-2/MC-2: egui DragAndDrop payload,
    /// not OS drag). A drop is REJECTED while a mutation is in flight (RISK-3 / MC-3).
    fn show_kanban(
        &mut self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        definition: &BlockViewDefinition,
        results: &BlockViewResults,
    ) -> Option<BlockViewEvent> {
        let mut view = KanbanSubView {
            in_flight: self.in_flight,
            drag: &mut self.kanban_drag,
        };
        let outcome = view.show(ui, palette, results);
        if let Some((block_id, from_key, to_key)) = outcome {
            // Card-move = a REAL tag-edge mutation. This is correct ONLY for TAG grouping (lane key = a
            // tag block id). For FIELD grouping the lane key is a field VALUE ("note"/"pinned"/…), so a
            // tag mutation would add a BOGUS tag edge to a non-existent TagHub id (the group_by=Field
            // card-move risk). The MT-027 contract scopes card-move to tag-lane semantics, so a
            // field-grouped drop is a SAFE no-op here (never a corrupting tag write) rather than an
            // unconditional bogus mutation. `None`/`Tag` => tag semantics (the default Kanban grouping).
            let tag_grouped = !matches!(definition.group_by, Some(BlockViewGroupBy::Field { .. }));
            if tag_grouped {
                let (add_tags, remove_tags) = card_move_tags(&from_key, &to_key);
                self.status = "Moving card…".to_owned();
                return Some(BlockViewEvent::CardMove { block_id, add_tags, remove_tags });
            }
        }
        None
    }

    /// Render the Calendar sub-view + the date-range inputs (RISK-5 / MC-5 regex-validated).
    fn show_calendar(
        &mut self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        definition: &BlockViewDefinition,
        results: &BlockViewResults,
    ) -> Option<BlockViewEvent> {
        let mut event = None;

        // Date-range inputs (server-side filtering — RISK-5 / MC-5: regex-validate before emitting).
        ui.horizontal(|ui| {
            ui.label("From:");
            let from = ui.add(
                egui::TextEdit::singleline(&mut self.date_from_input)
                    .hint_text("YYYY-MM-DD")
                    .desired_width(110.0),
            );
            emit_text_field_node(ui, from.id, CALENDAR_DATE_FROM_AUTHOR_ID, &self.date_from_input);

            ui.label("To:");
            let to = ui.add(
                egui::TextEdit::singleline(&mut self.date_to_input)
                    .hint_text("YYYY-MM-DD")
                    .desired_width(110.0),
            );
            emit_text_field_node(ui, to.id, CALENDAR_DATE_TO_AUTHOR_ID, &self.date_to_input);

            let apply = ui.button("Apply range");
            emit_button_node(ui, apply.id, "bcv.calendar.apply-range", "Apply date range", false);
            if apply.clicked() && !self.in_flight {
                match self.validated_date_range() {
                    Ok((from, to)) => {
                        self.date_error = None;
                        event = Some(BlockViewEvent::DateRange { date_from: from, date_to: to });
                    }
                    Err(msg) => self.date_error = Some(msg),
                }
            }
        });
        if let Some(err) = &self.date_error {
            ui.colored_label(palette.error_text, err);
        }

        ui.separator();

        let calendar = CalendarSubView { definition, results };
        calendar.show(ui, palette);
        event
    }

    /// Validate the two date inputs against `^\d{4}-\d{2}-\d{2}$` (RISK-5 / MC-5). An empty input is a
    /// cleared bound (`None`); a non-empty input that fails the shape is an error (the backend filter is
    /// never sent an invalid date that could crash it).
    fn validated_date_range(&self) -> Result<(Option<String>, Option<String>), String> {
        let from = self.validate_one(&self.date_from_input).map_err(|_| {
            format!("Invalid 'from' date '{}': expected YYYY-MM-DD", self.date_from_input)
        })?;
        let to = self.validate_one(&self.date_to_input).map_err(|_| {
            format!("Invalid 'to' date '{}': expected YYYY-MM-DD", self.date_to_input)
        })?;
        Ok((from, to))
    }

    fn validate_one(&self, input: &str) -> Result<Option<String>, ()> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        if is_iso_date(trimmed) {
            Ok(Some(trimmed.to_owned()))
        } else {
            Err(())
        }
    }
}

/// True iff `s` matches `^\d{4}-\d{2}-\d{2}$` (the RISK-5 / MC-5 date shape) — no regex crate dependency.
pub fn is_iso_date(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 10 {
        return false;
    }
    for (i, &b) in bytes.iter().enumerate() {
        let ok = match i {
            4 | 7 => b == b'-',
            _ => b.is_ascii_digit(),
        };
        if !ok {
            return false;
        }
    }
    true
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// TableSubView — a sortable typed-column table over REAL query results. A header click does NOT sort
// client-side: it emits a Sort event (re-query). Rows render VIRTUALLY (RISK-6 / MC-6).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// The table sub-view: borrows the definition (columns + sort) and results (rows) for one frame.
pub struct TableSubView<'a> {
    pub definition: &'a BlockViewDefinition,
    pub results: &'a BlockViewResults,
}

impl TableSubView<'_> {
    /// Render the header (sort buttons) + virtual rows. Returns a [`BlockViewEvent::Sort`] if a header
    /// was clicked this frame (rejected while `in_flight` — RISK-3 / MC-3).
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        in_flight: bool,
    ) -> Option<BlockViewEvent> {
        let mut event = None;
        let columns = self.definition.effective_columns();

        // Header: one sort button per column. Click flips the direction (backend-driven re-sort).
        ui.horizontal(|ui| {
            for &field in &columns {
                let indicator = self
                    .definition
                    .sort
                    .filter(|s| s.field == field)
                    .map(|s| s.direction.indicator())
                    .unwrap_or("");
                let label = format!("{}{indicator}", field.label());
                let btn = ui.button(&label);
                emit_button_node(ui, btn.id, &table_sort_author_id(field), &label, false);
                if btn.clicked() && !in_flight {
                    let sort = flip_direction(self.definition.sort, field);
                    event = Some(BlockViewEvent::Sort { sort });
                }
            }
        });
        ui.separator();

        // VIRTUAL rows (RISK-6 / MC-6): ScrollArea::show_rows renders only the visible window, so a
        // 10k-row result never lays out offscreen rows. Each row is an addressable AccessKit node.
        let row_count = self.results.blocks.len();
        if row_count == 0 {
            ui.weak("No blocks match this view.");
            return event;
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(
            ui,
            TABLE_ROW_HEIGHT,
            row_count,
            |ui, row_range| {
                for idx in row_range {
                    let block = &self.results.blocks[idx];
                    self.row(ui, palette, block, &columns);
                }
            },
        );
        event
    }

    /// Render one table row as an addressable AccessKit node (label = the joined cell values so a swarm
    /// agent / test can read the row content by `bcv.table.row.{block_id}`).
    fn row(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        block: &LoomBlockRow,
        columns: &[BlockViewField],
    ) {
        let author_id = table_row_author_id(&block.block_id);
        let id = egui::Id::new(&author_id);
        let cells: Vec<String> = columns.iter().map(|&f| block.cell_value(f)).collect();
        let label = cells.join(" | ");
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), TABLE_ROW_HEIGHT), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter().text(
                egui::pos2(rect.left() + 4.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                &label,
                egui::FontId::proportional(13.0),
                palette.text,
            );
        }
        let _ = resp;
        let label_for_node = label.clone();
        ui.ctx().accesskit_node_builder(id, move |node| {
            node.set_role(accesskit::Role::Row);
            node.set_author_id(author_id.clone());
            node.set_label(label_for_node.clone());
        });
    }
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KanbanSubView — a tag-grouped Kanban over REAL query results. Dragging a card between lanes is a REAL
// mutation (updateLoomBlock add/remove-tags), then re-query — never a local lane-array mutation.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// The Kanban sub-view: drives the egui DragAndDrop payload + drop detection. Borrows the host's
/// in-flight flag (drop guard) and drag state for one frame.
pub struct KanbanSubView<'a> {
    pub in_flight: bool,
    pub drag: &'a mut Option<KanbanDragState>,
}

impl KanbanSubView<'_> {
    /// Render the lanes + cards. Returns `Some((block_id, from_lane_key, to_lane_key))` when a card was
    /// dropped on a DIFFERENT lane this frame (the host turns this into a [`BlockViewEvent::CardMove`]).
    /// A drop is REJECTED while `in_flight` (RISK-3 / MC-3) and when from == to.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        results: &BlockViewResults,
    ) -> Option<(String, String, String)> {
        if results.groups.is_empty() {
            ui.weak("No lanes in this view.");
            return None;
        }
        let mut outcome = None;
        ui.horizontal_top(|ui| {
            for lane in &results.groups {
                if let Some(o) = self.lane(ui, palette, lane) {
                    outcome = Some(o);
                }
            }
        });
        outcome
    }

    /// Render one lane (drop target) + its cards (drag sources). Returns the drop outcome if a card was
    /// dropped on this lane from a different lane.
    fn lane(
        &mut self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        lane: &BlockViewLane,
    ) -> Option<(String, String, String)> {
        let lane_author = kanban_lane_author_id(&lane.key);
        let mut outcome = None;

        // The lane is a drop target: egui's dnd_drop_zone gives the inner rect a drop response. The
        // payload type is KanbanDragState (typed, not OS drag — RISK-2 / MC-2).
        let frame = egui::Frame::group(ui.style()).fill(palette.surface);
        let (_, payload) = ui.dnd_drop_zone::<KanbanDragState, ()>(frame, |ui| {
            ui.set_min_width(180.0);
            let header = ui.heading(lane.label());
            emit_lane_node(ui, header.id, &lane_author, lane.label());

            for block in &lane.blocks {
                self.card(ui, palette, &lane.key, block);
            }
        });

        // A drop landed on this lane. Reject it while a mutation is in flight (RISK-3 / MC-3) or when
        // the source lane is this same lane (a no-op move). Otherwise emit the (block, from, to) tuple;
        // the host performs the REAL updateLoomBlock + re-query (never a local lane mutation).
        if let Some(state) = payload {
            if !self.in_flight && state.from_lane_key != lane.key {
                outcome = Some((
                    state.block_id.clone(),
                    state.from_lane_key.clone(),
                    lane.key.clone(),
                ));
            }
            // The drop consumed the drag — clear the transient drag state regardless of accept/reject.
            *self.drag = None;
        }
        outcome
    }

    /// Render one draggable card (`bcv.kanban.card.{block_id}`). Drag start records the
    /// [`KanbanDragState`] into both the egui DragAndDrop payload store (so the drop zone reads it) and
    /// the host's `drag` field (so a busy-indicator can show). Dragging is disabled while in flight.
    fn card(&mut self, ui: &mut egui::Ui, palette: &HsPalette, lane_key: &str, block: &LoomBlockRow) {
        let author_id = kanban_card_author_id(&block.block_id);
        let id = egui::Id::new(&author_id);
        let label = block.display_title().to_owned();

        let state = KanbanDragState {
            block_id: block.block_id.clone(),
            from_lane_key: lane_key.to_owned(),
        };

        // egui's dnd_drag_source makes the card a drag source carrying the typed payload. While a
        // mutation is in flight we still render the card but do not start a new drag (busy guard).
        let resp = ui
            .dnd_drag_source(id, state.clone(), |ui| {
                let (rect, r) = ui.allocate_exact_size(
                    egui::vec2(168.0, 28.0),
                    egui::Sense::click_and_drag(),
                );
                if ui.is_rect_visible(rect) {
                    ui.painter().rect_filled(rect, 4.0, palette.surface_strong);
                    ui.painter().text(
                        egui::pos2(rect.left() + 6.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        &label,
                        egui::FontId::proportional(13.0),
                        palette.text,
                    );
                }
                r
            })
            .response;

        if resp.drag_started() && !self.in_flight {
            *self.drag = Some(state);
        }

        let label_for_node = label.clone();
        ui.ctx().accesskit_node_builder(id, move |node| {
            node.set_role(accesskit::Role::ListItem);
            node.set_author_id(author_id.clone());
            node.set_label(label_for_node.clone());
        });
    }
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// CalendarSubView — buckets REAL query results by a date field. The date window is server-side; this
// only groups the rows it was given (parity with the React projection).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// The calendar sub-view: borrows the definition (date field) + results (rows) for one frame.
pub struct CalendarSubView<'a> {
    pub definition: &'a BlockViewDefinition,
    pub results: &'a BlockViewResults,
}

impl CalendarSubView<'_> {
    /// Render the day buckets (each `bcv.calendar.day.{date}`) with their entries
    /// (`bcv.calendar.entry.{block_id}`). Buckets are ordered by their key (React `orderedKeys.sort()`).
    /// This NEVER filters the rows — the date window is applied server-side; the widget only groups.
    pub fn show(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        let buckets = self.buckets();
        if buckets.is_empty() {
            ui.weak("No blocks in this date range.");
            return;
        }
        ui.horizontal_top(|ui| {
            for (key, blocks) in &buckets {
                let day_author = calendar_day_author_id(key);
                let frame = egui::Frame::group(ui.style()).fill(palette.surface);
                frame.show(ui, |ui| {
                    ui.set_min_width(150.0);
                    let header = ui.heading(key);
                    emit_day_node(ui, header.id, &day_author, key);
                    for block in blocks {
                        let entry_author = calendar_entry_author_id(&block.block_id);
                        let entry_id = egui::Id::new(&entry_author);
                        let label = block.display_title().to_owned();
                        let resp = ui.label(&label);
                        let _ = resp;
                        let label_for_node = label.clone();
                        ui.ctx().accesskit_node_builder(entry_id, move |node| {
                            node.set_role(accesskit::Role::ListItem);
                            node.set_author_id(entry_author.clone());
                            node.set_label(label_for_node.clone());
                        });
                    }
                });
            }
        });
    }

    /// Group the result rows into `(date_key, blocks)` buckets, ordered by date key. The bucket key is
    /// [`bucket_key`] (the React `bucketKey`); the order is the sorted key set (React `orderedKeys`).
    fn buckets(&self) -> Vec<(String, Vec<&LoomBlockRow>)> {
        use std::collections::BTreeMap;
        let field = self.definition.calendar_date_field;
        let mut map: BTreeMap<String, Vec<&LoomBlockRow>> = BTreeMap::new();
        for block in &self.results.blocks {
            map.entry(bucket_key(block, field)).or_default().push(block);
        }
        map.into_iter().collect()
    }
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// AccessKit node emitters (live nodes through egui's own hook — HBR-SWARM).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Emit a button/switcher AccessKit node (Role::Button + Action::Click + author_id; `selected` marks
/// the active kind/tab via the toggled state).
fn emit_button_node(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str, selected: bool) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.add_action(accesskit::Action::Click);
        if selected {
            node.set_toggled(accesskit::Toggled::True);
        }
    });
}

/// Emit a Role::Status AccessKit node (the status strip — "Re-sorting…" / "Moving card…" / "").
fn emit_status_node(ui: &egui::Ui, id: egui::Id, author_id: &str, value: &str) {
    let author = author_id.to_owned();
    let value = value.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Status);
        node.set_author_id(author.clone());
        node.set_value(value.clone());
        node.set_label(value.clone());
    });
}

/// Emit a text field's AccessKit node (Role::TextInput + author_id + current value) so a swarm agent
/// can type into the new-view title or the calendar date inputs.
fn emit_text_field_node(ui: &egui::Ui, id: egui::Id, author_id: &str, value: &str) {
    let author = author_id.to_owned();
    let value = value.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::TextInput);
        node.set_author_id(author.clone());
        node.set_value(value.clone());
    });
}

/// Emit a Kanban lane header AccessKit node (Role::Group + author_id + label).
fn emit_lane_node(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Group);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
    });
}

/// Emit a calendar day-header AccessKit node (Role::Group + author_id + the date key label).
fn emit_day_node(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Group);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, created: &str, updated: &str, journal: Option<&str>) -> LoomBlockRow {
        LoomBlockRow {
            block_id: id.to_owned(),
            title: Some(format!("Title {id}")),
            original_filename: None,
            content_type: "note".to_owned(),
            journal_date: journal.map(ToOwned::to_owned),
            created_at: created.to_owned(),
            updated_at: updated.to_owned(),
            pinned: false,
            favorite: false,
            backlink_count: 0,
            mention_count: 0,
            tag_count: 0,
        }
    }

    // ── PROOF1(a): flipDirection asc/desc toggle logic ───────────────────────────────────────────────

    #[test]
    fn flip_direction_same_field_toggles_asc_desc() {
        // No current sort -> new field -> asc.
        let s = flip_direction(None, BlockViewField::Title);
        assert_eq!(s, BlockViewSort { field: BlockViewField::Title, direction: BlockViewSortDirection::Asc });

        // Same field currently asc -> desc.
        let s = flip_direction(Some(s), BlockViewField::Title);
        assert_eq!(s.direction, BlockViewSortDirection::Desc, "same field asc -> desc");

        // Same field currently desc -> asc.
        let s = flip_direction(Some(s), BlockViewField::Title);
        assert_eq!(s.direction, BlockViewSortDirection::Asc, "same field desc -> asc");
    }

    #[test]
    fn flip_direction_new_field_is_asc() {
        let current = Some(BlockViewSort { field: BlockViewField::Title, direction: BlockViewSortDirection::Desc });
        // A DIFFERENT field always starts at asc, regardless of the prior direction (React rule).
        let s = flip_direction(current, BlockViewField::Updated);
        assert_eq!(s, BlockViewSort { field: BlockViewField::Updated, direction: BlockViewSortDirection::Asc });
    }

    // ── PROOF1(b): bucketKey for all three date-field variants ───────────────────────────────────────

    #[test]
    fn bucket_key_journal_date_uses_value_or_undated() {
        let b = row("blk-1", "2026-01-01T10:00:00Z", "2026-02-02T11:00:00Z", Some("2026-03-15"));
        assert_eq!(bucket_key(&b, Some(BlockViewField::JournalDate)), "2026-03-15");
        // Missing journal_date -> "undated".
        let b2 = row("blk-2", "2026-01-01T10:00:00Z", "2026-02-02T11:00:00Z", None);
        assert_eq!(bucket_key(&b2, Some(BlockViewField::JournalDate)), "undated");
        // Empty journal_date string also -> "undated".
        let b3 = row("blk-3", "2026-01-01T10:00:00Z", "2026-02-02T11:00:00Z", Some(""));
        assert_eq!(bucket_key(&b3, Some(BlockViewField::JournalDate)), "undated");
    }

    #[test]
    fn bucket_key_updated_and_created_slice_iso_date() {
        let b = row("blk-1", "2026-01-01T10:00:00Z", "2026-02-02T11:00:00Z", None);
        // updated -> updated_at[0..10]
        assert_eq!(bucket_key(&b, Some(BlockViewField::Updated)), "2026-02-02");
        // created -> created_at[0..10]
        assert_eq!(bucket_key(&b, Some(BlockViewField::Created)), "2026-01-01");
        // default (None) -> created_at[0..10]
        assert_eq!(bucket_key(&b, None), "2026-01-01");
        // any other field (e.g. Title) -> created_at[0..10] (React `default` branch)
        assert_eq!(bucket_key(&b, Some(BlockViewField::Title)), "2026-01-01");
    }

    // ── PROOF1(c): BLOCK_VIEW_UNTAGGED_LANE sentinel handling in add_tags/remove_tags ────────────────

    #[test]
    fn untagged_sentinel_value_matches_backend() {
        // The const MUST equal the backend/api.ts value EXACTLY (RISK-4 / MC-4).
        assert_eq!(BLOCK_VIEW_UNTAGGED_LANE, "__untagged__");
    }

    #[test]
    fn card_move_tags_handle_untagged_lane() {
        // tag-a -> tag-b: add [tag-b], remove [tag-a].
        let (add, remove) = card_move_tags("tag-a", "tag-b");
        assert_eq!(add, vec!["tag-b".to_owned()]);
        assert_eq!(remove, vec!["tag-a".to_owned()]);

        // FROM untagged -> tag-b: add [tag-b], remove [] (no tag to remove).
        let (add, remove) = card_move_tags(BLOCK_VIEW_UNTAGGED_LANE, "tag-b");
        assert_eq!(add, vec!["tag-b".to_owned()]);
        assert!(remove.is_empty(), "moving FROM untagged removes no tag");

        // tag-a -> untagged: add [] (no tag to add), remove [tag-a].
        let (add, remove) = card_move_tags("tag-a", BLOCK_VIEW_UNTAGGED_LANE);
        assert!(add.is_empty(), "moving TO untagged adds no tag");
        assert_eq!(remove, vec!["tag-a".to_owned()]);
    }

    // ── cellValue parity (React LoomTableView.cellValue) ─────────────────────────────────────────────

    #[test]
    fn cell_value_matches_react() {
        let mut b = row("blk-9", "2026-01-01T00:00:00Z", "2026-02-02T00:00:00Z", Some("2026-03-03"));
        b.title = None;
        b.original_filename = Some("file.md".to_owned());
        // title falls back to original_filename then block_id.
        assert_eq!(b.cell_value(BlockViewField::Title), "file.md");
        b.original_filename = None;
        assert_eq!(b.cell_value(BlockViewField::Title), "blk-9");
        // booleans render yes/no.
        b.pinned = true;
        assert_eq!(b.cell_value(BlockViewField::Pinned), "yes");
        assert_eq!(b.cell_value(BlockViewField::Favorite), "no");
        // counts render as numbers.
        b.backlink_count = 3;
        b.tag_count = 5;
        assert_eq!(b.cell_value(BlockViewField::BacklinkCount), "3");
        assert_eq!(b.cell_value(BlockViewField::TagCount), "5");
        assert_eq!(b.cell_value(BlockViewField::ContentType), "note");
        assert_eq!(b.cell_value(BlockViewField::JournalDate), "2026-03-03");
    }

    // ── lane label + effective columns parity ────────────────────────────────────────────────────────

    #[test]
    fn lane_label_untagged_else_key() {
        let untagged = BlockViewLane { key: BLOCK_VIEW_UNTAGGED_LANE.to_owned(), blocks: vec![] };
        assert_eq!(untagged.label(), "Untagged");
        let tag = BlockViewLane { key: "tag-a".to_owned(), blocks: vec![] };
        assert_eq!(tag.label(), "tag-a");
    }

    #[test]
    fn effective_columns_default_is_title_updated() {
        let def = BlockViewDefinition::of_kind(BlockViewKind::Table);
        assert_eq!(def.effective_columns(), vec![BlockViewField::Title, BlockViewField::Updated]);
        let mut def2 = def;
        def2.columns = vec![BlockViewField::ContentType];
        assert_eq!(def2.effective_columns(), vec![BlockViewField::ContentType]);
    }

    // ── RISK-5 / MC-5: date validation ───────────────────────────────────────────────────────────────

    #[test]
    fn iso_date_regex_validation() {
        assert!(is_iso_date("2026-06-21"));
        assert!(!is_iso_date("2026-6-21"), "single-digit month must fail");
        assert!(!is_iso_date("2026/06/21"), "slashes must fail");
        assert!(!is_iso_date("2026-06-21T00:00:00Z"), "datetime must fail");
        assert!(!is_iso_date("nonsense"));
        assert!(!is_iso_date(""), "empty handled by caller as a cleared bound, not here");
    }

    #[test]
    fn validated_date_range_empty_is_cleared_bound() {
        let v = BlockCollectionView::new("ws", "view-1");
        // Both inputs empty -> both bounds cleared (None), no error.
        let r = v.validated_date_range().expect("empty inputs are valid (cleared bounds)");
        assert_eq!(r, (None, None));
    }

    #[test]
    fn validated_date_range_rejects_bad_shape() {
        let mut v = BlockCollectionView::new("ws", "view-1");
        v.date_from_input = "2026/01/01".to_owned();
        assert!(v.validated_date_range().is_err(), "bad 'from' shape must be rejected (RISK-5)");
        v.date_from_input = "2026-01-01".to_owned();
        v.date_to_input = "bad".to_owned();
        assert!(v.validated_date_range().is_err(), "bad 'to' shape must be rejected");
        v.date_to_input = "2026-12-31".to_owned();
        let r = v.validated_date_range().expect("both valid");
        assert_eq!(r, (Some("2026-01-01".to_owned()), Some("2026-12-31".to_owned())));
    }

    // ── kind/field wire round-trips ──────────────────────────────────────────────────────────────────

    #[test]
    fn kind_wire_round_trip() {
        for k in [BlockViewKind::Table, BlockViewKind::Kanban, BlockViewKind::Calendar] {
            assert_eq!(BlockViewKind::parse_str(k.as_str()), k);
        }
        // unknown -> table (never panic on a backend addition).
        assert_eq!(BlockViewKind::parse_str("mystery"), BlockViewKind::Table);
    }

    #[test]
    fn field_wire_round_trip() {
        for f in [
            BlockViewField::Title,
            BlockViewField::Created,
            BlockViewField::Updated,
            BlockViewField::JournalDate,
            BlockViewField::ContentType,
            BlockViewField::Pinned,
            BlockViewField::Favorite,
            BlockViewField::BacklinkCount,
            BlockViewField::MentionCount,
            BlockViewField::TagCount,
        ] {
            assert_eq!(BlockViewField::parse_str(f.as_str()), Some(f));
        }
        assert_eq!(BlockViewField::parse_str("nope"), None);
    }

    // ── author_id sanitization (id integrity) ────────────────────────────────────────────────────────

    #[test]
    fn author_ids_are_sanitized() {
        let row_id = table_row_author_id("ws:1/blk 7#x");
        assert!(row_id.starts_with(TABLE_ROW_AUTHOR_ID_PREFIX));
        let suffix = &row_id[TABLE_ROW_AUTHOR_ID_PREFIX.len()..];
        assert!(
            suffix.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "row author_id suffix must be [a-z0-9-]; got '{suffix}'"
        );
        // The sort author id uses the fixed enum string.
        assert_eq!(table_sort_author_id(BlockViewField::Title), "bcv.table.sort.title");
        // A lane key with special chars sanitizes too.
        let lane_id = kanban_lane_author_id("Tag/With:Colon");
        assert!(lane_id.starts_with(KANBAN_LANE_AUTHOR_ID_PREFIX));
    }

    // ── RED-TEAM CONTROL: the drag payload must be Send + Sync + 'static for egui's DragAndDrop store ──

    #[test]
    fn kanban_drag_state_is_send_sync_static() {
        fn assert_send_sync_static<T: Send + Sync + 'static>() {}
        assert_send_sync_static::<KanbanDragState>();
    }

    // ── AC10 parity: an empty result set yields empty buckets / no lanes / no rows, no panic ─────────

    #[test]
    fn calendar_buckets_empty_result_is_empty() {
        let def = BlockViewDefinition::of_kind(BlockViewKind::Calendar);
        let results = BlockViewResults::default();
        let cal = CalendarSubView { definition: &def, results: &results };
        assert!(cal.buckets().is_empty(), "AC10: empty result -> no buckets, no panic");
    }

    #[test]
    fn calendar_buckets_group_and_order_by_date() {
        let mut def = BlockViewDefinition::of_kind(BlockViewKind::Calendar);
        def.calendar_date_field = Some(BlockViewField::JournalDate);
        let results = BlockViewResults {
            kind_str: "calendar".to_owned(),
            blocks: vec![
                row("b1", "2026-01-01T00:00:00Z", "2026-01-01T00:00:00Z", Some("2026-03-02")),
                row("b2", "2026-01-01T00:00:00Z", "2026-01-01T00:00:00Z", Some("2026-03-01")),
                row("b3", "2026-01-01T00:00:00Z", "2026-01-01T00:00:00Z", Some("2026-03-01")),
            ],
            groups: vec![],
            total_returned: 3,
        };
        let cal = CalendarSubView { definition: &def, results: &results };
        let buckets = cal.buckets();
        // Two distinct day buckets, ordered ascending; the 2026-03-01 bucket holds 2 entries.
        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets[0].0, "2026-03-01");
        assert_eq!(buckets[0].1.len(), 2);
        assert_eq!(buckets[1].0, "2026-03-02");
        assert_eq!(buckets[1].1.len(), 1);
    }
}
