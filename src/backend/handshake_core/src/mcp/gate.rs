use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::ace::ArtifactHandle;
use crate::bundles::redactor::SecretRedactor;
use crate::bundles::schemas::RedactionMode;
use crate::capabilities::CapabilityRegistry;
use crate::flight_recorder::duckdb::store_tool_payload_redacted;
use crate::flight_recorder::{
    canonical_json_sha256_hex, FlightRecorder, FlightRecorderActor, FlightRecorderEvent,
    FlightRecorderEventType,
};
use crate::storage::{AccessMode, AiJobMcpUpdate, Database};

use super::client::{JsonRpcMcpClient, McpDispatcher, PendingMeta};
use super::discovery::{
    McpResourceDescriptor, McpToolDescriptor, ResourcesListResult,
};
use super::errors::{McpError, McpResult};
use super::fr_events;
use super::jsonrpc::{JsonRpcId, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
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
pub struct ToolTransportBindings {
    pub mcp_name: String,
}

#[derive(Clone, Debug)]
pub struct ToolRegistryEntry {
    pub tool_id: String,
    pub tool_version: String,
    pub input_schema: Value,
    pub output_schema: Option<Value>,
    pub side_effect: String,
    pub idempotency: String,
    pub determinism: String,
    pub availability: String,
    pub required_capabilities: Vec<String>,
    pub transport_bindings: ToolTransportBindings,
}

impl ToolRegistryEntry {
    fn handshake_meta(&self) -> Value {
        json!({
            "tool_version": &self.tool_version,
            "side_effect": &self.side_effect,
            "idempotency": &self.idempotency,
            "determinism": &self.determinism,
            "availability": &self.availability,
            "required_capabilities": &self.required_capabilities,
        })
    }

    fn tool_descriptor(&self) -> McpToolDescriptor {
        McpToolDescriptor {
            name: self.tool_id.clone(),
            description: None,
            input_schema: Some(self.input_schema.clone()),
            meta: Some(json!({ "handshake": self.handshake_meta() })),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GateConfig {
    pub allowed_tools: Option<HashSet<String>>,
    pub tool_policies: HashMap<String, ToolPolicy>,
    pub tool_registry: Vec<ToolRegistryEntry>,
    pub request_timeout: Duration,
    pub consent_timeout: Duration,
    pub reconnect: ReconnectConfig,
}

impl GateConfig {
    pub fn minimal() -> Self {
        Self {
            allowed_tools: None,
            tool_policies: HashMap::new(),
            tool_registry: Vec::new(),
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
    progress_bindings: Arc<Mutex<HashMap<String, ProgressBinding>>>,
}

#[derive(Clone, Debug)]
struct ProgressBinding {
    ctx: McpContext,
    tool_name: String,
    capability_id: Option<String>,
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
        } else if notification.method == "notifications/progress" {
            let Some(params) = &notification.params else {
                return;
            };
            let token_value = params.get("token").cloned().unwrap_or(Value::Null);
            let token = match token_value {
                Value::String(s) => s,
                Value::Number(n) => n.to_string(),
                _ => String::new(),
            };
            if token.is_empty() {
                return;
            }
            let progress = params.get("progress").and_then(|v| v.as_f64());
            let message = params.get("message").and_then(|v| v.as_str());
            let binding = match self.progress_bindings.lock() {
                Ok(g) => g.get(&token).cloned(),
                Err(poisoned) => poisoned.into_inner().get(&token).cloned(),
            };
            let ctx = binding.as_ref().map(|b| &b.ctx);
            let tool_name = binding.as_ref().map(|b| b.tool_name.as_str());
            let capability_id = binding.as_ref().and_then(|b| b.capability_id.as_deref());
            let _ = fr_events::record_progress(
                Arc::clone(&self.flight_recorder),
                ctx,
                &self.server_id,
                &token,
                tool_name,
                capability_id,
                progress,
                message,
            );
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

        if let JsonRpcId::String(token) = &response.id {
            if let Ok(mut guard) = self.progress_bindings.lock() {
                guard.remove(token);
            }
        }
    }
}

pub struct GatedMcpClient {
    server_id: String,
    flight_recorder: Arc<dyn FlightRecorder>,
    capability_registry: Arc<CapabilityRegistry>,
    consent_provider: Arc<dyn ConsentProvider>,
    gate: GateConfig,
    inner: JsonRpcMcpClient,
    db: Option<Arc<dyn Database>>,
    progress_bindings: Arc<Mutex<HashMap<String, ProgressBinding>>>,
}

fn strip_ref_scheme(uri: &str) -> McpResult<&str> {
    if let Some(path) = uri.strip_prefix("ref://") {
        if path.trim().is_empty() {
            return Err(McpError::SecurityViolation("ref uri path is empty".to_string()));
        }
        return Ok(path);
    }

    if uri.starts_with("file://") {
        return Err(McpError::SecurityViolation(
            "file:// uris are rejected for host-side reference hydration".to_string(),
        ));
    }

    if uri.contains("://") {
        return Err(McpError::SecurityViolation(format!(
            "unknown uri scheme rejected: {}",
            uri
        )));
    }

    Err(McpError::SecurityViolation(format!(
        "expected ref:// uri, got: {}",
        uri
    )))
}

const HTC_SCHEMA_VERSION: &str = "htc-1.0";
const HTC_MAX_PAYLOAD_BYTES: usize = 32 * 1024;
const HTC_VALIDATION_ERROR_CODE: &str = "VAL-HTC-001";

fn json_encoded_len(value: &Value) -> usize {
    serde_json::to_vec(value).map(|bytes| bytes.len()).unwrap_or(usize::MAX)
}

fn htc_schema() -> &'static Value {
    static HTC: OnceLock<Value> = OnceLock::new();
    HTC.get_or_init(|| {
        serde_json::from_str(include_str!("../../../../../assets/schemas/htc_v1.json"))
            .unwrap_or_else(|err| {
                json!({
                    "$comment": format!("INVALID HTC schema JSON embedded in binary: {err}"),
                    "not": {},
                })
            })
    })
}

fn default_actor_payload() -> Value {
    json!({
        "kind": "agent",
        "agent_id": null,
        "model_id": null,
    })
}

fn canonical_tool_id_segment(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len().min(64));
    for ch in raw.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.trim_matches('_').is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

pub fn canonical_mcp_tool_id(server_id: &str, tool_name: &str) -> String {
    let server = canonical_tool_id_segment(server_id);
    let mut tool_segments: Vec<String> = tool_name
        .split('.')
        .filter(|seg| !seg.trim().is_empty())
        .map(canonical_tool_id_segment)
        .collect();
    if tool_segments.is_empty() {
        tool_segments.push("unknown".to_string());
    }
    format!("mcp.{server}.{}", tool_segments.join("."))
}

fn is_simple_semver(value: &str) -> bool {
    let segments: Vec<&str> = value.trim().split('.').collect();
    if segments.len() != 3 {
        return false;
    }
    segments
        .into_iter()
        .all(|seg| !seg.is_empty() && seg.chars().all(|c| c.is_ascii_digit()))
}

fn default_htc_resources() -> Value {
    json!({
        "workspace_ids": [],
        "artifacts": [],
        "files": [],
        "urls": [],
    })
}

fn htc_timing(started_at: chrono::DateTime<Utc>, ended_at: chrono::DateTime<Utc>) -> Value {
    let duration_ms = ended_at
        .signed_duration_since(started_at)
        .num_milliseconds()
        .max(0);
    json!({
        "started_at": started_at.to_rfc3339(),
        "ended_at": ended_at.to_rfc3339(),
        "duration_ms": duration_ms,
    })
}

fn htc_error_object(
    code: &str,
    kind: &str,
    message: Option<String>,
    retryable: Option<bool>,
    details: Option<Value>,
) -> Value {
    json!({
        "code": code,
        "kind": kind,
        "message": message,
        "retryable": retryable,
        "details": details,
    })
}

impl GatedMcpClient {
    async fn connect_internal<T: McpTransport + 'static>(
        server_id: impl Into<String>,
        transport: T,
        flight_recorder: Arc<dyn FlightRecorder>,
        capability_registry: Arc<CapabilityRegistry>,
        consent_provider: Arc<dyn ConsentProvider>,
        gate: GateConfig,
        agentic_mode_enabled: bool,
        db: Option<Arc<dyn Database>>,
    ) -> McpResult<Self> {
        let server_id = server_id.into();
        let mut transport = AutoReconnectTransport::new(transport, gate.reconnect.clone());
        let progress_bindings: Arc<Mutex<HashMap<String, ProgressBinding>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let dispatcher: Arc<dyn McpDispatcher> = Arc::new(GateDispatcher {
            server_id: server_id.clone(),
            flight_recorder: Arc::clone(&flight_recorder),
            agentic_mode_enabled,
            progress_bindings: Arc::clone(&progress_bindings),
        });
        let inner = JsonRpcMcpClient::connect(&mut transport, dispatcher).await?;

        Ok(Self {
            server_id,
            flight_recorder,
            capability_registry,
            consent_provider,
            gate,
            inner,
            db,
            progress_bindings,
        })
    }

    pub async fn connect<T: McpTransport + 'static>(
        server_id: impl Into<String>,
        transport: T,
        flight_recorder: Arc<dyn FlightRecorder>,
        capability_registry: Arc<CapabilityRegistry>,
        consent_provider: Arc<dyn ConsentProvider>,
        gate: GateConfig,
        agentic_mode_enabled: bool,
    ) -> McpResult<Self> {
        Self::connect_internal(
            server_id,
            transport,
            flight_recorder,
            capability_registry,
            consent_provider,
            gate,
            agentic_mode_enabled,
            None,
        )
        .await
    }

    pub async fn connect_with_db<T: McpTransport + 'static>(
        server_id: impl Into<String>,
        transport: T,
        flight_recorder: Arc<dyn FlightRecorder>,
        capability_registry: Arc<CapabilityRegistry>,
        consent_provider: Arc<dyn ConsentProvider>,
        gate: GateConfig,
        agentic_mode_enabled: bool,
        db: Arc<dyn Database>,
    ) -> McpResult<Self> {
        Self::connect_internal(
            server_id,
            transport,
            flight_recorder,
            capability_registry,
            consent_provider,
            gate,
            agentic_mode_enabled,
            Some(db),
        )
        .await
    }

    pub async fn refresh_tools(&self) -> McpResult<Vec<McpToolDescriptor>> {
        let mut tools = self
            .gate
            .tool_registry
            .iter()
            .map(ToolRegistryEntry::tool_descriptor)
            .collect::<Vec<_>>();
        tools.sort_by(|a, b| a.name.cmp(&b.name));

        for window in tools.windows(2) {
            if window[0].name == window[1].name {
                return Err(McpError::Protocol(format!(
                    "duplicate tool_id in tool registry: {}",
                    window[0].name
                )));
            }
        }

        Ok(tools)
    }

    pub async fn resources_list(&self) -> McpResult<Vec<McpResourceDescriptor>> {
        let value = self
            .inner
            .send_request("resources/list", Some(json!({})), None)?
            .await?;
        let result: ResourcesListResult = serde_json::from_value(value)?;
        Ok(result.resources)
    }

    fn resolve_tool_registry_entry(&self, tool_id_or_alias: &str) -> McpResult<&ToolRegistryEntry> {
        if let Some(entry) = self
            .gate
            .tool_registry
            .iter()
            .find(|entry| entry.tool_id == tool_id_or_alias)
        {
            return Ok(entry);
        }

        let mut matches = self.gate.tool_registry.iter().filter(|entry| {
            entry.transport_bindings.mcp_name == tool_id_or_alias
        });
        let Some(first) = matches.next() else {
            return Err(McpError::UnknownTool(tool_id_or_alias.to_string()));
        };
        if matches.next().is_some() {
            return Err(McpError::Protocol(format!(
                "mcp tool alias matches multiple registry entries: {}",
                tool_id_or_alias
            )));
        }
        Ok(first)
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

    pub async fn tools_call_htc(
        &self,
        ctx: McpContext,
        tool_id_or_alias: &str,
        arguments: Value,
    ) -> McpResult<Value> {
        let started_at_utc = Utc::now();
        let started_at_instant = Instant::now(); // WAIVER [CX-573E] duration/timeout bookkeeping only

        let tool_call_id = Uuid::new_v4();

        let (registry_entry, registry_error) = match self.resolve_tool_registry_entry(tool_id_or_alias) {
            Ok(entry) => (Some(entry), None),
            Err(err) => (None, Some(err)),
        };

        let tool_id = match registry_entry {
            Some(entry) => entry.tool_id.clone(),
            None => {
                if tool_id_or_alias.starts_with("mcp.") {
                    tool_id_or_alias.to_string()
                } else {
                    canonical_mcp_tool_id(&self.server_id, tool_id_or_alias)
                }
            }
        };
        let mcp_tool_name = registry_entry
            .map(|entry| entry.transport_bindings.mcp_name.clone())
            .unwrap_or_else(|| tool_id_or_alias.to_string());

        let tool_version = registry_entry
            .map(|entry| entry.tool_version.as_str())
            .filter(|v| is_simple_semver(v))
            .unwrap_or("0.0.0")
            .to_string();

        let side_effect = match registry_entry {
            Some(entry) => match entry.side_effect.as_str() {
                "READ" | "WRITE" | "EXECUTE" => entry.side_effect.clone(),
                _ => "READ".to_string(),
            },
            None => "READ".to_string(),
        };
        let idempotency = match registry_entry {
            Some(entry) => match entry.idempotency.as_str() {
                "IDEMPOTENT" | "IDEMPOTENT_WITH_KEY" | "NON_IDEMPOTENT" => {
                    entry.idempotency.clone()
                }
                _ => "IDEMPOTENT".to_string(),
            },
            None => "IDEMPOTENT".to_string(),
        };
        let idempotency_key = if idempotency == "IDEMPOTENT_WITH_KEY" {
            Some(tool_call_id.to_string())
        } else {
            None
        };

        let redactor = SecretRedactor::new();
        let (args_redacted, args_redaction_logs) = redactor.redact_value(
            &arguments,
            RedactionMode::SafeDefault,
            "tool_gate/mcp/args",
        );

        let (args_handle, args_hash) = if let Some(conn) = self.flight_recorder.duckdb_connection() {
            let conn = conn.lock().map_err(|_| {
                McpError::FlightRecorder("duckdb connection lock error".to_string())
            })?;
            store_tool_payload_redacted(&conn, tool_call_id, "args", &args_redacted)
                .map_err(|e| McpError::FlightRecorder(e.to_string()))?
        } else {
            let artifact_path = format!("data/flight_recorder/tool_payloads/{tool_call_id}/args.json");
            (
                ArtifactHandle::new(tool_call_id, artifact_path),
                canonical_json_sha256_hex(&args_redacted),
            )
        };
        let args_ref = args_handle.canonical_id();
        let args_hash = args_hash;

        let args_inline = if matches!(args_redacted, Value::Object(_))
            && json_encoded_len(&args_redacted) <= HTC_MAX_PAYLOAD_BYTES
        {
            args_redacted.clone()
        } else {
            json!({})
        };

        let request_envelope = json!({
            "schema_version": HTC_SCHEMA_VERSION,
            "tool_call_id": tool_call_id.to_string(),
            "trace_id": ctx.trace_id.to_string(),
            "session_id": ctx.session_id,
            "actor": default_actor_payload(),
            "tool_id": tool_id,
            "tool_version": tool_version,
            "args": args_inline,
            "args_ref": args_ref,
            "idempotency_key": idempotency_key,
            "dry_run": ctx.access_mode != AccessMode::ApplyScoped,
        });
        if let Err(err) = schema::validate_instance(htc_schema(), &request_envelope) {
            let details = err.to_string();
            let ended_at_utc = Utc::now();

            let response_envelope = json!({
                "schema_version": HTC_SCHEMA_VERSION,
                "tool_call_id": tool_call_id.to_string(),
                "trace_id": ctx.trace_id.to_string(),
                "ok": false,
                "error": htc_error_object(
                    HTC_VALIDATION_ERROR_CODE,
                    "validation",
                    Some(details.clone()),
                    Some(false),
                    None,
                ),
                "timing": htc_timing(started_at_utc, ended_at_utc),
                "resources": default_htc_resources(),
            });
            let _ = schema::validate_instance(htc_schema(), &response_envelope);

            let payload = json!({
                "type": "tool_call",
                "trace_id": ctx.trace_id.to_string(),
                "tool_call_id": tool_call_id.to_string(),
                "tool_id": tool_id,
                "tool_version": tool_version,
                "transport": "mcp",
                "side_effect": side_effect,
                "idempotency": idempotency,
                "idempotency_key": idempotency_key,
                "actor": default_actor_payload(),
                "ok": false,
                "args_ref": args_ref,
                "args_hash": args_hash,
                "error": {
                    "code": HTC_VALIDATION_ERROR_CODE,
                    "kind": "validation",
                    "message": details.clone(),
                    "retryable": false,
                },
                "timing": htc_timing(started_at_utc, ended_at_utc),
            });
            let mut event = FlightRecorderEvent::new(
                FlightRecorderEventType::ToolCall,
                FlightRecorderActor::Agent,
                ctx.trace_id,
                payload,
            );
            if let Some(job_id) = ctx.job_id {
                event = event.with_job_id(job_id.to_string());
            }
            if let Some(workflow_run_id) = ctx.workflow_run_id.as_deref() {
                event = event.with_workflow_id(workflow_run_id.to_string());
            }
            self.flight_recorder
                .record_event(event)
                .await
                .map_err(|e| McpError::FlightRecorder(e.to_string()))?;

            return Err(McpError::SchemaValidation {
                details: format!("{HTC_VALIDATION_ERROR_CODE}: request envelope invalid\n{details}"),
            });
        }

        let registry_entry = match (registry_entry, registry_error) {
            (Some(entry), None) => entry,
            (None, Some(err)) => {
                let (reason, code, kind) = match &err {
                    McpError::UnknownTool(_) => ("unknown_tool", "unknown_tool", "validation"),
                    _ => ("tool_registry_error", "tool_registry_error", "protocol"),
                };
                let _ = fr_events::record_gate_decision(
                    Arc::clone(&self.flight_recorder),
                    &ctx,
                    &self.server_id,
                    Some(tool_id.as_str()),
                    "deny",
                    reason,
                    json!({ "error": err.to_string() }),
                );

                let ended_at_utc = Utc::now();
                let payload = json!({
                    "type": "tool_call",
                    "trace_id": ctx.trace_id.to_string(),
                    "tool_call_id": tool_call_id.to_string(),
                    "tool_id": tool_id,
                    "tool_version": tool_version,
                    "transport": "mcp",
                    "side_effect": side_effect,
                    "idempotency": idempotency,
                    "idempotency_key": idempotency_key,
                    "actor": default_actor_payload(),
                    "ok": false,
                    "args_ref": args_ref,
                    "args_hash": args_hash,
                    "error": {
                        "code": code,
                        "kind": kind,
                        "message": err.to_string(),
                        "retryable": false,
                    },
                    "timing": htc_timing(started_at_utc, ended_at_utc),
                });
                let mut event = FlightRecorderEvent::new(
                    FlightRecorderEventType::ToolCall,
                    FlightRecorderActor::Agent,
                    ctx.trace_id,
                    payload,
                );
                if let Some(job_id) = ctx.job_id {
                    event = event.with_job_id(job_id.to_string());
                }
                if let Some(workflow_run_id) = ctx.workflow_run_id.as_deref() {
                    event = event.with_workflow_id(workflow_run_id.to_string());
                }
                self.flight_recorder
                    .record_event(event)
                    .await
                    .map_err(|e| McpError::FlightRecorder(e.to_string()))?;

                return Err(err);
            }
            _ => {
                return Err(McpError::Protocol(
                    "tool registry resolution invariant violated".to_string(),
                ));
            }
        };

        if let Some(allowed) = &self.gate.allowed_tools {
            if !allowed.contains(&tool_id) && !allowed.contains(&mcp_tool_name) {
                let err = McpError::CapabilityDenied(format!(
                    "tool not in allowed_tools: {}",
                    tool_id
                ));
                let _ = fr_events::record_gate_decision(
                    Arc::clone(&self.flight_recorder),
                    &ctx,
                    &self.server_id,
                    Some(tool_id.as_str()),
                    "deny",
                    "tool_not_allowed",
                    json!({ "error": err.to_string() }),
                );

                let ended_at_utc = Utc::now();
                let payload = json!({
                    "type": "tool_call",
                    "trace_id": ctx.trace_id.to_string(),
                    "tool_call_id": tool_call_id.to_string(),
                    "tool_id": tool_id,
                    "tool_version": tool_version,
                    "transport": "mcp",
                    "side_effect": side_effect,
                    "idempotency": idempotency,
                    "idempotency_key": idempotency_key,
                    "actor": default_actor_payload(),
                    "ok": false,
                    "args_ref": args_ref,
                    "args_hash": args_hash,
                    "error": {
                        "code": "tool_not_allowed",
                        "kind": "policy",
                        "message": err.to_string(),
                        "retryable": false,
                    },
                    "timing": htc_timing(started_at_utc, ended_at_utc),
                });
                let mut event = FlightRecorderEvent::new(
                    FlightRecorderEventType::ToolCall,
                    FlightRecorderActor::Agent,
                    ctx.trace_id,
                    payload,
                );
                if let Some(job_id) = ctx.job_id {
                    event = event.with_job_id(job_id.to_string());
                }
                if let Some(workflow_run_id) = ctx.workflow_run_id.as_deref() {
                    event = event.with_workflow_id(workflow_run_id.to_string());
                }
                self.flight_recorder
                    .record_event(event)
                    .await
                    .map_err(|e| McpError::FlightRecorder(e.to_string()))?;

                return Err(err);
            }
        }

        if registry_entry.input_schema.is_null() {
            let err = McpError::SchemaValidation {
                details: "missing tool inputSchema".to_string(),
            };
            let _ = fr_events::record_gate_decision(
                Arc::clone(&self.flight_recorder),
                &ctx,
                &self.server_id,
                Some(tool_id.as_str()),
                "deny",
                "missing_input_schema",
                json!({ "error": err.to_string() }),
            );

            let ended_at_utc = Utc::now();
            let payload = json!({
                "type": "tool_call",
                "trace_id": ctx.trace_id.to_string(),
                "tool_call_id": tool_call_id.to_string(),
                "tool_id": tool_id,
                "tool_version": tool_version,
                "transport": "mcp",
                "side_effect": side_effect,
                "idempotency": idempotency,
                "idempotency_key": idempotency_key,
                "actor": default_actor_payload(),
                "ok": false,
                "args_ref": args_ref,
                "args_hash": args_hash,
                "error": {
                    "code": "missing_input_schema",
                    "kind": "validation",
                    "message": err.to_string(),
                    "retryable": false,
                },
                "timing": htc_timing(started_at_utc, ended_at_utc),
            });
            let mut event = FlightRecorderEvent::new(
                FlightRecorderEventType::ToolCall,
                FlightRecorderActor::Agent,
                ctx.trace_id,
                payload,
            );
            if let Some(job_id) = ctx.job_id {
                event = event.with_job_id(job_id.to_string());
            }
            if let Some(workflow_run_id) = ctx.workflow_run_id.as_deref() {
                event = event.with_workflow_id(workflow_run_id.to_string());
            }
            self.flight_recorder
                .record_event(event)
                .await
                .map_err(|e| McpError::FlightRecorder(e.to_string()))?;

            return Err(err);
        }

        if let Err(err) = schema::validate_instance(&registry_entry.input_schema, &arguments) {
            let _ = fr_events::record_gate_decision(
                Arc::clone(&self.flight_recorder),
                &ctx,
                &self.server_id,
                Some(tool_id.as_str()),
                "deny",
                "schema_validation_failed",
                json!({ "error": err.to_string() }),
            );

            let ended_at_utc = Utc::now();
            let payload = json!({
                "type": "tool_call",
                "trace_id": ctx.trace_id.to_string(),
                "tool_call_id": tool_call_id.to_string(),
                "tool_id": tool_id,
                "tool_version": tool_version,
                "transport": "mcp",
                "side_effect": side_effect,
                "idempotency": idempotency,
                "idempotency_key": idempotency_key,
                "actor": default_actor_payload(),
                "ok": false,
                "args_ref": args_ref,
                "args_hash": args_hash,
                "error": {
                    "code": "schema_validation_failed",
                    "kind": "validation",
                    "message": err.to_string(),
                    "retryable": false,
                },
                "timing": htc_timing(started_at_utc, ended_at_utc),
            });
            let mut event = FlightRecorderEvent::new(
                FlightRecorderEventType::ToolCall,
                FlightRecorderActor::Agent,
                ctx.trace_id,
                payload,
            );
            if let Some(job_id) = ctx.job_id {
                event = event.with_job_id(job_id.to_string());
            }
            if let Some(workflow_run_id) = ctx.workflow_run_id.as_deref() {
                event = event.with_workflow_id(workflow_run_id.to_string());
            }
            self.flight_recorder
                .record_event(event)
                .await
                .map_err(|e| McpError::FlightRecorder(e.to_string()))?;

            return Err(err);
        }

        let policy = self.tool_policy(&tool_id);
        let mut required_caps = registry_entry.required_capabilities.clone();
        if let Some(cap) = policy.required_capability.clone() {
            if !required_caps.iter().any(|c| c == &cap) {
                required_caps.push(cap);
            }
        }
        for cap in required_caps.iter().map(|s| s.as_str()) {
            if let Err(err) = self.enforce_capability(cap, &ctx.granted_capabilities) {
                let _ = fr_events::record_gate_decision(
                    Arc::clone(&self.flight_recorder),
                    &ctx,
                    &self.server_id,
                    Some(tool_id.as_str()),
                    "deny",
                    "capability_denied",
                    json!({ "capability_id": cap, "error": err.to_string() }),
                );

                let ended_at_utc = Utc::now();
                let payload = json!({
                    "type": "tool_call",
                    "trace_id": ctx.trace_id.to_string(),
                    "tool_call_id": tool_call_id.to_string(),
                    "tool_id": tool_id,
                    "tool_version": tool_version,
                    "transport": "mcp",
                    "side_effect": side_effect,
                    "idempotency": idempotency,
                    "idempotency_key": idempotency_key,
                    "actor": default_actor_payload(),
                    "ok": false,
                    "args_ref": args_ref,
                    "args_hash": args_hash,
                    "error": {
                        "code": "capability_denied",
                        "kind": "capability",
                        "message": err.to_string(),
                        "retryable": false,
                    },
                    "timing": htc_timing(started_at_utc, ended_at_utc),
                    "capability_ids": &required_caps,
                });
                let mut event = FlightRecorderEvent::new(
                    FlightRecorderEventType::ToolCall,
                    FlightRecorderActor::Agent,
                    ctx.trace_id,
                    payload,
                );
                if let Some(job_id) = ctx.job_id {
                    event = event.with_job_id(job_id.to_string());
                }
                if let Some(workflow_run_id) = ctx.workflow_run_id.as_deref() {
                    event = event.with_workflow_id(workflow_run_id.to_string());
                }
                self.flight_recorder
                    .record_event(event)
                    .await
                    .map_err(|e| McpError::FlightRecorder(e.to_string()))?;

                return Err(err);
            }
        }
        if let Err(err) = self
            .enforce_consent(
                &ctx,
                tool_id.as_str(),
                required_caps.first().map(|s| s.as_str()),
                &policy,
            )
            .await
        {
            let _ = fr_events::record_gate_decision(
                Arc::clone(&self.flight_recorder),
                &ctx,
                &self.server_id,
                Some(tool_id.as_str()),
                "deny",
                "consent_denied",
                json!({ "error": err.to_string() }),
            );

            let ended_at_utc = Utc::now();
            let payload = json!({
                "type": "tool_call",
                "trace_id": ctx.trace_id.to_string(),
                "tool_call_id": tool_call_id.to_string(),
                "tool_id": tool_id,
                "tool_version": tool_version,
                "transport": "mcp",
                "side_effect": side_effect,
                "idempotency": idempotency,
                "idempotency_key": idempotency_key,
                "actor": default_actor_payload(),
                "ok": false,
                "args_ref": args_ref,
                "args_hash": args_hash,
                "error": {
                    "code": "consent_denied",
                    "kind": "policy",
                    "message": err.to_string(),
                    "retryable": false,
                },
                "timing": htc_timing(started_at_utc, ended_at_utc),
                "capability_ids": &required_caps,
            });
            let mut event = FlightRecorderEvent::new(
                FlightRecorderEventType::ToolCall,
                FlightRecorderActor::Agent,
                ctx.trace_id,
                payload,
            );
            if let Some(job_id) = ctx.job_id {
                event = event.with_job_id(job_id.to_string());
            }
            if let Some(workflow_run_id) = ctx.workflow_run_id.as_deref() {
                event = event.with_workflow_id(workflow_run_id.to_string());
            }
            self.flight_recorder
                .record_event(event)
                .await
                .map_err(|e| McpError::FlightRecorder(e.to_string()))?;

            return Err(err);
        }
        if let Err(err) = self.enforce_path_policy(&ctx, &policy, &arguments) {
            let _ = fr_events::record_gate_decision(
                Arc::clone(&self.flight_recorder),
                &ctx,
                &self.server_id,
                Some(tool_id.as_str()),
                "deny",
                "security_violation",
                json!({ "error": err.to_string() }),
            );

            let ended_at_utc = Utc::now();
            let payload = json!({
                "type": "tool_call",
                "trace_id": ctx.trace_id.to_string(),
                "tool_call_id": tool_call_id.to_string(),
                "tool_id": tool_id,
                "tool_version": tool_version,
                "transport": "mcp",
                "side_effect": side_effect,
                "idempotency": idempotency,
                "idempotency_key": idempotency_key,
                "actor": default_actor_payload(),
                "ok": false,
                "args_ref": args_ref,
                "args_hash": args_hash,
                "error": {
                    "code": "security_violation",
                    "kind": "policy",
                    "message": err.to_string(),
                    "retryable": false,
                },
                "timing": htc_timing(started_at_utc, ended_at_utc),
                "capability_ids": &required_caps,
            });
            let mut event = FlightRecorderEvent::new(
                FlightRecorderEventType::ToolCall,
                FlightRecorderActor::Agent,
                ctx.trace_id,
                payload,
            );
            if let Some(job_id) = ctx.job_id {
                event = event.with_job_id(job_id.to_string());
            }
            if let Some(workflow_run_id) = ctx.workflow_run_id.as_deref() {
                event = event.with_workflow_id(workflow_run_id.to_string());
            }
            self.flight_recorder
                .record_event(event)
                .await
                .map_err(|e| McpError::FlightRecorder(e.to_string()))?;

            return Err(err);
        }

        fr_events::record_tool_call(
            Arc::clone(&self.flight_recorder),
            &ctx,
            &self.server_id,
            mcp_tool_name.as_str(),
            tool_call_id,
            &tool_id,
            tool_version.as_str(),
            required_caps.first().map(|s| s.as_str()),
            Some(args_ref.as_str()),
            Some(args_hash.as_str()),
        )?;

        let meta = PendingMeta {
            started_at: started_at_instant,
            method: "tools/call".to_string(),
            ctx: Some(ctx.clone()),
            tool_name: Some(tool_id.clone()),
            capability_id: required_caps.first().cloned(),
        };

        let mut request_id: Option<JsonRpcId> = None;
        let mut progress_token: Option<String> = None;
        let mut mcp_job_id: Option<String> = None;

        if let (Some(db), Some(job_id)) = (self.db.as_ref(), ctx.job_id) {
            let token = self.inner.reserve_progress_token();

            db.update_ai_job_mcp_fields(
                job_id,
                AiJobMcpUpdate {
                    mcp_server_id: Some(self.server_id.clone()),
                    mcp_call_id: Some(token.clone()),
                    mcp_progress_token: Some(token.clone()),
                },
            )
            .await
            .map_err(|e| McpError::Protocol(format!("durable progress mapping failed: {e}")))?;

            let binding = ProgressBinding {
                ctx: ctx.clone(),
                tool_name: mcp_tool_name.clone(),
                capability_id: required_caps.first().cloned(),
            };
            match self.progress_bindings.lock() {
                Ok(mut guard) => {
                    guard.insert(token.clone(), binding);
                }
                Err(poisoned) => {
                    poisoned.into_inner().insert(token.clone(), binding);
                }
            }

            request_id = Some(JsonRpcId::String(token.clone()));
            progress_token = Some(token);
            mcp_job_id = Some(job_id.to_string());
        }

        let mut params = json!({ "name": mcp_tool_name.as_str(), "arguments": arguments });
        if let Value::Object(map) = &mut params {
            if let Some(token) = progress_token.as_deref() {
                map.insert(
                    "progress_token".to_string(),
                    Value::String(token.to_string()),
                );
            }
            if let Some(job_id) = mcp_job_id.as_deref() {
                map.insert("job_id".to_string(), Value::String(job_id.to_string()));
            }
            map.insert(
                "_meta".to_string(),
                json!({
                    "handshake": {
                        "schema_version": HTC_SCHEMA_VERSION,
                        "trace_id": ctx.trace_id.to_string(),
                        "tool_call_id": tool_call_id.to_string(),
                        "session_id": ctx.session_id,
                        "actor": default_actor_payload(),
                        "tool_id": tool_id,
                        "tool_version": tool_version,
                        "transport": "mcp",
                        "side_effect": side_effect,
                        "idempotency": idempotency,
                        "idempotency_key": idempotency_key,
                        "args_ref": args_ref,
                        "args_hash": args_hash,
                        "dry_run": ctx.access_mode != AccessMode::ApplyScoped,
                        "required_capabilities": &required_caps,
                        "redaction_applied": !args_redaction_logs.is_empty(),
                    }
                }),
            );
        }

        let call = match request_id {
            Some(id) => self
                .inner
                .send_request_with_id(id, "tools/call", Some(params), Some(meta))?,
            None => self.inner.send_request("tools/call", Some(params), Some(meta))?,
        };

        let result = match tokio::time::timeout(self.gate.request_timeout, call).await {
            Ok(result) => result,
            Err(_) => {
                let timeout_payload = json!({"timeout_ms": self.gate.request_timeout.as_millis()});
                let _ = fr_events::record_gate_decision(
                    Arc::clone(&self.flight_recorder),
                    &ctx,
                    &self.server_id,
                    Some(tool_id.as_str()),
                    "timeout",
                    "request_timeout",
                    timeout_payload,
                );

                if let Some(token) = progress_token.as_deref() {
                    match self.progress_bindings.lock() {
                        Ok(mut guard) => {
                            guard.remove(token);
                        }
                        Err(poisoned) => {
                            poisoned.into_inner().remove(token);
                        }
                    }
                }

                let ended_at_utc = Utc::now();
                let duration_ms = started_at_instant.elapsed().as_millis();

                let response_envelope = json!({
                    "schema_version": HTC_SCHEMA_VERSION,
                    "tool_call_id": tool_call_id.to_string(),
                    "trace_id": ctx.trace_id.to_string(),
                    "ok": false,
                    "error": htc_error_object(
                        "request_timeout",
                        "timeout",
                        Some(format!("request timed out after {:?}", self.gate.request_timeout)),
                        Some(true),
                        None,
                    ),
                    "timing": htc_timing(started_at_utc, ended_at_utc),
                    "resources": default_htc_resources(),
                });
                let _ = schema::validate_instance(htc_schema(), &response_envelope);

                let _ = fr_events::record_tool_result(
                    Arc::clone(&self.flight_recorder),
                    &ctx,
                    &self.server_id,
                    mcp_tool_name.as_str(),
                    tool_call_id,
                    &tool_id,
                    tool_version.as_str(),
                    required_caps.first().map(|s| s.as_str()),
                    "timeout",
                    Some(duration_ms),
                    Some("timeout"),
                    None,
                    None,
                );

                let payload = json!({
                    "type": "tool_call",
                    "trace_id": ctx.trace_id.to_string(),
                    "tool_call_id": tool_call_id.to_string(),
                    "tool_id": tool_id,
                    "tool_version": tool_version,
                    "transport": "mcp",
                    "side_effect": side_effect,
                    "idempotency": idempotency,
                    "idempotency_key": idempotency_key,
                    "actor": default_actor_payload(),
                    "ok": false,
                    "args_ref": args_ref,
                    "args_hash": args_hash,
                    "error": {
                        "code": "request_timeout",
                        "kind": "timeout",
                        "message": format!("request timed out after {:?}", self.gate.request_timeout),
                        "retryable": true,
                    },
                    "timing": htc_timing(started_at_utc, ended_at_utc),
                    "capability_ids": &required_caps,
                });
                let mut event = FlightRecorderEvent::new(
                    FlightRecorderEventType::ToolCall,
                    FlightRecorderActor::Agent,
                    ctx.trace_id,
                    payload,
                );
                if let Some(job_id) = ctx.job_id {
                    event = event.with_job_id(job_id.to_string());
                }
                if let Some(workflow_run_id) = ctx.workflow_run_id.as_deref() {
                    event = event.with_workflow_id(workflow_run_id.to_string());
                }
                self.flight_recorder
                    .record_event(event)
                    .await
                    .map_err(|e| McpError::FlightRecorder(e.to_string()))?;

                return Err(McpError::Timeout(format!(
                    "request timed out after {:?}",
                    self.gate.request_timeout
                )));
            }
        };

        let ended_at_utc = Utc::now();
        let duration_ms = started_at_instant.elapsed().as_millis();

        match result {
            Ok(tool_result) => {
                let (result_redacted, _) = redactor.redact_value(
                    &tool_result,
                    RedactionMode::SafeDefault,
                    "tool_gate/mcp/result",
                );

                let (result_handle, result_hash) =
                    if let Some(conn) = self.flight_recorder.duckdb_connection() {
                        let conn = conn.lock().map_err(|_| {
                            McpError::FlightRecorder("duckdb connection lock error".to_string())
                        })?;
                        store_tool_payload_redacted(&conn, tool_call_id, "result", &result_redacted)
                            .map_err(|e| McpError::FlightRecorder(e.to_string()))?
                    } else {
                        let artifact_path =
                            format!("data/flight_recorder/tool_payloads/{tool_call_id}/result.json");
                        (
                            ArtifactHandle::new(tool_call_id, artifact_path),
                            canonical_json_sha256_hex(&result_redacted),
                        )
                    };
                let result_ref = result_handle.canonical_id();

                let result_inline = if json_encoded_len(&result_redacted) <= HTC_MAX_PAYLOAD_BYTES {
                    Some(result_redacted.clone())
                } else {
                    None
                };

                let response_envelope = json!({
                    "schema_version": HTC_SCHEMA_VERSION,
                    "tool_call_id": tool_call_id.to_string(),
                    "trace_id": ctx.trace_id.to_string(),
                    "ok": true,
                    "result": result_inline,
                    "result_ref": result_ref,
                    "error": null,
                    "timing": htc_timing(started_at_utc, ended_at_utc),
                    "resources": default_htc_resources(),
                });
                schema::validate_instance(htc_schema(), &response_envelope).map_err(|e| {
                    McpError::SchemaValidation {
                        details: format!("{HTC_VALIDATION_ERROR_CODE}: response envelope invalid\n{e}"),
                    }
                })?;

                fr_events::record_tool_result(
                    Arc::clone(&self.flight_recorder),
                    &ctx,
                    &self.server_id,
                    mcp_tool_name.as_str(),
                    tool_call_id,
                    &tool_id,
                    tool_version.as_str(),
                    required_caps.first().map(|s| s.as_str()),
                    "success",
                    Some(duration_ms),
                    None,
                    Some(result_ref.as_str()),
                    Some(result_hash.as_str()),
                )?;

                let payload = json!({
                    "type": "tool_call",
                    "trace_id": ctx.trace_id.to_string(),
                    "tool_call_id": tool_call_id.to_string(),
                    "tool_id": tool_id,
                    "tool_version": tool_version,
                    "transport": "mcp",
                    "side_effect": side_effect,
                    "idempotency": idempotency,
                    "idempotency_key": idempotency_key,
                    "actor": default_actor_payload(),
                    "ok": true,
                    "args_ref": args_ref,
                    "args_hash": args_hash,
                    "result_ref": result_ref,
                    "result_hash": result_hash,
                    "timing": htc_timing(started_at_utc, ended_at_utc),
                    "capability_ids": &required_caps,
                });
                let mut event = FlightRecorderEvent::new(
                    FlightRecorderEventType::ToolCall,
                    FlightRecorderActor::Agent,
                    ctx.trace_id,
                    payload,
                );
                if let Some(job_id) = ctx.job_id {
                    event = event.with_job_id(job_id.to_string());
                }
                if let Some(workflow_run_id) = ctx.workflow_run_id.as_deref() {
                    event = event.with_workflow_id(workflow_run_id.to_string());
                }
                self.flight_recorder
                    .record_event(event)
                    .await
                    .map_err(|e| McpError::FlightRecorder(e.to_string()))?;

                Ok(result_redacted)
            }
            Err(err) => {
                let response_envelope = json!({
                    "schema_version": HTC_SCHEMA_VERSION,
                    "tool_call_id": tool_call_id.to_string(),
                    "trace_id": ctx.trace_id.to_string(),
                    "ok": false,
                    "error": htc_error_object(
                        "mcp_error",
                        "transport",
                        Some(err.to_string()),
                        Some(false),
                        None,
                    ),
                    "timing": htc_timing(started_at_utc, ended_at_utc),
                    "resources": default_htc_resources(),
                });
                let _ = schema::validate_instance(htc_schema(), &response_envelope);

                let _ = fr_events::record_tool_result(
                    Arc::clone(&self.flight_recorder),
                    &ctx,
                    &self.server_id,
                    mcp_tool_name.as_str(),
                    tool_call_id,
                    &tool_id,
                    tool_version.as_str(),
                    required_caps.first().map(|s| s.as_str()),
                    "error",
                    Some(duration_ms),
                    Some("mcp_error"),
                    None,
                    None,
                );

                let payload = json!({
                    "type": "tool_call",
                    "trace_id": ctx.trace_id.to_string(),
                    "tool_call_id": tool_call_id.to_string(),
                    "tool_id": tool_id,
                    "tool_version": tool_version,
                    "transport": "mcp",
                    "side_effect": side_effect,
                    "idempotency": idempotency,
                    "idempotency_key": idempotency_key,
                    "actor": default_actor_payload(),
                    "ok": false,
                    "args_ref": args_ref,
                    "args_hash": args_hash,
                    "error": {
                        "code": "mcp_error",
                        "kind": "transport",
                        "message": err.to_string(),
                        "retryable": false,
                    },
                    "timing": htc_timing(started_at_utc, ended_at_utc),
                    "capability_ids": &required_caps,
                });
                let mut event = FlightRecorderEvent::new(
                    FlightRecorderEventType::ToolCall,
                    FlightRecorderActor::Agent,
                    ctx.trace_id,
                    payload,
                );
                if let Some(job_id) = ctx.job_id {
                    event = event.with_job_id(job_id.to_string());
                }
                if let Some(workflow_run_id) = ctx.workflow_run_id.as_deref() {
                    event = event.with_workflow_id(workflow_run_id.to_string());
                }
                self.flight_recorder
                    .record_event(event)
                    .await
                    .map_err(|e| McpError::FlightRecorder(e.to_string()))?;

                Err(err)
            }
        }
    }

    pub async fn tools_call(
        &self,
        ctx: McpContext,
        tool_name: &str,
        arguments: Value,
    ) -> McpResult<Value> {
        self.tools_call_htc(ctx, tool_name, arguments).await
    }

    pub fn resolve_ref_uri(&self, ctx: &McpContext, uri: &str) -> McpResult<Vec<u8>> {
        let path_str = strip_ref_scheme(uri)?;
        let canonical = security::canonicalize_under_roots(path_str, &ctx.allowed_roots)?;
        let bytes = std::fs::read(&canonical).map_err(|e| {
            McpError::Protocol(format!("failed to read ref uri target {}: {e}", canonical.display()))
        })?;

        self.inner.send_notification(
            "notifications/resource_released",
            Some(json!({ "uri": uri })),
        )?;

        Ok(bytes)
    }
}
