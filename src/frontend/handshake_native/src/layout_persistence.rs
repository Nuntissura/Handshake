//! Per-project work-surface layout persistence (WP-KERNEL-011 MT-009).
//!
//! Serializes and restores the FULL work-surface layout for a project so the operator's arrangement
//! survives an app restart. A snapshot composes the authoritative state already owned by the earlier
//! C2 MTs, never a re-typed shadow copy of it:
//!
//! - [`crate::split_layout::SplitWeights`] — the 2x2 divider fractions (MT-006);
//! - per-pane [`crate::tab_bar::TabBarState`] — tab order, active tab, pinned/dirty flags (MT-007);
//! - pop-out geometry + open state per pane (MT-008);
//! - the [`crate::pane_registry::PaneRecord`] set — the pane registry source of truth (MT-005);
//! - the active pane id.
//!
//! ## Storage backend (CX-503S / Data Posture — SurrealDB-authoritative, NO local-file authority)
//!
//! The native layout is **SurrealDB/EventLedger-authoritative**. The running `handshake_core`
//! backend already owns a durable workbench-layout surface in its embedded SurrealDB store (RocksDB
//! engine, namespace `handshake` / database `primary`, opened in-process at startup), exposed over
//! its REST API as `GET`/`PUT /workspaces/:workspace_id/workbench/layout`. The native
//! shell persists the layout by sending the snapshot through that endpoint — there is NO local JSON
//! file, NO SQLite, and NO alternate on-disk authority. The backend stores the snapshot as an opaque
//! `layout_state` JSON value (and stamps an EventLedger event id), so the native shell's richer field
//! set (full `PaneRecord` registry + per-pane `TabBarState`) round-trips through the same endpoint the
//! React workbench uses, distinguished by this snapshot's own `schema_id`.
//!
//! The HTTP transport is abstracted behind [`LayoutTransport`] so the persistence state machine
//! ([`LayoutPersistenceManager`]) can be unit-tested by stubbing the send at a seam — no live server
//! is needed for the manager's debounce / retry / last-known-good / fallback logic. The real
//! transport is [`crate::backend_client::WorkbenchLayoutClient`].
//!
//! ## Restore-time geometry clamp (deferred from MT-008)
//!
//! A restored pop-out position is clamped **once** at restore time against the FULL monitor/desktop
//! extent, and only when the saved position is provably outside every monitor — via
//! [`crate::popout_window::PopOutGeometry::clamped_to`]. A live pop-out is never snapped (that
//! belongs to MT-008's `show_all`, which deliberately does not clamp). This is the MT-008 red-team
//! "off-screen geometry" control, applied at the one safe moment: reopening a saved layout.

#[cfg(test)]
use std::collections::BTreeMap;

use serde_json::Value;

use crate::module_switcher::ModuleId;
#[cfg(test)]
use crate::pane_registry::PaneId;
use crate::pane_registry::PaneRecord;
use crate::popout_window::PopOutGeometry;
use crate::split_layout::SplitWeights;
use crate::tab_bar::TabBarState;

pub use handshake_layout_snapshot::{
    CANONICAL_PANE_IDS, DrawersState, LAYOUT_SCHEMA_ID, LAYOUT_SNAPSHOT_VERSION, LayoutError,
};

pub type LayoutSnapshot = handshake_layout_snapshot::LayoutSnapshot<
    PaneRecord, TabBarState, ModuleId, SplitWeights, PopOutGeometry,
>;
pub type PopOutSnapshot = handshake_layout_snapshot::PopOutSnapshot<PopOutGeometry>;

impl handshake_layout_snapshot::LayoutModule for ModuleId {
    fn legacy_default() -> Self {
        Self::Main
    }
}

impl handshake_layout_snapshot::LayoutGeometry for PopOutGeometry {
    type Extent = egui::Rect;

    fn clamped_to(&self, extent: Self::Extent) -> Self {
        PopOutGeometry::clamped_to(*self, extent)
    }
}

/// Validate the native binding to the same pane/tab owners before transport or LKG admission.
pub fn validate_snapshot(snapshot:&LayoutSnapshot)->Result<(),LayoutError> {
    use handshake_layout_snapshot::{KumihimoLayoutState,KumihimoWindowId};
    snapshot.validate()?;
    let Some(KumihimoLayoutState::Captured {layout})=&snapshot.kumihimo else {return Ok(());};
    let core=layout.restore_core()?;
    let model=core.snapshot();
    let invalid=|message:&str|LayoutError::Serde(format!("Kumihimo host binding: {message}"));
    let mut viewports=std::collections::BTreeSet::new();
    for window in &layout.windows {
        let viewport=match &window.window {
            KumihimoWindowId::Root=>egui::ViewportId::ROOT,
            KumihimoWindowId::PopOut {pane_id}=>egui::ViewportId::from_hash_of(pane_id.as_ref()),
        };
        if !viewports.insert(viewport) {return Err(invalid("logical windows collide at the host viewport boundary"));}
    }
    for (pane,record) in &snapshot.panes {
        if record.pane_id!=*pane {
            return Err(invalid("pane registry key differs from its owner identity"));
        }
    }
    for window in &layout.windows {
        if let KumihimoWindowId::PopOut {pane_id}=&window.window {
            if !window.active.is_empty() && (window.active.as_slice()!=[pane_id.clone()]
                || !snapshot.panes.contains_key(pane_id)
                || !snapshot.pop_outs.get(pane_id).is_some_and(|popout|popout.open)) {
                return Err(invalid("active pop-out has no open native pane owner"));
            }
        }
        for pane in &window.active {
            let bar=snapshot.tab_bars.get(pane).ok_or_else(||invalid("pane has no tab owner"))?;
            if bar.pane_id!=*pane || (!bar.tabs.is_empty()&&bar.active_index>=bar.tabs.len()) {
                return Err(invalid("tab owner or active index is invalid"));
            }
            let frame=model.frames.get(window.frames.get(pane).ok_or_else(||invalid("pane frame binding is missing"))?)
                .ok_or_else(||invalid("pane frame is missing"))?;
            let mut expected=Vec::new();
            for tab in &bar.tabs {
                let key=serde_json::to_string(&(&tab.pane_type,&tab.content_id)).map_err(|error|invalid(&error.to_string()))?;
                let binding=layout.views.iter().find(|view|view.window==window.window
                    &&view.pane_id==*pane&&view.resource_key==key).ok_or_else(||invalid("tab has no canonical view binding"))?;
                let view=&model.views[&binding.view];
                if view.label!=tab.label()||view.pinned!=tab.pinned||view.unsaved!=tab.dirty {
                    return Err(invalid("view metadata differs from its native tab owner"));
                }
                expected.push(binding.view);
            }
            if frame.views!=expected || frame.selected_view!=expected.get(bar.active_index).copied() {
                return Err(invalid("view order/selection differs from the native tab owner"));
            }
        }
    }
    Ok(())
}

/// The synchronous transport seam for the layout endpoint. The real implementation
/// ([`crate::backend_client::WorkbenchLayoutClient`]) blocks on `reqwest` inside the app's tokio
/// runtime; a test stub implements this trait in-memory so the [`LayoutPersistenceManager`] state
/// machine (debounce / retry / LKG / fallback) is provable with NO live server.
///
/// Both methods speak the backend's JSON `layout_state` blob directly:
/// - `load` maps to `GET /workspaces/:id/workbench/layout`, returning `Ok(None)` when the backend has
///   no layout stored for the workspace yet (first run), `Ok(Some(value))` with the stored blob, or
///   `Err(Transport(..))` on an HTTP failure.
/// - `save` maps to `PUT /workspaces/:id/workbench/layout`, returning `Ok(())` on success or
///   `Err(Transport(..))` on an HTTP failure.
///
/// Bound `Send + Sync` so the [`LayoutPersistenceManager`] is itself `Send` and the app can run a
/// debounced flush off the egui UI thread (on a short-lived worker that bridges to the tokio runtime),
/// keeping the network off the UI thread (HBR-QUIET).
pub trait LayoutTransport: Send + Sync {
    /// `GET` the stored `layout_state` blob for `workspace_id`.
    fn load(&self, workspace_id: &str) -> Result<Option<Value>, LayoutError>;
    /// `PUT` the `layout_state` blob for `workspace_id`.
    fn save(&self, workspace_id: &str, layout_state: Value) -> Result<(), LayoutError>;
}

/// Status the UI can read about the last persistence operation (HBR: important state is visible).
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutPersistenceStatus {
    /// No save/load attempted yet.
    Idle,
    /// A save is debounced and pending flush.
    Pending,
    /// The last save/load succeeded.
    Saved,
    /// The last load applied a stored layout (vs. falling back to default).
    Loaded,
    /// The last load found no stored layout (first run for the project) — default in use.
    Default,
    /// The last operation failed; carries the rendered error and the remaining retry budget.
    Error { message: String, retries_left: u32 },
}

/// How many times a transient save failure is retried before the manager gives up for this flush and
/// surfaces an error status. Permanent (data) errors are NOT retried.
pub const SAVE_MAX_RETRIES: u32 = 3;

/// The persistence state machine: debounce-on-change, retry-on-transient-failure, an in-memory
/// last-known-good held after each successful load/save, and a status the UI can read.
///
/// The manager is transport-agnostic ([`LayoutTransport`]) and holds NO egui/app state, so it is a
/// pure, directly-drivable state machine — unit tests exercise the full debounce/retry/LKG/fallback
/// logic by ticking a logical clock and stubbing the transport, with no live backend and no egui
/// frame. The app owns one manager and feeds it real wall-clock instants + the real HTTP transport.
///
/// ## Why debounce
///
/// A split-divider drag or a tab reorder fires many layout-affecting changes per second. Saving on
/// every one would hammer the backend. `mark_dirty` records the change time; `due_to_flush` reports
/// when the quiet period has elapsed so rapid changes coalesce into one `PUT`.
pub struct LayoutPersistenceManager {
    transport: Box<dyn LayoutTransport + Send + Sync>,
    /// Quiet period: a flush is due once this much time has passed since the last `mark_dirty`.
    debounce: std::time::Duration,
    /// When the most recent layout-affecting change happened (the debounce anchor). `None` when clean.
    dirty_since: Option<std::time::Instant>,
    /// The most recently successfully loaded/saved snapshot, used as the fallback if a later load
    /// fails (rubric: failure paths are explicit; never apply a corrupt layout).
    last_known_good: Option<LayoutSnapshot>,
    status: LayoutPersistenceStatus,
}

impl LayoutPersistenceManager {
    /// Construct a manager over `transport` with the given debounce quiet period.
    pub fn new(
        transport: Box<dyn LayoutTransport + Send + Sync>,
        debounce: std::time::Duration,
    ) -> Self {
        Self {
            transport,
            debounce,
            dirty_since: None,
            last_known_good: None,
            status: LayoutPersistenceStatus::Idle,
        }
    }

    /// Current UI-readable status.
    pub fn status(&self) -> &LayoutPersistenceStatus {
        &self.status
    }

    /// The held last-known-good snapshot, if any.
    pub fn last_known_good(&self) -> Option<&LayoutSnapshot> {
        self.last_known_good.as_ref()
    }

    /// Whether a save is currently debounced and pending.
    pub fn is_dirty(&self) -> bool {
        self.dirty_since.is_some()
    }

    /// Record that the layout changed at `now`. Resets the debounce window so rapid successive changes
    /// coalesce into one flush. Sets the status to `Pending` so the UI shows an unsaved indicator.
    pub fn mark_dirty(&mut self, now: std::time::Instant) {
        self.dirty_since = Some(now);
        self.status = LayoutPersistenceStatus::Pending;
    }

    /// Whether a debounced flush is due at `now` (the quiet period elapsed since the last change).
    /// `false` when clean or still inside the debounce window.
    pub fn due_to_flush(&self, now: std::time::Instant) -> bool {
        match self.dirty_since {
            Some(since) => now.duration_since(since) >= self.debounce,
            None => false,
        }
    }

    /// Flush a debounced save if one is due: capture-and-save `snapshot` to the backend, retrying a
    /// transient transport failure up to [`SAVE_MAX_RETRIES`] times. A successful save clears the dirty
    /// flag, records `snapshot` as the new last-known-good, and sets status `Saved`. A permanent (data)
    /// error is not retried. Returns `true` if a flush was attempted (due), `false` if nothing was due.
    ///
    /// The caller passes the freshly-captured snapshot (the app's current layout) and the current
    /// `now`; the manager owns only the timing + retry + LKG bookkeeping, never the layout capture.
    pub fn flush_if_due(&mut self, now: std::time::Instant, snapshot: &LayoutSnapshot) -> bool {
        if !self.due_to_flush(now) {
            return false;
        }
        self.save_now(snapshot);
        true
    }

    /// Save `snapshot` immediately (bypassing the debounce window), retrying transient failures. Used
    /// for the flush-on-due path and for an explicit save-on-exit. Clears the dirty flag on success.
    pub fn save_now(&mut self, snapshot: &LayoutSnapshot) {
        if let Err(error)=validate_snapshot(snapshot) {self.reject_restore(error);return;}
        let layout_state = snapshot.to_layout_state();
        let mut attempt = 0u32;
        loop {
            match self
                .transport
                .save(&snapshot.project_id, layout_state.clone())
            {
                Ok(()) => {
                    self.dirty_since = None;
                    self.last_known_good = Some(snapshot.clone());
                    self.status = LayoutPersistenceStatus::Saved;
                    return;
                }
                Err(e) if e.is_transient() && attempt < SAVE_MAX_RETRIES => {
                    attempt += 1;
                    // Loop and retry. Real backoff/sleep is the caller's concern (the app schedules
                    // the next frame's flush); the manager retries within the call up to the budget so
                    // a brief blip self-heals without losing the dirty flag.
                    continue;
                }
                Err(e) => {
                    // Permanent error, or transient budget exhausted: surface it but KEEP the dirty
                    // flag set so a later flush tries again (data is not lost on a save failure).
                    self.status = LayoutPersistenceStatus::Error {
                        message: e.to_string(),
                        retries_left: SAVE_MAX_RETRIES.saturating_sub(attempt),
                    };
                    return;
                }
            }
        }
    }

    /// Load + validate the stored layout for `workspace_id`, applying the documented fallback chain:
    ///
    /// - backend has a valid blob -> `Ok(Some(snapshot))`, recorded as the new last-known-good,
    ///   status `Loaded`;
    /// - backend has no blob (first run) -> `Ok(None)`, status `Default` (caller keeps the seeded
    ///   default layout);
    /// - backend blob is corrupt / foreign-schema / wrong-project, OR the transport fails -> fall back
    ///   to the in-memory last-known-good if present (`Ok(Some(lkg))`), else `Ok(None)` with status
    ///   `Error`. This NEVER returns a snapshot that failed validation, so the caller can apply the
    ///   result unconditionally (no infinite restore loop: `validate` gates, fallback is infallible).
    ///
    /// The returned snapshot is NOT yet clamped; the caller clamps it once against the monitor extent
    /// when applying (the restore-time MT-008 clamp lives in `apply_layout_snapshot`).
    pub fn load(&mut self, workspace_id: &str) -> Result<Option<LayoutSnapshot>, LayoutError> {
        match self.transport.load(workspace_id) {
            Ok(Some(value)) => match LayoutSnapshot::from_layout_state(value) {
                Ok(snapshot) => {
                    if let Err(error)=validate_snapshot(&snapshot) {return Ok(self.fall_back_to_lkg(error,workspace_id));}
                    if snapshot.project_id != workspace_id {
                        return Ok(self.fall_back_to_lkg(LayoutError::ProjectMismatch {
                            requested: workspace_id.to_owned(),
                            snapshot: snapshot.project_id.clone(),
                        },workspace_id));
                    }
                    self.last_known_good = Some(snapshot.clone());
                    self.status = LayoutPersistenceStatus::Loaded;
                    Ok(Some(snapshot))
                }
                // Corrupt or foreign-schema blob: fall back to LKG -> default, never apply garbage.
                Err(e) => Ok(self.fall_back_to_lkg(e,workspace_id)),
            },
            Ok(None) => {
                self.status = LayoutPersistenceStatus::Default;
                Ok(None)
            }
            // Transport failure: fall back to LKG -> default.
            Err(e) => Ok(self.fall_back_to_lkg(e,workspace_id)),
        }
    }

    pub fn reject_restore(&mut self,cause:LayoutError) {
        self.status=LayoutPersistenceStatus::Error {message:cause.to_string(),retries_left:0};
    }

    /// Set an `Error` status carrying `cause` and return the last-known-good snapshot if held (else
    /// `None`, so the caller uses the seeded default). This is the single fallback decision point so a
    /// failed load can never apply an unvalidated snapshot.
    fn fall_back_to_lkg(&mut self, cause: LayoutError,workspace_id:&str) -> Option<LayoutSnapshot> {
        self.status = LayoutPersistenceStatus::Error {
            message: cause.to_string(),
            retries_left: 0,
        };
        self.last_known_good.as_ref().filter(|snapshot|snapshot.project_id==workspace_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_registry::{DirtyState, LockState, PaneAuthority, PaneRecord, PaneType};
    use crate::tab_bar::{TabBarState, TabState};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    fn pid(s: &str) -> PaneId {
        std::sync::Arc::from(s)
    }

    fn sample_record(id: &str, ty: PaneType) -> PaneRecord {
        PaneRecord::new(
            pid(id),
            ty,
            "proj-1",
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        )
    }

    /// A representative non-default snapshot: changed split weights, a multi-tab pinned pane, an
    /// active pane, and a popped-out pane with a specific geometry. Used by the round-trip tests so
    /// they prove EVERY composed MT's state survives the backend-JSON mapping, not just the default.
    fn sample_snapshot() -> LayoutSnapshot {
        // Seed the two current canonical panes plus two extra legacy panes. Validation requires
        // pane-a/pane-b and tolerates extra older-layout panes.
        let mut panes = BTreeMap::new();
        panes.insert(pid("pane-a"), sample_record("pane-a", PaneType::Workspace));
        panes.insert(
            pid("pane-b"),
            sample_record("pane-b", PaneType::InferenceLab),
        );
        panes.insert(
            pid("pane-c"),
            sample_record("pane-c", PaneType::MediaDownloader),
        );
        panes.insert(
            pid("pane-d"),
            sample_record("pane-d", PaneType::FontManager),
        );

        let mut tab_bars = BTreeMap::new();
        let mut bar_a = TabBarState::new(
            pid("pane-a"),
            vec![
                TabState::new(PaneType::Workspace),
                TabState::new(PaneType::Problems),
            ],
        );
        bar_a.tabs[1].pinned = true;
        bar_a.stabilize_pins();
        bar_a.active_index = 0;
        tab_bars.insert(pid("pane-a"), bar_a);
        tab_bars.insert(
            pid("pane-b"),
            TabBarState::new(pid("pane-b"), vec![TabState::new(PaneType::InferenceLab)]),
        );
        tab_bars.insert(
            pid("pane-c"),
            TabBarState::new(
                pid("pane-c"),
                vec![TabState::new(PaneType::MediaDownloader)],
            ),
        );
        tab_bars.insert(
            pid("pane-d"),
            TabBarState::new(pid("pane-d"), vec![TabState::new(PaneType::FontManager)]),
        );

        let mut pop_outs = BTreeMap::new();
        pop_outs.insert(
            pid("pane-b"),
            PopOutSnapshot {
                geometry: PopOutGeometry::at(egui::pos2(640.0, 480.0)),
                open: true,
            },
        );

        LayoutSnapshot::new(
            "proj-1",
            SplitWeights {
                vertical: 0.42,
                horizontal: 0.66,
            },
            Some(pid("pane-a")),
            ModuleId::Main,
            panes,
            tab_bars,
            pop_outs,
        )
    }

    // ── Snapshot validate + schema gates ────────────────────────────────────────────────────────

    #[test]
    fn snapshot_validates_when_schema_and_version_match() {
        assert!(sample_snapshot().validate().is_ok());
    }

    #[test]
    fn snapshot_rejects_wrong_schema_id() {
        let mut snap = sample_snapshot();
        snap.schema_id = "hsk.workbench_layout_state@1".to_owned(); // the React schema, not ours
        match snap.validate() {
            Err(LayoutError::SchemaMismatch { expected, .. }) => {
                assert_eq!(expected, LAYOUT_SCHEMA_ID)
            }
            other => panic!("expected SchemaMismatch, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_rejects_wrong_version() {
        let mut snap = sample_snapshot();
        snap.version = LAYOUT_SNAPSHOT_VERSION + 1;
        assert!(matches!(
            snap.validate(),
            Err(LayoutError::VersionMismatch { .. })
        ));
    }

    #[test]
    fn validate_rejects_missing_pane() {
        // A schema+version-valid snapshot that dropped a canonical pane is structurally corrupt:
        // `validate` must reject it (PERMANENT data error) so the caller falls back to LKG/default
        // instead of applying a short-paned layout. pane-b is the named first-missing pane.
        let mut snap = sample_snapshot();
        snap.panes.remove(&pid("pane-b"));
        match snap.validate() {
            Err(LayoutError::MissingPane { id }) => assert_eq!(id, "pane-b"),
            other => panic!("expected MissingPane {{ id: \"pane-b\" }}, got {other:?}"),
        }
        // And it is a permanent (non-transient) error, so the manager triggers fallback, not retry.
        assert!(!LayoutError::MissingPane {
            id: "pane-b".into()
        }
        .is_transient());
    }

    #[test]
    fn validate_rejects_unknown_active_pane() {
        // A dangling `active_pane` is a corrupt reference: reject it so a restore never points the
        // highlight at a non-existent pane.
        let mut snap = sample_snapshot();
        snap.active_pane = Some(pid("pane-z"));
        match snap.validate() {
            Err(LayoutError::UnknownActivePane { id }) => assert_eq!(id, "pane-z"),
            other => panic!("expected UnknownActivePane, got {other:?}"),
        }
        assert!(!LayoutError::UnknownActivePane {
            id: "pane-z".into()
        }
        .is_transient());
    }

    #[test]
    fn validate_accepts_legacy_extra_active_panes() {
        // Custom saved layouts can legitimately keep focus on non-default panes as long as those panes
        // are still present in the saved registry.
        let mut snap = sample_snapshot();
        snap.active_pane = Some(pid("pane-c"));
        assert!(snap.validate().is_ok());

        snap.active_pane = Some(pid("pane-d"));
        assert!(snap.validate().is_ok());
    }

    #[test]
    fn schema_id_and_version_serialize_into_layout_state() {
        let value = sample_snapshot().to_layout_state();
        assert_eq!(value["schema_id"], serde_json::json!(LAYOUT_SCHEMA_ID));
        assert_eq!(value["version"], serde_json::json!(LAYOUT_SNAPSHOT_VERSION));
    }

    // ── LayoutSnapshot <-> backend layout_state JSON mapping (round-trip identity) ───────────────

    /// The headline mapping test: snapshot -> `layout_state` blob -> snapshot yields an IDENTICAL
    /// snapshot (serialized-form equality, the persisted contract). This is exactly what `PUT` then
    /// `GET` against the backend does to the blob, proving the mapping is an identity for every
    /// composed MT's state.
    #[test]
    fn layout_state_mapping_round_trips_identically() {
        let snap = sample_snapshot();
        let blob = snap.to_layout_state();
        let back = LayoutSnapshot::from_layout_state(blob).expect("round-trip parse");
        assert_eq!(snap.to_layout_state(), back.to_layout_state());
    }

    #[test]
    fn from_layout_state_rejects_foreign_schema_blob() {
        // A blob that parses structurally but carries the React schema id must be rejected, so a
        // native load never applies the React workbench's own layout blob.
        let mut snap = sample_snapshot();
        snap.schema_id = "hsk.workbench_layout_state@1".to_owned();
        let blob = serde_json::to_value(&snap).unwrap();
        assert!(matches!(
            LayoutSnapshot::from_layout_state(blob),
            Err(LayoutError::SchemaMismatch { .. })
        ));
    }

    #[test]
    fn from_layout_state_rejects_corrupt_blob() {
        let garbage = serde_json::json!({ "schema_id": LAYOUT_SCHEMA_ID, "version": 1 }); // missing fields
        assert!(matches!(
            LayoutSnapshot::from_layout_state(garbage),
            Err(LayoutError::Serde(_))
        ));
    }

    // ── Restore-time clamp (MT-008-deferred) ────────────────────────────────────────────────────

    #[test]
    fn restore_clamps_only_off_monitor_pop_outs() {
        let mut snap = sample_snapshot();
        let full_desktop =
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(3840.0, 1080.0));
        snap.pop_outs.insert(
            pid("pane-b"),
            PopOutSnapshot {
                geometry: PopOutGeometry::at(egui::pos2(2200.0, 300.0)), // second monitor
                open: true,
            },
        );
        snap.pop_outs.insert(
            pid("pane-a"),
            PopOutSnapshot {
                geometry: PopOutGeometry::at(egui::pos2(9000.0, 9000.0)), // off all monitors
                open: true,
            },
        );
        let clamped = snap.clamp_pop_outs_to(full_desktop);
        assert_eq!(
            clamped.pop_outs.get(&pid("pane-b")).unwrap().geometry.pos,
            egui::pos2(2200.0, 300.0),
            "a pop-out on the second monitor must NOT be snapped"
        );
        assert_eq!(
            clamped.pop_outs.get(&pid("pane-a")).unwrap().geometry.pos,
            crate::popout_window::FALLBACK_POPOUT_POS,
            "a pop-out off all monitors must reset to the fallback position"
        );
    }

    #[test]
    fn restore_clamp_is_idempotent() {
        let snap = sample_snapshot();
        let extent = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1920.0, 1080.0));
        let once = snap.clamp_pop_outs_to(extent);
        let twice = once.clone().clamp_pop_outs_to(extent);
        assert_eq!(once.to_layout_state(), twice.to_layout_state());
    }

    // ── Stub transport for manager unit tests (no live server) ──────────────────────────────────

    /// A scriptable in-memory transport. `save_results`/`load_results` are queues of scripted
    /// outcomes consumed front-to-back; `saved_blobs` records every blob a `save` accepted so a test
    /// can assert debounce coalescing (one save per flush) and content. Built on `Arc<Mutex<_>>` (the
    /// trait is `Send + Sync`) so a test keeps a handle to inspect calls after handing the box to the
    /// manager.
    #[derive(Clone, Default)]
    struct StubTransport {
        inner: Arc<Mutex<StubState>>,
    }

    #[derive(Default)]
    struct StubState {
        save_results: std::collections::VecDeque<Result<(), LayoutError>>,
        load_results: std::collections::VecDeque<Result<Option<Value>, LayoutError>>,
        saved_blobs: Vec<Value>,
        save_calls: u32,
        load_calls: u32,
    }

    impl StubTransport {
        fn new() -> Self {
            Self::default()
        }
        fn push_save(&self, r: Result<(), LayoutError>) {
            self.inner.lock().unwrap().save_results.push_back(r);
        }
        fn push_load(&self, r: Result<Option<Value>, LayoutError>) {
            self.inner.lock().unwrap().load_results.push_back(r);
        }
        fn save_calls(&self) -> u32 {
            self.inner.lock().unwrap().save_calls
        }
        fn load_calls(&self) -> u32 {
            self.inner.lock().unwrap().load_calls
        }
        fn last_saved_blob(&self) -> Option<Value> {
            self.inner.lock().unwrap().saved_blobs.last().cloned()
        }
    }

    impl LayoutTransport for StubTransport {
        fn load(&self, _workspace_id: &str) -> Result<Option<Value>, LayoutError> {
            let mut s = self.inner.lock().unwrap();
            s.load_calls += 1;
            s.load_results
                .pop_front()
                .unwrap_or_else(|| Err(LayoutError::Transport("no scripted load result".into())))
        }
        fn save(&self, _workspace_id: &str, layout_state: Value) -> Result<(), LayoutError> {
            let mut s = self.inner.lock().unwrap();
            s.save_calls += 1;
            let result = s
                .save_results
                .pop_front()
                .unwrap_or_else(|| Err(LayoutError::Transport("no scripted save result".into())));
            if result.is_ok() {
                s.saved_blobs.push(layout_state);
            }
            result
        }
    }

    fn manager_with(stub: StubTransport, debounce: Duration) -> LayoutPersistenceManager {
        LayoutPersistenceManager::new(Box::new(stub), debounce)
    }

    // ── Debounce ────────────────────────────────────────────────────────────────────────────────

    #[test]
    fn debounce_coalesces_rapid_changes_into_one_save() {
        let stub = StubTransport::new();
        stub.push_save(Ok(()));
        let mut mgr = manager_with(stub.clone(), Duration::from_millis(200));
        let t0 = Instant::now();

        // Three rapid changes within the debounce window: each resets the anchor, none is due yet.
        mgr.mark_dirty(t0);
        assert!(!mgr.flush_if_due(t0 + Duration::from_millis(50), &sample_snapshot()));
        mgr.mark_dirty(t0 + Duration::from_millis(50));
        assert!(!mgr.flush_if_due(t0 + Duration::from_millis(120), &sample_snapshot()));
        mgr.mark_dirty(t0 + Duration::from_millis(120));
        assert_eq!(
            stub.save_calls(),
            0,
            "no save while changes keep coalescing"
        );

        // Quiet period elapses after the LAST change -> exactly one flush.
        assert!(mgr.flush_if_due(t0 + Duration::from_millis(400), &sample_snapshot()));
        assert_eq!(stub.save_calls(), 1, "rapid changes coalesce into one save");
        assert!(!mgr.is_dirty(), "dirty cleared after successful flush");
        assert_eq!(mgr.status(), &LayoutPersistenceStatus::Saved);
    }

    #[test]
    fn flush_not_due_inside_debounce_window() {
        let stub = StubTransport::new();
        let mut mgr = manager_with(stub.clone(), Duration::from_millis(200));
        let t0 = Instant::now();
        mgr.mark_dirty(t0);
        assert!(!mgr.due_to_flush(t0 + Duration::from_millis(199)));
        assert!(mgr.due_to_flush(t0 + Duration::from_millis(200)));
        assert!(!mgr.flush_if_due(t0 + Duration::from_millis(100), &sample_snapshot()));
        assert_eq!(stub.save_calls(), 0);
    }

    #[test]
    fn flush_when_clean_is_noop() {
        let stub = StubTransport::new();
        let mut mgr = manager_with(stub.clone(), Duration::from_millis(10));
        assert!(!mgr.flush_if_due(Instant::now(), &sample_snapshot()));
        assert_eq!(stub.save_calls(), 0);
    }

    #[test]
    fn saved_blob_matches_snapshot_layout_state() {
        let stub = StubTransport::new();
        stub.push_save(Ok(()));
        let mut mgr = manager_with(stub.clone(), Duration::ZERO);
        let snap = sample_snapshot();
        mgr.mark_dirty(Instant::now());
        assert!(mgr.flush_if_due(Instant::now(), &snap));
        assert_eq!(
            stub.last_saved_blob().unwrap(),
            snap.to_layout_state(),
            "the blob PUT to the backend is exactly the snapshot's layout_state"
        );
    }

    // ── Retry + LKG on save ─────────────────────────────────────────────────────────────────────

    #[test]
    fn save_retries_transient_failure_then_succeeds() {
        let stub = StubTransport::new();
        stub.push_save(Err(LayoutError::Transport("blip 1".into())));
        stub.push_save(Err(LayoutError::Transport("blip 2".into())));
        stub.push_save(Ok(()));
        let mut mgr = manager_with(stub.clone(), Duration::ZERO);
        mgr.save_now(&sample_snapshot());
        assert_eq!(
            stub.save_calls(),
            3,
            "two transient failures retried, third succeeds"
        );
        assert_eq!(mgr.status(), &LayoutPersistenceStatus::Saved);
        assert!(
            mgr.last_known_good().is_some(),
            "LKG populated after successful save"
        );
        assert!(!mgr.is_dirty());
    }

    #[test]
    fn save_gives_up_after_retry_budget_and_keeps_dirty() {
        let stub = StubTransport::new();
        for _ in 0..(SAVE_MAX_RETRIES + 1) {
            stub.push_save(Err(LayoutError::Transport("down".into())));
        }
        let mut mgr = manager_with(stub.clone(), Duration::ZERO);
        mgr.mark_dirty(Instant::now());
        mgr.save_now(&sample_snapshot());
        assert_eq!(
            stub.save_calls(),
            SAVE_MAX_RETRIES + 1,
            "initial attempt + SAVE_MAX_RETRIES retries"
        );
        assert!(matches!(
            mgr.status(),
            LayoutPersistenceStatus::Error { .. }
        ));
        assert!(
            mgr.is_dirty(),
            "dirty kept so a later flush retries (no data loss)"
        );
    }

    #[test]
    fn save_permanent_error_is_not_retried() {
        let stub = StubTransport::new();
        // A serde/data error is permanent; the manager must NOT burn the retry budget on it.
        stub.push_save(Err(LayoutError::Serde("bad".into())));
        let mut mgr = manager_with(stub.clone(), Duration::ZERO);
        mgr.save_now(&sample_snapshot());
        assert_eq!(stub.save_calls(), 1, "permanent error tried exactly once");
        assert!(matches!(
            mgr.status(),
            LayoutPersistenceStatus::Error { .. }
        ));
    }

    // ── Load fallback chain ─────────────────────────────────────────────────────────────────────

    #[test]
    fn load_applies_valid_stored_blob_and_records_lkg() {
        let stub = StubTransport::new();
        stub.push_load(Ok(Some(sample_snapshot().to_layout_state())));
        let mut mgr = manager_with(stub.clone(), Duration::ZERO);
        let loaded = mgr
            .load("proj-1")
            .expect("load ok")
            .expect("snapshot present");
        assert_eq!(
            loaded.to_layout_state(),
            sample_snapshot().to_layout_state()
        );
        assert_eq!(stub.load_calls(), 1);
        assert_eq!(mgr.status(), &LayoutPersistenceStatus::Loaded);
        assert!(mgr.last_known_good().is_some(), "valid load records LKG");
    }

    #[test]
    fn load_missing_blob_is_default_not_error() {
        let stub = StubTransport::new();
        stub.push_load(Ok(None));
        let mut mgr = manager_with(stub.clone(), Duration::ZERO);
        assert!(mgr.load("brand-new").expect("load ok").is_none());
        assert_eq!(mgr.status(), &LayoutPersistenceStatus::Default);
    }

    #[test]
    fn load_transport_failure_falls_back_to_lkg() {
        let stub = StubTransport::new();
        // First load succeeds (populates LKG); second load fails transport -> returns the LKG.
        stub.push_load(Ok(Some(sample_snapshot().to_layout_state())));
        stub.push_load(Err(LayoutError::Transport("down".into())));
        let mut mgr = manager_with(stub.clone(), Duration::ZERO);
        let _ = mgr.load("proj-1").expect("first load");
        let fallback = mgr
            .load("proj-1")
            .expect("second load returns Ok with fallback");
        assert!(fallback.is_some(), "transport failure falls back to LKG");
        assert_eq!(
            fallback.unwrap().to_layout_state(),
            sample_snapshot().to_layout_state()
        );
        assert!(matches!(
            mgr.status(),
            LayoutPersistenceStatus::Error { .. }
        ));
    }

    #[test]
    fn load_corrupt_blob_with_no_lkg_falls_back_to_default() {
        let stub = StubTransport::new();
        // A blob that fails validation, no LKG held -> Ok(None) so the caller uses the seeded default
        // (never applies the corrupt blob; no infinite restore loop).
        let mut bad = sample_snapshot();
        bad.schema_id = "hsk.workbench_layout_state@1".to_owned();
        stub.push_load(Ok(Some(serde_json::to_value(&bad).unwrap())));
        let mut mgr = manager_with(stub.clone(), Duration::ZERO);
        let result = mgr.load("proj-1").expect("load ok");
        assert!(result.is_none(), "corrupt blob with no LKG -> default");
        assert!(matches!(
            mgr.status(),
            LayoutPersistenceStatus::Error { .. }
        ));
    }

    #[test]
    fn load_wrong_project_blob_is_rejected() {
        let stub = StubTransport::new();
        // The blob is valid but for a DIFFERENT project than requested.
        stub.push_load(Ok(Some(sample_snapshot().to_layout_state()))); // project_id = "proj-1"
        let mut mgr = manager_with(stub.clone(), Duration::ZERO);
        let result = mgr.load("proj-2").expect("load ok"); // request a different workspace
        assert!(
            result.is_none(),
            "wrong-project blob with no LKG -> default"
        );
        assert!(matches!(
            mgr.status(),
            LayoutPersistenceStatus::Error { .. }
        ));
    }

    #[test]
    fn is_transient_classifies_errors() {
        assert!(LayoutError::Transport("x".into()).is_transient());
        assert!(!LayoutError::Serde("x".into()).is_transient());
        assert!(!LayoutError::VersionMismatch {
            found: 2,
            expected: 1
        }
        .is_transient());
    }
    fn with_canonical_dock_model(mut snapshot:LayoutSnapshot)->LayoutSnapshot {
        let mut owner=crate::kumihimo::NativeHost::new().unwrap();
        let panes:Vec<_>=snapshot.panes.keys().cloned().collect();
        let area=egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(1200.0,800.0));
        owner.project(egui::ViewportId::ROOT,&panes,area,snapshot.split_weights).unwrap();
        let bars=snapshot.tab_bars.iter().map(|(id,bar)|(id.clone(),bar.clone())).collect();
        owner.reconcile_tabs(egui::ViewportId::ROOT,&bars).unwrap();
        owner.dock(egui::ViewportId::ROOT,&pid("pane-a"),&pid("pane-b"),kumihimo_core::Edge::Bottom).unwrap();
        for (pane,popout) in &snapshot.pop_outs {
            if popout.open {
                let viewport=egui::ViewportId::from_hash_of(pane.as_ref());
                owner.project(viewport,&[pane.clone()],area,SplitWeights::default()).unwrap();
                owner.reconcile_tabs(viewport,&bars).unwrap();
            }
        }
        snapshot.kumihimo=Some(handshake_layout_snapshot::KumihimoLayoutState::Captured {layout:owner.capture().unwrap().unwrap()});
        snapshot
    }

    #[test]
    fn invalid_canonical_model_never_replaces_lkg_or_reaches_save_transport() {
        let good=with_canonical_dock_model(sample_snapshot());
        validate_snapshot(&good).unwrap();
        let stub=StubTransport::new();stub.push_load(Ok(Some(good.to_layout_state())));
        let mut manager=manager_with(stub.clone(),Duration::ZERO);
        manager.load("proj-1").unwrap().unwrap();
        let lkg=manager.last_known_good().unwrap().to_layout_state();
        let mut corrupt=good.clone();
        if let Some(handshake_layout_snapshot::KumihimoLayoutState::Captured {layout})=&mut corrupt.kumihimo {
            layout.model_json="{\"invalid\":true}".into();
        }
        stub.push_load(Ok(Some(corrupt.to_layout_state())));
        let fallback=manager.load("proj-1").unwrap().unwrap();
        assert_eq!(fallback.to_layout_state(),lkg);
        assert!(matches!(manager.status(),LayoutPersistenceStatus::Error {..}));
        assert_eq!(manager.last_known_good().unwrap().to_layout_state(),lkg);
        manager.save_now(&corrupt);
        assert_eq!(stub.save_calls(),0);
        assert_eq!(manager.last_known_good().unwrap().to_layout_state(),lkg);
        stub.push_load(Err(LayoutError::Transport("offline".into())));
        assert!(manager.load("different-workspace").unwrap().is_none());
        assert_eq!(manager.last_known_good().unwrap().to_layout_state(),lkg);
    }

    #[test]
    fn legacy_layout_without_asset_model_remains_readable_and_explicit_capture_failure_rejects() {
        let legacy=sample_snapshot();
        assert!(legacy.kumihimo.is_none());
        let loaded=LayoutSnapshot::from_layout_state(legacy.to_layout_state()).unwrap();
        validate_snapshot(&loaded).unwrap();
        assert_eq!(loaded.to_layout_state(),legacy.to_layout_state());
        let mut failed=loaded;
        failed.kumihimo=Some(handshake_layout_snapshot::KumihimoLayoutState::CaptureFailed {reason:"retired owner".into()});
        assert!(validate_snapshot(&failed).is_err());
    }

    #[test]
    fn active_popout_owner_and_tab_corruption_never_enters_lkg() {
        use handshake_layout_snapshot::{KumihimoLayoutState,KumihimoWindowId};
        let good=with_canonical_dock_model(sample_snapshot());validate_snapshot(&good).unwrap();
        let mut unknown=good.clone();
        if let Some(KumihimoLayoutState::Captured {layout})=&mut unknown.kumihimo {
            let old=KumihimoWindowId::PopOut {pane_id:pid("pane-b")};
            let new=KumihimoWindowId::PopOut {pane_id:pid("unknown-native-pane")};
            let window=layout.windows.iter_mut().find(|window|window.window==old).unwrap();
            let frame=window.frames.remove(&pid("pane-b")).unwrap();window.frames.insert(pid("unknown-native-pane"),frame);
            window.active=vec![pid("unknown-native-pane")];window.window=new.clone();
            for view in &mut layout.views {if view.window==old {view.window=new.clone();view.pane_id=pid("unknown-native-pane");}}
            layout.restore_core().unwrap(); // canonical model/root remain valid; the host owner is invalid
        }
        let mut closed=good.clone();closed.pop_outs.get_mut(&pid("pane-b")).unwrap().open=false;
        let mut wrong_tab=good.clone();
        if let Some(KumihimoLayoutState::Captured {layout})=&mut wrong_tab.kumihimo {
            let view=layout.views.iter().find(|view|matches!(view.window,KumihimoWindowId::PopOut {..})).unwrap().view;
            let mut model=kumihimo_layout_json::decode(layout.model_json.as_bytes()).unwrap();
            model.views.iter_mut().find(|entry|entry.id==view).unwrap().label="different popout tab".into();
            layout.model_json=String::from_utf8(kumihimo_layout_json::encode(&model).unwrap()).unwrap();
            layout.restore_core().unwrap();
        }
        let stub=StubTransport::new();stub.push_load(Ok(Some(good.to_layout_state())));
        let mut manager=manager_with(stub.clone(),Duration::ZERO);manager.load("proj-1").unwrap();
        let baseline=manager.last_known_good().unwrap().to_layout_state();
        for corrupt in [unknown,closed,wrong_tab] {
            assert!(validate_snapshot(&corrupt).is_err());stub.push_load(Ok(Some(corrupt.to_layout_state())));
            assert_eq!(manager.load("proj-1").unwrap().unwrap().to_layout_state(),baseline);
            assert!(matches!(manager.status(),LayoutPersistenceStatus::Error {..}));
            assert_eq!(manager.last_known_good().unwrap().to_layout_state(),baseline);
            manager.save_now(&corrupt);assert_eq!(stub.save_calls(),0);
        }
    }

}
