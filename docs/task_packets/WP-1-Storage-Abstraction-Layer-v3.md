# Task Packet: WP-1-Storage-Abstraction-Layer-v3

## METADATA
- TASK_ID: WP-1-Storage-Abstraction-Layer-v3
- WP_ID: WP-1-Storage-Abstraction-Layer-v3
- BASE_WP_ID: WP-1-Storage-Abstraction-Layer
- DATE: 2026-01-09T19:03:58.905Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** Done
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja090120261951

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Storage-Abstraction-Layer-v3.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Revalidate the Storage Abstraction Layer (One Storage API + Trait Purity) against SPEC_CURRENT and ensure the Phase 1 closure audit is satisfied (no `sqlx::` or `SqlitePool` references outside `src/backend/handshake_core/src/storage/`).
- Why: Phase 1 cannot close without WP-1-Storage-Abstraction-Layer (CX-DBP-030); storage portability must be enforced by auditable, deterministic rules, not historical claims.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/storage/retention.rs
  - docs/task_packets/WP-1-Storage-Abstraction-Layer-v3.md
  - docs/refinements/WP-1-Storage-Abstraction-Layer-v3.md
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
```bash
# Run before handoff:
just pre-work WP-1-Storage-Abstraction-Layer-v3

# Mandatory audit (CX-DBP-030): fail if any hits exist outside storage/
$hits = rg -n "sqlx::" src | Where-Object { $_ -notmatch 'src\\backend\\handshake_core\\src\\storage\\' }
if ($hits) { $hits; throw "FAIL: sqlx:: outside src/backend/handshake_core/src/storage/" } else { "PASS: no sqlx:: outside storage/" }

$hits = rg -n "\\bSqlitePool\\b" src | Where-Object { $_ -notmatch 'src\\backend\\handshake_core\\src\\storage\\' }
if ($hits) { $hits; throw "FAIL: SqlitePool outside src/backend/handshake_core/src/storage/" } else { "PASS: no SqlitePool outside storage/" }

# Storage portability audit (repo-provided guard)
just validator-dal-audit

# If any non-doc files changed:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

just cargo-clean
just post-work WP-1-Storage-Abstraction-Layer-v3
```

### DONE_MEANS
- Phase 1 closure audit satisfied (CX-DBP-030): no `sqlx::` references outside `src/backend/handshake_core/src/storage/` (evidence: command output in ## EVIDENCE).
- Phase 1 closure audit satisfied (CX-DBP-030): no `SqlitePool` references outside `src/backend/handshake_core/src/storage/` (evidence: command output in ## EVIDENCE).
- Trait purity invariant satisfied (CX-DBP-040): `Database` trait does not expose backend-specific pool types or accessors (e.g., no `sqlite_pool()`).
- Storage layer guard passes: `just validator-dal-audit` returns PASS with zero violations.
- Workflow gates pass: `just pre-work WP-1-Storage-Abstraction-Layer-v3` and `just post-work WP-1-Storage-Abstraction-Layer-v3`.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.103.md (recorded_at: 2026-01-09T19:03:58.905Z)
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
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
- SEARCH_TERMS:
  - "CX-DBP-030"
  - "CX-DBP-010"
  - "CX-DBP-040"
  - "sqlx::"
  - "SqlitePool"
  - "sqlite_pool"
  - "state.pool"
  - "fr_pool"
  - "trait Database"
  - "Arc<dyn Database>"
- RUN_COMMANDS:
  ```bash
  just validator-dal-audit
  ```
- RISK_MAP:
  - "Hidden pool leakage" -> Violates portability; blocks Phase 1 closure (CX-DBP-030)
  - "Trait impurity" -> Backend-specific types leak into business logic; portability breaks (CX-DBP-040)
  - "Validator false negative" -> Manual audit commands must still be executed and recorded (CX-DBP-030)

## SKELETON
- Proposed interfaces/types/contracts:
  - (a) Audit-only; no code changes expected.
  - Re-verify invariants against SPEC_CURRENT:
    - [CX-DBP-010] One Storage API: all DB access stays behind `src/backend/handshake_core/src/storage/` via `Arc<dyn Database>`.
    - [CX-DBP-040] Trait purity: `crate::storage::Database` exposes no backend-specific pool types/accessors.
    - [CX-DBP-030] Mandatory audit: zero `sqlx::` / `SqlitePool` references outside `src/backend/handshake_core/src/storage/`.
- Open questions:
  - None if audits remain clean. If any out-of-scope hits are found, stop and request scope expansion before changing code.
- Notes:
  - Expected touched files (docs-only): `docs/task_packets/WP-1-Storage-Abstraction-Layer-v3.md` (SKELETON + EVIDENCE/HYGIENE/STATUS_HANDOFF updates).
  - No trait/signature changes planned (including `Database`); no pool-leak removals expected based on initial scans.

SKELETON APPROVED

## IMPLEMENTATION
- Audit-only (no code changes expected).
- Ran the packet TEST_PLAN audits and hygiene commands; outputs recorded in `## EVIDENCE`.

## HYGIENE
- Audit-only hygiene run (no code changes).
- Commands executed (see `## EVIDENCE` for raw outputs):
  - `just validator-scan`
  - `just validator-dal-audit`
  - `just validator-git-hygiene`
  - `just cargo-clean`

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `src/backend/handshake_core/src/storage/mod.rs`
- **Start**: 1
- **End**: 874
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
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.103.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: Done (validated)
- What changed in this update: Recorded SKELETON approval marker; executed audit/hygiene commands; appended raw outputs to `## EVIDENCE`.
- Next step / handoff hint: Run `just post-work WP-1-Storage-Abstraction-Layer-v3` with staged packet changes, then commit evidence for Validator review (no code changes expected).

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

### 2026-01-09 Audit Evidence (WP-1-Storage-Abstraction-Layer-v3)

```text
Command: just pre-work WP-1-Storage-Abstraction-Layer-v3
Output:
Checking Phase Gate for WP-1-Storage-Abstraction-Layer-v3...
? GATE PASS: Workflow sequence verified.

Pre-work validation for WP-1-Storage-Abstraction-Layer-v3...

Check 1: Task packet file exists
PASS: Found WP-1-Storage-Abstraction-Layer-v3.md

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

```text
Command: Mandatory audit (sqlx:: / SqlitePool outside src/backend/handshake_core/src/storage/)
Output:
HITS(sqlx:: outside src/backend/handshake_core/src/storage/): <none>
HITS(SqlitePool outside src/backend/handshake_core/src/storage/): <none>
```

```text
Command: just validator-dal-audit
Output:
validator-dal-audit: PASS (DAL checks clean).
```

```text
Command: just validator-scan
Output:
validator-scan: PASS - no forbidden patterns detected in backend sources.
```

```text
Command: just validator-git-hygiene
Output:
validator-git-hygiene: PASS - .gitignore coverage and artifact checks clean.
```

```text
Command: just cargo-clean
Output:
cargo clean -p handshake_core --manifest-path src/backend/handshake_core/Cargo.toml --target-dir \"../Cargo Target/handshake-cargo-target\"
     Removed 0 files
```

```text
Command: just post-work WP-1-Storage-Abstraction-Layer-v3
Output:
Checking Phase Gate for WP-1-Storage-Abstraction-Layer-v3...
? GATE PASS: Workflow sequence verified.

Post-work validation for WP-1-Storage-Abstraction-Layer-v3 (deterministic manifest + gates)...

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

### 2026-01-09 VALIDATION REPORT - WP-1-Storage-Abstraction-Layer-v3
Verdict: PASS

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Storage-Abstraction-Layer-v3.md (status: Done)
- Spec: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.103.md
  - 2.3.12.1 [CX-DBP-010] (Pillar 1: One Storage API)
  - 2.3.12.3 [CX-DBP-040] (Trait Purity Invariant)
  - 2.3.12.5 [CX-DBP-030] (Phase 1 mandatory audit)
- Codex: Handshake Codex v1.4.md
- Protocol: docs/VALIDATOR_PROTOCOL.md

Files Checked:
- docs/task_packets/WP-1-Storage-Abstraction-Layer-v3.md
- docs/refinements/WP-1-Storage-Abstraction-Layer-v3.md
- docs/SPEC_CURRENT.md
- Handshake_Master_Spec_v02.103.md
- docs/TASK_BOARD.md
- docs/WP_TRACEABILITY_REGISTRY.md
- src/backend/handshake_core/src/storage/mod.rs
- src/backend/handshake_core/src/lib.rs

Findings:
- [CX-DBP-030] Mandatory audit (spec: Handshake_Master_Spec_v02.103.md:3104):
  - PASS: no sqlx:: outside storage (docs/task_packets/WP-1-Storage-Abstraction-Layer-v3.md:217)
  - PASS: no SqlitePool outside storage (docs/task_packets/WP-1-Storage-Abstraction-Layer-v3.md:218)
- [CX-DBP-010] One Storage API (spec: Handshake_Master_Spec_v02.103.md:2909):
  - PASS: AppState uses trait object storage only (src/backend/handshake_core/src/lib.rs:26)
  - PASS: validator-dal-audit clean (docs/task_packets/WP-1-Storage-Abstraction-Layer-v3.md:224)
- [CX-DBP-040] Trait Purity Invariant (spec: Handshake_Master_Spec_v02.103.md:3009):
  - PASS: Database trait surface exposes no backend-specific pool types/accessors (src/backend/handshake_core/src/storage/mod.rs:739)
- Forbidden Patterns [CX-573E]:
  - PASS: validator-scan clean (docs/task_packets/WP-1-Storage-Abstraction-Layer-v3.md:230)
- Storage DAL Audit (CX-DBP-VAL-010..014):
  - PASS: validator-dal-audit clean (docs/task_packets/WP-1-Storage-Abstraction-Layer-v3.md:224)

Tests / Commands:
- just pre-work WP-1-Storage-Abstraction-Layer-v3: PASS (docs/task_packets/WP-1-Storage-Abstraction-Layer-v3.md:209)
- just post-work WP-1-Storage-Abstraction-Layer-v3: PASS (docs/task_packets/WP-1-Storage-Abstraction-Layer-v3.md:263)
  - Note: rerunning post-work after committing a docs-only change can fail due to "git status clean" enforcement in scripts/validation/post-work-check.mjs.
- cargo test: NOT RUN (no non-doc files changed; conditional in TEST_PLAN).

REASON FOR PASS:
- Spec acceptance audit satisfied: zero sqlx:: and SqlitePool references outside src/backend/handshake_core/src/storage/ (CX-DBP-030).
- Storage API remains trait-based and backend-agnostic (CX-DBP-010, CX-DBP-040).
- Validator hygiene gates pass (validator-scan, validator-dal-audit, validator-git-hygiene); no forbidden patterns detected.
