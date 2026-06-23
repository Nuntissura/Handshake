//! Unified undo scope (WP-KERNEL-012 MT-035, cluster E5 — melt-together).
//!
//! ## What this is (the ONE undo-scope policy across all editor panes)
//!
//! [`UnifiedUndoScope`] is the in-memory, session-scoped undo authority every editor surface (code,
//! rich-text, graph, canvas, stage) shares through the MT-031 [`crate::interop::InteractionBus`]. It
//! encodes Handshake's five undo policies so the four panes behave like ONE tool rather than four
//! editors with ad-hoc per-pane undo:
//!
//! - **POLICY-1 (local-first):** Ctrl+Z in a focused pane undoes the most recent action IN THAT PANE,
//!   not the most recent action globally (the VS Code / Obsidian model). Each pane owns a
//!   [`PaneUndoRing`]; [`UnifiedUndoScope::undo_local`] touches only the focused pane's ring.
//! - **POLICY-2 (cross-pane ring):** an action that touches multiple panes atomically
//!   (embed-from-atelier, route-to-stage, canvas placement) is pushed to the ONE
//!   [`CrossPaneUndoRing`]; the dedicated `undo-cross-pane` command (Ctrl+Shift+Z) undoes the most
//!   recent cross-pane action.
//! - **POLICY-3 (session-scoped, NEVER persisted):** the whole scope is in-memory only. It deliberately
//!   does NOT derive or implement `Serialize`/`Deserialize` — a `#[derive(Serialize)]` here would be an
//!   AC-3 FAILURE. On app restart the scope is empty; there is nothing on disk to reload.
//! - **POLICY-4 (canvas compensating undo):** a canvas placement create/move calls the backend
//!   immediately, so its undo is a COMPENSATING backend call (delete/re-place via the MT-026
//!   `/loom/canvas` placement routes). That work is async, so an [`UndoAction`] may carry an
//!   [`UndoAction::undo_async_fn`] the bus dispatches onto the tokio runtime; the snapshot of the
//!   previous placement is captured AT ACTION-CREATE TIME, not at undo-invoke time (RISK-2 / MC-2).
//! - **POLICY-5 (caps):** a [`PaneUndoRing`] is capped at [`PANE_RING_CAP`] (200); the
//!   [`CrossPaneUndoRing`] at [`CROSS_PANE_RING_CAP`] (50). Pushing past the cap silently drops the
//!   OLDEST entry so an hour of editing without a save cannot OOM the stack.
//!
//! ## Weak back-references (RISK-3 / MC-3 — no retain cycle)
//!
//! An [`UndoAction`]'s closures restore pane state (a rope snapshot, a document tree snapshot, a
//! placement list). That pane state is ALSO held by the host / bus, so a closure capturing a strong
//! `Arc` to it would form a retain cycle (the bus holds the action which holds the pane which is held by
//! the bus). The closures therefore capture `std::sync::Weak<...>` back-references and
//! [`Weak::upgrade`] only during invocation, skipping silently (returning an `ok=false`
//! [`UndoResult`]) when the pane has been dropped. This module does not force that — it accepts any
//! `Fn` closure — but the pane adapters that build [`UndoAction`]s use the `Weak` pattern, and the
//! [`UndoResult::pane_dropped`] helper is the canonical "pane gone" result.
//!
//! ## NOT CRDT undo
//!
//! This is the in-memory UI-layer undo for immediate local feedback only. CRDT-based collaborative
//! undo is handled by the CRDT backend (MT-038 / MT-039); this stack never touches CRDT state.

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::pane_registry::PaneId;

/// Maximum operations retained per pane local ring (POLICY-5 contract: 200). Pushing past it drops the
/// oldest entry.
pub const PANE_RING_CAP: usize = 200;

/// Maximum operations retained in the cross-pane ring (POLICY-5 contract: 50). Pushing past it drops the
/// oldest entry.
pub const CROSS_PANE_RING_CAP: usize = 50;

/// The outcome of invoking an undo or redo closure. Returned (not panicked) so a stale snapshot, a
/// dropped pane, or a failed compensating backend call degrades to a recorded error the bus can log to
/// the Flight Recorder (MT-036) rather than aborting the egui frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoResult {
    /// True when the undo/redo applied cleanly.
    pub ok: bool,
    /// A human/agent-readable error when `ok` is false (e.g. "pane dropped", a buffer error, a
    /// compensating-call failure message). `None` on success.
    pub error: Option<String>,
}

impl UndoResult {
    /// A successful undo/redo.
    pub fn ok() -> Self {
        Self { ok: true, error: None }
    }

    /// A failed undo/redo carrying a reason.
    pub fn err(reason: impl Into<String>) -> Self {
        Self { ok: false, error: Some(reason.into()) }
    }

    /// The canonical result when the back-referenced pane state was dropped before the undo fired
    /// (RISK-3 / MC-3): a benign no-op failure, never a panic.
    pub fn pane_dropped() -> Self {
        Self::err("pane state dropped before undo invocation (Weak upgrade failed)")
    }

    /// True when the canvas compensating call (or any async undo) is still in flight; the bus treats
    /// this as "dispatched, awaiting completion" rather than a hard failure.
    pub fn dispatched_async() -> Self {
        Self { ok: true, error: None }
    }
}

/// A synchronous undo/redo closure (rope/doc-tree snapshot restore). `Send + Sync` so the whole scope
/// (held in the `Arc<Mutex<InteractionBus>>`) stays `Send + Sync` for egui's data store.
pub type UndoFn = Arc<dyn Fn() -> UndoResult + Send + Sync>;

/// An asynchronous undo/redo closure (the canvas compensating backend call — POLICY-4). The bus
/// dispatches this onto the app's tokio runtime; it returns a boxed future so the trait object stays
/// `Send + Sync`-safe across the `Arc<Mutex<…>>` boundary.
pub type UndoAsyncFn =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = UndoResult> + Send>> + Send + Sync>;

/// One reversible action recorded on a ring. The local-pane case uses the synchronous
/// [`UndoAction::undo_fn`] / [`UndoAction::redo_fn`]; the canvas-compensating case additionally carries
/// [`UndoAction::undo_async_fn`] (POLICY-4). The closures themselves capture `Weak` back-references to
/// pane state (RISK-3 / MC-3) — this struct stores them opaquely.
#[derive(Clone)]
pub struct UndoAction {
    /// A short human/agent-readable description (shown in the "Show Undo History" inspector — MC-5).
    pub description: String,
    /// The synchronous undo closure (restore the previous pane snapshot). Always present.
    pub undo_fn: UndoFn,
    /// The synchronous redo closure (re-apply the edit). Always present.
    pub redo_fn: UndoFn,
    /// The OPTIONAL async undo closure for a compensating backend call (canvas — POLICY-4). When
    /// `Some`, the bus dispatches it onto the tokio runtime instead of calling `undo_fn` synchronously.
    pub undo_async_fn: Option<UndoAsyncFn>,
    /// The OPTIONAL async redo closure (the compensating call's inverse — re-place on canvas).
    pub redo_async_fn: Option<UndoAsyncFn>,
}

impl std::fmt::Debug for UndoAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UndoAction")
            .field("description", &self.description)
            .field("undo_fn", &"<fn>")
            .field("redo_fn", &"<fn>")
            .field("undo_async_fn", &self.undo_async_fn.as_ref().map(|_| "<async fn>"))
            .field("redo_async_fn", &self.redo_async_fn.as_ref().map(|_| "<async fn>"))
            .finish()
    }
}

impl UndoAction {
    /// Build a synchronous local-pane undo action (code / rich-text / graph). The closures restore /
    /// re-apply a snapshot; they should capture `Weak` back-refs to the pane state (RISK-3 / MC-3).
    pub fn sync(description: impl Into<String>, undo_fn: UndoFn, redo_fn: UndoFn) -> Self {
        Self {
            description: description.into(),
            undo_fn,
            redo_fn,
            undo_async_fn: None,
            redo_async_fn: None,
        }
    }

    /// Build a canvas-compensating undo action (POLICY-4). `undo_fn`/`redo_fn` remain the synchronous
    /// fallbacks (e.g. an in-memory placement-list mutation); `undo_async_fn`/`redo_async_fn` are the
    /// compensating backend calls the bus dispatches onto the tokio runtime. The previous-placement
    /// snapshot must be captured by the async closures AT CONSTRUCTION TIME (RISK-2 / MC-2).
    pub fn async_compensating(
        description: impl Into<String>,
        undo_fn: UndoFn,
        redo_fn: UndoFn,
        undo_async_fn: UndoAsyncFn,
        redo_async_fn: UndoAsyncFn,
    ) -> Self {
        Self {
            description: description.into(),
            undo_fn,
            redo_fn,
            undo_async_fn: Some(undo_async_fn),
            redo_async_fn: Some(redo_async_fn),
        }
    }

    /// True when this action's undo requires an async (compensating backend) dispatch (POLICY-4). The
    /// bus reads this to decide whether to call `undo_fn` synchronously or hand `undo_async_fn` to the
    /// runtime.
    pub fn is_async(&self) -> bool {
        self.undo_async_fn.is_some()
    }
}

/// A bounded per-pane undo ring (POLICY-1 local + POLICY-5 cap). Holds an undo stack and a redo stack;
/// pushing a fresh action clears the redo stack (the standard linear-history model — no redo branch).
#[derive(Debug, Clone)]
pub struct PaneUndoRing {
    /// The pane this ring belongs to (POLICY-1: the focused pane's ring is the one Ctrl+Z touches).
    pub pane_id: PaneId,
    /// Applied actions, oldest at the front. Length never exceeds `cap`.
    ring: VecDeque<UndoAction>,
    /// Undone actions available to redo, most-recently-undone at the back.
    redo_ring: VecDeque<UndoAction>,
    /// The cap; pushing past it drops the oldest entry (POLICY-5).
    cap: usize,
}

impl PaneUndoRing {
    /// A ring for `pane_id` with the default [`PANE_RING_CAP`].
    pub fn new(pane_id: PaneId) -> Self {
        Self::with_cap(pane_id, PANE_RING_CAP)
    }

    /// A ring with an explicit cap (a small cap is useful in focused tests). A cap of 0 is clamped to 1
    /// so at least one undo is always possible.
    pub fn with_cap(pane_id: PaneId, cap: usize) -> Self {
        Self {
            pane_id,
            ring: VecDeque::new(),
            redo_ring: VecDeque::new(),
            cap: cap.max(1),
        }
    }

    /// Push an applied action. The redo stack is cleared first (a fresh edit forks linear history). If
    /// the ring is at the cap, the OLDEST entry is dropped (POLICY-5).
    pub fn push(&mut self, action: UndoAction) {
        self.redo_ring.clear();
        self.ring.push_back(action);
        while self.ring.len() > self.cap {
            self.ring.pop_front();
        }
    }

    /// Pop the most recent action for undo (moving it to the redo stack). `None` when nothing to undo.
    /// The caller invokes the popped action's `undo_fn` / `undo_async_fn`.
    pub fn pop_undo(&mut self) -> Option<UndoAction> {
        let action = self.ring.pop_back()?;
        self.redo_ring.push_back(action.clone());
        Some(action)
    }

    /// Pop the most recently undone action for redo (moving it back to the undo stack). `None` when
    /// nothing to redo. The caller invokes the popped action's `redo_fn` / `redo_async_fn`.
    pub fn pop_redo(&mut self) -> Option<UndoAction> {
        let action = self.redo_ring.pop_back()?;
        self.ring.push_back(action.clone());
        Some(action)
    }

    /// Replace the MOST RECENT undo entry in place (MT-035 typing-coalescing — RISK-1 / MC-1). Used by
    /// the rich-text 500ms batcher: rapid keystrokes within the window coalesce into the SAME tail entry
    /// (its `undo_fn` keeps restoring the batch-START snapshot; its `redo_fn` is swapped to re-apply the
    /// latest `after`), so N keystrokes produce ONE undo entry rather than N and never silently drop the
    /// in-between edits from history. Returns `true` when a tail entry existed and was replaced, `false`
    /// when the ring was empty (the caller should `push` a fresh entry instead). Does NOT touch the redo
    /// ring (a coalesced edit is still a fresh edit; the caller clears redo via `push` on the first entry
    /// of a batch).
    pub fn replace_tail(&mut self, action: UndoAction) -> bool {
        if self.ring.is_empty() {
            return false;
        }
        let last = self.ring.len() - 1;
        self.ring[last] = action;
        true
    }

    /// Number of actions available to undo (the local "Undo ({n})" indicator count — AC-6).
    pub fn undo_len(&self) -> usize {
        self.ring.len()
    }

    /// Number of actions available to redo.
    pub fn redo_len(&self) -> usize {
        self.redo_ring.len()
    }

    /// True when there is at least one action to undo.
    pub fn can_undo(&self) -> bool {
        !self.ring.is_empty()
    }

    /// True when there is at least one action to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo_ring.is_empty()
    }

    /// The most recent `n` undo entries' descriptions, newest first (the "Show Undo History" inspector
    /// — MC-5).
    pub fn recent_descriptions(&self, n: usize) -> Vec<String> {
        self.ring.iter().rev().take(n).map(|a| a.description.clone()).collect()
    }
}

/// The single bounded cross-pane undo ring (POLICY-2 + POLICY-5 cap). Same shape as [`PaneUndoRing`]
/// but pane-agnostic — it records actions that touch multiple panes atomically (embed-from-atelier,
/// route-to-stage, canvas placement). Undone via the dedicated `undo-cross-pane` command (Ctrl+Shift+Z).
#[derive(Debug, Clone, Default)]
pub struct CrossPaneUndoRing {
    ring: VecDeque<UndoAction>,
    redo_ring: VecDeque<UndoAction>,
    cap: usize,
}

impl CrossPaneUndoRing {
    /// A cross-pane ring with the default [`CROSS_PANE_RING_CAP`].
    pub fn new() -> Self {
        Self::with_cap(CROSS_PANE_RING_CAP)
    }

    /// A cross-pane ring with an explicit cap (clamped to >= 1).
    pub fn with_cap(cap: usize) -> Self {
        Self {
            ring: VecDeque::new(),
            redo_ring: VecDeque::new(),
            cap: cap.max(1),
        }
    }

    /// Push a cross-pane action (clears the redo stack; drops the oldest past the cap — POLICY-5).
    pub fn push(&mut self, action: UndoAction) {
        self.redo_ring.clear();
        self.ring.push_back(action);
        while self.ring.len() > self.cap {
            self.ring.pop_front();
        }
    }

    /// Pop the most recent cross-pane action for undo (moving it to the redo stack).
    pub fn pop_undo(&mut self) -> Option<UndoAction> {
        let action = self.ring.pop_back()?;
        self.redo_ring.push_back(action.clone());
        Some(action)
    }

    /// Pop the most recently undone cross-pane action for redo.
    pub fn pop_redo(&mut self) -> Option<UndoAction> {
        let action = self.redo_ring.pop_back()?;
        self.ring.push_back(action.clone());
        Some(action)
    }

    /// Number of cross-pane actions available to undo.
    pub fn undo_len(&self) -> usize {
        self.ring.len()
    }

    /// Number of cross-pane actions available to redo.
    pub fn redo_len(&self) -> usize {
        self.redo_ring.len()
    }

    /// True when there is at least one cross-pane action to undo.
    pub fn can_undo(&self) -> bool {
        !self.ring.is_empty()
    }

    /// The most recent `n` cross-pane entries' descriptions, newest first (MC-5 inspector).
    pub fn recent_descriptions(&self, n: usize) -> Vec<String> {
        self.ring.iter().rev().take(n).map(|a| a.description.clone()).collect()
    }
}

/// The one unified undo scope. Holds a [`PaneUndoRing`] per pane plus the single [`CrossPaneUndoRing`].
///
/// POLICY-3 (session-scoped, NEVER persisted): this type deliberately does NOT derive or implement
/// `Serialize`/`Deserialize`. The AC-3 proof asserts a fresh scope is empty and that the type cannot be
/// serialized — adding a `#[derive(Serialize)]` here is a contract FAILURE.
#[derive(Debug, Default)]
pub struct UnifiedUndoScope {
    /// One bounded undo ring per pane (POLICY-1 local-first).
    pane_rings: HashMap<PaneId, PaneUndoRing>,
    /// The single cross-pane ring (POLICY-2).
    cross_pane_ring: CrossPaneUndoRing,
}

impl UnifiedUndoScope {
    /// A fresh, empty scope (no pane rings, empty cross-pane ring). On app restart this is the only
    /// state that exists (POLICY-3).
    pub fn new() -> Self {
        Self {
            pane_rings: HashMap::new(),
            cross_pane_ring: CrossPaneUndoRing::new(),
        }
    }

    /// Push a local-pane action onto `pane_id`'s ring, creating the ring on first use (POLICY-1).
    pub fn push_local(&mut self, pane_id: PaneId, action: UndoAction) {
        self.pane_rings
            .entry(pane_id.clone())
            .or_insert_with(|| PaneUndoRing::new(pane_id))
            .push(action);
    }

    /// Pop the most recent local action for `pane_id` for undo. `None` when the pane has no ring or
    /// nothing to undo. The CALLER invokes the returned action (so the bus can choose sync vs async
    /// dispatch). This is the local-first primitive (POLICY-1).
    pub fn pop_undo_local(&mut self, pane_id: &PaneId) -> Option<UndoAction> {
        self.pane_rings.get_mut(pane_id)?.pop_undo()
    }

    /// Pop the most recently undone local action for `pane_id` for redo.
    pub fn pop_redo_local(&mut self, pane_id: &PaneId) -> Option<UndoAction> {
        self.pane_rings.get_mut(pane_id)?.pop_redo()
    }

    /// Replace `pane_id`'s most recent local undo entry in place (MT-035 typing-coalescing — RISK-1 /
    /// MC-1). Returns `true` when a tail entry existed and was replaced, `false` when the pane has no
    /// ring or an empty ring (the caller then `push_local`s a fresh entry). See [`PaneUndoRing::replace_tail`].
    pub fn replace_local_tail(&mut self, pane_id: &PaneId, action: UndoAction) -> bool {
        match self.pane_rings.get_mut(pane_id) {
            Some(ring) => ring.replace_tail(action),
            None => false,
        }
    }

    /// Push a cross-pane action onto the single cross-pane ring (POLICY-2).
    pub fn push_cross_pane(&mut self, action: UndoAction) {
        self.cross_pane_ring.push(action);
    }

    /// Pop the most recent cross-pane action for undo (Ctrl+Shift+Z — POLICY-2). The caller invokes it.
    pub fn pop_undo_cross_pane(&mut self) -> Option<UndoAction> {
        self.cross_pane_ring.pop_undo()
    }

    /// Pop the most recently undone cross-pane action for redo.
    pub fn pop_redo_cross_pane(&mut self) -> Option<UndoAction> {
        self.cross_pane_ring.pop_redo()
    }

    /// The number of undoable actions in `pane_id`'s local ring (the "Undo ({n})" indicator — AC-6).
    /// `0` when the pane has no ring yet.
    pub fn local_undo_count(&self, pane_id: &PaneId) -> usize {
        self.pane_rings.get(pane_id).map(|r| r.undo_len()).unwrap_or(0)
    }

    /// The number of undoable cross-pane actions (the cross-pane indicator / inspector).
    pub fn cross_pane_undo_count(&self) -> usize {
        self.cross_pane_ring.undo_len()
    }

    /// True when `pane_id`'s local ring can undo.
    pub fn can_undo_local(&self, pane_id: &PaneId) -> bool {
        self.pane_rings.get(pane_id).map(|r| r.can_undo()).unwrap_or(false)
    }

    /// True when `pane_id`'s local ring can redo.
    pub fn can_redo_local(&self, pane_id: &PaneId) -> bool {
        self.pane_rings.get(pane_id).map(|r| r.can_redo()).unwrap_or(false)
    }

    /// True when the cross-pane ring can undo.
    pub fn can_undo_cross_pane(&self) -> bool {
        self.cross_pane_ring.can_undo()
    }

    /// True when the WHOLE scope holds no actions (every pane ring empty AND the cross-pane ring empty).
    /// The POLICY-3 proof asserts a fresh scope is empty.
    pub fn is_empty(&self) -> bool {
        self.cross_pane_ring.undo_len() == 0
            && self.cross_pane_ring.redo_len() == 0
            && self.pane_rings.values().all(|r| r.undo_len() == 0 && r.redo_len() == 0)
    }

    /// The most recent local + cross-pane entry descriptions for the "Show Undo History" inspector
    /// (MC-5): `(local_descriptions, cross_pane_descriptions)`, newest first, each capped at `n`.
    pub fn history_preview(&self, pane_id: &PaneId, n: usize) -> (Vec<String>, Vec<String>) {
        let local = self
            .pane_rings
            .get(pane_id)
            .map(|r| r.recent_descriptions(n))
            .unwrap_or_default();
        let cross = self.cross_pane_ring.recent_descriptions(n);
        (local, cross)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    fn pane(id: &str) -> PaneId {
        Arc::from(id)
    }

    /// Build a sync action whose undo_fn pushes `tag` onto `log` and whose redo_fn pushes `tag`+"-redo".
    fn logging_action(tag: &'static str, log: Arc<Mutex<Vec<String>>>) -> UndoAction {
        let undo_log = log.clone();
        let redo_log = log;
        UndoAction::sync(
            tag,
            Arc::new(move || {
                undo_log.lock().unwrap().push(tag.to_owned());
                UndoResult::ok()
            }),
            Arc::new(move || {
                redo_log.lock().unwrap().push(format!("{tag}-redo"));
                UndoResult::ok()
            }),
        )
    }

    /// (a) Push 3 actions to a pane ring, undo 3 times, assert all undo_fns called in REVERSE order.
    #[test]
    fn three_pushes_three_undos_reverse_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut scope = UnifiedUndoScope::new();
        let p = pane("pane-code");
        for tag in ["a", "b", "c"] {
            scope.push_local(p.clone(), logging_action(tag, log.clone()));
        }
        assert_eq!(scope.local_undo_count(&p), 3);
        for _ in 0..3 {
            let action = scope.pop_undo_local(&p).expect("an action to undo");
            assert_eq!((action.undo_fn)(), UndoResult::ok());
        }
        assert_eq!(*log.lock().unwrap(), vec!["c", "b", "a"], "undo fires newest-first (reverse)");
        assert!(scope.pop_undo_local(&p).is_none(), "ring is empty after 3 undos");
    }

    /// (b) Redo after undo re-applies in forward order.
    #[test]
    fn redo_after_undo_restores() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut scope = UnifiedUndoScope::new();
        let p = pane("pane-rich");
        scope.push_local(p.clone(), logging_action("x", log.clone()));
        let undo = scope.pop_undo_local(&p).unwrap();
        (undo.undo_fn)();
        assert_eq!(*log.lock().unwrap(), vec!["x"]);
        // Now redo it.
        let redo = scope.pop_redo_local(&p).unwrap();
        (redo.redo_fn)();
        assert_eq!(*log.lock().unwrap(), vec!["x", "x-redo"]);
        assert!(scope.pop_redo_local(&p).is_none(), "nothing left to redo");
    }

    /// (c) POLICY-5: pushing past a cap-5 pane ring drops the oldest (5 entries, not 6).
    #[test]
    fn pane_ring_cap_drops_oldest() {
        let mut ring = PaneUndoRing::with_cap(pane("p"), 5);
        let log = Arc::new(Mutex::new(Vec::new()));
        for tag in ["1", "2", "3", "4", "5", "6"] {
            ring.push(logging_action(Box::leak(tag.to_owned().into_boxed_str()), log.clone()));
        }
        assert_eq!(ring.undo_len(), 5, "cap-5 ring holds 5 after 6 pushes");
        // The oldest ("1") was dropped; the newest 5 remain newest-first.
        assert_eq!(ring.recent_descriptions(5), vec!["6", "5", "4", "3", "2"]);
    }

    /// POLICY-5 at the real contract caps: 201 pushes to a cap-200 pane ring -> 200; 51 to cap-50
    /// cross-pane -> 50.
    #[test]
    fn contract_caps_200_and_50() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut pane_ring = PaneUndoRing::new(pane("p")); // default 200
        for _ in 0..201 {
            pane_ring.push(logging_action("z", log.clone()));
        }
        assert_eq!(pane_ring.undo_len(), 200, "pane ring caps at 200 after 201 pushes");

        let mut cross = CrossPaneUndoRing::new(); // default 50
        for _ in 0..51 {
            cross.push(logging_action("c", log.clone()));
        }
        assert_eq!(cross.undo_len(), 50, "cross-pane ring caps at 50 after 51 pushes");
    }

    /// (d) Cross-pane ring undo invokes the last cross-pane action's undo_fn.
    #[test]
    fn cross_pane_undo_invokes_last() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut scope = UnifiedUndoScope::new();
        scope.push_cross_pane(logging_action("embed", log.clone()));
        scope.push_cross_pane(logging_action("route", log.clone()));
        assert_eq!(scope.cross_pane_undo_count(), 2);
        let action = scope.pop_undo_cross_pane().expect("a cross-pane action");
        assert_eq!(action.description, "route", "the LAST cross-pane action is undone first");
        (action.undo_fn)();
        assert_eq!(*log.lock().unwrap(), vec!["route"]);
    }

    /// POLICY-1 (local-first isolation): undoing in one pane does NOT touch another pane's ring.
    #[test]
    fn local_first_isolation() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut scope = UnifiedUndoScope::new();
        let code = pane("pane-code");
        let rich = pane("pane-rich");
        scope.push_local(code.clone(), logging_action("code-edit", log.clone()));
        scope.push_local(rich.clone(), logging_action("rich-edit", log.clone()));
        // Undo in the code pane: only the code action pops; the rich ring is untouched.
        let action = scope.pop_undo_local(&code).expect("code action");
        assert_eq!(action.description, "code-edit");
        assert_eq!(scope.local_undo_count(&code), 0, "code ring drained");
        assert_eq!(scope.local_undo_count(&rich), 1, "rich ring UNTOUCHED by code undo (POLICY-1)");
    }

    /// A fresh edit (push) after some undos clears the redo stack (linear history, no branch).
    #[test]
    fn fresh_push_clears_redo() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut ring = PaneUndoRing::new(pane("p"));
        ring.push(logging_action("a", log.clone()));
        ring.push(logging_action("b", log.clone()));
        ring.pop_undo(); // undo "b" -> redo stack has "b"
        assert_eq!(ring.redo_len(), 1);
        ring.push(logging_action("c", log.clone())); // fresh edit forks history
        assert_eq!(ring.redo_len(), 0, "a fresh push clears the redo stack");
        assert_eq!(ring.undo_len(), 2, "a and c remain");
    }

    /// POLICY-3: a fresh scope is empty (the AC-3 in-memory assertion; the no-Serialize half is a
    /// compile-time / source proof in the integration test).
    #[test]
    fn fresh_scope_is_empty() {
        let scope = UnifiedUndoScope::new();
        assert!(scope.is_empty(), "a fresh scope holds no undo state (POLICY-3 session-scoped)");
        assert_eq!(scope.local_undo_count(&pane("any")), 0);
        assert_eq!(scope.cross_pane_undo_count(), 0);
    }

    /// RISK-3 / MC-3: a Weak back-ref whose target was dropped yields a benign `pane_dropped` result,
    /// never a panic. (Models the pane-adapter closure pattern.)
    #[test]
    fn weak_back_ref_dropped_is_benign() {
        let pane_state = Arc::new(Mutex::new(String::from("original")));
        let weak = Arc::downgrade(&pane_state);
        let action = UndoAction::sync(
            "edit",
            Arc::new(move || match weak.upgrade() {
                Some(state) => {
                    *state.lock().unwrap() = "restored".to_owned();
                    UndoResult::ok()
                }
                None => UndoResult::pane_dropped(),
            }),
            Arc::new(UndoResult::ok),
        );
        // Drop the pane state BEFORE invoking the undo (the pane closed).
        drop(pane_state);
        let result = (action.undo_fn)();
        assert!(!result.ok, "undo against a dropped pane fails benignly");
        assert_eq!(result, UndoResult::pane_dropped());
    }

    /// POLICY-4: an async-compensating action reports `is_async()` and carries an async undo closure
    /// the bus would dispatch onto the runtime.
    #[test]
    fn async_compensating_action_is_flagged() {
        let calls = Arc::new(AtomicUsize::new(0));
        let undo_calls = calls.clone();
        let action = UndoAction::async_compensating(
            "place canvas block",
            Arc::new(UndoResult::ok),
            Arc::new(UndoResult::ok),
            Arc::new(move || {
                let c = undo_calls.clone();
                Box::pin(async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    UndoResult::ok()
                })
            }),
            Arc::new(|| Box::pin(async { UndoResult::ok() })),
        );
        assert!(action.is_async(), "a canvas compensating action is async (POLICY-4)");
        // Drive the async closure once on a throwaway runtime to prove it is real (not a stub).
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let fut = (action.undo_async_fn.as_ref().unwrap())();
        let res = rt.block_on(fut);
        assert!(res.ok);
        assert_eq!(calls.load(Ordering::SeqCst), 1, "the async compensating closure actually ran");
    }
}
