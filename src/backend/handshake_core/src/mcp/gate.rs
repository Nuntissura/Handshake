use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::capabilities::CapabilityRegistry;
use crate::flight_recorder::FlightRecorder;
use crate::storage::AccessMode;

use super::client::{JsonRpcMcpClient, McpDispatcher, PendingMeta};
use super::discovery::{
    McpResourceDescriptor, McpToolDescriptor, ResourcesListResult, ToolsListResult,
};
use super::errors::{McpError, McpResult};
use super::fr_events;
use super::jsonrpc::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use super::schema;
use super::security;
use super::transport::{AutoReconnectTransport, McpTransport, ReconnectConfig};

#[derive(Clone, Debug)]
pub struct McpContext {
    pub job_id: Option<Uuid>,
    pub trace_id: Uuid,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
    pub workflow_run_id: Option<String>,
    pub granted_capabilities: Vec<String>,
    pub access_mode: AccessMode,
    pub human_consent_obtained: bool,
    pub agentic_mode_enabled: bool,
    pub allowed_roots: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsentDecision {
    Allow,
    Deny,
    Timeout,
}

#[async_trait]
pub trait ConsentProvider: Send + Sync {
    async fn request_consent(
        &self,
        ctx: &McpContext,
        server_id: &str,
        tool_name: &str,
        capability_id: Option<&str>,
    ) -> ConsentDecision;
}

pub struct StaticConsentProvider {
    decision: ConsentDecision,
}

impl StaticConsentProvider {
    pub fn new(decision: ConsentDecision) -> Self {
        Self { decision }
    }
}

#[async_trait]
impl ConsentProvider for StaticConsentProvider {
    async fn request_consent(
        &self,
        _ctx: &McpContext,
        _server_id: &str,
        _tool_name: &str,
        _capability_id: Option<&str>,
    ) -> ConsentDecision {
        self.decision
    }
}

#[derive(Clone, Debug)]
pub struct ToolPolicy {
    pub required_capability: Option<String>,
    pub requires_consent: bool,
    pub path_argument: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GateConfig {
    pub allowed_tools: Option<HashSet<String>>,
    pub tool_policies: HashMap<String, ToolPolicy>,
    pub request_timeout: Duration,
    pub consent_timeout: Duration,
    pub reconnect: ReconnectConfig,
}

impl GateConfig {
    pub fn minimal() -> Self {
        Self {
            allowed_tools: None,
            tool_policies: HashMap::new(),
            request_timeout: Duration::from_secs(60),
            consent_timeout: Duration::from_secs(30),
            reconnect: ReconnectConfig::default(),
        }
    }
}

struct GateDispatcher {
    server_id: String,
    flight_recorder: Arc<dyn FlightRecorder>,
    agentic_mode_enabled: bool,
}

#[async_trait]
impl McpDispatcher for GateDispatcher {
    async fn handle_notification(&self, notification: JsonRpcNotification) {
        if notification.method == "logging/message" {
            if let Some(params) = &notification.params {
                let _ = fr_events::record_logging_message(
                    Arc::clone(&self.flight_recorder),
                    &self.server_id,
                    params,
                );
            }
        }
    }

    async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "ping" => JsonRpcResponse::ok(request.id, json!({})),
            "sampling/createMessage" => {
                if !self.agentic_mode_enabled {
                    JsonRpcResponse::err(
                        request.id,
                        -32601,
                        "sampling/createMessage disabled (agentic_mode=false)",
                        None,
                    )
                } else {
                    JsonRpcResponse::err(
                        request.id,
                        -32601,
                        "sampling/createMessage not implemented in MVP",
                        None,
                    )
                }
            }
            _ => JsonRpcResponse::err(request.id, -32601, "method not allowed", None),
        }
    }

    async fn handle_response(&self, meta: Option<PendingMeta>, response: &JsonRpcResponse) {
        let Some(meta) = meta else {
            return;
        };
        if meta.method != "tools/call" {
            return;
        }
        let (Some(ctx), Some(tool_name)) = (meta.ctx, meta.tool_name) else {
            return;
        };
        let duration_ms = meta.started_at.elapsed().as_millis();
        let (status, error_code) = if response.error.is_some() {
            ("error", Some("jsonrpc"))
        } else {
            ("success", None)
        };
        let result_payload = response.result.clone().unwrap_or(Value::Null);
        let _ = fr_events::record_tool_result(
            Arc::clone(&self.flight_recorder),
            &ctx,
            &self.server_id,
            &tool_name,
            meta.capability_id.as_deref(),
            status,
            Some(duration_ms),
            error_code,
            &result_payload,
        );
    }
}

pub struct GatedMcpClient {
    server_id: String,
    flight_recorder: Arc<dyn FlightRecorder>,
    capability_registry: Arc<CapabilityRegistry>,
    consent_provider: Arc<dyn ConsentProvider>,
    gate: GateConfig,
    inner: JsonRpcMcpClient,
    tools: Arc<Mutex<HashMap<String, McpToolDescriptor>>>,
}

impl GatedMcpClient {
    pub async fn connect<T: McpTransport + 'static>(
        server_id: impl Into<String>,
        transport: T,
        flight_recorder: Arc<dyn FlightRecorder>,
        capability_registry: Arc<CapabilityRegistry>,
        consent_provider: Arc<dyn ConsentProvider>,
        gate: GateConfig,
        agentic_mode_enabled: bool,
    ) -> McpResult<Self> {
        let server_id = server_id.into();
        let mut transport = AutoReconnectTransport::new(transport, gate.reconnect.clone());
        let dispatcher: Arc<dyn McpDispatcher> = Arc::new(GateDispatcher {
            server_id: server_id.clone(),
            flight_recorder: Arc::clone(&flight_recorder),
            agentic_mode_enabled,
        });
        let inner = JsonRpcMcpClient::connect(&mut transport, dispatcher).await?;

        Ok(Self {
            server_id,
            flight_recorder,
            capability_registry,
            consent_provider,
            gate,
            inner,
            tools: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn refresh_tools(&self) -> McpResult<Vec<McpToolDescriptor>> {
        let value = self
            .inner
            .send_request(
                "tools/list",
                Some(json!({})),
                Some(PendingMeta {
                    started_at: Instant::now(), // WAIVER [CX-573E] duration/timeout bookkeeping only
                    method: "tools/list".to_string(),
                    ctx: None,
                    tool_name: None,
                    capability_id: None,
                }),
            )?
            .await?;
        let result: ToolsListResult = serde_json::from_value(value)?;
        let mut guard = self
            .tools
            .lock()
            .map_err(|_| McpError::Transport("tools cache lock error".to_string()))?;
        guard.clear();
        for tool in &result.tools {
            guard.insert(tool.name.clone(), tool.clone());
        }
        Ok(result.tools)
    }

    pub async fn resources_list(&self) -> McpResult<Vec<McpResourceDescriptor>> {
        let value = self
            .inner
            .send_request("resources/list", Some(json!({})), None)?
            .await?;
        let result: ResourcesListResult = serde_json::from_value(value)?;
        Ok(result.resources)
    }

    fn tool_descriptor(&self, tool_name: &str) -> McpResult<McpToolDescriptor> {
        let guard = self
            .tools
            .lock()
            .map_err(|_| McpError::Transport("tools cache lock error".to_string()))?;
        guard
            .get(tool_name)
            .cloned()
            .ok_or_else(|| McpError::UnknownTool(tool_name.to_string()))
    }

    fn tool_policy(&self, tool_name: &str) -> ToolPolicy {
        self.gate
            .tool_policies
            .get(tool_name)
            .cloned()
            .unwrap_or(ToolPolicy {
                required_capability: None,
                requires_consent: false,
                path_argument: None,
            })
    }

    fn enforce_capability(&self, capability_id: &str, granted: &[String]) -> McpResult<()> {
        let allowed = self
            .capability_registry
            .enforce_can_perform(capability_id, granted)
            .map_err(|e| McpError::CapabilityDenied(e.to_string()))?;
        if !allowed {
            return Err(McpError::CapabilityDenied(format!(
                "capability denied: {}",
                capability_id
            )));
        }
        Ok(())
    }

    async fn enforce_consent(
        &self,
        ctx: &McpContext,
        tool_name: &str,
        capability_id: Option<&str>,
        policy: &ToolPolicy,
    ) -> McpResult<()> {
        let needs_consent = policy.requires_consent
            || ctx.access_mode == AccessMode::ApplyScoped
            || capability_id
                .map(|c| c.starts_with("fs.") || c.starts_with("net."))
                .unwrap_or(false);

        if !needs_consent {
            return Ok(());
        }

        if ctx.human_consent_obtained {
            return Ok(());
        }

        let decision = match tokio::time::timeout(
            self.gate.consent_timeout,
            self.consent_provider
                .request_consent(ctx, &self.server_id, tool_name, capability_id),
        )
        .await
        {
            Ok(d) => d,
            Err(_) => ConsentDecision::Timeout,
        };

        match decision {
            ConsentDecision::Allow => Ok(()),
            ConsentDecision::Deny => {
                Err(McpError::ConsentDenied("human consent denied".to_string()))
            }
            ConsentDecision::Timeout => Err(McpError::ConsentDenied(
                "human consent timed out".to_string(),
            )),
        }
    }

    fn enforce_path_policy(
        &self,
        ctx: &McpContext,
        policy: &ToolPolicy,
        arguments: &Value,
    ) -> McpResult<()> {
        let Some(arg_name) = &policy.path_argument else {
            return Ok(());
        };
        let Some(path_str) = arguments.get(arg_name).and_then(|v| v.as_str()) else {
            return Err(McpError::SecurityViolation(format!(
                "missing string path argument: {}",
                arg_name
            )));
        };
        let _ = security::canonicalize_under_roots(path_str, &ctx.allowed_roots)?;
        Ok(())
    }

    pub async fn tools_call(
        &self,
        ctx: McpContext,
        tool_name: &str,
        arguments: Value,
    ) -> McpResult<Value> {
        if let Some(allowed) = &self.gate.allowed_tools {
            if !allowed.contains(tool_name) {
                let err =
                    McpError::CapabilityDenied(format!("tool not in allowed_tools: {}", tool_name));
                let _ = fr_events::record_gate_decision(
                    Arc::clone(&self.flight_recorder),
                    &ctx,
                    &self.server_id,
                    Some(tool_name),
                    "deny",
                    "tool_not_allowed",
                    json!({ "error": err.to_string() }),
                );
                return Err(err);
            }
        }

        let desc = match self.tool_descriptor(tool_name) {
            Ok(d) => d,
            Err(err) => {
                let _ = fr_events::record_gate_decision(
                    Arc::clone(&self.flight_recorder),
                    &ctx,
                    &self.server_id,
                    Some(tool_name),
                    "deny",
                    "unknown_tool",
                    json!({ "error": err.to_string() }),
                );
                return Err(err);
            }
        };
        let schema_value = desc.input_schema.ok_or_else(|| McpError::SchemaValidation {
            details: "missing tool inputSchema".to_string(),
        });
        let schema_value = match schema_value {
            Ok(s) => s,
            Err(err) => {
                let _ = fr_events::record_gate_decision(
                    Arc::clone(&self.flight_recorder),
                    &ctx,
                    &self.server_id,
                    Some(tool_name),
                    "deny",
                    "missing_input_schema",
                    json!({ "error": err.to_string() }),
                );
                return Err(err);
            }
        };
        if let Err(err) = schema::validate_instance(&schema_value, &arguments) {
            let _ = fr_events::record_gate_decision(
                Arc::clone(&self.flight_recorder),
                &ctx,
                &self.server_id,
                Some(tool_name),
                "deny",
                "schema_validation_failed",
                json!({ "error": err.to_string() }),
            );
            return Err(err);
        }

        let policy = self.tool_policy(tool_name);
        if let Some(cap) = policy.required_capability.as_deref() {
            if let Err(err) = self.enforce_capability(cap, &ctx.granted_capabilities) {
                let _ = fr_events::record_gate_decision(
                    Arc::clone(&self.flight_recorder),
                    &ctx,
                    &self.server_id,
                    Some(tool_name),
                    "deny",
                    "capability_denied",
                    json!({ "capability_id": cap, "error": err.to_string() }),
                );
                return Err(err);
            }
        }
        if let Err(err) = self
            .enforce_consent(
                &ctx,
                tool_name,
                policy.required_capability.as_deref(),
                &policy,
            )
            .await
        {
            let _ = fr_events::record_gate_decision(
                Arc::clone(&self.flight_recorder),
                &ctx,
                &self.server_id,
                Some(tool_name),
                "deny",
                "consent_denied",
                json!({ "error": err.to_string() }),
            );
            return Err(err);
        }
        if let Err(err) = self.enforce_path_policy(&ctx, &policy, &arguments) {
            let _ = fr_events::record_gate_decision(
                Arc::clone(&self.flight_recorder),
                &ctx,
                &self.server_id,
                Some(tool_name),
                "deny",
                "security_violation",
                json!({ "error": err.to_string() }),
            );
            return Err(err);
        }

        fr_events::record_tool_call(
            Arc::clone(&self.flight_recorder),
            &ctx,
            &self.server_id,
            tool_name,
            policy.required_capability.as_deref(),
            &arguments,
        )?;

        let started_at = Instant::now(); // WAIVER [CX-573E] duration/timeout bookkeeping only
        let params = json!({ "name": tool_name, "arguments": arguments });
        let meta = PendingMeta {
            started_at,
            method: "tools/call".to_string(),
            ctx: Some(ctx.clone()),
            tool_name: Some(tool_name.to_string()),
            capability_id: policy.required_capability.clone(),
        };

        let call = self
            .inner
            .send_request("tools/call", Some(params), Some(meta))?;
        match tokio::time::timeout(self.gate.request_timeout, call).await {
            Ok(result) => result,
            Err(_) => {
                let timeout_payload = json!({"timeout_ms": self.gate.request_timeout.as_millis()});
                let _ = fr_events::record_gate_decision(
                    Arc::clone(&self.flight_recorder),
                    &ctx,
                    &self.server_id,
                    Some(tool_name),
                    "timeout",
                    "request_timeout",
                    timeout_payload.clone(),
                );
                let _ = fr_events::record_tool_result(
                    Arc::clone(&self.flight_recorder),
                    &ctx,
                    &self.server_id,
                    tool_name,
                    policy.required_capability.as_deref(),
                    "timeout",
                    Some(self.gate.request_timeout.as_millis()),
                    Some("timeout"),
                    &timeout_payload,
                );
                Err(McpError::Timeout(format!(
                    "request timed out after {:?}",
                    self.gate.request_timeout
                )))
            }
        }
    }
}
