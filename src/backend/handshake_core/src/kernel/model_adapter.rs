use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[cfg(feature = "runtime-full")]
use crate::mcp::gate::{KernelMcpGateDecisionKind, KernelMcpToolGateDecision};

use super::context_bundle::{ContextBundle, canonical_json_bytes, sha256_hex};
use super::{KernelActor, KernelError, KernelEventType, KernelResult};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ModelAdapterRequest {
    pub context_bundle: ContextBundle,
    pub actor: KernelActor,
}

impl ModelAdapterRequest {
    pub fn new(context_bundle: ContextBundle, actor: KernelActor) -> Self {
        Self {
            context_bundle,
            actor,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KernelToolRequest {
    pub tool_request_id: String,
    pub event_type: KernelEventType,
    pub tool_id: String,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToolDecisionKind {
    Allow,
    Deny,
}

impl ToolDecisionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolDecisionRecord {
    pub gate_id: String,
    pub tool_decision_id: String,
    pub tool_request_id: String,
    pub tool_id: String,
    pub canonical_tool_id: String,
    pub decision: ToolDecisionKind,
    pub reason: String,
    pub policy_source: String,
    pub args_ref: String,
    pub args_hash: String,
    pub result_ref: String,
    pub result_hash: String,
    pub event_type: KernelEventType,
}

#[cfg(feature = "runtime-full")]
impl From<KernelMcpGateDecisionKind> for ToolDecisionKind {
    fn from(value: KernelMcpGateDecisionKind) -> Self {
        match value {
            KernelMcpGateDecisionKind::Allow => Self::Allow,
            KernelMcpGateDecisionKind::Deny => Self::Deny,
        }
    }
}

#[cfg(feature = "runtime-full")]
impl ToolDecisionRecord {
    pub fn from_mcp_gate_decision(
        decision: KernelMcpToolGateDecision,
        event_type: KernelEventType,
    ) -> Self {
        Self {
            gate_id: decision.gate_id,
            tool_decision_id: decision.tool_decision_id,
            tool_request_id: decision.tool_request_id,
            tool_id: decision.tool_id,
            canonical_tool_id: decision.canonical_tool_id,
            decision: decision.decision.into(),
            reason: decision.reason,
            policy_source: decision.policy_source,
            args_ref: decision.args_ref,
            args_hash: decision.args_hash,
            result_ref: decision.result_ref,
            result_hash: decision.result_hash,
            event_type,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactProposalDraft {
    pub artifact_proposal_id: String,
    pub event_type: KernelEventType,
    pub artifact_kind: String,
    pub content_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelAdapterOutput {
    pub adapter_id: String,
    pub context_bundle_id: String,
    pub response_text: String,
    pub response_event_type: KernelEventType,
    pub tool_request: KernelToolRequest,
    pub artifact_proposal: ArtifactProposalDraft,
    pub artifact_payload: serde_json::Value,
    pub output_hash: String,
}

#[async_trait]
pub trait ModelAdapter: Send + Sync {
    fn adapter_id(&self) -> &str;
    async fn invoke(&self, request: ModelAdapterRequest) -> KernelResult<ModelAdapterOutput>;
}

#[derive(Clone, Debug)]
pub struct DummyEchoModelAdapter {
    adapter_id: String,
}

impl DummyEchoModelAdapter {
    pub fn new(adapter_id: impl Into<String>) -> Self {
        Self {
            adapter_id: adapter_id.into(),
        }
    }
}

#[async_trait]
impl ModelAdapter for DummyEchoModelAdapter {
    fn adapter_id(&self) -> &str {
        &self.adapter_id
    }

    async fn invoke(&self, request: ModelAdapterRequest) -> KernelResult<ModelAdapterOutput> {
        if self.adapter_id.trim().is_empty() {
            return Err(KernelError::InvalidEvent("adapter_id is required"));
        }
        let context_bundle = request.context_bundle;
        let response_text = format!(
            "dummy-echo:{}:{}",
            self.adapter_id, context_bundle.context_hash
        );
        let artifact_payload = json!({
            "adapter_id": self.adapter_id,
            "context_bundle_id": context_bundle.context_bundle_id,
            "context_hash": context_bundle.context_hash,
            "response_text": response_text,
        });
        let output_hash = sha256_hex(&canonical_json_bytes(&artifact_payload));
        let tool_request = KernelToolRequest {
            tool_request_id: format!("TOOLREQ-{}", &output_hash[..16]),
            event_type: KernelEventType::ToolRequestRecorded,
            tool_id: "read_trace".to_string(),
            reason: "deterministic dummy proof request".to_string(),
        };
        let artifact_proposal = ArtifactProposalDraft {
            artifact_proposal_id: format!("AP-{}", &output_hash[16..32]),
            event_type: KernelEventType::ArtifactProposed,
            artifact_kind: "dummy_adapter_output".to_string(),
            content_hash: output_hash.clone(),
        };

        Ok(ModelAdapterOutput {
            adapter_id: self.adapter_id.clone(),
            context_bundle_id: context_bundle.context_bundle_id,
            response_text,
            response_event_type: KernelEventType::ModelResponseRecorded,
            tool_request,
            artifact_proposal,
            artifact_payload,
            output_hash,
        })
    }
}

#[derive(Clone, Debug)]
pub struct StructuredSummaryModelAdapter {
    adapter_id: String,
}

impl StructuredSummaryModelAdapter {
    pub fn new(adapter_id: impl Into<String>) -> Self {
        Self {
            adapter_id: adapter_id.into(),
        }
    }
}

#[async_trait]
impl ModelAdapter for StructuredSummaryModelAdapter {
    fn adapter_id(&self) -> &str {
        &self.adapter_id
    }

    async fn invoke(&self, request: ModelAdapterRequest) -> KernelResult<ModelAdapterOutput> {
        if self.adapter_id.trim().is_empty() {
            return Err(KernelError::InvalidEvent("adapter_id is required"));
        }
        let context_bundle = request.context_bundle;
        let response_text = format!(
            "structured-summary:{}:{}",
            self.adapter_id, context_bundle.context_bundle_id
        );
        let artifact_payload = json!({
            "adapter_id": self.adapter_id,
            "context_bundle_id": context_bundle.context_bundle_id,
            "context_hash": context_bundle.context_hash,
            "summary": {
                "event_contract": "kernel_v1_first_slice",
                "response_text": response_text
            }
        });
        let output_hash = sha256_hex(&canonical_json_bytes(&artifact_payload));
        let tool_request = KernelToolRequest {
            tool_request_id: format!("TOOLREQ-{}", &output_hash[..16]),
            event_type: KernelEventType::ToolRequestRecorded,
            tool_id: "read_trace".to_string(),
            reason: "deterministic structured proof request".to_string(),
        };
        let artifact_proposal = ArtifactProposalDraft {
            artifact_proposal_id: format!("AP-{}", &output_hash[16..32]),
            event_type: KernelEventType::ArtifactProposed,
            artifact_kind: "structured_summary_adapter_output".to_string(),
            content_hash: output_hash.clone(),
        };

        Ok(ModelAdapterOutput {
            adapter_id: self.adapter_id.clone(),
            context_bundle_id: context_bundle.context_bundle_id,
            response_text,
            response_event_type: KernelEventType::ModelResponseRecorded,
            tool_request,
            artifact_proposal,
            artifact_payload,
            output_hash,
        })
    }
}
