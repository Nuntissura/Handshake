//! Cross-surface drag payload + the CKC/Atelier embed reference shape (WP-KERNEL-012 MT-033, cluster E5).
//!
//! ## What this is
//!
//! [`DragPayload`] is the ONE typed value an egui `dnd_drag_source` stages and a `dnd_drop_zone` reads
//! when a user drags a CKC/Atelier item (media / character / moodboard) from the [`crate::atelier_side_panel`]
//! into a native note (the rich-text editor) or onto the canvas board. It is the native peer of the
//! React `dataTransfer` MIME payloads the legacy editors used (`AtelierPanel.tsx` drag-out +
//! `LoomCanvasBoard.tsx` / `LoomTransclusionView.tsx` drop-in).
//!
//! ## The embed is an hsLink ATOM by ref_kind — NOT an invented `atelier_embed` node (the MT-014 lesson)
//!
//! The MT-033 contract scope text proposes inserting a custom `{node_type:"atelierEmbed", ...}` block
//! into the rich document. The KERNEL_BUILDER gate (2026-06-23) and the verified backend reality
//! established that this would NOT round-trip: the backend `knowledge_rich_documents.content_json` is the
//! Tiptap/ProseMirror shape, and a media/CKC embed is the EXISTING inline atom
//! [`crate::rich_editor::document_model::node::HsLinkNode`] (`{type:"hsLink", attrs:{refKind, refValue,
//! label, resolved}}`) discriminated by `refKind` — exactly how MT-014 modelled media embeds
//! (`images`/`video`/`album`/`slideshow`) and MT-015 modelled wikilinks. A dropped CKC item therefore
//! becomes an `hsLink` atom whose `refKind` is in the CKC family ([`ATELIER_EMBED_REF_KINDS`]) and whose
//! `refValue` is the atelier `item_id`, so it ROUND-TRIPS `content_json` through `saveRichDocument` /
//! `loadRichDocument` (AC-2) with zero backend changes and zero invented node types the backend would
//! drop on save.
//!
//! ## The canvas-add resolves to a loom block id — NOT an unsupported `atelier_item_id` field (MT-026)
//!
//! The contract scope text proposes `PUT /canvas/{id}/graph` with a `CanvasNodeInput{atelier_item_id}`.
//! MT-026 VERIFIED the real canvas route is `POST .../canvas-boards/{block_id}/placements` whose body is
//! `{placed_block_id, x, y, w, h}` — there is NO `atelier_item_id` field. So a canvas drop reuses the
//! existing [`crate::graph::canvas_board::CanvasDragPayload`] (`{block_id, title?}`); the atelier item's
//! `loom_block_id` (the everything-is-a-block MT-032 layer) is the `placed_block_id`. An atelier item
//! that has NOT yet been resolved to a loom block id ([`AtelierRef::loom_block_id`] is `None`) CANNOT be
//! placed on the canvas — [`DragPayload::canvas_drag_payload`] returns `None` and the host surfaces a
//! typed "needs a loom block id" state rather than POSTing an unsupported field (RISK-3 / MC-3).

use serde::{Deserialize, Serialize};

/// The CKC/Atelier `hsLink` `refKind` family — the values a dropped CKC item's embed atom carries so it
/// is discriminable from the media-render kinds (`images`/`video`/`album`/`slideshow`,
/// [`crate::rich_editor::embeds::asset_resolver::MEDIA_EMBED_REF_KINDS`]) and from wikilink kinds. These
/// render as the non-media `hsLink` CHIP (a labelled reference), not an inline image, so an unresolved or
/// remote CKC item never blocks the editor on an asset fetch. These are valid `refKind` strings the
/// opaque-JSONB `content_json` round-trips losslessly.
pub const ATELIER_EMBED_REF_KINDS: [&str; 8] = [
    "atelier",
    "media",
    "media_album",
    "folder",
    "source_url",
    "character",
    "character_sheet",
    "moodboard",
];

/// Which CKC/Atelier artifact kind a [`DragPayload::AtelierRef`] references. It starts from the MT-033
/// drag-in list and adds the MT-009 versioned `CharacterSheet` ref so downstream modules can target a
/// specific inspected sheet version. The [`AtelierItemKind::ref_kind`] mapping is the `hsLink` `refKind`
/// the embed atom carries so a save/reload round-trip preserves the kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AtelierItemKind {
    /// A media asset (image/video/album from the atelier intake store).
    Media,
    /// A CKC/Atelier media collection reference. This is a chip, not a renderable rich-editor `album`.
    MediaAlbum,
    /// A CKC/Atelier source-folder provenance reference.
    Folder,
    /// A CKC/Atelier source-URL provenance reference.
    SourceUrl,
    /// A character (atelier character record).
    Character,
    /// A versioned character sheet (`atelier://sheet/{character}/{version}`).
    CharacterSheet,
    /// A moodboard / collection.
    Moodboard,
}

impl AtelierItemKind {
    /// The `hsLink` `refKind` string this kind serializes to in `content_json` (a member of
    /// [`ATELIER_EMBED_REF_KINDS`]). This is the WIRE discriminator the embed renderer + a reload read.
    pub fn ref_kind(self) -> &'static str {
        match self {
            AtelierItemKind::Media => "media",
            AtelierItemKind::MediaAlbum => "media_album",
            AtelierItemKind::Folder => "folder",
            AtelierItemKind::SourceUrl => "source_url",
            AtelierItemKind::Character => "character",
            AtelierItemKind::CharacterSheet => "character_sheet",
            AtelierItemKind::Moodboard => "moodboard",
        }
    }

    /// Parse a backend/embed `refKind` string back into a kind, or `None` for a non-CKC kind (so the
    /// embed renderer can tell a CKC chip from a media embed / wikilink). `"atelier"` (the generic CKC
    /// kind) maps to [`AtelierItemKind::Media`] as the safe default for an unspecified atelier item.
    pub fn from_ref_kind(ref_kind: &str) -> Option<Self> {
        match ref_kind {
            "media" | "atelier" => Some(AtelierItemKind::Media),
            "media_album" | "collection" => Some(AtelierItemKind::MediaAlbum),
            "folder" | "source_folder" => Some(AtelierItemKind::Folder),
            "source_url" | "url" => Some(AtelierItemKind::SourceUrl),
            "character" => Some(AtelierItemKind::Character),
            "character_sheet" | "sheet" | "sheet_version" => Some(AtelierItemKind::CharacterSheet),
            "moodboard" => Some(AtelierItemKind::Moodboard),
            _ => None,
        }
    }

    /// A short human/agent-facing badge label for the kind (shown on the side-panel row + the embed chip
    /// + the canvas card header).
    pub fn badge(self) -> &'static str {
        match self {
            AtelierItemKind::Media => "Media",
            AtelierItemKind::MediaAlbum => "Album",
            AtelierItemKind::Folder => "Folder",
            AtelierItemKind::SourceUrl => "URL",
            AtelierItemKind::Character => "Character",
            AtelierItemKind::CharacterSheet => "Sheet",
            AtelierItemKind::Moodboard => "Moodboard",
        }
    }
}

/// A reference to a CKC/Atelier artifact being dragged across surfaces. Carries everything a drop target
/// needs to build the embed atom (`item_id` -> `refValue`, `item_kind` -> `refKind`, `label`) WITHOUT a
/// back-reference into the atelier store, plus the OPTIONAL `loom_block_id` (the everything-is-a-block
/// MT-032 address) needed to place the item on the canvas as a real block reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtelierRef {
    /// The atelier item id (the backend `atelier_intake_item.item_id` / character id / moodboard id),
    /// used as the embed atom's `refValue`.
    pub item_id: String,
    /// The CKC artifact kind, mapped to the embed atom's `refKind`.
    pub item_kind: AtelierItemKind,
    /// The display label (shown on the embed chip / canvas card; `refValue` is the stable id).
    pub label: String,
    /// The Loom block id this atelier item resolves to (MT-032 "everything is a block"), when known.
    /// `Some` => the item is placeable on the canvas as a `loom://` block reference; `None` => the item
    /// has NOT been resolved to a block id yet, so a canvas drop is a typed "needs a loom block id"
    /// no-op (RISK-3 / MC-3) rather than POSTing an unsupported `atelier_item_id` field.
    pub loom_block_id: Option<String>,
}

impl AtelierRef {
    /// Build an atelier reference. `loom_block_id` is `None` until the item is resolved to a block id.
    pub fn new(
        item_id: impl Into<String>,
        item_kind: AtelierItemKind,
        label: impl Into<String>,
    ) -> Self {
        Self {
            item_id: item_id.into(),
            item_kind,
            label: label.into(),
            loom_block_id: None,
        }
    }

    /// Same as [`Self::new`] but with a known `loom_block_id` (a CKC item already mirrored as a Loom
    /// block — placeable on the canvas).
    pub fn with_loom_block(
        item_id: impl Into<String>,
        item_kind: AtelierItemKind,
        label: impl Into<String>,
        loom_block_id: impl Into<String>,
    ) -> Self {
        Self {
            item_id: item_id.into(),
            item_kind,
            label: label.into(),
            loom_block_id: Some(loom_block_id.into()),
        }
    }

    /// Build a versioned CKC character-sheet reference. The `item_id`/`refValue`
    /// is the portable sheet address, not a copied block of sheet text.
    pub fn character_sheet_version(
        character_internal_id: impl AsRef<str>,
        sheet_version_id: impl AsRef<str>,
        label: impl Into<String>,
    ) -> Self {
        Self::new(
            format!(
                "atelier://sheet/{}/{}",
                character_internal_id.as_ref(),
                sheet_version_id.as_ref()
            ),
            AtelierItemKind::CharacterSheet,
            label,
        )
    }

    /// Build a CKC media-album/collection reference. The `media_album`
    /// refKind is deliberately distinct from rich-editor `album` embeds.
    pub fn media_album(collection_ref: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(collection_ref, AtelierItemKind::MediaAlbum, label)
    }

    /// The `hsLink` `refKind` this reference's embed atom carries (the `item_kind` mapping).
    pub fn ref_kind(&self) -> &'static str {
        self.item_kind.ref_kind()
    }

    /// The display label, falling back to `"{refKind}:{item_id}"` when the label is blank (mirrors the
    /// `HsLinkNode` blank-label render), so a chip is never empty.
    pub fn display_label(&self) -> String {
        if self.label.trim().is_empty() {
            format!("{}:{}", self.ref_kind(), self.item_id)
        } else {
            self.label.clone()
        }
    }
}

/// A reference to a Loom block being dragged across surfaces (the everything-is-a-block address). Carries
/// the block id + the workspace it lives in so a drop target can build a `loom://{workspace}/{block}`
/// reference or a canvas placement directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoomBlockRef {
    /// The backend block id (the canvas `placed_block_id` / the rich-text transclusion `refValue`).
    pub block_id: String,
    /// The workspace the block lives in (for the `loom://` address).
    pub workspace_id: String,
}

impl LoomBlockRef {
    /// Build a Loom block reference.
    pub fn new(block_id: impl Into<String>, workspace_id: impl Into<String>) -> Self {
        Self {
            block_id: block_id.into(),
            workspace_id: workspace_id.into(),
        }
    }
}

/// The one typed drag payload an editor surface stages/reads through egui's `DragAndDrop` channel
/// (`dnd_drag_source` / `dnd_drop_zone`). It must be `Send + Sync + 'static` for egui's payload store
/// (compile-gated by the `drag_payload_is_send_sync_static` test), and it is `Serialize`/`Deserialize`
/// so a unit test proves the AtelierRef round-trips losslessly (AC-1 of the unit family).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DragPayload {
    /// A CKC/Atelier artifact dragged from the atelier side panel.
    AtelierRef(AtelierRef),
    /// A Loom block dragged from a graph/canvas/note surface.
    LoomBlockRef(LoomBlockRef),
    /// Plain text dragged across surfaces (the universal fallback).
    PlainText(String),
}

impl DragPayload {
    /// Convert an `AtelierRef` payload into the inline `hsLink` atom that gets inserted into a rich
    /// document at the caret (the rich-text drop path). `refKind` is the CKC kind, `refValue` is the
    /// atelier `item_id`, `label` is the display label, `resolved=true` (a dropped item is a deliberate,
    /// known reference). Returns `None` for a non-atelier payload (only an `AtelierRef` becomes an embed
    /// atom). This is the SINGLE place the embed shape is built, so the round-trip + the drop path use
    /// the same construction (no parallel reimplementation).
    pub fn to_hs_link(&self) -> Option<crate::rich_editor::document_model::node::HsLinkNode> {
        match self {
            DragPayload::AtelierRef(r) => {
                let mut node = crate::rich_editor::document_model::node::HsLinkNode::new(
                    r.ref_kind(),
                    r.item_id.clone(),
                    r.label.clone(),
                );
                node.resolved = true;
                Some(node)
            }
            DragPayload::LoomBlockRef(_) | DragPayload::PlainText(_) => None,
        }
    }

    /// Convert this payload into the canvas board's [`crate::graph::canvas_board::CanvasDragPayload`]
    /// (a block-id reference), or `None` when it cannot be placed as a block.
    ///
    /// - An `AtelierRef` is placeable ONLY when it carries a `loom_block_id` (MT-026: the placement body
    ///   takes a `placed_block_id`, never an `atelier_item_id`); an unresolved atelier item returns
    ///   `None` so the host shows a typed "needs a loom block id" state, NOT a fake POST (RISK-3 / MC-3).
    /// - A `LoomBlockRef` is always placeable (it already carries the block id).
    /// - Plain text is not a block reference -> `None`.
    pub fn canvas_drag_payload(&self) -> Option<crate::graph::canvas_board::CanvasDragPayload> {
        match self {
            DragPayload::AtelierRef(r) => r.loom_block_id.as_ref().map(|block_id| {
                crate::graph::canvas_board::CanvasDragPayload {
                    block_id: block_id.clone(),
                    title: Some(r.display_label()),
                }
            }),
            DragPayload::LoomBlockRef(b) => Some(
                crate::graph::canvas_board::CanvasDragPayload::new(b.block_id.clone()),
            ),
            DragPayload::PlainText(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RED-TEAM CONTROL (RISK-1): the drag payload MUST be `Send + Sync + 'static` for egui's
    /// DragAndDrop store — a compile error here is the gate (same control as the canvas/tab payloads).
    #[test]
    fn drag_payload_is_send_sync_static() {
        fn assert_send_sync_static<T: Send + Sync + 'static>() {}
        assert_send_sync_static::<DragPayload>();
        assert_send_sync_static::<AtelierRef>();
    }

    /// AC (unit): a `DragPayload::AtelierRef` serializes and deserializes losslessly (the wire shape the
    /// drag channel + any persisted clipboard carries).
    #[test]
    fn atelier_ref_serde_round_trips() {
        let payload = DragPayload::AtelierRef(AtelierRef::with_loom_block(
            "item-7",
            AtelierItemKind::Character,
            "Aria",
            "blk-42",
        ));
        let json = serde_json::to_string(&payload).expect("serialize");
        let back: DragPayload = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(payload, back, "AtelierRef must round-trip losslessly");
        // The decoded ref carries every field.
        match back {
            DragPayload::AtelierRef(r) => {
                assert_eq!(r.item_id, "item-7");
                assert_eq!(r.item_kind, AtelierItemKind::Character);
                assert_eq!(r.label, "Aria");
                assert_eq!(r.loom_block_id.as_deref(), Some("blk-42"));
            }
            other => panic!("expected AtelierRef, got {other:?}"),
        }
    }

    /// The `item_kind` -> `refKind` mapping is in the CKC family and round-trips through `from_ref_kind`.
    #[test]
    fn item_kind_ref_kind_mapping_is_ckc_family() {
        for kind in [
            AtelierItemKind::Media,
            AtelierItemKind::MediaAlbum,
            AtelierItemKind::Folder,
            AtelierItemKind::SourceUrl,
            AtelierItemKind::Character,
            AtelierItemKind::CharacterSheet,
            AtelierItemKind::Moodboard,
        ] {
            let rk = kind.ref_kind();
            assert!(
                ATELIER_EMBED_REF_KINDS.contains(&rk),
                "refKind '{rk}' must be in the CKC family {ATELIER_EMBED_REF_KINDS:?}"
            );
            assert_eq!(
                AtelierItemKind::from_ref_kind(rk),
                Some(kind),
                "round-trips refKind '{rk}'"
            );
        }
        // The CKC refKinds are DISTINCT from the media-render kinds (so a CKC chip is never routed to the
        // image/video renderer).
        for media in crate::rich_editor::embeds::asset_resolver::MEDIA_EMBED_REF_KINDS {
            assert!(
                !ATELIER_EMBED_REF_KINDS.contains(&media) || media == "atelier",
                "CKC refKinds must not collide with media-render kinds (collision on '{media}')"
            );
        }
    }

    /// A versioned CKC sheet ref serializes as an hsLink atom whose refKind is
    /// distinct from generic character refs, so downstream tools can request
    /// exactly the sheet version a model inspected.
    #[test]
    fn character_sheet_ref_becomes_hs_link_atom() {
        let payload = DragPayload::AtelierRef(AtelierRef::character_sheet_version(
            "char-uuid",
            "sheet-uuid",
            "Mira sheet v4",
        ));
        let link = payload
            .to_hs_link()
            .expect("character sheet ref becomes hsLink");
        assert_eq!(link.ref_kind, "character_sheet");
        assert_eq!(link.ref_value, "atelier://sheet/char-uuid/sheet-uuid");
        assert_eq!(link.label, "Mira sheet v4");
        assert!(link.resolved);
    }

    /// AC (unit): an `AtelierRef` payload becomes an `hsLink` atom with the CKC `refKind`, `refValue =
    /// item_id`, label preserved, `resolved=true` — the EXACT shape that round-trips `content_json`.
    #[test]
    fn atelier_ref_becomes_hs_link_atom() {
        let payload = DragPayload::AtelierRef(AtelierRef::new(
            "char-9",
            AtelierItemKind::Character,
            "Mira",
        ));
        let link = payload.to_hs_link().expect("AtelierRef becomes an hsLink");
        assert_eq!(link.ref_kind, "character");
        assert_eq!(link.ref_value, "char-9");
        assert_eq!(link.label, "Mira");
        assert!(
            link.resolved,
            "a deliberately dropped item is a resolved reference"
        );
        // A non-atelier payload does NOT become an embed atom.
        assert!(DragPayload::PlainText("x".into()).to_hs_link().is_none());
        assert!(DragPayload::LoomBlockRef(LoomBlockRef::new("b", "w"))
            .to_hs_link()
            .is_none());
    }

    /// RISK-3 / MC-3: an UNRESOLVED atelier item (no `loom_block_id`) is NOT placeable on the canvas
    /// (returns `None`), so the host never POSTs an unsupported `atelier_item_id`. A resolved item + a
    /// LoomBlockRef ARE placeable as block references.
    #[test]
    fn canvas_drag_payload_requires_a_loom_block_id() {
        // Unresolved atelier item -> no canvas placement.
        let unresolved =
            DragPayload::AtelierRef(AtelierRef::new("item-1", AtelierItemKind::Media, "Pic"));
        assert!(
            unresolved.canvas_drag_payload().is_none(),
            "RISK-3: an atelier item with no loom_block_id cannot be placed (no fake atelier_item_id POST)"
        );
        // Resolved atelier item -> placeable as the resolved block reference.
        let resolved = DragPayload::AtelierRef(AtelierRef::with_loom_block(
            "item-1",
            AtelierItemKind::Media,
            "Pic",
            "blk-7",
        ));
        let cdp = resolved
            .canvas_drag_payload()
            .expect("resolved item is placeable");
        assert_eq!(
            cdp.block_id, "blk-7",
            "placed_block_id is the loom block id, not the atelier item id"
        );
        assert_eq!(cdp.title.as_deref(), Some("Pic"));
        // A LoomBlockRef is always placeable.
        let block = DragPayload::LoomBlockRef(LoomBlockRef::new("blk-9", "ws-1"));
        assert_eq!(block.canvas_drag_payload().unwrap().block_id, "blk-9");
        // Plain text is not a block reference.
        assert!(DragPayload::PlainText("hi".into())
            .canvas_drag_payload()
            .is_none());
    }

    /// A blank label falls back to `"{refKind}:{item_id}"` so a chip/card is never empty.
    #[test]
    fn blank_label_falls_back_to_ref() {
        let r = AtelierRef::new("item-3", AtelierItemKind::Moodboard, "   ");
        assert_eq!(r.display_label(), "moodboard:item-3");
        let r2 = AtelierRef::new("item-4", AtelierItemKind::Media, "Sunset");
        assert_eq!(r2.display_label(), "Sunset");
    }
}
