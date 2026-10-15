use std::sync::Arc;

use crate::WorkspaceActivationRequest;

pub struct WorkspaceActivationState<Account, Handle> {
    required: bool,
    has_committed_workspace: bool,
    generation: u64,
    admitted: Option<(Arc<Account>, String)>,
    requests: Vec<WorkspaceActivationRequest<Account, Handle>>,
    desired: Option<String>,
    error: Option<String>,
}

impl<Account, Handle> Default for WorkspaceActivationState<Account, Handle> {
    fn default() -> Self {
        Self {
            required: false,
            has_committed_workspace: false,
            generation: 0,
            admitted: None,
            requests: Vec::new(),
            desired: None,
            error: None,
        }
    }
}

impl<Account, Handle> WorkspaceActivationState<Account, Handle> {
    pub fn required(&self) -> bool {
        self.required
    }

    pub fn set_required(&mut self, required: bool) {
        self.required = required;
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn has_committed_workspace(&self) -> bool {
        self.has_committed_workspace
    }

    pub fn queue_workspace(&mut self, workspace_id: &str) -> bool {
        if self.requests.is_empty() {
            return true;
        }
        if self.desired.as_deref() != Some(workspace_id) {
            self.generation = self.generation.wrapping_add(1);
            self.desired = Some(workspace_id.to_owned());
        }
        false
    }

    pub fn start_request(&mut self, workspace_id: &str) -> u64 {
        self.invalidate();
        self.desired = Some(workspace_id.to_owned());
        self.error = None;
        self.generation
    }

    pub fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.admitted = None;
    }

    pub fn mark_backend_readiness_lost(&mut self) {
        self.invalidate();
        for request in &mut self.requests {
            request.backend_readiness_lost = true;
        }
    }

    pub fn requests(&self) -> &[WorkspaceActivationRequest<Account, Handle>] {
        &self.requests
    }

    pub fn take_request(&mut self, index: usize) -> WorkspaceActivationRequest<Account, Handle> {
        self.requests.remove(index)
    }

    pub fn reinsert_request(
        &mut self,
        index: usize,
        request: WorkspaceActivationRequest<Account, Handle>,
    ) {
        self.requests.insert(index, request);
    }

    pub fn transfer_pending_to(&mut self, fresh: &mut Self) {
        fresh.requests = std::mem::take(&mut self.requests);
        fresh.generation = self.generation;
        fresh.desired = None;
    }

    pub fn admitted(&self) -> Option<(&Arc<Account>, &str)> {
        self.admitted
            .as_ref()
            .map(|(account, workspace)| (account, workspace.as_str()))
    }

    pub fn admit(&mut self, account: Arc<Account>, workspace_id: String) {
        self.admitted = Some((account, workspace_id));
        self.has_committed_workspace = true;
        self.error = None;
        self.desired = None;
    }

    pub fn take_desired(&mut self) -> Option<String> {
        self.desired.take()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn set_error(&mut self, error: Option<String>) {
        self.error = error;
    }

    pub fn clear_desired(&mut self) {
        self.desired = None;
    }
}
