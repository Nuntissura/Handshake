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
- **Status:** In Progress
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
- Current WP_STATUS: In Progress (SKELETON pending approval)
- What changed in this update: Claimed CODER_MODEL + CODER_REASONING_STRENGTH; filled SKELETON (audit-only).
- Next step / handoff hint: Await "SKELETON APPROVED"; then append audit command outputs to ## EVIDENCE and proceed through HYGIENE/EVALUATION per TEST_PLAN (no code changes expected).

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
