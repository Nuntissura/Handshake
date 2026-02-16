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
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES | NO
- TRUST_BOUNDARY: <fill> (examples: client->server, server->storage, job->apply)
- SERVER_SOURCES_OF_TRUTH:
  - <fill> (what the server loads/verifies instead of trusting the client)
- REQUIRED_PROVENANCE_FIELDS:
  - <fill> (role_id, contract_id, model_id/tool_id, evidence refs, before/after spans, etc.)
- VERIFICATION_PLAN:
  - <fill> (how provenance/audit is verified and recorded; include non-spoofable checks when required)
- ERROR_TAXONOMY_PLAN:
  - <fill> (distinct error classes: stale/mismatch vs spoof attempt vs true scope violation)
- UI_GUARDRAILS:
  - <fill> (prevent stale apply; preview before apply; disable conditions)
- VALIDATOR_ASSERTIONS:
  - <fill> (what the validator must prove; spec anchors; fields present; trust boundary enforced)

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
