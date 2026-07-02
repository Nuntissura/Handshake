// EXTENSION SEAM: this file is a design-only stub. Future surfaces implement EditorSurface and call
// register_surface(). No existing production code calls into this module at runtime. See
// WP-KERNEL-012 MT-036 for the design contract.
//
//! WP-KERNEL-012 MT-036 (E5 — designed extension seams for image/spreadsheet/engines).
//!
//! ## Why this file is DESIGN-ONLY (and why that is legitimate, not scaffolding)
//!
//! The WP-012 interconnection contract requires that FUTURE editor surfaces (an image/photo editor,
//! a spreadsheet, a mechanical engine) attach to the SAME melt-together substrate — the shared
//! selection, the one event ledger, and the unified undo — WITHOUT a breaking refactor of the existing
//! emitter or bus. Designing that seam NOW (when the substrate is fresh) is far cheaper than retrofitting
//! it when the first future surface arrives. The MT contract EXPLICITLY scopes this file as design-only
//! Rust trait stubs that COMPILE but have NO runtime path; it is distinct from hidden unwired scaffolding
//! masquerading as a live feature (the no-mockup rule's target). Everything here carries
//! `#[allow(dead_code)]` because, by design, no production code calls it until a future surface registers.
//!
//! ## Object safety (RISK-3 / MC-3 — the hard constraint)
//!
//! [`EditorSurface`] MUST stay object-safe so the registry can hold `Box<dyn EditorSurface>`. The rules
//! the design obeys (and that a reviewer must re-check before extending it):
//!   - every method takes `&self` (no `self` by value, no `&mut self` that would prevent shared dispatch);
//!   - NO generic methods (a generic method makes the trait non-dispatchable as `dyn`);
//!   - NO associated types without a default;
//!   - parameters use CONCRETE types ([`SharedSelection`], [`NativeEditorEvent`],
//!     [`NativeEditorEventEmitter`]) so no type parameter leaks into the vtable.
//!
//! The FUTURE method signatures a real surface will add are written as comments at the bottom of this
//! file; each is pre-verified object-safe so adding it later does not break the `dyn` registry.

#![allow(dead_code)]

use std::collections::HashMap;

use crate::event_emitter::{NativeEditorEvent, NativeEditorEventEmitter};
use crate::interop::interaction_bus::SharedSelection;

/// The result of a surface-local undo/redo the registry surfaces to the unified undo scope. A future
/// surface returns `Some` when it handled the undo locally; `None` when it had nothing to undo (so the
/// unified scope can fall through to the next ring). Concrete (not generic) to preserve object safety.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoResult {
    /// True when the surface applied the undo/redo cleanly.
    pub ok: bool,
    /// A short description of what was undone/redone (for the undo-history inspector).
    pub description: String,
}

/// A future editor surface that plugs into the melt-together substrate. DESIGN-ONLY: no production code
/// implements or calls this today. Object-safe (all `&self`, concrete params, no generics/associated
/// types) so [`EditorSurfaceRegistry`] can hold `Box<dyn EditorSurface>`.
pub trait EditorSurface: Send + Sync {
    /// A stable surface id (e.g. `"image_editor"`, `"spreadsheet"`, `"engine"`). `&'static str` so the
    /// registry keys on it without allocation.
    fn surface_id(&self) -> &'static str;

    /// Called when the shared selection changes: the future surface reacts (e.g. highlights a referenced
    /// region, updates a property inspector). Receives the CONCRETE [`SharedSelection`] (object-safe).
    fn on_selection_changed(&self, selection: &SharedSelection);

    /// Called when a native editor event is emitted: the future surface observes the cross-surface event
    /// stream (e.g. to mirror a document_saved into its own cache). Receives the CONCRETE event +
    /// the emitter so it can emit its OWN follow-on events through the SAME ledger (object-safe).
    fn on_event_emitted(&self, event: &NativeEditorEvent, emitter: &NativeEditorEventEmitter);

    /// A surface-local undo (the unified scope dispatches this when the focused pane is this surface).
    /// `Some(result)` when handled; `None` when nothing to undo.
    fn undo_local(&self) -> Option<UndoResult>;

    /// A surface-local redo (mirror of [`Self::undo_local`]).
    fn redo_local(&self) -> Option<UndoResult>;
}

/// The registry future surfaces register into at startup. Stored ALONGSIDE the shared command/event bus
/// (the MT-031 `interop::interaction_bus`) and the MT-035 `UnifiedUndoScope`. On a SharedSelection change
/// the bus would iterate the registry and call [`EditorSurface::on_selection_changed`]; on an
/// `emit_event` it would call [`EditorSurface::on_event_emitted`]. In production (before any future
/// surface registers) the registry is EMPTY and every iteration is a no-op — adding the first future
/// surface changes NO existing code.
#[derive(Default)]
pub struct EditorSurfaceRegistry {
    surfaces: HashMap<&'static str, Box<dyn EditorSurface>>,
}

impl EditorSurfaceRegistry {
    /// A fresh empty registry (the production state until a future surface registers).
    pub fn new() -> Self {
        Self {
            surfaces: HashMap::new(),
        }
    }

    /// Register a future surface (called once at startup by that surface's bootstrap). Keyed by
    /// [`EditorSurface::surface_id`]; a re-registration replaces the prior instance.
    pub fn register_surface(&mut self, surface: Box<dyn EditorSurface>) {
        let id = surface.surface_id();
        self.surfaces.insert(id, surface);
    }

    /// How many surfaces are registered (0 in production today).
    pub fn len(&self) -> usize {
        self.surfaces.len()
    }

    /// True when no surface is registered (the production state today — the iterations are no-ops).
    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
    }

    /// True when a surface with `surface_id` is registered.
    pub fn contains(&self, surface_id: &str) -> bool {
        self.surfaces.contains_key(surface_id)
    }

    /// Fan a SharedSelection change out to every registered surface (a no-op when empty — production).
    pub fn dispatch_selection_changed(&self, selection: &SharedSelection) {
        for surface in self.surfaces.values() {
            surface.on_selection_changed(selection);
        }
    }

    /// Fan an emitted event out to every registered surface (a no-op when empty — production).
    pub fn dispatch_event_emitted(
        &self,
        event: &NativeEditorEvent,
        emitter: &NativeEditorEventEmitter,
    ) {
        for surface in self.surfaces.values() {
            surface.on_event_emitted(event, emitter);
        }
    }
}

// ── FUTURE method signatures (pre-verified object-safe; written as comments per MC-3) ─────────────────
//
// When a future surface needs richer integration, these are the signatures it may add to the
// EditorSurface trait. Each is object-safe (all &self, concrete params, no generics, no associated
// types). They are documented here so a future author does NOT accidentally add a generic/Self-by-value
// method that would break the `Box<dyn EditorSurface>` registry:
//
//   fn on_clipboard_paste(&self, payload: &crate::interop::interaction_bus::ClipboardPayload);
//   fn on_command_dispatched(&self, command_id: &str);
//   fn accesskit_author_id(&self) -> &'static str;       // stable a11y address for the surface root
//   fn route_to_stage(&self) -> Option<crate::stage_pane::StageContent>;
//   fn supports_action(&self, action: crate::event_emitter::NativeEditorAction) -> bool;
//
// NON-object-safe shapes a future author MUST AVOID (each would make `dyn EditorSurface` illegal):
//   fn handle<T: Serialize>(&self, value: T);            // BAD: generic method
//   fn build(self) -> Box<dyn EditorSurface>;            // BAD: Self by value
//   type Selection;                                      // BAD: associated type without default

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_emitter::{NativeEditorEvent, NativeEditorEventEmitter, UndoScope};
    use std::sync::{Arc, Mutex};

    /// A mock future surface (proves the trait is object-safe + the registry dispatches callbacks). It
    /// records each callback so a test can assert the registry called it on selection change + emit.
    struct MockSurface {
        selection_changes: Arc<Mutex<usize>>,
        events_observed: Arc<Mutex<Vec<String>>>,
    }
    impl EditorSurface for MockSurface {
        fn surface_id(&self) -> &'static str {
            "mock_image_editor"
        }
        fn on_selection_changed(&self, _selection: &SharedSelection) {
            *self.selection_changes.lock().unwrap() += 1;
        }
        fn on_event_emitted(&self, event: &NativeEditorEvent, _emitter: &NativeEditorEventEmitter) {
            self.events_observed
                .lock()
                .unwrap()
                .push(event.action.as_str().to_owned());
        }
        fn undo_local(&self) -> Option<UndoResult> {
            Some(UndoResult {
                ok: true,
                description: "mock undo".to_owned(),
            })
        }
        fn redo_local(&self) -> Option<UndoResult> {
            None
        }
    }

    #[test]
    fn registry_is_empty_in_production() {
        // The production state: no surface registered -> every iteration is a no-op.
        let reg = EditorSurfaceRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        // Dispatching on an empty registry is a safe no-op (does not panic).
        reg.dispatch_selection_changed(&SharedSelection::None);
    }

    #[test]
    fn registered_mock_surface_receives_selection_and_event_callbacks() {
        // The seam-design proof: a registered Box<dyn EditorSurface> receives on_selection_changed AND
        // on_event_emitted (proving the object-safe trait + the registry fan-out work).
        let selection_changes = Arc::new(Mutex::new(0usize));
        let events_observed = Arc::new(Mutex::new(Vec::new()));
        let mut reg = EditorSurfaceRegistry::new();
        reg.register_surface(Box::new(MockSurface {
            selection_changes: Arc::clone(&selection_changes),
            events_observed: Arc::clone(&events_observed),
        }));
        assert_eq!(reg.len(), 1);
        assert!(reg.contains("mock_image_editor"));

        reg.dispatch_selection_changed(&SharedSelection::None);
        assert_eq!(*selection_changes.lock().unwrap(), 1);

        let emitter = NativeEditorEventEmitter::new(
            "WS-1",
            Arc::new(crate::event_emitter::RuntimeChatLedgerTransport::new(
                "http://test",
            )),
            None,
        );
        let event = NativeEditorEvent::undo_fired(UndoScope::Local, "pane-rich", "act", "WS-1");
        reg.dispatch_event_emitted(&event, &emitter);
        assert_eq!(
            events_observed.lock().unwrap().as_slice(),
            &["undo_fired".to_owned()]
        );
    }

    #[test]
    fn trait_is_object_safe() {
        // Compile-time proof of object safety: if EditorSurface were not object-safe, constructing a
        // Box<dyn EditorSurface> would not compile.
        let _boxed: Box<dyn EditorSurface> = Box::new(MockSurface {
            selection_changes: Arc::new(Mutex::new(0)),
            events_observed: Arc::new(Mutex::new(Vec::new())),
        });
        assert_eq!(_boxed.surface_id(), "mock_image_editor");
        assert_eq!(_boxed.undo_local().unwrap().description, "mock undo");
        assert!(_boxed.redo_local().is_none());
    }
}
