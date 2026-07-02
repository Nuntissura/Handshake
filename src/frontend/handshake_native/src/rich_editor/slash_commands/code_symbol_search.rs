//! The `/code-ref` code-symbol search dialog (WP-KERNEL-012 MT-034, cluster E5).
//!
//! When the operator runs the `/code-ref` slash command, this dialog opens a floating
//! [`egui::Window`] with a fuzzy-search input over code symbols (the EXISTING code-nav lookup,
//! `lookupCodeSymbols` -> `GET /knowledge/code/symbols`). Selecting a result inserts a `code`-kind
//! `hsLink` atom at the caret (via [`super::executor::insert_code_ref_atom`]) — the SAME node MT-014
//! media embeds + MT-015 wikilinks render, NOT an invented `code_ref` node, so it round-trips the
//! backend `content_json` (AC-1).
//!
//! ## Async load discipline (HBR-QUIET, no perpetual spinner)
//!
//! The lookup runs off the egui thread on the app tokio runtime and delivers into a one-slot cell the
//! dialog drains each frame (the MT-008/MT-015 delivery-cell shape). The dialog animates only while a
//! lookup is genuinely in flight; an idle / empty / failed state is neutral (the MT-015 idle-repaint
//! lesson). A backend failure renders a typed inline error, never a panic.
//!
//! ## AccessKit (HBR-SWARM / HBR-VIS)
//!
//! - dialog container -> `code-symbol-search` (Role::Dialog),
//! - search input      -> `code-symbol-search-input` (Role::TextField/TextInput),
//! - each result row   -> `code-symbol-result-{symbol_entity_id}` (Role::ListItem, `[Press]`),
//!
//! all via the SAME `accesskit_node_builder` hook the shell + slash menu use.

use std::sync::{Arc, Mutex};

use crate::code_editor::code_nav::{CodeNavClient, CodeSymbolNavProjection};
use crate::error::AppError;

/// One-slot delivery cell for an off-thread code-symbol lookup (the egui dialog drains it next frame).
pub type CodeSymbolLookupCell = Arc<Mutex<Option<Result<Vec<CodeSymbolNavProjection>, AppError>>>>;

/// The live state of the open `/code-ref` code-symbol search dialog. Owned by `RichEditorState`
/// (`Some` while the dialog is open). Holds the query, the loaded results, the load flag, the typed
/// error, the off-thread delivery cell, and the workspace + runtime context the lookup needs.
#[derive(Clone)]
pub struct CodeSymbolSearchState {
    /// The current query text in the dialog's search input.
    pub query: String,
    /// The loaded result projections (capped by the lookup limit). Empty until a search resolves.
    pub results: Vec<CodeSymbolNavProjection>,
    /// True while a lookup is in flight (the only animating state).
    pub loading: bool,
    /// A typed error from the last lookup, if it failed (rendered as an inline chip; fail-closed).
    pub error: Option<String>,
    /// The off-thread delivery cell the spawned lookup writes; drained each frame.
    pub cell: CodeSymbolLookupCell,
    /// The workspace id the lookup scopes to (set by the shell via the editor's code-ref context).
    pub workspace_id: String,
    /// The tokio runtime handle the lookup spawns onto (`None` in a headless unit test that drives the
    /// dialog state directly without a live backend).
    pub runtime: Option<tokio::runtime::Handle>,
    /// The code-nav client the lookup uses (production reqwest by default; injectable for tests).
    pub client: CodeNavClient,
}

impl std::fmt::Debug for CodeSymbolSearchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeSymbolSearchState")
            .field("query", &self.query)
            .field("results", &self.results.len())
            .field("loading", &self.loading)
            .field("error", &self.error)
            .field("workspace_id", &self.workspace_id)
            .finish()
    }
}

impl CodeSymbolSearchState {
    /// Open a fresh dialog scoped to `workspace_id`, spawning lookups onto `runtime` (when present).
    pub fn open(workspace_id: impl Into<String>, runtime: Option<tokio::runtime::Handle>) -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            loading: false,
            error: None,
            cell: Arc::new(Mutex::new(None)),
            workspace_id: workspace_id.into(),
            runtime,
            client: CodeNavClient::production(),
        }
    }

    /// Drain the off-thread lookup delivery cell into `results` / `error`, clearing the loading flag.
    /// Returns `true` when a result (success or failure) was applied this frame (the caller requests a
    /// repaint so the result shows promptly). Called once per frame BEFORE rendering.
    pub fn drain(&mut self) -> bool {
        let taken = self.cell.lock().ok().and_then(|mut g| g.take());
        match taken {
            Some(Ok(results)) => {
                self.results = results;
                self.error = None;
                self.loading = false;
                true
            }
            Some(Err(e)) => {
                self.results.clear();
                self.error = Some(e.to_string());
                self.loading = false;
                true
            }
            None => false,
        }
    }

    /// Spawn a code-symbol lookup for the current `query` off the egui thread (HBR-QUIET). A blank
    /// query clears the results without a backend call. No-op (with a typed error) when no runtime is
    /// installed (a headless test drives the state directly instead). Sets `loading` while in flight.
    pub fn spawn_lookup(&mut self) {
        let query = self.query.trim().to_owned();
        if query.is_empty() {
            self.results.clear();
            self.loading = false;
            return;
        }
        let Some(runtime) = self.runtime.clone() else {
            // No runtime -> cannot reach the backend; surface an honest typed state (never a panic).
            self.error = Some("no runtime: code-symbol search needs the app runtime".to_owned());
            return;
        };
        self.loading = true;
        self.error = None;
        let cell = self.cell.clone();
        let client = self.client.clone();
        let workspace_id = self.workspace_id.clone();
        runtime.spawn(async move {
            let result = client
                .lookup_symbols(
                    &workspace_id,
                    &query,
                    crate::code_editor::code_nav::SYMBOL_LOOKUP_LIMIT,
                )
                .await;
            if let Ok(mut guard) = cell.lock() {
                *guard = Some(result);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_editor::code_nav::CodeSymbolNavProjection;

    fn proj(id: &str, name: &str) -> CodeSymbolNavProjection {
        CodeSymbolNavProjection {
            symbol_entity_id: id.to_owned(),
            display_name: name.to_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn drain_applies_success_and_clears_loading() {
        let mut st = CodeSymbolSearchState::open("ws-1", None);
        st.loading = true;
        *st.cell.lock().unwrap() = Some(Ok(vec![proj("ent-1", "MyStruct")]));
        assert!(st.drain(), "a delivered result is applied");
        assert_eq!(st.results.len(), 1);
        assert_eq!(st.results[0].display_name, "MyStruct");
        assert!(!st.loading, "loading cleared after a result");
        assert!(st.error.is_none());
        assert!(!st.drain(), "the cell is emptied after one drain");
    }

    #[test]
    fn drain_applies_failure_as_typed_error() {
        let mut st = CodeSymbolSearchState::open("ws-1", None);
        st.loading = true;
        *st.cell.lock().unwrap() = Some(Err(AppError::Http("down".into())));
        assert!(st.drain());
        assert!(st.results.is_empty());
        assert!(
            st.error.is_some(),
            "a failure surfaces as a typed error, not a panic"
        );
        assert!(!st.loading);
    }

    #[test]
    fn blank_query_clears_without_backend() {
        let mut st = CodeSymbolSearchState::open("ws-1", None);
        st.results = vec![proj("ent-1", "x")];
        st.query = "   ".to_owned();
        st.spawn_lookup();
        assert!(st.results.is_empty(), "a blank query clears results");
        assert!(!st.loading);
    }

    #[test]
    fn lookup_without_runtime_is_typed_error_not_panic() {
        let mut st = CodeSymbolSearchState::open("ws-1", None);
        st.query = "MyStruct".to_owned();
        st.spawn_lookup(); // no runtime installed
        assert!(
            st.error.is_some(),
            "no-runtime is a typed error state, never a panic"
        );
    }
}
