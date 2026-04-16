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

#[cfg(test)]
fn golden_skill_bank_entry() -> SkillBankLogEntry {
    SkillBankLogEntry {
        version: "2.0.0-distillation-v1".to_string(),
        log_id: Uuid::new_v4(),
        timestamp: Utc::now(),
        session: SessionMeta {
            session_id: Uuid::new_v4(),
            turn_index: 42,
            task_id: Some("wp-distillation-task-001".to_string()),
            user_id_hash: Some("userhash-001".to_string()),
            workspace_id: Some("workspace-distill".to_string()),
        },
        task: TaskMeta {
            r#type: "distillation".to_string(),
            subtype: Some("teacher_student".to_string()),
            language: Some("rust".to_string()),
            tags: vec!["distillation".to_string(), "loRA".to_string()],
            request_summary: Some("capture durable teacher/student lineage".to_string()),
        },
        engine: EngineMeta {
            actor_role: ActorRole::Teacher,
            model_name: "gpt-dummy".to_string(),
            model_family: Some("llm-base-v1".to_string()),
            model_revision: Some("rev-1".to_string()),
            provider: Some("local".to_string()),
            tokenizer_id: Some("tok-001".to_string()),
            tokenizer_family: Some("bpe".to_string()),
            context_window_tokens: Some(8192),
            precision: Some("fp16".to_string()),
            inference_params: HashMap::from([
                ("temperature".to_string(), serde_json::json!(0.7)),
                ("top_p".to_string(), serde_json::json!(0.9)),
            ]),
        },
        context_refs: ContextRefs {
            files: vec![FileContextRef {
                path: "src/backend/handshake_core/src/models/skill_bank.rs".to_string(),
                hash: Some("file-hash-001".to_string()),
                selection_ranges: vec![FileSelectionRange {
                    start_line: 1,
                    end_line: 10,
                }],
            }],
            spec_sections: vec!["9.1.1".to_string()],
            requirements: vec!["distillation lineage".to_string()],
            tools_invoked: Vec::new(),
        },
        snapshots_input: ChatSnapshot {
            format: SnapshotFormat::Chatml,
            messages: vec![ChatMessage {
                id: Uuid::new_v4(),
                parent_id: None,
                role: Role::User,
                content: Content::Plain("input".to_string()),
                metadata: HashMap::new(),
            }],
            focus_message_id: None,
        },
        snapshots_output_raw: ChatSnapshot {
            format: SnapshotFormat::Chatml,
            messages: vec![ChatMessage {
                id: Uuid::new_v4(),
                parent_id: None,
                role: Role::Assistant,
                content: Content::Plain("output".to_string()),
                metadata: HashMap::new(),
            }],
            focus_message_id: None,
        },
        snapshots_output_final: None,
        quality: QualityMeta {
            quality_tag: QualityTag::Good,
            thumb: ThumbValue::Up,
            score: Some(0.92),
            source: Some("e2e-fixture".to_string()),
            labels: vec!["reviewed".to_string()],
            auto_eval: AutoEvalMeta {
                tests_passed: 18,
                tests_failed: 0,
                compile_success: Some(true),
                security_flags: vec!["none".to_string()],
                toxicity_scores: HashMap::from([("overall".to_string(), 0.01)]),
                style_score: Some(0.88),
                reasoning_score: Some(0.71),
                factuality_score: Some(0.79),
            },
            user_edit_stats: UserEditStats {
                output_was_edited: false,
                edit_char_fraction: Some(0.0),
                edit_summary: None,
                style_only_edit: Some(false),
            },
            data_trust_score: Some(0.93),
            reward_features: HashMap::from([("entropy".to_string(), 0.2)]),
        },
        telemetry: TelemetryMeta {
            latency_ms: Some(120),
            prompt_tokens: Some(14),
            completion_tokens: Some(34),
            total_tokens: Some(48),
            truncation_occurred: Some(false),
            cache_hit: Some(true),
            output_char_len: Some(1024),
            output_line_count: Some(24),
        },
        environment: EnvironmentMeta {
            handshake_version: Some("v2.0.0".to_string()),
            orchestrator_build: Some("ci-001".to_string()),
            git_commit: Some("placeholder".to_string()),
            os: Some("test".to_string()),
            hardware_profile: Some("cpu".to_string()),
            config_profile: Some("default".to_string()),
        },
        privacy: PrivacyMeta {
            contains_secrets: false,
            pii_present: false,
            can_export_off_device: false,
            redaction_applied: false,
        },
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    /// Build a minimal valid SkillBankLogEntry using plain-string Content
    /// and all default nested metadata fields.
    fn minimal_entry_plain_content() -> SkillBankLogEntry {
        let msg_id = Uuid::new_v4();
        SkillBankLogEntry {
            version: "1.0.0".to_string(),
            log_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            session: SessionMeta {
                session_id: Uuid::new_v4(),
                turn_index: 0,
                task_id: None,
                user_id_hash: None,
                workspace_id: None,
            },
            task: TaskMeta {
                r#type: "code_generation".to_string(),
                subtype: None,
                language: Some("rust".to_string()),
                tags: vec![],
                request_summary: None,
            },
            engine: EngineMeta {
                actor_role: ActorRole::Student,
                model_name: "test-model".to_string(),
                model_family: None,
                model_revision: None,
                provider: None,
                tokenizer_id: None,
                tokenizer_family: None,
                context_window_tokens: None,
                precision: None,
                inference_params: HashMap::new(),
            },
            context_refs: ContextRefs::default(),
            snapshots_input: ChatSnapshot {
                format: SnapshotFormat::Chatml,
                messages: vec![ChatMessage {
                    id: msg_id,
                    parent_id: None,
                    role: Role::User,
                    content: Content::Plain("write a function".to_string()),
                    metadata: HashMap::new(),
                }],
                focus_message_id: Some(msg_id),
            },
            snapshots_output_raw: ChatSnapshot {
                format: SnapshotFormat::Chatml,
                messages: vec![ChatMessage {
                    id: Uuid::new_v4(),
                    parent_id: Some(msg_id),
                    role: Role::Assistant,
                    content: Content::Plain("fn hello() {}".to_string()),
                    metadata: HashMap::new(),
                }],
                focus_message_id: None,
            },
            snapshots_output_final: None,
            quality: QualityMeta {
                quality_tag: QualityTag::Unrated,
                thumb: ThumbValue::None,
                score: None,
                source: None,
                labels: vec![],
                auto_eval: AutoEvalMeta::default(),
                user_edit_stats: UserEditStats::default(),
                data_trust_score: None,
                reward_features: HashMap::new(),
            },
            telemetry: TelemetryMeta::default(),
            environment: EnvironmentMeta::default(),
            privacy: PrivacyMeta::default(),
        }
    }

    #[test]
    fn round_trip_plain_content_with_defaults() {
        let entry = minimal_entry_plain_content();
        let json = serde_json::to_string_pretty(&entry).expect("serialize");
        let back: SkillBankLogEntry = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back.version, entry.version);
        assert_eq!(back.log_id, entry.log_id);
        assert_eq!(back.session.session_id, entry.session.session_id);
        assert_eq!(back.quality.auto_eval.tests_passed, 0);
        assert!(!back.privacy.pii_present);
        // Plain content survives the round trip
        match &back.snapshots_input.messages[0].content {
            Content::Plain(s) => assert_eq!(s, "write a function"),
            Content::Segments(_) => panic!("expected plain content"),
        }
    }

    #[test]
    fn round_trip_segmented_content() {
        let mut entry = minimal_entry_plain_content();
        // Replace input with segmented content
        entry.snapshots_input.messages[0].content = Content::Segments(vec![
            ContentSegment {
                r#type: ContentSegmentType::Text,
                text: "Please fix this:".to_string(),
                language: None,
                file_path: None,
            },
            ContentSegment {
                r#type: ContentSegmentType::Code,
                text: "fn broken() { }".to_string(),
                language: Some("rust".to_string()),
                file_path: Some("src/lib.rs".to_string()),
            },
        ]);
        // Also populate some non-default nested metadata
        entry.quality.auto_eval.tests_passed = 3;
        entry.quality.auto_eval.compile_success = Some(true);
        entry.quality.score = Some(0.85);
        entry.telemetry.prompt_tokens = Some(150);
        entry.telemetry.completion_tokens = Some(42);
        entry.environment.handshake_version = Some("0.1.0".to_string());
        entry.privacy.pii_present = true;
        entry.privacy.redaction_applied = true;

        let json = serde_json::to_string_pretty(&entry).expect("serialize");
        let back: SkillBankLogEntry = serde_json::from_str(&json).expect("deserialize");

        // Segmented content round trips
        match &back.snapshots_input.messages[0].content {
            Content::Segments(segs) => {
                assert_eq!(segs.len(), 2);
                assert_eq!(segs[0].r#type, ContentSegmentType::Text);
                assert_eq!(segs[1].language.as_deref(), Some("rust"));
                assert_eq!(segs[1].file_path.as_deref(), Some("src/lib.rs"));
            }
            Content::Plain(_) => panic!("expected segmented content"),
        }
        // Non-default metadata fields survive
        assert_eq!(back.quality.auto_eval.tests_passed, 3);
        assert_eq!(back.quality.auto_eval.compile_success, Some(true));
        assert_eq!(back.quality.score, Some(0.85));
        assert_eq!(back.telemetry.prompt_tokens, Some(150));
        assert_eq!(back.environment.handshake_version.as_deref(), Some("0.1.0"));
        assert!(back.privacy.pii_present);
        assert!(back.privacy.redaction_applied);
    }

    #[test]
    fn round_trip_golden_distillation_entry() {
        let entry = golden_skill_bank_entry();
        let json = serde_json::to_string_pretty(&entry).expect("serialize");
        let back = serde_json::from_str::<SkillBankLogEntry>(&json).expect("deserialize");

        assert_eq!(back.version, entry.version);
        assert_eq!(back.quality.data_trust_score, entry.quality.data_trust_score);
        assert!(back.quality.auto_eval.compile_success.unwrap_or_default());
        assert_eq!(back.engine.actor_role, ActorRole::Teacher);
        assert_eq!(back.context_refs.spec_sections.first().map(|value| value.as_str()), Some("9.1.1"));
        assert_eq!(back.quality.labels.first().map(|value| value.as_str()), Some("reviewed"));
    }

    #[test]
    fn deserialize_with_missing_optional_fields() {
        // Minimal JSON with only required fields — all defaulted nested
        // metadata should fill in from Default impls.
        let msg_id = Uuid::new_v4();
        let log_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let ts = Utc::now();
        let json = serde_json::json!({
            "version": "1.0.0",
            "log_id": log_id,
            "timestamp": ts,
            "session": {
                "session_id": session_id,
                "turn_index": 1
            },
            "task": { "type": "debug" },
            "engine": {
                "actor_role": "teacher",
                "model_name": "gpt-4"
            },
            "context_refs": {},
            "snapshots_input": {
                "format": "openai_chat",
                "messages": [{
                    "id": msg_id,
                    "role": "user",
                    "content": "help"
                }]
            },
            "snapshots_output_raw": {
                "format": "openai_chat",
                "messages": []
            },
            "quality": {
                "quality_tag": "good",
                "auto_eval": {},
                "user_edit_stats": {}
            },
            "telemetry": {},
            "environment": {},
            "privacy": {}
        });

        let entry: SkillBankLogEntry =
            serde_json::from_value(json).expect("deserialize minimal JSON");
        assert_eq!(entry.log_id, log_id);
        assert_eq!(entry.engine.actor_role, ActorRole::Teacher);
        assert_eq!(entry.quality.quality_tag, QualityTag::Good);
        assert_eq!(entry.quality.thumb, ThumbValue::None);
        assert_eq!(entry.quality.auto_eval.tests_passed, 0);
        assert!(!entry.privacy.contains_secrets);
        assert!(entry.telemetry.latency_ms.is_none());
    }
}
