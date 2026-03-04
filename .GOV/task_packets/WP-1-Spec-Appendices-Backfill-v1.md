# Task Packet: WP-1-Spec-Appendices-Backfill-v1

## METADATA
- TASK_ID: WP-1-Spec-Appendices-Backfill-v1
- WP_ID: WP-1-Spec-Appendices-Backfill-v1
- BASE_WP_ID: WP-1-Spec-Appendices-Backfill
- DATE: 2026-03-04T19:45:01.142Z
- MERGE_BASE_SHA: e5bd640599ece9cb95e3f4770c700c4212da493d
- REQUESTOR: Ilja Smets
- AGENT_ID: CodexCLI (Orchestrator)
- ROLE: Orchestrator
- AGENTIC_MODE: YES
<!-- Allowed: YES | NO -->
- ORCHESTRATOR_MODEL: GPT-5.2
<!-- Required if AGENTIC_MODE=YES -->
- ORCHESTRATION_STARTED_AT_UTC: 2026-03-04T19:51:52.0434183Z
<!-- RFC3339 UTC; required if AGENTIC_MODE=YES -->
- CODER_MODEL: N/A (Orchestrator executes)
- CODER_REASONING_STRENGTH: N/A
<!-- Allowed: LOW | MEDIUM | HIGH | EXTRA_HIGH -->
- **Status:** Ready for Dev
- RISK_TIER: MEDIUM
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: GOV
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: NO
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: NONE
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: NONE
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- USER_SIGNATURE: ilja040320262011
- PACKET_FORMAT_VERSION: 2026-02-01

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: N/A

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: ALLOWED
- OPERATOR_APPROVAL_EVIDENCE: use agents to speed up the process.
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Spec-Appendices-Backfill-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Backfill Master Spec EOF appendices (Section 12) with a comprehensive, machine-readable feature registry + primitive/tool/tech matrix + per-feature UI guidance + interaction matrix.
- Why: Reduce drift and cognitive load by giving humans/LLMs an in-spec index and interaction map that remains self-contained and anchored to Main Body truth.
- IN_SCOPE_PATHS:
  - Handshake_Master_Spec_v02.141.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/BUILD_ORDER.md
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - .GOV/roles_shared/TASK_BOARD.md
  - .GOV/roles_shared/SIGNATURE_AUDIT.md
  - .GOV/roles/orchestrator/ORCHESTRATOR_GATES.json
  - .GOV/task_packets/WP-1-Spec-Appendices-Backfill-v1.md
  - .GOV/refinements/WP-1-Spec-Appendices-Backfill-v1.md
- OUT_OF_SCOPE:
  - src/** (product code)
  - app/** (product code)
  - tests/** (product code)
  - Spec Main Body changes outside Section 12, except version bump + minimal cross-references if required

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Spec-Appendices-Backfill-v1
# ...task-specific commands...
just cargo-clean
just post-work WP-1-Spec-Appendices-Backfill-v1 --range e5bd640599ece9cb95e3f4770c700c4212da493d..HEAD
```

### DONE_MEANS
- Master Spec has updated EOF appendix blocks with valid JSON and stable IDs:
  - HS-APPX-FEATURE-REGISTRY lists Phase 1 features with stable `feature_id` values and spec anchors.
  - HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX includes current primitives/tools/technologies and links to features.
  - HS-APPX-UI-GUIDANCE has per-feature entries for Phase 1 user-facing features (legacy backfill).
  - HS-APPX-INTERACTION-MATRIX includes initial high-signal edges (feature<->feature and feature<->primitive/tool/tech).
- `just spec-eof-appendices-check` and `just gov-check` pass against `SPEC_CURRENT`.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.140.md (recorded_at: 2026-03-04T19:45:01.142Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Master Spec Section 12 [CX-SPEC-APPX-001] + [CX-SPEC-APPX-003] + [CX-SPEC-APPX-010] + [CX-SPEC-APPX-011] + [CX-SPEC-APPX-012] + [CX-SPEC-APPX-013]
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
  - path/to/file
- SEARCH_TERMS:
  - "exact symbol"
  - "error code"
- RUN_COMMANDS:
  ```bash
  # task-specific commands
  ```
- RISK_MAP:
  - "risk name" -> "impact"

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: NO

## IMPLEMENTATION
- (Coder fills after the docs-only skeleton checkpoint commit exists.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `Handshake_Master_Spec_v02.141.md`
- **Start**: 1
- **End**: 73553
- **Line Delta**: 73553
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `d01677a72b79523fef93b6d4072ebc5e0ec4b019`
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
- **Lint Results**: N/A (docs-only)
- **Artifacts**: `just spec-eof-appendices-check` PASS; `just gov-check` PASS
- **Timestamp**: 2026-03-04T21:20:51Z
- **Operator**: Ilja Smets (Operator); CodexCLI (Orchestrator)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.141.md
- **Notes**: New spec version file with populated EOF appendices (Section 12).

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: IMPLEMENTED (docs-only) - READY_FOR_VALIDATION
- What changed in this update:
  - Backfilled Master Spec EOF appendices in `Handshake_Master_Spec_v02.141.md` (feature registry, matrix, UI guidance, interaction matrix).
  - Updated `.GOV/roles_shared/SPEC_CURRENT.md` to point at `Handshake_Master_Spec_v02.141.md`.
  - Synced `.GOV/roles_shared/BUILD_ORDER.md` and updated `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.
  - Updated this task packet + refinement to reflect current SPEC_CURRENT and add deterministic validation manifest + evidence.
- Next step / handoff hint:
  - Validator: run `just post-work WP-1-Spec-Appendices-Backfill-v1 --range e5bd640599ece9cb95e3f4770c700c4212da493d..HEAD` and append findings in `## VALIDATION_REPORTS`.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- REQUIREMENT: "SPEC_CURRENT points to the authoritative spec file."
  - EVIDENCE: `.GOV/roles_shared/SPEC_CURRENT.md:1`
- REQUIREMENT: "HS-APPX-FEATURE-REGISTRY lists Phase 1 features with stable feature_id values and spec anchors."
  - EVIDENCE: `Handshake_Master_Spec_v02.141.md:70272`
- REQUIREMENT: "HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX includes current primitives/tools/technologies and links to features."
  - EVIDENCE: `Handshake_Master_Spec_v02.141.md:70529`
- REQUIREMENT: "HS-APPX-UI-GUIDANCE has per-feature entries for Phase 1 user-facing features (legacy backfill)."
  - EVIDENCE: `Handshake_Master_Spec_v02.141.md:73273`
- REQUIREMENT: "HS-APPX-INTERACTION-MATRIX includes initial high-signal edges."
  - EVIDENCE: `Handshake_Master_Spec_v02.141.md:73433`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `just spec-eof-appendices-check`
  - EXIT_CODE: 0
  - PROOF_LINES: spec-eof-appendices-check ok: Handshake_Master_Spec_v02.141.md
  - COMMAND: `just gov-check`
  - EXIT_CODE: 0
  - PROOF_LINES: gov-check ok
  - COMMAND: `just post-work WP-1-Spec-Appendices-Backfill-v1 --range e5bd640599ece9cb95e3f4770c700c4212da493d..HEAD`
  - EXIT_CODE: 0
  - PROOF_LINES: RESULT: PASS

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
