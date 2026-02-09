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
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** In Progress
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
  - .GOV/roles_shared/OSS_REGISTER.md
  - .GOV/roles_shared/TASK_BOARD.md
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - docs/OSS_REGISTER.md
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
- Current WP_STATUS: In Progress (started 2026-02-09)
- What changed in this update: Bootstrapped coder claim fields/status.
- Next step / handoff hint: Run `just pre-work WP-1-Supply-Chain-Cargo-Deny-Clean-v1`, then update `sqlx`/`time` in `src/backend/handshake_core` and re-run `cargo deny`.

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

### Validator evidence (2026-02-09T02:20:42Z)

- NOTE: Validation executed against committed `HEAD=ff9d303443bcb8d704b35692a8ee29212d84ddb0` (coder-provided HEAD SHA was not provided). At validation start, the worktree contained uncommitted changes (`src/backend/handshake_core/Cargo.lock`). Range-based gates validate committed blobs only.

- COMMAND: `pwd; git rev-parse --show-toplevel; git rev-parse --abbrev-ref HEAD; git status -sb; git worktree list`
  - EXIT_CODE: 0
  - WORKTREE_DIR: `D:/Projects/LLM projects/wt-WP-1-Supply-Chain-Cargo-Deny-Clean-v1`
  - BRANCH: `feat/WP-1-Supply-Chain-Cargo-Deny-Clean-v1`
  - GIT_SHA_BEFORE: `ff9d303443bcb8d704b35692a8ee29212d84ddb0`
  - GIT_SHA_AFTER: `ff9d303443bcb8d704b35692a8ee29212d84ddb0`
  - LOG_PATH: `.handshake/logs/WP-1-Supply-Chain-Cargo-Deny-Clean-v1/validator-cx-wt-001-20260209T022042Z.log`
  - OUTPUT_SHA256: `efcfd2340a9e82fcb1963b1f7ebf99d53cc1c3f070e084418730627ca43dc425`
  - PROOF_LINES:
    - `## feat/WP-1-Supply-Chain-Cargo-Deny-Clean-v1`
    - ` M src/backend/handshake_core/Cargo.lock`

- COMMAND: `cd src/backend/handshake_core; cargo deny check advisories licenses bans sources`
  - EXIT_CODE: 1
  - WORKTREE_DIR: `D:/Projects/LLM projects/wt-WP-1-Supply-Chain-Cargo-Deny-Clean-v1`
  - BRANCH: `feat/WP-1-Supply-Chain-Cargo-Deny-Clean-v1`
  - GIT_SHA_BEFORE: `ff9d303443bcb8d704b35692a8ee29212d84ddb0`
  - GIT_SHA_AFTER: `ff9d303443bcb8d704b35692a8ee29212d84ddb0`
  - LOG_PATH: `.handshake/logs/WP-1-Supply-Chain-Cargo-Deny-Clean-v1/validator-cargo-deny-check.log`
  - OUTPUT_SHA256: `d1b9f7888023b7374828914accdd0878fbffae6a8f7e98622c329cec82d37815`
  - PROOF_LINES:
    - `advisories FAILED, bans ok, licenses ok, sources ok`
    - `    ├ ID: RUSTSEC-2024-0363`
    -     ├ Solution: Upgrade to >=0.8.1 (try `cargo update -p sqlx`)
    - `    ├ ID: RUSTSEC-2026-0009`
    -     ├ Solution: Upgrade to >=0.3.47 (try `cargo update -p time`)

- COMMAND: `just deny`
  - EXIT_CODE: 1
  - WORKTREE_DIR: `D:/Projects/LLM projects/wt-WP-1-Supply-Chain-Cargo-Deny-Clean-v1`
  - BRANCH: `feat/WP-1-Supply-Chain-Cargo-Deny-Clean-v1`
  - GIT_SHA_BEFORE: `ff9d303443bcb8d704b35692a8ee29212d84ddb0`
  - GIT_SHA_AFTER: `ff9d303443bcb8d704b35692a8ee29212d84ddb0`
  - LOG_PATH: `.handshake/logs/WP-1-Supply-Chain-Cargo-Deny-Clean-v1/validator-just-deny.log`
  - OUTPUT_SHA256: `9d4c2c4d3c20bd26fb596ce5ff9de4e42c40fbd47e5ac85c3395a83d172b74bd`
  - PROOF_LINES:
    - `advisories FAILED, bans ok, licenses ok, sources ok`
    - error: Recipe `deny` failed on line 96 with exit code 1

- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - EXIT_CODE: 0
  - WORKTREE_DIR: `D:/Projects/LLM projects/wt-WP-1-Supply-Chain-Cargo-Deny-Clean-v1`
  - BRANCH: `feat/WP-1-Supply-Chain-Cargo-Deny-Clean-v1`
  - GIT_SHA_BEFORE: `ff9d303443bcb8d704b35692a8ee29212d84ddb0`
  - GIT_SHA_AFTER: `ff9d303443bcb8d704b35692a8ee29212d84ddb0`
  - LOG_PATH: `.handshake/logs/WP-1-Supply-Chain-Cargo-Deny-Clean-v1/validator-cargo-test-handshake-core.log`
  - OUTPUT_SHA256: `3e2f028d0af8fc7beedd958718fcc4c4433d83b6a60526c630a312800f54bfa6`
  - PROOF_LINES:
    - `running 157 tests`
    - `test result: ok. 157 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`

- COMMAND: `just post-work WP-1-Supply-Chain-Cargo-Deny-Clean-v1 --range ff9d3034..HEAD`
  - EXIT_CODE: 1
  - WORKTREE_DIR: `D:/Projects/LLM projects/wt-WP-1-Supply-Chain-Cargo-Deny-Clean-v1`
  - BRANCH: `feat/WP-1-Supply-Chain-Cargo-Deny-Clean-v1`
  - GIT_SHA_BEFORE: `ff9d303443bcb8d704b35692a8ee29212d84ddb0`
  - GIT_SHA_AFTER: `ff9d303443bcb8d704b35692a8ee29212d84ddb0`
  - LOG_PATH: `.handshake/logs/WP-1-Supply-Chain-Cargo-Deny-Clean-v1/validator-just-post-work.log`
  - OUTPUT_SHA256: `09ff6a6d6f9ff674e5baa625f32fb67ddb2d817825275761ce03158419da81a8`
  - PROOF_LINES:
    - `Post-work validation FAILED (deterministic manifest gate; not tests)`
    - `EVIDENCE_MAPPING has no file:line evidence`
    - `EVIDENCE must include at least one COMMAND + EXIT_CODE entry for modern packets`
    - `No files changed in range ff9d3034..HEAD`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### VALIDATION REPORT - WP-1-Supply-Chain-Cargo-Deny-Clean-v1

Verdict: FAIL

Validation Claims (do not collapse into a single PASS):
- GATES_PASS (deterministic manifest gate: `just post-work WP-1-Supply-Chain-Cargo-Deny-Clean-v1`; not tests): FAIL
- TEST_PLAN_PASS (packet TEST_PLAN commands, verbatim): FAIL
- SPEC_CONFORMANCE_CONFIRMED (DONE_MEANS + SPEC_ANCHOR -> evidence mapping): NO

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Supply-Chain-Cargo-Deny-Clean-v1.md` (**Status:** In Progress)
- Refinement: `.GOV/refinements/WP-1-Supply-Chain-Cargo-Deny-Clean-v1.md` (USER_REVIEW_STATUS: APPROVED)
- Spec target: `.GOV/roles_shared/SPEC_CURRENT.md` -> `Handshake_Master_Spec_v02.125.md`
- Spec anchors validated (presence confirmed in `Handshake_Master_Spec_v02.125.md`):
  - [CX-631] HYGIENE_COMMANDS includes `cargo deny check advisories licenses bans sources`
  - Phase Closure Gate: "Supply chain audit clean (zero violations)"
  - `deny.toml` template index (supply-chain policy shape)

Validation Target:
- Coder-provided HEAD_SHA: NOT PROVIDED
- Committed `HEAD` at validation time: `ff9d303443bcb8d704b35692a8ee29212d84ddb0`
- Post-work range used: `ff9d3034..HEAD` (committed diff was empty; uncommitted changes existed in worktree at start)

Files Checked:
- `AGENTS.md`
- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- `.GOV/roles/validator/agentic/AGENTIC_PROTOCOL.md`
- `.GOV/roles_shared/EVIDENCE_LEDGER.md`
- `.GOV/roles_shared/BOUNDARY_RULES.md`
- `.GOV/roles_shared/SPEC_CURRENT.md`
- `.GOV/task_packets/WP-1-Supply-Chain-Cargo-Deny-Clean-v1.md`
- `.GOV/refinements/WP-1-Supply-Chain-Cargo-Deny-Clean-v1.md`
- `Handshake Codex v1.4.md`
- `Handshake_Master_Spec_v02.125.md` (anchor windows)
- `justfile` (deny recipe)
- `.github/workflows/ci.yml` (cargo-deny step)
- `src/backend/handshake_core/Cargo.toml` (sqlx dependency declaration)
- `src/backend/handshake_core/Cargo.lock` (time/sqlx locked versions)

Findings:
- Supply-chain advisories: FAIL.
  - Evidence: `cd src/backend/handshake_core; cargo deny check advisories licenses bans sources` exit 1 (OUTPUT_SHA256: `d1b9f7888023b7374828914accdd0878fbffae6a8f7e98622c329cec82d37815`).
  - Violations observed:
    - RUSTSEC-2024-0363: `sqlx 0.8.0` (requires upgrade to `>=0.8.1`).
    - RUSTSEC-2026-0009: `time 0.3.44` (requires upgrade to `>=0.3.47`).
- Repo command surface: PARTIAL.
  - `just deny` correctly targets backend manifest (`--manifest-path src/backend/handshake_core/Cargo.toml`) but fails due to advisories (exit 1; OUTPUT_SHA256: `9d4c2c4d3c20bd26fb596ce5ff9de4e42c40fbd47e5ac85c3395a83d172b74bd`).
- Deterministic manifest gate: FAIL.
  - Evidence: `just post-work ... --range ff9d3034..HEAD` exit 1 (OUTPUT_SHA256: `09ff6a6d6f9ff674e5baa625f32fb67ddb2d817825275761ce03158419da81a8`).
  - Gate failures include missing `EVIDENCE_MAPPING`, placeholder/invalid manifest fields under `## VALIDATION`, and empty committed diff range.

Tests (packet TEST_PLAN):
- `cd src/backend/handshake_core; cargo deny check advisories licenses bans sources`: FAIL (exit 1)
- `just deny`: FAIL (exit 1)
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`: PASS (exit 0; OUTPUT_SHA256: `3e2f028d0af8fc7beedd958718fcc4c4433d83b6a60526c630a312800f54bfa6`)

REASON FOR FAIL:
- Supply-chain gate not satisfied: `cargo deny` advisories still report active vulnerabilities (sqlx/time), violating DONE_MEANS and Phase Closure Gate "zero violations".
- Deterministic validation gate not satisfied: `just post-work` fails due to incomplete packet manifest/evidence mapping and no committed changes in the provided range.
- Validation target ambiguity: coder did not provide a committed HEAD SHA to validate; branch HEAD remained at docs-only commit `ff9d3034` during this run.

Actionable Next Steps (no waivers assumed):
- Coder:
  - Upgrade `sqlx` to `>=0.8.1` and ensure `Cargo.lock` resolves to a patched version (then re-run `cargo deny`).
  - Upgrade `time` to `>=0.3.47` (transitive; ensure `Cargo.lock` resolves to `time >=0.3.47`).
  - Commit the dependency remediation changes and provide the resulting `HEAD_SHA` for validation.
  - Fill `## EVIDENCE_MAPPING` with DONE_MEANS/SPEC_ANCHOR -> `path:line` mappings (Spec also mandates evidence mapping for MUSTs).
  - Replace placeholder fields under `## VALIDATION` with real manifest entries (file windows + pre/post SHA1s) so `just post-work` can pass deterministically.
- Validator (after coder commits):
  - Re-run TEST_PLAN commands and `just post-work WP-1-Supply-Chain-Cargo-Deny-Clean-v1 --range ff9d3034..HEAD_SHA` against the committed target, then append a new Validation Report block with PASS/FAIL.
