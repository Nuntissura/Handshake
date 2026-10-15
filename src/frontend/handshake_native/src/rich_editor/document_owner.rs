use std::convert::Infallible;
use std::sync::{Arc, Mutex};

use handshake_rich_document_model::edit_state::{DocumentEditState, EditCommit, EditError};
use handshake_rich_document_model::history::UndoManager;
use handshake_rich_document_model::node::BlockNode;
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::local_account::AuthenticatedContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorSaveContext {
    pub document_incarnation: Uuid,
    pub context_generation: u64,
    pub edit_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentIdentity {
    workspace_id: String,
    document_id: String,
    document_incarnation: Uuid,
}

impl DocumentIdentity {
    pub fn workspace_id(&self) -> &str { &self.workspace_id }
    pub fn document_id(&self) -> &str { &self.document_id }
    pub fn document_incarnation(&self) -> Uuid { self.document_incarnation }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DocumentOwnerError {
    #[error("document owner is retired or its account session is inactive")]
    Unavailable,
    #[error("document owner context or edit revision changed")]
    StaleContext,
    #[error("document owner counter exhausted")]
    CounterExhausted,
    #[error("loaded document version is older than the known server version")]
    OlderServerVersion,
    #[error("document identity differs from the owner's bound document")]
    WrongDocument,
    #[error("initial load would replace unsaved document edits")]
    UnsavedChanges,
    #[error("document initialization was rejected")]
    InitializationRejected,
}

#[derive(Debug)]
pub enum DocumentOwnerEditError<E> {
    Owner(DocumentOwnerError),
    Mutation(E),
}

pub struct DocumentSaveSnapshot {
    content_json: Value,
    context: EditorSaveContext,
    document_identity: DocumentIdentity,
}

pub struct DocumentEditCandidate {
    owner: Arc<Mutex<OwnerState>>,
    context: EditorSaveContext,
    document: BlockNode,
    history: UndoManager,
    mutation_identity: Arc<()>,
}

impl DocumentEditCandidate {
    pub fn context(&self) -> EditorSaveContext { self.context }
    pub fn parts_mut(&mut self) -> (&mut BlockNode, &mut UndoManager) {
        (&mut self.document, &mut self.history)
    }
}

impl DocumentSaveSnapshot {
    pub fn content_json(&self) -> &Value {
        &self.content_json
    }

    pub fn context(&self) -> EditorSaveContext {
        self.context
    }

    pub fn document_identity(&self) -> &DocumentIdentity {
        &self.document_identity
    }

    pub fn into_parts(self) -> (Value, EditorSaveContext) {
        (self.content_json, self.context)
    }
}

pub struct RichDocumentLoadToken {
    owner: Arc<Mutex<OwnerState>>,
    context: EditorSaveContext,
    request_id: u64,
    document_id: String,
    pane_id: String,
    view_incarnation: Uuid,
    dispatch_generation: u64,
    observed_server_floor: u64,
}

impl RichDocumentLoadToken {
    pub fn document_id(&self) -> &str { &self.document_id }
    pub fn pane_id(&self) -> &str { &self.pane_id }
    pub fn view_incarnation(&self) -> Uuid { self.view_incarnation }
    pub fn dispatch_generation(&self) -> u64 { self.dispatch_generation }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLoadDisposition {
    Initialized,
    AlreadyLoaded,
}

#[derive(Clone)]
pub struct DocumentEditOwner {
    inner: Arc<Mutex<OwnerState>>,
}

#[derive(Clone)]
struct OwnerBinding {
    workspace_id: String,
    document_id: String,
    endpoint: String,
    runtime_id: Option<tokio::runtime::Id>,
    authentication: Option<Arc<AuthenticatedContext>>,
}

#[derive(Clone)]
struct OwnerState {
    edits: DocumentEditState,
    incarnation: Uuid,
    generation: u64,
    retired: bool,
    binding: Option<OwnerBinding>,
    saved_revision: u64,
    known_server_version: u64,
    load_request_id: u64,
    loaded: bool,
    mutation_identity: Arc<()>,
}

impl OwnerState {
    fn current_context(&self) -> Option<EditorSaveContext> {
        if self.retired
            || self.binding.as_ref().is_some_and(|binding| {
                binding
                    .authentication
                    .as_ref()
                    .is_some_and(|authentication| !authentication.is_active())
            })
        {
            return None;
        }
        Some(EditorSaveContext {
            document_incarnation: self.incarnation,
            context_generation: self.generation,
            edit_revision: self.edits.edit_revision(),
        })
    }
}

pub struct DocumentSaveGate<'a> {
    state: &'a mut OwnerState,
}

impl DocumentSaveGate<'_> {
    pub fn document(&self) -> &BlockNode {
        self.state.edits.document()
    }

    pub fn current_context(&self) -> Option<EditorSaveContext> {
        self.state.current_context()
    }

    pub fn document_identity(&self) -> Option<DocumentIdentity> {
        self.state.binding.as_ref().map(|binding| DocumentIdentity {
            workspace_id: binding.workspace_id.clone(), document_id: binding.document_id.clone(),
            document_incarnation: self.state.incarnation,
        })
    }

    pub fn known_server_version(&self) -> u64 {
        self.state.known_server_version
    }

    pub fn document_is_loaded(&self) -> bool {
        self.state.loaded
    }

    pub fn is_dirty(&self) -> bool {
        self.state.saved_revision != self.state.edits.edit_revision()
    }

    pub fn observe_server_version(&mut self, expected: EditorSaveContext, version: u64) -> Result<(), DocumentOwnerError> {
        if self.current_context() != Some(expected) { return Err(DocumentOwnerError::StaleContext) }
        if version < self.state.known_server_version { return Err(DocumentOwnerError::OlderServerVersion) }
        self.state.known_server_version = version;
        Ok(())
    }

    pub fn accept_server_document(
        &mut self,
        expected: EditorSaveContext,
        document: BlockNode,
        server_version: u64,
    ) -> Result<EditorSaveContext, DocumentOwnerError> {
        if self.current_context() != Some(expected) { return Err(DocumentOwnerError::StaleContext) }
        if server_version < self.state.known_server_version { return Err(DocumentOwnerError::OlderServerVersion) }
        self.state.edits.edit(expected.edit_revision, |candidate, history| {
            *candidate = document;
            *history = UndoManager::new();
            Ok::<_, Infallible>(())
        }).map_err(|error| match error {
            EditError::StaleRevision => DocumentOwnerError::StaleContext,
            EditError::RevisionOverflow => DocumentOwnerError::CounterExhausted,
            EditError::Mutation(never) => match never {},
        })?;
        self.state.known_server_version = server_version;
        self.state.saved_revision = self.state.edits.edit_revision();
        self.state.loaded = true;
        self.state.mutation_identity = Arc::new(());
        self.current_context().ok_or(DocumentOwnerError::Unavailable)
    }

    pub fn record_save_commit(&mut self, saved: EditorSaveContext, server_version: u64) -> bool {
        let Some(current) = self.current_context() else { return false };
        if saved.document_incarnation != current.document_incarnation
            || saved.context_generation != current.context_generation
            || saved.edit_revision > current.edit_revision
            || server_version < self.state.known_server_version
        {
            return false;
        }
        self.state.known_server_version = server_version;
        if saved == current {
            self.state.saved_revision = current.edit_revision;
            true
        } else {
            false
        }
    }
}

impl DocumentEditOwner {
    pub fn new(document: BlockNode) -> Self {
        Self {
            inner: Arc::new(Mutex::new(OwnerState {
                edits: DocumentEditState::new(document),
                incarnation: Uuid::now_v7(),
                generation: 0,
                retired: false,
                binding: None,
                saved_revision: 0,
                known_server_version: 0,
                load_request_id: 0,
                loaded: false,
                mutation_identity: Arc::new(()),
            })),
        }
    }

    pub fn same_owner(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    pub fn document_identity(&self) -> Option<DocumentIdentity> {
        self.with_save_gate(|gate| gate.document_identity())
    }

    pub fn prepare_edit(&self) -> Result<DocumentEditCandidate, DocumentOwnerError> {
        let state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        let context = state.current_context().ok_or(DocumentOwnerError::Unavailable)?;
        Ok(DocumentEditCandidate {
            owner: Arc::clone(&self.inner), context,
            document: state.edits.document().clone(), history: state.edits.history().clone(),
            mutation_identity: Arc::clone(&state.mutation_identity),
        })
    }

    pub fn commit_edit(&self, candidate: DocumentEditCandidate) -> Result<EditCommit<()>, DocumentOwnerError> {
        if !Arc::ptr_eq(&self.inner, &candidate.owner) { return Err(DocumentOwnerError::StaleContext) }
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        if state.current_context() != Some(candidate.context)
            || !Arc::ptr_eq(&state.mutation_identity, &candidate.mutation_identity)
        { return Err(DocumentOwnerError::StaleContext) }
        let commit = state.edits.edit(candidate.context.edit_revision, |document, history| {
            *document = candidate.document;
            *history = candidate.history;
            Ok::<_, Infallible>(())
        }).map_err(|error| match error {
            EditError::StaleRevision => DocumentOwnerError::StaleContext,
            EditError::RevisionOverflow => DocumentOwnerError::CounterExhausted,
            EditError::Mutation(never) => match never {},
        })?;
        state.mutation_identity = Arc::new(());
        Ok(commit)
    }

    pub fn current_context(&self) -> Result<EditorSaveContext, DocumentOwnerError> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
            .current_context().ok_or(DocumentOwnerError::Unavailable)
    }

    pub fn document_snapshot(&self) -> Result<BlockNode, DocumentOwnerError> {
        let state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        state.current_context().ok_or(DocumentOwnerError::Unavailable)?;
        Ok(state.edits.document().clone())
    }

    pub fn with_document<R>(&self, read: impl FnOnce(&BlockNode) -> R) -> Result<R, DocumentOwnerError> {
        let state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        state.current_context().ok_or(DocumentOwnerError::Unavailable)?;
        Ok(read(state.edits.document()))
    }

    pub fn history_snapshot(&self) -> Result<UndoManager, DocumentOwnerError> {
        let state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        state.current_context().ok_or(DocumentOwnerError::Unavailable)?;
        Ok(state.edits.history().clone())
    }

    pub fn edit<R, E>(
        &self,
        expected: EditorSaveContext,
        mutate: impl FnOnce(&mut BlockNode, &mut UndoManager) -> Result<R, E>,
    ) -> Result<EditCommit<R>, DocumentOwnerEditError<E>> {
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        let current = state.current_context()
            .ok_or(DocumentOwnerEditError::Owner(DocumentOwnerError::Unavailable))?;
        if current != expected {
            return Err(DocumentOwnerEditError::Owner(DocumentOwnerError::StaleContext));
        }
        let commit = state.edits.edit(expected.edit_revision, mutate).map_err(|error| match error {
            EditError::StaleRevision => DocumentOwnerEditError::Owner(DocumentOwnerError::StaleContext),
            EditError::RevisionOverflow => DocumentOwnerEditError::Owner(DocumentOwnerError::CounterExhausted),
            EditError::Mutation(error) => DocumentOwnerEditError::Mutation(error),
        })?;
        state.mutation_identity = Arc::new(());
        Ok(commit)
    }

    pub fn save_snapshot(&self) -> Result<DocumentSaveSnapshot, DocumentOwnerError> {
        let state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        let context = state.current_context().ok_or(DocumentOwnerError::Unavailable)?;
        let binding = state.binding.as_ref().ok_or(DocumentOwnerError::WrongDocument)?;
        Ok(DocumentSaveSnapshot {
            content_json: handshake_rich_document_model::doc_json::to_content_json_value(state.edits.document()),
            context,
            document_identity: DocumentIdentity {
                workspace_id: binding.workspace_id.clone(), document_id: binding.document_id.clone(),
                document_incarnation: state.incarnation,
            },
        })
    }

    pub fn with_save_gate<R>(&self, apply: impl FnOnce(&mut DocumentSaveGate<'_>) -> R) -> R {
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        apply(&mut DocumentSaveGate { state: &mut state })
    }

    pub fn bind_context(
        &self,
        workspace_id: &str,
        document_id: &str,
        endpoint: &str,
        runtime: Option<&tokio::runtime::Handle>,
        authentication: Option<Arc<AuthenticatedContext>>,
    ) -> Result<bool, DocumentOwnerError> {
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        if state.retired { return Err(DocumentOwnerError::Unavailable) }
        if workspace_id.is_empty() || document_id.is_empty() {
            return Err(DocumentOwnerError::WrongDocument);
        }
        if state.binding.as_ref().is_some_and(|binding|
            binding.workspace_id != workspace_id || binding.document_id != document_id)
        {
            return Err(DocumentOwnerError::WrongDocument);
        }
        let runtime_id = runtime.map(tokio::runtime::Handle::id);
        let same = state.binding.as_ref().is_some_and(|binding| {
            binding.workspace_id == workspace_id && binding.document_id == document_id
                && binding.endpoint == endpoint && binding.runtime_id == runtime_id
                && match (&binding.authentication, &authentication) {
                    (Some(left), Some(right)) => Arc::ptr_eq(left, right)
                        && left.account_id == right.account_id && left.principal_id == right.principal_id
                        && left.session_id == right.session_id && left.access_space_id == right.access_space_id,
                    (None, None) => true,
                    _ => false,
                }
        });
        if same { return Ok(false) }
        state.generation = state.generation.checked_add(1).ok_or(DocumentOwnerError::CounterExhausted)?;
        state.binding = Some(OwnerBinding {
            workspace_id: workspace_id.to_owned(), document_id: document_id.to_owned(),
            endpoint: endpoint.to_owned(), runtime_id, authentication,
        });
        Ok(true)
    }

    pub fn invalidate(&self) {
        self.inner.lock().unwrap_or_else(|error| error.into_inner()).retired = true;
    }

    pub fn advance_context(&self) -> Result<EditorSaveContext, DocumentOwnerError> {
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        state.current_context().ok_or(DocumentOwnerError::Unavailable)?;
        state.generation = state.generation.checked_add(1).ok_or(DocumentOwnerError::CounterExhausted)?;
        state.current_context().ok_or(DocumentOwnerError::Unavailable)
    }

    pub fn begin_load(&self, pane_id: &str, view_incarnation: Uuid, dispatch_generation: u64) -> Result<RichDocumentLoadToken, DocumentOwnerError> {
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        let context = state.current_context().ok_or(DocumentOwnerError::Unavailable)?;
        state.load_request_id = state.load_request_id.checked_add(1).ok_or(DocumentOwnerError::CounterExhausted)?;
        let document_id = state.binding.as_ref().ok_or(DocumentOwnerError::WrongDocument)?.document_id.clone();
        Ok(RichDocumentLoadToken {
            owner: Arc::clone(&self.inner), context, request_id: state.load_request_id,
            document_id, pane_id: pane_id.to_owned(), view_incarnation, dispatch_generation,
            observed_server_floor: state.known_server_version,
        })
    }

    pub fn apply_loaded_document<R>(
        &self,
        token: RichDocumentLoadToken,
        document: BlockNode,
        server_version: u64,
        install: impl FnOnce(&mut DocumentSaveGate<'_>, DocumentLoadDisposition) -> Result<R, DocumentOwnerError>,
    ) -> Result<R, DocumentOwnerError> {
        if !Arc::ptr_eq(&self.inner, &token.owner) { return Err(DocumentOwnerError::StaleContext) }
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        if state.current_context() != Some(token.context) || state.load_request_id != token.request_id {
            return Err(DocumentOwnerError::StaleContext);
        }
        if server_version < state.known_server_version || server_version < token.observed_server_floor {
            return Err(DocumentOwnerError::OlderServerVersion);
        }
        if state.loaded {
            return install(&mut DocumentSaveGate { state: &mut state }, DocumentLoadDisposition::AlreadyLoaded);
        }
        if state.saved_revision != state.edits.edit_revision() {
            return Err(DocumentOwnerError::UnsavedChanges);
        }
        let next_generation = state.generation.checked_add(1).ok_or(DocumentOwnerError::CounterExhausted)?;
        let before_install = state.clone();
        state.edits.edit(token.context.edit_revision, |candidate, history| {
            *candidate = document;
            *history = UndoManager::new();
            Ok::<_, Infallible>(())
        }).map_err(|error| match error {
            EditError::StaleRevision => DocumentOwnerError::StaleContext,
            EditError::RevisionOverflow => DocumentOwnerError::CounterExhausted,
            EditError::Mutation(never) => match never {},
        })?;
        state.known_server_version = server_version;
        state.saved_revision = state.edits.edit_revision();
        state.generation = next_generation;
        state.loaded = true;
        state.mutation_identity = Arc::new(());
        match install(&mut DocumentSaveGate { state: &mut state }, DocumentLoadDisposition::Initialized) {
            Ok(result) => Ok(result),
            Err(error) => { *state = before_install; Err(error) }
        }
    }

    pub fn admit_load_failure<R>(
        &self,
        token: RichDocumentLoadToken,
        failure: impl FnOnce(&mut DocumentSaveGate<'_>) -> R,
    ) -> Result<R, DocumentOwnerError> {
        if !Arc::ptr_eq(&self.inner, &token.owner) { return Err(DocumentOwnerError::StaleContext) }
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        if state.current_context() != Some(token.context) || state.load_request_id != token.request_id
            || state.known_server_version > token.observed_server_floor
        { return Err(DocumentOwnerError::StaleContext) }
        Ok(failure(&mut DocumentSaveGate { state: &mut state }))
    }
}
