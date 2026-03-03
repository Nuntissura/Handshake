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
  - src/backend/handshake_core/src/mcp/gate.rs
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
- CX-573F (2026-03-03): Post-work range includes governance surface changes from merged main (/.GOV/** and justfile). Scope for product code remains restricted to IN_SCOPE_PATHS; out-of-scope changes are governance-only.

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
  - src/backend/handshake_core/src/mcp/gate.rs
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
  - Scope mismatch: RESOLVED by adding `src/backend/handshake_core/src/mcp/gate.rs` to IN_SCOPE_PATHS; enforcement should occur at Tool Gate per spec "Capability rule (HARD)" (Handshake_Master_Spec_v02.139.md 4.3.9.16.4) and HTC session binding (6.0.2.5).
  - Schema versioning: DECISION for this WP is to extend `hsk.*@0.4` structs with new fields as serde optional (no bump to `@0.5` unless a hard incompatibility is discovered, then escalate).
  - Broadcast/fan-out target enumeration source-of-truth: receipt `session_ids[]` vs ProjectionPlan; current ProjectionPlanV0_4 has no session binding.
  - Revocation surface: where does operator-triggered revocation enter (workflow protocol / locus op / admin op)? Implement internal primitive now; wire surface later if out-of-scope.
- Notes:
  - END_TO_END_CLOSURE_PLAN [CX-E2E-001] already present below; enforcement will treat `job_inputs` fields as untrusted at trust boundaries and derive session/consent/capability truth from stored ModelSession + receipt artifacts.
  - No product code changes until SKELETON is approved.

## SKELETON APPROVED
SKELETON APPROVED
- Approved_at_utc: 2026-03-03T05:50:41Z
- Approved_by: role_validator
- Scope amendment: add `src/backend/handshake_core/src/mcp/gate.rs` to IN_SCOPE_PATHS (aligns DONE_MEANS "Tool Gate" + spec HARD rule 4.3.9.16.4).
- Schema decision: extend `hsk.*@0.4` structs with optional fields (no `@0.5` bump in this WP).

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
- Implemented session-scoped capability enforcement in MCP Tool Gate using DB-backed ModelSession grants/tokens (deny-by-default when HTC session_id present) and TRUST-003 parent narrowing.
- Extended hsk.*@0.4 cloud consent schemas with optional consent_scope/session binding + validity/revocation fields; added strict validation when CloudEscalationRequest.session_id is present.
- Enforced INV-CONSENT-001 at model_run dispatch (cloud tier or backend=cloud): require CloudEscalationBundle, validate hash binding, bind session_id + consent_receipt_id, and emit FR-EVT-CLOUD-* deny/execute events.
- Implemented INV-CONSENT-003 primitive: revoke consent_receipt_id cancels pending model_run jobs and blocks affected sessions (consent_revoked).
- Enforced TRUST-001/002 at ModelRun inbound trust boundary: downgrade external SYSTEM -> USER with provenance attribution; validate/persist cross-session provenance fields when provided.
- Updated/added tests for MCP gate, MCP e2e, model_session_scheduler consent gating, revocation, and trust boundary behavior.

## HYGIENE
- Ran: just pre-work WP-1-Session-Scoped-Capabilities-Consent-Gate-v1
- Ran: cargo test (targeted) for handshake_core: mcp_gate_tests, mcp_e2e_tests, model_session_scheduler_tests
- Ran: just test (with CARGO_TARGET_DIR override to avoid space-containing path on Windows)

## VALIDATION
- (Deterministic manifest for audit. This records the "What" (hashes/lines) for the Validator's "How/Why" audit. It is NOT a claim of official Validation.)

- **Target File**: `justfile`
- **Start**: 57
- **End**: 245
- **Line Delta**: 5
- **Pre-SHA1**: `fdbb900798a9eb07e14c4d085dced45923553f94`
- **Post-SHA1**: `4da0b2b0367554d1e5c2de3938ea002cfe95b3f5`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
  - [x] all_links_resolvable

- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 4762
- **End**: 4863
- **Line Delta**: 58
- **Pre-SHA1**: `12373dd82250732fd97aff658bc0bba68eb27ba9`
- **Post-SHA1**: `11fc877b0073c0ef6147c00377e72debf445009b`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
  - [x] all_links_resolvable

- **Target File**: `src/backend/handshake_core/src/llm/guard.rs`
- **Start**: 12
- **End**: 682
- **Line Delta**: 216
- **Pre-SHA1**: `7eb568e8a669b047cfc4be3c63695496b5b35d5a`
- **Post-SHA1**: `9159e5fa6c62fd478c4862ec2fa3b30a4f3b7768`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
  - [x] all_links_resolvable

- **Target File**: `src/backend/handshake_core/src/llm/mod.rs`
- **Start**: 407
- **End**: 417
- **Line Delta**: 10
- **Pre-SHA1**: `7f6acfdef2a761522cdeefa55f54abb9bf2ff639`
- **Post-SHA1**: `4b833f9d8f9a2f6bf3867de7ca1ef28f75dafa45`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
  - [x] all_links_resolvable

- **Target File**: `src/backend/handshake_core/src/mcp/gate.rs`
- **Start**: 403
- **End**: 1345
- **Line Delta**: 254
- **Pre-SHA1**: `8ea82398453f0f4eb3902504c6200cd4552b7ebc`
- **Post-SHA1**: `7dfdca609427853c2640e83c44fc988049cad11f`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
  - [x] all_links_resolvable

- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 37
- **End**: 11091
- **Line Delta**: 463
- **Pre-SHA1**: `fee6ab73ce755f3c72a4e330600c58760e399a93`
- **Post-SHA1**: `8bf16dea97e1503fb9a27bfd1d0a346d3f521952`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
  - [x] all_links_resolvable

- **Target File**: `src/backend/handshake_core/tests/mcp_e2e_tests.rs`
- **Start**: 14
- **End**: 314
- **Line Delta**: 25
- **Pre-SHA1**: `3e05973e506997731764dda68d4e8cdccb81f02d`
- **Post-SHA1**: `b3c03021e526a0963b6a628ca5c93d04e796376f`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
  - [x] all_links_resolvable

- **Target File**: `src/backend/handshake_core/tests/mcp_gate_tests.rs`
- **Start**: 7
- **End**: 1136
- **Line Delta**: 434
- **Pre-SHA1**: `84367469af501ee4573b4286eb8653ed1949ea76`
- **Post-SHA1**: `8182e539fb917b53cac985fdbaf463f20e77e99f`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
  - [x] all_links_resolvable

- **Target File**: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs`
- **Start**: 11
- **End**: 924
- **Line Delta**: 459
- **Pre-SHA1**: `259531a4a96a24984d24e6fcaf95ca245146f1e4`
- **Post-SHA1**: `7a773197bbfaf518d942fbb77f0d50ded4da9576`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
  - [x] all_links_resolvable

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: IN_PROGRESS (ready for validator audit)
- What changed in this update:
  - Implemented session-scoped capability enforcement + TRUST-003 in MCP Tool Gate.
  - Enforced cloud consent gate invariants for model_run (INV-CONSENT-001..003) with FR-EVT-CLOUD-* events.
  - Enforced TRUST-001/002 on ModelRun inbound session_messages (SYSTEM downgrade + provenance validation/persistence).
  - Added/updated tests covering session-scoped MCP gate, cloud model_run consent, revocation, and provenance rules.
- Next step / handoff hint:
  - Run `just post-work WP-1-Session-Scoped-Capabilities-Consent-Gate-v1 --range 1cb1bae85f51a83bac1dc28580199b6e15bec157..HEAD` and review warnings.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- REQUIREMENT: "Tool Gate enforces session-scoped capability intersection when session_id is present (deny-by-default)"
  - EVIDENCE: `src/backend/handshake_core/src/mcp/gate.rs:1169`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_gate_tests.rs:718`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_gate_tests.rs:805`
- REQUIREMENT: "TRUST-003 is enforced for session-scoped capabilities: child sessions cannot widen vs parent"
  - EVIDENCE: `src/backend/handshake_core/src/mcp/gate.rs:1192`
- REQUIREMENT: "Cloud model_run dispatch is blocked without valid ConsentReceipt bound to session (INV-CONSENT-001) and denials observable via FR-EVT-CLOUD-*"
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:3357`
  - EVIDENCE: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:489`
- REQUIREMENT: "Cloud escalation executed is observable via FR-EVT-CLOUD-004 (no raw payloads)"
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:3541`
  - EVIDENCE: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:557`
- REQUIREMENT: "BROADCAST_SCOPED receipt enumerates session_ids; non-enumerated target session is blocked (INV-CONSENT-002)"
  - EVIDENCE: `src/backend/handshake_core/src/llm/guard.rs:290`
  - EVIDENCE: `src/backend/handshake_core/src/llm/guard.rs:626`
- REQUIREMENT: "Revocation cancels pending model_run jobs and transitions affected sessions to BLOCKED (INV-CONSENT-003)"
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:3782`
  - EVIDENCE: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:846`
- REQUIREMENT: "TRUST-001/002 enforced: external sources cannot inject SYSTEM; cross-session routed messages require provenance and are persisted"
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2919`
  - EVIDENCE: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:330`
  - EVIDENCE: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:385`
  - EVIDENCE: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:451`
- REQUIREMENT: "FR-EVT-CLOUD payload validator allows consent_scope/session_id/session_ids fields"
  - EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:4765`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Session-Scoped-Capabilities-Consent-Gate-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

- COMMAND: `just pre-work WP-1-Session-Scoped-Capabilities-Consent-Gate-v1`
  - EXIT_CODE: 0
  - PROOF_LINES: `Pre-work validation PASSED`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir D:\\hs-cargo-target --test mcp_gate_tests`
  - EXIT_CODE: 0
  - PROOF_LINES: `test result: ok. 17 passed; 0 failed`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir D:\\hs-cargo-target --test mcp_e2e_tests`
  - EXIT_CODE: 0
  - PROOF_LINES: `test result: ok. 1 passed; 0 failed`

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir D:\\hs-cargo-target --test model_session_scheduler_tests`
  - EXIT_CODE: 0
  - PROOF_LINES: `test result: ok. 11 passed; 0 failed`

- COMMAND: `just --set CARGO_TARGET_DIR D:\\hs-cargo-target test`
  - EXIT_CODE: 0
  - PROOF_LINES: `Finished \`test\` profile` / `Doc-tests handshake_core`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### VALIDATION REPORT - WP-1-Session-Scoped-Capabilities-Consent-Gate-v1
Verdict: FAIL

Validation Claims (do not collapse into a single PASS):
- GATES_PASS (deterministic manifest gate: `just post-work WP-1-Session-Scoped-Capabilities-Consent-Gate-v1 --range 1cb1bae85f51a83bac1dc28580199b6e15bec157..HEAD`; not tests): FAIL
- TEST_PLAN_PASS (packet TEST_PLAN commands, verbatim): PASS
- SPEC_CONFORMANCE_CONFIRMED (DONE_MEANS + SPEC_ANCHOR -> evidence mapping): NO

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Session-Scoped-Capabilities-Consent-Gate-v1.md (status: In Progress)
- Refinement: .GOV/refinements/WP-1-Session-Scoped-Capabilities-Consent-Gate-v1.md (SPEC_TARGET_RESOLVED: Handshake_Master_Spec_v02.139.md)
- Spec Anchors (refinement):
  - 6.0.2.5 Canonical invocation envelope (HTC-1.0) (MUST) - session-scoped capability intersection (Normative)
  - 4.3.9.12 ModelSession (Normative) - consent + capability_token_ids
  - 4.3.9.14 Cloud Consent-Gate Lifecycle for Parallel Sessions (Normative) - INV-CONSENT-001/002/003
  - 4.3.9.20 Inbound Trust Boundary Rules (Normative) - TRUST-001/002/003/004
  - 11.5 FR-EVT-007 ToolCallEvent (Normative)
  - 11.5.8 FR-EVT-CLOUD-001..004 (Cloud Escalation Events) (Normative)

Worktree/Branch Verified:
- worktree_dir: ../wt-WP-1-Session-Scoped-Capabilities-Consent-Gate-v1
- branch: feat/WP-1-Session-Scoped-Capabilities-Consent-Gate-v1
- head_sha: 34f0460863b30dfebb1591dac5ccca63356d6fa0
- validated_range: 1cb1bae85f51a83bac1dc28580199b6e15bec157..34f0460863b30dfebb1591dac5ccca63356d6fa0

Files Checked:
- src/backend/handshake_core/src/mcp/gate.rs
- src/backend/handshake_core/src/mcp/fr_events.rs
- src/backend/handshake_core/src/workflows.rs
- src/backend/handshake_core/src/llm/guard.rs
- src/backend/handshake_core/src/flight_recorder/mod.rs
- src/backend/handshake_core/tests/mcp_gate_tests.rs
- src/backend/handshake_core/tests/mcp_e2e_tests.rs
- src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- justfile
- .GOV/task_packets/WP-1-Session-Scoped-Capabilities-Consent-Gate-v1.md
- .GOV/refinements/WP-1-Session-Scoped-Capabilities-Consent-Gate-v1.md

Findings:
- Deterministic manifest discipline failure (C701):
  - The packet `## VALIDATION` manifest entries only list `- [x] all_links_resolvable` and omit the required per-entry gate checklist items (anchors_present, window_matches_plan, rails_untouched_outside_window, filename_canonical_and_openable, pre/post SHA1 captured, line_delta_equals_expected, manifest_written_and_path_returned, current_file_matches_preimage, etc).
  - `just post-work ...` reports many warnings of the form: "Manifest[n]: gate not checked but inferred as PASS: <gate>" and also notes `Git hygiene waiver detected [CX-573F]` which relaxes strict git checks.
  - Per VALIDATOR_PROTOCOL "Deterministic Manifest Gate" requirements, missing/unchecked gates are FAIL even if the script infers PASS.

- DONE_MEANS / SPEC mismatch: session-scoped tool capability denials do not emit FR-EVT-007 (ToolCallEvent):
  - In `src/backend/handshake_core/src/mcp/gate.rs`, when session_id is present and `session_scoped_grants` do not satisfy a required capability, the gate returns `McpError::CapabilityDenied(...)` after recording only `mcp.gate.decision` (via `fr_events::record_gate_decision`), but it does not record a `FlightRecorderEventType::ToolCall` event.
  - This contradicts DONE_MEANS: "missing required capability yields ... FR-EVT-007 (ToolCallEvent) evidence" and refinement anchor 11.5 (ToolCallEvent).
  - Additional related risk: failures inside `resolve_session_scoped_grants(session_id).await?` return early via `?` before a ToolCall event is recorded, meaning "deny-by-default" paths (DB unavailable / session lookup errors) may also lack FR-EVT-007 evidence.

- Waiver scope note (CX-573F):
  - Range includes many governance-surface changes (/.GOV/** and justfile). Waiver is recorded in packet and the post-work gate allows this, but it increases audit surface and risk of mixed provenance for the deterministic gate scripts used as evidence.

Tests (run by Validator):
- just pre-work WP-1-Session-Scoped-Capabilities-Consent-Gate-v1: PASS (with warnings)
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir D:\\hs-cargo-target --test mcp_gate_tests: PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir D:\\hs-cargo-target --test mcp_e2e_tests: PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir D:\\hs-cargo-target --test model_session_scheduler_tests: PASS
- just --set CARGO_TARGET_DIR D:\\hs-cargo-target test: PASS
- just post-work WP-1-Session-Scoped-Capabilities-Consent-Gate-v1 --range 1cb1bae85f51a83bac1dc28580199b6e15bec157..HEAD: PASS (with warnings; treated as FAIL per manifest discipline above)

REASON FOR FAIL:
- Packet deterministic manifest gate checklist is incomplete and the post-work gate output explicitly reports many "gate not checked but inferred" warnings; VALIDATOR_PROTOCOL requires explicit gates, not inference.
- Session-scoped capability denials do not produce FR-EVT-007 ToolCallEvent evidence, violating DONE_MEANS + spec anchor 11.5.

Required Remediation:
1) Fix Tool Gate to emit FR-EVT-007 ToolCallEvent on session-scoped capability denial (and parent-capability denial) and on deny-by-default session grant resolution failures (DB unavailable / session lookup errors). Add/extend tests to assert the ToolCall event exists for these deny paths.
2) Update the task packet `## VALIDATION` manifest entries to include the full required per-entry gate checklist (no inference warnings). Re-run `just post-work ... --range 1cb1bae..HEAD` until the deterministic manifest gate passes without "gate not checked but inferred" warnings.

Residual Risks / Notes:
- Given the waiver and mixed governance changes in-range, consider minimizing audit surface by isolating governance-only changes outside the WP merge window (or re-cutting a clean branch) after remediation.
