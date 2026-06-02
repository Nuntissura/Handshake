//! Conservative local/cloud output comparison traces for distillation review.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::models::skill_bank::{
    ActorRole, AutoEvalMeta, ChatMessage, ChatSnapshot, Content, ContextRefs, EngineMeta,
    EnvironmentMeta, PrivacyMeta, QualityMeta, QualityTag, Role, SessionMeta, SkillBankLogEntry,
    SnapshotFormat, TaskMeta, TelemetryMeta, ThumbValue, UserEditStats,
};

use super::redaction::redact_entry;

pub const DISTILLATION_SWARM_TRACE_VERSION: &str = "distillation_swarm_trace_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmTraceOutputSource {
    Local,
    Cloud,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmTraceOutput {
    pub source: SwarmTraceOutputSource,
    pub label: String,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmTraceEligibility {
    pub distillation_allowed: bool,
    pub local_output_allowed: bool,
    pub cloud_output_allowed: bool,
}

impl SwarmTraceEligibility {
    pub fn eligible() -> Self {
        Self {
            distillation_allowed: true,
            local_output_allowed: true,
            cloud_output_allowed: true,
        }
    }

    fn ineligible_reasons(&self) -> Vec<SwarmTraceIneligibleReason> {
        let mut reasons = Vec::new();
        if !self.distillation_allowed {
            reasons.push(SwarmTraceIneligibleReason::DistillationNotAllowed);
        }
        if !self.local_output_allowed {
            reasons.push(SwarmTraceIneligibleReason::LocalOutputNotAllowed);
        }
        if !self.cloud_output_allowed {
            reasons.push(SwarmTraceIneligibleReason::CloudOutputNotAllowed);
        }
        reasons
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmTraceIneligibleReason {
    DistillationNotAllowed,
    LocalOutputNotAllowed,
    CloudOutputNotAllowed,
    EmptyPrompt,
    EmptyLocalLabel,
    EmptyCloudLabel,
    EmptyLocalOutput,
    EmptyCloudOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmTraceCandidate {
    pub trace_id: Uuid,
    pub sample_id: String,
    pub prompt: String,
    pub local_label: String,
    pub local_output: String,
    pub cloud_label: String,
    pub cloud_output: String,
    pub comparison_labels: Vec<String>,
    pub eligibility: SwarmTraceEligibility,
}

impl SwarmTraceCandidate {
    pub fn new(
        trace_id: Uuid,
        sample_id: impl Into<String>,
        prompt: impl Into<String>,
        local_label: impl Into<String>,
        local_output: impl Into<String>,
        cloud_label: impl Into<String>,
        cloud_output: impl Into<String>,
    ) -> Self {
        Self {
            trace_id,
            sample_id: sample_id.into(),
            prompt: prompt.into(),
            local_label: local_label.into(),
            local_output: local_output.into(),
            cloud_label: cloud_label.into(),
            cloud_output: cloud_output.into(),
            comparison_labels: vec!["local_cloud_output_comparison".to_string()],
            eligibility: SwarmTraceEligibility::eligible(),
        }
    }

    pub fn with_comparison_labels(mut self, labels: Vec<String>) -> Self {
        self.comparison_labels = labels;
        self
    }

    pub fn with_eligibility(mut self, eligibility: SwarmTraceEligibility) -> Self {
        self.eligibility = eligibility;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmTraceRedaction {
    pub field: String,
    pub secrets_found: bool,
    pub pii_found: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistillationSwarmTraceBundle {
    pub version: String,
    pub trace_id: Uuid,
    pub sample_id: String,
    pub prompt: String,
    pub outputs: Vec<SwarmTraceOutput>,
    pub comparison_labels: Vec<String>,
    pub eligibility: SwarmTraceEligibility,
    pub redactions_applied: Vec<SwarmTraceRedaction>,
    pub captured_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SwarmTraceError {
    #[error("distillation swarm trace is ineligible: {reasons:?}")]
    Ineligible {
        reasons: Vec<SwarmTraceIneligibleReason>,
    },
}

pub fn capture_distillation_swarm_trace(
    candidate: SwarmTraceCandidate,
) -> Result<DistillationSwarmTraceBundle, SwarmTraceError> {
    capture_distillation_swarm_trace_at(candidate, Utc::now())
}

pub fn capture_distillation_swarm_trace_at(
    candidate: SwarmTraceCandidate,
    captured_at_utc: DateTime<Utc>,
) -> Result<DistillationSwarmTraceBundle, SwarmTraceError> {
    let reasons = validate_candidate(&candidate);
    if !reasons.is_empty() {
        return Err(SwarmTraceError::Ineligible { reasons });
    }

    let mut redactions_applied = Vec::new();
    let prompt = redact_trace_field("prompt", &candidate.prompt, &mut redactions_applied);
    let local_output = redact_trace_field(
        "outputs.local.output",
        &candidate.local_output,
        &mut redactions_applied,
    );
    let cloud_output = redact_trace_field(
        "outputs.cloud.output",
        &candidate.cloud_output,
        &mut redactions_applied,
    );

    Ok(DistillationSwarmTraceBundle {
        version: DISTILLATION_SWARM_TRACE_VERSION.to_string(),
        trace_id: candidate.trace_id,
        sample_id: candidate.sample_id,
        prompt,
        outputs: vec![
            SwarmTraceOutput {
                source: SwarmTraceOutputSource::Local,
                label: candidate.local_label.trim().to_string(),
                output: local_output,
            },
            SwarmTraceOutput {
                source: SwarmTraceOutputSource::Cloud,
                label: candidate.cloud_label.trim().to_string(),
                output: cloud_output,
            },
        ],
        comparison_labels: candidate.comparison_labels,
        eligibility: candidate.eligibility,
        redactions_applied,
        captured_at_utc,
    })
}

fn validate_candidate(candidate: &SwarmTraceCandidate) -> Vec<SwarmTraceIneligibleReason> {
    let mut reasons = candidate.eligibility.ineligible_reasons();
    if candidate.prompt.trim().is_empty() {
        reasons.push(SwarmTraceIneligibleReason::EmptyPrompt);
    }
    if candidate.local_label.trim().is_empty() {
        reasons.push(SwarmTraceIneligibleReason::EmptyLocalLabel);
    }
    if candidate.cloud_label.trim().is_empty() {
        reasons.push(SwarmTraceIneligibleReason::EmptyCloudLabel);
    }
    if candidate.local_output.trim().is_empty() {
        reasons.push(SwarmTraceIneligibleReason::EmptyLocalOutput);
    }
    if candidate.cloud_output.trim().is_empty() {
        reasons.push(SwarmTraceIneligibleReason::EmptyCloudOutput);
    }
    reasons
}

fn redact_trace_field(
    field: &str,
    text: &str,
    redactions_applied: &mut Vec<SwarmTraceRedaction>,
) -> String {
    // Route trace strings through the existing Skill Bank redactor so trace
    // capture stays aligned with distillation privacy behavior.
    let redaction = redact_entry(&skill_bank_entry_with_input(text));
    if redaction.secrets_found || redaction.pii_found {
        redactions_applied.push(SwarmTraceRedaction {
            field: field.to_string(),
            secrets_found: redaction.secrets_found,
            pii_found: redaction.pii_found,
        });
    }

    match &redaction.redacted_entry.snapshots_input.messages[0].content {
        Content::Plain(text) => text.clone(),
        Content::Segments(_) => String::new(),
    }
}

fn skill_bank_entry_with_input(text: &str) -> SkillBankLogEntry {
    let msg_id = Uuid::now_v7();
    SkillBankLogEntry {
        version: "1.0.0".to_string(),
        log_id: Uuid::now_v7(),
        timestamp: Utc::now(),
        session: SessionMeta {
            session_id: Uuid::now_v7(),
            turn_index: 0,
            task_id: None,
            user_id_hash: None,
            workspace_id: None,
        },
        task: TaskMeta {
            r#type: "distillation_swarm_trace".to_string(),
            subtype: None,
            language: None,
            tags: vec![],
            request_summary: None,
        },
        engine: EngineMeta {
            actor_role: ActorRole::Teacher,
            model_name: "distillation-swarm-trace-redaction".to_string(),
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
            format: SnapshotFormat::RawText,
            messages: vec![ChatMessage {
                id: msg_id,
                parent_id: None,
                role: Role::User,
                content: Content::Plain(text.to_string()),
                metadata: HashMap::new(),
            }],
            focus_message_id: Some(msg_id),
        },
        snapshots_output_raw: ChatSnapshot {
            format: SnapshotFormat::RawText,
            messages: vec![],
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

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate() -> SwarmTraceCandidate {
        SwarmTraceCandidate::new(
            Uuid::now_v7(),
            "sample-1",
            "Explain the retry path.",
            "local:student",
            "Use exponential backoff and return typed errors.",
            "cloud:teacher",
            "Use exponential backoff, jitter, and return typed errors.",
        )
        .with_comparison_labels(vec![
            "retry-policy".to_string(),
            "error-contract".to_string(),
        ])
    }

    #[test]
    fn trace_captures_local_and_cloud_outputs_with_labels() {
        let captured_at_utc = DateTime::parse_from_rfc3339("2026-06-02T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let bundle = capture_distillation_swarm_trace_at(candidate(), captured_at_utc).unwrap();

        assert_eq!(bundle.version, DISTILLATION_SWARM_TRACE_VERSION);
        assert_eq!(bundle.sample_id, "sample-1");
        assert_eq!(bundle.outputs.len(), 2);
        assert_eq!(bundle.outputs[0].source, SwarmTraceOutputSource::Local);
        assert_eq!(bundle.outputs[0].label, "local:student");
        assert_eq!(
            bundle.outputs[0].output,
            "Use exponential backoff and return typed errors."
        );
        assert_eq!(bundle.outputs[1].source, SwarmTraceOutputSource::Cloud);
        assert_eq!(bundle.outputs[1].label, "cloud:teacher");
        assert_eq!(
            bundle.comparison_labels,
            vec!["retry-policy".to_string(), "error-contract".to_string()]
        );
        assert!(bundle.redactions_applied.is_empty());
        assert_eq!(bundle.captured_at_utc, captured_at_utc);
    }

    #[test]
    fn trace_redacts_sensitive_strings_before_capture() {
        let mut raw = candidate();
        raw.prompt = "Use OPENAI_API_KEY=sk-supersecret123 for user@example.com".to_string();
        raw.local_output = "Local saw Bearer sk-local-token and 555-123-4567".to_string();
        raw.cloud_output = "Cloud saw api_key=sk-cloud-token".to_string();

        let bundle = capture_distillation_swarm_trace_at(raw, Utc::now()).unwrap();

        assert!(!bundle.prompt.contains("sk-supersecret123"));
        assert!(!bundle.prompt.contains("user@example.com"));
        assert!(!bundle.outputs[0].output.contains("sk-local-token"));
        assert!(!bundle.outputs[0].output.contains("555-123-4567"));
        assert!(!bundle.outputs[1].output.contains("sk-cloud-token"));
        assert!(bundle.prompt.contains("[REDACTED_ENV]"));
        assert!(bundle.prompt.contains("[REDACTED_EMAIL]"));
        assert_eq!(bundle.redactions_applied.len(), 3);
        assert!(bundle
            .redactions_applied
            .iter()
            .any(|r| r.field == "prompt" && r.secrets_found && r.pii_found));
        assert!(bundle
            .redactions_applied
            .iter()
            .any(|r| r.field == "outputs.local.output" && r.secrets_found && r.pii_found));
        assert!(bundle
            .redactions_applied
            .iter()
            .any(|r| r.field == "outputs.cloud.output" && r.secrets_found));
    }

    #[test]
    fn trace_refuses_ineligible_candidates() {
        let raw = candidate().with_eligibility(SwarmTraceEligibility {
            distillation_allowed: false,
            local_output_allowed: true,
            cloud_output_allowed: true,
        });

        let err = capture_distillation_swarm_trace_at(raw, Utc::now()).unwrap_err();

        assert_eq!(
            err,
            SwarmTraceError::Ineligible {
                reasons: vec![SwarmTraceIneligibleReason::DistillationNotAllowed]
            }
        );
    }
}
