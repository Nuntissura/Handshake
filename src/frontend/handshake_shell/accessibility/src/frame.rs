//! The host's semantic tree of one window, as it is now: nodes named by the host's own stable keys.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use accesskit::{Node, Rect, Role};
use handshake_shell_input::TextTarget;

/// The most nodes one window's frame may have.
pub const MAX_NODES: usize = 100_000;

/// The host's name for a node of one window, kept as long as the node is the same thing on screen. The crate turns it
/// into an AccessKit id that stays with it across frames; the same key in two windows names two nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeKey(pub u64);

/// A relation of one node to another, by key. The crate turns the key into the other node's id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Relation {
    /// The other node controls this one's content or presence.
    Controls,
    /// The other node gives details.
    Details,
    /// The other node describes it.
    DescribedBy,
    /// Reading goes on at the other node.
    FlowTo,
    /// The other node is its label.
    LabelledBy,
    /// The other node is in its radio group; list every button of the group, itself included.
    RadioGroup,
    /// The other node is its active descendant.
    ActiveDescendant,
    /// The other node is its error message.
    ErrorMessage,
    /// The other node is where its in-page link goes.
    InPageLinkTarget,
    /// It is a member of the other node.
    MemberOf,
    /// The other node follows it on its line.
    NextOnLine,
    /// The other node precedes it on its line.
    PreviousOnLine,
    /// It is a popup of the other node.
    PopupFor,
}

/// A place in a text run, by key: the run (role [`Role::TextRun`]) and a character index into its
/// `character_lengths`, its length meaning the end.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyPosition {
    /// The run.
    pub node: NodeKey,
    /// The character.
    pub character_index: usize,
}

/// A text selection, by key; anchor and focus are equal for a caret.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeySelection {
    /// Where it started.
    pub anchor: KeyPosition,
    /// Its moving end, or the caret.
    pub focus: KeyPosition,
}

/// One node of a frame.
///
/// `node` is an AccessKit core node: role, name (`label`), value, bounds, actions, `author_id`, text run properties and
/// the rest. Bounds are logical pixels in the parent's coordinates; the crate scales the whole window at once. Ids set
/// in `node` itself (children, relations, text selection) are not the host's to give: they are cleared and counted
/// ([`crate::AccessReport::ids_cleared`]), and set from the fields below.
#[derive(Clone, PartialEq)]
pub struct SemanticNode {
    /// The host's key.
    pub key: NodeKey,
    /// What the node is.
    pub node: Node,
    /// Its children, in order.
    pub children: Vec<NodeKey>,
    /// Its relations to other nodes of the frame.
    pub relations: Vec<(Relation, NodeKey)>,
    /// Its text selection or caret, over text runs below it.
    pub text_selection: Option<KeySelection>,
    /// The input crate's text target it shows, if it is one: an accessibility focus request on it moves that text
    /// focus ([`crate::AccessRequest::text_target`]), and [`SemanticFrame::focus_on_text`] finds it.
    pub text_target: Option<TextTarget>,
}

impl SemanticNode {
    /// A node without children or relations.
    pub fn new(key: NodeKey, node: Node) -> Self {
        Self {
            key,
            node,
            children: Vec::new(),
            relations: Vec::new(),
            text_selection: None,
            text_target: None,
        }
    }
}

/// Without names, values or text: they may be private.
impl fmt::Debug for SemanticNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SemanticNode")
            .field("key", &self.key)
            .field("role", &self.node.role())
            .field("children", &self.children)
            .field("text_target", &self.text_target)
            .finish_non_exhaustive()
    }
}

/// One window's semantic tree as the host has it now. Each update gives the whole tree; the crate sends what changed.
#[derive(Clone, Default, PartialEq)]
pub struct SemanticFrame {
    /// The window's accessible name. May be private: never printed.
    pub name: Option<String>,
    /// The nodes directly in the window, in order.
    pub top: Vec<NodeKey>,
    /// Every node, each once, each the child of one node or in `top`.
    pub nodes: Vec<SemanticNode>,
    /// The node that has the keyboard focus while the window has it; `None` for the window itself.
    pub focus: Option<NodeKey>,
}

impl SemanticFrame {
    /// Put the focus on the node that shows `target`, the input crate's text focus of the window
    /// (`InputTracker::text_focus`); on the window itself when no node shows it or there is none.
    pub fn focus_on_text(&mut self, target: Option<TextTarget>) {
        self.focus = target.and_then(|target| {
            self.nodes
                .iter()
                .find(|node| node.text_target == Some(target))
                .map(|node| node.key)
        });
    }
}

impl fmt::Debug for SemanticFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SemanticFrame")
            .field("top", &self.top)
            .field("nodes", &self.nodes)
            .field("focus", &self.focus)
            .finish_non_exhaustive()
    }
}

/// What is wrong with a text run. The adapter reads runs from other threads and cannot handle these.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextRunFault {
    /// It has no value: its text.
    NoValue,
    /// A character of length 0.
    EmptyCharacter,
    /// A character ends inside a UTF-8 sequence of the text, or after its end.
    NotOnBoundary,
    /// The characters do not add up to the text.
    NotCovering,
    /// The word lengths do not add up to the characters.
    WordLengths,
    /// Character positions or widths, not one per character.
    Positions,
}

/// Why a frame was not taken. The window keeps the tree it had.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FrameRefusal {
    /// No tree for the window: it was not opened or attached, or it is gone.
    UnknownWindow,
    /// More than [`MAX_NODES`] nodes.
    TooManyNodes(usize),
    /// Two nodes with one key.
    DuplicateKey(NodeKey),
    /// A child or a top node that is not in the frame; `parent` is `None` for `top`.
    UnknownChild {
        /// Whose.
        parent: Option<NodeKey>,
        /// Which.
        child: NodeKey,
    },
    /// A node is the child of two nodes, or twice of one.
    TwoParents(NodeKey),
    /// A node the window does not reach through `top` and children; a cycle is one.
    Unreachable(NodeKey),
    /// The focus is on a node that is not in the frame.
    UnknownFocus(NodeKey),
    /// A relation to a node that is not in the frame.
    UnknownRelation {
        /// Whose.
        from: NodeKey,
        /// To which.
        to: NodeKey,
    },
    /// A text run the adapter cannot read.
    BadTextRun {
        /// Which.
        key: NodeKey,
        /// What.
        fault: TextRunFault,
    },
    /// A text selection on something other than text runs below its node, or past a run's end.
    BadSelection(NodeKey),
    /// Two nodes show one text target.
    DuplicateTextTarget(TextTarget),
}

impl FrameRefusal {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::UnknownWindow => "a11y.frame.unknown_window",
            Self::TooManyNodes(_) => "a11y.frame.too_many_nodes",
            Self::DuplicateKey(_) => "a11y.frame.duplicate_key",
            Self::UnknownChild { .. } => "a11y.frame.unknown_child",
            Self::TwoParents(_) => "a11y.frame.two_parents",
            Self::Unreachable(_) => "a11y.frame.unreachable",
            Self::UnknownFocus(_) => "a11y.frame.unknown_focus",
            Self::UnknownRelation { .. } => "a11y.frame.unknown_relation",
            Self::BadTextRun { .. } => "a11y.frame.bad_text_run",
            Self::BadSelection(_) => "a11y.frame.bad_selection",
            Self::DuplicateTextTarget(_) => "a11y.frame.duplicate_text_target",
        }
    }
}

/// Check a frame before anything of it is used. On success, every node's index by key and its parent.
pub(crate) fn check(frame: &SemanticFrame) -> Result<Checked, FrameRefusal> {
    if frame.nodes.len() > MAX_NODES {
        return Err(FrameRefusal::TooManyNodes(frame.nodes.len()));
    }
    let mut index = BTreeMap::new();
    let mut targets = BTreeSet::new();
    for (at, node) in frame.nodes.iter().enumerate() {
        if index.insert(node.key, at).is_some() {
            return Err(FrameRefusal::DuplicateKey(node.key));
        }
        if let Some(target) = node.text_target {
            if !targets.insert(target) {
                return Err(FrameRefusal::DuplicateTextTarget(target));
            }
        }
    }
    let mut parent: BTreeMap<NodeKey, Option<NodeKey>> = BTreeMap::new();
    let lists = std::iter::once((None, &frame.top))
        .chain(frame.nodes.iter().map(|node| (Some(node.key), &node.children)));
    for (from, children) in lists {
        for child in children {
            if !index.contains_key(child) {
                return Err(FrameRefusal::UnknownChild {
                    parent: from,
                    child: *child,
                });
            }
            if parent.insert(*child, from).is_some() {
                return Err(FrameRefusal::TwoParents(*child));
            }
        }
    }
    // One parent each: from the window, a walk reaches every node once, and a cycle is never reached.
    let mut reached = BTreeSet::new();
    let mut walk: Vec<NodeKey> = frame.top.clone();
    while let Some(key) = walk.pop() {
        if reached.insert(key) {
            walk.extend(&frame.nodes[index[&key]].children);
        }
    }
    if let Some(node) = frame.nodes.iter().find(|node| !reached.contains(&node.key)) {
        return Err(FrameRefusal::Unreachable(node.key));
    }
    if let Some(focus) = frame.focus.filter(|focus| !index.contains_key(focus)) {
        return Err(FrameRefusal::UnknownFocus(focus));
    }
    let checked = Checked { index, parent };
    for node in &frame.nodes {
        if let Some((_, to)) = node.relations.iter().find(|(_, to)| !checked.has(*to)) {
            return Err(FrameRefusal::UnknownRelation {
                from: node.key,
                to: *to,
            });
        }
        if node.node.role() == Role::TextRun {
            if let Some(fault) = text_run_fault(&node.node) {
                return Err(FrameRefusal::BadTextRun {
                    key: node.key,
                    fault,
                });
            }
        }
        if let Some(selection) = node.text_selection {
            for end in [selection.anchor, selection.focus] {
                if !checked.selects(frame, node.key, end) {
                    return Err(FrameRefusal::BadSelection(node.key));
                }
            }
        }
    }
    Ok(checked)
}

/// What [`check`] learnt of a frame.
pub(crate) struct Checked {
    index: BTreeMap<NodeKey, usize>,
    parent: BTreeMap<NodeKey, Option<NodeKey>>,
}

impl Checked {
    fn has(&self, key: NodeKey) -> bool {
        self.index.contains_key(&key)
    }

    /// `end` is in a text run below `owner`, at most at the run's end.
    fn selects(&self, frame: &SemanticFrame, owner: NodeKey, end: KeyPosition) -> bool {
        let Some(&at) = self.index.get(&end.node) else {
            return false;
        };
        let run = &frame.nodes[at].node;
        if run.role() != Role::TextRun || end.character_index > run.character_lengths().len() {
            return false;
        }
        let mut above = self.parent.get(&end.node).copied().flatten();
        while let Some(key) = above {
            if key == owner {
                return true;
            }
            above = self.parent.get(&key).copied().flatten();
        }
        false
    }
}

/// What makes a text run unreadable for the adapter, if anything. Its consumer slices the value by the character
/// lengths and unwraps the value (accesskit_consumer 0.31.0 `text.rs`), on the threads assistive tools call it from.
pub(crate) fn text_run_fault(run: &Node) -> Option<TextRunFault> {
    let Some(text) = run.value() else {
        return Some(TextRunFault::NoValue);
    };
    let lengths = run.character_lengths();
    let mut end = 0usize;
    for length in lengths {
        if *length == 0 {
            return Some(TextRunFault::EmptyCharacter);
        }
        end += usize::from(*length);
        if !text.is_char_boundary(end) {
            return Some(TextRunFault::NotOnBoundary);
        }
    }
    if end != text.len() {
        return Some(TextRunFault::NotCovering);
    }
    let words = run.word_lengths();
    if !words.is_empty() && words.iter().map(|word| usize::from(*word)).sum::<usize>() != lengths.len() {
        return Some(TextRunFault::WordLengths);
    }
    let fits = |values: Option<&[f32]>| values.is_none_or(|values| values.len() == lengths.len());
    if !fits(run.character_positions()) || !fits(run.character_widths()) {
        return Some(TextRunFault::Positions);
    }
    None
}

/// A text run of `text` with `bounds` (logical, in its parent's coordinates): one character per `char`, words ending
/// after their trailing white space, and, when given, each character's start and width along the run (logical, from
/// the run's start). For a run of text the host lays out itself; Kumihimo's shaping gives the positions.
///
/// # Errors
///
/// [`TextRunFault::Positions`] when `characters` is not one pair per `char`.
pub fn text_run(
    text: &str,
    bounds: Rect,
    characters: Option<&[(f32, f32)]>,
) -> Result<Node, TextRunFault> {
    let lengths: Vec<u8> = text
        .chars()
        .map(|c| u8::try_from(c.len_utf8()).unwrap_or(u8::MAX))
        .collect();
    if characters.is_some_and(|characters| characters.len() != lengths.len()) {
        return Err(TextRunFault::Positions);
    }
    let mut words = Vec::new();
    let mut word = 0u8;
    let mut in_space = false;
    for c in text.chars() {
        let space = c.is_whitespace();
        if (in_space && !space) || word == u8::MAX {
            words.push(word);
            word = 0;
        }
        in_space = space;
        word += 1;
    }
    if word > 0 {
        words.push(word);
    }
    let mut run = Node::new(Role::TextRun);
    run.set_value(text);
    run.set_bounds(bounds);
    run.set_character_lengths(lengths);
    run.set_word_lengths(words);
    if let Some(characters) = characters {
        run.set_character_positions(characters.iter().map(|(at, _)| *at).collect::<Vec<_>>());
        run.set_character_widths(characters.iter().map(|(_, width)| *width).collect::<Vec<_>>());
    }
    Ok(run)
}
