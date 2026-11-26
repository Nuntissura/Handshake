//! The windows of the shell program (MT-214 S6, AC214-8): which kinds there are, the book of every window the program
//! opened, and the plain decisions about them. Composition only: what a detached window shows is not here, and no
//! window of any kind has more than the shell frame until the UI layer publishes.
//!
//! The contract names three kinds: the root window, detached panes, and the detached Settings window. They share one
//! GPU device and one owner ledger because the program has one presenter on one owner (`run_program` is the only place
//! the device is opened); each has its own claim, surface and accessibility tree.

use std::collections::{BTreeMap, BTreeSet};

use handshake_shell_window::{LogicalSize, WindowKey};

/// What a window of the shell is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum WindowKind {
    /// The shell's root window. Closing it ends the program.
    Root,
    /// A pane taken out of the root window into a window of its own.
    DetachedPane,
    /// The Settings surface taken out into a window of its own.
    DetachedSettings,
}

impl WindowKind {
    /// Every kind, so a test sees them all.
    pub const ALL: [Self; 3] = [Self::Root, Self::DetachedPane, Self::DetachedSettings];

    /// A stable name, for diagnostics, and the name of the window's one accessibility group.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::DetachedPane => "detached_pane",
            Self::DetachedSettings => "detached_settings",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Root => 0,
            Self::DetachedPane => 1,
            Self::DetachedSettings => 2,
        }
    }
}

/// A window of the shell other than the root.
#[derive(Clone, Debug, PartialEq)]
pub struct DetachedSpec {
    kind: WindowKind,
    /// The window's title and its accessibility name.
    pub title: String,
    /// The window's size.
    pub size: LogicalSize,
}

impl DetachedSpec {
    /// A detached window of `kind`; `None` for [`WindowKind::Root`], which the program opens itself.
    pub fn new(kind: WindowKind, title: &str, size: LogicalSize) -> Option<Self> {
        (kind != WindowKind::Root).then(|| Self {
            kind,
            title: title.to_owned(),
            size,
        })
    }

    /// Its kind: never the root.
    pub fn kind(&self) -> WindowKind {
        self.kind
    }
}

/// One step of what a program does by itself after the root window is shown. Run in order, one step finishing before the
/// next begins; a request of the UI layer to detach or close a window takes the same road once it exists.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Action {
    /// Open a detached window.
    Open(DetachedSpec),
    /// Close the `n`-th detached window the program opened, counted from 0 in the order of opening. Only that window's
    /// surface and claim are released; the others go on.
    CloseDetached(usize),
    /// Wait until the presenter has created this many windows in all, each with its surface.
    AwaitCreated(usize),
    /// Wait until this many frames were presented in all.
    AwaitPresented(u64),
    /// Wait until every window that was open when the last [`Action::CloseDetached`] ran has presented a frame since,
    /// and record whether they did.
    AwaitSurvivors,
}

/// Windows opened, by kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KindCounts {
    counts: [u64; 3],
}

impl KindCounts {
    /// How many of `kind`.
    pub fn of(&self, kind: WindowKind) -> u64 {
        self.counts[kind.index()]
    }

    /// All kinds.
    pub fn total(&self) -> u64 {
        self.counts.iter().sum()
    }
}

/// Every window the program asked for, for as long as the process lives. The window crate never uses a key again; the
/// book checks it, since the presenter's client numbers and claims follow the key (AC214-8).
#[derive(Debug, Default)]
pub(crate) struct WindowBook {
    seen: BTreeSet<u64>,
    live: BTreeMap<WindowKey, WindowKind>,
    detached: Vec<WindowKey>,
    opened: KindCounts,
    reused: u64,
    closed: u64,
}

impl WindowBook {
    /// A window was opened. Returns `false`, and counts it, when its key was used before in this run.
    pub(crate) fn opened(&mut self, key: WindowKey, kind: WindowKind) -> bool {
        if !self.seen.insert(key.number()) {
            self.reused += 1;
            return false;
        }
        self.live.insert(key, kind);
        self.opened.counts[kind.index()] += 1;
        if kind != WindowKind::Root {
            self.detached.push(key);
        }
        true
    }

    /// A window was closed or destroyed. Its key stays in the book: it is never opened again.
    pub(crate) fn closed(&mut self, key: WindowKey) {
        if self.live.remove(&key).is_some() {
            self.closed += 1;
        }
    }

    pub(crate) fn kind_of(&self, key: WindowKey) -> Option<WindowKind> {
        self.live.get(&key).copied()
    }

    /// The `n`-th detached window opened, whether or not it is still open.
    pub(crate) fn detached(&self, n: usize) -> Option<WindowKey> {
        self.detached.get(n).copied()
    }

    pub(crate) fn live_keys(&self) -> Vec<WindowKey> {
        self.live.keys().copied().collect()
    }

    pub(crate) fn counts(&self) -> KindCounts {
        self.opened
    }

    pub(crate) fn reused(&self) -> u64 {
        self.reused
    }

    pub(crate) fn closed_count(&self) -> u64 {
        self.closed
    }
}

/// Whether the windows that were open at `before` and still are in `now` all presented a frame since, and at least one
/// is. `before` and `now` are (window number, frames the queue took). A window that is gone is not a survivor.
pub(crate) fn survivors_presented(before: &[(u64, u64)], now: &[(u64, u64)]) -> bool {
    let mut survivors = 0;
    for (window, was) in before {
        if let Some((_, is)) = now.iter().find(|(number, _)| number == window) {
            survivors += 1;
            if is <= was {
                return false;
            }
        }
    }
    survivors > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(number: u64) -> WindowKey {
        WindowKey::from_number(number)
    }

    #[test]
    fn a_detached_spec_is_never_the_root() {
        let size = LogicalSize::new(100.0, 100.0);
        assert!(DetachedSpec::new(WindowKind::Root, "root", size).is_none());
        for kind in [WindowKind::DetachedPane, WindowKind::DetachedSettings] {
            let spec = DetachedSpec::new(kind, "a", size).expect("detached");
            assert_eq!(spec.kind(), kind);
        }
    }

    #[test]
    fn kinds_have_distinct_names_and_the_book_counts_each() {
        let names: BTreeSet<&str> = WindowKind::ALL.iter().map(|kind| kind.name()).collect();
        assert_eq!(names.len(), WindowKind::ALL.len());
        let mut book = WindowBook::default();
        assert!(book.opened(key(1), WindowKind::Root));
        assert!(book.opened(key(2), WindowKind::DetachedPane));
        assert!(book.opened(key(3), WindowKind::DetachedPane));
        assert!(book.opened(key(4), WindowKind::DetachedSettings));
        let counts = book.counts();
        assert_eq!(counts.of(WindowKind::Root), 1);
        assert_eq!(counts.of(WindowKind::DetachedPane), 2);
        assert_eq!(counts.of(WindowKind::DetachedSettings), 1);
        assert_eq!(counts.total(), 4);
        assert_eq!(book.detached(0), Some(key(2)));
        assert_eq!(book.detached(2), Some(key(4)));
        assert_eq!(book.detached(3), None);
    }

    #[test]
    fn closing_one_window_keeps_the_others_in_the_book() {
        let mut book = WindowBook::default();
        for (number, kind) in [
            (1, WindowKind::Root),
            (2, WindowKind::DetachedPane),
            (3, WindowKind::DetachedSettings),
        ] {
            assert!(book.opened(key(number), kind));
        }
        book.closed(key(2));
        assert_eq!(book.live_keys(), [key(1), key(3)]);
        assert_eq!(book.kind_of(key(2)), None);
        assert_eq!(book.kind_of(key(3)), Some(WindowKind::DetachedSettings));
        assert_eq!(book.closed_count(), 1);
        book.closed(key(2));
        assert_eq!(book.closed_count(), 1, "a window closes once");
        assert_eq!(book.detached(0), Some(key(2)), "the order of opening does not shift");
    }

    #[test]
    fn a_key_is_never_used_again_in_a_run_even_after_its_window_closed() {
        let mut book = WindowBook::default();
        assert!(book.opened(key(5), WindowKind::DetachedPane));
        assert!(!book.opened(key(5), WindowKind::DetachedSettings), "live key reused");
        book.closed(key(5));
        assert!(!book.opened(key(5), WindowKind::DetachedPane), "closed key reused");
        assert_eq!(book.reused(), 2);
        assert_eq!(book.counts().total(), 1, "a reused key is not a window");
        assert!(book.live_keys().is_empty());
    }

    #[test]
    fn survivors_must_each_present_after_the_close_and_the_closed_window_does_not_count() {
        assert!(survivors_presented(&[(1, 4), (2, 3), (3, 5)], &[(1, 5), (3, 6)]));
        assert!(!survivors_presented(&[(1, 4), (3, 5)], &[(1, 5), (3, 5)]), "one stood still");
        assert!(!survivors_presented(&[(1, 4), (3, 5)], &[(1, 4), (3, 6)]), "the other stood still");
        assert!(!survivors_presented(&[(1, 4)], &[]), "no survivor proves nothing");
        assert!(!survivors_presented(&[], &[(1, 4)]));
        assert!(!survivors_presented(&[(1, 4)], &[(7, 9)]), "a new window is no survivor");
    }
}
