# Task Packet: WP-1-Spec-Enrichment-Product-Governance-Consistency-v1

## METADATA
- TASK_ID: WP-1-Spec-Enrichment-Product-Governance-Consistency-v1
- WP_ID: WP-1-Spec-Enrichment-Product-Governance-Consistency-v1
- BASE_WP_ID: WP-1-Spec-Enrichment-Product-Governance-Consistency (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-12T03:19:27.380Z
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
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja120220260342
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Spec-Enrichment-Product-Governance-Consistency-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Perform a spec-only consistency correction so the Master Spec no longer cites repo-local `docs/TASK_BOARD.md` or `docs/task_packets/{WP_ID}.md` as runtime sources of truth, and so all runtime governance state references align with the repo/runtime boundary rules and the canonical runtime governance root `.handshake/gov/`.
- Why: Spec internal drift reintroduces confusion between repo governance workspace and product runtime governance state, which causes recurring boundary regressions and non-portable runtime behavior.
- IN_SCOPE_PATHS:
  - .GOV/task_packets/WP-1-Spec-Enrichment-Product-Governance-Consistency-v1.md
  - .GOV/task_packets/stubs/WP-1-Spec-Enrichment-Product-Governance-Consistency-v1.md
  - .GOV/refinements/WP-1-Spec-Enrichment-Product-Governance-Consistency-v1.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.125.md
- OUT_OF_SCOPE:
  - Any product code changes (this WP is spec-only)
  - Any weakening of the repo/runtime boundary rule (runtime MUST NOT read/write `/.GOV/**`)
  - Refactoring repo governance workflows or scripts beyond what is required to keep spec references consistent

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Spec-Enrichment-Product-Governance-Consistency-v1

# Spec regression (must PASS after new spec version is created and SPEC_CURRENT updated):
just validator-spec-regression

# Mechanical checks for legacy runtime path references (run on the new spec file):
rg -n "docs/TASK_BOARD\\.md|docs/task_packets/" Handshake_Master_Spec_v*.md -S
rg -n "\\.handshake/gov/" Handshake_Master_Spec_v*.md -S

just cargo-clean
just post-work WP-1-Spec-Enrichment-Product-Governance-Consistency-v1 --range fadbbeb81693b7aa82ecd7eb8eca78dfc28c0049..HEAD
```

### DONE_MEANS
- A new Master Spec version file is created (v02.126 or next) and `.GOV/roles_shared/SPEC_CURRENT.md` is updated to point to it.
- The updated spec no longer cites `docs/TASK_BOARD.md` or `docs/task_packets/{WP_ID}.md` as runtime sources of truth for Locus/task tracking.
- The updated spec uses one consistent vocabulary: repo governance workspace `/.GOV/**` vs product runtime governance state root `.handshake/gov/`, with no contradictions.
- `just validator-spec-regression` + `just post-work WP-1-Spec-Enrichment-Product-Governance-Consistency-v1 --range fadbbeb81693b7aa82ecd7eb8eca78dfc28c0049..HEAD` PASS.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.125.md (recorded_at: 2026-02-12T03:19:27.380Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md 2.3.15 (Locus Work Tracking System; Task Board/Packet refs) + repo/runtime boundary (HARD) + runtime root `.handshake/gov/`
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- BASE_WP_ID: WP-1-Spec-Enrichment-Product-Governance-Consistency
- v1 (THIS PACKET; activated from stub):
  - Stub source: .GOV/task_packets/stubs/WP-1-Spec-Enrichment-Product-Governance-Consistency-v1.md
  - Prior official packets: NONE
  - Purpose of v1: first activation into an executable packet with signed refinement + deterministic gates.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - .GOV/refinements/WP-1-Spec-Enrichment-Product-Governance-Consistency-v1.md
  - .GOV/task_packets/stubs/WP-1-Spec-Enrichment-Product-Governance-Consistency-v1.md
  - Handshake_Master_Spec_v02.125.md (anchors: 2.3.15 + boundary rules section containing `.handshake/gov/`)
- SEARCH_TERMS:
  - "docs/TASK_BOARD.md"
  - "docs/task_packets/"
  - "Repo/runtime boundary (HARD)"
  - ".handshake/gov/"
  - "locus_sync_task_board"
- RUN_COMMANDS:
  ```bash
  rg -n "docs/TASK_BOARD\\.md|docs/task_packets/" Handshake_Master_Spec_v02.125.md -S
  rg -n "Repo/runtime boundary|\\.handshake/gov/" Handshake_Master_Spec_v02.125.md -S
  ```
- RISK_MAP:
  - "Partial spec edit" -> "Leaves contradictory examples; boundary regressions recur."
  - "Drive-specific paths" -> "Spec examples accidentally include machine-local absolute paths; breaks determinism."
  - "Unclear artifact refs" -> "Task packet refs remain repo-local paths; Locus integration stays non-portable."

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
  - LOG_PATH: `.handshake/logs/WP-1-Spec-Enrichment-Product-Governance-Consistency-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
