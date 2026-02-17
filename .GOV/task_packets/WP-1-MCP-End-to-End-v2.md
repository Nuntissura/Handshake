# Task Packet: WP-1-MCP-End-to-End-v2

## METADATA
- TASK_ID: WP-1-MCP-End-to-End-v2
- WP_ID: WP-1-MCP-End-to-End-v2
- BASE_WP_ID: WP-1-MCP-End-to-End (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-16T22:03:19.337Z
- MERGE_BASE_SHA: e048533f2ddbfbef1f14aa8de5dc75eb8dc2c51b
- REQUESTOR: Operator (ilja)
- AGENT_ID: codex-cli (gpt-5.2)
- ROLE: Orchestrator
- AGENTIC_MODE: YES
- ORCHESTRATOR_MODEL: gpt-5.2
- ORCHESTRATION_STARTED_AT_UTC: 2026-02-16T22:03:19.337Z
- CODER_MODEL: gpt-5.2
- CODER_REASONING_STRENGTH: EXTRA_HIGH (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja160220262157
- PACKET_FORMAT_VERSION: 2026-02-01

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: ALLOWED
- OPERATOR_APPROVAL_EVIDENCE: ok coder can use agents in the mcp end to end v2 wp
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- RULES (HARD):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + DONE_MEANS before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - Follow: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-MCP-End-to-End-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement an end-to-end MCP-backed job flow (Rust Host MCP client + Gate + persistence + Flight Recorder evidence) that exercises MCP tool calls, reference payloads, durable progress, and logging/message semantics per Master Spec 11.3.x.
- Why: Prior packet `WP-1-MCP-End-to-End` failed validation due to missing implementation. This v2 packet aligns to current Master Spec v02.126 and provides the end-to-end evidence chain needed for Phase 1 closure. This WP is downstream of `WP-1-MCP-Skeleton-Gate-v2` (validated PASS on 2026-02-16) and is now ready for development.
- IN_SCOPE_PATHS:
  - .GOV/refinements/WP-1-MCP-End-to-End-v2.md
  - .GOV/task_packets/WP-1-MCP-End-to-End-v2.md
  - .GOV/roles_shared/TASK_BOARD.md
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - .GOV/scripts/validation/pre-work-check.mjs
  - src/backend/handshake_core/src/mcp/client.rs
  - src/backend/handshake_core/src/mcp/fr_events.rs
  - src/backend/handshake_core/src/mcp/gate.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/tests/mcp_e2e_tests.rs
- OUT_OF_SCOPE:
  - Any MCP transport implementation work not required for the end-to-end evidence chain (owned by WP-1-MCP-Skeleton-Gate-v2)
  - Full Docling/ASR sidecar/server implementations (Phase 2 feature work)
  - UI work (frontend surfaces); this WP proves backend contracts and Flight Recorder evidence, not UX polish
  - Any spec edits or spec enrichment (ENRICHMENT_NEEDED=NO per refinement)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-MCP-End-to-End-v2

# Focused backend tests for MCP E2E behavior (added/updated by this WP):
cd src/backend/handshake_core; cargo test -j 1 --test mcp_e2e_tests

# Regression safety:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

just cargo-clean
just post-work WP-1-MCP-End-to-End-v2 --range e048533f2ddbfbef1f14aa8de5dc75eb8dc2c51b..HEAD
```

### DONE_MEANS
- A single end-to-end MCP-backed job is exercised by tests and produces a tool call + tool result through the Gate (deny-by-default enforced; allow path explicit).
- Reference-based binary protocol is implemented for at least one test fixture: server returns `ref://...` and host resolves it without trusting `file://` URIs; host emits the corresponding release notification/event per spec intent.
- Durable progress mapping exists: MCP `notifications/progress` token is persisted to SQLite `ai_jobs` fields (`mcp_server_id`, `mcp_call_id`, `mcp_progress_token`) and can be correlated back to the originating job/workflow_run_id.
- Flight Recorder evidence exists for MCP traffic with event_kind including at least: `mcp.tool_call`, `mcp.tool_result` (or equivalent), `mcp.progress`, and `mcp.logging`, with correlation fields (job_id, workflow_run_id, trace_id, server_id, tool_name) present in payload or row fields per spec anchor.
- Red-team hardening is implemented for symlink/root bypass and sampling/createMessage injection paths per spec anchor (canonicalization/no-follow; untrusted fencing; default-deny for undeclared capabilities).
- `just pre-work WP-1-MCP-End-to-End-v2` and `just post-work WP-1-MCP-End-to-End-v2 --range e048533f2ddbfbef1f14aa8de5dc75eb8dc2c51b..HEAD` both run with EXIT_CODE=0.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.126.md (recorded_at: 2026-02-16T22:03:19.337Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.126.md 11.3.1.1 The MCP Topology: Inverting the Client-Server Relationship
  - Handshake_Master_Spec_v02.126.md 11.3.2 Implementation Target 1: Rust Gate Interceptor (Middleware Design)
  - Handshake_Master_Spec_v02.126.md 11.3.3 Implementation Target 2: Reference-Based Binary Protocol
  - Handshake_Master_Spec_v02.126.md 11.3.4 Implementation Target 3: Durable Progress Mapping (SQLite Integration)
  - Handshake_Master_Spec_v02.126.md 11.3.6 Logging Sink (DuckDB) - MCP event_kind mapping + logging/message payload contract
  - Handshake_Master_Spec_v02.126.md 11.3.7 Red Team Security Audit (Symlinks + Sampling)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.
-
- BASE_WP_ID: WP-1-MCP-End-to-End
- WP_ID: WP-1-MCP-End-to-End-v2
- SPEC_TARGET: Handshake_Master_Spec_v02.126.md (from .GOV/roles_shared/SPEC_CURRENT.md)
- Prior packets:
  - .GOV/task_packets/WP-1-MCP-End-to-End.md (legacy; status: SUPERSEDED on TASK_BOARD; signature was never locked; validator verdict: FAIL on 2025-12-26)
- Preserved requirements (from prior packet scope / DONE_MEANS):
  - Capability metadata propagation through MCP requests/responses (deny-by-default; explicit allowlist)
  - Logging chain to Flight Recorder for gate decisions + tool calls + results
  - Tests for allowed/denied paths
- New/expanded requirements in v2 (current Master Spec 11.3.x):
  - Topology clarity (Host is MCP Client; Orchestrator is MCP Server) and correlation metadata (job_id/workflow_run_id/trace_id)
  - Reference-based binary protocol (`ref://`) handling and resource release signaling
  - Durable progress mapping (SQLite ai_jobs fields + index on progress token)
  - Explicit DuckDB Flight Recorder event_kind mapping for MCP (`mcp.tool_call`, `mcp.progress`, `mcp.logging`, plus tool result)
  - Red-team hardening (symlink roots bypass; sampling/createMessage injection)
- Dependency note: This packet assumes MCP client+Gate foundations from `WP-1-MCP-Skeleton-Gate-v2` exist on `main` (validated PASS on 2026-02-16). This WP may proceed.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.126.md
  - .GOV/refinements/WP-1-MCP-End-to-End-v2.md
  - .GOV/task_packets/WP-1-MCP-End-to-End-v2.md
  - .GOV/task_packets/WP-1-MCP-Skeleton-Gate-v2.md (dependency)
  - src/backend/handshake_core/src/mcp/gate.rs
  - src/backend/handshake_core/src/mcp/client.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/workflows.rs
- SEARCH_TERMS:
  - "tools/call"
  - "notifications/progress"
  - "\"mcp.tool_call\""
  - "mcp.tool_result"
  - "mcp.progress"
  - "mcp.logging"
  - "logging/message"
  - "ref://"
  - "resource_released"
  - "mcp_progress_token"
  - "mcp_call_id"
  - "mcp_server_id"
  - "trace_id"
  - "workflow_run_id"
  - "sampling/createMessage"
  - "roots/list"
  - "canonicalize"
  - "O_NOFOLLOW"
- RUN_COMMANDS:
  ```bash
  # Governance gates:
  just pre-work WP-1-MCP-End-to-End-v2

  # Backend tests:
  cd src/backend/handshake_core; cargo test -j 1 --test mcp_e2e_tests

  # Hygiene:
  just cargo-clean
  just post-work WP-1-MCP-End-to-End-v2 --range e048533f2ddbfbef1f14aa8de5dc75eb8dc2c51b..HEAD
  ```
- RISK_MAP:
  - "Missing durable progress persistence" -> "Progress UI and auditability cannot correlate MCP progress to ai_jobs; spec nonconformance"
  - "Reference URI trust bypass (file:// or path traversal)" -> "Potential data exfiltration or unintended file reads; security incident"
  - "Incomplete Flight Recorder event_kind mapping" -> "Audit gap; validator cannot prove end-to-end chain"
  - "Spoofed MCP provenance fields (trace_id/job_id)" -> "False audit trails; trust boundary violation"
  - "Sampling/createMessage injection path not fenced" -> "Prompt injection into trusted system prompt; phishing/social engineering risk"

## SKELETON
- Proposed interfaces/types/contracts:
- `McpJobContext` (job_id, workflow_run_id, trace_id, server_id, tool_name, capability_profile_id)
- Durable progress mapping persisted in SQLite ai_jobs (`mcp_server_id`, `mcp_call_id`, `mcp_progress_token`)
- Reference payload handling for `ref://` URIs (host-side resolution; release signaling)
- Open questions:
- What is the minimal MCP server stub strategy for tests (in-process fake vs. stdio child)?
- Notes:
- Dependency satisfied: `WP-1-MCP-Skeleton-Gate-v2` validated PASS on 2026-02-16 (lock-set overlap acknowledged; coordinate carefully).

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: host(client)->orchestrator(server) JSON-RPC is untrusted; host must gate/verify capabilities and must derive audit fields from job/workflow context (not trust server-provided values blindly).
- SERVER_SOURCES_OF_TRUTH:
  - Host-side job/workflow context (job_id, workflow_run_id, trace_id) is the source of truth for correlation; server may echo but is not trusted.
  - Host-side storage (SQLite ai_jobs) is source of truth for durable progress tokens and correlation.
- REQUIRED_PROVENANCE_FIELDS:
  - job_id
  - workflow_run_id
  - trace_id
  - server_id
  - tool_name
  - capability_profile_id (or equivalent capability binding)
  - gate_decision (allow/deny/timeout) where applicable
- VERIFICATION_PLAN:
  - Ensure Gate injects correlation fields into outbound tool calls and logs an `mcp.tool_call` Flight Recorder event derived from host context.
  - Ensure every tool result produces a paired event (`mcp.tool_result` or equivalent) and that progress/logging events include job/workflow correlation.
- ERROR_TAXONOMY_PLAN:
  - capability_denied (policy/consent)
  - undeclared_capability (server tries forbidden method; JSON-RPC -32601)
  - invalid_reference_uri (unexpected scheme or path traversal attempt)
  - progress_token_mismatch (notifications/progress token does not map to ai_jobs)
- UI_GUARDRAILS:
  - N/A for this WP (backend evidence + tests only; UI work is out of scope)
- VALIDATOR_ASSERTIONS:
  - Validator can point to file:line evidence that required MCP events are recorded with correct event_kind + correlation fields per SPEC_ANCHOR.
  - Validator can confirm durable progress persistence fields exist in SQLite and are written during the end-to-end test flow.

## IMPLEMENTATION
- Durable progress mapping: reserve a progress token per tools/call and persist (mcp_server_id, mcp_call_id, mcp_progress_token) to SQLite ai_jobs when a DB handle is available.
- Progress telemetry: bind notifications/progress tokens to job/workflow context and record mcp.progress Flight Recorder events with correlation fields.
- Reference hydration: implement host-side ref:// hydration with allowed-roots canonicalization + symlink rejection; reject file:// and unknown schemes; emit notifications/resource_released after hydration.
- JSON-RPC support: add caller-supplied request IDs (string IDs) so tools/call id can align with the durable progress token.
- E2E test: add an in-process MCP stub (DuplexTransport) that exercises deny-by-default allowlist, durable progress persistence + lookup, ref hydration + release, and required Flight Recorder event kinds.
- SQLite fix: correct list_ai_jobs QueryBuilder projections so FromRow mapping works (needed for the E2E sanity check).

## HYGIENE
- Ran: just pre-work WP-1-MCP-End-to-End-v2
- Ran: cd src/backend/handshake_core; cargo test -j 1 --test mcp_e2e_tests
- Ran: cd src/backend/handshake_core; cargo test -j 1 (workaround for Windows incremental file-lock issues with default parallelism)
- Anti-vibe spot-check: rg -n "\\.unwrap\\(" src/backend/handshake_core/src/mcp src/backend/handshake_core/src/storage (no matches)

## VALIDATION
- **Target File**: `src/backend/handshake_core/src/mcp/client.rs`
- **Start**: 1
- **End**: 306
- **Line Delta**: 56
- **Pre-SHA1**: `fbd671e0173d4825565b38b492ae3ed87694d712`
- **Post-SHA1**: `5c48a339ded902acda20ddfa96df82a7121a8116`
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
- **End**: 361
- **Line Delta**: 85
- **Pre-SHA1**: `a94760b4a8ab0fb3526c262e62744b2a2168c820`
- **Post-SHA1**: `53cf0b9007e08517c547e1bdf83ffbe482785dd1`
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
- **End**: 731
- **Line Delta**: 203
- **Pre-SHA1**: `d0f8d8d2b35646c0125c6b7a5aed8f0f812c3850`
- **Post-SHA1**: `f4275d3da4404bbe0183fe0f121e639d80796fb5`
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

- **Target File**: `src/backend/handshake_core/src/storage/mod.rs`
- **Start**: 1
- **End**: 1581
- **Line Delta**: 36
- **Pre-SHA1**: `bdebf6460660ffcf5a6efbc1609007f17abef1d1`
- **Post-SHA1**: `76c93729938ff7a30750e2d69425235ac4c287b2`
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

- **Target File**: `src/backend/handshake_core/src/storage/sqlite.rs`
- **Start**: 1
- **End**: 2837
- **Line Delta**: 112
- **Pre-SHA1**: `dcd3454d782f76d23c5dc9988e05e71a1776a548`
- **Post-SHA1**: `0b8715ab4d5cb77f68d0a2100c0d620c8c343660`
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

- **Target File**: `src/backend/handshake_core/tests/mcp_e2e_tests.rs`
- **Start**: 1
- **End**: 415
- **Line Delta**: 415
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `2562ddd0e17c6d330651368b2e94f03411600750`
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

- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.126.md
- **Notes**: Range base for post-work: e048533f2ddbfbef1f14aa8de5dc75eb8dc2c51b

## STATUS_HANDOFF
- Current WP_STATUS: Implementation complete; pending commit + post-work gate (2026-02-17)
- What changed in this update:
  - MCP Gate: durable progress mapping + progress telemetry; ref:// hydration + release; scheme rejection for file://.
  - SQLite: ai_jobs mcp_* columns + index; DB methods to update/get/find MCP fields.
  - Tests: new end-to-end MCP test covering tool_call/tool_result, progress, logging/message, and ref hydration.
- Next step / handoff hint:
  - Commit changes on feat/WP-1-MCP-End-to-End-v2.
  - Run: just cargo-clean
  - Run: just post-work WP-1-MCP-End-to-End-v2 --range e048533f2ddbfbef1f14aa8de5dc75eb8dc2c51b..HEAD
  - Request Validator audit in ## VALIDATION_REPORTS (append-only).

## EVIDENCE_MAPPING
- REQUIREMENT: "A single end-to-end MCP-backed job is exercised by tests and produces a tool call + tool result through the Gate (deny-by-default enforced; allow path explicit)."
  - EVIDENCE: `src/backend/handshake_core/src/mcp/gate.rs:484`
  - EVIDENCE: `src/backend/handshake_core/src/mcp/fr_events.rs:79`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_e2e_tests.rs:395`

- REQUIREMENT: "Reference-based binary protocol is implemented for at least one test fixture: server returns ref://... and host resolves it without trusting file:// URIs; host emits the corresponding release notification/event per spec intent."
  - EVIDENCE: `src/backend/handshake_core/src/mcp/gate.rs:241`
  - EVIDENCE: `src/backend/handshake_core/src/mcp/gate.rs:717`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_e2e_tests.rs:359`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_e2e_tests.rs:366`

- REQUIREMENT: "Durable progress mapping exists: MCP notifications/progress token is persisted to SQLite ai_jobs fields (mcp_server_id, mcp_call_id, mcp_progress_token) and can be correlated back to the originating job/workflow_run_id."
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:1483`
  - EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:2401`
  - EVIDENCE: `src/backend/handshake_core/src/mcp/gate.rs:624`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_e2e_tests.rs:345`

- REQUIREMENT: "Flight Recorder evidence exists for MCP traffic with event_kind including at least: mcp.tool_call, mcp.tool_result, mcp.progress, and mcp.logging, with correlation fields present per spec anchor."
  - EVIDENCE: `src/backend/handshake_core/src/mcp/fr_events.rs:79`
  - EVIDENCE: `src/backend/handshake_core/src/mcp/fr_events.rs:140`
  - EVIDENCE: `src/backend/handshake_core/src/mcp/fr_events.rs:205`
  - EVIDENCE: `src/backend/handshake_core/src/mcp/fr_events.rs:238`
  - EVIDENCE: `src/backend/handshake_core/tests/mcp_e2e_tests.rs:395`

- REQUIREMENT: "Red-team hardening is implemented for symlink/root bypass and sampling/createMessage injection paths per spec anchor (canonicalization/no-follow; untrusted fencing; default-deny for undeclared capabilities)."
  - EVIDENCE: `src/backend/handshake_core/src/mcp/security.rs:38`
  - EVIDENCE: `src/backend/handshake_core/src/mcp/gate.rs:465`
  - EVIDENCE: `src/backend/handshake_core/src/mcp/gate.rs:171`

## EVIDENCE
- COMMAND: `just pre-work WP-1-MCP-End-to-End-v2`
  - EXIT_CODE: 0

- COMMAND: `cd src/backend/handshake_core; cargo test -j 1 --test mcp_e2e_tests`
  - EXIT_CODE: 0
  - PROOF_LINES:
    - `test mcp_e2e_persists_progress_mapping_records_fr_events_and_hydrates_ref ... ok`

- COMMAND: `cd src/backend/handshake_core; cargo test -j 1`
  - EXIT_CODE: 0
  - PROOF_LINES:
    - `running 178 tests`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
