//! Interactive transclusion read-through preview (WP-KERNEL-012 MT-015) — the native port of
//! `app/src/components/LoomTransclusionView.tsx`.
//!
//! Renders a [`crate::rich_editor::document_model::node::TransclusionNode`] as a bordered preview box:
//!   - a "Transclusion" badge + the source label,
//!   - the resolved source's `content_json` rendered as a plain-text preview truncated to 3 lines,
//!   - an "Open block" link button (enqueues [`EditorEvent::TransclusionOpenRequested`] for the shell),
//!   - fail-closed states: a spinner while resolving, the typed unresolved reason, and a typed error
//!     chip on failure — with a "Remove embed" button on a 404 of a deleted block (MC-003).
//!
//! State + async resolution live in [`crate::rich_editor::wikilinks::runtime::WikilinkRuntime`]
//! (owned by the editor frame), reused exactly like MT-014's `EmbedRuntime`. This view borrows it
//! `&mut`, drives `ensure_transclusion` (once, cached), and renders the cached state.
//!
//! AccessKit (impl note): the container author_id is `transclusion-{block_id}`, the open button is
//! `transclusion-open-{block_id}`, the remove button is `transclusion-remove-{block_id}` — registered
//! through the SAME WP-011 live-emission hook the embeds use (no separate a11y layer).

use egui::accesskit;

use crate::rich_editor::document_model::node::TransclusionNode;
use crate::rich_editor::wikilinks::inline_view::EditorEvent;
use crate::rich_editor::wikilinks::runtime::{TransclusionState, WikilinkRuntime};
use crate::theme::HsPalette;

/// The AccessKit author_id for a transclusion container (`transclusion-{block_id}`).
pub fn container_author_id(ref_value: &str) -> String {
    format!("transclusion-{ref_value}")
}

/// The AccessKit author_id for a transclusion "Open block" button (`transclusion-open-{block_id}`).
pub fn open_author_id(ref_value: &str) -> String {
    format!("transclusion-open-{ref_value}")
}

/// The AccessKit author_id for a transclusion "Remove embed" button (`transclusion-remove-{block_id}`).
pub fn remove_author_id(ref_value: &str) -> String {
    format!("transclusion-remove-{ref_value}")
}

/// WP-KERNEL-012 MT-045 (wave-2 remediation): the AccessKit author_id for a transclusion CYCLE
/// indicator (`transclusion-cycle-{block_id}`) — the visible guard rendered instead of the
/// read-through preview when the chain starting at this node is cyclic.
pub fn cycle_author_id(ref_value: &str) -> String {
    format!("transclusion-cycle-{ref_value}")
}

/// Project a transclusion `content_json` (a Tiptap doc value) into a plain-text preview, truncated to
/// `max_lines` lines (the contract's "content_preview truncated to 3 lines"). Walks every node's
/// `text` field in document order, joining block nodes with newlines. Pure + unit-testable.
pub fn preview_text(content_json: &serde_json::Value, max_lines: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    collect_block_lines(content_json, &mut lines);
    if lines.len() > max_lines {
        lines.truncate(max_lines);
        lines.push("…".to_owned());
    }
    lines.join("\n")
}

/// Collect one line of plain text per top-level block in `node.content`, concatenating the text of
/// each block's inline descendants.
fn collect_block_lines(node: &serde_json::Value, lines: &mut Vec<String>) {
    let Some(content) = node.get("content").and_then(|c| c.as_array()) else {
        return;
    };
    for block in content {
        let mut line = String::new();
        collect_inline_text(block, &mut line);
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            lines.push(trimmed.to_owned());
        }
    }
}

/// Recursively concatenate every `text` field under `node` into `out` (inline text of a block).
fn collect_inline_text(node: &serde_json::Value, out: &mut String) {
    if let Some(text) = node.get("text").and_then(|t| t.as_str()) {
        out.push_str(text);
    }
    if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
        for child in content {
            collect_inline_text(child, out);
        }
    }
}

/// Render a transclusion node interactively into `ui`. Drives the cached resolution and renders the
/// spinner/resolved/unresolved/failed state. Fail-closed: never blank, never a panic. Returns an
/// optional [`EditorEvent`] the caller enqueues into `RichEditorState.pending_events` (an "Open block"
/// click). A "Remove embed" click is handled by mutating the runtime + signaling the caller to delete
/// the node (returned via the `removed` out-flag).
///
/// `removed` is set true when the operator clicked "Remove embed" (MC-003) so the renderer can issue a
/// DeleteNode transaction for this node.
pub fn render_transclusion(
    ui: &mut egui::Ui,
    node: &TransclusionNode,
    runtime: &mut WikilinkRuntime,
    palette: &HsPalette,
) -> (Option<EditorEvent>, bool) {
    let ref_value = node.ref_value.clone();
    runtime.ensure_transclusion(&ref_value);

    let mut event: Option<EditorEvent> = None;
    let mut removed = false;

    let frame = egui::Frame::new()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .inner_margin(8.0)
        .corner_radius(6.0);

    let container = frame.show(ui, |ui| {
        // Header: badge + source label.
        ui.horizontal(|ui| {
            ui.colored_label(palette.accent, "⟢ Transclusion");
            ui.colored_label(palette.text_subtle, &ref_value);
        });

        match runtime.transclusions.get(&ref_value).cloned() {
            None | Some(TransclusionState::Resolving) => {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new());
                    ui.colored_label(palette.text, format!("Resolving transclusion {ref_value}…"));
                });
            }
            Some(TransclusionState::Resolved(t)) => {
                // The source label + the truncated read-through preview.
                let source = t
                    .source_document_id
                    .clone()
                    .unwrap_or_else(|| ref_value.clone());
                ui.colored_label(palette.text_subtle, format!("Source: {source}"));
                // WP-KERNEL-012 MT-045 (wave-2 remediation): guard CYCLIC transclusion chains on the
                // PRODUCT render path. The runtime walks the live resolution cache with the product
                // `resolve_transclusion_chain` (the same cycle-safe algorithm the LR-05 perf proof
                // drives); a detected repeat renders a VISIBLE typed cycle indicator instead of the
                // read-through preview — never a panic, never an unguarded recursive resolve.
                if let Some(cycle_at) = runtime.detect_transclusion_cycle(&ref_value) {
                    let chip = ui.colored_label(
                        palette.error_text,
                        format!(
                            "⟳ Transclusion cycle detected (cycle_detected at block {cycle_at}) — \
                             preview suppressed"
                        ),
                    );
                    emit_node_author(
                        ui.ctx(),
                        chip.id,
                        accesskit::Role::Label,
                        &cycle_author_id(&ref_value),
                    );
                } else {
                    let preview = t
                        .content_json
                        .as_ref()
                        .map(|c| preview_text(c, 3))
                        .unwrap_or_default();
                    if preview.is_empty() {
                        ui.colored_label(palette.text_subtle, "(empty source document)");
                    } else {
                        ui.colored_label(palette.text, preview);
                    }
                }
                // "Open block" link button -> enqueue the open event for the shell to route.
                let open = ui.button("Open block →");
                emit_node_author(
                    ui.ctx(),
                    open.id,
                    accesskit::Role::Button,
                    &open_author_id(&ref_value),
                );
                if open.clicked() {
                    event = Some(EditorEvent::TransclusionOpenRequested {
                        ref_value: ref_value.clone(),
                    });
                }
            }
            Some(TransclusionState::Unresolved(reason)) => {
                // A clean "not yet a source" state (NOT an error): the typed reason, visible.
                ui.colored_label(
                    palette.text_subtle,
                    format!("Transclusion unresolved ({reason}): {ref_value}"),
                );
            }
            Some(TransclusionState::Failed(err)) => {
                // A typed error chip (never blank). On a 404 (deleted block) offer "Remove embed"
                // (MC-003) — a network error does NOT offer remove (it should retry, not delete).
                ui.colored_label(
                    palette.error_text,
                    format!("Transclusion failed ({}): {err}", err.kind_str()),
                );
                if err.is_not_found() {
                    let remove = ui.button("Remove embed");
                    emit_node_author(
                        ui.ctx(),
                        remove.id,
                        accesskit::Role::Button,
                        &remove_author_id(&ref_value),
                    );
                    if remove.clicked() {
                        removed = true;
                    }
                }
            }
        }
    });
    emit_node_author(
        ui.ctx(),
        container.response.id,
        accesskit::Role::Group,
        &container_author_id(&ref_value),
    );

    if removed {
        runtime.mark_removed(&ref_value);
    }
    (event, removed)
}

/// Emit a stable AccessKit author_id (+ role) onto an already-rendered node, REUSING the WP-011
/// live-emission hook (same helper shape as the embeds dispatch). For an interactive Button egui
/// already chose `Role::Button`, so we set only the author_id; for a container we set the role too.
fn emit_node_author(ctx: &egui::Context, id: egui::Id, role: accesskit::Role, author_id: &str) {
    let role_for_closure = role;
    let author = author_id.to_owned();
    ctx.accesskit_node_builder(id, move |node| {
        if !matches!(role_for_closure, accesskit::Role::Button) {
            node.set_role(role_for_closure);
        }
        node.set_author_id(author);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn author_ids_match_contract_shape() {
        assert_eq!(container_author_id("BLK-1"), "transclusion-BLK-1");
        assert_eq!(open_author_id("BLK-1"), "transclusion-open-BLK-1");
        assert_eq!(remove_author_id("BLK-1"), "transclusion-remove-BLK-1");
    }

    #[test]
    fn preview_text_truncates_to_three_lines() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [
                {"type":"paragraph","content":[{"type":"text","text":"line one"}]},
                {"type":"paragraph","content":[{"type":"text","text":"line two"}]},
                {"type":"paragraph","content":[{"type":"text","text":"line three"}]},
                {"type":"paragraph","content":[{"type":"text","text":"line four"}]}
            ]
        });
        let preview = preview_text(&doc, 3);
        let lines: Vec<&str> = preview.lines().collect();
        // 3 content lines + the truncation ellipsis line.
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "line one");
        assert_eq!(lines[2], "line three");
        assert_eq!(
            lines[3], "…",
            "an over-long preview is truncated with an ellipsis line"
        );
    }

    #[test]
    fn preview_text_concatenates_inline_runs() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [
                {"type":"paragraph","content":[
                    {"type":"text","text":"bold "},
                    {"type":"text","text":"and italic"}
                ]}
            ]
        });
        assert_eq!(preview_text(&doc, 3), "bold and italic");
    }

    #[test]
    fn preview_text_empty_doc_is_empty() {
        let doc = serde_json::json!({"type":"doc","content":[]});
        assert_eq!(preview_text(&doc, 3), "");
        let blank = serde_json::json!({"type":"doc","content":[{"type":"paragraph"}]});
        assert_eq!(
            preview_text(&blank, 3),
            "",
            "a blank paragraph contributes no preview line"
        );
    }
}
