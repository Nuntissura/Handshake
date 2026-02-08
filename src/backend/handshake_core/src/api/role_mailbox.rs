use std::fs;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use crate::role_mailbox::{
    AddTranscriptionLinkRequest, CreateRoleMailboxMessageRequest, RoleId, RoleMailbox,
    RoleMailboxContext, RoleMailboxMessage, RoleMailboxMessageType, TranscriptionLink,
};
use crate::runtime_governance::RuntimeGovernancePaths;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateMessageApiRequest {
    pub thread_id: Option<String>,
    pub thread_subject: Option<String>,
    pub thread_participants: Option<Vec<RoleId>>,
    pub context: RoleMailboxContext,
    pub from_role: RoleId,
    pub to_roles: Vec<RoleId>,
    pub message_type: RoleMailboxMessageType,
    pub body: String,
    pub attachments: Vec<crate::ace::ArtifactHandle>,
    pub relates_to_message_id: Option<String>,
    pub transcription_links: Vec<TranscriptionLink>,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize)]
pub struct AddTranscriptionApiRequest {
    pub thread_id: String,
    pub message_id: String,
    pub link: TranscriptionLink,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/role_mailbox/index", get(read_index))
        .route("/role_mailbox/messages", post(create_message))
        .route("/role_mailbox/transcriptions", post(add_transcription_link))
        .with_state(state)
}

async fn read_index() -> Result<Json<Value>, String> {
    let runtime_paths = RuntimeGovernancePaths::resolve().map_err(|e| e.to_string())?;
    let index_path = runtime_paths.role_mailbox_export_dir().join("index.json");
    let raw = fs::read_to_string(&index_path).map_err(|e| e.to_string())?;
    let parsed: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    Ok(Json(parsed))
}

async fn create_message(
    State(state): State<AppState>,
    Json(req): Json<CreateMessageApiRequest>,
) -> Result<Json<RoleMailboxMessage>, String> {
    let mailbox =
        RoleMailbox::new_for_repo(state.flight_recorder.clone()).map_err(|e| e.to_string())?;
    let internal = CreateRoleMailboxMessageRequest {
        thread_id: req.thread_id,
        thread_subject: req.thread_subject,
        thread_participants: req.thread_participants,
        context: req.context,
        from_role: req.from_role,
        to_roles: req.to_roles,
        message_type: req.message_type,
        body: req.body,
        attachments: req.attachments,
        relates_to_message_id: req.relates_to_message_id,
        transcription_links: req.transcription_links,
        idempotency_key: req.idempotency_key,
    };

    mailbox
        .create_message(internal)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

async fn add_transcription_link(
    State(state): State<AppState>,
    Json(req): Json<AddTranscriptionApiRequest>,
) -> Result<Json<Value>, String> {
    let mailbox =
        RoleMailbox::new_for_repo(state.flight_recorder.clone()).map_err(|e| e.to_string())?;
    let internal = AddTranscriptionLinkRequest {
        thread_id: req.thread_id,
        message_id: req.message_id,
        link: req.link,
    };
    mailbox
        .add_transcription_link(internal)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
