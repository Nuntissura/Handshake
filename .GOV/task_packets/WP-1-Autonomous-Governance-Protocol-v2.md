# Task Packet: WP-1-Autonomous-Governance-Protocol-v2

## METADATA
- TASK_ID: WP-1-Autonomous-Governance-Protocol-v2
- WP_ID: WP-1-Autonomous-Governance-Protocol-v2
- BASE_WP_ID: WP-1-Autonomous-Governance-Protocol (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-19T14:59:33.593Z
- MERGE_BASE_SHA: b9d96a0019ffac9308968cb51ed0f7735c04f3b2
- REQUESTOR: Operator (ilja)
- AGENT_ID: codex-cli:gpt-5.2 (orchestrator)
- ROLE: Orchestrator
- AGENTIC_MODE: NO
- ORCHESTRATOR_MODEL: N/A
- ORCHESTRATION_STARTED_AT_UTC: N/A
- CODER_MODEL: codex-cli:gpt-5.2
- CODER_REASONING_STRENGTH: LOW
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja190220261548
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Autonomous-Governance-Protocol-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement the Master Spec v02.132 Autonomous Governance Protocol: AutomationLevel enforcement and normalization; GovernanceDecision + AutoSignature artifacts for gate approvals; FR-EVT-GOV-001..005 emission + schema validation; LOCKED semantics and cloud escalation denial.
- Why: Required to make autonomous/hybrid workflows auditable and deterministic, and to align the imported Stage spec blocks to the canonical governance schemas and event IDs.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/models.rs
  - src/backend/handshake_core/src/capabilities.rs
- OUT_OF_SCOPE:
  - Any new Master Spec changes (already completed in v02.132)
  - Any new Flight Recorder governance event IDs beyond FR-EVT-GOV-001..005
  - Allowing AutoSignature for cloud escalation or policy violations
  - Any UI/UX work not required for conformance (Stage/App surfaces)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Autonomous-Governance-Protocol-v2

# Backend format/lint/test:
just fmt-backend
just lint-backend
just test-backend

# (If applicable) any targeted tests added for governance automation events:
# cd src/backend/handshake_core && cargo test <name>

just cargo-clean
just post-work WP-1-Autonomous-Governance-Protocol-v2 --range b9d96a0019ffac9308968cb51ed0f7735c04f3b2..HEAD
```

### DONE_MEANS
- AutomationLevel canonicalization implemented per Spec 2.6.8.12.6.1 (including legacy ASSISTED/SUPERVISED normalization to HYBRID) and LOCKED fail-closed semantics.
- GovernanceDecision artifact is produced for every autonomous/hybrid gate approval per 2.6.8.12.3 and linked into Flight Recorder via decision_id + gate_type + target_ref.
- AutoSignature artifact is implemented per 2.6.8.12.6.3; binding checks enforced; forbidden for cloud escalation and policy violations.
- FR-EVT-GOV-001..005 events are emitted for governance automation decisions and schema-validated at ingestion per 11.5.7; no new event IDs are introduced.
- Cloud escalation remains explicitly human-gated and is denied in LOCKED per 11.1.7.3 and 2.6.8.12.6.1.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.132.md (recorded_at: 2026-02-19T14:59:33.593Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.132.md 2.6.8.12 (Autonomous Governance Protocol) + 2.6.8.12.6 (Canonicalization) + 11.5.7 (FR-EVT-GOV-001..005) + 11.1.7.3 (Cloud escalation rules) + 10.13 merge alignment note
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior stub/planning packet: `.GOV/task_packets/stubs/WP-1-Autonomous-Governance-Protocol-v1.md` (STUB; not activated).
  - Preserved: implement AutomationLevel gating, GovernanceDecision artifacts, AutoSignature constraints, and FR-EVT-GOV-* emission with cloud escalation always human-gated.
  - Changed/clarified: anchored to Master Spec v02.132 canonicalization (2.6.8.12.6) to resolve cross-section conflicts and pin LOCKED semantics + Stage import alignment.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.132.md
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/runtime_governance.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
- SEARCH_TERMS:
  - "AutomationLevel"
  - "governance decision"
  - "AutoSignature"
  - "gov_decision_created"
  - "gov_auto_signature_created"
  - "FR-EVT-GOV"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Autonomous-Governance-Protocol-v2
  ```
- RISK_MAP:
  - "AutoSignature abuse" -> "unsafe gate satisfaction (cloud/policy) if binding checks are missing"
  - "LOCKED semantics drift" -> "system proceeds without review; must fail-closed"
  - "PII/data leakage" -> "decision rationale/evidence exported; events must be refs only"

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: workflow/gate_evaluation -> decision artifact -> apply (server-side verification; no trust in client/UI)
- SERVER_SOURCES_OF_TRUTH:
  - GovernanceDecision + AutoSignature artifacts (decision_id, gate_type, target_ref) loaded and verified server-side before applying
  - Runtime GovernanceMode + AutomationLevel (resolved from Work Profile / session policy) pinned per job/session
- REQUIRED_PROVENANCE_FIELDS:
  - decision_id, gate_type, target_ref, automation_level, actor.kind/model_id, timestamps, evidence_refs (refs only)
- VERIFICATION_PLAN:
  - Validate FR event payloads at ingestion; reject unknown types/ids
  - Validate AutoSignature binding to GovernanceDecision (decision_id + gate_type + target_ref) before applying
- ERROR_TAXONOMY_PLAN:
  - stale/mismatch (binding mismatch) vs invalid schema/payload vs forbidden gate (cloud/policy) vs LOCKED fail-closed halt
- UI_GUARDRAILS:
  - N/A (Phase 1: backend enforcement; UI changes only if required to unblock gate flows)
- VALIDATOR_ASSERTIONS:
  - Spec anchors implemented: 2.6.8.12.3/2.6.8.12.4/2.6.8.12.6; 11.5.7; 11.1.7.3
  - FR-EVT-GOV event schema validation rejects malformed payloads
  - AutoSignature is forbidden for cloud escalation and policy violations

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
- Current WP_STATUS: BOOTSTRAP (coder started; gates executed; reviewing spec/task packet for skeleton)
- What changed in this update: No product code changes yet. Environment unblocked; worktree/branch verified; pre-work gate run.
- Next step / handoff hint: Draft `## SKELETON` (types/contracts + FR payload validation plan) + docs-only skeleton checkpoint commit; wait for Validator "SKELETON APPROVED" before implementation.

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
  - LOG_PATH: `.handshake/logs/WP-1-Autonomous-Governance-Protocol-v2/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
