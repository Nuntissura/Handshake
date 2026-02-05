# Task Packet: WP-1-Storage-Foundation-v3

## METADATA
- TASK_ID: WP-1-Storage-Foundation-v3
- WP_ID: WP-1-Storage-Foundation-v3
- DATE: 2025-12-31T04:04:35.586Z
- REQUESTOR: ilja
- AGENT_ID: Codex CLI (Orchestrator)
- ROLE: Orchestrator
- **Status:** Done
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja311220250445

## User Context (Non-Technical)
We are fixing a core portability rule: only the storage layer may talk directly to the database library.
Right now a shared type file (`models.rs`) exposes database-library error types (`sqlx::...`) outside storage.
This breaks the "portable storage boundary" and blocks Phase 1 closure until the leak is removed.

## SCOPE
- What: Remove `sqlx::`-typed errors from `src/backend/handshake_core/src/models.rs` so no database-library types appear outside `src/backend/handshake_core/src/storage/`.
- Why: Master Spec v02.98 requires a trait-pure storage boundary and a mandatory audit proving zero `sqlx::`/`SqlitePool` usage outside storage for Phase 1 closure.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/models.rs
  - src/backend/handshake_core/src/storage/mod.rs (reference; may change only if required to keep errors provider-opaque)
- OUT_OF_SCOPE:
  - Any new database features, migrations, or schema changes
  - Any direct DB access added outside `src/backend/handshake_core/src/storage/`
  - Frontend / UI changes

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Storage-Foundation-v3

# Mandatory audit (Master Spec v02.98 2.3.12.5): must be EMPTY
rg -n "sqlx::" src/backend/handshake_core/src --glob '!storage/**'
rg -n "SqlitePool" src/backend/handshake_core/src --glob '!storage/**'

# Compile + tests
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Storage boundary audit (cross-check)
just validator-dal-audit

# External target hygiene (required by Validator Protocol)
just cargo-clean

# Deterministic closure gate (requires VALIDATION manifest filled)
just post-work WP-1-Storage-Foundation-v3
```

### DONE_MEANS
- Mandatory audit passes (spec 2.3.12.5): `rg -n "sqlx::" src/backend/handshake_core/src --glob '!storage/**'` returns zero matches.
- Mandatory audit passes (spec 2.3.12.5): `rg -n "SqlitePool" src/backend/handshake_core/src --glob '!storage/**'` returns zero matches.
- `src/backend/handshake_core/src/models.rs` contains zero references to `sqlx::` and does not expose backend-specific DB types (spec 2.3.12.3).
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` passes.
- `just validator-dal-audit` passes.
- `just post-work WP-1-Storage-Foundation-v3` passes with a fully filled COR-701 VALIDATION manifest (ASCII-only packet).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.98.md (recorded_at: 2025-12-31T04:04:35.586Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.98.md sections 2.3.12.3 (CX-DBP-021) and 2.3.12.5 (CX-DBP-030)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- Approval: Task packet creation authorized by USER_SIGNATURE `ilja311220250445` (no spec enrichment proposed)

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - Handshake_Master_Spec_v02.98.md (sections 2.3.12.3 and 2.3.12.5)
  - src/backend/handshake_core/src/models.rs
  - src/backend/handshake_core/src/storage/mod.rs (StorageError boundary)
  - .GOV/scripts/validation/validator-dal-audit.mjs (understand audit expectations)
- SEARCH_TERMS:
  - "pub enum Error"
  - "sqlx::Error"
  - "sqlx::migrate::MigrateError"
  - "StorageError"
  - "Trait Purity Invariant"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Storage-Foundation-v3
  rg -n "sqlx::" src/backend/handshake_core/src --glob '!storage/**'
  rg -n "SqlitePool" src/backend/handshake_core/src --glob '!storage/**'
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just validator-dal-audit
  ```
- RISK_MAP:
  - "Remove sqlx types but break compilation" -> "Build fails; fix by updating call sites or changing Error shape"
  - "Accidentally reintroduce sqlx outside storage" -> "Mandatory audit fails; blocks Phase 1 closure"
  - "Lose useful error detail" -> "Harder debugging; ensure error strings preserve enough context"

## SKELETON
- Proposed interfaces/types/contracts:
  - Replace `crate::models::Error` sqlx variants with a provider-opaque `Storage(#[from] crate::storage::StorageError)` variant.
  - Use `#[error(transparent)]` on the new Storage variant to preserve existing error text.
  - `crate::models::Error` contains zero `sqlx::` references; no DB-library types appear outside storage.
- Expected file changes (exact):
  - src/backend/handshake_core/src/models.rs
- Open questions:
  - None (rg shows no `models::Error` usages outside models; re-check before implementation).
- Notes:
  - Do not add any new `sqlx::` usage outside `src/backend/handshake_core/src/storage/`.
  - If any additional file changes are required, stop and update SKELETON before implementation.

## IMPLEMENTATION
- Replaced sqlx-derived variants in `crate::models::Error` with a provider-opaque `StorageError` wrapper to keep DB types inside storage.

## HYGIENE
- just pre-work WP-1-Storage-Foundation-v3: PASS
- rg -n "sqlx::" src/backend/handshake_core/src --glob '!storage/**': matches only in storage paths (glob limitation on this host)
- rg -n "SqlitePool" src/backend/handshake_core/src --glob '!storage/**': matches only in storage paths (glob limitation on this host)
- rg -n "sqlx::" src/backend/handshake_core/src --glob '!**/storage/**': PASS (no matches)
- rg -n "SqlitePool" src/backend/handshake_core/src --glob '!**/storage/**': PASS (no matches)
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS
- just validator-dal-audit: PASS
- just cargo-clean: PASS
- Manual validation: COMPLETE (automated review removed per policy)
- just post-work WP-1-Storage-Foundation-v3: PASS (LF->CRLF warnings from git)
- rg -n "split_whitespace|unwrap\\(|todo!\\(|unimplemented!\\(|panic!\\(|expect\\(" src/backend/handshake_core/src/models.rs: PASS (no matches)

## VALIDATION
- Target File: `src/backend/handshake_core/src/models.rs`
- Start: 9
- End: 13
- Line Delta: -3
- Pre-SHA1: `2ac71bd55946aff534dd9f8d231d77440da3c3e8`
- Post-SHA1: `6898f215475ced6a5286a10d33cd828a0f7c10f2`
- Gates Passed:
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
- Lint Results: cargo test PASS; validator-dal-audit PASS
- Artifacts: None
- Timestamp: 2025-12-31T05:55:35+01:00
- Operator: Codex CLI (Coder)
- Spec Target Resolved: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.98.md
- Notes: rg with --glob '!storage/**' includes storage hits; corrected --glob '!**/storage/**' shows no matches outside storage.

## STATUS_HANDOFF
- Current WP_STATUS: Done
- What changed in this update: Moved to In Progress; SKELETON proposed.
- What changed in this update (2025-12-31): Implemented models::Error StorageError wrapper; validation commands executed; ready for validator review.
- Next step / handoff hint: Await "SKELETON APPROVED" before implementation; then run TEST_PLAN + `just post-work`.
- Next step / handoff hint (post-impl): Validator review; if PASS, move WP to Done.

---

## VALIDATION REPORT - 2025-12-31 (Validator)
Verdict: PASS

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Storage-Foundation-v3.md (**Status:** Done)
- Spec (SPEC_CURRENT): .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.98.md (2.3.12.3 [CX-DBP-021], 2.3.12.5 [CX-DBP-030])
- Codex: Handshake Codex v1.4.md
- Protocol: .GOV/roles/validator/VALIDATOR_PROTOCOL.md

Files Checked:
- src/backend/handshake_core/src/models.rs
- src/backend/handshake_core/src/storage/mod.rs
- .GOV/task_packets/WP-1-Storage-Foundation-v3.md
- .GOV/roles_shared/TASK_BOARD.md
- .GOV/roles_shared/SPEC_CURRENT.md
- Handshake_Master_Spec_v02.98.md
- .gitignore

Commands Run:
- node .GOV/scripts/validation/gate-check.mjs WP-1-Storage-Foundation-v3: PASS
- node .GOV/scripts/validation/pre-work-check.mjs WP-1-Storage-Foundation-v3: PASS
- rg -n "sqlx::" src/backend/handshake_core/src --glob '!**/storage/**': PASS (no matches)
- rg -n "SqlitePool" src/backend/handshake_core/src --glob '!**/storage/**': PASS (no matches)
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS
- just validator-scan: PASS
- just validator-dal-audit: PASS
- just cargo-clean: PASS
- just post-work WP-1-Storage-Foundation-v3: PASS (warnings: WP file untracked for concurrency check; git status check unavailable in this environment)
- just validator-git-hygiene: FAIL (tooling EPERM spawning cmd.exe); manual equivalent checks ran:
  - .gitignore required patterns present: PASS
  - git ls-files artifact regex: PASS (no matches)
  - git ls-files --others --exclude-standard (>10MB): PASS (none)

Findings:
- [CX-DBP-021] Trait purity: SATISFIED. `crate::models::Error` no longer exposes `sqlx::...` types (src/backend/handshake_core/src/models.rs).
- [CX-DBP-030] Mandatory audit: SATISFIED. No `sqlx::` or `SqlitePool` references exist outside `src/backend/handshake_core/src/storage/` per the commands above.
- COR-701 manifest: PRESENT and post-work gate passes (.GOV/task_packets/WP-1-Storage-Foundation-v3.md).

Tests:
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS

REASON FOR PASS:
- DONE_MEANS satisfied with validator-run audits/tests; implementation matches SKELETON and is confined to the in-scope target file.



