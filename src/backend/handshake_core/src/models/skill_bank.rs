//! Skill Bank data model types (Master Spec Section 9.1.1).
//!
//! Defines the in-orchestrator representations that map to the Skill Bank
//! storage and distillation pipeline. All types derive `Serialize`/`Deserialize`
//! for JSON persistence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Enums (spec Literal types)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
    Router,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentSegmentType {
    Text,
    Code,
    Diff,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotFormat {
    Chatml,
    OpenaiChat,
    RawText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityTag {
    Good,
    Bad,
    NeedsEdit,
    Unrated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThumbValue {
    Up,
    Down,
    Neutral,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorRole {
    Student,
    Teacher,
    Tool,
    Router,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolInvocationStatus {
    Success,
    Error,
    Timeout,
    Skipped,
}

// ---------------------------------------------------------------------------
// Chat and content structures (spec 1.1.1)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSegment {
    pub r#type: ContentSegmentType,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Plain(String),
    Segments(Vec<ContentSegment>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    pub role: Role,
    pub content: Content,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSnapshot {
    pub format: SnapshotFormat,
    pub messages: Vec<ChatMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus_message_id: Option<Uuid>,
}

// ---------------------------------------------------------------------------
// Skill Bank entry and metadata (spec 1.1.2)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub session_id: Uuid,
    pub turn_index: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMeta {
    pub r#type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineMeta {
    pub actor_role: ActorRole,
    pub model_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokenizer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokenizer_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window_tokens: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precision: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub inference_params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSelectionRange {
    pub start_line: i64,
    pub end_line: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContextRef {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selection_ranges: Vec<FileSelectionRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocationRef {
    pub invocation_id: Uuid,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    pub status: ToolInvocationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncated_output: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRefs {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FileContextRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spec_sections: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools_invoked: Vec<ToolInvocationRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoEvalMeta {
    #[serde(default)]
    pub tests_passed: i32,
    #[serde(default)]
    pub tests_failed: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compile_success: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security_flags: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub toxicity_scores: HashMap<String, f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub factuality_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEditStats {
    #[serde(default)]
    pub output_was_edited: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit_char_fraction: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_only_edit: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMeta {
    pub quality_tag: QualityTag,
    #[serde(default = "default_thumb_none")]
    pub thumb: ThumbValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(default)]
    pub auto_eval: AutoEvalMeta,
    #[serde(default)]
    pub user_edit_stats: UserEditStats,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_trust_score: Option<f64>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub reward_features: HashMap<String, f64>,
}

fn default_thumb_none() -> ThumbValue {
    ThumbValue::None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncation_occurred: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_hit: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_char_len: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_line_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handshake_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orchestrator_build: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyMeta {
    #[serde(default)]
    pub contains_secrets: bool,
    #[serde(default)]
    pub pii_present: bool,
    #[serde(default)]
    pub can_export_off_device: bool,
    #[serde(default)]
    pub redaction_applied: bool,
}

// ---------------------------------------------------------------------------
// Top-level Skill Bank log entry (spec 1.1.2)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBankLogEntry {
    pub version: String,
    pub log_id: Uuid,
    pub timestamp: DateTime<Utc>,

    pub session: SessionMeta,
    pub task: TaskMeta,
    pub engine: EngineMeta,
    pub context_refs: ContextRefs,

    pub snapshots_input: ChatSnapshot,
    pub snapshots_output_raw: ChatSnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshots_output_final: Option<ChatSnapshot>,

    pub quality: QualityMeta,
    pub telemetry: TelemetryMeta,
    pub environment: EnvironmentMeta,
    pub privacy: PrivacyMeta,
}

// ---------------------------------------------------------------------------
// Default impls for nested metadata types used with #[serde(default)]
// ---------------------------------------------------------------------------

impl Default for AutoEvalMeta {
    fn default() -> Self {
        Self {
            tests_passed: 0,
            tests_failed: 0,
            compile_success: None,
            security_flags: Vec::new(),
            toxicity_scores: HashMap::new(),
            style_score: None,
            reasoning_score: None,
            factuality_score: None,
        }
    }
}

impl Default for UserEditStats {
    fn default() -> Self {
        Self {
            output_was_edited: false,
            edit_char_fraction: None,
            edit_summary: None,
            style_only_edit: None,
        }
    }
}

impl Default for TelemetryMeta {
    fn default() -> Self {
        Self {
            latency_ms: None,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            truncation_occurred: None,
            cache_hit: None,
            output_char_len: None,
            output_line_count: None,
        }
    }
}

impl Default for EnvironmentMeta {
    fn default() -> Self {
        Self {
            handshake_version: None,
            orchestrator_build: None,
            git_commit: None,
            os: None,
            hardware_profile: None,
            config_profile: None,
        }
    }
}

impl Default for PrivacyMeta {
    fn default() -> Self {
        Self {
            contains_secrets: false,
            pii_present: false,
            can_export_off_device: false,
            redaction_applied: false,
        }
    }
}

impl Default for ContextRefs {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            spec_sections: Vec::new(),
            requirements: Vec::new(),
            tools_invoked: Vec::new(),
        }
    }
}
