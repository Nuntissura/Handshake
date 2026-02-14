# Task Packet: WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2

## METADATA
- TASK_ID: WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2
- WP_ID: WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2
- BASE_WP_ID: WP-1-Spec-Enrichment-MT-ContextPack-Defaults (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-14T17:05:24.845Z
- MERGE_BASE_SHA: fadbbeb81693b7aa82ecd7eb8eca78dfc28c0049
- REQUESTOR: ilja (Operator)
- AGENT_ID: CodexCLI-GPT-5.2
- ROLE: Orchestrator
- AGENTIC_MODE: NO
- ORCHESTRATOR_MODEL: N/A
- ORCHESTRATION_STARTED_AT_UTC: N/A
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: LOW
- USER_SIGNATURE: ilja140220261758
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Land Master Spec v02.126 Phase 1 defaults for MT ContextPacks usage (SourceRef-first targeting, ContextPackPolicy knobs, stale handling semantics, anchors-first minimum payload) and bind MT Context Compilation Pipeline step 6 to these defaults + FreshnessGuard observability tokens.
- Why: Removes ambiguity for MT Context compilation (ContextPacks vs Shadow Workspace fallback) and unblocks WP-1-Model-Onboarding-ContextPacks-v1 by making default targeting + staleness outcomes normative and observable.
- IN_SCOPE_PATHS:
  - Handshake_Master_Spec_v02.126.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/refinements/WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v1.md
  - .GOV/refinements/WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2.md
  - .GOV/task_packets/WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2.md
  - .GOV/roles_shared/TASK_BOARD.md
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - .GOV/roles_shared/SIGNATURE_AUDIT.md
  - .GOV/roles/orchestrator/ORCHESTRATOR_GATES.json
- OUT_OF_SCOPE:
  - Any product code changes under `src/`, `app/`, or `tests/`
  - Implementing ContextPacks refresh/builders or MT runtime behavior (tracked by implementation WPs, e.g., WP-1-Model-Onboarding-ContextPacks-v1)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2

# Spec + governance checks:
just validator-spec-regression
just gov-check
```

### DONE_MEANS
- `Handshake_Master_Spec_v02.126.md` includes the Phase 1 defaults and bindings in:
  - §2.6.6.7.14.7 ContextPacks
  - §2.6.6.7.14.11 ContextPackFreshnessGuard
  - §2.6.6.8.8.2 MT Context Compilation Pipeline (Step 6)
- `.GOV/roles_shared/SPEC_CURRENT.md` points to `Handshake_Master_Spec_v02.126.md` with updated date.
- `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md` maps BASE_WP_ID -> active packet `.GOV/task_packets/WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2.md`.
- `.GOV/roles_shared/TASK_BOARD.md` lists `WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2` in `## Ready for Dev`.
- `just pre-work WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2` PASS.
- `just validator-spec-regression` PASS and `just gov-check` PASS.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.126.md (recorded_at: 2026-02-14T17:05:24.845Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.126.md §2.6.6.7.14.7 / §2.6.6.7.14.11 / §2.6.6.8.8.2
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.
- Prior packets for BASE_WP_ID: NONE (v1 was refinement-only spec enrichment; no official task packet existed).

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.126.md
  - .GOV/refinements/WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2.md
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - .GOV/roles_shared/TASK_BOARD.md
- SEARCH_TERMS:
  - "ContextPackPolicy"
  - "ContextPackFreshnessGuard"
  - "MTContextCompilationPipeline"
  - "context_pack_stale:"
- RUN_COMMANDS:
  ```bash
  rg -n "2\\.6\\.6\\.7\\.14\\.7|2\\.6\\.6\\.7\\.14\\.11|2\\.6\\.6\\.8\\.8\\.2" Handshake_Master_Spec_v02.126.md
  just validator-spec-regression
  just gov-check
  just pre-work WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2
  ```
- RISK_MAP:
  - "SPEC_CURRENT mismatch" -> "Refinement/packet gates fail; coder cannot pass pre-work."
  - "Traceability not updated" -> "TASK_BOARD/registry drift; active packet mapping ambiguous."
  - "Partial landing of v02.126 changes" -> "Implementation WPs remain blocked/ambiguous on staleness handling."

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: NO
- TRUST_BOUNDARY: N/A
- SERVER_SOURCES_OF_TRUTH:
  - N/A
- REQUIRED_PROVENANCE_FIELDS:
  - N/A
- VERIFICATION_PLAN:
  - N/A
- ERROR_TAXONOMY_PLAN:
  - N/A
- UI_GUARDRAILS:
  - N/A
- VALIDATOR_ASSERTIONS:
  - N/A

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
  - LOG_PATH: `.handshake/logs/WP-1-Spec-Enrichment-MT-ContextPack-Defaults-v2/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
