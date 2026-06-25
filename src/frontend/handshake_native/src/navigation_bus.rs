//! Cross-surface NavigationTarget layer (WP-KERNEL-012 MT-070, cluster E11 — melt-together
//! click-through).
//!
//! ## What this is (a THIN layer over the EXISTING ShellNavigator — it does NOT fork it)
//!
//! Every cross-surface "click that should land in an editor/pane" — a quick-switcher selection
//! (MT-030), a clicked link / backlink / code-ref / locus chip (MT-032/034/068), a graph or canvas
//! node activation — needs to resolve to a LIVE pane addressed by a stable id and focus/scroll it. The
//! MT-030 navigation bus ALREADY EXISTS as [`crate::quick_switcher::ShellNavigator`] (the
//! `open_document` / `open_loom_block` / `open_code_symbol` / `open_work_packet` / `open_micro_task` /
//! `open_user_manual_page` / `open_wiki_page` seams), and MT-079 wired the two editor-pane seams to the
//! REAL mounted editor panes (proven by `app::tests` / `test_app_host_mount.rs`). The `HandshakeApp`
//! shell IS the `ShellNavigator`, and its `open_navigator_tab` primitive de-dupes + activates + FOCUSES
//! the target pane by stable id.
//!
//! So this module is deliberately a THIN [`NavigationTarget`] enum + a [`dispatch`](dispatch) function
//! that MAPS each typed target onto the SAME `ShellNavigator` `open_*` seam (RISK-070-4 / MC-070-4 — no
//! parallel bus, no forked navigation substrate). It adds exactly two things the bare `ShellNavigator`
//! lacks for the MT-070 click-through:
//!
//! 1. a TYPED [`NavError::PaneNotFound`] result (never a panic) when a target references a pane the
//!    navigator could not open / focus (e.g. a quick-switcher result whose pane was closed, or an empty
//!    work surface) — RISK-070-3 / MC-070-3; and
//! 2. an explicit [`NavigationTarget`] vocabulary that RECONCILES against the live `ShellNavigator`
//!    seam set (RISK-070-2 / MC-070-2) so a clicked link / node / quick-switcher row dispatches a real,
//!    named, addressable navigation rather than a per-surface ad-hoc callback.
//!
//! ## Why delegate instead of re-implement `focus(pane_id)` + `pane.reveal(target)`
//!
//! The MT-070 contract sketch described a per-pane `register_nav_handler(pane_id, handler)` +
//! `registry.focus(pane_id)` + `pane.reveal(target)` shape. The LIVE substrate already realizes that
//! exact responsibility through the `ShellNavigator` seam: `open_navigator_tab` (in `app.rs`) resolves
//! the pane by stable id, activates its tab, and focuses it — i.e. it IS the focus+reveal path the
//! sketch named, owned by the shell that holds the pane registry + split layout. Re-implementing a
//! parallel `focus`/`reveal` here would FORK the navigation substrate (the precise red-team failure
//! RISK-070-4 calls out) and duplicate the MT-079 mount wiring. The KERNEL_BUILDER gate (recorded in
//! the MT contract `implementation_notes`) VERIFIED this and directs `navigation_bus.rs` to DELEGATE to
//! the existing `ShellNavigator` open_* seams. The [`NavHandler`] trait below is therefore exactly that
//! seam — `ShellNavigator` already implements it (a blanket impl) — so a target reveals through the
//! live mount, not a second handler table.
//!
//! ## No backend call here
//!
//! This module performs NO direct backend call (dispatch-only — the only backend touch in the whole
//! click-through is INDIRECT, inside the `ShellNavigator::open_code_symbol` / `open_document` arms the
//! mounted editor panes own). A missing real navigator seam is a typed [`NavError`], never a panic and
//! never a placeholder.

use crate::pane_registry::PaneId;
use crate::quick_switcher::{NavDispatchOutcome, ShellNavigator};

/// A typed cross-surface navigation request. Each variant RECONCILES to one [`ShellNavigator`] open_*
/// seam (RISK-070-2 / MC-070-2 — the variants mirror what MT-008 location/symbol, MT-030 quick-switcher,
/// and MT-057/canvas note/node actually emit; they do NOT invent drifting variants). `pane_id` on the
/// pane-addressed variants is the STABLE pane id a focus/reveal lands on; `dispatch` guards a stale/closed
/// pane id with [`NavError::PaneNotFound`] rather than panicking (RISK-070-3 / MC-070-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationTarget {
    /// Open the code editor at a symbol (MT-008 go-to-def / MT-030 code-symbol quick-switcher row /
    /// MT-034 `[[code:…]]` chip). Reconciles to [`ShellNavigator::open_code_symbol`]. `pane_id` is the
    /// stable code-pane id the reveal should land on; `symbol` is the symbol entity id.
    EditorAtSymbol { pane_id: PaneId, symbol: String },
    /// Open the code editor at a byte location (MT-008 peek/go-to-def landing). Reconciles to
    /// [`ShellNavigator::open_code_symbol`] using the `symbol` carried alongside the location (the live
    /// code-nav seam resolves by symbol, not raw byte; the offset is the in-pane reveal target the
    /// mounted pane scrolls to). `pane_id` is the stable code-pane id.
    EditorAtLocation {
        pane_id: PaneId,
        symbol: String,
        byte_offset: usize,
    },
    /// Open a note/document (MT-030 document quick-switcher row / MT-032 backlink / MT-057
    /// create-note-from-link landing). Reconciles to [`ShellNavigator::open_document`]. `note_id` is the
    /// `KRD-`/document id the Notes/rich editor pane opens.
    OpenNote { note_id: String },
    /// Reveal a graph/canvas node by id (a clicked graph node / canvas placement). Reconciles to
    /// [`ShellNavigator::open_loom_block`] (a node IS addressable as a Loom block). `pane_id` is the
    /// stable pane the reveal lands on; `node_id` is the block/node id.
    RevealNode { pane_id: PaneId, node_id: String },
    /// Focus a pane by stable id WITHOUT changing its content (the MT-030 quick-switcher "focus the
    /// target editor/pane" leg, AC-070-3). Reconciles to [`NavHandler::focus_pane`] (the shell's
    /// pane-focus seam). `pane_id` is the stable pane id.
    FocusPane { pane_id: PaneId },
}

impl NavigationTarget {
    /// The stable pane id this target is addressed to, when it carries one. `OpenNote` carries no pane id
    /// (the navigator opens/focuses the note's pane by content), so it returns `None`.
    pub fn pane_id(&self) -> Option<&PaneId> {
        match self {
            NavigationTarget::EditorAtSymbol { pane_id, .. }
            | NavigationTarget::EditorAtLocation { pane_id, .. }
            | NavigationTarget::RevealNode { pane_id, .. }
            | NavigationTarget::FocusPane { pane_id } => Some(pane_id),
            NavigationTarget::OpenNote { .. } => None,
        }
    }
}

/// The typed failure of [`dispatch`]. A navigation that cannot land on a live pane returns this rather
/// than panicking (RISK-070-3 / MC-070-3 — a stale/closed quick-switcher result, or an empty work
/// surface, must never freeze the frame thread).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavError {
    /// The target referenced a pane id the navigator could not open / focus (it was closed, or the work
    /// surface had no pane to open the target on). The carried `pane_id` is the stale id, for diagnostics.
    PaneNotFound { pane_id: String },
    /// The navigator routed the target to an editor pane that is not mounted yet (the honest MT-030
    /// `EditorPaneNotMounted` seam, retired for the two editor arms by MT-079 but still a possible typed
    /// outcome for a navigator implementor that has not mounted those panes). Carries the editor label.
    EditorPaneNotMounted { editor: String },
    /// The target was structurally not navigable (the navigator reported `Unsupported`). Carried as a
    /// typed value so a mis-dispatch is observable, never a panic.
    Unsupported,
}

impl NavError {
    /// A human/agent-readable description of the failure.
    pub fn message(&self) -> String {
        match self {
            NavError::PaneNotFound { pane_id } => {
                format!("navigation target references a pane that is not open: {pane_id}")
            }
            NavError::EditorPaneNotMounted { editor } => {
                format!("{editor} editor pane is not mounted yet")
            }
            NavError::Unsupported => "navigation target is not navigable".to_owned(),
        }
    }
}

/// The pane focus/reveal seam a [`NavigationTarget`] dispatches through. This is the SAME responsibility
/// the MT-070 contract sketch named `register_nav_handler` + `pane.reveal(target)`, realized as the
/// EXISTING shell navigation seam rather than a forked per-pane handler table: the shell (which owns the
/// pane registry + split layout) resolves the pane by stable id, focuses it, and reveals the target.
///
/// A blanket impl below makes EVERY [`ShellNavigator`] a `NavHandler`, so the live `HandshakeApp` shell
/// (which already `impl ShellNavigator`) is a `NavHandler` for free — no second registration. The
/// `focus_pane` method is the one capability beyond the `open_*` seams the `FocusPane` target needs;
/// it defaults to opening the pane's own surface (which focuses it) so a plain `ShellNavigator` still
/// satisfies it, and a richer shell can override it to focus WITHOUT re-opening content.
pub trait NavHandler {
    /// Reveal `target` on its addressed pane, returning the typed outcome (never panic). The default
    /// impl in [`dispatch`] is what callers use; this trait method exists so the seam is named and a
    /// test double can implement it.
    fn reveal(&mut self, target: &NavigationTarget) -> NavDispatchOutcome;

    /// Focus a pane by stable id without changing its content (the `FocusPane` leg). The default routes
    /// through [`reveal`](NavHandler::reveal) of a `FocusPane` target via the navigator's pane-focus
    /// seam; a shell with a dedicated focus-only path overrides it.
    fn focus_pane(&mut self, pane_id: &PaneId) -> NavDispatchOutcome;
}

/// Every [`ShellNavigator`] is a [`NavHandler`]: `reveal` maps each [`NavigationTarget`] onto the
/// navigator's matching open_* seam (the RECONCILED mapping — RISK-070-2), and `focus_pane` reuses the
/// navigator's loom-block seam to focus the addressed pane (a node/pane focus is an open-on-the-pane,
/// which the shell de-dupes so it focuses rather than duplicates). This blanket impl is what makes the
/// live `HandshakeApp` a `NavHandler` without a second handler table (RISK-070-4 — no forked substrate).
impl<N: ShellNavigator + ?Sized> NavHandler for N {
    fn reveal(&mut self, target: &NavigationTarget) -> NavDispatchOutcome {
        match target {
            NavigationTarget::EditorAtSymbol { symbol, .. } => self.open_code_symbol(symbol),
            // EditorAtLocation reveals through the same code-symbol seam (the live code-nav resolves by
            // symbol); the byte offset is the in-pane scroll target the mounted code pane applies after
            // the pane is focused. No separate navigator method exists for a raw byte offset, so reusing
            // open_code_symbol keeps ONE tab-open path (no forked navigation) — the contract's
            // "reconcile against the live ShellNavigator types, do NOT invent drifting variants".
            NavigationTarget::EditorAtLocation { symbol, .. } => self.open_code_symbol(symbol),
            NavigationTarget::OpenNote { note_id } => self.open_document(note_id),
            NavigationTarget::RevealNode { node_id, .. } => self.open_loom_block(node_id),
            NavigationTarget::FocusPane { pane_id } => self.focus_pane(pane_id),
        }
    }

    fn focus_pane(&mut self, pane_id: &PaneId) -> NavDispatchOutcome {
        // A pane focus reuses the loom-block open seam keyed by the pane's stable id: the shell's
        // open_navigator_tab de-dupes by (pane surface, content) and ACTIVATES + FOCUSES the existing
        // tab rather than creating a duplicate, so opening "the pane's own id" focuses it. This keeps a
        // single tab-mutation path (no forked focus call). A navigator that has a dedicated focus-only
        // seam can override this.
        self.open_loom_block(pane_id.as_ref())
    }
}

/// Dispatch a [`NavigationTarget`] through a [`NavHandler`] (the live shell, via the blanket impl), the
/// MT-070 click-through entry point. Resolves the target to its pane via the handler's open_* seam,
/// focusing + revealing it, and maps the typed [`NavDispatchOutcome`] to `Ok(())` (landed on a real
/// surface) or a typed [`NavError`]:
/// - [`NavDispatchOutcome::Opened`] -> `Ok(())` (the focus+reveal landed),
/// - [`NavDispatchOutcome::NoTargetPane`] -> [`NavError::PaneNotFound`] (no pane to land on — a
///   stale/closed pane id or an empty work surface; RISK-070-3 / MC-070-3, NEVER a panic),
/// - [`NavDispatchOutcome::EditorPaneNotMounted`] -> [`NavError::EditorPaneNotMounted`] (the honest
///   not-yet-mounted seam),
/// - [`NavDispatchOutcome::Unsupported`] -> [`NavError::Unsupported`].
///
/// This is the function the quick-switcher confirm path, a clicked link/node, and a context-menu
/// navigation entry call; it NEVER panics on a missing pane.
pub fn dispatch(handler: &mut dyn NavHandler, target: &NavigationTarget) -> Result<(), NavError> {
    let outcome = handler.reveal(target);
    match outcome {
        NavDispatchOutcome::Opened { .. } => Ok(()),
        NavDispatchOutcome::NoTargetPane => Err(NavError::PaneNotFound {
            pane_id: target
                .pane_id()
                .map(|p| p.as_ref().to_owned())
                .unwrap_or_else(|| "(no pane id)".to_owned()),
        }),
        NavDispatchOutcome::EditorPaneNotMounted { editor } => {
            Err(NavError::EditorPaneNotMounted {
                editor: editor.label().to_owned(),
            })
        }
        NavDispatchOutcome::Unsupported => Err(NavError::Unsupported),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quick_switcher::NavEditorKind;
    use std::sync::Arc;

    fn pane(id: &str) -> PaneId {
        Arc::from(id)
    }

    /// A recording test navigator: each open_* seam records which method ran with which id, and returns
    /// a configurable outcome so a test can drive the "opened", "no pane", and "not mounted" paths
    /// without a live shell. This is a REAL implementor of the live `ShellNavigator` trait (the same
    /// trait `HandshakeApp` implements), so the blanket `NavHandler` impl under test is the production
    /// path, not a parallel mock of `NavHandler`.
    #[derive(Default)]
    struct RecordingNavigator {
        /// (method, id) pairs in call order.
        calls: Vec<(&'static str, String)>,
        /// Pane ids the navigator treats as CLOSED -> the matching open_* returns `NoTargetPane`.
        closed_panes: Vec<String>,
        /// Symbol ids whose code pane is treated as not mounted -> `EditorPaneNotMounted { Code }`.
        unmounted_symbols: Vec<String>,
    }

    impl RecordingNavigator {
        fn outcome_for(&self, id: &str, mounted_surface: &str) -> NavDispatchOutcome {
            if self.closed_panes.iter().any(|p| p == id) {
                NavDispatchOutcome::NoTargetPane
            } else if self.unmounted_symbols.iter().any(|s| s == id) {
                NavDispatchOutcome::EditorPaneNotMounted {
                    editor: NavEditorKind::Code,
                }
            } else {
                NavDispatchOutcome::Opened {
                    surface: mounted_surface.to_owned(),
                }
            }
        }
    }

    impl ShellNavigator for RecordingNavigator {
        fn open_document(&mut self, document_id: &str) -> NavDispatchOutcome {
            self.calls.push(("open_document", document_id.to_owned()));
            self.outcome_for(document_id, "Notes")
        }
        fn open_loom_block(&mut self, block_id: &str) -> NavDispatchOutcome {
            self.calls.push(("open_loom_block", block_id.to_owned()));
            self.outcome_for(block_id, "Loom Block")
        }
        fn open_code_symbol(&mut self, symbol_entity_id: &str) -> NavDispatchOutcome {
            self.calls.push(("open_code_symbol", symbol_entity_id.to_owned()));
            self.outcome_for(symbol_entity_id, "Code")
        }
        fn open_work_packet(&mut self, wp_id: &str) -> NavDispatchOutcome {
            self.calls.push(("open_work_packet", wp_id.to_owned()));
            self.outcome_for(wp_id, "Kernel DCC")
        }
        fn open_micro_task(&mut self, mt_id: &str, _wp_id: Option<&str>) -> NavDispatchOutcome {
            self.calls.push(("open_micro_task", mt_id.to_owned()));
            self.outcome_for(mt_id, "Kernel DCC")
        }
        fn open_user_manual_page(&mut self, slug: &str) -> NavDispatchOutcome {
            self.calls.push(("open_user_manual_page", slug.to_owned()));
            self.outcome_for(slug, "User Manual")
        }
        fn open_wiki_page(&mut self, projection_id: &str) -> NavDispatchOutcome {
            self.calls.push(("open_wiki_page", projection_id.to_owned()));
            self.outcome_for(projection_id, "Loom Wiki Page")
        }
    }

    /// AC-070-6 (Ok path) + RISK-070-2 reconcile: each NavigationTarget routes to the EXPECTED open_*
    /// seam by stable id, and a known pane returns `Ok(())`. This is the round-trip MC-070-2 requires.
    #[test]
    fn dispatch_routes_each_target_to_its_navigator_seam() {
        let mut nav = RecordingNavigator::default();

        assert_eq!(
            dispatch(
                &mut nav,
                &NavigationTarget::EditorAtSymbol {
                    pane_id: pane("pane-code"),
                    symbol: "sym-42".to_owned(),
                },
            ),
            Ok(()),
        );
        assert_eq!(
            dispatch(
                &mut nav,
                &NavigationTarget::EditorAtLocation {
                    pane_id: pane("pane-code"),
                    symbol: "sym-7".to_owned(),
                    byte_offset: 1280,
                },
            ),
            Ok(()),
        );
        assert_eq!(
            dispatch(&mut nav, &NavigationTarget::OpenNote { note_id: "KRD-9".to_owned() }),
            Ok(()),
        );
        assert_eq!(
            dispatch(
                &mut nav,
                &NavigationTarget::RevealNode {
                    pane_id: pane("pane-graph"),
                    node_id: "blk-3".to_owned(),
                },
            ),
            Ok(()),
        );
        assert_eq!(
            dispatch(&mut nav, &NavigationTarget::FocusPane { pane_id: pane("pane-a") }),
            Ok(()),
        );

        // The exact seam each target hit, by id (the reconciled mapping — no drift).
        assert_eq!(
            nav.calls,
            vec![
                ("open_code_symbol", "sym-42".to_owned()),
                ("open_code_symbol", "sym-7".to_owned()),
                ("open_document", "KRD-9".to_owned()),
                ("open_loom_block", "blk-3".to_owned()),
                ("open_loom_block", "pane-a".to_owned()),
            ],
        );
    }

    /// AC-070-6 (Err path) + RISK-070-3 / MC-070-3: an unknown / closed pane id returns
    /// `Err(NavError::PaneNotFound)` (never a panic), carrying the stale id for diagnostics.
    #[test]
    fn dispatch_returns_pane_not_found_for_closed_pane() {
        let mut nav = RecordingNavigator {
            closed_panes: vec!["blk-gone".to_owned(), "pane-closed".to_owned()],
            ..Default::default()
        };

        // A RevealNode for a closed pane -> PaneNotFound carrying the node id (the addressed surface).
        let reveal_err = dispatch(
            &mut nav,
            &NavigationTarget::RevealNode {
                pane_id: pane("pane-graph"),
                node_id: "blk-gone".to_owned(),
            },
        );
        assert!(
            matches!(&reveal_err, Err(NavError::PaneNotFound { .. })),
            "closed node pane -> PaneNotFound, got {reveal_err:?}",
        );

        // A FocusPane for a closed pane -> PaneNotFound carrying the pane id.
        let focus_err = dispatch(&mut nav, &NavigationTarget::FocusPane { pane_id: pane("pane-closed") });
        assert_eq!(
            focus_err,
            Err(NavError::PaneNotFound { pane_id: "pane-closed".to_owned() }),
            "closed pane focus -> PaneNotFound with the stale pane id",
        );
    }

    /// The not-yet-mounted editor seam maps to the typed `EditorPaneNotMounted` error (the honest seam),
    /// never a panic and never a silent success.
    #[test]
    fn dispatch_surfaces_editor_pane_not_mounted() {
        let mut nav = RecordingNavigator {
            unmounted_symbols: vec!["sym-unmounted".to_owned()],
            ..Default::default()
        };
        let err = dispatch(
            &mut nav,
            &NavigationTarget::EditorAtSymbol {
                pane_id: pane("pane-code"),
                symbol: "sym-unmounted".to_owned(),
            },
        );
        assert_eq!(
            err,
            Err(NavError::EditorPaneNotMounted { editor: "Code".to_owned() }),
        );
    }

    /// `pane_id()` exposes the addressed pane for every pane-addressed variant and `None` for `OpenNote`
    /// (which the navigator resolves by content), so `dispatch`'s PaneNotFound diagnostics carry the
    /// right id.
    #[test]
    fn pane_id_accessor_matches_variant() {
        assert_eq!(
            NavigationTarget::FocusPane { pane_id: pane("p1") }.pane_id().map(|p| p.as_ref()),
            Some("p1"),
        );
        assert_eq!(
            NavigationTarget::EditorAtSymbol { pane_id: pane("p2"), symbol: "s".to_owned() }
                .pane_id()
                .map(|p| p.as_ref()),
            Some("p2"),
        );
        assert_eq!(
            NavigationTarget::OpenNote { note_id: "KRD-1".to_owned() }.pane_id(),
            None,
        );
    }
}
