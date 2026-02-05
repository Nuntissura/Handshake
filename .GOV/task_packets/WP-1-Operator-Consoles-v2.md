# Task Packet: WP-1-Operator-Consoles-v2

## Metadata
- **TASK_ID**: WP-1-Operator-Consoles-v2
- **DATE**: 2025-12-29
- **REQUESTOR**: User
- **AGENT_ID**: validator-gpt
- **ROLE**: Validator (acting as Coder for remediation)
- **STATUS**: Done
- **USER_SIGNATURE**: `ilja291220250407`
- **SUPERSEDES**: WP-1-Operator-Consoles-v1 (spec drift vs `.GOV/roles_shared/SPEC_CURRENT.md` + spec-to-code gaps found during revalidation)

---

## User Context (Non-Technical Explainer)

Operator Consoles are the "control room" for debugging what Handshake is doing. This packet fixes spec compliance gaps so that Problems/Jobs/Timeline/Evidence can reliably deep-link, group diagnostics deterministically, and store diagnostics in DuckDB as defined by the Master Spec.

---

## Goal

### SCOPE
Remediate Operator Consoles v1 implementation to conform to Master Spec v02.98 main-body requirements for:
- 10.5.5.1-10.5.5.4 (Problems, Jobs, Timeline, Evidence Drawer)
- 11.4 DIAG-SCHEMA-001/003 and 11.4.2/11.4.3 (DuckDB schema + `DiagnosticsStore`)
- 11.5 FR-EVT-003 (DiagnosticEvent fields needed for console filtering/linking)

### In-scope paths
- Backend:
  - `src/backend/handshake_core/src/diagnostics/mod.rs`
  - `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
  - `src/backend/handshake_core/src/flight_recorder/mod.rs`
  - `src/backend/handshake_core/src/api/diagnostics.rs`
  - `src/backend/handshake_core/src/api/flight_recorder.rs`
  - `src/backend/handshake_core/src/api/jobs.rs`
  - `src/backend/handshake_core/src/jobs.rs`
- Frontend:
  - `app/src/components/operator/ProblemsView.tsx`
  - `app/src/components/operator/JobsView.tsx`
  - `app/src/components/operator/TimelineView.tsx`
  - `app/src/components/operator/EvidenceDrawer.tsx`
  - `app/src/App.tsx`
  - `app/src/lib/api.ts`

### Out of scope
- Implementing 10.5.5.5-10.5.5.7 (Policy Inspector, Connector Health, Index Doctor).
- Full Debug Bundle export system (10.5.6.*) - handled in WP-1-Debug-Bundle-*.

---

## Quality Gate

### RISK_TIER: HIGH
- Cross-module changes (backend + frontend + schema/determinism)
- Core observability and operator triage loop

### DONE_MEANS (spec compliance)
- DIAG-SCHEMA-001:
  - `DiagnosticRange` JSON field names match the spec (`startLine`, `startColumn`, `endLine`, `endColumn`).
- DIAG-SCHEMA-003:
  - Fingerprint canonical tuple treats absent optional fields as explicit `null` (including absent `locations`).
  - Fingerprint location identifiers are sorted deterministically (order-independent).
- 11.4.2 DuckDB schema:
  - `diagnostics` table matches the spec types for `severity`, `link_confidence`, and `timestamp`.
  - Timestamp parsing is compatible with DuckDB timestamp formatting.
- 10.5.5.1 Problems:
  - Supports open/ack/mute/resolved group status (local-only metadata allowed).
  - Provides access to raw diagnostic instances for a fingerprint group (not just grouped row).
- 10.5.5.2 Jobs:
  - Adds workspace (`wsid`) filter capability for job listing (requires associating jobs with workspace when created from `doc_id`).
  - Inputs/Outputs are hash-based and deterministic.
- 10.5.5.3 Timeline:
  - Adds a surface filter for events (at minimum for DiagnosticEvent via payload fields).
- 10.5.5.4 Evidence Drawer:
  - Raw JSON view is redacted-by-default (full JSON requires explicit operator action).
  - Provides "copy as coder prompt" action.
  - Provides deep-link actions (Open Job / Open Timeline slice) and emits a Diagnostic on navigation failure (VAL-NAV-001).

### TEST_PLAN
```bash
# Backend
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# Frontend
pnpm -C app run lint
pnpm -C app test

# Governance / workflow
just validator-scan WP-1-Operator-Consoles-v2
just validator-spec-regression
just post-work WP-1-Operator-Consoles-v2
```

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

---

## Authority
- **SPEC_CURRENT**: `Handshake_Master_Spec_v02.98.md`
- **SPEC_ANCHOR**:
  - 10.5.5.1-10.5.5.4 (Operator Consoles surfaces)
  - 11.4 (DIAG-SCHEMA-001/003, 11.4.2, 11.4.3)
  - 11.5 FR-EVT-003 (DiagnosticEvent)
- **Codex**: `Handshake Codex v1.4.md`
- **Task Board**: `.GOV/roles_shared/TASK_BOARD.md`

---

## BOOTSTRAP
- **FILES_TO_OPEN**:
  - `.GOV/roles_shared/SPEC_CURRENT.md`
  - `Handshake_Master_Spec_v02.98.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `src/backend/handshake_core/src/diagnostics/mod.rs`
  - `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
  - `app/src/components/operator/ProblemsView.tsx`
  - `app/src/components/operator/EvidenceDrawer.tsx`
  - `app/src/App.tsx`
- **SEARCH_TERMS**:
  - `"DIAG-SCHEMA-001"`, `"DIAG-SCHEMA-003"`
  - `"startLine"`, `"start_line"`
  - `"CREATE TABLE diagnostics"`
  - `"link_confidence"`, `"severity"`
  - `"VAL-NAV-001"`, `"copy as coder prompt"`
- **RUN_COMMANDS**: See TEST_PLAN.
- **RISK_MAP**:
  - "Schema type mismatch" -> DuckDB diagnostics sink
  - "Fingerprint drift" -> Problems grouping determinism
  - "Navigation no-op" -> Operator triage loop failure (VAL-NAV-001)
  - "UI leaks raw payload" -> Safe-default violation (10.5.3 P-04)

---

## SKELETON (Proposed)

### Backend (Rust)
- `DiagnosticRange` serde field renames to match DIAG-SCHEMA-001 (`startLine`, `startColumn`, `endLine`, `endColumn`).
- `compute_fingerprint()` canonical tuple:
  - `locations: null` when absent
  - location canonical entries sorted by stable key (path|uri|entity_id|wsid + range)
- DuckDB `diagnostics` table DDL updated to match 11.4.2; `parse_timestamp` accepts both RFC3339 and DuckDB timestamp formats.
- `FrEvt003Diagnostic` payload extended with `wsid` and `surface` (string) for console filtering.
- Jobs:
  - `create_job(..., entity_refs: Vec<EntityRef>)`
  - `GET /api/jobs` supports `wsid` query param (filters by workspace entity_ref)
  - `POST /api/jobs` populates workspace entity_ref when `doc_id` is present.

### Frontend (React)
- `DiagnosticRange` type uses `startLine/startColumn/endLine/endColumn` (spec-canonical).
- Problems:
  - local status map by fingerprint (`open|acknowledged|muted|resolved`)
  - fetch and render raw instances for selected fingerprint group
- Jobs:
  - add wsid filter input and pass to `listJobs`
  - inputs/outputs hash computed via SHA-256 (WebCrypto)
- Timeline:
  - add surface filter input and pass to `getEvents`
- Evidence Drawer:
  - redacted-by-default JSON view (toggle for full JSON)
  - "copy as coder prompt" action
  - "Open Job" / "Open Timeline" actions using App-level navigation callbacks
  - on navigation failure, call `createDiagnostic` with `source=system` (VAL-NAV-001)

---

## Skeleton Approval

**RISK_TIER**: HIGH
**USER_SIGNATURE**: `ilja291220250407`

SKELETON APPROVED ilja291220250407

---

## Validation

- Target File: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
- Start: 7
- End: 700
- Line Delta: 28
- Pre-SHA1: bf38ecf8d8e808e3b01e4fd85382fa0803c0ee40
- Post-SHA1: 44976b5950da198a1d792020817d7602b22513c8
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
- **Lint Results**: `pnpm -C app run lint` PASS
- **Artifacts**: None
- **Timestamp**: 2025-12-29T04:44:18Z
- **Operator**: validator-gpt (ilja291220250407)

*(APPEND-ONLY once validation starts.)*

## Validation Report (Append Only)

VALIDATION REPORT - WP-1-Operator-Consoles-v2
Verdict: PASS

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Operator-Consoles-v2.md (status: Done)
- Spec: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.98.md (anchors: 10.5.5.1-10.5.5.4, 11.4 DIAG-SCHEMA-001/003, 11.4.2, 11.4.3, 11.5 FR-EVT-003)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md

Files Checked:
- .GOV/task_packets/WP-1-Operator-Consoles-v2.md
- .GOV/roles_shared/SPEC_CURRENT.md
- Handshake_Master_Spec_v02.98.md
- .GOV/roles/validator/VALIDATOR_PROTOCOL.md
- .GOV/roles_shared/TASK_BOARD.md
- src/backend/handshake_core/src/diagnostics/mod.rs
- src/backend/handshake_core/src/flight_recorder/duckdb.rs
- src/backend/handshake_core/src/flight_recorder/mod.rs
- src/backend/handshake_core/src/api/flight_recorder.rs
- src/backend/handshake_core/src/api/jobs.rs
- src/backend/handshake_core/src/jobs.rs
- src/backend/handshake_core/src/storage/mod.rs
- src/backend/handshake_core/src/storage/sqlite.rs
- src/backend/handshake_core/src/storage/postgres.rs
- src/backend/handshake_core/src/bundles/zip.rs
- src/backend/handshake_core/src/bundles/validator.rs
- app/src/lib/api.ts
- app/src/components/operator/ProblemsView.tsx
- app/src/components/operator/JobsView.tsx
- app/src/components/operator/TimelineView.tsx
- app/src/components/operator/EvidenceDrawer.tsx
- app/src/App.tsx

Findings (Spec -> Evidence):
- DIAG-SCHEMA-001: DiagnosticRange serializes startLine/startColumn/endLine/endColumn at src/backend/handshake_core/src/diagnostics/mod.rs:275.
- DIAG-SCHEMA-001 (frontend type): app/src/lib/api.ts:182.
- DIAG-SCHEMA-003: locations is explicit null when absent at src/backend/handshake_core/src/diagnostics/mod.rs:546.
- DIAG-SCHEMA-003: deterministic location ordering in canonical tuple at src/backend/handshake_core/src/diagnostics/mod.rs:555 and src/backend/handshake_core/src/diagnostics/mod.rs:622; covered by src/backend/handshake_core/src/diagnostics/mod.rs:971.
- 11.4.2: DuckDB diagnostics schema matches spec (enums + timestamp TIMESTAMP) at src/backend/handshake_core/src/flight_recorder/duckdb.rs:223.
- 11.4.2: diagnostics query casts enum/timestamp to VARCHAR for stable Rust mapping at src/backend/handshake_core/src/flight_recorder/duckdb.rs:351.
- 11.5 FR-EVT-003: DiagnosticEvent payload fields (diagnostic_id, wsid?, severity?, source?) at src/backend/handshake_core/src/flight_recorder/mod.rs:223.
- 10.5.5.1 Problems: local status map + raw instance access by fingerprint at app/src/components/operator/ProblemsView.tsx:46 and app/src/components/operator/ProblemsView.tsx:90.
- 10.5.5.2 Jobs: wsid filter (frontend -> API -> storage) at app/src/components/operator/JobsView.tsx:19, app/src/lib/api.ts:483, src/backend/handshake_core/src/api/jobs.rs:117, src/backend/handshake_core/src/storage/sqlite.rs:1173, src/backend/handshake_core/src/storage/postgres.rs:990.
- 10.5.5.2 Jobs: deterministic IO hashes via stable JSON + SHA-256 at app/src/components/operator/JobsView.tsx:37.
- 10.5.5.3 Timeline: surface filter + event_id focus at app/src/components/operator/TimelineView.tsx:11 and src/backend/handshake_core/src/api/flight_recorder.rs:26.
- 10.5.5.4 Evidence Drawer: redacted-by-default JSON + copy-as-coder-prompt + deep links with VAL-NAV-001 on failure at app/src/components/operator/EvidenceDrawer.tsx:52, app/src/components/operator/EvidenceDrawer.tsx:59, app/src/components/operator/EvidenceDrawer.tsx:311, and app/src/App.tsx:152.

Hygiene:
- just validator-scan: PASS
- just validator-spec-regression: PASS
- just validator-dal-audit: PASS
- just post-work WP-1-Operator-Consoles-v2: PASS

Tests:
- just cargo-clean: PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS
- pnpm -C app run lint: PASS
- pnpm -C app test: PASS

Risks & Suggested Actions:
- None required for this WP scope.

Improvements & Future Proofing:
- Consider narrowing .GOV/scripts/validation/validator-scan.mjs to exclude #[cfg(test)] modules if test-only patterns should be allowed (policy decision).

REASON FOR PASS:
- DONE_MEANS items satisfied with file:line evidence; required validator gates and TEST_PLAN commands ran and passed; COR-701 manifest validated via just post-work.

Timestamp: 2025-12-29T05:03:30Z
Operator: validator-gpt (ilja291220250407)

---

Packet Status Update (append-only):
- **Status:** Ready for Dev

REVALIDATION REPORT - WP-1-Operator-Consoles-v2
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Operator-Consoles-v2.md (previous Status: Done)
- Spec: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.98.md (anchors: 10.5.5.1-10.5.5.4, 11.4.2/11.4.3, 11.5 FR-EVT-003)
- Codex: Handshake Codex v1.4.md
- Validator Protocol: .GOV/roles/validator/VALIDATOR_PROTOCOL.md

Blocking Failures (Protocol Gates):
- Packet completeness gate: `just validator-packet-complete WP-1-Operator-Consoles-v2` FAIL because the packet used `**STATUS**` instead of the required `**Status:**` marker (this appended Status line remediates the format issue going forward).
- Deterministic manifest gate: `just post-work WP-1-Operator-Consoles-v2` FAIL
  - C701-G05: post_sha1 mismatch for `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
  - C701-G04: git diff touches lines outside declared window [7, 700] for `src/backend/handshake_core/src/flight_recorder/duckdb.rs`

Validation Commands Run (non-blocking but recorded):
- just cargo-clean: PASS
- just validator-spec-regression: PASS
- just validator-scan: PASS
- just validator-dal-audit: PASS
- cargo test --manifest-path src/backend/handshake_core/Cargo.toml: PASS
- pnpm -C app run lint: PASS
- pnpm -C app test: PASS

REASON FOR FAIL:
- `just post-work WP-1-Operator-Consoles-v2` does not pass, so the COR-701 deterministic manifest does not match the current code state. Per VALIDATOR_PROTOCOL, this blocks a PASS verdict regardless of other test results.

Required Fixes (to return to Done):
- Re-open remediation as a new WP (recommended: WP-1-Operator-Consoles-v3) or explicitly revalidate WP-1-Operator-Consoles-v2 in a clean diff context.
- Re-capture a correct COR-701 Validation manifest for `src/backend/handshake_core/src/flight_recorder/duckdb.rs` so that:
  - manifest post_sha1 matches the file on disk
  - git diff stays within the declared Start/End window
  - `just post-work WP-1-Operator-Consoles-v2` passes

Task Board Update:
- Move WP-1-Operator-Consoles-v2 from Done -> Ready for Dev (FAIL revalidation; deterministic manifest gate failure).

Timestamp: 2025-12-30T20:17:39.6328260+01:00
Validator: codex-cli (Validator role)



