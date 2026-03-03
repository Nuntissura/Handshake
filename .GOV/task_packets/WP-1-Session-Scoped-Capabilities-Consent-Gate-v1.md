# Task Packet: WP-1-Session-Scoped-Capabilities-Consent-Gate-v1

## METADATA
- TASK_ID: WP-1-Session-Scoped-Capabilities-Consent-Gate-v1
- WP_ID: WP-1-Session-Scoped-Capabilities-Consent-Gate-v1
- BASE_WP_ID: WP-1-Session-Scoped-Capabilities-Consent-Gate
- DATE: 2026-03-03T01:24:47.996Z
- MERGE_BASE_SHA: 1cb1bae85f51a83bac1dc28580199b6e15bec157 (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
- REQUESTOR: Operator (ilja)
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- AGENTIC_MODE: NO
- ORCHESTRATOR_MODEL: N/A (required if AGENTIC_MODE=YES)
- ORCHESTRATION_STARTED_AT_UTC: N/A (RFC3339 UTC; required if AGENTIC_MODE=YES)
- CODER_MODEL: CodexCLI-GPT-5.2
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-ModelSession-Core-Scheduler, WP-1-Unified-Tool-Surface-Contract, WP-1-Capability-SSoT, WP-1-Cloud-Escalation-Consent
- BUILD_ORDER_BLOCKS: WP-1-Session-Spawn-Contract, WP-1-Provider-Feature-Coverage-Agentic-Ready, WP-1-Workspace-Safety-Parallel-Sessions, WP-1-Session-Crash-Recovery-Checkpointing, WP-1-Session-Observability-Spans-FR
- USER_SIGNATURE: ilja030320260206
- PACKET_FORMAT_VERSION: 2026-02-01

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: N/A

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: ALLOWED
- OPERATOR_APPROVAL_EVIDENCE: ok coder A, no orchestrator agents, coder can use agents.
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Session-Scoped-Capabilities-Consent-Gate-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement deny-by-default session-scoped capability enforcement in Tool Gate (HTC-1.0 `session_id`), and enforce cloud consent-gate invariants for parallel sessions (including broadcast/fan-out) with strict trust-boundary message provenance rules.
- Why: Multi-session execution becomes a remote action pipeline unless every tool call and cloud escalation is bound to per-session effective capabilities + durable consent receipts, and inbound SYSTEM/provenance is fail-closed.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/llm/guard.rs
  - src/backend/handshake_core/src/llm/mod.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/schemas/capability_registry.schema.json
  - src/backend/handshake_core/tests/mcp_gate_tests.rs
  - src/backend/handshake_core/tests/mcp_e2e_tests.rs
  - src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- OUT_OF_SCOPE:
  - Session spawn lifecycle contract (WP-1-Session-Spawn-Contract-v1)
  - Provider streaming/tool-calling parity work (WP-1-Provider-Feature-Coverage-Agentic-Ready-v1)
  - Workspace isolation mechanics (WP-1-Workspace-Safety-Parallel-Sessions-v1)
  - Crash recovery checkpoint/resume mechanics (WP-1-Session-Crash-Recovery-Checkpointing-v1)
  - Full session observability family expansion (WP-1-Session-Observability-Spans-FR-v1)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Session-Scoped-Capabilities-Consent-Gate-v1

# Backend tests (prefer targeted first, then full suite as needed):
cargo test -p handshake_core mcp_gate_tests
cargo test -p handshake_core mcp_e2e_tests
cargo test -p handshake_core model_session_scheduler_tests
just test

just cargo-clean
just post-work WP-1-Session-Scoped-Capabilities-Consent-Gate-v1 --range 1cb1bae85f51a83bac1dc28580199b6e15bec157..HEAD
```

### DONE_MEANS
- Tool Gate enforces session-scoped capability intersection when `session_id` is present (deny-by-default): missing required capability yields a deterministic deny with structured error and FR-EVT-007 (ToolCallEvent) evidence.
- Cloud `model_run` dispatch is blocked without a valid `ConsentReceipt` bound to the target session (INV-CONSENT-001); denials are observable via FR-EVT-CLOUD-* (no inline payloads).
- BROADCAST_SCOPED receipts enumerate session_ids at issuance; attempting to fan-out to any non-enumerated session is blocked (INV-CONSENT-002).
- Revoking a receipt cancels all pending covered `model_run` jobs and transitions affected ModelSessions to `BLOCKED` (INV-CONSENT-003).
- TRUST-001/002 are enforced: external sources cannot inject `SYSTEM` role messages; cross-session routed messages carry source attribution + hashes, and provenance violations are rejected or downgraded deterministically.
- TRUST-003 is enforced for session-scoped capabilities: child sessions cannot widen capabilities vs parent; violations are blocked and logged.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.139.md (recorded_at: 2026-03-03T01:24:47.996Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md 6.0.2.5; 4.3.9.12; 4.3.9.14; 4.3.9.20; 11.5 (FR-EVT-007); 11.5.8 (FR-EVT-CLOUD-001..004)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - .GOV/refinements/WP-1-Session-Scoped-Capabilities-Consent-Gate-v1.md
  - .GOV/task_packets/WP-1-Session-Scoped-Capabilities-Consent-Gate-v1.md
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/llm/guard.rs
  - src/backend/handshake_core/src/mex/runtime.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
- SEARCH_TERMS:
  - "session-scoped capability intersection"
  - "required_capabilities"
  - "capability_token_ids"
  - "consent_receipt_id"
  - "ProjectionPlan"
  - "ConsentReceipt"
  - "FR-EVT-007"
  - "FR-EVT-CLOUD-"
  - "TRUST-001"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Session-Scoped-Capabilities-Consent-Gate-v1
  cargo test -p handshake_core mcp_gate_tests
  cargo test -p handshake_core model_session_scheduler_tests
  ```
- RISK_MAP:
  - "capability confused-deputy" -> "cross-session privilege escalation if global caps applied without intersection"
  - "receipt replay/broadcast bypass" -> "silent fan-out to non-enumerated sessions (INV-CONSENT-002 violation)"
  - "SYSTEM provenance spoof" -> "prompt-injection indistinguishable from runtime policy (TRUST-001 violation)"
  - "partial revocation" -> "orphaned cloud calls continue after operator revocation (INV-CONSENT-003 violation)"

## SKELETON
- Proposed interfaces/types/contracts:
  - Session-scoped capability resolution (deny-by-default):
    - `ResolvedSessionCapabilities` (capabilities.rs): `{ session_id, parent_session_id?, effective_capability_ids[], capability_token_ids[] }`
    - `resolve_effective_capabilities_for_session(db, registry, session_id) -> Result<ResolvedSessionCapabilities, CapabilityGateError>`
    - `CapabilityGateErrorKind` + `CapabilityGateError` (capabilities.rs / workflows.rs) with stable kinds:
      - `capability_mismatch`
      - `consent_missing_or_invalid`
      - `provenance_violation`
  - Child-session narrowing (TRUST-003):
    - `resolve_effective_capabilities_for_session(...)` intersects child grants/tokens with parent effective set (fail-closed if parent missing).
  - Cloud consent-gate binding + broadcast scope (INV-CONSENT-001..003):
    - Extend `ConsentReceiptV0_4` (llm/guard.rs) with explicit session binding + scope:
      - `consent_scope: ConsentScope` (enum: SINGLE_CALL | SESSION_SCOPED | WP_SCOPED | BROADCAST_SCOPED)
      - `session_ids: Vec<String>` (for BROADCAST_SCOPED; single-element for SESSION_SCOPED)
      - `valid_from_utc: Option<String>` / `valid_until_utc: Option<String>` (enforced; deny when expired)
    - Extend `CloudEscalationRequestV0_4` (llm/guard.rs) with:
      - `session_id: String`
      - `consent_scope: ConsentScope`
    - `CloudEscalationBundleV0_4::validate_for_payload_sha256(...)` additionally validates:
      - receipt/session binding (contains session_id; BROADCAST_SCOPED enumerates targets)
      - validity window (now within [valid_from, valid_until])
  - Scheduler consent gate enforcement + revocation (workflows.rs):
    - `validate_consent_for_model_run_dispatch(db, session_id, consent_receipt_id, now) -> Result<(), CapabilityGateError>`
    - `revoke_consent_receipt(db, consent_receipt_id, reason) -> Result<(), WorkflowError>`:
      - cancel pending ModelRun jobs covered by receipt
      - transition affected ModelSessions to `BLOCKED`
  - Inbound trust boundary enforcement (TRUST-001/002) (workflows.rs + storage/mod.rs):
    - `InboundMessageProvenance` struct (workflows.rs): `{ source_kind, source_session_id?, source_role?, content_hash?, trusted }`
    - `sanitize_inbound_session_messages(inputs) -> Result<Vec<NewSessionMessage>, CapabilityGateError>`:
      - external SYSTEM is rejected or downgraded deterministically (recorded)
      - cross-session routed messages require provenance fields (else provenance_violation)
  - Flight Recorder payload compatibility (flight_recorder/mod.rs):
    - Allow optional `session_id` / `session_ids` on FR-EVT-CLOUD-* payloads (kept bounded tokens).
- Open questions:
  - Scope mismatch risk: session-scoped tool enforcement for MCP currently lives in `src/backend/handshake_core/src/mcp/gate.rs` (not listed in IN_SCOPE_PATHS). Plan is to enforce via session-effective `granted_capabilities` computed upstream in in-scope runtime/workflows code; confirm this satisfies "Tool Gate" DONE_MEANS or request Orchestrator to add `src/backend/handshake_core/src/mcp/gate.rs` to IN_SCOPE_PATHS.
  - Schema versioning: extend `hsk.*@0.4` structs with new fields (serde optional) vs bump to `@0.5` (spec alignment decision needed).
  - Broadcast/fan-out target enumeration source-of-truth: receipt `session_ids[]` vs ProjectionPlan; current ProjectionPlanV0_4 has no session binding.
  - Revocation surface: where does operator-triggered revocation enter (workflow protocol / locus op / admin op)? Implement internal primitive now; wire surface later if out-of-scope.
- Notes:
  - END_TO_END_CLOSURE_PLAN [CX-E2E-001] already present below; enforcement will treat `job_inputs` fields as untrusted at trust boundaries and derive session/consent/capability truth from stored ModelSession + receipt artifacts.
  - No product code changes until SKELETON is approved.

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: tool_call request (HTC envelope) -> Tool Gate -> engine execution
- SERVER_SOURCES_OF_TRUTH:
  - ModelSession record (db) for `session_id` -> capability_grants + capability_token_ids + consent_receipt_id
  - CapabilityRegistry SSoT + resolved scoped axis rules
  - ConsentReceipt + ProjectionPlan artifacts (hash-bound) for cloud escalation
- REQUIRED_PROVENANCE_FIELDS:
  - trace_id, tool_call_id, session_id, actor.kind/agent_id/model_id
  - tool_id, tool_version, required_capabilities, effective_capability_ids (post-intersection)
  - consent_receipt_id, projection_plan_id (for cloud paths)
- VERIFICATION_PLAN:
  - Deny-by-default when `session_id` is present and no effective grants/tokens satisfy required capabilities.
  - Emit FR-EVT-007 for every tool call with capability_ids[] + ok/denied and redacted args/result refs.
  - Emit FR-EVT-CLOUD-* events for cloud escalation consent/deny/execute (no raw payloads).
- ERROR_TAXONOMY_PLAN:
  - capability_mismatch (session effective caps do not satisfy required_capabilities)
  - consent_missing_or_invalid (receipt missing/expired/not bound to session or broadcast list)
  - provenance_violation (TRUST-001/002: SYSTEM injection or spoofed source attribution)
- UI_GUARDRAILS:
  - N/A (backend enforcement; UI surfaces may exist but are out-of-scope here)
- VALIDATOR_ASSERTIONS:
  - Tool Gate denies unauthorized session-scoped tool calls and records FR-EVT-007 with `model_session_id` correlation.
  - Scheduler blocks cloud `model_run` dispatch without valid ConsentReceipt bound to session; revocation cancels pending jobs and sets sessions BLOCKED.

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
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

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
  - LOG_PATH: `.handshake/logs/WP-1-Session-Scoped-Capabilities-Consent-Gate-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
