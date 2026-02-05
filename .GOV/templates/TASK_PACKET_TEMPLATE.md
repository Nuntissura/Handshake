# TASK_PACKET_TEMPLATE

Copy this into each new task packet and fill all fields.

Requirements:
- Keep packets ASCII-only (required by deterministic gates).
- Use SPEC_BASELINE for provenance (spec at creation time).
- Use SPEC_TARGET as the authoritative spec for closure/revalidation (usually .GOV/roles_shared/SPEC_CURRENT.md).
- WP_ID and filename MUST NOT include date/time stamps; use `-v{N}` for revisions (e.g., `WP-1-Tokenization-Service-v3`).
- If multiple packets exist for the same Base WP, update `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md` (Base WP -> Active Packet).

---

# Task Packet: {{WP_ID}}

## METADATA
- TASK_ID: {{WP_ID}}
- WP_ID: {{WP_ID}}
- BASE_WP_ID: {{WP_ID}} (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: {{DATE_ISO}}
- MERGE_BASE_SHA: {{MERGE_BASE_SHA}} (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
- REQUESTOR: {{REQUESTOR}}
- AGENT_ID: {{AGENT_ID}}
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: LOW | MEDIUM | HIGH
- USER_SIGNATURE: {{USER_SIGNATURE}}
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/{{WP_ID}}.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What:
- Why:
- IN_SCOPE_PATHS:
  - path/to/file
- OUT_OF_SCOPE:
  - out/of/scope/path

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work {{WP_ID}}
# ...task-specific commands...
just cargo-clean
just post-work {{WP_ID}} --range {{MERGE_BASE_SHA}}..HEAD
```

### DONE_MEANS
- measurable criterion 1
- measurable criterion 2

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: {{SPEC_BASELINE}} (recorded_at: {{DATE_ISO}})
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: {{SPEC_ANCHOR}}
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
  - LOG_PATH: `.handshake/logs/{{WP_ID}}/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)


