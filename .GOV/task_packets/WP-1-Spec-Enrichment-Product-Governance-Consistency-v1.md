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
- CODER_MODEL: GPT-5.2
- CODER_REASONING_STRENGTH: HIGH (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** In Progress
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
  - Handshake_Master_Spec_v02.126.md
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
  - Handshake_Master_Spec_v02.125.md (baseline anchors: 2.3.15 + boundary rules section containing `.handshake/gov/`)
  - Handshake_Master_Spec_v02.126.md (verify corrected refs)
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
- N/A (spec-only consistency correction)
- Open questions:
  - None
- Notes:
  - Create new Master Spec version file (v02.126) and update `.GOV/roles_shared/SPEC_CURRENT.md`.
  - Replace stale runtime work-tracking path examples so Locus/runtime governance refs use `.handshake/gov/` (not repo-local `docs/` paths).

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
- Created `Handshake_Master_Spec_v02.126.md` from v02.125 and updated Locus/work-tracking + governance event path examples to use runtime governance root `.handshake/gov/`.
- Updated `.GOV/roles_shared/SPEC_CURRENT.md` to reference `Handshake_Master_Spec_v02.126.md`.

## HYGIENE
- Commands run (see ## EVIDENCE for outputs):
  - `just pre-work WP-1-Spec-Enrichment-Product-Governance-Consistency-v1`
  - `rg -n "docs/TASK_BOARD\\.md|docs/task_packets/" Handshake_Master_Spec_v02.126.md -S`
  - `rg -n "\\.handshake/gov/" Handshake_Master_Spec_v02.126.md -S`
  - `just validator-spec-regression`
  - `just cargo-clean`

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `Handshake_Master_Spec_v02.126.md`
- **Start**: 1
- **End**: 62681
- **Line Delta**: 62681
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `7260b4ada693263799ff39dd909653863cf0e503`
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
- **Lint Results**:
- **Artifacts**:
- **Timestamp**: 2026-02-12T11:27:26.0244118+01:00
- **Operator**: ilja
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.126.md
- **Notes**: New file; preimage does not exist in HEAD so Pre-SHA1 is a sentinel value.

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress
- What changed in this update:
  - `Handshake_Master_Spec_v02.126.md`: spec-only consistency correction (work-tracking + governance path examples).
  - `.GOV/roles_shared/SPEC_CURRENT.md`: points to v02.126.
  - `.GOV/task_packets/WP-1-Spec-Enrichment-Product-Governance-Consistency-v1.md`: claimed coder fields + updated scope + recorded evidence/manifest.
- Next step / handoff hint:
  - Stage changes and run `just post-work WP-1-Spec-Enrichment-Product-Governance-Consistency-v1` (and then `--range {MERGE_BASE_SHA}..HEAD` after commit).

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "A new Master Spec version file is created (v02.126 or next) and `.GOV/roles_shared/SPEC_CURRENT.md` is updated to point to it."
  - EVIDENCE: `Handshake_Master_Spec_v02.126.md:37`
  - EVIDENCE: `.GOV/roles_shared/SPEC_CURRENT.md:5`
  - REQUIREMENT: "The updated spec no longer cites repo-local docs paths as runtime sources of truth for Locus/task tracking."
  - EVIDENCE: `Handshake_Master_Spec_v02.126.md:5407`
  - EVIDENCE: `Handshake_Master_Spec_v02.126.md:5408`
  - REQUIREMENT: "Repo governance workspace `/.GOV/**` vs runtime governance state root `.handshake/gov/` vocabulary is consistent."
  - EVIDENCE: `Handshake_Master_Spec_v02.126.md:28515`
  - EVIDENCE: `Handshake_Master_Spec_v02.126.md:28519`
  - REQUIREMENT: "Spec regression + deterministic gates executed per TEST_PLAN."
  - EVIDENCE: `.GOV/task_packets/WP-1-Spec-Enrichment-Product-Governance-Consistency-v1.md:215`
  - EVIDENCE: `.GOV/task_packets/WP-1-Spec-Enrichment-Product-Governance-Consistency-v1.md:223`

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `just pre-work WP-1-Spec-Enrichment-Product-Governance-Consistency-v1`
  - EXIT_CODE: 0
  - PROOF_LINES: `Pre-work validation PASSED`

  - COMMAND: `rg -n "docs/TASK_BOARD\\.md|docs/task_packets/" Handshake_Master_Spec_v02.126.md -S`
  - EXIT_CODE: 1

  - COMMAND: `just validator-spec-regression`
  - EXIT_CODE: 0
  - PROOF_LINES: `validator-spec-regression: PASS`

  - COMMAND: `just cargo-clean`
  - EXIT_CODE: 0
  - PROOF_LINES: `Removed 0 files`

  - COMMAND: `just post-work WP-1-Spec-Enrichment-Product-Governance-Consistency-v1`
  - EXIT_CODE: 0
  - PROOF_LINES: `Post-work validation PASSED`

  - COMMAND: `just post-work WP-1-Spec-Enrichment-Product-Governance-Consistency-v1 --range fadbbeb81693b7aa82ecd7eb8eca78dfc28c0049..HEAD`
  - EXIT_CODE: 0
  - PROOF_LINES: `Post-work validation PASSED`
  - LOG_PATH: `.handshake/logs/WP-1-Spec-Enrichment-Product-Governance-Consistency-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
