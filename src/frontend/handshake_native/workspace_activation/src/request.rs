use std::sync::Arc;

pub struct WorkspaceActivationRequest<Account, Handle> {
    pub operation_id: uuid::Uuid,
    pub generation: u64,
    pub backend_readiness_lost: bool,
    pub workspace_id: String,
    pub account: Arc<Account>,
    pub handle: Option<Handle>,
}
