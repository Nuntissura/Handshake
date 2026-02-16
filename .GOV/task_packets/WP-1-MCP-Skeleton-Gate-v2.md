# Task Packet: WP-1-MCP-Skeleton-Gate-v2

## METADATA
- TASK_ID: WP-1-MCP-Skeleton-Gate-v2
- WP_ID: WP-1-MCP-Skeleton-Gate-v2
- BASE_WP_ID: WP-1-MCP-Skeleton-Gate (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-15T23:41:52.974Z
- MERGE_BASE_SHA: 0f7cfda43997ab72baf7b0150ced57d4c2600a06
- REQUESTOR: Operator (ilja)
- AGENT_ID: codex-cli (gpt-5.2)
- ROLE: Orchestrator
- AGENTIC_MODE: YES
- ORCHESTRATOR_MODEL: gpt-5.2
- ORCHESTRATION_STARTED_AT_UTC: 2026-02-15T23:41:52.974Z
- CODER_MODEL: gpt-5.2
- CODER_REASONING_STRENGTH: EXTRA_HIGH (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja160220260031
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-MCP-Skeleton-Gate-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement the MVP MCP client + Rust Gate interceptor (middleware) so all MCP traffic is capability/consent-gated and traceable, with at least one stubbed end-to-end tool call exercised by tests.
- Why: Unblocks WP-1-MCP-End-to-End-v2 and Phase 1/2 MCP-based integrations by making MCP calls auditable (Flight Recorder) and safe (Gate enforcement).
- IN_SCOPE_PATHS:
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/mcp/client.rs
  - src/backend/handshake_core/src/mcp/discovery.rs
  - src/backend/handshake_core/src/mcp/errors.rs
  - src/backend/handshake_core/src/mcp/fr_events.rs
  - src/backend/handshake_core/src/mcp/gate.rs
  - src/backend/handshake_core/src/mcp/jsonrpc.rs
  - src/backend/handshake_core/src/mcp/mod.rs
  - src/backend/handshake_core/src/mcp/schema.rs
  - src/backend/handshake_core/src/mcp/security.rs
  - src/backend/handshake_core/src/mcp/transport/duplex.rs
  - src/backend/handshake_core/src/mcp/transport/mod.rs
  - src/backend/handshake_core/src/mcp/transport/stdio.rs
  - src/backend/handshake_core/tests/mcp_gate_tests.rs
- OUT_OF_SCOPE:
  - Docling ingestion implementation (Phase 2; this WP only provides the MCP plumbing/gate needed to support it)
  - Full reference-based binary protocol (Target 2) beyond what is required for basic conformance tests
  - Multi-user sync / CRDT / cloud-only MCP assumptions

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-MCP-Skeleton-Gate-v2
# ...task-specific commands...
just cargo-clean
just post-work WP-1-MCP-Skeleton-Gate-v2 --range 0f7cfda43997ab72baf7b0150ced57d4c2600a06..HEAD
```

### DONE_MEANS
- MCP client transport exists (at least one transport) and can connect to a local stub MCP server in tests.
- Rust Gate interceptor wraps MCP traffic and enforces: capability scope + human-in-the-loop consent where required (deny/timeout paths are explicit).
- MCP `tools/call` request/response and `logging/message` are recorded into Flight Recorder with correlation fields (job_id and trace_id or paired event linkage).
- Security hardening implemented for MCP file/resource access per spec red-team guidance (no naive prefix checks; canonicalization/no-follow where applicable).
- `just pre-work WP-1-MCP-Skeleton-Gate-v2` and `just post-work WP-1-MCP-Skeleton-Gate-v2 --range 0f7cfda43997ab72baf7b0150ced57d4c2600a06..HEAD` both PASS.

### ROLLBACK_HINT
```bash
git revert <commit-sha>  # revert WP commit(s) on feat/WP-1-MCP-Skeleton-Gate-v2
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.126.md (recorded_at: 2026-02-15T23:41:52.974Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.126.md 11.3 Auth/Session/MCP Primitives
  - Handshake_Master_Spec_v02.126.md 11.3.2 Implementation Target 1: The Rust 'Gate' Interceptor (Middleware Design)
  - Handshake_Master_Spec_v02.126.md 11.3.6 Implementation Target 5: Logging Sink (MCP logging/message -> DuckDB Flight Recorder)
  - Handshake_Master_Spec_v02.126.md 11.3.7 Red Team Security Audit (Symlinks + Sampling)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packets:
  - `.GOV/task_packets/WP-1-MCP-Skeleton-Gate.md` (historical; validator verdict recorded there: FAIL due to packet incompleteness / outdated pointers)
- Preserved requirements:
  - Implement MCP transport + Gate middleware (capability/consent/logging) per Master Spec Main Body.
- Changes in v2 packet:
  - Re-anchor to current Master Spec `Handshake_Master_Spec_v02.126.md` and include the required refinement/signature/prepare gates.
  - Add explicit security hardening scope (symlink + sampling considerations) and measurable DONE_MEANS.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.126.md (11.3.x; 11.5 for FR event shapes)
  - .GOV/task_packets/WP-1-MCP-Skeleton-Gate.md (prior packet)
  - src/backend/handshake_core/src/terminal/guards.rs (consent gating analog)
  - src/backend/handshake_core/src/llm/guard.rs (consent artifacts + policy enforcement analog)
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/workflows.rs
- SEARCH_TERMS:
  - "human_consent_obtained"
  - "FlightRecorder"
  - "Capability"
  - "DuckDB"
  - "jsonrpc"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-MCP-Skeleton-Gate-v2
  cd src/backend/handshake_core; cargo fmt
  cd src/backend/handshake_core; cargo clippy --all-targets --all-features
  cd src/backend/handshake_core; cargo test
  pnpm -C app run lint
  pnpm -C app test
  just cargo-clean
  just post-work WP-1-MCP-Skeleton-Gate-v2 --range 0f7cfda43997ab72baf7b0150ced57d4c2600a06..HEAD
  ```
- RISK_MAP:
  - "MCP gate bypass" -> "capability/consent enforcement broken; unsafe tool execution"
  - "insufficient traceability" -> "cannot debug/validate MCP usage; violates spec traceability invariants"
  - "symlink/path traversal" -> "exfiltration of host files via MCP roots/resources"

## SKELETON
SKELETON APPROVED
- Module layout (proposed; all under `src/backend/handshake_core/src/`):
  - `mcp/` (new): MCP client + JSON-RPC types + transport(s) + discovery + schema validation helpers.
    - `mcp/jsonrpc.rs`: `JsonRpcMessage` + `JsonRpcRequest/Notification/Response` + `JsonRpcError`.
    - `mcp/transport/mod.rs`: `McpTransport` trait + `TransportKind` + reconnection contract.
    - `mcp/transport/stdio.rs`: `StdioTransport` (production); spawns sidecar, reads/writes JSON-RPC lines.
    - `mcp/client.rs`: `McpClient` trait + `JsonRpcMcpClient<T: McpTransport>`.
    - `mcp/discovery.rs`: `McpToolDescriptor`/`McpResourceDescriptor` + caching.
    - `mcp/schema.rs`: JSON Schema validation helpers (reuse `jsonschema` patterns from `capability_registry_workflow.rs`).
    - `mcp/errors.rs`: `McpError` (transport/protocol/schema/security/capability/consent).
    - `mcp/fr_events.rs`: shared helper to write MCP rows into `fr_events` (mirror `mex/runtime.rs` insertion pattern).
    - `mcp/gate.rs`: Gate interceptor trait + default gate implementation + wrapper around client/transport.
  - `jobs.rs` / `workflows.rs` (existing): add MCP job plumbing ONLY after skeleton approval (Implementation phase).
  - `tests/` (new): integration tests using a local stub MCP server (test-only; never built into production).

- Core types / contracts (proposed; no logic yet):
  - JSON-RPC envelope (per spec anchor 11.3.2):
    - `enum JsonRpcMessage { Request(JsonRpcRequest), Notification(JsonRpcNotification), Response(JsonRpcResponse) }`
    - `struct JsonRpcRequest { jsonrpc: String, id: JsonRpcId, method: String, params: Option<serde_json::Value> }`
    - `struct JsonRpcNotification { jsonrpc: String, method: String, params: Option<serde_json::Value> }`
    - `struct JsonRpcResponse { jsonrpc: String, id: JsonRpcId, result: Option<serde_json::Value>, error: Option<JsonRpcError> }`
    - `struct JsonRpcError { code: i64, message: String, data: Option<serde_json::Value> }`
    - `enum JsonRpcId { Number(i64), String(String) }` (reject null IDs for requests that expect responses)

  - Transport abstraction (outbound + inbound interception requires a bidirectional contract):
    - `#[async_trait] trait McpTransport {`
      - `fn kind(&self) -> TransportKind`
      - `async fn send(&self, msg: &JsonRpcMessage) -> Result<(), McpError>`
      - `async fn recv(&self) -> Result<JsonRpcMessage, McpError>`
      - `async fn reconnect(&self) -> Result<(), McpError>` (exponential backoff required by spec)
      - `async fn close(&self)`
      - `fn server_id(&self) -> &str`
    - `enum TransportKind { Stdio, SseHttp }` (MVP: implement `Stdio` first; `SseHttp` future)

  - Client API:
    - `#[async_trait] trait McpClient {`
      - `async fn initialize(&self, ctx: &McpContext) -> Result<McpInitializeResult, McpError>`
      - `async fn list_tools(&self, ctx: &McpContext) -> Result<Vec<McpToolDescriptor>, McpError>`
      - `async fn call_tool(&self, ctx: &McpContext, req: McpToolCall) -> Result<McpToolResult, McpError>`
      - `async fn list_resources(&self, ctx: &McpContext) -> Result<Vec<McpResourceDescriptor>, McpError>` (optional in MVP; allow stubbing)
      - `async fn read_resource(&self, ctx: &McpContext, uri: &McpUri) -> Result<McpResourceContents, McpError>` (only if required for tests)
    - `struct McpToolCall { server_id: String, tool_name: String, arguments: serde_json::Value }`
    - `struct McpToolResult { content: serde_json::Value, is_error: bool, redactions_applied: bool }`
    - `struct McpInitializeResult { negotiated: McpNegotiatedCapabilities }`
    - `struct McpNegotiatedCapabilities { supports_roots: bool, supports_sampling: bool, supports_progress: bool, ... }`

  - Gate interceptor (middleware) (per spec anchor 11.3.2):
    - `#[async_trait] trait McpInterceptor {`
      - `async fn on_outbound(&self, ctx: &McpContext, msg: &mut JsonRpcMessage) -> Result<(), McpError>`
      - `async fn on_inbound(&self, ctx: &McpContext, msg: &mut JsonRpcMessage) -> Result<(), McpError>`
      - `async fn on_response(&self, ctx: &McpContext, req: &JsonRpcRequest, resp: &mut JsonRpcResponse) -> Result<(), McpError>`
    - `struct GateLayer<T> { inner: T, interceptor: Arc<dyn McpInterceptor> }` (wraps `McpTransport` or `McpClient`)
    - `struct McpContext {`
      - `server_id: String`
      - `job_id: Option<String>`
      - `workflow_run_id: Option<String>`
      - `trace_id: uuid::Uuid` (required for tool calls; source: AI Job; serialize to string only at JSON/DB boundaries)
      - `task_id: Option<String>`
      - `session_id: Option<String>`
      - `capability_profile_id: Option<String>`
      - `human_consent_obtained: bool`
      - `agentic_mode_enabled: bool`
      - `sampling_context: SamplingContext` (see security plan)
      - `allowed_tools: Vec<String>` (session-scoped allowlist)
      - `granted_capabilities: Vec<String>` (session/job scoped)
    - `enum SamplingContext { None, SamplingCreateMessage }`

  - Pending requests + cancellation (protocol nuance required by spec 11.3.2.3):
    - Goal: prevent "zombie" requests and ensure Stop/timeout actively cancels server work.
    - `struct PendingRequest {`
      - `id: JsonRpcId`
      - `method: String`
      - `started_at: std::time::Instant`
      - `timeout: std::time::Duration`
      - `job_id: Option<String>`
      - `trace_id: uuid::Uuid`
    - `type PendingRequests = dashmap::DashMap<JsonRpcId, PendingRequest>` (or equivalent concurrent map)
    - Ownership/location:
      - Pending map + timeout enforcement live in the Gate middleware wrapper (e.g., `GateLayer` / `GatedMcpClient`), not in the UI.
      - Key is always `JsonRpcId` (the request id).
    - Timeout policy (configurable):
      - `struct TimeoutPolicy { default_tool_timeout: Duration, deep_research_timeout: Duration }`
      - Gate selects timeout per tool/method (MVP: default for all tools; extend later).
    - Cancellation trigger points:
      - User clicks "Stop" -> the Future associated with the tool call is dropped -> Drop guard triggers cancellation.
      - Request exceeds timeout -> Gate proactively cancels + returns a timeout error to the UI.
    - Cancellation transport message (outbound notification):
      - JSON-RPC notification: `method = "notifications/cancelled"` with the relevant `requestId` (JsonRpcId).
    - Implementation constraint to plan for:
      - Drop cannot `await`; cancellation must be sent via a non-blocking mechanism (e.g., spawn a tokio task or signal a transport writer loop) so it is "immediate" from the caller's perspective.

- Capability + consent decision points (explicit; default-deny where ambiguous):
  - Outbound (`tools/call`):
    - Check `allowed_tools` contains `tool_name` (deny -> JSON-RPC error; record decision).
    - Resolve required capabilities for `(server_id, tool_name)` using a host-side mapping (TBD exact source; see Open Questions).
    - Enforce capability registry (mirror `terminal/guards.rs` + `CapabilityRegistry::enforce_can_perform` patterns).
    - If tool requires human confirmation (per policy) and `human_consent_obtained=false`: pause/timeout path is explicit (MVP: return `McpError::ConsentRequired` + record `mcp.gate.decision`).
    - Validate `arguments` against the tool's JSON Schema before sending (see schema plan).

  - Inbound (`sampling/createMessage`):
    - Treat as untrusted; deny by default unless `agentic_mode_enabled=true` AND an explicit consent artifact is present (mirror `llm/guard.rs` pattern).
    - Never allow tool side-effects while `sampling_context=SamplingCreateMessage` (if a response payload "looks like a tool call", treat as text-only; do not execute).

  - Inbound method/capability firewall:
    - If server attempts to use undeclared/unsupported capability or method (e.g., `roots/list` without negotiation): reject with JSON-RPC `-32601` and record `mcp.gate.decision`.

  - Response analysis (DLP/redaction):
    - Scan tool results for sensitive patterns; redact before returning to UI/LLM context; record `redactions_applied=true` in the paired FR event payload.

- Flight Recorder event plan (DuckDB `fr_events`) (per spec anchor 11.3.6 + existing `mex/runtime.rs` pattern):
  - All MCP rows use `FlightRecorder::duckdb_connection()` and insert into `fr_events` (no new DB files).
  - `event_kind` mapping (minimum set for MVP):
    - `mcp.tool_call` (before outbound `tools/call` send)
    - `mcp.tool_result` (after response/error)
    - `mcp.progress` (from MCP progress notifications, if observed)
    - `mcp.logging` OR `fields.event_kind` (from `logging/message`, per spec contract)
    - `mcp.gate.decision` (allow/deny/timeout/consent-required decisions)
  - Correlation fields:
    - `fr_events.job_id` = `ctx.job_id` (string) when available (required for DONE_MEANS tests).
    - `payload.trace_id` = `ctx.trace_id.to_string()` (always set for tool calls/results).
    - `payload.workflow_run_id` mirrors `ctx.workflow_run_id` when available.
  - Payload keys for tool call/result events MUST include the conformance-required keys (mirror `mex/conformance.rs`):
    - `tool_name`, `tool_version`, `inputs`, `outputs`, `status`, `duration_ms`, `error_code`, `job_id`, `workflow_run_id`, `trace_id`, `capability_id`
    - plus MCP-specific fields: `server_id`, `method`, `tool_arguments_schema_id` (optional), `capabilities` (list), `consent_required` (bool), `decision` (for gate events)
  - `logging/message` handling:
    - Persist full `params` JSON as `payload`.
    - `event_kind` = `params.fields.event_kind` if present else `mcp.logging`.
    - `source` = `params.fields.server_id` (or transport `server_id` fallback).
    - `level`/`message` map from `params.level`/`params.message`.

- Security plan (paths/URIs + sampling hardening):
  - Paths/URIs:
    - Represent MCP resource URIs as a parsed `McpUri` type; do not treat server-provided `file://...` as a host path string.
    - Resolve/authorize file access ONLY through a host-owned `AllowedRoots` mapping keyed by `(server_id, root_id)` (treat URI as a key into that mapping).
    - When mapping to a host path, enforce canonicalization + starts_with allowed root (mirror `terminal/guards.rs::validate_cwd` pattern).
    - For file open/read: best-effort no-follow enforcement (Unix `O_NOFOLLOW` where available) + deny symlinks (explicit test coverage required by DONE_MEANS).
  - Sampling hardening (spec anchor 11.3.7):
    - Fence untrusted server content; never concatenate untrusted content into system prompts.
    - Deny side-effects during sampling contexts; require explicit human approval (no silent auto-approve).

- Open questions / decisions needed (blockers for Implementation if unresolved):
  1. Canonical source for `(server_id, tool_name) -> required capabilities + consent_required` mapping (config file vs capability registry extension vs hardcoded MVP for stub).
  2. What is the MVP consent artifact for MCP (env-based receipt like `llm/guard.rs` vs UI modal receipt stored in DB / mailbox)?
  3. Which MCP message field should carry Handshake metadata (`trace_id`, `job_id`) so servers can correlate (custom `params._handshake` envelope vs existing MCP `meta`/`context` field if defined)?
  4. DLP/redaction: confirm which redaction utility to reuse (PatternRedactor/SecretRedactor) and which patterns are mandatory for MCP tool results.

- Notes:
  - Stub server is test-only; production builds must not include it (spec stub policy).
  - No product logic changes occur until "SKELETON APPROVED".

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: Rust Host (trusted) <-> MCP server process/transport (untrusted) <-> filesystem/resources (high risk)
- SERVER_SOURCES_OF_TRUTH:
  - Capability registry (`CapabilityRegistry`) + host-owned `(server_id, tool_name)` policy mapping (never trust server to self-declare required capabilities).
  - Host-owned consent artifacts/receipts (never trust server to claim consent).
  - Host-owned allowed-roots mapping for any file/resource resolution (never trust server-provided `file://` as a host path).
- REQUIRED_PROVENANCE_FIELDS:
  - job_id, trace_id, workflow_run_id, session_id, task_id
  - server_id, tool_name, transport_kind
  - capability_profile_id, granted_capabilities, enforced_capability_id(s)
  - consent_required + consent_decision (allow/deny/timeout)
  - redactions_applied (DLP)
- VERIFICATION_PLAN:
  - Gate enforces allowlist + capability registry + consent policy BEFORE sending `tools/call`.
  - Gate validates tool params against tool JSON Schema; on failure returns explicit error and records `mcp.gate.decision` + error into FR.
  - Gate records `tool.call` + `tool.result` into `fr_events`, correlated by `fr_events.job_id` and `payload.trace_id`.
  - Gate rejects inbound requests for undeclared capabilities/methods with JSON-RPC `-32601` and records decision.
  - Gate hardens any path/URI handling via canonicalization + bounds + no-follow policies; tests cover escape + symlink attempt.
- ERROR_TAXONOMY_PLAN:
  - `SchemaValidationFailed` (tool args invalid)
  - `CapabilityDenied` (policy/capability registry)
  - `ConsentRequired` / `ConsentDenied` / `ConsentTimeout`
  - `MethodNotAllowed` (inbound undeclared capability -> -32601)
  - `SecurityViolation` (path traversal/symlink)
  - `TransportError` / `ProtocolError`
  - `DlpRedacted` (non-fatal; annotate result + FR payload)
- UI_GUARDRAILS:
  - MVP: no UI changes expected.
  - If consent UI is required later: modal must show exact tool + args + server_id; deny is default; no silent approvals.
- VALIDATOR_ASSERTIONS:
  - MCP tool call tests prove allow/deny/timeout paths with explicit errors.
  - `fr_events` contains `tool.call` + `tool.result` and at least one `mcp.logging` row with correlation (job_id + trace_id).
  - Symlink/path traversal attempt is blocked (spec anchor 11.3.7.1) with explicit error + FR decision event.
  - Sampling/createMessage is denied by default and cannot trigger tool side effects (spec anchor 11.3.7.2).

## IMPLEMENTATION
- Added `handshake_core::mcp` module with JSON-RPC types + transport traits (`stdio` + in-memory duplex for tests), plus discovery helpers for `tools/list` and `resources/list`.
- Implemented `McpClient` with pending request tracking keyed by `JsonRpcId` and a proactive cancellation path (send `notifications/cancelled` when a pending call is dropped/timed out).
- Implemented `GatedMcpClient` that wraps tool calls with allowlist + schema validation + capability enforcement + consent policy, plus hardening for path/root bounds and sampling/createMessage firewalling.
- Flight Recorder sink inserts `tool.call`, `tool.result`, `mcp.logging`, and `mcp.gate.decision` rows into DuckDB `fr_events` with `job_id` + `trace_id` correlation (`trace_id` is `uuid::Uuid` in Rust).
- Added end-to-end tests using a stub MCP server over `DuplexTransport` to exercise tool calls, logging, allow/deny/timeout, schema failures, cancellation, and path/symlink hardening.

## HYGIENE
- Formatting/lint/tests recorded in `## EVIDENCE` (focused `cargo test -j 1 --test mcp_gate_tests`, full `cargo test -j 1`, `cargo clippy --all-targets --all-features -j 1`).
- Coverage (HIGH RISK_TIER): `cargo tarpaulin` recorded with `--include-files src/mcp/* src/mcp/transport/*` (new MCP code coverage 86.03%).
- Final gate sequence pending: `just cargo-clean` + `just post-work WP-1-MCP-Skeleton-Gate-v2 --range 0f7cfda43997ab72baf7b0150ced57d4c2600a06..HEAD`.

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)

- **Target File**: `src/backend/handshake_core/src/lib.rs`
- **Start**: 1
- **End**: 37
- **Line Delta**: 1
- **Pre-SHA1**: `f1ad10c086150071cfca8a4d2b3fb67e992f0a75`
- **Post-SHA1**: `79723ac3493861294b0325385fc8061ae89b20cf`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mcp/client.rs`
- **Start**: 1
- **End**: 221
- **Line Delta**: 221
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `e9dda26510e421e9b4307024c9af7cff19dd413b`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mcp/discovery.rs`
- **Start**: 1
- **End**: 36
- **Line Delta**: 36
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `40bb1b8f3d4df8053eb99b3aa3610a84c59fc0d9`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mcp/errors.rs`
- **Start**: 1
- **End**: 33
- **Line Delta**: 33
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `7aeeda2e8ecb3134b484ac4dac23763e98cf2026`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mcp/fr_events.rs`
- **Start**: 1
- **End**: 276
- **Line Delta**: 276
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `a94760b4a8ab0fb3526c262e62744b2a2168c820`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mcp/gate.rs`
- **Start**: 1
- **End**: 525
- **Line Delta**: 525
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `449857e2857757ab197ecf142c25dc84f7114187`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mcp/jsonrpc.rs`
- **Start**: 1
- **End**: 128
- **Line Delta**: 128
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `7534e921507683a4809de6940b9863fc23b1bcd9`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mcp/mod.rs`
- **Start**: 1
- **End**: 16
- **Line Delta**: 16
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `3a17accc4ff803f33ac7c1f223377865d5962107`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mcp/schema.rs`
- **Start**: 1
- **End**: 25
- **Line Delta**: 25
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `af8318aed9ac024c21b7cc4b38dee97f9fb6cfa3`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mcp/security.rs`
- **Start**: 1
- **End**: 96
- **Line Delta**: 96
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `33d08ece1f0fb2f7021b7704c55e22b5b0f98917`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mcp/transport/duplex.rs`
- **Start**: 1
- **End**: 72
- **Line Delta**: 72
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `53aaef2d47f3244b6717b65a4406ab4ebd07dfd5`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mcp/transport/mod.rs`
- **Start**: 1
- **End**: 41
- **Line Delta**: 41
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `47e00a7ce3d7c590509bc286c6b06c7da88a0824`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/src/mcp/transport/stdio.rs`
- **Start**: 1
- **End**: 100
- **Line Delta**: 100
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `941cb1687f22e1574822a6030990fe2bdf5380b8`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Target File**: `src/backend/handshake_core/tests/mcp_gate_tests.rs`
- **Start**: 1
- **End**: 1066
- **Line Delta**: 1066
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `1a1a283c575618faf88ca930653c996a58657c27`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- Spec Target Resolved: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.126.md

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress (IMPLEMENTATION complete; HYGIENE/gates pending)
- What changed in this update: Raised new MCP module coverage to >=85% and updated deterministic VALIDATION manifests + evidence mapping anchors to current line numbers.
- Next step / handoff hint: Run `just cargo-clean`, run `just post-work WP-1-MCP-Skeleton-Gate-v2 --range 0f7cfda43997ab72baf7b0150ced57d4c2600a06..HEAD`, then hand off to Validator.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`

- REQUIREMENT: "MCP client transport exists (at least one transport) and can connect to a local stub MCP server in tests."
  - EVIDENCE: `src/backend/handshake_core/src/mcp/transport/duplex.rs:8`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_gate_tests.rs:397`

- REQUIREMENT: "Rust Gate interceptor wraps MCP traffic and enforces: capability scope + human-in-the-loop consent where required (deny/timeout paths are explicit)."
  - EVIDENCE: `src/backend/handshake_core/src/mcp/gate.rs:279`
  - EVIDENCE: `src/backend/handshake_core/src/mcp/gate.rs:293`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_gate_tests.rs:632`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_gate_tests.rs:680`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_gate_tests.rs:758`

- REQUIREMENT: "MCP `tools/call` request/response and `logging/message` are recorded into Flight Recorder with correlation fields (job_id and trace_id or paired event linkage)."
  - EVIDENCE: `src/backend/handshake_core/src/mcp/fr_events.rs:79`
  - EVIDENCE: `src/backend/handshake_core/src/mcp/fr_events.rs:153`
  - EVIDENCE: `src/backend/handshake_core/src/mcp/gate.rs:114`
  - EVIDENCE: `src/backend/handshake_core/src/mcp/gate.rs:150`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_gate_tests.rs:374`

- REQUIREMENT: "Handshake_Master_Spec_v02.126.md 11.3.2.3 Handling Protocol Nuances: \"Pending\" States and Cancellation"
  - EVIDENCE: `src/backend/handshake_core/src/mcp/client.rs:183`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_gate_tests.rs:758`

- REQUIREMENT: "Handshake_Master_Spec_v02.126.md 11.3.2.4 The Gate as a Schema Validator"
  - EVIDENCE: `src/backend/handshake_core/src/mcp/gate.rs:411`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_gate_tests.rs:579`

- REQUIREMENT: "Security hardening implemented for MCP file/resource access per spec red-team guidance (no naive prefix checks; canonicalization/no-follow where applicable)."
  - EVIDENCE: `src/backend/handshake_core/src/mcp/security.rs:38`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_gate_tests.rs:817`

- REQUIREMENT: "MCP Gate decisions are recorded into `fr_events` (deny/timeout) for traceability."
  - EVIDENCE: `src/backend/handshake_core/src/mcp/fr_events.rs:247`
  - EVIDENCE: `src/backend/handshake_core/src/mcp/gate.rs:355`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_gate_tests.rs:896`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-MCP-Skeleton-Gate-v2/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

- COMMAND: `just pre-work WP-1-MCP-Skeleton-Gate-v2`
  - EXIT_CODE: 0
  - PROOF_LINES:
    - Checking Phase Gate for WP-1-MCP-Skeleton-Gate-v2...
    - Pre-work validation PASSED

- COMMAND: `cd src/backend/handshake_core; cargo fmt`
  - EXIT_CODE: 0
  - PROOF_LINES:
    - (no output)

- COMMAND: `cd src/backend/handshake_core; cargo clippy --all-targets --all-features -j 1`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-MCP-Skeleton-Gate-v2/cargo-clippy-all-targets-all-features-j1-d0a8aaf.log`
  - LOG_SHA256: `D451C7976B3AF1FC9335CC6901F7B529F85C05575708BBFB8C9977E5EF184FDE`
  - PROOF_LINES:
    - Finished `dev` profile [unoptimized + debuginfo]

- COMMAND: `cd src/backend/handshake_core; cargo test -j 1 --test mcp_gate_tests`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-MCP-Skeleton-Gate-v2/cargo-test-mcp_gate_tests-j1-d0a8aaf.log`
  - LOG_SHA256: `BA4BECC3970D4A5897643C2BBAE66DA09562B021E525F39DE33E0C6DFCEF5012`
  - PROOF_LINES:
    - running 6 tests
    - test result: ok. 6 passed; 0 failed

- COMMAND: `cd src/backend/handshake_core; cargo test -j 1`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-MCP-Skeleton-Gate-v2/cargo-test-j1-d0a8aaf.log`
  - LOG_SHA256: `3E09BCB85DB92364BE5BA31BDCF9DD051F2C69F64BE9CDA2CEECB2D6856B6CBA`
  - PROOF_LINES:
    - Doc-tests handshake_core
    - test result: ok.

- COMMAND: `cd src/backend/handshake_core; cargo test -j 1 --test mcp_gate_tests`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-MCP-Skeleton-Gate-v2/cargo-test-mcp_gate_tests-j1-HEAD-20260216-102628.log`
  - LOG_SHA256: `4F0D5D667FAF6569CBFB47AB454A6E919919FC0D7691D76082C822E0724DA602`
  - PROOF_LINES:
    - running 13 tests
    - test result: ok. 13 passed; 0 failed

- COMMAND: `cd src/backend/handshake_core; cargo tarpaulin --engine Llvm --out Html --output-dir coverage -j 1 --skip-clean --include-files src/mcp/* src/mcp/transport/* --tests --test mcp_gate_tests`
  - EXIT_CODE: 0
  - LOG_PATH: `.handshake/logs/WP-1-MCP-Skeleton-Gate-v2/cargo-tarpaulin-mcp-only-j1-HEAD-20260216-102701.log`
  - LOG_SHA256: `B38BDA99C5B1D631744B30957CC5535F5B478EF6A4DE9190F7AA61181B5A6515`
  - PROOF_LINES:
    - 86.03% coverage, 499/580 lines covered

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
