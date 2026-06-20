//! Contract snapshot / verification endpoint for the live AccessKit tree.
//!
//! `collect_tree_snapshot` projects the live `accesskit::TreeUpdate` (the exact value egui hands the
//! platform adapter each frame) into a stable, machine-readable list of every node that carries a
//! stable `author_id`. This is the verification surface for MT-025's acceptance criterion ("panes +
//! chrome are findable in the live tree"): a test (or an out-of-process tool) can assert that every
//! expected `author_id` is present with the right role, without scraping pixels or relying on
//! display text.
//!
//! The snapshot is intentionally derived from the *live* `TreeUpdate`, not from `PaneRegistry`'s
//! in-memory builder, so it proves the LIVE emission actually happened — closing the recurring gap
//! where nodes existed in memory but never reached the frame's accessibility tree.

use egui::accesskit;
use serde::Serialize;

/// One stable-id node from the live AccessKit tree, reduced to the fields a model uses to address it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccessNodeSnapshot {
    /// Stable, model-meaningful match key (kebab-case). Only nodes that set this are included.
    pub author_id: String,
    /// Semantic role, as the debug name of the `accesskit::Role` (stable across the egui 0.33
    /// family; avoids leaking the non-serializable enum into the snapshot contract).
    pub role: String,
    /// Human/model-readable label, when set.
    pub label: Option<String>,
    /// The raw AccessKit node id (u64), for callers that address by id rather than author_id.
    pub node_id: u64,
}

/// A deterministic projection of the live AccessKit tree: every node that carries a stable
/// `author_id`, sorted by `author_id` so the snapshot is stable run-to-run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccessTreeSnapshot {
    pub nodes: Vec<AccessNodeSnapshot>,
}

impl AccessTreeSnapshot {
    /// Look up a node by its stable `author_id`.
    pub fn by_author_id(&self, author_id: &str) -> Option<&AccessNodeSnapshot> {
        self.nodes.iter().find(|n| n.author_id == author_id)
    }

    /// True when every supplied `author_id` is present in the live tree.
    pub fn contains_all(&self, author_ids: &[&str]) -> bool {
        author_ids
            .iter()
            .all(|id| self.by_author_id(id).is_some())
    }

    /// All `author_id`s present, in snapshot (sorted) order.
    pub fn author_ids(&self) -> Vec<&str> {
        self.nodes.iter().map(|n| n.author_id.as_str()).collect()
    }
}

/// Project a live `accesskit::TreeUpdate` into a stable-id snapshot.
///
/// The `TreeUpdate` is obtained from `egui::FullOutput::platform_output.accesskit_update` after a
/// frame runs with AccessKit enabled (kittest enables it automatically; eframe enables it on the
/// real window). Only nodes with an `author_id` are included — egui emits many anonymous layout
/// nodes that are not part of the stable-id contract.
pub fn collect_tree_snapshot(update: &accesskit::TreeUpdate) -> AccessTreeSnapshot {
    let mut nodes: Vec<AccessNodeSnapshot> = update
        .nodes
        .iter()
        .filter_map(|(node_id, node)| {
            node.author_id().map(|author_id| AccessNodeSnapshot {
                author_id: author_id.to_owned(),
                role: format!("{:?}", node.role()),
                label: node.label().map(|l| l.to_owned()),
                node_id: node_id.0,
            })
        })
        .collect();
    nodes.sort_by(|a, b| a.author_id.cmp(&b.author_id));
    AccessTreeSnapshot { nodes }
}
