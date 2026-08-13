use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use tauri::State;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use handshake_core::swarm_orchestration::resource_scope::ExactResourceScopeAttribution;

const SESSION_CHAT_LOG_SCHEMA_VERSION_V0_1: &str = "hsk.session_chat_log@0.1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionChatRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ans001ValidationRecordV0_1 {
    pub compliant: bool,
    pub violation_clauses: Vec<String>,
}

fn deserialize_option_value_preserve_null<'de, D>(
    deserializer: D,
) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(Value::deserialize(deserializer)?))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionChatLogEntryV0_1 {
    pub schema_version: String,

    pub session_id: String,
    pub turn_index: u64,
    pub created_at_utc: String,
    pub message_id: String,

    /// Trusted product-local scope stamped by the Tauri state. Legacy rows
    /// deserialize as `None` and are hidden by account-facing readers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_scope: Option<ExactResourceScopeAttribution>,

    pub role: SessionChatRole,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_role: Option<String>,

    pub content: String,

    #[serde(
        default,
        deserialize_with = "deserialize_option_value_preserve_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub ans001: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ans001_validation: Option<Ans001ValidationRecordV0_1>,
}

#[derive(Debug, Deserialize)]
pub struct SessionChatLogEntryV0_1Input {
    pub role: SessionChatRole,
    pub content: String,

    #[serde(default)]
    pub model_role: Option<String>,

    #[serde(default)]
    pub ans001: Option<Value>,

    #[serde(default)]
    pub ans001_validation: Option<Ans001ValidationRecordV0_1>,

    #[serde(default)]
    pub message_id: Option<String>,
}

pub struct SessionChatLogState {
    pub session_id: String,
    pub next_turn_index: Mutex<u64>,
    pub app_data_root: PathBuf,
    pub resource_scope: ExactResourceScopeAttribution,
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn require_uuid_string_non_nil(label: &str, raw: &str) -> Result<Uuid, String> {
    let trimmed = raw.trim();
    let id = Uuid::parse_str(trimmed).map_err(|e| format!("{label} must be a UUID string: {e}"))?;
    if id == Uuid::nil() {
        return Err(format!("{label} must be a non-nil UUID"));
    }
    Ok(id)
}

fn sessions_root(app_data_root: &Path) -> PathBuf {
    app_data_root.join("sessions")
}

fn session_dir(app_data_root: &Path, session_id: &str) -> PathBuf {
    sessions_root(app_data_root).join(session_id)
}

fn chat_log_path(app_data_root: &Path, session_id: &str) -> PathBuf {
    session_dir(app_data_root, session_id).join("chat.jsonl")
}

impl SessionChatLogState {
    pub fn new(app_data_root: PathBuf, resource_scope: ExactResourceScopeAttribution) -> Self {
        let session_id = Uuid::now_v7().to_string();
        Self {
            session_id,
            next_turn_index: Mutex::new(0),
            app_data_root,
            resource_scope,
        }
    }
}

#[tauri::command]
pub fn session_chat_get_session_id(state: State<SessionChatLogState>) -> String {
    state.session_id.clone()
}

#[tauri::command]
pub fn session_chat_append(
    state: State<SessionChatLogState>,
    entry: SessionChatLogEntryV0_1Input,
) -> Result<(), String> {
    let mut turn_guard = state
        .next_turn_index
        .lock()
        .map_err(|_| "session chat state mutex poisoned".to_string())?;

    let turn_index = *turn_guard + 1;
    *turn_guard = turn_index;

    let created_at_utc = now_rfc3339();
    let message_id = match entry.message_id {
        Some(id) => require_uuid_string_non_nil("message_id", &id)?.to_string(),
        None => Uuid::now_v7().to_string(),
    };

    let (model_role, ans001, ans001_validation) = match entry.role {
        SessionChatRole::User => {
            if entry.model_role.is_some() {
                return Err("model_role must not be present for role=user".to_string());
            }
            if entry.ans001.is_some() {
                return Err("ans001 must not be present for role=user".to_string());
            }
            if entry.ans001_validation.is_some() {
                return Err("ans001_validation must not be present for role=user".to_string());
            }
            (None, None, None)
        }
        SessionChatRole::Assistant => {
            let model_role = entry.model_role.map(|s| s.trim().to_string());
            if let Some(ref role) = model_role {
                if role.is_empty() {
                    return Err("model_role must be a non-empty string when present".to_string());
                }
            }

            let is_frontend = model_role.as_deref() == Some("frontend");
            if is_frontend {
                let ans001 = entry.ans001.or(Some(Value::Null));
                if let Some(ref val) = ans001 {
                    if !val.is_object() && !val.is_null() {
                        return Err("ans001 must be an object or null".to_string());
                    }
                }
                (model_role, ans001, entry.ans001_validation)
            } else {
                if entry.ans001.is_some() {
                    return Err(
                        "ans001 must only be present when role=assistant and model_role=frontend"
                            .to_string(),
                    );
                }
                if entry.ans001_validation.is_some() {
                    return Err(
                        "ans001_validation must only be present for model_role=frontend"
                            .to_string(),
                    );
                }
                (model_role, None, None)
            }
        }
    };

    let full_entry = SessionChatLogEntryV0_1 {
        schema_version: SESSION_CHAT_LOG_SCHEMA_VERSION_V0_1.to_string(),
        session_id: state.session_id.clone(),
        turn_index,
        created_at_utc,
        message_id,
        resource_scope: Some(state.resource_scope.clone()),
        role: entry.role,
        model_role,
        content: entry.content,
        ans001,
        ans001_validation,
    };

    let dir = session_dir(&state.app_data_root, &state.session_id);
    fs::create_dir_all(&dir).map_err(|e| format!("create_dir_all failed: {e}"))?;

    let path = chat_log_path(&state.app_data_root, &state.session_id);
    let json_line = serde_json::to_string(&full_entry).map_err(|e| e.to_string())?;
    let mut bytes = Vec::with_capacity(json_line.len() + 1);
    bytes.extend_from_slice(json_line.as_bytes());
    bytes.push(b'\n');

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("open chat log failed: {e}"))?;
    file.write_all(&bytes)
        .map_err(|e| format!("append chat log failed: {e}"))?;
    let _ = file.flush();
    let _ = file.sync_data();

    Ok(())
}

/// Shared, command-free reader for a session's `chat.jsonl`. Parses every row,
/// sorts by `(turn_index, created_at_utc, message_id)`, and returns the full
/// ordered set. A missing file yields an empty vec (honest — a swarm composite
/// `instance_id` has no chat file). Reused by both [`session_chat_read`] and the
/// unified session-transcript aggregator so the parse stays single-sourced.
pub fn read_chat_log(
    app_data_root: &Path,
    session_id: &str,
) -> Result<Vec<SessionChatLogEntryV0_1>, String> {
    let path = chat_log_path(app_data_root, session_id);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(&path).map_err(|e| format!("open chat log failed: {e}"))?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| format!("read chat log line {idx} failed: {e}"))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let entry: SessionChatLogEntryV0_1 = serde_json::from_str(trimmed).map_err(|e| {
            format!(
                "parse chat log line {idx} failed (expected {}): {e}",
                SESSION_CHAT_LOG_SCHEMA_VERSION_V0_1
            )
        })?;
        entries.push(entry);
    }

    entries.sort_by(|a, b| {
        (a.turn_index, &a.created_at_utc, &a.message_id).cmp(&(
            b.turn_index,
            &b.created_at_utc,
            &b.message_id,
        ))
    });

    Ok(entries)
}

/// Account-facing reader. It fails closed on legacy, malformed, or foreign
/// rows and requires every disclosed row to carry the exact trusted scope.
pub fn read_chat_log_for_scope(
    app_data_root: &Path,
    session_id: &str,
    scope: &ExactResourceScopeAttribution,
) -> Result<Vec<SessionChatLogEntryV0_1>, String> {
    let path = chat_log_path(app_data_root, session_id);
    let file = match fs::File::open(&path) {
        Ok(file) => file,
        Err(_) => return Ok(Vec::new()),
    };
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(_) => return Ok(Vec::new()),
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let row_scope = value
            .get("resource_scope")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok());
        if row_scope.as_ref() != Some(scope) {
            continue;
        }
        let entry: SessionChatLogEntryV0_1 = serde_json::from_value(value).map_err(|error| {
            format!(
                "parse authorized chat log line {idx} failed (expected {}): {error}",
                SESSION_CHAT_LOG_SCHEMA_VERSION_V0_1
            )
        })?;
        entries.push(entry);
    }
    entries.sort_by(|a, b| {
        (a.turn_index, &a.created_at_utc, &a.message_id).cmp(&(
            b.turn_index,
            &b.created_at_utc,
            &b.message_id,
        ))
    });
    Ok(entries)
}

#[tauri::command]
pub fn session_chat_read(
    state: State<SessionChatLogState>,
    session_id: String,
    limit: Option<u64>,
) -> Result<Vec<SessionChatLogEntryV0_1>, String> {
    let session_id = session_id.trim().to_string();
    let _ = require_uuid_string_non_nil("session_id", &session_id)?;

    let mut entries =
        read_chat_log_for_scope(&state.app_data_root, &session_id, &state.resource_scope)?;

    if let Some(limit) = limit {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let limit = limit as usize;
        if entries.len() > limit {
            entries = entries.split_off(entries.len() - limit);
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_core::swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId,
        WorkspaceScopeRef,
    };

    fn exact_scope(workspace: &str) -> ExactResourceScopeAttribution {
        ExactResourceScopeAttribution {
            owner_account_id: OwnerAccountId::mint(),
            actor_principal_id: ActorPrincipalId::mint(),
            authenticated_session_id: AuthenticatedSessionRef::mint(),
            access_space_id: AccessSpaceRef::mint(),
            workspace_id: WorkspaceScopeRef::new(workspace).expect("workspace"),
        }
    }

    fn valid_row(
        session_id: &str,
        scope: Option<&ExactResourceScopeAttribution>,
        content: &str,
    ) -> Value {
        let mut row = serde_json::json!({
            "schema_version": SESSION_CHAT_LOG_SCHEMA_VERSION_V0_1,
            "session_id": session_id,
            "turn_index": 1,
            "created_at_utc": "2026-08-10T00:00:00Z",
            "message_id": Uuid::now_v7().to_string(),
            "role": "user",
            "content": content,
        });
        if let Some(scope) = scope {
            row.as_object_mut().expect("row object").insert(
                "resource_scope".to_string(),
                serde_json::to_value(scope).expect("scope JSON"),
            );
        }
        row
    }

    #[test]
    fn scoped_chat_reader_hides_malformed_foreign_legacy_and_missing() {
        let root = tempfile::tempdir().expect("app data root");
        let owner = exact_scope("owner-workspace");
        let foreign = exact_scope("foreign-workspace");
        let session_id = Uuid::now_v7().to_string();
        let dir = session_dir(root.path(), &session_id);
        fs::create_dir_all(&dir).expect("session dir");
        let contents = format!(
            "{{broken\n{}\n{}\n",
            valid_row(&session_id, Some(&foreign), "foreign"),
            valid_row(&session_id, None, "legacy")
        );
        fs::write(chat_log_path(root.path(), &session_id), contents).expect("chat log");

        assert!(read_chat_log_for_scope(root.path(), &session_id, &owner)
            .expect("foreign rows hidden")
            .is_empty());
        assert!(
            read_chat_log_for_scope(root.path(), &Uuid::now_v7().to_string(), &owner)
                .expect("missing hidden identically")
                .is_empty()
        );
    }

    #[test]
    fn scoped_chat_reader_ignores_foreign_corruption_but_reports_owner_corruption() {
        let root = tempfile::tempdir().expect("app data root");
        let owner = exact_scope("owner-workspace");
        let foreign = exact_scope("foreign-workspace");
        let session_id = Uuid::now_v7().to_string();
        let dir = session_dir(root.path(), &session_id);
        fs::create_dir_all(&dir).expect("session dir");
        let owner_row = valid_row(&session_id, Some(&owner), "owner");
        let mut foreign_corrupt = serde_json::json!({ "resource_scope": foreign });
        foreign_corrupt["untrusted"] = Value::Bool(true);
        fs::write(
            chat_log_path(root.path(), &session_id),
            format!("{foreign_corrupt}\n{owner_row}\n"),
        )
        .expect("mixed chat log");
        let rows = read_chat_log_for_scope(root.path(), &session_id, &owner)
            .expect("foreign corruption cannot deny owner");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].content, "owner");

        let owner_corrupt = serde_json::json!({ "resource_scope": owner });
        fs::write(
            chat_log_path(root.path(), &session_id),
            format!("{owner_corrupt}\n"),
        )
        .expect("owner corrupt chat log");
        assert!(read_chat_log_for_scope(root.path(), &session_id, &owner)
            .expect_err("matching owner corruption remains observable for recovery")
            .contains("parse authorized chat log line"));
    }
}
