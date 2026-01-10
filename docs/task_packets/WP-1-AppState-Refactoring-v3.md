# Task Packet: WP-1-AppState-Refactoring-v3

## METADATA
- TASK_ID: WP-1-AppState-Refactoring-v3
- WP_ID: WP-1-AppState-Refactoring-v3
- BASE_WP_ID: WP-1-AppState-Refactoring
- DATE: 2026-01-09T22:37:39.386Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja090120262335

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-AppState-Refactoring-v3.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Revalidate AppState refactoring against SPEC_CURRENT (no raw `SqlitePool` / `DuckDbConnection` exposure; all DB access flows through the storage boundary via `Arc<dyn Database>`).
- Why: Phase 1 cannot close without WP-1-AppState-Refactoring (CX-DBP-030); legacy packets fail current workflow gates and must be replaced with a COR-701 compliant v3 packet.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/main.rs
  - src/backend/handshake_core/src/jobs.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - src/backend/handshake_core/src/api/mod.rs
  - src/backend/handshake_core/src/api/bundles.rs
  - src/backend/handshake_core/src/api/canvases.rs
  - src/backend/handshake_core/src/api/diagnostics.rs
  - src/backend/handshake_core/src/api/flight_recorder.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - src/backend/handshake_core/src/api/logs.rs
  - src/backend/handshake_core/src/api/paths.rs
  - src/backend/handshake_core/src/api/workspaces.rs
  - .gitattributes
  - docs/task_packets/WP-1-AppState-Refactoring-v3.md
  - docs/refinements/WP-1-AppState-Refactoring-v3.md
  - docs/TASK_BOARD.md
  - docs/WP_TRACEABILITY_REGISTRY.md
- OUT_OF_SCOPE:
  - Any Master Spec edits/version bumps (Spec is locked; use spec enrichment workflow if needed).
  - Migration framework work (WP-1-Migration-Framework) and dual-backend CI work (WP-1-Dual-Backend-Tests).
  - Any changes to `scripts/` validation tooling (report gaps instead).

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```powershell
# Run before handoff:
just pre-work WP-1-AppState-Refactoring-v3

# Mandatory audit (CX-DBP-010): fail if any direct pool access exists outside storage/
$hits = rg -n "\\bstate\\.pool\\b|\\bstate\\.fr_pool\\b" src/backend/handshake_core/src | Where-Object { $_ -notmatch 'src\\backend\\handshake_core\\src\\storage\\' }
if ($hits) { $hits; throw "FAIL: state.pool/state.fr_pool outside src/backend/handshake_core/src/storage/" } else { "PASS: no state.pool/state.fr_pool outside storage/" }

# AppState surface audit (CX-DBP-030): fail if AppState exposes raw pool types
$hits = rg -n "\\bSqlitePool\\b|\\bDuckDbConnection\\b|\\bfr_pool\\b|\\bsqlite_pool\\b" src/backend/handshake_core/src/lib.rs
if ($hits) { $hits; throw "FAIL: forbidden pool surface in AppState (src/backend/handshake_core/src/lib.rs)" } else { "PASS: AppState has no raw pool surface (src/backend/handshake_core/src/lib.rs)" }

# Storage portability audit (repo-provided guard)
just validator-dal-audit

# If any non-doc files changed:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

just cargo-clean
just post-work WP-1-AppState-Refactoring-v3
```

### DONE_MEANS
- Phase 1 closure requirement satisfied (CX-DBP-030): `AppState` does not expose raw `SqlitePool` or `DuckDbConnection`; `AppState` holds `Arc<dyn Database>` (evidence: audit command output in ## EVIDENCE).
- One Storage API enforced (CX-DBP-010): no `state.pool` / `state.fr_pool` access outside `src/backend/handshake_core/src/storage/` (evidence: audit command output in ## EVIDENCE).
- Trait purity invariant satisfied (CX-DBP-040): `Database` trait does not expose backend-specific pool types or accessors (e.g., no `sqlite_pool()`).
- Storage layer guard passes: `just validator-dal-audit` returns PASS with zero violations.
- Workflow gates pass: `just pre-work WP-1-AppState-Refactoring-v3` and `just post-work WP-1-AppState-Refactoring-v3`.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.103.md (recorded_at: 2026-01-09T22:37:39.386Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.103.md 2.3.12.5 [CX-DBP-030]
  - Handshake_Master_Spec_v02.103.md 2.3.12.1 [CX-DBP-010]
  - Handshake_Master_Spec_v02.103.md 2.3.12.3 [CX-DBP-040]
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.103.md
  - docs/TASK_BOARD.md
  - docs/WP_TRACEABILITY_REGISTRY.md
  - src/backend/handshake_core/src/lib.rs
  - src/backend/handshake_core/src/main.rs
  - src/backend/handshake_core/src/api/mod.rs
  - src/backend/handshake_core/src/storage/mod.rs
- SEARCH_TERMS:
  - "CX-DBP-030"
  - "CX-DBP-010"
  - "CX-DBP-040"
  - "pub struct AppState"
  - "Arc<dyn Database>"
  - "SqlitePool"
  - "DuckDbConnection"
  - "fr_pool"
  - "sqlite_pool"
  - "state.pool"
  - "state.fr_pool"
  - "sqlx::query"
  - "trait Database"
- RUN_COMMANDS:
  ```bash
  just validator-dal-audit
  ```
- RISK_MAP:
  - "Pool leakage path remains" -> Violates portability; blocks Phase 1 closure (CX-DBP-030)
  - "Direct pool access in handlers/services" -> Violates One Storage API; portability breaks (CX-DBP-010)
  - "Trait impurity regression" -> Backend-specific types leak into business logic (CX-DBP-040)

## SKELETON
- Proposed interfaces/types/contracts:
  - `AppState` continues to expose only `storage: Arc<dyn Database>` (no raw pools on the AppState surface).
  - `Database` trait remains backend-agnostic (no backend pool accessors; no `SqlitePool`/`DuckDbConnection` types in the trait surface).
- Open questions:
  - None. Next step is to run the audits in TEST_PLAN and capture evidence; remediate only if any violations are found.
- Notes:
  - Initial grep audits show no `state.pool`/`state.fr_pool` access outside `src/backend/handshake_core/src/storage/` and no forbidden pool identifiers in `src/backend/handshake_core/src/lib.rs`.

SKELETON APPROVED

## IMPLEMENTATION
- Ran revalidation audits per TEST_PLAN (grep audits + `just validator-dal-audit`).
- No code changes required based on the audit evidence recorded in `## EVIDENCE`.

## HYGIENE
- Ran: `just validator-scan`, `just validator-dal-audit`, `just validator-git-hygiene`.
- Ran: `just cargo-clean`.
- Skipped: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` (no non-doc file changes in this WP; TEST_PLAN conditional).

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/src/lib.rs`
- **Start**: 24
- **End**: 32
- **Line Delta**: 0
- **Pre-SHA1**: `06feb3889dec4667bbeb8a1c3192e61df096acd8`
- **Post-SHA1**: `06feb3889dec4667bbeb8a1c3192e61df096acd8`
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
- **Target File**: `src/backend/handshake_core/src/storage/mod.rs`
- **Start**: 738
- **End**: 851
- **Line Delta**: 0
- **Pre-SHA1**: `e189a0045bec8b6d990637ae34548095658adcde`
- **Post-SHA1**: `e189a0045bec8b6d990637ae34548095658adcde`
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
- **Target File**: `.gitattributes`
- **Start**: 1
- **End**: 4
- **Line Delta**: 0
- **Pre-SHA1**: `c28054853382463cf1c8bd32fc5a0e4c7938bd3b`
- **Post-SHA1**: `c28054853382463cf1c8bd32fc5a0e4c7938bd3b`
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
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.103.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress (implementation complete; awaiting Validator review/status-sync)
- What changed in this update: Recorded SKELETON approval, captured audit evidence, and filled COR-701 manifest (no non-doc file changes).
- Post-work gate: PASS (`just post-work WP-1-AppState-Refactoring-v3`; output recorded in `## EVIDENCE`)
- Next step / handoff hint: Validator can merge bootstrap claim commit `fc2ae8ab` into `main` and status-sync `docs/TASK_BOARD.md` on `main`.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

- Command: just pre-work WP-1-AppState-Refactoring-v3
  Output:
  ```text
  Checking Phase Gate for WP-1-AppState-Refactoring-v3...
  ? GATE PASS: Workflow sequence verified.

  Pre-work validation for WP-1-AppState-Refactoring-v3...

  Check 1: Task packet file exists
  PASS: Found WP-1-AppState-Refactoring-v3.md

  Check 2: Task packet structure
  PASS: All required fields present

  Check 2.7: Technical Refinement gate
  PASS: Refinement file exists and is approved/signed

  Check 2.8: WP checkpoint commit gate

  Check 3: Deterministic manifest template
  PASS: Manifest fields present
  PASS: Gates checklist present

  ==================================================
  Pre-work validation PASSED

  You may proceed with implementation.
  ```

- Command: rg -n "\\bstate\\.pool\\b|\\bstate\\.fr_pool\\b" src/backend/handshake_core/src (excluding src/backend/handshake_core/src/storage/)
  Output:
  ```text
  (no hits)
  ```

- Command: rg -n "\\bSqlitePool\\b|\\bDuckDbConnection\\b|\\bfr_pool\\b|\\bsqlite_pool\\b" src/backend/handshake_core/src/lib.rs
  Output:
  ```text
  (no hits)
  ```

- Command: rg -n "\\bSqlitePool\\b|\\bDuckDbConnection\\b" src/backend/handshake_core/src/storage/mod.rs
  Output:
  ```text
  (no hits)
  ```

- Command: just validator-dal-audit
  Output:
  ```text
  validator-dal-audit: PASS (DAL checks clean).
  ```

- Command: just validator-scan
  Output:
  ```text
  validator-scan: PASS - no forbidden patterns detected in backend sources.
  ```

- Command: just validator-git-hygiene
  Output:
  ```text
  validator-git-hygiene: PASS - .gitignore coverage and artifact checks clean.
  ```

- Command: just cargo-clean
  Output:
  ```text
  cargo clean -p handshake_core --manifest-path src/backend/handshake_core/Cargo.toml --target-dir \"../Cargo Target/handshake-cargo-target\"
       Removed 0 files
  ```

- Command: just post-work WP-1-AppState-Refactoring-v3
  Output:
  ```text
  Checking Phase Gate for WP-1-AppState-Refactoring-v3...
  ? GATE PASS: Workflow sequence verified.

  Post-work validation for WP-1-AppState-Refactoring-v3 (deterministic manifest + gates)...

  Check 1: Validation manifest present
  warning: in the working copy of 'docs/task_packets/WP-1-AppState-Refactoring-v3.md', LF will be replaced by CRLF the next time Git touches it

  Check 2: Manifest fields

  Check 3: File integrity (per manifest entry)

  Check 4: Git status
  warning: in the working copy of 'docs/task_packets/WP-1-AppState-Refactoring-v3.md', LF will be replaced by CRLF the next time Git touches it

  ==================================================
  Post-work validation PASSED

  You may proceed with commit.
  ```

- Command: just post-work WP-1-AppState-Refactoring-v3 (after LF line-ending fix)
  Output:
  ```text
  Checking Phase Gate for WP-1-AppState-Refactoring-v3...
  ? GATE PASS: Workflow sequence verified.

  Post-work validation for WP-1-AppState-Refactoring-v3 (deterministic manifest + gates)...

  Check 1: Validation manifest present

  Check 2: Manifest fields

  Check 3: File integrity (per manifest entry)

  Check 4: Git status

  ==================================================
  Post-work validation PASSED

  You may proceed with commit.
  ```

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
