# Task Packet: WP-1-Runtime-Governance-NoExpect-v1

## METADATA
- TASK_ID: WP-1-Runtime-Governance-NoExpect-v1
- WP_ID: WP-1-Runtime-Governance-NoExpect-v1
- BASE_WP_ID: WP-1-Runtime-Governance-NoExpect (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-11T18:00:51.895Z
- MERGE_BASE_SHA: 84aee8219bc5ae38115af33f914d4639dbad9688 (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
- REQUESTOR: ilja (Operator) / Validator directive
- AGENT_ID: CodexCLI-GPT-5.2
- ROLE: Orchestrator
- AGENTIC_MODE: YES (YES | NO)
- ORCHESTRATOR_MODEL: GPT-5.2 (Codex CLI) (required if AGENTIC_MODE=YES)
- ORCHESTRATION_STARTED_AT_UTC: 2026-02-11T18:00:51.895Z (RFC3339 UTC; required if AGENTIC_MODE=YES)
- CODER_MODEL: CodexCLI-GPT-5.2
- CODER_REASONING_STRENGTH: MEDIUM (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: LOW
- USER_SIGNATURE: ilja110220261846
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Runtime-Governance-NoExpect-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Remove `.expect(...)` usage in runtime governance tests by refactoring them to return `Result` and propagate errors via `?` (no panics).
- Why: Satisfy Forbidden Pattern Audit requirements ([CX-573E]) and the repo's mechanical scan gate (`just product-scan`) by eliminating `expect`/`unwrap` patterns in the in-scope file.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/runtime_governance.rs
- OUT_OF_SCOPE:
  - Any runtime governance behavior changes outside of test-only error propagation
  - Changes outside `src/backend/handshake_core/src/runtime_governance.rs`

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Runtime-Governance-NoExpect-v1

# Hygiene scan gate (forbidden patterns):
just product-scan

# Backend tests:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Mechanical manifest gate:
just post-work WP-1-Runtime-Governance-NoExpect-v1 --range 84aee8219bc5ae38115af33f914d4639dbad9688..HEAD
```

### DONE_MEANS
- `src/backend/handshake_core/src/runtime_governance.rs` contains no `.expect(` and no `.unwrap(`.
- `just product-scan` passes.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` passes.
- `just post-work WP-1-Runtime-Governance-NoExpect-v1 --range 84aee8219bc5ae38115af33f914d4639dbad9688..HEAD` passes.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.125.md (recorded_at: 2026-02-11T18:00:51.895Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake Codex v1.4.md [CX-573E] FORBIDDEN_PATTERN_AUDIT (HARD) (enforced via `just product-scan`)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packets: NONE (this is the initial v1 packet for BASE_WP_ID=WP-1-Runtime-Governance-NoExpect).
- Preserved vs changed: N/A (no prior versions).

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - .GOV/refinements/WP-1-Runtime-Governance-NoExpect-v1.md
  - .GOV/task_packets/WP-1-Runtime-Governance-NoExpect-v1.md
  - .GOV/roles_shared/SIGNATURE_AUDIT.md
  - .GOV/roles/orchestrator/ORCHESTRATOR_GATES.json
  - src/backend/handshake_core/src/runtime_governance.rs
- SEARCH_TERMS:
  - ".expect("
  - ".unwrap("
  - "RuntimeGovernancePaths::from_workspace_root"
  - "tempdir("
- RUN_COMMANDS:
  ```bash
  rg -n "\\.expect\\(" src/backend/handshake_core/src/runtime_governance.rs
  rg -n "\\.unwrap\\(" src/backend/handshake_core/src/runtime_governance.rs
  just product-scan
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just post-work WP-1-Runtime-Governance-NoExpect-v1 --range 84aee8219bc5ae38115af33f914d4639dbad9688..HEAD
  ```
- RISK_MAP:
  - "Test behavior drift" -> "runtime_governance tests become too permissive; ensure failures still surface via Result"
  - "Missed forbidden pattern" -> "product-scan still fails; re-grep for expect/unwrap and fix remaining call sites"

## SKELETON
- Proposed interfaces/types/contracts:
- N/A (hygiene-only refactor in tests; no new interfaces)
- Open questions: NONE
- Notes: Prefer `fn test_name() -> std::io::Result<()> { ...; Ok(()) }` and `?` over `expect`/`unwrap`.

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
- **Target File**: `src/backend/handshake_core/src/runtime_governance.rs`
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
- Current WP_STATUS: Ready for Dev
- What changed in this update: Task packet created/filled; refinement approved/signed; prepare recorded; ready for coder implementation.
- Next step / handoff hint: Update `src/backend/handshake_core/src/runtime_governance.rs` tests to remove `expect`/`unwrap` by returning `Result` and using `?`; then run TEST_PLAN.

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
  - LOG_PATH: `.handshake/logs/WP-1-Runtime-Governance-NoExpect-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

