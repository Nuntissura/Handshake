# Task Packet: WP-1-Debug-Bundle-v2

## Metadata
- **TASK_ID**: WP-1-Debug-Bundle-v2
- **DATE**: 2025-12-29
- **REQUESTOR**: User
- **AGENT_ID**: orchestrator-opus
- **ROLE**: Orchestrator
- **STATUS**: Ready for Dev
- **SUPERSEDES**: WP-1-Debug-Bundle (stale SPEC_ANCHOR v02.84, incomplete structure)

---

## User Context (Non-Technical Explainer)

A Debug Bundle is a "bug report in a box" - a ZIP file containing everything an AI coder needs to understand and fix a problem without asking follow-up questions. When something goes wrong in Handshake, the operator can click "Export Debug Bundle" and get a standardized package containing:

1. **What failed** - The error message and context
2. **What happened** - A timeline of events from the Flight Recorder
3. **What was involved** - Job details, diagnostics, environment info
4. **What to do** - A pre-written prompt for an LLM coder

The key innovation is **redaction-safe by default**: secrets, passwords, file paths, and personal information are automatically removed, making it safe to share with external AI tools without leaking sensitive data.

---

## Scope

### What
Implement the Debug Bundle export system per 10.5.6.5-12, including:
- Backend: DebugBundleExporter trait (HSK-TRAIT-005), bundle file generators, secret redactor
- API: Export/status/download/validate endpoints
- Frontend: Export modal, progress display, completion view
- Job: `debug_bundle_export_v0` profile with capability gating
- Validators: VAL-BUNDLE-001 implementation

### Why
- **Phase 1 Acceptance Criterion**: "Debug Bundle export is redacted-by-default, deterministic for the same selection, and passes the validator pack in CI"
- **Risk Mitigation**: "Operator cannot produce deterministic bug evidence (Debug Bundle + Problems) -> LLM coding loop stalls"
- **Enables**: AI-assisted debugging workflow where coders receive complete, safe evidence packets

### IN_SCOPE_PATHS
**Backend (Rust):**
- `src/backend/handshake_core/src/bundles/` (NEW - bundle module)
- `src/backend/handshake_core/src/bundles/mod.rs` (NEW - module root)
- `src/backend/handshake_core/src/bundles/exporter.rs` (NEW - DebugBundleExporter trait + impl)
- `src/backend/handshake_core/src/bundles/schemas.rs` (NEW - BundleManifest, BundleEnv, etc.)
- `src/backend/handshake_core/src/bundles/redactor.rs` (NEW - SecretRedactor)
- `src/backend/handshake_core/src/bundles/templates.rs` (NEW - repro.md, coder_prompt.md generators)
- `src/backend/handshake_core/src/bundles/validator.rs` (NEW - VAL-BUNDLE-001)
- `src/backend/handshake_core/src/bundles/zip.rs` (NEW - deterministic ZIP creation)
- `src/backend/handshake_core/src/api/bundles.rs` (NEW - API endpoints)
- `src/backend/handshake_core/src/api/mod.rs` (route wiring)
- `src/backend/handshake_core/src/lib.rs` (module exports)
- `src/backend/handshake_core/src/workflows.rs` (job profile registration)
- `src/backend/handshake_core/src/capabilities.rs` (capability registry - add `export.debug_bundle`)
- `src/backend/handshake_core/src/flight_recorder/mod.rs` (FR-EVT-005 DebugBundleExportEvent + renumber security event)
- `src/backend/handshake_core/src/flight_recorder/duckdb.rs` (event storage if needed)

**Frontend (React):**
- `app/src/components/operator/DebugBundleExport.tsx` (NEW - export modal)
- `app/src/components/operator/DebugBundleProgress.tsx` (NEW - progress view)
- `app/src/components/operator/DebugBundleComplete.tsx` (NEW - completion view)
- `app/src/lib/api.ts` (bundle API client)
- `app/src/components/operator/EvidenceDrawer.tsx` (add export button)
- `app/src/components/operator/JobsView.tsx` (add export context menu)
- `app/src/components/operator/ProblemsView.tsx` (add export context menu)
- `app/src/components/operator/TimelineView.tsx` (add "Export time range" action)

### OUT_OF_SCOPE
- Workspace Bundle export (10.5.6A) - Separate WP
- Cloud sharing/upload of bundles - Phase 2+
- Custom redaction patterns UI - Phase 2+
- Bundle import/replay - Phase 2+

---

## Quality Gate

### RISK_TIER: HIGH
**Justification**: Security-critical (redaction), Phase 1 acceptance blocker, foundation for LLM-assisted debugging workflow.

### HARDENED_INVARIANTS [CX-VAL-HARD]
1. **SAFE_DEFAULT redaction**: No secrets, PII, or absolute paths in SAFE_DEFAULT bundles
2. **Deterministic output**: Same inputs + same redaction mode = same bundle hash
3. **Capability gating**: Export requires `export.debug_bundle` capability
4. **FR event emission**: Every export MUST emit FR-EVT-005 (DebugBundleExportEvent)

### FR-EVT RENUMBERING NOTE
**Current code state:**
- FR-EVT-005 = Security violation event (line 238 in mod.rs)
- FR-EVT-006 = Workflow recovery event
- FR-EVT-007 = Terminal command event

**Spec requirement (11.5 line 30867):**
- FR-EVT-005 = DebugBundleExportEvent

**Resolution:** Renumber existing security violation from FR-EVT-005 to FR-EVT-008, then implement FR-EVT-005 as DebugBundleExportEvent per spec.

### TEST_PLAN
```bash
# 1. Backend compilation and tests
cargo test --manifest-path src/backend/handshake_core/Cargo.toml

# 2. Frontend lint and tests
pnpm -C app run lint
pnpm -C app test

# 3. Bundle schema validation
just validator-scan WP-1-Debug-Bundle-v2

# 4. Redaction compliance test
cargo test --manifest-path src/backend/handshake_core/Cargo.toml redaction

# 5. Determinism test
# Create two bundles with same inputs, verify identical hashes
cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundle_determinism

# 6. VAL-BUNDLE-001 validator test
cargo test --manifest-path src/backend/handshake_core/Cargo.toml val_bundle_001

# 7. Full hygiene
just gate-check
just post-work WP-1-Debug-Bundle-v2
```

### DONE_MEANS
**Backend (HSK-TRAIT-005 + schemas):**
- [ ] `DebugBundleExporter` trait implements exact signatures from 10.5.6.6
- [ ] `BundleManifest` struct matches 10.5.6.5.1 exactly
- [ ] `BundleEnv` struct matches 10.5.6.5.2 exactly
- [ ] `BundleJob` struct matches 10.5.6.5.3 exactly
- [ ] `BundleDiagnostic` struct matches 10.5.6.5.4 exactly
- [ ] `RetentionReport` struct matches 10.5.6.5.6 exactly
- [ ] `RedactionReport` struct matches 10.5.6.5.7 exactly
- [ ] `repro.md` template matches 10.5.6.5.8 exactly
- [ ] `coder_prompt.md` template matches 10.5.6.5.9 exactly

**Backend (redaction):**
- [ ] SecretRedactor implements all pattern categories from 10.5.6.9
- [ ] Redaction output format: `[REDACTED:<category>:<detector_id>]`
- [ ] SAFE_DEFAULT blocks all absolute paths, secrets, PII
- [ ] RedactionReport captures all redactions with locations

**Backend (determinism):**
- [ ] ZIP normalization per 10.5.6.10 (sorted entries, fixed timestamps, DEFLATE 6)
- [ ] SHA-256 for all hashes, hex-encoded lowercase
- [ ] Bundle hash computation per 10.5.6.10

**Backend (API):**
- [ ] `POST /api/bundles/debug/export` - initiate export
- [ ] `GET /api/bundles/debug/:bundle_id` - get status
- [ ] `GET /api/bundles/debug/:bundle_id/download` - download ZIP
- [ ] `GET /api/bundles/debug/exportable` - list exportable items
- [ ] `POST /api/bundles/debug/:bundle_id/validate` - run VAL-BUNDLE-001

**Backend (job profile + capabilities + FR event):**
- [ ] `debug_bundle_export_v0` profile registered in `workflows.rs`
- [ ] Capability `export.debug_bundle` added to `capabilities.rs`
- [ ] Capabilities enforced: `export.debug_bundle`, `fr.read`, `diagnostics.read`, `jobs.read`
- [ ] FR-EVT-005 renumbered: security violation -> FR-EVT-008 in `flight_recorder/mod.rs`
- [ ] FR-EVT-005 = DebugBundleExportEvent implemented per 11.5
- [ ] Export completion emits FR-EVT-005 with correct payload

**Frontend (10.5.6.11):**
- [ ] Export triggers from: Evidence Drawer, Jobs View, Problems View, Timeline View
- [ ] Export modal with scope selection and redaction mode radio buttons
- [ ] Progress display with step indicators
- [ ] Completion view with Copy Path, Open Folder, Done buttons

**Validators:**
- [ ] VAL-BUNDLE-001: All 5 check categories implemented (files, schema, consistency, redaction, missing evidence)
- [ ] VAL-REDACT-001: SAFE_DEFAULT leak check passes
- [ ] Bundle determinism test passes (same inputs = same hash)

**Acceptance Criteria (AC):**
- [ ] AC-OPS-005: Debug Bundle contains required minimum files
- [ ] AC-OPS-006: SAFE_DEFAULT export contains no secrets/PII

### ROLLBACK_HINT
```bash
git revert <commit-sha>
# New files only - bundles/ module is additive
```

---

## BOOTSTRAP (Coder Work Plan)

### FILES_TO_OPEN
```
docs/START_HERE.md
docs/SPEC_CURRENT.md
Handshake_Master_Spec_v02.98.md 10.5.6.5-12 (lines 26553-27382)
src/backend/handshake_core/src/flight_recorder/mod.rs
src/backend/handshake_core/src/flight_recorder/duckdb.rs
src/backend/handshake_core/src/diagnostics/mod.rs
src/backend/handshake_core/src/api/mod.rs
app/src/components/operator/EvidenceDrawer.tsx
app/src/lib/api.ts
```

### SEARCH_TERMS
```
"DebugBundleExporter"
"BundleManifest"
"BundleScope"
"RedactionMode"
"SAFE_DEFAULT"
"VAL-BUNDLE-001"
"FR-EVT-005"
"debug_bundle_export"
"SecretRedactor"
"bundle_hash"
"coder_prompt"
"repro.md"
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
| Secret leakage in SAFE_DEFAULT | Redactor | Comprehensive pattern list from 10.5.6.9; test with known secrets |
| Non-deterministic ZIP | zip.rs | Fixed timestamps, sorted entries, consistent compression |
| Path leakage on Windows vs Unix | Redactor | Normalize paths before redaction; test both path formats |
| Large bundle size | Exporter | Enforce 10,000 event limit; paginate if needed |
| Missing FR events | Exporter | Integration test: export -> verify FR-EVT-005 exists |
| Schema drift | schemas.rs | Copy types verbatim from 10.5.6.5; add spec reference comments |

---

## Authority

### SPEC_ANCHOR
- **10.5.5.8** Debug Bundle Export (high-level MUST)
- **10.5.6** Debug Bundle (export artifact) - structure and goals
- **10.5.6.5** Bundle File Schemas (Normative) - all 9 file schemas
- **10.5.6.6** DebugBundleExporter Trait (HSK-TRAIT-005)
- **10.5.6.7** API Endpoints
- **10.5.6.8** Job Profile `debug_bundle_export_v0`
- **10.5.6.9** Secret Redactor Integration
- **10.5.6.10** Determinism & Hashing
- **10.5.6.11** Frontend UI Specification
- **10.5.6.12** VAL-BUNDLE-001 (expanded validator)
- **11.5** FR-EVT-005 DebugBundleExportEvent

### SPEC_CURRENT
- `Handshake_Master_Spec_v02.98.md`

### Codex
- `Handshake Codex v1.4.md`

### Task Board
- `docs/TASK_BOARD.md`

---

## Notes

### Strategic Pause Approval
- **Signature**: `ilja291220250100`
- **Date**: 2025-12-29
- **Enrichment**: Added 10.5.6.5-12 with normative schemas, trait, API, job profile, redactor, determinism rules, and frontend UI spec.

### Assumptions
- Operator Consoles v1 is complete (Evidence Drawer, Jobs View, Problems View exist)
- Flight Recorder DuckDB backend is stable and queryable
- Diagnostics system from WP-1-Operator-Consoles-v1 is available
- Workflow Engine can register new job profiles

### Open Questions
- None blocking. Spec 10.5.6.5-12 is comprehensive.

### Dependencies
- **Requires**: WP-1-Operator-Consoles-v1 (DONE), WP-1-Flight-Recorder-v2 (DONE), WP-1-Workflow-Engine-v3 (DONE)
- **Enables**: AI-assisted debugging workflow, WP-1-Workspace-Bundle

### Implementation Phases (Suggested)
1. **FR-EVT renumbering**: Renumber security violation FR-EVT-005 -> FR-EVT-008
2. **Backend schemas**: BundleManifest, BundleEnv, BundleJob, etc. (10.5.6.5)
3. **Secret Redactor**: Pattern matching + redaction output (10.5.6.9)
4. **Bundle generators**: trace.jsonl, diagnostics.jsonl, templates (10.5.6.5.4-9)
5. **Deterministic ZIP**: Normalization + hashing (10.5.6.10)
6. **DebugBundleExporter trait + impl**: Core export logic (10.5.6.6)
7. **Job profile + capability**: Register in workflows.rs and capabilities.rs (10.5.6.8)
8. **API endpoints**: Export/status/download/validate (10.5.6.7)
9. **VAL-BUNDLE-001**: Validator implementation (10.5.6.12)
10. **Frontend - all 4 triggers**: Evidence Drawer, Jobs, Problems, Timeline (10.5.6.11)
11. **Integration tests**: End-to-end export + validation + FR event check

---

## Validation

*(To be completed by Coder after implementation)*

- Target File: src\backend\handshake_core\src\bundles\exporter.rs
- Start: 1
- End: 1165
- Line Delta: 1165
- Pre-SHA1: 0000000000000000000000000000000000000000
- Post-SHA1: b8b394b5713720a42417d5c312465b5aa8482c7a
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
  - [x] ai_review (REQUIRED - HIGH risk tier)
  - [x] task_board_updated
  - [x] commit_ready
- **Lint Results**: PASS
- **Artifacts**: diagnostics.jsonl, trace.jsonl, bundle_manifest.json, env.json, jobs.json, redaction_report.json, retention_report.json, repro.md, coder_prompt.md
- **Timestamp**: 2025-12-29T12:30:00Z
- **Operator**: validator-gemini

---

## Skeleton Approval

**RISK_TIER**: HIGH
**USER_SIGNATURE**: `ilja291220250100`

**IMPORTANT: This packet is locked. No edits allowed.**
**If changes needed: Create NEW packet (WP-1-Debug-Bundle-v3), do NOT edit this one.**

---

**Last Updated:** 2025-12-29
**Orchestrator:** orchestrator-opus


