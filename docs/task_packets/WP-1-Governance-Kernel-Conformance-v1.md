# Task Packet: WP-1-Governance-Kernel-Conformance-v1

## METADATA
- TASK_ID: WP-1-Governance-Kernel-Conformance-v1
- WP_ID: WP-1-Governance-Kernel-Conformance-v1
- BASE_WP_ID: WP-1-Governance-Kernel-Conformance (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-16T21:18:11.041Z
- REQUESTOR: Operator (ilja)
- AGENT_ID: CodexCLI-GPT-5.2 (Orchestrator)
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** Done
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja160120262149

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Governance-Kernel-Conformance-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Remediate Governance Kernel drift in CI + local hooks + docs by aligning all governance reference checks and messaging with the current Governance Reference (`Handshake Codex v1.4.md`) and by eliminating hard-coded references to non-existent legacy files (notably `Handshake Codex v0.8.md`).
- Why: The Master Spec requires CI/hook parity and treats governance drift as a first-class failure mode. The current repo fails CI traceability due to a hard-coded legacy codex filename and provides misleading guidance ("Codex v0.8"), encouraging bypasses and policy drift.
- IN_SCOPE_PATHS:
  - .github/workflows/ci.yml
  - scripts/hooks/pre-commit
  - scripts/validation/ci-traceability-check.mjs
  - scripts/validation/governance-reference.mjs
  - docs/task_packets/README.md
  - docs/ORCHESTRATOR_PROTOCOL.md
  - justfile
  - Justfile
- OUT_OF_SCOPE:
  - src/** (explicitly includes src/backend/handshake_core/**)
  - app/**
  - tests/**
  - docs/SPEC_CURRENT.md and any `Handshake_Master_Spec_v*.md` (ENRICHMENT_NEEDED=NO; no spec edits)
  - docs/TASK_BOARD.md and docs/WP_TRACEABILITY_REGISTRY.md (Orchestrator-only coordination to avoid parallel edit collisions)
  - Any edits to historical task packets in docs/task_packets/*.md (locked history; do not "catch up" old packets)
  - Handshake_logger_*.md (historical log; do not rewrite)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Governance-Kernel-Conformance-v1

# Smoke checks (expected PASS after implementation):
node scripts/validation/ci-traceability-check.mjs
just codex-check

# Codex drift checks (expected NO MATCHES in the in-scope surfaces):
rg -n "Handshake Codex v0\\.8\\.md|Codex v0\\.8" .github/workflows/ci.yml scripts/hooks/pre-commit scripts/validation/ci-traceability-check.mjs docs/task_packets/README.md docs/ORCHESTRATOR_PROTOCOL.md justfile Justfile && exit 1 || exit 0

# Note: `scripts/hooks/pre-commit` runs in bash and expects staged files; do not treat it as a standalone test unless you have a safe staged set.

just cargo-clean
just post-work WP-1-Governance-Kernel-Conformance-v1
```

### DONE_MEANS
- CI traceability no longer fails on a missing legacy codex file: `node scripts/validation/ci-traceability-check.mjs` validates the current Governance Reference from `docs/SPEC_CURRENT.md` (currently `Handshake Codex v1.4.md`) instead of `Handshake Codex v0.8.md`.
- `scripts/hooks/pre-commit` references the current Governance Reference (file name + version string) and does not instruct users to consult `Handshake Codex v0.8.md`.
- `.github/workflows/ci.yml` no longer prints stale "Codex v0.8" messaging and its doc drift checks align with the current Governance Reference without breaking on locked historical task packets.
- `docs/task_packets/README.md` links to `Handshake Codex v1.4.md` (not `v0.8`).
- `docs/ORCHESTRATOR_PROTOCOL.md` examples reference `Handshake Codex v1.4.md` (not `v0.8`).
- The TEST_PLAN smoke checks pass and `just post-work WP-1-Governance-Kernel-Conformance-v1` passes.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.112.md (recorded_at: 2026-01-16T21:18:11.041Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.112.md 7.5.4.7 + 7.5.4.9.2
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.112.md (7.5.4.7, 7.5.4.9.2)
  - docs/ORCHESTRATOR_PROTOCOL.md
  - docs/GOV_KERNEL/90_REFERENCE_IMPLEMENTATION_HANDSHAKE.md
  - .github/workflows/ci.yml
  - scripts/validation/ci-traceability-check.mjs
  - scripts/hooks/pre-commit
  - docs/task_packets/README.md
- SEARCH_TERMS:
  - "Codex v0.8"
  - "Handshake Codex v0.8.md"
  - "Handshake Codex v1.4.md"
  - "CI Traceability Check"
  - "Pre-commit validation"
  - "Governance Reference"
- RUN_COMMANDS:
  ```bash
  rg -n "Codex v0\\.8|Handshake Codex v0\\.8\\.md" .github scripts/hooks scripts/validation docs/task_packets/README.md docs/ORCHESTRATOR_PROTOCOL.md justfile Justfile
  node scripts/validation/ci-traceability-check.mjs
  ```
- RISK_MAP:
  - "CI/hook still points to legacy codex" -> "CI hard-fail + governance bypass incentive"
  - "Doc drift guard scans locked history" -> "CI false positives; forces rewriting immutable packets"
  - "Multiple sources of truth for codex filename" -> "recurring drift across scripts/workflows"
  - "Changing CI/hook behavior breaks developer loop" -> "blocked commits and reduced throughput"

## SKELETON
- Proposed interfaces/types/contracts:
  - scripts/validation/governance-reference.mjs
    - Export: resolveGovernanceReference(): { codexFilename, codexPathAbs, specCurrentPathAbs }
    - CLI: --print-file | --print-path | --json
- Open questions:
  - None.
- Notes:
  - Single source of truth is docs/SPEC_CURRENT.md ("Governance Reference" value).
  - scripts/validation/ci-traceability-check.mjs must fail if resolved codex file is missing (no v0.8 hardcode).
  - .github/workflows/ci.yml drift scan must exclude docs/task_packets/** and docs/refinements/**, but still scan docs/task_packets/README.md.
  - scripts/hooks/pre-commit messaging must use the resolver when available; on resolver failure, fall back to pointing at docs/SPEC_CURRENT.md.

SKELETON APPROVED

## IMPLEMENTATION
- Added a Governance Reference resolver sourced from docs/SPEC_CURRENT.md (scripts/validation/governance-reference.mjs).
- Updated scripts/validation/ci-traceability-check.mjs to derive the required codex filename from docs/SPEC_CURRENT.md and require that resolved file exists.
- Updated scripts/hooks/pre-commit messaging to reference the resolved Governance Reference (fallback: docs/SPEC_CURRENT.md if resolver fails).
- Updated .github/workflows/ci.yml doc drift scan to exclude locked history (docs/task_packets/**, docs/refinements/**) while still scanning docs/task_packets/README.md.
- Removed stale "Codex v0.8" references from in-scope docs and justfile comments.

## HYGIENE
- Commands executed (see TEST_PLAN; recorded as exit codes only):
  - just pre-work WP-1-Governance-Kernel-Conformance-v1 -> exit code 0
  - node scripts/validation/ci-traceability-check.mjs -> exit code 0
  - just codex-check -> exit code 0
  - rg drift scan (Handshake Codex v0.8.md / Codex v0.8) across in-scope surfaces -> exit code 1 (no matches)
  - just cargo-clean -> exit code 0
  - just post-work WP-1-Governance-Kernel-Conformance-v1 -> exit code 0 (warning: new file not present in HEAD)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.

### Manifest Entry 1: .github/workflows/ci.yml
- **Target File**: `.github/workflows/ci.yml`
- **Start**: 1
- **End**: 99999
- **Line Delta**: 1
- **Pre-SHA1**: `172de3c127b08269e3bfb921165f141d84daf1fa`
- **Post-SHA1**: `705864ed9f3ea9e7b0b284b2c8c607fa6057631a`
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
- **Lint Results**: N/A (YAML workflow change)
- **Artifacts**: None
- **Timestamp**: 2026-01-16
- **Operator**: Coder-2 (GPT-5.2)
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.112.md
- **Notes**: Doc drift scan excludes locked history (docs/task_packets/**, docs/refinements/**) while still scanning docs/task_packets/README.md.

### Manifest Entry 2: scripts/hooks/pre-commit
- **Target File**: `scripts/hooks/pre-commit`
- **Start**: 1
- **End**: 99999
- **Line Delta**: 12
- **Pre-SHA1**: `049236cb75d5462a8e2b844a12fb6a1a0059cd7a`
- **Post-SHA1**: `91eaca800ed76205cb2c10c5667e380909a8bf9b`
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
- **Lint Results**: N/A (bash hook messaging change)
- **Artifacts**: None
- **Timestamp**: 2026-01-16
- **Operator**: Coder-2 (GPT-5.2)
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.112.md
- **Notes**: Hook messaging references resolved Governance Reference (fallback: docs/SPEC_CURRENT.md if resolver fails).

### Manifest Entry 3: scripts/validation/ci-traceability-check.mjs
- **Target File**: `scripts/validation/ci-traceability-check.mjs`
- **Start**: 1
- **End**: 99999
- **Line Delta**: 21
- **Pre-SHA1**: `870b83e5ec54cf46b85fdb81e914eb0c318251be`
- **Post-SHA1**: `f79b0c4d32304fbe4bb000ffeca7bd6bb0bd0346`
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
- **Lint Results**: N/A (node script change)
- **Artifacts**: None
- **Timestamp**: 2026-01-16
- **Operator**: Coder-2 (GPT-5.2)
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.112.md
- **Notes**: Derives Governance Reference from docs/SPEC_CURRENT.md and fails if resolved codex file is missing.

### Manifest Entry 4: scripts/validation/governance-reference.mjs
- **Target File**: `scripts/validation/governance-reference.mjs`
- **Start**: 1
- **End**: 99999
- **Line Delta**: 97
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `605636be40fd911c589359a94b1207609f454b23`
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
- **Lint Results**: N/A (new node script)
- **Artifacts**: None
- **Timestamp**: 2026-01-16
- **Operator**: Coder-2 (GPT-5.2)
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.112.md
- **Notes**: SSoT resolver for Governance Reference (docs/SPEC_CURRENT.md).

### Manifest Entry 5: justfile
- **Target File**: `justfile`
- **Start**: 1
- **End**: 99999
- **Line Delta**: 0
- **Pre-SHA1**: `ca54c9591c1aaabcec31acfae49c80d5823e6b13`
- **Post-SHA1**: `1b8f94e741fd193f2b9921f062361701e6a371a6`
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
- **Lint Results**: N/A (comment change only)
- **Artifacts**: None
- **Timestamp**: 2026-01-16
- **Operator**: Coder-2 (GPT-5.2)
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.112.md
- **Notes**: Remove stale "Codex v0.8" label from governance command surface comments.

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: Implementation complete; ready for Validator review.
- What changed in this update:
  - Removed/neutralized stale Codex v0.8 references in CI/hook/docs surfaces in scope.
  - Updated CI traceability to resolve Governance Reference via docs/SPEC_CURRENT.md (no legacy codex hardcode).
  - Normalized IN_SCOPE_PATHS entry for scripts/validation/governance-reference.mjs (removed trailing commentary so post-work scope gate matches exact path).
- Next step / handoff hint: Validator review against DONE_MEANS + Master Spec anchors; merge when ready.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Evidence summary (minimal; ASCII-only):
  - CI traceability check banner now prints resolved Governance Reference filename from docs/SPEC_CURRENT.md.
  - Pre-commit hook banner now prints resolved Governance Reference filename when resolver is available.
  - In-scope drift scan (Codex v0.8) returned no matches (see HYGIENE entry for exit code).

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### VALIDATION REPORT — WP-1-Governance-Kernel-Conformance-v1 (2026-01-16)
Verdict: PASS

Scope Inputs:
- Task Packet: `docs/task_packets/WP-1-Governance-Kernel-Conformance-v1.md` (WP_STATUS at `docs/task_packets/WP-1-Governance-Kernel-Conformance-v1.md:282`)
- Spec Target: `docs/SPEC_CURRENT.md` -> `Handshake_Master_Spec_v02.112.md` (`docs/SPEC_CURRENT.md:5`)
- Governance Reference: `Handshake Codex v1.4.md` (`docs/SPEC_CURRENT.md:13`)
- Worktree/Branch: `D:\Projects\LLM projects\wt-WP-1-Governance-Kernel-Conformance-v1` / `feat/WP-1-Governance-Kernel-Conformance-v1`
- Implementation commits reviewed: `400a48ab` (implementation) + `cebe5255` (traceability sync)

Files Checked:
- `docs/SPEC_CURRENT.md`
- `docs/WP_TRACEABILITY_REGISTRY.md`
- `docs/TASK_BOARD.md`
- `docs/task_packets/WP-1-Governance-Kernel-Conformance-v1.md`
- `docs/task_packets/README.md`
- `docs/ORCHESTRATOR_PROTOCOL.md`
- `.github/workflows/ci.yml`
- `scripts/validation/governance-reference.mjs`
- `scripts/validation/ci-traceability-check.mjs`
- `scripts/hooks/pre-commit`
- `justfile`

Findings (evidence):
- SSoT confirmed: Governance Reference is sourced from `docs/SPEC_CURRENT.md` (`docs/SPEC_CURRENT.md:11-13`).
- Resolver parses the Governance Reference filename from `docs/SPEC_CURRENT.md` and returns filename + abs path (`scripts/validation/governance-reference.mjs:47-60`).
- CI traceability check imports resolver and requires the resolved codex file exists (`scripts/validation/ci-traceability-check.mjs:11-24`, `scripts/validation/ci-traceability-check.mjs:119-130`).
- Pre-commit hook prints the resolved Governance Reference when Node is available, else points developers to `docs/SPEC_CURRENT.md` (`scripts/hooks/pre-commit:8-20`, `scripts/hooks/pre-commit:58-59`).
- CI doc drift scan excludes locked history (`docs/task_packets/**`, `docs/refinements/**`) while still scanning `docs/task_packets/README.md` (`.github/workflows/ci.yml:48-50`).
- In-scope legacy references removed: validation grep found no `Handshake Codex v0.8.md` / `Codex v0.8` matches across the declared surfaces (coder hygiene step, corroborated by validator spot-check).
- Task Board and traceability registry are consistent for this WP (`docs/WP_TRACEABILITY_REGISTRY.md:81`, `docs/TASK_BOARD.md:97`).
- Orchestrator protocol authority snippet references the correct Codex (`docs/ORCHESTRATOR_PROTOCOL.md:1272-1276`).
- Task packet README links to the correct Codex (`docs/task_packets/README.md:63`).
- `justfile` governance command header is non-stale (`justfile:82`).

Tests:
- `node scripts/validation/ci-traceability-check.mjs` -> PASS (exit 0)
- `just codex-check` -> PASS (exit 0)

REASON FOR PASS:
- Removes the hard-coded legacy governance reference dependency by deriving the required Codex filename from `docs/SPEC_CURRENT.md` and enforcing file existence in CI/hook tooling, preventing recurring drift/CI failures while keeping historical task packet/refinement history immutable.

Risks & Suggested Actions:
- If `docs/SPEC_CURRENT.md` governance-reference formatting changes (marker or bold filename), the resolver will hard-fail; this is acceptable for governance enforcement but may warrant a future micro-test asserting the parser contract.
- Closure work (validator-owned): after merge to `main`, move `WP-1-Governance-Kernel-Conformance-v1` on `docs/TASK_BOARD.md` from `## In Progress` to `## Done` with `[VALIDATED]` per protocol.
