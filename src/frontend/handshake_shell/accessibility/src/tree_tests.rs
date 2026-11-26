//! The tree of one window against the consumer the asset adapter reads from (accesskit_consumer 0.31.0): every update
//! is applied there, so an update the adapter would panic on fails here, and what the consumer ends with is compared
//! with the whole tree.

use std::collections::{BTreeMap, BTreeSet};

use accesskit::{Action, Node, NodeId, Rect, Role, TreeUpdate};
use accesskit_consumer::{Node as Read, Tree as Consumer, TreeChangeHandler};
use handshake_shell_input::TextTarget;
use handshake_shell_window::{LogicalSize, ScaleFactor};

use crate::frame::{
    text_run, FrameRefusal, KeyPosition, KeySelection, NodeKey, Relation, SemanticFrame,
    SemanticNode, TextRunFault,
};
use crate::tree::{WindowTree, ROOT};

struct Ignore;

impl TreeChangeHandler for Ignore {
    fn node_added(&mut self, _: &Read) {}
    fn node_updated(&mut self, _: &Read, _: &Read) {}
    fn focus_moved(&mut self, _: Option<&Read>, _: Option<&Read>) {}
    fn node_removed(&mut self, _: &Read) {}
}

fn key(number: u64) -> NodeKey {
    NodeKey(number)
}

fn labelled(number: u64, role: Role, label: &str) -> SemanticNode {
    let mut node = Node::new(role);
    node.set_label(label);
    SemanticNode::new(key(number), node)
}

/// A window with `top` and the nodes, each with its children.
fn frame(top: &[u64], nodes: Vec<SemanticNode>) -> SemanticFrame {
    SemanticFrame {
        name: Some("window".to_owned()),
        top: top.iter().copied().map(key).collect(),
        nodes,
        focus: None,
    }
}

fn with_children(mut node: SemanticNode, children: &[u64]) -> SemanticNode {
    node.children = children.iter().copied().map(key).collect();
    node
}

fn apply(tree: &mut WindowTree, consumer: &mut Consumer, frame: SemanticFrame) -> Option<TreeUpdate> {
    let (update, _) = tree.apply(frame).expect("a good frame");
    if let Some(update) = &update {
        consumer.update_and_process_changes(update.clone(), &mut Ignore);
    }
    update
}

/// The consumer holds exactly the whole tree: the same nodes with the same data under the same parents, and the focus.
fn same(tree: &WindowTree, consumer: &Consumer) {
    let whole = tree.full();
    let expected: BTreeMap<NodeId, &Node> = whole.nodes.iter().map(|(id, node)| (*id, node)).collect();
    let state = consumer.state();
    let mut seen = BTreeSet::new();
    let mut walk = vec![state.root()];
    while let Some(node) = walk.pop() {
        assert!(seen.insert(node.id()), "{:?} twice", node.id());
        assert_eq!(Some(&node.data()), expected.get(&node.id()), "{:?}", node.id());
        for (at, child) in node.children().enumerate() {
            assert_eq!(expected[&node.id()].children()[at], child.id());
            walk.push(child);
        }
    }
    assert_eq!(seen.len(), expected.len(), "the consumer has every node and no other");
    assert_eq!(state.focus_id_in_tree(), whole.focus);
}

fn consumer_of(tree: &WindowTree) -> Consumer {
    Consumer::new(tree.full(), false)
}

#[test]
fn ids_stay_with_their_keys_across_frames() {
    let mut tree = WindowTree::new(ScaleFactor::ONE);
    let mut consumer = consumer_of(&tree);
    let nodes = |labels: &[(u64, &str)]| -> Vec<SemanticNode> {
        labels
            .iter()
            .map(|(number, label)| labelled(*number, Role::Button, label))
            .collect()
    };
    apply(&mut tree, &mut consumer, frame(&[10, 20, 30], nodes(&[(10, "a"), (20, "b"), (30, "c")])));
    let first: Vec<NodeId> = [10, 20, 30].iter().map(|n| tree.id_of(key(*n)).unwrap()).collect();
    assert!(first.iter().all(|id| *id != ROOT), "the window's id is its own");
    // Reordered, one gone, one new, one renamed: ids stay with keys, not places.
    apply(&mut tree, &mut consumer, frame(&[30, 40, 10], nodes(&[(30, "c"), (40, "d"), (10, "a2")])));
    same(&tree, &consumer);
    assert_eq!(tree.id_of(key(10)), Some(first[0]));
    assert_eq!(tree.id_of(key(30)), Some(first[2]));
    assert_eq!(tree.id_of(key(20)), None, "a key that went has no id");
    let new = tree.id_of(key(40)).unwrap();
    assert!(!first.contains(&new), "a new key gets an id never given");
    // The key that went comes back: a new id, never the one a tool may have kept.
    apply(&mut tree, &mut consumer, frame(&[20], nodes(&[(20, "b")])));
    same(&tree, &consumer);
    let back = tree.id_of(key(20)).unwrap();
    assert!(back != first[1] && back != new && !first.contains(&back));
    // The same key in another window's tree is another tree's business: ids are per window.
    let mut other = WindowTree::new(ScaleFactor::ONE);
    let mut other_consumer = consumer_of(&other);
    apply(&mut other, &mut other_consumer, frame(&[20], nodes(&[(20, "b")])));
    assert_eq!(other.id_of(key(20)), Some(NodeId(2)), "numbered from the start in each window");
}

/// A small deterministic generator, so the sequence is the same in every run.
struct Numbers(u64);

impl Numbers {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

/// A random tree of up to 14 of 24 keys: each key under the window or an earlier one, so there is no cycle.
fn random_frame(numbers: &mut Numbers) -> SemanticFrame {
    let mut keys: Vec<u64> = (0..24).filter(|_| numbers.below(3) != 0).collect();
    keys.truncate(14);
    for at in (1..keys.len()).rev() {
        let other = usize::try_from(numbers.below(at as u64 + 1)).unwrap();
        keys.swap(at, other);
    }
    let mut nodes: Vec<SemanticNode> = Vec::new();
    let mut top = Vec::new();
    for (at, number) in keys.iter().enumerate() {
        let role = [Role::Button, Role::Label, Role::GenericContainer][usize::try_from(numbers.below(3)).unwrap()];
        let mut node = labelled(*number, role, ["x", "y", "z"][usize::try_from(numbers.below(3)).unwrap()]);
        let edge = f64::from(u32::try_from(numbers.below(50)).unwrap());
        node.node.set_bounds(Rect::new(edge, edge, edge + 20.0, edge + 10.0));
        if numbers.below(2) == 0 {
            node.node.add_action(Action::Click);
        }
        nodes.push(node);
        let parent = numbers.below(at as u64 + 1);
        if parent == 0 {
            top.push(key(*number));
        } else {
            let parent = usize::try_from(parent - 1).unwrap();
            nodes[parent].children.push(key(*number));
        }
    }
    let focus = match keys.len() {
        0 => None,
        count => (numbers.below(2) == 0).then(|| key(keys[usize::try_from(numbers.below(count as u64)).unwrap()])),
    };
    SemanticFrame {
        name: Some(["w", "v"][usize::try_from(numbers.below(2)).unwrap()].to_owned()),
        top,
        nodes,
        focus,
    }
}

/// Mutations caught: an update without the parent of a new or moved node, or with a node that went (both panic in the
/// consumer); a node left behind or with old data (the comparison); an id given twice; an id moving to another key.
#[test]
fn incremental_updates_rebuild_the_same_tree_and_never_give_an_id_twice() {
    let mut numbers = Numbers(0x9E37_79B9_7F4A_7C15);
    let mut tree = WindowTree::new(ScaleFactor::ONE);
    let mut consumer = consumer_of(&tree);
    let mut given: BTreeMap<NodeId, NodeKey> = BTreeMap::new();
    let mut previous: BTreeMap<NodeKey, NodeId> = BTreeMap::new();
    for step in 0..400 {
        let frame = random_frame(&mut numbers);
        let keys: Vec<NodeKey> = frame.nodes.iter().map(|node| node.key).collect();
        apply(&mut tree, &mut consumer, frame);
        same(&tree, &consumer);
        let mut now = BTreeMap::new();
        for key in keys {
            let id = tree.id_of(key).unwrap();
            if let Some(before) = previous.get(&key) {
                assert_eq!(*before, id, "step {step}: {key:?} kept its id");
            } else {
                assert!(given.insert(id, key).is_none(), "step {step}: {id:?} given twice");
            }
            now.insert(key, id);
        }
        previous = now;
        if step % 50 == 0 {
            // A new consumer from the whole tree is the same as the one updated step by step.
            same(&tree, &consumer_of(&tree));
        }
    }
    assert!(given.len() > 100, "the sequence exercised many keys: {}", given.len());
}

#[test]
fn an_unchanged_frame_sends_nothing_and_one_change_sends_one_node() {
    let mut tree = WindowTree::new(ScaleFactor::ONE);
    let mut consumer = consumer_of(&tree);
    let make = |label: &str| {
        frame(
            &[1],
            vec![
                with_children(labelled(1, Role::Group, "group"), &[2, 3]),
                labelled(2, Role::Button, "ok"),
                labelled(3, Role::Button, label),
            ],
        )
    };
    let first = apply(&mut tree, &mut consumer, make("cancel")).unwrap();
    assert_eq!(first.nodes.len(), 4, "the window and three nodes");
    assert!(apply(&mut tree, &mut consumer, make("cancel")).is_none(), "nothing changed");
    let one = apply(&mut tree, &mut consumer, make("close")).unwrap();
    assert_eq!(one.nodes.len(), 1);
    assert_eq!(one.nodes[0].0, tree.id_of(key(3)).unwrap());
    assert_eq!(one.focus, ROOT);
    same(&tree, &consumer);
}

/// Mutations caught: bounds scaled node by node (every node would be sent), or not scaled at all (the boxes stay).
#[test]
fn bounds_follow_the_windows_scale_through_the_root_alone() {
    let mut tree = WindowTree::new(ScaleFactor::ONE);
    let mut consumer = consumer_of(&tree);
    let mut group = with_children(labelled(1, Role::Group, "group"), &[2]);
    group.node.set_bounds(Rect::new(10.0, 20.0, 110.0, 50.0));
    let mut button = labelled(2, Role::Button, "ok");
    button.node.set_bounds(Rect::new(5.0, 5.0, 45.0, 25.0));
    apply(&mut tree, &mut consumer, frame(&[1], vec![group, button]));
    let (group, button) = (tree.id_of(key(1)).unwrap(), tree.id_of(key(2)).unwrap());
    let boxed = |consumer: &Consumer, id: NodeId| {
        consumer.state().node_by_id(id).unwrap().bounding_box().unwrap()
    };
    assert_eq!(boxed(&consumer, group), Rect::new(10.0, 20.0, 110.0, 50.0));
    assert_eq!(boxed(&consumer, button), Rect::new(5.0, 5.0, 45.0, 25.0), "in its parent's coordinates, untransformed");
    for (dpi, factor) in [(144, 1.5), (192, 2.0), (120, 1.25), (96, 1.0)] {
        let update = tree.set_scale(ScaleFactor::from_dpi(dpi).unwrap()).unwrap();
        assert_eq!(update.nodes.len(), 1, "the root alone");
        assert_eq!(update.nodes[0].0, ROOT);
        consumer.update_and_process_changes(update, &mut Ignore);
        assert_eq!(boxed(&consumer, group), Rect::new(10.0 * factor, 20.0 * factor, 110.0 * factor, 50.0 * factor));
        assert_eq!(boxed(&consumer, button), Rect::new(5.0 * factor, 5.0 * factor, 45.0 * factor, 25.0 * factor));
        same(&tree, &consumer);
    }
    assert!(tree.set_scale(ScaleFactor::ONE).is_none(), "the same scale changes nothing");
    let update = tree.set_client(LogicalSize::new(300.0, 200.0)).unwrap();
    consumer.update_and_process_changes(update, &mut Ignore);
    assert_eq!(consumer.state().root().bounding_box(), Some(Rect::new(0.0, 0.0, 300.0, 200.0)));
}

fn run_of(number: u64, text: &str) -> SemanticNode {
    SemanticNode::new(key(number), text_run(text, Rect::new(0.0, 0.0, 100.0, 16.0), None).unwrap())
}

/// Each refusal leaves the tree as it was. Mutations caught: any check left out lets its frame through.
#[test]
fn frames_that_would_break_the_adapter_are_refused() {
    let mut tree = WindowTree::new(ScaleFactor::ONE);
    let mut consumer = consumer_of(&tree);
    let good = frame(&[1], vec![with_children(labelled(1, Role::Group, "g"), &[2]), labelled(2, Role::Button, "b")]);
    apply(&mut tree, &mut consumer, good.clone());
    let before = tree.full();
    let kept = tree.id_of(key(2));
    let mut cases: Vec<(SemanticFrame, FrameRefusal)> = Vec::new();
    let mut duplicate = good.clone();
    duplicate.nodes.push(labelled(2, Role::Button, "again"));
    cases.push((duplicate, FrameRefusal::DuplicateKey(key(2))));
    let mut unknown = good.clone();
    unknown.nodes[0].children.push(key(9));
    cases.push((unknown, FrameRefusal::UnknownChild { parent: Some(key(1)), child: key(9) }));
    let mut unknown_top = good.clone();
    unknown_top.top.push(key(8));
    cases.push((unknown_top, FrameRefusal::UnknownChild { parent: None, child: key(8) }));
    let mut twice = good.clone();
    twice.top.push(key(2));
    cases.push((twice, FrameRefusal::TwoParents(key(2))));
    let mut cycle = good.clone();
    cycle.nodes.push(with_children(labelled(3, Role::Group, "c"), &[4]));
    cycle.nodes.push(with_children(labelled(4, Role::Group, "d"), &[3]));
    cases.push((cycle, FrameRefusal::Unreachable(key(3))));
    let mut lone = good.clone();
    lone.nodes.push(labelled(5, Role::Button, "lone"));
    cases.push((lone, FrameRefusal::Unreachable(key(5))));
    let mut focus = good.clone();
    focus.focus = Some(key(7));
    cases.push((focus, FrameRefusal::UnknownFocus(key(7))));
    let mut relation = good.clone();
    relation.nodes[1].relations.push((Relation::LabelledBy, key(6)));
    cases.push((relation, FrameRefusal::UnknownRelation { from: key(2), to: key(6) }));
    let mut targets = good.clone();
    targets.nodes[0].text_target = Some(TextTarget(1));
    targets.nodes[1].text_target = Some(TextTarget(1));
    cases.push((targets, FrameRefusal::DuplicateTextTarget(TextTarget(1))));
    // Text runs the consumer would slice wrongly or unwrap.
    let bad_run = |change: &dyn Fn(&mut Node), fault| {
        let mut run = frame(&[1], vec![with_children(labelled(1, Role::Label, "t"), &[2]), run_of(2, "héllo")]);
        change(&mut run.nodes[1].node);
        (run, FrameRefusal::BadTextRun { key: key(2), fault })
    };
    cases.push(bad_run(&|node: &mut Node| node.clear_value(), TextRunFault::NoValue));
    cases.push(bad_run(&|node: &mut Node| node.set_character_lengths(vec![1u8, 1, 1, 1, 1, 1]), TextRunFault::NotOnBoundary));
    cases.push(bad_run(&|node: &mut Node| node.set_character_lengths(vec![1u8, 2, 1, 1]), TextRunFault::NotCovering));
    cases.push(bad_run(&|node: &mut Node| node.set_character_lengths(vec![0u8, 1, 2, 1, 1, 1]), TextRunFault::EmptyCharacter));
    cases.push(bad_run(&|node: &mut Node| node.set_word_lengths(vec![4u8]), TextRunFault::WordLengths));
    cases.push(bad_run(&|node: &mut Node| node.set_character_positions(vec![0.0f32]), TextRunFault::Positions));
    cases.push(bad_run(&|node: &mut Node| node.set_character_widths(vec![1.0f32, 2.0]), TextRunFault::Positions));
    // Selections: a run of another node, a node that is no run, past a run's end.
    let selection = |owner: usize, run: u64, index| {
        let mut text = frame(
            &[1, 3],
            vec![
                with_children(labelled(1, Role::TextInput, "t"), &[2]),
                run_of(2, "abc"),
                with_children(labelled(3, Role::TextInput, "u"), &[4]),
                run_of(4, "de"),
            ],
        );
        let end = KeyPosition { node: key(run), character_index: index };
        text.nodes[owner].text_selection = Some(KeySelection { anchor: end, focus: end });
        text
    };
    cases.push((selection(0, 4, 0), FrameRefusal::BadSelection(key(1))));
    cases.push((selection(0, 1, 0), FrameRefusal::BadSelection(key(1))));
    cases.push((selection(0, 2, 4), FrameRefusal::BadSelection(key(1))));
    for (case, expected) in cases {
        assert_eq!(tree.apply(case).map(|_| ()), Err(expected));
        assert_eq!(tree.full(), before, "{expected:?} left the tree as it was");
        assert_eq!(tree.id_of(key(2)), kept, "{expected:?} left the ids as they were");
    }
    // The good selections of the same frame are taken, the end of a run included.
    for (owner, run, index) in [(0, 2, 3), (2, 4, 0)] {
        apply(&mut tree, &mut consumer, selection(owner, run, index));
        same(&tree, &consumer);
    }
}

/// Mutations caught: ids the host set reaching the adapter (they would name other nodes or none), relations not
/// turned into ids.
#[test]
fn ids_the_host_set_are_cleared_and_relations_follow_keys() {
    let mut tree = WindowTree::new(ScaleFactor::ONE);
    let mut consumer = consumer_of(&tree);
    // A label's text is its value (accesskit_consumer `label_comes_from_value`); the field has no name of its own.
    let mut label = Node::new(Role::Label);
    label.set_value("Name");
    let mut field = SemanticNode::new(key(2), Node::new(Role::TextInput));
    field.node.set_children(vec![NodeId(77)]);
    field.node.push_labelled_by(NodeId(78));
    field.node.set_active_descendant(NodeId(79));
    field.relations.push((Relation::LabelledBy, key(1)));
    field.relations.push((Relation::DescribedBy, key(3)));
    let (update, cleared) = tree
        .apply(frame(&[1, 2, 3], vec![SemanticNode::new(key(1), label), field, labelled(3, Role::Label, "hint")]))
        .unwrap();
    consumer.update_and_process_changes(update.unwrap(), &mut Ignore);
    assert_eq!(cleared, 3, "children, labelled by and active descendant set by the host");
    let id = tree.id_of(key(2)).unwrap();
    let node = tree.node(id).unwrap();
    assert!(node.children().is_empty());
    assert_eq!(node.active_descendant(), None);
    assert_eq!(node.labelled_by(), [tree.id_of(key(1)).unwrap()]);
    assert_eq!(node.described_by(), [tree.id_of(key(3)).unwrap()]);
    same(&tree, &consumer);
    let read = consumer.state().node_by_id(id).unwrap();
    assert_eq!(read.label().as_deref(), Some("Name"), "the label comes through the relation");
}

#[test]
fn focus_follows_the_frame_and_the_text_focus() {
    let mut tree = WindowTree::new(ScaleFactor::ONE);
    let mut consumer = Consumer::new(tree.full(), true);
    let make = |focus: Option<u64>| {
        let mut first = labelled(1, Role::TextInput, "first");
        first.text_target = Some(TextTarget(11));
        let mut second = labelled(2, Role::TextInput, "second");
        second.text_target = Some(TextTarget(12));
        let mut made = frame(&[1, 2], vec![first, second]);
        made.focus = focus.map(key);
        made
    };
    apply(&mut tree, &mut consumer, make(Some(2)));
    assert_eq!(consumer.state().focus_id(), tree.id_of(key(2)));
    // Only the focus moved: the update carries no node, and still the focus.
    let update = apply(&mut tree, &mut consumer, make(Some(1))).unwrap();
    assert!(update.nodes.is_empty());
    assert_eq!(consumer.state().focus_id(), tree.id_of(key(1)));
    apply(&mut tree, &mut consumer, make(None));
    assert_eq!(consumer.state().focus_id(), Some(ROOT), "the window itself");
    // The input crate's text focus picks the node that shows it.
    let mut following = make(None);
    following.focus_on_text(Some(TextTarget(12)));
    assert_eq!(following.focus, Some(key(2)));
    following.focus_on_text(Some(TextTarget(99)));
    assert_eq!(following.focus, None, "no node shows it: the window");
    following.focus_on_text(None);
    assert_eq!(following.focus, None);
    same(&tree, &consumer);
}

/// Mutations caught: character lengths by byte or by UTF-16 unit, words that do not cover the characters, positions
/// not one per character.
#[test]
fn text_runs_are_read_by_the_adapters_consumer() {
    let text = "héllo wörld  zz";
    let positions: Vec<(f32, f32)> = (0..text.chars().count()).map(|at| (at as f32 * 7.0, 7.0)).collect();
    let run = text_run(text, Rect::new(0.0, 0.0, 105.0, 16.0), Some(&positions)).unwrap();
    assert_eq!(crate::frame::text_run_fault(&run), None);
    assert_eq!(run.character_lengths().len(), text.chars().count());
    assert_eq!(run.word_lengths(), [6, 7, 2], "a word ends after its white space");
    assert_eq!(text_run(text, Rect::ZERO, Some(&positions[1..])), Err(TextRunFault::Positions));
    let long = "a".repeat(600);
    let long_run = text_run(&long, Rect::ZERO, None).unwrap();
    assert_eq!(long_run.word_lengths(), [255, 255, 90], "a word longer than a length holds is split");
    let mut tree = WindowTree::new(ScaleFactor::from_dpi(144).unwrap());
    let mut consumer = consumer_of(&tree);
    let mut field = with_children(labelled(1, Role::TextInput, "text"), &[2]);
    let caret = KeyPosition { node: key(2), character_index: 6 };
    field.text_selection = Some(KeySelection { anchor: caret, focus: caret });
    apply(&mut tree, &mut consumer, frame(&[1], vec![field, SemanticNode::new(key(2), run)]));
    same(&tree, &consumer);
    let read = consumer.state().node_by_id(tree.id_of(key(1)).unwrap()).unwrap();
    assert!(read.supports_text_ranges());
    assert_eq!(read.document_range().text(), text);
    assert!(read.text_selection().is_some(), "the caret reads back");
}
