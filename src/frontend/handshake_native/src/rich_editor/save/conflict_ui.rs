//! The conflict-resolution window, the draft-recovery banner, the export format picker, and the
//! mockable file-save sink for the rich-text editor (WP-KERNEL-012 MT-020).
//!
//! Port of the `RichDocumentView.tsx` conflict/merge + draft-recovery UI. All surfaces:
//! - reuse the WP-011 [`crate::theme`] palette (no hardcoded hex), and
//! - register their interactive nodes through the EXISTING WP-011 accessibility hook
//!   (`ctx.accesskit_node_builder`), with the EXACT author_ids the MT contract names — NO new
//!   AccessKit registry is invented.
//!
//! ## AccessKit author_ids (the contract ids)
//!
//! - conflict window root: `conflict-dialog`
//! - Keep yours button:    `conflict-keep-yours`
//! - Keep server button:   `conflict-keep-server`
//! - Open merge button:    `conflict-open-merge`
//! - Keep-yours confirm:   `conflict-keep-yours-confirm` (MC-003 secondary confirmation)
//! - draft banner root:    `draft-recovery-banner`
//! - Restore draft button: `draft-restore`
//! - Discard button:       `draft-discard`
//! - export format picker: `export-format-picker` + per-format `export-format-{ext}`
//!
//! ## File-save sink (HBR-QUIET / MC-004 / red-team RISK-4)
//!
//! The real OS file dialog steals focus and blocks, so it is a thin, user-initiated shell. A
//! headless test uses [`PathFileSaveSink`] (writes to a path, never opens a dialog). The real
//! [`NativeFileSaveSink::spawn`] runs `rfd` on a DEDICATED std::thread (NOT the egui frame thread)
//! and RETURNS IMMEDIATELY with a [`PendingFileSave`] handle the host POLLS non-blockingly each
//! frame — so the frame thread NEVER blocks while the dialog is open (the app stays responsive).
//! This is the user-initiated, reviewed HBR-QUIET exception (never automatic focus theft).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use egui::accesskit::Role;

use crate::theme::HsPalette;

use super::draft_manager::DraftManager;
use super::export::ExportOutput;
use super::save_manager::{SaveManager, SaveState};

// ── AccessKit author_ids (the exact contract ids) ───────────────────────────────────────────────

/// Conflict window root author_id (the AC id).
pub const CONFLICT_DIALOG_AUTHOR_ID: &str = "conflict-dialog";
/// Keep-yours button author_id.
pub const CONFLICT_KEEP_YOURS_AUTHOR_ID: &str = "conflict-keep-yours";
/// Keep-server button author_id.
pub const CONFLICT_KEEP_SERVER_AUTHOR_ID: &str = "conflict-keep-server";
/// Open-merge button author_id.
pub const CONFLICT_OPEN_MERGE_AUTHOR_ID: &str = "conflict-open-merge";
/// Keep-yours secondary-confirmation button author_id (MC-003).
pub const CONFLICT_KEEP_YOURS_CONFIRM_AUTHOR_ID: &str = "conflict-keep-yours-confirm";

/// Draft-recovery banner root author_id.
pub const DRAFT_BANNER_AUTHOR_ID: &str = "draft-recovery-banner";
/// Restore-draft button author_id.
pub const DRAFT_RESTORE_AUTHOR_ID: &str = "draft-restore";
/// Discard-draft button author_id.
pub const DRAFT_DISCARD_AUTHOR_ID: &str = "draft-discard";

/// Export format-picker popup root author_id.
pub const EXPORT_PICKER_AUTHOR_ID: &str = "export-format-picker";

// ── Conflict UI outcome ─────────────────────────────────────────────────────────────────────────

/// What the conflict surface decided this frame. The host (`rich_editor_widget`) applies it against
/// the save manager + doc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictOutcome {
    /// No decision this frame.
    None,
    /// "Keep yours" clicked — the host calls [`SaveManager::request_keep_yours`] (shows the confirm).
    RequestKeepYours,
    /// The keep-yours confirmation was accepted — the host calls [`SaveManager::confirm_keep_yours`].
    ConfirmKeepYours,
    /// The keep-yours confirmation was cancelled — the host calls [`SaveManager::cancel_keep_yours`].
    CancelKeepYours,
    /// "Keep server" clicked — the host calls [`SaveManager::keep_server`] and rebuilds the doc.
    KeepServer,
}

/// Render the conflict window when [`SaveManager`] is in a conflict (or its keep-yours
/// confirmation). Returns the [`ConflictOutcome`] the host applies. When the manager is not in a
/// conflict state this renders nothing and returns `None`.
///
/// The window shows: the server version (read-only block list) vs the operator's version, and the
/// three choices [Keep yours] [Keep server] [Open merge]. "Open merge" is deferred (the contract:
/// show "Merge not yet available"). Choosing "Keep yours" surfaces a secondary confirmation
/// (MC-003) inside the SAME window before the destructive overwrite.
pub fn show_conflict_window(
    ctx: &egui::Context,
    save: &SaveManager,
    palette: &HsPalette,
) -> ConflictOutcome {
    let mut outcome = ConflictOutcome::None;
    let confirming = matches!(save.state, SaveState::ConfirmKeepYours { .. });
    let (server_version, server_blocks, local_blocks) = match &save.state {
        SaveState::Conflict {
            server,
            local_content,
        }
        | SaveState::ConfirmKeepYours {
            server,
            local_content,
        } => (
            server.doc_version,
            block_summaries(server.content_json.as_ref()),
            block_summaries(Some(local_content)),
        ),
        _ => return ConflictOutcome::None,
    };

    let window = egui::Window::new("Save conflict")
        .id(egui::Id::new("rich-editor-conflict-window"))
        .collapsible(false)
        .resizable(true)
        .order(egui::Order::Foreground)
        .default_width(560.0);
    let resp = window.show(ctx, |ui| {
        ui.colored_label(
            palette.text_subtle,
            "Someone else saved a newer version of this document while you were editing.",
        );
        ui.add_space(6.0);
        ui.columns(2, |cols| {
            // Left: server version (read-only).
            cols[0]
                .label(egui::RichText::new(format!("Server version (v{server_version})")).strong());
            render_block_list(&mut cols[0], "conflict-server", &server_blocks, palette);
            // Right: your version.
            cols[1].label(egui::RichText::new("Your version").strong());
            render_block_list(&mut cols[1], "conflict-yours", &local_blocks, palette);
        });
        ui.add_space(8.0);

        if confirming {
            // MC-003: the destructive-overwrite confirmation. NO immediate re-save happened; the
            // operator must accept this before the overwrite.
            ui.colored_label(
                palette.error_text,
                "This will overwrite the server version permanently. Continue?",
            );
            ui.horizontal(|ui| {
                let confirm = ui.button("Yes, overwrite the server version");
                emit_button_id(ui, &confirm, CONFLICT_KEEP_YOURS_CONFIRM_AUTHOR_ID);
                if confirm.clicked() {
                    outcome = ConflictOutcome::ConfirmKeepYours;
                }
                let cancel = ui.button("Cancel");
                if cancel.clicked() {
                    outcome = ConflictOutcome::CancelKeepYours;
                }
            });
        } else {
            ui.horizontal(|ui| {
                let keep_yours = ui.button("Keep yours");
                emit_button_id(ui, &keep_yours, CONFLICT_KEEP_YOURS_AUTHOR_ID);
                if keep_yours.clicked() {
                    outcome = ConflictOutcome::RequestKeepYours;
                }
                let keep_server = ui.button("Keep server");
                emit_button_id(ui, &keep_server, CONFLICT_KEEP_SERVER_AUTHOR_ID);
                if keep_server.clicked() {
                    outcome = ConflictOutcome::KeepServer;
                }
                let open_merge = ui.button("Open merge");
                emit_button_id(ui, &open_merge, CONFLICT_OPEN_MERGE_AUTHOR_ID);
                if open_merge.clicked() {
                    // Deferred to a future MT — show the honest "not available" note (no panic, no
                    // silent no-op). The button is addressable now so the future MT just wires it.
                }
            });
            ui.colored_label(palette.text_subtle, "Merge not yet available.");
        }
    });

    // Tag the window root with the conflict-dialog author_id (the AC id). egui's Window response
    // carries the window's container id.
    if let Some(inner) = resp {
        let node_id = inner.response.id;
        ctx.accesskit_node_builder(node_id, |node| {
            node.set_role(Role::Dialog);
            node.set_author_id(CONFLICT_DIALOG_AUTHOR_ID.to_owned());
            node.set_label("Save conflict".to_owned());
        });
    }
    outcome
}

/// A one-line summary of a doc node's blocks for the read-only conflict comparison view. Each entry
/// is `(kind_label, plain_text)`. Walks only the top-level blocks (a deep diff is the future merge
/// MT); this is enough to let the operator SEE which version is which (the contract's read-only
/// rendered block list).
fn block_summaries(content_json: Option<&serde_json::Value>) -> Vec<(String, String)> {
    let Some(v) = content_json else {
        return vec![("(empty)".to_string(), String::new())];
    };
    // Parse to the model; on a parse failure show a single placeholder (never panic).
    let Ok(doc) = crate::rich_editor::document_model::doc_json::from_json_value(v) else {
        return vec![("(unparseable)".to_string(), String::new())];
    };
    let mut out = Vec::new();
    for child in &doc.children {
        if let Some(b) = child.as_block() {
            let kind = b.kind.to_json_type().to_string();
            let text = super::export::export_plain_text(
                &crate::rich_editor::document_model::node::BlockNode::doc(vec![b.clone()]),
            );
            out.push((kind, text));
        }
    }
    if out.is_empty() {
        out.push(("(empty)".to_string(), String::new()));
    }
    out
}

/// Render a read-only block list (kind + text) into `ui` (the conflict comparison panels). `salt`
/// uniquely identifies the column's ScrollArea so the two panels (server / yours) never share an
/// egui id (a shared id picks up the wrong panel's scroll/debug state).
fn render_block_list(
    ui: &mut egui::Ui,
    salt: &str,
    blocks: &[(String, String)],
    palette: &HsPalette,
) {
    egui::ScrollArea::vertical()
        .id_salt(salt)
        .max_height(220.0)
        .show(ui, |ui| {
            for (kind, text) in blocks {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(palette.text_subtle, format!("[{kind}]"));
                    // A non-selectable label so the read-only comparison text never shows a
                    // click/selection highlight wash (it is reference-only, not editable).
                    ui.add(egui::Label::new(text.as_str()).selectable(false));
                });
            }
        });
}

// ── Draft-recovery banner ───────────────────────────────────────────────────────────────────────

/// What the draft banner decided this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftBannerOutcome {
    /// No decision.
    None,
    /// "Restore draft" — the host calls [`DraftManager::restore_draft`] and rebuilds the doc.
    Restore,
    /// "Discard" — the host calls [`DraftManager::discard_draft`].
    Discard,
    /// The banner was dismissed without discarding — [`DraftManager::dismiss_banner`].
    Dismiss,
}

/// Render the "Draft recovery" banner when [`DraftManager::banner_visible`]. Returns the operator's
/// choice. Reuses the theme palette; the banner + its two buttons are addressable by the contract
/// author_ids.
pub fn show_draft_banner(
    ui: &mut egui::Ui,
    draft: &DraftManager,
    palette: &HsPalette,
) -> DraftBannerOutcome {
    if !draft.banner_visible() {
        return DraftBannerOutcome::None;
    }
    let mut outcome = DraftBannerOutcome::None;
    let frame = egui::Frame::group(ui.style()).fill(palette.surface);
    let resp = frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.colored_label(
                palette.text,
                "Unsaved draft recovered from a previous session.",
            );
            let restore = ui.button("Restore draft");
            emit_button_id(ui, &restore, DRAFT_RESTORE_AUTHOR_ID);
            if restore.clicked() {
                outcome = DraftBannerOutcome::Restore;
            }
            let discard = ui.button("Discard");
            emit_button_id(ui, &discard, DRAFT_DISCARD_AUTHOR_ID);
            if discard.clicked() {
                outcome = DraftBannerOutcome::Discard;
            }
            let dismiss = ui.button("Keep editing");
            if dismiss.clicked() {
                outcome = DraftBannerOutcome::Dismiss;
            }
        });
    });
    // Tag the banner container with the draft-recovery-banner author_id (a Group, non-interactive
    // — a Group carrying an author_id is allowed; only un-named INTERACTIVE nodes trip the gate).
    let node_id = resp.response.id;
    ui.ctx().accesskit_node_builder(node_id, |node| {
        node.set_role(Role::Group);
        node.set_author_id(DRAFT_BANNER_AUTHOR_ID.to_owned());
        node.set_label("Draft recovery".to_owned());
    });
    outcome
}

// ── Export format picker ────────────────────────────────────────────────────────────────────────

/// Render a minimal export format-picker popup as a button group (the contract's "minimal button
/// group in this MT"). Returns the chosen [`super::export::ExportFormat`] when the operator clicks a
/// row, else `None`. The popup root + each row are addressable.
pub fn show_export_picker(ui: &mut egui::Ui) -> Option<super::export::ExportFormat> {
    use super::export::ExportFormat;
    let mut chosen = None;
    let resp = egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(egui::RichText::new("Export as…").strong());
        for fmt in ExportFormat::all() {
            let btn = ui.button(fmt.label());
            emit_button_id(ui, &btn, &format!("export-format-{}", fmt.extension()));
            if btn.clicked() {
                chosen = Some(fmt);
            }
        }
    });
    let node_id = resp.response.id;
    ui.ctx().accesskit_node_builder(node_id, |node| {
        node.set_role(Role::Group);
        node.set_author_id(EXPORT_PICKER_AUTHOR_ID.to_owned());
        node.set_label("Export format picker".to_owned());
    });
    chosen
}

/// Attach a stable author_id to an interactive (clickable) button response so the WP-011 HBR-SWARM
/// gate (`assert_no_unnamed_interactive`) accepts it. Keeps egui's derived Button role/actions; we
/// only add the address (the same pattern `rich_editor_widget` uses for the surface node).
fn emit_button_id(ui: &egui::Ui, resp: &egui::Response, author_id: &str) {
    let author = author_id.to_owned();
    ui.ctx().accesskit_node_builder(resp.id, move |node| {
        node.set_author_id(author.clone());
    });
}

// ── File save sink (mockable; HBR-QUIET) ────────────────────────────────────────────────────────

/// A SYNCHRONOUS, non-dialog destination an export's bytes are written to — used by the headless
/// tests so the export-to-bytes core is exercised end-to-end without focus-stealing OS UI. The
/// PRODUCTION path does NOT use this trait: the real OS dialog steals focus + blocks, so it goes
/// through [`NativeFileSaveSink::spawn`] (a dedicated thread + a [`PendingFileSave`] the host polls
/// non-blockingly each frame), never a synchronous call on the egui frame thread (MC-004 / RISK-4).
pub trait FileSaveSink {
    /// Persist `output` (its bytes, suggested filename, MIME). Returns the path written, or `None`
    /// if the write failed. Implementations of THIS trait write directly to a known path and never
    /// open a dialog, so they are safe to call synchronously (the headless test sink).
    fn save(&self, output: &ExportOutput) -> Option<PathBuf>;
}

/// A headless file-save sink that writes the export bytes to a fixed directory under the suggested
/// filename — NEVER opens a dialog. Used by the unit/kittest tests so the export-to-bytes core is
/// exercised end-to-end without focus-stealing OS UI (HBR-QUIET).
#[derive(Debug, Clone)]
pub struct PathFileSaveSink {
    /// The directory exports are written into.
    pub dir: PathBuf,
}

impl PathFileSaveSink {
    /// Build a sink writing into `dir` (created on first save).
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }
}

impl FileSaveSink for PathFileSaveSink {
    fn save(&self, output: &ExportOutput) -> Option<PathBuf> {
        std::fs::create_dir_all(&self.dir).ok()?;
        let path = self.dir.join(&output.filename);
        std::fs::write(&path, &output.content).ok()?;
        Some(path)
    }
}

/// The production file-save dialog launcher: opens the native `rfd` save dialog on a DEDICATED
/// std::thread (NOT the egui frame thread — MC-004 / HBR-QUIET / red-team RISK-4) and writes the
/// bytes to the chosen path. This is the user-initiated, reviewed HBR-QUIET exception: it runs ONLY
/// when the operator clicks an export format, never automatically, and never steals focus on a
/// background/automated run.
///
/// ## NON-BLOCKING (the frame-freeze fix)
///
/// [`NativeFileSaveSink::spawn`] spawns the dialog thread and RETURNS IMMEDIATELY with a
/// [`PendingFileSave`] handle — it never calls `recv()`/`join()` on the caller. The host stores the
/// handle in the editor state and POLLS it non-blockingly each frame with [`PendingFileSave::poll`]
/// (the proven `Arc<Mutex<Option<..>>>` drain pattern the save/draft managers use). So the egui frame
/// thread NEVER blocks while the OS save dialog is open (it stays responsive the entire time) — the
/// earlier synchronous `rx.recv()` + `h.join()` on the frame thread that froze the app is gone.
#[derive(Debug, Clone, Default)]
pub struct NativeFileSaveSink;

impl NativeFileSaveSink {
    /// Spawn the dialog on a dedicated thread and return immediately. The chosen path (or `None` on
    /// cancel / spawn failure / write failure) lands in the returned handle's slot, polled by the host
    /// each frame. NEVER blocks the caller (the frame thread stays live while the dialog is open).
    pub fn spawn(output: &ExportOutput) -> PendingFileSave {
        let filename = output.filename.clone();
        let bytes = output.content.clone();
        let slot: Arc<Mutex<Option<Option<PathBuf>>>> = Arc::new(Mutex::new(None));
        let slot_for_thread = Arc::clone(&slot);
        let spawned = std::thread::Builder::new()
            .name("hs-export-save-dialog".to_string())
            .spawn(move || {
                let picked = rfd::FileDialog::new().set_file_name(&filename).save_file();
                let written =
                    picked.and_then(|path| std::fs::write(&path, &bytes).ok().map(|_| path));
                // Deliver the outcome (Some(path) | None) into the slot the host polls.
                if let Ok(mut s) = slot_for_thread.lock() {
                    *s = Some(written);
                }
            });
        // If the thread could not even spawn, fail closed: record a resolved `None` so the host's
        // poll completes the pending save on the next frame (no dialog, no write, no panic, no hang).
        if spawned.is_err() {
            if let Ok(mut s) = slot.lock() {
                *s = Some(None);
            }
        }
        PendingFileSave { slot }
    }
}

/// A non-blocking handle to an in-progress native save-dialog. The host stores ONE of these in the
/// editor state while a dialog is open and polls it each frame; once the operator picks a path (or
/// cancels), [`Self::poll`] returns `Some(outcome)` and the host drops the handle.
#[derive(Debug, Clone)]
pub struct PendingFileSave {
    /// Outer `Option`: not-yet-resolved (`None`) vs resolved. Inner `Option<PathBuf>`: the written
    /// path, or `None` on cancel / failure. Filled by the dialog thread, drained by the host.
    slot: Arc<Mutex<Option<Option<PathBuf>>>>,
}

impl PendingFileSave {
    /// Non-blockingly check whether the dialog resolved. Returns `Some(Some(path))` when the operator
    /// saved, `Some(None)` when they cancelled or the write failed, and `None` while the dialog is
    /// still open. NEVER blocks — safe to call every frame from the egui frame thread.
    pub fn poll(&self) -> Option<Option<PathBuf>> {
        self.slot.lock().ok().and_then(|mut s| s.take())
    }

    /// TEST SEAM: a pre-resolved handle (no real dialog) so a headless test can assert the poll-drain
    /// contract without opening OS UI.
    pub fn resolved_for_test(path: Option<PathBuf>) -> Self {
        Self {
            slot: Arc::new(Mutex::new(Some(path))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::save::export::ExportOutput;

    #[test]
    fn path_sink_writes_bytes_to_named_file() {
        let dir = std::env::temp_dir().join(format!("hs-mt020-sink-{}", std::process::id()));
        let sink = PathFileSaveSink::new(&dir);
        let output = ExportOutput {
            content: b"hello export".to_vec(),
            filename: "doc.txt".to_string(),
            mime: "text/plain;charset=utf-8".to_string(),
        };
        let path = sink
            .save(&output)
            .expect("the path sink writes without a dialog");
        assert_eq!(std::fs::read(&path).unwrap(), b"hello export");
        // Cleanup.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn author_id_constants_match_contract() {
        // Guard the contract author_ids so a rename is a deliberate, visible change (the ACs assert
        // these exact strings).
        assert_eq!(CONFLICT_DIALOG_AUTHOR_ID, "conflict-dialog");
        assert_eq!(CONFLICT_KEEP_YOURS_AUTHOR_ID, "conflict-keep-yours");
        assert_eq!(CONFLICT_KEEP_SERVER_AUTHOR_ID, "conflict-keep-server");
        assert_eq!(
            CONFLICT_KEEP_YOURS_CONFIRM_AUTHOR_ID,
            "conflict-keep-yours-confirm"
        );
        assert_eq!(DRAFT_BANNER_AUTHOR_ID, "draft-recovery-banner");
        assert_eq!(DRAFT_RESTORE_AUTHOR_ID, "draft-restore");
        assert_eq!(DRAFT_DISCARD_AUTHOR_ID, "draft-discard");
    }
}
