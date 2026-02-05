# Task Packet: WP-1-OSS-Register-Enforcement-v1

## METADATA
- TASK_ID: WP-1-OSS-Register-Enforcement-v1
- WP_ID: WP-1-OSS-Register-Enforcement-v1
- DATE: 2026-01-01T14:29:22.074Z
- REQUESTOR: ilja
- AGENT_ID: Codex CLI (Orchestrator)
- ROLE: Orchestrator
- **Status:** Done
- RISK_TIER: HIGH
- RISK_TIER_JUSTIFICATION: This gate prevents OSS register drift and copyleft (GPL/AGPL) in-process contamination, which is a Phase 1 closure and distribution-risk blocker.
- USER_SIGNATURE: ilja010120261528

## USER_CONTEXT (Non-Technical Explainer)
The OSS Register is the project's "bill of materials" for open-source components. This packet adds an automated test so that if someone adds a dependency without registering it (or adds GPL/AGPL code in-process), the build fails immediately instead of discovering the issue later.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-OSS-Register-Enforcement-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Add OSS register enforcement per SPEC_CURRENT v02.100 by implementing a backend unit test that:
  - checks that dependencies from `src/backend/handshake_core/Cargo.lock` and `app/package.json` exist in `.GOV/roles_shared/OSS_REGISTER.md`, and
  - enforces the copyleft isolation rule: GPL/AGPL components MUST NOT be `embedded_lib` (must be `external_process` or `external_service`).
- Why: SPEC_CURRENT explicitly requires this enforcement; without it, OSS provenance and license posture can drift silently.
- IN_SCOPE_PATHS:
  - .GOV/roles_shared/OSS_REGISTER.md
  - src/backend/handshake_core/tests/oss_register_enforcement_tests.rs
- OUT_OF_SCOPE:
  - Any changes to `src/backend/handshake_core/src/**` (this WP is test + register only).
  - Any changes to `Cargo.toml` / `Cargo.lock` / `app/package.json` (no dependency changes; enforcement only).
  - Any UI changes in `app/` or `src/frontend/`.
  - Any changes to files listed in `.GOV/task_packets/WP-1-Flight-Recorder-v3.md` IN_SCOPE_PATHS (parallel coder safety).

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
- `.GOV/roles_shared/OSS_REGISTER.md` includes an explicit `IntegrationMode` field for every entry as one of: `embedded_lib`, `external_process`, `external_service`.
- `src/backend/handshake_core/tests/oss_register_enforcement_tests.rs` exists and deterministically fails if:
  - any crate in `src/backend/handshake_core/Cargo.lock` is missing from `.GOV/roles_shared/OSS_REGISTER.md`, or
  - any npm dependency in `app/package.json` (dependencies + devDependencies) is missing from `.GOV/roles_shared/OSS_REGISTER.md`, or
  - any register entry with license `GPL-*` or `AGPL-*` is marked as `embedded_lib`.
- All commands in TEST_PLAN pass and `just post-work WP-1-OSS-Register-Enforcement-v1` returns PASS (COR-701 manifest filled; ASCII-only packet).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.100.md (recorded_at: 2026-01-01T14:29:22.074Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.100.md 11.10.4.2 (OSS Register Enforcement) and 11.7.4.3-11.7.4.4 (Copyleft isolation + OSS Register requirement)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.100.md
  - .GOV/roles_shared/OSS_REGISTER.md
  - src/backend/handshake_core/Cargo.lock
  - src/backend/handshake_core/Cargo.toml
  - app/package.json
  - .GOV/scripts/validation/validator-spec-regression.mjs
  - .GOV/task_packets/WP-1-Flight-Recorder-v3.md
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
  - "Register format drift" -> "parser breaks; document a strict minimal format in .GOV/roles_shared/OSS_REGISTER.md"
  - "Copyleft isolation not enforced" -> "legal/compliance risk"

## SKELETON
- Proposed interfaces/types/contracts:
- Minimal deterministic format requirement for `.GOV/roles_shared/OSS_REGISTER.md`:
  - Each row MUST include: `Component`, `License`, `IntegrationMode`.
  - IntegrationMode MUST be one of: `embedded_lib`, `external_process`, `external_service`.
- Unit test contract:
  - Enumerate crate package names from `src/backend/handshake_core/Cargo.lock`.
  - Enumerate npm package names from `app/package.json` dependencies + devDependencies.
  - Assert every name exists in `.GOV/roles_shared/OSS_REGISTER.md`.
  - Assert GPL/AGPL entries are not `embedded_lib`.
- Open questions:
- Notes:

## IMPLEMENTATION
- SKELETON APPROVED received 2026-01-01
- Updated .GOV/roles_shared/OSS_REGISTER.md: Added IntegrationMode column to all tables, added all 429 Cargo.lock transitive deps
- Created src/backend/handshake_core/tests/oss_register_enforcement_tests.rs with 5 tests

## HYGIENE
- cargo test oss_register_enforcement: 5 passed
- cargo test (full): 157 passed
- just validator-scan: PASS
- just cargo-clean: 6.0GiB removed

## VALIDATION
### Manifest Entry 1: .GOV/roles_shared/OSS_REGISTER.md
- **Target File**: `.GOV/roles_shared/OSS_REGISTER.md`
- **Start**: 1
- **End**: 461
- **Line Delta**: 378
- **Pre-SHA1**: `e4cb58703342563d22881c5b2e72073ce49a68b3`
- **Post-SHA1**: `56844d12956c6fe917459a9c6c603e8b28dbab59`
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

### Manifest Entry 2: src/backend/handshake_core/tests/oss_register_enforcement_tests.rs
- **Target File**: `src/backend/handshake_core/tests/oss_register_enforcement_tests.rs`
- **Start**: 1
- **End**: 314
- **Line Delta**: 314
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `7eaf8e5068375641c164ce6e47240badfc5b5ff2`
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

- **Lint Results**: validator-scan PASS
- **Artifacts**: None
- **Timestamp**: 2026-01-01T15:30:00Z
- **Operator**: Coder-B (Claude Opus 4.5)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.100.md
- **Notes**: New file creation for test; full register update for IntegrationMode column

## STATUS_HANDOFF
- Current WP_STATUS: Implementation complete, tests passing
- What changed in this update:
  - .GOV/roles_shared/OSS_REGISTER.md: Added IntegrationMode column per S11.7.4.4, added all 429 Cargo.lock packages
  - src/backend/handshake_core/tests/oss_register_enforcement_tests.rs: New file with 5 enforcement tests
- Next step / handoff hint: Ready for Validator review

## EVIDENCE
### Test Run: cargo test oss_register_enforcement
```
running 5 tests
test oss_register_enforcement::test_no_gpl_agpl_present ... ok
test oss_register_enforcement::test_register_format_valid ... ok
test oss_register_enforcement::test_copyleft_isolation ... ok
test oss_register_enforcement::test_package_json_coverage ... ok
test oss_register_enforcement::test_cargo_lock_coverage ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Test Run: cargo test (full backend)
```
test result: ok. 157 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Hygiene: just validator-scan
```
validator-scan: PASS - no forbidden patterns detected in backend sources.
```

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

## WAIVERS GRANTED [CX-573F] (APPEND-ONLY)
- [WAIVER-2026-01-01-WORKFLOW-ROLLOUT] Coders not yet aligned to new branch/worktree workflow; allowed for this WP only. Approver: ilja. Expires: on WP closure.

## VALIDATION_REPORTS
- VALIDATION REPORT - WP-1-OSS-Register-Enforcement-v1
  - Verdict: PASS
  - Spec checked:
    - Handshake_Master_Spec_v02.100.md:33905 (11.10.4 OSS Register Enforcement)
    - Handshake_Master_Spec_v02.100.md:30995 (11.7.4.3 Copyleft isolation rule)
    - Handshake_Master_Spec_v02.100.md:31000 (11.7.4.4 OSS Component Register requirement)
  - Evidence mapping (requirements -> code/docs):
    - OSS register contains IntegrationMode column with canonical values:
      - .GOV/roles_shared/OSS_REGISTER.md:11
    - Deterministic enforcement test exists and is strict:
      - src/backend/handshake_core/tests/oss_register_enforcement_tests.rs:47 (strict header + allowed IntegrationMode values)
      - src/backend/handshake_core/tests/oss_register_enforcement_tests.rs:225 (Cargo.lock coverage)
      - src/backend/handshake_core/tests/oss_register_enforcement_tests.rs:246 (package.json deps+devDeps coverage)
      - src/backend/handshake_core/tests/oss_register_enforcement_tests.rs:267 (GPL/AGPL must be external_process)
  - Commands executed (validator):
    - just cargo-clean: PASS
    - just validator-spec-regression: PASS
    - cargo test --manifest-path src/backend/handshake_core/Cargo.toml oss_register_enforcement: PASS (5 tests)
    - cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS
    - just validator-scan: PASS
    - just post-work WP-1-OSS-Register-Enforcement-v1: PASS (staged-only; warning about unrelated unstaged changes)
  - REASON FOR PASS:
    - DONE_MEANS satisfied with direct evidence and passing TEST_PLAN commands; deterministic post-work gate passes for the staged WP diff set.

