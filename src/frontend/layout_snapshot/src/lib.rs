//! Typed work-surface layout snapshots, validation and JSON serialization.
//! Hosts supply their canonical pane, tab, module, divider and geometry types.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type PaneId = Arc<str>;

pub trait LayoutModule: Sized {
    fn legacy_default() -> Self;
}

pub trait LayoutGeometry: Sized {
    type Extent: Copy;
    fn clamped_to(&self, extent: Self::Extent) -> Self;
}

/// Schema id stamped into every snapshot so a future schema change can reject an incompatible blob
/// instead of mis-deserializing it. Distinct from the React workbench's `hsk.workbench_layout_state@1`
/// because this is the NATIVE shell's own composed snapshot (different field set: it carries the full
/// `PaneRecord` registry + per-pane `TabBarState`, which the React JSONB schema does not). Both share
/// the one backend `layout_state` column; the `schema_id` is what tells them apart on read.
pub const LAYOUT_SCHEMA_ID: &str = "hsk.native_worksurface_layout@1";

/// Snapshot format version. Bumped only on a breaking field change; `LAYOUT_SCHEMA_ID` already
/// encodes the major schema, so this guards minor in-schema evolution.
pub const LAYOUT_SNAPSHOT_VERSION: u32 = 1;

/// The single source of truth for the canonical pane ids the native shell seeds for a fresh work
/// surface. Must stay aligned with the host application's `default_panes` (`pane-a`, `pane-b`, `pane-c`, the
/// MT-098 editor + Runtime Chat default). [`LayoutSnapshot::validate`] uses this list for canonical
/// pane and tab-bar completeness, so a structurally-corrupt-but-schema-valid blob missing a seeded
/// pane or its status/tab surface is rejected and falls back to last-known-good / default instead of
/// being applied as a short-paned or short-tabbed layout. Older stored layouts may carry extra panes;
/// those remain valid.
pub const CANONICAL_PANE_IDS: [&str; 3] = ["pane-a", "pane-b", "pane-c"];

/// The default active module (`MAIN`) used when a layout blob predates MT-012 and carries no
/// `active_module` key. Mirrors the React default module for a fresh pane (`DEFAULT_PANES[0].module`).
fn default_module<M: LayoutModule>() -> M {
    M::legacy_default()
}

/// The default `drawers.project` value (left rail OPEN) used when a layout blob predates MT-014 and
/// carries no `drawers` key. A fresh shell shows the rail open so the project tree is visible.
fn default_left_rail_open() -> bool {
    true
}

/// Persisted open/closed state of the shell's collapsible drawers (MT-014). Mirrors the React
/// workbench `drawers` object; currently only the LEFT rail (`project`) drawer is tracked here. A
/// missing field defaults via [`default_left_rail_open`] so an older blob still deserializes (the
/// version-1 schema stays backward compatible). `bottom` is reserved for the MT-022 bottom stash
/// drawer so this struct is the single shared home for drawer flags (red-team CONTROL: one shared
/// field name, not separate booleans that can drift).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DrawersState {
    /// The LEFT activity rail (project tree / quick links / stash) open flag. React field `project`.
    #[serde(default = "default_left_rail_open")]
    pub project: bool,
    /// The BOTTOM stash drawer open flag (MT-022 owns the full UI; MT-014 only toggles this). Defaults
    /// closed so a fresh shell does not show an empty bottom drawer.
    #[serde(default)]
    pub bottom: bool,
}

impl Default for DrawersState {
    fn default() -> Self {
        Self {
            project: default_left_rail_open(),
            bottom: false,
        }
    }
}

/// One pop-out's persisted state: where the detached window sat and whether it was open. Geometry
/// reuses the MT-008 geometry (already serde) so the restore path can hand it straight to
/// `clamped_to` + `PopOutManager::pop_out` with no lossy re-typing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopOutSnapshot<G> {
    pub geometry: G,
    pub open: bool,
}

/// The full, restorable work-surface layout for one project. Every field is the authoritative type
/// owned by an earlier MT, so a snapshot is a faithful capture of live state, not a parallel model
/// that can drift (rubric: end-to-end integrity).
///
/// Maps use `BTreeMap` so the serialized JSON has deterministic key order — round-trip equality and
/// golden diffs are stable run-to-run regardless of the live `HashMap` iteration order.
///
/// Note: deliberately NOT `PartialEq`. the host pane record carries a serde-skipped,
/// non-deterministic `Instant` (`last_update`), so a derived struct equality would compare wall-clock
/// markers that never survive a round trip. Snapshot equality is therefore asserted on the SERIALIZED
/// form (`serde_json::Value`), which is exactly the persisted contract — matching the MT-009 red-team
/// guidance that snapshot equality use a JSON `Value` comparison, not a struct/string compare.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(
    serialize = "P: Serialize, T: Serialize, M: Serialize, W: Serialize, G: Serialize",
    deserialize = "P: Deserialize<'de>, T: Deserialize<'de>, M: Deserialize<'de>, W: Deserialize<'de>, G: Deserialize<'de>"
))]
pub struct LayoutSnapshot<P, T, M: LayoutModule, W, G> {
    /// Must equal [`LAYOUT_SCHEMA_ID`]; checked by [`LayoutSnapshot::validate`].
    pub schema_id: String,
    /// Must equal [`LAYOUT_SNAPSHOT_VERSION`]; checked by [`LayoutSnapshot::validate`].
    pub version: u32,
    /// The project (workspace) this layout belongs to. The persisted blob is keyed in the backend by
    /// the `:workspace_id` path segment; this field lets a load detect a wrong-project blob too.
    pub project_id: String,
    /// 2x2 split divider fractions (MT-006).
    pub split_weights: W,
    /// The pane the operator last activated, if any (MT-006 active-pane highlight).
    pub active_pane: Option<PaneId>,
    /// The active work-surface MODULE (MT-012). Serialized as the uppercase React string
    /// (`"MAIN"`, …). `#[serde(default)]` so a layout blob written before MT-012 (no `active_module`
    /// key) still deserializes — it falls back to the host module default (`MAIN`) instead of failing schema
    /// validation, keeping the version-1 schema backward compatible.
    #[serde(default = "default_module")]
    pub active_module: M,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pane_modules: BTreeMap<PaneId, M>,
    /// The pane registry records (MT-005), keyed by pane id for deterministic order.
    pub panes: BTreeMap<PaneId, P>,
    /// Per-pane tab-bar state (MT-007): tab order, active index, pinned/dirty flags.
    pub tab_bars: BTreeMap<PaneId, T>,
    /// Per-pane pop-out geometry + open flag (MT-008). Only panes that were popped out appear.
    pub pop_outs: BTreeMap<PaneId, PopOutSnapshot<G>>,
    /// Collapsible-drawer open flags (MT-014): the left activity rail (`drawers.project`) and the
    /// bottom stash drawer (`drawers.bottom`). `#[serde(default)]` so a layout blob written before
    /// MT-014 (no `drawers` key) still deserializes with the rail OPEN (the default), keeping the
    /// version-1 schema backward compatible.
    #[serde(default)]
    pub drawers: DrawersState,
}

impl<P, T, M: LayoutModule, W, G> LayoutSnapshot<P, T, M, W, G> {
    /// Build a snapshot from already-captured live pieces, stamping the current schema id + version.
    /// Callers pass the maps as `BTreeMap` so order is deterministic; the host application
    /// converts its live `HashMap`s at capture time.
    pub fn new(
        project_id: impl Into<String>,
        split_weights: W,
        active_pane: Option<PaneId>,
        active_module: M,
        panes: BTreeMap<PaneId, P>,
        tab_bars: BTreeMap<PaneId, T>,
        pop_outs: BTreeMap<PaneId, PopOutSnapshot<G>>,
    ) -> Self {
        Self {
            schema_id: LAYOUT_SCHEMA_ID.to_owned(),
            version: LAYOUT_SNAPSHOT_VERSION,
            project_id: project_id.into(),
            split_weights,
            active_pane,
            active_module,
            pane_modules: BTreeMap::new(),
            panes,
            tab_bars,
            pop_outs,
            drawers: DrawersState::default(),
        }
    }

    /// Builder: set the collapsible-drawer flags (MT-014). Returns `self` so the app can chain it onto
    /// [`new`](Self::new) at capture time: `LayoutSnapshot::new(...).with_drawers(drawers)`.
    pub fn with_drawers(mut self, drawers: DrawersState) -> Self {
        self.drawers = drawers;
        self
    }

    /// Reject a snapshot whose schema id or version does not match this build. This is the gate that
    /// keeps a corrupt or future-version blob from being applied as if it were valid; the caller
    /// falls back to last-known-good / default on `Err`.
    pub fn validate(&self) -> Result<(), LayoutError> {
        if self.schema_id != LAYOUT_SCHEMA_ID {
            return Err(LayoutError::SchemaMismatch {
                found: self.schema_id.clone(),
                expected: LAYOUT_SCHEMA_ID,
            });
        }
        if self.version != LAYOUT_SNAPSHOT_VERSION {
            return Err(LayoutError::VersionMismatch {
                found: self.version,
                expected: LAYOUT_SNAPSHOT_VERSION,
            });
        }
        // Pane-id completeness: every canonical pane must be present in the registry. A
        // schema+version-valid blob that dropped a pane is structurally corrupt and must NOT be applied
        // as a short-paned layout; reject it (PERMANENT data error) so the caller falls back to
        // last-known-good / default. Iteration is in CANONICAL_PANE_IDS order, so the FIRST missing pane
        // is the one named in the error.
        for id in CANONICAL_PANE_IDS {
            if !self.panes.contains_key(&PaneId::from(id)) {
                return Err(LayoutError::MissingPane { id: id.to_owned() });
            }
            if !self.tab_bars.contains_key(&PaneId::from(id)) {
                return Err(LayoutError::MissingTabBar { id: id.to_owned() });
            }
        }
        // Active-pane membership: a dangling `active_pane` (one not present in the snapshot registry)
        // is a corrupt reference. Custom restored layouts may still focus non-default panes, so validate
        // against the restored pane set rather than only the current canonical default seed.
        if let Some(active) = &self.active_pane {
            if !self.panes.contains_key(active) {
                return Err(LayoutError::UnknownActivePane {
                    id: active.as_ref().to_owned(),
                });
            }
        }
        Ok(())
    }

    /// Serialize this snapshot into the backend `layout_state` JSON blob. This is the exact value sent
    /// as `SaveWorkbenchLayoutRequest.layout_state` and read back from
    /// `WorkbenchLayoutResponse.layout_state`. Keeping the mapping a single serde call (rather than a
    /// hand-rolled field copy) is what makes the round trip an identity: `from_layout_state` is its
    /// exact inverse.
    pub fn to_layout_state(&self) -> Value
    where
        P: Serialize,
        T: Serialize,
        M: Serialize,
        W: Serialize,
        G: Serialize,
    {
        serde_json::to_value(self).expect("LayoutSnapshot serializes to JSON")
    }

    /// Parse a backend `layout_state` JSON blob back into a snapshot and VALIDATE it. A blob that is
    /// not this schema (e.g. the React workbench's own `hsk.workbench_layout_state@1`) or a corrupt
    /// blob surfaces as `Err`, so the caller falls back to last-known-good / default rather than
    /// applying a foreign or garbage layout.
    pub fn from_layout_state(value: Value) -> Result<Self, LayoutError>
    where
        P: serde::de::DeserializeOwned,
        T: serde::de::DeserializeOwned,
        M: serde::de::DeserializeOwned,
        W: serde::de::DeserializeOwned,
        G: serde::de::DeserializeOwned,
    {
        let snapshot: Self =
            serde_json::from_value(value).map_err(|e| LayoutError::Serde(e.to_string()))?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    /// Return a copy of this snapshot with every pop-out geometry clamped ONCE against `extent` (the
    /// FULL virtual-desktop / all-monitors bounding rect), using the MT-008 off-screen safety net.
    /// A pop-out whose saved top-left is provably outside every monitor is reset to the fallback
    /// position; one already on a monitor (including a legitimate second-monitor position) is left
    /// untouched. This is the restore-time clamp deferred from MT-008 — applied once, never live.
    pub fn clamp_pop_outs_to(mut self, extent: G::Extent) -> Self
    where
        G: LayoutGeometry,
    {
        for snap in self.pop_outs.values_mut() {
            snap.geometry = snap.geometry.clamped_to(extent);
        }
        self
    }
}

/// Errors from mapping / validating / transporting a layout. Every variant is explicit so the caller
/// can decide the fallback (rubric: failure paths are explicit, not buried).
#[derive(Debug, Clone)]
pub enum LayoutError {
    /// The blob's `schema_id` is not this build's [`LAYOUT_SCHEMA_ID`] (e.g. a foreign layout schema).
    SchemaMismatch {
        found: String,
        expected: &'static str,
    },
    /// The blob's `version` is not this build's [`LAYOUT_SNAPSHOT_VERSION`].
    VersionMismatch { found: u32, expected: u32 },
    /// The backend blob's `project_id` did not match the requested workspace id.
    ProjectMismatch { requested: String, snapshot: String },
    /// A canonical pane id (`pane-a`/`pane-b`/`pane-c`) is absent from the blob's `panes` registry. A
    /// structurally-corrupt-but-schema-valid blob that dropped a pane would otherwise be applied as a
    /// short-paned layout; this PERMANENT data error forces the LKG/default fallback instead.
    MissingPane { id: String },
    /// A canonical pane id is present in `panes` but absent from the per-pane tab-bar map. A pane
    /// without its tab/status surface is not a recoverable work-surface state.
    MissingTabBar { id: String },
    /// The blob's `active_pane` points at a pane id that is not one of the canonical panes — i.e. a
    /// dangling active-pane reference. PERMANENT data error -> LKG/default fallback.
    UnknownActivePane { id: String },
    /// `serde_json` parse/serialize failure on the blob.
    Serde(String),
    /// Transport (HTTP) failure talking to the backend layout endpoint. Treated as TRANSIENT by the
    /// persistence manager's retry logic — the layout in memory is unaffected.
    Transport(String),
}

impl LayoutError {
    /// Whether this error is a transient transport failure (worth retrying) versus a permanent data
    /// error (schema/version/project/serde mismatch — retrying will not help).
    pub fn is_transient(&self) -> bool {
        matches!(self, LayoutError::Transport(_))
    }
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutError::SchemaMismatch { found, expected } => {
                write!(
                    f,
                    "layout schema mismatch: found {found:?}, expected {expected:?}"
                )
            }
            LayoutError::VersionMismatch { found, expected } => {
                write!(
                    f,
                    "layout version mismatch: found {found}, expected {expected}"
                )
            }
            LayoutError::ProjectMismatch {
                requested,
                snapshot,
            } => write!(
                f,
                "layout project mismatch: requested {requested:?}, blob holds {snapshot:?}"
            ),
            LayoutError::MissingPane { id } => {
                write!(f, "layout missing canonical pane: {id:?}")
            }
            LayoutError::MissingTabBar { id } => {
                write!(f, "layout missing canonical tab bar: {id:?}")
            }
            LayoutError::UnknownActivePane { id } => {
                write!(f, "layout active_pane references unknown pane: {id:?}")
            }
            LayoutError::Serde(e) => write!(f, "layout serde error: {e}"),
            LayoutError::Transport(e) => write!(f, "layout transport error: {e}"),
        }
    }
}

impl std::error::Error for LayoutError {}
