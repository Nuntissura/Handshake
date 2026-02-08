# Task Packet: WP-1-Supply-Chain-Cargo-Deny-Clean-v1

## METADATA
- TASK_ID: WP-1-Supply-Chain-Cargo-Deny-Clean-v1
- WP_ID: WP-1-Supply-Chain-Cargo-Deny-Clean-v1
- BASE_WP_ID: WP-1-Supply-Chain-Cargo-Deny-Clean (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-08T21:26:20.542Z
- MERGE_BASE_SHA: 0092ad1dcfec98e064f9eb97185ac493dedb7b42
- REQUESTOR: ilja (Operator) / Phase 1 closure gate (cargo deny clean)
- AGENT_ID: CodexCLI-GPT-5.2
- ROLE: Orchestrator
- AGENTIC_MODE: YES
- ORCHESTRATOR_MODEL: GPT-5.2 (Codex CLI)
- ORCHESTRATION_STARTED_AT_UTC: 2026-02-08T21:26:20.542Z
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja080220262221
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Supply-Chain-Cargo-Deny-Clean-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Remediate supply-chain hygiene so `cargo deny check advisories licenses bans sources` passes with zero violations for Handshake backend, and ensure repo hygiene commands (`just deny`, `just validate`) invoke cargo-deny in the correct manifest context (no root Cargo.toml failure).
- Why: Master Spec Phase Closure Gate requires supply chain clean (cargo deny + npm audit). Currently, cargo-deny fails due to active advisories (sqlx/time) and license classification (ring), and `just deny` fails from repo root because there is no root Cargo.toml.
- IN_SCOPE_PATHS:
  - .GOV/refinements/WP-1-Supply-Chain-Cargo-Deny-Clean-v1.md
  - .GOV/task_packets/WP-1-Supply-Chain-Cargo-Deny-Clean-v1.md
  - .GOV/roles_shared/TASK_BOARD.md
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - justfile
  - deny.toml
  - src/backend/handshake_core/Cargo.toml
  - src/backend/handshake_core/Cargo.lock
  - .github/workflows/ci.yml (only if needed for CI parity)
- OUT_OF_SCOPE:
  - Unrelated dependency upgrades not required to satisfy cargo-deny
  - Any feature work outside supply-chain hygiene and CI parity

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Supply-Chain-Cargo-Deny-Clean-v1

# Cargo deny must pass (0 violations):
cd src/backend/handshake_core; cargo deny check advisories licenses bans sources

# Repo command surface must work from root:
just deny

# Regression safety:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Hygiene (external target dir):
just cargo-clean

# (Optional but recommended for parity):
just validate

just post-work WP-1-Supply-Chain-Cargo-Deny-Clean-v1 --range 0092ad1dcfec98e064f9eb97185ac493dedb7b42..HEAD
```

### DONE_MEANS
- `cd src/backend/handshake_core; cargo deny check advisories licenses bans sources` exits 0 (no advisories, no license errors).
- Advisory remediation is implemented (preferred): sqlx upgraded to >= 0.8.1 and time upgraded to >= 0.3.47 (or an explicit, narrowly scoped ignore is recorded with justification in the packet if unavoidable).
- License classification failure for `ring` is resolved via policy clarification in `deny.toml` (or other deterministic remediation) so cargo-deny no longer reports `ring` as unlicensed.
- `just deny` succeeds from repo root (no "missing Cargo.toml" failure).
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` continues to pass.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.125.md (recorded_at: 2026-02-08T21:26:20.542Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.125.md [CX-631] (cargo deny in hygiene commands) + [CX-609B] (Phase Closure Gate: supply chain clean) + 7.5.4.9.2 (deny.toml template index)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- BASE_WP_ID: WP-1-Supply-Chain-Cargo-Deny-Clean
- Prior packets: NONE (v1 is the first revision for this base WP).
- v1 (THIS PACKET):
  - Establishes an executable remediation packet to make cargo-deny clean (0 violations) and align `just`/CI command surfaces with the spec's closure gate.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.125.md (anchors: [CX-631], [CX-609B], deny.toml template index)
  - deny.toml
  - justfile
  - src/backend/handshake_core/Cargo.toml
  - src/backend/handshake_core/Cargo.lock
  - .github/workflows/ci.yml
- SEARCH_TERMS:
  - "RUSTSEC-2024-0363"
  - "RUSTSEC-2026-0009"
  - "ring"
  - "sqlx 0.8.0"
  - "time 0.3.44"
  - "cargo deny check"
- RUN_COMMANDS:
  ```bash
  cd src/backend/handshake_core; cargo deny check advisories licenses bans sources
  rg -n "cargo deny" justfile .github/workflows/ci.yml -S
  ```
- RISK_MAP:
  - "overbroad ignores" -> "paper over real vulnerabilities/licenses; violates closure gate intent"
  - "dependency upgrade breakage" -> "backend compile/test failures; requires targeted update + cargo test"
  - "CI parity drift" -> "local validate passes but CI does not (or vice versa); add/align workflow step if needed"

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
  - LOG_PATH: `.handshake/logs/WP-1-Supply-Chain-Cargo-Deny-Clean-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

