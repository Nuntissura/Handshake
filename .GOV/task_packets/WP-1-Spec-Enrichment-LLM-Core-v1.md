# Task Packet: WP-1-Spec-Enrichment-LLM-Core-v1

## METADATA
- TASK_ID: WP-1-Spec-Enrichment-LLM-Core-v1
- WP_ID: WP-1-Spec-Enrichment-LLM-Core-v1
- DATE: 2026-01-04T00:10:12.232Z
- REQUESTOR: User
- AGENT_ID: orchestrator-gpt-5.2
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja040120260108

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Spec-Enrichment-LLM-Core-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Apply the approved spec enrichment for LLM core trace_id transport and llm_inference event shape, then bump the Master Spec version and update .GOV/roles_shared/SPEC_CURRENT.md.
- Why: This unblocks a clean redo of the LLM core WP under a spec that is explicitly and deterministically covered.
- IN_SCOPE_PATHS:
  - Handshake_Master_Spec_v02.100.md
  - Handshake_Master_Spec_v02.101.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/refinements/WP-1-Spec-Enrichment-LLM-Core-v1.md
  - .GOV/task_packets/WP-1-Spec-Enrichment-LLM-Core-v1.md
  - .GOV/roles_shared/SIGNATURE_AUDIT.md
  - .GOV/roles/orchestrator/ORCHESTRATOR_GATES.json
- OUT_OF_SCOPE:
  - Any product code changes in src/ or app/
  - Any unrelated spec edits outside the approved verbatim enrichment text

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Spec-Enrichment-LLM-Core-v1

# Spec regression:
just validator-spec-regression

# Deterministic gate:
just cargo-clean
just post-work WP-1-Spec-Enrichment-LLM-Core-v1
```

### DONE_MEANS
- Master Spec version bump created: Handshake_Master_Spec_v02.101.md exists and includes the approved verbatim enrichment text.
- SPEC_CURRENT updated: .GOV/roles_shared/SPEC_CURRENT.md points to Handshake_Master_Spec_v02.101.md.
- Audit trail: USER_SIGNATURE is present in .GOV/roles_shared/SIGNATURE_AUDIT.md and gate logs (.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json) for this WP.
- Workflow gates: just pre-work and just post-work for this WP pass.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.100.md (recorded_at: 2026-01-04T00:10:12.232Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 4.2.3, 11.5
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.100.md
  - .GOV/refinements/WP-1-Spec-Enrichment-LLM-Core-v1.md
- SEARCH_TERMS:
  - "4.2.3 LLM Client Adapter"
  - "CompletionRequest"
  - "trace_id"
  - "llm_inference"
  - "11.5 Flight Recorder Event Shapes"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Spec-Enrichment-LLM-Core-v1
  just validator-spec-regression
  just cargo-clean
  just post-work WP-1-Spec-Enrichment-LLM-Core-v1
  ```
- RISK_MAP:
  - "Edits diverge from approved verbatim enrichment" -> "Protocol violation; redo required"
  - "SPEC_CURRENT not updated" -> "Downstream WPs anchor wrong spec"

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
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

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)


