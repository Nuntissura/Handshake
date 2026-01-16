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
- **Status:** Ready for Dev
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
  - scripts/validation/governance-reference.mjs (new; optional SSoT helper)
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
- Open questions:
- Notes:

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
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
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
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
