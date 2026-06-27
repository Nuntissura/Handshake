//! The **Diagnostics** Settings section for the native Handshake shell (WP-KERNEL-012 MT-087).
//!
//! ## What this module owns
//!
//! The in-app Diagnostics Panel (Master Spec v02.196 §5.8.4 + §10.12.5 three-tier model) hosted as a
//! SECTION inside the existing WP-011 [`crate::settings_dialog`] dialog — opened via Settings ->
//! Diagnostics — NOT a docked worksurface pane.
//!
//! Operator steer (2026-06-27): diagnostics lives in the OPTIONS/SETTINGS window, not the worksurface
//! (the worksurface is stripped to notes-only in MT-097). So this MT renders the panel widget
//! ([`crate::diagnostics::DiagnosticsPanel`]) as a settings section, mirroring the
//! [`crate::settings_editor_section`] pattern: a thin RENDER-ONLY wrapper over read-only inputs the
//! shell supplies, returning NOTHING the shell must apply (the panel is a pure PROJECTION of live
//! `internal_diagnostics` state — §5.8.4 — so it has no settings to mutate or persist).
//!
//! ## Why this is render-only (no outcome, no persistence)
//!
//! Unlike the editor sections (which mutate + persist `WorkspaceSettingsState`), the Diagnostics
//! section is a READ-ONLY observability surface: it shows the live heartbeat / frame-time / resource /
//! last-N events / Tier-3 Palmistry placeholder and changes NO setting. There is therefore no
//! `SettingsOutcome` variant for it — [`render`] just draws the panel and returns. The live data it
//! projects flows in through the read-only [`crate::diagnostics::DiagnosticsView`] the shell rebuilds
//! each frame from the `HandshakeApp` producers (and the panel reads the last-N events directly from
//! the process-global recorder), so the section holds NO own authority (RISK-007-2).
//!
//! ## AccessKit (HBR-VIS / HBR-SWARM — out-of-process steering)
//!
//! The panel widget owns its own stable author_ids (`diagnostics_panel` Region + the section Groups),
//! so this section is a thin host: it renders the panel and the panel emits the AccessKit subtree. A
//! no-context model + swarm agents address the surface by `diagnostics_panel` and the child section ids.

use crate::diagnostics::{DiagnosticsPanel, DiagnosticsView};
use crate::theme::HsPalette;

/// The search keywords that surface the Diagnostics section in the settings search box (so a model /
/// operator typing "diagnostics", "frame", "heartbeat", "cpu", "palmistry"... finds it). Exposed so the
/// dialog's `setting_matches_query` call uses the SAME list the section advertises.
pub const DIAGNOSTICS_SEARCH_KEYWORDS: &[&str] = &[
    "diagnostics",
    "diagnostic",
    "health",
    "heartbeat",
    "frame",
    "frametime",
    "fps",
    "cpu",
    "rss",
    "memory",
    "gpu",
    "resource",
    "events",
    "palmistry",
    "freeze",
    "crash",
    "internal",
];

/// Read-only inputs the Diagnostics section renders from: the live diagnostics view (the shell rebuilds
/// it each frame from the `HandshakeApp` producers) + the active palette (so the panel reads theme
/// tokens, never a literal — CONTROL-4). The section never borrows `&mut` app state.
pub struct DiagnosticsSettingsView<'a> {
    /// The live `internal_diagnostics` projection the shell built this frame (heartbeat / frame-stats /
    /// resource / GPU / dropped / ring-status). The last-N events are read by the panel directly from
    /// the process-global recorder (the true projection), not carried here.
    pub diagnostics: &'a DiagnosticsView,
    /// The active resolved palette (theme tokens) the panel paints with.
    pub palette: &'a HsPalette,
}

/// Render the Diagnostics section: just host the [`DiagnosticsPanel`] widget over the read-only view.
/// The panel emits its own AccessKit subtree (`diagnostics_panel` Region + child Groups) and projects
/// the live state. Returns nothing — the section changes no setting (pure observability).
pub fn render(ui: &mut egui::Ui, view: &DiagnosticsSettingsView<'_>) {
    DiagnosticsPanel.show(ui, view.diagnostics, view.palette);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_keywords_include_the_obvious_terms() {
        for term in ["diagnostics", "heartbeat", "frame", "cpu", "palmistry", "events"] {
            assert!(
                DIAGNOSTICS_SEARCH_KEYWORDS.contains(&term),
                "the Diagnostics section must be findable by '{term}'"
            );
        }
    }
}
