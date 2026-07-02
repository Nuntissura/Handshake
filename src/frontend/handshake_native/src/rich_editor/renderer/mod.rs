//! WYSIWYG renderer for the native rich-text editor (WP-KERNEL-012 MT-012).
//!
//! This module is the visible editing surface for the E2 rich-text cluster: a scrollable
//! egui widget that renders the MT-011 [`document_model`] block tree as styled
//! paragraphs/headings/quotes/code/lists/tables, positions a blinking caret, accepts
//! keyboard + IME input, and issues [`Transaction`]s back to the model.
//!
//! ## Engine + integration decisions (all REUSE, no shell fork)
//!
//! - **Rendering engine: egui `LayoutJob` + epaint `Galley`** (contract RENDERING ENGINE
//!   RECONCILIATION), NOT cosmic-text. Native caret hit-testing via
//!   `Galley::pos_from_cursor`. See [`line_layout`] for the verified-fact rationale
//!   (incl. the italic-via-skew deviation from the contract's stale ITALIC NOTE).
//! - **IME: `egui::Event::Ime`** (contract KERNEL_BUILDER gate), NOT a direct `winit`
//!   dep. See [`ime_handler`].
//! - **Theme: REUSE `crate::theme`** — every color is an `HsPalette` token, no hardcoded
//!   hex (CONTROL-4).
//! - **AccessKit: REUSE the WP-011 `crate::accessibility` live-emission path** — nodes
//!   are written through `ctx.accesskit_node_builder(id, …)` (the same hook
//!   `accessibility::live` uses), so they land in the real per-frame tree a swarm agent
//!   reads out-of-process. The root carries `author_id = "rich-editor-root"` with
//!   `Role::TextInput` (the AC-10 contract id); each paragraph block carries
//!   `author_id = format!("re-block-{path_hash}")` with `Role::Paragraph`.
//! - **NodeId: a DETERMINISTIC hash of the block path** (NOT random — RISK-4 / MC-004),
//!   so a swarm agent references a block by a stable value across repaints. The
//!   collision-free property is proven over a 500-paragraph document
//!   ([`tests::block_node_ids_collision_free_over_500_paragraphs`]).
//!
//! The pane wiring (mounting the widget through `pane_registry` + `split_layout`) lives
//! in [`rich_editor_widget`] via a `PaneFactory`, the same seam the code editor uses.

pub mod block_renderer;
pub mod caret;
pub mod ime_handler;
pub mod input_handler;
pub mod line_layout;
pub mod rich_editor_widget;

use egui::accesskit;

/// The AC-10 author_id for the top-level editor widget (`Role::TextInput`). A swarm agent
/// addresses the whole editing surface by this stable key.
pub const RICH_EDITOR_ROOT_AUTHOR_ID: &str = "rich-editor-root";

/// The fixed AccessKit `NodeId` for the root editor container. Chosen in a HIGH band
/// (>= 1_000_000) deliberately disjoint from the WP-011 shell's small fixed identity
/// bands (chrome 10..97, panes >= 100) and from the per-block hash band (which is seeded
/// into the SAME high space below but offset so the root cannot collide with a block).
/// The root is a single hand-assigned id, so it cannot self-collide.
pub const RICH_EDITOR_ROOT_NODE_ID: u64 = 1_000_000;

/// The author_id PREFIX for each rendered block node (`re-block-{path_hash}`), per the MT
/// contract. The `{path_hash}` is the deterministic [`block_path_hash`] of the block's
/// child-index path.
pub const BLOCK_AUTHOR_ID_PREFIX: &str = "re-block-";

/// The base of the per-block AccessKit `NodeId` band. Block ids are
/// `BLOCK_NODE_ID_BASE + block_path_hash(path)` reduced into a band, kept strictly above
/// the root id and the shell bands so a block node can never collide with shell chrome or
/// the root. The 32-bit hash space sits above this base.
pub const BLOCK_NODE_ID_BASE: u64 = 2_000_000;

/// Compute the stable AccessKit author_id for the block at `path` (the child-index path
/// from the doc root). Deterministic: the same path always yields the same id across
/// repaints and process restarts (RISK-4: NOT random), so a swarm agent can target a
/// block by a value it computed independently.
pub fn block_author_id(path: &[usize]) -> String {
    format!("{BLOCK_AUTHOR_ID_PREFIX}{}", block_path_hash(path))
}

/// A deterministic 32-bit FNV-1a hash of a block path. FNV-1a is a stable, well-distributed
/// non-cryptographic hash with no random seed (unlike `std::collections::hash_map`'s
/// `RandomState`, which would give a DIFFERENT value each run — exactly the bug RISK-4
/// warns about). 32 bits is ample for a single document's block count and keeps the id
/// human-readable in the author_id string.
pub fn block_path_hash(path: &[usize]) -> u32 {
    // FNV-1a over the little-endian bytes of each path element, with a separator byte
    // between elements so `[1, 23]` and `[12, 3]` hash differently.
    const FNV_OFFSET: u32 = 0x811c_9dc5;
    const FNV_PRIME: u32 = 0x0100_0193;
    let mut hash = FNV_OFFSET;
    let mix = |byte: u8, h: &mut u32| {
        *h ^= byte as u32;
        *h = h.wrapping_mul(FNV_PRIME);
    };
    for &idx in path {
        for b in (idx as u64).to_le_bytes() {
            mix(b, &mut hash);
        }
        mix(0xff, &mut hash); // element separator
    }
    hash
}

/// The fixed AccessKit `NodeId` (u64) for the block at `path`, derived from its
/// deterministic path hash and offset into the per-block band. Disjoint from the root id
/// and the shell bands by construction (base 2_000_000 + a 32-bit hash never reaches the
/// root's 1_000_000 nor the shell's < 100 band).
pub fn block_node_id(path: &[usize]) -> u64 {
    BLOCK_NODE_ID_BASE + block_path_hash(path) as u64
}

/// The fixed `egui::Id` backing the root editor node's AccessKit `NodeId`. Uses the same
/// `from_high_entropy_bits` mechanism the shell chrome uses to pin a stable id.
///
/// # Safety
/// `from_high_entropy_bits` assumes a well-distributed value for egui's `IdMap`; a single
/// hand-assigned, never-reused fixed id (1_000_000) cannot self-collide.
pub fn root_egui_id() -> egui::Id {
    unsafe { egui::Id::from_high_entropy_bits(RICH_EDITOR_ROOT_NODE_ID) }
}

/// The AccessKit role for the root editor container (the AC-10 contract role).
pub const ROOT_ROLE: accesskit::Role = accesskit::Role::TextInput;

/// The AccessKit role for a rendered paragraph/heading block (the contract role).
pub const BLOCK_ROLE: accesskit::Role = accesskit::Role::Paragraph;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn block_path_hash_is_deterministic() {
        // RISK-4: the same path hashes identically every call (no random seed).
        let p = [0usize, 3, 7];
        assert_eq!(block_path_hash(&p), block_path_hash(&p));
        assert_eq!(block_author_id(&p), block_author_id(&p));
        // Different paths hash differently for the common shapes.
        assert_ne!(block_path_hash(&[1, 23]), block_path_hash(&[12, 3]));
        assert_ne!(block_path_hash(&[0]), block_path_hash(&[1]));
    }

    #[test]
    fn block_node_ids_collision_free_over_500_paragraphs() {
        // MC-004: a 500-paragraph doc (top-level blocks at paths [0]..[499]) must yield
        // 500 DISTINCT NodeIds and 500 distinct author_ids — zero collisions.
        let mut ids: HashSet<u64> = HashSet::new();
        let mut authors: HashSet<String> = HashSet::new();
        for i in 0..500usize {
            let path = [i];
            assert!(
                ids.insert(block_node_id(&path)),
                "NodeId collision at block {i}"
            );
            assert!(
                authors.insert(block_author_id(&path)),
                "author_id collision at block {i}"
            );
        }
        assert_eq!(ids.len(), 500);
        assert_eq!(authors.len(), 500);
    }

    // The root id and every block id sit far above the shell's fixed bands (< 100) and the
    // pane base (>= 100, but < 1_000_000), so a rich-editor node can never collide with shell
    // chrome / panes. These are compile-time invariants over `const` node-id allocations, so
    // they are enforced with `const { assert!(...) }` rather than a runtime `assert!` (which
    // clippy would flag as assertions_on_constants / "optimized out") — same pattern as
    // code_editor/editor_view.rs.
    const _: () = assert!(
        RICH_EDITOR_ROOT_NODE_ID > 100,
        "root node id must sit above the shell chrome/pane bands"
    );
    const _: () = assert!(
        BLOCK_NODE_ID_BASE > RICH_EDITOR_ROOT_NODE_ID,
        "block node id band must sit above the root id"
    );

    #[test]
    fn block_ids_disjoint_from_root_and_shell_bands() {
        // The runtime loop proves each concrete block id lands in its band and never equals
        // the root id; the band ordering itself is proven by the `const _` asserts above.
        for i in [0usize, 1, 499, 1000] {
            let id = block_node_id(&[i]);
            assert!(id >= BLOCK_NODE_ID_BASE, "block id below its band");
            assert_ne!(
                id, RICH_EDITOR_ROOT_NODE_ID,
                "block id must not equal the root id"
            );
        }
    }

    #[test]
    fn nested_block_paths_hash_distinctly() {
        // Nested blocks (e.g. a list item's paragraph) get distinct ids from their parents
        // and siblings, so a deeply-structured doc still has unique block addresses.
        let mut ids = HashSet::new();
        for top in 0..20usize {
            for child in 0..20usize {
                assert!(
                    ids.insert(block_node_id(&[top, child])),
                    "collision at [{top},{child}]"
                );
            }
        }
        assert_eq!(ids.len(), 400);
    }
}
