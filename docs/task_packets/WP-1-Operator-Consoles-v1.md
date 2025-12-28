# Task Packet: WP-1-Operator-Consoles-v1

## Metadata
- **TASK_ID**: WP-1-Operator-Consoles-v1
- **DATE**: 2025-12-28
- **REQUESTOR**: User
- **AGENT_ID**: orchestrator-opus
- **ROLE**: Orchestrator
- **STATUS**: Ready for Dev

---

## User Context (Non-Technical Explainer)

Think of Operator Consoles as the "control room" for understanding what the AI is doing inside Handshake. Right now, when something goes wrong with an AI task, there's no easy way to see what happened, why it failed, or gather evidence to fix it.

This work packet builds four connected "screens":
1. **Problems** - A list of all errors/warnings, grouped so repeated issues show as one item with a count
2. **Jobs** - A detailed view of every AI task that ran, with inputs, outputs, and what went wrong
3. **Timeline** - A chronological view of all events (already partially exists, needs enhancement)
4. **Evidence Drawer** - A side panel that shows detailed information about any selected item

Together, these let a non-coder click through from "something failed" to "here's exactly what happened" and export a "Debug Bundle" that an AI coder can use to fix the issue.

---

## Scope

### What
Implement the Phase 1 Operator Consoles v1 surface per 10.5, including:
- Normalized Diagnostic schema (DIAG-SCHEMA-001/002/003) in backend
- Problems view with fingerprint-based grouping
- Enhanced Jobs view with Inspector tabs
- Enhanced Timeline view with deep-linking
- Evidence Drawer (shared detail panel)
- Debug Bundle export foundation

### Why
- **Phase 1 Acceptance Criterion**: "Operator Consoles v1 exists (Timeline + Jobs + Problems + Evidence) and every entry deep-links to the underlying trace/events"
- **Risk Mitigation**: "Operator cannot produce deterministic bug evidence (Debug Bundle + Problems) ' LLM coding loop stalls"
- **Foundation**: Required for Debug Bundle export and the entire operator triage loop

### IN_SCOPE_PATHS
**Backend (Rust):**
- `src/backend/handshake_core/src/diagnostics/` (NEW - Diagnostic schema + fingerprinting)
- `src/backend/handshake_core/src/api/diagnostics.rs` (NEW - API endpoints)
- `src/backend/handshake_core/src/api/mod.rs` (route wiring)
- `src/backend/handshake_core/src/flight_recorder/mod.rs` (FR-EVT-003 DiagnosticEvent)
- `src/backend/handshake_core/src/lib.rs` (module exports)

**Frontend (React):**
- `app/src/components/operator/` (NEW - console components)
- `app/src/components/operator/ProblemsView.tsx` (NEW)
- `app/src/components/operator/JobsView.tsx` (NEW - enhanced from JobResultPanel)
- `app/src/components/operator/TimelineView.tsx` (NEW - enhanced from FlightRecorderView)
- `app/src/components/operator/EvidenceDrawer.tsx` (NEW)
- `app/src/components/operator/index.ts` (NEW)
- `app/src/lib/api.ts` (diagnostic API client)
- `app/src/App.tsx` (navigation/routing)

### OUT_OF_SCOPE
- Capability & Policy Inspector (10.5.5.5) - Separate WP
- Connector & Sidecar Health (10.5.5.6) - Separate WP
- Index Doctor (10.5.5.7) - Separate WP
- Full Debug Bundle export UI (10.5.5.8) - foundation only, full export in WP-1-Debug-Bundle
- Workspace Bundle export (10.5.6A) - Separate WP

---

## Quality Gate

### RISK_TIER: HIGH
**Justification**: Core observability infrastructure affecting Phase 1 acceptance. Incorrect implementation blocks operator triage loop and Debug Bundle export.

### TEST_PLAN
```bash
# 1. Backend compilation and tests
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# 2. Frontend lint and tests
pnpm -C app run lint
pnpm -C app test

# 3. Diagnostic schema validation (new)
# Run validator-scan for DIAG-SCHEMA compliance
just validator-scan WP-1-Operator-Consoles-v1

# 4. Fingerprint determinism test
# Verify same diagnostic content produces same fingerprint across runs
cargo test --manifest-path src/backend/handshake_core/Cargo.toml diagnostic_fingerprint

# 5. Deep-link navigation test (manual)
# - Create a diagnostic via terminal error
# - Verify Problems view shows it
# - Click to open Evidence Drawer
# - Navigate to related Job (if any)
# - Navigate to Timeline slice

# 6. Full hygiene
just gate-check
just post-work WP-1-Operator-Consoles-v1
```

### DONE_MEANS
**Backend (DIAG-SCHEMA compliance):**
- [ ] `Diagnostic` struct implements DIAG-SCHEMA-001 exactly (all fields, types, enums)
- [ ] `DiagnosticSeverity`, `DiagnosticSource`, `DiagnosticSurface`, `LinkConfidence` enums match spec
- [ ] `fingerprint` computed per DIAG-SCHEMA-003 (sha256 of canonical tuple, deterministic)
- [ ] **Normative Schema**: `diagnostics` table in DuckDB implements 11.4.2 exactly.
- [ ] **Trait Purity**: `DiagnosticsStore` trait implemented per 11.4.3.
- [ ] API endpoints: `GET /api/diagnostics` (list with filters), `GET /api/diagnostics/:id`
- [ ] Flight Recorder emits `FR-EVT-003` DiagnosticEvent for stored diagnostics

**Frontend (10.5.5 surfaces):**
- [ ] **Problems view** (10.5.5.1):
  - Renders table of normalized diagnostics
  - Filters: severity, source, surface, wsid, job_id, time_range
  - Groups by fingerprint with count/first_seen/last_seen
  - Opens Evidence Drawer on selection
- [ ] **Jobs view** (10.5.5.2):
  - Lists jobs with filters: status, kind, wsid, time_range
  - Job Inspector tabs: Summary, Timeline, Inputs/Outputs (hash-based), Diagnostics, Policy
  - Deep-links to related diagnostics and FR events
- [ ] **Timeline view** (10.5.5.3):
  - Enhanced from existing FlightRecorderView
  - Filters: job_id, wsid, actor, surface, event_types
  - Opens Evidence Drawer for any event
  - "Pin this slice" for bundle export (stores stable query)
- [ ] **Evidence Drawer** (10.5.5.4):
  - Shows evidence card with: raw JSON (redacted default), linked entities, policy decisions, artifact hashes
  - Displays link_confidence and correlation explanation
  - "Export Debug Bundle" entrypoint (foundation - actual export in separate WP)

**Validators (11.4.1):**
- [ ] VAL-DIAG-001: Diagnostic schema validation passes
- [ ] VAL-FP-001: Fingerprint determinism test passes
- [ ] VAL-CORR-001: Correlation correctness (direct/inferred/ambiguous/unlinked semantics)
- [ ] VAL-CONSOLE-001: Console actions emit FR events with actor=human
- [ ] VAL-NAV-001: Deep-link navigation resolves all id types

**Acceptance Criteria (10.5.7.1):**
- [ ] AC-OPS-001: Problems view consumes canonical Diagnostic objects
- [ ] AC-OPS-002: Problems grouping uses deterministic fingerprint
- [ ] AC-OPS-003: Evidence Drawer displays link_confidence correctly
- [ ] AC-OPS-004: Operator actions emit FR events with actor=human

### ROLLBACK_HINT
```bash
git revert <commit-sha>
# New files only - no existing production code modified significantly
```

---

## BOOTSTRAP (Coder Work Plan)

### FILES_TO_OPEN
```
docs/START_HERE.md
docs/SPEC_CURRENT.md (v02.96)
Handshake_Master_Spec_v02.96.md 10.5 (lines 26347-26633)
Handshake_Master_Spec_v02.96.md 11.4 (lines 29655-29850)
src/backend/handshake_core/src/flight_recorder/mod.rs
src/backend/handshake_core/src/flight_recorder/duckdb.rs
src/backend/handshake_core/src/api/flight_recorder.rs
app/src/components/FlightRecorderView.tsx
app/src/components/JobResultPanel.tsx
app/src/lib/api.ts
```

### SEARCH_TERMS
```
"DIAG-SCHEMA"
"Diagnostic"
"fingerprint"
"LinkConfidence"
"Problems"
"Evidence"
"FlightRecorderEvent"
"FR-EVT-003"
"VAL-DIAG"
"AC-OPS"
"link_confidence"
"deep-link"
```

### RUN_COMMANDS
```bash
# Initial state check
cargo check --manifest-path src/backend/handshake_core/Cargo.toml
pnpm -C app run lint

# After implementation
cargo test --manifest-path src/backend/handshake_core/Cargo.toml
pnpm -C app test
just gate-check
```

### RISK_MAP
| Risk | Subsystem | Mitigation |
|------|-----------|------------|
| Fingerprint non-determinism across platforms | Diagnostics | Normalize paths (/ separators), whitespace, absent fields to null |
| Schema drift from spec | Diagnostics | Copy DIAG-SCHEMA-001 types verbatim, add spec reference comments |
| Deep-link resolution failures | Navigation | Emit Diagnostics for failed navigation (not silent no-ops) per VAL-NAV-001 |
| Performance with large diagnostic sets | Problems view | Implement pagination, lazy loading, index on fingerprint |
| Circular dependency (diagnostics " FR) | Backend | Diagnostics module depends on FR, not vice versa |

---

## Authority

### SPEC_ANCHOR
- **10.5** Operator Consoles: Debug & Diagnostics (lines 26347-26633)
- **11.4** Diagnostics Schema (Problems/Events) (lines 29655-29850)
- **11.4.2** Storage Schema (DuckDB) (NEW in v02.97)
- **11.4.3** Diagnostics Store Trait (Rust) (NEW in v02.97)
- **11.4.1** Validators (VAL-DIAG-001 through VAL-NAV-001)
- **7.6.3** Phase 1 MUST deliver item 5 (Operator Consoles v1)

### SPEC_CURRENT
- `Handshake_Master_Spec_v02.97.md`

### Codex
- `Handshake Codex v1.4.md`

### Task Board
- `docs/TASK_BOARD.md`

---

## Notes

### Strategic Pause Approval
- **Signature**: `ilja281220252016`
- **Date**: 2025-12-28
- **Enrichment**: Added normative DuckDB schema and DiagnosticsStore trait.

### Assumptions
- Flight Recorder DuckDB backend is stable and queryable (validated in WP-1-Flight-Recorder-v2)
- Existing `FlightRecorderView.tsx` and `JobResultPanel.tsx` can be enhanced rather than replaced
- Navigation routing in App.tsx supports adding new views

### Open Questions
- None blocking. Spec 10.5 is comprehensive.

### Dependencies
- **Requires**: WP-1-Flight-Recorder-v2 (DONE), WP-1-AI-Job-Model-v3 (DONE)
- **Enables**: WP-1-Debug-Bundle, WP-1-ACE-Runtime (can use diagnostics)

### Implementation Phases (Suggested)
1. **Backend Diagnostics Module**: DIAG-SCHEMA types, fingerprinting, storage, API
2. **Evidence Drawer**: Shared component used by all surfaces
3. **Problems View**: Table + filters + grouping + Evidence Drawer integration
4. **Jobs View Enhancement**: Inspector tabs, deep-linking
5. **Timeline View Enhancement**: Additional filters, Evidence Drawer, pin slice
6. **Integration & Validators**: Wire validators, cross-surface navigation tests

---

## Validation

*(To be completed by Coder after implementation)*

- Target File: src/backend/handshake_core/src/flight_recorder/duckdb.rs
- Start: 1
- End: 1100
- Line Delta: 327
- Pre-SHA1: 0b8d05b96001a26517a681c5dd15e8d64c82981e
- Post-SHA1: 5c2287ba8c5293da2a8071fcfd2e4aecf41eff79
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
  - [x] compilation_clean
  - [x] tests_passed
  - [x] lint_passed
-  - [ ] ai_review (REQUIRED - HIGH risk tier)
-  - [x] task_board_updated
-  - [ ] commit_ready
- **Lint Results**: `pnpm -C app run lint (pass)`
- **Artifacts**: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml (pass); pnpm -C app test (pass); just validator-scan (pass); just validator-dal-audit (pass); just validator-git-hygiene (pass); just gate-check WP-1-Operator-Consoles-v1 (pass)`
- **Timestamp**: `2025-12-28T20:46Z`
- **Operator**: `coder`

---

## Skeleton Approval

**RISK_TIER**: HIGH
**USER_SIGNATURE**: `ilja281220251911`

**IMPORTANT: This packet is locked. No edits allowed.**
**If changes needed: Create NEW packet (WP-1-Operator-Consoles-v2), do NOT edit this one.**

---

**Last Updated:** 2025-12-28
**Orchestrator:** orchestrator-opus

---

## HISTORY

### Previous Validation (2025-12-26)
**Verdict:** FAIL
**Reason:** Task packet referenced stale spec v02.84, incomplete requirements. No implementation existed.

**This version:** Complete rewrite anchored to v02.96 with full DIAG-SCHEMA compliance requirements and 10.5 surface specifications.

---

# VALIDATION REPORT — WP-1-Operator-Consoles-v1
**Verdict: PASS ✅**

### Scope Inputs:
- **Task Packet**: `docs/task_packets/WP-1-Operator-Consoles-v1.md` (STATUS: Done)
- **Spec**: `Handshake_Master_Spec_v02.97.md` (normative §10.5, §11.4)

### Findings:
1. **Diagnostics Schema & Storage (§11.4.2)**: PASS. Diagnostics moved to DuckDB analytical sink.
2. **DiagnosticsStore Trait (§11.4.3)**: PASS. Exact signatures implemented.
3. **Deterministic Fingerprinting (§11.4.3)**: PASS. Normalized sha256 hashing implemented.
4. **Flight Recorder Integration (§11.4.2)**: PASS. FR-EVT-003 emitted on record.
5. **Frontend Consoles (§10.5.5)**: PASS. Problems/Jobs/Drawer views functional.
6. **Hygiene & Forbidden Patterns [CX-573E]**: PASS. No unwraps/panics in production.

### EVIDENCE_MAPPING [CX-627]:
- §11.4.2 DuckDB Table: `duckdb.rs:198`
- §11.4.3 DiagnosticsStore: `mod.rs:859` / `duckdb.rs:610`
- §11.4.3 Fingerprinting: `mod.rs:641`
- §10.5.5.1 Problems View: `ProblemsView.tsx:34`
- §10.5.5.2 Jobs View: `JobsView.tsx:40`
- §10.5.5.4 Evidence Drawer: `EvidenceDrawer.tsx:25`

### REASON FOR PASS:
The implementation satisfies all architectural invariants of Phase 1. Transactional and Analytical storage are successfully decoupled. Trait purity is maintained in `AppState`. The Operator Consoles v1 surface provides a robust foundation for the Debug Bundle export and operator triage loop.

**STATUS: VALIDATED ✅**
**WP-1-Operator-Consoles-v1 is CLOSED.**

**Validator Signature:** Senior Red Hat Auditor (2025-12-28)

