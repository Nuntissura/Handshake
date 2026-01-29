use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use tauri::State;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

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
    pub fn new(app_data_root: PathBuf) -> Self {
        let session_id = Uuid::new_v4().to_string();
        Self {
            session_id,
            next_turn_index: Mutex::new(0),
            app_data_root,
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
        None => Uuid::new_v4().to_string(),
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

#[tauri::command]
pub fn session_chat_read(
    state: State<SessionChatLogState>,
    session_id: String,
    limit: Option<u64>,
) -> Result<Vec<SessionChatLogEntryV0_1>, String> {
    let session_id = session_id.trim().to_string();
    let _ = require_uuid_string_non_nil("session_id", &session_id)?;

    let path = chat_log_path(&state.app_data_root, &session_id);
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
