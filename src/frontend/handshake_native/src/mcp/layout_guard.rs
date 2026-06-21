//! Restartable layout guard for agent-driven pane operations (WP-KERNEL-011 MT-028).
//!
//! When a swarm agent drives a pane-layout change (split, move, pop-out, divider drag), a conflicting
//! concurrent change — by another agent OR by a torn intermediate state — could corrupt the layout.
//! [`LayoutGuard`] checkpoints the FULL layout BEFORE an agent-driven layout op and can ROLL IT BACK on
//! conflict, restoring the pre-op arrangement without panicking.
//!
//! ## What is checkpointed: the real [`LayoutSnapshot`], cloned (NOT `egui_tiles::Tree`)
//!
//! The MT-028 contract sketched a `LayoutGuard` over an `egui_tiles::Tree`. The shell AS BUILT does NOT
//! use `egui_tiles::Tree` for its work-surface layout — [`crate::split_layout`] is a hand-rolled 2x2
//! splitter, and the authoritative, serializable layout state is [`crate::layout_persistence::
//! LayoutSnapshot`] (split fractions + pane registry + per-pane tab bars + pop-out geometry + drawer
//! flags), captured via `HandshakeApp::capture_layout_snapshot()` and restored via
//! `apply_layout_snapshot()`. So the guard checkpoints a CLONE of that real `LayoutSnapshot` — which the
//! contract's own implementation note explicitly prefers ("Prefer clone over serialize") — and rollback
//! returns the checkpoint for the host to re-apply through its existing validated `apply_layout_snapshot`
//! path. This builds the guarantee on the real layout authority instead of a type the shell does not use.
//!
//! ## Agent-driven only — the operator never rolls back
//!
//! A [`LayoutGuard`] is created ONLY around an AGENT-driven layout op. Operator UI actions (the human
//! dragging a divider) are never wrapped and never rolled back — the operator's direct manipulation is
//! always authoritative (the contract's explicit rule). The guard is the swarm-safety net, not a general
//! undo.
//!
//! ## `#[must_use]` + drop discipline
//!
//! The guard is `#[must_use]`: the compiler warns if it is created but neither [`LayoutGuard::commit`]
//! nor [`LayoutGuard::into_rollback`] is called. On `Drop` WITHOUT an explicit decision, the guard does
//! NOT auto-rollback (drop timing is non-deterministic and an auto-rollback could clobber a later valid
//! state); it logs a warning so a forgotten decision is visible. The caller is expected to make the
//! commit/rollback decision explicitly at the end of the op.

use crate::layout_persistence::LayoutSnapshot;

/// A checkpoint of the layout taken before an agent-driven layout op, with explicit commit/rollback.
///
/// Hold one for the span of ONE agent-driven layout mutation:
///
/// ```ignore
/// let guard = LayoutGuard::checkpoint(app.capture_layout_snapshot());
/// // ... agent-driven layout op ...
/// if conflict_detected {
///     let prior = guard.into_rollback();          // restore the pre-op layout
///     app.apply_layout_snapshot(prior, extent)?;  // through the existing validated path
/// } else {
///     guard.commit();                              // keep the new layout
/// }
/// ```
#[must_use = "a LayoutGuard must be explicitly commit()ted or rolled back; dropping it without a \
              decision leaves the layout in whatever (possibly intermediate) state the op left it"]
pub struct LayoutGuard {
    /// The pre-op layout clone. `Some` until `commit`/`into_rollback` consumes it; used by `Drop` to warn
    /// if the guard was forgotten.
    checkpoint: Option<LayoutSnapshot>,
}

impl LayoutGuard {
    /// Take a checkpoint of the current layout before an agent-driven op. Pass the snapshot captured from
    /// the live shell (`HandshakeApp::capture_layout_snapshot()`).
    pub fn checkpoint(current: LayoutSnapshot) -> Self {
        Self { checkpoint: Some(current) }
    }

    /// The op succeeded — discard the checkpoint; the new layout is authoritative. Consumes the guard so
    /// the `#[must_use]` obligation is satisfied and no rollback can follow a commit.
    pub fn commit(mut self) {
        // Take the checkpoint so Drop sees `None` and does not log a "forgotten guard" warning.
        let _ = self.checkpoint.take();
    }

    /// The op conflicted — return the pre-op layout so the host can re-apply it through its existing
    /// validated `apply_layout_snapshot` path. Consumes the guard. The returned snapshot is the exact
    /// clone taken at [`Self::checkpoint`] time, so re-applying it restores the prior arrangement.
    #[must_use = "the returned snapshot must be re-applied via apply_layout_snapshot to actually roll back"]
    pub fn into_rollback(mut self) -> LayoutSnapshot {
        self.checkpoint
            .take()
            .expect("checkpoint is present until commit/into_rollback consumes it exactly once")
    }

    /// Read-only peek at the checkpoint (tests / diagnostics). `None` only after commit/into_rollback.
    pub fn checkpoint_ref(&self) -> Option<&LayoutSnapshot> {
        self.checkpoint.as_ref()
    }
}

impl Drop for LayoutGuard {
    fn drop(&mut self) {
        // A forgotten guard (neither committed nor rolled back) is a coder bug surfaced here, NOT an
        // auto-rollback: drop timing is non-deterministic in an async context, so silently restoring the
        // checkpoint could clobber a later valid state. Warn loudly instead (the contract's drop rule).
        if self.checkpoint.is_some() {
            tracing::warn!(
                "LayoutGuard dropped without commit() or into_rollback(): the layout is left in \
                 whatever state the agent op produced. This is a coder bug — make the commit/rollback \
                 decision explicitly at the end of the op."
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_persistence::{LayoutSnapshot, CANONICAL_PANE_IDS};
    use crate::module_switcher::ModuleId;
    use crate::pane_registry::{
        DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
    };
    use crate::split_layout::SplitWeights;
    use crate::tab_bar::{TabBarState, TabState};
    use std::collections::BTreeMap;

    fn pid(s: &str) -> PaneId {
        std::sync::Arc::from(s)
    }

    /// A minimal valid layout snapshot with a distinguishing split-weight value so before/after can be
    /// compared. Uses the real `LayoutSnapshot::new` so the checkpoint is the genuine layout authority.
    ///
    /// Seeds ALL FOUR canonical panes (`pane-a`..`pane-d`) with minimal valid pane records + tab bars so
    /// the snapshot passes the pane-completeness gate in `LayoutSnapshot::validate` (MT-009 remediation):
    /// a snapshot with empty `panes` would now be (correctly) rejected as structurally corrupt.
    fn layout_with_vertical_fraction(vertical: f32) -> LayoutSnapshot {
        let weights = SplitWeights { vertical, ..SplitWeights::default() };

        let mut panes = BTreeMap::new();
        let mut tab_bars = BTreeMap::new();
        for id in CANONICAL_PANE_IDS {
            panes.insert(
                pid(id),
                PaneRecord::new(
                    pid(id),
                    PaneType::Workspace,
                    "proj-1",
                    None,
                    LockState::Unlocked,
                    DirtyState::Clean,
                    PaneAuthority::System,
                ),
            );
            tab_bars.insert(
                pid(id),
                TabBarState::new(pid(id), vec![TabState::new(PaneType::Workspace)]),
            );
        }

        LayoutSnapshot::new(
            "proj-1",
            weights,
            None,
            ModuleId::Main,
            panes,
            tab_bars,
            BTreeMap::new(),
        )
    }

    #[test]
    fn rollback_returns_the_exact_checkpoint() {
        let before = layout_with_vertical_fraction(0.25);
        let guard = LayoutGuard::checkpoint(before.clone());
        // Simulate an agent op having mutated the live layout to a different fraction; rollback returns
        // the PRE-OP snapshot so re-applying it restores the prior arrangement.
        let restored = guard.into_rollback();
        assert_eq!(
            restored.split_weights.vertical, before.split_weights.vertical,
            "rollback restores the pre-op split fraction"
        );
        // And the restored snapshot still validates (so apply_layout_snapshot will accept it).
        assert!(restored.validate().is_ok(), "rolled-back snapshot is still valid");
    }

    #[test]
    fn commit_discards_checkpoint_without_panicking() {
        let guard = LayoutGuard::checkpoint(layout_with_vertical_fraction(0.4));
        guard.commit(); // consumes; Drop sees None -> no warning, no panic.
    }

    #[test]
    fn checkpoint_ref_exposes_pre_op_state_before_decision() {
        let guard = LayoutGuard::checkpoint(layout_with_vertical_fraction(0.33));
        let peek = guard.checkpoint_ref().expect("checkpoint present before decision");
        assert!((peek.split_weights.vertical - 0.33).abs() < f32::EPSILON);
        guard.commit();
    }
}
