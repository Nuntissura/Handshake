# Task Packet: WP-1-MCP-End-to-End-v2

## METADATA
- TASK_ID: WP-1-MCP-End-to-End-v2
- WP_ID: WP-1-MCP-End-to-End-v2
- BASE_WP_ID: WP-1-MCP-End-to-End (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-16T22:03:19.337Z
- MERGE_BASE_SHA: 0f7cfda43997ab72baf7b0150ced57d4c2600a06
- REQUESTOR: Operator (ilja)
- AGENT_ID: codex-cli (gpt-5.2)
- ROLE: Orchestrator
- AGENTIC_MODE: YES
- ORCHESTRATOR_MODEL: gpt-5.2
- ORCHESTRATION_STARTED_AT_UTC: 2026-02-16T22:03:19.337Z
- CODER_MODEL: gpt-5.2
- CODER_REASONING_STRENGTH: EXTRA_HIGH
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
- Why: Prior packet `WP-1-MCP-End-to-End` failed validation due to missing implementation. This v2 packet aligns to current Master Spec v02.126 and provides the end-to-end evidence chain needed for Phase 1 closure. This WP is downstream of `WP-1-MCP-Skeleton-Gate-v2` and remains BLOCKED until the upstream WP is VALIDATED and merged to `main`.
- IN_SCOPE_PATHS:
  - .GOV/refinements/WP-1-MCP-End-to-End-v2.md
  - .GOV/task_packets/WP-1-MCP-End-to-End-v2.md
  - .GOV/roles_shared/TASK_BOARD.md
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - src/backend/handshake_core/src/mcp/
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
just post-work WP-1-MCP-End-to-End-v2 --range 0f7cfda43997ab72baf7b0150ced57d4c2600a06..HEAD
```

### DONE_MEANS
- A single end-to-end MCP-backed job is exercised by tests and produces a tool call + tool result through the Gate (deny-by-default enforced; allow path explicit).
- Reference-based binary protocol is implemented for at least one test fixture: server returns `ref://...` and host resolves it without trusting `file://` URIs; host emits the corresponding release notification/event per spec intent.
- Durable progress mapping exists: MCP `notifications/progress` token is persisted to SQLite `ai_jobs` fields (`mcp_server_id`, `mcp_call_id`, `mcp_progress_token`) and can be correlated back to the originating job/workflow_run_id.
- Flight Recorder evidence exists for MCP traffic with event_kind including at least: `mcp.tool_call`, `mcp.tool_result` (or equivalent), `mcp.progress`, and `mcp.logging`, with correlation fields (job_id, workflow_run_id, trace_id, server_id, tool_name) present in payload or row fields per spec anchor.
- Red-team hardening is implemented for symlink/root bypass and sampling/createMessage injection paths per spec anchor (canonicalization/no-follow; untrusted fencing; default-deny for undeclared capabilities).
- `just pre-work WP-1-MCP-End-to-End-v2` and `just post-work WP-1-MCP-End-to-End-v2 --range 0f7cfda43997ab72baf7b0150ced57d4c2600a06..HEAD` both PASS.

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
- Dependency note: This packet assumes MCP client+Gate foundations from `WP-1-MCP-Skeleton-Gate-v2` exist on `main`. Until that upstream WP is VALIDATED+merged, this WP remains BLOCKED and must not start implementation.

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
  just post-work WP-1-MCP-End-to-End-v2 --range 0f7cfda43997ab72baf7b0150ced57d4c2600a06..HEAD
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
- This WP is BLOCKED until `WP-1-MCP-Skeleton-Gate-v2` is VALIDATED and merged to `main` (dependency and lock-set overlap).

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
- Current WP_STATUS: Started (2026-02-17)
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
  - LOG_PATH: `.handshake/logs/WP-1-MCP-End-to-End-v2/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
