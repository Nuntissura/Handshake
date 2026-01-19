# Task Packet: WP-1-OSS-Governance-v2

## METADATA
- TASK_ID: WP-1-OSS-Governance-v2
- WP_ID: WP-1-OSS-Governance-v2
- BASE_WP_ID: WP-1-OSS-Governance (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-19T03:27:56.888Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: gpt-5.2
- CODER_REASONING_STRENGTH: HIGH
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja190120260338
- SUPERSEDES: WP-1-OSS-Governance (historical FAIL; v2 is protocol-clean remediation)

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-OSS-Governance-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Align OSS governance artifacts to SPEC_CURRENT v02.113 by migrating `docs/OSS_REGISTER.md` to the required register schema and updating the backend enforcement test to parse the new schema and enforce dependency coverage + copyleft isolation.
- Why: Prevent license posture drift, preserve deterministic auditability, and satisfy Spec Phase 1 OSS governance requirements (register completeness + isolation + enforcement).
- IN_SCOPE_PATHS:
  - docs/OSS_REGISTER.md
  - src/backend/handshake_core/tests/oss_register_enforcement_tests.rs
  - docs/task_packets/WP-1-OSS-Governance-v2.md (evidence + validation manifest updates only)
- OUT_OF_SCOPE:
  - Any Master Spec edits/version bumps.
  - Any changes to app/ or src/ outside the IN_SCOPE_PATHS above.
  - Do not edit `src/backend/handshake_core/src/workflows.rs` or `src/backend/handshake_core/src/capability_registry_workflow.rs` (owned by WP-1-Capability-SSoT-v2).
  - Do not edit `docs/TASK_BOARD.md` or `docs/WP_TRACEABILITY_REGISTRY.md` (orchestrator maintains).
  - Do not edit `docs/refinements/WP-1-OSS-Governance-v2.md` (locked).
  - If additional file changes are required beyond IN_SCOPE_PATHS, STOP and request an Orchestrator scope update before proceeding.

## DEPENDENCIES
- None.

## WAIVERS GRANTED
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-OSS-Governance-v2

# Backend format/lint/tests (required):
just cargo-clean
just fmt
just lint
cargo test --manifest-path src/backend/handshake_core/Cargo.toml oss_register_enforcement

# Re-run hygiene gates after implementation:
just cargo-clean
just post-work WP-1-OSS-Governance-v2
```

### DONE_MEANS
- `docs/OSS_REGISTER.md` uses the Spec v02.113 register schema required columns (11.7.5.7.1) for its OSS tables; no remaining legacy 5-column header format is used.
- Every OSS register row has columns for: `component_id`, `name`, `upstream_ref`, `license`, `integration_mode_default`, `capabilities_required`, `pinning_policy`, `compliance_notes`, `test_fixture`, `used_by_modules`.
- `src/backend/handshake_core/tests/oss_register_enforcement_tests.rs` parses the new schema and enforces (11.10.4 item 2):
  - Every crate in `src/backend/handshake_core/Cargo.lock` exists in `docs/OSS_REGISTER.md` (match against `name`, case-insensitive).
  - Every npm package in `app/package.json` (dependencies + devDependencies) exists in `docs/OSS_REGISTER.md` (match against `name`, case-insensitive).
  - GPL/AGPL entries are not embedded; they must be `external_process` (per 11.10.4 item 2, and compatible with 11.7.4.3).
- `just pre-work WP-1-OSS-Governance-v2` and `just post-work WP-1-OSS-Governance-v2` pass.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.113.md (recorded_at: 2026-01-19T03:27:56.888Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.113.md:46633 11.7.4.2 License preference order
  - Handshake_Master_Spec_v02.113.md:46642 11.7.4.3 Copyleft isolation rule
  - Handshake_Master_Spec_v02.113.md:46647 11.7.4.4 OSS Component Register requirement
  - Handshake_Master_Spec_v02.113.md:46764 11.7.5.7.1 Register schema (required columns)
  - Handshake_Master_Spec_v02.113.md:47024 11.7.5.10 Conformance tests: Register completeness + License isolation
  - Handshake_Master_Spec_v02.113.md:49552 11.10.4 (2) OSS Register Enforcement (backend unit test)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Prior packet (historical FAIL, outdated): docs/task_packets/WP-1-OSS-Governance.md
- Stub backlog item (planning only): docs/task_packets/stubs/WP-1-OSS-Governance-v2.md
- Preserved: The OSS governance goal (OSS register + copyleft isolation + deterministic enforcement).
- Changed: v2 is protocol-clean (signed refinement, SPEC_CURRENT v02.113 anchors, narrow IN_SCOPE_PATHS, deterministic gates) and targets the current register schema requirements (11.7.5.7.1).

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.113.md
  - docs/refinements/WP-1-OSS-Governance-v2.md
  - docs/task_packets/WP-1-OSS-Governance-v2.md
  - docs/task_packets/WP-1-OSS-Register-Enforcement-v1.md
  - docs/OSS_REGISTER.md
  - src/backend/handshake_core/tests/oss_register_enforcement_tests.rs
  - src/backend/handshake_core/Cargo.lock
  - app/package.json
- SEARCH_TERMS:
  - "11.7.4.2"
  - "11.7.4.3"
  - "11.7.4.4"
  - "11.7.5.7.1"
  - "11.10.4"
  - "OSS_REGISTER"
  - "oss_register_enforcement_tests"
  - "HEADER_PATTERN"
  - "IntegrationMode"
  - "integration_mode_default"
  - "HSK-OSS-00"
  - "Cargo.lock packages not in"
  - "package.json deps not in"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-OSS-Governance-v2

  rg -n "OSS_REGISTER|oss_register_enforcement_tests|HSK-OSS|IntegrationMode|integration_mode_default|component_id|upstream_ref|pinning_policy" -S .

  cargo test --manifest-path src/backend/handshake_core/Cargo.toml oss_register_enforcement
  ```
- RISK_MAP:
  - "Schema mismatch" -> "Parser/test drift; gate fails or silently misses entries"
  - "False negatives" -> "Missing dependencies not detected; auditability breaks"
  - "Copyleft contamination" -> "GPL/AGPL embedded; legal/compliance risk"
  - "Non-deterministic register edits" -> "Unstable diffs; hard to review and validate"

## SKELETON
SKELETON APPROVED
- Proposed interfaces/types/contracts:
  - OSS Register schema (Spec v02.113 11.7.5.7.1):
    - All OSS tables in `docs/OSS_REGISTER.md` use the same 10-column Markdown header (exact match, case + order):
      - `| component_id | name | upstream_ref | license | integration_mode_default | capabilities_required | pinning_policy | compliance_notes | test_fixture | used_by_modules |`
    - Data rows MUST have exactly 10 cells (preserve empty cells; do not drop them during parsing).
    - `integration_mode_default` values are restricted to: `embedded_lib`, `external_process`, `external_service`.
    - `name` is the dependency coverage key (case-insensitive):
      - Rust: `src/backend/handshake_core/Cargo.lock` [[package]] `name`
      - NPM: `app/package.json` dependency keys (dependencies + devDependencies)
    - Copyleft isolation (Spec 11.7.4.3 + 11.10.4 item 2):
      - Treat a row as copyleft if `license` matches `\bAGPL\b|\bGPL\b` (case-insensitive; do not match LGPL/MPL).
      - Copyleft rows MUST have `integration_mode_default == external_process`.
  - Enforcement test contract (`src/backend/handshake_core/tests/oss_register_enforcement_tests.rs`):
    - Replace legacy `HEADER_PATTERN` (5 columns) with the v02.113 schema header (10 columns).
    - Update `RegisterEntry` to parse and retain at minimum: `name`, `license`, `integration_mode_default` (optionally `component_id` for diagnostics).
    - Update Markdown row splitting to preserve empty cells (e.g., `trim_matches('|').split('|')`), then enforce `cols.len() == 10`.
- Open questions:
  - None.
- Notes:
  - Do not start implementation changes until the SKELETON is reviewed and the packet has "SKELETON APPROVED" appended (per gate-check + CX-625).

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.

### Manifest Entry 1: docs/OSS_REGISTER.md
- **Target File**: `docs/OSS_REGISTER.md`
- **Start**: 1
- **End**: 461
- **Line Delta**: 0
- **Pre-SHA1**: `086fb4516c1d1e492cbc88f567018751c61473a0`
- **Post-SHA1**: `5a0d295da4d8d663537c6a8b98a979c60b0d06ee`
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
- **Line Delta**: -23
- **Pre-SHA1**: `33cce89476d9c1ddb719a90eb15733351b833df7`
- **Post-SHA1**: `e2e7bc802b7019b7c938441629411af9dde1a764`
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
- **Lint Results**: `just fmt`, `just lint`
- **Artifacts**: None (repo-tracked)
- **Timestamp**: 2026-01-19T00:00:00Z
- **Operator**: Codex CLI (CODER)
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
- **Notes**: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml oss_register_enforcement`

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS:
- What changed in this update:
- Next step / handoff hint:

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

### Evidence: Post-work evidence
- `git rev-parse HEAD` (implementation commit):
```text
86ce408be60f598a0a29a240666b7a02b86882fd
```

- `just cargo-clean`:
```text
cargo clean -p handshake_core --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Cargo Target/handshake-cargo-target"
     Removed 1317 files, 6.4GiB total
```

- `just post-work WP-1-OSS-Governance-v2`:
```text
Checking Phase Gate for WP-1-OSS-Governance-v2...
? GATE PASS: Workflow sequence verified.

Post-work validation for WP-1-OSS-Governance-v2 (deterministic manifest + gates)...

Check 1: Validation manifest present

Check 2: Manifest fields

Check 3: File integrity (per manifest entry)

Check 4: Git status

==================================================
Post-work validation PASSED with warnings

Warnings:
  1. Manifest[1]: pre_sha1 matches merge-base(9fb57b2572b2ee933784e69e13da486337daae2c) for docs\\OSS_REGISTER.md (common after WP commits); prefer LF blob SHA1=086fb4516c1d1e492cbc88f567018751c61473a0
  2. Manifest[2]: pre_sha1 matches merge-base(9fb57b2572b2ee933784e69e13da486337daae2c) for src\\backend\\handshake_core\\tests\\oss_register_enforcement_tests.rs (common after WP commits); prefer LF blob SHA1=33cce89476d9c1ddb719a90eb15733351b833df7

You may proceed with commit.
? ROLE_MAILBOX_EXPORT_GATE PASS
```

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
