# Task Packet: WP-1-AppState-Refactoring-v3

## METADATA
- TASK_ID: WP-1-AppState-Refactoring-v3
- WP_ID: WP-1-AppState-Refactoring-v3
- BASE_WP_ID: WP-1-AppState-Refactoring
- DATE: 2026-01-09T22:37:39.386Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
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
