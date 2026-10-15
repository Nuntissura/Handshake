//! Native create-note intent adapter for wikilink resolution.

pub use handshake_wikilink_semantics::resolver::*;

use crate::rich_editor::wikilinks::inline_view::EditorEvent;

/// Build the [`EditorEvent::CreateNote`] command-bus intent for an UNRESOLVED `[[title]]` link. The
/// click handler emits THIS rather than calling `POST /knowledge/documents` inline on the egui frame
/// (RISK-007 / MC-007 — frame-freeze avoidance); the async intent handler performs the create and then
/// rewrites the originating mark to Resolved. The carried title is trimmed (matching the
/// `Unresolved { title }` shape `resolve_wikilink` produced).
pub fn create_note_intent(title: &str) -> EditorEvent {
    EditorEvent::CreateNote {
        title: title.trim().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_note_intent_carries_trimmed_title() {
        // AC-001: the create intent carries the link title (trimmed).
        let intent = create_note_intent("  My New Note  ");
        assert_eq!(
            intent,
            EditorEvent::CreateNote {
                title: "My New Note".into()
            }
        );
    }
}
