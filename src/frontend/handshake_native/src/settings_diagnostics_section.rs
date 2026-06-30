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
use crate::visual_debugger::{
    WORKSURFACE_INSPECTOR_DUMP_BUTTON_AUTHOR_ID, WORKSURFACE_INSPECTOR_STATUS_AUTHOR_ID,
};

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
    "child",
    "stall",
    "hang",
    "watchdog",
    "internal",
    "visual",
    "debugger",
    "inspector",
    "worksurface",
    "widget",
    "layout",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticsSectionOutcome {
    None,
    WorksurfaceInspectorDumpRequested,
}

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
    /// MT-102 Visual Debugger: transient last-dump status owned by the shell.
    pub worksurface_inspector_last_dump: Option<&'a str>,
}

/// Render the Diagnostics section: just host the [`DiagnosticsPanel`] widget over the read-only view.
/// The panel emits its own AccessKit subtree (`diagnostics_panel` Region + child Groups) and projects
/// the live state. Returns nothing — the section changes no setting (pure observability).
pub fn render(ui: &mut egui::Ui, view: &DiagnosticsSettingsView<'_>) -> DiagnosticsSectionOutcome {
    DiagnosticsPanel.show(ui, view.diagnostics, view.palette);

    ui.separator();
    let mut outcome = DiagnosticsSectionOutcome::None;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Worksurface Inspector").strong());
        let dump = ui.button("Dump JSON");
        set_author_id_and_label(
            ui,
            dump.id,
            WORKSURFACE_INSPECTOR_DUMP_BUTTON_AUTHOR_ID,
            "Dump worksurface snapshot JSON",
        );
        if dump.clicked() {
            outcome = DiagnosticsSectionOutcome::WorksurfaceInspectorDumpRequested;
        }
    });

    let status = view
        .worksurface_inspector_last_dump
        .unwrap_or("No worksurface inspector dump yet.");
    let response = ui.label(status);
    set_author_id_and_label(
        ui,
        response.id,
        WORKSURFACE_INSPECTOR_STATUS_AUTHOR_ID,
        status,
    );

    outcome
}

fn set_author_id_and_label(ui: &egui::Ui, widget_id: egui::Id, author_id: &str, label: &str) {
    let author_id = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(widget_id, move |node| {
        node.set_author_id(author_id);
        node.set_label(label);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_keywords_include_the_obvious_terms() {
        for term in [
            "diagnostics",
            "heartbeat",
            "frame",
            "cpu",
            "palmistry",
            "events",
            "stall",
            "hang",
            "visual",
            "debugger",
            "inspector",
        ] {
            assert!(
                DIAGNOSTICS_SEARCH_KEYWORDS.contains(&term),
                "the Diagnostics section must be findable by '{term}'"
            );
        }
    }
}
