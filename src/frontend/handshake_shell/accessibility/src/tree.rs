//! One window's tree as AccessKit has it: ids that stay with their keys, the window's scale on the root, and updates
//! that carry only what changed.

use std::collections::BTreeMap;

use accesskit::{
    Action, Affine, Node, NodeId, Rect, Role, TextPosition, TextSelection, Tree, TreeUpdate,
};
use handshake_shell_input::TextTarget;
use handshake_shell_window::{LogicalSize, ScaleFactor};

use crate::frame::{check, FrameRefusal, KeyPosition, KeySelection, NodeKey, Relation, SemanticFrame};
use crate::request::AccessTarget;

/// The window itself.
pub(crate) const ROOT: NodeId = NodeId(1);
/// The first id a host node gets.
const FIRST: u64 = 2;

/// Getters and clearers of the id-bearing properties of a node, which the host does not set itself.
type Many = (fn(&Node) -> &[NodeId], fn(&mut Node));
type One = (fn(&Node) -> Option<NodeId>, fn(&mut Node));
const MANY: [Many; 8] = [
    (Node::children, Node::clear_children),
    (Node::controls, Node::clear_controls),
    (Node::details, Node::clear_details),
    (Node::described_by, Node::clear_described_by),
    (Node::flow_to, Node::clear_flow_to),
    (Node::labelled_by, Node::clear_labelled_by),
    (Node::owns, Node::clear_owns),
    (Node::radio_group, Node::clear_radio_group),
];
const ONE: [One; 7] = [
    (Node::active_descendant, Node::clear_active_descendant),
    (Node::error_message, Node::clear_error_message),
    (Node::in_page_link_target, Node::clear_in_page_link_target),
    (Node::member_of, Node::clear_member_of),
    (Node::next_on_line, Node::clear_next_on_line),
    (Node::previous_on_line, Node::clear_previous_on_line),
    (Node::popup_for, Node::clear_popup_for),
];

/// Clear what names other nodes by id; how many properties were set.
fn clear_ids(node: &mut Node) -> u64 {
    let mut cleared = 0;
    for (get, clear) in MANY {
        if !get(node).is_empty() {
            clear(node);
            cleared += 1;
        }
    }
    for (get, clear) in ONE {
        if get(node).is_some() {
            clear(node);
            cleared += 1;
        }
    }
    if node.text_selection().is_some() {
        node.clear_text_selection();
        cleared += 1;
    }
    cleared
}

/// Set the relations a frame names. The list relations are built as one list and set: accesskit 0.21.1 `clear_*` leaves
/// the property's slot empty, and a later `push_*` on an emptied slot adds nothing (`lib.rs` `Properties::clear`,
/// `vec_type_methods`), so pushing after [`clear_ids`] would lose the relation (MT-214 E7-F2).
fn relate_all(node: &mut Node, relations: &[(Relation, NodeId)]) {
    let ids = |wanted: Relation| -> Vec<NodeId> {
        relations
            .iter()
            .filter(|(relation, _)| *relation == wanted)
            .map(|(_, to)| *to)
            .collect()
    };
    macro_rules! set_list {
        ($relation:expr, $set:ident) => {{
            let list = ids($relation);
            if !list.is_empty() {
                node.$set(list);
            }
        }};
    }
    set_list!(Relation::Controls, set_controls);
    set_list!(Relation::Details, set_details);
    set_list!(Relation::DescribedBy, set_described_by);
    set_list!(Relation::FlowTo, set_flow_to);
    set_list!(Relation::LabelledBy, set_labelled_by);
    set_list!(Relation::RadioGroup, set_radio_group);
    for (relation, to) in relations {
        match relation {
            Relation::ActiveDescendant => node.set_active_descendant(*to),
            Relation::ErrorMessage => node.set_error_message(*to),
            Relation::InPageLinkTarget => node.set_in_page_link_target(*to),
            Relation::MemberOf => node.set_member_of(*to),
            Relation::NextOnLine => node.set_next_on_line(*to),
            Relation::PreviousOnLine => node.set_previous_on_line(*to),
            Relation::PopupFor => node.set_popup_for(*to),
            Relation::Controls
            | Relation::Details
            | Relation::DescribedBy
            | Relation::FlowTo
            | Relation::LabelledBy
            | Relation::RadioGroup => {}
        }
    }
}

/// One window's tree.
#[derive(Debug)]
pub(crate) struct WindowTree {
    ids: BTreeMap<NodeKey, NodeId>,
    keys: BTreeMap<NodeId, NodeKey>,
    /// The next id to give. Ids are never given twice in one window, so an id an assistive tool kept from a node that
    /// went never names another.
    next: u64,
    /// The tree as it is now, root included.
    nodes: BTreeMap<NodeId, Node>,
    targets: BTreeMap<NodeKey, TextTarget>,
    focus: NodeId,
    scale: ScaleFactor,
    client: Option<LogicalSize>,
    name: Option<String>,
    top: Vec<NodeId>,
}

impl WindowTree {
    /// The window alone.
    pub(crate) fn new(scale: ScaleFactor) -> Self {
        let mut tree = Self {
            ids: BTreeMap::new(),
            keys: BTreeMap::new(),
            next: FIRST,
            nodes: BTreeMap::new(),
            targets: BTreeMap::new(),
            focus: ROOT,
            scale,
            client: None,
            name: None,
            top: Vec::new(),
        };
        tree.nodes.insert(ROOT, tree.root());
        tree
    }

    /// The window's node: its name, its top nodes, and its scale as the transform of everything below, so the host's
    /// logical bounds become the physical pixels UI Automation reports.
    fn root(&self) -> Node {
        let mut root = Node::new(Role::Window);
        if let Some(name) = &self.name {
            root.set_label(name.as_str());
        }
        root.set_children(self.top.clone());
        root.set_transform(Affine::scale(self.scale.factor()));
        if let Some(client) = self.client {
            root.set_bounds(Rect::new(0.0, 0.0, client.width, client.height));
        }
        // An assistive tool may ask to focus the window; the host gets the request and activates nothing.
        root.add_action(Action::Focus);
        root
    }

    /// Take a frame. The update carries the nodes that are new or changed, and the focus; `None` when nothing changed.
    /// Also how many ids the host had set itself.
    pub(crate) fn apply(
        &mut self,
        frame: SemanticFrame,
    ) -> Result<(Option<TreeUpdate>, u64), FrameRefusal> {
        check(&frame)?;
        let mut ids = BTreeMap::new();
        for node in &frame.nodes {
            let id = match self.ids.get(&node.key) {
                Some(id) => *id,
                None => {
                    let id = NodeId(self.next);
                    self.next += 1;
                    id
                }
            };
            ids.insert(node.key, id);
        }
        let id = |key: NodeKey| ids[&key];
        let position = |end: KeyPosition| TextPosition {
            node: id(end.node),
            character_index: end.character_index,
        };
        let mut cleared = 0;
        let mut nodes = BTreeMap::new();
        let mut targets = BTreeMap::new();
        for semantic in frame.nodes {
            let mut node = semantic.node;
            cleared += clear_ids(&mut node);
            node.set_children(semantic.children.iter().map(|key| id(*key)).collect::<Vec<_>>());
            let relations: Vec<(Relation, NodeId)> = semantic
                .relations
                .iter()
                .map(|(relation, to)| (*relation, id(*to)))
                .collect();
            relate_all(&mut node, &relations);
            if let Some(selection) = semantic.text_selection {
                node.set_text_selection(TextSelection {
                    anchor: position(selection.anchor),
                    focus: position(selection.focus),
                });
            }
            if let Some(target) = semantic.text_target {
                targets.insert(semantic.key, target);
            }
            nodes.insert(id(semantic.key), node);
        }
        self.top = frame.top.iter().map(|key| id(*key)).collect();
        self.name = frame.name;
        let focus = frame.focus.map_or(ROOT, id);
        nodes.insert(ROOT, self.root());
        self.keys = ids.iter().map(|(key, id)| (*id, *key)).collect();
        self.ids = ids;
        self.targets = targets;
        Ok((self.replace(nodes, focus), cleared))
    }

    /// The tree becomes `nodes`; what differs from before, and the focus.
    fn replace(&mut self, nodes: BTreeMap<NodeId, Node>, focus: NodeId) -> Option<TreeUpdate> {
        let changed: Vec<(NodeId, Node)> = nodes
            .iter()
            .filter(|(id, node)| self.nodes.get(*id) != Some(*node))
            .map(|(id, node)| (*id, node.clone()))
            .collect();
        let moved = self.focus != focus;
        self.nodes = nodes;
        self.focus = focus;
        (moved || !changed.is_empty()).then_some(TreeUpdate {
            nodes: changed,
            tree: None,
            focus,
        })
    }

    /// The window is drawn at another scale: only the root changes.
    pub(crate) fn set_scale(&mut self, scale: ScaleFactor) -> Option<TreeUpdate> {
        if self.scale == scale {
            return None;
        }
        self.scale = scale;
        Some(self.root_changed())
    }

    /// The client area has another size.
    pub(crate) fn set_client(&mut self, client: LogicalSize) -> Option<TreeUpdate> {
        if self.client == Some(client) {
            return None;
        }
        self.client = Some(client);
        Some(self.root_changed())
    }

    fn root_changed(&mut self) -> TreeUpdate {
        let root = self.root();
        self.nodes.insert(ROOT, root.clone());
        TreeUpdate {
            nodes: vec![(ROOT, root)],
            tree: None,
            focus: self.focus,
        }
    }

    /// The whole tree, for an adapter that activates or an inspector.
    pub(crate) fn full(&self) -> TreeUpdate {
        TreeUpdate {
            nodes: self.nodes.iter().map(|(id, node)| (*id, node.clone())).collect(),
            tree: Some(Tree::new(ROOT)),
            focus: self.focus,
        }
    }

    pub(crate) fn scale(&self) -> ScaleFactor {
        self.scale
    }

    pub(crate) fn id_of(&self, key: NodeKey) -> Option<NodeId> {
        self.ids.get(&key).copied()
    }

    pub(crate) fn target_of(&self, id: NodeId) -> Option<AccessTarget> {
        if id == ROOT {
            return Some(AccessTarget::Window);
        }
        self.keys.get(&id).map(|key| AccessTarget::Node(*key))
    }

    pub(crate) fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub(crate) fn text_target(&self, key: NodeKey) -> Option<TextTarget> {
        self.targets.get(&key).copied()
    }

    pub(crate) fn is_text_run(&self, key: NodeKey) -> bool {
        self.id_of(key)
            .and_then(|id| self.node(id))
            .is_some_and(|node| node.role() == Role::TextRun)
    }

    pub(crate) fn find_author(&self, author_id: &str) -> Option<NodeKey> {
        self.nodes
            .iter()
            .find(|(_, node)| node.author_id() == Some(author_id))
            .and_then(|(id, _)| self.keys.get(id).copied())
    }

    /// A selection an assistive tool sent, by key; `None` when a run is gone.
    pub(crate) fn key_selection(&self, selection: TextSelection) -> Option<KeySelection> {
        let end = |position: TextPosition| {
            self.keys.get(&position.node).map(|key| KeyPosition {
                node: *key,
                character_index: position.character_index,
            })
        };
        Some(KeySelection {
            anchor: end(selection.anchor)?,
            focus: end(selection.focus)?,
        })
    }
}
