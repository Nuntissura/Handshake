# Task Packet: WP-1-Unified-Tool-Surface-Contract-v1

## METADATA
- TASK_ID: WP-1-Unified-Tool-Surface-Contract-v1
- WP_ID: WP-1-Unified-Tool-Surface-Contract-v1
- BASE_WP_ID: WP-1-Unified-Tool-Surface-Contract (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-24T03:48:12.798Z
- MERGE_BASE_SHA: 35cd220dbfe573628ce1ab565a6363f0b993a1eb
- REQUESTOR: Operator (ilja)
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- AGENTIC_MODE: YES
- ORCHESTRATOR_MODEL: GPT-5.2 (Codex CLI) (required if AGENTIC_MODE=YES)
- ORCHESTRATION_STARTED_AT_UTC: 2026-02-24T03:48:12.798Z
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja240220260346
- PACKET_FORMAT_VERSION: 2026-02-01

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: DISALLOWED
- OPERATOR_APPROVAL_EVIDENCE: N/A
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Unified-Tool-Surface-Contract-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement the Unified Tool Surface Contract (HTC-1.0) as a single source of truth for tool identity/versioning/schemas and enforce a single Tool Gate path for all tool invocations (local + MCP + MEX + stage_bridge/other), with canonical FR-EVT-007 ToolCallEvent emission.
- Why: Eliminate dual-schema drift and tool-call bypass paths that break determinism, capability/consent enforcement, redaction, and auditability; satisfy Master Spec requirements for HTC envelope validation + conformance tests + MCP binding.
- IN_SCOPE_PATHS:
  - .GOV/refinements/WP-1-Unified-Tool-Surface-Contract-v1.md
  - .GOV/task_packets/WP-1-Unified-Tool-Surface-Contract-v1.md
  - assets/schemas/htc_v1.json
  - src/backend/handshake_core/src/mcp/gate.rs
  - src/backend/handshake_core/src/mcp/fr_events.rs
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/mex/conformance.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/tests/mcp_gate_tests.rs
- OUT_OF_SCOPE:
  - Tool Call Ledger / approvals UX beyond the minimum required to surface Tool Gate decisions deterministically (coordinate with Dev Command Center WP).
  - Phase 2+ Design Studio shell/IA work.
  - Handshake-as-MCP-server (local) beyond what is required to prove unified schema + Tool Gate enforcement.

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Unified-Tool-Surface-Contract-v1
# ...task-specific commands...
just cargo-clean
just post-work WP-1-Unified-Tool-Surface-Contract-v1 --range 35cd220dbfe573628ce1ab565a6363f0b993a1eb..HEAD
```

### DONE_MEANS
- `assets/schemas/htc_v1.json` exists as the schema SSoT; Tool Gate validates every envelope against it. Invalid envelopes are rejected with: `ok=false` + `error.kind="validation"` + `error.code="VAL-HTC-001"`. (Spec 6.0.2.5.1)
- Tool Registry is the single source of truth for tool identity/schemas/side-effects/idempotency/determinism/availability and required capabilities. MCP `tools/list` is generated from the Tool Registry and binds required `_meta.handshake` fields. (Spec 6.0.2.2-6.0.2.3, 11.3.0)
- All tool invocation entrypoints (local + MCP + MEX + stage_bridge/other) route through Tool Gate (no bypass). Conformance tests fail deterministically on any bypass or schema divergence. (Spec 6.0.2.9)
- Every tool invocation emits FR-EVT-007 (ToolCallEvent) with required correlation + identity fields (`trace_id`, `tool_call_id`, `tool_id`, `tool_version`) and artifact-first redacted refs/hashes computed after redaction. (Spec 11.5.2 FR-EVT-007)
- Conformance suite exists and is enforced in CI (schema validation, side_effect verification, payload caps/32KB, capability gating deny-by-default, FR emission, idempotency behavior). (Spec 6.0.2.9)
- `just pre-work WP-1-Unified-Tool-Surface-Contract-v1` and `just post-work WP-1-Unified-Tool-Surface-Contract-v1 --range 35cd220dbfe573628ce1ab565a6363f0b993a1eb..HEAD` both PASS.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.137.md (recorded_at: 2026-02-24T03:48:12.798Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.137.md 6.0.2.2-6.0.2.3 (tool identity/versioning; side_effect/idempotency/determinism; routing rule)
  - Handshake_Master_Spec_v02.137.md 6.0.2.5.1 (HTC-1.0 JSON Schema file `assets/schemas/htc_v1.json`; VAL-HTC-001)
  - Handshake_Master_Spec_v02.137.md 6.0.2.9 Conformance tests (MUST)
  - Handshake_Master_Spec_v02.137.md 11.3.0 Canonical Tool Contract Binding (Normative)
  - Handshake_Master_Spec_v02.137.md 11.5.2 FR-EVT-007 (ToolCallEvent)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packets:
  - `.GOV/task_packets/stubs/WP-1-Unified-Tool-Surface-Contract-v1.md` (stub; non-executable)
- Preserved requirements:
  - Unify tool invocation across transports via a single Tool Registry + Tool Gate + FR event model (no bypass), per Master Spec Main Body anchors in this packet.
- Changes in v1 packet:
  - Activated the stub into an official executable packet (`.GOV/task_packets/`) with signed Technical Refinement and recorded PREPARE.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.137.md (6.0.2.x; 11.3.0; 11.5.2 FR-EVT-007)
  - .GOV/task_packets/stubs/WP-1-Unified-Tool-Surface-Contract-v1.md
  - src/backend/handshake_core/src/mcp/gate.rs
  - src/backend/handshake_core/src/mcp/fr_events.rs
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/mex/conformance.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/tests/mcp_gate_tests.rs
- SEARCH_TERMS:
  - "Unified Tool Surface Contract"
  - "tools/list"
  - "tools/call"
  - "Tool._meta.handshake"
  - "VAL-HTC-001"
  - "FR-EVT-007"
  - "tool_call_id"
  - "idempotency_key"
- RUN_COMMANDS:
  ```bash
  rg -n "tools/list|tools/call|tool_call|tool_version|tool_id|idempotency_key" src/backend/handshake_core/src
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --jobs 1
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test mcp_gate_tests --jobs 1
  ```
- RISK_MAP:
  - "Dual schema drift" -> "bypass paths; incorrect schemas; broken capabilities/consent/audit"
  - "Secret/PII leakage in args/results" -> "improper Flight Recorder persistence; security incident"
  - "Idempotency/retry double-apply" -> "duplicate side effects; non-deterministic outcomes"
  - "Missing correlation fields" -> "cannot audit/trace tool calls across jobs/workflows"

## SKELETON
- Proposed interfaces/types/contracts (target files)
  - `assets/schemas/htc_v1.json`
    - JSON Schema (draft 2020-12) for HTC-1.0 request/response envelopes + standard error object (Spec 6.0.2.5/5.1).
    - 32KB sizing rule enforced in Rust (not expressible in JSON Schema).
  - `src/backend/handshake_core/src/flight_recorder/mod.rs`
    - Add FR-EVT-007 event type: `FlightRecorderEventType::ToolCall` (stored as `event_type="tool_call"` in `events`).
    - Add payload validator `validate_tool_call_payload(payload: &Value)` enforcing Spec 11.5.2 requirements:
      - REQUIRED: `type`, `trace_id`, `tool_call_id`, `tool_id`, `tool_version`, `ok`, `timing.started_at`, `timing.ended_at`, `timing.duration_ms`.
      - If present: `args_hash` / `result_hash` are sha256 hex; `args_ref` / `result_ref` are bounded artifact-handle strings.
    - Add enums used by Tool Registry + FR payload:
      - `ToolTransport = Local|Mcp|Mex|StageBridge|Other` (serde -> `"local"|"mcp"|...`)
      - `ToolSideEffect = Read|Write|Execute`
      - `ToolIdempotency = Idempotent|IdempotentWithKey|NonIdempotent`
      - `ToolDeterminism = Deterministic|BestEffort|NonDeterministic`
      - `ToolAvailability = OfflineOk|RequiresNetwork|BestEffortOffline`
    - Add helpers:
      - `redact_tool_value(value: &Value) -> Value` using `crate::bundles::redactor::SecretRedactor` + `RedactionMode::SafeDefault`
      - `sha256_canonical_json(value: &Value) -> String`
  - `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
    - Create table `tool_payloads` to persist redacted args/results as artifacts:
      - columns: `tool_call_id UUID`, `kind TEXT`, `payload_redacted JSON`, `payload_sha256 TEXT`, `created_at TIMESTAMP`
      - indexes: `(tool_call_id)`, `(kind)`
    - Add helper `store_tool_payload_redacted(...) -> (ref_string, sha256)` that writes ONLY redacted payload and returns a stable `args_ref`/`result_ref`.
  - `src/backend/handshake_core/src/mcp/gate.rs`
    - Tool Registry SSoT embedded in `GateConfig`:
      - `ToolRegistryEntry { tool_id, tool_version, input_schema, output_schema?, side_effect, idempotency, determinism, availability, required_capabilities[], transport_bindings }`
      - MCP binding uses `mcp_name` (name to send in MCP `tools/call`) to allow transitional aliasing for existing stub servers while keeping canonical `tool_id` dot-separated.
    - `refresh_tools()` becomes Tool Registry-derived `tools/list` (Spec 11.3.0) and populates cache from the registry (not remote `tools/list`).
    - Add `tools_call_htc(ctx, request_envelope: Value) -> McpResult<Value>` that returns an HTC response envelope:
      - Validates request + response envelopes against `assets/schemas/htc_v1.json` and enforces 32KB caps.
      - On validation failure: returns `ok=false` + `error.kind="validation"` + `error.code="VAL-HTC-001"` (Spec 6.0.2.5.1).
    - Existing `tools_call(ctx, tool_name, arguments)` remains for compatibility (out-of-scope callers/tests):
      - Builds an HTC request envelope internally (generated `tool_call_id`; actor default) and routes through `tools_call_htc`.
      - Returns raw result / maps `ok=false` envelopes back to `McpError` for current call sites.
    - Add MCP correlation metadata to forwarded `tools/call` params under `_meta.handshake`:
      - `trace_id`, `tool_call_id`, `session_id`, `actor`, `idempotency_key` (when present).
    - Emit FR-EVT-007 on completion (success/deny/timeout/tool_error) via `FlightRecorder::record_event`.
  - Legacy FR events (keep for now; migrate payloads to artifact-first)
    - `src/backend/handshake_core/src/mcp/fr_events.rs` and `src/backend/handshake_core/src/mex/runtime.rs` stop persisting raw args/results inline in `fr_events.payload`; store only redacted refs/hashes.
  - `src/backend/handshake_core/src/mex/runtime.rs`
    - Builds HTC request/response envelopes from `PlannedOperation` (transport=`mex`) and validates both via `htc_v1.json`.
    - Emits FR-EVT-007 for every operation using engine provenance for `tool_id`/`tool_version`.
  - Conformance tests (Spec 6.0.2.9)
    - `src/backend/handshake_core/tests/mcp_gate_tests.rs`:
      - Add HTC validation + VAL-HTC-001 tests, 32KB cap tests, capability deny-by-default, and FR-EVT-007 emission assertions.
      - Add bypass scan: fail if any `send_request(\"tools/call\")` call site exists outside `src/mcp/gate.rs`.
    - `src/backend/handshake_core/src/mex/conformance.rs`:
      - Extend to assert FR-EVT-007 exists in `events` for the operation and that args/results are artifact-first (refs + hashes).
      - Keep legacy `fr_events` assertions until migration is complete.
- Open questions / approvals needed
  - Artifact-handle string format for `args_ref` / `result_ref` returned from `tool_payloads` (proposal: a stable `artifact:`-prefixed handle).
  - Whether to keep emitting legacy `fr_events` tool.call/tool.result during Phase 1 (proposal: keep, but remove inline payload to avoid secret persistence).
  - Default actor mapping for MCP calls when explicit model identity is unavailable (proposal: `actor.kind="agent"` with null `agent_id`/`model_id`).

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: runtime/host -> tool implementation (local process and/or remote MCP server/MEX engine)
- SERVER_SOURCES_OF_TRUTH:
  - Tool Registry + HTC schema validation (`assets/schemas/htc_v1.json`)
  - Capabilities & Consent Model enforcement (deny-by-default)
- REQUIRED_PROVENANCE_FIELDS:
  - trace_id, tool_call_id
  - tool_id, tool_version, transport
  - side_effect, idempotency, idempotency_key (when applicable)
  - session_id, actor (agent_id/model_id)
  - capability_ids (asserted/required)
  - args_ref/args_hash + result_ref/result_hash (redacted, hash computed after redaction)
- VERIFICATION_PLAN:
  - Tool Gate validates HTC envelope and required correlation fields; rejects invalid input with VAL-HTC-001.
  - Conformance tests assert (a) no bypass, (b) schema alignment, (c) FR-EVT-007 emission with required fields.
- ERROR_TAXONOMY_PLAN:
  - validation (schema/required fields) vs capability_denied vs consent_required/escalate vs transport_error vs tool_error
- UI_GUARDRAILS:
  - Human-in-the-loop consent prompts are driven by Tool Gate decisions; do not allow alternate bypass invocation paths.
- VALIDATOR_ASSERTIONS:
  - Tool Gate is the single enforcement point for all transports (no bypass).
  - FR-EVT-007 is emitted for every tool call with required fields; args/results are artifact-first with redaction-before-hash.
  - MCP `tools/list` is Tool Registry-derived and binds required `_meta.handshake` fields.

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
- Current WP_STATUS: In Progress
- What changed in this update:
  - Coder claimed WP-1-Unified-Tool-Surface-Contract-v1 and started BOOTSTRAP (gate + spec/code reads + baseline test run).
- Next step / handoff hint:
  - Fill `## SKELETON` with concrete types + dataflow decisions; stop for explicit "SKELETON APPROVED" before editing product code.

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
  - LOG_PATH: `.handshake/logs/WP-1-Unified-Tool-Surface-Contract-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
