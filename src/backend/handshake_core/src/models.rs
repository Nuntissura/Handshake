pub use crate::capabilities::{CapabilityProfile, CapabilityRegistry, RegistryError};
pub use crate::storage::{AiJob, JobKind, JobState, WorkflowRun};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::ace::ArtifactHandle;
use crate::role_mailbox::GovernanceMode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),

    #[error("Capability Registry Error: {0}")]
    Registry(#[from] RegistryError),

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub component: &'static str,
    pub version: &'static str,
    pub db_status: String,
    pub migration_version: Option<i64>,
}

#[derive(Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct WorkspaceResponse {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateDocumentRequest {
    pub title: String,
}

#[derive(Serialize)]
pub struct DocumentResponse {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct BlockResponse {
    pub id: String,
    pub kind: String,
    pub sequence: i64,
    pub raw_content: String,
    pub display_content: String,
    pub derived_content: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct DocumentWithBlocksResponse {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub blocks: Vec<BlockResponse>,
}

#[derive(Deserialize)]
pub struct UpsertBlocksRequest {
    pub blocks: Vec<IncomingBlock>,
}

#[derive(Deserialize)]
pub struct IncomingBlock {
    pub id: Option<String>,
    pub kind: String,
    pub sequence: i64,
    pub raw_content: String,
    pub display_content: Option<String>,
    pub derived_content: Option<Value>,
}

#[derive(Deserialize)]
pub struct CreateCanvasRequest {
    pub title: String,
}

#[derive(Serialize)]
pub struct CanvasResponse {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct CanvasNodeResponse {
    pub id: String,
    pub canvas_id: String,
    pub kind: String,
    pub position_x: f64,
    pub position_y: f64,
    pub data: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct CanvasEdgeResponse {
    pub id: String,
    pub canvas_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub kind: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct CanvasWithGraphResponse {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub nodes: Vec<CanvasNodeResponse>,
    pub edges: Vec<CanvasEdgeResponse>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: &'static str,
}

// ----------------------------------------------------------------------------
// Spec Router job inputs (Master Spec §2.6.6.6.5)
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecRouterJobProfile {
    /// Prompt artifact to route
    pub prompt_ref: ArtifactHandle,
    /// SpecIntent identifier
    pub spec_intent_id: String,
    /// Optional operator override for governance mode
    pub mode_override: Option<GovernanceMode>,
    /// Optional override for deterministic prompt compilation.
    /// If None, runtime MUST use the workspace default SpecPromptPack (see Master Spec §2.6.8.5.2).
    pub spec_prompt_pack_id: Option<String>,
    /// Workspace and project context
    pub workspace_id: Uuid,
    pub project_id: Option<Uuid>,
    /// Workflow context for safety behavior (e.g., git workflows)
    pub workflow_context: WorkflowContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContext {
    pub version_control: VersionControl,
    pub repo_root: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VersionControl {
    None,
    Git,
}
