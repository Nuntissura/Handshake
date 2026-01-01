# Task Packet: WP-1-OSS-Register-Enforcement-v1

## METADATA
- TASK_ID: WP-1-OSS-Register-Enforcement-v1
- WP_ID: WP-1-OSS-Register-Enforcement-v1
- DATE: 2026-01-01T14:29:22.074Z
- REQUESTOR: ilja
- AGENT_ID: Codex CLI (Orchestrator)
- ROLE: Orchestrator
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- RISK_TIER_JUSTIFICATION: This gate prevents OSS register drift and copyleft (GPL/AGPL) in-process contamination, which is a Phase 1 closure and distribution-risk blocker.
- USER_SIGNATURE: ilja010120261528

## USER_CONTEXT (Non-Technical Explainer)
The OSS Register is the project's "bill of materials" for open-source components. This packet adds an automated test so that if someone adds a dependency without registering it (or adds GPL/AGPL code in-process), the build fails immediately instead of discovering the issue later.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-OSS-Register-Enforcement-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Add OSS register enforcement per SPEC_CURRENT v02.100 by implementing a backend unit test that:
  - checks that dependencies from `src/backend/handshake_core/Cargo.lock` and `app/package.json` exist in `docs/OSS_REGISTER.md`, and
  - enforces the copyleft isolation rule: GPL/AGPL components MUST NOT be `embedded_lib` (must be `external_process` or `external_service`).
- Why: SPEC_CURRENT explicitly requires this enforcement; without it, OSS provenance and license posture can drift silently.
- IN_SCOPE_PATHS:
  - docs/OSS_REGISTER.md
  - src/backend/handshake_core/tests/oss_register_enforcement_tests.rs
- OUT_OF_SCOPE:
  - Any changes to `src/backend/handshake_core/src/**` (this WP is test + register only).
  - Any changes to `Cargo.toml` / `Cargo.lock` / `app/package.json` (no dependency changes; enforcement only).
  - Any UI changes in `app/` or `src/frontend/`.
  - Any changes to files listed in `docs/task_packets/WP-1-Flight-Recorder-v3.md` IN_SCOPE_PATHS (parallel coder safety).

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-OSS-Register-Enforcement-v1

# Spec integrity:
just validator-spec-regression

# Targeted test (new):
cargo test --manifest-path src/backend/handshake_core/Cargo.toml oss_register_enforcement

# Full backend tests:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Hygiene:
just validator-scan
just cargo-clean
just post-work WP-1-OSS-Register-Enforcement-v1
```

### DONE_MEANS
- `docs/OSS_REGISTER.md` includes an explicit `IntegrationMode` field for every entry as one of: `embedded_lib`, `external_process`, `external_service`.
- `src/backend/handshake_core/tests/oss_register_enforcement_tests.rs` exists and deterministically fails if:
  - any crate in `src/backend/handshake_core/Cargo.lock` is missing from `docs/OSS_REGISTER.md`, or
  - any npm dependency in `app/package.json` (dependencies + devDependencies) is missing from `docs/OSS_REGISTER.md`, or
  - any register entry with license `GPL-*` or `AGPL-*` is marked as `embedded_lib`.
- All commands in TEST_PLAN pass and `just post-work WP-1-OSS-Register-Enforcement-v1` returns PASS (COR-701 manifest filled; ASCII-only packet).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.100.md (recorded_at: 2026-01-01T14:29:22.074Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.10.4.2 (OSS Register Enforcement) and 11.7.4.3-11.7.4.4 (Copyleft isolation + OSS Register requirement)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.100.md
  - docs/OSS_REGISTER.md
  - src/backend/handshake_core/Cargo.lock
  - src/backend/handshake_core/Cargo.toml
  - app/package.json
  - scripts/validation/validator-spec-regression.mjs
  - docs/task_packets/WP-1-Flight-Recorder-v3.md
- SEARCH_TERMS:
  - "OSS Register Enforcement"
  - "Copyleft isolation rule"
  - "external_process"
  - "external_service"
  - "embedded_lib"
  - "Cargo.lock"
  - "package.json"
  - "OSS_REGISTER"
  - "AGPL"
  - "GPL"
- RUN_COMMANDS:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml oss_register_enforcement
  ```
- RISK_MAP:
  - "False positives block work" -> "throughput hit; mitigate via clear error output listing missing items"
  - "False negatives miss deps" -> "license posture drift; ensure both Cargo.lock and app/package.json are checked"
  - "Register format drift" -> "parser breaks; document a strict minimal format in docs/OSS_REGISTER.md"
  - "Copyleft isolation not enforced" -> "legal/compliance risk"

## SKELETON
- Proposed interfaces/types/contracts:
- Minimal deterministic format requirement for `docs/OSS_REGISTER.md`:
  - Each row MUST include: `Component`, `License`, `IntegrationMode`.
  - IntegrationMode MUST be one of: `embedded_lib`, `external_process`, `external_service`.
- Unit test contract:
  - Enumerate crate package names from `src/backend/handshake_core/Cargo.lock`.
  - Enumerate npm package names from `app/package.json` dependencies + devDependencies.
  - Assert every name exists in `docs/OSS_REGISTER.md`.
  - Assert GPL/AGPL entries are not `embedded_lib`.
- Open questions:
- Notes:

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
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
