//! In-process SHELL EVENT BUS for the native work surface (WP-KERNEL-011 MT-014 FIX-B).
//!
//! ## What this provides
//!
//! A small, dependency-free publish/subscribe channel for cross-surface notifications that must reach
//! the left rail (and future surfaces) WITHOUT a backend round trip. It replaces the React DOM
//! `CustomEvent` listeners the `WorkspaceSidebar` used (`handshake:document-deleted`,
//! `handshake:canvas-deleted`, `handshake:loom-bookmarks-changed`) — see the MT-014 contract
//! `implementation_notes` "use a broadcast channel or event bus instead of DOM custom events" and the
//! red-team `minimum_controls` "Event bus must be an in-process broadcast channel (tokio broadcast or
//! std mpsc); subscribe at LeftRail construction".
//!
//! ## Why std mpsc, not tokio broadcast
//!
//! The crate's `tokio` dependency is built WITHOUT the `sync` feature (`tokio::sync::broadcast` is
//! therefore unavailable), and the contract explicitly allows `std mpsc`. Adding the `sync` feature
//! purely for this would widen the dependency surface for no behavioral gain, so this bus is built on
//! `std::sync::mpsc` — which is sufficient because the shell has exactly ONE consumer (the egui UI
//! thread draining the bus once per frame). [`ShellEventBus`] keeps a clonable [`ShellEventSender`]
//! (so any number of producers — future MTs that perform a delete — can publish) and a single
//! [`ShellEventReceiver`] the app owns and drains in `ui()`.
//!
//! ## Producers
//!
//! No production EMITTER exists yet: deleting a document / canvas / bookmark from another surface is a
//! FUTURE MT. This module + the app's per-frame drain + the test below prove the CONTROL MECHANISM is
//! real and ready, so a future MT only has to call [`ShellEventSender::send`] to make a deleted item
//! disappear from the tree with no stale row. This intentional "bus before producer" shape is recorded
//! as a disclosed deviation in the MT-014 handoff.

use std::sync::mpsc::{Receiver, Sender};

/// A cross-surface shell notification. Mirrors the three React `WorkspaceSidebar` DOM events the tree
/// listened to. Each variant carries the id of the affected entity so the drain can remove exactly
/// that row from the live tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellEvent {
    /// A document was deleted elsewhere (React `handshake:document-deleted`). Drop its tree row.
    DocumentDeleted { document_id: String },
    /// A canvas was deleted elsewhere (React `handshake:canvas-deleted`). Drop its tree row.
    CanvasDeleted { canvas_id: String },
    /// A bookmark/pin was removed elsewhere (React `handshake:loom-bookmarks-changed`). Drop its row.
    BookmarkRemoved { block_id: String },
}

/// The publish handle. Clonable so any number of producers can publish onto the same bus. A send that
/// fails (the receiver was dropped — i.e. the app is shutting down) is benign and silently ignored,
/// the same tolerance the project-tree async loader uses for a dropped channel.
#[derive(Debug, Clone)]
pub struct ShellEventSender {
    tx: Sender<ShellEvent>,
}

impl ShellEventSender {
    /// Publish an event onto the bus. Returns `true` if delivered, `false` if the receiver is gone.
    pub fn send(&self, event: ShellEvent) -> bool {
        self.tx.send(event).is_ok()
    }
}

/// The single consume handle. Owned by the app; drained once per frame via [`ShellEventReceiver::drain`].
#[derive(Debug)]
pub struct ShellEventReceiver {
    rx: Receiver<ShellEvent>,
}

impl ShellEventReceiver {
    /// Non-blocking drain of every queued event (in publish order). Call once per frame before render
    /// so a deleted item disappears from the tree on the very next frame.
    pub fn drain(&self) -> Vec<ShellEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            events.push(event);
        }
        events
    }
}

/// Construct a fresh shell event bus: a clonable sender + a single receiver. The app holds the
/// receiver and a copy of the sender (the sender copy is what future delete-performing surfaces clone
/// to publish onto). Built once at app construction (the "subscribe at LeftRail/app construction"
/// control).
pub fn new_shell_event_bus() -> (ShellEventSender, ShellEventReceiver) {
    let (tx, rx) = std::sync::mpsc::channel();
    (ShellEventSender { tx }, ShellEventReceiver { rx })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_returns_published_events_in_order() {
        let (tx, rx) = new_shell_event_bus();
        assert!(tx.send(ShellEvent::DocumentDeleted {
            document_id: "d1".into()
        }));
        assert!(tx.send(ShellEvent::CanvasDeleted {
            canvas_id: "c1".into()
        }));
        let drained = rx.drain();
        assert_eq!(
            drained,
            vec![
                ShellEvent::DocumentDeleted {
                    document_id: "d1".into()
                },
                ShellEvent::CanvasDeleted {
                    canvas_id: "c1".into()
                },
            ],
        );
        // A second drain with nothing queued is empty (the channel was emptied).
        assert!(rx.drain().is_empty());
    }

    #[test]
    fn send_after_receiver_dropped_is_benign() {
        let (tx, rx) = new_shell_event_bus();
        drop(rx);
        assert!(!tx.send(ShellEvent::BookmarkRemoved {
            block_id: "b1".into()
        }));
    }

    #[test]
    fn sender_is_clonable_for_multiple_producers() {
        let (tx, rx) = new_shell_event_bus();
        let tx2 = tx.clone();
        tx.send(ShellEvent::DocumentDeleted {
            document_id: "a".into(),
        });
        tx2.send(ShellEvent::DocumentDeleted {
            document_id: "b".into(),
        });
        assert_eq!(rx.drain().len(), 2, "events from two producers both arrive");
    }
}
