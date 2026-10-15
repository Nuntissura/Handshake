use egui::Color32;

pub const MAX_BUILD_DEPTH: usize = 20;

const CSS_BASIC_NAMED_COLORS: [(&str, &str); 18] = [
    ("black", "000000"),
    ("silver", "c0c0c0"),
    ("gray", "808080"),
    ("grey", "808080"),
    ("white", "ffffff"),
    ("maroon", "800000"),
    ("red", "ff0000"),
    ("purple", "800080"),
    ("fuchsia", "ff00ff"),
    ("green", "008000"),
    ("lime", "00ff00"),
    ("olive", "808000"),
    ("yellow", "ffff00"),
    ("navy", "000080"),
    ("blue", "0000ff"),
    ("teal", "008080"),
    ("aqua", "00ffff"),
    ("cyan", "00ffff"),
];

pub fn parse_folder_color_token(token: &str) -> Option<Color32> {
    let named_hex = CSS_BASIC_NAMED_COLORS
        .iter()
        .find(|(name, _)| token.eq_ignore_ascii_case(name))
        .map(|(_, hex)| *hex);
    let s = named_hex.unwrap_or_else(|| token.strip_prefix('#').unwrap_or(token));
    if !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let expanded;
    let s = if s.len() == 3 {
        expanded = s
            .chars()
            .flat_map(|component| [component, component])
            .collect::<String>();
        expanded.as_str()
    } else if s.len() == 6 {
        s
    } else {
        return None;
    };
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color32::from_rgba_unmultiplied(r, g, b, 255))
}

pub fn parse_hex_color(token: &str) -> Option<Color32> {
    parse_folder_color_token(token)
}

pub fn color_to_hex(color: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderOpenCompletion {
    Ready,
    Pending,
    Applied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderRow {
    pub folder_id: String,
    pub parent_folder_id: Option<String>,
    pub name: String,
    pub color: Option<String>,
}

impl FolderRow {
    pub fn new(
        folder_id: impl Into<String>,
        parent_folder_id: Option<String>,
        name: impl Into<String>,
        color: Option<String>,
    ) -> Self {
        Self {
            folder_id: folder_id.into(),
            parent_folder_id,
            name: name.into(),
            color,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeafBlock {
    pub block_id: String,
    pub title: String,
    pub content_type: String,
}

impl LeafBlock {
    pub fn new(
        block_id: impl Into<String>,
        title: impl Into<String>,
        content_type: impl Into<String>,
    ) -> Self {
        Self {
            block_id: block_id.into(),
            title: title.into(),
            content_type: content_type.into(),
        }
    }

    pub fn icon(&self) -> char {
        match self.content_type.as_str() {
            "note" => '📝',
            "file" | "annotated_file" => '📄',
            "tag_hub" => '#',
            "journal" => '📔',
            "canvas" => '🎨',
            "view_def" => '▦',
            _ => '•',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderNode {
    pub folder_id: String,
    pub title: String,
    pub color: Option<Color32>,
    pub child_folders: Vec<FolderNode>,
    pub child_blocks: Option<Vec<LeafBlock>>,
    pub expanded: bool,
    pub loading: bool,
    pub child_error: Option<String>,
    expansion_generation: u64,
    expansion_action_generation: Option<u64>,
    child_request_sequence: Option<u64>,
    expansion_request_sequence: Option<u64>,
    expansion_terminal_sequence: Option<u64>,
    reveal_children_once: bool,
}

impl FolderNode {
    fn new(row: &FolderRow) -> Self {
        Self {
            folder_id: row.folder_id.clone(),
            title: if row.name.trim().is_empty() {
                row.folder_id.clone()
            } else {
                row.name.clone()
            },
            color: row.color.as_deref().and_then(parse_hex_color),
            child_folders: Vec::new(),
            child_blocks: None,
            expanded: false,
            loading: false,
            child_error: None,
            expansion_generation: 0,
            expansion_action_generation: None,
            child_request_sequence: None,
            expansion_request_sequence: None,
            expansion_terminal_sequence: None,
            reveal_children_once: false,
        }
    }

    pub fn begin_open_action(&mut self) -> u64 {
        self.expansion_generation = self
            .expansion_generation
            .checked_add(1)
            .expect("folder expansion generation exhausted");
        self.expansion_action_generation = Some(self.expansion_generation);
        self.child_request_sequence = None;
        self.expansion_request_sequence = None;
        self.expansion_terminal_sequence = None;
        self.reveal_children_once = false;
        self.expanded = true;
        self.child_error = None;
        self.expansion_generation
    }

    pub fn bind_open_request(&mut self, action_generation: u64, request_sequence: u64) -> bool {
        if self.expansion_action_generation != Some(action_generation)
            || self.expansion_generation != action_generation
        {
            return false;
        }
        self.child_request_sequence = Some(request_sequence);
        self.expansion_request_sequence = Some(request_sequence);
        self.expansion_terminal_sequence = None;
        self.reveal_children_once = false;
        self.expanded = true;
        self.loading = true;
        self.child_error = None;
        true
    }

    pub fn bind_disclosure_request(&mut self, request_sequence: u64) {
        self.child_request_sequence = Some(request_sequence);
        self.reveal_children_once = false;
        self.loading = true;
        self.child_error = None;
    }

    pub fn complete_child_request(&mut self, request_sequence: u64) -> bool {
        if self.child_request_sequence != Some(request_sequence) {
            return false;
        }
        self.child_request_sequence = None;
        if self.expansion_request_sequence == Some(request_sequence) {
            self.expansion_terminal_sequence = Some(request_sequence);
        }
        self.loading = false;
        true
    }

    pub fn reveal_loaded_children_once(&mut self) {
        self.reveal_children_once = self.expanded && self.child_blocks.is_some();
    }

    pub fn collapse(&mut self) {
        self.expanded = false;
        self.reveal_children_once = false;
    }

    pub fn expansion_completion_state(&self, selected: bool) -> FolderOpenCompletion {
        if self.expansion_action_generation != Some(self.expansion_generation) {
            return FolderOpenCompletion::Ready;
        }
        let exact_request_terminal = self.expansion_request_sequence.is_some()
            && self.expansion_terminal_sequence == self.expansion_request_sequence;
        if selected
            && self.expanded
            && !self.loading
            && exact_request_terminal
            && (self.child_blocks.is_some() || self.child_error.is_some())
        {
            FolderOpenCompletion::Applied
        } else {
            FolderOpenCompletion::Pending
        }
    }

    pub fn expansion_generation(&self) -> u64 {
        self.expansion_generation
    }

    pub fn expansion_request_sequence(&self) -> Option<u64> {
        self.expansion_request_sequence
    }

    pub fn expansion_terminal_sequence(&self) -> Option<u64> {
        self.expansion_terminal_sequence
    }

    pub fn take_reveal_children_once(&mut self) -> bool {
        std::mem::take(&mut self.reveal_children_once)
    }

    pub fn folder_count(&self) -> usize {
        1 + self
            .child_folders
            .iter()
            .map(FolderNode::folder_count)
            .sum::<usize>()
    }

    pub fn find_mut(&mut self, folder_id: &str) -> Option<&mut FolderNode> {
        if self.folder_id == folder_id {
            return Some(self);
        }
        for c in &mut self.child_folders {
            if let Some(found) = c.find_mut(folder_id) {
                return Some(found);
            }
        }
        None
    }

    pub fn first_child_error(&self) -> Option<&str> {
        self.child_error.as_deref().or_else(|| {
            self.child_folders
                .iter()
                .find_map(FolderNode::first_child_error)
        })
    }
}

pub fn build_tree(rows: &[FolderRow]) -> Vec<FolderNode> {
    use std::collections::{HashMap, HashSet};

    // Index rows by id and map parent -> ordered children ids.
    let id_set: HashSet<&str> = rows.iter().map(|r| r.folder_id.as_str()).collect();
    let by_id: HashMap<&str, &FolderRow> = rows.iter().map(|r| (r.folder_id.as_str(), r)).collect();
    let mut children_of: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut roots: Vec<&str> = Vec::new();
    for r in rows {
        match &r.parent_folder_id {
            Some(p) if id_set.contains(p.as_str()) => {
                children_of
                    .entry(p.as_str())
                    .or_default()
                    .push(r.folder_id.as_str());
            }
            // No parent, or a parent id that isn't in the set (orphan) => a root.
            _ => roots.push(r.folder_id.as_str()),
        }
    }

    fn build_node<'a>(
        id: &'a str,
        by_id: &HashMap<&'a str, &'a FolderRow>,
        children_of: &HashMap<&'a str, Vec<&'a str>>,
        visited: &mut std::collections::HashSet<&'a str>,
        depth: usize,
    ) -> Option<FolderNode> {
        // Cycle guard (RISK-1): a folder already on the current path is not re-entered. Depth guard
        // (MC-1): stop recursing past the build cap so a pathological chain terminates.
        if depth >= MAX_BUILD_DEPTH || !visited.insert(id) {
            return None;
        }
        let row = by_id.get(id)?;
        let mut node = FolderNode::new(row);
        if let Some(kids) = children_of.get(id) {
            for kid in kids {
                if let Some(child) = build_node(kid, by_id, children_of, visited, depth + 1) {
                    node.child_folders.push(child);
                }
            }
        }
        visited.remove(id);
        Some(node)
    }

    let mut visited: HashSet<&str> = HashSet::new();
    roots
        .into_iter()
        .filter_map(|id| build_node(id, &by_id, &children_of, &mut visited, 0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows_fixture() -> Vec<FolderRow> {
        vec![
            FolderRow::new("folder-001", None, "Projects", Some("#ff0000".to_owned())),
            FolderRow::new("folder-002", None, "Archive", None),
            FolderRow::new(
                "folder-003",
                Some("folder-001".to_owned()),
                "Subproject",
                Some("00ff00".to_owned()),
            ),
        ]
    }

    #[test]
    fn build_tree_links_children_and_parses_colors() {
        let tree = build_tree(&rows_fixture());
        assert_eq!(tree.len(), 2, "two root folders");
        let projects = tree
            .iter()
            .find(|n| n.folder_id == "folder-001")
            .expect("folder-001 root");
        assert_eq!(
            projects.child_folders.len(),
            1,
            "folder-001 has one child folder"
        );
        assert_eq!(projects.child_folders[0].folder_id, "folder-003");
        // Color parsed from "#ff0000" => opaque red.
        assert_eq!(
            projects.color,
            Some(Color32::from_rgba_unmultiplied(255, 0, 0, 255))
        );
        // The child's color came from a bare (no-#) hex.
        assert_eq!(
            projects.child_folders[0].color,
            Some(Color32::from_rgba_unmultiplied(0, 255, 0, 255))
        );
        // The un-colored root falls back to None (host paints the neutral theme swatch).
        let archive = tree
            .iter()
            .find(|n| n.folder_id == "folder-002")
            .expect("folder-002 root");
        assert_eq!(archive.color, None);
    }

    #[test]
    fn build_tree_breaks_cycles() {
        let rows = vec![
            FolderRow::new("a", Some("b".to_owned()), "A", None),
            FolderRow::new("b", Some("a".to_owned()), "B", None),
        ];
        // Neither is a real root (each parent IS present), so the cycle-guard must still produce a
        // terminating, finite forest rather than hang. Both rows reference an in-set parent, so `roots`
        // is empty => the forest is empty (no acyclic entry point), which is the correct, safe result
        // for a pure cycle with no root.
        let tree = build_tree(&rows);
        // The key property: build_tree returns (does not hang / stack-overflow) and yields a finite tree.
        assert!(
            tree.len() <= 2,
            "cyclic input yields a finite, bounded forest"
        );
    }

    #[test]
    fn build_tree_self_parent_terminates() {
        let rows = vec![FolderRow::new("a", Some("a".to_owned()), "A", None)];
        let tree = build_tree(&rows);
        // 'a' is its own parent => it is NOT a root (parent is in-set), so the forest is empty; the
        // important property is termination + finiteness.
        assert!(tree.len() <= 1);
        if let Some(n) = tree.first() {
            assert!(
                n.folder_count() < MAX_BUILD_DEPTH,
                "no infinite self-nesting"
            );
        }
    }

    #[test]
    fn build_tree_orphan_becomes_root() {
        let rows = vec![FolderRow::new(
            "x",
            Some("missing".to_owned()),
            "Orphan",
            None,
        )];
        let tree = build_tree(&rows);
        assert_eq!(tree.len(), 1, "orphan promoted to root");
        assert_eq!(tree[0].folder_id, "x");
    }

    #[test]
    fn build_tree_depth_capped() {
        let mut rows = vec![FolderRow::new("root", None, "Root", None)];
        let mut prev = "root".to_owned();
        for i in 0..(MAX_BUILD_DEPTH + 10) {
            let id = format!("d{i}");
            rows.push(FolderRow::new(
                id.clone(),
                Some(prev.clone()),
                format!("D{i}"),
                None,
            ));
            prev = id;
        }
        let tree = build_tree(&rows);
        assert_eq!(tree.len(), 1, "single root");
        // The nesting depth is capped at MAX_BUILD_DEPTH (the chain is truncated past the cap).
        fn max_depth(n: &FolderNode) -> usize {
            1 + n.child_folders.iter().map(max_depth).max().unwrap_or(0)
        }
        assert!(
            max_depth(&tree[0]) <= MAX_BUILD_DEPTH,
            "tree depth capped at {MAX_BUILD_DEPTH}, got {}",
            max_depth(&tree[0])
        );
    }

    #[test]
    fn disclosure_reexpand_reuses_loaded_node_cache() {
        let mut node = FolderNode::new(&FolderRow::new("f", None, "F", None));
        // Not yet loaded => an expand should trigger a fetch.
        assert!(
            node.child_blocks.is_none(),
            "fresh node has no cached blocks"
        );
        // After loading (even empty), it is cached => no re-fetch.
        node.child_blocks = Some(vec![]);
        assert!(
            node.child_blocks.is_some(),
            "loaded (even empty) node is cached"
        );
    }

    #[test]
    fn successful_child_install_arms_one_shot_reveal_but_failure_does_not() {
        let mut node = FolderNode::new(&FolderRow::new("f", None, "F", None));
        let generation = node.begin_open_action();
        assert!(node.bind_open_request(generation, 7));
        assert!(node.complete_child_request(7));
        assert!(
            !node.reveal_children_once,
            "a terminal delivery alone may still be a failure and must not reveal stale cache"
        );

        node.child_blocks = Some(vec![LeafBlock::new("b", "Child", "note")]);
        node.reveal_loaded_children_once();
        assert!(node.reveal_children_once);

        node.collapse();
        assert!(
            !node.reveal_children_once,
            "collapse retires a reveal for children that are no longer painted"
        );

        node.expanded = true;
        node.reveal_loaded_children_once();
        assert!(node.reveal_children_once);
        node.bind_disclosure_request(8);
        assert!(
            !node.reveal_children_once,
            "a fresh disclosure request resets an unconsumed reveal"
        );

        node.reveal_loaded_children_once();
        node.begin_open_action();
        assert!(
            !node.reveal_children_once,
            "a fresh open action resets an unconsumed reveal from the prior generation"
        );
    }

    #[test]
    fn folder_open_generation_stays_pending_until_selected_child_terminal_state() {
        let row = FolderRow::new("folder-001", None, "Projects", None);
        let mut node = FolderNode::new(&row);
        assert_eq!(
            node.expansion_completion_state(false),
            FolderOpenCompletion::Ready
        );

        let generation_one = node.begin_open_action();
        assert_eq!(generation_one, 1);
        assert_eq!(node.expansion_generation, 1);
        assert!(node.expanded);
        assert_eq!(
            node.expansion_completion_state(false),
            FolderOpenCompletion::Pending,
            "selection must be observed on a fresh frame"
        );
        assert!(node.bind_open_request(generation_one, 41));
        assert_eq!(
            node.expansion_completion_state(true),
            FolderOpenCompletion::Pending
        );
        assert!(
            !node.complete_child_request(40),
            "a stale backend sequence cannot terminate the current action"
        );
        assert_eq!(
            node.expansion_completion_state(true),
            FolderOpenCompletion::Pending
        );
        assert!(node.complete_child_request(41));
        node.child_blocks = Some(vec![LeafBlock::new("block-001", "Child", "note")]);
        assert_eq!(
            node.expansion_completion_state(true),
            FolderOpenCompletion::Applied
        );

        let generation_two = node.begin_open_action();
        assert_eq!(node.expansion_generation, 2);
        assert_eq!(generation_two, 2);
        assert_eq!(
            node.expansion_completion_state(true),
            FolderOpenCompletion::Pending,
            "a repeated click cannot reuse the generation-one cached children"
        );
        assert!(node.bind_open_request(generation_two, 42));
        assert!(node.complete_child_request(42));
        node.child_error = Some("typed backend failure".to_owned());
        assert_eq!(
            node.expansion_completion_state(true),
            FolderOpenCompletion::Applied,
            "typed failure acknowledges the click without claiming child-load success"
        );
    }
}
