//! Accessibility of the kernel-owned shell (WP-KERNEL-012 MT-214 S4, AC214-7).
//!
//! The host's semantic tree of each window reaches UI Automation as an AccessKit tree: roles, names, values, bounds,
//! text runs, focus and actions, with node ids that stay with the host's keys across frames. What assistive tools ask
//! for comes back as typed [`AccessRequest`]s with their origin; a model's action through the steering route becomes
//! the same request with the model's attribution. Reading a tree never activates its window, and every window has a
//! tree of its own. Only AccessKit core types are in the interface (AC214-1), re-exported as [`accesskit`].
//!
//! # How a host uses it
//!
//! 1. Make one [`Accessibility`] per loop, on the loop's thread. [`Accessibility::set_notifier`] with a wake of the
//!    loop (`LaneWaker`), so the host learns that a request waits.
//! 2. On `WindowEvent::Created`, before the first `Show`, [`Accessibility::attach`] the window (Windows; elsewhere
//!    [`Accessibility::open`] gives a tree without an adapter).
//! 3. [`Accessibility::feed`] it every event: it follows each window's scale and client size, and forgets windows that
//!    went.
//! 4. Whenever the window's content changed, give its whole tree as a [`SemanticFrame`] to [`Accessibility::update`],
//!    bounds in logical pixels. Only what changed reaches the adapter. [`SemanticFrame::focus_on_text`] with
//!    `InputTracker::text_focus` puts the focus where the input crate has the text focus.
//! 5. When notified, [`Accessibility::take_requests`] with `Control::now`, and carry out what the host allows. A focus
//!    request moves the text focus through `InputTracker::focus_text` with [`AccessRequest::focus_by`]; nothing here
//!    or there activates a window.
//! 6. Before destroying a window, [`Accessibility::close`] it: its adapter holds a native token, so the window crate
//!    refuses `Destroy` and `finish` until then.
//!
//! # Ids, scale and focus
//!
//! A key keeps its AccessKit id for as long as it is in every frame; a key that is new, or comes back after a frame
//! without it, gets an id the window never had, so an id an assistive tool kept from a node that went names no other
//! node. The window is `NodeId(1)`. The window's scale is the transform of its root node: the host's logical bounds
//! become the physical pixels UI Automation reports, and a scale change sends the root alone. The focus is
//! [`SemanticFrame::focus`], or the window; UI Automation sees it while the window has the keyboard.
//!
//! # Who asked
//!
//! A request of the adapter has [`Origin::OtherProgram`]: an assistive tool or automation, which the operating system
//! does not tell apart from each other. A model's has [`Origin::ModelAction`]. Neither has the Operator's: nothing that
//! comes through the accessibility tree activates a window or uses the Operator's authority. A request names a key, not
//! an id; one for a node that is gone or an action it does not declare is dropped and counted.
//!
//! # The adapter, and why reading activates nothing
//!
//! The Operator's asset copy of accesskit_windows 0.29.2 (Kumihimo third-party, assets rev 8eadc5b5) through the
//! workspace's `[patch.crates-io]`, attached as accesskit_winit attaches it on Windows: a subclassing adapter on the
//! window before it is first shown (`src/platform.rs`). A UI Automation client reads by sending the window
//! `WM_GETOBJECT`, which the adapter answers with its provider; properties are then read from the provider on the
//! client's thread. Nothing in that path activates, shows, orders or focuses a window. The asset's change: the root of
//! a window of this process with `WS_EX_NOACTIVATE` (the window crate's quiet class) reports `ProviderOwnsSetFocus`,
//! so a client's `SetFocus` on it asks the provider, which queues a request, instead of focusing the window.
//!
//! # Failure modes
//!
//! | Seen | Meaning | What to do |
//! | --- | --- | --- |
//! | [`AttachRefusal::AlreadyShown`] | attached after the first `Show` | attach on `Created` |
//! | [`AttachRefusal::WrongThread`] | not on the loop's thread | attach from the loop's handlers |
//! | [`AttachRefusal::Adapter`] | the adapter panicked while attaching | nothing is attached; record it |
//! | [`FrameRefusal`] | the frame would break the adapter: unknown or doubled keys, a cycle, a bad text run | fix the host's frame; the old tree stays |
//! | [`AccessReport::ids_cleared`] above 0 | the host set ids in a node itself | give children and relations by key |
//! | [`AccessReport::requests_stale`], `requests_unsupported` | a tool acted on an old tree, or an undeclared action | none |
//! | [`AccessReport::requests_dropped`] above 0 | more than [`MAX_PENDING_REQUESTS`] waited | take requests when notified |
//! | [`AccessReport::placeholders`] above 0 | a tool activated while a frame was being taken | none; the next update is whole |
//! | [`AccessReport::raise_failed`] above 0 | the operating system refused an event | record it |

#![deny(missing_docs)]
#![deny(unsafe_code)]

mod frame;
mod hub;
#[cfg(windows)]
#[allow(unsafe_code)]
mod platform;
mod request;
mod tree;

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tree_tests;

#[cfg(test)]
#[path = "request_tests.rs"]
mod request_tests;

/// AccessKit core, the type vocabulary of the interface.
pub use accesskit;

pub use frame::{
    text_run, FrameRefusal, KeyPosition, KeySelection, NodeKey, Relation, SemanticFrame,
    SemanticNode, TextRunFault, MAX_NODES,
};
pub use hub::{AccessReport, Accessibility, AttachRefusal, TreeSource};
pub use request::{
    AccessRequest, AccessTarget, ActionRefusal, ActionSink, RequestData, MAX_PENDING_REQUESTS,
};

/// The window and input crates' types this crate's interface carries.
pub use handshake_shell_input::{FocusBy, TextTarget};
pub use handshake_shell_window::{Attribution, EventTime, Origin, WindowKey};
