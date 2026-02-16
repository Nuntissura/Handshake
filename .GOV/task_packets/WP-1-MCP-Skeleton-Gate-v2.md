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
  - src/backend/handshake_core/src/** (new MCP client + gate modules; workflow/job plumbing as needed)
  - src/backend/handshake_core/tests/** (new/updated tests for MCP gate behavior)
  - app/** (ONLY if required to surface existing Flight Recorder rows/events; avoid UX scope creep)
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
      - `trace_id: String` (required for tool calls; source: AI Job)
      - `task_id: Option<String>`
      - `session_id: Option<String>`
      - `capability_profile_id: Option<String>`
      - `human_consent_obtained: bool`
      - `agentic_mode_enabled: bool`
      - `sampling_context: SamplingContext` (see security plan)
      - `allowed_tools: Vec<String>` (session-scoped allowlist)
      - `granted_capabilities: Vec<String>` (session/job scoped)
    - `enum SamplingContext { None, SamplingCreateMessage }`

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
    - `payload.trace_id` = `ctx.trace_id` (always set for tool calls/results).
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
  - Gate records `mcp.tool_call` + `mcp.tool_result` into `fr_events`, correlated by `fr_events.job_id` and `payload.trace_id`.
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
  - `fr_events` contains `mcp.tool_call` + `mcp.tool_result` and at least one `logging/message`-derived row with correlation (job_id + trace_id).
  - Symlink/path traversal attempt is blocked (spec anchor 11.3.7.1) with explicit error + FR decision event.
  - Sampling/createMessage is denied by default and cannot trigger tool side effects (spec anchor 11.3.7.2).

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `path/to/file`
- **Start**: <line>
- **End**: <line>
- **Line Delta**: <adds - dels>
- **Pre-SHA1**: `<hash>`
- **Post-SHA1**: `<hash>`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress (BOOTSTRAP / claim)
- What changed in this update: Ran `just pre-work WP-1-MCP-Skeleton-Gate-v2`; claimed CODER_MODEL + CODER_REASONING_STRENGTH.
- Next step / handoff hint: Draft `## SKELETON` (docs-only) + make skeleton checkpoint commit; STOP for approval.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `path/to/file:line`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-MCP-Skeleton-Gate-v2/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

- COMMAND: `just pre-work WP-1-MCP-Skeleton-Gate-v2`
  - EXIT_CODE: `0`
  - PROOF_LINES:
    - Checking Phase Gate for WP-1-MCP-Skeleton-Gate-v2...
    - Pre-work validation PASSED

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
